//! Entegre/bağlamsal yardım (İP-17).
//!
//! Tek pencerede: **arama** + **kavram açıklamaları** (BAM/VCF nedir) + **kısayol kartı** +
//! **çevrimdışı temel doküman** bağlantıları.  Dış doküman bağlantısı **tıklanınca host onaylar**
//! (dış bağlantı = onay; İP-15 deseni); MVP'de çevrimdışı temel doküman + URL host'a iletilir.
//!
//! Saf içerik + egui adaptörü; tüm metin TR/EN (MK-53).  Renkler token'dan (MK-52).

use crate::i18n::Dil;
use crate::tokens::Tokenlar;

use super::metin;
use super::tutorial::Kavram;

/// Kısayol kartının tek satırı (eylem adı + tuş).  Tuş gösterimi `menu_bar` ile tutarlıdır.
struct KisayolSatiri {
    tr: &'static str,
    en: &'static str,
    tus: &'static str,
}

/// Temel kısayol kartı (en sık kullanılanlar; tam liste "Klavye Kısayolları" penceresinde — İP-13).
const KISAYOLLAR: &[KisayolSatiri] = &[
    KisayolSatiri {
        tr: "Komut paleti",
        en: "Command palette",
        tus: "Ctrl+Shift+P",
    },
    KisayolSatiri {
        tr: "Yeni proje",
        en: "New project",
        tus: "Ctrl+N",
    },
    KisayolSatiri {
        tr: "Proje aç",
        en: "Open project",
        tus: "Ctrl+O",
    },
    KisayolSatiri {
        tr: "Yeni sekme",
        en: "New tab",
        tus: "Ctrl+T",
    },
    KisayolSatiri {
        tr: "Kaydet",
        en: "Save",
        tus: "Ctrl+S",
    },
    KisayolSatiri {
        tr: "Alt paneli aç/kapa",
        en: "Toggle bottom panel",
        tus: "Ctrl+J",
    },
    KisayolSatiri {
        tr: "Editörü böl",
        en: "Split editor",
        tus: "Ctrl+\\",
    },
    KisayolSatiri {
        tr: "Ayarlar",
        en: "Settings",
        tus: "Ctrl+,",
    },
    KisayolSatiri {
        tr: "Geri al / Yinele",
        en: "Undo / Redo",
        tus: "Ctrl+Z / Ctrl+Y",
    },
];

/// Çevrimdışı temel doküman bölümleri (başlık + kısa içerik); ağ gerekmez.
struct DokumanBolum {
    baslik_tr: &'static str,
    baslik_en: &'static str,
    icerik_tr: &'static str,
    icerik_en: &'static str,
}

const DOKUMANLAR: &[DokumanBolum] = &[
    DokumanBolum {
        baslik_tr: "İlk adımlar",
        baslik_en: "First steps",
        icerik_tr: "Açılışta \"Demo Projeyi Aç\" ile örnek veriyle başlayın; sonra kendi verinizi \
                    \"Yeni Proje\" sihirbazıyla ekleyin.",
        icerik_en: "On launch, use \"Open Demo Project\" to start with sample data; then add your \
                    own data with the \"New Project\" wizard.",
    },
    DokumanBolum {
        baslik_tr: "Gizlilik",
        baslik_en: "Privacy",
        icerik_tr: "Veriniz varsayılan olarak tamamen yereldir. Hassas (PHI) veri hiçbir dış \
                    kanala (AI/ağ) gönderilmez.",
        icerik_en:
            "Your data is fully local by default. Sensitive (PHI) data is never sent to any \
                    external channel (AI/network).",
    },
];

/// Resmî dış doküman adresi (tıklanınca host onaylar + açar — İP-15/İP-18 ince adaptör).
pub const DOKUMAN_URL: &str = "https://biocraftengine.com/docs";

/// Yardım penceresinin bir karedeki sonucu.
#[derive(Debug, Clone)]
pub enum YardimEylem {
    /// Bir kavramın turunu/öğreticisini iste (şimdilik host bilgilendirir; gelecekte derin link).
    DisBaglanti(String),
}

