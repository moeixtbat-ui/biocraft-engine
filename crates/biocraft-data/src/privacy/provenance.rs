//! **Köken (provenance) kaydı** — her veri için kaynak/sürüm/tarih/BLAKE3 + lisans/atıf (MK-34, İP-10).
//!
//! `project/provenance.rs` proje **olay** günlüğüdür (oluşturuldu/göç edildi…).  Bu modül ise
//! **per-veri** kökenidir: bir veri *nereden*, *ne zaman*, *hangi sürümle*, *hangi lisansla* geldi.
//! Bilimsel veri setleri (referans genom, dbSNP, ClinVar…) için **lisans + atıf** yükümlülüğü de
//! kaydedilir — akademik kullanım ve yöntem bölümü için (`Cekirdek-Eklenti.md` ÇE-09 ile tutarlı).
//!
//! İki append-only günlük (`provenance/` altında):
//! - `koken.jsonl` — [`VeriKokeni`] kayıtları.
//! - `onaylar.jsonl` — [`super::consent::OnayKaydi`] dış-gönderim denetim izi.

use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use biocraft_types::{DataClassification, ErrorReport, Timestamp};

use crate::project::format;
use crate::project::integrity::io_hatasi;

use super::consent::OnayKaydi;

/// Bir bilimsel veri setinin **lisans + atıf** yükümlülüğü (akademik kullanım / yöntem bölümü).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LisansAtif {
    /// Lisans tanımlayıcısı (örn. `CC-BY-4.0`, `Public Domain`).
    pub lisans: String,
    /// Atıf metni (örn. "Sherry ST et al., dbSNP, Nucleic Acids Res. 2001").
    pub atif: String,
    /// Opsiyonel kaynak URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Tek bir verinin köken kaydı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VeriKokeni {
    /// Proje içindeki mantıksal kimlik/yol (örn. `data/inputs/ornek.vcf`).
    pub veri_kimligi: String,
    /// Kaynak (örn. "Kullanıcı yüklemesi", "NCBI dbSNP", "Ensembl GRCh38").
    pub kaynak: String,
    /// Kaynak sürümü (örn. "dbSNP build 156"); bilinmiyorsa boş.
    #[serde(default)]
    pub surum: String,
    /// Edinme/kayıt tarihi (UTC).
    pub tarih: Timestamp,
    /// İçeriğin BLAKE3 özeti (hex).
    pub blake3: String,
    /// Bu verinin sınıflandırması (dosya-başına).
    pub siniflandirma: DataClassification,
    /// Bilimsel set ise lisans/atıf yükümlülüğü (yoksa `None`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lisans_atif: Option<LisansAtif>,
}

impl VeriKokeni {
    /// Yeni bir köken kaydı (tarih = şimdi).
    pub fn yeni(
        veri_kimligi: impl Into<String>,
        kaynak: impl Into<String>,
        blake3: impl Into<String>,
        siniflandirma: DataClassification,
    ) -> Self {
        Self {
            veri_kimligi: veri_kimligi.into(),
            kaynak: kaynak.into(),
            surum: String::new(),
            tarih: chrono::Utc::now(),
            blake3: blake3.into(),
            siniflandirma,
            lisans_atif: None,
        }
    }

    /// Sürüm bilgisi ekler (akıcı kurucu).
    pub fn surum_ile(mut self, surum: impl Into<String>) -> Self {
        self.surum = surum.into();
        self
    }

    /// Lisans/atıf yükümlülüğü ekler (akıcı kurucu).
    pub fn lisans_ile(mut self, lisans_atif: LisansAtif) -> Self {
        self.lisans_atif = Some(lisans_atif);
        self
    }
}

// ─── Köken günlüğü (koken.jsonl) ──────────────────────────────────────────────

/// Bir köken kaydını günlüğe **ekler** (append-only JSONL).  Her veri yüklemesi böyle bir kayıt üretmeli.
pub fn koken_ekle(kok: &Path, koken: &VeriKokeni) -> Result<(), ErrorReport> {
    jsonl_ekle(&format::koken_yolu(kok), koken, "Köken kaydı")
}

/// Köken günlüğündeki tüm kayıtları okur (bozuk satırları atlar — günlük en iyi çabadır).
pub fn kokenleri_oku(kok: &Path) -> Result<Vec<VeriKokeni>, ErrorReport> {
    jsonl_oku(&format::koken_yolu(kok))
}

// ─── Onay defteri (onaylar.jsonl) ─────────────────────────────────────────────

/// Bir dış-gönderim onay kararını **deftere** işler (şeffaflık/denetim izi).
pub fn onay_ekle(kok: &Path, kayit: &OnayKaydi) -> Result<(), ErrorReport> {
    jsonl_ekle(&format::onay_yolu(kok), kayit, "Onay kaydı")
}

