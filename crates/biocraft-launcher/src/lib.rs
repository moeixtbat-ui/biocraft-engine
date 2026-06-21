//! biocraft-launcher — L4: Açılış istemcisi (İP-01).
//!
//! Uygulama açıldığında ilk görünen, motoru başlatan **Epic-benzeri** istemci.  Gösterdikleri:
//! son projeler, küratörlü bilim haberleri/şirket duyuruları (asenkron), donanım ön-kontrolü ve
//! "Yeni Proje / Proje Aç" eylemleri.  **Çevrimdışı da tam çalışır** (önbellek + durum göstergesi).
//!
//! Modüller:
//! - [`recent`] — son projeler (pin/arama/taşınmış-proje kurtarma; saf model).
//! - [`news`] — haber akışı (ayrı thread'de çeker, arayüzü bloklamaz; önbellek/çevrimdışı).
//! - [`hardware_check`] — donanım ön-kontrolü + yetenek matrisi (İP-08 profilini yeniden kullanır).
//! - [`launch`] — başlatma protokolü (motor argümanları) + launcher eylemleri.
//! - [`splash`] — açılış splash ekranı zamanlaması (E8; `--no-splash` ile atlanır).
//! - [`view`] — egui arayüzü (üst başlık + son projeler + haber + donanım kartı + splash).
//!
//! MK-40: L4 — L0/L1/L2/L3 ve aynı katman (biocraft-ui) hariç üst katman yasak.
//! MK-53: ortak TDA bileşenleri/token/i18n biocraft-ui'den yeniden kullanılır (kopyalanmaz).

// İP-16 zengin `ErrorReport` (büyük Err) bilinçli; mutlu yol optimize (mem/state ile aynı).
#![allow(clippy::result_large_err)]

pub mod hardware_check;
pub mod launch;
pub mod news;
pub mod recent;
pub mod splash;
pub mod view;

// Aynı katman (L4) — ortak bileşenleri/token'ları yeniden dışa aktar (üst katman tek sürüm görsün).
pub use biocraft_types;
pub use biocraft_ui;

pub use hardware_check::{DonanimDegerlendirme, ReferansDonanim, Yetenek, YetenekDurumu};
pub use launch::{BaslatmaArgumanlari, BaslatmaModu, LauncherEylem};
pub use news::{
    Haber, HaberAkisi, HaberDurumu, HaberKaynagi, HaberTuru, HaberYukleyici, YerelKaynak,
};
pub use recent::{proje_durumu, ProjeDurumu, SonProje, SonProjelerListesi};
pub use splash::{SplashDurumu, SPLASH_SURESI};
pub use view::LauncherDurumu;

use biocraft_types::ErrorReport;
use biocraft_ui::biocraft_state::KaliciDepo;

/// Son projeler listesinin kalıcı depo anahtarı.
pub const ANAHTAR_SON_PROJELER: &str = "son_projeler";
/// Haber akışı önbelleğinin kalıcı depo anahtarı (çevrimdışı için).
pub const ANAHTAR_HABER_ONBELLEK: &str = "haber_onbellek";

/// Son projeler listesini depodan yükler.  Yoksa/bozuksa **boş listeyle** döner (çökmez — degrade).
pub fn son_projeleri_yukle(depo: &dyn KaliciDepo) -> SonProjelerListesi {
    match depo.oku(ANAHTAR_SON_PROJELER) {
        Ok(Some(baytlar)) => SonProjelerListesi::serde_oku(&baytlar).unwrap_or_else(|e| {
            log::warn!(
                "Son projeler okunamadı, boş listeyle açılıyor: {} [{}]",
                e.neden,
                e.correlation_id.kisa()
            );
            SonProjelerListesi::yeni()
        }),
        Ok(None) => SonProjelerListesi::yeni(),
        Err(e) => {
            log::warn!(
                "Son projeler deposu okunamadı: {} [{}]",
                e.neden,
                e.correlation_id.kisa()
            );
            SonProjelerListesi::yeni()
        }
    }
}

/// Son projeler listesini depoya yazar (atomik + BLAKE3 bütünlük — `DosyaDepo`).
pub fn son_projeleri_kaydet(
    depo: &dyn KaliciDepo,
    liste: &SonProjelerListesi,
) -> Result<(), ErrorReport> {
    let baytlar = liste.serde_yaz()?;
    depo.yaz(ANAHTAR_SON_PROJELER, &baytlar)
}

/// Haber önbelleğini depodan yükler (çevrimdışı ilk gösterim için).  Yoksa/bozuksa `None`.
pub fn haber_onbellek_yukle(depo: &dyn KaliciDepo) -> Option<HaberAkisi> {
    match depo.oku(ANAHTAR_HABER_ONBELLEK) {
        Ok(Some(baytlar)) => serde_json::from_slice(&baytlar).ok(),
        _ => None,
    }
}

/// Taze haber akışını önbelleğe yazar (sonraki açılışta/çevrimdışıyken gösterilir).
pub fn haber_onbellek_kaydet(depo: &dyn KaliciDepo, akis: &HaberAkisi) -> Result<(), ErrorReport> {
    let baytlar = serde_json::to_vec(akis).map_err(|e| {
        ErrorReport::new(
            "Haber önbelleği kaydedilemedi",
            format!("Akış JSON'a çevrilemedi: {e}"),
            "Önemli değil; haberler bir sonraki açılışta yeniden çekilir.",
        )
    })?;
    depo.yaz(ANAHTAR_HABER_ONBELLEK, &baytlar)
}

// ─── Testler ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use biocraft_ui::biocraft_state::DosyaDepo;
    use chrono::Utc;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SAYAC: AtomicU64 = AtomicU64::new(0);

    fn gecici_kok(ad: &str) -> std::path::PathBuf {
        let n = SAYAC.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!(
            "biocraft_launcher_test_{}_{}_{}",
            ad,
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn son_projeler_gidis_donus_depo() {
        let kok = gecici_kok("recent");
        let depo = DosyaDepo::yeni(&kok);
        // Yokken boş.
        assert!(son_projeleri_yukle(&depo).bos_mu());
        // Yaz + geri oku.
        let mut l = SonProjelerListesi::yeni();
        l.acildi("/p/a", "A", Utc::now());
        son_projeleri_kaydet(&depo, &l).unwrap();
        let geri = son_projeleri_yukle(&depo);
        assert_eq!(geri.sayi(), 1);
        let _ = std::fs::remove_dir_all(&kok);
    }

    #[test]
    fn haber_onbellek_gidis_donus_depo() {
        let kok = gecici_kok("news");
        let depo = DosyaDepo::yeni(&kok);
        assert!(haber_onbellek_yukle(&depo).is_none());
        let akis = news::varsayilan_akis(Utc::now());
        haber_onbellek_kaydet(&depo, &akis).unwrap();
        let geri = haber_onbellek_yukle(&depo).unwrap();
        assert_eq!(geri.haberler.len(), akis.haberler.len());
        let _ = std::fs::remove_dir_all(&kok);
    }

    #[test]
    fn bozuk_depo_bos_listeye_duser() {
        let kok = gecici_kok("bozuk");
        let depo = DosyaDepo::yeni(&kok);
        depo.yaz(ANAHTAR_SON_PROJELER, b"{ bozuk").unwrap();
        // Çökmeden boş listeye düşmeli (degrade).
        assert!(son_projeleri_yukle(&depo).bos_mu());
        let _ = std::fs::remove_dir_all(&kok);
    }
}
