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

    // ── Gün 31.2 Bölüm A — katmanlı (elevation) + reform renkleri ──────────────────────
    /// Viewport/tuval arka — en koyu zemin (`zemin.cukur`).
    pub zemin_cukur: Color32,
    /// 1. katman yüzey — panel (`yuzey.1`).
    pub yuzey1: Color32,
    /// 2. katman yüzey — yan panel / activity bar (`yuzey.2`).
    pub yuzey2: Color32,
    /// 3. katman yüzey — kart / girdi alanı (`yuzey.3`).
    pub yuzey3: Color32,
    /// 4. katman yüzey — hover (`yuzey.4`).
    pub yuzey4: Color32,
    /// Seçili satır dolgusu (`yuzey.secili`, yarı saydam vurgu).
    pub yuzey_secili: Color32,
    /// İnce ayraç kenarlığı (`kenar.ince`).
    pub kenar_ince: Color32,
    /// Odaklı girdinin belirgin kenarlığı (`kenar.belirgin`).
    pub kenar_belirgin: Color32,
    /// Sönük / üçüncül metin (`metin.sonuk`).
    pub metin_sonuk: Color32,
    /// Devre dışı metin (`metin.devredisi`).
    pub metin_devredisi: Color32,
    /// Vurgu hover varyantı (`vurgu.hover`).
    pub vurgu_hover: Color32,
    /// Vurgu aktif (basılı) varyantı (`vurgu.aktif`).
    pub vurgu_aktif: Color32,
    /// Vurgu sönük dolgu (`vurgu.zemin`).
    pub vurgu_zemin: Color32,
    /// Odak halkası rengi (`odak.halka`, erişilebilirlik).
    pub odak_halka: Color32,
    /// Metin seçimi dolgusu (`secim.zemin`).
    pub secim_zemin: Color32,
    /// Modal arka karartma (`ortu.scrim`).
    pub scrim: Color32,
    /// Gölge (elevation) rengi (`golge.renk`).
    pub golge: Color32,

    // ── Node port tipi renkleri (İP-05) ────────────────────────────────────────────────
    /// Sayı portu (`port.sayi`).
    pub port_sayi: Color32,
    /// Metin portu (`port.metin`).
    pub port_metin: Color32,
    /// Mantık portu (`port.mantik`).
    pub port_mantik: Color32,
    /// Dizi portu (`port.dizi`).
    pub port_dizi: Color32,
    /// Veri portu (`port.veri`).
    pub port_veri: Color32,

    // ── Kod söz dizimi renkleri (İP-06; VS Code Dark+ uyumlu) ───────────────────────────
    /// Anahtar kelime (`kod.anahtar`).
    pub kod_anahtar: Color32,
    /// Dize/string (`kod.dize`).
    pub kod_dize: Color32,
    /// Yorum (`kod.yorum`).
    pub kod_yorum: Color32,
    /// Sayı (`kod.sayi`).
    pub kod_sayi: Color32,
    /// Fonksiyon (`kod.fonksiyon`).
    pub kod_fonksiyon: Color32,
    /// Tip (`kod.tip`).
    pub kod_tip: Color32,
    /// Değişken (`kod.degisken`).
    pub kod_degisken: Color32,
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
            zemin_cukur: renk32(p.zemin_cukur()),
            yuzey1: renk32(p.yuzey1()),
            yuzey2: renk32(p.yuzey2()),
            yuzey3: renk32(p.yuzey3()),
            yuzey4: renk32(p.yuzey4()),
            yuzey_secili: renk32(p.yuzey_secili()),
            kenar_ince: renk32(p.kenar_ince()),
            kenar_belirgin: renk32(p.kenar_belirgin()),
            metin_sonuk: renk32(p.renk("metin.sonuk")),
            metin_devredisi: renk32(p.renk("metin.devredisi")),
            vurgu_hover: renk32(p.vurgu_hover()),
            vurgu_aktif: renk32(p.vurgu_aktif()),
            vurgu_zemin: renk32(p.vurgu_zemin()),
            odak_halka: renk32(p.odak_halka()),
            secim_zemin: renk32(p.renk("secim.zemin")),
            scrim: renk32(p.scrim()),
            golge: renk32(p.golge()),
            port_sayi: renk32(p.renk("port.sayi")),
            port_metin: renk32(p.renk("port.metin")),
            port_mantik: renk32(p.renk("port.mantik")),
            port_dizi: renk32(p.renk("port.dizi")),
            port_veri: renk32(p.renk("port.veri")),
            kod_anahtar: renk32(p.renk("kod.anahtar")),
            kod_dize: renk32(p.renk("kod.dize")),
            kod_yorum: renk32(p.renk("kod.yorum")),
            kod_sayi: renk32(p.renk("kod.sayi")),
            kod_fonksiyon: renk32(p.renk("kod.fonksiyon")),
            kod_tip: renk32(p.renk("kod.tip")),
            kod_degisken: renk32(p.renk("kod.degisken")),
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
    /// Boşluk ölçeği (geriye-uyumlu xs/s/m/l/xl).
    pub bosluk: Bosluk,
    /// Standart köşe yarıçapı (px) — geriye-uyumlu (kart/modal `r-md`).
    pub yaricap: f32,
    /// Gün 31.2 Bölüm A ölçü token'ları (boşluk/yarıçap/kenarlık/gölge/hareket/yerleşim/yoğunluk).
    pub olcu: biocraft_render::olcu::Olcu,
    /// Bu token setinin koyu taban olup olmadığı.
    pub koyu: bool,
}

