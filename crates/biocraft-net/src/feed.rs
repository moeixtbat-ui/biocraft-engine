//! Bilim Pazarı **salt-okur içerik akışı** — İP-18 (asenkron, **arayüzü asla bloklamaz**).
//!
//! Mağaza öğeleri + haber/makale kartları küratörlü bir uzak akıştan (RSS/REST) gelir.  Bu modül,
//! launcher haber akışıyla ([`biocraft_launcher::news`] benzeri) **aynı dürüst eşzamanlılık
//! kalıbını** uygular ama **L3'tedir** (UI'siz, saf mantık; egui üst katmanda):
//! - Çekme ([`PazarKaynagi::getir`]) ayrı bir `std::thread`'de çalışır (Tokio değil — ağır bağımlılık
//!   eklemeden MK-07 "pull-based" eşzamanlılık).
//! - Arayüz her karede [`PazarYukleyici::yokla`] ile kanalı **bloklamadan** yoklar (`try_recv`).
//! - Yükleniyorken üst katman iskelet (skeleton) gösterir; gelince dolar (TDA madde 6).
//! - Ağ başarısızsa **son önbellek** gösterilir + "çevrimdışı" durumu (madde 11); sessiz değil,
//!   "şu an içerik yüklenemiyor — tekrar dene" sunulur (madde 4).
//!
//! **Salt-okur (MVP):** bu akış yalnızca **gelen** içeriği taşır; yayınlama/yorum yazma YOK.
//! Halihazırda onaylı bilgi etkilenmez (vizyon notu).  Gerçek HTTP/RSS istemcisi [`PazarKaynagi`]
//! trait'inin arkasındadır; MVP'de küratörlü örnek akış ([`YerelPazarKaynagi`]) kullanılır.  Gerçek
//! ağ getirme (`ureq`/`reqwest` ince adaptörü) sonradan eklenir — `MVP-sonrasi.md` §10.1/§10.2.

use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use biocraft_types::{ErrorReport, Timestamp};

use crate::katalog::{
    DogrulamaDurumu, Fiyat, HaberKarti, HaberTuru, Kategori, OgeTuru, PazarOgesi, PazarVerisi,
    Yorum,
};

/// Pazar içeriğini sağlayan kaynak (gerçek ağ bunun arkasındadır → test-edilebilir + değiştirilebilir).
///
/// `Send`: içerik ayrı bir thread'de çekildiği için kaynak thread'e taşınabilmelidir.
pub trait PazarKaynagi: Send {
    /// İçeriği getirir (ağ/dosya).  Başarısızlık standart [`ErrorReport`] döner (panik yok).
    fn getir(&self) -> Result<PazarVerisi, ErrorReport>;
}

/// MVP küratörlü örnek akış — uzak JSON/RSS akışının yerel karşılığı.
///
/// `gecikme`: gerçek ağ gecikmesini taklit eder (iskelet/asenkron yolu canlı göstermek için).
/// `basarisiz`: çevrimdışı/önbellek yolunu test/demoda zorlamak için ağ hatasını simüle eder.
pub struct YerelPazarKaynagi {
    gecikme: Duration,
    now: Timestamp,
    basarisiz: bool,
}

impl YerelPazarKaynagi {
    /// Belirtilen taklit gecikmesiyle bir kaynak kurar.
    pub fn yeni(gecikme: Duration, now: Timestamp) -> Self {
        Self {
            gecikme,
            now,
            basarisiz: false,
        }
    }

    /// Ağ hatasını simüle eden bir kaynak (çevrimdışı yolu göstermek için).
    pub fn basarisiz(now: Timestamp) -> Self {
        Self {
            gecikme: Duration::ZERO,
            now,
            basarisiz: true,
        }
    }
}

impl PazarKaynagi for YerelPazarKaynagi {
    fn getir(&self) -> Result<PazarVerisi, ErrorReport> {
        if !self.gecikme.is_zero() {
            thread::sleep(self.gecikme); // ayrı thread'de; arayüzü etkilemez.
        }
        if self.basarisiz {
            return Err(ErrorReport::new(
                "Pazar içeriği yüklenemedi",
                "Küratörlü içerik akışına şu an ulaşılamıyor.",
                "İnternet bağlantınızı kontrol edip 'Tekrar Dene' deyin.",
            )
            .with_eylem("Tekrar Dene"));
        }
        Ok(kuratorlu_veri(self.now))
    }
}

