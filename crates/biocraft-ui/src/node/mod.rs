//! Node motoru — görsel akış sistemi (İP-05, MK-54).
//!
//! Unreal Blueprint mantığında node tabanlı görsel akış motoru.  **Bugün (Gün 20) kurulan 1. kısım:**
//! tuval (pan/zoom/minimap/tümünü-sığdır), **tipli & renkli portlar**, **DAG döngü kısıtı**, node
//! durum halkaları ve **undo/redo entegrasyonu**.  Node'ların çoğu eklentilerden gelir; motor
//! temelde durur (MK-54).
//!
//! ## Katmanlar (egui'den bağımsız çekirdek + ince çizim)
//! - [`port`] — tipli/renkli portlar, uyumluluk, dönüştürücü öneri kaydı.
//! - [`graph`] — saf node grafiği (node/bağlantı/not) + bağlantı doğrulama.
//! - [`dag`] — döngü tespiti + topolojik sıralama (`petgraph`'sız).
//! - [`commands`] — geri-alınabilir düzenleme komutları ([`biocraft_state::Komut`]).
//! - [`katalog`] — palet için node türleri kaydı + demo katalog.
//! - [`canvas`] — egui tuvali (çizim + tüm etkileşim).
//!
//! ## Gün 21 — İP-05 TAMAMLANDI
//! - [`run`] — paralel (std::thread::scope) + bellek-bütçeli + hata-izolasyonlu + iptal/ilerlemeli
//!   çalıştırma motoru; bağımsız dallar eş zamanlı, OOM yok.
//! - [`cache`] — sonuç önbelleği; değişmeyen node atlanır, yalnız değişen alt-graf yeniden hesaplanır.
//! - [`serialize`] — `.bcflow` (JSON; sürüm + node id + bağlantı + parametre; göç fonksiyonu MK-59) +
//!   PNG/SVG görsel dışa aktarma (saf-Rust, dış bağımlılıksız).
//! - [`kod`] — temel node'lar için "eşdeğer Python script" dışa aktarma (Kod→Node ters yön YOK — v1.x).
//! - Eklenti entegrasyonu: node'lar `biocraft-sdk` ([`biocraft_sdk::node::NodeKaydi`]) ile kaydedilir;
//!   [`run::YurutucuKayit`] çalıştırıcıları toplar.
// MK-40: L4 modülü; yalnız L0/L1/L2/L3'e bağlı.  MK-52 renk token'dan, MK-53 metin i18n'den.

pub mod cache;
pub mod canvas;
pub mod commands;
pub mod dag;
pub mod graph;
pub mod katalog;
pub mod kod;
pub mod port;
pub mod run;
pub mod serialize;

#[cfg(test)]
mod tests;

pub use cache::SonucOnbellek;
pub use canvas::NodeTuvali;
pub use graph::{
    Baglanti, BaglantiKimlik, BaglantiKontrol, Node, NodeDurumu, NodeGraf, NodeKimlik, NotKimlik,
    PortRef, YapiskanNot,
};
pub use katalog::{NodeKatalogGirdisi, NodeKatalogu};
pub use kod::python_disa_aktar;
pub use port::{Donusturucu, DonusturucuKayit, Port, PortYonu, VeriTuru};
pub use run::{
    calistir, CalismaAyari, CalismaSonucu, IlerlemeOlay, IptalJetonu, NodeSonuc, YurutucuKayit,
};
pub use serialize::{bcflow_kaydet, bcflow_yukle, png_disa_aktar, svg_disa_aktar, BcflowBelge};

// Eklentilerin node kaydetmek için kullandığı SDK kontrat tipleri — kolay erişim için yeniden dışa aktar.
pub use biocraft_sdk::node::{
    AkisDeger, NodeCalistirici, NodeKaydi, ParametreDeger, ParametreTanimi, ParametreTuru,
    Parametreler,
};
