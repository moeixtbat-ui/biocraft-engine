//! Launcher arayüzü — İP-01 (egui adaptörü; saf mantık diğer modüllerde).
//!
//! Bölgeler: üst başlık (marka + çevrimdışı göstergesi + eylemler), sol son-projeler paneli
//! (arama + pin/kaldır + taşınmış-proje yeniden bağla + boş-durum rehberi), merkez içerik
//! (haber akışı = iskelet/çevrimdışı/hata, donanım uyarı kartı + yetenek matrisi, "Yenilikler").
//! Splash görünürse yalnızca splash çizilir.  Tüm renkler token'dan, bileşenler İP-16'dan (MK-53).

use std::path::PathBuf;
use std::time::Instant;

use biocraft_ui::components::{ConfirmDialog, EmptyState, OnayKarari, Skeleton, StatusBadge};
use biocraft_ui::{egui, Dil, Tokenlar};

use crate::hardware_check::{DonanimDegerlendirme, YetenekDurumu};
use crate::launch::{BaslatmaArgumanlari, LauncherEylem};
use crate::news::{Haber, HaberDurumu, HaberYukleyici};
use crate::recent::{proje_durumu, ProjeDurumu, SonProje, SonProjelerListesi};
use crate::splash::SplashDurumu;

/// Launcher'ın tüm görünüm + etkileşim durumu (host bunu tutar).
pub struct LauncherDurumu {
    /// Son açılan projeler (kalıcı).
    pub recent: SonProjelerListesi,
    /// Asenkron haber yükleyici.
    pub haber: HaberYukleyici,
    /// Donanım değerlendirmesi (yetenek matrisi).
    pub donanim: DonanimDegerlendirme,
    /// Açılış splash durumu (E8).
    pub splash: SplashDurumu,
    /// "Tekrar Dene" tıklandı mı (host yeni bir haber çekmesi başlatır).
    pub haber_tekrar_istendi: bool,
    /// Son projeler listesi değişti mi (host kalıcı depoya yazar).
    pub recent_kirli: bool,

    // ── Geçici arayüz durumu ──
    arama: String,
    changelog_kapali: bool,
    donanim_uyari_kapali: bool,
    /// Dış bağlantı açmadan önce onay bekleyen URL (TDA: dış bağlantıya gitmeden onay).
    dis_onay: Option<String>,
}

impl LauncherDurumu {
    /// Yeni bir launcher durumu kurar.
    pub fn yeni(
        recent: SonProjelerListesi,
        haber: HaberYukleyici,
        donanim: DonanimDegerlendirme,
        splash: SplashDurumu,
    ) -> Self {
        Self {
            recent,
            haber,
            donanim,
            splash,
            haber_tekrar_istendi: false,
            recent_kirli: false,
            arama: String::new(),
            changelog_kapali: false,
            donanim_uyari_kapali: false,
            dis_onay: None,
        }
    }

