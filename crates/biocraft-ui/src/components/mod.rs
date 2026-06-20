//! İP-16 — ortak TDA (3. derece) arayüz bileşenleri (MK-53).
//!
//! "Bir kez yazılır, her yerde kullanılır": diğer paketler kendi bildirim/hata/boş
//! durumlarını **kopyalamaz**, buradan yeniden kullanır.  Tutarlılık (TDA madde 14)
//! buradan gelir.  Tüm bileşenler:
//! - rengi [`crate::tokens`]'dan alır (tema-duyarlı),
//! - metnini [`crate::i18n`]'den alır (EN/TR),
//! - standart egui widget'ları (buton/etiket) kullanır → klavye ve ekran okuyucu erişimi
//!   (AccessKit) İP-04 pencere host'u tarafından otomatik sağlanır.

pub mod confirm;
pub mod empty_state;
pub mod error_dialog;
pub mod estimate;
pub mod gallery;
pub mod progress;
pub mod skeleton;
pub mod status_badge;
pub mod toast;

pub use confirm::{ConfirmDialog, OnayKarari};
pub use empty_state::EmptyState;
pub use error_dialog::{ErrorDialog, HataDiyalogEylem};
pub use estimate::{EstimateDialog, TahminKarari};
pub use gallery::Gallery;
pub use progress::{IlerlemeEylem, IsIlerleme};
pub use skeleton::Skeleton;
pub use status_badge::{RozetEylem, StatusBadge};
pub use toast::{Toast, ToastEylem, ToastKind, ToastManager};

use crate::tokens::Tokenlar;

/// Token'lara göre standart bir "kart" çerçevesi üretir (yüzey + kenarlık + yuvarlatma).
/// Bileşenler arası görsel tutarlılığı (TDA madde 14) sağlar.
pub(crate) fn kart_cercevesi(tok: &Tokenlar) -> egui::Frame {
    egui::Frame {
        fill: tok.renk.yuzey,
        stroke: egui::Stroke::new(1.0, tok.renk.kenarlik),
        rounding: egui::Rounding::same(tok.yaricap),
        inner_margin: egui::Margin::same(tok.bosluk.m),
        ..Default::default()
    }
}

/// Token'lara göre renkli bir "bildirim/zemin" çerçevesi üretir (önem zemini + vurgu kenarı).
pub(crate) fn vurgu_cercevesi(
    zemin: egui::Color32,
    kenar: egui::Color32,
    tok: &Tokenlar,
) -> egui::Frame {
    egui::Frame {
        fill: zemin,
        stroke: egui::Stroke::new(1.0, kenar),
        rounding: egui::Rounding::same(tok.yaricap),
        inner_margin: egui::Margin::same(tok.bosluk.m),
        ..Default::default()
    }
}
