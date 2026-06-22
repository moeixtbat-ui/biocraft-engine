//! Hoş geldin / etkileşimli tanıtım turu (İP-17).
//!
//! Kısa (birkaç adım), **her adımı atlanabilir** bir tur: arayüzü tanıtır (6 bölge, proje açma,
//! paneller, komut paleti, AI yüzeyi).  Sıkıcı uzun metin değil; kısa kart + işaret.  Tur **tamamen
//! kapatılabilir** ("Atla") ve sonra **"Yardım > Tur"dan yeniden açılabilir** (durum sıfırlanabilir).
//!
//! Saf durum makinesi ([`TurDurumu`]) + egui örtü adaptörü ([`tur_ciz`]); tüm metin TR/EN (MK-53).

use crate::i18n::{ceviri, Anahtar, Dil};
use crate::tokens::Tokenlar;

use super::metin;

/// Turun adımları (sıra sabit).  Son adım [`TurAdim::Bitis`] kapanış/teşekkür.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurAdim {
    /// Hoş geldin + turun ne olduğu.
    HosGeldin,
    /// 6 bölgeli kabuk (Başlık/Activity/Yan/Editör/Alt/Durum).
    AltiBolge,
    /// Proje açma (launcher → yeni/örnek proje).
    ProjeAcma,
    /// Paneller (yan/alt/inspector açma-kapama).
    Paneller,
    /// Komut paleti (Ctrl+Shift+P — her şeye hızlı erişim).
    KomutPaleti,
    /// AI yüzeyi (yapılandırılmadı; dürüst — sahte değil).
    AiYuzeyi,
    /// Kapanış: artık hazırsınız.
    Bitis,
}

impl TurAdim {
    /// Tüm adımlar (sıra sabit).
    pub const TUMU: &'static [TurAdim] = &[
        TurAdim::HosGeldin,
        TurAdim::AltiBolge,
        TurAdim::ProjeAcma,
        TurAdim::Paneller,
        TurAdim::KomutPaleti,
        TurAdim::AiYuzeyi,
        TurAdim::Bitis,
    ];

    /// Adım sayısı.
    pub fn toplam() -> usize {
        Self::TUMU.len()
    }

    /// Bu adımın sıra indeksi.
    pub fn indeks(self) -> usize {
        Self::TUMU.iter().position(|a| *a == self).unwrap_or(0)
    }

    /// Adım ikonu.
    pub fn ikon(self) -> &'static str {
        match self {
            TurAdim::HosGeldin => "👋",
            TurAdim::AltiBolge => "🗂",
            TurAdim::ProjeAcma => "📂",
            TurAdim::Paneller => "🧰",
            TurAdim::KomutPaleti => "⌨",
            TurAdim::AiYuzeyi => "✨",
            TurAdim::Bitis => "🚀",
        }
    }

    /// Adımın kısa başlığı.
    pub fn baslik(self, tr: bool) -> &'static str {
        match self {
            TurAdim::HosGeldin => metin(
                tr,
                "BioCraft Engine'e hoş geldiniz",
                "Welcome to BioCraft Engine",
            ),
            TurAdim::AltiBolge => metin(
                tr,
                "Arayüzün altı bölgesi",
                "The six regions of the interface",
            ),
            TurAdim::ProjeAcma => metin(tr, "Proje açma", "Opening a project"),
            TurAdim::Paneller => metin(tr, "Paneller", "Panels"),
            TurAdim::KomutPaleti => metin(tr, "Komut paleti", "Command palette"),
            TurAdim::AiYuzeyi => metin(tr, "AI yüzeyi", "AI surface"),
            TurAdim::Bitis => metin(tr, "Hazırsınız! 🚀", "You're ready! 🚀"),
        }
    }

    /// Adımın kısa açıklaması (uzun metin değil — bir-iki cümle).
    pub fn aciklama(self, tr: bool) -> &'static str {
        match self {
            TurAdim::HosGeldin => metin(
                tr,
                "Bu kısa tur arayüzü tanıtır. İstediğiniz an \"Atla\" diyebilir, sonra \
                 \"Yardım > Tur\"dan tekrar açabilirsiniz.",
                "This short tour shows you around. You can \"Skip\" anytime and reopen it later \
                 from \"Help > Tour\".",
            ),
            TurAdim::AltiBolge => metin(
                tr,
                "Üstte başlık/menü, solda araç çubuğu ve yan panel, ortada editör, altta konsol, \
                 en altta durum çubuğu. Sağda inspector.",
                "Title/menu on top, activity bar and side panel on the left, editor in the center, \
                 console at the bottom, status bar at the very bottom. Inspector on the right.",
            ),
            TurAdim::ProjeAcma => metin(
                tr,
                "Açılış ekranından \"Yeni Proje\" ile sihirbazı başlatın ya da \"Demo Projeyi Aç\" \
                 ile örnek veriyle dolu bir projeyi hemen görün.",
                "From the launcher use \"New Project\" to start the wizard, or \"Open Demo Project\" \
                 to instantly see a project filled with sample data.",
            ),
            TurAdim::Paneller => metin(
                tr,
                "Yan panel, alt panel ve inspector'ı Görünüm menüsünden açıp kapatabilirsiniz; \
                 düzeninizi kaydedebilirsiniz.",
                "Toggle the side panel, bottom panel and inspector from the View menu; you can save \
                 your layout.",
            ),
            TurAdim::KomutPaleti => metin(
                tr,
                "Ctrl+Shift+P ile komut paletini açın: her komuta yazarak hızla ulaşırsınız.",
                "Press Ctrl+Shift+P to open the command palette: type to reach any command fast.",
            ),
            TurAdim::AiYuzeyi => metin(
                tr,
                "Alt paneldeki AI sekmesi bir yardımcı yüzeyidir. Şu an \"yapılandırılmadı\" — \
                 gerçek motor eklenti olarak gelir; sonuçlar her zaman doğrulanmalıdır.",
                "The AI tab in the bottom panel is an assistant surface. It is currently \
                 \"not configured\" — the real engine ships as a plugin; results must always be verified.",
            ),
            TurAdim::Bitis => metin(
                tr,
                "İşte bu kadar! İpuçları gerektiğinde belirir; her şeyi \"Yardım\" menüsünden \
                 yeniden bulabilirsiniz. İyi çalışmalar!",
                "That's it! Tips appear when relevant; you can find everything again under \"Help\". \
                 Happy building!",
            ),
        }
    }
}

