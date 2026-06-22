//! Tasarım ölçü token'ları — boşluk, yarıçap, kenarlık, gölge, hareket, yerleşim, yoğunluk
//! (Gün 31.2 Tasarım Reformu / Bölüm A.4–A.6).
//!
//! Renkler `tokens.json`'dan gelir (MK-52); **ölçüler de tek kaynaktan** gelmeli ki tüm ekranlar
//! aynı dili konuşsun (tutarlı aralık/yarıçap/yerleşim). Bu modül o tek kaynaktır.
//!
//! **Mimari (MK-40):** Bu modül egui'ye bağlı *değildir*; yalnızca saf sayısal değerleri (mantıksal
//! piksel) tutar. egui'ye (`Vec2`/`Rounding`/`Shadow`) dönüşüm UI katmanındaki ince adaptördedir.
//! Böylece aynı ölçü dili hem 2B (egui) hem 3B viewport yerleşiminde ortak kullanılır.
// MK-52 ruhu: token tek kaynak (renk + ölçü). MK-40: render katmanı egui'ye bağlı değildir.
// MK-04: hareket süreleri hafiftir; animasyon 60 FPS kare bütçesini bozmaz.

/// 4px tabanlı boşluk (spacing) skalası: 2 · 4 · 6 · 8 · 12 · 16 · 20 · 24 · 32 · 40 (A.4).
///
/// Tüm padding/margin/aralık değerleri bu adımlardan seçilir (rastgele piksel yok).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bosluk {
    /// 2 px — en sıkı iç aralık (rozet içi, ikon-metin).
    pub xxs: f32,
    /// 4 px.
    pub xs: f32,
    /// 6 px.
    pub sm: f32,
    /// 8 px.
    pub s: f32,
    /// 12 px (varsayılan iç padding).
    pub m: f32,
    /// 16 px.
    pub l: f32,
    /// 20 px.
    pub xl: f32,
    /// 24 px.
    pub xxl: f32,
    /// 32 px.
    pub x3: f32,
    /// 40 px — en geniş bölüm aralığı.
    pub x4: f32,
}

impl Bosluk {
    /// Standart boşluk skalası.
    pub const fn varsayilan() -> Self {
        Self {
            xxs: 2.0,
            xs: 4.0,
            sm: 6.0,
            s: 8.0,
            m: 12.0,
            l: 16.0,
            xl: 20.0,
            xxl: 24.0,
            x3: 32.0,
            x4: 40.0,
        }
    }
}

/// Köşe yarıçapı skalası (A.4): r0=0 (panel/dock) · sm=3 (girdi/buton) · md=6 (kart/modal) ·
/// pill=9999 (rozet/switch — tam yuvarlak).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Yaricap {
    /// 0 px — panel kenarları, dock (keskin).
    pub r0: f32,
    /// 3 px — girdi alanı / buton.
    pub sm: f32,
    /// 6 px — kart / modal.
    pub md: f32,
    /// 9999 px — hap (pill) rozet / switch.
    pub pill: f32,
}

impl Yaricap {
    /// Standart yarıçap skalası.
    pub const fn varsayilan() -> Self {
        Self {
            r0: 0.0,
            sm: 3.0,
            md: 6.0,
            pill: 9999.0,
        }
    }
}

/// Kenarlık kalınlıkları (A.4): 1px varsayılan, 2px odak/aktif vurgu çizgisi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KenarKalinlik {
    /// 1 px — varsayılan kenarlık/ayraç.
    pub ince: f32,
    /// 2 px — odak halkası / aktif vurgu çizgisi.
    pub vurgu: f32,
}

impl KenarKalinlik {
    /// Standart kenarlık kalınlıkları.
    pub const fn varsayilan() -> Self {
        Self {
            ince: 1.0,
            vurgu: 2.0,
        }
    }
}

/// Tek bir gölge (elevation) tanımı: kayma (offset) + yayılma (blur yaklaşımı) + alfa çarpanı.
///
/// egui'de gerçek blur yoktur; UI adaptörü bunu düz katman + ince kenarlık + `golge.renk`
/// (alfa `carpan` ile ölçeklenmiş) yaklaşımıyla çizer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Golge {
    /// Yatay kayma (px).
    pub dx: f32,
    /// Dikey kayma (px).
    pub dy: f32,
    /// Yayılma/bulanıklık yarıçapı (px; egui'de yumuşatma yaklaşımı).
    pub yayilma: f32,
    /// Gölge alfa çarpanı (0–1; `golge.renk` token alfası bununla ölçeklenir).
    pub carpan: f32,
}

/// Üç yükselti seviyesi (A.4): e1 (hafif) · e2 (kart/menü) · e3 (modal/drawer).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Yukselti {
    /// e1 — 0 1 2, %30.
    pub e1: Golge,
    /// e2 — 0 2 6, %40 (kart, dropdown).
    pub e2: Golge,
    /// e3 — 0 8 24, %50 (modal, drawer, ayrılmış pencere).
    pub e3: Golge,
}

