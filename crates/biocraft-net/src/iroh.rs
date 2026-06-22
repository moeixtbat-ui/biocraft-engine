//! **Iroh arayüz iskeleti** (İP-15) — QUIC/NAT-traversal için *yalnızca arayüz*; gerçek bağlantı YOK.
//!
//! Bu modül, gelecekteki dağıtık-ağ eklentisinin Iroh (QUIC tabanlı P2P) ile kuracağı bağlantının
//! **soyut arayüzünü** tanımlar.  **Önemli:**
//!
//! - **`iroh` crate'i bağımlılık olarak EKLENMEZ.**  Gerçek QUIC/ağ yığını ağır bağımlılıklar
//!   (quinn, rustls…) getirir ve "eklenti yokken sıfır maliyet" (MK-50) ilkesini bozar.
//! - Burada **hiçbir bağlantı kurma kodu yoktur** — ne soket açılır, ne handshake yapılır, ne de
//!   arka plan görevi başlar.  Yalnızca [`IrohUcKancasi`] trait'i ve yer-tutucu adres tipleri vardır.
//! - Gerçek `iroh::Endpoint` sarmalayan implementasyon dağıtık-ağ **eklentisinde** yaşar ve bu
//!   trait'i uygular; çekirdek onu [`crate::hooks`] üzerinden tanır.
//!
//! > Kısaca: bu, eklenti gelince "sancısız takılacak" arayüz noktasıdır (İP-15 kabul kriteri).

use serde::{Deserialize, Serialize};

use biocraft_types::ErrorReport;

use crate::contract::P2pYuku;
use crate::identity::DugumKimlik;

/// Bir düğümün **erişim adresi** (yer tutucu) — iroh `NodeAddr` karşılığı.
///
/// MVP'de yalnızca opak ipuçları taşır (gerçek soket adresi/relay çözümü eklentide).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DugumAdresi {
    /// Hedef düğümün kimliği.
    pub dugum: DugumKimlik,
    /// İsteğe bağlı doğrudan adres ipuçları (host:port).  Boş olabilir (relay üzerinden bulunur).
    pub dogrudan_ipuclari: Vec<String>,
    /// İsteğe bağlı relay/aktarma URL'si (NAT arkasındaki düğümler için).
    pub relay_url: Option<String>,
}

impl DugumAdresi {
    /// Yalnızca kimlikten (adres ipucu olmadan) bir adres kurar.
    pub fn kimlikten(dugum: DugumKimlik) -> Self {
        Self {
            dugum,
            dogrudan_ipuclari: Vec::new(),
            relay_url: None,
        }
    }
}

/// Açık bir P2P bağlantısının **soyut tutamacı** (eklenti tipi).  Çekirdek içeriğini bilmez.
pub trait BaglantiTutamac: Send + Sync {
    /// Karşı düğüm.
    fn karsi_dugum(&self) -> DugumKimlik;

    /// Bu bağlantı üzerinden bir yük gönderir.  Yük zaten çıkış kapısından geçmiştir ([`P2pYuku`]).
    fn yuk_gonder(&self, yuk: &P2pYuku) -> Result<(), Box<ErrorReport>>;
}

/// **Iroh uç-nokta (endpoint) arayüzü** — eklenti uygular; gerçek QUIC bağlantısını o yönetir.
///
/// `Send + Sync`: arka plan ağ runtime'ından kullanılabilir.  MVP'de implementasyon YOKTUR →
/// hiçbir bağlantı kurulmaz (gerçekten pasif, MK-50).
pub trait IrohUcKancasi: Send + Sync {
    /// Bu yereldeki uç-noktanın kimliği (iroh public key karşılığı).
    fn yerel_kimlik(&self) -> DugumKimlik;

    /// Verilen adrese bağlanır (gerçek bağlantı eklentide; çekirdek yalnızca arayüzü tanır).
    fn baglan(&self, adres: &DugumAdresi) -> Result<Box<dyn BaglantiTutamac>, Box<ErrorReport>>;

    /// Ağdaki erişilebilir düğümleri keşfeder (relay/mDNS; gerçek mantık eklentide).
    fn dugumleri_kesfet(&self) -> Result<Vec<DugumAdresi>, Box<ErrorReport>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dugum_adresi_kimlikten_bos_ipucu() {
        let a = DugumAdresi::kimlikten(DugumKimlik::yeni("n1"));
        assert!(a.dogrudan_ipuclari.is_empty());
        assert!(a.relay_url.is_none());
        assert_eq!(a.dugum.olarak_str(), "n1");
    }

    #[test]
    fn dugum_adresi_serilesir() {
        let a = DugumAdresi {
            dugum: DugumKimlik::yeni("n2"),
            dogrudan_ipuclari: vec!["1.2.3.4:1234".into()],
            relay_url: Some("https://relay.example".into()),
        };
        let json = serde_json::to_string(&a).unwrap();
        let geri: DugumAdresi = serde_json::from_str(&json).unwrap();
        assert_eq!(geri, a);
    }
}
