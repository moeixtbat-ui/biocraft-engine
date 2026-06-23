//! ÇE-01 — **Format otomatik tanıma**.
//!
//! Önce dosya **uzantısı** (gerekirse `.gz` soyularak), sonra doğrulama için **sihirli baytlar**
//! (magic) okunur.  Yalnızca ilk birkaç bayt okunur → tüm dosya açılmaz (MK-09).
//!
//! Bugün (Gün 34) tanınan: FASTA/FASTQ + SAM/BAM/CRAM.  VCF/BCF/BED/GFF/BigWig/2bit/PDB
//! **yarın** (Gün 35) eklenir → o ana dek bunlar `Bilinmeyen` döner (net hata).

use std::fs::File;
use std::io::Read;
use std::path::Path;

use biocraft_sdk::biocraft_types::ErrorReport;

/// Tanınan biyoinformatik veri formatları (bugünkü kapsam).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VeriFormati {
    /// FASTA dizilim (referans/çoklu dizi).
    Fasta,
    /// FASTQ dizilim + kalite skorları.
    Fastq,
    /// SAM düz-metin hizalama.
    Sam,
    /// BAM ikili (BGZF) hizalama.
    Bam,
    /// CRAM referans-sıkıştırmalı hizalama.
    Cram,
}

impl VeriFormati {
    /// Kullanıcıya/provenance'a yazılan kısa etiket.
    pub fn etiket(&self) -> &'static str {
        match self {
            VeriFormati::Fasta => "FASTA",
            VeriFormati::Fastq => "FASTQ",
            VeriFormati::Sam => "SAM",
            VeriFormati::Bam => "BAM",
            VeriFormati::Cram => "CRAM",
        }
    }

    /// Bir hizalama formatı mı (BAM/SAM/CRAM)?
    pub fn hizalama_mi(&self) -> bool {
        matches!(
            self,
            VeriFormati::Sam | VeriFormati::Bam | VeriFormati::Cram
        )
    }

    /// İçeriği BGZF blok-farkında okunan bir format mı (BAM; CRAM kendi konteynerini kullanır)?
    pub fn bgzf_mi(&self) -> bool {
        matches!(self, VeriFormati::Bam)
    }
}

/// Bir dosyanın formatını uzantı + sihirli baytlardan belirler.
pub fn formati_belirle(yol: &Path) -> Result<VeriFormati, ErrorReport> {
    // 1) Uzantı (varsa `.gz` soyulur: `ornek.fastq.gz` → `fastq`).
    if let Some(f) = uzantidan(yol) {
        return Ok(f);
    }
    // 2) Sihirli baytlar (yalnızca ilk 4 bayt) — uzantı yoksa/yardımcı olmadıysa.
    if let Some(f) = magicten(yol)? {
        return Ok(f);
    }
    Err(ErrorReport::new(
        "Dosya formatı tanınamadı",
        format!(
            "'{}' bilinen bir biyoinformatik formatına (FASTA/FASTQ/SAM/BAM/CRAM) uymuyor",
            yol.display()
        ),
        "Dosya uzantısını kontrol edin (.fasta/.fastq/.sam/.bam/.cram) veya doğru dosyayı seçin",
    )
    .with_eylem("Başka dosya seç"))
}

/// Uzantıya göre format (gerekirse `.gz` soyarak).  Tanınmazsa `None`.
fn uzantidan(yol: &Path) -> Option<VeriFormati> {
    let ad = yol.file_name()?.to_string_lossy().to_lowercase();
    // `.gz`/`.bgz` son ekini at (sıkıştırılmış metin/varyant formatları).
    let govde = ad
        .strip_suffix(".gz")
        .or_else(|| ad.strip_suffix(".bgz"))
        .unwrap_or(&ad);
    let son = govde.rsplit('.').next().unwrap_or("");
    match son {
        "fasta" | "fa" | "fna" | "ffn" | "faa" => Some(VeriFormati::Fasta),
        "fastq" | "fq" => Some(VeriFormati::Fastq),
        "sam" => Some(VeriFormati::Sam),
        "bam" => Some(VeriFormati::Bam),
        "cram" => Some(VeriFormati::Cram),
        _ => None,
    }
}

