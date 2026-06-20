//! Tasarım token'ları — tüm renklerin **tek kaynağı** (MK-52, İP-04 / Bölüm 0.8).
//!
//! **Kodda sabit renk YASAK.**  Her renk `assets/tokens.json`'dan gelir; bu dosya derleme
//! zamanında crate'e gömülür (`include_str!`) → uygulama harici dosya olmadan da çalışır
//! (akıllı varsayılan, çevrimdışı çalışır).  Kullanıcı kendi **özel temasını** (E2, Gün 24)
//! çalışma zamanında bunun üstüne ekleyip JSON olarak kaydedip geri yükleyebilir.
//!
//! **Mimari (MK-40):** Bu modül egui'ye bağlı *değildir*.  Renk burada saf [`Renk`] (RGBA8)
//! tipidir; egui `Color32` dönüşümü UI katmanındaki ince adaptördedir.  Böylece token sistemi
//! hem 2B (egui) hem 3B (wgpu shader) tarafında, tek doğruluk kaynağıyla kullanılır.
//!
//! - Tema değişimi O(1) (palet referansı takası) → **<100 ms**, yarı-uygulanmış ara durum yok
//!   (flicker yok): yeni palet bir bütün olarak atomik döner.
// MK-52: token + i18n + erişilebilirlik. MK-40: render katmanı egui'ye bağlı değildir.

use std::collections::BTreeMap;

use serde::Deserialize;
use thiserror::Error;

/// `assets/tokens.json` derleme zamanında gömülür (akıllı varsayılan; harici dosya gerekmez).
const GOMULU_TOKENLAR: &str = include_str!("../../../assets/tokens.json");

/// Her temanın doldurması **zorunlu** anlamsal renk anahtarları.  Bir tema bunlardan birini
/// eksik bırakırsa yükleme [`TokenHata::EksikAnahtar`] ile reddedilir (sessiz eksik renk yok).
pub const ANAHTARLAR: &[&str] = &[
    "bg.primary",
    "bg.secondary",
    "surface",
    "surface.alt",
    "border",
    "text.primary",
    "text.muted",
    "text.on-accent",
    "text.on-danger",
    "accent.primary",
    "success",
    "success.bg",
    "warning",
    "warning.bg",
    "error",
    "error.bg",
    "info",
    "info.bg",
    "skeleton",
];

/// Token sistemiyle ilgili hatalar (kütüphane hatası → `thiserror`, CLAUDE.md §3).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TokenHata {
    /// Renk metni `#RRGGBB` / `#RRGGBBAA` biçiminde değil.
    #[error("geçersiz renk değeri '{0}' (beklenen: #RRGGBB veya #RRGGBBAA)")]
    GecersizRenk(String),
    /// JSON ayrıştırılamadı.
    #[error("token JSON ayrıştırılamadı: {0}")]
    Ayristirma(String),
    /// Bir tema zorunlu bir anahtarı tanımlamamış.
    #[error("'{tema}' teması '{anahtar}' anahtarını tanımlamıyor (MK-52: tüm renkler token'dan)")]
    EksikAnahtar {
        /// Eksik anahtarı olan temanın kimliği.
        tema: String,
        /// Eksik anlamsal renk anahtarı.
        anahtar: String,
    },
    /// Hiç tema tanımlı değil.
    #[error("token dosyasında hiç tema yok")]
    TemaYok,
}

/// Saf RGBA8 renk (egui/wgpu'dan bağımsız).  `#RRGGBB` / `#RRGGBBAA` metninden ayrıştırılır.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Renk {
    /// Kırmızı bileşeni (0–255).
    pub r: u8,
    /// Yeşil bileşeni (0–255).
    pub g: u8,
    /// Mavi bileşeni (0–255).
    pub b: u8,
    /// Saydamlık (255 = opak).
    pub a: u8,
}

