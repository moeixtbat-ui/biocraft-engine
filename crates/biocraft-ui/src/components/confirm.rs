//! Onay diyaloğu (İP-16, TDA madde 7 — yıkıcı işlemde onay).
//!
//! Geri döndürülemez/önemli işlemlerde "Emin misiniz?" sorar.  Yıkıcı işlemlerde
//! onay butonu kırmızıdır; mümkünse "bu işlem geri alınabilir" notu gösterilir,
//! değilse "geri alınamaz" uyarısı verilir.

use crate::i18n::{ceviri, Anahtar, Dil};
use crate::tokens::Tokenlar;

/// Onay diyaloğu tanımı.
#[derive(Debug, Clone)]
pub struct ConfirmDialog {
    /// Diyalog başlığı (çağıran tarafından, yerelleştirilmiş).
    pub baslik: String,
    /// Açıklama metni.
    pub mesaj: String,
    /// İşlem yıkıcı mı (onay butonu kırmızı + "geri alınamaz" varsayılan)?
    pub yikici: bool,
    /// Varsa "şu şekilde geri alınabilir" notu (TDA: geri alma güvencesi).
    pub geri_alinabilir_notu: Option<String>,
    /// Onay butonu için özel etiket (yoksa "Evet").
    pub onayla_etiketi: Option<String>,
}

/// Kullanıcının onay kararı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnayKarari {
    /// İşlem onaylandı.
    Onayla,
    /// İşlem iptal edildi.
    Iptal,
}

impl ConfirmDialog {
    /// Başlık + mesaj ile (yıkıcı olmayan) bir onay diyaloğu kurar.
    pub fn yeni(baslik: impl Into<String>, mesaj: impl Into<String>) -> Self {
        Self {
            baslik: baslik.into(),
            mesaj: mesaj.into(),
            yikici: false,
            geri_alinabilir_notu: None,
            onayla_etiketi: None,
        }
    }

    /// İşlemi yıkıcı olarak işaretler (kırmızı onay butonu).
    pub fn yikici(mut self) -> Self {
        self.yikici = true;
        self
    }

    /// "Bu işlem geri alınabilir" notunu ekler.
    pub fn with_geri_alinabilir(mut self, not: impl Into<String>) -> Self {
        self.geri_alinabilir_notu = Some(not.into());
        self
    }

    /// Onay butonu için özel etiket belirler.
    pub fn with_onay_etiketi(mut self, etiket: impl Into<String>) -> Self {
        self.onayla_etiketi = Some(etiket.into());
        self
    }

    /// Diyaloğu çizer.  Karar verildiyse döndürür; aksi halde `None`.
    pub fn show(&self, ctx: &egui::Context, dil: Dil, tok: &Tokenlar) -> Option<OnayKarari> {
        let mut sonuc: Option<OnayKarari> = None;
        let baslik_ikon = if self.yikici { "⚠  " } else { "" };
        let baslik_renk = if self.yikici {
            tok.renk.hata
        } else {
            tok.renk.metin
        };

        egui::Window::new(
            egui::RichText::new(format!("{baslik_ikon}{}", self.baslik))
                .color(baslik_renk)
                .strong(),
        )
        .id(egui::Id::new("biocraft_onay_diyalog"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.add_space(tok.bosluk.s);
            ui.label(egui::RichText::new(&self.mesaj).color(tok.renk.metin));

            // Geri alınabilirlik notu / uyarısı.
            ui.add_space(tok.bosluk.s);
            if let Some(not) = &self.geri_alinabilir_notu {
                ui.label(
                    egui::RichText::new(format!("↩  {not}"))
                        .small()
                        .color(tok.renk.basari),
                );
            } else if self.yikici {
                ui.label(
                    egui::RichText::new(ceviri(dil, Anahtar::GeriAlinamaz))
                        .small()
                        .color(tok.renk.metin_soluk),
                );
            }

            ui.add_space(tok.bosluk.m);
            ui.horizontal(|ui| {
                if ui.button(ceviri(dil, Anahtar::Iptal)).clicked() {
                    sonuc = Some(OnayKarari::Iptal);
                }
                let onay_metni = self
                    .onayla_etiketi
                    .clone()
                    .unwrap_or_else(|| ceviri(dil, Anahtar::Evet).to_string());
                let onay_buton = if self.yikici {
                    egui::Button::new(
                        egui::RichText::new(onay_metni)
                            .color(egui::Color32::WHITE)
                            .strong(),
                    )
                    .fill(tok.renk.hata)
                } else {
                    egui::Button::new(
                        egui::RichText::new(onay_metni)
                            .color(egui::Color32::WHITE)
                            .strong(),
                    )
                    .fill(tok.renk.vurgu)
                };
                if ui.add(onay_buton).clicked() {
                    sonuc = Some(OnayKarari::Onayla);
                }
            });
        });

        // Esc → iptal (güvenli varsayılan).
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            sonuc = sonuc.or(Some(OnayKarari::Iptal));
        }
        sonuc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yikici_bayragi_ve_not_kurulabilir() {
        let d = ConfirmDialog::yeni("Sil?", "5 dosya silinecek")
            .yikici()
            .with_geri_alinabilir("Çöp kutusundan geri alınabilir");
        assert!(d.yikici);
        assert!(d.geri_alinabilir_notu.is_some());
    }

    #[test]
    fn esc_ile_iptal_doner() {
        let ctx = egui::Context::default();
        let d = ConfirmDialog::yeni("Sil?", "Emin misiniz?").yikici();
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        });
        let mut karar = None;
        let _ = ctx.run(input, |ctx| {
            karar = d.show(ctx, Dil::Tr, &Tokenlar::acik());
        });
        assert_eq!(karar, Some(OnayKarari::Iptal));
    }
}
