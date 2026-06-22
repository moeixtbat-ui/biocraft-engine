//! biocraft-render — L3: yüksek performanslı çizim altyapısı (İP-04).
//!
//! Bu crate BioCraft Engine'in **GPU/performans + tasarım motorudur**:
//! - [`frame_budget`] — ~16 ms/kare bütçesi, FPS ölçümü, Eco mod, GPU batch sınırı (MK-03/MK-04).
//! - [`tdr`] — GPU sürücü çökmesi (TDR/DeviceLost) kurtarma durum makinesi, <5 sn hedef (MK-04).
//! - [`backend`] — tek aktif backend seçimi: wgpu / CUDA (opsiyonel) / CPU yazılım fallback.
//! - [`gpu`] — winit penceresine bağlı wgpu cihaz/kuyruk/yüzey yönetimi (MK-01).
//! - [`tokens`] — tüm renklerin tek kaynağı (`assets/tokens.json`); tema + özel tema (MK-52).
//! - [`tipografi`] — font rolleri + boyut + DPI/ölçek farkındalığı (Bölüm 0.8).
//! - [`plot`] — 2B çizim (coverage/çizgi/scatter) modeli + culling/LOD.
//! - [`scene3d`] — 3B sahne temeli: geometri/kamera + wgpu çizici (kürdele/top-çubuk; ÇE-07).
//! - [`lod`] — görünür-alan culling + LOD seviye/seyreltme API'si (MK-04).
//!
//! **Mimari not (MK-40):** Bu katman egui'ye bağlı *değildir*.  egui↔wgpu çizim köprüsü ve
//! token→egui renk dönüşümü host/UI (biocraft-app, biocraft-ui) katmanındadır; render yalnızca
//! saf veri modeli (token/plot/3B geometri) + GPU zemini + performans/kurtarma mantığını sağlar.
//! Böylece render saf ve test-edilebilir kalır, egui sürüm bağımlılığı UI tarafında toplanır.
// MK-40: L3 katmanı — yalnızca L0/L1/L2 katmanlarına bağlı; üst katman yasak.
// MK-01: Çizim yığını = wgpu (+ host'ta egui). Bevy ECS/Tauri/Electron yasak.
// MK-52: tüm renkler token'dan (assets/tokens.json); kodda sabit renk yok.

pub mod backend;
pub mod frame_budget;
pub mod gpu;
pub mod lod;
pub mod olcu;
pub mod plot;
pub mod scene3d;
pub mod tdr;
pub mod tipografi;
pub mod tokens;

pub use backend::{
    backend_sec, cuda_kullanilabilir, Backend, BackendGerekce, BackendSecimi, BackendTercihi,
};
pub use frame_budget::{gpu_parca_boyutu, FrameBudget};
pub use gpu::{GpuContext, GpuHata};
pub use lod::{seyrelt, Dortgen, LodKademe, LodSecici};
pub use olcu::{
    Bosluk, Golge, Hareket, KenarKalinlik, Olcu, Yaricap, Yerlesim, Yogunluk, Yukselti,
};
pub use plot::{Aralik, CizimKomut, Nokta, Plot2B, Sekil, Seri, SeriTur};
pub use scene3d::{kure, ornek_top_cubuk, silindir, Kamera3B, Mesh, Sahne3B, Vertex};
pub use tdr::{KurtarmaPlani, TdrDurum, TdrKurtarma};
pub use tipografi::{FontAgirlik, FontRol, MetinRol, Tipografi};
pub use tokens::{Palet, Renk, Tema, TokenDeposu, TokenHata, TokenSeti, ANAHTARLAR};

// Alt katmanları üst katmanlar için yeniden dışa aktar (sürüm uyumu).
pub use biocraft_mem;
pub use biocraft_sdk;
pub use biocraft_types;
