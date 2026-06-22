//! Mağaza öğesi **detay görünümü** (İP-18) — ekran görüntüsü + geliştirici kimliği + doğrulama
//! rozeti + puan + fiyat/lisans/atıf + tek-tık kur/güncelle/kaldır + yorumlar.
//!
//! **Güvenli render:** tüm metin alanları düz metindir (HTML render YOK).  Ekran görüntüleri MVP'de
//! ham görsel olarak indirilmez; yalnızca **etiket** yer tutucuları gösterilir (hafif + güvenli).

use biocraft_net::{Fiyat, PazarOgesi};

use super::dogrulama::{dogrulama_rozeti, rozet_ipucu};
use super::reviews::{puan_ozeti, yorumlar_ciz};
use super::{KurulumDurum, PazarEylem};
use crate::i18n::Dil;
use crate::tokens::Tokenlar;

/// Bir öğenin tam detayını çizer; bir eylem (kur/güncelle/kaldır/rapor) üretirse döner.
pub fn detay_ciz(
    ui: &mut egui::Ui,
    oge: &PazarOgesi,
    kurulum: KurulumDurum,
    dil: Dil,
    tok: &Tokenlar,
) -> Option<PazarEylem> {
    let tr = matches!(dil, Dil::Tr);
    let mut eylem = None;

    // ── Başlık: ad + doğrulama rozeti ────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.heading(egui::RichText::new(&oge.ad).color(tok.renk.metin));
        dogrulama_rozeti(ui, oge.dogrulama, dil, tok);
    });
    // Geliştirici kimliği + sürüm + tür/kategori.
    ui.label(
        egui::RichText::new(format!(
            "{} · v{} · {} / {}",
            oge.yayinci,
            oge.surum,
            oge.tur.etiket(tr),
            oge.kategori.etiket(tr),
        ))
        .color(tok.renk.metin_soluk),
    );
    ui.label(
        egui::RichText::new(format!("ID: {}", oge.kimlik))
            .small()
            .monospace()
            .color(tok.renk.metin_soluk),
    );
    ui.add_space(tok.bosluk.s);

    // ── Eylem satırı: kur / güncelle / kaldır (tek tık) ───────────────────────
    if let Some(e) = eylem_satiri(ui, oge, kurulum, tr, tok) {
        eylem = Some(e);
    }
    ui.add_space(tok.bosluk.s);

    // ── Puan + indirme + son güncelleme ───────────────────────────────────────
    puan_ozeti(ui, oge.puan, oge.puan_sayisi, dil, tok);
    ui.label(
        egui::RichText::new(format!(
            "{} {} · {} {}",
            insan_sayi(oge.indirme),
            if tr { "indirme" } else { "downloads" },
            if tr {
                "son güncelleme:"
            } else {
                "last updated:"
            },
            oge.son_guncelleme,
        ))
        .small()
        .color(tok.renk.metin_soluk),
    );
    ui.add_space(tok.bosluk.s);

    // ── Fiyat / lisans / atıf (Bio-kredi yer tutucu; atıf/lisans şeffaf) ───────
    fiyat_lisans_ciz(ui, oge, tr, tok);
    ui.add_space(tok.bosluk.m);

    // ── Ekran görüntüleri (yer tutucu etiketler — güvenli/hafif) ──────────────
    if !oge.ekran_etiketleri.is_empty() {
        ui.label(
            egui::RichText::new(if tr {
                "Ekran görüntüleri"
            } else {
                "Screenshots"
            })
            .strong()
            .color(tok.renk.metin),
        );
        ui.add_space(tok.bosluk.xs);
        ui.horizontal_wrapped(|ui| {
            for etiket in &oge.ekran_etiketleri {
                egui::Frame::none()
                    .fill(tok.renk.yuzey_alt)
                    .stroke(egui::Stroke::new(1.0, tok.renk.kenarlik))
                    .rounding(tok.yaricap)
                    .inner_margin(egui::Margin::same(tok.bosluk.m))
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(150.0, 84.0));
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("🖼").size(28.0));
                            ui.label(
                                egui::RichText::new(etiket)
                                    .small()
                                    .color(tok.renk.metin_soluk),
                            );
                        });
                    });
            }
        });
        ui.add_space(tok.bosluk.m);
    }

    // ── Açıklama (düz metin) ──────────────────────────────────────────────────
    ui.label(
        egui::RichText::new(if tr { "Açıklama" } else { "Description" })
            .strong()
            .color(tok.renk.metin),
    );
    ui.add_space(tok.bosluk.xs);
    ui.label(egui::RichText::new(&oge.aciklama).color(tok.renk.metin));

    // "Beklemede" öğeler için dürüst uyarı şeridi.
    if matches!(
        oge.dogrulama,
        biocraft_net::DogrulamaDurumu::IncelemeBekliyor
    ) {
        ui.add_space(tok.bosluk.s);
        egui::Frame::none()
            .fill(tok.renk.uyari_zemin)
            .rounding(tok.yaricap)
            .inner_margin(egui::Margin::same(tok.bosluk.s))
            .show(ui, |ui| {
                ui.label(egui::RichText::new(rozet_ipucu(oge.dogrulama, tr)).color(tok.renk.uyari));
            });
    }
    ui.add_space(tok.bosluk.m);

    // ── Yorumlar + raporlama ──────────────────────────────────────────────────
    ui.separator();
    if let Some(e) = yorumlar_ciz(ui, &oge.kimlik, &oge.yorumlar, dil, tok) {
        eylem = Some(e);
    }

    eylem
}