    /// Bir kareyi çizer ve (varsa) kullanıcının tetiklediği eylemi döndürür.
    ///
    /// `simdi`: splash zamanlaması için (host `Instant::now()` verir).
    pub fn ciz(
        &mut self,
        ctx: &egui::Context,
        dil: Dil,
        tok: &Tokenlar,
        simdi: Instant,
    ) -> Option<LauncherEylem> {
        // Splash görünürse yalnızca onu çiz (arayüzü bloklamaz; yükleme arka planda sürmüştür).
        if self.splash.gorunur_mu(simdi) {
            self.splash_ciz(ctx, dil, tok, simdi);
            // Splash boyunca sürekli yeniden çiz (ilerleme çubuğu akıcı + süre dolunca kapanır).
            ctx.request_repaint();
            return None;
        }

        let tr = matches!(dil, Dil::Tr);
        let mut eylem: Option<LauncherEylem> = None;

        // Dış bağlantı onay diyaloğu (her şeyin üstünde; modal).
        if let Some(url) = self.dis_onay.clone() {
            let d = ConfirmDialog::yeni(
                if tr {
                    "Dış bağlantıyı aç?"
                } else {
                    "Open external link?"
                },
                format!(
                    "{}\n\n{url}",
                    if tr {
                        "Tarayıcınızda şu adres açılacak:"
                    } else {
                        "This address will open in your browser:"
                    }
                ),
            )
            .with_onay_etiketi(if tr { "Aç" } else { "Open" });
            match d.show(ctx, dil, tok) {
                Some(OnayKarari::Onayla) => {
                    self.dis_onay = None;
                    eylem = Some(LauncherEylem::DisBaglantiAc(url));
                }
                Some(OnayKarari::Iptal) => self.dis_onay = None,
                None => {}
            }
        }

        // ── Üst başlık: marka + çevrimdışı göstergesi + ana eylemler ──
        egui::TopBottomPanel::top("launcher_baslik")
            .exact_height(56.0)
            .show(ctx, |ui| {
                ui.add_space(tok.bosluk.s);
                ui.horizontal(|ui| {
                    ui.add_space(tok.bosluk.m);
                    ui.label(
                        egui::RichText::new("🧬 BioCraft Engine")
                            .size(22.0)
                            .strong()
                            .color(tok.renk.vurgu),
                    );
                    // Çevrimdışı göstergesi (haber önbellekten geliyorsa).
                    if self.haber.durum().cevrimdisi_mi() {
                        ui.add_space(tok.bosluk.s);
                        let _ = StatusBadge::Cevrimdisi.show(ui, dil, tok);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(tok.bosluk.m);
                        if ui.button(if tr { "Yardım" } else { "Help" }).clicked() {
                            eylem = Some(LauncherEylem::Yardim);
                        }
                        if ui.button(if tr { "Ayarlar" } else { "Settings" }).clicked() {
                            eylem = Some(LauncherEylem::Ayarlar);
                        }
                        if ui
                            .button(if tr { "Proje Aç" } else { "Open Project" })
                            .clicked()
                        {
                            eylem = Some(LauncherEylem::ProjeAc);
                        }
                        let yeni = egui::Button::new(
                            egui::RichText::new(if tr {
                                "＋ Yeni Proje"
                            } else {
                                "＋ New Project"
                            })
                            .color(tok.renk.vurgu_ustu)
                            .strong(),
                        )
                        .fill(tok.renk.vurgu);
                        if ui.add(yeni).clicked() {
                            eylem = Some(LauncherEylem::YeniProje);
                        }
                    });
                });
            });

        // ── Sol panel: son projeler ──
        egui::SidePanel::left("launcher_son_projeler")
            .resizable(true)
            .default_width(360.0)
            .width_range(280.0..=560.0)
            .show(ctx, |ui| {
                if let Some(e) = self.son_projeler_ciz(ui, dil, tok) {
                    eylem = Some(e);
                }
            });