impl Renk {
    /// Bileşenlerden renk üretir (alfa = 255).
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// `#RRGGBB` veya `#RRGGBBAA` (büyük/küçük harf) metnini ayrıştırır.
    pub fn hexten(metin: &str) -> Result<Self, TokenHata> {
        let s = metin.strip_prefix('#').unwrap_or(metin);
        let bayt = |i: usize| -> Option<u8> { u8::from_str_radix(s.get(i..i + 2)?, 16).ok() };
        match s.len() {
            6 => Ok(Self {
                r: bayt(0).ok_or_else(|| TokenHata::GecersizRenk(metin.into()))?,
                g: bayt(2).ok_or_else(|| TokenHata::GecersizRenk(metin.into()))?,
                b: bayt(4).ok_or_else(|| TokenHata::GecersizRenk(metin.into()))?,
                a: 255,
            }),
            8 => Ok(Self {
                r: bayt(0).ok_or_else(|| TokenHata::GecersizRenk(metin.into()))?,
                g: bayt(2).ok_or_else(|| TokenHata::GecersizRenk(metin.into()))?,
                b: bayt(4).ok_or_else(|| TokenHata::GecersizRenk(metin.into()))?,
                a: bayt(6).ok_or_else(|| TokenHata::GecersizRenk(metin.into()))?,
            }),
            _ => Err(TokenHata::GecersizRenk(metin.into())),
        }
    }

    /// `#RRGGBB` (alfa 255) veya `#RRGGBBAA` metnine geri çevirir (özel tema kaydı için).
    pub fn hexe(&self) -> String {
        if self.a == 255 {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
        }
    }

    /// sRGB-kodlu bileşeni doğrusal (linear) uzaya çevirir (wgpu clear/shader doğruluğu için).
    fn srgb_dogrusal(c: u8) -> f32 {
        let x = c as f32 / 255.0;
        if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    }

    /// Doğrusal (linear) `[r,g,b,a]` döndürür — wgpu `Color` / 3B shader malzeme rengi için.
    ///
    /// Uygulamanın yüzeyi doğrusal `Bgra8Unorm` olduğundan, sRGB token değeri burada bir kez
    /// doğrusala çevrilir; böylece ekrandaki renk token tablosuyla bire bir eşleşir.
    pub fn dogrusal_f32(&self) -> [f32; 4] {
        [
            Self::srgb_dogrusal(self.r),
            Self::srgb_dogrusal(self.g),
            Self::srgb_dogrusal(self.b),
            self.a as f32 / 255.0,
        ]
    }
}

/// Bir temanın tam anlamsal renk paleti.  Anahtar → [`Renk`] eşlemesi + ergonomik erişimciler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palet {
    renkler: BTreeMap<String, Renk>,
}

impl Palet {
    /// Anahtarla renk getirir.  Anahtar yoksa görünür bir nöbetçi renk (macenta) döner; bu,
    /// eksik token'ın gözden kaçmamasını sağlar (yine de [`TokenDeposu`] yükleme sırasında
    /// eksik anahtarları zaten reddeder, bu yol pratikte tetiklenmez).
    pub fn renk(&self, anahtar: &str) -> Renk {
        self.renkler
            .get(anahtar)
            .copied()
            .unwrap_or(Renk::rgb(0xFF, 0x00, 0xFF))
    }

    /// Bir anahtarın tanımlı olup olmadığı.
    pub fn icerir(&self, anahtar: &str) -> bool {
        self.renkler.contains_key(anahtar)
    }

