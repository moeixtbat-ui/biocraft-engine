//! Puan/yorum gösterimi + **raporlama temeli** (İP-18).
//!
//! **Salt-okur (MVP):** yorumlar yalnızca **gösterilir**; yazma/yayınlama bu sürümde yoktur
//! (`MVP-sonrasi.md` §10.1).  Kullanıcı kötü içeriği **bildirebilir** (moderasyon temeli); gerçek
//! moderasyon/itibar arka ucu + içerik sorumluluğunun hukuki çerçevesi sonra (`Hukuk-ve-Operasyon.md`).

use biocraft_net::{RaporSebebi, Yorum};

use super::PazarEylem;
use crate::i18n::Dil;
use crate::tokens::Tokenlar;

/// Bir 0–5 puanı yıldız dizgesine çevirir ("★★★★☆").
pub fn yildizlar(puan: f32) -> String {
    let dolu = puan.round().clamp(0.0, 5.0) as usize;
    let bos = 5 - dolu;
    format!("{}{}", "★".repeat(dolu), "☆".repeat(bos))
}

/// Ortalama puan özeti (yıldız + sayısal + oy sayısı).
pub fn puan_ozeti(ui: &mut egui::Ui, puan: f32, sayi: u32, dil: Dil, tok: &Tokenlar) {
    let tr = matches!(dil, Dil::Tr);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(yildizlar(puan))
                .color(tok.renk.uyari)
                .size(15.0),
        );
        ui.label(
            egui::RichText::new(format!("{puan:.1}"))
                .strong()
                .color(tok.renk.metin),
        );
        ui.label(
            egui::RichText::new(format!(
                "({sayi} {})",
                if tr { "değerlendirme" } else { "ratings" }
            ))
            .color(tok.renk.metin_soluk),
        );
    });
}

/// Yorum listesi + raporlama düğmesini çizer; bir eylem (rapor) üretirse döner.
pub fn yorumlar_ciz(
    ui: &mut egui::Ui,
    kimlik: &str,
    yorumlar: &[Yorum],
    dil: Dil,
    tok: &Tokenlar,
) -> Option<PazarEylem> {
    let tr = matches!(dil, Dil::Tr);
    let mut eylem = None;

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(if tr { "Yorumlar" } else { "Reviews" })
                .strong()
                .color(tok.renk.metin),
        );
        // Raporlama temeli: sebep seçimli "Bildir" menüsü (moderasyon sinyali).
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.menu_button(
                if tr {
                    "⚑ İçeriği bildir"
                } else {
                    "⚑ Report content"
                },
                |ui| {
                    for &sebep in RaporSebebi::TUMU {
                        if ui.button(sebep.etiket(tr)).clicked() {
                            eylem = Some(PazarEylem::Raporla {
                                kimlik: kimlik.to_string(),
                                sebep,
                            });
                            ui.close_menu();
                        }
                    }
                },
            );
        });
    });
    ui.add_space(tok.bosluk.xs);

    if yorumlar.is_empty() {
        ui.label(
            egui::RichText::new(if tr {
                "Henüz yorum yok."
            } else {
                "No reviews yet."
            })
            .italics()
            .color(tok.renk.metin_soluk),
        );
    } else {
        for y in yorumlar {
            egui::Frame::none()
                .fill(tok.renk.yuzey_alt)
                .rounding(tok.yaricap)
                .inner_margin(egui::Margin::same(tok.bosluk.s))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&y.yazar).strong().color(tok.renk.metin));
                        ui.label(
                            egui::RichText::new(yildizlar(y.puan as f32))
                                .color(tok.renk.uyari)
                                .size(12.0),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(&y.tarih)
                                    .small()
                                    .color(tok.renk.metin_soluk),
                            );
                        });
                    });
                    // Yorum metni düz metin olarak gösterilir (HTML render YOK — güvenli).
                    ui.label(egui::RichText::new(&y.metin).color(tok.renk.metin));
                });
            ui.add_space(tok.bosluk.xs);
        }
    }

    // Salt-okur notu (sahte "yorum yaz" işlevi yok — MK-48 ruhu).
    ui.label(
        egui::RichText::new(if tr {
            "Yorum yazma bu sürümde kapalı (salt-okur). Yakında."
        } else {
            "Posting reviews is disabled in this version (read-only). Coming soon."
        })
        .small()
        .italics()
        .color(tok.renk.metin_soluk),
    );

    eylem
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yildizlar_dogru() {
        assert_eq!(yildizlar(5.0), "★★★★★");
        assert_eq!(yildizlar(0.0), "☆☆☆☆☆");
        assert_eq!(yildizlar(3.4), "★★★☆☆"); // 3.4 → 3 dolu
        assert_eq!(yildizlar(3.6), "★★★★☆"); // 3.6 → 4 dolu
                                             // Aralık dışı güvenli.
        assert_eq!(yildizlar(9.0), "★★★★★");
    }

    #[test]
    fn yorumlar_headless_cizilir_ve_rapor_uretebilir() {
        let yorumlar = vec![Yorum::yeni("ada", 5, "harika", "2026-06-01")];
        for dil in [Dil::Tr, Dil::En] {
            let ctx = egui::Context::default();
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let tok = Tokenlar::koyu();
                    puan_ozeti(ui, 4.5, 10, dil, &tok);
                    let _ = yorumlar_ciz(ui, "biocraft.x.y", &yorumlar, dil, &tok);
                });
            });
        }
    }

    #[test]
    fn bos_yorum_listesi_panik_yok() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let tok = Tokenlar::koyu();
                let _ = yorumlar_ciz(ui, "x", &[], Dil::Tr, &tok);
            });
        });
    }
}
