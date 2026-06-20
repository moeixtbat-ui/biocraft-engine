//! biocraft-app — L5: Ana binary; winit + wgpu + egui pencere host'u (İP-04, MK-01).
//!
//! Açılışta gerçek bir pencere açar ve İP-16 örnek galerisini ([`biocraft_ui::Gallery`])
//! canlı çizer.  Üç temel güvence burada birleşir:
//! - **Kare bütçesi (MK-03):** her kare ölçülür, FPS durum çubuğunda gösterilir; VSync ile ~60 FPS.
//! - **GPU TDR kurtarma (MK-04):** `T` tuşu sürücü çökmesini simüle eder; cihaz <5 sn'de yeniden
//!   kurulur, "GPU yeniden başlatıldı" bildirimi gösterilir, uygulama **çökmez**.
//! - **CPU fallback:** GPU yoksa (veya `--cpu` ile) yazılım rasterleştiriciyle akıcı pencere + uyarı.
//!
//! Render altyapısı (cihaz/kuyruk/kurtarma/bütçe) [`biocraft_render`]'dadır; egui↔wgpu çizim
//! köprüsü bu host katmanındadır (MK-40: render egui'ye bağlı değildir).

use std::sync::Arc;
use std::time::{Duration, Instant};

use biocraft_render::{
    Backend, BackendTercihi, FrameBudget, GpuContext, KurtarmaPlani, TdrKurtarma,
};
use biocraft_ui::Gallery;

use egui_wgpu::ScreenDescriptor;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

fn main() {
    // MK-08: Aşamalı başlatma. Kendi katmanlarımız "info"; wgpu/naga arka plan gürültüsü
    // (Vulkan yükleyici mesajları vb.) "warn/error" ile susturulur (RUST_LOG ile değişir).
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("info,wgpu_core=warn,wgpu_hal=error,naga=warn,wgpu=warn"),
    )
    .init();

    // Basit CLI: `--cpu` → yazılım (CPU) backend'ini zorla (GPU'yu devre dışı bırakma testi).
    let tercih = if std::env::args().any(|a| a == "--cpu") {
        log::info!("--cpu bayrağı algılandı: yazılım (CPU) backend'i zorlanıyor.");
        BackendTercihi::CpuZorla
    } else {
        BackendTercihi::Otomatik
    };

    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            eprintln!("Olay döngüsü oluşturulamadı: {e}");
            std::process::exit(1);
        }
    };
    // MK-03: Sürekli yeniden çizim (Poll) + VSync sunum → akıcı, kare kaçırmayan döngü.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut uygulama = Uygulama::yeni(tercih);
    if let Err(e) = event_loop.run_app(&mut uygulama) {
        eprintln!("Uygulama döngüsü hatası: {e}");
        std::process::exit(1);
    }
}

/// Uygulama kabuğu; pencere/GPU `resumed` olayında oluşturulur.
struct Uygulama {
    tercih: BackendTercihi,
    durum: Option<Sahne>,
}

impl Uygulama {
    fn yeni(tercih: BackendTercihi) -> Self {
        Self {
            tercih,
            durum: None,
        }
    }
}

/// Pencere + GPU + egui durumu (resumed sonrası yaşar).
struct Sahne {
    pencere: Arc<Window>,
    gpu: GpuContext,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    gallery: Gallery,
    budget: FrameBudget,
    tdr: TdrKurtarma,
    /// "GPU yeniden başlatıldı" bildirimi (metin + gösterim başlangıcı).
    tdr_bildirim: Option<(String, Instant)>,
}

impl ApplicationHandler for Uygulama {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.durum.is_some() {
            return; // yalnızca bir kez kur (masaüstünde resumed tek kez tetiklenir).
        }

