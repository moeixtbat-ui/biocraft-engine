//! Eklenti **çökme izolasyonu** + kaynak görünürlüğü (MK-15, İP-07).
//!
//! Bir eklenti çökerse (WASM trap'i veya alt-süreç çökmesi) **çekirdek + diğer
//! eklentiler ayakta kalır.** Çöken eklenti yalıtılır, kapatılır, kullanıcı
//! bilgilendirilir ve uygunsa **"yeniden başlat"** sunulur.  Sürekli çöken bir eklenti
//! (çökme döngüsü) için artık yeniden başlatma sunulmaz (`KaliciHata`) — kullanıcı
//! sonsuz döngüye düşmez.
//!
//! Bu modül **saf bir denetim defteri**dir (politika + kayıt); fiili izolasyonu
//! sağlayan mekanizma çalıştırıcıdadır: WASM trap'i `EklentiYaniti::Hata` döner
//! (çekirdeği düşürmez), alt-süreç ise ayrı PID'de çalışır (çökerse host'a sıçramaz).
//! Burada bu olaylar kaydedilir, sağlık/kaynak durumu tutulur, karar üretilir.

use biocraft_types::ErrorReport;
use std::collections::BTreeMap;

/// Bir eklentinin "kalıcı hata" sayılması için art arda izin verilen çökme sayısı.
pub const MAKS_COKME: u32 = 3;

/// Bir eklentinin anlık sağlık durumu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EklentiSagligi {
    /// Çalışıyor, sorun yok.
    Saglikli,
    /// Çöktü; `sayi` = toplam çökme adedi.  Yeniden başlatılabilir.
    Coktu { sebep: String, sayi: u32 },
    /// Kullanıcı/çekirdek tarafından kapatıldı.
    Kapatildi,
    /// Çok kez çöktü; artık otomatik yeniden başlatma sunulmaz (kullanıcı elle/güvenli mod).
    KaliciHata { sayi: u32 },
}

impl EklentiSagligi {
    /// Bu durumda eklenti çalışıyor sayılır mı?
    pub fn calisiyor_mu(&self) -> bool {
        matches!(self, EklentiSagligi::Saglikli)
    }
}

/// Bir eklentinin anlık kaynak kullanımı (eklenti başına şeffaflık — TDA m.11).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct KaynakKullanim {
    /// CPU kullanımı (%).
    pub cpu_yuzde: f32,
    /// RAM kullanımı (bayt).
    pub ram_bayt: u64,
    /// GPU kullanımı (%); ölçülemiyorsa 0.
    pub gpu_yuzde: f32,
}

/// Çökme bildirimi sonrası çekirdeğin vereceği karar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CokmeKarari {
    /// Eklenti yalıtıldı mı (her zaman `true` — çekirdek ayakta).
    pub yalitildi: bool,
    /// Kullanıcıya "yeniden başlat" sunulsun mu? (çökme döngüsünde `false`).
    pub yeniden_baslat_sun: bool,
    /// Kullanıcıya gösterilecek standart bildirim.
    pub bildirim: ErrorReport,
}

/// Tek bir eklentinin izolasyon kaydı.
#[derive(Debug, Clone)]
struct Denetim {
    saglik: EklentiSagligi,
    kaynak: KaynakKullanim,
    toplam_cokme: u32,
}

impl Default for Denetim {
    fn default() -> Self {
        Self {
            saglik: EklentiSagligi::Saglikli,
            kaynak: KaynakKullanim::default(),
            toplam_cokme: 0,
        }
    }
}

/// Tüm yüklü eklentilerin sağlık/kaynak durumunu izleyen izolasyon yöneticisi.
#[derive(Debug, Clone, Default)]
pub struct IzolasyonYoneticisi {
    kayitlar: BTreeMap<String, Denetim>,
}

impl IzolasyonYoneticisi {
    /// Boş bir izolasyon yöneticisi.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir eklentiyi (yükleme sırasında) sağlıklı olarak kaydeder.
    pub fn kaydet(&mut self, kimlik: impl Into<String>) {
        self.kayitlar.entry(kimlik.into()).or_default();
    }

