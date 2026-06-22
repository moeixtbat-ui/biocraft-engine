//! **Kimlik / güven arayüzü** (İP-15) — düğüm kimliği, güven seviyesi, itibar ve kötü-düğüm izolasyonu.
//!
//! Burada yalnızca **arayüz/veri tipleri** vardır; gerçek imza doğrulama, itibar puanlama ve kötü
//! düğüm tespiti dağıtık-ağ **eklentisinde** uygulanır.  Varsayılan güven **yoktur** (fail-closed):
//! tanınmayan bir düğüm güvenilmez kabul edilir, ona iş/sonuç gönderme kararı eklenti politikasına
//! kalır.

use serde::{Deserialize, Serialize};

/// Bir ağ düğümünün **kimliği** — kararlı, kriptografik kimlik (örn. iroh `NodeId` / public key).
///
/// MVP'de yalnızca opak bir kimlik dizesidir; gerçek anahtar üretimi/doğrulama eklentide (bkz.
/// [`crate::iroh`]).  `Eq`/`Hash` ile düğümler harita anahtarı olabilir.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DugumKimlik(pub String);

impl DugumKimlik {
    /// Opak kimlik dizesinden kurar.
    pub fn yeni(kimlik: impl Into<String>) -> Self {
        Self(kimlik.into())
    }

    /// Ham kimlik dizesi.
    pub fn olarak_str(&self) -> &str {
        &self.0
    }
}

/// Bir düğüme duyulan **güven seviyesi** (büyük = daha güvenilir).
///
/// Varsayılan [`GuvenSeviyesi::Bilinmiyor`]'dur: tanınmayan düğüme güvenilmez (fail-closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub enum GuvenSeviyesi {
    /// Tanınmayan düğüm — varsayılan; güvenilmez kabul edilir.
    #[default]
    Bilinmiyor,
    /// İtibarı düşük / yeni düğüm.
    Dusuk,
    /// Geçmişi tutarlı düğüm.
    Orta,
    /// Doğrulanmış/uzun süreli güvenilir düğüm.
    Yuksek,
    /// Kötü davranış nedeniyle **izole edilmiş** — iş/sonuç alışverişi yapılmaz.
    Engelli,
}

impl GuvenSeviyesi {
    /// Bu düğümle iş/sonuç alışverişi *aday* mı?  `Engelli` ve `Bilinmiyor` için `false`
    /// (fail-closed); gerçek eşik/politika eklentide ayarlanabilir.
    pub fn alisverise_aday_mi(self) -> bool {
        matches!(
            self,
            GuvenSeviyesi::Dusuk | GuvenSeviyesi::Orta | GuvenSeviyesi::Yuksek
        )
    }
}

/// Bir düğümün **itibar kaydı** (arayüz) — sonuç doğrulama/itibar için temel.
///
/// Gerçek puanlama (başarılı/başarısız iş oranı, sonuç doğrulama, çapraz kontrol) eklentide yapılır;
/// burada yalnızca taşınacak alanlar sabitlenir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItibarKaydi {
    /// Düğüm kimliği.
    pub dugum: DugumKimlik,
    /// Mevcut güven seviyesi.
    pub guven: GuvenSeviyesi,
    /// Başarıyla tamamlanan iş sayısı (eklenti doldurur).
    pub basarili_is: u64,
    /// Doğrulamada başarısız/uyuşmayan iş sayısı (eklenti doldurur).
    pub basarisiz_is: u64,
}

impl ItibarKaydi {
    /// Yeni (tanınmayan) bir düğüm için sıfır itibarlı kayıt — varsayılan güven `Bilinmiyor`.
    pub fn yeni(dugum: DugumKimlik) -> Self {
        Self {
            dugum,
            guven: GuvenSeviyesi::default(),
            basarili_is: 0,
            basarisiz_is: 0,
        }
    }
}

/// **Kimlik/güven sağlayıcı arayüzü** — eklenti uygular; çekirdek yalnızca çağırır.
///
/// `Send + Sync`: arka plan ağ runtime'ından erişilebilir (gerçek eklentide).  MVP'de hiçbir
/// implementasyon kayıtlı değildir → bu yollar hiç çağrılmaz (sıfır maliyet, MK-50).
pub trait KimlikSaglayici: Send + Sync {
    /// Bu yereldeki düğümün kimliği.
    fn yerel_kimlik(&self) -> DugumKimlik;

    /// Bir düğümün itibar kaydını döndürür (bilinmiyorsa varsayılan/yeni kayıt).
    fn itibar(&self, dugum: &DugumKimlik) -> ItibarKaydi;

    /// Bir düğümü kötü davranış nedeniyle **izole eder** (kötü-düğüm izolasyon kancası).
    fn izole_et(&self, dugum: &DugumKimlik);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varsayilan_guven_bilinmiyor_ve_fail_closed() {
        // Tanınmayan düğüm: varsayılan güven yok → alışverişe aday değil.
        let it = ItibarKaydi::yeni(DugumKimlik::yeni("abc"));
        assert_eq!(it.guven, GuvenSeviyesi::Bilinmiyor);
        assert!(!it.guven.alisverise_aday_mi());
        assert!(!GuvenSeviyesi::Engelli.alisverise_aday_mi());
    }

    #[test]
    fn guven_siralanir() {
        assert!(GuvenSeviyesi::Dusuk < GuvenSeviyesi::Orta);
        assert!(GuvenSeviyesi::Orta < GuvenSeviyesi::Yuksek);
        assert!(GuvenSeviyesi::Orta.alisverise_aday_mi());
    }

    #[test]
    fn dugum_kimlik_harita_anahtari_olur() {
        let a = DugumKimlik::yeni("node-1");
        let b = DugumKimlik::yeni("node-1");
        assert_eq!(a, b);
        assert_eq!(a.olarak_str(), "node-1");
    }
}
