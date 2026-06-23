//! ÇE-02 — **LOD (ayrıntı düzeyi) + culling + downsampling**.
//!
//! MK-04 (akıcılık / kare bütçesi) ve MK-09 (out-of-core / yalnız görünen pencere) bu modülde
//! buluşur:
//! * **Culling:** yalnızca görünen bölgeyle örtüşen öğeler çizime girer (ekran-dışı çizilmez).
//! * **LOD:** yakınlaşma seviyesine (bp/piksel) göre baz / öğe / özet düzeyi seçilir.
//! * **Downsampling:** öğe sayısı kare bütçesini aşarsa **deterministik** seyreltme (özet izi
//!   ayrıca yoğunluğu korur) → yoğun bölgede bile akıcılık.
//! * **Kapsama binleme + read yığını (pileup):** hizalama izinin out-of-core özetleri.
//!
//! Hepsi saf fonksiyondur; gerçek dosya okuma `data_io` okuyucularıyla yapılır, sonuç buraya
//! (bellek-içi öğe listeleri) verilir.

use super::canvas::{GenomBolge, Tuval};

/// 60 FPS hedefi (MK-04): bir karenin bütçesi (ms).
pub const HEDEF_KARE_SURESI_MS: f32 = 1000.0 / 60.0;

/// Bir izde tek karede çizilecek **azami öğe** (read/özellik) sayısı varsayılanı.  Aşılırsa
/// downsampling/özet devreye girer (kare bütçesini korur).  Tuval genişliğiyle de ölçeklenir.
pub const VARSAYILAN_OGE_BUTCESI: usize = 4_000;

/// Çizim ayrıntı düzeyi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LodSeviyesi {
    /// Çok yakın: tek tek bazlar/harfler görünür (bp başına birkaç piksel).
    Baz,
    /// Orta: öğeler (read/özellik) tek tek kutu olarak çizilir.
    Oge,
    /// Uzak/yoğun: tek tek çizmek yerine yoğunluk/özet (histogram) çizilir.
    Ozet,
}

/// Yakınlaşma (bp/piksel) ve öğe sayısına göre LOD seçer.
///
/// * `bp_per_piksel <= 1/EŞIK_BAZ_PIKSEL` → her baz ≥ birkaç piksel → **Baz**.
/// * Öğe sayısı bütçeyi aşıyorsa → **Özet**.
/// * Aksi halde → **Öğe**.
pub fn lod_sec(bp_per_piksel: f64, oge_sayisi: usize, oge_butcesi: usize) -> LodSeviyesi {
    // Bir baz en az ~8 piksel ise harf düzeyine geç (bp/px ≤ 0.125).
    const ESIK_BAZ_PIKSEL: f64 = 8.0;
    if bp_per_piksel <= 1.0 / ESIK_BAZ_PIKSEL {
        LodSeviyesi::Baz
    } else if oge_sayisi > oge_butcesi {
        LodSeviyesi::Ozet
    } else {
        LodSeviyesi::Oge
    }
}

/// Bir öğenin genomik ayak izini (1-tabanlı kapsayıcı `[bas, bit]`) veren özellik.
/// Culling/binleme/yığın bu trait üzerinden çalışır (read, özellik, varyant ortak).
pub trait Konumlu {
    /// 1-tabanlı başlangıç.
    fn bas(&self) -> u64;
    /// 1-tabanlı bitiş (kapsayıcı).
    fn bit(&self) -> u64;
}

// Referans da konumludur → görünür parça dilimleri (`&[&T]`) yığın/binleme fonksiyonlarına
// kopyalanmadan verilebilir.
impl<T: Konumlu> Konumlu for &T {
    fn bas(&self) -> u64 {
        (**self).bas()
    }
    fn bit(&self) -> u64 {
        (**self).bit()
    }
}

/// Görünen bölgeyle örtüşen öğelerin indekslerini döndürür (culling; ekran-dışı atılır).
pub fn gorunur_indeksler<T: Konumlu>(ogeler: &[T], bolge: &GenomBolge) -> Vec<usize> {
    ogeler
        .iter()
        .enumerate()
        .filter(|(_, o)| bolge.ortusur(o.bas(), o.bit()))
        .map(|(i, _)| i)
        .collect()
}

