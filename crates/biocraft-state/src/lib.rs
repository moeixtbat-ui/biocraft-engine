//! biocraft-state — L2: Uygulama durumu, otomatik kayıt, çökme kurtarma, geri-al/yinele
//! (MK-28, MK-36, MK-37, MK-38).
//!
//! İş kaybını önleyen **self-healing durum altyapısı** (İP-11).  Güvenceler:
//! - **Kalıcı durum (MK-38):** Açık sekmeler, panel boyutları, görünüm (tema), dil/tercih ayrı bir
//!   yapıda ([`UygulamaDurumu`]) tutulur ve her açılışta geri yüklenir (egui immediate-mode olduğu
//!   için kalıcı durum ekrandan ayrı yaşamak zorundadır).
//! - **Otomatik kayıt (MK-28 kural 1):** Periyodik + değişiklikte ([`autosave`]); diske **atomik**
//!   ve **BLAKE3 bütünlük** mühürlü yazılır ([`store`]) → yarım yazma/bozulma olmaz.
//! - **Çökme kurtarma (MK-28 kural 2/3):** Temiz-kapanış bayrağıyla çökme tespit edilir
//!   ([`recovery`]); çökme sonrası açılışta "kurtarılan oturum" sunulur, durum yine yüklenir.
//! - **Geri-al / yinele (MK-36):** Düzenlenebilir **her** işlem bir ters-işlemli [`command::Komut`]
//!   olarak ifade edilir; [`undo::GeriAlYigini`] çok-adımlı geçmiş tutar.  Genel arayüz: sonraki
//!   paketler (node/kod/ayar/görünüm) kendi modelleriyle aynı motoru kullanır.
//! - **Çakışma tespiti (madde 18):** Aynı dosya iki yerde değişirse [`conflict`] uyarır ve sürüm
//!   seçimi sunar (sessiz ezme yok).
//! - **Yerel geçmiş:** Zaman damgalı anlık görüntüler ([`history`]; temel düzey, git sonra).
//!
//! MK-37: Her yazma/komut **tek mantıksal depoya** dokunur; "çok-depoda tek atomik işlem" vaat
//! edilmez ([`command::BirlesikKomut`] farklı depoları birleştirmeyi reddeder).
//! MK-40: L2 — yalnızca L0/L1'e bağlı; üst katman yasak.

// İP-16 standart hata şeması `ErrorReport` bilinçli olarak zengindir (ne/neden/çözüm + teknik
// detay + correlation_id).  Bu yüzden `Result<_, ErrorReport>` büyük Err taşır; depo/durum API'leri
// hata yolunda değil mutlu yolda optimize edildiğinden bu kabul edilebilir (biocraft-mem ile aynı).
#![allow(clippy::result_large_err)]

// MK-40: L2 katmanı — yalnızca L0/L1 katmanlarına bağlı.
pub use biocraft_sdk;
pub use biocraft_types;

pub mod autosave;
pub mod command;
pub mod conflict;
pub mod durum_komutlari;
pub mod history;
pub mod recovery;
pub mod state;
pub mod store;
pub mod undo;

use std::time::Instant;

use biocraft_types::{ErrorReport, Timestamp};

pub use autosave::OtomatikKayit;
pub use command::{BirlesikKomut, DepoKimligi, Komut};
pub use conflict::{
    damgala, CakismaBilgisi, CakismaIzleyici, CakismaKarari, CozumSecimi, SurumDamgasi,
};
pub use durum_komutlari::{
    DilDegistir, PanelGenisligiDegistir, SekmeEkle, SekmeKapat, TemaDegistir, DEPO_UYGULAMA_DURUMU,
};
pub use history::{AnlikGoruntu, YerelGecmis, VARSAYILAN_GECMIS_DERINLIGI};
pub use recovery::KurtarmaKarari;
pub use state::{
    AcikSekme, DilSecimi, PanelDurumu, PencereDurumu, TemaSecimi, UygulamaDurumu, DURUM_SURUMU,
};
pub use store::{DosyaDepo, KaliciDepo};
pub use undo::{GeriAlYigini, VARSAYILAN_DERINLIK};

