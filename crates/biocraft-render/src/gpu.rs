//! wgpu cihaz/kuyruk/yüzey yönetimi (İP-04, MK-01, MK-04).
//!
//! [`GpuContext`] bir winit penceresine bağlı wgpu cihazını, kuyruğunu ve yüzeyini tutar.
//! Sürücü çökerse [`GpuContext::yeniden_kur`] cihazı/yüzeyi sıfırdan oluşturur (TDR kurtarma —
//! bkz. [`crate::tdr`]).  CPU fallback, wgpu *fallback adapter*'ı (yazılım rasterleştirici)
//! istenerek elde edilir; donanım GPU bulunamazsa otomatik olarak buna düşülür.

use std::sync::Arc;

use winit::window::Window;

use crate::backend::{backend_turet, Backend, BackendTercihi};

/// GPU başlatma/kurtarma hataları (üst katmanda İP-16 hata şemasına sarmalanır).
#[derive(Debug, thiserror::Error)]
pub enum GpuHata {
    /// Ne donanım ne de yazılım (fallback) adapter'ı bulunabildi.
    #[error("Uygun bir GPU/yazılım adapter'ı bulunamadı")]
    AdapterYok,
    /// Pencereden çizim yüzeyi (surface) oluşturulamadı.
    #[error("Çizim yüzeyi oluşturulamadı: {0}")]
    Yuzey(String),
    /// wgpu cihazı istenemedi.
    #[error("GPU cihazı istenemedi: {0}")]
    Cihaz(#[from] wgpu::RequestDeviceError),
}

/// Pencereye bağlı wgpu bağlamı: cihaz + kuyruk + yüzey + yapılandırma.
pub struct GpuContext {
    /// wgpu örneği (backend seçimi burada yapılır).
    pub instance: wgpu::Instance,
    /// Seçilen fiziksel/yazılım adapter.
    pub adapter: wgpu::Adapter,
    /// Mantıksal cihaz (kaynak oluşturma).
    pub device: wgpu::Device,
    /// Komut kuyruğu (gönderim).
    pub queue: wgpu::Queue,
    /// Pencere çizim yüzeyi (pencere `Arc` olduğundan `'static`).
    pub surface: wgpu::Surface<'static>,
    /// Yüzey yapılandırması (boyut/format/sunum modu).
    pub config: wgpu::SurfaceConfiguration,
    pencere: Arc<Window>,
    backend: Backend,
}

impl GpuContext {
    /// Verilen pencere için wgpu bağlamını kurar (senkron; içeride bloklar — UI thread'i
    /// yalnızca başlatmada kısa süre bekler).  `tercih` CPU'yu zorlayabilir; aksi halde
    /// önce donanım GPU denenir, bulunamazsa yazılım (fallback) adapter'ına otomatik düşülür.
    pub fn yeni(pencere: Arc<Window>, tercih: BackendTercihi) -> Result<Self, GpuHata> {
        pollster::block_on(Self::yeni_async(pencere, tercih))
    }

    async fn yeni_async(pencere: Arc<Window>, tercih: BackendTercihi) -> Result<Self, GpuHata> {
        // MK-01: wgpu birincil — DX12/Vulkan/Metal (Windows'ta DX12 + WARP yazılım fallback).
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(pencere.clone())
            .map_err(|e| GpuHata::Yuzey(e.to_string()))?;

        let cpu_zorla = matches!(tercih, BackendTercihi::CpuZorla);

        // Önce tercih edilen adapter (donanım veya zorlanmış yazılım).
        let mut adapter = istek_adapter(&instance, &surface, cpu_zorla).await;
        // Donanım bulunamadıysa yazılım (fallback) adapter'ına düş (İP-04 CPU fallback).
        if adapter.is_none() && !cpu_zorla {
            log::warn!("Donanım GPU adapter'ı bulunamadı → yazılım (CPU) fallback deneniyor.");
            adapter = istek_adapter(&instance, &surface, true).await;
        }
        let adapter = adapter.ok_or(GpuHata::AdapterYok)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("biocraft-cihaz"),
                    required_features: wgpu::Features::empty(),
                    // WARP/yazılım adapter'larıyla da uyumlu kalmak için düşük taban limitler.
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await?;

        let backend = backend_turet(&adapter.get_info());
        let boyut = pencere.inner_size();
        let caps = surface.get_capabilities(&adapter);
        // egui kendi gama dönüşümünü yaptığı için doğrusal (Unorm) yüzey formatı tercih eder;
        // sRGB yüzeyde renkler iki kez dönüşüp fazla parlak görünür (İP-04 token/tema doğruluğu).
        // 3B sahneler (ÇE-07) gerekirse kendi gama işini shader'da yapar.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or_else(|| caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: boyut.width.max(1),
            height: boyut.height.max(1),
            // MK-03: VSync ile sunum → kareler ekran yenilemesine senkron (60 FPS hedefi).
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        log::info!(
            "GPU bağlamı kuruldu: backend={:?} adapter='{}' format={:?}",
            backend,
            adapter.get_info().name,
            format
        );

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            config,
            pencere,
            backend,
        })
    }

    /// Aktif backend (Gpu / Cpu).
    pub fn backend(&self) -> Backend {
        self.backend
    }

    /// Seçilen adapter'ın okunabilir adı (durum çubuğu/log için).
    pub fn adapter_adi(&self) -> String {
        self.adapter.get_info().name
    }

    /// Pencere yeniden boyutlandığında yüzeyi yeniden yapılandır.
    pub fn yeniden_boyutla(&mut self, genislik: u32, yukseklik: u32) {
        if genislik > 0 && yukseklik > 0 {
            self.config.width = genislik;
            self.config.height = yukseklik;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Yüzeyi mevcut yapılandırmayla yeniden uygula (`SurfaceError::Lost/Outdated` sonrası
    /// hafif kurtarma — cihaz hâlâ sağlıklı).
    pub fn yuzey_tazele(&mut self) {
        self.surface.configure(&self.device, &self.config);
    }

    /// TDR kurtarma: cihaz/kuyruk/yüzeyi sıfırdan yeniden oluşturur (MK-04).
    /// `cpu_zorla` true ise yazılım adapter'ına düşülür (tekrarlı çökme sonrası).
    /// Pencere değişmediği için aynı pencere tutamacı yeniden kullanılır.
    pub fn yeniden_kur(&mut self, cpu_zorla: bool) -> Result<(), GpuHata> {
        let tercih = if cpu_zorla {
            BackendTercihi::CpuZorla
        } else {
            BackendTercihi::Otomatik
        };
        let yeni = Self::yeni(self.pencere.clone(), tercih)?;
        *self = yeni;
        Ok(())
    }
}

/// wgpu adapter isteği yardımcı (donanım veya fallback).
async fn istek_adapter(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'static>,
    fallback: bool,
) -> Option<wgpu::Adapter> {
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(surface),
            force_fallback_adapter: fallback,
        })
        .await
}
