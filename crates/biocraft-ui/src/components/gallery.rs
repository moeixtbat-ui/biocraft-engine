//! Örnek galeri ekranı (İP-16 Kabul Kriteri: "Tüm bileşenler örnek galeride çalışır").
//!
//! Tek bir ekranda sekiz TDA bileşeninin tamamını canlı gösterir; üstte **tema**
//! (açık/koyu) ve **dil** (TR/EN) değiştirici vardır.  Bu ekran hem geliştirici
//! referansıdır hem de tema/i18n/erişilebilirlik güvencelerinin gözle doğrulandığı yerdir.

use biocraft_mem::{
    akisla_isle, dosya_butce_kontrol, hesap_plani, insan_bayt, AcmaSecenegi, AkisAyar,
    BellekBileseni, BellekOrkestratoru, ButceKarari, OnbellekTutamac, OncelikDurumu, OncelikModu,
    Rezervasyon,
};
use biocraft_types::{ErrorReport, JobStatus};

use crate::components::{
    ButceDialog, ConfirmDialog, EmptyState, ErrorDialog, EstimateDialog, HataDiyalogEylem,
    IlerlemeEylem, IsIlerleme, OnayKarari, RozetEylem, Skeleton, StatusBadge, TahminKarari, Toast,
    ToastManager,
};
use crate::i18n::{ceviri, Anahtar, Dil};
use crate::tokens::{Tema, Tokenlar};

/// Galeri demo bütçesi (256 MB) ve birim sabitleri.
const MB: u64 = 1024 * 1024;
const DEMO_BUTCE: u64 = 256 * MB;

/// Dile göre iki sabit metinden birini seçen küçük yardımcı (yalnızca galeriye özel metinler).
fn metin(dil: Dil, tr: &'static str, en: &'static str) -> &'static str {
    match dil {
        Dil::Tr => tr,
        Dil::En => en,
    }
}

/// Örnek galeri durumu.
pub struct Gallery {
    /// Aktif dil.
    pub dil: Dil,
    /// Aktif tema (Koyu / Açık / Yüksek-kontrast).
    pub tema: Tema,
    /// Örnek 2B çizim (bir kez üretilir; her karede yeniden kurulmaz — performans).
    plot: biocraft_render::plot::Plot2B,
    toasts: ToastManager,
    is: IsIlerleme,
    hata_diyalog: ErrorDialog,
    hata_rapor: Option<ErrorReport>,
    onay: Option<ConfirmDialog>,
    tahmin: Option<EstimateDialog>,
    sahte_ilerleme: f32,
    son_olay: Option<String>,
    // İP-08 demosu: bellek orkestratörü + öncelik + bütçe diyaloğu + canlı rezervasyonlar.
    ork: BellekOrkestratoru,
    oncelik: OncelikModu,
    butce_dialog: Option<ButceDialog>,
    demo_rez: Vec<Rezervasyon>,
    demo_cache: Vec<OnbellekTutamac>,
}

impl Default for Gallery {
    fn default() -> Self {
        Self {
            dil: Dil::Tr,
            tema: Tema::Koyu,
            plot: crate::plot::ornek_plot(),
            toasts: ToastManager::new(),
            is: IsIlerleme::yeni("Varyantlar taranıyor"),
            hata_diyalog: ErrorDialog::new(),
            hata_rapor: None,
            onay: None,
            tahmin: None,
            sahte_ilerleme: 0.0,
            son_olay: None,
            ork: BellekOrkestratoru::yeni(DEMO_BUTCE),
            oncelik: OncelikModu::Denge,
            butce_dialog: None,
            demo_rez: Vec::new(),
            demo_cache: Vec::new(),
        }
    }
}

impl Gallery {
    /// Yeni bir galeri durumu.
    pub fn new() -> Self {
        Self::default()
    }

    /// Aktif temanın token paketi (host'un 3B malzeme/clear rengini token'dan alması için).
    pub fn aktif_tokenlar(&self) -> Tokenlar {
        Tokenlar::temalı(self.tema)
    }