        // ── Merkez: haber + donanım + yenilikler ──
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Donanım uyarısı (referans altıysa) — kullanıcı DIŞLANMAZ, bilgilendirilir (MK-05).
                if self.donanim.referans_alti && !self.donanim_uyari_kapali {
                    self.donanim_karti_ciz(ui, dil, tok);
                }
                // "Yenilikler" (changelog) kartı.
                if !self.changelog_kapali {
                    self.yenilikler_karti_ciz(ui, dil, tok);
                }
                // Haber akışı.
                self.haber_ciz(ui, dil, tok);
            });
        });

        eylem
    }

    /// Splash ekranı (logo/DNA + slogan + ilerleme); tıklayınca erken kapanır.
    fn splash_ciz(&mut self, ctx: &egui::Context, dil: Dil, tok: &Tokenlar, simdi: Instant) {
        let tr = matches!(dil, Dil::Tr);
        let ilerleme = self.splash.ilerleme(simdi);
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(tok.renk.zemin))
            .show(ctx, |ui| {
                let yanit = ui.interact(
                    ui.max_rect(),
                    egui::Id::new("splash_tikla"),
                    egui::Sense::click(),
                );
                if yanit.clicked() {
                    self.splash.kapat();
                }
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.32);
                    ui.label(egui::RichText::new("🧬").size(72.0).color(tok.renk.vurgu));
                    ui.add_space(tok.bosluk.m);
                    ui.label(
                        egui::RichText::new("BioCraft Engine")
                            .size(30.0)
                            .strong()
                            .color(tok.renk.metin),
                    );
                    ui.add_space(tok.bosluk.xs);
                    ui.label(
                        egui::RichText::new(if tr {
                            "Yaşamı tasarlamanın motoru"
                        } else {
                            "The engine for designing life"
                        })
                        .size(15.0)
                        .color(tok.renk.metin_soluk),
                    );
                    ui.add_space(tok.bosluk.xl);
                    ui.add(
                        egui::ProgressBar::new(ilerleme)
                            .desired_width(220.0)
                            .fill(tok.renk.vurgu),
                    );
                });
            });
    }

    /// Sol son-projeler paneli (arama + liste + boş durum).
    fn son_projeler_ciz(
        &mut self,
        ui: &mut egui::Ui,
        dil: Dil,
        tok: &Tokenlar,
    ) -> Option<LauncherEylem> {
        let tr = matches!(dil, Dil::Tr);
        let mut eylem = None;

        ui.add_space(tok.bosluk.s);
        ui.label(
            egui::RichText::new(if tr {
                "Son Projeler"
            } else {
                "Recent Projects"
            })
            .size(16.0)
            .strong()
            .color(tok.renk.metin),
        );
        ui.add_space(tok.bosluk.xs);

        // Boş liste → rehber (TDA madde 5).
        if self.recent.bos_mu() {
            let bos = EmptyState::yeni(
                "📂",
                if tr {
                    "Henüz proje yok"
                } else {
                    "No projects yet"
                },
                if tr {
                    "İlk projenizi oluşturarak başlayın."
                } else {
                    "Get started by creating your first project."
                },
            )
            .with_eylem(if tr {
                "＋ Yeni Proje"
            } else {
                "＋ New Project"
            });
            if bos.show(ui, tok) {
                eylem = Some(LauncherEylem::YeniProje);
            }
            return eylem;
        }

        // Arama kutusu.
        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.add(
                egui::TextEdit::singleline(&mut self.arama)
                    .hint_text(if tr {
                        "Projelerde ara…"
                    } else {
                        "Search projects…"
                    })
                    .desired_width(f32::INFINITY),
            );
        });
        ui.add_space(tok.bosluk.xs);

        // Filtrelenmiş + sıralı listeyi sahiplenerek kopyala (ödünç-alma çakışmasını önler;
        // küçük liste → ucuz).  Satır eylemleri toplanır, döngüden sonra uygulanır.
        let projeler: Vec<SonProje> = self.recent.ara(&self.arama).into_iter().cloned().collect();

        let mut sabit_degis: Option<PathBuf> = None;
        let mut kaldir: Option<PathBuf> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if projeler.is_empty() {
                    ui.add_space(tok.bosluk.m);
                    ui.label(
                        egui::RichText::new(if tr {
                            "Aramayla eşleşen proje yok."
                        } else {
                            "No projects match your search."
                        })
                        .color(tok.renk.metin_soluk),
                    );
                }
                for p in &projeler {
                    let durum = proje_durumu(&p.yol, |q| q.exists());
                    if let Some(e) =
                        proje_satiri_ciz(ui, tok, tr, p, durum, &mut sabit_degis, &mut kaldir)
                    {
                        eylem = Some(e);
                    }
                    ui.add_space(tok.bosluk.xs);
                }
            });

        // Toplanan satır eylemlerini uygula.
        if let Some(yol) = sabit_degis {
            self.recent.sabit_degistir(&yol);
            self.recent_kirli = true;
        }
        if let Some(yol) = kaldir {
            self.recent.kaldir(&yol);
            self.recent_kirli = true;
        }
        eylem
    }

    /// Donanım uyarı kartı + yetenek matrisi (referans altıysa).
    fn donanim_karti_ciz(&mut self, ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) {
        let tr = matches!(dil, Dil::Tr);
        let cerceve = egui::Frame {
            fill: tok.renk.uyari_zemin,
            stroke: egui::Stroke::new(1.0, tok.renk.uyari),
            rounding: egui::Rounding::same(tok.yaricap),
            inner_margin: egui::Margin::same(tok.bosluk.m),
            ..Default::default()
        };
        cerceve.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("⚠").size(18.0).color(tok.renk.uyari));
                ui.label(
                    egui::RichText::new(if tr {
                        "Donanımınız önerilen tabanın altında"
                    } else {
                        "Your hardware is below the recommended baseline"
                    })
                    .strong()
                    .color(tok.renk.metin),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(if tr { "Anladım" } else { "Got it" }).clicked() {
                        self.donanim_uyari_kapali = true;
                    }
                });
            });
            ui.label(egui::RichText::new(self.donanim.ozet(tr)).color(tok.renk.metin_soluk));
            ui.add_space(tok.bosluk.xs);
            ui.label(
                egui::RichText::new(if tr {
                    "Endişelenmeyin — uygulamayı yine de kullanabilirsiniz. Ne yapabileceğiniz:"
                } else {
                    "Don't worry — you can still use the app. What you can do:"
                })
                .italics()
                .color(tok.renk.metin_soluk),
            );
            ui.add_space(tok.bosluk.xs);

            // Yetenek matrisi.
            for y in &self.donanim.matris {
                let (ikon_renk, _) = match y.durum {
                    YetenekDurumu::Tam => (tok.renk.basari, ()),
                    YetenekDurumu::Sinirli => (tok.renk.uyari, ()),
                    YetenekDurumu::Yok => (tok.renk.hata, ()),
                };
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(y.durum.ikon())
                            .strong()
                            .color(ikon_renk),
                    );
                    ui.label(egui::RichText::new(y.ad(tr)).strong().color(tok.renk.metin));
                    ui.label(
                        egui::RichText::new(format!("[{}]", y.durum.etiket(tr)))
                            .small()
                            .color(ikon_renk),
                    );
                });
                ui.label(
                    egui::RichText::new(y.aciklama(tr))
                        .small()
                        .color(tok.renk.metin_soluk),
                );
                ui.add_space(tok.bosluk.xs);
            }
        });
        ui.add_space(tok.bosluk.m);
    }

    /// "Yenilikler" (changelog) kartı — sürüm notu haberlerinden özet.
    fn yenilikler_karti_ciz(&mut self, ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) {
        let tr = matches!(dil, Dil::Tr);
        let cerceve = egui::Frame {
            fill: tok.renk.bilgi_zemin,
            stroke: egui::Stroke::new(1.0, tok.renk.bilgi),
            rounding: egui::Rounding::same(tok.yaricap),
            inner_margin: egui::Margin::same(tok.bosluk.m),
            ..Default::default()
        };
        cerceve.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(if tr { "✨ Yenilikler" } else { "✨ What's New" })
                        .strong()
                        .color(tok.renk.metin),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(if tr { "Kapat" } else { "Dismiss" }).clicked() {
                        self.changelog_kapali = true;
                    }
                });
            });
            ui.label(
                egui::RichText::new(if tr {
                    "Sürüm 0.1 — eklenti host'u (WASM sandbox + imza + çevrimdışı kurulum) tamamlandı; Faz 2 (launcher) başladı."
                } else {
                    "Version 0.1 — plugin host (WASM sandbox + signing + offline install) is complete; Phase 2 (launcher) has begun."
                })
                .color(tok.renk.metin_soluk),
            );
            if ui
                .link(if tr { "Tüm sürüm notları →" } else { "All release notes →" })
                .clicked()
            {
                // Dış bağlantı → onay diyaloğu üzerinden açılır (TDA: onaysız gitme).
                self.dis_onay = Some("https://biocraftengine.com/changelog".into());
            }
        });
        ui.add_space(tok.bosluk.m);
    }

    /// Haber akışı bölümü (iskelet / çevrimdışı / hata / liste).
    fn haber_ciz(&mut self, ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) {
        let tr = matches!(dil, Dil::Tr);

        ui.label(
            egui::RichText::new(if tr {
                "Haberler & Duyurular"
            } else {
                "News & Announcements"
            })
            .size(16.0)
            .strong()
            .color(tok.renk.metin),
        );
        ui.add_space(tok.bosluk.xs);

        match self.haber.durum() {
            HaberDurumu::Yukleniyor => {
                // İskelet (TDA madde 6): yüklenirken boş bırakma.
                Skeleton::liste(ui, tok, 3);
            }
            HaberDurumu::Hata(rapor) => {
                // Sessiz değil: ne olduğu + "tekrar dene" (madde 4).
                let cerceve = egui::Frame {
                    fill: tok.renk.hata_zemin,
                    stroke: egui::Stroke::new(1.0, tok.renk.hata),
                    rounding: egui::Rounding::same(tok.yaricap),
                    inner_margin: egui::Margin::same(tok.bosluk.m),
                    ..Default::default()
                };
                let ne_oldu = rapor.ne_oldu.clone();
                cerceve.show(ui, |ui| {
                    ui.label(egui::RichText::new(format!("⚠ {ne_oldu}")).color(tok.renk.metin));
                    ui.label(
                        egui::RichText::new(if tr {
                            "Şu an haberler yüklenemiyor."
                        } else {
                            "News can't be loaded right now."
                        })
                        .small()
                        .color(tok.renk.metin_soluk),
                    );
                    if ui
                        .button(if tr { "Tekrar Dene" } else { "Retry" })
                        .clicked()
                    {
                        self.haber_tekrar_istendi = true;
                    }
                });
            }
            HaberDurumu::Yuklendi(akis) | HaberDurumu::Cevrimdisi(akis) => {
                let cevrimdisi = self.haber.durum().cevrimdisi_mi();
                if cevrimdisi {
                    ui.label(
                        egui::RichText::new(if tr {
                            "○ Çevrimdışı — son kaydedilen haberler gösteriliyor."
                        } else {
                            "○ Offline — showing last cached news."
                        })
                        .small()
                        .color(tok.renk.metin_soluk),
                    );
                    ui.add_space(tok.bosluk.xs);
                }
                // Akışı kopyala (ödünç çakışmasını önle) → kartları çiz, bağlantı eylemi topla.
                let haberler: Vec<Haber> = akis.haberler.clone();
                let mut acilacak: Option<String> = None;
                for h in &haberler {
                    haber_karti_ciz(ui, tok, tr, h, &mut acilacak);
                    ui.add_space(tok.bosluk.xs);
                }
                if let Some(url) = acilacak {
                    // Dış bağlantı → önce onay (TDA: onaysız dış bağlantıya gitme).
                    self.dis_onay = Some(url);
                }
            }
        }
    }
}

