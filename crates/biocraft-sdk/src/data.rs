//! SDK — Veri erişim **kontratı** (veri tanımları).
//!
//! Eklenti veriye **doğrudan** erişemez; her erişim bir [`Capability`] ile kapılıdır
//! (MK-13).  Bu modül, bir eklentinin yapacağı veri erişim isteğini **tarif eden**
//! veri tipini taşır.  Gerçek izin denetimi + VFS çözümü **çekirdek host'unda**
//! (`biocraft-plugin-host`) yapılır; SDK yalnızca sözleşmeyi tanımlar.

use biocraft_types::Capability;
use serde::{Deserialize, Serialize};

/// Bir eklentinin yapmak istediği veri erişimi (host bunu yetkiye karşı denetler).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VeriIstegi {
    /// Bu erişim için gereken yetenek (örn. dosya için `Fs`, veritabanı için `Db`).
    pub gereken_yetki: Capability,
    /// İnsan tarafından okunur açıklama (kurulumda/teşhiste gösterilir).
    pub aciklama: String,
}

impl VeriIstegi {
    /// Yeni bir veri erişim isteği tanımlar.
    pub fn yeni(gereken_yetki: Capability, aciklama: impl Into<String>) -> Self {
        Self {
            gereken_yetki,
            aciklama: aciklama.into(),
        }
    }

    /// Verilen yetki kümesi bu isteği **karşılıyor mu?** (Saf yardımcı; nihai
    /// denetim host'ta tekrar yapılır — savunmada derinlik.)
    pub fn karsilaniyor_mu(&self, verilen: &[Capability]) -> bool {
        verilen.contains(&self.gereken_yetki)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn veri_istegi_yetki_denetimi() {
        let istek = VeriIstegi::yeni(Capability::Fs, "örnek dosyayı oku");
        assert!(istek.karsilaniyor_mu(&[Capability::Fs, Capability::Net]));
        assert!(!istek.karsilaniyor_mu(&[Capability::Net]));
    }

    #[test]
    fn veri_istegi_serde_gidis_donus() {
        let istek = VeriIstegi::yeni(Capability::Db, "yerel DB sorgusu");
        let json = serde_json::to_string(&istek).unwrap();
        let geri: VeriIstegi = serde_json::from_str(&json).unwrap();
        assert_eq!(istek, geri);
    }
}
