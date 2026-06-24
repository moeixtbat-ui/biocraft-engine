//! ÇE-01 — **VCF / BCF** varyant okuma.
//!
//! * **İndeksli** (`.tbi`/`.csi` mevcut + BGZF): `chr:start-end` **bölge sorgusu** yalnızca o
//!   bölgenin bloklarını okur (out-of-core, MK-09; "milyonlarca satır" RAM'e alınmaz).
//! * **Linear** (düz `.vcf`, indeks yok): akışlı tarama + bölge süzme (out-of-core, ama indeksli
//!   değil → büyük dosyada yavaş; "BGZF'le + indeksle" önerilir).
//!
//! VCF (lazy `vcf::Record`) ve BCF (`bcf::Record`) kayıtları ortak [`vcf::variant::Record`]
//! trait'i üzerinden **tek yolla** okunur (hizalama BAM/SAM/CRAM birliği ile aynı desen).
//! Her kayıttan kromozom/konum/kimlik/ref/alt/kalite/filtre + **INFO** alanları + **FORMAT**
//! (örnek sütun) anahtarları çıkarılır.

use std::fs::File;
use std::ops::Bound;
use std::path::{Path, PathBuf};

use noodles::bcf;
use noodles::core::Region;
use noodles::vcf;
use noodles::vcf::variant::record::samples::series::value::genotype::Phasing;
use noodles::vcf::variant::record::samples::series::Value as SeriesValue;
use noodles::vcf::variant::record::{
    AlternateBases as _, Filters as _, Ids as _, Info as _, ReferenceBases as _, Samples as _,
};
use noodles::vcf::variant::Record as VariantRecord;

use biocraft_sdk::biocraft_types::ErrorReport;

use super::budget::BellekButcesi;
use super::detect::{formati_belirle, VeriFormati};

type VcfIndexed = vcf::io::IndexedReader<noodles::bgzf::io::Reader<File>>;
type BcfIndexed = bcf::io::IndexedReader<noodles::bgzf::io::Reader<File>>;

/// Varyant dosyasının özeti (başlık) — format, örnek adları, bölge sorgusu indeksli mi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaryantBasligi {
    /// Dosya formatı (VCF/BCF).
    pub format: VeriFormati,
    /// Örnek (sample) adları.
    pub ornekler: Vec<String>,
    /// Bölge sorgusu indeksli (hızlı) mı yoksa linear (yavaş) mi?
    pub indeksli: bool,
}

/// Tek bir varyant kaydının sadeleştirilmiş görünümü.
#[derive(Debug, Clone, PartialEq)]
pub struct VaryantKaydi {
    /// Kromozom/kontig adı (CHROM).
    pub kromozom: String,
    /// 1-tabanlı konum (POS).
    pub konum: usize,
    /// Kimlik(ler) (ID; yoksa ".").
    pub kimlik: String,
    /// Referans aleli (REF).
    pub referans: String,
    /// Alternatif alel(ler) (ALT).
    pub alternatifler: Vec<String>,
    /// Kalite skoru (QUAL; yoksa `None`).
    pub kalite: Option<f32>,
    /// Filtreler (FILTER; "PASS" veya süzgeç adları).
    pub filtreler: Vec<String>,
    /// INFO alanları (anahtar, metinleştirilmiş değer) — bellek için ilk birkaçı.
    pub info: Vec<(String, String)>,
    /// Örnek (sample) sayısı.
    pub ornek_sayisi: usize,
    /// FORMAT anahtarları (örnek sütun adları: GT/DP/…).
    pub format_anahtarlari: Vec<String>,
    /// Her örnek için **GT** (genotip) metni (`0/1`, `1|1`, `./.` …) — örnek sırasıyla.
    /// GT yoksa örnek başına ".".  Genotip ızgarası (ÇE-04) zigositeyi bundan çözer.
    pub genotipler: Vec<String>,
}