/// Kur/Güncelle/Kaldır düğme satırı.  Kurulu öğe geri-alınabilir biçimde kaldırılabilir (İP-07).
fn eylem_satiri(
    ui: &mut egui::Ui,
    oge: &PazarOgesi,
    kurulum: KurulumDurum,
    tr: bool,
    tok: &Tokenlar,
) -> Option<PazarEylem> {
    let mut eylem = None;
    ui.horizontal(|ui| match kurulum {
        KurulumDurum::KuruluDegil => {
            let etiket = if tr { "⬇ Kur" } else { "⬇ Install" };
            if vurgu_dugme(ui, etiket, tok).clicked() {
                eylem = Some(PazarEylem::Kur(oge.kimlik.clone()));
            }
        }
        KurulumDurum::Kurulu => {
            ui.add_enabled(
                false,
                egui::Button::new(if tr { "✓ Kurulu" } else { "✓ Installed" }),
            );
            if ui
                .button(if tr { "🗑 Kaldır" } else { "🗑 Remove" })
                .clicked()
            {
                eylem = Some(PazarEylem::Kaldir(oge.kimlik.clone()));
            }
        }
        KurulumDurum::GuncellemeVar => {
            if vurgu_dugme(ui, if tr { "⬆ Güncelle" } else { "⬆ Update" }, tok).clicked() {
                eylem = Some(PazarEylem::Guncelle(oge.kimlik.clone()));
            }
            if ui
                .button(if tr { "🗑 Kaldır" } else { "🗑 Remove" })
                .clicked()
            {
                eylem = Some(PazarEylem::Kaldir(oge.kimlik.clone()));
            }
        }
    });
    eylem
}

/// Fiyat + lisans + atıf bloğu.  Ücretli ise Bio-kredi **yer tutucu** açıkça etiketlenir.
fn fiyat_lisans_ciz(ui: &mut egui::Ui, oge: &PazarOgesi, tr: bool, tok: &Tokenlar) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(if tr { "Fiyat:" } else { "Price:" }).color(tok.renk.metin_soluk),
        );
        let renk = if oge.fiyat.ucretsiz_mi() {
            tok.renk.basari
        } else {
            tok.renk.uyari
        };
        ui.label(
            egui::RichText::new(oge.fiyat.etiket(tr))
                .strong()
                .color(renk),
        );
    });
    if let Fiyat::Ucretli { .. } = oge.fiyat {
        ui.label(
            egui::RichText::new(if tr {
                "Not: Bio-kredi yalnızca yer tutucudur; bu sürümde gerçek ödeme/ekonomi yoktur."
            } else {
                "Note: Bio-credits are only a placeholder; no real payment/economy in this version."
            })
            .small()
            .italics()
            .color(tok.renk.metin_soluk),
        );
    }
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(if tr { "Lisans:" } else { "License:" })
                .color(tok.renk.metin_soluk),
        );
        ui.label(egui::RichText::new(&oge.lisans).color(tok.renk.metin));
    });
    if let Some(atif) = &oge.atif {
        ui.label(
            egui::RichText::new(format!(
                "{} {atif}",
                if tr { "Atıf:" } else { "Attribution:" }
            ))
            .small()
            .color(tok.renk.metin_soluk),
        );
    }
}

/// Vurgu renkli (accent) bir düğme.
fn vurgu_dugme(ui: &mut egui::Ui, etiket: &str, tok: &Tokenlar) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(etiket)
                .color(tok.renk.vurgu_ustu)
                .strong(),
        )
        .fill(tok.renk.vurgu),
    )
}

/// Büyük sayıları kısaltır (1234 → "1.2K", 5_300_000 → "5.3M").
fn insan_sayi(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biocraft_net::{Kategori, OgeTuru};

    #[test]
    fn insan_sayi_kisaltir() {
        assert_eq!(insan_sayi(500), "500");
        assert_eq!(insan_sayi(1_200), "1.2K");
        assert_eq!(insan_sayi(5_300_000), "5.3M");
    }

    #[test]
    fn detay_headless_cizilir_tum_durumlar() {
        let mut oge = PazarOgesi::yeni(
            "biocraft.x.y",
            "Test",
            "Acme",
            OgeTuru::Eklenti,
            Kategori::Analiz,
        );
        oge.aciklama = "Açıklama metni".into();
        oge.ekran_etiketleri = vec!["A".into(), "B".into()];
        oge.fiyat = Fiyat::Ucretli { bio_kredi: 5 };
        for kurulum in [
            KurulumDurum::KuruluDegil,
            KurulumDurum::Kurulu,
            KurulumDurum::GuncellemeVar,
        ] {
            for dil in [Dil::Tr, Dil::En] {
                let ctx = egui::Context::default();
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let tok = Tokenlar::koyu();
                        let _ = detay_ciz(ui, &oge, kurulum, dil, &tok);
                    });
                });
            }
        }
    }
}
