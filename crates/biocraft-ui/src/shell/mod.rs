//! 6-bölge ana kabuk (İP-03, MK-51) — VS Code benzeri pencere düzeni.
//!
//! 0.9 tablosundaki bölgeleri egui panelleriyle kurar.  **Bu gün (Gün 11) dört bölge + menü**
//! teslim edilir; Editor/Canvas sekmeleri + alt Panel Gün 12'de eklenir.
//!
//! | Bölge | Modül | Boyut |
//! | --- | --- | --- |
//! | Title Bar (+ menü) | [`title_bar`] / [`menu_bar`] | 32 px (üst) |
//! | Activity Bar | [`activity_bar`] | 48 px (sol) |
//! | Side Panel | [`side_panel`] | 200–600 px (yeniden boyutlanır) |
//! | Status Bar | [`status_bar`] | 22 px (alt) |
//!
//! **Tasarım ilkesi:** Saf mantık (bölge ölçüleri, mod↔seçim eşlemesi, aksiyon tanımları)
//! egui'den ayrı ve test edilebilir ([`layout`], [`activity_bar`], [`menu_bar`]); egui çizimi
//! ince bir kabuktur.  Kalıcı durum (seçili mod, panel genişliği) `biocraft-state`'tedir
//! (immediate-mode'da düzen ayrı state'te tutulur — İP-11 ile uyumlu).
//!
//! Tüm renkler token'dan (MK-52), tüm metinler i18n'den (MK-53); ölçüler mantıksal piksel olduğundan
//! DPI/4K/çoklu monitör ölçeklemesi egui `pixels_per_point` ile akıcıdır.

pub mod activity_bar;
pub mod bottom_panel;
pub mod editor_area;
pub mod layout;
pub mod menu_bar;
pub mod side_panel;
pub mod split;
pub mod status_bar;
pub mod title_bar;

pub use activity_bar::{aktivite_cubugu, ActivityMod};
pub use bottom_panel::{alt_panel_ciz, AltPanel, AltSekme};
pub use editor_area::{
    birakma_onizleme, dosya_turu, BirakmaOnizleme, EditorAlani, KapatmaIstegi, Sekme, SekmeGrubu,
    SekmeTuru,
};
pub use layout::{
    yan_panel_araligi, yan_panel_sikistir, AKTIVITE_GENISLIK, BASLIK_YUKSEKLIK, DURUM_YUKSEKLIK,
    YAN_PANEL_MAX, YAN_PANEL_MIN, YAN_PANEL_VARSAYILAN,
};
pub use menu_bar::{menu_cubugu, KabukAksiyon};
pub use side_panel::yan_panel;
pub use split::{bol_boyut, BolmeYonu};
pub use status_bar::{durum_cubugu, DurumBilgisi};
pub use title_bar::baslik_cubugu;