    /// Sekiz bileşen bölümünün başlıkları (test: tüm bileşenler gösteriliyor mu).
    pub fn bolum_basliklari(dil: Dil) -> [&'static str; 8] {
        [
            metin(dil, "Bildirimler (Toast)", "Notifications (Toast)"),
            metin(dil, "Hata diyaloğu", "Error dialog"),
            metin(dil, "Boş durum", "Empty state"),
            metin(dil, "Yükleme iskeleti", "Loading skeleton"),
            metin(dil, "Onay diyaloğu", "Confirmation dialog"),
            metin(dil, "Büyük işlem tahmini", "Large-operation estimate"),
            metin(dil, "İlerleme / İş", "Progress / Job"),
            metin(dil, "Durum rozetleri", "Status badges"),
        ]
    }

    /// Tüm galeriyi çizer (üst bar + bölümler + üst-üste binen toast'lar ve modaller).
    pub fn show(&mut self, ctx: &egui::Context) {
        // Tema token'larını uygula: TÜM egui yüzeyleri (panel/pencere/widget) token'dan renklenir
        // (MK-52) → yüksek-kontrast teması da her yere yansır.  Renkler bundan sonra token'dan.
        let tok = Tokenlar::temalı(self.tema);
        ctx.set_visuals(tok.egui_visuals());
        let basliklar = Self::bolum_basliklari(self.dil);

        // ── Üst bar: başlık + dil + tema ──────────────────────────────────────
        egui::TopBottomPanel::top("biocraft_galeri_ust").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(metin(
                    self.dil,
                    "BioCraft — TDA Bileşen Galerisi",
                    "BioCraft — TDA Component Gallery",
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Tema değiştirici (Koyu → Açık → Yüksek-kontrast döngüsü).
                    if ui.button(self.tema.dugme_etiketi(self.dil)).clicked() {
                        self.tema = self.tema.sonraki();
                    }
                    ui.separator();
                    // Dil değiştirici.
                    if ui.selectable_label(self.dil == Dil::En, "EN").clicked() {
                        self.dil = Dil::En;
                    }
                    if ui.selectable_label(self.dil == Dil::Tr, "TR").clicked() {
                        self.dil = Dil::Tr;
                    }
                });
            });
        });

        // ── Gövde: kaydırılabilir bölümler ────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Sahte işin ilerlemesini her karede biraz ilerlet (canlı demo).
                if !self.is.iptal_istendi() && !matches!(self.is.durum, JobStatus::Bitti) {
                    self.sahte_ilerleme = (self.sahte_ilerleme + 0.4).min(100.0);
                    if self.sahte_ilerleme >= 100.0 {
                        self.is.durumu_ayarla(JobStatus::Bitti);
                    } else {
                        self.is.durumu_ayarla(JobStatus::Calisiyor {
                            ilerleme: Some(self.sahte_ilerleme as u8),
                        });
                    }
                }

                // 1) Bildirimler / Toast
                bolum_basligi(ui, &tok, basliklar[0]);
                ui.horizontal_wrapped(|ui| {
                    if ui.button(metin(self.dil, "Başarı", "Success")).clicked() {
                        self.toasts.ekle(
                            ctx,
                            Toast::basari(metin(self.dil, "Proje kaydedildi.", "Project saved.")),
                        );
                    }
                    if ui.button(metin(self.dil, "Uyarı", "Warning")).clicked() {
                        self.toasts.ekle(
                            ctx,
                            Toast::uyari(metin(
                                self.dil,
                                "Disk alanı azalıyor.",
                                "Low disk space.",
                            )),
                        );
                    }
                    if ui.button(metin(self.dil, "Hata", "Error")).clicked() {
                        self.toasts.ekle(
                            ctx,
                            Toast::hata(metin(self.dil, "Bağlantı koptu.", "Connection lost.")),
                        );
                    }
                    if ui.button(metin(self.dil, "Bilgi", "Info")).clicked() {
                        self.toasts.ekle(
                            ctx,
                            Toast::bilgi(metin(
                                self.dil,
                                "3 güncelleme var.",
                                "3 updates available.",
                            ))
                            .with_eylem(metin(self.dil, "Göster", "Show")),
                        );
                    }
                });
                ui.add_space(tok.bosluk.l);

                // 2) Hata diyaloğu
                bolum_basligi(ui, &tok, basliklar[1]);
                if ui
                    .button(metin(
                        self.dil,
                        "Hata diyaloğunu göster",
                        "Show error dialog",
                    ))
                    .clicked()
                {
                    self.hata_rapor = Some(
                        ErrorReport::new(
                            metin(self.dil, "Proje açılamadı.", "Could not open the project."),
                            metin(
                                self.dil,
                                "Proje dosyası başka bir programca kilitlenmiş.",
                                "The project file is locked by another program.",
                            ),
                            metin(
                                self.dil,
                                "Dosyayı kullanan programı kapatıp tekrar deneyin.",
                                "Close the program using the file and retry.",
                            ),
                        )
                        .with_eylem(ceviri(self.dil, Anahtar::TekrarDene))
                        .with_teknik_detay("OS error 32: ERROR_SHARING_VIOLATION"),
                    );
                }
                ui.add_space(tok.bosluk.l);

                // 3) Boş durum
                bolum_basligi(ui, &tok, basliklar[2]);
                let bos = EmptyState::yeni(
                    "📂",
                    metin(self.dil, "Henüz proje yok", "No projects yet"),
                    metin(
                        self.dil,
                        "Başlamak için yeni bir proje oluşturun.",
                        "Create a new project to get started.",
                    ),
                )
                .with_eylem(metin(self.dil, "Yeni Proje", "New Project"));
                if bos.show(ui, &tok) {
                    self.son_olay = Some(
                        metin(self.dil, "Yeni Proje tıklandı", "New Project clicked").to_string(),
                    );
                }
                ui.add_space(tok.bosluk.l);

                // 4) Yükleme iskeleti
                bolum_basligi(ui, &tok, basliklar[3]);
                Skeleton::paragraf(ui, &tok, 3);
                ui.add_space(tok.bosluk.s);
                Skeleton::liste(ui, &tok, 2);
                ui.add_space(tok.bosluk.l);

                // 5) Onay diyaloğu
                bolum_basligi(ui, &tok, basliklar[4]);
                if ui
                    .button(metin(self.dil, "Projeyi sil…", "Delete project…"))
                    .clicked()
                {
                    self.onay = Some(
                        ConfirmDialog::yeni(
                            metin(self.dil, "Projeyi sil?", "Delete project?"),
                            metin(
                                self.dil,
                                "Bu proje ve tüm verileri kaldırılacak.",
                                "This project and all its data will be removed.",
                            ),
                        )
                        .yikici()
                        .with_geri_alinabilir(metin(
                            self.dil,
                            "30 gün boyunca çöp kutusundan geri alınabilir.",
                            "Recoverable from trash for 30 days.",
                        )),
                    );
                }
                ui.add_space(tok.bosluk.l);

                // 6) Büyük işlem tahmini
                bolum_basligi(ui, &tok, basliklar[5]);
                if ui
                    .button(metin(
                        self.dil,
                        "Büyük dosyayı indeksle…",
                        "Index large file…",
                    ))
                    .clicked()
                {
                    self.tahmin = Some(EstimateDialog::yeni(
                        metin(
                            self.dil,
                            "12 GB BAM dosyası indekslenecek.",
                            "A 12 GB BAM file will be indexed.",
                        ),
                        300.0,
                    ));
                }
                ui.add_space(tok.bosluk.l);

                // 7) İlerleme / İş
                bolum_basligi(ui, &tok, basliklar[6]);
                if self.is.show(ui, self.dil, &tok) == Some(IlerlemeEylem::Iptal) {
                    self.son_olay =
                        Some(metin(self.dil, "İş iptal edildi", "Job cancelled").to_string());
                }
                if ui
                    .button(metin(self.dil, "İşi yeniden başlat", "Restart job"))
                    .clicked()
                {
                    self.is = IsIlerleme::yeni(metin(
                        self.dil,
                        "Varyantlar taranıyor",
                        "Scanning variants",
                    ));
                    self.sahte_ilerleme = 0.0;
                }
                ui.add_space(tok.bosluk.l);

                // 8) Durum rozetleri
                bolum_basligi(ui, &tok, basliklar[7]);
                ui.horizontal_wrapped(|ui| {
                    let _ = StatusBadge::Cevrimici.show(ui, self.dil, &tok);
                    let _ = StatusBadge::Cevrimdisi.show(ui, self.dil, &tok);
                    let _ = StatusBadge::KaynakYetersiz.show(ui, self.dil, &tok);
                    let _ = StatusBadge::Sogutuluyor.show(ui, self.dil, &tok);
                    let eklenti_yok = StatusBadge::EklentiYok {
                        ad: metin(self.dil, "Dağıtık Ağ", "Distributed Net").to_string(),
                    };
                    if eklenti_yok.show(ui, self.dil, &tok) == Some(RozetEylem::Indir) {
                        self.son_olay = Some(
                            metin(self.dil, "Eklenti indiriliyor…", "Downloading plugin…")
                                .to_string(),
                        );
                    }
                    let tasinmis = StatusBadge::TasinmisKaynak {
                        ad: "genome.bam".to_string(),
                    };
                    if tasinmis.show(ui, self.dil, &tok) == Some(RozetEylem::YenidenBagla) {
                        self.son_olay = Some(
                            metin(
                                self.dil,
                                "Kaynak yeniden bağlanıyor…",
                                "Reconnecting resource…",
                            )
                            .to_string(),
                        );
                    }
                });
                ui.add_space(tok.bosluk.l);

                // 9) 2B Çizim (İP-04 render demosu: coverage çizgi + varyant scatter; renk token'dan).
                ui.label(
                    egui::RichText::new(metin(
                        self.dil,
                        "2B Çizim — coverage (çizgi) + varyant (scatter)",
                        "2D Plot — coverage (line) + variants (scatter)",
                    ))
                    .size(16.0)
                    .strong()
                    .color(tok.renk.vurgu),
                );
                ui.add_space(tok.bosluk.xs);
                crate::plot::PlotWidget::yeni(&self.plot)
                    .yukseklik(170.0)
                    .goster(ui, &tok);
                ui.add_space(tok.bosluk.l);

                // 10) Bellek bütçesi & öncelik (İP-08 — canlı orkestratör demosu).
                self.bellek_bolumu(ui, ctx, &tok);
                ui.add_space(tok.bosluk.l);

                // Son işlem geri bildirimi (TDA madde 15).
                if let Some(olay) = &self.son_olay {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!(
                            "{}: {olay}",
                            metin(self.dil, "Son olay", "Last event")
                        ))
                        .italics()
                        .color(tok.renk.metin_soluk),
                    );
                }
            });
        });

        // ── Modaller (üst düzey pencereler; ctx üzerine çizilir) ───────────────
        let mut hata_eylem = None;
        if let Some(rapor) = &self.hata_rapor {
            hata_eylem = self.hata_diyalog.show(ctx, self.dil, &tok, rapor);
        }
        if let Some(e) = hata_eylem {
            match e {
                HataDiyalogEylem::Kapat => self.hata_rapor = None,
                HataDiyalogEylem::EylemTiklandi => {
                    self.toasts.ekle(
                        ctx,
                        Toast::bilgi(metin(self.dil, "Tekrar deneniyor…", "Retrying…")),
                    );
                    self.hata_rapor = None;
                }
                HataDiyalogEylem::KimlikKopyalandi => {
                    self.toasts.ekle(
                        ctx,
                        Toast::basari(metin(self.dil, "Kimlik kopyalandı.", "ID copied.")),
                    );
                }
            }
        }

        let mut onay_karar = None;
        if let Some(d) = &self.onay {
            onay_karar = d.show(ctx, self.dil, &tok);
        }
        if let Some(k) = onay_karar {
            self.onay = None;
            self.son_olay = Some(match k {
                OnayKarari::Onayla => {
                    metin(self.dil, "Proje silindi", "Project deleted").to_string()
                }
                OnayKarari::Iptal => {
                    metin(self.dil, "Silme iptal edildi", "Deletion cancelled").to_string()
                }
            });
        }

        let mut tahmin_karar = None;
        if let Some(d) = &self.tahmin {
            tahmin_karar = d.show(ctx, self.dil, &tok);
        }
        if let Some(k) = tahmin_karar {
            self.tahmin = None;
            self.son_olay = Some(match k {
                TahminKarari::Devam => {
                    metin(self.dil, "İndeksleme başladı", "Indexing started").to_string()
                }
                TahminKarari::Iptal => {
                    metin(self.dil, "İndeksleme iptal", "Indexing cancelled").to_string()
                }
            });
        }

        // Bellek bütçesi diyaloğu (İP-08).
        let mut butce_karar = None;
        if let Some(d) = &self.butce_dialog {
            butce_karar = d.show(ctx, self.dil, &tok);
        }
        if let Some(secenek) = butce_karar {
            self.butce_dialog = None;
            self.son_olay = Some(
                match secenek {
                    AcmaSecenegi::AkisModu => {
                        metin(self.dil, "Akış modunda açılıyor", "Opening in stream mode")
                    }
                    AcmaSecenegi::CloudBurst => {
                        metin(self.dil, "Bulut (yer tutucu)", "Cloud (placeholder)")
                    }
                    AcmaSecenegi::Iptal => metin(self.dil, "Açma iptal edildi", "Open cancelled"),
                }
                .to_string(),
            );
        }

        // ── Üst-üste binen bildirimler (sağ-üst) ──────────────────────────────
        let _ = self.toasts.show(ctx, self.dil, &tok);
    }

    /// İP-08 demosu: bellek göstergesi + rezervasyon/önbellek/baskı butonları +
    /// bütçe diyaloğu tetikleyici + out-of-core akış + öncelik modu (canlı orkestratör).
    fn bellek_bolumu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, tok: &Tokenlar) {
        let dil = self.dil;
        bolum_basligi(
            ui,
            tok,
            metin(
                dil,
                "Bellek bütçesi & öncelik (İP-08)",
                "Memory budget & priority (İP-08)",
            ),
        );

        // Bellek göstergesi (durum çubuğu adayı): rezerve / toplam + doluluk çubuğu.
        let durum = self.ork.durum();
        ui.label(
            egui::RichText::new(format!(
                "{}: {} / {}  ·  {}: {}",
                metin(dil, "Rezerve", "Reserved"),
                insan_bayt(durum.rezerve),
                insan_bayt(durum.toplam_butce),
                metin(dil, "boş", "free"),
                insan_bayt(durum.bos),
            ))
            .color(tok.renk.metin),
        );
        ui.add(
            egui::ProgressBar::new(durum.doluluk())
                .desired_width(360.0)
                .show_percentage(),
        );
        ui.add_space(tok.bosluk.s);

        // Rezervasyon / önbellek / bellek baskısı (MK-21/MK-22 canlı).
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(metin(dil, "+32 MB rezerve et", "+32 MB reserve"))
                .clicked()
            {
                match self.ork.rezerve_et(BellekBileseni::Subprocess, 32 * MB) {
                    Ok(r) => self.demo_rez.push(r),
                    Err(rapor) => {
                        // MK-22: bütçe aşımı → ÇÖKME YOK; kullanıcı bilgilendirilir.
                        self.toasts.ekle(ctx, Toast::hata(rapor.ne_oldu.clone()));
                        self.son_olay = Some(rapor.neden);
                    }
                }
            }
            if ui
                .button(metin(dil, "+32 MB önbellek", "+32 MB cache"))
                .clicked()
            {
                match self.ork.onbellek_ekle(BellekBileseni::Render, 32 * MB) {
                    Ok(c) => self.demo_cache.push(c),
                    Err(rapor) => {
                        self.toasts.ekle(ctx, Toast::hata(rapor.ne_oldu));
                    }
                }
            }
            if ui
                .button(metin(
                    dil,
                    "Bellek baskısı (temizle)",
                    "Memory pressure (clear)",
                ))
                .clicked()
            {
                let ozet = self.ork.bellek_baskisi();
                self.demo_cache.retain(|c| c.canli());
                self.toasts.ekle(
                    ctx,
                    Toast::bilgi(format!(
                        "{}: {} ({} {})",
                        metin(dil, "Önbellek boşaltıldı", "Cache evicted"),
                        insan_bayt(ozet.bosaltilan_bayt),
                        ozet.bosaltilan_adet,
                        metin(dil, "öğe", "items"),
                    )),
                );
            }
            if ui.button(metin(dil, "Sıfırla", "Reset")).clicked() {
                self.demo_rez.clear();
                self.demo_cache.clear();
            }
        });
        ui.add_space(tok.bosluk.s);

        // Büyük dosya → bütçe diyaloğu (stream/iptal) + out-of-core akış demosu.
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(metin(
                    dil,
                    "4 TB dosya aç (bütçe kontrolü)",
                    "Open 4 TB file (budget check)",
                ))
                .clicked()
            {
                let dort_tb = 4 * 1024 * 1024 * MB; // 4 TB
                if let ButceKarari::AkisOnerilir(teklif) =
                    dosya_butce_kontrol(dort_tb, 3.0, &self.ork)
                {
                    self.butce_dialog = Some(ButceDialog::yeni(teklif));
                }
            }
            if ui
                .button(metin(dil, "8 MB veriyi akışla işle", "Stream-process 8 MB"))
                .clicked()
            {
                // MK-09: 8 MB veri 256 KB pencerelerle işlenir; tepe rezervasyon = bir pencere.
                let veri = vec![0u8; 8 * MB as usize];
                let _ = match akisla_isle(
                    &veri[..],
                    AkisAyar::pencere(256 * 1024),
                    &self.ork,
                    BellekBileseni::VeriTabani,
                    |_parca| {},
                ) {
                    Ok(ozet) => self.toasts.ekle(
                        ctx,
                        Toast::basari(format!(
                            "{}: {} {} · {} {}",
                            metin(dil, "Akış bitti", "Stream done"),
                            ozet.parca_sayisi,
                            metin(dil, "parça", "chunks"),
                            metin(dil, "tepe", "peak"),
                            insan_bayt(ozet.tepe_rezervasyon_bayt),
                        )),
                    ),
                    Err(rapor) => self.toasts.ekle(ctx, Toast::hata(rapor.ne_oldu)),
                };
            }
        });
        ui.add_space(tok.bosluk.s);

        // İşleme önceliği — mod değişince worker sayısı değişir (Zero-Impact mantığı mem'de test edilir).
        let oncelik_ad = match (dil, self.oncelik) {
            (Dil::Tr, OncelikModu::ArayuzOncelikli) => "Arayüz öncelikli",
            (Dil::En, OncelikModu::ArayuzOncelikli) => "UI priority",
            (Dil::Tr, OncelikModu::Denge) => "Denge",
            (Dil::En, OncelikModu::Denge) => "Balanced",
            (Dil::Tr, OncelikModu::MaksimumHesap) => "Maksimum hesap",
            (Dil::En, OncelikModu::MaksimumHesap) => "Max compute",
        };
        let cekirdek = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let plan = hesap_plani(
            OncelikDurumu {
                modu: self.oncelik,
                kullanici_aktif: false,
            },
            cekirdek,
        );
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{}: {}  ·  {} {} / {} {}",
                    metin(dil, "İşleme önceliği", "Processing priority"),
                    oncelik_ad,
                    plan.worker_sayisi,
                    metin(dil, "worker", "workers"),
                    cekirdek,
                    metin(dil, "çekirdek", "cores"),
                ))
                .color(tok.renk.metin),
            );
            if ui
                .button(metin(dil, "Modu değiştir", "Change mode"))
                .clicked()
            {
                self.oncelik = self.oncelik.dongu();
            }
        });
    }
}

