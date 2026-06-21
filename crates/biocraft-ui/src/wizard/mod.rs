//! Proje Sihirbazı — İP-02 (çok adımlı yeni-proje oluşturma akışı).
//!
//! Yeni proje oluştururken şablon / proje-bilgisi / veri-ayarı / **veri-sınıflandırma (zorunlu)** /
//! gizlilik / dağıtık-ağ seçeneklerini toplar.  Veri sınıflandırma (Normal / Hassas-PHI / Sentetik)
//! ZORUNLUDUR — gizliliğin temeli budur (MK-42).
//!
//! Mimari: saf durum + doğrulama [`steps`]'te (egui'siz, birim-testlenebilir); bu modül durumu
//! tutar + gezinmeyi (Geri/İleri) yönetir + egui görünümünü çizer.  Sihirbaz **dosya sistemine
//! dokunmaz** → "İptal" her zaman temizdir (gerçek dosya kurulumu + atomik temizlik Gün 17
//! `biocraft-data`'da).  Tüm renk token'dan, tüm metin TR/EN (MK-52, MK-53).

pub mod steps;

pub use steps::{
    ad_gecerli, etiketleri_ayristir, orcid_gecerli, siniflandirma_aciklama, siniflandirma_ad,
    BuyukVeriStratejisi, DagitikAg, DogrulamaHatasi, GizlilikGuvenlik, ProjeBilgisi, ProjeSablonu,
    ProjeTaslagi, SihirbazAdim, SihirbazBaglam, SihirbazSonucu, VeriAyarlari, VeriYerlesimi,
    DAGITIK_AG_EKLENTI_URL, SINIFLANDIRMALAR,
};

use biocraft_types::DataClassification;

use crate::i18n::{ceviri, Anahtar, Dil};
use crate::tokens::Tokenlar;

/// Dile göre iki sabit metinden birini seçen küçük yardımcı (sihirbaza özel metinler için).
fn metin(tr: bool, t: &'static str, e: &'static str) -> &'static str {
    if tr {
        t
    } else {
        e
    }
}

/// Çok adımlı proje sihirbazının tüm durumu (host bunu `Option` olarak tutar; açıkken çizer).
#[derive(Debug, Clone)]
pub struct ProjeSihirbazi {
    /// Aktif adım.
    pub adim: SihirbazAdim,
    /// Seçilen şablon.
    pub sablon: ProjeSablonu,
    /// Proje bilgisi (ad/konum/açıklama/kurum/etiket/ORCID).
    pub bilgi: ProjeBilgisi,
    /// Veri ayarları.
    pub veri: VeriAyarlari,
    /// Sınıflandırma + gizlilik + güvenlik.
    pub gizlilik: GizlilikGuvenlik,
    /// Dağıtık ağ.
    pub dagitik: DagitikAg,
}

impl ProjeSihirbazi {
    /// Bağlamdan (donanım/eklenti/konum) akıllı varsayılanlarla yeni bir sihirbaz kurar.
    ///
    /// Akıllı varsayılan (MK-05/MK-09): düşük RAM'de **akış modu açık** gelir.
    pub fn yeni(baglam: SihirbazBaglam) -> Self {
        Self {
            adim: SihirbazAdim::Sablon,
            sablon: ProjeSablonu::Genomik,
            bilgi: ProjeBilgisi {
                konum: baglam.varsayilan_konum.to_string_lossy().to_string(),
                ..Default::default()
            },
            veri: VeriAyarlari {
                yerlesim: VeriYerlesimi::Yerel,
                buyuk_veri: BuyukVeriStratejisi::Referans,
                // Akıllı varsayılan: düşük RAM → akış modu açık (out-of-core öner — MK-09).
                akis_modu: baglam.dusuk_ram,
            },
            gizlilik: GizlilikGuvenlik::varsayilan(),
            dagitik: DagitikAg {
                eklenti_kurulu: baglam.dagitik_eklenti_kurulu,
                etkin: false,
            },
        }
    }

    // ── Doğrulama ─────────────────────────────────────────────────────────────

