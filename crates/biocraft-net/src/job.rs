//! **İş tanımı / sonuç toplama** soyutlaması (İP-15) — dağıtık hesaplamanın arayüz iskeleti.
//!
//! Bir [`Is`], ağa dağıtılacak hesaplama birimidir; **yalnızca** kapıdan geçmiş [`P2pYuku`]'ler
//! taşır (ham/PHI veri zaten yük olarak inşa edilemez — bkz. [`crate::contract`]).  Gerçek dağıtım,
//! parçalama, yeniden atama ve sonuç birleştirme dağıtık-ağ **eklentisinde** ([`IsDagitici`]) yapılır.

use serde::{Deserialize, Serialize};

use biocraft_types::ErrorReport;

use crate::contract::P2pYuku;
use crate::identity::DugumKimlik;

/// Bir dağıtık işin **kimliği** (eklenti üretir; sonuç toplamada kullanılır).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IsKimlik(pub String);

impl IsKimlik {
    /// Kimlik dizesinden kurar.
    pub fn yeni(kimlik: impl Into<String>) -> Self {
        Self(kimlik.into())
    }
}

/// Bir işin **dayanıklılık politikası** (arayüz) — düğüm düşerse ne olur?
///
/// Gerçek yeniden-atama/kısmi-sonuç mantığı eklentide; burada yalnızca niyet beyan edilir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayaniklilikPolitikasi {
    /// Bir düğüm düşerse iş başka düğüme **yeniden atansın** mı?
    pub yeniden_ata: bool,
    /// **Kısmi sonuç** kabul edilebilir mi (tüm düğümler tamamlamasa da)?
    pub kismi_sonuc_kabul: bool,
    /// Aynı iş kaç düğümde **tekrarlansın** (sonuç çapraz-doğrulama için)?  1 = tekrar yok.
    pub tekrar_sayisi: u8,
}

impl Default for DayaniklilikPolitikasi {
    /// Güvenli varsayılan: yeniden atama açık, kısmi sonuç kapalı, tekrar yok.
    fn default() -> Self {
        Self {
            yeniden_ata: true,
            kismi_sonuc_kabul: false,
            tekrar_sayisi: 1,
        }
    }
}

/// Ağa dağıtılacak bir **iş**.
///
/// `yukler` yalnızca kapıdan geçmiş [`P2pYuku`]'lerdir → ham/PHI veri bir işe konulamaz (MK-50).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Is {
    /// İnsan-okunur iş türü/adı (örn. "hizalama", "varyant-çağırma").
    pub tur: String,
    /// İşin taşıdığı yükler — hepsi sınıflandırma kapısından geçmiştir.
    pub yukler: Vec<P2pYuku>,
    /// Dayanıklılık politikası.
    pub dayaniklilik: DayaniklilikPolitikasi,
}

impl Is {
    /// Belirli türde, verilen yüklerle bir iş kurar (varsayılan dayanıklılık).
    pub fn yeni(tur: impl Into<String>, yukler: Vec<P2pYuku>) -> Self {
        Self {
            tur: tur.into(),
            yukler,
            dayaniklilik: DayaniklilikPolitikasi::default(),
        }
    }

    /// İşin toplam bayt boyutu (bütçe/şeffaflık).
    pub fn toplam_bayt(&self) -> usize {
        self.yukler.iter().map(|y| y.bayt_sayisi()).sum()
    }

    /// **Tüm** yükler hâlâ çıkış kapısına uygun mu? (savunma katmanı — gönderim öncesi son denetim).
    pub fn tum_yukler_kapidan_gecer_mi(&self) -> bool {
        self.yukler.iter().all(|y| y.kapidan_gecer_mi())
    }
}

/// Bir işin **durumu** (sonuç toplama).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsDurumu {
    /// Sıraya alındı, henüz bir düğüme atanmadı.
    Beklemede,
    /// Bir/birden çok düğümde çalışıyor.
    Calisiyor,
    /// Tamamlandı — sonuç hazır.
    Tamamlandi,
    /// Bir düğüm düştü, başka düğüme yeniden atandı (dayanıklılık).
    YenidenAtandi,
    /// Başarısız (standart hata).
    Hata(Box<ErrorReport>),
}

/// Bir işin **sonucu** — sonuç da bir [`P2pYuku`]'dür (kapıdan geçer; ham veri içermez).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsSonucu {
    /// Hangi iş.
    pub is: IsKimlik,
    /// Sonucu üreten düğüm.
    pub dugum: DugumKimlik,
    /// Sonuç yükü (içerik türü genellikle `Sonuc`).
    pub yuk: P2pYuku,
}

/// **İş dağıtıcı arayüzü** — eklenti uygular; iş gönderme ve sonuç toplamanın soyutlaması.
///
/// `Send + Sync`: arka plan runtime'ından kullanılabilir.  MVP'de implementasyon kayıtlı değildir
/// (sıfır maliyet, MK-50); dağıtık-ağ eklentisi takılınca [`crate::hooks`] üzerinden bağlanır.
pub trait IsDagitici: Send + Sync {
    /// Bir işi ağa gönderir; iş kimliği döner.
    fn gonder(&self, is: Is) -> Result<IsKimlik, Box<ErrorReport>>;

    /// Bir işin güncel durumunu sorgular.
    fn durum(&self, is: &IsKimlik) -> Result<IsDurumu, Box<ErrorReport>>;

    /// Tamamlanmış bir işin sonuçlarını toplar (kısmi de olabilir).
    fn sonuclari_topla(&self, is: &IsKimlik) -> Result<Vec<IsSonucu>, Box<ErrorReport>>;

    /// Bir işi iptal eder.
    fn iptal(&self, is: &IsKimlik) -> Result<(), Box<ErrorReport>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::P2pYuku;
    use biocraft_types::DataClassification;

    fn ornek_is() -> Is {
        let y = P2pYuku::metadata(DataClassification::Normal, "param", vec![1, 2, 3, 4]).unwrap();
        Is::yeni("hizalama", vec![y])
    }

    #[test]
    fn is_yalnizca_kapidan_gecmis_yuk_tasir() {
        let is = ornek_is();
        assert_eq!(is.toplam_bayt(), 4);
        assert!(is.tum_yukler_kapidan_gecer_mi());
    }

    #[test]
    fn varsayilan_dayaniklilik_guvenli() {
        let d = DayaniklilikPolitikasi::default();
        assert!(d.yeniden_ata);
        assert!(!d.kismi_sonuc_kabul);
        assert_eq!(d.tekrar_sayisi, 1);
    }

    #[test]
    fn is_serilesir() {
        let is = ornek_is();
        let json = serde_json::to_string(&is).unwrap();
        let geri: Is = serde_json::from_str(&json).unwrap();
        assert_eq!(geri.tur, "hizalama");
        assert_eq!(geri.toplam_bayt(), 4);
    }
}
