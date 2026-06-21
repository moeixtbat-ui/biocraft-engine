//! biocraft-ui — L4: 6-bölge kabuk, menü, paneller, komut paleti, tuval stub (MK-51)
//! ve ortak TDA (3. derece) arayüz bileşenleri (İP-16, MK-53).
//!
//! Title+Menü / Activity / Side / Editor / Panel / Status; komut paleti <50 ms (nucleo).
//!
//! İP-16 bileşenleri [`components`] altında toplanır; tasarım token'ları [`tokens`],
//! çok dillilik [`i18n`] modülündedir.  "Bir kez yazılır, her yerde kullanılır" (MK-53).
// MK-40: L4 katmanı — yalnızca L0/L1/L2/L3 katmanlarına bağlı; üst katman yasak.
// MK-01: UI yığını = winit + wgpu + egui (Tauri/Electron/Bevy yasak).

// İP-16: `ErrorReport` projenin standart, zengin (çok-alanlı) kullanıcı-görünür hata tipidir
// (~136 bayt).  `.bcflow` yükleme gibi nadir, sıcak-olmayan yollarda bu tipi yalnızca clippy
// `result_large_err` için `Box`'lamak evrensel hata şemasının ergonomisini bozardı; bu yüzden
// lint bilinçli kapatılır (biocraft-mem ile aynı gerekçe).
#![allow(clippy::result_large_err)]

pub mod components;
pub mod i18n;
pub mod node;
pub mod plot;
pub mod shell;
pub mod tipografi;
pub mod tokens;
pub mod wizard;

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
// İP-05: Node motoru (görsel akış sistemi) — tuval + tipli portlar + DAG + undo/redo +
// paralel/önbellekli çalıştırma + `.bcflow` kayıt + SVG/PNG + node→Python (Gün 21 TAM).
pub use node::{
    bcflow_kaydet, bcflow_yukle, calistir as node_calistir, png_disa_aktar, python_disa_aktar,
    svg_disa_aktar, AkisDeger, BaglantiKontrol, CalismaAyari, CalismaSonucu, IptalJetonu,
    NodeDurumu, NodeGraf, NodeKatalogu, NodeKaydi, NodeKimlik, NodeTuvali, ParametreDeger,
    Parametreler, Port, SonucOnbellek, VeriTuru, YurutucuKayit,
};
pub use plot::PlotWidget;
// İP-03: 6-bölge ana kabuk (Title+Menü / Activity / Side / Editor / Bottom / Status + Inspector).
pub use shell::{
    aktivite_cubugu, alt_panel_ciz, baslik_cubugu, birakma_onizleme, dosya_turu,
    durum_cubugu as kabuk_durum_cubugu, yan_panel, ActivityMod, AltPanel, AltSekme,
    BirakmaOnizleme, BolmeYonu, DurumBilgisi, EditorAlani, KabukAksiyon, KapatmaIstegi, Sekme,
    SekmeGrubu, SekmeTuru,
};
pub use tipografi::{fontlari_yukle, metin_stilleri, FontDurumu};
pub use tokens::{Onem, Tema, Tokenlar};
// İP-02: Proje Sihirbazı (çok adımlı yeni-proje akışı + veri sınıflandırma zorunlu).
pub use wizard::{
    ProjeSablonu, ProjeSihirbazi, ProjeTaslagi, SihirbazAdim, SihirbazBaglam, SihirbazSonucu,
};
