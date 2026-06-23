//! ÇE-02 — **Çoklu örnek** karşılaştırması (Gün 37): birden çok BAM/örnek izi, **senkron**
//! gezinme (vaka/kontrol).
//!
//! Senkronun anahtarı yapısaldır: tüm örnek izleri **aynı** [`Tuval`](super::canvas::Tuval)
//! (tek koordinat doğruluk kaynağı) üzerinden çizilir → biri kaydığında hepsi kayar; ayrı
//! bölge durumu **yoktur** (olası "senkron kayması" hatası kökten önlenir).  İki yerleşim:
//! * **Yan yana** (üst üste istiflenmiş lanes): her örnek kendi kapsama + hizalama izine sahip.
//! * **Üst üste** (overlay): örneklerin kapsaması **tek lane**'de farklı renklerle çizilir.

use super::cizim::CizimRengi;
use super::tracks::{Iz, IzTuru};
use super::veri::OkumaParcasi;

/// Tek bir örnek (BAM/CRAM kaynağı) — vaka/kontrol karşılaştırmasında bir katman.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ornek {
    /// Benzersiz kimlik (iz kimliği öneki; kararlı anahtar).
    pub kimlik: String,
    /// Kullanıcıya görünen ad.
    pub ad: String,
}

impl Ornek {
    /// Bir örnek kurar.
    pub fn yeni(kimlik: impl Into<String>, ad: impl Into<String>) -> Self {
        Self {
            kimlik: kimlik.into(),
            ad: ad.into(),
        }
    }

    /// Bu örneğin kapsama izi kimliği.
    pub fn kapsama_iz_kimlik(&self) -> String {
        format!("{}.kapsama", self.kimlik)
    }

    /// Bu örneğin hizalama (read) izi kimliği.
    pub fn hizalama_iz_kimlik(&self) -> String {
        format!("{}.reads", self.kimlik)
    }
}

/// Çoklu örnek yerleşim modu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KarsilastirmaModu {
    /// Her örnek kendi kapsama + hizalama izine sahip; izler dikey istiflenir.
    YanYana,
    /// Tüm örneklerin kapsaması tek lane'de farklı renklerle üst üste (karşılaştırma).
    UstUste,
}

/// Overlay (üst üste) lane'in iz kimliği.
pub const OVERLAY_KIMLIK: &str = "ornek.overlay.kapsama";

/// Verilen örnekler + mod için tarayıcıya eklenecek izleri üretir.
///
/// * **Yan yana:** her örnek → `[kapsama, hizalama]` (örnek adı izlerin adına yazılır).
/// * **Üst üste:** tek `overlay kapsama` izi (örnek katmanları çizimde renklendirilir).
pub fn ornek_izleri(ornekler: &[Ornek], mod_: KarsilastirmaModu) -> Vec<Iz> {
    match mod_ {
        KarsilastirmaModu::YanYana => {
            let mut izler = Vec::with_capacity(ornekler.len() * 2);
            for o in ornekler {
                izler.push(Iz::yeni(
                    o.kapsama_iz_kimlik(),
                    format!("{} • kapsama", o.ad),
                    IzTuru::Kapsama,
                ));
                izler.push(Iz::yeni(
                    o.hizalama_iz_kimlik(),
                    format!("{} • okumalar", o.ad),
                    IzTuru::Hizalama,
                ));
            }
            izler
        }
        KarsilastirmaModu::UstUste => {
            vec![Iz::yeni(
                OVERLAY_KIMLIK,
                "Örnek karşılaştırma (kapsama)",
                IzTuru::Kapsama,
            )]
        }
    }
}

/// Bir örnek katmanı — overlay kapsamada tek bir örneğin okumaları + rengi.
#[derive(Debug, Clone, PartialEq)]
pub struct OrnekKatman {
    /// Örnek adı (lejant/tooltip).
    pub ad: String,
    /// Bu örneğe atanan ayırt edici renk.
    pub renk: CizimRengi,
    /// Örneğin görünen penceredeki okumaları (kapsama bundan binlenir).
    pub okumalar: Vec<OkumaParcasi>,
}

/// Örnek indeksine ayırt edici bir renk atar (4'lük palet döner; daha fazlası tekrarlanır).
pub fn ornek_rengi(indeks: usize) -> CizimRengi {
    const PALET: [CizimRengi; 4] = [
        CizimRengi::OrnekA,
        CizimRengi::OrnekB,
        CizimRengi::OrnekC,
        CizimRengi::OrnekD,
    ];
    PALET[indeks % PALET.len()]
}

/// `(örnek_adı, okumalar)` çiftlerini renkli overlay katmanlarına çevirir.
pub fn katmanlar(ornek_okumalari: &[(String, Vec<OkumaParcasi>)]) -> Vec<OrnekKatman> {
    ornek_okumalari
        .iter()
        .enumerate()
        .map(|(i, (ad, okumalar))| OrnekKatman {
            ad: ad.clone(),
            renk: ornek_rengi(i),
            okumalar: okumalar.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ornekler() -> Vec<Ornek> {
        vec![
            Ornek::yeni("vaka", "Vaka"),
            Ornek::yeni("kontrol", "Kontrol"),
        ]
    }

    #[test]
    fn yan_yana_her_ornege_iki_iz() {
        let izler = ornek_izleri(&ornekler(), KarsilastirmaModu::YanYana);
        assert_eq!(izler.len(), 4);
        assert_eq!(izler[0].kimlik, "vaka.kapsama");
        assert_eq!(izler[1].kimlik, "vaka.reads");
        assert_eq!(izler[2].kimlik, "kontrol.kapsama");
    }

    #[test]
    fn ust_uste_tek_overlay() {
        let izler = ornek_izleri(&ornekler(), KarsilastirmaModu::UstUste);
        assert_eq!(izler.len(), 1);
        assert_eq!(izler[0].kimlik, OVERLAY_KIMLIK);
    }

    #[test]
    fn katman_renkleri_ayrik() {
        let v: Vec<(String, Vec<OkumaParcasi>)> =
            vec![("Vaka".into(), vec![]), ("Kontrol".into(), vec![])];
        let k = katmanlar(&v);
        assert_eq!(k.len(), 2);
        assert_ne!(k[0].renk, k[1].renk, "örnekler ayrı renk almalı");
        assert_eq!(k[0].renk, CizimRengi::OrnekA);
        // 5. örnek paleti baştan döner.
        assert_eq!(ornek_rengi(4), CizimRengi::OrnekA);
    }
}
