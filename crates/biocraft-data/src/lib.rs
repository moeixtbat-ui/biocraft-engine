//! biocraft-data — L2: Veri katmanı (MK-31, MK-33, MK-34, MK-59).
//!
//! İçerik (Gün 18 itibarıyla):
//! - [`project`] — **taşınabilir, zengin proje formatı**: klasör yapısı + `biocraft.toml` manifest
//!   (kimlik/ORCID/veri-sınıflandırma/sürüm+göç geçmişi/harici-büyük-veri referansları) + BLAKE3
//!   bütünlük (açılışta doğrula, bozuk/eksik dosya net hata) + tek dosya `.bcproj` dışa aktarımı
//!   (ZIP stored; hassas ayar varsayılan hariç).
//! - [`privacy`] — **gizlilik, veri yönetimi ve provenance** (İP-10): çekirdek-zorlamalı veri
//!   sınıflandırma çıkış kapısı (PHI/hassas dış kanallara fiziksel olarak çıkamaz — MK-42/43),
//!   gizlilik profili (yerel-varsayılan; proje global'i ezer), dış-iletişim onay akışı,
//!   anonimleştirme temeli, per-veri köken (kaynak/sürüm/lisans/atıf) + köken gezgini, ve
//!   KVKK/GDPR hakları (tam ihraç / güvenli silme / erişim).
//!
//! İleride: SQLite (config/meta) + DuckDB (analitik) + RocksDB (KV/cache) entegrasyonu.
//
// MK-40: L2 katmanı — yalnızca L0/L1 katmanlarına bağlı; üst katman yasak.

// İP-16 standart hata şeması (`ErrorReport`) bilinçli olarak zengindir (ne/neden/çözüm + teknik
// detay + correlation_id) → `Result<_, ErrorReport>` büyük olur.  `biocraft-mem` ile aynı gerekçe.
#![allow(clippy::result_large_err)]

pub use biocraft_sdk;
pub use biocraft_types;

pub mod privacy;
pub mod project;

// Sık kullanılan proje formatı API'sini kök seviyede yeniden dışa aktar (kolay erişim).
pub use project::{
    ac, disa_aktar, olustur, AcilanProje, BuyukVeriStratejisi, Determinizm, DisaAktarRaporu,
    DisaAktarSecenekleri, KurulanProje, Manifest, ProjeKurulumGirdisi, VeriYerlesimi,
    BCPROJ_UZANTI,
};

// Sık kullanılan gizlilik API'sini kök seviyede yeniden dışa aktar (İP-10).
pub use privacy::{
    cikis_denetle, gonderim_degerlendir, guvenli_sil, tam_veri_ihrac, veri_envanteri, CikisKarari,
    DisKanal, GizlilikProfili, GonderimDurumu, KokenGezgini, OnayKarari, OnayTalebi, VeriKokeni,
};
