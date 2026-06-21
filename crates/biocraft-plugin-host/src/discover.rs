//! Eklenti keşfi — klasör tarama + manifest okuma (İP-07).
//!
//! Bir eklenti klasörü, kökünde [`MANIFEST_DOSYA`] (`biocraft.toml`) bulundurur.
//! [`kesfet`] verilen dizinin alt klasörlerini tarar; manifest'i olanları okur/doğrular.
//! Manifest'i bozuk olan klasör **sessizce atlanmaz** — sonuç listesinde bir `Err` olarak
//! döner (kullanıcı neyin neden yüklenemediğini görür, TDA m.1).

use crate::manifest::Manifest;
use biocraft_types::ErrorReport;
use std::path::{Path, PathBuf};

/// Eklenti kökünde aranan manifest dosyasının adı.
pub const MANIFEST_DOSYA: &str = "biocraft.toml";

/// Keşfedilmiş (manifest'i okunup doğrulanmış) bir eklenti.
#[derive(Debug, Clone)]
pub struct KesfedilenEklenti {
    /// Doğrulanmış manifest.
    pub manifest: Manifest,
    /// Eklentinin kök dizini (giriş dosyası + VFS kökü buradan çözülür).
    pub kok_dizin: PathBuf,
}

/// Tek bir eklenti klasörünün manifest'ini okur ve doğrular.
pub fn manifest_oku(kok_dizin: &Path) -> Result<KesfedilenEklenti, ErrorReport> {
    let mp = kok_dizin.join(MANIFEST_DOSYA);
    let metin = std::fs::read_to_string(&mp).map_err(|e| {
        ErrorReport::new(
            "Eklenti manifest'i bulunamadı",
            format!("'{}' okunamadı", mp.display()),
            "Eklenti klasöründe bir biocraft.toml dosyası olduğundan emin olun",
        )
        .with_teknik_detay(e.to_string())
    })?;
    let manifest = Manifest::ayristir(&metin)?;
    Ok(KesfedilenEklenti {
        manifest,
        kok_dizin: kok_dizin.to_path_buf(),
    })
}

/// Verilen dizinin **alt klasörlerini** tarar; `biocraft.toml` içerenleri okur.
///
/// Her klasör için bir sonuç döner: başarılıysa `Ok(KesfedilenEklenti)`, manifest
/// bozuksa `Err(ErrorReport)`.  Dizin okunamazsa boş liste döner.
pub fn kesfet(dizin: &Path) -> Vec<Result<KesfedilenEklenti, ErrorReport>> {
    let mut sonuc = Vec::new();
    let okuma = match std::fs::read_dir(dizin) {
        Ok(o) => o,
        Err(_) => return sonuc,
    };
    for giris in okuma.flatten() {
        let yol = giris.path();
        if yol.is_dir() && yol.join(MANIFEST_DOSYA).is_file() {
            sonuc.push(manifest_oku(&yol));
        }
    }
    // Belirleyici sıra (dosya sistemi sırası platforma göre değişir) — kimliğe göre sırala.
    sonuc.sort_by(|a, b| {
        let ad = |r: &Result<KesfedilenEklenti, ErrorReport>| {
            r.as_ref()
                .map(|k| k.manifest.kimlik.metni().to_string())
                .unwrap_or_default()
        };
        ad(a).cmp(&ad(b))
    });
    sonuc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ornek_dizin() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ornek")
    }

    #[test]
    fn ornek_eklenti_manifesti_okunur() {
        let k = manifest_oku(&ornek_dizin()).unwrap();
        assert_eq!(k.manifest.kimlik.metni(), "biocraft.ornek.merhaba");
        assert_eq!(k.manifest.giris, "merhaba.wat");
    }

    #[test]
    fn kesfet_alt_klasorde_ornegi_bulur() {
        // crates/biocraft-plugin-host/ altında ornek/ klasörü manifest içerir.
        let crate_dizin = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let bulunan = kesfet(&crate_dizin);
        assert!(
            bulunan
                .iter()
                .filter_map(|r| r.as_ref().ok())
                .any(|k| k.manifest.kimlik.metni() == "biocraft.ornek.merhaba"),
            "ornek eklenti keşfedilmeliydi"
        );
    }

    #[test]
    fn olmayan_dizin_bos_liste() {
        let bos = kesfet(Path::new("___boyle_bir_dizin_yok___"));
        assert!(bos.is_empty());
    }
}
