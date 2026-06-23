//! ÇE-02 — Genom tuvali: **koordinat sistemi + görünüm dönüşümü** (genom ↔ ekran).
//!
//! Tuval, görünen bir [`GenomBolge`]'yi (kromozom + 1-tabanlı kapsayıcı aralık) yatay piksel
//! uzayına eşler.  Tüm tarayıcı (cetvel, izler, isabet testi, çizim listesi) **yalnızca** bu
//! dönüşümü kullanır → *tek koordinat doğruluk kaynağı*.  Cetvel ile veri koordinatları aynı
//! eşlemeden geçtiği için "koordinat kayması" (muhtemel hata) yapısal olarak engellenir.
//!
//! ## Koordinat sözleşmesi (kritik)
//! * **1-tabanlı, kapsayıcı** aralık (`baslangic..=bitis`) — IGV görüntü ölçeği +
//!   [`AnotasyonKaydi`](crate::data_io::AnotasyonKaydi) (1-tabanlı) +
//!   [`HizalamaKaydi.konum`](crate::data_io::HizalamaKaydi) (1-tabanlı) ile **birebir** aynı.
//!   BED'in 0-tabanlı yarı-açık ham gösterimi okuyucu (`data_io`) katmanında zaten 1-tabanlıya
//!   çevrilir; tuval daima 1-tabanlı görür → dönüşüm tek yerde, kayma yok.
//! * `genom_to_ekran(pos)` bir bazın **sol** kenarının x'ini verir; `[bas..=bit]` aralığının
//!   sağ kenarı, `bit` bazının **sonudur** (yani `bit+1`'in sol kenarı).

use biocraft_sdk::biocraft_types::ErrorReport;

/// Tarayıcıda görünen genomik bölge: kromozom + **1-tabanlı kapsayıcı** aralık.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenomBolge {
    /// Kromozom/kontig adı (referans dizilerinden biri).
    pub kromozom: String,
    /// 1-tabanlı başlangıç (≥ 1).
    pub baslangic: u64,
    /// 1-tabanlı bitiş (kapsayıcı; ≥ `baslangic`).
    pub bitis: u64,
}

impl GenomBolge {
    /// Bir bölge kurar.  `baslangic ≥ 1` ve `baslangic ≤ bitis` olmalıdır; aksi halde net hata.
    pub fn yeni(
        kromozom: impl Into<String>,
        baslangic: u64,
        bitis: u64,
    ) -> Result<Self, ErrorReport> {
        if baslangic < 1 {
            return Err(gecersiz_bolge_hatasi(
                "başlangıç 1-tabanlı olmalı (en küçük 1)",
            ));
        }
        if bitis < baslangic {
            return Err(gecersiz_bolge_hatasi("bitiş başlangıçtan küçük olamaz"));
        }
        Ok(Self {
            kromozom: kromozom.into(),
            baslangic,
            bitis,
        })
    }

    /// Görünen baz sayısı (kapsayıcı aralık uzunluğu): `bitis - baslangic + 1`.
    pub fn uzunluk(&self) -> u64 {
        self.bitis - self.baslangic + 1
    }

    /// Aralığın orta noktası (1-tabanlı; kaydırma/yakınlaştırma odağı için).
    pub fn merkez(&self) -> u64 {
        self.baslangic + (self.uzunluk() - 1) / 2
    }

    /// `pos` (1-tabanlı) bu bölgenin içinde mi?
    pub fn kapsar(&self, pos: u64) -> bool {
        pos >= self.baslangic && pos <= self.bitis
    }

    /// `[bas..=bit]` aralığı bu bölge ile (kapsayıcı) örtüşüyor mu? (culling testi.)
    pub fn ortusur(&self, bas: u64, bit: u64) -> bool {
        bas <= self.bitis && bit >= self.baslangic
    }

    /// `merkez` çevresinde `uzunluk` bp'lik bir bölge kurar (yakınlaştırma/"bölgeye git" için).
    /// `uzunluk` en az 1'dir; sol kenar 1'in altına taşarsa 1'e sabitlenir.
    pub fn merkezli(kromozom: impl Into<String>, merkez: u64, uzunluk: u64) -> Self {
        let uzunluk = uzunluk.max(1);
        let yari = (uzunluk - 1) / 2;
        let bas = merkez.saturating_sub(yari).max(1);
        let bit = bas + uzunluk - 1;
        Self {
            kromozom: kromozom.into(),
            baslangic: bas,
            bitis: bit,
        }
    }

