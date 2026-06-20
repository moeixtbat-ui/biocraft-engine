//! GPU sürücü çökmesi (TDR / DeviceLost) kurtarma durum makinesi (MK-04).
//!
//! GPU işleri ≤100 ms parçalara bölündüğü için (bkz. [`crate::frame_budget`]) sürücü
//! nadiren zaman aşımına uğrar; yine de uğrarsa uygulama **çökmez**: durum kaydedilir,
//! cihaz yeniden oluşturulur ( **hedef <5 sn** — MK-04), tekrarlı çökmede iş CPU'ya düşürülür.
//!
//! Bu modül saf bir durum makinesidir (gerçek zaman/`Instant` okumaz; süreler dışarıdan
//! verilir) → birim-testlenebilir.  Cihazı fiilen yeniden kuran kod [`crate::gpu`]'dadır.

use std::time::Duration;

/// MK-04: kurtarma hedefi — cihaz bu süre içinde yeniden ayağa kaldırılmalı.
pub const KURTARMA_HEDEFI: Duration = Duration::from_secs(5);

/// Varsayılan: kaç ardışık başarısız kurtarma denemesinden sonra CPU'ya düşülür.
pub const VARSAYILAN_MAX_DENEME: u32 = 3;

/// GPU cihazının kurtarma durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TdrDurum {
    /// Cihaz sağlıklı.
    Saglikli,
    /// Cihaz kayboldu; yeniden oluşturuluyor.
    Kurtariliyor,
    /// Kurtarıldı (kullanıcıya geçici "GPU yeniden başlatıldı" bildirimi gösterilir).
    Kurtarildi,
    /// Tekrarlı başarısızlık → CPU yazılım moduna düşüldü.
    CpuDustu,
}

/// `cihaz_kayboldu` sonrası host'un uygulayacağı eylem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KurtarmaPlani {
    /// Cihazı (mümkünse aynı backend ile) yeniden oluştur.
    CihazYenidenKur,
    /// Artık GPU denenmeyecek; CPU yazılım backend'ine geç.
    CpuyaDus,
}

/// TDR/DeviceLost kurtarma durum makinesi.
pub struct TdrKurtarma {
    durum: TdrDurum,
    hedef: Duration,
    deneme: u32,
    max_deneme: u32,
    son_kurtarma: Option<Duration>,
}

impl TdrKurtarma {
    /// Varsayılan: <5 sn hedef, 3 ardışık denemeden sonra CPU.
    pub fn yeni() -> Self {
        Self::ozel(KURTARMA_HEDEFI, VARSAYILAN_MAX_DENEME)
    }

    /// Özel hedef süre ve maksimum ardışık deneme sayısı.
    pub fn ozel(hedef: Duration, max_deneme: u32) -> Self {
        Self {
            durum: TdrDurum::Saglikli,
            hedef,
            deneme: 0,
            max_deneme,
            son_kurtarma: None,
        }
    }

    /// Güncel durum.
    pub fn durum(&self) -> TdrDurum {
        self.durum
    }

    /// MK-04 kurtarma hedefi (<5 sn).
    pub fn hedef(&self) -> Duration {
        self.hedef
    }

    /// Mevcut ardışık başarısız deneme sayısı.
    pub fn deneme_sayisi(&self) -> u32 {
        self.deneme
    }

    /// En son başarılı kurtarmanın süresi (varsa).
    pub fn son_kurtarma_suresi(&self) -> Option<Duration> {
        self.son_kurtarma
    }

    /// Cihaz kaybı/sürücü çökmesi bildirildi → yapılacak eylemi üretir.
    /// `max_deneme` ardışık deneme aşılırsa CPU'ya düşülür.
    pub fn cihaz_kayboldu(&mut self) -> KurtarmaPlani {
        self.deneme = self.deneme.saturating_add(1);
        if self.deneme > self.max_deneme {
            self.durum = TdrDurum::CpuDustu;
            KurtarmaPlani::CpuyaDus
        } else {
            self.durum = TdrDurum::Kurtariliyor;
            KurtarmaPlani::CihazYenidenKur
        }
    }

    /// Cihaz `gecen` sürede yeniden kuruldu → durum `Kurtarildi`, ardışık deneme sayacı sıfırlanır.
    /// Sürenin hedefi tutturup tutturmadığını [`TdrKurtarma::hedefte_mi`] ile sorgulayın.
    pub fn cihaz_kuruldu(&mut self, gecen: Duration) {
        self.son_kurtarma = Some(gecen);
        self.durum = TdrDurum::Kurtarildi;
        self.deneme = 0;
    }