/// INFO listesinde tutulacak en fazla alan sayısı (özet; bellek koruması).
const AZAMI_INFO: usize = 16;

/// Format-bağımsız varyant okuyucu.
pub struct VaryantOkuyucu {
    ic: Ic,
    header: vcf::Header,
    /// Açılan dosyanın yolu — tüm-dosya akışlı taraması ([`VaryantOkuyucu::tum_tara`]) için.
    yol: PathBuf,
}

enum Ic {
    VcfIndeksli(VcfIndexed),
    BcfIndeksli(BcfIndexed),
    /// Düz VCF: indeks yok → bölge sorgusunda dosya yeniden açılıp lineer taranır.
    VcfDuz {
        yol: PathBuf,
    },
    /// İndekssiz BCF (BGZF ama indekssiz): akışlı tarama.
    BcfDuz {
        yol: PathBuf,
    },
}

impl VaryantOkuyucu {
    /// Bir varyant dosyası açar.  Format otomatik tanınır.  Yanında `.tbi`/`.csi` indeks varsa
    /// **indeksli** (hızlı bölge sorgusu), yoksa **linear** mod seçilir.
    pub fn ac(yol: &Path) -> Result<(Self, VaryantBasligi), ErrorReport> {
        let format = formati_belirle(yol)?;
        if !format.varyant_mi() {
            return Err(ErrorReport::new(
                "Varyant dosyası değil",
                format!("'{}' bir VCF/BCF dosyası değil", yol.display()),
                "VCF (.vcf/.vcf.gz) veya BCF (.bcf) uzantılı bir dosya seçin",
            ));
        }
        let indeksli_yol = indeks_var_mi(yol);

        match (format, indeksli_yol) {
            (VeriFormati::Vcf, true) => {
                let mut r = vcf::io::indexed_reader::Builder::default()
                    .build_from_path(yol)
                    .map_err(|e| acilma_hatasi(yol, "VCF", ".tbi/.csi", e))?;
                let header = r.read_header().map_err(|e| baslik_hatasi(yol, &e))?;
                let basligi = baslik_ozeti(format, &header, true);
                Ok((
                    Self {
                        ic: Ic::VcfIndeksli(r),
                        header,
                        yol: yol.to_path_buf(),
                    },
                    basligi,
                ))
            }
            (VeriFormati::Bcf, true) => {
                let mut r = bcf::io::indexed_reader::Builder::default()
                    .build_from_path(yol)
                    .map_err(|e| acilma_hatasi(yol, "BCF", ".csi", e))?;
                let header = r.read_header().map_err(|e| baslik_hatasi(yol, &e))?;
                let basligi = baslik_ozeti(format, &header, true);
                Ok((
                    Self {
                        ic: Ic::BcfIndeksli(r),
                        header,
                        yol: yol.to_path_buf(),
                    },
                    basligi,
                ))
            }
            (VeriFormati::Vcf, false) => {
                let mut r = vcf::io::reader::Builder::default()
                    .build_from_path(yol)
                    .map_err(|e| io_hatasi(yol, "VCF okuma", &e))?;
                let header = r.read_header().map_err(|e| baslik_hatasi(yol, &e))?;
                let basligi = baslik_ozeti(format, &header, false);
                Ok((
                    Self {
                        ic: Ic::VcfDuz {
                            yol: yol.to_path_buf(),
                        },
                        header,
                        yol: yol.to_path_buf(),
                    },
                    basligi,
                ))
            }
            (VeriFormati::Bcf, false) => {
                let mut r = bcf::io::Reader::new(
                    File::open(yol).map_err(|e| io_hatasi(yol, "BCF okuma", &e))?,
                );
                let header = r.read_header().map_err(|e| baslik_hatasi(yol, &e))?;
                let basligi = baslik_ozeti(format, &header, false);
                Ok((
                    Self {
                        ic: Ic::BcfDuz {
                            yol: yol.to_path_buf(),
                        },
                        header,
                        yol: yol.to_path_buf(),
                    },
                    basligi,
                ))
            }
            _ => unreachable!("format.varyant_mi() yalnız Vcf/Bcf'e izin verir"),
        }
    }

