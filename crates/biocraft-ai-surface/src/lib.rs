//! biocraft-ai-surface — L3: AI yüzey/iskelet stub (MK-46, MK-48, MK-49).
//!
//! MVP: yalnızca yüzeysel; öğeler "yapılandırılmadı" etiketli (sahte işlev yok).
//! AI tamamen kapatılabilir; arayüz tam çalışır.
//!
//! **Çıkış kapısı sözleşmesi (İP-10/MK-42):** Bir **dış** AI çağrısı (bulut model / AI havuzuna katkı)
//! eklendiğinde, gönderilecek bağlam **gönderimden önce**
//! `biocraft_data::privacy::classify::cikis_denetle(sinif, DisKanal::DisAi)` ile denetlenmelidir.
//! PHI dış AI'a gidemez (çekirdek engeli, atlanamaz).  Anonimleştirme için bkz.
//! `biocraft_data::privacy::anonymize`.
// MK-40: L3 katmanı — yalnızca L0/L1/L2 katmanlarına bağlı; üst katman yasak.

pub use biocraft_sdk;
pub use biocraft_types;
