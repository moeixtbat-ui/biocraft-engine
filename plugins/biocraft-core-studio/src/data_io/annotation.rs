//! ÇE-01 — **BED / GFF3 / GTF** anotasyon okuma.
//!
//! Özellik (feature) aralıkları: gen/transkript/ekson vb.  Hepsi **akışlı** (out-of-core, MK-09)
//! okunur; `chr:start-end` **bölge sorgusu** ile yalnızca örtüşen özellikler toplanır (dosya
//! yeniden açılıp lineer taranır — anotasyon dosyaları görece küçüktür; BGZF+`.tbi` indeksli
//! sürüm büyük dosyada hızlandırma için ileride bağlanır).
//!
//! * **BED:** kromozom + aralık (+ opsiyonel ad).  noodles `bed::Record<3>` + ek alanlar.
//! * **GFF3 / GTF:** kromozom + kaynak + **tür** (gene/transcript/exon) + aralık + şerit +
//!   öznitelikler (ID/gene_id/Name…).  Her ikisi noodles'ta ortak `gff::feature::RecordBuf`
//!   verir → **tek yol**.

use std::fs::File;
use std::io::BufReader;
use std::ops::Bound;
use std::path::{Path, PathBuf};

use noodles::bed;
use noodles::core::Region;
use noodles::gff;
use noodles::gtf;

use biocraft_sdk::biocraft_types::ErrorReport;

use super::budget::BellekButcesi;
use super::detect::{formati_belirle, VeriFormati};

/// Anotasyon dosyasının özeti.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnotasyonBasligi {
    /// Dosya formatı (BED/GFF/GTF).
    pub format: VeriFormati,
}

/// Tek bir anotasyon özelliği (feature).
#[derive(Debug, Clone, PartialEq)]
pub struct AnotasyonKaydi {
    /// Kromozom/kontig adı.
    pub kromozom: String,
    /// 1-tabanlı başlangıç.
    pub baslangic: usize,
    /// 1-tabanlı bitiş (kapsayıcı).
    pub bitis: usize,
    /// Özellik türü (gene/transcript/exon…); BED'de "region".
    pub tur: String,
    /// Ad/kimlik (GFF/GTF: ID/gene_id/Name; BED: 4. sütun) — yoksa `None`.
    pub ad: Option<String>,
    /// Şerit (+ / - / . / ?); BED'de '.'.
    pub serit: char,
    /// Kaynak (GFF/GTF 2. sütun) — BED'de `None`.
    pub kaynak: Option<String>,
}

/// Format-bağımsız anotasyon okuyucu (linear/akışlı).
pub struct AnotasyonOkuyucu {
    format: VeriFormati,
    yol: PathBuf,
}

impl AnotasyonOkuyucu {
    /// Bir anotasyon dosyası açar (format otomatik tanınır; BED/GFF/GTF olmalı).
    pub fn ac(yol: &Path) -> Result<(Self, AnotasyonBasligi), ErrorReport> {
        let format = formati_belirle(yol)?;
        if !format.anotasyon_mi() {
            return Err(ErrorReport::new(
                "Anotasyon dosyası değil",
                format!("'{}' bir BED/GFF/GTF dosyası değil", yol.display()),
                "BED (.bed), GFF3 (.gff/.gff3) veya GTF (.gtf) uzantılı bir dosya seçin",
            ));
        }
        Ok((
            Self {
                format,
                yol: yol.to_path_buf(),
            },
            AnotasyonBasligi { format },
        ))
    }

    /// Tüm özellikleri akışla okur; her biri için `gozlemci(&AnotasyonKaydi)` çağrılır.  Toplam
    /// özellik sayısını döndürür (dosya RAM'e alınmaz — MK-09).
    pub fn akis<F>(&self, mut gozlemci: F) -> Result<usize, ErrorReport>
    where
        F: FnMut(&AnotasyonKaydi),
    {
        let mut sayi = 0;
        self.her_kayit(|k| {
            gozlemci(&k);
            sayi += 1;
            Ok(true)
        })?;
        Ok(sayi)
    }

