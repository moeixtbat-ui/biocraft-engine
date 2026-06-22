//! Side Panel (200–600 px, yeniden boyutlanır) — bağlama (aktif moda) göre içerik (İP-03).
//!
//! İçerik [`ActivityMod`]'a göre değişir; bugün her mod için **yer tutucu** içerik gösterilir
//! (gerçek dosya ağacı/eklenti/arama/AI/DB akışları sonraki paketlerde).  Eksik özellikler
//! "henüz yok / yapılandırılmadı" diye **açıkça** işaretlenir — sessiz başarısızlık yok (TDA m.1).
//!
//! Panel genişliği egui tarafından sürükleyerek değiştirilebilir; ölçülen genişlik döndürülür ve
//! `biocraft-app` tarafından kalıcı duruma yazılır → oturumlar arası korunur (kabul kriteri).
// MK-52: tüm renkler token'dan; metinler i18n'den (MK-53).

use crate::components::EmptyState;
use crate::i18n::Dil;
use crate::shell::activity_bar::ActivityMod;
use crate::shell::layout::{yan_panel_araligi, yan_panel_sikistir};
use crate::tokens::{Onem, Tokenlar};

/// Side Panel'i çizer ve panelin ölçülen güncel genişliğini döner (kalıcı duruma yazılır).
///
/// `varsayilan_genislik`: kalıcı durumdan geri yüklenen genişlik (ilk karede uygulanır, sonra
/// kullanıcı sürüklemesi geçerli olur).  `[200, 600]` aralığına `width_range` ile zorlanır.
pub fn yan_panel(
    ctx: &egui::Context,
    mod_: ActivityMod,
    dil: Dil,
    tok: &Tokenlar,
    varsayilan_genislik: f32,
) -> f32 {
    let yanit = egui::SidePanel::left("biocraft_yan")
        .resizable(true)
        .default_width(yan_panel_sikistir(varsayilan_genislik))
        .width_range(yan_panel_araligi())
        .show(ctx, |ui| {
            // Başlık: aktif mod ikon + adı.
            ui.add_space(tok.bosluk.s);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(mod_.ikon())
                        .size(16.0)
                        .color(tok.renk.vurgu),
                );
                ui.heading(mod_.baslik(dil));
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    mod_icerigi(ui, mod_, dil, tok);
                });
        });
    yanit.response.rect.width()
}

/// Aktif moda göre yer tutucu içerik (her biri ileride gerçek panelle değişir).
fn mod_icerigi(ui: &mut egui::Ui, mod_: ActivityMod, dil: Dil, tok: &Tokenlar) {
    let tr = matches!(dil, Dil::Tr);
    match mod_ {
        ActivityMod::Proje => {
            // Örnek (statik) proje ağacı — gerçek ağaç İP-01/02 ile bağlanır.
            ui.label(
                egui::RichText::new(if tr {
                    "Örnek proje (yer tutucu)"
                } else {
                    "Sample project (placeholder)"
                })
                .color(tok.renk.metin_soluk)
                .small(),
            );
            ui.add_space(tok.bosluk.xs);
            agac_dugumu(ui, 0, "📂", if tr { "Projem" } else { "MyProject" }, tok);
            agac_dugumu(ui, 1, "📄", "genom.fasta", tok);
            agac_dugumu(ui, 1, "📄", "varyantlar.vcf", tok);
            agac_dugumu(ui, 1, "📁", if tr { "sonuçlar" } else { "results" }, tok);
            agac_dugumu(ui, 2, "📄", "rapor.txt", tok);
        }
        ActivityMod::Eklentiler => {
            EmptyState::yeni(
                "🧩",
                if tr {
                    "Kurulu eklenti yok"
                } else {
                    "No plugins installed"
                },
                if tr {
                    "BioCraft Studio çekirdek eklentisi ileride buradan yönetilecek."
                } else {
                    "The BioCraft Studio core plugin will be managed here later."
                },
            )
            .show(ui, tok);
        }
        ActivityMod::Arama => {
            ui.label(if tr { "Ara:" } else { "Search:" });
            // İşlevsiz arama kutusu (gerçek arama sonraki paketlerde) — yer tutucu.
            let mut bos = String::new();
            ui.add_enabled(
                false,
                egui::TextEdit::singleline(&mut bos).hint_text(if tr {
                    "proje içinde ara…"
                } else {
                    "search in project…"
                }),
            );
            ui.add_space(tok.bosluk.xs);
            ui.label(
                egui::RichText::new(if tr {
                    "Arama, proje/genom verisi yüklendiğinde etkinleşir."
                } else {
                    "Search activates once project/genome data is loaded."
                })
                .color(tok.renk.metin_soluk)
                .small(),
            );
        }
        ActivityMod::Ai => {
            // MK-48: MVP'de AI yüzeyi "yapılandırılmadı" etiketli; sahte AI gösterilmez.
            EmptyState::yeni(
                "✨",
                if tr { "AI yapılandırılmadı" } else { "AI not configured" },
                if tr {
                    "AI yüzeyi bu sürümde yapılandırılmadı (MK-48). Sağlayıcı/anahtar ileride eklenir."
                } else {
                    "The AI surface is not configured in this version (MK-48). Provider/key added later."
                },
            )
            .show(ui, tok);
        }
        ActivityMod::Veritabani => {
            EmptyState::yeni(
                "🗄",
                if tr {
                    "Bağlı veritabanı yok"
                } else {
                    "No database connected"
                },
                if tr {
                    "Referans/varyant veritabanları (dbSNP, ClinVar…) ileride buraya bağlanır."
                } else {
                    "Reference/variant databases (dbSNP, ClinVar…) will connect here later."
                },
            )
            .show(ui, tok);
        }
        ActivityMod::Ayar => {
            ui.label(
                egui::RichText::new(if tr {
                    "Hızlı ayarlar"
                } else {
                    "Quick settings"
                })
                .color(tok.renk.metin_soluk)
                .small(),
            );
            ui.add_space(tok.bosluk.xs);
            // Tam ayar ekranına yönlendirme (İP-12) — Görünüm → Ayarlar veya Ctrl+,.
            ui.colored_label(
                tok.onem_rengi(Onem::Bilgi),
                if tr {
                    "ⓘ Tüm ayarlar için: Görünüm → Ayarlar (Ctrl+,). Kategorili + aranabilir."
                } else {
                    "ⓘ For all settings: View → Settings (Ctrl+,). Categorized + searchable."
                },
            );
        }
    }
}

/// Basit girintili ağaç düğümü (yer tutucu proje ağacı için).
fn agac_dugumu(ui: &mut egui::Ui, derinlik: usize, ikon: &str, ad: &str, tok: &Tokenlar) {
    ui.horizontal(|ui| {
        ui.add_space(derinlik as f32 * tok.bosluk.m);
        ui.label(ikon);
        ui.label(ad);
    });
}