    /// Tüm (anahtar, renk) çiftleri üzerinde gezinir (özel tema kaydı/denetimi için).
    pub fn girdiler(&self) -> impl Iterator<Item = (&str, Renk)> + '_ {
        self.renkler.iter().map(|(k, v)| (k.as_str(), *v))
    }

    /// Bir anahtarın rengini değiştirir/ekler (özel tema düzenleme — E2).
    pub fn ayarla(&mut self, anahtar: impl Into<String>, renk: Renk) {
        self.renkler.insert(anahtar.into(), renk);
    }

    // ── Çapa erişimciler (Spec 0.8 + sık kullanılan anlamsal anahtarlar) ───────────────
    /// Ana pencere/uygulama zemini.
    pub fn zemin(&self) -> Renk {
        self.renk("bg.primary")
    }
    /// İkincil zemin (panel arkası).
    pub fn zemin_alt(&self) -> Renk {
        self.renk("bg.secondary")
    }
    /// Kart/panel yüzeyi.
    pub fn yuzey(&self) -> Renk {
        self.renk("surface")
    }
    /// İkincil yüzey.
    pub fn yuzey_alt(&self) -> Renk {
        self.renk("surface.alt")
    }
    /// Kenarlık.
    pub fn kenarlik(&self) -> Renk {
        self.renk("border")
    }
    /// Birincil metin.
    pub fn metin(&self) -> Renk {
        self.renk("text.primary")
    }
    /// İkincil/soluk metin.
    pub fn metin_soluk(&self) -> Renk {
        self.renk("text.muted")
    }
    /// Vurgu (accent) dolgu üstündeki metin.
    pub fn vurgu_ustu(&self) -> Renk {
        self.renk("text.on-accent")
    }
    /// Tehlike (error) dolgu üstündeki metin.
    pub fn hata_ustu(&self) -> Renk {
        self.renk("text.on-danger")
    }
    /// Marka/vurgu rengi.
    pub fn vurgu(&self) -> Renk {
        self.renk("accent.primary")
    }
}

/// Yerleşik üç temadan biri ya da kullanıcının özel teması (E2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tema {
    /// Koyu (varsayılan).
    Koyu,
    /// Açık.
    Acik,
    /// Yüksek kontrast (erişilebilirlik).
    YuksekKontrast,
    /// Kullanıcı tanımlı özel tema (kimliği taşır).
    Ozel(String),
}

impl Tema {
    /// JSON'daki tema kimliğinden ([`TokenSeti::kimlik`]) eşleşen yerleşik temayı seçer.
    pub fn kimlikten(kimlik: &str) -> Self {
        match kimlik {
            "koyu" => Tema::Koyu,
            "acik" => Tema::Acik,
            "yuksek_kontrast" => Tema::YuksekKontrast,
            diger => Tema::Ozel(diger.to_string()),
        }
    }

    /// Bu temanın depodaki kimlik dizesi.
    pub fn kimlik(&self) -> &str {
        match self {
            Tema::Koyu => "koyu",
            Tema::Acik => "acik",
            Tema::YuksekKontrast => "yuksek_kontrast",
            Tema::Ozel(k) => k,
        }
    }
}

/// Bir temanın çözülmüş hâli: kimlik + insan-okunur ad + koyu bayrağı + tam palet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenSeti {
    /// Depodaki benzersiz kimlik (örn. "koyu", "yuksek_kontrast", özel: "benim-temam").
    pub kimlik: String,
    /// Ayarlar arayüzünde gösterilecek ad ("Koyu", "Yüksek Kontrast"…).
    pub ad: String,
    /// Bu tema koyu mu (egui taban görünümü + ikon seçimi için).
    pub koyu_mu: bool,
    /// Tam anlamsal renk paleti.
    pub palet: Palet,
}

// ── JSON şeması (yalnızca ayrıştırma; sonra domain tiplerine çevrilir) ──────────────────
#[derive(Deserialize)]
struct DosyaKok {
    #[allow(dead_code)]
    surum: u32,
    temalar: Vec<DosyaTema>,
}

#[derive(Deserialize)]
struct DosyaTema {
    kimlik: String,
    ad: String,
    koyu_mu: bool,
    renkler: BTreeMap<String, String>,
}

/// Tüm temaları tutan ve aktif temayı yöneten depo (tema değiştirme + özel tema kaydet/yükle).
#[derive(Debug, Clone)]
pub struct TokenDeposu {
    setler: Vec<TokenSeti>,
    aktif: usize,
}

