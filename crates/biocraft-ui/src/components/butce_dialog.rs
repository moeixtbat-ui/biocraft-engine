//! Bellek bütçesi diyaloğu (İP-08, TDA madde 1/11).
//!
//! Bir dosya açılmadan önce tahmini RAM ihtiyacı bütçeye sığmıyorsa gösterilir.
//! `biocraft-mem` (L2) yalnızca **kararı/teklifi** üretir ([`AkisTeklifi`]); bu bileşen
//! onu görsel diyaloğa dönüştürür (egui = L4) — katman ayrımı (MK-40) korunur.
//!
//! Kullanıcıya üç yol sunulur: **akış (stream) modunda aç**, **bulutta işle** (MVP'de
//! yer tutucu — pasif buton) ve **iptal**.  Karar verildiğinde [`AcmaSecenegi`] döner.

use biocraft_mem::{insan_bayt, AcmaSecenegi, AkisTeklifi};

use crate::i18n::{butce_metni, ceviri, Anahtar, Dil};
use crate::tokens::Tokenlar;

/// Bellek bütçesi diyaloğu — bir [`AkisTeklifi`]'ni sarar.
#[derive(Debug, Clone)]
pub struct ButceDialog {
    /// `biocraft-mem`'in ürettiği bütçe teklifi (dosya/RAM/boş + seçenekler).
    pub teklif: AkisTeklifi,
}

impl ButceDialog {
    /// Bir bütçe teklifinden diyalog kurar.
    pub fn yeni(teklif: AkisTeklifi) -> Self {
        Self { teklif }
    }

    /// Bir açma seçeneğinin diyalogdaki (dile çevrilebilir) etiketini verir.
    fn secenek_anahtari(secenek: AcmaSecenegi) -> Anahtar {
        match secenek {
            AcmaSecenegi::AkisModu => Anahtar::ButceAkisModu,
            AcmaSecenegi::CloudBurst => Anahtar::ButceBulut,
            AcmaSecenegi::Iptal => Anahtar::Iptal,
        }
    }

    /// Diyaloğu çizer.  Karar verildiyse seçilen [`AcmaSecenegi`]'ni döndürür; aksi halde `None`.
    pub fn show(&self, ctx: &egui::Context, dil: Dil, tok: &Tokenlar) -> Option<AcmaSecenegi> {
        let mut sonuc: Option<AcmaSecenegi> = None;

        egui::Window::new(
            egui::RichText::new(format!("📊  {}", ceviri(dil, Anahtar::ButceBasligi)))
                .color(tok.renk.uyari)
                .strong(),
        )
        .id(egui::Id::new("biocraft_butce_diyalog"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_max_width(460.0);
            ui.add_space(tok.bosluk.s);

            // Açıklama (sade dil) — sayılar insan-okunur biçimde.
            let metin = butce_metni(
                dil,
                &insan_bayt(self.teklif.dosya_bayt),
                &insan_bayt(self.teklif.tahmini_ram_bayt),
                &insan_bayt(self.teklif.bos_bayt),
            );
            ui.label(egui::RichText::new(metin).color(tok.renk.metin));

            ui.add_space(tok.bosluk.m);
            ui.horizontal(|ui| {
                for &secenek in &self.teklif.secenekler {
                    let etiket = ceviri(dil, Self::secenek_anahtari(secenek));
                    match secenek {
                        // Önerilen yol: vurgulu buton.
                        AcmaSecenegi::AkisModu => {
                            let buton = egui::Button::new(
                                egui::RichText::new(etiket)
                                    .color(tok.renk.vurgu_ustu)
                                    .strong(),
                            )
                            .fill(tok.renk.vurgu);
                            if ui.add(buton).clicked() {
                                sonuc = Some(secenek);
                            }
                        }
                        // Yer tutucu (MVP'de etkin değil): pasif buton.
                        AcmaSecenegi::CloudBurst => {
                            let yanit = ui.add_enabled(
                                secenek.etkin(),
                                egui::Button::new(
                                    egui::RichText::new(etiket).color(tok.renk.metin_soluk),
                                ),
                            );
                            yanit.on_disabled_hover_text(match dil {
                                Dil::Tr => "Bulut işleme yakında gelecek.",
                                Dil::En => "Cloud processing is coming soon.",
                            });
                        }
                        AcmaSecenegi::Iptal => {
                            if ui.button(etiket).clicked() {
                                sonuc = Some(secenek);
                            }
                        }
                    }
                }
            });
        });

        // Esc = iptal (TDA: her modal kaçışla kapanabilir).
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            sonuc = sonuc.or(Some(AcmaSecenegi::Iptal));
        }
        sonuc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biocraft_mem::{dosya_butce_kontrol, BellekOrkestratoru, ButceKarari};

    fn ornek_teklif() -> AkisTeklifi {
        // 800 MB dosya × 3 = ~2.4 GB tahmini, 1 GB bütçe → akış teklifi.
        let ork = BellekOrkestratoru::yeni(1024 * 1024 * 1024);
        match dosya_butce_kontrol(800 * 1024 * 1024, 3.0, &ork) {
            ButceKarari::AkisOnerilir(t) => t,
            _ => panic!("akış teklifi bekleniyordu"),
        }
    }

    #[test]
    fn butce_diyalogu_headless_cizilir() {
        let d = ButceDialog::yeni(ornek_teklif());
        for dil in [Dil::Tr, Dil::En] {
            let ctx = egui::Context::default();
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                let _ = d.show(ctx, dil, &Tokenlar::koyu());
            });
        }
    }

    #[test]
    fn secenek_etiket_eslemesi_dolu() {
        // Her seçenek bir çeviri anahtarına eşlenmeli (boş etiket olmamalı).
        for s in [
            AcmaSecenegi::AkisModu,
            AcmaSecenegi::CloudBurst,
            AcmaSecenegi::Iptal,
        ] {
            let a = ButceDialog::secenek_anahtari(s);
            assert!(!ceviri(Dil::Tr, a).is_empty());
            assert!(!ceviri(Dil::En, a).is_empty());
        }
    }
}
