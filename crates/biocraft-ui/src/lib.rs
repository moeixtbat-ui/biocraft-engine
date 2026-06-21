//! biocraft-ui — L4: 6-bölge kabuk, menü, paneller, komut paleti, tuval stub (MK-51)
//! ve ortak TDA (3. derece) arayüz bileşenleri (İP-16, MK-53).
//!
//! Title+Menü / Activity / Side / Editor / Panel / Status; komut paleti <50 ms (nucleo).
//!
//! İP-16 bileşenleri [`components`] altında toplanır; tasarım token'ları [`tokens`],
//! çok dillilik [`i18n`] modülündedir.  "Bir kez yazılır, her yerde kullanılır" (MK-53).
// MK-40: L4 katmanı — yalnızca L0/L1/L2/L3 katmanlarına bağlı; üst katman yasak.
// MK-01: UI yığını = winit + wgpu + egui (Tauri/Electron/Bevy yasak).

pub mod components;
pub mod i18n;
pub mod plot;
pub mod shell;
pub mod tipografi;
pub mod tokens;

// egui'yi yeniden dışa aktar: üst katmanlar (launcher/app) sürüm uyumu için
// `biocraft_ui::egui` üzerinden erişebilir.
pub use egui;

pub use biocraft_mem;
pub use biocraft_render;
pub use biocraft_sdk;
pub use biocraft_state;
pub use biocraft_types;

// İP-16 bileşenlerini kök seviyede pratik erişim için yeniden dışa aktar.
pub use components::{
    ButceDialog, ConfirmDialog, EmptyState, ErrorDialog, EstimateDialog, Gallery, IsIlerleme,
    Skeleton, StatusBadge, Toast, ToastManager,
};
pub use i18n::Dil;
pub use plot::PlotWidget;
// İP-03: 6-bölge ana kabuk (Title+Menü / Activity / Side / Status).
pub use shell::{
    aktivite_cubugu, baslik_cubugu, durum_cubugu as kabuk_durum_cubugu, yan_panel, ActivityMod,
    DurumBilgisi, KabukAksiyon,
};
pub use tipografi::{fontlari_yukle, metin_stilleri, FontDurumu};
pub use tokens::{Onem, Tema, Tokenlar};