/// İlk baytlardan format tahmini (yalnızca CRAM ve düz FASTA güvenle ayırt edilebilir;
/// gzip/BGZF altındaki BAM uzantı olmadan kesinleştirilemez → `None`).
fn magicten(yol: &Path) -> Result<Option<VeriFormati>, ErrorReport> {
    let mut bas = [0u8; 4];
    let mut f = File::open(yol).map_err(|e| {
        ErrorReport::new(
            "Dosya açılamadı",
            format!("'{}' format tanıma için açılamadı", yol.display()),
            "Dosya yolunu ve okuma iznini kontrol edin",
        )
        .with_teknik_detay(e.to_string())
    })?;
    let n = f.read(&mut bas).unwrap_or(0);
    let bas = &bas[..n];
    // CRAM dosyaları "CRAM" sihirli baytıyla başlar (sıkıştırılmamış, doğrudan okunur).
    if bas.starts_with(b"CRAM") {
        return Ok(Some(VeriFormati::Cram));
    }
    // FASTA kayıtları '>' ile başlar (boşluk/satır sonu atlanmadan ilk anlamlı bayt).
    if bas.first() == Some(&b'>') {
        return Ok(Some(VeriFormati::Fasta));
    }
    // Not: '@' hem FASTQ hem SAM başlığında olabilir → tek başına ayırt edilemez (uzantı gerekir).
    //      gzip/BGZF (1f 8b) altındaki içerik (BAM/vcf.gz/…) magic ile kesinleştirilemez.
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    fn yaz(ad: &str, icerik: &[u8]) -> PathBuf {
        let mut yol = std::env::temp_dir();
        yol.push(format!("biocraft_detect_{}_{ad}", std::process::id()));
        File::create(&yol).unwrap().write_all(icerik).unwrap();
        yol
    }

    #[test]
    fn uzantidan_tanima() {
        assert_eq!(uzantidan(Path::new("a.fasta")), Some(VeriFormati::Fasta));
        assert_eq!(uzantidan(Path::new("a.fa")), Some(VeriFormati::Fasta));
        assert_eq!(uzantidan(Path::new("a.fastq.gz")), Some(VeriFormati::Fastq));
        assert_eq!(uzantidan(Path::new("a.BAM")), Some(VeriFormati::Bam));
        assert_eq!(uzantidan(Path::new("a.cram")), Some(VeriFormati::Cram));
        assert_eq!(uzantidan(Path::new("a.txt")), None);
    }

    #[test]
    fn magic_cram_ve_fasta() {
        let cram = yaz("x.dat", b"CRAM\x03\x00");
        assert_eq!(formati_belirle(&cram).unwrap(), VeriFormati::Cram);
        let fasta = yaz("y.dat", b">sq0\nACGT\n");
        assert_eq!(formati_belirle(&fasta).unwrap(), VeriFormati::Fasta);
        let _ = std::fs::remove_file(&cram);
        let _ = std::fs::remove_file(&fasta);
    }

    #[test]
    fn taninmaz_format_net_hata() {
        let yol = yaz("z.dat", b"rastgele veri\n");
        let hata = formati_belirle(&yol).unwrap_err();
        assert_eq!(hata.ne_oldu, "Dosya formatı tanınamadı");
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn etiket_ve_siniflar() {
        assert_eq!(VeriFormati::Bam.etiket(), "BAM");
        assert!(VeriFormati::Bam.hizalama_mi());
        assert!(VeriFormati::Bam.bgzf_mi());
        assert!(!VeriFormati::Fasta.hizalama_mi());
        assert!(VeriFormati::Cram.hizalama_mi());
        assert!(!VeriFormati::Cram.bgzf_mi());
    }
}
