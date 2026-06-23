//! ÇE-01 — **Format otomatik tanıma**.
//!
//! Önce dosya **uzantısı** (gerekirse `.gz`/`.bgz` soyularak), sonra doğrulama/yedek için
//! **sihirli baytlar** (magic) okunur.  Yalnızca ilk birkaç bayt / ilk satır okunur → tüm dosya
//! açılmaz (MK-09).
//!
//! Kapsam (Gün 34 + Gün 35):
//! * **Dizilim:** FASTA/FASTQ.
//! * **Hizalama:** SAM/BAM/CRAM.
//! * **Varyant:** VCF/BCF.
//! * **Anotasyon:** BED/GFF3/GTF/Wig + GenBank.
//! * **Referans/sinyal:** 2bit / BigWig / BigBed.
//! * **Yapı:** PDB / mmCIF.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use biocraft_sdk::biocraft_types::ErrorReport;

/// Tanınan biyoinformatik veri formatları.
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
    /// VCF varyant (düz metin veya BGZF).
    Vcf,
    /// BCF ikili (BGZF) varyant.
    Bcf,
    /// BED anotasyon aralıkları.
    Bed,
    /// GFF3 anotasyon (gen/transkript/ekson).
    Gff,
    /// GTF anotasyon.
    Gtf,
    /// Wig (wiggle) sinyal (düz metin).
    Wig,
    /// GenBank düz-metin dizi + özellik (feature).
    GenBank,
    /// UCSC 2bit ikili referans dizi.
    TwoBit,
    /// UCSC BigWig ikili sinyal.
    BigWig,
    /// UCSC BigBed ikili aralık.
    BigBed,
    /// PDB düz-metin 3B yapı.
    Pdb,
    /// mmCIF (PDBx) 3B yapı.
    MmCif,
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
            VeriFormati::Vcf => "VCF",
            VeriFormati::Bcf => "BCF",
            VeriFormati::Bed => "BED",
            VeriFormati::Gff => "GFF3",
            VeriFormati::Gtf => "GTF",
            VeriFormati::Wig => "Wig",
            VeriFormati::GenBank => "GenBank",
            VeriFormati::TwoBit => "2bit",
            VeriFormati::BigWig => "BigWig",
            VeriFormati::BigBed => "BigBed",
            VeriFormati::Pdb => "PDB",
            VeriFormati::MmCif => "mmCIF",
        }
    }

    /// Bir hizalama formatı mı (BAM/SAM/CRAM)?
    pub fn hizalama_mi(&self) -> bool {
        matches!(
            self,
            VeriFormati::Sam | VeriFormati::Bam | VeriFormati::Cram
        )
    }

    /// Bir varyant formatı mı (VCF/BCF)?
    pub fn varyant_mi(&self) -> bool {
        matches!(self, VeriFormati::Vcf | VeriFormati::Bcf)
    }

    /// Bir anotasyon formatı mı (BED/GFF/GTF)?
    pub fn anotasyon_mi(&self) -> bool {
        matches!(self, VeriFormati::Bed | VeriFormati::Gff | VeriFormati::Gtf)
    }

    /// Bir 3B yapı formatı mı (PDB/mmCIF)?
    pub fn yapi_mi(&self) -> bool {
        matches!(self, VeriFormati::Pdb | VeriFormati::MmCif)
    }

    /// İçeriği her zaman BGZF blok-farkında okunan bir ikili format mı (BAM/BCF; CRAM kendi
    /// konteynerini kullanır, VCF düz metin de olabilir)?
    pub fn bgzf_mi(&self) -> bool {
        matches!(self, VeriFormati::Bam | VeriFormati::Bcf)
    }
}

/// Bir dosyanın formatını uzantı + sihirli baytlardan belirler.
pub fn formati_belirle(yol: &Path) -> Result<VeriFormati, ErrorReport> {
    // 1) Uzantı (varsa `.gz`/`.bgz` soyulur: `ornek.vcf.gz` → `vcf`).
    if let Some(f) = uzantidan(yol) {
        return Ok(f);
    }
    // 2) Sihirli baytlar / ilk satır — uzantı yoksa/yardımcı olmadıysa.
    if let Some(f) = magicten(yol)? {
        return Ok(f);
    }
    Err(ErrorReport::new(
        "Dosya formatı tanınamadı",
        format!(
            "'{}' bilinen bir biyoinformatik formatına (FASTA/FASTQ/SAM/BAM/CRAM/VCF/BCF/BED/GFF/GTF/2bit/BigWig/PDB/mmCIF/GenBank) uymuyor",
            yol.display()
        ),
        "Dosya uzantısını kontrol edin veya doğru dosyayı seçin",
    )
    .with_eylem("Başka dosya seç"))
}

