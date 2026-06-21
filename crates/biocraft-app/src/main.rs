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

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use biocraft_mem::{
    profil_cikar, DonanimMuhafiz, DonanimProfili, OtoAyar, SistemSensoru, TermalEsikler,
};
use biocraft_render::{
    ornek_top_cubuk, BackendTercihi, FrameBudget, GpuContext, Kamera3B, KurtarmaPlani, Sahne3B,
    TdrKurtarma, Tipografi,
};
// İP-11: self-healing durum altyapısı (kalıcı durum + otomatik kayıt + çökme kurtarma).
use biocraft_state::{
    DilSecimi, DosyaDepo, DurumYoneticisi, KabukDurumu, KurtarmaKarari, TemaSecimi,
};
// İP-11 Gün 10: geri-al/yinele motoru + çakışma tespiti + yerel geçmiş (canlı demo).
use biocraft_state::{
    simdi, AcikSekme, CakismaBilgisi, CakismaIzleyici, CakismaKarari, GeriAlYigini, Komut,
    PanelGenisligiDegistir, SekmeEkle, SekmeKapat, SurumDamgasi, TemaDegistir, UygulamaDurumu,
    YerelGecmis,
};
// İP-03: ana kabuk (Title+Menü / Activity / Side / Editör+split / Alt panel / Inspector / Status).
use biocraft_ui::components::{ConfirmDialog, OnayKarari};
use biocraft_ui::{
    aktivite_cubugu, alt_panel_ciz, baslik_cubugu, birakma_onizleme, kabuk_durum_cubugu, yan_panel,
    ActivityMod, AltPanel, AltSekme, BolmeYonu, Dil, DurumBilgisi, EditorAlani, Gallery,
    KabukAksiyon, KapatmaIstegi, SekmeTuru, Tema, Tokenlar,
};

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
    /// İP-11: self-healing durum yöneticisi (kalıcı durum + otomatik kayıt + kurtarma).
    yonetici: DurumYoneticisi,
    /// İP-11: açılışta önceki oturumun düzgün kapanıp kapanmadığı kararı (çökme kurtarma).
    kurtarma_karari: KurtarmaKarari,
}

impl Uygulama {
    fn yeni(tercih: BackendTercihi, emulate_min: bool) -> Self {
        // İP-11/MK-38: kalıcı durumu disk üzerinde (kullanıcı veri klasörü) tut.
        let dizin = durum_dizini();
        log::info!("Durum klasörü: {}", dizin.display());
        let depo = Box::new(DosyaDepo::yeni(&dizin));
        let (yonetici, kurtarma_karari) = DurumYoneticisi::ac(depo, Instant::now());
        if kurtarma_karari.kurtarma_mi() {
            log::warn!(
                "Önceki oturum düzgün kapanmamış → açılışta 'kurtarılan oturum' sunulacak (MK-28)."
            );
        }
        Self {
            tercih,
            emulate_min,
            durum: None,
            yonetici,
            kurtarma_karari,
        }
    }

    /// Arayüzden türeyen kalıcı durumu (tema/dil/pencere/panel) yöneticiyle eşitler ve
    /// otomatik kayıt zamanı geldiyse diske yazar (MK-38: periyodik + değişiklikte).
    fn senkron_ve_kaydet(&mut self) {
        // 1) Arayüzün güncel durumunu oku (Sahne ödünç alımı bu blokta biter).
        let okunan = {
            let Some(sahne) = self.durum.as_ref() else {
                return;
            };
            let olcek = sahne.pencere.scale_factor() as f32;
            let boyut = sahne.pencere.inner_size();
            (
                (boyut.width as f32 / olcek).round() as u32,
                (boyut.height as f32 / olcek).round() as u32,
                sahne.pencere.is_maximized(),
                tema_durum(sahne.gallery.tema),
                dil_durum(sahne.gallery.dil),
                sahne.son_panel_genislik,
                sahne.kabuk_durumu_oku(),
            )
        };
        let (genislik, yukseklik, buyutulmus, tema, dil, panel_w, kabuk) = okunan;

        // 2) Değişen bir şey varsa durumu güncelle (kirli işaretle → otomatik kayıt tetiklenir).
        let d = self.yonetici.durum();
        let degisti = d.tema != tema
            || d.dil != dil
            || d.pencere.genislik != genislik
            || d.pencere.yukseklik != yukseklik
            || d.pencere.buyutulmus != buyutulmus
            || (d.panel.sag_panel_genislik - panel_w).abs() > 0.5
            || kabuk_farkli(&d.kabuk, &kabuk);
        let simdi = Instant::now();
        if degisti {
            self.yonetici.durum_guncelle(
                |d| {
                    d.tema = tema;
                    d.dil = dil;
                    d.pencere.genislik = genislik;
                    d.pencere.yukseklik = yukseklik;
                    d.pencere.buyutulmus = buyutulmus;
                    d.panel.sag_panel_genislik = panel_w;
                    d.kabuk = kabuk;
                },
                simdi,
            );
        }

        // 3) Otomatik kayıt zamanı geldiyse yaz (sessizce başarısız olma — MK-28 kural 3).
        if let Err(e) = self.yonetici.belki_kaydet(simdi) {
            log::warn!(
                "Otomatik kayıt başarısız: {} [{}]",
                e.neden,
                e.correlation_id.kisa()
            );
        }
    }
}

/// İki kabuk düzeni "anlamlı ölçüde" farklı mı? (f32 ölçülerde sub-piksel gürültüyü yok say.)
fn kabuk_farkli(a: &KabukDurumu, b: &KabukDurumu) -> bool {
    a.aktif_mod != b.aktif_mod
        || a.yan_panel_acik != b.yan_panel_acik
        || (a.yan_panel_genislik - b.yan_panel_genislik).abs() > 0.5
        || a.alt_panel_acik != b.alt_panel_acik
        || (a.alt_panel_yukseklik - b.alt_panel_yukseklik).abs() > 0.5
        || a.alt_panel_sekme != b.alt_panel_sekme
        || a.inspector_acik != b.inspector_acik
        || (a.inspector_genislik - b.inspector_genislik).abs() > 0.5
        || a.bolme_yonu != b.bolme_yonu
        || (a.bolme_orani - b.bolme_orani).abs() > 0.01
        || a.yogun_mod != b.yogun_mod
}

