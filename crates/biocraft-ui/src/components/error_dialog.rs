//! Hata diyaloğu (İP-16, TDA madde 4 — anlamlı hata mesajı).
//!
//! [`biocraft_types::ErrorReport`] standart şemasını çizer:
//! **ne oldu → neden → nasıl çözülür → (katlanır) teknik detay → correlation_id**.
//! Şablon tip düzeyinde zorunludur (`ErrorReport::new` üç alanı ister), bu yüzden
//! diyalog asla "kriptik kod" gösteremez.

use biocraft_types::ErrorReport;

use crate::i18n::{ceviri, Anahtar, Dil};
use crate::tokens::Tokenlar;

/// Hata diyaloğu denetleyicisi.  (Teknik-detay aç/kapa durumu egui'nin kendi
/// bellek alanında [`egui::CollapsingHeader`] tarafından tutulur.)
#[derive(Default)]
pub struct ErrorDialog;

/// Kullanıcının hata diyaloğunda yaptığı seçim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HataDiyalogEylem {
    /// Diyalog kapatıldı (Kapat butonu, pencere X'i veya Esc).
    Kapat,
    /// Çözüm eylem butonuna tıklandı (örn. "Tekrar dene").
    EylemTiklandi,
    /// Korelasyon kimliği panoya kopyalandı.
    KimlikKopyalandi,
}

impl ErrorDialog {
    /// Yeni bir hata diyaloğu durumu.
    pub fn new() -> Self {
        Self
    }

    /// Diyaloğu çizer.  Kullanıcı bir şey yaptıysa ilgili eylemi döndürür.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        dil: Dil,
        tok: &Tokenlar,
        rapor: &ErrorReport,
    ) -> Option<HataDiyalogEylem> {
        let mut sonuc: Option<HataDiyalogEylem> = None;
        let mut pencere_acik = true;

        egui::Window::new(
            egui::RichText::new(format!("⛔  {}", ceviri(dil, Anahtar::HataBasligi)))
                .color(tok.renk.hata)
                .strong(),
        )
        .id(egui::Id::new("biocraft_hata_diyalog"))
        .collapsible(false)
        .resizable(false)
        .open(&mut pencere_acik)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_max_width(460.0);

            bolum(ui, tok, ceviri(dil, Anahtar::NeOldu), &rapor.ne_oldu);
            bolum(ui, tok, ceviri(dil, Anahtar::Neden), &rapor.neden);
            bolum(
                ui,
                tok,
                ceviri(dil, Anahtar::NasilCozulur),
                &rapor.nasil_cozulur,
            );

            // Çözüm için opsiyonel birincil eylem butonu.
            if let Some(etiket) = &rapor.eylem_etiketi {
                ui.add_space(tok.bosluk.s);
                let buton = egui::Button::new(
                    egui::RichText::new(etiket)
                        .color(egui::Color32::WHITE)
                        .strong(),
                )
                .fill(tok.renk.vurgu);
                if ui.add(buton).clicked() {
                    sonuc = Some(HataDiyalogEylem::EylemTiklandi);
                }
            }

            // Katlanır teknik detay (varsayılan gizli).
            if let Some(detay) = &rapor.teknik_detay {
                ui.add_space(tok.bosluk.s);
                egui::CollapsingHeader::new(ceviri(dil, Anahtar::TeknikDetay))
                    .id_salt("biocraft_hata_teknik_detay")
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(detay)
                                .monospace()
                                .color(tok.renk.metin_soluk),
                        );
                    });
            }

            ui.add_space(tok.bosluk.s);
            ui.separator();

            // Korelasyon kimliği + kopyala.
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{}: {}",
                        ceviri(dil, Anahtar::KorelasyonKimligi),
                        rapor.correlation_id
                    ))
                    .small()
                    .color(tok.renk.metin_soluk),
                );
                if ui
                    .small_button("⧉")
                    .on_hover_text(ceviri(dil, Anahtar::Kopyala))
                    .clicked()
                {
                    ui.output_mut(|o| o.copied_text = rapor.correlation_id.to_string());
                    sonuc = Some(HataDiyalogEylem::KimlikKopyalandi);
                }
            });

            ui.add_space(tok.bosluk.s);
            ui.horizontal(|ui| {
                if ui.button(ceviri(dil, Anahtar::Kapat)).clicked() {
                    sonuc = Some(HataDiyalogEylem::Kapat);
                }
            });
        });

        // Pencere X'i veya Esc → kapat.
        if !pencere_acik {
            sonuc = sonuc.or(Some(HataDiyalogEylem::Kapat));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            sonuc = sonuc.or(Some(HataDiyalogEylem::Kapat));
        }
        sonuc
    }
}

/// Başlık + içerik biçiminde tek bir hata bölümü çizer.
fn bolum(ui: &mut egui::Ui, tok: &Tokenlar, baslik: &str, icerik: &str) {
    ui.add_space(tok.bosluk.s);
    ui.label(egui::RichText::new(baslik).strong().color(tok.renk.metin));
    ui.label(egui::RichText::new(icerik).color(tok.renk.metin));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diyalog_calisir_ve_kapanabilir() {
        // Esc ile kapanma: headless egui bağlamında bir kare çalıştır.
        let ctx = egui::Context::default();
        let mut dlg = ErrorDialog::new();
        let rapor = ErrorReport::new(
            "Proje açılamadı",
            "Dosya bulunamadı",
            "Dosyayı yeniden bağlayın",
        )
        .with_teknik_detay("ENOENT");

        // Esc tuşu basılı bir girdi simüle et.
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        });

        let mut yakalanan: Option<HataDiyalogEylem> = None;
        let _ = ctx.run(input, |ctx| {
            yakalanan = dlg.show(ctx, Dil::Tr, &Tokenlar::acik(), &rapor);
        });
        assert_eq!(yakalanan, Some(HataDiyalogEylem::Kapat));
    }
}