/// Onay defterindeki tüm kayıtları okur.
pub fn onaylari_oku(kok: &Path) -> Result<Vec<OnayKaydi>, ErrorReport> {
    jsonl_oku(&format::onay_yolu(kok))
}

// ─── Ortak JSONL yardımcıları ─────────────────────────────────────────────────

/// Bir kaydı JSONL dosyasına ekler (klasörü yoksa oluşturur).
fn jsonl_ekle<T: Serialize>(yol: &Path, kayit: &T, ad: &str) -> Result<(), ErrorReport> {
    if let Some(ana) = yol.parent() {
        fs::create_dir_all(ana).map_err(|e| io_hatasi("Köken klasörü oluşturulamadı", ana, &e))?;
    }
    let satir = serde_json::to_string(kayit).map_err(|e| {
        ErrorReport::new(
            format!("{ad} yazılamadı"),
            "Kayıt JSON'a dönüştürülürken sorun oluştu.",
            "Bu bir iç hatadır; lütfen bildirin.",
        )
        .with_teknik_detay(format!("json ser: {e}"))
    })?;
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(yol)
        .map_err(|e| io_hatasi("Günlük açılamadı", yol, &e))?;
    writeln!(f, "{satir}").map_err(|e| io_hatasi("Günlüğe yazılamadı", yol, &e))?;
    Ok(())
}

/// Bir JSONL dosyasındaki tüm kayıtları okur (yoksa boş; bozuk satırları atlar).
fn jsonl_oku<T: for<'de> Deserialize<'de>>(yol: &Path) -> Result<Vec<T>, ErrorReport> {
    let metin = match fs::read_to_string(yol) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_hatasi("Günlük okunamadı", yol, &e)),
    };
    let mut kayitlar = Vec::new();
    for satir in metin.lines() {
        if satir.trim().is_empty() {
            continue;
        }
        if let Ok(k) = serde_json::from_str::<T>(satir) {
            kayitlar.push(k);
        }
    }
    Ok(kayitlar)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::privacy::classify::DisKanal;
    use crate::privacy::consent::{GonderimOzeti, OnayKarari, OnayTalebi};

    fn gecici_kok(etiket: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "bc_priv_prov_{}_{}_{}",
            etiket,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&p);
        format::iskele_olustur(&p).unwrap();
        p
    }

    #[test]
    fn koken_ekle_oku_lisans_atif_korur() {
        let kok = gecici_kok("koken");
        let k = VeriKokeni::yeni(
            "data/inputs/dbsnp.vcf",
            "NCBI dbSNP",
            "ab".repeat(32),
            DataClassification::Normal,
        )
        .surum_ile("build 156")
        .lisans_ile(LisansAtif {
            lisans: "Public Domain".into(),
            atif: "Sherry ST et al., dbSNP, NAR 2001".into(),
            url: Some("https://www.ncbi.nlm.nih.gov/snp/".into()),
        });
        koken_ekle(&kok, &k).unwrap();

        let okunan = kokenleri_oku(&kok).unwrap();
        assert_eq!(okunan.len(), 1);
        assert_eq!(okunan[0].kaynak, "NCBI dbSNP");
        assert_eq!(okunan[0].surum, "build 156");
        let la = okunan[0].lisans_atif.as_ref().unwrap();
        assert_eq!(la.lisans, "Public Domain");
        assert!(la.atif.contains("Sherry"));

        let _ = fs::remove_dir_all(&kok);
    }

    #[test]
    fn onay_defteri_ekle_oku() {
        let kok = gecici_kok("onay");
        let talep = OnayTalebi::yeni(
            DisKanal::DisApi,
            "NCBI",
            "arama",
            GonderimOzeti {
                oge_sayisi: 1,
                siniflar: vec![DataClassification::Normal],
                boyut_bayt: 100,
                alanlar: vec![],
            },
        );
        onay_ekle(&kok, &OnayKaydi::yeni(&talep, OnayKarari::Onaylandi)).unwrap();
        onay_ekle(&kok, &OnayKaydi::yeni(&talep, OnayKarari::Reddedildi)).unwrap();

        let kayitlar = onaylari_oku(&kok).unwrap();
        assert_eq!(kayitlar.len(), 2);
        assert_eq!(kayitlar[0].karar, OnayKarari::Onaylandi);
        assert_eq!(kayitlar[1].karar, OnayKarari::Reddedildi);

        let _ = fs::remove_dir_all(&kok);
    }

    #[test]
    fn koken_yoksa_bos_doner() {
        let kok = gecici_kok("bos");
        assert!(kokenleri_oku(&kok).unwrap().is_empty());
        let _ = fs::remove_dir_all(&kok);
    }
}
