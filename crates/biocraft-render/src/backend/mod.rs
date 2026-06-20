//! GPU backend seçimi (İP-04): wgpu birincil, cudarc opsiyonel, CPU yazılım fallback.
//!
//! **Tek aktif backend** kuralı (MK-04 / İP-04 "tek aktif backend/workload"): wgpu ve CUDA
//! aynı anda VRAM kullanmaz.  Bu modül *hangi* backend'in seçileceğine saf mantıkla karar
//! verir; gerçek cihaz kurulumu [`crate::gpu`]'dadır.

mod cpu;
#[cfg(feature = "cuda")]
mod cuda;
// Dosya adı spec gereği `wgpu.rs`; modül adı `wgpu` crate'iyle çakışmasın diye yeniden adlandırıldı.
#[path = "wgpu.rs"]
mod wgpu_backend;

pub use cpu::cpu_notu;
pub use wgpu_backend::backend_turet;

/// Aktif çizim backend'i (aynı anda yalnızca biri etkin olur).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Donanım GPU (wgpu: DX12 / Vulkan / Metal).
    Gpu,
    /// CUDA (cudarc) — opsiyonel, `--features cuda` + uygun NVIDIA donanımı.
    Cuda,
    /// CPU yazılım rasterleştirme (wgpu fallback adapter: WARP / lavapipe vb.).
    Cpu,
}

impl Backend {
    /// Kısa, kullanıcıya gösterilebilir etiket.
    pub fn etiket(self) -> &'static str {
        match self {
            Backend::Gpu => "GPU (wgpu)",
            Backend::Cuda => "CUDA",
            Backend::Cpu => "CPU (yazılım)",
        }
    }

    /// CPU yazılım modu mu? (kullanıcıya "sınırlı performans" uyarısı için)
    pub fn yazilim_mi(self) -> bool {
        matches!(self, Backend::Cpu)
    }
}

/// Kullanıcı/işletim backend tercihi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendTercihi {
    /// Otomatik: GPU varsa GPU, yoksa CPU.
    #[default]
    Otomatik,
    /// CPU'yu zorla (test / "GPU'yu devre dışı bırak" senaryosu).
    CpuZorla,
    /// Mümkünse CUDA (yoksa GPU'ya, o da yoksa CPU'ya düşer).
    CudaTercih,
}

/// Backend seçim sonucu + gerekçe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendSecimi {
    /// Seçilen tek aktif backend.
    pub aktif: Backend,
    /// Neden bu backend seçildi.
    pub gerekce: BackendGerekce,
}

/// Backend seçim gerekçesi (kullanıcıya/loga şeffaflık).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendGerekce {
    /// Donanım GPU bulundu ve seçildi.
    DonanimGpu,
    /// Kullanıcı CPU'yu zorladı.
    CpuZorlandi,
    /// GPU yok → CPU yazılım fallback.
    GpuYokCpuFallback,
    /// CUDA istendi ve mevcut.
    CudaSecildi,
    /// CUDA istendi ama yok → GPU'ya düşüldü.
    CudaYokGpuFallback,
}

/// Saf seçim mantığı: tercih + donanım durumundan **tek aktif backend** üretir.
pub fn backend_sec(tercih: BackendTercihi, cuda_var: bool, gpu_var: bool) -> BackendSecimi {
    match tercih {
        BackendTercihi::CpuZorla => BackendSecimi {
            aktif: Backend::Cpu,
            gerekce: BackendGerekce::CpuZorlandi,
        },
        BackendTercihi::CudaTercih => {
            if cuda_var {
                BackendSecimi {
                    aktif: Backend::Cuda,
                    gerekce: BackendGerekce::CudaSecildi,
                }
            } else if gpu_var {
                BackendSecimi {
                    aktif: Backend::Gpu,
                    gerekce: BackendGerekce::CudaYokGpuFallback,
                }
            } else {
                BackendSecimi {
                    aktif: Backend::Cpu,
                    gerekce: BackendGerekce::GpuYokCpuFallback,
                }
            }
        }
        BackendTercihi::Otomatik => {
            if gpu_var {
                BackendSecimi {
                    aktif: Backend::Gpu,
                    gerekce: BackendGerekce::DonanimGpu,
                }
            } else {
                BackendSecimi {
                    aktif: Backend::Cpu,
                    gerekce: BackendGerekce::GpuYokCpuFallback,
                }
            }
        }
    }
}

/// CUDA derleme zamanında etkin mi *ve* çalışma zamanında kullanılabilir mi?
/// `cuda` feature kapalıyken daima `false` (iskelet — İP-04 / MVP-sonrası §5.2).
pub fn cuda_kullanilabilir() -> bool {
    #[cfg(feature = "cuda")]
    {
        cuda::cuda_var()
    }
    #[cfg(not(feature = "cuda"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otomatik_gpu_varsa_gpu() {
        let s = backend_sec(BackendTercihi::Otomatik, false, true);
        assert_eq!(s.aktif, Backend::Gpu);
        assert_eq!(s.gerekce, BackendGerekce::DonanimGpu);
    }

    #[test]
    fn otomatik_gpu_yoksa_cpu() {
        let s = backend_sec(BackendTercihi::Otomatik, false, false);
        assert_eq!(s.aktif, Backend::Cpu);
        assert_eq!(s.gerekce, BackendGerekce::GpuYokCpuFallback);
        assert!(s.aktif.yazilim_mi());
    }

    #[test]
    fn cpu_zorla_her_durumda_cpu() {
        let s = backend_sec(BackendTercihi::CpuZorla, true, true);
        assert_eq!(s.aktif, Backend::Cpu);
        assert_eq!(s.gerekce, BackendGerekce::CpuZorlandi);
    }

    #[test]
    fn cuda_tercih_varsa_cuda_yoksa_gpu() {
        let s = backend_sec(BackendTercihi::CudaTercih, true, true);
        assert_eq!(s.aktif, Backend::Cuda);
        let s2 = backend_sec(BackendTercihi::CudaTercih, false, true);
        assert_eq!(s2.aktif, Backend::Gpu);
        assert_eq!(s2.gerekce, BackendGerekce::CudaYokGpuFallback);
        let s3 = backend_sec(BackendTercihi::CudaTercih, false, false);
        assert_eq!(s3.aktif, Backend::Cpu);
    }

    #[test]
    fn cuda_varsayilan_kapali() {
        // Feature kapalı build'de CUDA hiçbir zaman kullanılabilir değildir.
        assert!(!cuda_kullanilabilir());
    }

    #[test]
    fn etiketler_anlamli() {
        assert_eq!(Backend::Gpu.etiket(), "GPU (wgpu)");
        assert_eq!(Backend::Cpu.etiket(), "CPU (yazılım)");
        assert!(!Backend::Gpu.yazilim_mi());
        assert!(Backend::Cpu.yazilim_mi());
    }
}
