//! **Gizlilik profili** — yerel-varsayılan + granüler ama akıllı ayarlar (MK-41, İP-10).
//!
//! İki katman:
//! - **Global varsayılan** ([`GizlilikProfili::varsayilan_global`]): tamamen yerel, telemetri kapalı,
//!   AI havuzuna katkı yok, her dış gönderim onaya tabi — **gizlilik-öncelikli** (privacy by default).
//! - **Proje override** ([`proje_ile_coz`]): her proje kendi `[gizlilik]` ayarını taşır ve **global'i
//!   geçersiz kılar** (İP-02 manifest `[gizlilik]`).
//!
//! Bu profil, **kanalın açık olup olmadığını** yönetir (örn. P2P kapalı, AI havuzu kapalı).  Sınıf
//! ekseninden (PHI engeli) **bağımsızdır**: profil bir kanalı açsa bile PHI yine [`super::classify`]
//! tarafından bloklanır.  Yani bir dış gönderim üç kapıdan geçer: **sınıf** (PHI engeli) → **profil**
//! (kanal açık mı) → **onay** (kullanıcı evet dedi mi).

use serde::{Deserialize, Serialize};

use crate::project::manifest;

use super::classify::DisKanal;

/// Telemetri toplama düzeyi.  Varsayılan **kapalı** (İP-10: "telemetri varsayılan kapalı").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TelemetriDuzeyi {
    /// Hiç telemetri gönderilmez (varsayılan).
    Kapali,
    /// Yalnızca minimal + anonim (çökme sayısı vb.; kişisel/işlemsel veri yok).
    MinimalAnonim,
    /// Tam kullanım istatistiği (yine de anonim; yalnızca açık opt-in).
    Tam,
}

/// Granüler gizlilik tercihleri.  Akıllı (gizlilik-öncelikli) varsayılanlarla gelir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GizlilikProfili {
    /// Tamamen yerel çalış: hiçbir dış kanal etkin değil (çevrimdışı = tam gizlilik).
    pub tamamen_yerel: bool,
    /// Anonimleştirilmiş sonuçların AI havuzuna katkısı (varsayılan Hayır — opt-in).
    pub ai_havuzu_katki: bool,
    /// Telemetri düzeyi (varsayılan Kapalı).
    pub telemetri: TelemetriDuzeyi,
    /// Dağıtık (P2P) ağ etkin mi (varsayılan Hayır).
    pub dagitik_ag_etkin: bool,
    /// Her dış gönderim açık onay ister mi (varsayılan Evet — kapatılması önerilmez).
    pub her_dis_gonderim_onay: bool,
}

impl GizlilikProfili {
    /// **Global varsayılan** — gizlilik-öncelikli: tamamen yerel, telemetri kapalı, AI havuzu kapalı.
    pub fn varsayilan_global() -> Self {
        Self {
            tamamen_yerel: true,
            ai_havuzu_katki: false,
            telemetri: TelemetriDuzeyi::Kapali,
            dagitik_ag_etkin: false,
            her_dis_gonderim_onay: true,
        }
    }

    /// Bu profil, verilen **dış kanalı** etkinleştiriyor mu?
    ///
    /// `tamamen_yerel` açıkken **hiçbir** dış kanal etkin değildir (her şeyden önce gelir).  Aksi hâlde
    /// kanal kendi ayarına bakar.  Not: bu yalnızca "kanal açık mı"yı söyler; gönderim için ayrıca
    /// sınıf (PHI engeli) ve kullanıcı onayı gerekir.
    pub fn dis_kanal_etkin_mi(&self, kanal: DisKanal) -> bool {
        if self.tamamen_yerel {
            return false; // çevrimdışı/yerel mod: tüm dış kanallar kapalı.
        }
        match kanal {
            DisKanal::P2p => self.dagitik_ag_etkin,
            DisKanal::DisAi => self.ai_havuzu_katki,
            DisKanal::DisApi => true, // dış API açık (yine de onay gerekir).
            DisKanal::Telemetri => self.telemetri != TelemetriDuzeyi::Kapali,
            DisKanal::Paylasim => true, // kullanıcı paylaşımı açık (yine de onay gerekir).
        }
    }
}

impl Default for GizlilikProfili {
    fn default() -> Self {
        Self::varsayilan_global()
    }
}