    /// Bir bölgeyi (`chr:start-end`) sorgular: örtüşen özellikleri toplar (linear; out-of-core).
    /// En fazla `max_kayit`; bütçe aşılırsa reddedilir (İP-08).
    pub fn bolge_sorgu(
        &self,
        bolge: &str,
        butce: &BellekButcesi,
        max_kayit: usize,
    ) -> Result<Vec<AnotasyonKaydi>, ErrorReport> {
        let region: Region = bolge.parse().map_err(|_| gecersiz_bolge(bolge))?;
        let (hedef_ad, r_bas, r_bit) = region_araligi(&region);

        let mut sonuc = Vec::new();
        let mut tahmini: u64 = 0;
        self.her_kayit(|k| {
            if sonuc.len() >= max_kayit {
                return Ok(false); // dur
            }
            if k.kromozom == hedef_ad && k.baslangic <= r_bit && k.bitis >= r_bas {
                tahmini += tahmini_bayt(&k);
                butce.kontrol(tahmini)?;
                sonuc.push(k);
            }
            Ok(true)
        })?;
        Ok(sonuc)
    }

    /// Ortak akış motoru: her kayıt için `f(kayit) -> Result<devam_mı>` çağırır.
    fn her_kayit<F>(&self, mut f: F) -> Result<(), ErrorReport>
    where
        F: FnMut(AnotasyonKaydi) -> Result<bool, ErrorReport>,
    {
        match self.format {
            VeriFormati::Bed => {
                let mut r = bed::io::Reader::<3, _>::new(BufReader::new(
                    File::open(&self.yol).map_err(|e| io_hatasi(&self.yol, "BED okuma", &e))?,
                ));
                let mut kayit = bed::Record::<3>::default();
                loop {
                    let n = r
                        .read_record(&mut kayit)
                        .map_err(|e| ayristirma_hatasi(&self.yol, "BED", &e))?;
                    if n == 0 {
                        break;
                    }
                    let k = bed_kayit(&kayit)?;
                    if !f(k)? {
                        break;
                    }
                }
                Ok(())
            }
            VeriFormati::Gff => {
                let mut r = gff::io::Reader::new(BufReader::new(
                    File::open(&self.yol).map_err(|e| io_hatasi(&self.yol, "GFF okuma", &e))?,
                ));
                for res in r.record_bufs() {
                    let rb = res.map_err(|e| ayristirma_hatasi(&self.yol, "GFF", &e))?;
                    let k = gff_kayit(&rb);
                    if !f(k)? {
                        break;
                    }
                }
                Ok(())
            }
            VeriFormati::Gtf => {
                let mut r = gtf::io::Reader::new(BufReader::new(
                    File::open(&self.yol).map_err(|e| io_hatasi(&self.yol, "GTF okuma", &e))?,
                ));
                for res in r.record_bufs() {
                    let rb = res.map_err(|e| ayristirma_hatasi(&self.yol, "GTF", &e))?;
                    let k = gff_kayit(&rb);
                    if !f(k)? {
                        break;
                    }
                }
                Ok(())
            }
            _ => unreachable!("anotasyon_mi() yalnız Bed/Gff/Gtf'e izin verir"),
        }
    }
}

// ─── Kayıt dönüşümleri ──────────────────────────────────────────────────────────

fn bed_kayit(kayit: &bed::Record<3>) -> Result<AnotasyonKaydi, ErrorReport> {
    let kromozom = kayit.reference_sequence_name().to_string();
    let baslangic = kayit
        .feature_start()
        .map_err(|e| alan_hatasi("BED başlangıç", &e))?
        .get();
    let bitis = match kayit.feature_end() {
        Some(r) => r.map_err(|e| alan_hatasi("BED bitiş", &e))?.get(),
        None => baslangic,
    };
    // 4. sütun (varsa) = ad.
    let ad = kayit
        .other_fields()
        .get(0)
        .map(|b| b.to_string())
        .filter(|s| !s.is_empty());
    Ok(AnotasyonKaydi {
        kromozom,
        baslangic,
        bitis,
        tur: "region".to_string(),
        ad,
        serit: '.',
        kaynak: None,
    })
}

