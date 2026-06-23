//! ÇE-01 — **FASTA / FASTQ** okuma.
//!
//! * **FASTA:** `.fai` indeksiyle **rastgele bölge** erişimi (`sq0:5-8`) — yalnızca istenen
//!   parça belleğe alınır (MK-09); büyük referans genomda bile "load all" yok.  İndeks yoksa
//!   net hata + **"İndeks oluştur"** akışı ([`fai_olustur`]).
//! * **FASTQ:** kayıt kayıt **akışlı** okuma (tüm dosya RAM'e alınmaz) + **kalite skoru**
//!   istatistiği (Phred min/maks/ortalama).
//!
//! noodles ile uygulanır; ham ayrıştırma elle yazılmaz (doğruluk-kritik).

use std::fs::File;
use std::io::BufReader;
use std::ops::Bound;
use std::path::{Path, PathBuf};

use noodles::core::Region;
use noodles::fasta;
use noodles::fastq;

use biocraft_sdk::biocraft_types::ErrorReport;

use super::budget::BellekButcesi;

// ─── FASTA ────────────────────────────────────────────────────────────────────

/// İndeksli FASTA okuyucu — yol + yüklenmiş `.fai` indeksi taşır.  Her bölge sorgusu dosyayı
/// indeks üzerinden seek ile açar (out-of-core; okuyucu durumu tutulmaz → basit ve güvenli).
pub struct FastaOkuyucu {
    yol: PathBuf,
    index: fasta::fai::Index,
}

/// Bir bölge sorgusunun sonucu — yalnızca istenen alt-dizi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiziParcasi {
    /// Dizi (kromozom/kontig) adı.
    pub ad: String,
    /// Sorgulanan bölge metni (örn. `sq0:5-8`).
    pub bolge: String,
    /// İstenen alt-dizinin baytları (yalnızca bu parça bellekte).
    pub diziler: Vec<u8>,
}

impl FastaOkuyucu {
    /// İndeksli FASTA açar.  `.fai` yoksa **net hata** döner (çözüm: [`fai_olustur`]).
    pub fn ac(yol: &Path) -> Result<Self, ErrorReport> {
        let fai_yol = fai_yolu(yol);
        if !fai_yol.exists() {
            return Err(indeks_yok_hatasi(yol, &fai_yol));
        }
        let index = fasta::fai::fs::read(&fai_yol).map_err(|e| {
            ErrorReport::new(
                "FASTA indeksi okunamadı",
                format!(
                    "'{}' (.fai) bozuk veya geçersiz olabilir",
                    fai_yol.display()
                ),
                "İndeksi yeniden oluşturun",
            )
            .with_eylem("İndeks oluştur")
            .with_teknik_detay(e.to_string())
        })?;
        Ok(Self {
            yol: yol.to_path_buf(),
            index,
        })
    }

    /// İndekste tanımlı dizi (kromozom/kontig) adları.
    pub fn dizi_adlari(&self) -> Vec<String> {
        self.index
            .as_ref()
            .iter()
            .map(|r| r.name().to_string())
            .collect()
    }

    /// Bir bölgenin (örn. `sq0:5-8`) dizisini **indeksli** çeker.  Yalnızca istenen parça
    /// belleğe alınır; öncesinde **bellek bütçesi** denetlenir (İP-08).
    pub fn bolge(&self, bolge: &str, butce: &BellekButcesi) -> Result<DiziParcasi, ErrorReport> {
        let region: Region = bolge.parse().map_err(|_| {
            ErrorReport::new(
                "Geçersiz bölge ifadesi",
                format!("'{bolge}' bir bölgeye çözülemedi (beklenen biçim: ad:başlangıç-bitiş)"),
                "Örn. 'sq0:5-8' veya yalnızca 'sq0' yazın",
            )
        })?;

        // Bütçe: bölge uzunluğunu tahmin et (üst sınır = dizi uzunluğu), materyalize etmeden önce denetle.
        let ad = region.name().to_string();
        let dizi_uzunlugu = self
            .index
            .as_ref()
            .iter()
            .find(|r| r.name() == region.name())
            .map(|r| r.length())
            .ok_or_else(|| {
                ErrorReport::new(
                    "Dizi bulunamadı",
                    format!("'{ad}' bu FASTA indeksinde tanımlı değil"),
                    "Geçerli bir dizi adı kullanın (dizi_adlari ile listeleyin)",
                )
            })?;
        let tahmin = bolge_uzunlugu(&region, dizi_uzunlugu);
        butce.kontrol(tahmin)?;

        // İndeksli seek + yalnızca istenen parçayı oku (out-of-core).
        let mut okuyucu = fasta::io::Reader::new(BufReader::new(
            File::open(&self.yol).map_err(|e| io_hatasi(&self.yol, "FASTA okuma", &e))?,
        ));
        let kayit = okuyucu.query(&self.index, &region).map_err(|e| {
            ErrorReport::new(
                "Bölge okunamadı",
                format!("'{bolge}' FASTA'dan çekilemedi"),
                "Bölgenin dizi sınırları içinde olduğundan emin olun",
            )
            .with_teknik_detay(e.to_string())
        })?;

        Ok(DiziParcasi {
            ad,
            bolge: bolge.to_string(),
            diziler: kayit.sequence().as_ref().to_vec(),
        })
    }
}

