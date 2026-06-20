//! Out-of-core akışlı işleme — MK-09.
//!
//! Büyük veri **asla** tek seferde RAM'e alınmaz; sabit boyutlu bir **pencere** (okuma
//! penceresi / sayfa) kadar bellek rezerve edilir ve dosya parça parça işlenir.  Böylece
//! 4 TB'lık bir dosya bile, yalnızca bir pencere kadar (örn. 1 MB) bellekle taranabilir.
//!
//! Rezervasyon orkestratörden alınır (MK-21): akış sırasında tepe bellek = bir pencere.
//! Eğer tüm dosyayı yüklemeye kalksaydık orkestratör bunu reddederdi (MK-22) — burada
//! bunu kanıtlayan testler var.
//!
//! İki yol sunulur:
//! - [`akisla_isle`] / [`dosya_akisla_isle`]: `Read` üzerinden tamponlu akış (taşınabilir).
//! - [`mmap_ile_isle`]: bellek-eşlemeli (mmap) dosya; pencere pencere dilimlenir
//!   (gereksiz kopya yok — spec'teki mmap yolu).

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use biocraft_types::ErrorReport;
use memmap2::Mmap;

use crate::orchestrator::{BellekBileseni, BellekOrkestratoru};

/// Akış yapılandırması — okuma penceresi (sayfa) boyutu.
#[derive(Debug, Clone, Copy)]
pub struct AkisAyar {
    /// Her adımda işlenecek pencere (bayt).  Tepe bellek ≈ bu değer.
    pub pencere_bayt: usize,
}

impl Default for AkisAyar {
    fn default() -> Self {
        // 1 MiB makul varsayılan: yeterince büyük (az sistem çağrısı), yeterince küçük (düşük tepe).
        Self {
            pencere_bayt: 1024 * 1024,
        }
    }
}

impl AkisAyar {
    /// Belirli pencere boyutuyla yapılandırma.
    pub fn pencere(pencere_bayt: usize) -> Self {
        Self { pencere_bayt }
    }
}

/// Bir akış işleminin özeti.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AkisOzet {
    /// İşlenen toplam bayt.
    pub toplam_bayt: u64,
    /// Kaç parça (pencere) işlendi.
    pub parca_sayisi: u64,
    /// İşlem boyunca tutulan tepe rezervasyon (bayt) = bir pencere.
    pub tepe_rezervasyon_bayt: u64,
}

/// **Bir `Read` kaynağını pencere pencere işle (MK-09).**
///
/// Yalnızca bir pencere kadar bellek rezerve edilir; `f` her parçada çağrılır.
/// Bütçe yetersizse (pencere bile sığmıyorsa) [`ErrorReport`] döner (panik yok).
pub fn akisla_isle<R, F>(
    mut kaynak: R,
    ayar: AkisAyar,
    ork: &BellekOrkestratoru,
    bilesen: BellekBileseni,
    mut f: F,
) -> Result<AkisOzet, ErrorReport>
where
    R: Read,
    F: FnMut(&[u8]),
{
    let pencere = ayar.pencere_bayt.max(1);

    // MK-09/MK-21: TÜM dosyayı değil, yalnızca bir pencere kadar rezerve et.
    let rez = ork.rezerve_et(bilesen, pencere as u64)?;
    let tepe = rez.bayt();

    let mut tampon = vec![0u8; pencere];
    let mut toplam: u64 = 0;
    let mut parca: u64 = 0;

    loop {
        let n = kaynak.read(&mut tampon).map_err(okuma_hatasi)?;
        if n == 0 {
            break;
        }
        f(&tampon[..n]);
        toplam += n as u64;
        parca += 1;
    }

    drop(rez); // pencere belleğini geri ver
    Ok(AkisOzet {
        toplam_bayt: toplam,
        parca_sayisi: parca,
        tepe_rezervasyon_bayt: tepe,
    })
}