    /// VCF/BCF başlığına erişim (örnek adları, INFO/FORMAT tanımları).
    pub fn header(&self) -> &vcf::Header {
        &self.header
    }

    /// Bir bölgeyi (`chr:start-end`) sorgular.  En fazla `max_kayit` kayıt; bütçe aşılırsa
    /// reddedilir (İP-08).  İndeksli (hızlı) ya da linear (yavaş) yola göre çalışır.
    pub fn bolge_sorgu(
        &mut self,
        bolge: &str,
        butce: &BellekButcesi,
        max_kayit: usize,
    ) -> Result<Vec<VaryantKaydi>, ErrorReport> {
        let region: Region = bolge.parse().map_err(|_| gecersiz_bolge(bolge))?;

        match &mut self.ic {
            Ic::VcfIndeksli(r) => {
                // Query'nin kendisi iteratör değil; `.records()` blok-farkında iteratör verir.
                let sorgu = r
                    .query(&self.header, &region)
                    .map_err(|e| bolge_hatasi(bolge, &e))?
                    .records();
                topla(sorgu, &self.header, butce, max_kayit)
            }
            Ic::BcfIndeksli(r) => {
                let sorgu = r
                    .query(&self.header, &region)
                    .map_err(|e| bolge_hatasi(bolge, &e))?
                    .records();
                topla(sorgu, &self.header, butce, max_kayit)
            }
            Ic::VcfDuz { yol } => {
                let mut r = vcf::io::reader::Builder::default()
                    .build_from_path(&*yol)
                    .map_err(|e| io_hatasi(yol, "VCF okuma", &e))?;
                r.read_header().map_err(|e| baslik_hatasi(yol, &e))?;
                linear_bolge(r.records(), &self.header, &region, butce, max_kayit)
            }
            Ic::BcfDuz { yol } => {
                let mut r = bcf::io::Reader::new(
                    File::open(&*yol).map_err(|e| io_hatasi(yol, "BCF okuma", &e))?,
                );
                r.read_header().map_err(|e| baslik_hatasi(yol, &e))?;
                linear_bolge(r.records(), &self.header, &region, butce, max_kayit)
            }
        }
    }

    /// **Tüm dosyayı** baştan **akışlı** (streaming) tarar — bölge belirtilmeden (ör. "tüm
    /// kromozomlar" görünümü).  İndeks gerektirmez: dosya yeniden plain (indekssiz) okuyucuyla
    /// açılıp kayıtlar tek tek okunur → **out-of-core** (MK-09; tüm dosya RAM'e alınmaz, yalnız
    /// `max_kayit`'a + bütçeye kadar toplanır).  Çok büyük dosyada bölge sorgusu (indeksli) tercih
    /// edilir; bu, "bütün varyantları gör" / küçük-orta dosya için pratik yoldur.
    pub fn tum_tara(
        &mut self,
        butce: &BellekButcesi,
        max_kayit: usize,
    ) -> Result<Vec<VaryantKaydi>, ErrorReport> {
        let format = formati_belirle(&self.yol)?;
        match format {
            VeriFormati::Vcf => {
                let mut r = vcf::io::reader::Builder::default()
                    .build_from_path(&self.yol)
                    .map_err(|e| io_hatasi(&self.yol, "VCF okuma", &e))?;
                r.read_header().map_err(|e| baslik_hatasi(&self.yol, &e))?;
                topla(r.records(), &self.header, butce, max_kayit)
            }
            VeriFormati::Bcf => {
                let mut r = bcf::io::Reader::new(
                    File::open(&self.yol).map_err(|e| io_hatasi(&self.yol, "BCF okuma", &e))?,
                );
                r.read_header().map_err(|e| baslik_hatasi(&self.yol, &e))?;
                topla(r.records(), &self.header, butce, max_kayit)
            }
            _ => unreachable!("ac() yalnız Vcf/Bcf açar"),
        }
    }
}