/// `<dosya>` için beklenen `.fai` indeks yolu (`<dosya>.fai`).
pub fn fai_yolu(yol: &Path) -> PathBuf {
    let mut s = yol.as_os_str().to_os_string();
    s.push(".fai");
    PathBuf::from(s)
}

/// Bir FASTA için `.fai` indeksi **oluşturur** (kullanıcı "indeksle" derse).  Üretilen yolu döndürür.
pub fn fai_olustur(yol: &Path) -> Result<PathBuf, ErrorReport> {
    let index = fasta::fs::index(yol).map_err(|e| {
        ErrorReport::new(
            "FASTA indekslenemedi",
            format!(
                "'{}' indekslenirken hata oluştu (dosya bozuk olabilir)",
                yol.display()
            ),
            "Dosyanın geçerli bir FASTA olduğundan emin olun",
        )
        .with_teknik_detay(e.to_string())
    })?;
    let fai_yol = fai_yolu(yol);
    fasta::fai::fs::write(&fai_yol, &index).map_err(|e| {
        ErrorReport::new(
            "İndeks yazılamadı",
            format!("'{}' diske yazılamadı", fai_yol.display()),
            "Klasör yazma iznini ve boş disk alanını kontrol edin",
        )
        .with_teknik_detay(e.to_string())
    })?;
    Ok(fai_yol)
}

/// İndekssiz **akışlı** FASTA okuma: her kayıt için `gozlemci(ad, dizi_uzunlugu)` çağrılır;
/// toplam kayıt sayısını döndürür.  Tüm dosya RAM'e alınmaz (MK-09).
pub fn fasta_akis<F>(yol: &Path, mut gozlemci: F) -> Result<usize, ErrorReport>
where
    F: FnMut(&str, usize),
{
    let mut okuyucu = fasta::io::Reader::new(BufReader::new(
        File::open(yol).map_err(|e| io_hatasi(yol, "FASTA akış", &e))?,
    ));
    let mut sayi = 0;
    for sonuc in okuyucu.records() {
        let kayit = sonuc.map_err(|e| ayristirma_hatasi(yol, "FASTA", &e))?;
        let ad = String::from_utf8_lossy(kayit.name());
        gozlemci(&ad, kayit.sequence().len());
        sayi += 1;
    }
    Ok(sayi)
}

// ─── FASTQ ──────────────────────────────────────────────────────────────────────

/// FASTQ akışlı okumanın kalite skoru özeti (Phred; ASCII-33 ofsetli ham bayttan).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KaliteOzeti {
    /// Kayıt (read) sayısı.
    pub kayit_sayisi: u64,
    /// Toplam baz (nükleotit) sayısı.
    pub toplam_baz: u64,
    /// En düşük Phred skoru (boşsa 0).
    pub min_phred: u8,
    /// En yüksek Phred skoru.
    pub max_phred: u8,
    /// Phred skorlarının toplamı (ortalama için).
    phred_toplam: u64,
}

impl KaliteOzeti {
    /// Ortalama Phred skoru (kayıt yoksa 0.0).
    pub fn ortalama_phred(&self) -> f64 {
        if self.toplam_baz == 0 {
            0.0
        } else {
            self.phred_toplam as f64 / self.toplam_baz as f64
        }
    }
}

/// FASTQ'yu **akışlı** okur; her kayıt için `gozlemci(ad, dizi_uzunlugu)` çağrılır ve kalite
/// skoru istatistiği biriktirilir.  Tüm dosya RAM'e alınmaz (MK-09).
pub fn fastq_akis<F>(yol: &Path, mut gozlemci: F) -> Result<KaliteOzeti, ErrorReport>
where
    F: FnMut(&str, usize),
{
    let mut okuyucu = fastq::io::Reader::new(BufReader::new(
        File::open(yol).map_err(|e| io_hatasi(yol, "FASTQ akış", &e))?,
    ));
    let mut ozet = KaliteOzeti::default();
    let mut ilk = true;
    for sonuc in okuyucu.records() {
        let kayit = sonuc.map_err(|e| ayristirma_hatasi(yol, "FASTQ", &e))?;
        let ad = String::from_utf8_lossy(kayit.name());
        let dizi = kayit.sequence();
        gozlemci(&ad, dizi.len());
        ozet.kayit_sayisi += 1;
        ozet.toplam_baz += dizi.len() as u64;
        // Kalite: ASCII karakterinden Phred = bayt - 33 (Sanger/Illumina 1.8+).
        for &q in kayit.quality_scores() {
            let phred = q.saturating_sub(33);
            ozet.phred_toplam += phred as u64;
            if ilk {
                ozet.min_phred = phred;
                ozet.max_phred = phred;
                ilk = false;
            } else {
                ozet.min_phred = ozet.min_phred.min(phred);
                ozet.max_phred = ozet.max_phred.max(phred);
            }
        }
    }
    Ok(ozet)
}