/// Küratörlü örnek pazar içeriği (MVP).  Gerçek küratörlü uzak akış sonradan olgunlaşır.
///
/// **Dürüstlük:** resmi BioCraft öğeleri [`DogrulamaDurumu::Resmi`]; topluluk öğeleri
/// **[`DogrulamaDurumu::IncelemeBekliyor`]** ("doğrulama: beklemede") — sahte "doğrulandı" YOK.
pub fn kuratorlu_veri(now: Timestamp) -> PazarVerisi {
    PazarVerisi {
        ogeler: vec![
            PazarOgesi {
                kimlik: "biocraft.studio.core".into(),
                ad: "BioCraft Studio".into(),
                yayinci: "BioCraft".into(),
                ozet: "Genom tarayıcı + varyant + 3B + veritabanı (çekirdek eklenti).".into(),
                aciklama: "Çekirdek analiz/görüntüleme eklentisi: genom tarayıcı, varyant inceleme, \
                           3B yapı görüntüleme ve veritabanı erişimi (BLAST/PDB/NCBI). Varsayılan \
                           kurulu gelir; out-of-core akışla milyarlarca bazlık veri akıcı işlenir."
                    .into(),
                tur: OgeTuru::Eklenti,
                kategori: Kategori::Analiz,
                surum: "0.1.0".into(),
                fiyat: Fiyat::AcikKaynak,
                lisans: "Apache-2.0".into(),
                atif: None,
                puan: 4.8,
                puan_sayisi: 124,
                indirme: 5230,
                son_guncelleme: "2026-06-21".into(),
                dogrulama: DogrulamaDurumu::Resmi,
                ekran_etiketleri: vec![
                    "Genom tarayıcı".into(),
                    "Varyant tablosu".into(),
                    "3B yapı".into(),
                ],
                yorumlar: vec![
                    Yorum::yeni(
                        "ada",
                        5,
                        "BAM dosyalarını akıcı açıyor; bellek sorunu yaşamadım.",
                        "2026-06-19",
                    ),
                    Yorum::yeni("kerem", 4, "3B görüntüleyici hızlı; daha çok format isterim.", "2026-06-15"),
                ],
                paket: None,
            },
            PazarOgesi {
                kimlik: "biocraft.dagitik-ag.net".into(),
                ad: "Dağıtık Ağ (P2P)".into(),
                yayinci: "BioCraft".into(),
                ozet: "Gönüllü dağıtık hesaplama ağı — kancalar çekirdekte, motor burada.".into(),
                aciklama: "İP-15 pasif kancalarını dolduran resmi eklenti: gerçek Iroh/QUIC bağlantısı, \
                           iş dağıtımı ve Bio-kredi ekonomisi. PHI asla ağa çıkamaz (sınır çekirdektedir)."
                    .into(),
                tur: OgeTuru::Eklenti,
                kategori: Kategori::Arac,
                surum: "0.1.0".into(),
                fiyat: Fiyat::AcikKaynak,
                lisans: "Apache-2.0".into(),
                atif: None,
                puan: 4.2,
                puan_sayisi: 31,
                indirme: 870,
                son_guncelleme: "2026-06-22".into(),
                dogrulama: DogrulamaDurumu::Resmi,
                ekran_etiketleri: vec!["Ağ durumu".into(), "Kaynak paylaşımı".into()],
                yorumlar: Vec::new(),
                paket: None,
            },
            PazarOgesi {
                kimlik: "io.acme.rnaseq-pro".into(),
                ad: "RNA-seq Pro".into(),
                yayinci: "Acme Bio".into(),
                ozet: "Diferansiyel ifade + yol analizi node paketi.".into(),
                aciklama: "RNA-seq say matrislerinden diferansiyel ifade ve yol zenginleştirme \
                           node'ları. Doğrulanmış yayıncı imzasıyla gelir (MK-16)."
                    .into(),
                tur: OgeTuru::Eklenti,
                kategori: Kategori::Analiz,
                surum: "2.3.1".into(),
                fiyat: Fiyat::Ucretli { bio_kredi: 12 },
                lisans: "Tescilli".into(),
                atif: None,
                puan: 4.5,
                puan_sayisi: 58,
                indirme: 1640,
                son_guncelleme: "2026-06-10".into(),
                dogrulama: DogrulamaDurumu::DogrulanmisYayinci,
                ekran_etiketleri: vec!["Volcano plot".into(), "Yol haritası".into()],
                yorumlar: vec![Yorum::yeni(
                    "deniz",
                    5,
                    "Yol analizi çok pratik; sonuçlar referans araçla uyumlu.",
                    "2026-06-12",
                )],
                paket: None,
            },
            PazarOgesi {
                kimlik: "community.varyant-sablonu".into(),
                ad: "Varyant İnceleme Şablonu".into(),
                yayinci: "topluluk:bio_lab".into(),
                ozet: "VCF yükle → filtrele → tablo → 3B; hazır iş akışı.".into(),
                aciklama: "Topluluk katkısı bir iş akışı şablonu. Henüz biçimsel doğrulama \
                           hattından geçmedi (durum: beklemede); kendi sorumluluğunuzla kullanın."
                    .into(),
                tur: OgeTuru::Sablon,
                kategori: Kategori::Gorsellestirme,
                surum: "1.1.0".into(),
                fiyat: Fiyat::Ucretsiz,
                lisans: "CC-BY-4.0".into(),
                atif: None,
                puan: 4.0,
                puan_sayisi: 12,
                indirme: 430,
                son_guncelleme: "2026-06-05".into(),
                dogrulama: DogrulamaDurumu::IncelemeBekliyor,
                ekran_etiketleri: vec!["Akış şeması".into()],
                yorumlar: vec![Yorum::yeni("ufuk", 4, "İyi başlangıç; filtre eşiklerini değiştirdim.", "2026-06-07")],
                paket: None,
            },
            PazarOgesi {
                kimlik: "data.ensembl-grch38-mini".into(),
                ad: "GRCh38 Mini Referans".into(),
                yayinci: "Ensembl (yansıma)".into(),
                ozet: "Küçültülmüş insan referans alt kümesi (öğretim/test).".into(),
                aciklama: "Öğretim ve test için küçültülmüş insan referans genomu alt kümesi. \
                           Açık lisans; atıf gereklidir."
                    .into(),
                tur: OgeTuru::VeriSeti,
                kategori: Kategori::Veritabani,
                surum: "2024.1".into(),
                fiyat: Fiyat::AcikKaynak,
                lisans: "CC-BY-4.0".into(),
                atif: Some("Ensembl release 111, GRCh38.p14 (alt küme).".into()),
                puan: 4.7,
                puan_sayisi: 22,
                indirme: 980,
                son_guncelleme: "2026-05-30".into(),
                dogrulama: DogrulamaDurumu::Kuratorlu,
                ekran_etiketleri: vec!["FASTA".into(), "İndeks".into()],
                yorumlar: Vec::new(),
                paket: None,
            },
        ],
        haberler: vec![
            HaberKarti {
                baslik: "BioCraft Engine — Faz 4: AI yüzeyi ve dağıtık ağ kancaları".into(),
                ozet: "AI sağlayıcı soyutlaması, dürüst çıktı şeması ve pasif P2P kancaları eklendi."
                    .into(),
                kaynak: "BioCraft".into(),
                tarih: "2026-06-22".into(),
                baglanti: Some("https://biocraftengine.com/blog/faz-4".into()),
                dogrulanmis: true,
                tur: HaberTuru::Duyuru,
            },
            HaberKarti {
                baslik: "Tek-hücre dizilemede yeni out-of-core yöntemler".into(),
                ozet: "Milyarlarca okumayı belleğe almadan akışla işleyen yaklaşımlar gözden geçirildi."
                    .into(),
                kaynak: "Nature Methods".into(),
                tarih: "2026-06-18".into(),
                baglanti: None,
                dogrulanmis: true,
                tur: HaberTuru::Makale,
            },
            HaberKarti {
                baslik: "GRCh38 referansına küçük güncelleme yayımlandı".into(),
                ozet: "Yeni yama sürümü ve düzeltmeler; pazar veri setleri güncellenecek.".into(),
                kaynak: "Ensembl".into(),
                tarih: "2026-06-14".into(),
                baglanti: Some("https://www.ensembl.org/info/website/news.html".into()),
                dogrulanmis: true,
                tur: HaberTuru::VeriSeti,
            },
            HaberKarti {
                baslik: "Sürüm 0.1 — eklenti host'u + imza + çevrimdışı kurulum".into(),
                ozet: "WASM sandbox, Ed25519 imza ve .bcext çevrimdışı kurulum hazır.".into(),
                kaynak: "BioCraft".into(),
                tarih: "2026-06-21".into(),
                baglanti: Some("https://biocraftengine.com/changelog".into()),
                dogrulanmis: true,
                tur: HaberTuru::SurumNotu,
            },
        ],
        son_guncelleme: Some(now),
    }
}

