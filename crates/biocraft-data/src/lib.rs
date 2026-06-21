//! biocraft-data — L2: Veri katmanı (MK-31, MK-33, MK-34, MK-59).
//!
//! İçerik (Gün 17 itibarıyla):
//! - [`project`] — **taşınabilir, zengin proje formatı**: klasör yapısı + `biocraft.toml` manifest
//!   (kimlik/ORCID/veri-sınıflandırma/sürüm+göç geçmişi/harici-büyük-veri referansları) + BLAKE3
//!   bütünlük (açılışta doğrula, bozuk/eksik dosya net hata) + tek dosya `.bcproj` dışa aktarımı
//!   (ZIP stored; hassas ayar varsayılan hariç).
//!
//! İleride: SQLite (config/meta) + DuckDB (analitik) + RocksDB (KV/cache) entegrasyonu.
//
// MK-40: L2 katmanı — yalnızca L0/L1 katmanlarına bağlı; üst katman yasak.

// İP-16 standart hata şeması (`ErrorReport`) bilinçli olarak zengindir (ne/neden/çözüm + teknik
// detay + correlation_id) → `Result<_, ErrorReport>` büyük olur.  `biocraft-mem` ile aynı gerekçe.
#![allow(clippy::result_large_err)]

pub use biocraft_sdk;
pub use biocraft_types;

pub mod project;

// Sık kullanılan proje formatı API'sini kök seviyede yeniden dışa aktar (kolay erişim).
pub use project::{
    ac, disa_aktar, olustur, AcilanProje, BuyukVeriStratejisi, Determinizm, DisaAktarRaporu,
    DisaAktarSecenekleri, KurulanProje, Manifest, ProjeKurulumGirdisi, VeriYerlesimi,
    BCPROJ_UZANTI,
};
