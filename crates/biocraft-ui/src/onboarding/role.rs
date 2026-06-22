//! "Rolün?" seçimi (İP-17, K1) — ilk açılışta **atlanabilir** rol sorusu.
//!
//! Amaç: deneyimi kişiye göre yumuşatmak.  Öğrenci/Araştırmacı/Geliştirici seçimi, **önerilen**
//! başlangıç şablonunu ve hangi ipuçlarının öne çıkacağını belirler.  Seçim **dayatılmaz**
//! (deneyimli kullanıcı "Atla" diyebilir) ve **kaydedilir** (bir daha sorulmaz; "Yardım > Tur"dan
//! tekrar açılabilir).  Saf model + küçük egui adaptörü; tüm metin TR/EN (MK-53).

use crate::i18n::{ceviri, Anahtar, Dil};
use crate::tokens::Tokenlar;

use super::metin;
use super::templates::OnboardingSablon;

/// Kullanıcının kendini tanımladığı rol.  Yalnızca **öneri** üretir; hiçbir özelliği kısıtlamaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rol {
    /// Öğrenci: en görsel/rehberli başlangıç; kavram ipuçları öne çıkar.
    Ogrenci,
    /// Araştırmacı: analiz/varyant odaklı; veri-bilim akışları öne çıkar.
    Arastirmaci,
    /// Geliştirici: kod/node/eklenti odaklı; boş başlangıç dayatılmaz (kendi kurar).
    Gelistirici,
}

impl Rol {
    /// Diyalogda gösterilecek tüm roller (sıra sabit).
    pub const TUMU: &'static [Rol] = &[Rol::Ogrenci, Rol::Arastirmaci, Rol::Gelistirici];

    /// Rol ikonu (renk view'da token'dan).
    pub fn ikon(&self) -> &'static str {
        match self {
            Rol::Ogrenci => "🎓",
            Rol::Arastirmaci => "🔬",
            Rol::Gelistirici => "🛠",
        }
    }

    /// Dile göre kısa ad.
    pub fn ad(&self, tr: bool) -> &'static str {
        match self {
            Rol::Ogrenci => metin(tr, "Öğrenci", "Student"),
            Rol::Arastirmaci => metin(tr, "Araştırmacı", "Researcher"),
            Rol::Gelistirici => metin(tr, "Geliştirici", "Developer"),
        }
    }

    /// Dile göre tek cümlelik açıklama (ne uyarlanır).
    pub fn aciklama(&self, tr: bool) -> &'static str {
        match self {
            Rol::Ogrenci => metin(
                tr,
                "Adım adım rehber + kavram ipuçları (BAM/VCF nedir) öne çıkar.",
                "Step-by-step guidance + concept tips (what is BAM/VCF) come forward.",
            ),
            Rol::Arastirmaci => metin(
                tr,
                "Varyant/analiz akışları ve veri panelleri öne çıkar.",
                "Variant/analysis flows and data panels come forward.",
            ),
            Rol::Gelistirici => metin(
                tr,
                "Kod ve node editörü öne çıkar; hiçbir şey dayatılmaz, kendiniz kurarsınız.",
                "Code and node editors come forward; nothing is forced, you set it up yourself.",
            ),
        }
    }

    /// Bu role **önerilen** başlangıç şablonu (yalnızca varsayılan; kullanıcı değiştirebilir).
    pub fn onerilen_sablon(&self) -> OnboardingSablon {
        match self {
            // En görsel/anlaşılır giriş.
            Rol::Ogrenci => OnboardingSablon::GenomGorsel,
            // Tipik araştırma görevi.
            Rol::Arastirmaci => OnboardingSablon::VaryantInceleme,
            // Deneyimliye dayatma yok: boş başlasın, kendi akışını kursun.
            Rol::Gelistirici => OnboardingSablon::Bos,
        }
    }

    /// Kalıcılık için kararlı anahtar (serde'de saklanır).
    pub fn kod(&self) -> &'static str {
        match self {
            Rol::Ogrenci => "ogrenci",
            Rol::Arastirmaci => "arastirmaci",
            Rol::Gelistirici => "gelistirici",
        }
    }

    /// Kalıcı anahtardan rolü çözer (tanınmıyorsa `None`).
    pub fn koddan(s: &str) -> Option<Rol> {
        match s {
            "ogrenci" => Some(Rol::Ogrenci),
            "arastirmaci" => Some(Rol::Arastirmaci),
            "gelistirici" => Some(Rol::Gelistirici),
            _ => None,
        }
    }
}

/// "Rolün?" diyaloğunun bir karedeki sonucu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RolEylem {
    /// Bir rol seçildi (kaydet + öneriyi uygula).
    Sec(Rol),
    /// Kullanıcı "Atla" dedi (dayatma yok; yine kaydedilir → tekrar sorulmaz).
    Atla,
}