fn gff_kayit(rb: &gff::feature::RecordBuf) -> AnotasyonKaydi {
    let serit = match rb.strand() {
        gff::feature::record::Strand::Forward => '+',
        gff::feature::record::Strand::Reverse => '-',
        gff::feature::record::Strand::Unknown => '?',
        gff::feature::record::Strand::None => '.',
    };
    let ad = oznitelik_bul(
        rb.attributes(),
        &[b"ID", b"gene_id", b"transcript_id", b"Name"],
    );
    AnotasyonKaydi {
        kromozom: rb.reference_sequence_name().to_string(),
        baslangic: rb.start().get(),
        bitis: rb.end().get(),
        tur: rb.ty().to_string(),
        ad,
        serit,
        kaynak: Some(rb.source().to_string()).filter(|s| !s.is_empty() && s != "."),
    }
}

/// Öznitelik haritasından, verilen anahtarlardan ilk bulunanı metin olarak döndürür.
fn oznitelik_bul(
    attrs: &gff::feature::record_buf::Attributes,
    anahtarlar: &[&[u8]],
) -> Option<String> {
    for &a in anahtarlar {
        if let Some(deger) = attrs.get(a) {
            if let Some(s) = deger.as_string() {
                return Some(s.to_string());
            }
        }
    }
    None
}

/// Bir kaydın bellekteki kaba bayt tahmini (bütçe muhasebesi).
fn tahmini_bayt(k: &AnotasyonKaydi) -> u64 {
    (64 + k.kromozom.len()
        + k.tur.len()
        + k.ad.as_ref().map(|s| s.len()).unwrap_or(0)
        + k.kaynak.as_ref().map(|s| s.len()).unwrap_or(0)) as u64
}

/// Bölgeyi (ad, 1-tabanlı başlangıç, bitiş) üçlüsüne çevirir; sınırsız uçlar genişletilir.
fn region_araligi(region: &Region) -> (String, usize, usize) {
    let bas = match region.start() {
        Bound::Included(p) | Bound::Excluded(p) => p.get(),
        Bound::Unbounded => 1,
    };
    let bit = match region.end() {
        Bound::Included(p) | Bound::Excluded(p) => p.get(),
        Bound::Unbounded => usize::MAX,
    };
    (region.name().to_string(), bas, bit)
}

// ─── Hatalar ────────────────────────────────────────────────────────────────────

fn gecersiz_bolge(bolge: &str) -> ErrorReport {
    ErrorReport::new(
        "Geçersiz bölge ifadesi",
        format!("'{bolge}' bir bölgeye çözülemedi (beklenen: ad:başlangıç-bitiş)"),
        "Örn. 'chr1:100-200' veya yalnızca 'chr1' yazın",
    )
}

fn io_hatasi(yol: &Path, baglam: &str, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "Dosya açılamadı",
        format!("'{}' {baglam} için açılamadı", yol.display()),
        "Dosya yolunu ve okuma iznini kontrol edin",
    )
    .with_teknik_detay(e.to_string())
}

fn ayristirma_hatasi(yol: &Path, format: &str, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        format!("{format} dosyası ayrıştırılamadı"),
        format!(
            "'{}' içeriği {format} biçimine uymuyor (bozuk veya kesilmiş olabilir)",
            yol.display()
        ),
        "Dosyanın bütünlüğünü kontrol edin; gerekirse yeniden indirin",
    )
    .with_teknik_detay(e.to_string())
}

