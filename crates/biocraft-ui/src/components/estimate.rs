//! Büyük işlem öncesi tahmin diyaloğu (İP-16, TDA madde 16).
//!
//! Uzun sürecek bir işlemden önce "Bu işlem yaklaşık ~X sürebilir, devam?" diye sorar.
//! Süre, [`crate::i18n::insan_sure`] ile dile göre okunaklı biçime çevrilir.

use crate::i18n::{ceviri, insan_sure, tahmin_metni, Anahtar, Dil};
use crate::tokens::Tokenlar;

/// Büyük işlem tahmin diyaloğu.
#[derive(Debug, Clone)]
pub struct EstimateDialog {
    /// İşlemi tanımlayan kısa mesaj (örn. "12 GB BAM dosyası indekslenecek").
    pub mesaj: String,
    /// Tahmini süre (saniye).
    pub tahmini_saniye: f64,
}

/// Kullanıcının tahmin diyaloğundaki kararı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TahminKarari {
    /// İşleme devam et.
    Devam,
    /// Vazgeç.
    Iptal,
}

impl EstimateDialog {
    /// Mesaj + tahmini süre (saniye) ile yeni bir diyalog kurar.
    pub fn yeni(mesaj: impl Into<String>, tahmini_saniye: f64) -> Self {
        Self {
            mesaj: mesaj.into(),
            tahmini_saniye,
        }
    }

    /// Diyaloğu çizer.  Karar verildiyse döndürür; aksi halde `None`.
    pub fn show(&self, ctx: &egui::Context, dil: Dil, tok: &Tokenlar) -> Option<TahminKarari> {
        let mut sonuc: Option<TahminKarari> = None;

        egui::Window::new(
            egui::RichText::new(format!("⏳  {}", ceviri(dil, Anahtar::TahminBasligi)))
                .color(tok.renk.bilgi)
                .strong(),
        )
        .id(egui::Id::new("biocraft_tahmin_diyalog"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.add_space(tok.bosluk.s);
            ui.label(egui::RichText::new(&self.mesaj).color(tok.renk.metin));
            ui.add_space(tok.bosluk.s);
            let sure = insan_sure(dil, self.tahmini_saniye);
            ui.label(
                egui::RichText::new(tahmin_metni(dil, &sure))
                    .strong()
                    .color(tok.renk.metin),
            );

            ui.add_space(tok.bosluk.m);
            ui.horizontal(|ui| {
                if ui.button(ceviri(dil, Anahtar::Iptal)).clicked() {
                    sonuc = Some(TahminKarari::Iptal);
                }
                let devam = egui::Button::new(
                    egui::RichText::new(ceviri(dil, Anahtar::Devam))
                        .color(egui::Color32::WHITE)
                        .strong(),
                )
                .fill(tok.renk.vurgu);
                if ui.add(devam).clicked() {
                    sonuc = Some(TahminKarari::Devam);
                }
            });
        });

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            sonuc = sonuc.or(Some(TahminKarari::Iptal));
        }
        sonuc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tahmin_headless_cizilir() {
        let ctx = egui::Context::default();
        let d = EstimateDialog::yeni("Büyük dosya indekslenecek", 300.0);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let _ = d.show(ctx, Dil::Tr, &Tokenlar::acik());
        });
    }
}
