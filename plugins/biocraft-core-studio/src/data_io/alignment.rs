//! ÇE-01 — **BAM / SAM / CRAM** hizalama okuma.
//!
//! * **BAM:** BGZF blok-farkında (MK-32); `.bai`/`.csi` ile **indeksli bölge sorgusu**
//!   (`chr:start-end`) — yalnızca bölgenin blokları okunur (out-of-core, MK-09).
//! * **CRAM:** `.crai` indeksi + **referans dizisi** çözümü (referans repository).  Referans
//!   verilmezse **net hata** (CRAM referans olmadan çözülemez).
//! * **SAM:** düz metin; indekssiz → **akışlı lineer tarama** ile bölge süzülür (out-of-core,
//!   ama indeksli değil → büyük dosyada yavaş; "BAM'e dönüştür + indeksle" önerilir).
//!
//! Tüm kayıt erişimi noodles `sam::alignment::Record` trait'i üzerinden tektir (BAM/SAM/CRAM ortak).

use std::fs::File;
use std::io::BufReader;
use std::ops::Bound;
use std::path::{Path, PathBuf};

use noodles::bam;
use noodles::core::Region;
use noodles::cram;
use noodles::fasta;
use noodles::sam;
use noodles::sam::alignment::record::Cigar as _; // alignment_span() için trait kapsamı
use noodles::sam::alignment::Record as AlignmentRecord;

use biocraft_sdk::biocraft_types::ErrorReport;

use super::budget::BellekButcesi;
use super::detect::{formati_belirle, VeriFormati};

type BamReader = bam::io::IndexedReader<noodles::bgzf::io::Reader<File>>;
type CramReader = cram::io::IndexedReader<File>;

/// Hizalama dosyasının özeti (başlık) — formatı, referans dizileri, bölge sorgusu indeksli mi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HizalamaBasligi {
    /// Dosya formatı (BAM/SAM/CRAM).
    pub format: VeriFormati,
    /// Referans dizileri: (ad, uzunluk).
    pub referans_diziler: Vec<(String, usize)>,
    /// Bölge sorgusu indeksli mi (BAM/CRAM = true) yoksa lineer mi (SAM = false)?
    pub indeksli: bool,
}

/// Tek bir hizalama kaydının (read) sadeleştirilmiş görünümü.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HizalamaKaydi {
    /// Read adı (QNAME).
    pub ad: String,
    /// Bayrak bitleri (FLAG).
    pub bayrak: u16,
    /// Hizalandığı referans adı (eşlenmemişse `None`).
    pub referans: Option<String>,
    /// 1-tabanlı hizalama başlangıç konumu (eşlenmemişse `None`).
    pub konum: Option<usize>,
    /// Eşleme kalitesi (MAPQ; yoksa `None`).
    pub mapq: Option<u8>,
    /// Dizi (SEQ) uzunluğu.
    pub dizi_uzunlugu: usize,
}

/// Format-bağımsız hizalama okuyucu.
pub struct HizalamaOkuyucu {
    ic: Ic,
    header: sam::Header,
}

enum Ic {
    Bam(BamReader),
    Cram(CramReader),
    /// SAM düz metin: indeks yok → bölge sorgusunda dosya yeniden açılıp lineer taranır.
    Sam {
        yol: PathBuf,
    },
}

