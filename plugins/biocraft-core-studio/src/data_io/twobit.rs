//! ÇE-01 — **UCSC 2bit** referans dizi okuma — **saf-Rust** (yeni dış bağımlılık yok).
//!
//! 2bit, referans genomları yoğun saklayan ikili bir UCSC formatıdır: her baz **2 bit**
//! (`T=0, C=1, A=2, G=3`), 'N' blokları ve yumuşak-maske (lowercase) blokları ayrı listelerde.
//! İçinde **dizi başına ofset indeksi** olduğundan rastgele bölge erişimi **out-of-core**
//! yapılır (MK-09): yalnızca istenen aralığın paketli baytları okunur, tüm dizi RAM'e ALINMAZ.
//!
//! Biçim, iyi belgeli ve basit-ikili olduğundan elle (güvenle) yazılır; karmaşık ikili indeksli
//! BigWig'in aksine doğruluk-kritik B+/R-ağacı içermez.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Bound;
use std::path::{Path, PathBuf};

use noodles::core::Region;

use biocraft_sdk::biocraft_types::ErrorReport;

use super::budget::BellekButcesi;

const IMZA_LE: u32 = 0x1A41_2743;

/// 2-bit kod → baz tablosu (UCSC: T,C,A,G).
const BAZLAR: [u8; 4] = [b'T', b'C', b'A', b'G'];

/// İndeksli 2bit okuyucu — yol + dizi adı→ofset indeksi taşır.
pub struct TwoBitOkuyucu {
    yol: PathBuf,
    big_endian: bool,
    diziler: Vec<(String, u64)>, // (ad, kayıt ofseti)
}

/// Bir bölge sorgusunun sonucu — yalnızca istenen alt-dizi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwoBitParca {
    /// Dizi adı.
    pub ad: String,
    /// Sorgulanan bölge metni.
    pub bolge: String,
    /// İstenen alt-dizinin baytları (N → 'N', maske → küçük harf).
    pub diziler: Vec<u8>,
}

impl TwoBitOkuyucu {
    /// 2bit dosyasını açar; başlık + dizi indeksini (ad→ofset) okur.  Tüm dosya açılmaz.
    pub fn ac(yol: &Path) -> Result<Self, ErrorReport> {
        let mut f = File::open(yol).map_err(|e| io_hatasi(yol, &e))?;

        let mut imza = [0u8; 4];
        f.read_exact(&mut imza).map_err(|e| bicim_hatasi(yol, &e))?;
        let imza_le = u32::from_le_bytes(imza);
        let imza_be = u32::from_be_bytes(imza);
        let big_endian = if imza_le == IMZA_LE {
            false
        } else if imza_be == IMZA_LE {
            true
        } else {
            return Err(ErrorReport::new(
                "2bit imzası tanınmadı",
                format!(
                    "'{}' geçerli bir 2bit dosyası değil (sihirli imza uyuşmuyor)",
                    yol.display()
                ),
                "Dosyanın gerçek bir UCSC .2bit olduğundan emin olun",
            ));
        };

        let _version = oku_u32(&mut f, big_endian, yol)?;
        let dizi_sayisi = oku_u32(&mut f, big_endian, yol)? as usize;
        let _reserved = oku_u32(&mut f, big_endian, yol)?;

        // Aşırı büyük sayıma karşı koruma (bozuk dosya → bellek patlatma yok).
        if dizi_sayisi > 1_000_000 {
            return Err(bozuk_hatasi(yol, "olağandışı dizi sayısı"));
        }

        let mut diziler = Vec::with_capacity(dizi_sayisi);
        for _ in 0..dizi_sayisi {
            let ad_boyu = oku_u8(&mut f, yol)? as usize;
            let mut ad = vec![0u8; ad_boyu];
            f.read_exact(&mut ad).map_err(|e| bicim_hatasi(yol, &e))?;
            let ofset = oku_u32(&mut f, big_endian, yol)? as u64;
            diziler.push((String::from_utf8_lossy(&ad).into_owned(), ofset));
        }

        Ok(Self {
            yol: yol.to_path_buf(),
            big_endian,
            diziler,
        })
    }