    /// Bölgeyi `[1, kromozom_uzunlugu]` sınırlarına sıkıştırır (uzunluğu **korur**: pencere
    /// sınıra dayanırsa içeri kaydırır).  `kromozom_uzunlugu` bilinmiyorsa (`None`) yalnız sol
    /// kenar 1'e sabitlenir.
    pub fn sinirla(&self, kromozom_uzunlugu: Option<u64>) -> GenomBolge {
        let uzun = self.uzunluk();
        let mut bas = self.baslangic.max(1);
        let mut bit = bas + uzun - 1;
        if let Some(maks) = kromozom_uzunlugu {
            let maks = maks.max(1);
            if uzun >= maks {
                // Pencere tüm kromozomdan büyük → kromozomu kapla.
                bas = 1;
                bit = maks;
            } else if bit > maks {
                bit = maks;
                bas = bit - uzun + 1;
            }
        }
        GenomBolge {
            kromozom: self.kromozom.clone(),
            baslangic: bas,
            bitis: bit,
        }
    }

    /// `chr:start-end` biçiminde insan-okur metin (1-tabanlı; cetvel/başlık için).
    pub fn etiket(&self) -> String {
        format!("{}:{}-{}", self.kromozom, self.baslangic, self.bitis)
    }
}

/// Görünüm penceresi: bir [`GenomBolge`]'yi yatay piksel uzayına eşleyen dönüşüm.
///
/// Hesaplar `f64`'te yapılır (Mb ölçeğinde kayan-nokta hassasiyeti korunur), ekran koordinatı
/// `f32` olarak döner (egui/wgpu uzayı).
#[derive(Debug, Clone, PartialEq)]
pub struct Tuval {
    /// Tuvalin yatay piksel genişliği (> 0).
    pub genislik_px: f32,
    /// Görünen bölge.
    pub bolge: GenomBolge,
}

impl Tuval {
    /// Bir tuval kurar.  `genislik_px` ≤ 0 ise 1'e sabitlenir (sıfıra bölme yok).
    pub fn yeni(genislik_px: f32, bolge: GenomBolge) -> Self {
        Self {
            genislik_px: genislik_px.max(1.0),
            bolge,
        }
    }

    /// Piksel başına baz sayısı (bp/piksel) — yakınlaştırma seviyesi ölçütü (LOD bunu kullanır).
    pub fn bp_per_piksel(&self) -> f64 {
        self.bolge.uzunluk() as f64 / self.genislik_px as f64
    }

    /// Baz başına piksel (piksel/bp) — bir bazın ekran genişliği.
    pub fn piksel_per_bp(&self) -> f64 {
        self.genislik_px as f64 / self.bolge.uzunluk() as f64
    }

    /// 1-tabanlı `pos`'un **sol kenarının** ekran x'i (piksel).  Bölge dışındaki pozisyonlar
    /// da lineer olarak eşlenir (negatif/taşan değer dönebilir; culling ayrıca yapılır).
    pub fn genom_to_ekran(&self, pos: u64) -> f32 {
        let ofset = pos as f64 - self.bolge.baslangic as f64;
        (ofset * self.piksel_per_bp()) as f32
    }

    /// `[bas..=bit]` (1-tabanlı kapsayıcı) aralığının ekran x-aralığı `(sol, sag)` piksel.
    /// `sag`, `bit` bazının sonudur (sol_kenar(bit+1)).  En az 1 piksel genişlik garanti edilir.
    pub fn aralik_ekran(&self, bas: u64, bit: u64) -> (f32, f32) {
        let sol = self.genom_to_ekran(bas);
        let sag = self.genom_to_ekran(bit + 1);
        if sag - sol < 1.0 {
            (sol, sol + 1.0)
        } else {
            (sol, sag)
        }
    }

    /// Ekran x'inden (piksel) 1-tabanlı genom pozisyonuna (bazın bulunduğu konum).  Sonuç
    /// `[baslangic, bitis]` aralığına sıkıştırılır (tuval kenarına tıklama güvenli).
    pub fn ekran_to_genom(&self, x: f32) -> u64 {
        let bp_ofset = (x as f64 * self.bp_per_piksel()).floor();
        let ham = self.bolge.baslangic as f64 + bp_ofset;
        if ham < self.bolge.baslangic as f64 {
            self.bolge.baslangic
        } else if ham > self.bolge.bitis as f64 {
            self.bolge.bitis
        } else {
            ham as u64
        }
    }
}

