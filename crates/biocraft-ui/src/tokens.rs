//! Tasarım token'ları (MK-53, TDA madde 14) — renk, boşluk, köşe yarıçapı.
//!
//! Tüm TDA bileşenleri rengini ve aralığını **buradan** alır; asla doğrudan sabit
//! renk yazmaz.  Tema değiştiğinde (açık/koyu) token'lar yeniden üretilir ve bileşenler
//! otomatik olarak uyar.  Bu, "tema değişince renkler token'dan mı geliyor?" güvencesini
//! tek noktada sağlar.

use egui::Color32;

/// Bir bileşenin anlam/önem sınıfı; somut renk eşlemesi token'dan gelir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Onem {
    /// Başarı / olumlu (yeşil).
    Basari,
    /// Uyarı / dikkat (amber).
    Uyari,
    /// Hata / yıkıcı (kırmızı).
    Hata,
    /// Bilgi / nötr-mavi.
    Bilgi,
    /// Nötr (vurgusuz).
    Notr,
}

/// Anlamsal renk paleti.  Açık ve koyu tema için ayrı üretilir.
#[derive(Debug, Clone, Copy)]
pub struct Renkler {
    /// Başarı vurgu rengi (metin/ikon/kenarlık).
    pub basari: Color32,
    /// Başarı arka plan (zemin) rengi.
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
    /// Panel/kart yüzey rengi.
    pub yuzey: Color32,
    /// İkincil (daha koyu/açık) yüzey rengi.
    pub yuzey_alt: Color32,
    /// Kenarlık rengi.
    pub kenarlik: Color32,
    /// Birincil metin rengi.
    pub metin: Color32,
    /// İkincil/soluk metin rengi.
    pub metin_soluk: Color32,
    /// Yükleme iskeleti dolgu rengi.
    pub iskelet: Color32,
}

/// Boşluk ölçeği (4'ün katları) — tutarlı aralıklar için.
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

/// Tüm tasarım token'larının paketi.  `temadan(koyu)` ile üretilir.
#[derive(Debug, Clone, Copy)]
pub struct Tokenlar {
    /// Anlamsal renk paleti.
    pub renk: Renkler,
    /// Boşluk ölçeği.
    pub bosluk: Bosluk,
    /// Standart köşe yarıçapı (px).
    pub yaricap: f32,
    /// Bu token setinin koyu tema olup olmadığı.
    pub koyu: bool,
}

impl Tokenlar {
    /// Açık tema token'ları.
    pub fn acik() -> Self {
        Self {
            renk: Renkler {
                basari: Color32::from_rgb(0x2E, 0x7D, 0x32),
                basari_zemin: Color32::from_rgb(0xE6, 0xF4, 0xEA),
                uyari: Color32::from_rgb(0xB4, 0x53, 0x09),
                uyari_zemin: Color32::from_rgb(0xFD, 0xF1, 0xDD),
                hata: Color32::from_rgb(0xC6, 0x28, 0x28),
                hata_zemin: Color32::from_rgb(0xFB, 0xE7, 0xE7),
                bilgi: Color32::from_rgb(0x15, 0x65, 0xC0),
                bilgi_zemin: Color32::from_rgb(0xE6, 0xF0, 0xFB),
                vurgu: Color32::from_rgb(0x4F, 0x46, 0xE5),
                yuzey: Color32::from_rgb(0xFF, 0xFF, 0xFF),
                yuzey_alt: Color32::from_rgb(0xF1, 0xF3, 0xF5),
                kenarlik: Color32::from_rgb(0xD0, 0xD7, 0xDE),
                metin: Color32::from_rgb(0x1A, 0x1D, 0x21),
                metin_soluk: Color32::from_rgb(0x5C, 0x63, 0x6A),
                iskelet: Color32::from_rgb(0xE2, 0xE6, 0xEA),
            },
            bosluk: Bosluk::varsayilan(),
            yaricap: 8.0,
            koyu: false,
        }
    }

    /// Koyu tema token'ları.
    pub fn koyu() -> Self {
        Self {
            renk: Renkler {
                basari: Color32::from_rgb(0x66, 0xBB, 0x6A),
                basari_zemin: Color32::from_rgb(0x1B, 0x2E, 0x20),
                uyari: Color32::from_rgb(0xFB, 0xBF, 0x24),
                uyari_zemin: Color32::from_rgb(0x33, 0x2A, 0x12),
                hata: Color32::from_rgb(0xEF, 0x53, 0x50),
                hata_zemin: Color32::from_rgb(0x33, 0x1D, 0x1D),
                bilgi: Color32::from_rgb(0x42, 0xA5, 0xF5),
                bilgi_zemin: Color32::from_rgb(0x16, 0x27, 0x36),
                vurgu: Color32::from_rgb(0x81, 0x8C, 0xF8),
                yuzey: Color32::from_rgb(0x1E, 0x22, 0x27),
                yuzey_alt: Color32::from_rgb(0x26, 0x2B, 0x31),
                kenarlik: Color32::from_rgb(0x3A, 0x41, 0x4A),
                metin: Color32::from_rgb(0xE6, 0xE9, 0xED),
                metin_soluk: Color32::from_rgb(0x9A, 0xA3, 0xAD),
                iskelet: Color32::from_rgb(0x2D, 0x33, 0x3B),
            },
            bosluk: Bosluk::varsayilan(),
            yaricap: 8.0,
            koyu: true,
        }
    }

    /// Tema bayrağına göre uygun token setini döndürür.
    pub fn temadan(koyu: bool) -> Self {
        if koyu {
            Self::koyu()
        } else {
            Self::acik()
        }
    }

    /// egui bağlamının aktif temasından token üretir.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acik_ve_koyu_tema_farkli_renk_vermeli() {
        let a = Tokenlar::acik();
        let k = Tokenlar::koyu();
        // Tema değişince yüzey ve metin renkleri gerçekten değişmeli (token'dan gelir).
        assert_ne!(a.renk.yuzey, k.renk.yuzey);
        assert_ne!(a.renk.metin, k.renk.metin);
        assert!(!a.koyu && k.koyu);
    }

    #[test]
    fn temadan_dogru_set_secer() {
        assert!(Tokenlar::temadan(true).koyu);
        assert!(!Tokenlar::temadan(false).koyu);
    }

    #[test]
    fn onem_renkleri_token_paletinden_gelir() {
        let t = Tokenlar::acik();
        assert_eq!(t.onem_rengi(Onem::Hata), t.renk.hata);
        assert_eq!(t.onem_rengi(Onem::Basari), t.renk.basari);
        assert_eq!(t.onem_zemini(Onem::Bilgi), t.renk.bilgi_zemin);
    }
}
