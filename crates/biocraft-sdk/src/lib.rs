//! biocraft-sdk — L1: Eklenti SDK'sı ve ortak yardımcılar (MK-17).
//!
//! Eklentiler birbirine doğrudan bağlanmaz; yalnızca bu crate üzerinden konuşur.
// MK-40: L1 katmanı — yalnızca L0'a (biocraft-types) bağlı; üst katman yasak.

/// BioCraft temel tiplerini yeniden dışa aktar; SDK kullananlar tek bağımlılıkla erişir.
pub use biocraft_types;