/// Pazar yüklemenin asenkron durum makinesi (üst katman bunu okur).
#[derive(Debug, Clone)]
pub enum PazarDurumu {
    /// Arka planda çekiliyor → üst katman iskelet gösterir (madde 6).
    Yukleniyor,
    /// Başarıyla yüklendi (taze içerik).
    Yuklendi(PazarVerisi),
    /// Ağ başarısız ama **önbellek** var → çevrimdışı göster (madde 11).
    Cevrimdisi(PazarVerisi),
    /// Ne taze ne önbellek; hata + "tekrar dene" (madde 4).
    Hata(ErrorReport),
}

impl PazarDurumu {
    /// Şu an iskelet gösterilmeli mi (yükleniyor)?
    pub fn yukleniyor_mu(&self) -> bool {
        matches!(self, PazarDurumu::Yukleniyor)
    }

    /// Gösterilecek içerik (yüklendi veya çevrimdışı önbellek); yoksa `None`.
    pub fn veri(&self) -> Option<&PazarVerisi> {
        match self {
            PazarDurumu::Yuklendi(v) | PazarDurumu::Cevrimdisi(v) => Some(v),
            _ => None,
        }
    }

    /// Çevrimdışı (önbellekten) mi gösteriliyor?
    pub fn cevrimdisi_mi(&self) -> bool {
        matches!(self, PazarDurumu::Cevrimdisi(_))
    }