/// Düzgün kapanış: durumu kaydet + oturumu "temiz" işaretle (sonraki açılışta kurtarma sunulmaz).
///
/// Serbest fonksiyondur: yalnızca `DurumYoneticisi`'ne dokunur; böylece pencere olayı sırasında
/// `Sahne` (= `&mut self.durum`) ödünç alınmışken bile (ayrık alan) güvenle çağrılabilir.
fn temiz_kapat_yap(yonetici: &mut DurumYoneticisi) {
    if let Err(e) = yonetici.temiz_kapat(Instant::now()) {
        log::warn!(
            "Kapanışta durum kaydedilemedi: {} [{}]",
            e.neden,
            e.correlation_id.kisa()
        );
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
    /// İP-11: sağ panelin son ölçülen genişliği (kalıcı duruma yazılır → oturumlar arası korunur).
    son_panel_genislik: f32,
    /// İP-11: açılışta "kurtarılan oturum" bandı gösterilsin mi (çökme sonrası; kullanıcı kapatınca biter).
    kurtarma_sunulacak: bool,
    /// İP-11 Gün 10: geri-al/yinele + çakışma + yerel geçmiş canlı demosu (yüzen pencere).
    duzenleme: DuzenlemeDemo,
    /// İP-03: Activity Bar'da seçili ana mod (Side Panel içeriğini belirler; kalıcı).
    aktif_mod: ActivityMod,
    /// İP-03: Side Panel açık mı (Görünüm → Yan Paneli Aç/Kapa; kalıcı).
    yan_panel_acik: bool,
    /// İP-03: Side Panel'in son ölçülen genişliği (kalıcı duruma yazılır → oturumlar arası korunur).
    yan_panel_genislik: f32,
    /// İP-03: kabuk aksiyonları için kısa süreli durum bildirimi (örn. "Komut paleti yakında").
    kabuk_bildirim: Option<(String, Instant)>,
    // ── İP-03 Gün 12: editör sekmeleri + split + alt panel + inspector + özel düzen ──
    /// Editör/Canvas alanı (sekmeli + yan-yana bölme).
    editor: EditorAlani,
    /// Alt Panel (Konsol/İşler/AI/Günlük).
    alt_panel: AltPanel,
    /// Inspector (sağ özellik paneli) docked olarak açık mı?
    inspector_acik: bool,
    /// Inspector'ın son ölçülen genişliği (kalıcı).
    inspector_genislik: f32,
    /// Yoğun mod açık mı? (kapalı = sade mod, daha geniş boşluk.)
    yogun_mod: bool,
    /// Bileşen demoları (galeri) merkezde açık mı? (editör yerine geçer.)
    gallery_acik: bool,
    /// Kaydedilmemiş sekme kapatma onayı (Gün-4 onay diyaloğu) — bekleyen istek + diyalog.
    kapatma_onayi: Option<(KapatmaIstegi, ConfirmDialog)>,
    /// Özel düzen yöneticisi penceresi açık mı?
    duzen_penceresi_acik: bool,
    /// Düzen yöneticisindeki ad girdisi (kaydetmek için).
    duzen_ad: String,
    /// Inspector ayrı pencereye (detach) taşındıysa o pencerenin kaynakları.
    detach: Option<DetachPenceresi>,
    /// Bu karede detach geçişi istendi mi (ana döngü pencere oluşturur/kapatır).
    detach_toggle_istendi: bool,
    /// Backend tercihi (detach penceresinin GPU bağlamını ana pencereyle aynı kurmak için).
    tercih: BackendTercihi,
}

/// Ayrılmış (detach) bir panelin ayrı OS penceresi — kendi GPU yüzeyi + egui bağlamı (İP-03).
///
/// Çoklu monitör + DPI ölçekleme: ayrı winit penceresi kendi `scale_factor`'ını taşır; egui
/// `pixels_per_point` her pencere için bağımsız uygulanır.  Render bağlamı (yüzey + renderer) bu
/// pencereye **kendi** `GpuContext`'i ile bağlıdır → "detach penceresi boş" olmaz.
struct DetachPenceresi {
    pencere: Arc<Window>,
    gpu: GpuContext,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

/// İP-11 Gün 10 canlı demo: geri-al/yinele + çakışma tespiti + yerel geçmiş.
///
/// Kalıcı (gerçek) durumdan **ayrı** bir "kum havuzu" model üzerinde çalışır; böylece geri-al/yinele
/// gösterimi gerçek oturumu (klavye tema döngüsü vb.) etkilemez.  Aynı genel motor
/// ([`biocraft_state`]) kullanılır: sonraki paketler (node/kod/ayar) bu kalıbı kendi modelleriyle
/// aynen tekrarlar.
struct DuzenlemeDemo {
    /// Üzerinde düzenleme yapılan kum-havuzu model (gerçek kalıcı durumdan ayrı).
    durum: UygulamaDurumu,
    /// Çok-adımlı geri-al/yinele motoru (MK-36).
    yigin: GeriAlYigini<UygulamaDurumu>,
    /// Zaman damgalı yerel geçmiş (anlık görüntüler).
    gecmis: YerelGecmis,
    /// Çakışma tespiti için taban sürüm izleyici (madde 18).
    izleyici: CakismaIzleyici,
    /// Şu an çözüm bekleyen çakışma (varsa → sürüm seçimi sunulur, sessiz ezme yok).
    aktif_cakisma: Option<CakismaBilgisi>,
    /// "Başka pencere/araç diske yazdı" senaryosunu taklit eden içerik (çakışma demosu).
    disk_icerik: Option<Vec<u8>>,
    /// Eklenen sekmelere benzersiz ad vermek için sayaç.
    sekme_sayac: u32,
    /// Son işlemin kısa bildirimi (panelde gösterilir).
    son_mesaj: Option<String>,
}

/// Demo kum-havuzu modelinin mantıksal depo yolu (çakışma izleme anahtarı).
const DEMO_YOL: &str = "demo.bcproj";

impl DuzenlemeDemo {
    fn yeni() -> Self {
        let durum = UygulamaDurumu::default();
        let mut izleyici = CakismaIzleyici::yeni();
        // Taban sürüm: mevcut içerik (yükleme anı) — çakışma karşılaştırmasının referansı.
        if let Ok(baytlar) = durum.serde_yaz() {
            izleyici.taban_kaydet(DEMO_YOL, SurumDamgasi::yeni(&baytlar, simdi()));
        }
        Self {
            durum,
            yigin: GeriAlYigini::yeni(),
            gecmis: YerelGecmis::yeni(),
            izleyici,
            aktif_cakisma: None,
            disk_icerik: None,
            sekme_sayac: 0,
            son_mesaj: None,
        }
    }

    /// Bir komutu kum-havuzu modele uygular (geri-al yığınına ekler).
    fn calistir(&mut self, komut: Box<dyn Komut<UygulamaDurumu>>) {
        let aciklama = komut.aciklama();
        match self.yigin.calistir(&mut self.durum, komut) {
            Ok(()) => self.son_mesaj = Some(aciklama),
            Err(e) => self.son_mesaj = Some(format!("Hata: {}", e.ne_oldu)),
        }
    }

    /// Bir sonraki temayı döngüsel seçer (Koyu→Açık→YüksekKontrast→Koyu).
    fn sonraki_tema(t: TemaSecimi) -> TemaSecimi {
        match t {
            TemaSecimi::Koyu => TemaSecimi::Acik,
            TemaSecimi::Acik => TemaSecimi::YuksekKontrast,
            TemaSecimi::YuksekKontrast => TemaSecimi::Koyu,
        }
    }

    /// "Kaydet" denemesi: yazmadan önce çakışma denetimi (sessiz ezme yok — madde 18).
    fn kaydet_dene(&mut self) {
        let Some(taban) = self.izleyici.taban(DEMO_YOL).cloned() else {
            return;
        };
        // Diskteki güncel sürüm: başka yazıcı varsa onun içeriği, yoksa taban (disk değişmemiş).
        let diskteki = match &self.disk_icerik {
            Some(b) => SurumDamgasi::yeni(b, simdi()),
            None => taban,
        };
        match self.izleyici.yazmadan_once(DEMO_YOL, &diskteki) {
            CakismaKarari::GuvenliYaz => {
                if let Ok(b) = self.durum.serde_yaz() {
                    self.izleyici
                        .taban_kaydet(DEMO_YOL, SurumDamgasi::yeni(&b, simdi()));
                    self.gecmis.anlik_al("Kayıt", &b, simdi());
                }
                self.son_mesaj = Some("Güvenle kaydedildi (çakışma yok)".to_string());
            }
            CakismaKarari::Cakisma(bilgi) => {
                self.son_mesaj = Some("ÇAKIŞMA: aynı dosya iki yerde değişti".to_string());
                self.aktif_cakisma = Some(bilgi);
            }
        }
    }
}

impl ApplicationHandler for Uygulama {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.durum.is_some() {
            return; // yalnızca bir kez kur (masaüstünde resumed tek kez tetiklenir).
        }

        // İP-11/MK-38: kalıcı durumdan pencere boyutu/maksimize + tema/dil/panel geri yüklenir.
        let kayitli_pencere = self.yonetici.durum().pencere;
        let kayitli_tema = self.yonetici.durum().tema;
        let kayitli_dil = self.yonetici.durum().dil;
        let kayitli_panel_w = self.yonetici.durum().panel.sag_panel_genislik;
        // İP-03: kabuk durumu (seçili Activity mod + Side Panel düzeni) geri yüklenir.
        let kayitli_kabuk = self.yonetici.durum().kabuk;

        let pencere = match event_loop.create_window(
            Window::default_attributes()
                .with_title("BioCraft Engine — İP-04 Render Host")
                .with_inner_size(LogicalSize::new(
                    kayitli_pencere.genislik as f64,
                    kayitli_pencere.yukseklik as f64,
                )),
        ) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                log::error!("Pencere oluşturulamadı: {e}");
                event_loop.exit();
                return;
            }
        };
        // Önceki oturum maksimize bırakılmışsa geri yükle.
        if kayitli_pencere.buyutulmus {
            pencere.set_maximized(true);
        }

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

        // Galeri geri yüklenen görünüm (tema) + dil ile başlar (MK-38: oturumlar arası kalıcı).
        let mut gallery = Gallery::new();
        gallery.tema = tema_ui(kayitli_tema);
        gallery.dil = dil_ui(kayitli_dil);

        // İP-03 Gün 12: editör + alt panel + inspector kalıcı kabuk durumundan geri yüklenir.
        let mut editor = EditorAlani::yeni();
        editor.bolmeyi_ayarla(
            BolmeYonu::secimden(kayitli_kabuk.bolme_yonu),
            kayitli_kabuk.bolme_orani,
            dil_ui(kayitli_dil),
        );
        let mut alt_panel = AltPanel::yeni();
        alt_panel.acik = kayitli_kabuk.alt_panel_acik;
        alt_panel.yukseklik = kayitli_kabuk.alt_panel_yukseklik;
        alt_panel.aktif = AltSekme::secimden(kayitli_kabuk.alt_panel_sekme);

        self.durum = Some(Sahne {
            pencere,
            gpu,
            egui_ctx,
            egui_state,
            egui_renderer,
            gallery,
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
            son_panel_genislik: kayitli_panel_w,
            kurtarma_sunulacak: self.kurtarma_karari.kurtarma_mi(),
            duzenleme: DuzenlemeDemo::yeni(),
            aktif_mod: ActivityMod::secimden(kayitli_kabuk.aktif_mod),
            yan_panel_acik: kayitli_kabuk.yan_panel_acik,
            yan_panel_genislik: kayitli_kabuk.yan_panel_genislik,
            kabuk_bildirim: None,
            editor,
            alt_panel,
            inspector_acik: kayitli_kabuk.inspector_acik,
            inspector_genislik: kayitli_kabuk.inspector_genislik,
            yogun_mod: kayitli_kabuk.yogun_mod,
            gallery_acik: false,
            kapatma_onayi: None,
            duzen_penceresi_acik: false,
            duzen_ad: String::new(),
            detach: None,
            detach_toggle_istendi: false,
            tercih: self.tercih,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(sahne) = self.durum.as_mut() else {
            return;
        };

        // İP-03: ayrılmış (detach) pencerenin olayları ayrı işlenir (kendi GPU/egui bağlamı).
        if sahne.detach.as_ref().is_some_and(|d| d.pencere.id() == id) {
            sahne.detach_olay(event);
            return;
        }

        // Olayı egui'ye ilet (girdi/işaretçi/IME).
        let yanit = sahne
            .egui_state
            .on_window_event(sahne.pencere.as_ref(), &event);

        match event {
            // İP-11: kapanışta durumu kaydet + oturumu "temiz" işaretle (kurtarma çökme-dışı çıkmasın).
            // `self.yonetici` alanına doğrudan erişilir (sahne = &mut self.durum ile ayrık alan).
            WindowEvent::CloseRequested => {
                temiz_kapat_yap(&mut self.yonetici);
                event_loop.exit();
            }
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
                    Key::Named(NamedKey::Escape) => {
                        temiz_kapat_yap(&mut self.yonetici);
                        event_loop.exit();
                    }
                    _ => {}
                }
            }
            // İP-03: kareyi çiz; menüden "Çıkış" seçildiyse temiz kapat + döngüyü kapat.
            WindowEvent::RedrawRequested => {
                let cikis = sahne.ciz(&mut self.yonetici);
                if cikis {
                    temiz_kapat_yap(&mut self.yonetici);
                    event_loop.exit();
                    return;
                }
                // İP-03: Inspector ayır/geri-tak istendiyse ayrı pencereyi oluştur/kapat.
                if sahne.detach_toggle_istendi {
                    sahne.detach_toggle_istendi = false;
                    sahne.detach_toggle(event_loop);
                }
            }
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
            // Ayrılmış pencere de canlı kalsın (kendi yeniden-çizimi).
            if let Some(d) = sahne.detach.as_ref() {
                d.pencere.request_redraw();
            }
        }
        // İP-11/MK-38: arayüz durumunu (tema/dil/pencere/panel) eşitle + otomatik kayıt.
        self.senkron_ve_kaydet();
    }
}

