//! **Güvenli mod** — 3. parti eklentileri kapalı başlatma (teşhis, İP-07).
//!
//! Güvenli modda yalnızca **resmi** (BioCraft imzalı) eklentiler yüklenir; tüm 3. parti
//! eklentiler (doğrulanmış/bilinmeyen/imzasız) **atlanır.** Böylece bir eklentinin yol
//! açtığı sorun teşhis edilir.  Güvenli mod kullanıcı tarafından (`--safe-mode`) veya
//! **tekrarlı çökme** sonrası önerilerek etkinleşir (bkz. [`crate::isolate`]).

use crate::signature::ImzaDurumu;

/// Güvenli modun neden açıldığı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuvenliModSebep {
    /// Kullanıcı bilinçli açtı (örn. `--safe-mode` bayrağı).
    KullaniciSecti,
    /// Tekrarlı çökme nedeniyle önerildi/açıldı.
    TekrarliCokme,
}

/// Güvenli mod durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GuvenliMod {
    etkin: bool,
    sebep: Option<GuvenliModSebep>,
}

impl GuvenliMod {
    /// Kapalı (normal) mod.
    pub fn kapali() -> Self {
        Self::default()
    }

    /// Belirtilen sebeple açık güvenli mod.
    pub fn acik(sebep: GuvenliModSebep) -> Self {
        Self {
            etkin: true,
            sebep: Some(sebep),
        }
    }

    /// Güvenli mod açık mı?
    pub fn etkin_mi(&self) -> bool {
        self.etkin
    }

    /// Açıksa nedeni.
    pub fn sebep(&self) -> Option<GuvenliModSebep> {
        self.sebep
    }
}

/// Bir eklenti adayı için yükleme kararı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YuklemeKarari {
    /// Yüklenebilir.
    Yukle,
    /// Güvenli mod nedeniyle atlandı (kullanıcıya açıklamasıyla gösterilir).
    Atlandi { aciklama: String },
}

impl YuklemeKarari {
    /// Yüklenmeli mi?
    pub fn yuklenebilir(&self) -> bool {
        matches!(self, YuklemeKarari::Yukle)
    }
}

/// Güvenli mod + imza durumuna göre bir eklentinin yüklenip yüklenmeyeceğine karar verir.
///
/// * Normal mod → her zaman `Yukle` (imza politikası ayrıca `signature` modülünde denetlenir).
/// * Güvenli mod → yalnızca **resmi** eklenti yüklenir; gerisi `Atlandi`.
pub fn karar(guvenli_mod: GuvenliMod, imza: &ImzaDurumu, kimlik: &str) -> YuklemeKarari {
    if !guvenli_mod.etkin_mi() {
        return YuklemeKarari::Yukle;
    }
    if imza.resmi_mi() {
        YuklemeKarari::Yukle
    } else {
        YuklemeKarari::Atlandi {
            aciklama: format!(
                "Güvenli mod açık: '{kimlik}' resmi olmadığı için atlandı (3. parti eklentiler kapalı)"
            ),
        }
    }
}

/// Bir aday listesini güvenli moda göre süzer; `(kimlik, karar)` çiftleri döndürür.
pub fn filtrele<'a>(
    guvenli_mod: GuvenliMod,
    adaylar: impl IntoIterator<Item = (&'a str, &'a ImzaDurumu)>,
) -> Vec<(String, YuklemeKarari)> {
    adaylar
        .into_iter()
        .map(|(kimlik, imza)| (kimlik.to_string(), karar(guvenli_mod, imza, kimlik)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resmi() -> ImzaDurumu {
        ImzaDurumu::Resmi {
            yayinci: "BioCraft".into(),
        }
    }

    #[test]
    fn normal_mod_hepsini_yukler() {
        let gm = GuvenliMod::kapali();
        assert!(karar(gm, &ImzaDurumu::Imzasiz, "biocraft.a.b").yuklenebilir());
        assert!(karar(gm, &resmi(), "biocraft.c.d").yuklenebilir());
    }

    #[test]
    fn guvenli_mod_3party_atlar_resmiyi_yukler() {
        let gm = GuvenliMod::acik(GuvenliModSebep::KullaniciSecti);
        assert!(gm.etkin_mi());
        // Resmi → yüklenir.
        assert!(karar(gm, &resmi(), "biocraft.studio.ana").yuklenebilir());
        // İmzasız 3. parti → atlanır.
        assert!(!karar(gm, &ImzaDurumu::Imzasiz, "biocraft.acme.arac").yuklenebilir());
        // Doğrulanmış ama 3. parti → yine atlanır (yalnızca resmi).
        let dogrulanmis = ImzaDurumu::Dogrulanmis {
            yayinci: "Acme".into(),
        };
        assert!(!karar(gm, &dogrulanmis, "biocraft.acme.arac").yuklenebilir());
    }

    #[test]
    fn filtrele_karisik_liste() {
        let gm = GuvenliMod::acik(GuvenliModSebep::TekrarliCokme);
        let r = resmi();
        let i = ImzaDurumu::Imzasiz;
        let adaylar = vec![("biocraft.studio.ana", &r), ("biocraft.acme.arac", &i)];
        let sonuc = filtrele(gm, adaylar);
        assert_eq!(sonuc.len(), 2);
        assert!(sonuc[0].1.yuklenebilir()); // resmi
        assert!(!sonuc[1].1.yuklenebilir()); // 3. parti
    }

    #[test]
    fn sebep_korunur() {
        let gm = GuvenliMod::acik(GuvenliModSebep::TekrarliCokme);
        assert_eq!(gm.sebep(), Some(GuvenliModSebep::TekrarliCokme));
        assert_eq!(GuvenliMod::kapali().sebep(), None);
    }
}
