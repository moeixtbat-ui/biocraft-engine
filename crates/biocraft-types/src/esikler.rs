//! Edge-case (sınır durum) eşikleri — somut, test edilebilir sabitler ve davranışlar
//! (İP-21, Bölüm 0.12).
//!
//! Spec, sınır durumlar için **sayısal eşikler** verir; bunları tek yerde tanımlayıp
//! test ederiz, böylece tüm katmanlar aynı politikayı uygular (disk koruması, ağ yeniden
//! deneme, zaman aşımı).  Gerçek disk/ağ ölçümü üst katmanlardadır (sysinfo/IPC); burada
//! **karar mantığı** vardır — saf ve deterministik (golden'lanabilir).

use serde::{Deserialize, Serialize};

// ─── Disk doluluk eşikleri (0.12: %10 uyarı / %2 salt-okunur) ─────────────────

/// Boş disk oranı bu yüzdenin altına inince **uyarı** gösterilir (kullanıcı bilgilendirilir).
pub const DISK_UYARI_YUZDESI: f64 = 10.0;
/// Boş disk oranı bu yüzdenin altına inince yazma durdurulur → **salt-okunur** mod
/// (veri bozulmasını önler; MK-22 ile uyumlu).
pub const DISK_SALT_OKUNUR_YUZDESI: f64 = 2.0;

/// Disk doluluk durumu sınıflandırması.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiskDurumu {
    /// Yeterli boş alan.
    Yeterli,
    /// Az kaldı — kullanıcı uyarılır, yazma devam eder.
    Uyari,
    /// Kritik — yeni yazma durdurulur (salt-okunur).
    SaltOkunur,
}

impl DiskDurumu {
    /// Boş alan yüzdesinden (0–100) disk durumunu sınıflandırır.
    pub fn siniflandir(bos_yuzde: f64) -> Self {
        if bos_yuzde < DISK_SALT_OKUNUR_YUZDESI {
            DiskDurumu::SaltOkunur
        } else if bos_yuzde < DISK_UYARI_YUZDESI {
            DiskDurumu::Uyari
        } else {
            DiskDurumu::Yeterli
        }
    }

    /// Yeni yazmaya izin var mı?
    pub fn yazilabilir(&self) -> bool {
        !matches!(self, DiskDurumu::SaltOkunur)
    }
}

// ─── Ağ kesintisi: üstel geri çekilme (0.12: 1s→60s, max 5 deneme, jitter) ────

/// İlk yeniden deneme gecikmesi (saniye).
pub const GERICEKILME_TABAN_SANIYE: u64 = 1;
/// En fazla yeniden deneme gecikmesi — üst sınır (saniye).
pub const GERICEKILME_TAVAN_SANIYE: u64 = 60;
/// En fazla yeniden deneme sayısı; sonra kalıcı hata.
pub const GERICEKILME_MAKS_DENEME: u32 = 5;

/// Üstel geri çekilme zamanlayıcısı (ağ/dış çağrı yeniden denemesi için).
///
/// Gecikme = `taban * 2^deneme`, `tavan` ile sınırlı.  **Jitter** (sapma) gerçek bekleme
/// sırasında [`Gericekilme::jitterli`] ile uygulanır; çekirdek hesap deterministiktir (test).
#[derive(Debug, Clone, Copy)]
pub struct Gericekilme {
    /// Şu ana dek yapılan deneme sayısı.
    pub deneme: u32,
}

impl Gericekilme {
    /// Sıfırıncı denemeden başlatır.
    pub fn yeni() -> Self {
        Self { deneme: 0 }
    }

    /// Bir sonraki deneme yapılmalı mı? (maks. deneme aşılmadıysa)
    pub fn devam_eder_mi(&self) -> bool {
        self.deneme < GERICEKILME_MAKS_DENEME
    }

    /// Bu deneme için (jitter'sız) gecikme — saniye.  Üstel, tavanla sınırlı.
    pub fn gecikme_saniye(&self) -> u64 {
        let katsayi = 1u64.checked_shl(self.deneme).unwrap_or(u64::MAX);
        GERICEKILME_TABAN_SANIYE
            .saturating_mul(katsayi)
            .min(GERICEKILME_TAVAN_SANIYE)
    }