impl Yukselti {
    /// Standart üç-seviye yükselti skalası.
    pub const fn varsayilan() -> Self {
        Self {
            e1: Golge {
                dx: 0.0,
                dy: 1.0,
                yayilma: 2.0,
                carpan: 0.30,
            },
            e2: Golge {
                dx: 0.0,
                dy: 2.0,
                yayilma: 6.0,
                carpan: 0.40,
            },
            e3: Golge {
                dx: 0.0,
                dy: 8.0,
                yayilma: 24.0,
                carpan: 0.50,
            },
        }
    }
}

/// Hareket (animasyon) süreleri (A.4), milisaniye. easing = ease-out (UI tarafında uygulanır).
///
/// **MK-04:** Süreler hafiftir; hiçbir animasyon 60 FPS kare bütçesini bozmaz. "Azaltılmış
/// hareket" erişilebilirlik tercihinde ([`Hareket::azaltilmis`]) tüm süreler `anlik`'a iner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hareket {
    /// 0 ms — anında (geçişsiz).
    pub anlik: f32,
    /// 90 ms — hover/küçük geçiş.
    pub hizli: f32,
    /// 150 ms — sekme/panel geçişi.
    pub taban: f32,
    /// 240 ms — drawer/modal açılışı.
    pub yavas: f32,
}

impl Hareket {
    /// Standart süre skalası.
    pub const fn varsayilan() -> Self {
        Self {
            anlik: 0.0,
            hizli: 90.0,
            taban: 150.0,
            yavas: 240.0,
        }
    }

    /// "Azaltılmış hareket" (İP-12 erişilebilirlik ayarı): tüm süreler `0`'a iner.
    pub const fn azaltilmis() -> Self {
        Self {
            anlik: 0.0,
            hizli: 0.0,
            taban: 0.0,
            yavas: 0.0,
        }
    }
}

/// Sabit yerleşim metrikleri (A.6) — kabuk bölge boyutları (mantıksal px).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Yerlesim {
    /// Üst başlık şeridi yüksekliği (32).
    pub baslik_cubugu: f32,
    /// Activity bar genişliği (48; ikon 22).
    pub aktivite_cubugu: f32,
    /// Activity bar ikon hedef boyutu (22).
    pub aktivite_ikon: f32,
    /// Side bar varsayılan genişliği (260).
    pub yan_panel: f32,
    /// Side bar minimum genişliği (180).
    pub yan_panel_min: f32,
    /// Side bar maksimum genişliği (480).
    pub yan_panel_max: f32,
    /// Sekme yüksekliği (35).
    pub sekme: f32,
    /// Status bar yüksekliği (22).
    pub durum_cubugu: f32,
    /// Alt panel (Content Drawer) varsayılan yüksekliği (220).
    pub alt_panel: f32,
    /// Inspector / Details paneli genişliği (300).
    pub inspector: f32,
    /// Bölücü (splitter) sürükleme bölgesi kalınlığı (4).
    pub bolucu: f32,
}

impl Yerlesim {
    /// Spec A.6 sabit boyut sözlüğü.
    pub const fn varsayilan() -> Self {
        Self {
            baslik_cubugu: 32.0,
            aktivite_cubugu: 48.0,
            aktivite_ikon: 22.0,
            yan_panel: 260.0,
            yan_panel_min: 180.0,
            yan_panel_max: 480.0,
            sekme: 35.0,
            durum_cubugu: 22.0,
            alt_panel: 220.0,
            inspector: 300.0,
            bolucu: 4.0,
        }
    }
}

/// Arayüz yoğunluğu (A.5): Kompakt (UE5 yoğun, varsayılan) / Rahat.  İP-12 `panel_yogunlugu`
/// ayarına **görsel olarak** bağlanır (model zaten var); kontrol yüksekliği + satır aralığını ölçekler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Yogunluk {
    /// Kompakt — UE5 yoğun yerleşim (varsayılan).
    #[default]
    Kompakt,
    /// Rahat — daha geniş dokunma hedefleri / satır aralığı.
    Rahat,
}

impl Yogunluk {
    /// Bu yoğunlukta varsayılan kontrol (buton/girdi) yüksekliği (px).
    pub fn kontrol_yuksekligi(&self) -> f32 {
        match self {
            Yogunluk::Kompakt => 26.0,
            Yogunluk::Rahat => 30.0,
        }
    }

    /// Satır aralığı çarpanı (kompakt sıkı, rahat ferah).
    pub fn satir_carpani(&self) -> f32 {
        match self {
            Yogunluk::Kompakt => 1.0,
            Yogunluk::Rahat => 1.18,
        }
    }
}

/// Tüm ölçü token'larının (renk dışı tasarım sistemi) tek paketi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Olcu {
    /// Boşluk (spacing) skalası.
    pub bosluk: Bosluk,
    /// Köşe yarıçapı skalası.
    pub yaricap: Yaricap,
    /// Kenarlık kalınlıkları.
    pub kenar: KenarKalinlik,
    /// Yükselti (gölge) skalası.
    pub yukselti: Yukselti,
    /// Hareket (animasyon) süreleri.
    pub hareket: Hareket,
    /// Sabit yerleşim metrikleri.
    pub yerlesim: Yerlesim,
    /// Arayüz yoğunluğu.
    pub yogunluk: Yogunluk,
}

