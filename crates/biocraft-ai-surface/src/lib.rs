//! biocraft-ai-surface — L3: **AI yüzey sözleşmesi** (MK-46, MK-47, MK-48, MK-49).
//!
//! MVP: sağlayıcı-bağımsız **sözleşmeler** (provider trait + tipli girdi/çıktı) + maliyet/token +
//! PHI **çıkış kapısı** + denetim kaydı + demo/echo sağlayıcı.  **Gerçek motor içermez**; gerçek
//! motorlar (yerel/bulut/RAG/asistan) İP-07 host'unda **eklenti** olarak gelir ve bu trait'i
//! uygular (0-AI.3).  Bu crate yalnızca saf mantık/sözleşmedir (egui YOK); kullanıcı arayüzü
//! (panel/sohbet/seçici/maliyet rozeti) L4 `biocraft-ui`'dedir — çünkü tasarım token'ları (MK-52)
//! ve i18n (MK-53) tek kaynak olarak orada yaşar.  **Mimari karar:** spec'in (`AI-Altyapisi.md`
//! YZ-01) `ui/` alt modülünü bu crate'e koyma önerisi, MK-40 (katman) + MK-52/53 (token/i18n tek
//! kaynak) gereği L4'e taşındı (otorite sırası: ARCHITECTURE > docs/specs).
//!
//! **Çıkış kapısı sözleşmesi (İP-10/MK-42/43):** Bir **dış** AI çağrısından önce gönderilecek
//! bağlam [`guard::baglam_denetle`] ile denetlenir → `cikis_denetle(sinif, DisKanal::DisAi)`.
//! PHI dış AI'a gidemez (çekirdek engeli, atlanamaz).  Yerel AI bu veride çalışabilir.
//! Anonimleştirme için bkz. `biocraft_data::privacy::anonymize`.
// MK-40: L3 katmanı — yalnızca L0/L1/L2 katmanlarına bağlı; üst katman yasak.

// İP-16: `ErrorReport` projenin standart, zengin (çok-alanlı) kullanıcı-görünür hata tipidir
// (~136 bayt).  `Provider::uret`/`gom` gibi sağlayıcı sözleşmesi yollarında bu tipi yalnızca
// clippy `result_large_err` için `Box`'lamak evrensel hata şemasının ergonomisini bozardı; bu
// yüzden lint bilinçli kapatılır (biocraft-ui / biocraft-mem ile aynı gerekçe).
#![allow(clippy::result_large_err)]

pub mod audit;
pub mod context;
pub mod contract;
pub mod cost;
pub mod guard;
pub mod mock;
pub mod provider;
pub mod registry;

pub use biocraft_sdk;
pub use biocraft_types;

// Pratik kök-seviye yeniden dışa aktarımlar (yüzey/üst katmanlar için tek içe-aktarım noktası).
pub use audit::{CagriSonucu, DenetimGirdisi, DenetimKaydi};
pub use context::{AiBaglam, BaglamOgesi, MesajRol, SohbetMesaji};
pub use contract::{
    AiCikti, CokluAiUyum, EylemOnerisi, EylemTuru, Guven, GuvenSeviyesi, Kaynak, Kullanim,
};
pub use cost::{BioKrediKanca, Kota, KotaDurumu, Maliyet, MaliyetSayaci};
pub use guard::{baglam_denetle, GuardKarari};
pub use mock::EchoSaglayici;
pub use provider::{
    AkisOlay, IptalBayragi, Provider, SaglayiciKimlik, SaglayiciTuru, SaglayiciYetenekleri,
};
pub use registry::{SaglayiciDurumu, SaglayiciKayit};