    /// Belirli bir adımın doğrulama hatalarını döndürür (boş = geçerli).
    pub fn adim_hatalari(&self, adim: SihirbazAdim) -> Vec<DogrulamaHatasi> {
        let mut h = Vec::new();
        match adim {
            SihirbazAdim::Sablon => {} // Her zaman bir şablon seçili.
            SihirbazAdim::Bilgi => {
                let ad = self.bilgi.ad.trim();
                if ad.is_empty() {
                    h.push(DogrulamaHatasi::AdBos);
                } else if !ad_gecerli(ad) {
                    h.push(DogrulamaHatasi::AdGecersizKarakter);
                }
                if self.bilgi.konum.trim().is_empty() {
                    h.push(DogrulamaHatasi::KonumBos);
                }
                let orcid = self.bilgi.orcid_ham.trim();
                if !orcid.is_empty() && !orcid_gecerli(orcid) {
                    h.push(DogrulamaHatasi::OrcidGecersiz);
                }
            }
            SihirbazAdim::Veri => {} // Enum alanları her zaman geçerli.
            SihirbazAdim::Gizlilik => {
                // ZORUNLU: sınıflandırma seçilmeden geçilemez (MK-42).
                if self.gizlilik.siniflandirma.is_none() {
                    h.push(DogrulamaHatasi::SiniflandirmaSecilmedi);
                }
            }
            SihirbazAdim::Dagitik => {} // Eklenti yoksa "etkin" zaten seçilemez; her hâl geçerli.
            SihirbazAdim::Ozet => {
                // Özet, kendinden önceki tüm adımların hatalarını toplar.
                for &a in SihirbazAdim::TUMU.iter().take(SihirbazAdim::Ozet.indeks()) {
                    h.extend(self.adim_hatalari(a));
                }
            }
        }
        h
    }

    /// Verilen adım geçerli mi?
    pub fn adim_gecerli(&self, adim: SihirbazAdim) -> bool {
        self.adim_hatalari(adim).is_empty()
    }

    /// Aktif adımda "İleri" etkin mi (geçerliyse)?
    pub fn ileri_aktif(&self) -> bool {
        self.adim_gecerli(self.adim)
    }

    /// Tüm sihirbaz geçerli mi ("Oluştur" için)?
    pub fn tumden_gecerli(&self) -> bool {
        self.adim_hatalari(SihirbazAdim::Ozet).is_empty()
    }

    // ── Gezinme ───────────────────────────────────────────────────────────────

    /// Geçerliyse bir sonraki adıma geçer (geçersizse hiçbir şey yapmaz).
    pub fn ileri(&mut self) {
        if self.ileri_aktif() {
            if let Some(s) = self.adim.sonraki() {
                self.adim = s;
            }
        }
    }

    /// Bir önceki adıma döner (ilk adımda hiçbir şey yapmaz).
    pub fn geri(&mut self) {
        if let Some(o) = self.adim.onceki() {
            self.adim = o;
        }
    }

    /// Bir sınıflandırma seçer ve (PHI ise) güvenli kilitleri uygular (MK-42).
    pub fn siniflandirma_sec(&mut self, c: DataClassification) {
        self.gizlilik.siniflandirma = Some(c);
        self.gizlilik.phi_kilitlerini_uygula();
    }

    // ── Taslak üretimi ────────────────────────────────────────────────────────

    /// Tüm adımlar geçerliyse oluşturulacak proje taslağını üretir; aksi halde `None`.
    pub fn taslak_uret(&self) -> Option<ProjeTaslagi> {
        let siniflandirma = self.gizlilik.siniflandirma?;
        if !self.tumden_gecerli() {
            return None;
        }
        let orcid = {
            let o = self.bilgi.orcid_ham.trim();
            if o.is_empty() {
                None
            } else {
                Some(o.to_string())
            }
        };
        Some(ProjeTaslagi {
            sablon: self.sablon,
            ad: self.bilgi.ad.trim().to_string(),
            konum: std::path::PathBuf::from(self.bilgi.konum.trim()),
            aciklama: self.bilgi.aciklama.trim().to_string(),
            kurum: self.bilgi.kurum.trim().to_string(),
            etiketler: etiketleri_ayristir(&self.bilgi.etiketler_ham),
            orcid,
            veri: self.veri,
            siniflandirma,
            tamamen_yerel: self.gizlilik.tamamen_yerel,
            ai_havuzu_katki: self.gizlilik.ai_havuzu_katki,
            sifreleme: self.gizlilik.sifreleme,
            dagitik_ag_etkin: self.dagitik.eklenti_kurulu && self.dagitik.etkin,
        })
    }