/// `indeksler`'i `butce`'ye **deterministik** olarak seyreltir (eşit aralıklı örnekleme →
/// dağılım korunur, görsel boşluk oluşmaz).  Bütçe altındaysa aynen döner.
pub fn seyrelt(indeksler: &[usize], butce: usize) -> Vec<usize> {
    if butce == 0 {
        return Vec::new();
    }
    if indeksler.len() <= butce {
        return indeksler.to_vec();
    }
    let n = indeksler.len();
    (0..butce)
        .map(|k| {
            // Her bütçe yuvasını kaynak aralığın ortasına eşle (eşit aralıklı, deterministik).
            let idx = (k * n + n / 2) / butce;
            indeksler[idx.min(n - 1)]
        })
        .collect()
}

/// [`seyrelt`] gibi bütçeye seyreltir ama `korunan` kümesindeki indeksleri **her zaman tutar**
/// (varyant/önemli read'ler gizlenmez; yalnız fazlalık seyreltilir).  Önce korunanlar alınır,
/// kalan bütçe geri kalanlardan deterministik örneklenir.  Çıktı `indeksler`'deki sırayı korur.
pub fn seyrelt_koruyarak(indeksler: &[usize], butce: usize, korunan: &[usize]) -> Vec<usize> {
    if butce == 0 {
        return Vec::new();
    }
    if indeksler.len() <= butce {
        return indeksler.to_vec();
    }
    let korunan_kume: std::collections::HashSet<usize> = korunan.iter().copied().collect();

    // Korunanları (görünür olanları) ve geri kalanları ayır.
    let mut korunan_gorunur: Vec<usize> = Vec::new();
    let mut diger: Vec<usize> = Vec::new();
    for &i in indeksler {
        if korunan_kume.contains(&i) {
            korunan_gorunur.push(i);
        } else {
            diger.push(i);
        }
    }

    // Korunanlar bütçeyi tek başına aşıyorsa onları da deterministik seyrelt (yine de hepsini
    // değil; aşırı yoğun varyant bölgesinde bile akıcılık korunur — MK-04).
    if korunan_gorunur.len() >= butce {
        return seyrelt(&korunan_gorunur, butce);
    }

    let kalan_butce = butce - korunan_gorunur.len();
    let secilen_diger: std::collections::HashSet<usize> =
        seyrelt(&diger, kalan_butce).into_iter().collect();

    // Orijinal sırayı koruyarak birleştir.
    indeksler
        .iter()
        .copied()
        .filter(|i| korunan_kume.contains(i) || secilen_diger.contains(i))
        .collect()
}

/// Tuval genişliğine ölçeklenmiş öğe bütçesi: dar tuvalde daha az, geniş tuvalde daha çok öğe
/// (piksel başına ~`taban_carpan` öğe), `taban` ile sınırlı.  Kare bütçesinin (MK-04) pratik karşılığı.
pub fn oge_butcesi(tuval: &Tuval, taban: usize) -> usize {
    let pikselle = (tuval.genislik_px as usize).saturating_mul(4);
    pikselle.clamp(256, taban)
}

// ─── Kapsama (coverage) binleme — out-of-core özet ──────────────────────────────