impl HizalamaOkuyucu {
    /// Bir hizalama dosyası açar.  Format otomatik tanınır.  CRAM için `referans` (indeksli FASTA)
    /// gereklidir; verilmezse net hata.  BAM/CRAM indeksi yoksa "İndeks oluştur" önerilir.
    pub fn ac(yol: &Path, referans: Option<&Path>) -> Result<(Self, HizalamaBasligi), ErrorReport> {
        let format = formati_belirle(yol)?;
        match format {
            VeriFormati::Bam => {
                let mut r = bam::io::indexed_reader::Builder::default()
                    .build_from_path(yol)
                    .map_err(|e| acilma_hatasi(yol, "BAM", ".bai/.csi", e))?;
                let header = r.read_header().map_err(|e| baslik_hatasi(yol, &e))?;
                let basligi = baslik_ozeti(format, &header, true);
                Ok((
                    Self {
                        ic: Ic::Bam(r),
                        header,
                    },
                    basligi,
                ))
            }
            VeriFormati::Cram => {
                let ref_yol = referans.ok_or_else(|| cram_referans_gerekli(yol))?;
                let repo = referans_repo(ref_yol)?;
                let mut r = cram::io::indexed_reader::Builder::default()
                    .set_reference_sequence_repository(repo)
                    .build_from_path(yol)
                    .map_err(|e| acilma_hatasi(yol, "CRAM", ".crai", e))?;
                let header = r.read_header().map_err(|e| baslik_hatasi(yol, &e))?;
                let basligi = baslik_ozeti(format, &header, true);
                Ok((
                    Self {
                        ic: Ic::Cram(r),
                        header,
                    },
                    basligi,
                ))
            }
            VeriFormati::Sam => {
                let mut r = sam::io::Reader::new(BufReader::new(
                    File::open(yol).map_err(|e| io_hatasi(yol, "SAM okuma", &e))?,
                ));
                let header = r.read_header().map_err(|e| baslik_hatasi(yol, &e))?;
                let basligi = baslik_ozeti(format, &header, false);
                Ok((
                    Self {
                        ic: Ic::Sam {
                            yol: yol.to_path_buf(),
                        },
                        header,
                    },
                    basligi,
                ))
            }
            _ => Err(ErrorReport::new(
                "Hizalama dosyası değil",
                format!("'{}' bir BAM/SAM/CRAM dosyası değil", yol.display()),
                "BAM, SAM veya CRAM uzantılı bir dosya seçin",
            )),
        }
    }

    /// Başlığa (referans dizileri / örnek bilgisi) erişim.
    pub fn header(&self) -> &sam::Header {
        &self.header
    }

    /// Bir bölgeyi (`chr:start-end`) sorgular.  En fazla `max_kayit` kayıt toplanır; bütçe
    /// aşılırsa reddedilir (İP-08).  BAM/CRAM indeksli (hızlı); SAM lineer (yavaş).
    pub fn bolge_sorgu(
        &mut self,
        bolge: &str,
        butce: &BellekButcesi,
        max_kayit: usize,
    ) -> Result<Vec<HizalamaKaydi>, ErrorReport> {
        let region: Region = bolge.parse().map_err(|_| {
            ErrorReport::new(
                "Geçersiz bölge ifadesi",
                format!("'{bolge}' bir bölgeye çözülemedi (beklenen: ad:başlangıç-bitiş)"),
                "Örn. 'sq0:10-20' veya yalnızca 'sq0' yazın",
            )
        })?;

        match &mut self.ic {
            Ic::Bam(r) => {
                // BAM Query'nin kendisi iteratör değil; `.records()` blok-farkında iteratörü verir.
                let sorgu = r
                    .query(&self.header, &region)
                    .map_err(|e| bolge_hatasi(bolge, &e))?
                    .records();
                topla(sorgu, &self.header, butce, max_kayit)
            }
            Ic::Cram(r) => {
                let sorgu = r
                    .query(&self.header, &region)
                    .map_err(|e| bolge_hatasi(bolge, &e))?;
                topla(sorgu, &self.header, butce, max_kayit)
            }
            Ic::Sam { yol } => sam_lineer_bolge(yol, &self.header, &region, butce, max_kayit),
        }
    }
}

// ─── Ortak kayıt dönüşümü (BAM/SAM/CRAM tek yol) ────────────────────────────────

/// Bir indekslenmiş sorgu iteratörünü `HizalamaKaydi` listesine toplar (bütçe + üst sınır gözeterek).
fn topla<I, R>(
    it: I,
    header: &sam::Header,
    butce: &BellekButcesi,
    max_kayit: usize,
) -> Result<Vec<HizalamaKaydi>, ErrorReport>
where
    I: Iterator<Item = std::io::Result<R>>,
    R: AlignmentRecord,
{
    let mut sonuc = Vec::new();
    let mut tahmini: u64 = 0;
    for res in it {
        if sonuc.len() >= max_kayit {
            break;
        }
        let kayit = res.map_err(|e| kayit_okuma_hatasi(&e))?;
        let h = kayit_to_hizalama(&kayit, header)?;
        tahmini += tahmini_kayit_bayt(&h);
        butce.kontrol(tahmini)?;
        sonuc.push(h);
    }
    Ok(sonuc)
}

