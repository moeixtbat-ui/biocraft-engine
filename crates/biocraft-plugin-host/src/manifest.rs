//! Eklenti manifest'i (`biocraft.toml`) — şema, ayrıştırma, doğrulama, uyumluluk.
//!
//! İP-07: Keşif manifest-tabanlıdır.  Klasördeki `biocraft.toml` okunur; kimlik,
//! katman, giriş dosyası, **ABI/SemVer** ve **istenen yetenekler** doğrulanır.  Çekirdek
//! sürüm uyumu (min/max) denetlenir; uyumsuzsa yükleme **engellenir + neden** (MK-13/MK-14).
//!
//! Tüm kullanıcı-görünür hatalar standart [`ErrorReport`] şemasına uyar (İP-16).

use biocraft_sdk::{yetenek_ayristir, EklentiKatmani};
use biocraft_types::{Capability, ErrorReport, Version};
use serde::Deserialize;

/// Eklenti kimliği — `biocraft.<yayinci>.<eklenti>` biçiminde (3 bölüm, ilki `biocraft`).
///
/// Örnek: `biocraft.ornek.merhaba`.  Bölümler küçük harf/rakam/`_`/`-` içerebilir.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EklentiKimligi(String);

impl EklentiKimligi {
    /// Bir dizgeyi kimlik olarak doğrular.  Biçim hatalıysa açıklayıcı `ErrorReport` döner.
    pub fn dogrula(s: &str) -> Result<Self, ErrorReport> {
        let bolumler: Vec<&str> = s.split('.').collect();
        let bicim_hatasi = || {
            ErrorReport::new(
                "Eklenti kimliği geçersiz",
                format!("'{s}' beklenen 'biocraft.<yayinci>.<eklenti>' biçiminde değil"),
                "Manifest'teki [eklenti].kimlik alanını üç bölümlü yapın (örn. biocraft.firmam.aracim)",
            )
        };
        if bolumler.len() != 3 {
            return Err(bicim_hatasi());
        }
        if bolumler[0] != "biocraft" {
            return Err(ErrorReport::new(
                "Eklenti kimliği geçersiz",
                format!(
                    "kimlik 'biocraft' ile başlamalı, '{}' ile başlıyor",
                    bolumler[0]
                ),
                "Manifest'teki kimliği 'biocraft.' ön ekiyle başlatın",
            ));
        }
        let gecerli_bolum = |b: &str| {
            !b.is_empty()
                && b.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        };
        if !bolumler[1..].iter().all(|b| gecerli_bolum(b)) {
            return Err(bicim_hatasi());
        }
        Ok(Self(s.to_string()))
    }

    /// Kimliğin tam metni.
    pub fn metni(&self) -> &str {
        &self.0
    }

    /// Yayıncı bölümü (ikinci segment).
    pub fn yayinci(&self) -> &str {
        self.0.split('.').nth(1).unwrap_or("")
    }

    /// Eklenti bölümü (üçüncü segment).
    pub fn eklenti(&self) -> &str {
        self.0.split('.').nth(2).unwrap_or("")
    }
}

// ─── TOML ham şeması (serde) ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct HamManifest {
    eklenti: HamEklenti,
    uyumluluk: HamUyumluluk,
    #[serde(default)]
    yetkiler: HamYetkiler,
}

#[derive(Debug, Deserialize)]
struct HamEklenti {
    kimlik: String,
    ad: String,
    surum: String,
    katman: String,
    giris: String,
}

#[derive(Debug, Deserialize)]
struct HamUyumluluk {
    abi: String,
    cekirdek_min: String,
    #[serde(default)]
    cekirdek_max: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct HamYetkiler {
    #[serde(default)]
    istenen: Vec<String>,
}

// ─── Doğrulanmış manifest ─────────────────────────────────────────────────────

/// Doğrulanmış, tip-güvenli eklenti manifest'i.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Eklenti kimliği (`biocraft.<yayinci>.<eklenti>`).
    pub kimlik: EklentiKimligi,
    /// Kullanıcıya görünen ad.
    pub ad: String,
    /// Eklenti sürümü (SemVer).
    pub surum: Version,
    /// Çalıştırma katmanı (MK-12).
    pub katman: EklentiKatmani,
    /// Giriş dosyasının manifest'e **göreli** yolu (örn. `merhaba.wat`).
    pub giris: String,
    /// Hedeflenen ABI sürümü (MK-14, SemVer).
    pub abi: Version,
    /// Uyumlu en düşük çekirdek sürümü.
    pub cekirdek_min: Version,
    /// Uyumlu en yüksek çekirdek sürümü (opsiyonel; yoksa üst sınır yok).
    pub cekirdek_max: Option<Version>,
    /// Eklentinin **istediği** yetenekler (kurulumda kullanıcı onaylar — MK-13).
    pub istenen_yetkiler: Vec<Capability>,
}