// ─── Ortak kayıt toplama ────────────────────────────────────────────────────────

/// İndeksli sorgu iteratörünü `VaryantKaydi` listesine toplar (bütçe + üst sınır).
fn topla<I, R>(
    it: I,
    header: &vcf::Header,
    butce: &BellekButcesi,
    max_kayit: usize,
) -> Result<Vec<VaryantKaydi>, ErrorReport>
where
    I: Iterator<Item = std::io::Result<R>>,
    R: VariantRecord,
{
    let mut sonuc = Vec::new();
    let mut tahmini: u64 = 0;
    for res in it {
        if sonuc.len() >= max_kayit {
            break;
        }
        let kayit = res.map_err(|e| kayit_okuma_hatasi(&e))?;
        let v = kayit_to_varyant(&kayit, header)?;
        tahmini += tahmini_bayt(&v);
        butce.kontrol(tahmini)?;
        sonuc.push(v);
    }
    Ok(sonuc)
}

/// Linear (indekssiz) bölge taraması: kayıt kayıt okur, bölgeyle örtüşeni süzer (out-of-core).
fn linear_bolge<I, R>(
    it: I,
    header: &vcf::Header,
    region: &Region,
    butce: &BellekButcesi,
    max_kayit: usize,
) -> Result<Vec<VaryantKaydi>, ErrorReport>
where
    I: Iterator<Item = std::io::Result<R>>,
    R: VariantRecord,
{
    let (hedef_ad, r_bas, r_bit) = region_araligi(region);
    let mut sonuc = Vec::new();
    let mut tahmini: u64 = 0;
    for res in it {
        if sonuc.len() >= max_kayit {
            break;
        }
        let kayit = res.map_err(|e| kayit_okuma_hatasi(&e))?;
        let v = kayit_to_varyant(&kayit, header)?;
        // Bölge süzme: kromozom + [konum, konum+ref_uzunluk-1] ile [r_bas, r_bit] örtüşmesi.
        if v.kromozom != hedef_ad {
            continue;
        }
        let bas = v.konum;
        let bit = bas + v.referans.len().max(1) - 1;
        if bas <= r_bit && bit >= r_bas {
            tahmini += tahmini_bayt(&v);
            butce.kontrol(tahmini)?;
            sonuc.push(v);
        }
    }
    Ok(sonuc)
}

