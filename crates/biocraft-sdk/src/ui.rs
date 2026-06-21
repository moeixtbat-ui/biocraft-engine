//! SDK — UI uzantı noktası **kontratı** (veri tanımları).
//!
//! Eklenti; panel/sekme/menü/komut/ayar **kaydı** ilan eder; çekirdek bunları
//! güvenli alanlarda gösterir (MK-17).  Bu modül yalnızca **veri kontratını** tanımlar
//! (L1 → egui'ye bağlanamaz).  Gerçek gösterim + çakışma yönetimi (iki eklenti aynı
//! alanı isterse öncelik/sıra) **çekirdek tarafında, Gün 14'te** yapılır (İP-07 notu).

use serde::{Deserialize, Serialize};

/// Bir eklentinin genişletebileceği UI alanı türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiUzantiTuru {
    /// Yan/alt panel.
    Panel,
    /// Editör sekmesi.
    Sekme,
    /// Menü öğesi.
    Menu,
    /// Komut paleti komutu.
    Komut,
    /// Ayarlar sayfası.
    Ayar,
}

/// Bir eklentinin ilan ettiği tek bir UI uzantı kaydı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiKayit {
    /// Kayıt kimliği (eklenti içinde benzersiz; çakışma yönetimi çekirdekte).
    pub kimlik: String,
    /// Kullanıcıya görünen başlık.
    pub baslik: String,
    /// Hangi UI alanına eklenecek.
    pub tur: UiUzantiTuru,
}

impl UiKayit {
    /// Yeni bir UI uzantı kaydı oluşturur.
    pub fn yeni(kimlik: impl Into<String>, baslik: impl Into<String>, tur: UiUzantiTuru) -> Self {
        Self {
            kimlik: kimlik.into(),
            baslik: baslik.into(),
            tur,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_kayit_serde_gidis_donus() {
        let k = UiKayit::yeni("panel.ornek", "Örnek Panel", UiUzantiTuru::Panel);
        let json = serde_json::to_string(&k).unwrap();
        let geri: UiKayit = serde_json::from_str(&json).unwrap();
        assert_eq!(k, geri);
    }
}