fn alan_hatasi(baglam: &str, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "Anotasyon alanı okunamadı",
        format!("{baglam} ayrıştırılamadı (geçersiz koordinat olabilir)"),
        "Dosyanın geçerli olduğundan emin olun",
    )
    .with_teknik_detay(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn yaz(ad: &str, icerik: &[u8]) -> PathBuf {
        let mut yol = std::env::temp_dir();
        yol.push(format!("biocraft_annot_{}_{ad}", std::process::id()));
        File::create(&yol).unwrap().write_all(icerik).unwrap();
        yol
    }

    #[test]
    fn bed_okur_ve_bolge_suzer() {
        // BED 0-tabanlı yarı-açık: chr1 100-200 (ad geneA), chr1 500-600, chr2 10-20.
        let p = yaz(
            "a.bed",
            b"chr1\t100\t200\tgeneA\nchr1\t500\t600\tgeneB\nchr2\t10\t20\tgeneC\n",
        );
        let (okuyucu, basligi) = AnotasyonOkuyucu::ac(&p).unwrap();
        assert_eq!(basligi.format, VeriFormati::Bed);

        // Tümünü akışla say.
        let toplam = okuyucu.akis(|_| {}).unwrap();
        assert_eq!(toplam, 3);

        // chr1:150-250 → ilk kayıt (100-200) örtüşür.
        let kayitlar = okuyucu
            .bolge_sorgu("chr1:150-250", &BellekButcesi::sinirsiz(), 100)
            .unwrap();
        assert_eq!(kayitlar.len(), 1);
        assert_eq!(kayitlar[0].kromozom, "chr1");
        assert_eq!(kayitlar[0].ad.as_deref(), Some("geneA"));
        assert_eq!(kayitlar[0].tur, "region");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn gff_okur_tur_ve_oznitelik() {
        let p = yaz(
            "a.gff3",
            b"##gff-version 3\n\
chr1\tHAVANA\tgene\t1000\t2000\t.\t+\t.\tID=gene1;Name=BRCA\n\
chr1\tHAVANA\texon\t1000\t1200\t.\t+\t.\tID=exon1;Parent=gene1\n\
chr2\tHAVANA\tgene\t50\t90\t.\t-\t.\tID=gene2\n",
        );
        let (okuyucu, basligi) = AnotasyonOkuyucu::ac(&p).unwrap();
        assert_eq!(basligi.format, VeriFormati::Gff);

        let kayitlar = okuyucu
            .bolge_sorgu("chr1:1-3000", &BellekButcesi::sinirsiz(), 100)
            .unwrap();
        assert_eq!(kayitlar.len(), 2);
        assert_eq!(kayitlar[0].tur, "gene");
        assert_eq!(kayitlar[0].baslangic, 1000);
        assert_eq!(kayitlar[0].bitis, 2000);
        assert_eq!(kayitlar[0].serit, '+');
        assert_eq!(kayitlar[0].ad.as_deref(), Some("gene1"));
        assert_eq!(kayitlar[0].kaynak.as_deref(), Some("HAVANA"));
        assert_eq!(kayitlar[1].tur, "exon");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn gtf_okur() {
        let p = yaz(
            "a.gtf",
            b"chr1\tensembl\tgene\t100\t900\t.\t+\t.\tgene_id \"ENSG1\"; gene_name \"FOO\";\n",
        );
        let (okuyucu, _) = AnotasyonOkuyucu::ac(&p).unwrap();
        let kayitlar = okuyucu
            .bolge_sorgu("chr1:1-1000", &BellekButcesi::sinirsiz(), 100)
            .unwrap();
        assert_eq!(kayitlar.len(), 1);
        assert_eq!(kayitlar[0].tur, "gene");
        assert_eq!(kayitlar[0].ad.as_deref(), Some("ENSG1"));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn anotasyon_olmayan_reddeder() {
        let p = yaz("x.fasta", b">sq0\nACGT\n");
        let hata = AnotasyonOkuyucu::ac(&p).err().expect("hata bekleniyor");
        assert_eq!(hata.ne_oldu, "Anotasyon dosyası değil");
        let _ = std::fs::remove_file(&p);
    }
}
