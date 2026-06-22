//! Tipografi sistemi — font rolleri, boyutlar ve DPI/ölçek farkındalığı (İP-04 / Bölüm 0.8).
//!
//! Üç açık/ücretsiz lisanslı aile (hepsi OFL-1.1):
//! - **Inter** → gövde/arayüz (13 px; Gün 31.2 / A.3 VS Code tabanı)
//! - **JetBrains Mono** → kod (13 px)
//! - **Space Grotesk** → başlık/display
//!
//! **Mimari (MK-40):** Bu modül egui'ye bağlı *değildir*; yalnızca rol→boyut eşlemesini ve
//! DPI ölçek matematiğini tutar (4K + çoklu monitör akıcılığı).  Gerçek font baytlarının
//! egui'ye kurulması UI katmanındaki ince adaptördedir; `.ttf` yoksa egui gömülü fontuna
//! düşülür (sessiz değil, bilgilendirerek — TDA madde 1).

/// Bir metnin işlevsel rolü; somut aile/boyut buradan türer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontRol {
    /// Gövde/arayüz metni (Inter, 13 px).
    Govde,
    /// Başlık/display (Space Grotesk).
    Baslik,
    /// Kod/monospace (JetBrains Mono, 13 px).
    Kod,
}

impl FontRol {
    /// Bu rolün açık-lisanslı font ailesinin adı.
    pub fn aile(&self) -> &'static str {
        match self {
            FontRol::Govde => "Inter",
            FontRol::Baslik => "Space Grotesk",
            FontRol::Kod => "JetBrains Mono",
        }
    }

    /// Bu rol için beklenen `assets/fonts/<dosya>` adı (varsa otomatik yüklenir).
    pub fn dosya_adi(&self) -> &'static str {
        match self {
            FontRol::Govde => "Inter-Regular.ttf",
            FontRol::Baslik => "SpaceGrotesk-Medium.ttf",
            FontRol::Kod => "JetBrainsMono-Regular.ttf",
        }
    }

    /// Tüm roller (testler ve font kurulumu için).
    pub const TUMU: [FontRol; 3] = [FontRol::Govde, FontRol::Baslik, FontRol::Kod];
}

/// Font ağırlığı (kalınlık) token'ı (Gün 31.2 / A.3).  egui değişken ağırlığı font dosyası
/// olmadan uygulayamaz; bu enum **anlamsal** ağırlığı taşır (kalın font varsa onu seçmek,
/// yoksa egui'nin sahte-kalınına / normal ağırlığına düşmek için).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontAgirlik {
    /// 400 — gövde metni.
    Normal,
    /// 500 — etiket / orta vurgu.
    Orta,
    /// 600 — başlık / kalın gövde.
    YariKalin,
}

impl FontAgirlik {
    /// CSS-benzeri sayısal ağırlık (400/500/600).
    pub fn deger(&self) -> u16 {
        match self {
            FontAgirlik::Normal => 400,
            FontAgirlik::Orta => 500,
            FontAgirlik::YariKalin => 600,
        }
    }

    /// Bu ağırlık kalın sayılır mı (egui `strong`/sahte-kalın seçimi için).
    pub fn kalin_mi(&self) -> bool {
        self.deger() >= 600
    }
}

/// Tip skala rolü (Gün 31.2 / A.3) — somut boyut + ağırlık + satır yüksekliği buradan türer.
///
/// UI yoğun (VS Code 13px tabanı): display 28/600 · h1 22/600 · h2 18/600 · h3 (panel başlık)
/// 15/600 · gövde 13/400 · gövde-kalın 13/600 · küçük 12/400 · etiket 11/500 · kod 13/400.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetinRol {
    /// 28/600 — büyük display başlık (karşılama/splash).
    Display,
    /// 22/600 — birinci seviye başlık.
    H1,
    /// 18/600 — ikinci seviye başlık.
    H2,
    /// 15/600 — panel başlığı (üçüncü seviye).
    H3,
    /// 13/400 — gövde / arayüz metni.
    Govde,
    /// 13/600 — vurgulu gövde.
    GovdeKalin,
    /// 12/400 — küçük / yardımcı metin.
    Kucuk,
    /// 11/500 — etiket / BÜYÜK-harf rozet (harf aralığı +0.3).
    Etiket,
    /// 13/400 — kod / monospace (satır yüksekliği 1.5).
    Kod,
}

impl MetinRol {
    /// Bu rolün font ağırlığı.
    pub fn agirlik(&self) -> FontAgirlik {
        match self {
            MetinRol::Display
            | MetinRol::H1
            | MetinRol::H2
            | MetinRol::H3
            | MetinRol::GovdeKalin => FontAgirlik::YariKalin,
            MetinRol::Etiket => FontAgirlik::Orta,
            MetinRol::Govde | MetinRol::Kucuk | MetinRol::Kod => FontAgirlik::Normal,
        }
    }