impl Sahne {
    /// Bir kareyi çiz: egui çalıştır → tessellate → wgpu ile sun.  Kare süresi ölçülür (MK-03).
    ///
    /// `yonetici`: özel düzen kaydet/yükle/sil için kalıcı duruma erişim (disjoint alan).
    /// Dönüş: kullanıcı menüden **Çıkış**'ı seçtiyse `true` (çağıran temiz kapatıp döngüyü kapatır).
    fn ciz(&mut self, yonetici: &mut DurumYoneticisi) -> bool {
        let kare_basi = Instant::now();

        // Süresi dolan geçici bildirimleri temizle (~4 sn göster): TDR + kabuk aksiyon bildirimi.
        if let Some((_, gosterim)) = &self.tdr_bildirim {
            if gosterim.elapsed() > Duration::from_secs(4) {
                self.tdr_bildirim = None;
                self.tdr.bildirim_gosterildi();
            }
        }
        if let Some((_, gosterim)) = &self.kabuk_bildirim {
            if gosterim.elapsed() > Duration::from_secs(4) {
                self.kabuk_bildirim = None;
            }
        }

        let fps = self.budget.fps();
        let backend = self.gpu.backend();
        // Status Bar bildirimi: önce TDR (donanım), yoksa kabuk aksiyon bildirimi.
        let bildirim = self
            .tdr_bildirim
            .as_ref()
            .or(self.kabuk_bildirim.as_ref())
            .map(|(m, _)| m.clone());
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
        let tema = self.gallery.tema;
        let tex_id = self.sahne3b_tex;
        let yogun = self.yogun_mod;
        // İP-11: kurtarma bandı + panel genişliği yakalama için yerel değişkenler (kapanıştan sonra okunur).
        let kurtarma = self.kurtarma_sunulacak;
        let yan_acik = self.yan_panel_acik;
        let yan_varsayilan = self.yan_panel_genislik;
        let inspector_docked = self.inspector_acik && self.detach.is_none();
        let inspector_varsayilan = self.inspector_genislik;
        // İP-03: Inspector için odaktaki etkin sekmenin (sahip-değer) bilgisi (borrow karışmasın).
        let secili: Option<(String, SekmeTuru, bool, bool)> = self
            .editor
            .odak_aktif_sekme()
            .map(|s| (s.baslik.clone(), s.tur, s.kaydedilmemis, s.sabit));
        // Status Bar "aktif iş" özeti: çalışan arka plan işi sayısı.
        let calisan = self.alt_panel.calisan_sayisi();
        let aktif_islem = (calisan > 0).then(|| {
            if matches!(dil, Dil::Tr) {
                format!("{calisan} iş çalışıyor")
            } else {
                format!("{calisan} job(s) running")
            }
        });

        let mut kurtarma_kapat = false;
        let mut olculen_yan_w = self.yan_panel_genislik;
        let mut olculen_inspector_w = self.inspector_genislik;
        let mut secilen_aksiyon: Option<KabukAksiyon> = None;
        // İP-03 Gün 12: closure içinden toplanan eylemler (closure sonrası uygulanır).
        let mut editor_kapatma: Option<KapatmaIstegi> = None;
        let mut detach_istendi = false;
        let mut duzen_kaydet: Option<String> = None;
        let mut duzen_yukle: Option<KabukDurumu> = None;
        let mut duzen_sil: Option<String> = None;
        // Context klonu (ucuz Arc) → kapanış self.gallery'yi ödünç alırken self.egui_ctx çakışmaz.
        let ctx = self.egui_ctx.clone();
        let full = ctx.run(raw, |c| {
            // TÜM egui yüzeyini token'dan boya; yoğun/sade moda göre boşluk ölçeği (MK-52).
            c.set_visuals(tok.egui_visuals());
            c.style_mut(|st| {
                let (x, y) = if yogun { (6.0, 3.0) } else { (10.0, 8.0) };
                st.spacing.item_spacing = egui::vec2(x, y);
            });

            // 1) Title Bar (üst) + klasik menü + komut paleti + hızlı eylemler.
            if let Some(a) = baslik_cubugu(c, dil, tema, &tok, false, false) {
                secilen_aksiyon = Some(a);
            }
            // İP-11/MK-28: çökme sonrası "kurtarılan oturum" bandı (üstte; kullanıcı kapatınca biter).
            if kurtarma && kurtarma_banneri(c, dil, &tok) {
                kurtarma_kapat = true;
            }
            // 2) Status Bar (en alt) — canlı FPS/backend/donanım + bağlantı + token + aktif iş.
            //    ÖNCE eklenir → en dipte oturur; Alt Panel sonra eklenince üstünde yer alır.
            let durum_bilgi = DurumBilgisi {
                fps,
                backend,
                bildirim: bildirim.as_deref(),
                donanim: &donanim,
                oto: &self.oto_ayar,
                cevrimici: false,   // gerçek ağ İP-15; şimdilik çevrimdışı.
                token_sayaci: None, // AI yüzeyi (İP-14) bağlanınca dolar.
                aktif_islem: aktif_islem.as_deref(),
            };
            kabuk_durum_cubugu(c, &durum_bilgi, dil, &tok);
            // 3) Alt Panel (Status Bar üstünde) — Konsol/İşler/AI/Günlük.
            if self.alt_panel.acik {
                self.alt_panel.yukseklik = alt_panel_ciz(c, &mut self.alt_panel, dil, &tok);
            }
            // 4) Activity Bar (sol) — tıklanan mod Side Panel içeriğini değiştirir.
            aktivite_cubugu(c, &mut self.aktif_mod, dil, &tok);
            // 5) Side Panel (sol) — açık ise moda göre içerik.
            if yan_acik {
                olculen_yan_w = yan_panel(c, self.aktif_mod, dil, &tok, yan_varsayilan);
            }
            // 6) Inspector (sağ) — seçili öğenin özellikleri + 3B önizleme; ayrılabilir (detach).
            if inspector_docked {
                olculen_inspector_w = inspector_ciz(
                    c,
                    tex_id,
                    en3b,
                    boy3b,
                    secili.as_ref(),
                    dil,
                    &tok,
                    inspector_varsayilan,
                    &mut detach_istendi,
                );
            }
            // 7) Merkez: bileşen demoları açıksa galeri; değilse editör/canvas alanı (sekmeli + split).
            if self.gallery_acik {
                self.gallery.show(c);
            } else {
                egui::CentralPanel::default().show(c, |ui| {
                    editor_kapatma = self.editor.ciz(ui, dil, &tok);
                });
                // E14: OS dosya sürükle-bırak — hedef vurgu + önizleme + geçersizde iptal.
                surukle_birak_isle(c, &mut self.editor, &mut self.alt_panel, dil, &tok);
            }

            // Kaydedilmemiş sekme kapatma onayı (Gün-4 onay diyaloğu) — istek varsa diyalog kur.
            if let Some(istek) = editor_kapatma {
                if self.kapatma_onayi.is_none() {
                    let (baslik, mesaj, ged) = if matches!(dil, Dil::Tr) {
                        (
                            "Kaydedilmemiş sekme",
                            "Bu sekmede kaydedilmemiş değişiklikler var. Yine de kapatılsın mı?",
                            "Kapatırsanız değişiklikler kaybolur.",
                        )
                    } else {
                        (
                            "Unsaved tab",
                            "This tab has unsaved changes. Close it anyway?",
                            "Closing will discard the changes.",
                        )
                    };
                    let dlg = ConfirmDialog::yeni(baslik, mesaj)
                        .yikici()
                        .with_geri_alinabilir(ged);
                    self.kapatma_onayi = Some((istek, dlg));
                }
            }
            // Onay diyaloğunu çiz; kararı topla (mutasyon closure sonrası uygulanır).
            let mut onay_sonuc: Option<(KapatmaIstegi, OnayKarari)> = None;
            if let Some((istek, dlg)) = &self.kapatma_onayi {
                if let Some(k) = dlg.show(c, dil, &tok) {
                    onay_sonuc = Some((*istek, k));
                }
            }
            if let Some((istek, k)) = onay_sonuc {
                self.kapatma_onayi = None;
                if matches!(k, OnayKarari::Onayla) {
                    let grup = if istek.ikincil {
                        &mut self.editor.ikincil
                    } else {
                        &mut self.editor.birincil
                    };
                    grup.kapat_kimlik(istek.kimlik);
                }
            }

            // Özel düzen yöneticisi (yüzen pencere) — kaydet/yükle/sil (kalıcı duruma yazar).
            if self.duzen_penceresi_acik {
                duzen_yonetici_penceresi(
                    c,
                    &mut self.duzen_penceresi_acik,
                    &mut self.duzen_ad,
                    yonetici,
                    dil,
                    &tok,
                    &mut duzen_kaydet,
                    &mut duzen_yukle,
                    &mut duzen_sil,
                );
            }

            // İP-11 Gün 10: geri-al/yinele demosu — yüzen pencere (kabuk düzenini bozmaz).
            duzenleme_paneli(c, &mut self.duzenleme, dil, &tok);
        });
        // Ölçülen panel genişliklerini sakla (kalıcı duruma yazılır) + bandı gizle.
        self.yan_panel_genislik = olculen_yan_w;
        self.inspector_genislik = olculen_inspector_w;
        if kurtarma_kapat {
            self.kurtarma_sunulacak = false;
        }
        if detach_istendi {
            self.detach_toggle_istendi = true;
        }
        // Özel düzen eylemlerini uygula (closure dışında; self + yonetici serbest).
        if let Some(ad) = duzen_kaydet {
            let ad = ad.trim().to_string();
            if !ad.is_empty() {
                let kabuk = self.kabuk_durumu_oku();
                yonetici.durum_guncelle(
                    |d| {
                        d.ozel_duzenler.insert(ad.clone(), kabuk);
                    },
                    Instant::now(),
                );
                self.alt_panel.konsol_yaz(if matches!(dil, Dil::Tr) {
                    format!("Düzen kaydedildi: {ad}")
                } else {
                    format!("Layout saved: {ad}")
                });
            }
        }
        if let Some(kabuk) = duzen_yukle {
            self.kabuk_uygula(&kabuk, dil);
            self.alt_panel.konsol_yaz(if matches!(dil, Dil::Tr) {
                "Düzen yüklendi (%100 sadakat)."
            } else {
                "Layout loaded (100% fidelity)."
            });
        }
        if let Some(ad) = duzen_sil {
            yonetici.durum_guncelle(
                |d| {
                    d.ozel_duzenler.remove(&ad);
                },
                Instant::now(),
            );
        }

        // İP-03: seçilen kabuk aksiyonunu uygula (tema/dil/panel/çıkış); çıkış istendiyse bildir.
        let mut cikis_istendi = false;
        if let Some(a) = secilen_aksiyon {
            cikis_istendi = self.kabuk_aksiyon_uygula(a);
        }

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
                return cikis_istendi;
            }
            Err(hata) => {
                log::debug!("Yüzey hatası ({hata:?}) → tazeleniyor, kare atlanıyor.");
                self.gpu.yuzey_tazele();
                return cikis_istendi;
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
        cikis_istendi
    }

