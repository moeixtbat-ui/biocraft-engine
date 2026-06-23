//! ÇE-01 — **Provenance (köken) kaydı**: yüklenen her veri *nereden*, *ne zaman*, *hangi
//! formatla*, *hangi sürümle* geldi + içerik **BLAKE3** özeti (MK-34, İP-10).
//!
//! Bu kayıt **eklenti-yereldir** (MK-17: eklenti yalnızca `biocraft-sdk`'ya bağlı; motorun
//! `biocraft-data::privacy::provenance::VeriKokeni` tipine doğrudan erişemez).  Şekli bilinçli
//! olarak motorun `VeriKokeni`'ne **uyumludur** (kaynak/sürüm/tarih/BLAKE3 + lisans/atıf);
//! eklenti veriyi host'a teslim ederken bu kayıt ileride bir **SDK veri kontratı** üzerinden
//! motorun köken günlüğüne (`koken.jsonl`) yazılır.  Bugün: kayıt üretimi + serileştirme.

use std::path::Path;

use serde::{Deserialize, Serialize};

use biocraft_sdk::biocraft_types::{ErrorReport, Timestamp};

use super::detect::VeriFormati;
use super::integrity;

/// Bilimsel bir veri setinin **lisans + atıf** yükümlülüğü (akademik kullanım / yöntem bölümü).
/// Referans genom, dbSNP, ClinVar gibi setlerde zorunlu; kullanıcı dosyasında genelde `None`.
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

/// Tek bir verinin köken kaydı (per-dosya).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenans {
    /// Mantıksal kimlik/yol (örn. dosya adı `ornek.bam`).
    pub veri_kimligi: String,
    /// Kaynak (örn. "Kullanıcı yüklemesi", "NCBI dbSNP", "Ensembl GRCh38").
    pub kaynak: String,
    /// Otomatik tanınan format etiketi (FASTA/FASTQ/BAM/SAM/CRAM…).
    pub format: String,
    /// Kaynak/araç sürümü (bilinmiyorsa boş).
    #[serde(default)]
    pub surum: String,
    /// Edinme/kayıt tarihi (UTC).
    pub tarih: Timestamp,
    /// İçeriğin BLAKE3 özeti (hex).
    pub blake3: String,
    /// Dosya boyutu (bayt).
    pub boyut_bayt: u64,
    /// Bilimsel set ise lisans/atıf yükümlülüğü (yoksa `None`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lisans_atif: Option<LisansAtif>,
}

impl Provenans {
    /// Bir dosya için köken kaydı üretir: **streaming BLAKE3** (tüm dosya RAM'e ALINMAZ — MK-09)
    /// + boyut + şimdi (UTC).  `kaynak` çağıran tarafından verilir (kullanıcı yüklemesi / URL / DB).
    pub fn olustur(
        yol: &Path,
        kaynak: impl Into<String>,
        format: VeriFormati,
    ) -> Result<Self, ErrorReport> {
        let blake3 = integrity::blake3_dosya(yol)?;
        let boyut_bayt = std::fs::metadata(yol)
            .map_err(|e| {
                ErrorReport::new(
                    "Dosya bilgisi okunamadı",
                    format!("'{}' dosyasının boyutu alınamadı", yol.display()),
                    "Dosyanın hâlâ var olduğundan ve okunabilir olduğundan emin olun",
                )
                .with_teknik_detay(e.to_string())
            })?
            .len();
        let veri_kimligi = yol
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| yol.display().to_string());

        Ok(Self {
            veri_kimligi,
            kaynak: kaynak.into(),
            format: format.etiket().to_string(),
            surum: String::new(),
            tarih: chrono::Utc::now(),
            blake3,
            boyut_bayt,
            lisans_atif: None,
        })
    }

    /// Kaynak/araç sürümünü ekler (akıcı).
    pub fn with_surum(mut self, surum: impl Into<String>) -> Self {
        self.surum = surum.into();
        self
    }

    /// Bilimsel set lisans/atıf yükümlülüğünü ekler (ÇE-09 ile tutarlı).
    pub fn with_lisans_atif(mut self, la: LisansAtif) -> Self {
        self.lisans_atif = Some(la);
        self
    }

    /// Köken kaydını JSON'a serileştirir (köken günlüğüne / teşhise yazılır).
    pub fn to_json(&self) -> String {
        // serde_json::to_string yalnızca serileştirilemeyen tipte hata verir; bu yapı her zaman
        // serileştirilebilir → güvenli varsayılan.
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn gecici(ad: &str, icerik: &[u8]) -> std::path::PathBuf {
        let mut yol = std::env::temp_dir();
        yol.push(format!("biocraft_prov_{}_{ad}", std::process::id()));
        let mut f = std::fs::File::create(&yol).unwrap();
        f.write_all(icerik).unwrap();
        yol
    }

    #[test]
    fn olustur_blake3_boyut_ve_kimlik_doldurur() {
        let yol = gecici("a.fasta", b">sq0\nACGT\n");
        let p = Provenans::olustur(&yol, "Kullanıcı yüklemesi", VeriFormati::Fasta).unwrap();
        assert_eq!(p.veri_kimligi, yol.file_name().unwrap().to_string_lossy());
        assert_eq!(p.kaynak, "Kullanıcı yüklemesi");
        assert_eq!(p.format, "FASTA");
        assert_eq!(p.boyut_bayt, 10); // ">sq0\nACGT\n" = 10 bayt

        assert_eq!(p.blake3.len(), 64); // BLAKE3 = 32 bayt = 64 hex
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn json_serilestirme_alanlari_icerir() {
        let yol = gecici("b.fastq", b"@r1\nACGT\n+\nIIII\n");
        let p = Provenans::olustur(&yol, "Test", VeriFormati::Fastq)
            .unwrap()
            .with_surum("v1");
        let js = p.to_json();
        assert!(js.contains("\"blake3\""));
        assert!(js.contains("\"format\":\"FASTQ\""));
        assert!(js.contains("\"surum\":\"v1\""));
        let _ = std::fs::remove_file(&yol);
    }
}
