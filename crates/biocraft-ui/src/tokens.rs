//! Tasarım token'ları — **egui adaptörü** (MK-52, MK-53, TDA madde 14).
//!
//! Renklerin tek doğruluk kaynağı `biocraft-render`'daki [`biocraft_render::tokens`]
//! modülü + `assets/tokens.json`'dır.  **Bu dosyada sabit renk YOKTUR**; burada yalnızca
//! render'ın saf [`Renk`](biocraft_render::tokens::Renk) tipleri egui [`Color32`]'ye çevrilir
//! ve TDA bileşenlerinin beklediği anlamsal alanlara ([`Renkler`]) yerleştirilir.
//!
//! Böylece "tema değişince renkler token'dan mı geliyor?" güvencesi tek noktada sağlanır ve
//! aynı palet hem 2B (egui) hem 3B (wgpu) tarafında ortak kullanılır.
// MK-52: tüm renkler token'dan; bu adaptör yalnızca tip dönüşümü yapar, renk üretmez.

use std::sync::OnceLock;

use biocraft_render::tokens::{Palet, Renk, Tema as RenderTema, TokenDeposu, TokenSeti};
use egui::Color32;

use crate::i18n::Dil;

/// Gömülü token deposu yalnızca bir kez ayrıştırılır (her karede JSON çözmek yerine önbellek).
fn depo() -> &'static TokenDeposu {
    static D: OnceLock<TokenDeposu> = OnceLock::new();
    D.get_or_init(TokenDeposu::gomulu)
}

/// Saf [`Renk`]'i egui [`Color32`]'ye çevirir.
fn renk32(r: Renk) -> Color32 {
    Color32::from_rgba_unmultiplied(r.r, r.g, r.b, r.a)
}

/// Arayüzde seçilebilen yerleşik tema (Koyu/Açık/Yüksek-kontrast arası döngü).
///
/// Özel (kullanıcı) temaları render [`TokenDeposu`] üzerinden eklenir (E2); bu enum yalnızca
/// üç yerleşik temanın hızlı geçişi içindir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tema {
    /// Koyu (varsayılan).
    #[default]
    Koyu,
    /// Açık.
    Acik,
    /// Yüksek kontrast (erişilebilirlik).
    YuksekKontrast,
}

impl Tema {
    /// Döngüsel sonraki tema (tema değiştirici butonu).
    pub fn sonraki(self) -> Self {
        match self {
            Tema::Koyu => Tema::Acik,
            Tema::Acik => Tema::YuksekKontrast,
            Tema::YuksekKontrast => Tema::Koyu,
        }
    }

    /// Render katmanındaki karşılığı.
    fn render(self) -> RenderTema {
        match self {
            Tema::Koyu => RenderTema::Koyu,
            Tema::Acik => RenderTema::Acik,
            Tema::YuksekKontrast => RenderTema::YuksekKontrast,
        }
    }

    /// Bu tema koyu taban mı (egui temel görünümü için).
    pub fn koyu_mu(self) -> bool {
        depo()
            .set(&self.render())
            .map(|s| s.koyu_mu)
            .unwrap_or(true)
    }

    /// Tema değiştirici butonunun etiketi (bir sonraki temaya geçişi anlatır).
    pub fn dugme_etiketi(self, dil: Dil) -> &'static str {
        match (self, dil) {
            (Tema::Koyu, Dil::Tr) => "🌙 Koyu → Açık",
            (Tema::Koyu, Dil::En) => "🌙 Dark → Light",
            (Tema::Acik, Dil::Tr) => "☀ Açık → Yüksek Kontrast",
            (Tema::Acik, Dil::En) => "☀ Light → High Contrast",
            (Tema::YuksekKontrast, Dil::Tr) => "◑ Yüksek Kontrast → Koyu",
            (Tema::YuksekKontrast, Dil::En) => "◑ High Contrast → Dark",
        }
    }
}