/// Uzantıya göre format (gerekirse `.gz`/`.bgz` soyarak).  Tanınmazsa `None`.
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
        "vcf" => Some(VeriFormati::Vcf),
        "bcf" => Some(VeriFormati::Bcf),
        "bed" => Some(VeriFormati::Bed),
        "gff" | "gff3" => Some(VeriFormati::Gff),
        "gtf" | "gff2" => Some(VeriFormati::Gtf),
        "wig" => Some(VeriFormati::Wig),
        "gb" | "gbk" | "genbank" => Some(VeriFormati::GenBank),
        "2bit" => Some(VeriFormati::TwoBit),
        "bw" | "bigwig" => Some(VeriFormati::BigWig),
        "bb" | "bigbed" => Some(VeriFormati::BigBed),
        "pdb" | "ent" => Some(VeriFormati::Pdb),
        "cif" | "mmcif" => Some(VeriFormati::MmCif),
        _ => None,
    }
}

/// İlk baytlardan/satırdan format tahmini.  İkili formatlar (2bit/BigWig/BigBed/CRAM) sihirli
/// 32-bit imzayla, metin formatları (FASTA/VCF/GFF/GenBank/mmCIF/PDB) ayırt edici başlıkla
/// tanınır.  Belirlenemezse `None` (uzantı gerekir).
fn magicten(yol: &Path) -> Result<Option<VeriFormati>, ErrorReport> {
    let mut bas = [0u8; 16];
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

    // ── İkili sihirli 32-bit imzalar (her iki endianlık) ──
    if bas.len() >= 4 {
        let m = &bas[..4];
        // CRAM "CRAM".
        if m == b"CRAM" {
            return Ok(Some(VeriFormati::Cram));
        }
        // 2bit: 0x1A412743.
        if dort_bayt_imza(m, 0x1A41_2743) {
            return Ok(Some(VeriFormati::TwoBit));
        }
        // BigWig: 0x888FFC26.
        if dort_bayt_imza(m, 0x888F_FC26) {
            return Ok(Some(VeriFormati::BigWig));
        }
        // BigBed: 0x8789F2EB.
        if dort_bayt_imza(m, 0x8789_F2EB) {
            return Ok(Some(VeriFormati::BigBed));
        }
    }

    // ── Metin başlıkları ──
    // FASTA kayıtları '>' ile başlar.
    if bas.first() == Some(&b'>') {
        return Ok(Some(VeriFormati::Fasta));
    }
    // VCF: "##fileformat=VCF".
    if bas.starts_with(b"##fileformat=VCF") {
        return Ok(Some(VeriFormati::Vcf));
    }
    // GFF3: "##gff-version 3" / "##gff-version\t3".
    if bas.starts_with(b"##gff-version") {
        return Ok(Some(VeriFormati::Gff));
    }
    // GenBank: "LOCUS" ile başlar.
    if bas.starts_with(b"LOCUS") {
        return Ok(Some(VeriFormati::GenBank));
    }
    // mmCIF: "data_" blok adıyla başlar.
    if bas.starts_with(b"data_") {
        return Ok(Some(VeriFormati::MmCif));
    }
    // PDB: ilk kayıt genelde "HEADER"; bazı parçalar doğrudan "ATOM"/"HETATM" ile başlar.
    if bas.starts_with(b"HEADER") || bas.starts_with(b"ATOM  ") || bas.starts_with(b"HETATM") {
        return Ok(Some(VeriFormati::Pdb));
    }
    // Wig: "track type=wiggle" / "fixedStep" / "variableStep".
    if bas.starts_with(b"track type=wiggle")
        || bas.starts_with(b"fixedStep")
        || bas.starts_with(b"variableStep")
    {
        return Ok(Some(VeriFormati::Wig));
    }
    // Not: '@' hem FASTQ hem SAM başlığında; BED/GTF düz metin (ayırt edici başlık yok);
    //      gzip/BGZF (1f 8b) altındaki içerik (BAM/BCF/vcf.gz/…) → uzantı gerekir.
    Ok(None)
}

