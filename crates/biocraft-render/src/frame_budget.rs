//! Kare bütçesi yönetimi (MK-03): ~16.67 ms/kare (60 FPS).
//!
//! Arayüz **hiçbir kareyi kaçırmaz**: her karenin süresi ölçülür, ağır iş
//! [`FrameBudget::kalan`] ile kalan süreye sığacak parçalara bölünür ve taşan iş bir
//! sonraki kareye ertelenir.  Statik ekranda (kullanıcı etkileşimi/animasyon yokken)
//! güç tasarrufu için hedef FPS düşürülür (Eco mod — İP-04 "statik ekranda FPS düşürme").
//!
//! Ayrıca MK-04 gereği GPU işleri ≤100 ms parçalara bölünür ([`gpu_parca_boyutu`]).
//!
//! Bu modül saf (egui/wgpu'dan bağımsız) tutulur; tüm mantık birim-testlenebilir.

use std::collections::VecDeque;
use std::time::Duration;

/// 60 FPS hedefi (MK-03): bir karenin ideal süresi.
pub const HEDEF_60FPS: Duration = Duration::from_micros(16_667);

/// Eco (güç tasarrufu) modunda hedef: ~30 FPS.
pub const ECO_30FPS: Duration = Duration::from_micros(33_333);

/// Eco moduna geçmeden önce beklenen ardışık boşta-kare sayısı.
const ECO_ESIK_KARE: u32 = 30;

/// MK-04: Tek bir GPU gönderiminin (batch) üst sınırı — sürücü zaman aşımını (TDR)
/// önlemek için GPU işleri bu süreyi aşmayacak parçalara bölünür.
pub const GPU_BATCH_USTSINIR: Duration = Duration::from_millis(100);

/// MK-04: Öğe başına tahmini `oge_basina` süreyle, ≤100 ms'lik bir GPU batch'ine kaç
/// öğe sığacağını döndürür (en az 1).  `oge_basina` sıfırsa sınır yoktur ([`usize::MAX`]).
pub fn gpu_parca_boyutu(oge_basina: Duration) -> usize {
    if oge_basina.is_zero() {
        return usize::MAX;
    }
    let sinir = GPU_BATCH_USTSINIR.as_nanos();
    let bir = oge_basina.as_nanos().max(1);
    (sinir / bir).max(1) as usize
}

/// Kare süresi ölçümü + FPS tahmini + bütçe aşımı tespiti.
pub struct FrameBudget {
    hedef: Duration,
    son_kareler: VecDeque<Duration>,
    kapasite: usize,
    eco: bool,
    bosta_kare: u32,
}

impl FrameBudget {
    /// Verilen hedef FPS ile yeni bütçe (örn. `60.0`).
    pub fn yeni(hedef_fps: f32) -> Self {
        let hedef = if hedef_fps > 0.0 {
            Duration::from_secs_f32(1.0 / hedef_fps)
        } else {
            HEDEF_60FPS
        };
        Self {
            hedef,
            son_kareler: VecDeque::with_capacity(120),
            kapasite: 120,
            eco: false,
            bosta_kare: 0,
        }
    }

    /// 60 FPS hedefiyle varsayılan bütçe (MK-03).
    pub fn varsayilan() -> Self {
        Self::yeni(60.0)
    }

    /// Bir karenin gerçekleşen süresini kaydet (FPS penceresine ekler).
    pub fn kare_kaydet(&mut self, sure: Duration) {
        if self.son_kareler.len() == self.kapasite {
            self.son_kareler.pop_front();
        }
        self.son_kareler.push_back(sure);
    }

    /// Temel (Eco olmayan) kare hedefi.
    pub fn hedef(&self) -> Duration {
        self.hedef
    }

    /// Etkin (Eco'yu da dikkate alan) kare hedefi.
    pub fn etkin_hedef(&self) -> Duration {
        if self.eco {
            ECO_30FPS.max(self.hedef)
        } else {
            self.hedef
        }
    }

    /// Son kare hedefi aştı mı? (kare kaçırma riski → ağır iş ertelenmeli)
    pub fn butce_asildi(&self) -> bool {
        self.son_kareler
            .back()
            .is_some_and(|&s| s > self.etkin_hedef())
    }