impl Tokenlar {
    /// Bir render token setinden egui token paketi üretir.
    pub fn setten(set: &TokenSeti) -> Self {
        let olcu = biocraft_render::olcu::Olcu::varsayilan();
        Self {
            renk: Renkler::paletten(&set.palet),
            bosluk: Bosluk::varsayilan(),
            yaricap: olcu.yaricap.md,
            olcu,
            koyu: set.koyu_mu,
        }
    }

    /// Bir [`Golge`](biocraft_render::olcu::Golge) ölçüsünü egui gölgesine çevirir.
    ///
    /// egui'de gerçek blur yoktur; `golge.renk` token alfası ölçü `carpan`'ıyla ölçeklenir →
    /// kart/menü/modal için tutarlı yükselti hissi (MK-04: ucuz, kare bütçesini bozmaz).
    pub fn golge_shadow(&self, g: &biocraft_render::olcu::Golge) -> egui::epaint::Shadow {
        let temel = self.renk.golge;
        let alfa = (temel.a() as f32 * g.carpan).round().clamp(0.0, 255.0) as u8;
        egui::epaint::Shadow {
            offset: egui::vec2(g.dx, g.dy),
            blur: g.yayilma,
            spread: 0.0,
            color: renk32(Renk {
                r: temel.r(),
                g: temel.g(),
                b: temel.b(),
                a: alfa,
            }),
        }
    }

    /// Kart/menü (dropdown) için yükselti gölgesi (e2).
    pub fn golge_kart(&self) -> egui::epaint::Shadow {
        self.golge_shadow(&self.olcu.yukselti.e2)
    }