    /// Jitter uygulanmış gecikme: `[gecikme/2, gecikme]` aralığında.  `rastgele` 0.0–1.0
    /// arası (çağıran sağlar → test deterministik; üretimde OS rastgeleliği).
    pub fn jitterli(&self, rastgele: f64) -> f64 {
        let g = self.gecikme_saniye() as f64;
        let oran = rastgele.clamp(0.0, 1.0);
        g * (0.5 + 0.5 * oran)
    }

    /// Bir denemeyi işaretler (sayaç artar).
    pub fn ilerle(&mut self) {
        self.deneme = self.deneme.saturating_add(1);
    }
}

impl Default for Gericekilme {
    fn default() -> Self {
        Self::yeni()
    }
}

// ─── Zaman aşımı eşikleri (0.12: bağlantı 10s / boşta 60s) ────────────────────

/// Bağlantı kurma zaman aşımı (saniye).
pub const ZAMANASIMI_BAGLANTI_SANIYE: u64 = 10;
/// Boşta (idle) zaman aşımı — yanıt gelmezse (saniye).
pub const ZAMANASIMI_BOSTA_SANIYE: u64 = 60;

// ─── GPU kurtarma eşiği (0.12: <5s) ──────────────────────────────────────────

/// GPU çökmesi sonrası en geç kurtarma süresi hedefi (saniye).
pub const GPU_KURTARMA_HEDEF_SANIYE: u64 = 5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_siniflandirma_esikleri() {
        assert_eq!(DiskDurumu::siniflandir(50.0), DiskDurumu::Yeterli);
        assert_eq!(DiskDurumu::siniflandir(10.0), DiskDurumu::Yeterli); // sınır dahil yeterli
        assert_eq!(DiskDurumu::siniflandir(9.9), DiskDurumu::Uyari);
        assert_eq!(DiskDurumu::siniflandir(2.0), DiskDurumu::Uyari);
        assert_eq!(DiskDurumu::siniflandir(1.9), DiskDurumu::SaltOkunur);
        assert_eq!(DiskDurumu::siniflandir(0.0), DiskDurumu::SaltOkunur);
    }

    #[test]
    fn salt_okunur_yazmayi_durdurur() {
        assert!(DiskDurumu::Yeterli.yazilabilir());
        assert!(DiskDurumu::Uyari.yazilabilir());
        assert!(!DiskDurumu::SaltOkunur.yazilabilir());
    }

    #[test]
    fn gericekilme_ustel_tavan_ile_sinirli() {
        let mut g = Gericekilme::yeni();
        assert_eq!(g.gecikme_saniye(), 1); // 2^0
        g.ilerle();
        assert_eq!(g.gecikme_saniye(), 2); // 2^1
        g.ilerle();
        assert_eq!(g.gecikme_saniye(), 4); // 2^2
        g.ilerle();
        assert_eq!(g.gecikme_saniye(), 8);
        g.ilerle();
        assert_eq!(g.gecikme_saniye(), 16);
        // İlerledikçe tavan (60s) aşılmaz.
        for _ in 0..10 {
            g.ilerle();
            assert!(g.gecikme_saniye() <= GERICEKILME_TAVAN_SANIYE);
        }
    }

    #[test]
    fn gericekilme_maks_denemede_durur() {
        let mut g = Gericekilme::yeni();
        let mut sayac = 0;
        while g.devam_eder_mi() {
            g.ilerle();
            sayac += 1;
            assert!(sayac <= 100, "sonsuz döngü koruması");
        }
        assert_eq!(sayac, GERICEKILME_MAKS_DENEME);
    }

    #[test]
    fn jitter_yari_aralikta_kalir() {
        let g = Gericekilme { deneme: 3 }; // gecikme 8s
        assert_eq!(g.gecikme_saniye(), 8);
        // rastgele=0 → en düşük (gecikme/2), rastgele=1 → tam gecikme.
        assert_eq!(g.jitterli(0.0), 4.0);
        assert_eq!(g.jitterli(1.0), 8.0);
        let orta = g.jitterli(0.5);
        assert!((4.0..=8.0).contains(&orta));
    }

    #[test]
    fn zaman_asimi_sabitleri() {
        // Bağlantı zaman aşımı, boşta zaman aşımından kısa olmalı (0.12).
        let (baglanti, bosta) = (ZAMANASIMI_BAGLANTI_SANIYE, ZAMANASIMI_BOSTA_SANIYE);
        assert!(baglanti < bosta);
        assert_eq!(baglanti, 10);
        assert_eq!(bosta, 60);
    }
}