    /// Varsa hata raporu (üst katman "tekrar dene" gösterir).
    pub fn hata(&self) -> Option<&ErrorReport> {
        match self {
            PazarDurumu::Hata(e) => Some(e),
            _ => None,
        }
    }
}

/// Asenkron pazar yükleyici: ayrı thread'de çeker, arayüz kanalı bloklamadan yoklar (MK-07).
///
/// Mantık launcher haber yükleyicisiyle birebir aynıdır; ortak bir kalıp olarak iki yerde de
/// kullanılır (mağaza/haber akışı; gerçek ağ adaptörü `PazarKaynagi`'nin arkasında).
pub struct PazarYukleyici {
    durum: PazarDurumu,
    /// Arka plan thread'inden sonucu taşıyan kanal (None = aktif yükleme yok).
    alici: Option<Receiver<Result<PazarVerisi, ErrorReport>>>,
    /// Ağ başarısız olursa düşülecek önbellek içeriği (varsa).
    onbellek: Option<PazarVerisi>,
}

impl PazarYukleyici {
    /// Bir yükleyici kurar; `onbellek` varsa hata durumunda ona düşülür (çevrimdışı).
    pub fn yeni(onbellek: Option<PazarVerisi>) -> Self {
        Self {
            durum: PazarDurumu::Yukleniyor,
            alici: None,
            onbellek,
        }
    }