    /// İP-03: bir kabuk aksiyonunu (menü/hızlı eylem) uygular.  Dönüş: **Çıkış** seçildiyse `true`.
    ///
    /// Tema/dil değişimi `gallery` üzerinden yapılır (kalıcı duruma `senkron_ve_kaydet` yazar);
    /// böylece hem menü hem hızlı eylem hem de ileride komut paleti aynı tek davranışa bağlanır.
    fn kabuk_aksiyon_uygula(&mut self, aksiyon: KabukAksiyon) -> bool {
        let tr = matches!(self.gallery.dil, Dil::Tr);
        match aksiyon {
            KabukAksiyon::TemaDegistir => self.gallery.tema = self.gallery.tema.sonraki(),
            KabukAksiyon::DilDegistir => {
                self.gallery.dil = match self.gallery.dil {
                    Dil::Tr => Dil::En,
                    Dil::En => Dil::Tr,
                };
            }
            KabukAksiyon::YanPanelAcKapa => self.yan_panel_acik = !self.yan_panel_acik,
            // ── İP-03 Gün 12 ──
            KabukAksiyon::YeniSekme => self.editor.yeni_sekme(self.gallery.dil),
            KabukAksiyon::Kaydet => {
                let kaydedildi = self.editor.odak_grup_mut().aktifi_kaydet();
                if kaydedildi {
                    self.alt_panel.konsol_yaz(if tr {
                        "Etkin sekme kaydedildi (kaydedilmemiş işareti kalktı)."
                    } else {
                        "Active tab saved (unsaved mark cleared)."
                    });
                }
            }
            KabukAksiyon::AltPanelAcKapa => self.alt_panel.acik = !self.alt_panel.acik,
            KabukAksiyon::InspectorAcKapa => self.inspector_acik = !self.inspector_acik,
            KabukAksiyon::EditoruBol => self.editor.bolmeyi_degistir(self.gallery.dil),
            KabukAksiyon::YogunMod => self.yogun_mod = !self.yogun_mod,
            KabukAksiyon::DuzenYonetici => self.duzen_penceresi_acik = !self.duzen_penceresi_acik,
            KabukAksiyon::DemoGalerisi => self.gallery_acik = !self.gallery_acik,
            KabukAksiyon::KomutPaleti => {
                self.kabuk_bildirim = Some((
                    if tr {
                        "Komut paleti İP-13'te gelecek (güç kullanıcı yolu)."
                    } else {
                        "Command palette arrives in İP-13 (power-user path)."
                    }
                    .to_string(),
                    Instant::now(),
                ));
            }
            KabukAksiyon::Hakkinda => {
                self.kabuk_bildirim = Some((
                    if tr {
                        "BioCraft Engine — İP-03 ana kabuk (Gün 12: sekme/split/panel/düzen)."
                    } else {
                        "BioCraft Engine — İP-03 main shell (Day 12: tabs/split/panels/layouts)."
                    }
                    .to_string(),
                    Instant::now(),
                ));
            }
            KabukAksiyon::Cikis => return true,
            // Henüz ilgili paketi olmayan aksiyonlar menüde devre dışıdır; buraya düşmezler.
            _ => {}
        }
        false
    }

