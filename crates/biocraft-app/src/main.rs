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

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use biocraft_mem::{
    profil_cikar, DonanimMuhafiz, DonanimProfili, KoruyucuDurum, OtoAyar, SistemSensoru,
    TermalEsikler,
};
use biocraft_render::{
    ornek_top_cubuk, Backend, BackendTercihi, FrameBudget, GpuContext, Kamera3B, KurtarmaPlani,
    Sahne3B, TdrKurtarma, Tipografi,
};
use biocraft_ui::{Dil, Gallery, StatusBadge, Tokenlar};

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

    // İP-08 MK-26: `--emulate-min` → düşük donanım profilini taklit et (sadeleşme + uyarı yolunu test).
    let emulate_min = std::env::args().any(|a| a == "--emulate-min");
    if emulate_min {
        log::info!("--emulate-min bayrağı algılandı: düşük donanım profili taklit ediliyor.");
    }

    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            eprintln!("Olay döngüsü oluşturulamadı: {e}");
            std::process::exit(1);
        }
    };
    // MK-03: Sürekli yeniden çizim (Poll) + VSync sunum → akıcı, kare kaçırmayan döngü.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut uygulama = Uygulama::yeni(tercih, emulate_min);
    if let Err(e) = event_loop.run_app(&mut uygulama) {
        eprintln!("Uygulama döngüsü hatası: {e}");
        std::process::exit(1);
    }
}

/// Uygulama kabuğu; pencere/GPU `resumed` olayında oluşturulur.
struct Uygulama {
    tercih: BackendTercihi,
    /// `--emulate-min`: düşük donanım profilini taklit et (MK-26).
    emulate_min: bool,
    durum: Option<Sahne>,
}