    // ── egui görünümü ─────────────────────────────────────────────────────────

    /// Sihirbazı (tam ekran) çizer ve (varsa) kullanıcı sonucunu döndürür.
    ///
    /// `Olustur`/`Iptal` → host sihirbazı kapatır; `EklentiIndir` → host indirir, sihirbaz **açık
    /// kalır**.  Dönen `None` = kullanıcı henüz bitirmedi (adımlar arasında geziniyor).
    pub fn ciz(&mut self, ctx: &egui::Context, dil: Dil, tok: &Tokenlar) -> Option<SihirbazSonucu> {
        let tr = matches!(dil, Dil::Tr);
        let mut sonuc: Option<SihirbazSonucu> = None;

        // ── Üst: başlık + adım göstergesi + ilerleme çubuğu + adım noktaları ──
        egui::TopBottomPanel::top("sihirbaz_ust").show(ctx, |ui| {
            ui.add_space(tok.bosluk.s);
            ui.horizontal(|ui| {
                ui.add_space(tok.bosluk.m);
                ui.label(
                    egui::RichText::new(metin(
                        tr,
                        "＋ Yeni Proje Sihirbazı",
                        "＋ New Project Wizard",
                    ))
                    .size(20.0)
                    .strong()
                    .color(tok.renk.vurgu),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(tok.bosluk.m);
                    ui.label(
                        egui::RichText::new(format!(
                            "{} {}/{} — {}",
                            metin(tr, "Adım", "Step"),
                            self.adim.indeks() + 1,
                            SihirbazAdim::toplam(),
                            self.adim.baslik(tr),
                        ))
                        .color(tok.renk.metin_soluk),
                    );
                });
            });
            ui.add_space(tok.bosluk.xs);
            // İlerleme çubuğu (kaçıncı adımda olduğumuzu görsel olarak gösterir).
            let oran = (self.adim.indeks() + 1) as f32 / SihirbazAdim::toplam() as f32;
            ui.add(
                egui::ProgressBar::new(oran)
                    .desired_width(f32::INFINITY)
                    .fill(tok.renk.vurgu),
            );
            ui.add_space(tok.bosluk.xs);
            self.adim_noktalari_ciz(ui, tr, tok);
            ui.add_space(tok.bosluk.xs);
        });