    /// Canlı kabuk düzenini kalıcı [`KabukDurumu`]'na okur (otomatik kayıt + özel düzen kaydet için).
    fn kabuk_durumu_oku(&self) -> KabukDurumu {
        KabukDurumu {
            aktif_mod: self.aktif_mod.secime(),
            yan_panel_acik: self.yan_panel_acik,
            yan_panel_genislik: self.yan_panel_genislik,
            alt_panel_acik: self.alt_panel.acik,
            alt_panel_yukseklik: self.alt_panel.yukseklik,
            alt_panel_sekme: self.alt_panel.aktif.secime(),
            inspector_acik: self.inspector_acik,
            inspector_genislik: self.inspector_genislik,
            bolme_yonu: self.editor.yon.secime(),
            bolme_orani: self.editor.oran,
            yogun_mod: self.yogun_mod,
        }
    }

    /// Kayıtlı bir [`KabukDurumu`]'nu canlı kabuğa uygular (özel düzen yükle — %100 sadakat).
    fn kabuk_uygula(&mut self, k: &KabukDurumu, dil: Dil) {
        self.aktif_mod = ActivityMod::secimden(k.aktif_mod);
        self.yan_panel_acik = k.yan_panel_acik;
        self.yan_panel_genislik = k.yan_panel_genislik;
        self.alt_panel.acik = k.alt_panel_acik;
        self.alt_panel.yukseklik = k.alt_panel_yukseklik;
        self.alt_panel.aktif = AltSekme::secimden(k.alt_panel_sekme);
        self.inspector_acik = k.inspector_acik;
        self.inspector_genislik = k.inspector_genislik;
        self.yogun_mod = k.yogun_mod;
        self.editor
            .bolmeyi_ayarla(BolmeYonu::secimden(k.bolme_yonu), k.bolme_orani, dil);
    }

