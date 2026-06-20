//! biocraft-mem — L2: Global Memory Orchestrator + out-of-core akış (MK-21, MK-22, MK-09).
//!
//! **Çökmeyen bellek yönetimi.**  Tüm bileşenler (UI, alt süreç, DuckDB, eklenti) belleği
//! doğrudan değil [`BellekOrkestratoru`]'ndan **rezervasyonla** ister; bütçe aşılırsa talep
//! reddedilir (panik/OOM yok, MK-22) ve baskıda boştaki önbellekler LRU ile boşaltılır.
//!
//! Modüller:
//! - [`orchestrator`] — rezervasyon + LRU boşaltma + bellek baskısı (MK-21/MK-22).
//! - [`budget`] — dosya açmadan önce bütçe kontrolü + akış/iptal teklifi (MK-22/MK-09).
//! - [`priority`] — işleme öncelik modları + Zero-Impact hesap kısma kancası.
//! - [`akis`] — out-of-core: büyük veriyi pencere pencere işleme (stream + mmap, MK-09).
//! - [`birim`] — bayt → insan-okunur biçim yardımcısı.
//!
//! Donanım sıcaklık/termal koruma (Zero-Impact'in donanım tarafı) AYRI gün işidir
//! (İP-08 Donanım Koruma — Gün 8); bu crate bugün **bellek + bütçe + out-of-core** kapsar.
// MK-40: L2 katmanı — yalnızca L0/L1 katmanlarına bağlı; üst katman yasak.

// İP-16: `ErrorReport` projenin standart, zengin (çok-alanlı) kullanıcı-görünür hata tipidir.
// Her fallible API'yi yalnızca clippy `result_large_err` için `Box`'lamak, bu evrensel hata
// tipinin ergonomisini bozardı; üstelik bu yollar sıcak döngü değildir (rezervasyon dosya/iş
// granülerliğinde gerçekleşir, bayt başına değil).  Bu yüzden lint bilinçli olarak kapatılır.
#![allow(clippy::result_large_err)]

pub mod akis;
pub mod birim;
pub mod budget;
pub mod orchestrator;
pub mod priority;

// Pratik erişim için sık kullanılan tipleri kök seviyede yeniden dışa aktar.
pub use akis::{akisla_isle, dosya_akisla_isle, mmap_ile_isle, AkisAyar, AkisOzet};
pub use birim::insan_bayt;
pub use budget::{dosya_butce_kontrol, AcmaSecenegi, AkisTeklifi, ButceKarari, DosyaTahmini};
pub use orchestrator::{
    BellekBileseni, BellekDurumu, BellekOrkestratoru, BosaltmaOzeti, OnbellekTutamac, Rezervasyon,
};
pub use priority::{hesap_plani, HesapPlani, OncelikDurumu, OncelikModu};

pub use biocraft_sdk;
pub use biocraft_types;