/// Bir bölüm başlığı + ince ayraç çizer.
fn bolum_basligi(ui: &mut egui::Ui, tok: &Tokenlar, baslik: &str) {
    ui.label(
        egui::RichText::new(baslik)
            .size(16.0)
            .strong()
            .color(tok.renk.vurgu),
    );
    ui.add_space(tok.bosluk.xs);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn galeri_sekiz_bolum_gosterir() {
        assert_eq!(Gallery::bolum_basliklari(Dil::Tr).len(), 8);
        assert_eq!(Gallery::bolum_basliklari(Dil::En).len(), 8);
        // TR ve EN başlıkları farklı (çeviri yapılmış).
        assert_ne!(
            Gallery::bolum_basliklari(Dil::Tr)[0],
            Gallery::bolum_basliklari(Dil::En)[0]
        );
    }

    #[test]
    fn galeri_tum_tema_ve_dillerde_headless_cizilir() {
        // Tüm bileşenleri bir karede çiz; hiçbir tema/dil kombinasyonunda panik olmamalı
        // (2B plot bölümü + üç tema dahil).
        for tema in [Tema::Koyu, Tema::Acik, Tema::YuksekKontrast] {
            for dil in [Dil::Tr, Dil::En] {
                let ctx = egui::Context::default();
                let mut g = Gallery::new();
                g.tema = tema;
                g.dil = dil;
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    g.show(ctx);
                });
            }
        }
    }
}