    /// İP-03: Inspector'ı ayrı pencereye taşır veya (zaten ayrıksa) geri takar.
    ///
    /// Ayırırken **gerçek** bir ikinci winit penceresi + kendi `GpuContext` (yüzey) + egui bağlamı
    /// kurulur → çoklu monitör + DPI ölçekleme doğal (her pencere kendi `scale_factor`'ını taşır).
    fn detach_toggle(&mut self, event_loop: &ActiveEventLoop) {
        if self.detach.is_some() {
            self.detach = None; // pencere düşürülür → kapanır; Inspector docked'a döner.
            log::info!("Inspector geri takıldı (docked).");
            return;
        }
        let pencere = match event_loop.create_window(
            Window::default_attributes()
                .with_title("BioCraft Engine — Inspector")
                .with_inner_size(LogicalSize::new(360.0, 520.0)),
        ) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                log::error!("Inspector penceresi oluşturulamadı: {e}");
                return;
            }
        };
        let gpu = match GpuContext::yeni(pencere.clone(), self.tercih) {
            Ok(g) => g,
            Err(e) => {
                log::error!("Inspector penceresi GPU bağlamı kurulamadı: {e}");
                return;
            }
        };
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
        biocraft_ui::metin_stilleri(&egui_ctx, &Tipografi::varsayilan());
        log::info!(
            "Inspector ayrı pencereye taşındı (detach) — {}",
            gpu.backend().etiket()
        );
        self.detach = Some(DetachPenceresi {
            pencere,
            gpu,
            egui_ctx,
            egui_state,
            egui_renderer,
        });
    }

    /// Ayrılmış (detach) pencerenin olaylarını işler (girdi/boyut/kapat/yeniden-çizim).
    fn detach_olay(&mut self, event: WindowEvent) {
        {
            let Some(d) = self.detach.as_mut() else {
                return;
            };
            let _ = d.egui_state.on_window_event(d.pencere.as_ref(), &event);
            if let WindowEvent::Resized(boyut) = &event {
                d.gpu.yeniden_boyutla(boyut.width, boyut.height);
                d.pencere.request_redraw();
            }
        }
        match event {
            // Pencereyi kapatmak = paneli geri takmak (uygulama kapanmaz).
            WindowEvent::CloseRequested => {
                self.detach = None;
                log::info!("Inspector penceresi kapatıldı → docked.");
            }
            WindowEvent::RedrawRequested => {
                let redock = self.detach_ciz();
                if redock {
                    self.detach = None; // "Geri Tak" düğmesi.
                    log::info!("Inspector geri takıldı (docked).");
                }
            }
            _ => {}
        }
    }

    /// Ayrılmış pencereye bir kare çizer (Inspector gövdesi).  Dönüş: "Geri Tak" istendi mi?
    fn detach_ciz(&mut self) -> bool {
        // Önce self'ten gereken değerleri al (detach ödünç almadan önce).
        let tok = self.gallery.aktif_tokenlar();
        let dil = self.gallery.dil;
        let secili: Option<(String, SekmeTuru, bool, bool)> = self
            .editor
            .odak_aktif_sekme()
            .map(|s| (s.baslik.clone(), s.tur, s.kaydedilmemis, s.sabit));
        let zemin_lin = egui::Rgba::from(tok.renk.zemin).to_array();

        let Some(d) = self.detach.as_mut() else {
            return false;
        };
        let raw = d.egui_state.take_egui_input(d.pencere.as_ref());
        let mut redock = false;
        let ctx = d.egui_ctx.clone();
        let full = ctx.run(raw, |c| {
            c.set_visuals(tok.egui_visuals());
            egui::CentralPanel::default().show(c, |ui| {
                redock = inspector_govde(ui, secili.as_ref(), dil, &tok, true);
            });
        });
        d.egui_state
            .handle_platform_output(d.pencere.as_ref(), full.platform_output);
        let jobs = d.egui_ctx.tessellate(full.shapes, full.pixels_per_point);
        let ekran = ScreenDescriptor {
            size_in_pixels: [d.gpu.config.width, d.gpu.config.height],
            pixels_per_point: full.pixels_per_point,
        };
        let cikis = match d.gpu.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => {
                d.gpu.yuzey_tazele();
                return redock;
            }
        };
        let view = cikis
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = d
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("biocraft-detach-encoder"),
            });
        for (id, delta) in &full.textures_delta.set {
            d.egui_renderer
                .update_texture(&d.gpu.device, &d.gpu.queue, *id, delta);
        }
        let kullanici = d.egui_renderer.update_buffers(
            &d.gpu.device,
            &d.gpu.queue,
            &mut encoder,
            &jobs,
            &ekran,
        );
        {
            let mut rpass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("biocraft-detach-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
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
                .forget_lifetime();
            d.egui_renderer.render(&mut rpass, &jobs, &ekran);
        }
        for id in &full.textures_delta.free {
            d.egui_renderer.free_texture(id);
        }
        d.gpu.queue.submit(
            kullanici
                .into_iter()
                .chain(std::iter::once(encoder.finish())),
        );
        cikis.present();
        redock
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

/// İP-03 Inspector (sağ, docked): seçili öğenin özellikleri + 3B önizleme + "Ayır" düğmesi.
///
/// `detach_istendi`: kullanıcı "Ayır"a basarsa `true` yazılır (ana döngü ayrı pencere açar).
/// Dönüş: panelin ölçülen genişliği (kalıcı duruma yazılır → oturumlar arası korunur).
#[allow(clippy::too_many_arguments)]
fn inspector_ciz(
    ctx: &egui::Context,
    tex_id: egui::TextureId,
    en: u32,
    boy: u32,
    secili: Option<&(String, SekmeTuru, bool, bool)>,
    dil: Dil,
    tok: &Tokenlar,
    varsayilan_genislik: f32,
    detach_istendi: &mut bool,
) -> f32 {
    let tr = matches!(dil, Dil::Tr);
    let yanit = egui::SidePanel::right("biocraft_inspector")
        .resizable(true)
        .default_width(varsayilan_genislik)
        .width_range(180.0..=600.0)
        .show(ctx, |ui| {
            *detach_istendi = inspector_govde(ui, secili, dil, tok, false);
            ui.separator();
            // 3B önizleme (docked modda): off-screen sahnenin canlı dokusu.
            ui.label(
                egui::RichText::new(if tr {
                    "Önizleme — 3B Sahne"
                } else {
                    "Preview — 3D Scene"
                })
                .small()
                .color(tok.renk.metin_soluk),
            );
            let genislik = ui.available_width().max(32.0);
            let oran = boy as f32 / en as f32;
            let sized =
                egui::load::SizedTexture::new(tex_id, egui::vec2(genislik, genislik * oran));
            ui.add(egui::Image::new(sized));
            ui.label(
                egui::RichText::new(if tr {
                    "Native wgpu top-çubuk; malzeme token'dan (ÇE-07 temeli)."
                } else {
                    "Native wgpu ball-and-stick; material from tokens (ÇE-07 base)."
                })
                .small()
                .color(tok.renk.metin_soluk),
            );
        });
    yanit.response.rect.width()
}

/// Inspector gövdesi (docked ve ayrık pencere ortak içeriği): başlık + ayır/geri-tak + özellikler.
///
/// `detached`: bu gövde ayrı pencerede mi çiziliyor (düğme "Geri Tak", aksi halde "Ayır").
/// Dönüş: ayır/geri-tak düğmesine basıldıysa `true`.
fn inspector_govde(
    ui: &mut egui::Ui,
    secili: Option<&(String, SekmeTuru, bool, bool)>,
    dil: Dil,
    tok: &Tokenlar,
    detached: bool,
) -> bool {
    let tr = matches!(dil, Dil::Tr);
    let mut toggle = false;
    ui.add_space(tok.bosluk.s);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("🔎").size(16.0).color(tok.renk.vurgu));
        ui.heading("Inspector");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let etiket = match (detached, tr) {
                (true, true) => "⧉ Geri Tak",
                (true, false) => "⧉ Dock",
                (false, true) => "⧉ Ayır",
                (false, false) => "⧉ Detach",
            };
            if ui.button(etiket).clicked() {
                toggle = true;
            }
        });
    });
    ui.separator();
    match secili {
        Some((baslik, tur, kaydedilmemis, sabit)) => {
            ui.label(
                egui::RichText::new(if tr {
                    "Seçili sekme özellikleri"
                } else {
                    "Selected tab properties"
                })
                .small()
                .color(tok.renk.metin_soluk),
            );
            ui.add_space(tok.bosluk.xs);
            inspector_satir(ui, if tr { "Ad" } else { "Name" }, baslik, tok);
            inspector_satir(ui, if tr { "Tür" } else { "Type" }, tur.ad(dil), tok);
            inspector_satir(
                ui,
                if tr { "Durum" } else { "State" },
                if *kaydedilmemis {
                    if tr {
                        "• kaydedilmemiş"
                    } else {
                        "• unsaved"
                    }
                } else if tr {
                    "kayıtlı"
                } else {
                    "saved"
                },
                tok,
            );
            inspector_satir(
                ui,
                if tr { "Sabit" } else { "Pinned" },
                if *sabit {
                    if tr {
                        "evet"
                    } else {
                        "yes"
                    }
                } else if tr {
                    "hayır"
                } else {
                    "no"
                },
                tok,
            );
            ui.add_space(tok.bosluk.xs);
            ui.label(
                egui::RichText::new(if tr {
                    "Düzenlenebilir özellikler (track/node/varyant) gerçek içerikle gelir (İP-05/06)."
                } else {
                    "Editable properties (track/node/variant) arrive with real content (İP-05/06)."
                })
                .small()
                .color(tok.renk.metin_soluk),
            );
        }
        None => {
            biocraft_ui::EmptyState::yeni(
                "🔎",
                if tr { "Seçim yok" } else { "Nothing selected" },
                if tr {
                    "Bir sekme/öğe seçildiğinde özellikleri burada görünür."
                } else {
                    "Select a tab/item to see its properties here."
                },
            )
            .show(ui, tok);
        }
    }
    if detached {
        ui.add_space(tok.bosluk.s);
        ui.label(
            egui::RichText::new(if tr {
                "ⓘ 3B önizleme yalnızca docked (yapışık) modda görünür."
            } else {
                "ⓘ The 3D preview is shown only in docked mode."
            })
            .small()
            .color(tok.renk.metin_soluk),
        );
    }
    toggle
}

/// Inspector'da tek bir "ad: değer" özellik satırı.
fn inspector_satir(ui: &mut egui::Ui, ad: &str, deger: &str, tok: &Tokenlar) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{ad}:")).color(tok.renk.metin_soluk));
        ui.label(egui::RichText::new(deger).color(tok.renk.metin).strong());
    });
}

/// E14 (sürükle-bırak): OS'tan tuvale sürüklenen dosyanın hedefini vurgular + ne olacağını önizler;
/// bırakılınca uygun tür sekme açar (yükleme Gün-34), desteklenmeyen türü reddeder (günlüğe yazar).
fn surukle_birak_isle(
    ctx: &egui::Context,
    editor: &mut EditorAlani,
    alt_panel: &mut AltPanel,
    dil: Dil,
    tok: &Tokenlar,
) {
    let tr = matches!(dil, Dil::Tr);
    // Üzerinde dosya sürükleniyorsa ekranın ortasında önizleme/vurgu göster.
    let hovered: Vec<String> = ctx.input(|i| {
        i.raw
            .hovered_files
            .iter()
            .map(|f| {
                f.path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| f.mime.clone())
            })
            .collect()
    });
    if let Some(ilk) = hovered.first() {
        let onizleme = birakma_onizleme(ilk, dil);
        let renk = if onizleme.gecerli {
            tok.renk.basari
        } else {
            tok.renk.hata
        };
        egui::Area::new(egui::Id::new("biocraft_dnd_onizleme"))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .stroke(egui::Stroke::new(2.0, renk))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(if onizleme.gecerli { "⤵" } else { "⛔" })
                                .size(28.0)
                                .color(renk),
                        );
                        ui.label(egui::RichText::new(&onizleme.metin).color(tok.renk.metin));
                        if hovered.len() > 1 {
                            ui.label(
                                egui::RichText::new(if tr {
                                    format!("(+{} dosya daha)", hovered.len() - 1)
                                } else {
                                    format!("(+{} more files)", hovered.len() - 1)
                                })
                                .small()
                                .color(tok.renk.metin_soluk),
                            );
                        }
                    });
            });
    }

    // Bırakılan dosyalar: uygun tür sekme aç; desteklenmeyeni reddet.
    let dropped: Vec<(String, String)> = ctx.input(|i| {
        i.raw
            .dropped_files
            .iter()
            .map(|f| {
                let yol = f
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| f.name.clone());
                let ad = f
                    .path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| f.name.clone());
                (yol, ad)
            })
            .collect()
    });
    for (yol, ad) in dropped {
        let onizleme = birakma_onizleme(&yol, dil);
        match onizleme.tur {
            Some(tur) => {
                editor.yeni_sekme_uret(tur, ad.clone(), false);
                alt_panel.konsol_yaz(if tr {
                    format!(
                        "Bırakıldı: {ad} → {} sekmesi (yükleme Gün-34).",
                        tur.ad(dil)
                    )
                } else {
                    format!("Dropped: {ad} → {} tab (load on Day-34).", tur.ad(dil))
                });
            }
            None => alt_panel.gunluk_yaz(if tr {
                format!("[uyarı] Reddedildi (desteklenmeyen tür): {ad}")
            } else {
                format!("[warn] Rejected (unsupported type): {ad}")
            }),
        }
    }
}