/// 4 baytlık dilimi, verilen 32-bit imzaya (little **veya** big endian) eşitse `true`.
fn dort_bayt_imza(m: &[u8], imza: u32) -> bool {
    let le = u32::from_le_bytes([m[0], m[1], m[2], m[3]]);
    let be = u32::from_be_bytes([m[0], m[1], m[2], m[3]]);
    le == imza || be == imza
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
        assert_eq!(uzantidan(Path::new("a.fastq.gz")), Some(VeriFormati::Fastq));
        assert_eq!(uzantidan(Path::new("a.BAM")), Some(VeriFormati::Bam));
        assert_eq!(uzantidan(Path::new("a.cram")), Some(VeriFormati::Cram));
        assert_eq!(uzantidan(Path::new("a.vcf.gz")), Some(VeriFormati::Vcf));
        assert_eq!(uzantidan(Path::new("a.bcf")), Some(VeriFormati::Bcf));
        assert_eq!(uzantidan(Path::new("a.bed")), Some(VeriFormati::Bed));
        assert_eq!(uzantidan(Path::new("a.gff3")), Some(VeriFormati::Gff));
        assert_eq!(uzantidan(Path::new("a.gtf")), Some(VeriFormati::Gtf));
        assert_eq!(uzantidan(Path::new("a.gb")), Some(VeriFormati::GenBank));
        assert_eq!(uzantidan(Path::new("a.2bit")), Some(VeriFormati::TwoBit));
        assert_eq!(uzantidan(Path::new("a.bw")), Some(VeriFormati::BigWig));
        assert_eq!(uzantidan(Path::new("a.pdb")), Some(VeriFormati::Pdb));
        assert_eq!(uzantidan(Path::new("a.cif")), Some(VeriFormati::MmCif));
        assert_eq!(uzantidan(Path::new("a.txt")), None);
    }

    #[test]
    fn magic_ikili_imzalar() {
        // 2bit (little-endian imza diskte 43 27 41 1A).
        let tb = yaz("x.dat", &[0x43, 0x27, 0x41, 0x1A, 0, 0, 0, 0]);
        assert_eq!(formati_belirle(&tb).unwrap(), VeriFormati::TwoBit);
        // BigWig (little: 26 FC 8F 88).
        let bw = yaz("y.dat", &[0x26, 0xFC, 0x8F, 0x88, 0, 0, 0, 0]);
        assert_eq!(formati_belirle(&bw).unwrap(), VeriFormati::BigWig);
        let cram = yaz("c.dat", b"CRAM\x03\x00");
        assert_eq!(formati_belirle(&cram).unwrap(), VeriFormati::Cram);
        for p in [&tb, &bw, &cram] {
            let _ = std::fs::remove_file(p);
        }
    }

    #[test]
    fn magic_metin_basliklari() {
        let cases: &[(&[u8], VeriFormati)] = &[
            (b">sq0\nACGT\n", VeriFormati::Fasta),
            (b"##fileformat=VCFv4.3\n", VeriFormati::Vcf),
            (b"##gff-version 3\n", VeriFormati::Gff),
            (b"LOCUS       SCU49845     5028 bp\n", VeriFormati::GenBank),
            (b"data_1ABC\n", VeriFormati::MmCif),
            (b"HEADER    OXYGEN\n", VeriFormati::Pdb),
        ];
        for (icerik, beklenen) in cases {
            let yol = yaz("m.dat", icerik);
            assert_eq!(formati_belirle(&yol).unwrap(), *beklenen, "{icerik:?}");
            let _ = std::fs::remove_file(&yol);
        }
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
        assert_eq!(VeriFormati::Vcf.etiket(), "VCF");
        assert_eq!(VeriFormati::MmCif.etiket(), "mmCIF");
        assert!(VeriFormati::Bam.hizalama_mi());
        assert!(VeriFormati::Vcf.varyant_mi());
        assert!(VeriFormati::Bcf.varyant_mi());
        assert!(VeriFormati::Gff.anotasyon_mi());
        assert!(VeriFormati::Pdb.yapi_mi());
        assert!(VeriFormati::Bcf.bgzf_mi());
        assert!(!VeriFormati::Cram.bgzf_mi());
    }
}