impl TokenDeposu {
    /// Gömülü `assets/tokens.json`'dan depoyu kurar (akıllı varsayılan; harici dosya gerekmez).
    ///
    /// # Panik
    /// Gömülü dosya geçersizse panikler — bu bir *derleme-zamanı varlık* hatasıdır, kullanıcı
    /// kaynaklı değil; testler bunu yakalar (kodun parçası bozuksa erken sesli başarısızlık).
    pub fn gomulu() -> Self {
        Self::jsondan(GOMULU_TOKENLAR).expect("gömülü tokens.json geçerli olmalı")
    }

    /// Verilen JSON metninden depo kurar (zorunlu anahtarlar doğrulanır).
    pub fn jsondan(metin: &str) -> Result<Self, TokenHata> {
        let kok: DosyaKok =
            serde_json::from_str(metin).map_err(|e| TokenHata::Ayristirma(e.to_string()))?;
        if kok.temalar.is_empty() {
            return Err(TokenHata::TemaYok);
        }
        let mut setler = Vec::with_capacity(kok.temalar.len());
        for dt in kok.temalar {
            let mut renkler = BTreeMap::new();
            for (anahtar, deger) in &dt.renkler {
                renkler.insert(anahtar.clone(), Renk::hexten(deger)?);
            }
            // MK-52: her tema tüm anlamsal anahtarları tanımlamak zorunda (sessiz eksik renk yok).
            for &gerekli in ANAHTARLAR {
                if !renkler.contains_key(gerekli) {
                    return Err(TokenHata::EksikAnahtar {
                        tema: dt.kimlik.clone(),
                        anahtar: gerekli.to_string(),
                    });
                }
            }
            setler.push(TokenSeti {
                kimlik: dt.kimlik,
                ad: dt.ad,
                koyu_mu: dt.koyu_mu,
                palet: Palet { renkler },
            });
        }
        Ok(Self { setler, aktif: 0 })
    }

    /// Aktif token setini döndürür.
    pub fn aktif(&self) -> &TokenSeti {
        &self.setler[self.aktif]
    }

    /// Aktif temanın [`Tema`] kimliği.
    pub fn aktif_tema(&self) -> Tema {
        Tema::kimlikten(&self.setler[self.aktif].kimlik)
    }

    /// Tüm temaların (kimlik, ad) listesi — ayarlar arayüzü için.
    pub fn temalar(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.setler
            .iter()
            .map(|s| (s.kimlik.as_str(), s.ad.as_str()))
    }

    /// Aktif temayı değiştirir.  Tema bulunamazsa aktif tema değişmez ve `false` döner.
    ///
    /// İşlem O(1)'dir (yalnızca indeks takası) → tema geçişi anında uygulanır (<100 ms,
    /// flicker yok): çağıran bir sonraki karede yeni paleti bir bütün olarak okur.
    pub fn tema_degistir(&mut self, tema: &Tema) -> bool {
        if let Some(i) = self.setler.iter().position(|s| s.kimlik == tema.kimlik()) {
            self.aktif = i;
            true
        } else {
            false
        }
    }

    /// Bir sonraki temaya döngüsel olarak geçer ve yeni aktif temayı döndürür (kısayol/buton).
    pub fn sonraki_tema(&mut self) -> Tema {
        self.aktif = (self.aktif + 1) % self.setler.len();
        self.aktif_tema()
    }

    /// Belirli bir temanın setini (kimlikle) getirir.
    pub fn set(&self, tema: &Tema) -> Option<&TokenSeti> {
        self.setler.iter().find(|s| s.kimlik == tema.kimlik())
    }

    /// **E2 — Özel tema ekle/kaydet.**  Verilen seti depoya ekler (aynı kimlik varsa üzerine
    /// yazar) ve onu aktif yapar.  Böylece kullanıcı kendi temasını oluşturup hemen uygulayabilir.
    pub fn ozel_tema_ekle(&mut self, set: TokenSeti) {
        if let Some(i) = self.setler.iter().position(|s| s.kimlik == set.kimlik) {
            self.setler[i] = set;
            self.aktif = i;
        } else {
            self.setler.push(set);
            self.aktif = self.setler.len() - 1;
        }
    }

