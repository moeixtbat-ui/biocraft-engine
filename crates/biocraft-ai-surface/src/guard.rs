//! YZ-08 — **Gizlilik/güven sınırı (çapraz).**  AI güvenliğinin bel kemiği.
//!
//! Bir **dış** AI çağrısından (bulut/özel) ÖNCE, gönderilecek her bağlam öğesi çekirdek çıkış
//! kapısından geçer: `biocraft_data::privacy::classify::cikis_denetle(sinif, DisKanal::DisAi)`.
//! **PHI/hassas veri dış AI'a gidemez** — sınır eklentiye değil çekirdeğe (İP-10/L2) emanet;
//! MK-40 ile AI yüzeyi (L3) bu kapının altına inip atlayamaz.  **Yerel** sağlayıcıda veri
//! cihazdan çıkmadığından PHI'de bile çalışılabilir (0-AI.5/1).
// MK-42/43: PHI dış AI'a gidemez; engel kapatılamaz, atlanamaz.

use biocraft_data::privacy::classify::{cikis_denetle, DisKanal};
use biocraft_types::ErrorReport;

use crate::context::AiBaglam;
use crate::provider::SaglayiciTuru;

/// Çıkış kapısı kararı.
#[derive(Debug, Clone)]
pub enum GuardKarari {
    /// Gönderim güvenli (yerel sağlayıcı ya da hiçbir öğe engellenmiyor).
    Izinli,
    /// En az bir öğe engellendi — hangi öğeler + standart hata (kullanıcıya gösterilir).
    Engellendi {
        /// Engellenen öğelerin adları.
        engellenen_ogeler: Vec<String>,
        /// Standart İP-16 hatası (ne oldu / neden / nasıl çözülür).
        hata: Box<ErrorReport>,
    },
}

impl GuardKarari {
    /// İzinli mi?
    pub fn izinli_mi(&self) -> bool {
        matches!(self, GuardKarari::Izinli)
    }
}

/// Bir bağlamı, hedef sağlayıcı türü için denetler.
///
/// - **Yerel** sağlayıcı → her zaman izinli (veri cihazdan çıkmaz).
/// - **Dış** (bulut/özel) → her öğe `cikis_denetle(..., DisKanal::DisAi)` ile denetlenir; PHI/
///   hassas varsa **engellenir** (fail-closed).
pub fn baglam_denetle(baglam: &AiBaglam, tur: SaglayiciTuru) -> GuardKarari {
    // Yerel: cihazdan çıkış yok → izinli (0-AI.5/1).
    if !tur.dis_mi() {
        return GuardKarari::Izinli;
    }

    // Dış sağlayıcı: her öğeyi çekirdek çıkış kapısından geçir.
    let mut engellenen = Vec::new();
    let mut ilk_hata: Option<ErrorReport> = None;
    for oge in &baglam.ogeler {
        let karar = cikis_denetle(oge.sinif, DisKanal::DisAi);
        if let biocraft_data::privacy::classify::CikisKarari::Engellendi(h) = karar {
            engellenen.push(oge.ad.clone());
            if ilk_hata.is_none() {
                ilk_hata = Some(*h);
            }
        }
    }

    match ilk_hata {
        Some(h) => GuardKarari::Engellendi {
            engellenen_ogeler: engellenen,
            hata: Box::new(h),
        },
        None => GuardKarari::Izinli,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::BaglamOgesi;
    use biocraft_types::DataClassification;

    fn karma_baglam() -> AiBaglam {
        AiBaglam::sorgudan("özetle")
            .oge_ile(BaglamOgesi::yeni(
                "normal tablo",
                "x",
                DataClassification::Normal,
            ))
            .oge_ile(BaglamOgesi::yeni(
                "hasta kaydı",
                "y",
                DataClassification::HasasPhi,
            ))
    }

    #[test]
    fn yerel_her_zaman_izinli() {
        // PHI içerse bile yerel sağlayıcıya gönderilebilir (cihazdan çıkmaz).
        let k = baglam_denetle(&karma_baglam(), SaglayiciTuru::Yerel);
        assert!(k.izinli_mi(), "yerel sağlayıcı PHI'de bile çalışabilir");
    }

    #[test]
    fn bulutta_phi_engellenir() {
        let k = baglam_denetle(&karma_baglam(), SaglayiciTuru::Bulut);
        match k {
            GuardKarari::Engellendi {
                engellenen_ogeler, ..
            } => {
                assert_eq!(engellenen_ogeler, vec!["hasta kaydı".to_string()]);
            }
            GuardKarari::Izinli => panic!("PHI dış AI'a gitmemeliydi (MK-42/43)"),
        }
    }

    #[test]
    fn ozel_de_fail_closed() {
        // Özel/self-hosted da güvenli tarafta dış kabul edilir → PHI engellenir.
        let k = baglam_denetle(&karma_baglam(), SaglayiciTuru::Ozel);
        assert!(!k.izinli_mi(), "özel sağlayıcı fail-closed: PHI engellenir");
    }

    #[test]
    fn bulutta_phi_yoksa_izinli() {
        let b = AiBaglam::sorgudan("özetle")
            .oge_ile(BaglamOgesi::yeni(
                "sentetik",
                "x",
                DataClassification::Sentetik,
            ))
            .oge_ile(BaglamOgesi::yeni("normal", "y", DataClassification::Normal));
        assert!(baglam_denetle(&b, SaglayiciTuru::Bulut).izinli_mi());
    }
}