/// Bir bileşenin anlam/önem sınıfı; somut renk eşlemesi token'dan gelir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Onem {
    /// Başarı / olumlu.
    Basari,
    /// Uyarı / dikkat.
    Uyari,
    /// Hata / yıkıcı.
    Hata,
    /// Bilgi / nötr-bilgi.
    Bilgi,
    /// Nötr (vurgusuz).
    Notr,
}

/// Anlamsal renk paleti (egui `Color32`).  Tüm değerler token'dan türetilir; sabit renk yoktur.
#[derive(Debug, Clone, Copy)]
pub struct Renkler {
    /// Başarı vurgu rengi.
    pub basari: Color32,
    /// Başarı arka plan rengi.
    pub basari_zemin: Color32,
    /// Uyarı vurgu rengi.
    pub uyari: Color32,
    /// Uyarı arka plan rengi.
    pub uyari_zemin: Color32,
    /// Hata vurgu rengi.
    pub hata: Color32,
    /// Hata arka plan rengi.
    pub hata_zemin: Color32,
    /// Bilgi vurgu rengi.
    pub bilgi: Color32,
    /// Bilgi arka plan rengi.
    pub bilgi_zemin: Color32,
    /// Birincil eylem/marka vurgu rengi (accent).
    pub vurgu: Color32,
    /// Vurgu (accent) dolgu üstünde okunan metin rengi.
    pub vurgu_ustu: Color32,
    /// Hata (danger) dolgu üstünde okunan metin rengi.
    pub hata_ustu: Color32,
    /// Panel/kart yüzey rengi.
    pub yuzey: Color32,
    /// İkincil yüzey rengi.
    pub yuzey_alt: Color32,
    /// Kenarlık rengi.
    pub kenarlik: Color32,
    /// Birincil metin rengi.
    pub metin: Color32,
    /// İkincil/soluk metin rengi.
    pub metin_soluk: Color32,
    /// Yükleme iskeleti dolgu rengi.
    pub iskelet: Color32,
    /// Ana pencere/uygulama zemini (bg.primary).
    pub zemin: Color32,
    /// İkincil zemin (bg.secondary).
    pub zemin_alt: Color32,
}

impl Renkler {
    /// Render paletinden egui renk paletini üretir (yalnızca tip dönüşümü).
    fn paletten(p: &Palet) -> Self {
        Self {
            basari: renk32(p.renk("success")),
            basari_zemin: renk32(p.renk("success.bg")),
            uyari: renk32(p.renk("warning")),
            uyari_zemin: renk32(p.renk("warning.bg")),
            hata: renk32(p.renk("error")),
            hata_zemin: renk32(p.renk("error.bg")),
            bilgi: renk32(p.renk("info")),
            bilgi_zemin: renk32(p.renk("info.bg")),
            vurgu: renk32(p.vurgu()),
            vurgu_ustu: renk32(p.vurgu_ustu()),
            hata_ustu: renk32(p.hata_ustu()),
            yuzey: renk32(p.yuzey()),
            yuzey_alt: renk32(p.yuzey_alt()),
            kenarlik: renk32(p.kenarlik()),
            metin: renk32(p.metin()),
            metin_soluk: renk32(p.metin_soluk()),
            iskelet: renk32(p.renk("skeleton")),
            zemin: renk32(p.zemin()),
            zemin_alt: renk32(p.zemin_alt()),
        }
    }
}

/// Boşluk ölçeği (4'ün katları) — tutarlı aralıklar için.  Renk değildir (MK-52 kapsamı dışı).
#[derive(Debug, Clone, Copy)]
pub struct Bosluk {
    /// 4 px.
    pub xs: f32,
    /// 8 px.
    pub s: f32,
    /// 12 px.
    pub m: f32,
    /// 16 px.
    pub l: f32,
    /// 24 px.
    pub xl: f32,
}

impl Bosluk {
    /// Standart boşluk ölçeği.
    pub const fn varsayilan() -> Self {
        Self {
            xs: 4.0,
            s: 8.0,
            m: 12.0,
            l: 16.0,
            xl: 24.0,
        }
    }
}