    /// Modal/drawer için yükselti gölgesi (e3).
    pub fn golge_modal(&self) -> egui::epaint::Shadow {
        self.golge_shadow(&self.olcu.yukselti.e3)
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
            // ── Geriye-uyumlu anlamsal anahtarlar ──────────────────────────────────────
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
            // ── Gün 31.2 Bölüm A — katmanlı yüzey / kenar / metin / vurgu ───────────────
            "zemin.cukur" => self.renk.zemin_cukur,
            "zemin.taban" => self.renk.zemin,
            "yuzey.1" => self.renk.yuzey1,
            "yuzey.2" => self.renk.yuzey2,
            "yuzey.3" => self.renk.yuzey3,
            "yuzey.4" => self.renk.yuzey4,
            "yuzey.secili" => self.renk.yuzey_secili,
            "kenar.ince" => self.renk.kenar_ince,
            "kenar.varsayilan" => self.renk.kenarlik,
            "kenar.belirgin" => self.renk.kenar_belirgin,
            "metin.birincil" => self.renk.metin,
            "metin.ikincil" => self.renk.metin_soluk,
            "metin.sonuk" => self.renk.metin_sonuk,
            "metin.devredisi" => self.renk.metin_devredisi,
            "vurgu.taban" => self.renk.vurgu,
            "vurgu.hover" => self.renk.vurgu_hover,
            "vurgu.aktif" => self.renk.vurgu_aktif,
            "vurgu.zemin" => self.renk.vurgu_zemin,
            "vurgu.uzeri_metin" => self.renk.vurgu_ustu,
            "odak.halka" => self.renk.odak_halka,
            // ── Durum renkleri ─────────────────────────────────────────────────────────
            "durum.basari" => self.renk.basari,
            "durum.uyari" => self.renk.uyari,
            "durum.hata" => self.renk.hata,
            "durum.bilgi" => self.renk.bilgi,
            "secim.zemin" => self.renk.secim_zemin,
            // ── Node port renkleri (İP-05; token'dan, her temada doğru) ─────────────────
            "port.sayi" => self.renk.port_sayi,
            "port.metin" => self.renk.port_metin,
            "port.mantik" => self.renk.port_mantik,
            "port.dizi" => self.renk.port_dizi,
            "port.veri" => self.renk.port_veri,
            // ── Kod söz dizimi renkleri (İP-06; VS Code Dark+ uyumlu) ───────────────────
            "kod.anahtar" => self.renk.kod_anahtar,
            "kod.dize" => self.renk.kod_dize,
            "kod.yorum" => self.renk.kod_yorum,
            "kod.sayi" => self.renk.kod_sayi,
            "kod.fonksiyon" => self.renk.kod_fonksiyon,
            "kod.tip" => self.renk.kod_tip,
            "kod.degisken" => self.renk.kod_degisken,
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
        let r = &self.renk;
        let ince = self.olcu.kenar.ince;
        let vurgu_k = self.olcu.kenar.vurgu;
        let yari_sm = egui::Rounding::same(self.olcu.yaricap.sm);
        let yari_md = egui::Rounding::same(self.olcu.yaricap.md);

        // ── Zemin / yüzey katmanları (UE5 yoğun elevation) ─────────────────────────────
        v.override_text_color = Some(r.metin);
        v.panel_fill = r.zemin; // kabuk arka (zemin.taban)
        v.window_fill = r.yuzey2; // yüzen pencere/menü tabanı
        v.window_stroke = egui::Stroke::new(ince, r.kenarlik);
        v.window_rounding = yari_md;
        v.menu_rounding = yari_md;
        v.extreme_bg_color = r.yuzey3; // girdi/metin alanı zemini (kart yüzeyi)
        v.faint_bg_color = r.yuzey1; // zebra / sönük dolgu
        v.hyperlink_color = r.vurgu;
        v.window_shadow = self.golge_modal();
        v.popup_shadow = self.golge_kart();

        // ── Seçim (metin) — token'dan, alfa dahil ──────────────────────────────────────
        v.selection.bg_fill = r.secim_zemin;
        v.selection.stroke = egui::Stroke::new(ince, r.vurgu);

        // ── Widget durum matrisi (noninteractive / inactive / hovered / active) ────────
        v.widgets.noninteractive.bg_fill = r.yuzey1;
        v.widgets.noninteractive.weak_bg_fill = r.yuzey1;
        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(ince, r.kenar_ince);
        v.widgets.noninteractive.fg_stroke = egui::Stroke::new(ince, r.metin_soluk);
        v.widgets.noninteractive.rounding = yari_sm;

        v.widgets.inactive.bg_fill = r.yuzey3;
        v.widgets.inactive.weak_bg_fill = r.yuzey2;
        v.widgets.inactive.bg_stroke = egui::Stroke::new(ince, r.kenarlik);
        v.widgets.inactive.fg_stroke = egui::Stroke::new(ince, r.metin);
        v.widgets.inactive.rounding = yari_sm;

        v.widgets.hovered.bg_fill = r.yuzey4;
        v.widgets.hovered.weak_bg_fill = r.yuzey4;
        v.widgets.hovered.bg_stroke = egui::Stroke::new(ince, r.kenar_belirgin);
        v.widgets.hovered.fg_stroke = egui::Stroke::new(ince, r.metin);
        v.widgets.hovered.rounding = yari_sm;

        v.widgets.active.bg_fill = r.vurgu;
        v.widgets.active.weak_bg_fill = r.vurgu_aktif;
        v.widgets.active.bg_stroke = egui::Stroke::new(vurgu_k, r.vurgu);
        v.widgets.active.fg_stroke = egui::Stroke::new(vurgu_k, r.vurgu_ustu);
        v.widgets.active.rounding = yari_sm;

        // Açık (combo/menü açık) durumu hover ile aynı yüzey, belirgin kenar.
        v.widgets.open.bg_fill = r.yuzey4;
        v.widgets.open.weak_bg_fill = r.yuzey3;
        v.widgets.open.bg_stroke = egui::Stroke::new(ince, r.kenar_belirgin);
        v.widgets.open.fg_stroke = egui::Stroke::new(ince, r.metin);
        v.widgets.open.rounding = yari_sm;
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renkler_token_deposundan_gelir() {
        // Çapa değer (Gün 31.2 reform): koyu temada vurgu = accent.primary = #1FB8C9 (teal).
        let t = Tokenlar::koyu();
        assert_eq!(t.renk.vurgu, Color32::from_rgb(0x1F, 0xB8, 0xC9));
        // Açık temada metin = text.primary = #1B1E22.
        let a = Tokenlar::acik();
        assert_eq!(a.renk.metin, Color32::from_rgb(0x1B, 0x1E, 0x22));
    }

    #[test]
    fn reform_katmanli_renkler_ve_golge_token_paletinden() {
        // Bölüm A: katmanlı yüzeyler + port/kod renkleri token'dan gelir; gölge `golge.renk`
        // alfasını ölçü `carpan`'ıyla ölçekler.
        let t = Tokenlar::koyu();
        assert_eq!(t.renk.zemin_cukur, Color32::from_rgb(0x0A, 0x0B, 0x0C));
        assert_eq!(t.renk.yuzey1, Color32::from_rgb(0x15, 0x17, 0x1A));
        assert_eq!(t.renk.yuzey4, Color32::from_rgb(0x2A, 0x2F, 0x35));
        // anahtar_renk yeni anahtarları çözer (node port + kod söz dizimi).
        assert_eq!(t.anahtar_renk("port.veri"), t.renk.port_veri);
        assert_eq!(t.anahtar_renk("kod.anahtar"), t.renk.kod_anahtar);
        assert_eq!(t.anahtar_renk("yuzey.3"), t.renk.yuzey3);
        // Gölge: e3 (modal) alfa, e2 (kart) alfasından koyu; ofset/bulanıklık ölçüden.
        let modal = t.golge_modal();
        let kart = t.golge_kart();
        assert!(modal.color.a() > kart.color.a());
        assert_eq!(modal.blur, t.olcu.yukselti.e3.yayilma);
        // Yüksek kontrast / açık temada port renkleri farklı (her tema kendi paletinden).
        let yk = Tokenlar::yuksek_kontrast();
        assert_ne!(yk.renk.port_veri, t.renk.port_veri);
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
        // Kabuk arka = zemin.taban (UE5 yoğun); girdi alanı = yuzey.3 (kart yüzeyi).
        assert_eq!(v.panel_fill, t.renk.zemin);
        assert_eq!(v.extreme_bg_color, t.renk.yuzey3);
        assert_eq!(v.widgets.active.bg_fill, t.renk.vurgu);
    }
}