/// "Rolün?" diyaloğunu (modal) çizer; kullanıcı seçim/atlama yaptıysa döndürür.
///
/// Atlanabilir (sağ üstte ✕ + altta "Atla").  Hiçbir seçenek varsayılan-vurgulu değildir
/// (yönlendirme yok); kart tıklanınca seçim olur.
pub fn rol_dialog_ciz(ctx: &egui::Context, dil: Dil, tok: &Tokenlar) -> Option<RolEylem> {
    let tr = matches!(dil, Dil::Tr);
    let mut eylem: Option<RolEylem> = None;

    // Arkayı karartan örtü (modal hissi; panellerin üstünü de kapatır, pencerenin altında kalır).
    egui::Area::new(egui::Id::new("onboarding_rol_ortu"))
        .order(egui::Order::Middle)
        .fixed_pos(egui::Pos2::ZERO)
        .show(ctx, |ui| {
            let ekran = ctx.screen_rect();
            ui.painter()
                .rect_filled(ekran, 0.0, egui::Color32::from_black_alpha(160));
            ui.allocate_rect(ekran, egui::Sense::hover());
        });

    egui::Window::new("onboarding_rol")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(egui::Frame {
            fill: tok.renk.yuzey,
            stroke: egui::Stroke::new(1.0, tok.renk.kenarlik),
            rounding: egui::Rounding::same(tok.yaricap),
            inner_margin: egui::Margin::same(tok.bosluk.l),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.set_max_width(560.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(metin(tr, "Hoş geldiniz! 👋", "Welcome! 👋"))
                        .size(22.0)
                        .strong()
                        .color(tok.renk.vurgu),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // ✕ = atla (dayatma yok).
                    if ui
                        .button(egui::RichText::new("✕").color(tok.renk.metin_soluk))
                        .on_hover_text(ceviri(dil, Anahtar::Kapat))
                        .clicked()
                    {
                        eylem = Some(RolEylem::Atla);
                    }
                });
            });
            ui.label(
                egui::RichText::new(metin(
                    tr,
                    "Rolünüz nedir? Bunu deneyiminizi yumuşatmak için soruyoruz; \
                     dilediğiniz an değiştirebilir veya atlayabilirsiniz.",
                    "What is your role? We ask only to tailor your experience; \
                     you can change it anytime or skip.",
                ))
                .color(tok.renk.metin_soluk),
            );
            ui.add_space(tok.bosluk.m);

            for &r in Rol::TUMU {
                let cerceve = egui::Frame {
                    fill: tok.renk.yuzey_alt,
                    stroke: egui::Stroke::new(1.0, tok.renk.kenarlik),
                    rounding: egui::Rounding::same(tok.yaricap),
                    inner_margin: egui::Margin::same(tok.bosluk.m),
                    ..Default::default()
                };
                let yanit = cerceve
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(r.ikon()).size(28.0));
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(r.ad(tr)).strong().color(tok.renk.metin),
                                );
                                ui.label(
                                    egui::RichText::new(r.aciklama(tr))
                                        .small()
                                        .color(tok.renk.metin_soluk),
                                );
                            });
                        });
                    })
                    .response
                    .interact(egui::Sense::click());
                if yanit.hovered() {
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                }
                if yanit.clicked() {
                    eylem = Some(RolEylem::Sec(r));
                }
                ui.add_space(tok.bosluk.s);
            }

            ui.add_space(tok.bosluk.xs);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(metin(tr, "Şimdilik atla", "Skip for now"))
                        .clicked()
                    {
                        eylem = Some(RolEylem::Atla);
                    }
                });
            });
        });

    // Esc → atla (güvenli/özerk varsayılan; sihirbazla aynı refleks).
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        eylem = eylem.or(Some(RolEylem::Atla));
    }
    eylem
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tum_roller_iki_dilde_dolu_ve_farkli() {
        for &r in Rol::TUMU {
            assert!(!r.ad(true).is_empty() && !r.ad(false).is_empty());
            assert!(!r.aciklama(true).is_empty() && !r.aciklama(false).is_empty());
            assert_ne!(r.ad(true), r.ad(false), "rol adı çevrilmemiş: {r:?}");
            assert_ne!(r.aciklama(true), r.aciklama(false));
            assert!(!r.ikon().is_empty());
        }
    }

    #[test]
    fn rol_kod_gidis_donus() {
        for &r in Rol::TUMU {
            assert_eq!(Rol::koddan(r.kod()), Some(r));
        }
        assert_eq!(Rol::koddan("bilinmeyen"), None);
    }

    #[test]
    fn her_rol_bir_sablon_onerir() {
        // Geliştiriciye dayatma yok: boş şablon önerilmeli (kendi kurar).
        assert_eq!(Rol::Gelistirici.onerilen_sablon(), OnboardingSablon::Bos);
        // Öğrenci/araştırmacıya somut, çalışan bir başlangıç önerilir.
        assert_ne!(Rol::Ogrenci.onerilen_sablon(), OnboardingSablon::Bos);
        assert_ne!(Rol::Arastirmaci.onerilen_sablon(), OnboardingSablon::Bos);
    }

    #[test]
    fn rol_dialog_headless_cizilir() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |c| {
            let _ = rol_dialog_ciz(c, Dil::Tr, &Tokenlar::koyu());
        });
    }
}