/// Herhangi bir noodles hizalama kaydını sade `HizalamaKaydi`'ya çevirir (trait üzerinden).
fn kayit_to_hizalama<R: AlignmentRecord>(
    kayit: &R,
    header: &sam::Header,
) -> Result<HizalamaKaydi, ErrorReport> {
    let ad = kayit.name().map(|n| n.to_string()).unwrap_or_default();
    let bayrak = u16::from(kayit.flags().map_err(|e| kayit_okuma_hatasi(&e))?);
    let referans = referans_adi(kayit, header)?;
    let konum = match kayit.alignment_start() {
        Some(r) => Some(r.map_err(|e| kayit_okuma_hatasi(&e))?.get()),
        None => None,
    };
    let mapq = match kayit.mapping_quality() {
        Some(r) => Some(u8::from(r.map_err(|e| kayit_okuma_hatasi(&e))?)),
        None => None,
    };
    let dizi_uzunlugu = kayit.sequence().len();
    Ok(HizalamaKaydi {
        ad,
        bayrak,
        referans,
        konum,
        mapq,
        dizi_uzunlugu,
    })
}

/// Kaydın hizalandığı referans adını (header'dan) çözer; eşlenmemişse `None`.
fn referans_adi<R: AlignmentRecord>(
    kayit: &R,
    header: &sam::Header,
) -> Result<Option<String>, ErrorReport> {
    match kayit.reference_sequence_id(header) {
        Some(r) => {
            let id = r.map_err(|e| kayit_okuma_hatasi(&e))?;
            Ok(header
                .reference_sequences()
                .get_index(id)
                .map(|(ad, _)| ad.to_string()))
        }
        None => Ok(None),
    }
}

/// Bir kaydın bellekte kabaca kaç bayt tuttuğunun tahmini (bütçe muhasebesi için).
fn tahmini_kayit_bayt(h: &HizalamaKaydi) -> u64 {
    (64 + h.ad.len() + h.dizi_uzunlugu) as u64
}

// ─── SAM lineer bölge taraması (indekssiz; out-of-core akış) ────────────────────

