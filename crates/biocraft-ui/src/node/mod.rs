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
//! ## Yarına bırakılanlar (Gün 21)
//! Paralel çalıştırma + sonuç önbelleği + `.bcflow` kaydı + eklenti SDK node kaydı (`biocraft-sdk`).
// MK-40: L4 modülü; yalnız L0/L1/L2/L3'e bağlı.  MK-52 renk token'dan, MK-53 metin i18n'den.

pub mod canvas;
pub mod commands;
pub mod dag;
pub mod graph;
pub mod katalog;
pub mod port;

#[cfg(test)]
mod tests;

pub use canvas::NodeTuvali;
pub use graph::{
    Baglanti, BaglantiKimlik, BaglantiKontrol, Node, NodeDurumu, NodeGraf, NodeKimlik, NotKimlik,
    PortRef, YapiskanNot,
};
pub use katalog::{NodeKatalogGirdisi, NodeKatalogu};
pub use port::{Donusturucu, DonusturucuKayit, Port, PortYonu, VeriTuru};