    /// Bir eklentinin çöktüğünü bildirir; yalıtır, sayar ve karar üretir.
    ///
    /// Çekirdek **asla** panik yapmaz; çöken eklenti yalıtılır.  `MAKS_COKME`'den sonra
    /// kalıcı hata sayılır ve yeniden başlatma sunulmaz.
    pub fn cokme_bildir(&mut self, kimlik: &str, sebep: impl Into<String>) -> CokmeKarari {
        let sebep = sebep.into();
        let d = self.kayitlar.entry(kimlik.to_string()).or_default();
        d.toplam_cokme += 1;
        d.kaynak = KaynakKullanim::default(); // çöken eklenti kaynak tüketmiyor
        let sayi = d.toplam_cokme;

        let dongu = sayi >= MAKS_COKME;
        d.saglik = if dongu {
            EklentiSagligi::KaliciHata { sayi }
        } else {
            EklentiSagligi::Coktu {
                sebep: sebep.clone(),
                sayi,
            }
        };

        let bildirim = if dongu {
            ErrorReport::new(
                "Eklenti tekrar tekrar çöküyor",
                format!(
                    "'{kimlik}' eklentisi {sayi} kez çöktü ve devre dışı bırakıldı (çekirdek ve diğer eklentiler çalışmaya devam ediyor)"
                ),
                "Eklentiyi güncelleyin/kaldırın veya BioCraft'ı güvenli modda başlatıp teşhis edin",
            )
            .with_eylem("Eklentiyi yönet")
        } else {
            ErrorReport::new(
                "Bir eklenti çöktü",
                format!(
                    "'{kimlik}' eklentisi durdu ({sebep}); çekirdek ve diğer eklentiler etkilenmedi"
                ),
                "Eklentiyi yeniden başlatmayı deneyebilirsiniz",
            )
            .with_eylem("Yeniden başlat")
        };

        CokmeKarari {
            yalitildi: true,
            yeniden_baslat_sun: !dongu,
            bildirim,
        }
    }

    /// Çöken bir eklentiyi yeniden başlatır (sağlığı sıfırlar).
    ///
    /// Çökme döngüsündeki (`KaliciHata`) eklenti **yeniden başlatılamaz** → açıklayıcı hata.
    pub fn yeniden_baslat(&mut self, kimlik: &str) -> Result<(), ErrorReport> {
        let Some(d) = self.kayitlar.get_mut(kimlik) else {
            return Err(ErrorReport::new(
                "Eklenti bulunamadı",
                format!("'{kimlik}' kayıtlı değil"),
                "Eklentinin kurulu olduğundan emin olun",
            ));
        };
        if let EklentiSagligi::KaliciHata { sayi } = d.saglik {
            return Err(ErrorReport::new(
                "Eklenti yeniden başlatılamıyor",
                format!("'{kimlik}' {sayi} kez çöktüğü için kalıcı olarak devre dışı"),
                "Eklentiyi güncelleyin/kaldırın; geçici olarak güvenli modu kullanın",
            ));
        }
        d.saglik = EklentiSagligi::Saglikli;
        Ok(())
    }

    /// Bir eklentiyi kapatır (kullanıcı isteğiyle).
    pub fn kapat(&mut self, kimlik: &str) {
        if let Some(d) = self.kayitlar.get_mut(kimlik) {
            d.saglik = EklentiSagligi::Kapatildi;
            d.kaynak = KaynakKullanim::default();
        }
    }

    /// Bir eklentinin kaynak kullanımını günceller (watchdog/ölçüm besler).
    pub fn kaynak_guncelle(&mut self, kimlik: &str, kaynak: KaynakKullanim) {
        self.kayitlar.entry(kimlik.to_string()).or_default().kaynak = kaynak;
    }

    /// Bir eklentinin sağlık durumu.
    pub fn saglik(&self, kimlik: &str) -> Option<&EklentiSagligi> {
        self.kayitlar.get(kimlik).map(|d| &d.saglik)
    }

    /// Bir eklentinin kaynak kullanımı.
    pub fn kaynak(&self, kimlik: &str) -> Option<KaynakKullanim> {
        self.kayitlar.get(kimlik).map(|d| d.kaynak)
    }

    /// Bir eklenti yeniden başlatılabilir mi? (çökmüş ama döngüde değil)
    pub fn yeniden_baslatilabilir_mi(&self, kimlik: &str) -> bool {
        matches!(self.saglik(kimlik), Some(EklentiSagligi::Coktu { .. }))
    }