    /// Bu rolün satır yüksekliği çarpanı (UI ~1.35; kod 1.5; başlık daha sıkı 1.2).
    pub fn satir_yuksekligi(&self) -> f32 {
        match self {
            MetinRol::Display | MetinRol::H1 | MetinRol::H2 | MetinRol::H3 => 1.2,
            MetinRol::Kod => 1.5,
            MetinRol::Govde | MetinRol::GovdeKalin | MetinRol::Kucuk | MetinRol::Etiket => 1.35,
        }
    }

    /// Hangi font ailesi rolünden çizilir (başlıklar Başlık ailesi, kod Mono, gerisi Gövde).
    pub fn font_ailesi(&self) -> FontRol {
        match self {
            MetinRol::Display | MetinRol::H1 | MetinRol::H2 | MetinRol::H3 => FontRol::Baslik,
            MetinRol::Kod => FontRol::Kod,
            MetinRol::Govde | MetinRol::GovdeKalin | MetinRol::Kucuk | MetinRol::Etiket => {
                FontRol::Govde
            }
        }
    }

    /// Tüm tip skala rolleri (UI font kurulumu / testler için).
    pub const TUMU: [MetinRol; 9] = [
        MetinRol::Display,
        MetinRol::H1,
        MetinRol::H2,
        MetinRol::H3,
        MetinRol::Govde,
        MetinRol::GovdeKalin,
        MetinRol::Kucuk,
        MetinRol::Etiket,
        MetinRol::Kod,
    ];
}

/// DPI/ölçek farkında tipografi ölçeği.  Mantıksal (logical) px değerlerini taşır; efektif
/// (fiziksel) px, `olcek` (monitör DPI ölçeği) ile çarpılarak elde edilir.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tipografi {
    /// Gövde metni mantıksal boyutu (px) — VS Code 13px tabanı (Gün 31.2 / A.3).
    pub govde_px: f32,
    /// Kod metni mantıksal boyutu (px).
    pub kod_px: f32,
    /// H2 (section başlık) mantıksal boyutu (px).
    pub baslik_px: f32,
    /// H1 (birinci seviye başlık) mantıksal boyutu (px).
    pub h1_px: f32,
    /// H3 (panel başlık) mantıksal boyutu (px).
    pub h3_px: f32,
    /// Büyük display başlık mantıksal boyutu (px).
    pub display_px: f32,
    /// Küçük/yardımcı metin mantıksal boyutu (px).
    pub kucuk_px: f32,
    /// Etiket / rozet metni mantıksal boyutu (px).
    pub etiket_px: f32,
    /// Aktif DPI/ölçek katsayısı (winit `scale_factor`; 1.0 = 96 DPI, 2.0 = 192 DPI/4K).
    pub olcek: f32,
}

impl Tipografi {
    /// Gün 31.2 / A.3 tip skala (Inter 13 gövde / JetBrains Mono 13 kod / Space Grotesk başlık),
    /// ölçek 1.0.  Display 28 · H1 22 · H2 18 · H3 15 · gövde 13 · küçük 12 · etiket 11.
    pub const fn varsayilan() -> Self {
        Self {
            govde_px: 13.0,
            kod_px: 13.0,
            baslik_px: 18.0,
            h1_px: 22.0,
            h3_px: 15.0,
            display_px: 28.0,
            kucuk_px: 12.0,
            etiket_px: 11.0,
            olcek: 1.0,
        }
    }

    /// Bir tip skala rolünün **mantıksal** (ölçeklenmemiş) boyutu (A.3).
    pub fn rol_mantiksal(&self, rol: MetinRol) -> f32 {
        match rol {
            MetinRol::Display => self.display_px,
            MetinRol::H1 => self.h1_px,
            MetinRol::H2 => self.baslik_px,
            MetinRol::H3 => self.h3_px,
            MetinRol::Govde | MetinRol::GovdeKalin => self.govde_px,
            MetinRol::Kucuk => self.kucuk_px,
            MetinRol::Etiket => self.etiket_px,
            MetinRol::Kod => self.kod_px,
        }
    }

    /// Bir tip skala rolünün **efektif** (DPI ile ölçeklenmiş) boyutu.
    pub fn rol_efektif(&self, rol: MetinRol) -> f32 {
        self.rol_mantiksal(rol) * self.olcek
    }

    /// Verilen DPI ölçeğiyle (winit `scale_factor`) bir kopya döndürür (4K/çoklu monitör).
    pub fn olcekle(mut self, olcek: f32) -> Self {
        // Aşırı küçük/dev ölçekleri makul bir aralığa sıkıştır (bozuk monitör bilgisine karşı).
        self.olcek = olcek.clamp(0.5, 8.0);
        self
    }

