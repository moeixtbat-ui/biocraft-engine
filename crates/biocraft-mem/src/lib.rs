//! biocraft-mem — L2: Global Memory Orchestrator + Donanım Koruma (Zero-Impact)
//! (MK-21, MK-22, MK-09, MK-24, MK-25, MK-26, MK-29).
//!
//! **Çökmeyen bellek yönetimi + donanıma zarar vermeyen koruma.**  İki güvence bir arada:
//!
//! 1. **Bellek (OOM koruması):** Tüm bileşenler belleği doğrudan değil [`BellekOrkestratoru`]'ndan
//!    **rezervasyonla** ister; bütçe aşılırsa talep reddedilir (panik/OOM yok, MK-22).
//! 2. **Donanım (Zero-Impact):** Bağımsız bir watchdog GPU/CPU/NVMe sıcaklığını izler; eşik
//!    aşımında iş yükü **kademeli** azalır, kritikte durur + checkpoint alınır (MK-24).
//!
//! Modüller:
//! - [`orchestrator`] — rezervasyon + LRU boşaltma + bellek baskısı (MK-21/MK-22).
//! - [`budget`] — dosya açmadan önce bütçe kontrolü + akış/iptal teklifi (MK-22/MK-09).
//! - [`priority`] — işleme öncelik modları + Zero-Impact hesap kısma kancası.
//! - [`akis`] — out-of-core: büyük veriyi pencere pencere işleme (stream + mmap, MK-09).
//! - [`thermal`] — termal eşik tablosu + kademeli aksiyon (saf çekirdek, MK-24).
//! - [`hardware_guard`] — bağımsız watchdog thread'i + sensör soyutlaması + checkpoint (MK-24).
//! - [`disk_guard`] — disk doluluk (%10/%2) + yanlış-sürücü koruması (MK-25).
//! - [`autotune`] — donanım profili + Eco/Bio + düşük donanımda sadeleşme (MK-26).
//! - [`determinism`] — determinizm bayrağı kancası (MK-29).
//! - [`metrics`] — performans metrik toplayıcı (tepe/ortalama).
//! - [`birim`] — bayt → insan-okunur biçim yardımcısı.
// MK-40: L2 katmanı — yalnızca L0/L1 katmanlarına bağlı; üst katman yasak.

// İP-16: `ErrorReport` projenin standart, zengin (çok-alanlı) kullanıcı-görünür hata tipidir.
// Her fallible API'yi yalnızca clippy `result_large_err` için `Box`'lamak, bu evrensel hata
// tipinin ergonomisini bozardı; üstelik bu yollar sıcak döngü değildir (rezervasyon dosya/iş
// granülerliğinde gerçekleşir, bayt başına değil).  Bu yüzden lint bilinçli olarak kapatılır.
#![allow(clippy::result_large_err)]

pub mod akis;
pub mod autotune;
pub mod birim;
pub mod budget;
pub mod determinism;
pub mod disk_guard;
pub mod hardware_guard;
pub mod metrics;
pub mod orchestrator;
pub mod priority;
pub mod thermal;

// Pratik erişim için sık kullanılan tipleri kök seviyede yeniden dışa aktar.
pub use akis::{akisla_isle, dosya_akisla_isle, mmap_ile_isle, AkisAyar, AkisOzet};
pub use autotune::{profil_cikar, DonanimProfili, DonanimSinifi, OtoAyar, PerformansModu};
pub use birim::insan_bayt;
pub use budget::{dosya_butce_kontrol, AcmaSecenegi, AkisTeklifi, ButceKarari, DosyaTahmini};
pub use determinism::{DeterminizmBayragi, DeterminizmModu};
pub use disk_guard::{
    disk_durumu_oku, dogru_surucu_mu, yazma_oncesi_kontrol, DiskDurumu, DiskKarari, DiskKoruma,
};
pub use hardware_guard::{
    bos_checkpoint, CheckpointKanca, DonanimMuhafiz, DonanimOrnegi, DonanimSensoru, KoruyucuDurum,
    SabitSensor, SensorYok, SistemSensoru,
};
pub use metrics::TepeOrtalama;
pub use orchestrator::{
    BellekBileseni, BellekDurumu, BellekOrkestratoru, BosaltmaOzeti, OnbellekTutamac, Rezervasyon,
};
pub use priority::{hesap_plani, HesapPlani, OncelikDurumu, OncelikModu};
pub use thermal::{en_kotu_aksiyon, DonanimParca, TermalAksiyon, TermalEsikler};

pub use biocraft_sdk;
pub use biocraft_types;
