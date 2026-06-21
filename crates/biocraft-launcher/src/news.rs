//! Haber/duyuru akışı — İP-01 (asenkron, **arayüzü asla bloklamaz**).
//!
//! Bilim haberleri + şirket duyuruları + sürüm notları küratörlü bir uzak akıştan gelir.
//! **Kritik kural (spec/Dikkat):** ağ tamamen asenkron olmalı — launcher haber gelmesini
//! beklerken donmaz.  Bunu sağlamak için:
//! - Çekme ([`HaberKaynagi::getir`]) ayrı bir `std::thread`'de çalışır (subprocess.rs / watchdog
//!   ile aynı kalıp; Tokio değil — workspace'e ağır bağımlılık eklemeden MK-07 "pull-based"
//!   eşzamanlılık).
//! - Arayüz her karede [`HaberYukleyici::yokla`] ile kanalı **bloklamadan** yoklar (`try_recv`).
//! - Yükleniyorken view iskelet (skeleton) gösterir; gelince dolar (TDA madde 6).
//! - Ağ başarısızsa **son önbellek** gösterilir + "çevrimdışı" durumu (madde 11); sessiz değil,
//!   "şu an haber yüklenemiyor — tekrar dene" sunulur (madde 4).
//!
//! **MVP kapsamı:** gerçek HTTP/RSS istemcisi [`HaberKaynagi`] trait'inin arkasında durur;
//! MVP'de küratörlü örnek akış ([`YerelKaynak`]) kullanılır (uzak JSON akışının yerel karşılığı).
//! Gerçek ağ getirme (`ureq`/`reqwest` ince adaptörü) İP-15/İP-18'de bağlanır — `MVP-sonrasi.md`
//! §10.2.  Bu modülün asenkron + önbellek + iskelet + çevrimdışı mantığı **şimdi gerçektir**.

use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use biocraft_types::{ErrorReport, Timestamp};
use serde::{Deserialize, Serialize};

/// Bir haberin türü (rozet/filtre için).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HaberTuru {
    /// Bilim/araştırma haberi.
    BilimHaberi,
    /// Şirket duyurusu (etkinlik, ortaklık vb.).
    SirketDuyurusu,
    /// Sürüm notu / changelog (yeni özellikler).
    SurumNotu,
}

impl HaberTuru {
    /// Kısa, yerelleştirilmiş etiket.
    pub fn etiket(&self, tr: bool) -> &'static str {
        match (self, tr) {
            (HaberTuru::BilimHaberi, true) => "Bilim",
            (HaberTuru::BilimHaberi, false) => "Science",
            (HaberTuru::SirketDuyurusu, true) => "Duyuru",
            (HaberTuru::SirketDuyurusu, false) => "News",
            (HaberTuru::SurumNotu, true) => "Sürüm",
            (HaberTuru::SurumNotu, false) => "Release",
        }
    }
}

/// Tek bir haber/duyuru öğesi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Haber {
    /// Başlık.
    pub baslik: String,
    /// Kısa özet.
    pub ozet: String,
    /// Kaynak adı ("BioCraft", "Nature" vb.).
    pub kaynak: String,
    /// Tıklanınca açılacak dış bağlantı (opsiyonel; açmadan önce kullanıcı onayı istenir).
    #[serde(default)]
    pub baglanti: Option<String>,
    /// İnsan-okur tarih metni ("2026-06-20").
    pub tarih: String,
    /// Küratörlü/doğrulanmış kaynak mı (rozet)?
    #[serde(default)]
    pub dogrulanmis: bool,
    /// Haber türü.
    pub tur: HaberTuru,
}

/// Bir haber akışının tamamı (önbelleğe yazılan birim).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HaberAkisi {
    /// Akıştaki haberler (en yeni en üstte beklenir).
    pub haberler: Vec<Haber>,
    /// Bu akışın en son ne zaman çekildiği (önbellek tazeliği göstergesi).
    #[serde(default)]
    pub son_guncelleme: Option<Timestamp>,
}