/// Tek bir son-proje satırı (ad/yol/tarih + pin/kaldır + taşınmışsa yeniden-bağla).
fn proje_satiri_ciz(
    ui: &mut egui::Ui,
    tok: &Tokenlar,
    tr: bool,
    p: &SonProje,
    durum: ProjeDurumu,
    sabit_degis: &mut Option<PathBuf>,
    kaldir: &mut Option<PathBuf>,
) -> Option<LauncherEylem> {
    let mut eylem = None;
    let cerceve = egui::Frame {
        fill: tok.renk.yuzey,
        stroke: egui::Stroke::new(1.0, tok.renk.kenarlik),
        rounding: egui::Rounding::same(tok.yaricap),
        inner_margin: egui::Margin::same(tok.bosluk.s),
        ..Default::default()
    };
    cerceve.show(ui, |ui| {
        ui.horizontal(|ui| {
            // Pin yıldızı.
            let yildiz = if p.sabit { "★" } else { "☆" };
            if ui
                .button(egui::RichText::new(yildiz).color(if p.sabit {
                    tok.renk.vurgu
                } else {
                    tok.renk.metin_soluk
                }))
                .on_hover_text(if tr { "Sabitle/Kaldır" } else { "Pin/Unpin" })
                .clicked()
            {
                *sabit_degis = Some(p.yol.clone());
            }
            // Ad (tıkla → aç) — taşınmışsa pasif görünür.
            ui.vertical(|ui| {
                let bulundu = durum == ProjeDurumu::Mevcut;
                let ad_renk = if bulundu {
                    tok.renk.metin
                } else {
                    tok.renk.metin_soluk
                };
                let ad_yanit = ui.add_enabled(
                    bulundu,
                    egui::Button::new(egui::RichText::new(&p.ad).strong().color(ad_renk))
                        .frame(false),
                );
                if ad_yanit.clicked() {
                    eylem = Some(LauncherEylem::ProjeyiBaslat(BaslatmaArgumanlari::proje(
                        p.yol.clone(),
                    )));
                }
                ui.label(
                    egui::RichText::new(p.yol.to_string_lossy())
                        .small()
                        .color(tok.renk.metin_soluk),
                );
                if let Some(ozet) = &p.ozet {
                    ui.label(
                        egui::RichText::new(ozet)
                            .small()
                            .italics()
                            .color(tok.renk.metin_soluk),
                    );
                }
                ui.label(
                    egui::RichText::new(p.son_acilma.format("%Y-%m-%d %H:%M").to_string())
                        .small()
                        .color(tok.renk.metin_soluk),
                );
                // Taşınmış proje → uyarı rozeti + yeniden bağla (madde 19).
                if durum == ProjeDurumu::Bulunamadi {
                    ui.horizontal(|ui| {
                        let _ =
                            StatusBadge::TasinmisKaynak { ad: p.ad.clone() }.show(ui, Dil::Tr, tok);
                        if ui
                            .button(if tr { "Yeniden Bağla" } else { "Relink" })
                            .clicked()
                        {
                            eylem = Some(LauncherEylem::YenidenBagla(p.yol.clone()));
                        }
                    });
                }
            });
            // Kaldır.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if ui
                    .button("✕")
                    .on_hover_text(if tr {
                        "Listeden kaldır"
                    } else {
                        "Remove from list"
                    })
                    .clicked()
                {
                    *kaldir = Some(p.yol.clone());
                }
            });
        });
    });
    eylem
}