/// **Bir dosyayı akış (stream) modunda işle.**  Dosya `BufReader` ile pencere pencere
/// okunur — tümü RAM'e alınmaz.
pub fn dosya_akisla_isle<P, F>(
    yol: P,
    ayar: AkisAyar,
    ork: &BellekOrkestratoru,
    bilesen: BellekBileseni,
    f: F,
) -> Result<AkisOzet, ErrorReport>
where
    P: AsRef<Path>,
    F: FnMut(&[u8]),
{
    let dosya = File::open(yol.as_ref()).map_err(|e| dosya_hatasi(yol.as_ref(), e))?;
    let pencere = ayar.pencere_bayt.max(1);
    let okuyucu = BufReader::with_capacity(pencere, dosya);
    akisla_isle(okuyucu, ayar, ork, bilesen, f)
}

/// **Bir dosyayı bellek-eşlemeli (mmap) işle — MK-09 mmap yolu.**
///
/// Dosya adres alanına eşlenir, ancak yalnızca bir pencere kadar bellek **rezerve** edilir
/// ve veri pencere pencere dilimlenir (gereksiz kopya yok).  İşletim sistemi sayfaları
/// talep üzerine yükler; bizim çalışma-kümesi taahhüdümüz bir penceredir.
pub fn mmap_ile_isle<P, F>(
    yol: P,
    ayar: AkisAyar,
    ork: &BellekOrkestratoru,
    bilesen: BellekBileseni,
    mut f: F,
) -> Result<AkisOzet, ErrorReport>
where
    P: AsRef<Path>,
    F: FnMut(&[u8]),
{
    let dosya = File::open(yol.as_ref()).map_err(|e| dosya_hatasi(yol.as_ref(), e))?;
    // SAFETY: dosya bu fonksiyon süresince açık tutulur; harita ondan önce düşürülür.
    let harita = unsafe { Mmap::map(&dosya) }.map_err(okuma_hatasi)?;

    let pencere = ayar.pencere_bayt.max(1);
    let rez = ork.rezerve_et(bilesen, pencere as u64)?; // sadece pencere kadar
    let tepe = rez.bayt();

    let mut toplam: u64 = 0;
    let mut parca: u64 = 0;
    for dilim in harita.chunks(pencere) {
        f(dilim);
        toplam += dilim.len() as u64;
        parca += 1;
    }

    drop(rez);
    Ok(AkisOzet {
        toplam_bayt: toplam,
        parca_sayisi: parca,
        tepe_rezervasyon_bayt: tepe,
    })
}

// ─── Hata yardımcıları (standart şema İP-16) ─────────────────────────────────

fn okuma_hatasi(e: std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "Veri okunamadı",
        "Dosya akışı sırasında bir giriş/çıkış (I/O) hatası oluştu.",
        "Dosyanın erişilebilir olduğundan emin olun ve tekrar deneyin.",
    )
    .with_teknik_detay(e.to_string())
}