/// Görünen bölgeyi `kova_sayisi` eşit kovaya böler ve her kovaya düşen okuma derinliğini
/// (örtüşme sayısı) sayar.  Kova ekran piksel-sütunuyla eşleşir → histogram doğrudan çizilir.
/// Tüm dosya değil, yalnız verilen (görünen pencereye ait) okumalar işlenir (MK-09).
pub fn kapsama_binle<T: Konumlu>(ogeler: &[T], bolge: &GenomBolge, kova_sayisi: usize) -> Vec<u32> {
    let kova_sayisi = kova_sayisi.max(1);
    let mut kovalar = vec![0u32; kova_sayisi];
    let bolge_bas = bolge.baslangic;
    let uzun = bolge.uzunluk() as f64;
    for o in ogeler {
        // Öğenin bölgeyle kesişen kısmı.
        let bas = o.bas().max(bolge.baslangic);
        let bit = o.bit().min(bolge.bitis);
        if bas > bit {
            continue;
        }
        let ilk_kova = (((bas - bolge_bas) as f64 / uzun) * kova_sayisi as f64).floor() as usize;
        let son_kova = (((bit - bolge_bas) as f64 / uzun) * kova_sayisi as f64).floor() as usize;
        let (ilk, son) = (ilk_kova.min(kova_sayisi - 1), son_kova.min(kova_sayisi - 1));
        kovalar[ilk..=son].iter_mut().for_each(|c| *c += 1);
    }
    kovalar
}

// ─── Read yığını (pileup) yerleşimi ─────────────────────────────────────────────

/// Bir okumanın yerleştirildiği yığın satırı (0 = en üst).  `read_indeks` girdi listesindeki
/// konumdur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YiginYer {
    /// Girdi öğe listesindeki indeks.
    pub oge_indeksi: usize,
    /// Atanan satır (0 tabanlı; aşağı doğru artar).
    pub satir: usize,
}