/// Tek bir haber kartı (tür + doğrulanmış rozeti + başlık + özet + kaynak·tarih + bağlantı).
fn haber_karti_ciz(
    ui: &mut egui::Ui,
    tok: &Tokenlar,
    tr: bool,
    h: &Haber,
    acilacak: &mut Option<String>,
) {
    let cerceve = egui::Frame {
        fill: tok.renk.yuzey,
        stroke: egui::Stroke::new(1.0, tok.renk.kenarlik),
        rounding: egui::Rounding::same(tok.yaricap),
        inner_margin: egui::Margin::same(tok.bosluk.s),
        ..Default::default()
    };
    cerceve.show(ui, |ui| {
        ui.horizontal(|ui| {
            // Tür etiketi.
            ui.label(
                egui::RichText::new(h.tur.etiket(tr))
                    .small()
                    .strong()
                    .color(tok.renk.vurgu),
            );
            // Doğrulanmış rozeti.
            if h.dogrulanmis {
                ui.label(
                    egui::RichText::new(if tr {
                        "✔ doğrulanmış"
                    } else {
                        "✔ verified"
                    })
                    .small()
                    .color(tok.renk.basari),
                )
                .on_hover_text(if tr {
                    "Küratörlü/doğrulanmış kaynak"
                } else {
                    "Curated/verified source"
                });
            }
        });
        ui.label(
            egui::RichText::new(&h.baslik)
                .strong()
                .color(tok.renk.metin),
        );
        ui.label(egui::RichText::new(&h.ozet).color(tok.renk.metin_soluk));
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{} · {}", h.kaynak, h.tarih))
                    .small()
                    .color(tok.renk.metin_soluk),
            );
            if let Some(url) = &h.baglanti {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.link(if tr { "Detay →" } else { "Read →" }).clicked() {
                        *acilacak = Some(url.clone());
                    }
                });
            }
        });
    });
}
