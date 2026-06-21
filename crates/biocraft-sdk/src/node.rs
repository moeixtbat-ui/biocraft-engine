//! SDK — Node (düğüm) grafiği uzantı **kontratı** (veri tanımları).
//!
//! Eklenti, node tabanlı iş akışına (İP-05) yeni düğüm türleri ekleyebilir.
//! Bu modül düğümün **arayüz tanımını** (kimlik + portlar) taşır; gerçek yürütme
//! ve grafik tuvali çekirdek/İP-05 tarafındadır.  Eklentiler birbirine değil,
//! yalnızca bu kontrata bağlanır (MK-17).

use serde::{Deserialize, Serialize};

/// Bir port (bağlantı ucu) yönü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortYonu {
    /// Düğüme veri giren uç.
    Giris,
    /// Düğümden veri çıkan uç.
    Cikis,
}

/// Bir düğüm portunun tanımı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortTanimi {
    /// Port adı (düğüm içinde benzersiz).
    pub ad: String,
    /// Giriş mi çıkış mı.
    pub yon: PortYonu,
    /// Taşıdığı veri türünün etiketi (örn. "dizi", "tablo", "hizalama").
    pub veri_turu: String,
}

/// Bir eklentinin ilan ettiği düğüm türü.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeTanimi {
    /// Düğüm türü kimliği.
    pub kimlik: String,
    /// Kullanıcıya görünen başlık.
    pub baslik: String,
    /// Giriş/çıkış portları.
    pub portlar: Vec<PortTanimi>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_tanimi_serde_gidis_donus() {
        let n = NodeTanimi {
            kimlik: "node.hizala".into(),
            baslik: "Hizalama".into(),
            portlar: vec![
                PortTanimi {
                    ad: "girdi".into(),
                    yon: PortYonu::Giris,
                    veri_turu: "dizi".into(),
                },
                PortTanimi {
                    ad: "sonuc".into(),
                    yon: PortYonu::Cikis,
                    veri_turu: "hizalama".into(),
                },
            ],
        };
        let json = serde_json::to_string(&n).unwrap();
        let geri: NodeTanimi = serde_json::from_str(&json).unwrap();
        assert_eq!(n, geri);
    }
}