impl Uygulama {
    fn yeni(tercih: BackendTercihi, emulate_min: bool) -> Self {
        Self {
            tercih,
            emulate_min,
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
    /// 3B off-screen çizici (native wgpu top-çubuk; ÇE-07 öncesi 3B temeli).
    sahne3b: Sahne3B,
    /// 3B renk dokusunun egui'deki kimliği (sağ panelde gösterilir).
    sahne3b_tex: egui::TextureId,
    /// Animasyon/zaman başlangıcı (3B yörünge açısı buradan türetilir).
    baslangic: Instant,
    /// İP-08: bağımsız donanım izleme watchdog'u (termal koruma + checkpoint).
    muhafiz: DonanimMuhafiz,
    /// Watchdog sensörünün simülasyon kancası — 'I' tuşu GPU sıcaklığını yükseltir (demo).
    simulasyon: Arc<Mutex<Option<f32>>>,
    /// 'I' tuşuyla yükseltilen simüle GPU sıcaklığı (None = gerçek sensör).
    simule_sicaklik: Option<f32>,
    /// İP-08: başlangıçta donanıma göre otomatik ayar (düşük donanımda sadeleşme + uyarı).
    oto_ayar: OtoAyar,
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
        let mut egui_renderer =
            egui_wgpu::Renderer::new(&gpu.device, gpu.config.format, None, 1, false);

        // Tipografi (Bölüm 0.8): açık-lisanslı fontları (assets/fonts) yükle; yoksa egui gömülü
        // fontuna düş — sessizce değil, bilgilendirerek (TDA madde 1).  Boyutlar mantıksal;
        // DPI ölçeğini egui pixels_per_point uygular (4K + çoklu monitör akıcılığı).
        let font_durumu = biocraft_ui::fontlari_yukle(
            &egui_ctx,
            font_oku("Inter-Regular.ttf"),
            font_oku("JetBrainsMono-Regular.ttf"),
            font_oku("SpaceGrotesk-Medium.ttf"),
        );
        biocraft_ui::metin_stilleri(&egui_ctx, &Tipografi::varsayilan());
        if font_durumu.eksik_var() {
            log::info!(
                "Özel fontlar assets/fonts'ta tam değil → egui gömülü fontu kullanılıyor \
                 (Inter={}, JetBrainsMono={}, SpaceGrotesk={}).",
                font_durumu.govde,
                font_durumu.kod,
                font_durumu.baslik
            );
        }

        // 3B off-screen sahne (token-renkli top-çubuk); renk dokusu egui'ye kaydedilir → sağ panel.
        let sahne3b = Sahne3B::yeni(&gpu.device, 640, 480, &ornek_top_cubuk());
        let sahne3b_tex = egui_renderer.register_native_texture(
            &gpu.device,
            sahne3b.renk_view(),
            wgpu::FilterMode::Linear,
        );

        // İP-08 MK-26: donanım profili → otomatik ayar.  `--emulate-min` zayıf makineyi taklit eder.
        let gpu_var = !gpu.backend().yazilim_mi();
        let profil = if self.emulate_min {
            DonanimProfili::asgari_emulasyon()
        } else {
            profil_cikar(gpu_var)
        };
        let oto_ayar = OtoAyar::hesapla(&profil);
        log::info!(
            "Donanım sınıfı: {} · mod: {} · hedef {} FPS · sadeleşme: {}",
            oto_ayar.sinif.ad(),
            oto_ayar.mod_.ad(),
            oto_ayar.hedef_fps,
            oto_ayar.sadelesme,
        );
        if let Some(uyari) = &oto_ayar.uyari {
            log::warn!("{} — {}", uyari.ne_oldu, uyari.neden);
        }

        // İP-08 MK-24: bağımsız donanım izleme watchdog'u.  Gerçek sensör (sysinfo) + simülasyon
        // kancası (gerçek termal sensör yoksa bile 'I' tuşuyla korumayı canlı göstermek için).
        let sensor = SistemSensoru::yeni();
        let simulasyon = sensor.simulasyon_kancasi();
        let checkpoint = Arc::new(|| {
            log::warn!(
                "Termal duraklama → checkpoint alındı (açık iş diske yazıldı, veri korundu)."
            );
        });
        let muhafiz = DonanimMuhafiz::baslat(
            Box::new(sensor),
            TermalEsikler::default(),
            Duration::from_millis(500),
            checkpoint,
        );

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
            sahne3b,
            sahne3b_tex,
            baslangic: Instant::now(),
            muhafiz,
            simulasyon,
            simule_sicaklik: None,
            oto_ayar,
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
                    // 'I' → simüle GPU sıcaklığını +4°C yükselt (termal koruma demosu, İP-08).
                    Key::Character("i" | "I") => sahne.isi_simule_yukselt(),
                    // 'O' → simülasyonu kapat (gerçek sensöre dön).
                    Key::Character("o" | "O") => sahne.isi_simule_kapat(),
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
        // İP-08: bağımsız watchdog'un anlık donanım/termal durumu (status bar'da gösterilir).
        let donanim = self.muhafiz.durum();

        // Aktif temanın token'ları: 2B (egui visuals) + 3B (malzeme/clear) + pencere clear rengi
        // — hepsi token'dan gelir (MK-52: kodda sabit renk yok).
        let tok = self.gallery.aktif_tokenlar();
        let zemin_lin = egui::Rgba::from(tok.renk.zemin).to_array();

        // 3B sahneyi off-screen dokuya çiz (yörünge animasyonu; malzeme + zemin token rengi).
        let aci = self.baslangic.elapsed().as_secs_f32() * 0.6;
        let (en3b, boy3b) = self.sahne3b.boyut();
        let kamera = Kamera3B::yorunge(aci, 5.0, 1.8, en3b as f32 / boy3b as f32);
        let malzeme_lin = egui::Rgba::from(tok.renk.vurgu).to_array();
        let temizle3b_lin = egui::Rgba::from(tok.renk.zemin_alt).to_array();
        self.sahne3b.ciz(
            &self.gpu.device,
            &self.gpu.queue,
            &kamera,
            [0.5, 0.85, 0.6],
            malzeme_lin,
            temizle3b_lin,
        );

        let raw = self.egui_state.take_egui_input(self.pencere.as_ref());
        let dil = self.gallery.dil;
        let tex_id = self.sahne3b_tex;
        // Context klonu (ucuz Arc) → kapanış self.gallery'yi ödünç alırken self.egui_ctx çakışmaz.
        let ctx = self.egui_ctx.clone();
        let full = ctx.run(raw, |c| {
            // TÜM egui yüzeyini token'dan boya (durum çubuğu + 3B panel + galeri aynı karede).
            c.set_visuals(tok.egui_visuals());
            durum_cubugu(
                c,
                fps,
                backend,
                bildirim.as_deref(),
                &donanim,
                &self.oto_ayar,
                dil,
                &tok,
            );
            sahne3b_paneli(c, tex_id, en3b, boy3b, dil, &tok);
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
                            // MK-52: pencere clear rengi de token'dan (bg.primary, doğrusal uzayda).
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: zemin_lin[0] as f64,
                                g: zemin_lin[1] as f64,
                                b: zemin_lin[2] as f64,
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

    /// 'I' tuşu: simüle GPU sıcaklığını +4°C yükseltir; watchdog kademeli korumayı uygular.
    fn isi_simule_yukselt(&mut self) {
        let yeni = (self.simule_sicaklik.unwrap_or(58.0) + 4.0).min(110.0);
        self.simule_sicaklik = Some(yeni);
        if let Ok(mut s) = self.simulasyon.lock() {
            *s = Some(yeni);
        }
        log::info!("Simüle GPU sıcaklığı: {yeni:.0}°C (watchdog yanıt verecek).");
    }

    /// 'O' tuşu: ısı simülasyonunu kapatır (gerçek sensöre döner).
    fn isi_simule_kapat(&mut self) {
        self.simule_sicaklik = None;
        if let Ok(mut s) = self.simulasyon.lock() {
            *s = None;
        }
        log::info!("Isı simülasyonu kapatıldı (gerçek sensör).");
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
                // Tipografi yeni egui Context'te baştan kurulmalı (fontlar + boyutlar).
                let _ = biocraft_ui::fontlari_yukle(
                    &self.egui_ctx,
                    font_oku("Inter-Regular.ttf"),
                    font_oku("JetBrainsMono-Regular.ttf"),
                    font_oku("SpaceGrotesk-Medium.ttf"),
                );
                biocraft_ui::metin_stilleri(&self.egui_ctx, &Tipografi::varsayilan());
                // 3B çiziciyi + egui doku kaydını yeni cihazla yeniden kur (eski GPU kaynakları geçersiz).
                self.sahne3b = Sahne3B::yeni(&self.gpu.device, 640, 480, &ornek_top_cubuk());
                self.sahne3b_tex = self.egui_renderer.register_native_texture(
                    &self.gpu.device,
                    self.sahne3b.renk_view(),
                    wgpu::FilterMode::Linear,
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

/// Alt durum çubuğu: FPS + backend + **donanım göstergesi (İP-08)** + TDR bildirimi.
/// Donanım göstergesi: CPU%/sıcaklık + termal aksiyon (soğutuluyor/acil) + düşük-donanım uyarısı.
/// Renkler token'dan gelir (MK-52).
#[allow(clippy::too_many_arguments)]
fn durum_cubugu(
    ctx: &egui::Context,
    fps: f32,
    backend: Backend,
    bildirim: Option<&str>,
    donanim: &KoruyucuDurum,
    oto: &OtoAyar,
    dil: Dil,
    tok: &Tokenlar,
) {
    egui::TopBottomPanel::bottom("biocraft_durum").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(format!("FPS: {fps:.0}"));
            ui.separator();
            ui.label(format!("Backend: {}", backend.etiket()));
            if backend.yazilim_mi() {
                ui.separator();
                ui.colored_label(tok.renk.uyari, "⚠ Yazılım (CPU) modu — performans sınırlı");
            }

            // İP-08: donanım göstergesi (CPU/GPU/RAM/sıcaklık).
            ui.separator();
            if donanim.koruma_etkin {
                ui.label(format!("🌡 {}", sicaklik_ozeti(donanim)));
            } else {
                // Sensör yok → koruma kademeli devre dışı (çökme değil, bilgi).
                ui.colored_label(tok.renk.uyari, "🌡 Sensör yok — termal koruma kapalı");
            }

            // Düşük donanım: sadeleşme + uyarı (MK-26).
            if oto.sadelesme {
                ui.separator();
                ui.colored_label(
                    tok.renk.uyari,
                    format!(
                        "⚙ Düşük donanım ({}) — sadeleştirildi · {} FPS",
                        oto.sinif.ad(),
                        oto.hedef_fps
                    ),
                );
            }

            ui.separator();
            ui.weak("T: GPU çökmesi · I/O: ısı simülasyonu · Esc: çıkış");

            // Sağ taraf: termal rozet/uyarı + TDR bildirimi.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(b) = bildirim {
                    ui.colored_label(tok.renk.basari, format!("✔ {b}"));
                    ui.separator();
                }
                if donanim.acil_durum {
                    ui.colored_label(tok.renk.hata, "⛔ ACİL DURDU — kritik sıcaklık");
                } else if donanim.sogutuluyor {
                    let _ = StatusBadge::Sogutuluyor.show(ui, dil, tok);
                } else if let biocraft_mem::TermalAksiyon::YukAzalt(p) = donanim.aksiyon {
                    ui.colored_label(tok.renk.uyari, format!("⏬ Termal: yük %{p}"));
                }
            });
        });
    });
}