fn dosya_hatasi(yol: &Path, e: std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "Dosya açılamadı",
        format!("'{}' açılırken hata oluştu.", yol.display()),
        "Dosya yolunu ve erişim izinlerini kontrol edin.",
    )
    .with_teknik_detay(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const MB: u64 = 1024 * 1024;
    const KB: usize = 1024;

    /// Test için geçici bir dosya oluşturur; düşürülünce siler.
    struct GeciciDosya {
        yol: std::path::PathBuf,
    }
    impl GeciciDosya {
        fn olustur(ad: &str, icerik: &[u8]) -> Self {
            let mut yol = std::env::temp_dir();
            yol.push(format!("biocraft_mem_test_{}_{}", std::process::id(), ad));
            let mut f = File::create(&yol).unwrap();
            f.write_all(icerik).unwrap();
            f.flush().unwrap();
            Self { yol }
        }
    }
    impl Drop for GeciciDosya {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.yol);
        }
    }

    #[test]
    fn ram_dan_buyuk_veri_parca_parca_islenir() {
        // MK-09: bütçe 1 MB; veri 8 MB. Tümünü yükleme reddedilirdi; akış başarmalı.
        let ork = BellekOrkestratoru::yeni(MB);

        // Önce kanıt: tüm dosyayı (8 MB) rezerve etmek reddedilir.
        assert!(ork.rezerve_et(BellekBileseni::VeriTabani, 8 * MB).is_err());

        let veri = vec![7u8; 8 * MB as usize];
        let mut goren: u64 = 0;
        let ozet = akisla_isle(
            &veri[..],
            AkisAyar::pencere(256 * KB),
            &ork,
            BellekBileseni::VeriTabani,
            |parca| goren += parca.iter().map(|&b| b as u64).sum::<u64>(),
        )
        .expect("akış başarılı olmalı");

        // Tüm baytlar işlendi.
        assert_eq!(ozet.toplam_bayt, 8 * MB);
        assert_eq!(goren, 8 * MB * 7);
        // Tepe rezervasyon yalnızca bir pencere kadar.
        assert_eq!(ozet.tepe_rezervasyon_bayt, 256 * KB as u64);
        assert!(ozet.parca_sayisi >= 32);
        // Akış bitince bellek tamamen geri verildi.
        assert_eq!(ork.rezerve_edilen(), 0);
    }

    #[test]
    fn pencere_butceye_sigmazsa_reddedilir_panik_yok() {
        // Bütçe pencereden küçük → akış başlamadan reddetme (MK-22).
        let ork = BellekOrkestratoru::yeni(100 * 1024); // 100 KB
        let veri = vec![0u8; 1024];
        let sonuc = akisla_isle(
            &veri[..],
            AkisAyar::pencere(1024 * 1024), // 1 MB pencere > 100 KB bütçe
            &ork,
            BellekBileseni::Diger("t".into()),
            |_| {},
        );
        assert!(sonuc.is_err());
    }

    #[test]
    fn dosya_akisla_isle_tum_dosyayi_okur() {
        let ork = BellekOrkestratoru::yeni(4 * MB);
        let boyut = 3 * MB as usize + 123; // pencere katı olmayan boyut
        let gecici = GeciciDosya::olustur("akis", &vec![1u8; boyut]);

        let mut sayac: u64 = 0;
        let ozet = dosya_akisla_isle(
            &gecici.yol,
            AkisAyar::pencere(512 * KB),
            &ork,
            BellekBileseni::VeriTabani,
            |parca| sayac += parca.len() as u64,
        )
        .unwrap();

        assert_eq!(ozet.toplam_bayt, boyut as u64);
        assert_eq!(sayac, boyut as u64);
        assert_eq!(ork.rezerve_edilen(), 0);
    }

    #[test]
    fn mmap_ile_isle_tum_dosyayi_pencere_pencere_okur() {
        // MK-09 mmap yolu: tepe rezervasyon yine bir pencere.
        let ork = BellekOrkestratoru::yeni(2 * MB);
        let boyut = 5 * MB as usize + 7;
        let gecici = GeciciDosya::olustur("mmap", &vec![9u8; boyut]);

        let mut sayac: u64 = 0;
        let ozet = mmap_ile_isle(
            &gecici.yol,
            AkisAyar::pencere(MB as usize),
            &ork,
            BellekBileseni::VeriTabani,
            |parca| sayac += parca.len() as u64,
        )
        .unwrap();

        assert_eq!(ozet.toplam_bayt, boyut as u64);
        assert_eq!(sayac, boyut as u64);
        assert_eq!(ozet.tepe_rezervasyon_bayt, MB);
        assert_eq!(ork.rezerve_edilen(), 0);
    }

    #[test]
    fn olmayan_dosya_temiz_hata_dondurur() {
        let ork = BellekOrkestratoru::yeni(MB);
        let sonuc = dosya_akisla_isle(
            "kesinlikle_olmayan_dosya_xyz.dat",
            AkisAyar::default(),
            &ork,
            BellekBileseni::VeriTabani,
            |_| {},
        );
        assert!(sonuc.is_err());
        let hata = sonuc.unwrap_err();
        assert!(!hata.ne_oldu.is_empty());
        assert!(hata.teknik_detay.is_some());
    }
}