/// Tüm tasarım token'larının (egui tarafı) paketi.  Renkleri token deposundan alır.
#[derive(Debug, Clone, Copy)]
pub struct Tokenlar {
    /// Anlamsal renk paleti.
    pub renk: Renkler,
    /// Boşluk ölçeği.
    pub bosluk: Bosluk,
    /// Standart köşe yarıçapı (px).
    pub yaricap: f32,
    /// Bu token setinin koyu taban olup olmadığı.
    pub koyu: bool,
}

impl Tokenlar {
    /// Bir render token setinden egui token paketi üretir.
    pub fn setten(set: &TokenSeti) -> Self {
        Self {
            renk: Renkler::paletten(&set.palet),
            bosluk: Bosluk::varsayilan(),
            yaricap: 8.0,
            koyu: set.koyu_mu,
        }
    }

    /// Yerleşik [`Tema`] için token paketi.
    pub fn temalı(tema: Tema) -> Self {
        let render_tema = tema.render();
        let set = depo()
            .set(&render_tema)
            .expect("yerleşik tema token deposunda bulunmalı");
        Self::setten(set)
    }

    /// Açık tema token'ları.
    pub fn acik() -> Self {
        Self::temalı(Tema::Acik)
    }

    /// Koyu tema token'ları.
    pub fn koyu() -> Self {
        Self::temalı(Tema::Koyu)
    }

    /// Yüksek kontrast tema token'ları.
    pub fn yuksek_kontrast() -> Self {
        Self::temalı(Tema::YuksekKontrast)
    }

    /// Geriye-uyumluluk: koyu bayrağına göre Koyu/Açık seç.
    pub fn temadan(koyu: bool) -> Self {
        if koyu {
            Self::koyu()
        } else {
            Self::acik()
        }
    }

    /// egui bağlamının aktif (koyu/açık) temasından token üretir.
    pub fn ctx_ten(ctx: &egui::Context) -> Self {
        Self::temadan(ctx.style().visuals.dark_mode)
    }

    /// Önem sınıfının vurgu (metin/ikon/kenarlık) rengini döndürür.
    pub fn onem_rengi(&self, onem: Onem) -> Color32 {
        match onem {
            Onem::Basari => self.renk.basari,
            Onem::Uyari => self.renk.uyari,
            Onem::Hata => self.renk.hata,
            Onem::Bilgi => self.renk.bilgi,
            Onem::Notr => self.renk.metin_soluk,
        }
    }

    /// Anlamsal token anahtarını ("accent.primary", "info"…) egui rengine çözer.
    ///
    /// 2B plot serileri rengi token anahtarıyla taşır (MK-52); widget bu çözücüyü kullanır.
    /// Bilinmeyen anahtar birincil metin rengine düşer (görünür ama nötr).
    pub fn anahtar_renk(&self, anahtar: &str) -> Color32 {
        match anahtar {
            "accent.primary" => self.renk.vurgu,
            "success" | "success.bg" => self.renk.basari,
            "warning" | "warning.bg" => self.renk.uyari,
            "error" | "error.bg" => self.renk.hata,
            "info" | "info.bg" => self.renk.bilgi,
            "text.primary" => self.renk.metin,
            "text.muted" => self.renk.metin_soluk,
            "border" => self.renk.kenarlik,
            "surface" => self.renk.yuzey,
            "surface.alt" => self.renk.yuzey_alt,
            "bg.primary" => self.renk.zemin,
            "bg.secondary" => self.renk.zemin_alt,
            _ => self.renk.metin,
        }
    }

    /// Önem sınıfının arka plan (zemin) rengini döndürür.
    pub fn onem_zemini(&self, onem: Onem) -> Color32 {
        match onem {
            Onem::Basari => self.renk.basari_zemin,
            Onem::Uyari => self.renk.uyari_zemin,
            Onem::Hata => self.renk.hata_zemin,
            Onem::Bilgi => self.renk.bilgi_zemin,
            Onem::Notr => self.renk.yuzey_alt,
        }
    }