        let pencere = match event_loop.create_window(
            Window::default_attributes()
                .with_title("BioCraft Engine — İP-04 Render Host")
                .with_inner_size(LogicalSize::new(1280.0, 800.0)),
        ) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                log::error!("Pencere oluşturulamadı: {e}");
                event_loop.exit();
                return;
            }
        };

        let gpu = match GpuContext::yeni(pencere.clone(), self.tercih) {
            Ok(g) => g,
            Err(e) => {
                log::error!("GPU başlatılamadı: {e}");
                event_loop.exit();
                return;
            }
        };
        log::info!(
            "Render host hazır — backend: {} ({})",
            gpu.backend().etiket(),
            gpu.adapter_adi()
        );

        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            pencere.as_ref(),
            Some(pencere.scale_factor() as f32),
            None,
            Some(2048),
        );
        let egui_renderer =
            egui_wgpu::Renderer::new(&gpu.device, gpu.config.format, None, 1, false);

        self.durum = Some(Sahne {
            pencere,
            gpu,
            egui_ctx,
            egui_state,
            egui_renderer,
            gallery: Gallery::new(),
            budget: FrameBudget::varsayilan(),
            tdr: TdrKurtarma::yeni(),
            tdr_bildirim: None,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(sahne) = self.durum.as_mut() else {
            return;
        };

        // Olayı egui'ye ilet (girdi/işaretçi/IME).
        let yanit = sahne
            .egui_state
            .on_window_event(sahne.pencere.as_ref(), &event);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(boyut) => {
                sahne.gpu.yeniden_boyutla(boyut.width, boyut.height);
                sahne.pencere.request_redraw();
            }
            WindowEvent::KeyboardInput { event: ke, .. }
                if ke.state == ElementState::Pressed && !ke.repeat =>
            {
                match ke.logical_key.as_ref() {
                    // 'T' → GPU sürücü çökmesi (TDR/DeviceLost) simülasyonu.
                    Key::Character("t" | "T") => sahne.tdr_simule(),
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => sahne.ciz(),
            _ => {}
        }

        if yanit.repaint {
            sahne.pencere.request_redraw();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Sürekli animasyon/FPS için her boşta turunda yeniden çizim iste.
        if let Some(sahne) = self.durum.as_ref() {
            sahne.pencere.request_redraw();
        }
    }
}

impl Sahne {
    /// Bir kareyi çiz: egui çalıştır → tessellate → wgpu ile sun.  Kare süresi ölçülür (MK-03).
    fn ciz(&mut self) {
        let kare_basi = Instant::now();

        // Süresi dolan TDR bildirimini temizle (~4 sn göster).
        if let Some((_, gosterim)) = &self.tdr_bildirim {
            if gosterim.elapsed() > Duration::from_secs(4) {
                self.tdr_bildirim = None;
                self.tdr.bildirim_gosterildi();
            }
        }

        let fps = self.budget.fps();
        let backend = self.gpu.backend();
        let bildirim = self.tdr_bildirim.as_ref().map(|(m, _)| m.clone());

        let raw = self.egui_state.take_egui_input(self.pencere.as_ref());
        // Context klonu (ucuz Arc) → kapanış self.gallery'yi ödünç alırken self.egui_ctx çakışmaz.
        let ctx = self.egui_ctx.clone();
        let full = ctx.run(raw, |c| {
            durum_cubugu(c, fps, backend, bildirim.as_deref());
            self.gallery.show(c);
        });

        self.egui_state
            .handle_platform_output(self.pencere.as_ref(), full.platform_output);
        let jobs = self.egui_ctx.tessellate(full.shapes, full.pixels_per_point);
        let ekran = ScreenDescriptor {
            size_in_pixels: [self.gpu.config.width, self.gpu.config.height],
            pixels_per_point: full.pixels_per_point,
        };

        // Yüzey dokusunu al; kayıp/eskimişse tazele, bellek biterse cihazı kurtar (MK-04).
        let cikis = match self.gpu.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("Yüzey belleği tükendi → cihaz kurtarma deneniyor.");
                self.cihaz_kurtar();
                return;
            }
            Err(hata) => {
                log::debug!("Yüzey hatası ({hata:?}) → tazeleniyor, kare atlanıyor.");
                self.gpu.yuzey_tazele();
                return;
            }
        };
        let view = cikis
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("biocraft-encoder"),
            });

        for (id, delta) in &full.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.gpu.device, &self.gpu.queue, *id, delta);
        }
        let kullanici_komutlari = self.egui_renderer.update_buffers(
            &self.gpu.device,
            &self.gpu.queue,
            &mut encoder,
            &jobs,
            &ekran,
        );

        {
            let mut rpass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("biocraft-egui-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.04,
                                g: 0.05,
                                b: 0.07,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                // wgpu 22: egui_wgpu::Renderer::render 'static render pass bekler.
                .forget_lifetime();
            self.egui_renderer.render(&mut rpass, &jobs, &ekran);
        }

        for id in &full.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        // egui geri-arama komutları (varsa) render pass'ten ÖNCE; ardından ana encoder.
        self.gpu.queue.submit(
            kullanici_komutlari
                .into_iter()
                .chain(std::iter::once(encoder.finish())),
        );
        cikis.present();

        // MK-03: kare süresi kaydı + Eco mod (statik ekranda FPS düşürme) tespiti.
        self.budget.kare_kaydet(kare_basi.elapsed());
        if self.egui_ctx.has_requested_repaint() {
            self.budget.etkinlik_var();
        } else {
            self.budget.bosta();
        }
    }

    /// 'T' tuşu: GPU sürücü çökmesini (TDR/DeviceLost) simüle eder.
    fn tdr_simule(&mut self) {
        log::warn!("TDR/DeviceLost simülasyonu tetiklendi (kullanıcı 'T' tuşu).");
        self.cihaz_kurtar();
    }

    /// Cihazı yeniden kurarak TDR kurtarmasını çalıştırır (MK-04: hedef <5 sn).
    fn cihaz_kurtar(&mut self) {
        let plan = self.tdr.cihaz_kayboldu();
        let cpu_zorla = matches!(plan, KurtarmaPlani::CpuyaDus);
        let basla = Instant::now();
        match self.gpu.yeniden_kur(cpu_zorla) {
            Ok(()) => {
                // Cihaz değişti → egui yığınını tazele. Yeni bir Context, dokuları (font atlası
                // vb.) yeni renderer'a baştan yükletir; yalnızca renderer'ı yenilemek eski doku
                // kimliklerini geçersiz bırakıp ikinci bir çökmeye yol açardı.
                let yeni_ctx = egui::Context::default();
                self.egui_state = egui_winit::State::new(
                    yeni_ctx.clone(),
                    egui::ViewportId::ROOT,
                    self.pencere.as_ref(),
                    Some(self.pencere.scale_factor() as f32),
                    None,
                    Some(2048),
                );
                self.egui_ctx = yeni_ctx;
                self.egui_renderer = egui_wgpu::Renderer::new(
                    &self.gpu.device,
                    self.gpu.config.format,
                    None,
                    1,
                    false,
                );
                let gecen = basla.elapsed();
                self.tdr.cihaz_kuruldu(gecen);
                let ms = gecen.as_millis();
                let mesaj = if self.tdr.hedefte_mi(gecen) {
                    format!(
                        "GPU yeniden başlatıldı ({ms} ms) — {}",
                        self.gpu.backend().etiket()
                    )
                } else {
                    format!(
                        "GPU yeniden başlatıldı ({ms} ms — hedefin üzerinde!) — {}",
                        self.gpu.backend().etiket()
                    )
                };
                log::info!("{mesaj}");
                self.tdr_bildirim = Some((mesaj, Instant::now()));
            }
            Err(e) => {
                log::error!("Cihaz kurtarma başarısız: {e}");
                self.tdr_bildirim = Some((format!("GPU kurtarma başarısız: {e}"), Instant::now()));
            }
        }
    }
}

/// Alt durum çubuğu: FPS + aktif backend + (varsa) CPU uyarısı + TDR bildirimi.
fn durum_cubugu(ctx: &egui::Context, fps: f32, backend: Backend, bildirim: Option<&str>) {
    egui::TopBottomPanel::bottom("biocraft_durum").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(format!("FPS: {fps:.0}"));
            ui.separator();
            ui.label(format!("Backend: {}", backend.etiket()));
            if backend.yazilim_mi() {
                ui.separator();
                ui.colored_label(
                    egui::Color32::from_rgb(220, 160, 40),
                    "⚠ Yazılım (CPU) modu — performans sınırlı",
                );
            }
            ui.separator();
            ui.weak("T: GPU çökmesi simülasyonu · Esc: çıkış");
            if let Some(b) = bildirim {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(egui::Color32::from_rgb(90, 200, 120), format!("✔ {b}"));
                });
            }
        });
    });
}
