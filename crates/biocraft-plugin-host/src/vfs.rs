//! Sanal Dosya Sistemi (VFS) — capability-kısıtlı **handle** (MK-13).
//!
//! Eklentiye gerçek bir yol **verilmez**; bir sandbox **köküne** kısıtlı handle verilir.
//! Eklenti yalnızca **göreli** yol verir; VFS bunu kök içinde çözer ve kök dışına
//! çıkışı (`..`, mutlak yol, `C:\` ön eki) **sözlüksel olarak** (dosya sistemine
//! dokunmadan) reddeder.  Böylece eklenti diske doğrudan erişemez.
//!
//! Not: `fs` yetkisi denetimi `capability.rs`'tedir; bu modül yetki **verildikten sonra**
//! erişimin kök içinde kalmasını garanti eder (savunmada derinlik: iki ayrı kapı).

use biocraft_types::ErrorReport;
use std::path::{Component, Path, PathBuf};

/// Bir eklentiye verilen, tek bir köke kısıtlı sanal dosya sistemi handle'ı.
#[derive(Debug, Clone)]
pub struct SanalDosyaSistemi {
    kok: PathBuf,
}

impl SanalDosyaSistemi {
    /// Verilen kök dizine kısıtlı yeni bir VFS handle'ı oluşturur.
    pub fn yeni(kok: impl Into<PathBuf>) -> Self {
        Self { kok: kok.into() }
    }

    /// Sandbox kök dizini.
    pub fn kok(&self) -> &Path {
        &self.kok
    }

    /// Göreli bir yolu kök içinde **güvenle** çözer; kök dışına çıkışı reddeder.
    ///
    /// Reddedilenler (hepsi `ErrorReport`):
    /// * mutlak yol (`/x`, `C:\x`),
    /// * üst-dizin çıkışı (`..`),
    /// * sürücü/ön ek bileşeni.
    pub fn cozumle(&self, goreli: &str) -> Result<PathBuf, ErrorReport> {
        let kacis_hatasi = || {
            ErrorReport::new(
                "Eklenti sanal dosya sistemi dışına çıkamaz",
                format!("'{goreli}' yolu eklentinin sandbox kökünün dışına işaret ediyor"),
                "Eklenti yalnızca kendi klasörü içindeki göreli yolları kullanabilir",
            )
        };

        let yol = Path::new(goreli);
        let mut guvenli = self.kok.clone();
        for parca in yol.components() {
            match parca {
                // Kök içinde kalan normal bir bileşen.
                Component::Normal(s) => guvenli.push(s),
                // "." zararsız, atla.
                Component::CurDir => {}
                // ".." / mutlak / sürücü ön eki → kaçış denemesi, reddet.
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(kacis_hatasi());
                }
            }
        }
        Ok(guvenli)
    }

    /// Sandbox içindeki bir dosyayı okur (önce kök-içi çözümleme, sonra `fs::read`).
    pub fn oku(&self, goreli: &str) -> Result<Vec<u8>, ErrorReport> {
        let yol = self.cozumle(goreli)?;
        std::fs::read(&yol).map_err(|e| {
            ErrorReport::new(
                "Dosya okunamadı",
                format!("eklentinin istediği '{goreli}' dosyası açılamadı"),
                "Dosyanın eklenti klasöründe var olduğundan emin olun",
            )
            .with_teknik_detay(e.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vfs() -> SanalDosyaSistemi {
        // Gerçek dosya gerektirmeyen saf çözümleme testleri için sabit bir kök.
        SanalDosyaSistemi::yeni(PathBuf::from("/sandbox/kok"))
    }

    #[test]
    fn normal_yol_kok_icinde_cozulur() {
        let p = vfs().cozumle("veri/ornek.txt").unwrap();
        assert!(p.ends_with("ornek.txt"));
        assert!(p.starts_with("/sandbox/kok"));
    }

    #[test]
    fn ust_dizin_kacisi_reddedilir() {
        assert!(vfs().cozumle("../disari.txt").is_err());
        assert!(vfs().cozumle("veri/../../disari.txt").is_err());
    }

    #[test]
    fn mutlak_yol_reddedilir() {
        assert!(vfs().cozumle("/etc/passwd").is_err());
    }

    #[test]
    fn windows_surucu_onki_reddedilir() {
        // Prefix bileşeni yalnızca Windows'ta üretilir; orada sürücü ön ekini reddetmeli.
        #[cfg(windows)]
        assert!(vfs().cozumle(r"C:\Windows\system32").is_err());
    }

    #[test]
    fn nokta_zararsiz() {
        let p = vfs().cozumle("./veri/./ornek.txt").unwrap();
        assert!(p.ends_with("ornek.txt"));
    }

    #[test]
    fn gercek_dosya_okunur() {
        // Bu crate'in örnek veri dosyasını gerçek VFS köküyle oku.
        let kok = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ornek");
        let v = SanalDosyaSistemi::yeni(kok);
        let icerik = v.oku("veri/ornek.txt").unwrap();
        assert!(!icerik.is_empty());
    }

    #[test]
    fn kacis_okuma_da_engellenir() {
        let kok = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ornek");
        let v = SanalDosyaSistemi::yeni(kok);
        assert!(v.oku("../Cargo.toml").is_err());
    }
}