    /// Arka planda çekmeyi başlatır (arayüzü bloklamaz; thread spawn edilir).
    ///
    /// Tekrar çağrılırsa (örn. "tekrar dene") yeni bir çekme başlatılır.
    pub fn baslat<K: PazarKaynagi + 'static>(&mut self, kaynak: K) {
        let (gonderen, alici) = std::sync::mpsc::channel();
        thread::Builder::new()
            .name("biocraft-pazar".into())
            .spawn(move || {
                let _ = gonderen.send(kaynak.getir()); // alıcı kapanmışsa sessizce yut.
            })
            .map(|_| ())
            .unwrap_or_else(|e| {
                log::warn!("Pazar yükleme thread'i başlatılamadı: {e}");
            });
        self.alici = Some(alici);
        self.durum = PazarDurumu::Yukleniyor;
    }

    /// Kanalı **bloklamadan** yoklar (her karede çağrılır).  Durum değiştiyse `true` döner.
    pub fn yokla(&mut self) -> bool {
        let Some(alici) = self.alici.as_ref() else {
            return false;
        };
        match alici.try_recv() {
            Ok(Ok(veri)) => {
                self.onbellek = Some(veri.clone()); // taze veri yeni önbellek olur.
                self.durum = PazarDurumu::Yuklendi(veri);
                self.alici = None;
                true
            }
            Ok(Err(e)) => {
                self.durum = match self.onbellek.clone() {
                    Some(o) => PazarDurumu::Cevrimdisi(o),
                    None => PazarDurumu::Hata(e),
                };
                self.alici = None;
                true
            }
            Err(TryRecvError::Empty) => false, // henüz gelmedi → yükleniyor (bloklamaz).
            Err(TryRecvError::Disconnected) => {
                self.durum = match self.onbellek.clone() {
                    Some(o) => PazarDurumu::Cevrimdisi(o),
                    None => PazarDurumu::Hata(ErrorReport::new(
                        "Pazar yüklenemedi",
                        "Pazar yükleme işlemi beklenmedik şekilde sonlandı.",
                        "İnternet bağlantınızı kontrol edip 'Tekrar Dene' deyin.",
                    )),
                };
                self.alici = None;
                true
            }
        }
    }

    /// Güncel durum (üst katman bunu okur).
    pub fn durum(&self) -> &PazarDurumu {
        &self.durum
    }

    /// Aktif bir arka plan yüklemesi sürüyor mu?
    pub fn yukleme_aktif(&self) -> bool {
        self.alici.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::time::Instant;

    struct SahteKaynak {
        sonuc: Result<PazarVerisi, ErrorReport>,
    }
    impl PazarKaynagi for SahteKaynak {
        fn getir(&self) -> Result<PazarVerisi, ErrorReport> {
            self.sonuc.clone()
        }
    }

    /// Kanal gelene kadar bloklamadan yokla (testte kısa döngü; gerçek arayüz kare kare yoklar).
    fn yoklayarak_bekle(y: &mut PazarYukleyici) {
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
        let y = PazarYukleyici::yeni(None);
        assert!(y.durum().yukleniyor_mu());
    }

    #[test]
    fn basarili_cekme_yuklendi_olur() {
        let mut y = PazarYukleyici::yeni(None);
        let veri = kuratorlu_veri(Utc::now());
        y.baslat(SahteKaynak {
            sonuc: Ok(veri.clone()),
        });
        yoklayarak_bekle(&mut y);
        assert!(matches!(y.durum(), PazarDurumu::Yuklendi(_)));
        assert_eq!(y.durum().veri().unwrap().ogeler.len(), veri.ogeler.len());
    }

    #[test]
    fn hata_ve_onbellek_cevrimdisi_olur() {
        let onbellek = kuratorlu_veri(Utc::now());
        let mut y = PazarYukleyici::yeni(Some(onbellek.clone()));
        y.baslat(SahteKaynak {
            sonuc: Err(ErrorReport::new("ağ", "yok", "tekrar")),
        });
        yoklayarak_bekle(&mut y);
        assert!(y.durum().cevrimdisi_mi(), "önbellek varsa çevrimdışı");
        assert_eq!(
            y.durum().veri().unwrap().ogeler.len(),
            onbellek.ogeler.len()
        );
    }

    #[test]
    fn hata_ve_onbellek_yok_hata_olur() {
        let mut y = PazarYukleyici::yeni(None);
        y.baslat(SahteKaynak {
            sonuc: Err(ErrorReport::new("ağ", "yok", "tekrar")),
        });
        yoklayarak_bekle(&mut y);
        assert!(matches!(y.durum(), PazarDurumu::Hata(_)));
        assert!(y.durum().veri().is_none());
    }

    #[test]
    fn bos_yokla_bloklamaz() {
        let mut y = PazarYukleyici::yeni(None);
        assert!(!y.yokla());
    }

    #[test]
    fn yerel_kaynak_kuratorlu_veri_dolu() {
        let k = YerelPazarKaynagi::yeni(Duration::ZERO, Utc::now());
        let veri = k.getir().unwrap();
        assert!(!veri.bos_mu());
        assert!(veri
            .ogeler
            .iter()
            .all(|o| !o.ad.is_empty() && !o.kimlik.is_empty()));
        // Küratörlü kaynakta en az bir resmi (imzaya dayalı) öğe + en az bir "beklemede" öğe olmalı.
        assert!(veri.ogeler.iter().any(|o| o.dogrulama.guven_rozeti_mi()));
        assert!(veri
            .ogeler
            .iter()
            .any(|o| matches!(o.dogrulama, DogrulamaDurumu::IncelemeBekliyor)));
        // Haber akışında küratörlü kaynak rozetli kartlar var.
        assert!(veri.haberler.iter().any(|h| h.dogrulanmis));
    }

    #[test]
    fn basarisiz_kaynak_hata_dondurur() {
        let k = YerelPazarKaynagi::basarisiz(Utc::now());
        assert!(k.getir().is_err());
    }

    #[test]
    fn veri_serde_gidis_donus() {
        let veri = kuratorlu_veri(Utc::now());
        let baytlar = serde_json::to_vec(&veri).unwrap();
        let geri: PazarVerisi = serde_json::from_slice(&baytlar).unwrap();
        assert_eq!(veri, geri);
    }
}