/// Şu anki UTC zaman damgası — yerel geçmiş/çakışma damgaları için tek geçit.
///
/// Çekirdek mantık zaman damgasını **dışarıdan** alır (sahte saatle test edilebilirlik); host bu
/// yardımcıyı çağırarak gerçek saati enjekte eder (böylece üst katmanlar `chrono`'ya bağlanmaz).
pub fn simdi() -> Timestamp {
    chrono::Utc::now()
}

/// Kalıcı uygulama durumunun depo anahtarı.
pub const ANAHTAR_DURUM: &str = "uygulama_durumu";

/// Durum altyapısının çağıran-yüzü: depo + durum + otomatik kayıt politikasını birleştirir.
///
/// Kullanım (host): açılışta [`DurumYoneticisi::ac`] → her karede [`DurumYoneticisi::belki_kaydet`]
/// → değişiklikte [`DurumYoneticisi::durum_guncelle`] → kapanışta [`DurumYoneticisi::temiz_kapat`].
pub struct DurumYoneticisi {
    depo: Box<dyn KaliciDepo>,
    durum: UygulamaDurumu,
    otomatik: OtomatikKayit,
}

impl DurumYoneticisi {
    /// Depoyu açar: kalıcı durumu yükler (bozuksa güvenli varsayılana **düşer** — MK-28 kural 2,
    /// çökme yerine degrade) ve çökme kurtarma kararını üretir.
    ///
    /// `simdi`: otomatik kayıt zamanlayıcısının referans anı (`Instant::now()`).
    pub fn ac(depo: Box<dyn KaliciDepo>, simdi: Instant) -> (Self, KurtarmaKarari) {
        let durum = match depo.oku(ANAHTAR_DURUM) {
            Ok(Some(baytlar)) => match UygulamaDurumu::serde_oku(&baytlar) {
                Ok(d) => d,
                Err(e) => {
                    // Bozuk/eski durum: çökmek yerine varsayılanla aç + kullanıcıyı bilgilendir.
                    log::warn!(
                        "Kayıtlı durum okunamadı, varsayılana dönülüyor: {} [{}]",
                        e.neden,
                        e.correlation_id.kisa()
                    );
                    UygulamaDurumu::default()
                }
            },
            Ok(None) => UygulamaDurumu::default(), // ilk açılış.
            Err(e) => {
                log::warn!(
                    "Durum deposu okunamadı, varsayılana dönülüyor: {} [{}]",
                    e.neden,
                    e.correlation_id.kisa()
                );
                UygulamaDurumu::default()
            }
        };

        // Çökme tespiti + bu oturumu "çalışıyor" işaretle.
        let karar = recovery::acilis_kontrol(depo.as_ref()).unwrap_or_else(|e| {
            log::warn!(
                "Oturum bayrağı yazılamadı; kurtarma sunulmayacak: {} [{}]",
                e.neden,
                e.correlation_id.kisa()
            );
            KurtarmaKarari::TemizAcilis
        });

        let yonetici = Self {
            depo,
            durum,
            otomatik: OtomatikKayit::varsayilan(simdi),
        };
        (yonetici, karar)
    }

    /// Yüklenmiş/aktif kalıcı duruma salt-okunur erişim.
    pub fn durum(&self) -> &UygulamaDurumu {
        &self.durum
    }

    /// Durumu değiştirir ve **kirli** işaretler (otomatik kayıt zamanlayıcısını başlatır).
    pub fn durum_guncelle(&mut self, f: impl FnOnce(&mut UygulamaDurumu), simdi: Instant) {
        f(&mut self.durum);
        self.otomatik.degisiklik_oldu(simdi);
    }

    /// Kaydedilmemiş değişiklik var mı?
    pub fn kirli_mi(&self) -> bool {
        self.otomatik.kirli_mi()
    }