impl HaberAkisi {
    /// Akış boş mu?
    pub fn bos_mu(&self) -> bool {
        self.haberler.is_empty()
    }
}

/// Haber akışını sağlayan kaynak (gerçek ağ bunun arkasındadır → test-edilebilir + değiştirilebilir).
///
/// `Send`: akış ayrı bir thread'de çekildiği için kaynak thread'e taşınabilmelidir.
pub trait HaberKaynagi: Send {
    /// Akışı getirir (ağ/dosya).  Başarısızlık standart [`ErrorReport`] döner (panik yok).
    fn getir(&self) -> Result<HaberAkisi, ErrorReport>;
}

/// MVP küratörlü örnek akış — uzak JSON akışının yerel karşılığı.
///
/// `gecikme`: gerçek ağ gecikmesini taklit eder (iskelet/asenkron yolu canlı göstermek için).
pub struct YerelKaynak {
    gecikme: Duration,
    now: Timestamp,
}

impl YerelKaynak {
    /// Belirtilen taklit gecikmesiyle bir kaynak kurar.
    pub fn yeni(gecikme: Duration, now: Timestamp) -> Self {
        Self { gecikme, now }
    }
}

impl HaberKaynagi for YerelKaynak {
    fn getir(&self) -> Result<HaberAkisi, ErrorReport> {
        if !self.gecikme.is_zero() {
            thread::sleep(self.gecikme); // ayrı thread'de; arayüzü etkilemez.
        }
        Ok(varsayilan_akis(self.now))
    }
}

/// Küratörlü örnek akış içeriği (MVP).  Gerçek küratörlü uzak akış İP-18'de olgunlaşır.
pub fn varsayilan_akis(now: Timestamp) -> HaberAkisi {
    HaberAkisi {
        haberler: vec![
            Haber {
                baslik: "BioCraft Engine — Faz 2 başladı".into(),
                ozet: "Launcher, proje yöneticisi ve gizlilik/güvenlik sertleştirmesi geliyor."
                    .into(),
                kaynak: "BioCraft".into(),
                baglanti: Some("https://biocraftengine.com/blog/faz-2".into()),
                tarih: "2026-06-21".into(),
                dogrulanmis: true,
                tur: HaberTuru::SirketDuyurusu,
            },
            Haber {
                baslik: "Sürüm 0.1 — eklenti host'u tamamlandı".into(),
                ozet: "WASM sandbox + Ed25519 imza + çevrimdışı .bcext kurulum hazır.".into(),
                kaynak: "BioCraft".into(),
                baglanti: Some("https://biocraftengine.com/changelog".into()),
                tarih: "2026-06-21".into(),
                dogrulanmis: true,
                tur: HaberTuru::SurumNotu,
            },
            Haber {
                baslik: "Genom görselleştirmede yeni yöntemler".into(),
                ozet: "Out-of-core akış ile milyarlarca bazlık veri akıcı görüntülenebiliyor."
                    .into(),
                kaynak: "Bilim Bülteni".into(),
                baglanti: None,
                tarih: "2026-06-18".into(),
                dogrulanmis: true,
                tur: HaberTuru::BilimHaberi,
            },
        ],
        son_guncelleme: Some(now),
    }
}

/// Haber yüklemenin asenkron durum makinesi (arayüzde gösterilir).
#[derive(Debug, Clone)]
pub enum HaberDurumu {
    /// Arka planda çekiliyor → view iskelet gösterir (madde 6).
    Yukleniyor,
    /// Başarıyla yüklendi (taze akış).
    Yuklendi(HaberAkisi),
    /// Ağ başarısız ama **önbellek** var → çevrimdışı göster (madde 11).
    Cevrimdisi(HaberAkisi),
    /// Ne taze ne önbellek; hata + "tekrar dene" (madde 4).
    Hata(ErrorReport),
}