    /// Bütün egui arayüzünü token'lardan süren [`egui::Visuals`] üretir.
    ///
    /// Yalnızca TDA bileşenleri değil, tüm egui pencereleri/panelleri/widget'ları bu palet
    /// üzerinden renklenir → yüksek-kontrast teması gerçekten her yüzeye yansır (MK-52).
    pub fn egui_visuals(&self) -> egui::Visuals {
        let mut v = if self.koyu {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        v.override_text_color = Some(self.renk.metin);
        v.panel_fill = self.renk.zemin_alt;
        v.window_fill = self.renk.yuzey;
        v.window_stroke = egui::Stroke::new(1.0, self.renk.kenarlik);
        v.extreme_bg_color = self.renk.zemin;
        v.faint_bg_color = self.renk.yuzey_alt;
        v.hyperlink_color = self.renk.vurgu;
        v.selection.bg_fill = self.renk.vurgu.gamma_multiply(0.5);
        v.selection.stroke = egui::Stroke::new(1.0, self.renk.vurgu_ustu);

        v.widgets.noninteractive.bg_fill = self.renk.yuzey;
        v.widgets.noninteractive.weak_bg_fill = self.renk.yuzey;
        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, self.renk.kenarlik);
        v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, self.renk.metin_soluk);

        v.widgets.inactive.bg_fill = self.renk.yuzey_alt;
        v.widgets.inactive.weak_bg_fill = self.renk.yuzey_alt;
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, self.renk.kenarlik);
        v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, self.renk.metin);

        v.widgets.hovered.bg_fill = self.renk.vurgu.gamma_multiply(0.35);
        v.widgets.hovered.weak_bg_fill = self.renk.vurgu.gamma_multiply(0.35);
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, self.renk.vurgu);
        v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, self.renk.metin);

        v.widgets.active.bg_fill = self.renk.vurgu;
        v.widgets.active.weak_bg_fill = self.renk.vurgu;
        v.widgets.active.bg_stroke = egui::Stroke::new(1.0, self.renk.vurgu);
        v.widgets.active.fg_stroke = egui::Stroke::new(1.0, self.renk.vurgu_ustu);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renkler_token_deposundan_gelir() {
        // Çapa değer: koyu temada vurgu = accent.primary = #00E5FF (tokens.json).
        let t = Tokenlar::koyu();
        assert_eq!(t.renk.vurgu, Color32::from_rgb(0x00, 0xE5, 0xFF));
        // Açık temada metin = text.primary = #1A1A1A.
        let a = Tokenlar::acik();
        assert_eq!(a.renk.metin, Color32::from_rgb(0x1A, 0x1A, 0x1A));
    }

    #[test]
    fn uc_tema_farkli_zemin_verir() {
        let k = Tokenlar::koyu();
        let a = Tokenlar::acik();
        let yk = Tokenlar::yuksek_kontrast();
        assert_ne!(k.renk.zemin, a.renk.zemin);
        assert_ne!(a.renk.zemin, yk.renk.zemin);
        // Yüksek kontrast zemini saf siyah.
        assert_eq!(yk.renk.zemin, Color32::from_rgb(0, 0, 0));
        assert!(!a.koyu && k.koyu && yk.koyu);
    }

    #[test]
    fn tema_dongusu_uc_temayi_gezer() {
        assert_eq!(Tema::Koyu.sonraki(), Tema::Acik);
        assert_eq!(Tema::Acik.sonraki(), Tema::YuksekKontrast);
        assert_eq!(Tema::YuksekKontrast.sonraki(), Tema::Koyu);
    }

    #[test]
    fn onem_renkleri_token_paletinden_gelir() {
        let t = Tokenlar::acik();
        assert_eq!(t.onem_rengi(Onem::Hata), t.renk.hata);
        assert_eq!(t.onem_rengi(Onem::Basari), t.renk.basari);
        assert_eq!(t.onem_zemini(Onem::Bilgi), t.renk.bilgi_zemin);
    }

    #[test]
    fn egui_visuals_token_metin_rengini_kullanir() {
        let t = Tokenlar::koyu();
        let v = t.egui_visuals();
        assert_eq!(v.override_text_color, Some(t.renk.metin));
        assert_eq!(v.panel_fill, t.renk.zemin_alt);
    }
}