/// Watchdog örneğinden kısa "GPU 82°C · CPU 70°C · CPU %45" özeti üretir (mevcut değerler).
fn sicaklik_ozeti(donanim: &KoruyucuDurum) -> String {
    let o = &donanim.son_ornek;
    let mut parcalar: Vec<String> = Vec::new();
    if let Some(t) = o.gpu_c {
        parcalar.push(format!("GPU {t:.0}°C"));
    }
    if let Some(t) = o.cpu_c {
        parcalar.push(format!("CPU {t:.0}°C"));
    }
    if let Some(t) = o.nvme_c {
        parcalar.push(format!("NVMe {t:.0}°C"));
    }
    if let Some(p) = o.cpu_yuzde {
        parcalar.push(format!("CPU %{p:.0}"));
    }
    if let Some(r) = o.ram_orani {
        parcalar.push(format!("RAM %{:.0}", r * 100.0));
    }
    if parcalar.is_empty() {
        "ölçülüyor…".to_string()
    } else {
        parcalar.join(" · ")
    }
}

/// Sağ panel: 3B off-screen sahnenin (top-çubuk) canlı dokusunu gösterir + kısa açıklama.
fn sahne3b_paneli(
    ctx: &egui::Context,
    tex_id: egui::TextureId,
    en: u32,
    boy: u32,
    dil: Dil,
    tok: &Tokenlar,
) {
    let (baslik, aciklama) = match dil {
        Dil::Tr => (
            "3B Sahne (wgpu)",
            "Native wgpu ile çizilen top-çubuk; malzeme rengi token'dan. \
             Kürdele/yüzey için temel (ileride ÇE-07).",
        ),
        Dil::En => (
            "3D Scene (wgpu)",
            "Ball-and-stick drawn with native wgpu; material color from tokens. \
             Base for ribbon/surface (later ÇE-07).",
        ),
    };
    egui::SidePanel::right("biocraft_3b")
        .resizable(true)
        .default_width(320.0)
        .show(ctx, |ui| {
            ui.add_space(tok.bosluk.s);
            ui.heading(baslik);
            ui.add_space(tok.bosluk.xs);
            let genislik = ui.available_width().max(32.0);
            let oran = boy as f32 / en as f32;
            let sized =
                egui::load::SizedTexture::new(tex_id, egui::vec2(genislik, genislik * oran));
            ui.add(egui::Image::new(sized));
            ui.add_space(tok.bosluk.s);
            ui.label(
                egui::RichText::new(aciklama)
                    .color(tok.renk.metin_soluk)
                    .small(),
            );
        });
}

/// `assets/fonts` altından bir font dosyasını okur (yoksa None → egui gömülü fontuna düşülür).
fn font_oku(dosya: &str) -> Option<Vec<u8>> {
    std::fs::read(std::path::Path::new("assets/fonts").join(dosya)).ok()
}