    /// Otomatik kayıt zamanı geldiyse diske yazar.  Döndürdüğü `bool`: kayıt yapıldı mı.
    ///
    /// Her karede ucuza çağrılabilir; çoğu karede `kaydetmeli` `false` döner.
    pub fn belki_kaydet(&mut self, simdi: Instant) -> Result<bool, ErrorReport> {
        if self.otomatik.kaydetmeli(simdi) {
            self.kaydet()?;
            self.otomatik.kaydedildi(simdi);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Durumu hemen diske yazar (zamanlayıcıdan bağımsız) — kapanış/kritik anlar için.
    pub fn kaydet(&self) -> Result<(), ErrorReport> {
        let baytlar = self.durum.serde_yaz()?;
        self.depo.yaz(ANAHTAR_DURUM, &baytlar)
    }

    /// Düzgün kapanış: durumu kaydeder ve oturumu **temiz** işaretler (sonraki açılışta kurtarma yok).
    pub fn temiz_kapat(&mut self, simdi: Instant) -> Result<(), ErrorReport> {
        self.kaydet()?;
        self.otomatik.kaydedildi(simdi);
        recovery::temiz_kapat(self.depo.as_ref())
    }
}

// ─── Testler ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    static SAYAC: AtomicU64 = AtomicU64::new(0);

    /// Benzersiz bir geçici test klasörü üretir (sürece + sayaca göre çakışmasız).
    fn gecici_kok(ad: &str) -> PathBuf {
        let n = SAYAC.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!(
            "biocraft_state_test_{}_{}_{}",
            ad,
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn temizle(p: &PathBuf) {
        let _ = std::fs::remove_dir_all(p);
    }

    // ── store: atomik yazma + BLAKE3 bütünlük ──────────────────────────────

    #[test]
    fn store_yaz_oku_gidis_donus() {
        let kok = gecici_kok("yaz_oku");
        let depo = DosyaDepo::yeni(&kok);
        depo.yaz("a", b"merhaba dunya").unwrap();
        assert_eq!(
            depo.oku("a").unwrap().as_deref(),
            Some(&b"merhaba dunya"[..])
        );
        temizle(&kok);
    }

    #[test]
    fn store_olmayan_anahtar_none() {
        let kok = gecici_kok("none");
        let depo = DosyaDepo::yeni(&kok);
        assert!(depo.oku("yok").unwrap().is_none());
        assert!(!depo.var_mi("yok"));
        temizle(&kok);
    }

    #[test]
    fn store_uzerine_yazma_replace() {
        let kok = gecici_kok("replace");
        let depo = DosyaDepo::yeni(&kok);
        depo.yaz("a", b"eski").unwrap();
        depo.yaz("a", b"yeni surum daha uzun").unwrap();
        assert_eq!(
            depo.oku("a").unwrap().as_deref(),
            Some(&b"yeni surum daha uzun"[..])
        );
        temizle(&kok);
    }

    #[test]
    fn store_atomik_yazma_gecici_dosya_birakmaz() {
        let kok = gecici_kok("atomik");
        let depo = DosyaDepo::yeni(&kok);
        depo.yaz("a", b"veri").unwrap();
        // Klasörde yalnızca hedef .bcs kalmalı; .tmp- artığı OLMAMALI.
        let girdiler: Vec<String> = std::fs::read_dir(&kok)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(
            girdiler.len(),
            1,
            "tam olarak bir dosya kalmalı: {girdiler:?}"
        );
        assert!(girdiler[0].ends_with(".bcs"));
        assert!(
            !girdiler.iter().any(|g| g.contains(".tmp-")),
            "geçici dosya artığı kalmamalı"
        );
        temizle(&kok);
    }

    #[test]
    fn store_bozulma_tespit_edilir() {
        let kok = gecici_kok("bozulma");
        let depo = DosyaDepo::yeni(&kok);
        depo.yaz("a", b"dogru veri").unwrap();
        // Dosyanın yük baytlarını boz (BLAKE3 tutmayacak).
        let dosya = kok.join("a.bcs");
        let mut ham = std::fs::read(&dosya).unwrap();
        let son = ham.len() - 1;
        ham[son] ^= 0xFF;
        std::fs::write(&dosya, &ham).unwrap();
        // Okuma sessizce yanlış veri döndürmemeli; bozulmayı bildirmeli.
        assert!(depo.oku("a").is_err(), "bozulma Err olarak raporlanmalı");
        temizle(&kok);
    }

    #[test]
    fn store_baslik_bozulursa_hata() {
        let kok = gecici_kok("baslik");
        let depo = DosyaDepo::yeni(&kok);
        let dosya = kok.join("a.bcs");
        std::fs::write(&dosya, b"COP gecersiz icerik").unwrap();
        assert!(depo.oku("a").is_err());
        temizle(&kok);
    }

    #[test]
    fn store_sil_calisir() {
        let kok = gecici_kok("sil");
        let depo = DosyaDepo::yeni(&kok);
        depo.yaz("a", b"x").unwrap();
        assert!(depo.var_mi("a"));
        depo.sil("a").unwrap();
        assert!(!depo.var_mi("a"));
        depo.sil("a").unwrap(); // ikinci silme de sorunsuz.
        temizle(&kok);
    }

    // ── state: serde + varsayılan + göç ────────────────────────────────────

    #[test]
    fn state_varsayilan_makul() {
        let d = UygulamaDurumu::default();
        assert_eq!(d.surum, DURUM_SURUMU);
        assert_eq!(d.tema, TemaSecimi::Koyu);
        assert_eq!(d.dil, DilSecimi::Tr);
        assert_eq!(d.pencere.genislik, 1280);
        assert!(d.panel.sag_panel_acik);
        assert!(!d.kaydedilmemis_var());
    }

    #[test]
    fn state_serde_gidis_donus() {
        let mut d = UygulamaDurumu {
            tema: TemaSecimi::YuksekKontrast,
            dil: DilSecimi::En,
            aktif_sekme: Some(0),
            panel: PanelDurumu {
                sag_panel_genislik: 410.0,
                ..Default::default()
            },
            ..Default::default()
        };
        d.sekmeler.push(AcikSekme {
            yol: Some("ornek/genom.fasta".into()),
            baslik: "genom.fasta".into(),
            kaydedilmemis: true,
        });
        let baytlar = d.serde_yaz().unwrap();
        let geri = UygulamaDurumu::serde_oku(&baytlar).unwrap();
        assert_eq!(d, geri);
        assert!(geri.kaydedilmemis_var());
    }

    #[test]
    fn state_eski_surum_gocer() {
        // surum=0 (eski) bir durum → okunduğunda güncel sürüme yükseltilmeli.
        let json = br#"{"surum":0,"pencere":{"genislik":800,"yukseklik":600,"buyutulmus":false},
            "tema":"Acik","dil":"En","panel":{"sag_panel_acik":true,"sag_panel_genislik":300.0},
            "sekmeler":[],"aktif_sekme":null,"tercihler":{}}"#;
        let d = UygulamaDurumu::serde_oku(json).unwrap();
        assert_eq!(d.surum, DURUM_SURUMU, "eski sürüm güncel damgaya taşınmalı");
        assert_eq!(d.tema, TemaSecimi::Acik);
    }

    // ── autosave: zamanlama politikası (sahte saat) ────────────────────────

    #[test]
    fn autosave_temizken_kaydetmez() {
        let t0 = Instant::now();
        let a = OtomatikKayit::varsayilan(t0);
        assert!(
            !a.kaydetmeli(t0 + Duration::from_secs(3600)),
            "değişiklik yokken kayıt yok"
        );
    }

    #[test]
    fn autosave_periyodik_tetikler() {
        let t0 = Instant::now();
        let mut a = OtomatikKayit::yeni(Duration::from_secs(30), Duration::from_secs(2), t0);
        a.degisiklik_oldu(t0);
        // 31 sn sonra periyodik kayıt zamanı.
        assert!(a.kaydetmeli(t0 + Duration::from_secs(31)));
    }

    #[test]
    fn autosave_degisiklik_debounce_tetikler() {
        let t0 = Instant::now();
        let mut a = OtomatikKayit::yeni(Duration::from_secs(30), Duration::from_secs(2), t0);
        a.degisiklik_oldu(t0 + Duration::from_secs(5));
        // Değişiklikten hemen sonra DEĞİL (debounce); 2 sn durulunca evet.
        assert!(
            !a.kaydetmeli(t0 + Duration::from_secs(6)),
            "debounce: hemen kaydetme"
        );
        assert!(
            a.kaydetmeli(t0 + Duration::from_secs(7)),
            "2 sn durulunca kaydet"
        );
    }

    #[test]
    fn autosave_kayittan_sonra_temiz() {
        let t0 = Instant::now();
        let mut a = OtomatikKayit::yeni(Duration::from_secs(30), Duration::from_secs(2), t0);
        a.degisiklik_oldu(t0);
        let t1 = t0 + Duration::from_secs(3);
        assert!(a.kaydetmeli(t1));
        a.kaydedildi(t1);
        assert!(!a.kirli_mi());
        // Yeni değişiklik olmadan tekrar kaydetmez.
        assert!(!a.kaydetmeli(t1 + Duration::from_secs(60)));
    }

    // ── recovery: temiz-kapanış bayrağı + çökme tespiti ────────────────────

    #[test]
    fn recovery_ilk_acilis_temiz() {
        let kok = gecici_kok("rec_ilk");
        let depo = DosyaDepo::yeni(&kok);
        let karar = recovery::acilis_kontrol(&depo).unwrap();
        assert_eq!(karar, KurtarmaKarari::TemizAcilis, "ilk açılış çökme değil");
        temizle(&kok);
    }

    #[test]
    fn recovery_temiz_kapanis_sonrasi_kurtarma_yok() {
        let kok = gecici_kok("rec_temiz");
        let depo = DosyaDepo::yeni(&kok);
        let _ = recovery::acilis_kontrol(&depo).unwrap(); // 1. oturum açıldı (çalışıyor).
        recovery::temiz_kapat(&depo).unwrap(); // düzgün kapandı.
        let karar = recovery::acilis_kontrol(&depo).unwrap(); // 2. oturum.
        assert_eq!(
            karar,
            KurtarmaKarari::TemizAcilis,
            "temiz kapanışta kurtarma sunulmaz"
        );
        temizle(&kok);
    }

    #[test]
    fn recovery_cokme_tespit_edilir() {
        let kok = gecici_kok("rec_cokme");
        let depo = DosyaDepo::yeni(&kok);
        let _ = recovery::acilis_kontrol(&depo).unwrap(); // oturum açıldı (çalışıyor)...
                                                          // ...ve temiz_kapat ÇAĞRILMADI (çökme/zorla kapatma simülasyonu).
        let karar = recovery::acilis_kontrol(&depo).unwrap(); // sonraki açılış.
        assert_eq!(
            karar,
            KurtarmaKarari::KurtarmaSunulur,
            "çökme sonrası kurtarma sunulur"
        );
        temizle(&kok);
    }

    // ── DurumYoneticisi: uçtan uca orkestrasyon ────────────────────────────

    #[test]
    fn yonetici_kalici_durum_oturumlar_arasi() {
        let kok = gecici_kok("yon_kalici");

        // 1. oturum: tema/dil/panel değiştir + kaydet + temiz kapan.
        {
            let (mut y, karar) =
                DurumYoneticisi::ac(Box::new(DosyaDepo::yeni(&kok)), Instant::now());
            assert_eq!(karar, KurtarmaKarari::TemizAcilis);
            y.durum_guncelle(
                |d| {
                    d.tema = TemaSecimi::Acik;
                    d.dil = DilSecimi::En;
                    d.panel.sag_panel_genislik = 444.0;
                    d.sekmeler.push(AcikSekme {
                        yol: None,
                        baslik: "isimsiz".into(),
                        kaydedilmemis: false,
                    });
                },
                Instant::now(),
            );
            y.temiz_kapat(Instant::now()).unwrap();
        }

        // 2. oturum: aynı kök → durum aynen geri gelmeli.
        {
            let (y, karar) = DurumYoneticisi::ac(Box::new(DosyaDepo::yeni(&kok)), Instant::now());
            assert_eq!(karar, KurtarmaKarari::TemizAcilis, "temiz kapanmıştı");
            assert_eq!(y.durum().tema, TemaSecimi::Acik);
            assert_eq!(y.durum().dil, DilSecimi::En);
            assert_eq!(y.durum().panel.sag_panel_genislik, 444.0);
            assert_eq!(y.durum().sekmeler.len(), 1);
        }
        temizle(&kok);
    }

    #[test]
    fn yonetici_cokme_sonrasi_durum_ve_kurtarma() {
        let kok = gecici_kok("yon_cokme");

        // 1. oturum: değişiklik yap + KAYDET ama temiz_kapat ÇAĞIRMA (çökme).
        {
            let (mut y, _) = DurumYoneticisi::ac(Box::new(DosyaDepo::yeni(&kok)), Instant::now());
            y.durum_guncelle(|d| d.tema = TemaSecimi::YuksekKontrast, Instant::now());
            y.kaydet().unwrap(); // otomatik kayıt diske yazdı...
                                 // ...ama oturum bayrağı "çalışıyor" kaldı (drop = çökme gibi).
        }

        // 2. oturum: hem durum korunmuş olmalı hem kurtarma sunulmalı.
        {
            let (y, karar) = DurumYoneticisi::ac(Box::new(DosyaDepo::yeni(&kok)), Instant::now());
            assert_eq!(
                karar,
                KurtarmaKarari::KurtarmaSunulur,
                "çökme tespit edilmeli"
            );
            assert_eq!(
                y.durum().tema,
                TemaSecimi::YuksekKontrast,
                "durum korunmalı"
            );
        }
        temizle(&kok);
    }

    #[test]
    fn yonetici_bozuk_durum_varsayilana_duser() {
        let kok = gecici_kok("yon_bozuk");
        // Durum dosyasını geçerli zarf ama geçersiz JSON yüküyle yaz.
        let depo = DosyaDepo::yeni(&kok);
        depo.yaz(ANAHTAR_DURUM, b"{ bu gecerli json degil }")
            .unwrap();
        // Açılış çökmeden varsayılana düşmeli (MK-28 kural 2: degrade).
        let (y, _) = DurumYoneticisi::ac(Box::new(DosyaDepo::yeni(&kok)), Instant::now());
        assert_eq!(
            y.durum().tema,
            TemaSecimi::Koyu,
            "bozuk durum → güvenli varsayılan"
        );
        temizle(&kok);
    }

    #[test]
    fn yonetici_belki_kaydet_periyodik() {
        let kok = gecici_kok("yon_belki");
        let t0 = Instant::now();
        let (mut y, _) = DurumYoneticisi::ac(Box::new(DosyaDepo::yeni(&kok)), t0);
        // Değişiklik yoksa kayıt yok.
        assert!(!y.belki_kaydet(t0 + Duration::from_secs(60)).unwrap());
        // Değişiklik + 3 sn → kayıt yapılır.
        y.durum_guncelle(|d| d.dil = DilSecimi::En, t0);
        assert!(y.belki_kaydet(t0 + Duration::from_secs(3)).unwrap());
        assert!(!y.kirli_mi(), "kayıttan sonra temiz");
        temizle(&kok);
    }
}