    /// İndeksteki dizi adları.
    pub fn dizi_adlari(&self) -> Vec<String> {
        self.diziler.iter().map(|(a, _)| a.clone()).collect()
    }

    /// Bir dizinin (kromozom) toplam baz uzunluğu.
    pub fn dizi_uzunlugu(&self, ad: &str) -> Result<usize, ErrorReport> {
        let ofset = self.ofset(ad)?;
        let mut f = File::open(&self.yol).map_err(|e| io_hatasi(&self.yol, &e))?;
        f.seek(SeekFrom::Start(ofset))
            .map_err(|e| io_hatasi(&self.yol, &e))?;
        Ok(oku_u32(&mut f, self.big_endian, &self.yol)? as usize)
    }

    /// Bir bölgeyi (`s1:5-8`) **indeksli** çeker: yalnızca istenen aralığın paketli baytları
    /// okunur (out-of-core).  Bütçe önce denetlenir (İP-08).  N → 'N', maske → küçük harf.
    pub fn bolge(&self, bolge: &str, butce: &BellekButcesi) -> Result<TwoBitParca, ErrorReport> {
        let region: Region = bolge.parse().map_err(|_| gecersiz_bolge(bolge))?;
        let ad = region.name().to_string();
        let ofset = self.ofset(&ad)?;

        let mut f = File::open(&self.yol).map_err(|e| io_hatasi(&self.yol, &e))?;
        f.seek(SeekFrom::Start(ofset))
            .map_err(|e| io_hatasi(&self.yol, &e))?;

        let dna_size = oku_u32(&mut f, self.big_endian, &self.yol)? as usize;
        let n_bloklar = oku_bloklar(&mut f, self.big_endian, &self.yol)?;
        let maske_bloklar = oku_bloklar(&mut f, self.big_endian, &self.yol)?;
        let _reserved = oku_u32(&mut f, self.big_endian, &self.yol)?;
        // Paketli DNA bu konumdan başlar.
        let dna_baslangic = f.stream_position().map_err(|e| io_hatasi(&self.yol, &e))?;

        // Bölge → 0-tabanlı yarı-açık [s, e).
        let (s, e) = bolge_araligi(&region, dna_size);
        if s >= e {
            return Ok(TwoBitParca {
                ad,
                bolge: bolge.to_string(),
                diziler: Vec::new(),
            });
        }
        let uzunluk = e - s;
        butce.kontrol(uzunluk as u64)?;

        // Yalnızca gereken paketli baytları oku: bayt [s/4, ceil(e/4)).
        let bayt_bas = s / 4;
        let bayt_bit = e.div_ceil(4);
        let bayt_say = bayt_bit - bayt_bas;
        let mut paket = vec![0u8; bayt_say];
        f.seek(SeekFrom::Start(dna_baslangic + bayt_bas as u64))
            .map_err(|e| io_hatasi(&self.yol, &e))?;
        f.read_exact(&mut paket)
            .map_err(|e| bicim_hatasi(&self.yol, &e))?;

        // Çöz: her baz için 2-bit kod → baz.
        let mut diziler = Vec::with_capacity(uzunluk);
        for i in s..e {
            let yerel_bayt = i / 4 - bayt_bas;
            let kaydir = (3 - (i % 4)) * 2;
            let kod = (paket[yerel_bayt] >> kaydir) & 0b11;
            diziler.push(BAZLAR[kod as usize]);
        }

        // N blokları: ilgili konumları 'N' yap.
        bloklari_uygula(&n_bloklar, s, e, &mut diziler, |b| *b = b'N');
        // Maske blokları: küçük harf (yumuşak maske).
        bloklari_uygula(&maske_bloklar, s, e, &mut diziler, |b| {
            *b = b.to_ascii_lowercase()
        });

        Ok(TwoBitParca {
            ad,
            bolge: bolge.to_string(),
            diziler,
        })
    }