/// Bir proje bağlamı için **etkin** gizlilik profilini çözer: global varsayılan + (varsa) projenin
/// `[gizlilik]` override'ı.  **Proje her zaman global'i geçersiz kılar** (İP-02/İP-10).
///
/// Projeden gelen alanlar (`tamamen_yerel`, `ai_havuzu_katki`, `dagitik_ag_etkin`) doğrudan global'in
/// üzerine yazılır.  Manifest'te bulunmayan alanlar (`telemetri`, `her_dis_gonderim_onay`) global'den
/// korunur.
pub fn proje_ile_coz(
    global: &GizlilikProfili,
    proje: Option<&manifest::Gizlilik>,
) -> GizlilikProfili {
    let mut etkin = *global;
    if let Some(p) = proje {
        etkin.tamamen_yerel = p.tamamen_yerel;
        etkin.ai_havuzu_katki = p.ai_havuzu_katki;
        etkin.dagitik_ag_etkin = p.dagitik_ag_etkin;
    }
    etkin
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::manifest::{Determinizm, Gizlilik};

    #[test]
    fn global_varsayilan_tam_gizlilik() {
        let g = GizlilikProfili::varsayilan_global();
        assert!(g.tamamen_yerel);
        assert!(!g.ai_havuzu_katki);
        assert_eq!(g.telemetri, TelemetriDuzeyi::Kapali);
        assert!(!g.dagitik_ag_etkin);
        assert!(g.her_dis_gonderim_onay);
        // Tamamen yerel → hiçbir dış kanal etkin değil.
        for kanal in DisKanal::TUMU {
            assert!(
                !g.dis_kanal_etkin_mi(kanal),
                "{kanal:?} yerel modda açık olmamalı"
            );
        }
    }

    #[test]
    fn proje_global_i_gecersiz_kilar() {
        // Global tamamen yerel; proje override ile dış API + P2P açıyor.
        let global = GizlilikProfili::varsayilan_global();
        let proje = Gizlilik {
            tamamen_yerel: false,
            ai_havuzu_katki: true,
            dagitik_ag_etkin: true,
            determinizm: Determinizm::HizliKesif,
        };
        let etkin = proje_ile_coz(&global, Some(&proje));
        // Proje değerleri kazandı.
        assert!(!etkin.tamamen_yerel);
        assert!(etkin.ai_havuzu_katki);
        assert!(etkin.dagitik_ag_etkin);
        assert!(etkin.dis_kanal_etkin_mi(DisKanal::P2p));
        assert!(etkin.dis_kanal_etkin_mi(DisKanal::DisAi));
        // Manifest'te olmayan alanlar global'den korunur.
        assert_eq!(etkin.telemetri, TelemetriDuzeyi::Kapali);
        assert!(etkin.her_dis_gonderim_onay);
    }

    #[test]
    fn proje_global_i_daha_kisitlayici_yapabilir() {
        // Global dış kanalları açmış; proje tamamen yerel'e çekiyor → proje kazanır (daha kısıtlayıcı).
        let global = GizlilikProfili {
            tamamen_yerel: false,
            ai_havuzu_katki: true,
            telemetri: TelemetriDuzeyi::MinimalAnonim,
            dagitik_ag_etkin: true,
            her_dis_gonderim_onay: true,
        };
        let proje = Gizlilik {
            tamamen_yerel: true,
            ai_havuzu_katki: false,
            dagitik_ag_etkin: false,
            determinizm: Determinizm::TekrarUretilebilir,
        };
        let etkin = proje_ile_coz(&global, Some(&proje));
        assert!(etkin.tamamen_yerel);
        for kanal in DisKanal::TUMU {
            assert!(!etkin.dis_kanal_etkin_mi(kanal));
        }
    }

    #[test]
    fn proje_yoksa_global_aynen_kalir() {
        let global = GizlilikProfili::varsayilan_global();
        assert_eq!(proje_ile_coz(&global, None), global);
    }

    #[test]
    fn telemetri_kapaliyken_telemetri_kanali_kapali() {
        let g = GizlilikProfili {
            tamamen_yerel: false,
            telemetri: TelemetriDuzeyi::Kapali,
            ..GizlilikProfili::varsayilan_global()
        };
        assert!(!g.dis_kanal_etkin_mi(DisKanal::Telemetri));
        let g2 = GizlilikProfili {
            telemetri: TelemetriDuzeyi::MinimalAnonim,
            ..g
        };
        assert!(g2.dis_kanal_etkin_mi(DisKanal::Telemetri));
    }
}