impl HaberDurumu {
    /// Şu an iskelet gösterilmeli mi (yükleniyor)?
    pub fn yukleniyor_mu(&self) -> bool {
        matches!(self, HaberDurumu::Yukleniyor)
    }

    /// Gösterilecek akış (yüklendi veya çevrimdışı önbellek); yoksa `None`.
    pub fn akis(&self) -> Option<&HaberAkisi> {
        match self {
            HaberDurumu::Yuklendi(a) | HaberDurumu::Cevrimdisi(a) => Some(a),
            _ => None,
        }
    }

    /// Çevrimdışı (önbellekten) mi gösteriliyor?
    pub fn cevrimdisi_mi(&self) -> bool {
        matches!(self, HaberDurumu::Cevrimdisi(_))
    }
}

/// Asenkron haber yükleyici: ayrı thread'de çeker, arayüz kanalı bloklamadan yoklar (MK-07).
pub struct HaberYukleyici {
    durum: HaberDurumu,
    /// Arka plan thread'inden sonucu taşıyan kanal (None = aktif yükleme yok).
    alici: Option<Receiver<Result<HaberAkisi, ErrorReport>>>,
    /// Ağ başarısız olursa düşülecek önbellek akışı (varsa).
    onbellek: Option<HaberAkisi>,
}

impl HaberYukleyici {
    /// Bir yükleyici kurar; `onbellek` varsa hata durumunda ona düşülür (çevrimdışı).
    pub fn yeni(onbellek: Option<HaberAkisi>) -> Self {
        Self {
            durum: HaberDurumu::Yukleniyor,
            alici: None,
            onbellek,
        }
    }

    /// Arka planda çekmeyi başlatır (arayüzü bloklamaz; thread spawn edilir).
    ///
    /// Tekrar çağrılırsa (örn. "tekrar dene") yeni bir çekme başlatılır.
    pub fn baslat<K: HaberKaynagi + 'static>(&mut self, kaynak: K) {
        let (gonderen, alici) = std::sync::mpsc::channel();
        thread::Builder::new()
            .name("biocraft-haber".into())
            .spawn(move || {
                let _ = gonderen.send(kaynak.getir()); // alıcı kapanmışsa sessizce yut.
            })
            .map(|_| ())
            .unwrap_or_else(|e| {
                // Thread başlatılamadıysa (çok nadir): sentetik hata kanala konamaz; logla.
                log::warn!("Haber yükleme thread'i başlatılamadı: {e}");
            });
        self.alici = Some(alici);
        self.durum = HaberDurumu::Yukleniyor;
    }

    /// Kanalı **bloklamadan** yoklar (her karede çağrılır).  Durum değiştiyse `true` döner.
    ///
    /// Sonuç geldiyse: başarıda `Yuklendi`; hatada önbellek varsa `Cevrimdisi`, yoksa `Hata`.
    pub fn yokla(&mut self) -> bool {
        let Some(alici) = self.alici.as_ref() else {
            return false;
        };
        match alici.try_recv() {
            Ok(Ok(akis)) => {
                self.onbellek = Some(akis.clone()); // taze veri yeni önbellek olur.
                self.durum = HaberDurumu::Yuklendi(akis);
                self.alici = None;
                true
            }
            Ok(Err(e)) => {
                self.durum = match self.onbellek.clone() {
                    Some(o) => HaberDurumu::Cevrimdisi(o),
                    None => HaberDurumu::Hata(e),
                };
                self.alici = None;
                true
            }
            Err(TryRecvError::Empty) => false, // henüz gelmedi → yükleniyor (bloklamaz).
            Err(TryRecvError::Disconnected) => {
                // Thread sonuç göndermeden öldü → önbellek veya hata.
                self.durum = match self.onbellek.clone() {
                    Some(o) => HaberDurumu::Cevrimdisi(o),
                    None => HaberDurumu::Hata(ErrorReport::new(
                        "Haberler yüklenemedi",
                        "Haber yükleme işlemi beklenmedik şekilde sonlandı.",
                        "İnternet bağlantınızı kontrol edip 'Tekrar Dene' deyin.",
                    )),
                };
                self.alici = None;
                true
            }
        }
    }

    /// Güncel durum (view bunu okur).
    pub fn durum(&self) -> &HaberDurumu {
        &self.durum
    }

    /// Aktif bir arka plan yüklemesi sürüyor mu?
    pub fn yukleme_aktif(&self) -> bool {
        self.alici.is_some()
    }
}