    /// Bir rolün **mantıksal** (ölçeklenmemiş) boyutu.
    pub fn mantiksal(&self, rol: FontRol) -> f32 {
        match rol {
            FontRol::Govde => self.govde_px,
            FontRol::Baslik => self.baslik_px,
            FontRol::Kod => self.kod_px,
        }
    }

    /// Bir rolün **efektif** (DPI ile ölçeklenmiş, fiziksel) boyutu.
    pub fn efektif(&self, rol: FontRol) -> f32 {
        self.mantiksal(rol) * self.olcek
    }

    /// Display başlığın efektif boyutu.
    pub fn display_efektif(&self) -> f32 {
        self.display_px * self.olcek
    }

    /// Küçük metnin efektif boyutu.
    pub fn kucuk_efektif(&self) -> f32 {
        self.kucuk_px * self.olcek
    }
}

impl Default for Tipografi {
    fn default() -> Self {
        Self::varsayilan()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roller_dogru_aile_ve_dosya_verir() {
        assert_eq!(FontRol::Govde.aile(), "Inter");
        assert_eq!(FontRol::Kod.aile(), "JetBrains Mono");
        assert_eq!(FontRol::Baslik.aile(), "Space Grotesk");
        assert!(FontRol::Govde.dosya_adi().ends_with(".ttf"));
        assert_eq!(FontRol::TUMU.len(), 3);
    }

    #[test]
    fn mantiksal_boyutlar_reform_a3_ile_uyumlu() {
        // Gün 31.2 / A.3: VS Code 13px tabanı (gövde 14→13 bilinçli yoğunlaşma, MK-58).
        let t = Tipografi::varsayilan();
        assert_eq!(t.mantiksal(FontRol::Govde), 13.0);
        assert_eq!(t.mantiksal(FontRol::Kod), 13.0);
        assert_eq!(t.mantiksal(FontRol::Baslik), 18.0);
    }

    #[test]
    fn dpi_olcegi_efektif_boyutu_buyutur() {
        // 4K/200% ölçekte gövde 13 px → 26 fiziksel px (çoklu monitör akıcılığı).
        let t = Tipografi::varsayilan().olcekle(2.0);
        assert_eq!(t.efektif(FontRol::Govde), 26.0);
        assert_eq!(t.efektif(FontRol::Kod), 26.0);
        // 150% ölçek: H2 18 → 27.
        let t15 = Tipografi::varsayilan().olcekle(1.5);
        assert_eq!(t15.efektif(FontRol::Baslik), 27.0);
    }

    #[test]
    fn tip_skala_rolleri_a3_boyut_ve_agirlik() {
        // A.3 type scale: display 28/600 · h1 22/600 · h2 18/600 · h3 15/600 · gövde 13/400 ·
        // küçük 12/400 · etiket 11/500 · kod 13/400.
        let t = Tipografi::varsayilan();
        assert_eq!(t.rol_mantiksal(MetinRol::Display), 28.0);
        assert_eq!(t.rol_mantiksal(MetinRol::H1), 22.0);
        assert_eq!(t.rol_mantiksal(MetinRol::H2), 18.0);
        assert_eq!(t.rol_mantiksal(MetinRol::H3), 15.0);
        assert_eq!(t.rol_mantiksal(MetinRol::Govde), 13.0);
        assert_eq!(t.rol_mantiksal(MetinRol::Etiket), 11.0);
        // Ağırlıklar (anlamsal): başlıklar/gövde-kalın 600, etiket 500, gövde/küçük/kod 400.
        assert_eq!(MetinRol::H1.agirlik().deger(), 600);
        assert_eq!(MetinRol::GovdeKalin.agirlik().deger(), 600);
        assert_eq!(MetinRol::Etiket.agirlik().deger(), 500);
        assert_eq!(MetinRol::Govde.agirlik().deger(), 400);
        assert!(MetinRol::H1.agirlik().kalin_mi() && !MetinRol::Govde.agirlik().kalin_mi());
        // Kod satır yüksekliği 1.5; başlıklar daha sıkı (1.2).
        assert_eq!(MetinRol::Kod.satir_yuksekligi(), 1.5);
        assert!(MetinRol::H1.satir_yuksekligi() < MetinRol::Govde.satir_yuksekligi());
        // Rol → font ailesi eşlemesi (başlık ailesi / mono / gövde).
        assert_eq!(MetinRol::H2.font_ailesi(), FontRol::Baslik);
        assert_eq!(MetinRol::Kod.font_ailesi(), FontRol::Kod);
        assert_eq!(MetinRol::Govde.font_ailesi(), FontRol::Govde);
        assert_eq!(MetinRol::TUMU.len(), 9);
    }

    #[test]
    fn olcek_makul_araliga_sikistirilir() {
        assert_eq!(Tipografi::varsayilan().olcekle(0.1).olcek, 0.5);
        assert_eq!(Tipografi::varsayilan().olcekle(99.0).olcek, 8.0);
    }
}