fn gecersiz_bolge_hatasi(neden_ek: &str) -> ErrorReport {
    ErrorReport::new(
        "Geçersiz genom bölgesi",
        format!("bölge sınırları geçersiz ({neden_ek})"),
        "Bölgeyi 'chr1:1000-2000' gibi (başlangıç ≤ bitiş, 1-tabanlı) girin",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bolge_dogrulama_ve_uzunluk() {
        let b = GenomBolge::yeni("chr1", 100, 199).unwrap();
        assert_eq!(b.uzunluk(), 100); // kapsayıcı: 100..=199 = 100 baz
        assert_eq!(b.merkez(), 149);
        assert!(b.kapsar(100) && b.kapsar(199) && !b.kapsar(200));
        assert!(b.ortusur(150, 250)); // örtüşür
        assert!(!b.ortusur(200, 300)); // 200 > 199 → örtüşmez

        assert!(GenomBolge::yeni("chr1", 0, 10).is_err()); // 0-tabanlı reddedilir
        assert!(GenomBolge::yeni("chr1", 50, 49).is_err()); // bitiş < başlangıç
    }

    #[test]
    fn merkezli_ve_sinirla() {
        // 1000 çevresinde 101 bp → 950..=1050.
        let b = GenomBolge::merkezli("chrX", 1000, 101);
        assert_eq!(b.baslangic, 950);
        assert_eq!(b.bitis, 1050);
        assert_eq!(b.uzunluk(), 101);

        // Sol kenar 1'in altına taşarsa 1'e sabitlenir (uzunluk korunur).
        let kenar = GenomBolge::merkezli("chrX", 10, 101);
        assert_eq!(kenar.baslangic, 1);
        assert_eq!(kenar.uzunluk(), 101);

        // Kromozom uzunluğu 1000 iken 980..=1079 penceresi içeri kaydırılır.
        let tasan = GenomBolge::yeni("chr1", 980, 1079).unwrap();
        let s = tasan.sinirla(Some(1000));
        assert_eq!(s.bitis, 1000);
        assert_eq!(s.uzunluk(), 100, "sınırlama uzunluğu korur");

        // Pencere tüm kromozomdan büyükse kromozomu kaplar.
        let buyuk = GenomBolge::yeni("chr1", 1, 5000).unwrap();
        let s2 = buyuk.sinirla(Some(1000));
        assert_eq!((s2.baslangic, s2.bitis), (1, 1000));
    }

    #[test]
    fn ekran_donusumu_oda_simetrik() {
        // 100 bp genişliğinde bölge, 1000 piksel tuval → 10 piksel/bp.
        let t = Tuval::yeni(1000.0, GenomBolge::yeni("chr1", 1, 100).unwrap());
        assert!((t.piksel_per_bp() - 10.0).abs() < 1e-9);
        assert!((t.bp_per_piksel() - 0.1).abs() < 1e-9);

        // pos=1 sol kenarda (x=0); pos=11 → 100 px.
        assert!((t.genom_to_ekran(1) - 0.0).abs() < 1e-3);
        assert!((t.genom_to_ekran(11) - 100.0).abs() < 1e-3);

        // Geri dönüşüm: x=0 → 1, x=105 → pozisyon 11 (floor).
        assert_eq!(t.ekran_to_genom(0.0), 1);
        assert_eq!(t.ekran_to_genom(105.0), 11);

        // Kenar dışına tıklama sıkıştırılır.
        assert_eq!(t.ekran_to_genom(-50.0), 1);
        assert_eq!(t.ekran_to_genom(99999.0), 100);
    }

    #[test]
    fn aralik_ekran_en_az_bir_piksel() {
        // 1 Mb bölge, 800 px → 1 baz ~0.0008 px; yine de ≥1 px genişlik döner (görünür kalır).
        let t = Tuval::yeni(800.0, GenomBolge::yeni("chr1", 1, 1_000_000).unwrap());
        let (sol, sag) = t.aralik_ekran(500_000, 500_000);
        assert!(sag - sol >= 1.0);
    }
}