    fn ofset(&self, ad: &str) -> Result<u64, ErrorReport> {
        self.diziler
            .iter()
            .find(|(a, _)| a == ad)
            .map(|(_, o)| *o)
            .ok_or_else(|| {
                ErrorReport::new(
                    "Dizi bulunamadı",
                    format!("'{ad}' bu 2bit dosyasında tanımlı değil"),
                    "Geçerli bir dizi adı kullanın (dizi_adlari ile listeleyin)",
                )
            })
    }
}

// ─── Yardımcılar ────────────────────────────────────────────────────────────────

/// Bir blok listesini ((başlangıç, boyut) çiftleri) okur: sayım + başlangıçlar + boyutlar.
fn oku_bloklar(
    f: &mut File,
    big_endian: bool,
    yol: &Path,
) -> Result<Vec<(usize, usize)>, ErrorReport> {
    let sayi = oku_u32(f, big_endian, yol)? as usize;
    if sayi > 100_000_000 {
        return Err(bozuk_hatasi(yol, "olağandışı blok sayısı"));
    }
    let mut baslangic = Vec::with_capacity(sayi);
    for _ in 0..sayi {
        baslangic.push(oku_u32(f, big_endian, yol)? as usize);
    }
    let mut bloklar = Vec::with_capacity(sayi);
    for &bas in &baslangic {
        let boyut = oku_u32(f, big_endian, yol)? as usize;
        bloklar.push((bas, boyut));
    }
    Ok(bloklar)
}

/// `[s, e)` bölgesiyle örtüşen blok aralıklarına `eylem` uygular (yerel indeksle).
fn bloklari_uygula<F: Fn(&mut u8)>(
    bloklar: &[(usize, usize)],
    s: usize,
    e: usize,
    diziler: &mut [u8],
    eylem: F,
) {
    for &(b_bas, b_boy) in bloklar {
        let b_bit = b_bas + b_boy;
        let ust = b_bas.max(s);
        let alt = b_bit.min(e);
        for i in ust..alt {
            eylem(&mut diziler[i - s]);
        }
    }
}

/// Bölgeyi 0-tabanlı yarı-açık `[s, e)`'ye çevirir (1-tabanlı kapsayıcı girişi); diziye sıkıştırır.
fn bolge_araligi(region: &Region, dna_size: usize) -> (usize, usize) {
    let bas1 = match region.start() {
        Bound::Included(p) | Bound::Excluded(p) => p.get(),
        Bound::Unbounded => 1,
    };
    let bit1 = match region.end() {
        Bound::Included(p) | Bound::Excluded(p) => p.get(),
        Bound::Unbounded => dna_size,
    };
    let s = bas1.saturating_sub(1).min(dna_size);
    let e = bit1.min(dna_size);
    (s, e.max(s))
}

fn oku_u32(f: &mut File, big_endian: bool, yol: &Path) -> Result<u32, ErrorReport> {
    let mut b = [0u8; 4];
    f.read_exact(&mut b).map_err(|e| bicim_hatasi(yol, &e))?;
    Ok(if big_endian {
        u32::from_be_bytes(b)
    } else {
        u32::from_le_bytes(b)
    })
}

fn oku_u8(f: &mut File, yol: &Path) -> Result<u8, ErrorReport> {
    let mut b = [0u8; 1];
    f.read_exact(&mut b).map_err(|e| bicim_hatasi(yol, &e))?;
    Ok(b[0])
}

fn gecersiz_bolge(bolge: &str) -> ErrorReport {
    ErrorReport::new(
        "Geçersiz bölge ifadesi",
        format!("'{bolge}' bir bölgeye çözülemedi (beklenen: ad:başlangıç-bitiş)"),
        "Örn. 'chr1:1-100' veya yalnızca 'chr1' yazın",
    )
}

fn io_hatasi(yol: &Path, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "Dosya açılamadı",
        format!("'{}' 2bit okuma için açılamadı", yol.display()),
        "Dosya yolunu ve okuma iznini kontrol edin",
    )
    .with_teknik_detay(e.to_string())
}

