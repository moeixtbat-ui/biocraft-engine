//! Açılış splash ekranı zamanlaması — İP-01 (E8).
//!
//! Açılış öncesi **hafif** bir splash gösterilir (logo/DNA heliks + slogan), en fazla ~2 saniye.
//! Kurallar:
//! - Arayüzü **BLOKLAMAZ**: splash yalnızca bir karedir; launcher yüklemesi (son projeler, haber
//!   thread'i, donanım) arka planda sürer.  Splash kapanınca her şey hazırdır.
//! - `--no-splash` (E8) ile tamamen atlanır → launcher anında görünür.
//! - Zamanlama saftır: `Instant` **dışarıdan** verilir (sahte saatle test); egui çizimi view'da.

use std::time::{Duration, Instant};

/// Splash'in görünür kalacağı azami süre (spec: ~2 sn).
pub const SPLASH_SURESI: Duration = Duration::from_millis(2000);

/// Splash ekranının zamanlama durumu (saf).
#[derive(Debug, Clone, Copy)]
pub struct SplashDurumu {
    baslangic: Instant,
    sure: Duration,
    /// `--no-splash` ile atlandı mı (hiç gösterilmez)?
    atlandi: bool,
    /// Kullanıcı tıklayıp erken kapattı mı?
    kapatildi: bool,
}

impl SplashDurumu {
    /// Splash'i `simdi` anında başlatır.  `atla` = `--no-splash` bayrağı.
    pub fn yeni(simdi: Instant, atla: bool) -> Self {
        Self {
            baslangic: simdi,
            sure: SPLASH_SURESI,
            atlandi: atla,
            kapatildi: false,
        }
    }

    /// Test/özel süreyle başlatır.
    pub fn sure_ile(simdi: Instant, sure: Duration, atla: bool) -> Self {
        Self {
            baslangic: simdi,
            sure,
            atlandi: atla,
            kapatildi: false,
        }
    }

    /// Şu an splash gösterilmeli mi?  Atlandıysa/kapatıldıysa/süre dolduysa `false` (launcher görünür).
    pub fn gorunur_mu(&self, simdi: Instant) -> bool {
        if self.atlandi || self.kapatildi {
            return false;
        }
        simdi.duration_since(self.baslangic) < self.sure
    }

    /// İlerleme oranı 0.0..=1.0 (splash içi ilerleme çubuğu için).
    pub fn ilerleme(&self, simdi: Instant) -> f32 {
        if self.atlandi || self.sure.is_zero() {
            return 1.0;
        }
        let gecen = simdi.duration_since(self.baslangic).as_secs_f32();
        (gecen / self.sure.as_secs_f32()).clamp(0.0, 1.0)
    }

    /// Kullanıcı splash'e tıkladı → hemen kapat (atla).
    pub fn kapat(&mut self) {
        self.kapatildi = true;
    }
}

// ─── Testler ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baslangicta_gorunur() {
        let t0 = Instant::now();
        let s = SplashDurumu::yeni(t0, false);
        assert!(s.gorunur_mu(t0));
        assert!(s.gorunur_mu(t0 + Duration::from_millis(500)));
    }

    #[test]
    fn sure_dolunca_gizlenir() {
        let t0 = Instant::now();
        let s = SplashDurumu::yeni(t0, false);
        assert!(!s.gorunur_mu(t0 + SPLASH_SURESI + Duration::from_millis(1)));
    }

    #[test]
    fn no_splash_hic_gorunmez() {
        let t0 = Instant::now();
        let s = SplashDurumu::yeni(t0, true);
        assert!(!s.gorunur_mu(t0), "--no-splash ile anında atlanır");
        assert_eq!(s.ilerleme(t0), 1.0);
    }

    #[test]
    fn tiklayinca_erken_kapanir() {
        let t0 = Instant::now();
        let mut s = SplashDurumu::yeni(t0, false);
        assert!(s.gorunur_mu(t0));
        s.kapat();
        assert!(!s.gorunur_mu(t0), "tıklayınca hemen kapanır");
    }

    #[test]
    fn ilerleme_yarida_yarim() {
        let t0 = Instant::now();
        let s = SplashDurumu::sure_ile(t0, Duration::from_secs(2), false);
        let yari = s.ilerleme(t0 + Duration::from_secs(1));
        assert!((yari - 0.5).abs() < 0.05, "yarıda ~0.5 (oldu: {yari})");
        assert_eq!(s.ilerleme(t0 + Duration::from_secs(10)), 1.0);
    }
}