    /// Bu karede ağır işe ayrılabilecek kalan süre (kare başlangıcından beri `gecen`
    /// süre verilir).  Taşma olursa [`Duration::ZERO`] döner → iş bir sonraki kareye ertelenir.
    pub fn kalan(&self, gecen: Duration) -> Duration {
        self.etkin_hedef()
            .checked_sub(gecen)
            .unwrap_or(Duration::ZERO)
    }

    /// Yumuşatılmış (ortalama) FPS.  Henüz kare yoksa `0.0`.
    pub fn fps(&self) -> f32 {
        if self.son_kareler.is_empty() {
            return 0.0;
        }
        let toplam: Duration = self.son_kareler.iter().sum();
        let ort = toplam.as_secs_f32() / self.son_kareler.len() as f32;
        if ort > 0.0 {
            1.0 / ort
        } else {
            0.0
        }
    }

    /// Kullanıcı etkileşimi/animasyon var → Eco modundan çık.
    pub fn etkinlik_var(&mut self) {
        self.bosta_kare = 0;
        self.eco = false;
    }

    /// Bu kare boştaydı (etkileşim/animasyon yok) → eşik aşılınca Eco'ya geç.
    pub fn bosta(&mut self) {
        self.bosta_kare = self.bosta_kare.saturating_add(1);
        if self.bosta_kare >= ECO_ESIK_KARE {
            self.eco = true;
        }
    }

    /// Şu an Eco (güç tasarrufu) modunda mı?
    pub fn eco_mu(&self) -> bool {
        self.eco
    }
}

impl Default for FrameBudget {
    fn default() -> Self {
        Self::varsayilan()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hedef_60fps_yaklasik_16ms() {
        let b = FrameBudget::varsayilan();
        // 60 FPS ≈ 16.67 ms; ±0.1 ms tolerans.
        let ms = b.hedef().as_secs_f32() * 1000.0;
        assert!(
            (ms - 16.67).abs() < 0.1,
            "hedef {ms} ms, ~16.67 bekleniyordu"
        );
    }

    #[test]
    fn fps_ortalamadan_hesaplanir() {
        let mut b = FrameBudget::varsayilan();
        for _ in 0..10 {
            b.kare_kaydet(Duration::from_millis(10)); // 10 ms ⇒ 100 FPS
        }
        assert!(
            (b.fps() - 100.0).abs() < 1.0,
            "fps {} ~100 bekleniyordu",
            b.fps()
        );
    }

    #[test]
    fn bos_pencerede_fps_sifir() {
        assert_eq!(FrameBudget::varsayilan().fps(), 0.0);
    }

    #[test]
    fn butce_asimi_tespit_edilir() {
        let mut b = FrameBudget::varsayilan();
        b.kare_kaydet(Duration::from_millis(8));
        assert!(!b.butce_asildi(), "8 ms bütçe içinde olmalı");
        b.kare_kaydet(Duration::from_millis(40));
        assert!(b.butce_asildi(), "40 ms bütçeyi aşmalı");
    }

    #[test]
    fn kalan_sure_tasmada_sifirlanir() {
        let b = FrameBudget::varsayilan();
        assert_eq!(b.kalan(Duration::from_millis(50)), Duration::ZERO);
        assert!(b.kalan(Duration::from_millis(5)) > Duration::ZERO);
    }

    #[test]
    fn eco_modu_bostada_devreye_girer() {
        let mut b = FrameBudget::varsayilan();
        assert!(!b.eco_mu());
        for _ in 0..ECO_ESIK_KARE {
            b.bosta();
        }
        assert!(b.eco_mu(), "30 boşta kare sonrası Eco aktif olmalı");
        // Eco'da etkin hedef daha gevşek (≥ 30 FPS süresi).
        assert!(b.etkin_hedef() >= b.hedef());
        // Etkinlik gelince Eco kapanır.
        b.etkinlik_var();
        assert!(!b.eco_mu());
    }

    #[test]
    fn gpu_batch_100ms_ile_sinirli() {
        // 1 ms/öğe ⇒ 100 ms'ye 100 öğe sığar.
        assert_eq!(gpu_parca_boyutu(Duration::from_millis(1)), 100);
        // 200 ms/öğe ⇒ tek öğe bile sınırı aşar ama en az 1 döner.
        assert_eq!(gpu_parca_boyutu(Duration::from_millis(200)), 1);
        // 0 ⇒ sınırsız.
        assert_eq!(gpu_parca_boyutu(Duration::ZERO), usize::MAX);
    }
}