    /// Çökme döngüsüne (kalıcı hata) girmiş eklenti sayısı — güvenli mod önerisi için.
    pub fn kalici_hatali_sayisi(&self) -> usize {
        self.kayitlar
            .values()
            .filter(|d| matches!(d.saglik, EklentiSagligi::KaliciHata { .. }))
            .count()
    }

    /// Tüm eklentilerin `(kimlik, sağlık, kaynak)` dökümü (UI kaynak paneli).
    pub fn dokum(&self) -> Vec<(String, EklentiSagligi, KaynakKullanim)> {
        self.kayitlar
            .iter()
            .map(|(k, d)| (k.clone(), d.saglik.clone(), d.kaynak))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cokme_yalitilir_cekirdek_ayakta() {
        let mut y = IzolasyonYoneticisi::yeni();
        y.kaydet("biocraft.a.b");
        let karar = y.cokme_bildir("biocraft.a.b", "trap: bellek");
        assert!(karar.yalitildi);
        assert!(karar.yeniden_baslat_sun);
        assert!(matches!(
            y.saglik("biocraft.a.b"),
            Some(EklentiSagligi::Coktu { .. })
        ));
    }

    #[test]
    fn cokme_sonrasi_yeniden_baslatilir() {
        let mut y = IzolasyonYoneticisi::yeni();
        y.kaydet("biocraft.a.b");
        y.cokme_bildir("biocraft.a.b", "trap");
        assert!(y.yeniden_baslatilabilir_mi("biocraft.a.b"));
        y.yeniden_baslat("biocraft.a.b").unwrap();
        assert_eq!(y.saglik("biocraft.a.b"), Some(&EklentiSagligi::Saglikli));
    }

    #[test]
    fn cokme_dongusu_kalici_hata() {
        let mut y = IzolasyonYoneticisi::yeni();
        y.kaydet("biocraft.kotu.eklenti");
        // MAKS_COKME kez çök → kalıcı hata, yeniden başlatma sunulmaz.
        let mut son = None;
        for _ in 0..MAKS_COKME {
            son = Some(y.cokme_bildir("biocraft.kotu.eklenti", "trap"));
        }
        let son = son.unwrap();
        assert!(
            !son.yeniden_baslat_sun,
            "döngüde yeniden başlatma sunulmamalı"
        );
        assert!(matches!(
            y.saglik("biocraft.kotu.eklenti"),
            Some(EklentiSagligi::KaliciHata { .. })
        ));
        // Yeniden başlatma denemesi reddedilir.
        assert!(y.yeniden_baslat("biocraft.kotu.eklenti").is_err());
        assert_eq!(y.kalici_hatali_sayisi(), 1);
    }

    #[test]
    fn olmayan_eklenti_yeniden_baslatilamaz() {
        let mut y = IzolasyonYoneticisi::yeni();
        assert!(y.yeniden_baslat("yok").is_err());
    }

    #[test]
    fn kaynak_gorunur() {
        let mut y = IzolasyonYoneticisi::yeni();
        y.kaydet("biocraft.a.b");
        y.kaynak_guncelle(
            "biocraft.a.b",
            KaynakKullanim {
                cpu_yuzde: 12.5,
                ram_bayt: 5 * 1024 * 1024,
                gpu_yuzde: 0.0,
            },
        );
        let k = y.kaynak("biocraft.a.b").unwrap();
        assert_eq!(k.ram_bayt, 5 * 1024 * 1024);
        assert_eq!(k.cpu_yuzde, 12.5);
    }

    #[test]
    fn cokunce_kaynak_sifirlanir() {
        let mut y = IzolasyonYoneticisi::yeni();
        y.kaydet("biocraft.a.b");
        y.kaynak_guncelle(
            "biocraft.a.b",
            KaynakKullanim {
                cpu_yuzde: 50.0,
                ram_bayt: 100,
                gpu_yuzde: 0.0,
            },
        );
        y.cokme_bildir("biocraft.a.b", "trap");
        assert_eq!(y.kaynak("biocraft.a.b").unwrap().ram_bayt, 0);
    }

    #[test]
    fn dokum_tum_eklentileri_listeler() {
        let mut y = IzolasyonYoneticisi::yeni();
        y.kaydet("biocraft.a.bir");
        y.kaydet("biocraft.a.iki");
        assert_eq!(y.dokum().len(), 2);
    }
}