/// İP-03 özel düzen yöneticisi penceresi: adlandır → kaydet; listeden yükle/sil (kalıcı).
#[allow(clippy::too_many_arguments)]
fn duzen_yonetici_penceresi(
    ctx: &egui::Context,
    acik: &mut bool,
    ad: &mut String,
    yonetici: &DurumYoneticisi,
    dil: Dil,
    tok: &Tokenlar,
    kaydet: &mut Option<String>,
    yukle: &mut Option<KabukDurumu>,
    sil: &mut Option<String>,
) {
    let tr = matches!(dil, Dil::Tr);
    let mut pencere_acik = *acik;
    egui::Window::new(if tr {
        "Düzenleri Yönet"
    } else {
        "Manage Layouts"
    })
    .id(egui::Id::new("biocraft_duzen_yonetici"))
    .open(&mut pencere_acik)
    .default_width(320.0)
    .default_pos(egui::pos2(120.0, 120.0))
    .show(ctx, |ui| {
        ui.label(
            egui::RichText::new(if tr {
                "Kabuk düzenini adlandırıp kaydedin; sonra %100 sadakatle geri yükleyin."
            } else {
                "Name and save the shell layout; restore it later with 100% fidelity."
            })
            .small()
            .color(tok.renk.metin_soluk),
        );
        ui.add_space(tok.bosluk.xs);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(ad)
                    .hint_text(if tr { "düzen adı" } else { "layout name" })
                    .desired_width(160.0),
            );
            if ui
                .button(if tr { "💾 Kaydet" } else { "💾 Save" })
                .clicked()
            {
                *kaydet = Some(ad.clone());
            }
        });
        ui.separator();
        ui.label(if tr {
            "Kayıtlı düzenler:"
        } else {
            "Saved layouts:"
        });
        let isimler: Vec<String> = yonetici.durum().ozel_duzenler.keys().cloned().collect();
        if isimler.is_empty() {
            ui.weak(if tr { "(henüz yok)" } else { "(none yet)" });
        }
        for isim in isimler {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&isim).color(tok.renk.metin));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(if tr { "Sil" } else { "Delete" }).clicked() {
                        *sil = Some(isim.clone());
                    }
                    if ui.button(if tr { "Yükle" } else { "Load" }).clicked() {
                        if let Some(k) = yonetici.durum().ozel_duzenler.get(&isim) {
                            *yukle = Some(*k);
                        }
                    }
                });
            });
        }
    });
    *acik = pencere_acik;
}

/// İP-11/MK-28: çökme sonrası açılışta gösterilen "kurtarılan oturum" bandı.
///
/// Üst tarafta belirgin bir şeritle kullanıcıyı bilgilendirir (sessiz başarısızlık YOK — kural 3):
/// önceki oturum düzgün kapanmamış, ama açık düzen/sekmeler geri yüklenmiştir.  "Tamam"a basınca
/// `true` döner (band kapanır).
fn kurtarma_banneri(ctx: &egui::Context, dil: Dil, tok: &Tokenlar) -> bool {
    let (mesaj, dugme) = match dil {
        Dil::Tr => (
            "ⓘ Kurtarılan oturum: önceki oturum düzgün kapanmamıştı; açık düzeniniz \
             (tema, panel boyutu, sekmeler) geri yüklendi.",
            "Tamam",
        ),
        Dil::En => (
            "ⓘ Recovered session: the previous session did not close cleanly; your layout \
             (theme, panel size, tabs) was restored.",
            "OK",
        ),
    };
    let mut kapat = false;
    egui::TopBottomPanel::top("biocraft_kurtarma").show(ctx, |ui| {
        ui.add_space(tok.bosluk.xs);
        ui.horizontal(|ui| {
            ui.colored_label(tok.renk.basari, mesaj);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(dugme).clicked() {
                    kapat = true;
                }
            });
        });
        ui.add_space(tok.bosluk.xs);
    });
    kapat
}

