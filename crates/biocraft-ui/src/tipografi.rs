//! Tipografi — **egui adaptörü** (İP-04 / Bölüm 0.8).
//!
//! Render katmanındaki saf [`Tipografi`](biocraft_render::tipografi::Tipografi) sistemini
//! (rol→boyut + DPI ölçeği) egui metin stillerine bağlar ve açık-lisanslı fontları (Inter /
//! JetBrains Mono / Space Grotesk) kurar.  `.ttf` yoksa egui'nin gömülü açık fontlarına düşülür
//! ama bu **sessizce değil** [`FontDurumu`] ile bildirilir (TDA madde 1).

use biocraft_render::tipografi::{FontRol, Tipografi};
use egui::{FontFamily, FontId, TextStyle};

/// Başlık (Space Grotesk) için kullanılan özel egui font ailesi adı.
const BASLIK_AILE: &str = "biocraft-baslik";

/// Hangi rollerin gerçek gömülü fontu yüklendiği (eksikse egui varsayılanına düşülür).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FontDurumu {
    /// Gövde (Inter) gerçek dosyadan yüklendi mi.
    pub govde: bool,
    /// Kod (JetBrains Mono) gerçek dosyadan yüklendi mi.
    pub kod: bool,
    /// Başlık (Space Grotesk) gerçek dosyadan yüklendi mi.
    pub baslik: bool,
}

impl FontDurumu {
    /// Üç ailenin de gerçek dosyadan yüklenip yüklenmediği.
    pub fn hepsi_gomulu(&self) -> bool {
        self.govde && self.kod && self.baslik
    }

    /// En az bir aile egui varsayılan fontuna mı düştü.
    pub fn eksik_var(&self) -> bool {
        !self.hepsi_gomulu()
    }
}

/// Sağlanan font baytlarını egui'ye kurar; sağlanmayan roller egui varsayılanına düşer.
///
/// Roller (bkz. [`FontRol`]): Gövde = Inter (Proportional), Kod = JetBrains Mono (Monospace),
/// Başlık = Space Grotesk (özel "biocraft-baslik" ailesi → Heading stili bunu kullanır).
/// Başlık ailesi font yoksa bile **her zaman** kayıtlıdır (Proportional'a düşer) → `Heading`
/// stili güvenle bu aileyi kullanabilir.
pub fn fontlari_yukle(
    ctx: &egui::Context,
    inter: Option<Vec<u8>>,
    mono: Option<Vec<u8>>,
    baslik: Option<Vec<u8>>,
) -> FontDurumu {
    let mut defs = egui::FontDefinitions::default();
    let mut durum = FontDurumu::default();

    if let Some(bytes) = inter {
        defs.font_data
            .insert("Inter".into(), egui::FontData::from_owned(bytes));
        defs.families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "Inter".into());
        durum.govde = true;
    }
    if let Some(bytes) = mono {
        defs.font_data
            .insert("JetBrainsMono".into(), egui::FontData::from_owned(bytes));
        defs.families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "JetBrainsMono".into());
        durum.kod = true;
    }

    // Başlık ailesini her durumda kaydet: Space Grotesk varsa onu öne al, yoksa Proportional'a düş.
    let mut baslik_ailesi: Vec<String> = Vec::new();
    if let Some(bytes) = baslik {
        defs.font_data
            .insert("SpaceGrotesk".into(), egui::FontData::from_owned(bytes));
        baslik_ailesi.push("SpaceGrotesk".into());
        durum.baslik = true;
    }
    // Yedek olarak mevcut Proportional ailesini ekle (font eksikse başlık yine de okunur).
    if let Some(prop) = defs.families.get(&FontFamily::Proportional) {
        baslik_ailesi.extend(prop.iter().cloned());
    }
    defs.families
        .insert(FontFamily::Name(BASLIK_AILE.into()), baslik_ailesi);

    ctx.set_fonts(defs);
    durum
}

/// Render [`Tipografi`]'sinden egui metin stillerini (boyut + DPI ölçeği) uygular.
///
/// `Heading` → Space Grotesk ailesi; `Body`/`Button`/`Small` → Inter (Proportional);
/// `Monospace` → JetBrains Mono.  Boyutlar `t.olcek` (winit `scale_factor`) ile çarpılır
/// (4K + çoklu monitör akıcılığı).
pub fn metin_stilleri(ctx: &egui::Context, t: &Tipografi) {
    let mut style = (*ctx.style()).clone();
    let baslik_aile = FontFamily::Name(BASLIK_AILE.into());
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(t.display_efektif(), baslik_aile),
        ),
        (
            TextStyle::Body,
            FontId::new(t.efektif(FontRol::Govde), FontFamily::Proportional),
        ),
        (
            TextStyle::Button,
            FontId::new(t.efektif(FontRol::Govde), FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(t.efektif(FontRol::Kod), FontFamily::Monospace),
        ),
        (
            TextStyle::Small,
            FontId::new(t.kucuk_efektif(), FontFamily::Proportional),
        ),
    ]
    .into();
    ctx.set_style(style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_durumu_eksik_tespiti() {
        let bos = FontDurumu::default();
        assert!(bos.eksik_var());
        assert!(!bos.hepsi_gomulu());
        let tam = FontDurumu {
            govde: true,
            kod: true,
            baslik: true,
        };
        assert!(tam.hepsi_gomulu());
    }

    #[test]
    fn fontsuz_yukleme_varsayilana_duser_ama_baslik_ailesi_kayitli() {
        // Hiç .ttf verilmese bile başlık ailesi kayıtlı olmalı (metin_stilleri panik atmamalı).
        let ctx = egui::Context::default();
        let durum = fontlari_yukle(&ctx, None, None, None);
        assert!(durum.eksik_var());
        // Boyutları uygula + bir kare çiz → panik olmamalı (başlık ailesi çözülür).
        let t = Tipografi::varsayilan();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            metin_stilleri(ctx, &t);
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Başlık");
                ui.label("Gövde");
            });
        });
    }

    #[test]
    fn dpi_olcekli_boyut_uygulanir() {
        let ctx = egui::Context::default();
        fontlari_yukle(&ctx, None, None, None);
        let t = Tipografi::varsayilan().olcekle(2.0);
        metin_stilleri(&ctx, &t);
        let stil = ctx.style();
        let govde = &stil.text_styles[&TextStyle::Body];
        assert_eq!(govde.size, 28.0); // 14 × 2.0
    }
}
