//! biocraft-render — L3: yüksek performanslı çizim altyapısı (İP-04).
//!
//! Bu crate BioCraft Engine'in **GPU/performans motorudur**:
//! - [`frame_budget`] — ~16 ms/kare bütçesi, FPS ölçümü, Eco mod, GPU batch sınırı (MK-03/MK-04).
//! - [`tdr`] — GPU sürücü çökmesi (TDR/DeviceLost) kurtarma durum makinesi, <5 sn hedef (MK-04).
//! - [`backend`] — tek aktif backend seçimi: wgpu / CUDA (opsiyonel) / CPU yazılım fallback.
//! - [`gpu`] — winit penceresine bağlı wgpu cihaz/kuyruk/yüzey yönetimi (MK-01).
//!
//! **Mimari not (MK-40):** Bu katman egui'ye bağlı *değildir*.  egui↔wgpu çizim köprüsü
//! host (biocraft-app / ileride İP-03 kabuk) katmanındadır; render yalnızca üzerine çizilecek
//! GPU zeminini ve performans/kurtarma mantığını sağlar.  Böylece render saf ve test-edilebilir
//! kalır, egui sürüm bağımlılığı UI tarafında toplanır.
// MK-40: L3 katmanı — yalnızca L0/L1/L2 katmanlarına bağlı; üst katman yasak.
// MK-01: Çizim yığını = wgpu (+ host'ta egui). Bevy ECS/Tauri/Electron yasak.

pub mod backend;
pub mod frame_budget;
pub mod gpu;
pub mod tdr;

pub use backend::{
    backend_sec, cuda_kullanilabilir, Backend, BackendGerekce, BackendSecimi, BackendTercihi,
};
pub use frame_budget::{gpu_parca_boyutu, FrameBudget};
pub use gpu::{GpuContext, GpuHata};
pub use tdr::{KurtarmaPlani, TdrDurum, TdrKurtarma};

// Alt katmanları üst katmanlar için yeniden dışa aktar (sürüm uyumu).
pub use biocraft_mem;
pub use biocraft_sdk;
pub use biocraft_types;
