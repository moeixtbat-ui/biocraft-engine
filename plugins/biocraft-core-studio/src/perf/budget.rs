//! ÇE-12 — **Kare bütçesi + uyarlamalı detay + performans şeffaflığı** (MK-04, İP-04/İP-08).
//!
//! "Gerçek 60 FPS" iddiasını **ölçülebilir** kılar.  Eklenti GPU/zamanlayıcıya MK-17 gereği
//! dokunamaz; bu yüzden burada **karar mantığı** durur (saf, deterministik, birim-testlenir):
//! * Bir karenin işi 60 FPS bütçesine (**16.67 ms**) sığıyor mu? ([`KareButcesi`])
//! * Sığmıyorsa kaç öğe çizilebilir? → genom tarayıcı LOD'u bu hedefe **seyreltir**
//!   (downsampling), böylece büyük BAM/VCF akıcı kalır (önemli öğe gizlenmeden — ÇE-02).
//! * Donanım yetişmiyorsa **detay sadeleşir + kullanıcı uyarılır** (TDA 11; "performans
//!   şeffaflığı" göstergesi opsiyoneldir).
//!
//! Gerçek FPS motorda ölçülür; bu model ölçümü **alır** ([`PerformansGostergesi::kare_ekle`]) ve
//! kararı/uyarıyı/insan-okunur özeti üretir.

// ─── Kare bütçesi ──────────────────────────────────────────────────────────────

/// Hedef kare hızı (MK-04: gerçek 60 FPS).
pub const HEDEF_FPS: u32 = 60;
/// 60 FPS kare bütçesi — mikrosaniye (1_000_000 / 60 ≈ 16_667 µs ≈ 16.67 ms).
pub const KARE_BUTCESI_US: u64 = 1_000_000 / HEDEF_FPS as u64;

/// Bir kare bütçesi (mikrosaniye).  Çizilecek iş bu bütçeye sığmalı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KareButcesi {
    butce_us: u64,
}

impl KareButcesi {
    /// 60 FPS varsayılan bütçesi.
    pub fn fps60() -> Self {
        Self {
            butce_us: KARE_BUTCESI_US,
        }
    }

    /// Özel hedef FPS'ten bütçe (örn. 30 FPS düşük güç modu).
    pub fn fps(hedef: u32) -> Self {
        Self {
            butce_us: 1_000_000 / hedef.max(1) as u64,
        }
    }

    /// Bütçe (µs).
    pub fn us(&self) -> u64 {
        self.butce_us
    }

    /// Harcanan süre bütçeye sığıyor mu?
    pub fn siginir_mi(&self, harcanan_us: u64) -> bool {
        harcanan_us <= self.butce_us
    }

    /// **Bütçeye sığacak azami öğe sayısı.**  `oge_basina_us` = bir öğenin (read/varyant/atom)
    /// işleme/çizim maliyeti.  0 ise sınırsız (`usize::MAX`).  LOD seyreltme hedefi bundan gelir.
    pub fn azami_oge(&self, oge_basina_us: f64) -> usize {
        if oge_basina_us <= 0.0 {
            return usize::MAX;
        }
        (self.butce_us as f64 / oge_basina_us).floor().max(0.0) as usize
    }
}

impl Default for KareButcesi {
    fn default() -> Self {
        Self::fps60()
    }
}

// ─── Uyarlamalı detay seviyesi ─────────────────────────────────────────────────

/// Donanım/iş yüküne göre seçilen genel **detay seviyesi** (genom tarayıcı + varyant + yoğun iz
/// için; 3B'nin kendi [`crate::structure3d::fallback::KaliteSeviyesi`]'si var).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detay {
    /// Tam detay (her öğe çizilir).
    Tam,
    /// Azaltılmış (seyreltme/binleme; önemli öğeler korunur).
    Azaltilmis,
    /// Asgari (yalnız yoğunluk/özet — çok büyük veri / zayıf donanım).
    Asgari,
}

impl Detay {
    /// İnsan-okunur ad.
    pub fn ad(&self) -> &'static str {
        match self {
            Detay::Tam => "tam",
            Detay::Azaltilmis => "azaltılmış",
            Detay::Asgari => "asgari",
        }
    }
}

/// İstenen öğe sayısı + bütçeden detay seviyesi seçer.  Bütçeye sığıyorsa Tam; az aşıyorsa
/// Azaltılmış; çok aşıyorsa Asgari (özet).
pub fn detay_sec(istenen_oge: usize, butce: KareButcesi, oge_basina_us: f64) -> Detay {
    let azami = butce.azami_oge(oge_basina_us);
    if istenen_oge <= azami {
        Detay::Tam
    } else if istenen_oge <= azami.saturating_mul(8) {
        Detay::Azaltilmis
    } else {
        Detay::Asgari
    }
}

/// Detay seyreltmesi için kullanıcı uyarısı (TDA 11 — sadeleştirme şeffaf bildirilir).  Tam detayda
/// uyarı yok.
pub fn detay_uyarisi(istenen_oge: usize, detay: Detay) -> Option<String> {
    match detay {
        Detay::Tam => None,
        Detay::Azaltilmis => Some(format!(
            "Yoğun bölge ({istenen_oge} öğe) — akıcılık için görünüm seyreltildi (önemli öğeler korunur)."
        )),
        Detay::Asgari => Some(format!(
            "Çok yoğun bölge ({istenen_oge} öğe) — yoğunluk/özet gösterimine geçildi; yakınlaşınca ayrıntı artar."
        )),
    }
}

// ─── Performans şeffaflık göstergesi (opsiyonel; TDA) ──────────────────────────

