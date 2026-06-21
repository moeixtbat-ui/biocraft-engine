//! Köken (provenance) + format meta kaydı (MK-34).
//!
//! İki kayıt türü:
//! - **Meta** (`.biocraft_meta/meta.toml`): format sürümü, oluşturma tarihi, uygulanan göç sayısı.
//!   Bütünlük mührüyle (BCP1) korunur; açılışta beklenen özetle karşılaştırılır.
//! - **Provenance olay günlüğü** (`provenance/olaylar.jsonl`): append-only; her satır bir JSON olay
//!   (proje oluşturuldu, göç uygulandı, …).  Köken/yeniden-üretilebilirlik için iz bırakır.

use std::fs;
use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use biocraft_types::{ErrorReport, Timestamp, Version};

use super::format;
use super::integrity::io_hatasi;

/// `.biocraft_meta/meta.toml` içeriği — format altyapısı meta verisi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meta {
    /// Proje format sürümü (manifestteki ile tutarlı; MK-59).
    pub format_surumu: Version,
    /// Oluşturma tarihi (UTC).
    pub olusturma: Timestamp,
    /// Şu ana kadar uygulanan göç sayısı (manifest `[[goc]]` ile tutarlı).
    pub uygulanan_goc_sayisi: usize,
}

impl Meta {
    /// Yeni bir projenin başlangıç meta verisi.
    pub fn yeni(olusturma: Timestamp, goc_sayisi: usize) -> Self {
        Self {
            format_surumu: format::format_surumu(),
            olusturma,
            uygulanan_goc_sayisi: goc_sayisi,
        }
    }

    /// Meta'yı TOML metnine serileştirir.
    pub fn toml_metni(&self) -> Result<String, ErrorReport> {
        toml::to_string_pretty(self).map_err(|e| {
            ErrorReport::new(
                "Proje meta verisi yazılamadı",
                "Meta TOML biçimine dönüştürülürken bir sorun oluştu.",
                "Bu bir iç hatadır; lütfen hata kimliğiyle bildirin.",
            )
            .with_teknik_detay(format!("toml ser: {e}"))
        })
    }

    /// TOML metninden meta'yı ayrıştırır.
    pub fn toml_coz(metin: &str) -> Result<Self, ErrorReport> {
        toml::from_str(metin).map_err(|e| {
            ErrorReport::new(
                "Proje meta verisi okunamadı",
                ".biocraft_meta/meta.toml beklenen biçimde değil.",
                "Dosyayı yedekten geri yükleyin.",
            )
            .with_teknik_detay(format!("toml de: {e}"))
        })
    }
}

// ─── Provenance olay günlüğü ──────────────────────────────────────────────────

/// Köken olayının türü (kararlı dizgeler; ileride genişler).
pub mod olay_turu {
    /// Proje ilk kez oluşturuldu.
    pub const PROJE_OLUSTURULDU: &str = "proje_olusturuldu";
    /// Bir format göçü uygulandı (İP-19).
    pub const GOC_UYGULANDI: &str = "goc_uygulandi";
}

/// Provenance günlüğüne yazılan tek bir olay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenansOlay {
    /// Olay zamanı (UTC).
    pub zaman: Timestamp,
    /// Olay türü ([`olay_turu`] sabitleri).
    pub tur: String,
    /// İnsan-okunur açıklama.
    pub aciklama: String,
    /// Olayı kaydeden BioCraft sürümü.
    pub biocraft_surumu: Version,
}

impl ProvenansOlay {
    /// Yeni bir olay (zaman = şimdi).
    pub fn yeni(tur: &str, aciklama: impl Into<String>, biocraft_surumu: Version) -> Self {
        Self {
            zaman: Utc::now(),
            tur: tur.to_string(),
            aciklama: aciklama.into(),
            biocraft_surumu,
        }
    }
}

/// Provenance günlüğüne bir olay **ekler** (append-only JSONL; satır = bir JSON olay).
pub fn olay_ekle(kok: &Path, olay: &ProvenansOlay) -> Result<(), ErrorReport> {
    use std::io::Write;
    let yol = format::provenans_yolu(kok);
    if let Some(ana) = yol.parent() {
        fs::create_dir_all(ana)
            .map_err(|e| io_hatasi("Provenance klasörü oluşturulamadı", ana, &e))?;
    }
    let satir = serde_json::to_string(olay).map_err(|e| {
        ErrorReport::new(
            "Köken olayı yazılamadı",
            "Olay JSON'a dönüştürülürken sorun oluştu.",
            "Bu bir iç hatadır; lütfen bildirin.",
        )
        .with_teknik_detay(format!("json ser: {e}"))
    })?;
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&yol)
        .map_err(|e| io_hatasi("Provenance günlüğü açılamadı", &yol, &e))?;
    writeln!(f, "{satir}").map_err(|e| io_hatasi("Provenance günlüğüne yazılamadı", &yol, &e))?;
    Ok(())
}

/// Provenance günlüğündeki tüm olayları okur (bozuk satırları atlar — günlük en iyi çabadır).
pub fn olaylari_oku(kok: &Path) -> Result<Vec<ProvenansOlay>, ErrorReport> {
    let yol = format::provenans_yolu(kok);
    let metin = match fs::read_to_string(&yol) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_hatasi("Provenance günlüğü okunamadı", &yol, &e)),
    };
    let mut olaylar = Vec::new();
    for satir in metin.lines() {
        if satir.trim().is_empty() {
            continue;
        }
        if let Ok(o) = serde_json::from_str::<ProvenansOlay>(satir) {
            olaylar.push(o);
        }
    }
    Ok(olaylar)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_gidis_donus() {
        let m = Meta::yeni(Utc::now(), 1);
        let metin = m.toml_metni().unwrap();
        let geri = Meta::toml_coz(&metin).unwrap();
        assert_eq!(m, geri);
    }

    #[test]
    fn provenance_ekle_oku() {
        let gecici = std::env::temp_dir().join(format!("bc_prov_{}", std::process::id()));
        let _ = fs::remove_dir_all(&gecici);
        format::iskele_olustur(&gecici).unwrap();
        let v = Version::new(0, 1, 0);
        olay_ekle(
            &gecici,
            &ProvenansOlay::yeni(olay_turu::PROJE_OLUSTURULDU, "ilk", v.clone()),
        )
        .unwrap();
        olay_ekle(
            &gecici,
            &ProvenansOlay::yeni(olay_turu::GOC_UYGULANDI, "göç", v),
        )
        .unwrap();
        let okunan = olaylari_oku(&gecici).unwrap();
        assert_eq!(okunan.len(), 2);
        assert_eq!(okunan[0].tur, olay_turu::PROJE_OLUSTURULDU);
        let _ = fs::remove_dir_all(&gecici);
    }
}
