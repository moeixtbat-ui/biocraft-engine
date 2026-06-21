//! Otomatik kayıt politikası (MK-38: "periyodik + değişiklikte") — saf, test-edilebilir.
//!
//! Bu modül diske **yazmaz**; yalnızca *ne zaman* yazılması gerektiğine karar verir.  Böylece
//! zamanlama mantığı, gerçek dosya/saat olmadan birim testlerle birebir denetlenebilir.
//! [`crate::DurumYoneticisi`] her kare (frame) `kaydetmeli` sorar ve `true` ise depoya yazıp
//! `kaydedildi` ile politikayı sıfırlar.
//!
//! İki tetikleyici:
//! - **Periyodik:** Kirli (kaydedilmemiş değişiklik varsa) ve son kayıttan beri `periyot` geçmişse.
//! - **Değişiklikte (debounce):** Bir değişiklikten sonra, değişiklikler `min_aralik` kadar
//!   "durulunca" kaydeder — böylece hızlı ardışık değişikliklerde diske boğulmaz.

use std::time::{Duration, Instant};

/// Otomatik kayıt zamanlayıcısı.  Saat dışarıdan (`simdi: Instant`) verilir → testte sahte zaman.
#[derive(Debug, Clone)]
pub struct OtomatikKayit {
    /// En geç bu süre sonunda (kirliyse) kaydet.
    periyot: Duration,
    /// Değişiklik sonrası bu kadar durulma + iki kayıt arası asgari aralık (debounce).
    min_aralik: Duration,
    /// Kaydedilmemiş değişiklik var mı?
    kirli: bool,
    /// Son başarılı kayıt anı.
    son_kayit: Instant,
    /// Son değişiklik anı (debounce için); kayıttan sonra `None`.
    son_degisiklik: Option<Instant>,
}

impl OtomatikKayit {
    /// Özel periyot/aralıkla kurar.  `simdi`: kuruluş anı (ilk kayıt için referans).
    pub fn yeni(periyot: Duration, min_aralik: Duration, simdi: Instant) -> Self {
        Self {
            periyot,
            min_aralik,
            kirli: false,
            son_kayit: simdi,
            son_degisiklik: None,
        }
    }

    /// Varsayılan: 30 sn periyot + 2 sn debounce (etkileşimi bozmadan iş kaybını önler).
    pub fn varsayilan(simdi: Instant) -> Self {
        Self::yeni(Duration::from_secs(30), Duration::from_secs(2), simdi)
    }

    /// Bir değişiklik olduğunu bildirir (durumu "kirli" yapar + debounce saatini başlatır).
    pub fn degisiklik_oldu(&mut self, simdi: Instant) {
        self.kirli = true;
        self.son_degisiklik = Some(simdi);
    }

    /// Kaydedilmemiş değişiklik var mı?
    pub fn kirli_mi(&self) -> bool {
        self.kirli
    }

    /// Şu an kaydedilmeli mi? (periyodik **veya** değişiklik durulması tetiklerse).
    pub fn kaydetmeli(&self, simdi: Instant) -> bool {
        if !self.kirli {
            return false;
        }
        // Periyodik: son kayıttan beri yeterince zaman geçti.
        if simdi.saturating_duration_since(self.son_kayit) >= self.periyot {
            return true;
        }
        // Değişiklik debounce'u: değişiklik durdu (min_aralik) VE son kayıttan beri en az
        // min_aralik geçti (ardışık değişikliklerde diske boğulmayı önler).
        if let Some(d) = self.son_degisiklik {
            if simdi.saturating_duration_since(d) >= self.min_aralik
                && simdi.saturating_duration_since(self.son_kayit) >= self.min_aralik
            {
                return true;
            }
        }
        false
    }

    /// Başarılı kayıt sonrası politikayı sıfırlar (artık temiz).
    pub fn kaydedildi(&mut self, simdi: Instant) {
        self.kirli = false;
        self.son_kayit = simdi;
        self.son_degisiklik = None;
    }
}