/// Son karelerin süresini tutan **performans göstergesi** — FPS tahmini + bütçe durumu + kısa
/// insan-okunur özet (kullanıcı isteğe bağlı açar).  Halkasal tampon (sabit bellek).
#[derive(Debug, Clone)]
pub struct PerformansGostergesi {
    son_us: std::collections::VecDeque<u64>,
    kapasite: usize,
    butce: KareButcesi,
}

impl PerformansGostergesi {
    /// Belirli pencere boyutu (kaç kare ortalanır) + bütçeyle gösterge.
    pub fn yeni(pencere: usize, butce: KareButcesi) -> Self {
        Self {
            son_us: std::collections::VecDeque::with_capacity(pencere.max(1)),
            kapasite: pencere.max(1),
            butce,
        }
    }

    /// Bir karenin süresini (µs) ekler (en eski düşer).
    pub fn kare_ekle(&mut self, sure_us: u64) {
        if self.son_us.len() == self.kapasite {
            self.son_us.pop_front();
        }
        self.son_us.push_back(sure_us);
    }

    /// Ortalama kare süresi (µs); henüz örnek yoksa `None`.
    pub fn ortalama_us(&self) -> Option<u64> {
        if self.son_us.is_empty() {
            return None;
        }
        let toplam: u64 = self.son_us.iter().sum();
        Some(toplam / self.son_us.len() as u64)
    }

    /// Tahmini FPS (ortalama kare süresinden); örnek yoksa `None`.
    pub fn fps(&self) -> Option<u32> {
        self.ortalama_us().map(|us| {
            // Sıfır süre (ölçüm altında) → sınırsız FPS (checked_div None → MAX).
            1_000_000u64
                .checked_div(us)
                .map(|f| f as u32)
                .unwrap_or(u32::MAX)
        })
    }

    /// Ortalama kare bütçeye sığıyor mu? (akıcı mı)
    pub fn akici_mi(&self) -> bool {
        self.ortalama_us()
            .map(|us| self.butce.siginir_mi(us))
            .unwrap_or(true)
    }

    /// Kısa insan-okunur özet ("58 FPS — akıcı" / "34 FPS — sadeleştiriliyor").
    pub fn ozet(&self) -> String {
        match self.fps() {
            None => "ölçüm yok".to_string(),
            Some(fps) => {
                let durum = if self.akici_mi() {
                    "akıcı"
                } else {
                    "sadeleştiriliyor"
                };
                format!("{fps} FPS — {durum}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kare_butcesi_60fps_yaklasik_16667us() {
        let b = KareButcesi::fps60();
        assert_eq!(b.us(), 16_666);
        assert!(b.siginir_mi(16_000));
        assert!(!b.siginir_mi(20_000));
        // 30 FPS bütçesi iki kat.
        assert_eq!(KareButcesi::fps(30).us(), 33_333);
    }

    #[test]
    fn azami_oge_butceden_hesaplanir() {
        let b = KareButcesi::fps60(); // 16_666 µs
                                      // Öğe başına 1 µs → ~16_666 öğe sığar.
        assert_eq!(b.azami_oge(1.0), 16_666);
        // Öğe başına 10 µs → ~1_666.
        assert_eq!(b.azami_oge(10.0), 1_666);
        // Maliyet 0 → sınırsız.
        assert_eq!(b.azami_oge(0.0), usize::MAX);
    }

    #[test]
    fn detay_sec_butceye_gore() {
        let b = KareButcesi::fps60();
        let oge_us = 16.666; // ~1000 öğe sığar
        let azami = b.azami_oge(oge_us);
        assert!((999..=1001).contains(&azami));
        assert_eq!(detay_sec(500, b, oge_us), Detay::Tam);
        assert_eq!(detay_sec(azami, b, oge_us), Detay::Tam);
        assert_eq!(detay_sec(azami * 4, b, oge_us), Detay::Azaltilmis);
        assert_eq!(detay_sec(azami * 100, b, oge_us), Detay::Asgari);
    }

    #[test]
    fn detay_uyarisi_sadelestirmede_var_tamda_yok() {
        assert!(detay_uyarisi(100, Detay::Tam).is_none());
        assert!(detay_uyarisi(50_000, Detay::Azaltilmis)
            .unwrap()
            .contains("seyreltildi"));
        assert!(detay_uyarisi(5_000_000, Detay::Asgari)
            .unwrap()
            .contains("özet"));
    }

    #[test]
    fn gosterge_fps_ve_akicilik() {
        let mut g = PerformansGostergesi::yeni(4, KareButcesi::fps60());
        assert_eq!(g.fps(), None); // örnek yok
        assert!(g.akici_mi()); // örnek yokken iyimser
                               // 4 kare ~16 ms → ~62 FPS akıcı.
        for _ in 0..4 {
            g.kare_ekle(16_000);
        }
        assert!(g.akici_mi());
        assert!(g.fps().unwrap() >= 60);
        assert!(g.ozet().contains("akıcı"));
        // Yavaş kareler → bütçe aşılır.
        let mut y = PerformansGostergesi::yeni(4, KareButcesi::fps60());
        for _ in 0..4 {
            y.kare_ekle(30_000); // ~33 FPS
        }
        assert!(!y.akici_mi());
        assert!(y.ozet().contains("sadeleştiriliyor"));
    }

    #[test]
    fn gosterge_pencere_halkasal() {
        // Pencere 2 → yalnız son 2 kare sayılır (eski düşer).
        let mut g = PerformansGostergesi::yeni(2, KareButcesi::fps60());
        g.kare_ekle(50_000); // eski (düşecek)
        g.kare_ekle(16_000);
        g.kare_ekle(16_000);
        assert_eq!(g.ortalama_us(), Some(16_000));
        assert!(g.akici_mi());
    }
}
