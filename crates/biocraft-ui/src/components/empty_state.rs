//! Boş durum bileşeni (İP-16, TDA madde 5 — boş durum rehberi).
//!
//! Boş bir panel/liste asla anlamsız beyaz alan bırakmaz: ikon + "ne yapılacağı"
//! açıklaması + birincil eylem butonu gösterir (örn. "Yeni Proje", "Veri yükle").
//! Metinler çağıran taraftan (zaten yerelleştirilmiş) gelir.

use crate::tokens::Tokenlar;

/// Boş durum görünümü.
#[derive(Debug, Clone)]
pub struct EmptyState {
    /// Büyük ikon (emoji veya sembol).
    pub ikon: String,
    /// Kısa başlık ("Henüz proje yok").
    pub baslik: String,
    /// Ne yapılacağını anlatan açıklama.
    pub aciklama: String,
    /// Opsiyonel birincil eylem butonunun etiketi.
    pub eylem_etiketi: Option<String>,
}

impl EmptyState {
    /// İkon + başlık + açıklama ile yeni bir boş durum kurar.
    pub fn yeni(
        ikon: impl Into<String>,
        baslik: impl Into<String>,
        aciklama: impl Into<String>,
    ) -> Self {
        Self {
            ikon: ikon.into(),
            baslik: baslik.into(),
            aciklama: aciklama.into(),
            eylem_etiketi: None,
        }
    }

    /// Birincil eylem butonu ekler.
    pub fn with_eylem(mut self, etiket: impl Into<String>) -> Self {
        self.eylem_etiketi = Some(etiket.into());
        self
    }

    /// Bileşeni ortalanmış olarak çizer.  Birincil eyleme tıklanırsa `true` döner.
    pub fn show(&self, ui: &mut egui::Ui, tok: &Tokenlar) -> bool {
        let mut tiklandi = false;
        ui.vertical_centered(|ui| {
            ui.add_space(tok.bosluk.xl);
            ui.label(egui::RichText::new(&self.ikon).size(48.0));
            ui.add_space(tok.bosluk.s);
            ui.label(
                egui::RichText::new(&self.baslik)
                    .size(18.0)
                    .strong()
                    .color(tok.renk.metin),
            );
            ui.add_space(tok.bosluk.xs);
            ui.label(egui::RichText::new(&self.aciklama).color(tok.renk.metin_soluk));
            if let Some(etiket) = &self.eylem_etiketi {
                ui.add_space(tok.bosluk.m);
                let buton = egui::Button::new(
                    egui::RichText::new(etiket)
                        .color(tok.renk.vurgu_ustu)
                        .strong(),
                )
                .fill(tok.renk.vurgu);
                if ui.add(buton).clicked() {
                    tiklandi = true;
                }
            }
            ui.add_space(tok.bosluk.xl);
        });
        tiklandi
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bos_durum_eylemli_kurulabilir() {
        let bd = EmptyState::yeni("📭", "Henüz proje yok", "Yeni bir proje oluşturun.")
            .with_eylem("Yeni Proje");
        assert_eq!(bd.eylem_etiketi.as_deref(), Some("Yeni Proje"));
    }

    #[test]
    fn bos_durum_headless_cizilir() {
        let ctx = egui::Context::default();
        let bd = EmptyState::yeni("📭", "Boş", "Açıklama");
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = bd.show(ui, &Tokenlar::acik());
            });
        });
    }
}