/// Okumaları çakışmayacak satırlara yerleştirir (IGV "collapsed" yığını): her okuma, sağ kenarı
/// kendi sol kenarından küçük olan ilk uygun satıra konur (greedy, soldan sağa).  `bp_bosluk`
/// iki okuma arasında istenen en küçük boşluktur (görsel ayrım).  Döndürülen satır sayısı yığının
/// yüksekliğini belirler.  Öğeler `bas`'a göre sıralı varsayılır; değilse içeride sıralanır.
pub fn yigin_yerlesimi<T: Konumlu>(ogeler: &[T], bp_bosluk: u64) -> (Vec<YiginYer>, usize) {
    // (bas, bit, orijinal_indeks) sırala.
    let mut sirali: Vec<(u64, u64, usize)> = ogeler
        .iter()
        .enumerate()
        .map(|(i, o)| (o.bas(), o.bit(), i))
        .collect();
    sirali.sort_by_key(|&(bas, _, _)| bas);

    // Her satırın o ana kadarki en sağ dolu pozisyonu.
    let mut satir_sonu: Vec<u64> = Vec::new();
    let mut yerler = Vec::with_capacity(ogeler.len());

    for (bas, bit, idx) in sirali {
        // Bu okumanın sığacağı ilk satırı bul.
        let mut yerlesti = None;
        for (s, son) in satir_sonu.iter_mut().enumerate() {
            if bas > son.saturating_add(bp_bosluk) {
                *son = bit;
                yerlesti = Some(s);
                break;
            }
        }
        let satir = match yerlesti {
            Some(s) => s,
            None => {
                satir_sonu.push(bit);
                satir_sonu.len() - 1
            }
        };
        yerler.push(YiginYer {
            oge_indeksi: idx,
            satir,
        });
    }

    let satir_sayisi = satir_sonu.len();
    (yerler, satir_sayisi)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test öğesi.
    struct O(u64, u64);
    impl Konumlu for O {
        fn bas(&self) -> u64 {
            self.0
        }
        fn bit(&self) -> u64 {
            self.1
        }
    }

    #[test]
    fn lod_secimi() {
        // Çok yakın (0.05 bp/px ≈ 20 px/baz) → Baz.
        assert_eq!(lod_sec(0.05, 10, 1000), LodSeviyesi::Baz);
        // Orta yakınlık, az öğe → Öğe.
        assert_eq!(lod_sec(2.0, 10, 1000), LodSeviyesi::Oge);
        // Çok öğe → Özet.
        assert_eq!(lod_sec(2.0, 5000, 1000), LodSeviyesi::Ozet);
    }

    #[test]
    fn culling_ekran_disini_atar() {
        let ogeler = [O(1, 100), O(500, 600), O(2000, 2100)];
        let bolge = GenomBolge::yeni("chr1", 400, 1000).unwrap();
        let gorunur = gorunur_indeksler(&ogeler, &bolge);
        assert_eq!(gorunur, vec![1], "yalnız 500-600 örtüşür");
    }

    #[test]
    fn seyreltme_deterministik_ve_butceye_uyar() {
        let idx: Vec<usize> = (0..1000).collect();
        let s = seyrelt(&idx, 100);
        assert_eq!(s.len(), 100);
        // Deterministik: tekrar aynı sonuç.
        assert_eq!(s, seyrelt(&idx, 100));
        // Artan/dağılmış (ilk küçük, son büyük).
        assert!(s[0] < s[99]);
        assert!(s[99] >= 900, "üst uçtan örnek korunmalı");
        // Bütçe üstündeyse aynen döner.
        assert_eq!(seyrelt(&idx[0..50], 100).len(), 50);
    }

    #[test]
    fn seyrelt_koruyarak_onemlileri_tutar() {
        let idx: Vec<usize> = (0..1000).collect();
        // 7 ve 993 "önemli" (varyant) → her zaman görünür.
        let korunan = [7usize, 993];
        let s = seyrelt_koruyarak(&idx, 100, &korunan);
        assert_eq!(s.len(), 100);
        assert!(s.contains(&7), "önemli read korunmalı");
        assert!(s.contains(&993), "önemli read korunmalı");
        // Deterministik.
        assert_eq!(s, seyrelt_koruyarak(&idx, 100, &korunan));
        // Sıra korunur (artan).
        assert!(s.windows(2).all(|w| w[0] < w[1]));

        // Bütçe altındaysa aynen döner.
        assert_eq!(seyrelt_koruyarak(&idx[0..50], 100, &korunan).len(), 50);
    }

    #[test]
    fn kapsama_binleme() {
        // chr1:1-100, 10 kova (her kova 10 bp).  İki okuma: 1-50 ve 30-100.
        let ogeler = [O(1, 50), O(30, 100)];
        let bolge = GenomBolge::yeni("chr1", 1, 100).unwrap();
        let kovalar = kapsama_binle(&ogeler, &bolge, 10);
        assert_eq!(kovalar.len(), 10);
        // İlk kova (1-10): yalnız ilk okuma → derinlik 1.
        assert_eq!(kovalar[0], 1);
        // Ortadaki kova (31-40): iki okuma → derinlik 2.
        assert_eq!(kovalar[3], 2);
        // Son kova (91-100): yalnız ikinci okuma → derinlik 1.
        assert_eq!(kovalar[9], 1);
    }

    #[test]
    fn yigin_cakismayan_satirlar() {
        // Üç okuma: 1-100, 50-150 (ilkle çakışır), 200-300 (ilk satıra sığar).
        let ogeler = [O(1, 100), O(50, 150), O(200, 300)];
        let (yerler, satir_sayisi) = yigin_yerlesimi(&ogeler, 1);
        assert_eq!(satir_sayisi, 2, "çakışan ikinci okuma yeni satıra iner");

        // Her okumanın satırını indeksle bul.
        let satir = |idx: usize| yerler.iter().find(|y| y.oge_indeksi == idx).unwrap().satir;
        assert_eq!(satir(0), 0);
        assert_eq!(satir(1), 1, "50-150, 1-100 ile çakışır → satır 1");
        assert_eq!(satir(2), 0, "200-300, satır 0'a sığar");
    }

    #[test]
    fn oge_butcesi_tuval_ile_olcekli() {
        let bolge = GenomBolge::yeni("chr1", 1, 1000).unwrap();
        let dar = Tuval::yeni(50.0, bolge.clone());
        let genis = Tuval::yeni(2000.0, bolge);
        // Dar tuval alt sınıra (256) sıkışır; geniş tuval tabanla sınırlı.
        assert_eq!(oge_butcesi(&dar, VARSAYILAN_OGE_BUTCESI), 256);
        assert_eq!(
            oge_butcesi(&genis, VARSAYILAN_OGE_BUTCESI),
            VARSAYILAN_OGE_BUTCESI
        );
    }
}