/// İP-11 Gün 10: geri-al/yinele + çakışma tespiti + yerel geçmiş canlı demosu.
///
/// Kabul kriterlerini ekranda gösterir: (1) örnek işlem → çok-adımlı geri-al/yinele;
/// (2) her komut tek depoya dokunur (MK-37); (3) aynı dosya iki yerde değişince çakışma uyarısı;
/// (4) zaman damgalı geçmiş listesi.  Renkler token'dan (MK-52).
///
/// İP-03'ten itibaren ana kabuğun (Activity + Side Panel) sol kenarıyla çakışmaması için **yüzen
/// pencere** olarak çizilir; varsayılan kapalı (başlık çubuğu) — kullanıcı açıp inceleyebilir.
fn duzenleme_paneli(ctx: &egui::Context, demo: &mut DuzenlemeDemo, dil: Dil, tok: &Tokenlar) {
    let tr = matches!(dil, Dil::Tr);
    let baslik = if tr {
        "Geri Al / Yinele (İP-11)"
    } else {
        "Undo / Redo (İP-11)"
    };
    egui::Window::new(baslik)
        .id(egui::Id::new("biocraft_duzenleme")) // dil değişince konum/sırrı korunsun (sabit id).
        .default_open(false)
        .default_pos(egui::pos2(360.0, 88.0))
        .default_width(290.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(if tr {
                    "Kum-havuzu model — gerçek oturumu etkilemez."
                } else {
                    "Sandbox model — does not affect the real session."
                })
                .color(tok.renk.metin_soluk)
                .small(),
            );
            ui.separator();

            // (1) Örnek düzenleme işlemleri (her biri geri-alınabilir tek-depo komutu).
            ui.label(if tr { "İşlemler:" } else { "Operations:" });
            ui.horizontal_wrapped(|ui| {
                if ui.button(if tr { "🎨 Tema" } else { "🎨 Theme" }).clicked() {
                    let yeni = DuzenlemeDemo::sonraki_tema(demo.durum.tema);
                    let k = Box::new(TemaDegistir::yeni(&demo.durum, yeni));
                    demo.calistir(k);
                }
                if ui.button(if tr { "➕ Sekme" } else { "➕ Tab" }).clicked() {
                    demo.sekme_sayac += 1;
                    let ad = format!("belge-{}.fasta", demo.sekme_sayac);
                    let k = Box::new(SekmeEkle::yeni(AcikSekme {
                        yol: None,
                        baslik: ad,
                        kaydedilmemis: true,
                    }));
                    demo.calistir(k);
                }
                if ui.button(if tr { "➖ Sekme" } else { "➖ Tab" }).clicked()
                    && !demo.durum.sekmeler.is_empty()
                {
                    let son = demo.durum.sekmeler.len() - 1;
                    let k = Box::new(SekmeKapat::yeni(son));
                    demo.calistir(k);
                }
                if ui.button("↔ Panel").clicked() {
                    let yeni = (demo.durum.panel.sag_panel_genislik + 40.0).min(600.0);
                    let k = Box::new(PanelGenisligiDegistir::yeni(&demo.durum, yeni));
                    demo.calistir(k);
                }
            });

            ui.add_space(tok.bosluk.xs);
            // (1) Çok-adımlı geri-al / yinele.
            ui.horizontal(|ui| {
                let ga = demo.yigin.geri_alinabilir_mi();
                let yi = demo.yigin.yinelenebilir_mi();
                if ui
                    .add_enabled(
                        ga,
                        egui::Button::new(if tr { "↶ Geri Al" } else { "↶ Undo" }),
                    )
                    .clicked()
                {
                    let _ = demo.yigin.geri_al(&mut demo.durum);
                    demo.son_mesaj = Some(if tr { "Geri alındı" } else { "Undone" }.to_string());
                }
                if ui
                    .add_enabled(
                        yi,
                        egui::Button::new(if tr { "↷ Yinele" } else { "↷ Redo" }),
                    )
                    .clicked()
                {
                    let _ = demo.yigin.yinele(&mut demo.durum);
                    demo.son_mesaj = Some(if tr { "Yinelendi" } else { "Redone" }.to_string());
                }
            });
            if let Some(a) = demo.yigin.sonraki_geri_al() {
                ui.label(
                    egui::RichText::new(format!("↶ {a}"))
                        .small()
                        .color(tok.renk.metin_soluk),
                );
            }

            ui.add_space(tok.bosluk.xs);
            ui.separator();
            // Kum-havuzu modelin güncel durumu.
            ui.label(format!(
                "{}: {:?}  ·  {}: {}  ·  {}: {:.0}",
                if tr { "Tema" } else { "Theme" },
                demo.durum.tema,
                if tr { "Sekme" } else { "Tabs" },
                demo.durum.sekmeler.len(),
                "Panel",
                demo.durum.panel.sag_panel_genislik,
            ));

            // Komut geçmişi (geri-al yığını).
            ui.collapsing(
                if tr {
                    "Komut geçmişi"
                } else {
                    "Command history"
                },
                |ui| {
                    let liste = demo.yigin.gecmis_aciklamalari();
                    if liste.is_empty() {
                        ui.weak(if tr { "(boş)" } else { "(empty)" });
                    }
                    for (i, a) in liste.iter().enumerate() {
                        ui.label(format!("{}. {a}", i + 1));
                    }
                },
            );

            // (4) Zaman damgalı yerel geçmiş (anlık görüntüler).
            ui.collapsing(
                if tr {
                    "Yerel geçmiş (anlık görüntüler)"
                } else {
                    "Local history (snapshots)"
                },
                |ui| {
                    if demo.gecmis.bos_mu() {
                        ui.weak(if tr {
                            "(henüz yok — Kaydet veya 📸)"
                        } else {
                            "(none yet — Save or 📸)"
                        });
                    }
                    for g in demo.gecmis.listele() {
                        ui.label(format!("🕑 {} — {}", g.zaman.format("%H:%M:%S"), g.etiket));
                    }
                },
            );
            if ui
                .button(if tr {
                    "📸 Anlık görüntü al"
                } else {
                    "📸 Take snapshot"
                })
                .clicked()
            {
                if let Ok(b) = demo.durum.serde_yaz() {
                    demo.gecmis
                        .anlik_al(if tr { "Elle" } else { "Manual" }, &b, simdi());
                }
            }

            ui.add_space(tok.bosluk.xs);
            ui.separator();
            // (3) Çakışma tespiti: aynı dosya iki yerde değişince uyarı (sessiz ezme yok).
            ui.label(if tr {
                "Çakışma denetimi (madde 18):"
            } else {
                "Conflict check (item 18):"
            });
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(if tr {
                        "⚠ Başka yerde değiştir"
                    } else {
                        "⚠ Edit elsewhere"
                    })
                    .clicked()
                {
                    let mut sahte = demo.durum.clone();
                    sahte.sekmeler.push(AcikSekme {
                        yol: None,
                        baslik: "dış-değişiklik".to_string(),
                        kaydedilmemis: false,
                    });
                    if let Ok(b) = sahte.serde_yaz() {
                        demo.disk_icerik = Some(b);
                        demo.son_mesaj = Some(
                            if tr {
                                "Disk başka yerde değişti (simüle)"
                            } else {
                                "Disk changed elsewhere (simulated)"
                            }
                            .to_string(),
                        );
                    }
                }
                if ui
                    .button(if tr { "💾 Kaydet" } else { "💾 Save" })
                    .clicked()
                {
                    demo.kaydet_dene();
                }
            });

            // Çakışma varsa: sürüm seçimi sun (sessiz ezme YOK).
            if let Some(bilgi) = demo.aktif_cakisma.clone() {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.colored_label(
                        tok.renk.hata,
                        if tr {
                            "⛔ Çakışma: dosya siz düzenlerken başka yerde değişti."
                        } else {
                            "⛔ Conflict: file changed elsewhere while editing."
                        },
                    );
                    ui.label(format!(
                        "{}: {}",
                        if tr { "Dosya" } else { "File" },
                        bilgi.yol
                    ));
                    ui.label(
                        egui::RichText::new(if tr {
                            "Hangi sürüm korunsun?"
                        } else {
                            "Which version to keep?"
                        })
                        .small(),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .button(if tr { "Bizimkini yaz" } else { "Keep ours" })
                            .clicked()
                        {
                            if let Ok(b) = demo.durum.serde_yaz() {
                                demo.izleyici
                                    .taban_kaydet(DEMO_YOL, SurumDamgasi::yeni(&b, simdi()));
                                demo.gecmis.anlik_al(
                                    if tr {
                                        "Çözüm (bizim)"
                                    } else {
                                        "Resolved (ours)"
                                    },
                                    &b,
                                    simdi(),
                                );
                            }
                            demo.disk_icerik = None;
                            demo.aktif_cakisma = None;
                            demo.son_mesaj = Some(
                                if tr {
                                    "Bizim sürüm yazıldı"
                                } else {
                                    "Ours written"
                                }
                                .to_string(),
                            );
                        }
                        if ui
                            .button(if tr { "Diski koru" } else { "Keep disk" })
                            .clicked()
                        {
                            if let Some(b) = demo.disk_icerik.clone() {
                                if let Ok(d) = UygulamaDurumu::serde_oku(&b) {
                                    demo.durum = d;
                                    demo.yigin.temizle(); // model komple değişti → geçmiş geçersiz.
                                    demo.izleyici
                                        .taban_kaydet(DEMO_YOL, SurumDamgasi::yeni(&b, simdi()));
                                }
                            }
                            demo.disk_icerik = None;
                            demo.aktif_cakisma = None;
                            demo.son_mesaj = Some(
                                if tr {
                                    "Disk sürümü korundu"
                                } else {
                                    "Disk version kept"
                                }
                                .to_string(),
                            );
                        }
                        if ui.button(if tr { "İptal" } else { "Cancel" }).clicked() {
                            demo.aktif_cakisma = None;
                            demo.son_mesaj =
                                Some(if tr { "İptal edildi" } else { "Cancelled" }.to_string());
                        }
                    });
                });
            }

            if let Some(m) = demo.son_mesaj.clone() {
                ui.add_space(tok.bosluk.xs);
                ui.colored_label(tok.renk.basari, format!("ⓘ {m}"));
            }
        });
}

// ─── İP-11: durum eşleme + konum yardımcıları ────────────────────────────────

/// Kalıcı durumun saklanacağı kullanıcı veri klasörü (platforma göre).
///
/// Windows: `%APPDATA%\BioCraftEngine\state`; Linux/diğer: `$XDG_DATA_HOME` veya
/// `~/.local/share/BioCraftEngine/state`; hiçbiri yoksa geçici klasör (son çare).
fn durum_dizini() -> PathBuf {
    let taban = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))
        .unwrap_or_else(std::env::temp_dir);
    taban.join("BioCraftEngine").join("state")
}

/// Kalıcı tema seçimini UI temasına eşler (L2 nötr enum → L4 `Tema`).
fn tema_ui(t: TemaSecimi) -> Tema {
    match t {
        TemaSecimi::Koyu => Tema::Koyu,
        TemaSecimi::Acik => Tema::Acik,
        TemaSecimi::YuksekKontrast => Tema::YuksekKontrast,
    }
}

/// UI temasını kalıcı tema seçimine eşler (L4 `Tema` → L2 nötr enum).
fn tema_durum(t: Tema) -> TemaSecimi {
    match t {
        Tema::Koyu => TemaSecimi::Koyu,
        Tema::Acik => TemaSecimi::Acik,
        Tema::YuksekKontrast => TemaSecimi::YuksekKontrast,
    }
}

/// Kalıcı dil seçimini UI diline eşler.
fn dil_ui(d: DilSecimi) -> Dil {
    match d {
        DilSecimi::Tr => Dil::Tr,
        DilSecimi::En => Dil::En,
    }
}

/// UI dilini kalıcı dil seçimine eşler.
fn dil_durum(d: Dil) -> DilSecimi {
    match d {
        Dil::Tr => DilSecimi::Tr,
        Dil::En => DilSecimi::En,
    }
}

/// `assets/fonts` altından bir font dosyasını okur (yoksa None → egui gömülü fontuna düşülür).
fn font_oku(dosya: &str) -> Option<Vec<u8>> {
    std::fs::read(std::path::Path::new("assets/fonts").join(dosya)).ok()
}
