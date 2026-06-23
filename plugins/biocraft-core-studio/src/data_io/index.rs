//! ÇE-01 — **İndeks durumu + oluşturma**.
//!
//! Rastgele bölge erişimi indeks gerektirir: FASTA→`.fai`, BAM→`.bai`/`.csi`, CRAM→`.crai`.
//! İndeks yoksa kullanıcıya **"İndeks oluştur"** sunulur (FASTA/BAM otomatik üretilebilir).

use std::path::{Path, PathBuf};

use noodles::bam;

use biocraft_sdk::biocraft_types::ErrorReport;

use super::detect::VeriFormati;
use super::fasta;

/// Bir veri dosyasının indeks durumu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndeksDurumu {
    /// Dosya formatı.
    pub format: VeriFormati,
    /// Bu format için aranan indeks yolları (öncelik sırasıyla).
    pub beklenen: Vec<PathBuf>,
    /// Bulunan indeks (yoksa `None`).
    pub mevcut: Option<PathBuf>,
}

impl IndeksDurumu {
    /// İndeks mevcut mu?
    pub fn var(&self) -> bool {
        self.mevcut.is_some()
    }

    /// Bu format indeksli rastgele erişimi destekliyor mu (FASTA/BAM/CRAM = evet)?
    pub fn indekslenebilir(&self) -> bool {
        !self.beklenen.is_empty()
    }
}

/// Bir dosyanın indeks durumunu belirler (yalnızca dosya sisteminde varlık kontrolü).
pub fn indeks_durumu(yol: &Path, format: VeriFormati) -> IndeksDurumu {
    let beklenen = match format {
        VeriFormati::Fasta => vec![fasta::fai_yolu(yol)],
        VeriFormati::Bam => vec![ek(yol, ".bai"), ek(yol, ".csi")],
        VeriFormati::Cram => vec![ek(yol, ".crai")],
        // Varyant: tabix (.tbi) veya CSI (.csi) koordinat indeksi (BGZF gerektirir).
        VeriFormati::Vcf => vec![ek(yol, ".tbi"), ek(yol, ".csi")],
        VeriFormati::Bcf => vec![ek(yol, ".csi")],
        // BigWig/BigBed/2bit kendi iç indekslerini taşır (yan dosya yok); diğerleri lineer.
        VeriFormati::Fastq
        | VeriFormati::Sam
        | VeriFormati::Bed
        | VeriFormati::Gff
        | VeriFormati::Gtf
        | VeriFormati::Wig
        | VeriFormati::GenBank
        | VeriFormati::TwoBit
        | VeriFormati::BigWig
        | VeriFormati::BigBed
        | VeriFormati::Pdb
        | VeriFormati::MmCif => vec![],
    };
    let mevcut = beklenen.iter().find(|p| p.exists()).cloned();
    IndeksDurumu {
        format,
        beklenen,
        mevcut,
    }
}

/// Bir dosya için indeks **oluşturur** (FASTA→.fai, BAM→.bai).  Üretilen yolu döndürür.
/// CRAM/SAM otomatik üretilmez → yönlendirici hata.
pub fn indeks_olustur(yol: &Path, format: VeriFormati) -> Result<PathBuf, ErrorReport> {
    match format {
        VeriFormati::Fasta => fasta::fai_olustur(yol),
        VeriFormati::Bam => {
            // BAM koordinat-sıralı olmalı; değilse noodles net hata döner.
            let index = bam::fs::index(yol).map_err(|e| {
                ErrorReport::new(
                    "BAM indekslenemedi",
                    format!(
                        "'{}' indekslenemedi (koordinata göre sıralı olmayabilir)",
                        yol.display()
                    ),
                    "Dosyayı koordinata göre sıralayıp (samtools sort) yeniden deneyin",
                )
                .with_teknik_detay(e.to_string())
            })?;
            let bai = ek(yol, ".bai");
            bam::bai::fs::write(&bai, &index).map_err(|e| {
                ErrorReport::new(
                    "İndeks yazılamadı",
                    format!("'{}' diske yazılamadı", bai.display()),
                    "Klasör yazma iznini ve boş disk alanını kontrol edin",
                )
                .with_teknik_detay(e.to_string())
            })?;
            Ok(bai)
        }
        VeriFormati::Cram => Err(ErrorReport::new(
            "CRAM otomatik indekslenemiyor",
            "CRAM (.crai) indeksi bu sürümde otomatik üretilmiyor",
            "Referansla birlikte 'samtools index' ile .crai oluşturun",
        )),
        VeriFormati::Vcf | VeriFormati::Bcf => Err(ErrorReport::new(
            "Varyant indeksi otomatik üretilmiyor",
            format!(
                "{} için tabix/CSI indeksi bu sürümde otomatik üretilmiyor",
                format.etiket()
            ),
            "Dosyayı BGZF ile sıkıştırıp (bgzip) 'tabix' ile indeksleyin; indekssiz de açılır (linear)",
        )),
        VeriFormati::Sam
        | VeriFormati::Fastq
        | VeriFormati::Bed
        | VeriFormati::Gff
        | VeriFormati::Gtf
        | VeriFormati::Wig
        | VeriFormati::GenBank
        | VeriFormati::TwoBit
        | VeriFormati::BigWig
        | VeriFormati::BigBed
        | VeriFormati::Pdb
        | VeriFormati::MmCif => Err(ErrorReport::new(
            "Bu format yan-indeks ile indekslenmez",
            format!("{} rastgele erişim için ayrı bir yan-indeks kullanmaz", format.etiket()),
            "Bu format akışlı/lineer veya iç-indeksli okunur; ayrı indeks gerekmez",
        )),
    }
}

/// `<yol><ek>` yolu üretir (örn. `<dosya>.bai`).
fn ek(yol: &Path, ek: &str) -> PathBuf {
    let mut s = yol.as_os_str().to_os_string();
    s.push(ek);
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn yaz(ad: &str, icerik: &[u8]) -> PathBuf {
        let mut yol = std::env::temp_dir();
        yol.push(format!("biocraft_idx_{}_{ad}", std::process::id()));
        std::fs::File::create(&yol)
            .unwrap()
            .write_all(icerik)
            .unwrap();
        yol
    }

    #[test]
    fn fasta_indeks_durumu_ve_olustur() {
        let fa = yaz("r.fasta", b">sq0\nACGTACGT\n");
        let _ = std::fs::remove_file(fasta::fai_yolu(&fa));

        let d = indeks_durumu(&fa, VeriFormati::Fasta);
        assert!(d.indekslenebilir());
        assert!(!d.var());

        let yol = indeks_olustur(&fa, VeriFormati::Fasta).unwrap();
        assert!(yol.exists());
        let d2 = indeks_durumu(&fa, VeriFormati::Fasta);
        assert!(d2.var());

        let _ = std::fs::remove_file(fasta::fai_yolu(&fa));
        let _ = std::fs::remove_file(&fa);
    }

    #[test]
    fn sam_indekslenemez_yonlendirir() {
        let s = yaz("a.sam", b"@HD\tVN:1.6\n");
        let d = indeks_durumu(&s, VeriFormati::Sam);
        assert!(!d.indekslenebilir());
        let hata = indeks_olustur(&s, VeriFormati::Sam).unwrap_err();
        assert_eq!(hata.ne_oldu, "Bu format yan-indeks ile indekslenmez");
        let _ = std::fs::remove_file(&s);
    }
}
