//! biocraft-net — L3: Iroh P2P arayüzü (yalnız pasif kanca) stub (MK-50).
//!
//! Eklenti yokken sıfır maliyet; P2P yalnızca metadata/sonuç; ham/PHI asla (MK-42/43).
//!
//! **Çıkış kapısı sözleşmesi (İP-10/MK-43):** Bu crate gerçek gönderim kodu kazandığında, her giden
//! veri **gönderimden önce** `biocraft_data::privacy::classify::cikis_denetle(sinif, DisKanal::P2p)`
//! ile denetlenmelidir.  PHI/hassas `Engellendi` döner → gönderim YAPILMAZ.  Sınır çekirdektedir
//! (L2); bu L3 kanalı ona bağımlı olduğundan kapıyı atlayamaz.
// MK-40: L3 katmanı — yalnızca L0/L1/L2 katmanlarına bağlı; üst katman yasak.

pub use biocraft_ipc;
pub use biocraft_sdk;
pub use biocraft_types;