// ─── Yardımcılar ────────────────────────────────────────────────────────────────

/// Bölge uzunluğu tahmini (üst sınır = dizi uzunluğu); sınırsız uçlar diziye genişletilir.
fn bolge_uzunlugu(region: &Region, dizi_uzunlugu: u64) -> u64 {
    let bas = match region.start() {
        Bound::Included(p) | Bound::Excluded(p) => p.get() as u64,
        Bound::Unbounded => 1,
    };
    let bit = match region.end() {
        Bound::Included(p) | Bound::Excluded(p) => p.get() as u64,
        Bound::Unbounded => dizi_uzunlugu,
    };
    bit.saturating_sub(bas).saturating_add(1).min(dizi_uzunlugu)
}

fn indeks_yok_hatasi(yol: &Path, fai_yol: &Path) -> ErrorReport {
    ErrorReport::new(
        "FASTA indeksi (.fai) bulunamadı",
        format!(
            "'{}' için rastgele bölge erişimi indeks gerektirir; '{}' yok",
            yol.display(),
            fai_yol.display()
        ),
        "İndeksi şimdi oluşturabilirim (samtools faidx eşdeğeri)",
    )
    .with_eylem("İndeks oluştur")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn yaz(ad: &str, icerik: &[u8]) -> PathBuf {
        let mut yol = std::env::temp_dir();
        yol.push(format!("biocraft_fa_{}_{ad}", std::process::id()));
        File::create(&yol).unwrap().write_all(icerik).unwrap();
        yol
    }

    #[test]
    fn fasta_indeks_olustur_ve_bolge_oku() {
        // İki dizi; her dizi tek satır → fai üretilebilir.
        let fa = yaz("ref.fasta", b">sq0\nACGTACGTAC\n>sq1\nTTTTGGGGCC\n");
        let _ = std::fs::remove_file(fai_yolu(&fa));

        // İndeks yoksa açılış net hata verir.
        assert!(FastaOkuyucu::ac(&fa).is_err());

        // İndeks oluştur → açılır.
        let fai = fai_olustur(&fa).unwrap();
        assert!(fai.exists());
        let okuyucu = FastaOkuyucu::ac(&fa).unwrap();
        assert_eq!(okuyucu.dizi_adlari(), vec!["sq0", "sq1"]);

        // Bölge sorgusu (1-tabanlı, kapsayıcı): sq0:1-4 = "ACGT".
        let parca = okuyucu
            .bolge("sq0:1-4", &BellekButcesi::sinirsiz())
            .unwrap();
        assert_eq!(parca.ad, "sq0");
        assert_eq!(parca.diziler, b"ACGT");

        // sq1:5-8 = "GGGG".
        let p2 = okuyucu
            .bolge("sq1:5-8", &BellekButcesi::sinirsiz())
            .unwrap();
        assert_eq!(p2.diziler, b"GGGG");

        let _ = std::fs::remove_file(fai_yolu(&fa));
        let _ = std::fs::remove_file(&fa);
    }

    #[test]
    fn bolge_butce_asiminda_reddeder() {
        let fa = yaz("ref2.fasta", b">sq0\nACGTACGTAC\n");
        let _ = fai_olustur(&fa).unwrap();
        let okuyucu = FastaOkuyucu::ac(&fa).unwrap();
        // 10 baytlık bölge, 4 baytlık bütçe → red.
        let hata = okuyucu
            .bolge("sq0:1-10", &BellekButcesi::yeni(4))
            .unwrap_err();
        assert_eq!(hata.ne_oldu, "Bellek bütçesi aşıldı");
        let _ = std::fs::remove_file(fai_yolu(&fa));
        let _ = std::fs::remove_file(&fa);
    }

    #[test]
    fn fasta_akis_kayitlari_sayar() {
        let fa = yaz("multi.fasta", b">a\nACGT\n>b\nGGCC\n>c\nTTTT\n");
        let mut adlar = vec![];
        let sayi = fasta_akis(&fa, |ad, _| adlar.push(ad.to_string())).unwrap();
        assert_eq!(sayi, 3);
        assert_eq!(adlar, vec!["a", "b", "c"]);
        let _ = std::fs::remove_file(&fa);
    }

    #[test]
    fn fastq_kalite_ozeti() {
        // İki read; kalite 'I'=Phred 40, '#'=Phred 2.
        let fq = yaz("r.fastq", b"@r1\nACGT\n+\nIIII\n@r2\nGG\n+\n##\n");
        let ozet = fastq_akis(&fq, |_, _| {}).unwrap();
        assert_eq!(ozet.kayit_sayisi, 2);
        assert_eq!(ozet.toplam_baz, 6);
        assert_eq!(ozet.max_phred, 40); // 'I' = 73 - 33
        assert_eq!(ozet.min_phred, 2); // '#' = 35 - 33
        let _ = std::fs::remove_file(&fq);
    }
}