/// Turun durumu (host bunu kalıcı onboarding durumunda tutar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TurDurumu {
    /// Tur şu an görünür mü.
    pub aktif: bool,
    /// Aktif adım indeksi.
    pub adim: usize,
    /// Tur en az bir kez tamamlandı/atlandı mı (ilk açılış tespitinde kullanılır).
    pub tamamlandi: bool,
}

impl TurDurumu {
    /// Turu baştan başlatır (ilk açılış veya "Yardım > Tur" — durum sıfırlanabilir).
    pub fn baslat(&mut self) {
        self.aktif = true;
        self.adim = 0;
    }

    /// Aktif adım.
    pub fn aktif_adim(&self) -> TurAdim {
        TurAdim::TUMU
            .get(self.adim)
            .copied()
            .unwrap_or(TurAdim::HosGeldin)
    }

    /// Son adımda mıyız.
    pub fn son_adim_mi(&self) -> bool {
        self.adim + 1 >= TurAdim::toplam()
    }

    /// Bir sonraki adıma geçer; sondaysa turu bitirir (tamamlandı işaretler).
    pub fn ileri(&mut self) {
        if self.son_adim_mi() {
            self.bitir();
        } else {
            self.adim += 1;
        }
    }

    /// Bir önceki adıma döner (ilk adımda kalır).
    pub fn geri(&mut self) {
        self.adim = self.adim.saturating_sub(1);
    }

    /// Turu kapatır ve tamamlandı işaretler (atla = tamamlandı sayılır; tekrar sorulmaz).
    pub fn bitir(&mut self) {
        self.aktif = false;
        self.tamamlandi = true;
    }
}

