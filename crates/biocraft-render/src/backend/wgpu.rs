//! wgpu adapter bilgisinden [`Backend`] türetme (İP-04).
//!
//! (Bu dosya `backend::wgpu_backend` modülüdür; `wgpu` crate'iyle ad çakışmasını önlemek
//! için `#[path]` ile yeniden adlandırıldı — bkz. `backend/mod.rs`.)

use super::Backend;

/// wgpu adapter'ının cihaz türünden aktif backend'i belirler.
///
/// Yazılım rasterleştirici (WARP / lavapipe) adapter'ı [`wgpu::DeviceType::Cpu`] bildirir →
/// [`Backend::Cpu`]; aksi halde donanım GPU kabul edilir → [`Backend::Gpu`].
pub fn backend_turet(info: &wgpu::AdapterInfo) -> Backend {
    if info.device_type == wgpu::DeviceType::Cpu {
        Backend::Cpu
    } else {
        Backend::Gpu
    }
}