impl Manifest {
    /// `biocraft.toml` metnini ayrıştırıp doğrular.
    pub fn ayristir(toml_metni: &str) -> Result<Self, ErrorReport> {
        let ham: HamManifest = toml::from_str(toml_metni).map_err(|e| {
            ErrorReport::new(
                "Eklenti manifest'i okunamadı",
                "biocraft.toml dosyası bozuk veya eksik alan içeriyor",
                "Manifest'i örnek şemaya göre düzeltin",
            )
            .with_teknik_detay(e.to_string())
        })?;

        let kimlik = EklentiKimligi::dogrula(&ham.eklenti.kimlik)?;

        let surum = surum_ayristir(&ham.eklenti.surum, "[eklenti].surum")?;
        let abi = surum_ayristir(&ham.uyumluluk.abi, "[uyumluluk].abi")?;
        let cekirdek_min = surum_ayristir(&ham.uyumluluk.cekirdek_min, "[uyumluluk].cekirdek_min")?;
        let cekirdek_max = match &ham.uyumluluk.cekirdek_max {
            Some(s) => Some(surum_ayristir(s, "[uyumluluk].cekirdek_max")?),
            None => None,
        };

        let katman = EklentiKatmani::metinden(&ham.eklenti.katman).ok_or_else(|| {
            ErrorReport::new(
                "Eklenti katmanı tanınmadı",
                format!("'{}' geçerli bir katman değil", ham.eklenti.katman),
                "Katmanı şunlardan biri yapın: native, wasm, python, external",
            )
        })?;

        // Yetenekleri çöz; tanınmayan yetenek = hata (sessizce yok sayma yok).
        let mut istenen_yetkiler = Vec::new();
        for y in &ham.yetkiler.istenen {
            let cap = yetenek_ayristir(y).ok_or_else(|| {
                ErrorReport::new(
                    "Bilinmeyen yetki istendi",
                    format!("manifest '{y}' yetkisini istiyor ama böyle bir yetki yok"),
                    "İstenen yetkileri şunlarla sınırlayın: net, fs, gpu, ai, db",
                )
            })?;
            if !istenen_yetkiler.contains(&cap) {
                istenen_yetkiler.push(cap);
            }
        }

        Ok(Manifest {
            kimlik,
            ad: ham.eklenti.ad,
            surum,
            katman,
            giris: ham.eklenti.giris,
            abi,
            cekirdek_min,
            cekirdek_max,
            istenen_yetkiler,
        })
    }

    /// Bu eklenti, verilen **çekirdek** ve **ABI** sürümleriyle uyumlu mu?
    ///
    /// Üç kapı (hepsi geçmeli — MK-13/MK-14):
    /// 1. ABI aynı major (kırıcı değilse).
    /// 2. `cekirdek >= cekirdek_min`.
    /// 3. `cekirdek_max` varsa `cekirdek <= cekirdek_max`.
    pub fn uyumluluk_denetle(
        &self,
        cekirdek_surum: &Version,
        cekirdek_abi: &Version,
    ) -> Result<(), ErrorReport> {
        if !cekirdek_abi.uyumlu_mu(&self.abi) {
            return Err(ErrorReport::new(
                "Eklenti bu sürümle uyumsuz",
                format!(
                    "eklenti ABI {} istiyor; çekirdek ABI {} (ana sürüm farklı = kırıcı)",
                    self.abi, cekirdek_abi
                ),
                "Eklentinin bu çekirdek sürümüne uygun bir güncellemesini kurun",
            )
            .with_eylem("Güncelle"));
        }
        if cekirdek_surum < &self.cekirdek_min {
            return Err(ErrorReport::new(
                "Çekirdek sürümü çok eski",
                format!(
                    "eklenti en az çekirdek {} istiyor; mevcut sürüm {}",
                    self.cekirdek_min, cekirdek_surum
                ),
                "BioCraft Engine'i güncelleyin",
            )
            .with_eylem("BioCraft'ı güncelle"));
        }
        if let Some(maks) = &self.cekirdek_max {
            if cekirdek_surum > maks {
                return Err(ErrorReport::new(
                    "Çekirdek sürümü çok yeni",
                    format!(
                        "eklenti en fazla çekirdek {maks} destekliyor; mevcut sürüm {cekirdek_surum}"
                    ),
                    "Eklentinin daha yeni bir sürümünü bekleyin veya geliştiriciye bildirin",
                ));
            }
        }
        Ok(())
    }
}