fn sam_lineer_bolge(
    yol: &Path,
    header: &sam::Header,
    region: &Region,
    butce: &BellekButcesi,
    max_kayit: usize,
) -> Result<Vec<HizalamaKaydi>, ErrorReport> {
    let (hedef_ad, r_bas, r_bit) = region_araligi(region);

    let mut okuyucu = sam::io::Reader::new(BufReader::new(
        File::open(yol).map_err(|e| io_hatasi(yol, "SAM okuma", &e))?,
    ));
    okuyucu.read_header().map_err(|e| baslik_hatasi(yol, &e))?;

    let mut sonuc = Vec::new();
    let mut tahmini: u64 = 0;
    for res in okuyucu.records() {
        if sonuc.len() >= max_kayit {
            break;
        }
        let kayit = res.map_err(|e| kayit_okuma_hatasi(&e))?;

        // Referans adı eşleşmiyorsa atla.
        let Some(ref_ad) = referans_adi(&kayit, header)? else {
            continue;
        };
        if ref_ad != hedef_ad {
            continue;
        }
        // Hizalama aralığı [bas, bit] bölge [r_bas, r_bit] ile örtüşüyor mu? (kesin: CIGAR span ile)
        let Some(bas) = (match kayit.alignment_start() {
            Some(r) => Some(r.map_err(|e| kayit_okuma_hatasi(&e))?.get()),
            None => None,
        }) else {
            continue;
        };
        let span = kayit.cigar().alignment_span().unwrap_or(1).max(1);
        let bit = bas + span - 1;
        if bas <= r_bit && bit >= r_bas {
            let h = kayit_to_hizalama(&kayit, header)?;
            tahmini += tahmini_kayit_bayt(&h);
            butce.kontrol(tahmini)?;
            sonuc.push(h);
        }
    }
    Ok(sonuc)
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

// ─── CRAM referans repository ───────────────────────────────────────────────────

/// İndeksli FASTA referansından bir noodles `Repository` kurar (CRAM çözümü için).
/// Referansın `.fai` indeksi yoksa otomatik oluşturulur.
fn referans_repo(ref_yol: &Path) -> Result<fasta::Repository, ErrorReport> {
    // Referans FASTA indeksli (.fai) olmalı; yoksa üret.
    let fai = super::fasta::fai_yolu(ref_yol);
    if !fai.exists() {
        super::fasta::fai_olustur(ref_yol)?;
    }
    let okuyucu = fasta::io::indexed_reader::Builder::default()
        .build_from_path(ref_yol)
        .map_err(|e| {
            ErrorReport::new(
                "Referans FASTA açılamadı",
                format!(
                    "CRAM çözümü için referans '{}' açılamadı",
                    ref_yol.display()
                ),
                "Referansın geçerli, indekslenebilir bir FASTA olduğundan emin olun",
            )
            .with_teknik_detay(e.to_string())
        })?;
    let adaptor = fasta::repository::adapters::IndexedReader::new(okuyucu);
    Ok(fasta::Repository::new(adaptor))
}

// ─── Hatalar ────────────────────────────────────────────────────────────────────

fn cram_referans_gerekli(yol: &Path) -> ErrorReport {
    ErrorReport::new(
        "CRAM için referans dizisi gerekli",
        format!(
            "'{}' bir CRAM dosyasıdır; CRAM diziyi referansa göre sıkıştırır, açmak için referans FASTA gerekir",
            yol.display()
        ),
        "Hizalamanın yapıldığı referans genomu (FASTA) seçin",
    )
    .with_eylem("Referans seç")
}

/// İndeks bulunamadıysa (NotFound) "indeks oluştur"; başka hata ise genel açılma hatası.
fn acilma_hatasi(yol: &Path, format: &str, indeks_uzanti: &str, e: std::io::Error) -> ErrorReport {
    if e.kind() == std::io::ErrorKind::NotFound {
        ErrorReport::new(
            format!("{format} indeksi bulunamadı"),
            format!(
                "'{}' için bölge sorgusu indeks ({indeks_uzanti}) gerektirir; indeks yok",
                yol.display()
            ),
            "İndeksi şimdi oluşturabilirim (samtools index eşdeğeri)",
        )
        .with_eylem("İndeks oluştur")
        .with_teknik_detay(e.to_string())
    } else {
        ErrorReport::new(
            format!("{format} dosyası açılamadı"),
            format!(
                "'{}' okunamadı (bozuk veya beklenmeyen biçim olabilir)",
                yol.display()
            ),
            "Dosya bütünlüğünü kontrol edin; gerekirse yeniden indirin",
        )
        .with_teknik_detay(e.to_string())
    }
}

fn baslik_hatasi(yol: &Path, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "Hizalama başlığı okunamadı",
        format!("'{}' başlığı (header) ayrıştırılamadı", yol.display()),
        "Dosyanın geçerli bir hizalama dosyası olduğundan emin olun",
    )
    .with_teknik_detay(e.to_string())
}

fn bolge_hatasi(bolge: &str, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "Bölge sorgusu başarısız",
        format!("'{bolge}' bölgesi sorgulanırken hata oluştu"),
        "Bölge adının referans dizilerinden biri olduğundan emin olun (header'a bakın)",
    )
    .with_teknik_detay(e.to_string())
}

fn kayit_okuma_hatasi(e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "Hizalama kaydı okunamadı",
        "bir kayıt ayrıştırılırken hata oluştu (dosya bozuk veya kesilmiş olabilir)",
        "Dosya bütünlüğünü kontrol edin; gerekirse yeniden indirin/indeksleyin",
    )
    .with_teknik_detay(e.to_string())
}

fn io_hatasi(yol: &Path, baglam: &str, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "Dosya açılamadı",
        format!("'{}' {baglam} için açılamadı", yol.display()),
        "Dosya yolunu ve okuma iznini kontrol edin",
    )
    .with_teknik_detay(e.to_string())
}

fn baslik_ozeti(format: VeriFormati, header: &sam::Header, indeksli: bool) -> HizalamaBasligi {
    let referans_diziler = header
        .reference_sequences()
        .iter()
        .map(|(ad, harita)| (ad.to_string(), harita.length().get()))
        .collect();
    HizalamaBasligi {
        format,
        referans_diziler,
        indeksli,
    }
}