/// Herhangi bir noodles varyant kaydını (VCF/BCF) sade `VaryantKaydi`'ya çevirir (trait üzerinden).
fn kayit_to_varyant<R: VariantRecord>(
    kayit: &R,
    header: &vcf::Header,
) -> Result<VaryantKaydi, ErrorReport> {
    let kromozom = kayit
        .reference_sequence_name(header)
        .map_err(|e| kayit_okuma_hatasi(&e))?
        .to_string();
    let konum = match kayit.variant_start() {
        Some(r) => r.map_err(|e| kayit_okuma_hatasi(&e))?.get(),
        None => 0,
    };

    let ids: Vec<String> = kayit.ids().iter().map(|s| s.to_string()).collect();
    let kimlik = if ids.is_empty() {
        ".".to_string()
    } else {
        ids.join(";")
    };

    let referans: String = {
        let rb = kayit.reference_bases();
        let baytlar: std::io::Result<Vec<u8>> = rb.iter().collect();
        String::from_utf8_lossy(&baytlar.map_err(|e| kayit_okuma_hatasi(&e))?).into_owned()
    };

    let alternatifler: Vec<String> = {
        let ab = kayit.alternate_bases();
        let mut v = Vec::new();
        for r in ab.iter() {
            v.push(r.map_err(|e| kayit_okuma_hatasi(&e))?.to_string());
        }
        v
    };

    let kalite = match kayit.quality_score() {
        Some(r) => Some(r.map_err(|e| kayit_okuma_hatasi(&e))?),
        None => None,
    };

    let filtreler: Vec<String> = {
        let f = kayit.filters();
        let mut v = Vec::new();
        for r in f.iter(header) {
            v.push(r.map_err(|e| kayit_okuma_hatasi(&e))?.to_string());
        }
        v
    };

    let info: Vec<(String, String)> = {
        let i = kayit.info();
        let mut v = Vec::new();
        for r in i.iter(header) {
            if v.len() >= AZAMI_INFO {
                break;
            }
            let (anahtar, deger) = r.map_err(|e| kayit_okuma_hatasi(&e))?;
            let deger_s = match deger {
                Some(d) => format!("{d:?}"),
                None => String::new(),
            };
            v.push((anahtar.to_string(), deger_s));
        }
        v
    };

    let samples = kayit.samples().map_err(|e| kayit_okuma_hatasi(&e))?;
    let ornek_sayisi = samples.len();
    let format_anahtarlari: Vec<String> = {
        let mut v = Vec::new();
        for r in samples.column_names(header) {
            v.push(r.map_err(|e| kayit_okuma_hatasi(&e))?.to_string());
        }
        v
    };

    // Per-örnek GT (genotip) sütunu — varsa zigosite buradan çözülür (ÇE-04 genotip ızgarası).
    let genotipler: Vec<String> = match samples.select(header, "GT") {
        Some(seri) => {
            let seri = seri.map_err(|e| kayit_okuma_hatasi(&e))?;
            let mut v = Vec::with_capacity(ornek_sayisi);
            for r in seri.iter(header) {
                let deger = r.map_err(|e| kayit_okuma_hatasi(&e))?;
                v.push(gt_deger_metni(deger)?);
            }
            v
        }
        None => Vec::new(),
    };

    Ok(VaryantKaydi {
        kromozom,
        konum,
        kimlik,
        referans,
        alternatifler,
        kalite,
        filtreler,
        info,
        ornek_sayisi,
        format_anahtarlari,
        genotipler,
    })
}

/// Tek bir örnek hücresinin (FORMAT/GT) değerini VCF metnine çevirir.  Genotip için alel
/// indekslerini fazlama ayracıyla (`/` fazsız, `|` fazlı) birleştirir (`0/1`, `1|1`, `./.`).
fn gt_deger_metni(deger: Option<SeriesValue>) -> Result<String, ErrorReport> {
    use std::fmt::Write as _;
    match deger {
        None => Ok(".".to_string()),
        Some(SeriesValue::Genotype(gt)) => {
            let mut s = String::new();
            for (i, allel) in gt.iter().enumerate() {
                let (pos, faz) = allel.map_err(|e| kayit_okuma_hatasi(&e))?;
                if i > 0 {
                    s.push(match faz {
                        Phasing::Phased => '|',
                        Phasing::Unphased => '/',
                    });
                }
                match pos {
                    Some(n) => {
                        let _ = write!(s, "{n}");
                    }
                    None => s.push('.'),
                }
            }
            Ok(if s.is_empty() { ".".to_string() } else { s })
        }
        Some(SeriesValue::String(c)) => Ok(c.into_owned()),
        Some(SeriesValue::Integer(n)) => Ok(n.to_string()),
        Some(SeriesValue::Character(c)) => Ok(c.to_string()),
        Some(SeriesValue::Float(f)) => Ok(f.to_string()),
        Some(SeriesValue::Array(_)) => Ok(".".to_string()),
    }
}

