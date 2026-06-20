//! Tipografi sistemi — font rolleri, boyutlar ve DPI/ölçek farkındalığı (İP-04 / Bölüm 0.8).
//!
//! Üç açık/ücretsiz lisanslı aile (hepsi OFL-1.1):
//! - **Inter** → gövde/arayüz (14 px)
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
    /// Gövde/arayüz metni (Inter, 14 px).
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

/// DPI/ölçek farkında tipografi ölçeği.  Mantıksal (logical) px değerlerini taşır; efektif
/// (fiziksel) px, `olcek` (monitör DPI ölçeği) ile çarpılarak elde edilir.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tipografi {
    /// Gövde metni mantıksal boyutu (px).
    pub govde_px: f32,
    /// Kod metni mantıksal boyutu (px).
    pub kod_px: f32,
    /// Başlık (section) mantıksal boyutu (px).
    pub baslik_px: f32,
    /// Büyük display başlık mantıksal boyutu (px).
    pub display_px: f32,
    /// Küçük/yardımcı metin mantıksal boyutu (px).
    pub kucuk_px: f32,
    /// Aktif DPI/ölçek katsayısı (winit `scale_factor`; 1.0 = 96 DPI, 2.0 = 192 DPI/4K).
    pub olcek: f32,
}

impl Tipografi {
    /// Spec 0.8 varsayılan ölçeği (Inter 14 / JetBrains Mono 13 / Space Grotesk 20), ölçek 1.0.
    pub const fn varsayilan() -> Self {
        Self {
            govde_px: 14.0,
            kod_px: 13.0,
            baslik_px: 20.0,
            display_px: 28.0,
            kucuk_px: 12.0,
            olcek: 1.0,
        }
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
    fn mantiksal_boyutlar_spec_0_8_ile_uyumlu() {
        let t = Tipografi::varsayilan();
        assert_eq!(t.mantiksal(FontRol::Govde), 14.0);
        assert_eq!(t.mantiksal(FontRol::Kod), 13.0);
        assert_eq!(t.mantiksal(FontRol::Baslik), 20.0);
    }

    #[test]
    fn dpi_olcegi_efektif_boyutu_buyutur() {
        // 4K/200% ölçekte gövde 14 px → 28 fiziksel px (çoklu monitör akıcılığı).
        let t = Tipografi::varsayilan().olcekle(2.0);
        assert_eq!(t.efektif(FontRol::Govde), 28.0);
        assert_eq!(t.efektif(FontRol::Kod), 26.0);
        // 150% ölçek.
        let t15 = Tipografi::varsayilan().olcekle(1.5);
        assert_eq!(t15.efektif(FontRol::Baslik), 30.0);
    }

    #[test]
    fn olcek_makul_araliga_sikistirilir() {
        assert_eq!(Tipografi::varsayilan().olcekle(0.1).olcek, 0.5);
        assert_eq!(Tipografi::varsayilan().olcekle(99.0).olcek, 8.0);
    }
}