impl Olcu {
    /// Standart ölçü token paketi (Kompakt yoğunluk, tam hareket).
    pub const fn varsayilan() -> Self {
        Self {
            bosluk: Bosluk::varsayilan(),
            yaricap: Yaricap::varsayilan(),
            kenar: KenarKalinlik::varsayilan(),
            yukselti: Yukselti::varsayilan(),
            hareket: Hareket::varsayilan(),
            yerlesim: Yerlesim::varsayilan(),
            yogunluk: Yogunluk::Kompakt,
        }
    }

    /// Verilen yoğunlukla bir kopya döndürür (İP-12 `panel_yogunlugu` görsel bağlama).
    pub fn yogunlukla(mut self, yogunluk: Yogunluk) -> Self {
        self.yogunluk = yogunluk;
        self
    }

    /// "Azaltılmış hareket" tercihiyle bir kopya döndürür (animasyon süreleri sıfırlanır).
    pub fn hareketsiz(mut self) -> Self {
        self.hareket = Hareket::azaltilmis();
        self
    }
}

impl Default for Olcu {
    fn default() -> Self {
        Self::varsayilan()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bosluk_skalasi_4px_tabanli_ve_artan() {
        let b = Bosluk::varsayilan();
        let adimlar = [b.xxs, b.xs, b.sm, b.s, b.m, b.l, b.xl, b.xxl, b.x3, b.x4];
        // Spec A.4: 2,4,6,8,12,16,20,24,32,40 — kesinlikle artan.
        assert_eq!(
            adimlar,
            [2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 20.0, 24.0, 32.0, 40.0]
        );
        for pencere in adimlar.windows(2) {
            assert!(pencere[1] > pencere[0], "boşluk skalası artan olmalı");
        }
    }

    #[test]
    fn yaricap_ve_kenar_spec_degerleri() {
        let r = Yaricap::varsayilan();
        assert_eq!((r.r0, r.sm, r.md), (0.0, 3.0, 6.0));
        assert!(r.pill > 1000.0, "pill tam yuvarlak (büyük) olmalı");
        let k = KenarKalinlik::varsayilan();
        assert_eq!((k.ince, k.vurgu), (1.0, 2.0));
    }

    #[test]
    fn yukselti_seviyeleri_giderek_belirginlesir() {
        let y = Yukselti::varsayilan();
        assert!(y.e1.carpan < y.e2.carpan && y.e2.carpan < y.e3.carpan);
        assert!(y.e1.yayilma < y.e3.yayilma);
        assert_eq!((y.e3.dy, y.e3.yayilma), (8.0, 24.0));
    }

    #[test]
    fn hareket_sureleri_ve_azaltilmis_mod() {
        let h = Hareket::varsayilan();
        assert!(h.anlik < h.hizli && h.hizli < h.taban && h.taban < h.yavas);
        assert_eq!((h.hizli, h.taban, h.yavas), (90.0, 150.0, 240.0));
        // Azaltılmış hareket (erişilebilirlik): hepsi 0.
        let az = Hareket::azaltilmis();
        assert_eq!((az.hizli, az.taban, az.yavas), (0.0, 0.0, 0.0));
    }

    #[test]
    fn yerlesim_a6_sozlugu() {
        let l = Yerlesim::varsayilan();
        assert_eq!(l.baslik_cubugu, 32.0);
        assert_eq!(l.aktivite_cubugu, 48.0);
        assert_eq!(l.yan_panel, 260.0);
        assert!(l.yan_panel_min < l.yan_panel && l.yan_panel < l.yan_panel_max);
        assert_eq!(l.durum_cubugu, 22.0);
    }

    #[test]
    fn yogunluk_kompakt_rahattan_siki() {
        assert_eq!(Yogunluk::default(), Yogunluk::Kompakt);
        assert!(
            Yogunluk::Kompakt.kontrol_yuksekligi() < Yogunluk::Rahat.kontrol_yuksekligi(),
            "kompakt daha sıkı (küçük kontrol) olmalı"
        );
        assert!(Yogunluk::Kompakt.satir_carpani() < Yogunluk::Rahat.satir_carpani());
    }

    #[test]
    fn olcu_yogunluk_ve_hareketsiz_kopyalar() {
        let o = Olcu::varsayilan();
        assert_eq!(o.yogunluk, Yogunluk::Kompakt);
        assert_eq!(o.yogunlukla(Yogunluk::Rahat).yogunluk, Yogunluk::Rahat);
        assert_eq!(o.hareketsiz().hareket.taban, 0.0);
        // Yoğunluk değişimi diğer ölçüleri bozmaz (yalnız yoğunluk alanı değişir).
        assert_eq!(o.yogunlukla(Yogunluk::Rahat).bosluk, o.bosluk);
    }
}
