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

pub mod ai;
pub mod command;
pub mod components;
pub mod editor;
pub mod i18n;
pub mod node;
pub mod plot;
pub mod settings;
pub mod shell;
pub mod tipografi;
pub mod tokens;
pub mod wizard;

// egui'yi yeniden dışa aktar: üst katmanlar (launcher/app) sürüm uyumu için
// `biocraft_ui::egui` üzerinden erişebilir.
pub use egui;

// İP-14 (YZ-00/01/06/08): AI yüzeyi — sağlayıcı-bağımsız sözleşme (L3 `biocraft-ai-surface`)
// üstüne kullanıcı arayüzü.  Durum [`ai::AiYuzey`] + panel + maliyet rozeti.  Gerçek motor yok →
// "yapılandırılmadı" (MK-48); demo/örnekte echo sağlayıcı uçtan uca çalışır.
pub use ai::{ai_panel_ciz, maliyet_rozeti_ciz, AiPanelEylem, AiYuzey};
pub use biocraft_ai_surface;

pub use biocraft_mem;
pub use biocraft_render;
pub use biocraft_sdk;
pub use biocraft_state;
pub use biocraft_types;

// İP-13: Komut paleti (Ctrl+Shift+P, bulanık arama <50 ms) + özelleştirilebilir klavye kısayolları
// (çakışma uyarısı + varsayılana dön) + tuş seti profili kancası.  Menü ile palet TEK komut
// tanımına (KabukAksiyon) bağlanır (MK-51); tüm aksiyonlar klavyeyle erişilebilir (MK-52).
pub use command::{
    kisayol_penceresi, EklentiKomut, Kisayol, KisayolDuzenleyici, KisayolHaritasi, Komut,
    KomutKategori, KomutKaynak, KomutPaleti, PaletEylem, PaletModu, TusSetiProfili,
};
// İP-16 bileşenlerini kök seviyede pratik erişim için yeniden dışa aktar.
pub use components::{
    ButceDialog, ConfirmDialog, EmptyState, ErrorDialog, EstimateDialog, Gallery, IsIlerleme,
    Skeleton, StatusBadge, Toast, ToastManager,
};
pub use i18n::Dil;
// İP-06: Native kod editörü — sekme/ağaç + saf-Rust artımlı vurgulama + kodu ayrı süreçte
// çalıştırma (MK-02) + büyük dosya akışı (MK-09) + **node↔kod köprüsü (ortak workspace)** +
// **temel LSP** + **izole ortam** + **yerel geçmiş** (2. kısım, Gün 23).
pub use editor::{
    AkisGoruntuleyici, Belge, Calisma, CalismaAlani, CalismaModu, CalistirmaDurumu, DegiskenDeger,
    GecmisKaydi, KodDili, KodDugumTanimi, KodEditoru, MetinTampon, ProjeAgaci, YerelGecmis,
};
// İP-05: Node motoru (görsel akış sistemi) — tuval + tipli portlar + DAG + undo/redo +
// paralel/önbellekli çalıştırma + `.bcflow` kayıt + SVG/PNG + node→Python (Gün 21 TAM).
pub use node::{
    bcflow_kaydet, bcflow_yukle, calistir as node_calistir, png_disa_aktar, python_disa_aktar,
    svg_disa_aktar, AkisDeger, BaglantiKontrol, CalismaAyari, CalismaSonucu, IptalJetonu,
    NodeDurumu, NodeGraf, NodeKatalogu, NodeKaydi, NodeKimlik, NodeTuvali, ParametreDeger,
    Parametreler, Port, SonucOnbellek, VeriTuru, YurutucuKayit,
};
pub use plot::PlotWidget;
// İP-12: Kapsamlı, aranabilir, kategorize ayar sistemi (3. derece) — katmanlı kalıcılık +
// profil dışa/içe aktarma (hassas hariç) + eklenti ayar kaydı (SDK).
pub use settings::{
    AyarDeger, AyarEylem, AyarKategorisi, AyarKatmani, AyarKatmaniKaydi, AyarKayit, AyarProfili,
    AyarTanimi, AyarTuru, Ayarlar, SecimSecenegi, KATMAN_SURUMU,
};
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