/// Bir varyant kaydının bellekteki kaba bayt tahmini (bütçe muhasebesi).
fn tahmini_bayt(v: &VaryantKaydi) -> u64 {
    let alt: usize = v.alternatifler.iter().map(|a| a.len()).sum();
    let info: usize = v.info.iter().map(|(k, d)| k.len() + d.len()).sum();
    let gt: usize = v.genotipler.iter().map(|g| g.len() + 8).sum();
    (96 + v.kromozom.len() + v.referans.len() + alt + info + gt) as u64
}

/// Bir dosyanın yanında tabix (`.tbi`) veya CSI (`.csi`) indeks var mı?
fn indeks_var_mi(yol: &Path) -> bool {
    ek(yol, ".tbi").exists() || ek(yol, ".csi").exists()
}

fn ek(yol: &Path, ek: &str) -> PathBuf {
    let mut s = yol.as_os_str().to_os_string();
    s.push(ek);
    PathBuf::from(s)
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

fn acilma_hatasi(yol: &Path, format: &str, indeks_uzanti: &str, e: std::io::Error) -> ErrorReport {
    if e.kind() == std::io::ErrorKind::NotFound {
        ErrorReport::new(
            format!("{format} indeksi bulunamadı"),
            format!(
                "'{}' için indeksli bölge sorgusu ({indeks_uzanti}) gerektirir; indeks yok",
                yol.display()
            ),
            "Dosyayı BGZF ile sıkıştırıp (bgzip) tabix ile indeksleyin; indekssiz de açılır (yavaş, linear)",
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
        "Varyant başlığı okunamadı",
        format!("'{}' başlığı (header) ayrıştırılamadı", yol.display()),
        "Dosyanın geçerli bir VCF/BCF olduğundan emin olun",
    )
    .with_teknik_detay(e.to_string())
}

fn bolge_hatasi(bolge: &str, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "Bölge sorgusu başarısız",
        format!("'{bolge}' bölgesi sorgulanırken hata oluştu"),
        "Bölge adının başlıktaki kromozom/kontiglerden biri olduğundan emin olun",
    )
    .with_teknik_detay(e.to_string())
}

fn gecersiz_bolge(bolge: &str) -> ErrorReport {
    ErrorReport::new(
        "Geçersiz bölge ifadesi",
        format!("'{bolge}' bir bölgeye çözülemedi (beklenen: ad:başlangıç-bitiş)"),
        "Örn. 'chr1:100-200' veya yalnızca 'chr1' yazın",
    )
}

fn kayit_okuma_hatasi(e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "Varyant kaydı okunamadı",
        "bir kayıt ayrıştırılırken hata oluştu (dosya bozuk veya kesilmiş olabilir)",
        "Dosya bütünlüğünü kontrol edin; gerekirse yeniden indirin",
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

fn baslik_ozeti(format: VeriFormati, header: &vcf::Header, indeksli: bool) -> VaryantBasligi {
    let ornekler = header
        .sample_names()
        .iter()
        .map(|s| s.to_string())
        .collect();
    VaryantBasligi {
        format,
        ornekler,
        indeksli,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn yaz(ad: &str, icerik: &[u8]) -> PathBuf {
        let mut yol = std::env::temp_dir();
        yol.push(format!("biocraft_vcf_{}_{ad}", std::process::id()));
        File::create(&yol).unwrap().write_all(icerik).unwrap();
        yol
    }

    const VCF: &[u8] = b"\
##fileformat=VCFv4.3
##contig=<ID=chr1,length=1000>
##INFO=<ID=DP,Number=1,Type=Integer,Description=\"Depth\">
##FORMAT=<ID=GT,Number=1,Type=String,Description=\"Genotype\">
#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1
chr1\t100\trs1\tA\tT\t50\tPASS\tDP=30\tGT\t0/1
chr1\t150\t.\tG\tC,A\t60\tPASS\tDP=20\tGT\t1/1
chr1\t900\trs2\tT\tG\t40\tPASS\tDP=10\tGT\t0/0
";

    #[test]
    fn duz_vcf_acilir_ve_ornek_okunur() {
        let p = yaz("a.vcf", VCF);
        let (okuyucu, basligi) = VaryantOkuyucu::ac(&p).unwrap();
        assert_eq!(basligi.format, VeriFormati::Vcf);
        assert!(!basligi.indeksli);
        assert_eq!(basligi.ornekler, vec!["S1"]);
        assert_eq!(okuyucu.header().sample_names().len(), 1);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn duz_vcf_bolge_sorgu_suzer() {
        let p = yaz("b.vcf", VCF);
        let (mut okuyucu, _) = VaryantOkuyucu::ac(&p).unwrap();
        // chr1:90-160 → ilk iki kayıt (100, 150); 900 hariç.
        let kayitlar = okuyucu
            .bolge_sorgu("chr1:90-160", &BellekButcesi::sinirsiz(), 1000)
            .unwrap();
        assert_eq!(kayitlar.len(), 2);
        assert_eq!(kayitlar[0].kromozom, "chr1");
        assert_eq!(kayitlar[0].konum, 100);
        assert_eq!(kayitlar[0].kimlik, "rs1");
        assert_eq!(kayitlar[0].referans, "A");
        assert_eq!(kayitlar[0].alternatifler, vec!["T"]);
        assert_eq!(kayitlar[0].kalite, Some(50.0));
        assert_eq!(kayitlar[0].filtreler, vec!["PASS"]);
        assert!(kayitlar[0].info.iter().any(|(k, _)| k == "DP"));
        assert_eq!(kayitlar[0].ornek_sayisi, 1);
        assert_eq!(kayitlar[0].format_anahtarlari, vec!["GT"]);
        // GT (genotip) sütunu örnek başına okunur (zigosite çözümü için).
        assert_eq!(kayitlar[0].genotipler, vec!["0/1"]);
        assert_eq!(kayitlar[1].genotipler, vec!["1/1"]);
        // Çoklu ALT.
        assert_eq!(kayitlar[1].alternatifler, vec!["C", "A"]);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn tum_tara_tum_kayitlari_dondurur() {
        let p = yaz("f.vcf", VCF);
        let (mut okuyucu, _) = VaryantOkuyucu::ac(&p).unwrap();
        // Bölge belirtmeden tüm dosya akışlı taranır → 3 kayıt.
        let hepsi = okuyucu.tum_tara(&BellekButcesi::sinirsiz(), 1000).unwrap();
        assert_eq!(hepsi.len(), 3);
        assert_eq!(hepsi[2].konum, 900);
        assert_eq!(hepsi[2].genotipler, vec!["0/0"]);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn bolge_disindaki_kayit_gelmez() {
        let p = yaz("c.vcf", VCF);
        let (mut okuyucu, _) = VaryantOkuyucu::ac(&p).unwrap();
        let kayitlar = okuyucu
            .bolge_sorgu("chr1:800-1000", &BellekButcesi::sinirsiz(), 1000)
            .unwrap();
        assert_eq!(kayitlar.len(), 1);
        assert_eq!(kayitlar[0].konum, 900);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn butce_asiminda_reddeder() {
        let p = yaz("d.vcf", VCF);
        let (mut okuyucu, _) = VaryantOkuyucu::ac(&p).unwrap();
        let hata = okuyucu
            .bolge_sorgu("chr1:1-1000", &BellekButcesi::yeni(10), 1000)
            .unwrap_err();
        assert_eq!(hata.ne_oldu, "Bellek bütçesi aşıldı");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn varyant_olmayan_reddeder() {
        let p = yaz("e.fasta", b">sq0\nACGT\n");
        let hata = VaryantOkuyucu::ac(&p).err().expect("hata bekleniyor");
        assert_eq!(hata.ne_oldu, "Varyant dosyası değil");
        let _ = std::fs::remove_file(&p);
    }
}