fn bicim_hatasi(yol: &Path, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "2bit dosyası okunamadı",
        format!(
            "'{}' beklenenden kısa veya bozuk (2bit yapısı çözülemedi)",
            yol.display()
        ),
        "Dosya bütünlüğünü kontrol edin; gerekirse yeniden indirin",
    )
    .with_teknik_detay(e.to_string())
}

fn bozuk_hatasi(yol: &Path, neden: &str) -> ErrorReport {
    ErrorReport::new(
        "2bit dosyası bozuk",
        format!("'{}' içinde {neden} saptandı", yol.display()),
        "Dosyayı yeniden indirin; kaynağı doğrulayın",
    )
    .with_eylem("Yeniden indir")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Küçük bir 2bit dosyası kurar: tek dizi "s1" = 8 baz "ACGTACGT".
    fn ornek_2bit(ad: &str) -> PathBuf {
        let mut v: Vec<u8> = Vec::new();
        // Başlık (little-endian).
        v.extend_from_slice(&IMZA_LE.to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes()); // version
        v.extend_from_slice(&1u32.to_le_bytes()); // dizi sayısı
        v.extend_from_slice(&0u32.to_le_bytes()); // reserved
                                                  // İndeks: ad "s1", ofset = başlık(16) + indeks(1+2+4=7) = 23.
        v.push(2);
        v.extend_from_slice(b"s1");
        v.extend_from_slice(&23u32.to_le_bytes());
        // Kayıt @23: dnaSize=8, nBlock=0, maskBlock=0, reserved=0, DNA.
        v.extend_from_slice(&8u32.to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        // "ACGT" = A(10)C(01)G(11)T(00) = 0b10011100 = 0x9C; iki kez.
        v.push(0x9C);
        v.push(0x9C);

        let mut yol = std::env::temp_dir();
        yol.push(format!("biocraft_2bit_{}_{ad}", std::process::id()));
        File::create(&yol).unwrap().write_all(&v).unwrap();
        yol
    }

    #[test]
    fn acilir_dizi_listeler_ve_uzunluk() {
        let p = ornek_2bit("a.2bit");
        let r = TwoBitOkuyucu::ac(&p).unwrap();
        assert_eq!(r.dizi_adlari(), vec!["s1"]);
        assert_eq!(r.dizi_uzunlugu("s1").unwrap(), 8);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn bolge_cozme_dogru() {
        let p = ornek_2bit("b.2bit");
        let r = TwoBitOkuyucu::ac(&p).unwrap();
        // s1:1-4 = "ACGT".
        let p1 = r.bolge("s1:1-4", &BellekButcesi::sinirsiz()).unwrap();
        assert_eq!(p1.diziler, b"ACGT");
        // s1:5-8 = "ACGT".
        let p2 = r.bolge("s1:5-8", &BellekButcesi::sinirsiz()).unwrap();
        assert_eq!(p2.diziler, b"ACGT");
        // s1:3-6 = "GTAC" (kısmi bayt sınırı).
        let p3 = r.bolge("s1:3-6", &BellekButcesi::sinirsiz()).unwrap();
        assert_eq!(p3.diziler, b"GTAC");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn butce_asiminda_reddeder() {
        let p = ornek_2bit("c.2bit");
        let r = TwoBitOkuyucu::ac(&p).unwrap();
        let hata = r.bolge("s1:1-8", &BellekButcesi::yeni(4)).err().unwrap();
        assert_eq!(hata.ne_oldu, "Bellek bütçesi aşıldı");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn gecersiz_imza_reddeder() {
        let mut yol = std::env::temp_dir();
        yol.push(format!("biocraft_2bit_{}_bad", std::process::id()));
        File::create(&yol).unwrap().write_all(b"NOTABIT0").unwrap();
        let hata = TwoBitOkuyucu::ac(&yol).err().unwrap();
        assert_eq!(hata.ne_oldu, "2bit imzası tanınmadı");
        let _ = std::fs::remove_file(&yol);
    }
}