/// Tur kartını (örtülü modal) çizer; durum bu fonksiyon içinde güncellenir.
/// Bir adım/aksiyon değiştiyse `true` döner (host kalıcılığı kirletmek için kullanır).
pub fn tur_ciz(ctx: &egui::Context, durum: &mut TurDurumu, dil: Dil, tok: &Tokenlar) -> bool {
    if !durum.aktif {
        return false;
    }
    let tr = matches!(dil, Dil::Tr);
    let adim = durum.aktif_adim();
    let mut degisti = false;

    // Hafif karartma örtüsü (turu öne çıkarır; akıcılığı bozmaz).
    egui::Area::new(egui::Id::new("onboarding_tur_ortu"))
        .order(egui::Order::Middle)
        .fixed_pos(egui::Pos2::ZERO)
        .show(ctx, |ui| {
            let ekran = ctx.screen_rect();
            ui.painter()
                .rect_filled(ekran, 0.0, egui::Color32::from_black_alpha(120));
            ui.allocate_rect(ekran, egui::Sense::hover());
        });

    egui::Window::new("onboarding_tur")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -64.0))
        .frame(egui::Frame {
            fill: tok.renk.yuzey,
            stroke: egui::Stroke::new(1.0, tok.renk.vurgu),
            rounding: egui::Rounding::same(tok.yaricap),
            inner_margin: egui::Margin::same(tok.bosluk.l),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.set_max_width(520.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(adim.ikon()).size(28.0));
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(adim.baslik(tr))
                            .size(18.0)
                            .strong()
                            .color(tok.renk.metin),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{} {}/{}",
                            metin(tr, "Adım", "Step"),
                            durum.adim + 1,
                            TurAdim::toplam(),
                        ))
                        .small()
                        .color(tok.renk.metin_soluk),
                    );
                });
            });
            ui.add_space(tok.bosluk.s);
            ui.label(egui::RichText::new(adim.aciklama(tr)).color(tok.renk.metin));
            ui.add_space(tok.bosluk.m);

            // İlerleme noktaları.
            ui.horizontal(|ui| {
                for (i, _) in TurAdim::TUMU.iter().enumerate() {
                    let renk = if i == durum.adim {
                        tok.renk.vurgu
                    } else if i < durum.adim {
                        tok.renk.basari
                    } else {
                        tok.renk.kenarlik
                    };
                    ui.label(egui::RichText::new("●").small().color(renk));
                }
            });
            ui.add_space(tok.bosluk.s);

            ui.horizontal(|ui| {
                // Atla (her adımda; turu tamamen kapatır — özerklik).
                if ui
                    .button(metin(tr, "Atla", "Skip"))
                    .on_hover_text(metin(
                        tr,
                        "Turu kapat (Yardım > Tur'dan tekrar açılır)",
                        "Close the tour (reopen from Help > Tour)",
                    ))
                    .clicked()
                {
                    durum.bitir();
                    degisti = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // İleri / Bitir.
                    let ileri_etiket = if durum.son_adim_mi() {
                        metin(tr, "Bitir", "Finish")
                    } else {
                        ceviri(dil, Anahtar::Ileri)
                    };
                    let buton = egui::Button::new(
                        egui::RichText::new(ileri_etiket)
                            .color(tok.renk.vurgu_ustu)
                            .strong(),
                    )
                    .fill(tok.renk.vurgu);
                    if ui.add(buton).clicked() {
                        durum.ileri();
                        degisti = true;
                    }
                    // Geri (ilk adımda pasif).
                    if ui
                        .add_enabled(
                            durum.adim > 0,
                            egui::Button::new(ceviri(dil, Anahtar::Geri)),
                        )
                        .clicked()
                    {
                        durum.geri();
                        degisti = true;
                    }
                });
            });
        });

    // Esc → atla (turu kapatır; özerk varsayılan).
    if durum.aktif && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        durum.bitir();
        degisti = true;
    }
    degisti
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tum_adimlar_iki_dilde_dolu_ve_farkli() {
        for &a in TurAdim::TUMU {
            assert!(!a.baslik(true).is_empty() && !a.baslik(false).is_empty());
            assert!(!a.aciklama(true).is_empty() && !a.aciklama(false).is_empty());
            assert_ne!(a.baslik(true), a.baslik(false), "başlık çevrilmemiş: {a:?}");
            assert_ne!(a.aciklama(true), a.aciklama(false));
            assert!(!a.ikon().is_empty());
        }
    }

    #[test]
    fn tur_ilerler_ve_biter() {
        let mut t = TurDurumu::default();
        t.baslat();
        assert!(t.aktif);
        assert_eq!(t.adim, 0);
        // Sonuna kadar ilerle.
        for _ in 0..TurAdim::toplam() {
            t.ileri();
        }
        // Son adımda "ileri" → bitir.
        assert!(!t.aktif, "tur sonunda kapanmalı");
        assert!(t.tamamlandi, "tur tamamlandı işaretlenmeli");
    }

    #[test]
    fn tur_atlanabilir_ve_tekrar_acilabilir() {
        let mut t = TurDurumu::default();
        t.baslat();
        t.bitir(); // Atla
        assert!(!t.aktif && t.tamamlandi);
        // "Yardım > Tur" → yeniden baştan açılır (durum sıfırlanabilir).
        t.baslat();
        assert!(t.aktif);
        assert_eq!(t.adim, 0);
    }

    #[test]
    fn geri_ilk_adimda_kalir() {
        let mut t = TurDurumu::default();
        t.baslat();
        t.geri();
        assert_eq!(t.adim, 0);
    }

    #[test]
    fn tur_headless_cizilir() {
        let ctx = egui::Context::default();
        let mut t = TurDurumu::default();
        t.baslat();
        let _ = ctx.run(egui::RawInput::default(), |c| {
            let _ = tur_ciz(c, &mut t, Dil::Tr, &Tokenlar::koyu());
        });
    }
}