    /// Geçici "Kurtarildi" bildirimi gösterildikten sonra normale (`Saglikli`) dön.
    pub fn bildirim_gosterildi(&mut self) {
        if self.durum == TdrDurum::Kurtarildi {
            self.durum = TdrDurum::Saglikli;
        }
    }

    /// Verilen kurtarma süresi MK-04 hedefini (<5 sn) tutturdu mu?
    pub fn hedefte_mi(&self, gecen: Duration) -> bool {
        gecen <= self.hedef
    }

    /// Şu an CPU yazılım moduna düşülmüş mü?
    pub fn cpu_modu_mu(&self) -> bool {
        self.durum == TdrDurum::CpuDustu
    }
}

impl Default for TdrKurtarma {
    fn default() -> Self {
        Self::yeni()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baslangic_saglikli() {
        let t = TdrKurtarma::yeni();
        assert_eq!(t.durum(), TdrDurum::Saglikli);
        assert!(!t.cpu_modu_mu());
    }

    #[test]
    fn cihaz_kaybi_yeniden_kurma_planlar() {
        let mut t = TdrKurtarma::yeni();
        assert_eq!(t.cihaz_kayboldu(), KurtarmaPlani::CihazYenidenKur);
        assert_eq!(t.durum(), TdrDurum::Kurtariliyor);
    }

    #[test]
    fn basarili_kurtarma_durumu_ve_sayaci() {
        let mut t = TdrKurtarma::yeni();
        t.cihaz_kayboldu();
        t.cihaz_kuruldu(Duration::from_millis(800));
        assert_eq!(t.durum(), TdrDurum::Kurtarildi);
        assert_eq!(t.son_kurtarma_suresi(), Some(Duration::from_millis(800)));
        assert_eq!(t.deneme_sayisi(), 0, "başarılı kurtarma sayacı sıfırlar");
        // Bildirim gösterilince normale döner.
        t.bildirim_gosterildi();
        assert_eq!(t.durum(), TdrDurum::Saglikli);
    }

    #[test]
    fn hedef_5sn_kontrolu() {
        let t = TdrKurtarma::yeni();
        assert!(t.hedefte_mi(Duration::from_secs(4)), "<5 sn hedefte");
        assert!(
            t.hedefte_mi(Duration::from_secs(5)),
            "tam 5 sn sınırda kabul"
        );
        assert!(!t.hedefte_mi(Duration::from_secs(6)), ">5 sn hedef dışı");
    }

    #[test]
    fn tekrarli_cokmede_cpuya_dusulur() {
        let mut t = TdrKurtarma::ozel(KURTARMA_HEDEFI, 3);
        // 3 ardışık deneme: yeniden kurma.
        assert_eq!(t.cihaz_kayboldu(), KurtarmaPlani::CihazYenidenKur); // 1
        assert_eq!(t.cihaz_kayboldu(), KurtarmaPlani::CihazYenidenKur); // 2
        assert_eq!(t.cihaz_kayboldu(), KurtarmaPlani::CihazYenidenKur); // 3
                                                                        // 4. ardışık çökme: artık CPU'ya düş.
        assert_eq!(t.cihaz_kayboldu(), KurtarmaPlani::CpuyaDus);
        assert_eq!(t.durum(), TdrDurum::CpuDustu);
        assert!(t.cpu_modu_mu());
    }

    #[test]
    fn arada_basari_sayaci_sifirlar() {
        let mut t = TdrKurtarma::ozel(KURTARMA_HEDEFI, 3);
        t.cihaz_kayboldu();
        t.cihaz_kayboldu();
        t.cihaz_kuruldu(Duration::from_millis(500)); // başarı → sıfırla
                                                     // Yeniden 3 deneme daha mümkün olmalı (CPU'ya düşmeden).
        assert_eq!(t.cihaz_kayboldu(), KurtarmaPlani::CihazYenidenKur);
        assert_eq!(t.cihaz_kayboldu(), KurtarmaPlani::CihazYenidenKur);
        assert_eq!(t.cihaz_kayboldu(), KurtarmaPlani::CihazYenidenKur);
        assert!(!t.cpu_modu_mu());
    }
}