    /// **E2 — Özel temayı JSON olarak dışa aktarır** (kullanıcı diske kaydedebilir/paylaşabilir).
    /// Aynı `tokens.json` şemasını (tek temalı) üretir; geri `jsondan` ile yüklenebilir.
    pub fn tema_disa_aktar(&self, tema: &Tema) -> Option<String> {
        let set = self.set(tema)?;
        let mut renk_satirlari: Vec<String> = set
            .palet
            .girdiler()
            .map(|(k, v)| format!("        {:?}: {:?}", k, v.hexe()))
            .collect();
        renk_satirlari.sort();
        Some(format!(
            "{{\n  \"surum\": 1,\n  \"temalar\": [\n    {{\n      \"kimlik\": {:?},\n      \"ad\": {:?},\n      \"koyu_mu\": {},\n      \"renkler\": {{\n{}\n      }}\n    }}\n  ]\n}}\n",
            set.kimlik,
            set.ad,
            set.koyu_mu,
            renk_satirlari.join(",\n")
        ))
    }
}

impl Default for TokenDeposu {
    fn default() -> Self {
        Self::gomulu()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn hex_ayristirma_ve_geri_cevirme() {
        assert_eq!(Renk::hexten("#0A1628").unwrap(), Renk::rgb(10, 22, 40));
        assert_eq!(Renk::hexten("00E5FF").unwrap(), Renk::rgb(0, 229, 255));
        let saydam = Renk::hexten("#11223344").unwrap();
        assert_eq!(
            saydam,
            Renk {
                r: 17,
                g: 34,
                b: 51,
                a: 68
            }
        );
        assert_eq!(saydam.hexe(), "#11223344");
        assert_eq!(Renk::rgb(10, 22, 40).hexe(), "#0A1628");
        assert!(Renk::hexten("#XYZ").is_err());
        assert!(Renk::hexten("#FFF").is_err()); // 3 haneli kısaltma desteklenmiyor
    }

    #[test]
    fn srgb_dogrusal_uc_noktayi_dogru_verir() {
        // Siyah → 0, beyaz → 1; orta gri sRGB doğrusala düşer (gamma).
        assert_eq!(Renk::rgb(0, 0, 0).dogrusal_f32()[0], 0.0);
        assert!((Renk::rgb(255, 255, 255).dogrusal_f32()[0] - 1.0).abs() < 1e-6);
        let orta = Renk::rgb(188, 188, 188).dogrusal_f32()[0];
        assert!(
            orta > 0.45 && orta < 0.55,
            "0xBC sRGB ~0.5 doğrusal olmalı: {orta}"
        );
    }

    #[test]
    fn gomulu_tokenlar_yuklenir_ve_uc_tema_icerir() {
        let d = TokenDeposu::gomulu();
        let kimlikler: Vec<&str> = d.temalar().map(|(k, _)| k).collect();
        assert!(kimlikler.contains(&"koyu"));
        assert!(kimlikler.contains(&"acik"));
        assert!(kimlikler.contains(&"yuksek_kontrast"));
    }

    #[test]
    fn her_tema_tum_zorunlu_anahtarlari_icerir() {
        // MK-52 güvencesi: hiçbir renk eksik değil → hepsi token'dan gelir.
        let d = TokenDeposu::gomulu();
        for (kimlik, _) in d.temalar().collect::<Vec<_>>() {
            let set = d.set(&Tema::kimlikten(kimlik)).unwrap();
            for &a in ANAHTARLAR {
                assert!(
                    set.palet.icerir(a),
                    "{kimlik} teması {a} anahtarını içermeli"
                );
            }
        }
    }

    #[test]
    fn capa_degerleri_spec_0_8_tablosuyla_birebir() {
        let d = TokenDeposu::gomulu();
        let koyu = d.set(&Tema::Koyu).unwrap();
        assert_eq!(koyu.palet.zemin(), Renk::hexten("#0A1628").unwrap());
        assert_eq!(koyu.palet.zemin_alt(), Renk::hexten("#0F1E33").unwrap());
        assert_eq!(koyu.palet.vurgu(), Renk::hexten("#00E5FF").unwrap());
        assert_eq!(koyu.palet.metin(), Renk::hexten("#E6EDF3").unwrap());
        let acik = d.set(&Tema::Acik).unwrap();
        assert_eq!(acik.palet.zemin(), Renk::hexten("#FAFAFA").unwrap());
        assert_eq!(acik.palet.vurgu(), Renk::hexten("#0288D1").unwrap());
        assert_eq!(acik.palet.metin(), Renk::hexten("#1A1A1A").unwrap());
    }

    #[test]
    fn tema_degisimi_farkli_renk_verir_ve_100ms_altinda() {
        // Kabul: tema (Koyu/Açık/Yüksek-kontrast) değişiyor ve <100 ms'de uygulanıyor.
        let mut d = TokenDeposu::gomulu();
        let koyu_zemin = d.aktif().palet.zemin();
        let basla = Instant::now();
        assert!(d.tema_degistir(&Tema::Acik));
        let acik_zemin = d.aktif().palet.zemin();
        assert!(d.tema_degistir(&Tema::YuksekKontrast));
        let yk_zemin = d.aktif().palet.zemin();
        let gecen = basla.elapsed();
        assert!(
            gecen.as_millis() < 100,
            "tema değişimi <100 ms olmalı: {gecen:?}"
        );
        // Üç tema gerçekten farklı zemin verir (token'dan geliyor, sabit değil).
        assert_ne!(koyu_zemin, acik_zemin);
        assert_ne!(acik_zemin, yk_zemin);
    }

    #[test]
    fn bilinmeyen_temaya_gecis_aktifi_degistirmez() {
        let mut d = TokenDeposu::gomulu();
        let onceki = d.aktif_tema();
        assert!(!d.tema_degistir(&Tema::Ozel("yok-boyle".into())));
        assert_eq!(d.aktif_tema().kimlik(), onceki.kimlik());
    }

    #[test]
    fn sonraki_tema_dongusel_gezer() {
        let mut d = TokenDeposu::gomulu();
        let mut gorulen = std::collections::HashSet::new();
        for _ in 0..3 {
            gorulen.insert(d.sonraki_tema().kimlik().to_string());
        }
        assert_eq!(gorulen.len(), 3, "üç temanın hepsi döngüde görülmeli");
    }

    #[test]
    fn ozel_tema_olusturulup_kaydedilip_geri_yuklenir() {
        // E2: kullanıcı kendi temasını oluşturur, kaydeder (JSON) ve geri yükler.
        let mut d = TokenDeposu::gomulu();
        // Koyu temadan türet, vurgu rengini değiştir → "Benim Temam".
        let mut set = d.set(&Tema::Koyu).unwrap().clone();
        set.kimlik = "benim-temam".into();
        set.ad = "Benim Temam".into();
        set.palet.ayarla("accent.primary", Renk::rgb(255, 0, 128));
        d.ozel_tema_ekle(set);
        assert_eq!(d.aktif().kimlik, "benim-temam");
        assert_eq!(d.aktif().palet.vurgu(), Renk::rgb(255, 0, 128));

        // JSON'a aktar → tek başına geçerli bir tokens.json olmalı, geri yüklenebilmeli.
        let json = d
            .tema_disa_aktar(&Tema::Ozel("benim-temam".into()))
            .unwrap();
        let yeniden = TokenDeposu::jsondan(&json).expect("dışa aktarılan tema geri yüklenmeli");
        assert_eq!(yeniden.aktif().palet.vurgu(), Renk::rgb(255, 0, 128));
        assert_eq!(yeniden.aktif().ad, "Benim Temam");
    }

    #[test]
    fn eksik_anahtarli_tema_reddedilir() {
        let bozuk = r##"{ "surum": 1, "temalar": [
            { "kimlik": "x", "ad": "X", "koyu_mu": true, "renkler": { "bg.primary": "#000000" } }
        ] }"##;
        let hata = TokenDeposu::jsondan(bozuk).unwrap_err();
        assert!(matches!(hata, TokenHata::EksikAnahtar { .. }));
    }
}