        // ── Alt: gezinme (İptal | Geri · İleri/Oluştur) ──
        egui::TopBottomPanel::bottom("sihirbaz_alt").show(ctx, |ui| {
            ui.add_space(tok.bosluk.s);
            ui.horizontal(|ui| {
                ui.add_space(tok.bosluk.m);
                // İptal (her adımda; temiz çıkış).
                if ui.button(ceviri(dil, Anahtar::Iptal)).clicked() {
                    sonuc = Some(SihirbazSonucu::Iptal);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(tok.bosluk.m);
                    if self.adim == SihirbazAdim::Ozet {
                        // Son adım: Oluştur (yalnızca tümden geçerliyse etkin).
                        let olustur_aktif = self.tumden_gecerli();
                        let buton = egui::Button::new(
                            egui::RichText::new(ceviri(dil, Anahtar::Olustur))
                                .color(tok.renk.vurgu_ustu)
                                .strong(),
                        )
                        .fill(tok.renk.vurgu);
                        if ui.add_enabled(olustur_aktif, buton).clicked() {
                            if let Some(t) = self.taslak_uret() {
                                sonuc = Some(SihirbazSonucu::Olustur(Box::new(t)));
                            }
                        }
                    } else {
                        // Ara adım: İleri (yalnızca aktif adım geçerliyse etkin).
                        let ileri_aktif = self.ileri_aktif();
                        let buton = egui::Button::new(
                            egui::RichText::new(ceviri(dil, Anahtar::Ileri))
                                .color(if ileri_aktif {
                                    tok.renk.vurgu_ustu
                                } else {
                                    tok.renk.metin_soluk
                                })
                                .strong(),
                        )
                        .fill(if ileri_aktif {
                            tok.renk.vurgu
                        } else {
                            tok.renk.yuzey_alt
                        });
                        if ui.add_enabled(ileri_aktif, buton).clicked() {
                            self.ileri();
                        }
                    }
                    // Geri (ilk adımda pasif).
                    let geri_var = self.adim.onceki().is_some();
                    if ui
                        .add_enabled(geri_var, egui::Button::new(ceviri(dil, Anahtar::Geri)))
                        .clicked()
                    {
                        self.geri();
                    }
                });
            });
            ui.add_space(tok.bosluk.s);
        });

        // ── Merkez: aktif adımın içeriği ──
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(tok.bosluk.m);
                    match self.adim {
                        SihirbazAdim::Sablon => self.adim_sablon_ciz(ui, tr, tok),
                        SihirbazAdim::Bilgi => self.adim_bilgi_ciz(ui, tr, tok),
                        SihirbazAdim::Veri => self.adim_veri_ciz(ui, tr, tok),
                        SihirbazAdim::Gizlilik => self.adim_gizlilik_ciz(ui, tr, tok),
                        SihirbazAdim::Dagitik => {
                            if let Some(url) = self.adim_dagitik_ciz(ui, tr, tok) {
                                sonuc = Some(SihirbazSonucu::EklentiIndir(url));
                            }
                        }
                        SihirbazAdim::Ozet => self.adim_ozet_ciz(ui, tr, tok),
                    }
                    // Aktif adımın doğrulama uyarıları (anlık geri bildirim).
                    self.adim_hatalari_ciz(ui, tr, tok);
                });
        });

        // Esc → temiz iptal (güvenli varsayılan; launcher onay diyaloğuyla aynı refleks).
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            sonuc = sonuc.or(Some(SihirbazSonucu::Iptal));
        }
        sonuc
    }

    /// Üst şeritteki adım noktaları (tamamlanan ✓, aktif vurgu, gelecek soluk).
    fn adim_noktalari_ciz(&self, ui: &mut egui::Ui, tr: bool, tok: &Tokenlar) {
        ui.horizontal_wrapped(|ui| {
            ui.add_space(tok.bosluk.m);
            let aktif_i = self.adim.indeks();
            for (i, &a) in SihirbazAdim::TUMU.iter().enumerate() {
                let (renk, isaret) = if i < aktif_i {
                    (tok.renk.basari, "✓".to_string())
                } else if i == aktif_i {
                    (tok.renk.vurgu, (i + 1).to_string())
                } else {
                    (tok.renk.metin_soluk, (i + 1).to_string())
                };
                ui.label(egui::RichText::new(isaret).strong().color(renk));
                ui.label(
                    egui::RichText::new(a.baslik(tr))
                        .small()
                        .color(if i == aktif_i {
                            tok.renk.metin
                        } else {
                            tok.renk.metin_soluk
                        }),
                );
                if i + 1 < SihirbazAdim::TUMU.len() {
                    ui.label(egui::RichText::new("›").color(tok.renk.metin_soluk));
                }
            }
        });
    }

    /// Aktif adımın doğrulama hatalarını (varsa) kırmızı şeritte gösterir.
    fn adim_hatalari_ciz(&self, ui: &mut egui::Ui, tr: bool, tok: &Tokenlar) {
        let hatalar = self.adim_hatalari(self.adim);
        if hatalar.is_empty() {
            return;
        }
        ui.add_space(tok.bosluk.m);
        let cerceve = egui::Frame {
            fill: tok.renk.uyari_zemin,
            stroke: egui::Stroke::new(1.0, tok.renk.uyari),
            rounding: egui::Rounding::same(tok.yaricap),
            inner_margin: egui::Margin::same(tok.bosluk.s),
            ..Default::default()
        };
        cerceve.show(ui, |ui| {
            for h in &hatalar {
                ui.label(egui::RichText::new(format!("⚠ {}", h.mesaj(tr))).color(tok.renk.metin));
            }
        });
    }

    // ── Adım içerikleri ───────────────────────────────────────────────────────

    /// Adım 1: şablon seçimi (seçilebilir kartlar).
    fn adim_sablon_ciz(&mut self, ui: &mut egui::Ui, tr: bool, tok: &Tokenlar) {
        ui.label(
            egui::RichText::new(metin(
                tr,
                "Projenize bir başlangıç şablonu seçin. Şablon, hangi panellerin ön-kurulu \
                 geleceğini belirler.",
                "Pick a starting template. The template decides which panels come pre-installed.",
            ))
            .color(tok.renk.metin_soluk),
        );
        ui.add_space(tok.bosluk.m);
        for &s in ProjeSablonu::TUMU {
            let secili = self.sablon == s;
            let cerceve = egui::Frame {
                fill: if secili {
                    tok.renk.bilgi_zemin
                } else {
                    tok.renk.yuzey
                },
                stroke: egui::Stroke::new(
                    if secili { 2.0 } else { 1.0 },
                    if secili {
                        tok.renk.vurgu
                    } else {
                        tok.renk.kenarlik
                    },
                ),
                rounding: egui::Rounding::same(tok.yaricap),
                inner_margin: egui::Margin::same(tok.bosluk.m),
                ..Default::default()
            };
            let yanit = cerceve
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(s.ikon()).size(24.0));
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(s.ad(tr)).strong().color(tok.renk.metin));
                            ui.label(
                                egui::RichText::new(s.aciklama(tr))
                                    .small()
                                    .color(tok.renk.metin_soluk),
                            );
                        });
                    });
                })
                .response
                .interact(egui::Sense::click());
            if yanit.clicked() {
                self.sablon = s;
            }
            ui.add_space(tok.bosluk.s);
        }
    }

    /// Adım 2: proje bilgisi alanları.
    fn adim_bilgi_ciz(&mut self, ui: &mut egui::Ui, tr: bool, tok: &Tokenlar) {
        egui::Grid::new("sihirbaz_bilgi_grid")
            .num_columns(2)
            .spacing([tok.bosluk.m, tok.bosluk.s])
            .show(ui, |ui| {
                ui.label(metin(tr, "Proje adı *", "Project name *"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.bilgi.ad)
                        .hint_text(metin(
                            tr,
                            "örn. İnsan Genomu Çalışması",
                            "e.g. Human Genome Study",
                        ))
                        .desired_width(360.0),
                );
                ui.end_row();

                ui.label(metin(tr, "Konum *", "Location *"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.bilgi.konum)
                        .hint_text(metin(tr, "klasör yolu", "folder path"))
                        .desired_width(360.0),
                );
                ui.end_row();

                ui.label("");
                ui.label(
                    egui::RichText::new(metin(
                        tr,
                        "(Klasör seçici yakında — şimdilik yolu yazabilirsiniz.)",
                        "(Folder picker coming soon — type the path for now.)",
                    ))
                    .small()
                    .color(tok.renk.metin_soluk),
                );
                ui.end_row();

                ui.label(metin(tr, "Açıklama", "Description"));
                ui.add(
                    egui::TextEdit::multiline(&mut self.bilgi.aciklama)
                        .desired_rows(2)
                        .desired_width(360.0),
                );
                ui.end_row();

                ui.label(metin(tr, "Kurum", "Institution"));
                ui.add(egui::TextEdit::singleline(&mut self.bilgi.kurum).desired_width(360.0));
                ui.end_row();

                ui.label(metin(tr, "Etiketler", "Tags"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.bilgi.etiketler_ham)
                        .hint_text(metin(
                            tr,
                            "virgülle ayırın: genom, crispr",
                            "comma-separated: genome, crispr",
                        ))
                        .desired_width(360.0),
                );
                ui.end_row();

                ui.label(metin(tr, "ORCID", "ORCID"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.bilgi.orcid_ham)
                        .hint_text("0000-0002-1825-0097")
                        .desired_width(360.0),
                );
                ui.end_row();
            });
        ui.add_space(tok.bosluk.s);
        ui.label(
            egui::RichText::new(metin(tr, "* zorunlu alan", "* required field"))
                .small()
                .color(tok.renk.metin_soluk),
        );
    }

    /// Adım 3: veri ayarları.
    fn adim_veri_ciz(&mut self, ui: &mut egui::Ui, tr: bool, tok: &Tokenlar) {
        ui.label(
            egui::RichText::new(metin(tr, "Verinin yerleşimi", "Data placement"))
                .strong()
                .color(tok.renk.metin),
        );
        for y in [VeriYerlesimi::Yerel, VeriYerlesimi::Baglantili] {
            if ui.radio(self.veri.yerlesim == y, y.ad(tr)).clicked() {
                self.veri.yerlesim = y;
            }
        }
        ui.add_space(tok.bosluk.m);

        ui.label(
            egui::RichText::new(metin(tr, "Büyük dosyalar", "Large files"))
                .strong()
                .color(tok.renk.metin),
        );
        for b in [BuyukVeriStratejisi::Referans, BuyukVeriStratejisi::Gomulu] {
            if ui.radio(self.veri.buyuk_veri == b, b.ad(tr)).clicked() {
                self.veri.buyuk_veri = b;
            }
        }
        ui.label(
            egui::RichText::new(metin(
                tr,
                "Çok büyük veri (örn. 50 GB BAM) referansla tutulur; klasöre kopyalanmaz (MK-09).",
                "Very large data (e.g. 50 GB BAM) is kept by reference; not copied into the folder (MK-09).",
            ))
            .small()
            .color(tok.renk.metin_soluk),
        );
        ui.add_space(tok.bosluk.m);

        ui.checkbox(
            &mut self.veri.akis_modu,
            metin(
                tr,
                "Akış (stream) modu — büyük veriyi parça parça işle",
                "Stream mode — process large data piece by piece",
            ),
        );
        if self.veri.akis_modu {
            ui.label(
                egui::RichText::new(metin(
                    tr,
                    "Düşük RAM'de önerilir; tüm dosyayı belleğe almaz, çökme riskini azaltır.",
                    "Recommended on low RAM; doesn't load the whole file, reduces crash risk.",
                ))
                .small()
                .color(tok.renk.basari),
            );
        }
    }

    /// Adım 4: veri sınıflandırma (ZORUNLU) + gizlilik + güvenlik.
    fn adim_gizlilik_ciz(&mut self, ui: &mut egui::Ui, tr: bool, tok: &Tokenlar) {
        ui.label(
            egui::RichText::new(metin(
                tr,
                "Veri sınıflandırması (zorunlu)",
                "Data classification (required)",
            ))
            .strong()
            .color(tok.renk.metin),
        );
        ui.label(
            egui::RichText::new(metin(
                tr,
                "Bu seçim gizliliğin temelidir; seçilmeden devam edilemez.",
                "This choice is the basis of privacy; you can't continue without it.",
            ))
            .small()
            .color(tok.renk.metin_soluk),
        );
        ui.add_space(tok.bosluk.s);
        for &c in SINIFLANDIRMALAR {
            let secili = self.gizlilik.siniflandirma == Some(c);
            if ui.radio(secili, siniflandirma_ad(c, tr)).clicked() {
                self.siniflandirma_sec(c);
            }
            ui.label(
                egui::RichText::new(siniflandirma_aciklama(c, tr))
                    .small()
                    .color(tok.renk.metin_soluk),
            );
            ui.add_space(tok.bosluk.xs);
        }

        ui.add_space(tok.bosluk.m);
        ui.separator();
        ui.add_space(tok.bosluk.s);
        ui.label(
            egui::RichText::new(metin(tr, "Gizlilik & güvenlik", "Privacy & security"))
                .strong()
                .color(tok.renk.metin),
        );

        let kilit = self.gizlilik.phi_kilitli();
        if kilit {
            ui.label(
                egui::RichText::new(metin(
                    tr,
                    "🔒 PHI seçildi: tamamen-yerel + şifreleme zorunlu açık, AI havuzu zorunlu kapalı (MK-42).",
                    "🔒 PHI selected: fully-local + encryption forced on, AI pool forced off (MK-42).",
                ))
                .small()
                .color(tok.renk.uyari),
            );
        }
        ui.add_enabled_ui(!kilit, |ui| {
            ui.checkbox(
                &mut self.gizlilik.tamamen_yerel,
                metin(
                    tr,
                    "Tamamen yerel çalış (varsayılan)",
                    "Work fully local (default)",
                ),
            );
            ui.checkbox(
                &mut self.gizlilik.ai_havuzu_katki,
                metin(
                    tr,
                    "Anonimleştirilmiş sonuçları AI havuzuna katkıda bulun (varsayılan: Hayır)",
                    "Contribute anonymized results to the AI pool (default: No)",
                ),
            );
            ui.checkbox(
                &mut self.gizlilik.sifreleme,
                metin(
                    tr,
                    "Yerel şifreleme (varsayılan: açık)",
                    "Local encryption (default: on)",
                ),
            );
        });
    }

    /// Adım 5: dağıtık ağ.  Eklenti yoksa "[İndir]" yönlendirmesi döndürür.
    fn adim_dagitik_ciz(&mut self, ui: &mut egui::Ui, tr: bool, tok: &Tokenlar) -> Option<String> {
        let mut indir: Option<String> = None;
        ui.label(
            egui::RichText::new(metin(
                tr,
                "Dağıtık ağ, projeyi güvenilir düğümlerle paylaşılan hesaplamaya bağlar (opsiyonel).",
                "The distributed network connects the project to shared compute over trusted nodes (optional).",
            ))
            .color(tok.renk.metin_soluk),
        );
        ui.add_space(tok.bosluk.m);

        if self.dagitik.eklenti_kurulu {
            ui.checkbox(
                &mut self.dagitik.etkin,
                metin(
                    tr,
                    "Bu proje için dağıtık ağı etkinleştir",
                    "Enable distributed network for this project",
                ),
            );
        } else {
            // Eklenti kurulu DEĞİL → net "[İndir]" yönlendirmesi (İP-15 ile tutarlı).
            let cerceve = egui::Frame {
                fill: tok.renk.bilgi_zemin,
                stroke: egui::Stroke::new(1.0, tok.renk.bilgi),
                rounding: egui::Rounding::same(tok.yaricap),
                inner_margin: egui::Margin::same(tok.bosluk.m),
                ..Default::default()
            };
            cerceve.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🔌").size(18.0));
                    ui.label(
                        egui::RichText::new(metin(
                            tr,
                            "Dağıtık ağ için eklenti gerekli",
                            "Distributed network requires a plugin",
                        ))
                        .strong()
                        .color(tok.renk.metin),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let buton = egui::Button::new(
                            egui::RichText::new(ceviri(
                                if tr { Dil::Tr } else { Dil::En },
                                Anahtar::Indir,
                            ))
                            .color(tok.renk.vurgu_ustu)
                            .strong(),
                        )
                        .fill(tok.renk.vurgu);
                        if ui.add(buton).clicked() {
                            indir = Some(DAGITIK_AG_EKLENTI_URL.to_string());
                        }
                    });
                });
                ui.label(
                    egui::RichText::new(metin(
                        tr,
                        "Eklentiyi kurduktan sonra bu seçenek etkinleşir. Eklentisiz proje yine oluşturulur.",
                        "After installing the plugin this option becomes available. The project is still created without it.",
                    ))
                    .small()
                    .color(tok.renk.metin_soluk),
                );
            });
        }
        indir
    }

    /// Adım 6: özet + oluştur önizlemesi.
    fn adim_ozet_ciz(&mut self, ui: &mut egui::Ui, tr: bool, tok: &Tokenlar) {
        ui.label(
            egui::RichText::new(metin(
                tr,
                "Seçimlerinizi gözden geçirin. \"Oluştur\" ile proje kurulur.",
                "Review your choices. \"Create\" sets up the project.",
            ))
            .color(tok.renk.metin_soluk),
        );
        ui.add_space(tok.bosluk.m);

        let cerceve = egui::Frame {
            fill: tok.renk.yuzey,
            stroke: egui::Stroke::new(1.0, tok.renk.kenarlik),
            rounding: egui::Rounding::same(tok.yaricap),
            inner_margin: egui::Margin::same(tok.bosluk.m),
            ..Default::default()
        };
        cerceve.show(ui, |ui| {
            let satir = |ui: &mut egui::Ui, etiket: &str, deger: String| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(etiket)
                            .strong()
                            .color(tok.renk.metin_soluk),
                    );
                    ui.label(egui::RichText::new(deger).color(tok.renk.metin));
                });
            };
            satir(
                ui,
                metin(tr, "Şablon:", "Template:"),
                self.sablon.ad(tr).to_string(),
            );
            satir(
                ui,
                metin(tr, "Ad:", "Name:"),
                if self.bilgi.ad.trim().is_empty() {
                    metin(tr, "(boş)", "(empty)").to_string()
                } else {
                    self.bilgi.ad.trim().to_string()
                },
            );
            satir(
                ui,
                metin(tr, "Konum:", "Location:"),
                self.bilgi.konum.clone(),
            );
            if !self.bilgi.kurum.trim().is_empty() {
                satir(
                    ui,
                    metin(tr, "Kurum:", "Institution:"),
                    self.bilgi.kurum.trim().to_string(),
                );
            }
            let etiketler = etiketleri_ayristir(&self.bilgi.etiketler_ham);
            if !etiketler.is_empty() {
                satir(ui, metin(tr, "Etiketler:", "Tags:"), etiketler.join(", "));
            }
            satir(
                ui,
                metin(tr, "Veri:", "Data:"),
                format!(
                    "{} · {}{}",
                    self.veri.yerlesim.ad(tr),
                    self.veri.buyuk_veri.ad(tr),
                    if self.veri.akis_modu {
                        metin(tr, " · akış", " · stream")
                    } else {
                        ""
                    },
                ),
            );
            // Sınıflandırma — vurgulu (gizliliğin temeli).
            let sinif_metin = match self.gizlilik.siniflandirma {
                Some(c) => siniflandirma_ad(c, tr).to_string(),
                None => metin(tr, "⚠ SEÇİLMEDİ", "⚠ NOT SELECTED").to_string(),
            };
            let sinif_renk = match self.gizlilik.siniflandirma {
                Some(DataClassification::HasasPhi) => tok.renk.uyari,
                Some(_) => tok.renk.basari,
                None => tok.renk.hata,
            };
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(metin(tr, "Sınıflandırma:", "Classification:"))
                        .strong()
                        .color(tok.renk.metin_soluk),
                );
                ui.label(egui::RichText::new(sinif_metin).strong().color(sinif_renk));
            });
            satir(
                ui,
                metin(tr, "Gizlilik:", "Privacy:"),
                format!(
                    "{} · {} · {}",
                    if self.gizlilik.tamamen_yerel {
                        metin(tr, "yerel", "local")
                    } else {
                        metin(tr, "yerel değil", "not local")
                    },
                    if self.gizlilik.sifreleme {
                        metin(tr, "şifreli", "encrypted")
                    } else {
                        metin(tr, "şifresiz", "unencrypted")
                    },
                    if self.gizlilik.ai_havuzu_katki {
                        metin(tr, "AI havuzu: Evet", "AI pool: Yes")
                    } else {
                        metin(tr, "AI havuzu: Hayır", "AI pool: No")
                    },
                ),
            );
            satir(
                ui,
                metin(tr, "Dağıtık ağ:", "Distributed net:"),
                if !self.dagitik.eklenti_kurulu {
                    metin(tr, "eklenti yok", "no plugin").to_string()
                } else if self.dagitik.etkin {
                    metin(tr, "etkin", "enabled").to_string()
                } else {
                    metin(tr, "kapalı", "off").to_string()
                },
            );
        });
    }
}

#[cfg(test)]
mod tests;
