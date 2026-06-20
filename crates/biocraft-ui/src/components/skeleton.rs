//! Yükleme iskeleti (İP-16, TDA madde 6 — yükleniyor durumu).
//!
//! Donuk/boş ekran yerine içeriğin yerini tutan, hafifçe yanıp sönen (shimmer) gri
//! bloklar gösterir.  Uzun yüklemelerde [`yukleniyor`] yardımcı fonksiyonu spinner +
//! mesaj + opsiyonel iptal sunar.

use crate::i18n::{ceviri, Anahtar, Dil};
use crate::tokens::Tokenlar;

/// İskelet çizim yardımcıları (durumsuz; namespace görevi görür).
pub struct Skeleton;

impl Skeleton {
    /// Tek bir iskelet satırı çizer (verilen genişlik/yükseklikte, yanıp sönen).
    pub fn satir(ui: &mut egui::Ui, tok: &Tokenlar, genislik: f32, yukseklik: f32) {
        let now = ui.input(|i| i.time);
        // 0..1 arası yumuşak nabız.
        let nabiz = ((now * 1.6).sin() * 0.5 + 0.5) as f32;
        let renk = tok.renk.iskelet.gamma_multiply(0.55 + 0.45 * nabiz);
        let (rect, _resp) =
            ui.allocate_exact_size(egui::vec2(genislik, yukseklik), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(tok.yaricap * 0.5), renk);
        // Canlandırma için yeniden çizim iste.
        ui.ctx().request_repaint();
    }

    /// Birkaç satırlık paragraf iskeleti (son satır kısa).
    pub fn paragraf(ui: &mut egui::Ui, tok: &Tokenlar, satir_sayisi: usize) {
        let tam = ui.available_width().min(520.0);
        for i in 0..satir_sayisi {
            let genislik = if i + 1 == satir_sayisi {
                tam * 0.6
            } else {
                tam
            };
            Self::satir(ui, tok, genislik, 12.0);
            ui.add_space(tok.bosluk.s);
        }
    }

    /// Bir liste öğesi iskeleti (avatar bloğu + iki satır) — `adet` kadar tekrar.
    pub fn liste(ui: &mut egui::Ui, tok: &Tokenlar, adet: usize) {
        for _ in 0..adet {
            crate::components::kart_cercevesi(tok).show(ui, |ui| {
                ui.horizontal(|ui| {
                    Self::satir(ui, tok, 40.0, 40.0);
                    ui.vertical(|ui| {
                        Self::satir(ui, tok, 220.0, 12.0);
                        ui.add_space(tok.bosluk.xs);
                        Self::satir(ui, tok, 140.0, 12.0);
                    });
                });
            });
            ui.add_space(tok.bosluk.s);
        }
    }
}

/// Spinner + mesaj + opsiyonel iptal butonu çizer.  İptal tıklanırsa `true` döner.
pub fn yukleniyor(
    ui: &mut egui::Ui,
    dil: Dil,
    tok: &Tokenlar,
    mesaj: Option<&str>,
    iptal_edilebilir: bool,
) -> bool {
    let mut iptal = false;
    ui.horizontal(|ui| {
        ui.add(egui::Spinner::new().color(tok.renk.vurgu));
        ui.add_space(tok.bosluk.s);
        let metin = mesaj.unwrap_or(ceviri(dil, Anahtar::Yukleniyor));
        ui.label(egui::RichText::new(metin).color(tok.renk.metin));
        if iptal_edilebilir {
            ui.add_space(tok.bosluk.m);
            if ui.button(ceviri(dil, Anahtar::Iptal)).clicked() {
                iptal = true;
            }
        }
    });
    ui.ctx().request_repaint();
    iptal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iskelet_ve_yukleniyor_headless_cizilir() {
        let ctx = egui::Context::default();
        let tok = Tokenlar::koyu();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                Skeleton::paragraf(ui, &tok, 3);
                Skeleton::liste(ui, &tok, 2);
                let _ = yukleniyor(ui, Dil::Tr, &tok, Some("Veri yükleniyor…"), true);
            });
        });
    }
}
