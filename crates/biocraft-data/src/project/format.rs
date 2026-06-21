//! Proje klasör yapısı + format sürümü (MK-31, MK-59).
//!
//! Bir BioCraft projesi **bir klasördür**:
//! ```text
//! <proje>/
//!   biocraft.toml          ← manifest (kimlik/ORCID/sınıflandırma/sürüm+göç/harici-veri ref)
//!   data/
//!     inputs/              ← ham girdi verisi
//!     intermediate/        ← ara/türetilmiş veri
//!   flows/                 ← .bcflow node akışları (İP-05)
//!   scripts/               ← kullanıcı script'leri (Python/R)
//!   provenance/            ← köken/olay günlüğü (append-only)
//!   .biocraft_meta/        ← format sürümü + oluşturma tarihi + bütünlük mühürleri
//!     meta.toml
//!     butunluk.bcp
//! ```

use std::path::{Path, PathBuf};

use biocraft_types::{ErrorReport, Version};

use super::integrity::io_hatasi;

/// Manifest dosyasının adı (proje kökünde).
pub const MANIFEST_DOSYA: &str = "biocraft.toml";
/// Meta/bütünlük klasörü (gizli; format altyapısı).
pub const META_DIZIN: &str = ".biocraft_meta";
/// Meta dosyası (`.biocraft_meta` altında): format sürümü + oluşturma tarihi + göç sayısı.
pub const META_DOSYA: &str = "meta.toml";
/// Bütünlük mührü dosyası (`.biocraft_meta` altında): manifest + meta BLAKE3 özetleri (BCP1 zarflı).
pub const BUTUNLUK_DOSYA: &str = "butunluk.bcp";
/// Veri klasörü (`inputs`/`intermediate` alt klasörlerini içerir).
pub const VERI_DIZIN: &str = "data";
/// Node akışları klasörü (.bcflow; İP-05).
pub const FLOWS_DIZIN: &str = "flows";
/// Kullanıcı script'leri klasörü (Python/R).
pub const SCRIPTS_DIZIN: &str = "scripts";
/// Provenance (köken) klasörü.
pub const PROVENANS_DIZIN: &str = "provenance";
/// Provenance olay günlüğü (append-only JSONL; `provenance` altında).
pub const PROVENANS_DOSYA: &str = "olaylar.jsonl";
/// Per-veri köken (kaynak/sürüm/lisans/atıf) günlüğü (append-only JSONL; İP-10).
pub const KOKEN_DOSYA: &str = "koken.jsonl";
/// Dış-gönderim onay defteri (append-only JSONL; her dış gönderim şeffaflık için kaydedilir; İP-10).
pub const ONAY_DOSYA: &str = "onaylar.jsonl";

/// Oluşturulması gereken alt klasörler (sıra sabit; iç içe yollar dahil).
pub const ALT_DIZINLER: &[&str] = &[
    "data",
    "data/inputs",
    "data/intermediate",
    "flows",
    "scripts",
    PROVENANS_DIZIN,
    META_DIZIN,
];

/// Mevcut **proje format sürümü** (MK-59).  İleride şema genişledikçe artar; eski projeler göç
/// geçmişiyle yükseltilir (İP-19).  Baştan sürümlenmesi, sonradan göçü mümkün kılar.
pub fn format_surumu() -> Version {
    Version::new(1, 0, 0)
}

/// Proje kökünün altındaki bir alt yolu (yardımcı).
pub fn alt_yol(kok: &Path, parcalar: &[&str]) -> PathBuf {
    let mut p = kok.to_path_buf();
    for parca in parcalar {
        p.push(parca);
    }
    p
}

/// Manifest dosyasının tam yolu.
pub fn manifest_yolu(kok: &Path) -> PathBuf {
    kok.join(MANIFEST_DOSYA)
}

/// Meta dosyasının tam yolu.
pub fn meta_yolu(kok: &Path) -> PathBuf {
    alt_yol(kok, &[META_DIZIN, META_DOSYA])
}

/// Bütünlük mührü dosyasının tam yolu.
pub fn butunluk_yolu(kok: &Path) -> PathBuf {
    alt_yol(kok, &[META_DIZIN, BUTUNLUK_DOSYA])
}

/// Provenance günlüğünün tam yolu.
pub fn provenans_yolu(kok: &Path) -> PathBuf {
    alt_yol(kok, &[PROVENANS_DIZIN, PROVENANS_DOSYA])
}

/// Per-veri köken günlüğünün tam yolu (İP-10; kaynak/sürüm/lisans/atıf).
pub fn koken_yolu(kok: &Path) -> PathBuf {
    alt_yol(kok, &[PROVENANS_DIZIN, KOKEN_DOSYA])
}

/// Dış-gönderim onay defterinin tam yolu (İP-10; şeffaflık/denetim izi).
pub fn onay_yolu(kok: &Path) -> PathBuf {
    alt_yol(kok, &[PROVENANS_DIZIN, ONAY_DOSYA])
}

/// Proje kökünü ve tüm alt klasörlerini oluşturur (iskele).
///
/// Klasörlerden biri oluşturulamazsa açıklayıcı [`ErrorReport`] döner; çağıran (üst seviye)
/// yarım kalan klasörü **atomik temizlik** ile siler.
pub fn iskele_olustur(kok: &Path) -> Result<(), ErrorReport> {
    std::fs::create_dir_all(kok).map_err(|e| io_hatasi("Proje klasörü oluşturulamadı", kok, &e))?;
    for alt in ALT_DIZINLER {
        let yol = alt_yol(kok, &[alt]);
        std::fs::create_dir_all(&yol)
            .map_err(|e| io_hatasi("Proje alt klasörü oluşturulamadı", &yol, &e))?;
    }
    Ok(())
}

/// Bir klasörün proje gibi görünüp görünmediği (manifest dosyası var mı).
pub fn proje_mi(kok: &Path) -> bool {
    manifest_yolu(kok).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iskele_tum_klasorleri_olusturur() {
        let gecici = std::env::temp_dir().join(format!("bc_iskele_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&gecici);
        iskele_olustur(&gecici).unwrap();
        for alt in ALT_DIZINLER {
            assert!(alt_yol(&gecici, &[alt]).is_dir(), "eksik klasör: {alt}");
        }
        let _ = std::fs::remove_dir_all(&gecici);
    }

    #[test]
    fn format_surumu_bir_nokta_sifir() {
        assert_eq!(format_surumu(), Version::new(1, 0, 0));
    }
}
