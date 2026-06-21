//! **Gizlilik, veri yönetimi ve provenance** (İP-10) — gizlilik-öncelikli, çekirdek-zorlamalı.
//!
//! Yerel-varsayılan veri yönetimi: hiçbir veri varsayılan dışarı gitmez; PHI/hassas veri **çekirdek
//! seviyesinde** tüm dış kanallara (P2P/AI/API/telemetri/paylaşım) karşı korunur; her dış gönderim
//! açık onay ister; köken kaydı + köken gezgini; KVKK/GDPR hakları (erişim/ihraç/silme) (MK-41/42/43/34).
//!
//! Bir dış gönderim **üç kapıdan** geçer (en kısıtlayıcı önce):
//! 1. **Sınıf** ([`classify`]): PHI/hassas mutlak engellenir (atlanamaz).
//! 2. **Profil** ([`profile`]): kanal açık mı? (yerel-varsayılan → kapalı; proje global'i ezer.)
//! 3. **Onay** ([`consent`]): kullanıcı, ne/nereye/ne kadar gönderileceğini görüp onaylar.
//!
//! Diğer modüller: [`anonymize`] (anonimleştirme temeli; sınıf düşürme), [`provenance`] (per-veri
//! kaynak/sürüm/lisans/atıf + onay defteri), [`lineage_browser`] (köken gezgini), [`export`] (haklar:
//! tam ihraç + güvenli silme + envanter).

pub mod anonymize;
pub mod classify;
pub mod consent;
pub mod export;
pub mod lineage_browser;
pub mod profile;
pub mod provenance;

// Sık kullanılan gizlilik API'sini modül seviyesinde yeniden dışa aktar.
pub use classify::{cikis_denetle, kume_denetle, CikisKarari, DisKanal};
pub use consent::{
    gonderim_degerlendir, GonderimDurumu, GonderimOzeti, OnayKarari, OnayKaydi, OnayTalebi,
};
pub use export::{
    guvenli_sil, tam_veri_ihrac, veri_envanteri, Envanter, IhracRaporu, IhracSecenekleri,
    SilmeRaporu, SilmeSecenekleri,
};
pub use lineage_browser::{KokenGezgini, KokenSatiri};
pub use profile::{proje_ile_coz, GizlilikProfili, TelemetriDuzeyi};
pub use provenance::{koken_ekle, kokenleri_oku, onay_ekle, LisansAtif, VeriKokeni};