/// Entegre yardım penceresini (yüzen) çizer.  Kapatma `acik` üzerinden yapılır.
pub fn yardim_penceresi(
    ctx: &egui::Context,
    acik: &mut bool,
    arama: &mut String,
    dil: Dil,
    tok: &Tokenlar,
) -> Option<YardimEylem> {
    let tr = matches!(dil, Dil::Tr);
    let mut eylem: Option<YardimEylem> = None;
    let mut acik_yerel = *acik;

    egui::Window::new(metin(tr, "Yardım", "Help"))
        .open(&mut acik_yerel)
        .collapsible(true)
        .resizable(true)
        .default_width(520.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            // Arama kutusu (kavram + doküman başlıklarını süzer).
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.add(
                    egui::TextEdit::singleline(arama)
                        .hint_text(metin(tr, "Yardımda ara…", "Search help…"))
                        .desired_width(f32::INFINITY),
                );
            });
            let q = arama.trim().to_lowercase();
            let eslesir = |a: &str, b: &str| -> bool {
                q.is_empty() || a.to_lowercase().contains(&q) || b.to_lowercase().contains(&q)
            };

            egui::ScrollArea::vertical()
                .max_height(440.0)
                .show(ui, |ui| {
                    // ── Kavramlar ("BAM/VCF nedir?") ──
                    ui.add_space(tok.bosluk.s);
                    ui.label(
                        egui::RichText::new(metin(tr, "Kavramlar", "Concepts"))
                            .strong()
                            .color(tok.renk.vurgu),
                    );
                    let mut kavram_var = false;
                    for &k in Kavram::TUMU {
                        if !eslesir(k.terim(), k.aciklama(tr)) {
                            continue;
                        }
                        kavram_var = true;
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new(format!("{} —", k.terim()))
                                    .strong()
                                    .color(tok.renk.metin),
                            );
                            ui.label(
                                egui::RichText::new(k.aciklama(tr)).color(tok.renk.metin_soluk),
                            );
                        });
                    }
                    if !kavram_var {
                        ui.label(
                            egui::RichText::new(metin(tr, "(eşleşme yok)", "(no match)"))
                                .small()
                                .color(tok.renk.metin_soluk),
                        );
                    }

                    ui.add_space(tok.bosluk.m);
                    ui.separator();

                    // ── Kısayol kartı ──
                    ui.label(
                        egui::RichText::new(metin(tr, "Kısayol Kartı", "Shortcut Card"))
                            .strong()
                            .color(tok.renk.vurgu),
                    );
                    egui::Grid::new("yardim_kisayol_grid")
                        .num_columns(2)
                        .spacing([tok.bosluk.l, tok.bosluk.xs])
                        .striped(true)
                        .show(ui, |ui| {
                            for ks in KISAYOLLAR {
                                let etiket = if tr { ks.tr } else { ks.en };
                                if !eslesir(etiket, ks.tus) {
                                    continue;
                                }
                                ui.label(egui::RichText::new(etiket).color(tok.renk.metin));
                                ui.label(
                                    egui::RichText::new(ks.tus)
                                        .monospace()
                                        .color(tok.renk.metin_soluk),
                                );
                                ui.end_row();
                            }
                        });

                    ui.add_space(tok.bosluk.m);
                    ui.separator();

                    // ── Çevrimdışı temel doküman ──
                    ui.label(
                        egui::RichText::new(metin(tr, "Belgeler (çevrimdışı)", "Docs (offline)"))
                            .strong()
                            .color(tok.renk.vurgu),
                    );
                    for d in DOKUMANLAR {
                        let baslik = if tr { d.baslik_tr } else { d.baslik_en };
                        let icerik = if tr { d.icerik_tr } else { d.icerik_en };
                        if !eslesir(baslik, icerik) {
                            continue;
                        }
                        ui.label(egui::RichText::new(baslik).strong().color(tok.renk.metin));
                        ui.label(egui::RichText::new(icerik).color(tok.renk.metin_soluk));
                        ui.add_space(tok.bosluk.xs);
                    }

                    ui.add_space(tok.bosluk.s);
                    // Dış (çevrimiçi) doküman bağlantısı — tıklanınca host ONAYLAR (dış bağlantı).
                    if ui
                        .button(metin(
                            tr,
                            "🌐 Çevrimiçi belgeleri aç…",
                            "🌐 Open online docs…",
                        ))
                        .on_hover_text(DOKUMAN_URL)
                        .clicked()
                    {
                        eylem = Some(YardimEylem::DisBaglanti(DOKUMAN_URL.to_string()));
                    }
                });
        });

    *acik = acik_yerel;
    eylem
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kisayol_karti_dolu() {
        assert!(!KISAYOLLAR.is_empty());
        for k in KISAYOLLAR {
            assert!(!k.tr.is_empty() && !k.en.is_empty() && !k.tus.is_empty());
        }
    }

    #[test]
    fn dokumanlar_iki_dilde_dolu() {
        for d in DOKUMANLAR {
            assert!(!d.baslik_tr.is_empty() && !d.baslik_en.is_empty());
            assert!(!d.icerik_tr.is_empty() && !d.icerik_en.is_empty());
        }
    }

    #[test]
    fn yardim_penceresi_headless_cizilir() {
        let ctx = egui::Context::default();
        let mut acik = true;
        let mut arama = String::new();
        let _ = ctx.run(egui::RawInput::default(), |c| {
            let _ = yardim_penceresi(c, &mut acik, &mut arama, Dil::Tr, &Tokenlar::koyu());
        });
    }
}