/// Bir sürüm alanını ayrıştırır; hata mesajına alan adını ekler.
fn surum_ayristir(s: &str, alan: &str) -> Result<Version, ErrorReport> {
    s.parse::<Version>().map_err(|e| {
        ErrorReport::new(
            "Manifest'te geçersiz sürüm",
            format!("{alan} alanı ('{s}') ayrıştırılamadı: {e}"),
            "Sürümü 'ana.alt' veya 'ana.alt.yama' biçiminde yazın (örn. 0.1.0)",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GECERLI: &str = r#"
[eklenti]
kimlik = "biocraft.ornek.merhaba"
ad = "Merhaba Eklentisi"
surum = "0.1.0"
katman = "wasm"
giris = "merhaba.wat"

[uyumluluk]
abi = "0.1"
cekirdek_min = "0.1.0"

[yetkiler]
istenen = ["fs"]
"#;

    #[test]
    fn gecerli_manifest_ayristirilir() {
        let m = Manifest::ayristir(GECERLI).unwrap();
        assert_eq!(m.kimlik.metni(), "biocraft.ornek.merhaba");
        assert_eq!(m.kimlik.yayinci(), "ornek");
        assert_eq!(m.kimlik.eklenti(), "merhaba");
        assert_eq!(m.surum, Version::new(0, 1, 0));
        assert_eq!(m.katman, EklentiKatmani::Wasm);
        assert_eq!(m.abi, Version::new(0, 1, 0));
        assert_eq!(m.istenen_yetkiler, vec![Capability::Fs]);
        assert!(m.cekirdek_max.is_none());
    }

    #[test]
    fn kimlik_uc_bolum_olmali() {
        assert!(EklentiKimligi::dogrula("biocraft.ornek").is_err());
        assert!(EklentiKimligi::dogrula("biocraft.a.b.c").is_err());
        assert!(EklentiKimligi::dogrula("ornek.firma.eklenti").is_err()); // 'biocraft' ile başlamıyor
        assert!(EklentiKimligi::dogrula("biocraft.Firma.eklenti").is_err()); // büyük harf
        assert!(EklentiKimligi::dogrula("biocraft.firma.aracim").is_ok());
    }

    #[test]
    fn bilinmeyen_yetki_reddedilir() {
        let kotu = GECERLI.replace(r#"istenen = ["fs"]"#, r#"istenen = ["uzay"]"#);
        let hata = Manifest::ayristir(&kotu).unwrap_err();
        assert_eq!(hata.ne_oldu, "Bilinmeyen yetki istendi");
    }

    #[test]
    fn bilinmeyen_katman_reddedilir() {
        let kotu = GECERLI.replace(r#"katman = "wasm""#, r#"katman = "buhar""#);
        assert!(Manifest::ayristir(&kotu).is_err());
    }

    #[test]
    fn abi_farkli_major_uyumsuz() {
        let mut m = Manifest::ayristir(GECERLI).unwrap();
        m.abi = Version::new(1, 0, 0); // çekirdek ABI 0.1 → farklı major
        let hata = m
            .uyumluluk_denetle(&Version::new(0, 1, 0), &Version::new(0, 1, 0))
            .unwrap_err();
        assert_eq!(hata.ne_oldu, "Eklenti bu sürümle uyumsuz");
    }

    #[test]
    fn cekirdek_cok_eski_reddedilir() {
        let mut m = Manifest::ayristir(GECERLI).unwrap();
        m.cekirdek_min = Version::new(0, 5, 0);
        let hata = m
            .uyumluluk_denetle(&Version::new(0, 1, 0), &Version::new(0, 1, 0))
            .unwrap_err();
        assert_eq!(hata.ne_oldu, "Çekirdek sürümü çok eski");
    }

    #[test]
    fn cekirdek_cok_yeni_reddedilir() {
        let mut m = Manifest::ayristir(GECERLI).unwrap();
        m.cekirdek_max = Some(Version::new(0, 2, 0));
        let hata = m
            .uyumluluk_denetle(&Version::new(0, 9, 0), &Version::new(0, 1, 0))
            .unwrap_err();
        assert_eq!(hata.ne_oldu, "Çekirdek sürümü çok yeni");
    }

    #[test]
    fn uyumlu_manifest_gecer() {
        let m = Manifest::ayristir(GECERLI).unwrap();
        assert!(m
            .uyumluluk_denetle(&Version::new(0, 3, 0), &Version::new(0, 1, 0))
            .is_ok());
    }
}
