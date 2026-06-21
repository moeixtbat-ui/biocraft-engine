//! biocraft-ipc — L1: IPC/gRPC/Arrow Flight köprüleri + host↔eklenti **çağrı zarfı** (MK-39, MK-30).
//!
//! Kontrol kanalı gRPC; büyük veri Arrow Flight + shared memory üzerinden taşınır
//! (Tier-3 subprocess yolu Gün 14+).  Bu crate ayrıca, **sandbox sınırını geçen**
//! tek bir eklenti çağrısının istek/yanıt biçimini tanımlar ([`EklentiCagrisi`] /
//! [`EklentiYaniti`]).  Aynı zarf hem in-process WASM (Tier-2) hem de gelecekteki
//! out-of-process (Tier-3) yollarında kullanılır — tek mesaj şekli.

// MK-40: L1 katmanı — yalnızca L0'a (biocraft-types) bağlı; üst katman yasak.

/// Temel tipler IPC mesajlarında kullanılacak — yeniden dışa aktarım.
pub use biocraft_types;

use biocraft_types::{Capability, CorrelationId, ErrorReport};
use serde::{Deserialize, Serialize};

/// Host'tan eklentiye yapılan tek bir fonksiyon çağrısı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EklentiCagrisi {
    /// Çağrılacak dışa aktarılmış (export) fonksiyonun adı (örn. `"merhaba"`).
    pub fonksiyon: String,
    /// Bu çağrıyı loglarla eşleştiren korelasyon kimliği (İP-16).
    pub correlation_id: CorrelationId,
}

impl EklentiCagrisi {
    /// Verilen fonksiyon adıyla yeni bir çağrı (yeni korelasyon kimliği üretir).
    pub fn yeni(fonksiyon: impl Into<String>) -> Self {
        Self {
            fonksiyon: fonksiyon.into(),
            correlation_id: CorrelationId::new(),
        }
    }
}

/// Bir [`EklentiCagrisi`]'nın sonucu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EklentiYaniti {
    /// Çağrı başarıyla tamamlandı.
    Basari {
        /// Eklentinin döndürdüğü i32/i64 değer (anlamı fonksiyona göre).
        donen: i64,
        /// Çağrı sırasında eklentinin host günlüğüne yazdığı satırlar.
        gunluk: Vec<String>,
    },
    /// Eklenti **sahip olmadığı bir yeteneği** kullanmaya çalıştı; çağrı reddedildi (MK-13).
    YetkiReddi {
        /// Reddedilen yetenek (örn. `Fs`).
        yetki: Capability,
        /// Kullanıcıya gösterilecek standart hata raporu.
        rapor: ErrorReport,
    },
    /// Çağrı başka bir hatayla sonlandı (trap, kaynak limiti, bozuk modül…).
    Hata(ErrorReport),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cagri_benzersiz_korelasyon() {
        let a = EklentiCagrisi::yeni("merhaba");
        let b = EklentiCagrisi::yeni("merhaba");
        assert_eq!(a.fonksiyon, "merhaba");
        assert_ne!(a.correlation_id, b.correlation_id);
    }

    #[test]
    fn yanit_serde_gidis_donus() {
        let y = EklentiYaniti::Basari {
            donen: 16,
            gunluk: vec!["Merhaba BioCraft".into()],
        };
        let json = serde_json::to_string(&y).unwrap();
        let geri: EklentiYaniti = serde_json::from_str(&json).unwrap();
        assert_eq!(y, geri);
    }

    #[test]
    fn yetki_reddi_yaniti() {
        let y = EklentiYaniti::YetkiReddi {
            yetki: Capability::Fs,
            rapor: ErrorReport::new("Erişim reddedildi", "fs yetkisi yok", "İzni onayla"),
        };
        match y {
            EklentiYaniti::YetkiReddi { yetki, .. } => assert_eq!(yetki, Capability::Fs),
            _ => panic!("YetkiReddi bekleniyordu"),
        }
    }
}
