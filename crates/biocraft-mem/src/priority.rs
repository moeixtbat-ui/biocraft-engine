//! İşleme öncelik modları + Zero-Impact kancası — İP-08.
//!
//! Kullanıcı üç moddan birini seçer:
//! - **Arayüz öncelikli:** akıcılık öne çıkar, arka plan hesabı kısıtlı.
//! - **Denge:** makul arka plan hesabı + akıcı arayüz.
//! - **Maksimum hesap:** tüm çekirdekler işe koşulur (arayüz biraz takılabilir).
//!
//! **Zero-Impact kancası:** Kullanıcı başka bir işe/oyuna geçtiğinde (arayüz arkaplanda)
//! hesap kısılabilir — donanımı boş yere yormamak için.  Maksimum hesap modunda kullanıcı
//! bilinçli olarak tam gücü istediğinden kısma uygulanmaz.
//!
//! Bu modül **saf**tır: kaç worker thread çalışacağını hesaplar; thread'leri kendisi
//! kurmaz (onu iş katmanı/Rayon yapar).  Donanım sıcaklık/termal koruma AYRI gün (İP-08
//! Donanım Koruma, Gün 8) — burada yalnızca hesap önceliği vardır.

/// Kullanıcının seçtiği işleme önceliği.  Varsayılan: [`OncelikModu::Denge`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OncelikModu {
    /// Akıcı arayüz önce; arka plan hesabı kısıtlı (çekirdeklerin ~yarısı).
    ArayuzOncelikli,
    /// Denge: arayüz akıcı + makul arka plan (çekirdeklerin ~3/4'ü).
    #[default]
    Denge,
    /// Maksimum hesap: tüm çekirdekler (arayüz takılabilir).
    MaksimumHesap,
}

impl OncelikModu {
    /// Arka plan hesabına ayrılacak çekirdek oranı (0.0–1.0).
    pub fn taban_oran(&self) -> f32 {
        match self {
            OncelikModu::ArayuzOncelikli => 0.5,
            OncelikModu::Denge => 0.75,
            OncelikModu::MaksimumHesap => 1.0,
        }
    }

    /// Durum panelinde gösterilecek Türkçe ad.
    pub fn ad(&self) -> &'static str {
        match self {
            OncelikModu::ArayuzOncelikli => "Arayüz öncelikli",
            OncelikModu::Denge => "Denge",
            OncelikModu::MaksimumHesap => "Maksimum hesap",
        }
    }

    /// Sıradaki moda geç (UI'da tek butonla döngü için).
    pub fn dongu(&self) -> Self {
        match self {
            OncelikModu::ArayuzOncelikli => OncelikModu::Denge,
            OncelikModu::Denge => OncelikModu::MaksimumHesap,
            OncelikModu::MaksimumHesap => OncelikModu::ArayuzOncelikli,
        }
    }
}

/// Anlık öncelik durumu: seçili mod + kullanıcının şu an aktif olup olmadığı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OncelikDurumu {
    /// Seçili işleme modu.
    pub modu: OncelikModu,
    /// Kullanıcı şu an arayüzle etkileşiyor mu?  `true` ise Zero-Impact ile arka plan kısılır.
    /// (Genelde tersi: kullanıcı BAŞKA işe geçince burada `false` → tam hız verilebilir.
    /// Anlam: "etkileşim halinde arayüzü koru".)
    pub kullanici_aktif: bool,
}

impl OncelikDurumu {
    /// Verilen modla, kullanıcı etkin varsayımıyla başlar.
    pub fn yeni(modu: OncelikModu) -> Self {
        Self {
            modu,
            kullanici_aktif: true,
        }
    }

    /// Kullanıcı etkinlik bayrağını değiştirir (Zero-Impact kancası).
    pub fn kullanici_aktif_ayarla(&mut self, aktif: bool) {
        self.kullanici_aktif = aktif;
    }
}

/// Öncelik durumundan türetilen somut hesap planı.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HesapPlani {
    /// Çalıştırılacak arka plan worker thread sayısı (≥1).
    pub worker_sayisi: usize,
    /// Zero-Impact ile kısıldı mı?
    pub kisilmis: bool,
    /// Uygulanan etkin oran (0.0–1.0).
    pub oran: f32,
}

/// **Hesap planını hesapla.**  `cekirdek`: makinedeki mantıksal çekirdek sayısı.
///
/// Zero-Impact: kullanıcı arayüzle aktif etkileşim halindeyse (ve mod "maksimum" değilse)
/// arka plan hesabı yarıya kısılır ki arayüz akıcı kalsın / donanım boşa yorulmasın.
pub fn hesap_plani(durum: OncelikDurumu, cekirdek: usize) -> HesapPlani {
    let cekirdek = cekirdek.max(1);
    let mut oran = durum.modu.taban_oran();
    let mut kisilmis = false;

    if durum.kullanici_aktif && durum.modu != OncelikModu::MaksimumHesap {
        oran *= 0.5;
        kisilmis = true;
    }

    let worker = ((cekirdek as f32 * oran).round() as usize).max(1);
    HesapPlani {
        worker_sayisi: worker,
        kisilmis,
        oran,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_dongusu_uc_modu_dolasir() {
        let m = OncelikModu::ArayuzOncelikli;
        let m = m.dongu();
        assert_eq!(m, OncelikModu::Denge);
        let m = m.dongu();
        assert_eq!(m, OncelikModu::MaksimumHesap);
        let m = m.dongu();
        assert_eq!(m, OncelikModu::ArayuzOncelikli);
    }

    #[test]
    fn maksimum_hesap_tum_cekirdekleri_kullanir() {
        let durum = OncelikDurumu {
            modu: OncelikModu::MaksimumHesap,
            kullanici_aktif: true, // aktif olsa bile maksimumda kısılmaz
        };
        let plan = hesap_plani(durum, 8);
        assert_eq!(plan.worker_sayisi, 8);
        assert!(!plan.kisilmis);
    }

    #[test]
    fn arayuz_oncelikli_yari_cekirdek() {
        // Kullanıcı pasif (başka işte değil → tam taban oranı): 0.5 × 8 = 4.
        let durum = OncelikDurumu {
            modu: OncelikModu::ArayuzOncelikli,
            kullanici_aktif: false,
        };
        let plan = hesap_plani(durum, 8);
        assert_eq!(plan.worker_sayisi, 4);
        assert!(!plan.kisilmis);
    }

    #[test]
    fn zero_impact_kullanici_aktifken_kisilir() {
        // Denge modu (0.75) + kullanıcı aktif → ×0.5 = 0.375 × 8 ≈ 3.
        let durum = OncelikDurumu {
            modu: OncelikModu::Denge,
            kullanici_aktif: true,
        };
        let plan = hesap_plani(durum, 8);
        assert!(plan.kisilmis);
        assert_eq!(plan.worker_sayisi, 3); // round(3.0) = 3
        assert!(plan.oran < OncelikModu::Denge.taban_oran());
    }

    #[test]
    fn en_az_bir_worker_garanti() {
        let durum = OncelikDurumu {
            modu: OncelikModu::ArayuzOncelikli,
            kullanici_aktif: true,
        };
        // 1 çekirdek × 0.5 × 0.5 = 0.25 → round 0 → ama en az 1 olmalı.
        let plan = hesap_plani(durum, 1);
        assert_eq!(plan.worker_sayisi, 1);
    }

    #[test]
    fn sifir_cekirdek_bir_sayilir() {
        let plan = hesap_plani(OncelikDurumu::yeni(OncelikModu::Denge), 0);
        assert!(plan.worker_sayisi >= 1);
    }
}