// ─── Testler ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::time::Instant;

    struct SahteKaynak {
        sonuc: Result<HaberAkisi, ErrorReport>,
    }
    impl HaberKaynagi for SahteKaynak {
        fn getir(&self) -> Result<HaberAkisi, ErrorReport> {
            self.sonuc.clone()
        }
    }

    /// Kanal gelene kadar bloklamadan yokla (testte kısa döngü; gerçek arayüz kare kare yoklar).
    fn yoklayarak_bekle(y: &mut HaberYukleyici) {
        let baslangic = Instant::now();
        while y.yukleme_aktif() && baslangic.elapsed() < Duration::from_secs(5) {
            if y.yokla() {
                break;
            }
            thread::yield_now();
        }
    }

    #[test]
    fn baslangicta_yukleniyor() {
        let y = HaberYukleyici::yeni(None);
        assert!(y.durum().yukleniyor_mu());
    }

    #[test]
    fn basarili_cekme_yuklendi_olur() {
        let mut y = HaberYukleyici::yeni(None);
        let akis = varsayilan_akis(Utc::now());
        y.baslat(SahteKaynak {
            sonuc: Ok(akis.clone()),
        });
        yoklayarak_bekle(&mut y);
        assert!(matches!(y.durum(), HaberDurumu::Yuklendi(_)));
        assert_eq!(
            y.durum().akis().unwrap().haberler.len(),
            akis.haberler.len()
        );
    }

    #[test]
    fn hata_ve_onbellek_cevrimdisi_olur() {
        let onbellek = varsayilan_akis(Utc::now());
        let mut y = HaberYukleyici::yeni(Some(onbellek.clone()));
        y.baslat(SahteKaynak {
            sonuc: Err(ErrorReport::new("ağ", "yok", "tekrar")),
        });
        yoklayarak_bekle(&mut y);
        assert!(y.durum().cevrimdisi_mi(), "önbellek varsa çevrimdışı");
        assert_eq!(
            y.durum().akis().unwrap().haberler.len(),
            onbellek.haberler.len()
        );
    }

    #[test]
    fn hata_ve_onbellek_yok_hata_olur() {
        let mut y = HaberYukleyici::yeni(None);
        y.baslat(SahteKaynak {
            sonuc: Err(ErrorReport::new("ağ", "yok", "tekrar")),
        });
        yoklayarak_bekle(&mut y);
        assert!(matches!(y.durum(), HaberDurumu::Hata(_)));
        assert!(y.durum().akis().is_none());
    }

    #[test]
    fn bos_yokla_bloklamaz() {
        // Hiç yükleme başlamadıysa yokla() anında false döner (arayüz donmaz).
        let mut y = HaberYukleyici::yeni(None);
        assert!(!y.yokla());
    }

    #[test]
    fn yerel_kaynak_varsayilan_akis_dolu() {
        let k = YerelKaynak::yeni(Duration::ZERO, Utc::now());
        let akis = k.getir().unwrap();
        assert!(!akis.bos_mu());
        assert!(akis.haberler.iter().all(|h| !h.baslik.is_empty()));
        assert!(
            akis.haberler.iter().any(|h| h.dogrulanmis),
            "küratörlü kaynakta doğrulanmış rozet var"
        );
    }

    #[test]
    fn akis_serde_gidis_donus() {
        let akis = varsayilan_akis(Utc::now());
        let baytlar = serde_json::to_vec(&akis).unwrap();
        let geri: HaberAkisi = serde_json::from_slice(&baytlar).unwrap();
        assert_eq!(akis, geri);
    }
}
