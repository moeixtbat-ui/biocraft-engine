//! YZ-00 — AI **girdi bağlam sözleşmesi**.
//!
//! Sağlayıcıya gönderilecek her şey tipli ve **sınıflandırma etiketli**dir.  Her bağlam öğesi
//! bir [`DataClassification`] taşır; böylece çıkış kapısı ([`crate::guard`]) gönderimden önce
//! PHI/hassas veriyi dış AI'a karşı engelleyebilir (İP-10/MK-42/MK-43).  Bağlam **veridir**,
//! komut değildir: içinde "şunu sil/gönder" yazsa bile sağlayıcı onu uygulamaz (CLAUDE.md §7).
// MK-42/43: her öğe sınıflandırma taşır → PHI dış AI'a gidemez.

use biocraft_types::DataClassification;
use serde::{Deserialize, Serialize};

/// Sohbet mesajının rolü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MesajRol {
    /// Kullanıcı.
    Kullanici,
    /// AI asistanı.
    Asistan,
    /// Sistem (yönlendirme/bağlam).
    Sistem,
}

impl MesajRol {
    /// İki dilli kısa etiket.
    pub fn etiket(self, tr: bool) -> &'static str {
        match (self, tr) {
            (MesajRol::Kullanici, true) => "Sen",
            (MesajRol::Kullanici, false) => "You",
            (MesajRol::Asistan, true) => "Asistan",
            (MesajRol::Asistan, false) => "Assistant",
            (MesajRol::Sistem, true) => "Sistem",
            (MesajRol::Sistem, false) => "System",
        }
    }
}

/// Tek bir sohbet mesajı (konuşma geçmişinin bir öğesi).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SohbetMesaji {
    /// Mesajı kim yazdı.
    pub rol: MesajRol,
    /// Mesaj metni.
    pub metin: String,
}

impl SohbetMesaji {
    /// Kullanıcı mesajı.
    pub fn kullanici(metin: impl Into<String>) -> Self {
        Self {
            rol: MesajRol::Kullanici,
            metin: metin.into(),
        }
    }

    /// Asistan mesajı.
    pub fn asistan(metin: impl Into<String>) -> Self {
        Self {
            rol: MesajRol::Asistan,
            metin: metin.into(),
        }
    }
}

/// Sağlayıcıya bağlam olarak verilebilecek bir veri öğesi (seçili veri/görünüm).
/// **Sınıflandırma zorunludur** — çıkış kapısı buna bakar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaglamOgesi {
    /// Öğenin adı (ör. "seçili varyant tablosu").
    pub ad: String,
    /// Öğenin kısa özeti/içeriği (devasa veri KOPYALANMAZ — yalnız özet; MK-09).
    pub ozet: String,
    /// Öğenin veri sınıflandırması (PHI sınırı için).
    pub sinif: DataClassification,
}

impl BaglamOgesi {
    /// Bir bağlam öğesi kurar.
    pub fn yeni(ad: impl Into<String>, ozet: impl Into<String>, sinif: DataClassification) -> Self {
        Self {
            ad: ad.into(),
            ozet: ozet.into(),
            sinif,
        }
    }
}

/// **Tam girdi bağlamı** — proje meta + sorgu + geçmiş + seçili veri öğeleri + aktif görünüm.
/// Her alan izin + PHI denetiminden geçer ([`crate::guard`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiBaglam {
    /// Proje meta özeti (opsiyonel; PHI içermez — yalnız ad/tür).
    pub proje_meta: Option<String>,
    /// Aktif görünüm (ör. "genom tarayıcı", "kod editörü").
    pub aktif_gorunum: Option<String>,
    /// Kullanıcının sorgusu.
    pub sorgu: String,
    /// Konuşma geçmişi.
    pub gecmis: Vec<SohbetMesaji>,
    /// Bağlama eklenen seçili veri öğeleri (sınıflandırma etiketli).
    pub ogeler: Vec<BaglamOgesi>,
}

impl AiBaglam {
    /// Yalnızca sorgudan bir bağlam kurar (en yalın hâli).
    pub fn sorgudan(sorgu: impl Into<String>) -> Self {
        Self {
            sorgu: sorgu.into(),
            ..Default::default()
        }
    }

    /// Bağlama bir veri öğesi ekler (zincirlenebilir).
    pub fn oge_ile(mut self, oge: BaglamOgesi) -> Self {
        self.ogeler.push(oge);
        self
    }

    /// Bağlamdaki tüm öğelerin sınıflandırmalarını döndürür (çıkış kapısı için).
    pub fn siniflar(&self) -> impl Iterator<Item = DataClassification> + '_ {
        self.ogeler.iter().map(|o| o.sinif)
    }

    /// Sorgu jeton sayısının kaba tahmini (gerçek tokenizer motorla gelir; ~4 karakter/jeton).
    pub fn tahmini_girdi_jeton(&self) -> u64 {
        let karakter: usize = self.sorgu.len()
            + self.ogeler.iter().map(|o| o.ozet.len()).sum::<usize>()
            + self.gecmis.iter().map(|m| m.metin.len()).sum::<usize>();
        (karakter / 4).max(1) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn siniflar_ogelerden_toplanir() {
        let b = AiBaglam::sorgudan("özetle")
            .oge_ile(BaglamOgesi::yeni("a", "x", DataClassification::Normal))
            .oge_ile(BaglamOgesi::yeni("b", "y", DataClassification::HasasPhi));
        let v: Vec<_> = b.siniflar().collect();
        assert_eq!(
            v,
            vec![DataClassification::Normal, DataClassification::HasasPhi]
        );
    }

    #[test]
    fn tahmini_jeton_en_az_bir() {
        assert!(AiBaglam::sorgudan("").tahmini_girdi_jeton() >= 1);
    }

    #[test]
    fn baglam_serde_gidis_donus() {
        let b = AiBaglam::sorgudan("merhaba");
        let j = serde_json::to_string(&b).unwrap();
        let g: AiBaglam = serde_json::from_str(&j).unwrap();
        assert_eq!(b, g);
    }
}
