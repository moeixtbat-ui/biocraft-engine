//! Ayar **kategorileri**, **değer tipleri**, **doğrulama** ve yerleşik ayar **tanımları** (İP-12).
//!
//! Bu modül egui'ye bağlı **değildir** (saf model, birim-testlenir).  Her ayar bir [`AyarTanimi`]
//! ile tanımlanır: kararlı bir `anahtar`, kategori, değer tipi (+ aralık/seçenekler), **varsayılan**,
//! iki dilde başlık + **açıklama** ve bayraklar (gelişmiş/yeniden-başlat/hassas).  Tanımlar tek
//! doğruluk kaynağıdır; ekran ([`crate::settings`]) onları çizer, kalıcılık onlara göre **doğrular**.
//!
//! **MK-52:** renk/tema değerleri yalnızca token *anahtarı* tutar; somut RGB token deposundan gelir.
//! **Güvenli varsayılan (kabul kriteri):** geçersiz/aralık-dışı bir değer asla uygulanmaz —
//! [`AyarTuru::gecerli_kil`] onu aralığa sıkıştırır veya **varsayılana** düşürür.

use serde::{Deserialize, Serialize};

use crate::i18n::Dil;

/// Ayar ekranının üst düzey kategorileri (sol sütun).  Sıra ekranda da bu sıradır.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AyarKategorisi {
    /// Görünüm + davranış + bildirimler (tema, font, yoğunluk, animasyon…).
    Gorunum,
    /// Kod/metin editörü tercihleri.
    Editor,
    /// Performans + donanım koruması (Eco/Bio/Max, bellek, termal, göstergeler).
    Performans,
    /// Gizlilik + güvenlik (yerel-öncelikli, telemetri, onay) — İP-10.
    Gizlilik,
    /// Klavye kısayolları + tuş seti profili (tam özelleştirme İP-13).
    Kisayollar,
    /// AI yüzeyi (yapılandırılmadı — İP-14); token/maliyet göstergesi.
    Ai,
    /// Eklenti ayarları — her eklenti kendi bölümünü buraya kaydeder (SDK).
    Eklentiler,
    /// Gelişmiş / deneysel ayarlar (uyarılı; kazara bozulmaz).
    Gelismis,
}

impl AyarKategorisi {
    /// Tüm kategoriler, ekran sırasıyla.
    pub const TUMU: &'static [AyarKategorisi] = &[
        AyarKategorisi::Gorunum,
        AyarKategorisi::Editor,
        AyarKategorisi::Performans,
        AyarKategorisi::Gizlilik,
        AyarKategorisi::Kisayollar,
        AyarKategorisi::Ai,
        AyarKategorisi::Eklentiler,
        AyarKategorisi::Gelismis,
    ];

    /// Kategorinin küçük ikonu (sol sütun).
    pub fn ikon(self) -> &'static str {
        match self {
            AyarKategorisi::Gorunum => "🎨",
            AyarKategorisi::Editor => "📝",
            AyarKategorisi::Performans => "⚡",
            AyarKategorisi::Gizlilik => "🔒",
            AyarKategorisi::Kisayollar => "⌨",
            AyarKategorisi::Ai => "✨",
            AyarKategorisi::Eklentiler => "🧩",
            AyarKategorisi::Gelismis => "⚙",
        }
    }

    /// Kategorinin yerelleştirilmiş başlığı.
    pub fn baslik(self, dil: Dil) -> &'static str {
        use AyarKategorisi::*;
        use Dil::{En, Tr};
        match (self, dil) {
            (Gorunum, Tr) => "Görünüm",
            (Gorunum, En) => "Appearance",
            (Editor, Tr) => "Editör",
            (Editor, En) => "Editor",
            (Performans, Tr) => "Performans & Donanım",
            (Performans, En) => "Performance & Hardware",
            (Gizlilik, Tr) => "Gizlilik & Güvenlik",
            (Gizlilik, En) => "Privacy & Security",
            (Kisayollar, Tr) => "Kısayollar",
            (Kisayollar, En) => "Shortcuts",
            (Ai, Tr) => "AI",
            (Ai, En) => "AI",
            (Eklentiler, Tr) => "Eklentiler",
            (Eklentiler, En) => "Plugins",
            (Gelismis, Tr) => "Gelişmiş",
            (Gelismis, En) => "Advanced",
        }
    }
}

/// Bir seçim (Secim) ayarının tek seçeneği — kararlı `anahtar` + iki dilde etiket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecimSecenegi {
    /// Kararlı, kalıcı anahtar (diske bu yazılır; etiketten bağımsız).
    pub anahtar: String,
    /// Türkçe etiket.
    pub etiket_tr: String,
    /// İngilizce etiket.
    pub etiket_en: String,
}

impl SecimSecenegi {
    /// Yeni seçenek.
    pub fn yeni(
        anahtar: impl Into<String>,
        etiket_tr: impl Into<String>,
        etiket_en: impl Into<String>,
    ) -> Self {
        Self {
            anahtar: anahtar.into(),
            etiket_tr: etiket_tr.into(),
            etiket_en: etiket_en.into(),
        }
    }

    /// Yerelleştirilmiş etiket.
    pub fn etiket(&self, dil: Dil) -> &str {
        match dil {
            Dil::Tr => &self.etiket_tr,
            Dil::En => &self.etiket_en,
        }
    }
}

/// Bir ayarın **değeri** — kalıcılık/profil JSON'ında bu biçimde saklanır.
///
/// `#[serde(tag = "t", content = "v")]`: okunaklı + ileri-uyumlu (`{"t":"Mantik","v":true}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", content = "v")]
pub enum AyarDeger {
    /// Açık/kapalı (checkbox).
    Mantik(bool),
    /// Tam sayı (slider/sürükle).
    TamSayi(i64),
    /// Ondalık sayı (slider).
    Ondalik(f64),
    /// Serbest metin (örn. API anahtarı).
    Metin(String),
    /// Bir seçim anahtarı (radio/açılır liste).
    Secim(String),
}

impl AyarDeger {
    /// `bool` değerini okur (değilse `None`).
    pub fn mantik(&self) -> Option<bool> {
        match self {
            AyarDeger::Mantik(b) => Some(*b),
            _ => None,
        }
    }

    /// `i64` değerini okur (değilse `None`).
    pub fn tam_sayi(&self) -> Option<i64> {
        match self {
            AyarDeger::TamSayi(n) => Some(*n),
            _ => None,
        }
    }

    /// `f64` değerini okur (Tam sayı da kabul edilir; değilse `None`).
    pub fn ondalik(&self) -> Option<f64> {
        match self {
            AyarDeger::Ondalik(f) => Some(*f),
            AyarDeger::TamSayi(n) => Some(*n as f64),
            _ => None,
        }
    }

    /// Metin/Seçim değerini okur (değilse `None`).
    pub fn metin(&self) -> Option<&str> {
        match self {
            AyarDeger::Metin(s) | AyarDeger::Secim(s) => Some(s),
            _ => None,
        }
    }
}

/// Bir ayarın **tipi** + doğrulama/aralık/seçenek meta verisi (ekran widget'ını da belirler).
#[derive(Debug, Clone, PartialEq)]
pub enum AyarTuru {
    /// Açık/kapalı.
    Mantik,
    /// Tam sayı, `[min, max]` aralığında, `adim` çözünürlükle.
    TamSayi { min: i64, max: i64, adim: i64 },
    /// Ondalık, `[min, max]` aralığında.
    Ondalik { min: f64, max: f64, adim: f64 },
    /// Serbest metin, `azami_uzunluk` karaktere kadar.
    Metin { azami_uzunluk: usize },
    /// Sabit seçenekler arasından bir seçim.
    Secim { secenekler: Vec<SecimSecenegi> },
}

impl AyarTuru {
    /// Bir değeri bu tip için **güvenli** hâle getirir: aralığa sıkıştırır, tip uymazsa veya
    /// seçenek tanınmazsa `varsayilan`'a düşer.  **Hiçbir geçersiz değer uygulanmaz** (kabul kriteri).
    pub fn gecerli_kil(&self, deger: &AyarDeger, varsayilan: &AyarDeger) -> AyarDeger {
        match self {
            AyarTuru::Mantik => match deger.mantik() {
                Some(b) => AyarDeger::Mantik(b),
                None => varsayilan.clone(),
            },
            AyarTuru::TamSayi { min, max, .. } => match deger.tam_sayi().or_else(|| {
                // Ondalık geldiyse yuvarlayarak kurtar (ör. profil/elle düzenleme toleransı).
                deger.ondalik().map(|f| f.round() as i64)
            }) {
                Some(n) => AyarDeger::TamSayi(n.clamp(*min, *max)),
                None => varsayilan.clone(),
            },
            AyarTuru::Ondalik { min, max, .. } => match deger.ondalik() {
                Some(f) if f.is_finite() => AyarDeger::Ondalik(f.clamp(*min, *max)),
                _ => varsayilan.clone(),
            },
            AyarTuru::Metin { azami_uzunluk } => match deger.metin() {
                Some(s) => AyarDeger::Metin(s.chars().take(*azami_uzunluk).collect()),
                None => varsayilan.clone(),
            },
            AyarTuru::Secim { secenekler } => match deger.metin() {
                Some(s) if secenekler.iter().any(|o| o.anahtar == s) => {
                    AyarDeger::Secim(s.to_string())
                }
                // Tanınmayan/eksik seçenek → güvenli varsayılan.
                _ => varsayilan.clone(),
            },
        }
    }
}

/// Tek bir ayarın tam tanımı (tek doğruluk kaynağı).
#[derive(Debug, Clone, PartialEq)]
pub struct AyarTanimi {
    /// Kararlı, noktayla ayrılmış kimlik (örn. `"gorunum.tema"`); diske/profile bu yazılır.
    pub anahtar: String,
    /// Ait olduğu kategori.
    pub kategori: AyarKategorisi,
    /// Değer tipi + doğrulama meta verisi.
    pub tur: AyarTuru,
    /// Akıllı varsayılan (her zaman geçerli olmak zorunda).
    pub varsayilan: AyarDeger,
    /// Türkçe başlık.
    pub baslik_tr: String,
    /// İngilizce başlık.
    pub baslik_en: String,
    /// Türkçe açıklama (her ayarın açıklaması olmalı — kabul kriteri).
    pub aciklama_tr: String,
    /// İngilizce açıklama.
    pub aciklama_en: String,
    /// Gelişmiş/deneysel mi? (ayrı, uyarılı bölüm.)
    pub gelismis: bool,
    /// Değişiklik yeniden başlatma gerektirir mi? (net işaret.)
    pub yeniden_baslat: bool,
    /// Hassas mı? (API anahtarı gibi — profil dışa aktarımına **dahil edilmez**.)
    pub hassas: bool,
    /// Aramayı zenginleştiren ek anahtar kelimeler (görünmez; yalnız indeks).
    pub anahtar_kelimeler: String,
}

impl AyarTanimi {
    /// Çekirdek kurucu (genelde tip-özel yardımcılar üzerinden çağrılır).
    #[allow(clippy::too_many_arguments)]
    fn yeni(
        anahtar: impl Into<String>,
        kategori: AyarKategorisi,
        tur: AyarTuru,
        varsayilan: AyarDeger,
        baslik_tr: impl Into<String>,
        baslik_en: impl Into<String>,
        aciklama_tr: impl Into<String>,
        aciklama_en: impl Into<String>,
    ) -> Self {
        Self {
            anahtar: anahtar.into(),
            kategori,
            tur,
            varsayilan,
            baslik_tr: baslik_tr.into(),
            baslik_en: baslik_en.into(),
            aciklama_tr: aciklama_tr.into(),
            aciklama_en: aciklama_en.into(),
            gelismis: false,
            yeniden_baslat: false,
            hassas: false,
            anahtar_kelimeler: String::new(),
        }
    }

    /// Bu ayarı **gelişmiş** olarak işaretler.
    fn gelismis(mut self) -> Self {
        self.gelismis = true;
        self
    }

    /// Bu ayarı **yeniden başlatma gerektirir** olarak işaretler.
    fn yeniden_baslat(mut self) -> Self {
        self.yeniden_baslat = true;
        self
    }

    /// Bu ayarı **hassas** olarak işaretler (profil dışa aktarımından çıkarılır).
    fn hassas(mut self) -> Self {
        self.hassas = true;
        self
    }

    /// Aramaya ek anahtar kelimeler ekler.
    fn kelimeler(mut self, k: impl Into<String>) -> Self {
        self.anahtar_kelimeler = k.into();
        self
    }

    /// **Dış (eklenti) tanımı** için public kurucu (SDK akışı).
    ///
    /// Eklentiler kendi ayarlarını bu kurucuyla ilan eder; kategori
    /// [`crate::settings::AyarKayit::eklenti_ayari_kaydet`] içinde **Eklentiler**'e zorlanır.
    /// İnce ayar (gelişmiş/hassas/anahtar-kelime) için `with_*` yardımcıları kullanılır.
    #[allow(clippy::too_many_arguments)]
    pub fn yeni_dis(
        anahtar: impl Into<String>,
        kategori: AyarKategorisi,
        tur: AyarTuru,
        varsayilan: AyarDeger,
        baslik_tr: impl Into<String>,
        baslik_en: impl Into<String>,
        aciklama_tr: impl Into<String>,
        aciklama_en: impl Into<String>,
    ) -> Self {
        Self::yeni(
            anahtar,
            kategori,
            tur,
            varsayilan,
            baslik_tr,
            baslik_en,
            aciklama_tr,
            aciklama_en,
        )
    }

    /// Eklenti tanımını **gelişmiş** işaretler (zincirlenebilir).
    pub fn with_gelismis(self) -> Self {
        self.gelismis()
    }

    /// Eklenti tanımını **hassas** işaretler (profil dışa aktarımından çıkar).
    pub fn with_hassas(self) -> Self {
        self.hassas()
    }

    /// Eklenti tanımına arama anahtar kelimeleri ekler.
    pub fn with_kelimeler(self, k: impl Into<String>) -> Self {
        self.kelimeler(k)
    }

    /// Yerelleştirilmiş başlık.
    pub fn baslik(&self, dil: Dil) -> &str {
        match dil {
            Dil::Tr => &self.baslik_tr,
            Dil::En => &self.baslik_en,
        }
    }

    /// Yerelleştirilmiş açıklama.
    pub fn aciklama(&self, dil: Dil) -> &str {
        match dil {
            Dil::Tr => &self.aciklama_tr,
            Dil::En => &self.aciklama_en,
        }
    }

    /// Bir değeri bu ayar için güvenli hâle getirir (tip + varsayılan).
    pub fn gecerli_kil(&self, deger: &AyarDeger) -> AyarDeger {
        self.tur.gecerli_kil(deger, &self.varsayilan)
    }
}

// ─── Tip-özel kurucu yardımcıları (yerleşik tanımları okunaklı tutar) ──────────

#[allow(clippy::too_many_arguments)]
fn mantik(
    anahtar: &str,
    kategori: AyarKategorisi,
    varsayilan: bool,
    bt: &str,
    be: &str,
    at: &str,
    ae: &str,
) -> AyarTanimi {
    AyarTanimi::yeni(
        anahtar,
        kategori,
        AyarTuru::Mantik,
        AyarDeger::Mantik(varsayilan),
        bt,
        be,
        at,
        ae,
    )
}

#[allow(clippy::too_many_arguments)]
fn tam(
    anahtar: &str,
    kategori: AyarKategorisi,
    min: i64,
    max: i64,
    adim: i64,
    varsayilan: i64,
    bt: &str,
    be: &str,
    at: &str,
    ae: &str,
) -> AyarTanimi {
    AyarTanimi::yeni(
        anahtar,
        kategori,
        AyarTuru::TamSayi { min, max, adim },
        AyarDeger::TamSayi(varsayilan),
        bt,
        be,
        at,
        ae,
    )
}

#[allow(clippy::too_many_arguments)]
fn ondalik(
    anahtar: &str,
    kategori: AyarKategorisi,
    min: f64,
    max: f64,
    adim: f64,
    varsayilan: f64,
    bt: &str,
    be: &str,
    at: &str,
    ae: &str,
) -> AyarTanimi {
    AyarTanimi::yeni(
        anahtar,
        kategori,
        AyarTuru::Ondalik { min, max, adim },
        AyarDeger::Ondalik(varsayilan),
        bt,
        be,
        at,
        ae,
    )
}

#[allow(clippy::too_many_arguments)]
fn metin(
    anahtar: &str,
    kategori: AyarKategorisi,
    azami_uzunluk: usize,
    varsayilan: &str,
    bt: &str,
    be: &str,
    at: &str,
    ae: &str,
) -> AyarTanimi {
    AyarTanimi::yeni(
        anahtar,
        kategori,
        AyarTuru::Metin { azami_uzunluk },
        AyarDeger::Metin(varsayilan.to_string()),
        bt,
        be,
        at,
        ae,
    )
}

#[allow(clippy::too_many_arguments)]
fn secim(
    anahtar: &str,
    kategori: AyarKategorisi,
    secenekler: Vec<SecimSecenegi>,
    varsayilan: &str,
    bt: &str,
    be: &str,
    at: &str,
    ae: &str,
) -> AyarTanimi {
    debug_assert!(
        secenekler.iter().any(|o| o.anahtar == varsayilan),
        "seçim varsayılanı seçenekler arasında olmalı: {anahtar}"
    );
    AyarTanimi::yeni(
        anahtar,
        kategori,
        AyarTuru::Secim { secenekler },
        AyarDeger::Secim(varsayilan.to_string()),
        bt,
        be,
        at,
        ae,
    )
}

/// Kısa seçenek kurucu.
fn sec(anahtar: &str, tr: &str, en: &str) -> SecimSecenegi {
    SecimSecenegi::yeni(anahtar, tr, en)
}

// ─── Yerleşik ayar tanımları ──────────────────────────────────────────────────

/// Çekirdeğin sunduğu tüm yerleşik ayar tanımları (kategori sırasıyla).
///
/// Eklentiler bu listeye [`crate::settings::AyarKayit::eklenti_ayari_kaydet`] ile **Eklentiler**
/// kategorisinde kendi ayarlarını ekler (SDK).  Bir kez (kayıt kurulurken) üretilir; çalışma
/// zamanı get/set BTreeMap üzerinde olduğundan bu tek seferlik kurulum açılışı yavaşlatmaz.
pub fn yerlesik_tanimlar() -> Vec<AyarTanimi> {
    use AyarKategorisi as K;
    vec![
        // ── Görünüm ──
        secim(
            "gorunum.tema",
            K::Gorunum,
            vec![
                sec("koyu", "Koyu", "Dark"),
                sec("acik", "Açık", "Light"),
                sec("yuksek_kontrast", "Yüksek Kontrast", "High Contrast"),
            ],
            "koyu",
            "Tema",
            "Theme",
            "Arayüz renk teması. Yüksek kontrast erişilebilirlik içindir.",
            "Interface color theme. High contrast is for accessibility.",
        )
        .kelimeler("renk color koyu acik dark light"),
        secim(
            "gorunum.dil",
            K::Gorunum,
            vec![sec("tr", "Türkçe", "Turkish"), sec("en", "İngilizce", "English")],
            "tr",
            "Dil",
            "Language",
            "Arayüz dili. Tarih/sayı biçimi de bu seçime göre ayarlanır.",
            "Interface language. Date/number format follows this choice.",
        )
        .kelimeler("language locale tr en türkçe english"),
        secim(
            "gorunum.font_ailesi",
            K::Gorunum,
            vec![
                sec("sistem", "Sistem", "System"),
                sec("monospace", "Tek Aralık", "Monospace"),
                sec("serif", "Tırnaklı", "Serif"),
            ],
            "sistem",
            "Yazı tipi ailesi",
            "Font family",
            "Arayüz genel yazı tipi ailesi.",
            "General interface font family.",
        )
        .kelimeler("font yazıtipi typeface"),
        tam(
            "gorunum.font_boyutu",
            K::Gorunum,
            10,
            24,
            1,
            14,
            "Yazı tipi boyutu",
            "Font size",
            "Arayüz yazı boyutu (punto). Anında uygulanır.",
            "Interface font size (pt). Applied instantly.",
        )
        .kelimeler("font boyut size punto"),
        secim(
            "gorunum.arac_cubugu_boyutu",
            K::Gorunum,
            vec![
                sec("kucuk", "Küçük", "Small"),
                sec("orta", "Orta", "Medium"),
                sec("buyuk", "Büyük", "Large"),
            ],
            "orta",
            "Araç çubuğu boyutu",
            "Toolbar size",
            "Üst araç çubuğu düğme/ikon boyutu.",
            "Top toolbar button/icon size.",
        )
        .kelimeler("toolbar ikon icon"),
        secim(
            "gorunum.panel_yogunlugu",
            K::Gorunum,
            vec![
                sec("sade", "Sade", "Compact"),
                sec("normal", "Normal", "Normal"),
                sec("yogun", "Yoğun", "Dense"),
            ],
            "normal",
            "Panel yoğunluğu",
            "Panel density",
            "Panel ve listelerdeki boşluk miktarı. Yoğun = daha çok bilgi, az boşluk.",
            "Spacing in panels and lists. Dense = more info, less spacing.",
        )
        .kelimeler("density boşluk spacing yoğun"),
        ondalik(
            "gorunum.animasyon_hizi",
            K::Gorunum,
            0.0,
            2.0,
            0.1,
            1.0,
            "Animasyon hızı",
            "Animation speed",
            "Geçiş animasyonlarının hız çarpanı. 0 = animasyonları kapat.",
            "Speed multiplier for transition animations. 0 = disable animations.",
        )
        .kelimeler("animation motion hareket"),
        // Bildirim ayrıntı düzeyi — tür bazında ayrı kısılabilir (İP-12 + toast İP-16).
        secim(
            "bildirim.basari",
            K::Gorunum,
            bildirim_secenekleri(),
            "normal",
            "Bildirim: Başarı",
            "Notification: Success",
            "Başarı bildirimlerinin ayrıntı düzeyi. Kapalı = hiç gösterme.",
            "Detail level of success notifications. Off = never show.",
        )
        .kelimeler("notification toast bildirim başarı"),
        secim(
            "bildirim.uyari",
            K::Gorunum,
            bildirim_secenekleri(),
            "normal",
            "Bildirim: Uyarı",
            "Notification: Warning",
            "Uyarı bildirimlerinin ayrıntı düzeyi.",
            "Detail level of warning notifications.",
        )
        .kelimeler("notification toast bildirim uyarı"),
        secim(
            "bildirim.hata",
            K::Gorunum,
            bildirim_secenekleri(),
            "normal",
            "Bildirim: Hata",
            "Notification: Error",
            "Hata bildirimlerinin ayrıntı düzeyi. (Hatalar günlüğe her zaman yazılır.)",
            "Detail level of error notifications. (Errors are always logged.)",
        )
        .kelimeler("notification toast bildirim hata"),
        secim(
            "bildirim.bilgi",
            K::Gorunum,
            bildirim_secenekleri(),
            "normal",
            "Bildirim: Bilgi",
            "Notification: Info",
            "Bilgilendirme bildirimlerinin ayrıntı düzeyi.",
            "Detail level of informational notifications.",
        )
        .kelimeler("notification toast bildirim bilgi"),
        // ── Editör ──
        mantik(
            "editor.satir_numaralari",
            K::Editor,
            true,
            "Satır numaraları",
            "Line numbers",
            "Kod editöründe sol kenarda satır numaralarını göster.",
            "Show line numbers in the code editor gutter.",
        ),
        mantik(
            "editor.sozcuk_kaydir",
            K::Editor,
            false,
            "Sözcük kaydır",
            "Word wrap",
            "Uzun satırları pencereye sığacak şekilde alt satıra kaydır.",
            "Wrap long lines to fit the window width.",
        ),
        tam(
            "editor.sekme_genisligi",
            K::Editor,
            2,
            8,
            1,
            4,
            "Sekme genişliği",
            "Tab width",
            "Bir sekmenin (Tab) kaç boşluğa karşılık geldiği.",
            "How many spaces a tab corresponds to.",
        )
        .kelimeler("tab indent girinti"),
        tam(
            "editor.yazi_boyutu",
            K::Editor,
            10,
            24,
            1,
            13,
            "Editör yazı boyutu",
            "Editor font size",
            "Yalnızca kod/metin editörünün yazı boyutu (punto).",
            "Font size (pt) for the code/text editor only.",
        )
        .kelimeler("font boyut size"),
        tam(
            "editor.otomatik_kayit_saniye",
            K::Editor,
            0,
            600,
            5,
            30,
            "Otomatik kayıt sıklığı (sn)",
            "Auto-save interval (s)",
            "Değişiklikler kaç saniyede bir otomatik kaydedilsin. 0 = otomatik kayıt kapalı.",
            "How often changes are auto-saved, in seconds. 0 = disable auto-save.",
        )
        .kelimeler("autosave kayıt save"),
        mantik(
            "editor.kaydet_bicimlendir",
            K::Editor,
            false,
            "Kaydederken biçimlendir",
            "Format on save",
            "Dosya kaydedilince kodu otomatik biçimlendir (ruff/black — araç kuruluysa).",
            "Auto-format code when a file is saved (ruff/black — if the tool is installed).",
        )
        .kelimeler("format ruff black"),
        // ── Performans & Donanım ──
        secim(
            "performans.mod",
            K::Performans,
            vec![
                sec("eco", "Eco (sessiz/serin)", "Eco (quiet/cool)"),
                sec("bio", "Bio (dengeli)", "Bio (balanced)"),
                sec("max", "Max (tam güç)", "Max (full power)"),
            ],
            "bio",
            "Performans modu",
            "Performance mode",
            "Eco daha sessiz/serin çalışır; Max tüm gücü kullanır. Bio dengeli varsayılandır.",
            "Eco runs quieter/cooler; Max uses full power. Bio is the balanced default.",
        )
        .kelimeler("eco bio max güç power"),
        mantik(
            "performans.fps_goster",
            K::Performans,
            false,
            "FPS göstergesi",
            "FPS indicator",
            "Durum çubuğunda saniyedeki kare sayısını (FPS) göster.",
            "Show frames-per-second (FPS) in the status bar.",
        )
        .kelimeler("fps kare framerate gösterge"),
        mantik(
            "performans.bellek_goster",
            K::Performans,
            false,
            "Bellek göstergesi",
            "Memory indicator",
            "Durum çubuğunda kullanılan bellek miktarını göster.",
            "Show used memory in the status bar.",
        )
        .kelimeler("memory ram bellek gösterge"),
        mantik(
            "performans.sicaklik_goster",
            K::Performans,
            false,
            "Sıcaklık göstergesi",
            "Temperature indicator",
            "Durum çubuğunda GPU/işlemci sıcaklığını göster (donanım destekliyorsa).",
            "Show GPU/CPU temperature in the status bar (if hardware supports it).",
        )
        .kelimeler("temperature sıcaklık termal gösterge"),
        tam(
            "performans.bellek_limiti_mb",
            K::Performans,
            256,
            262_144,
            256,
            4096,
            "Bellek bütçesi (MB)",
            "Memory budget (MB)",
            "Uygulamanın kullanabileceği üst bellek sınırı. Aşılırsa akış (stream) moduna geçilir.",
            "Upper memory limit the app may use. Beyond it, stream mode kicks in.",
        )
        .kelimeler("memory ram bütçe budget limit"),
        mantik(
            "performans.gpu_hizlandirma",
            K::Performans,
            true,
            "GPU hızlandırma",
            "GPU acceleration",
            "Çizim ve hesaplama için GPU'yu kullan. Kapalıyken yalnızca işlemci kullanılır.",
            "Use the GPU for rendering and compute. When off, only the CPU is used.",
        )
        .yeniden_baslat()
        .kelimeler("gpu wgpu hızlandırma"),
        tam(
            "performans.termal_esik_c",
            K::Performans,
            60,
            95,
            1,
            85,
            "Termal eşik (°C)",
            "Thermal threshold (°C)",
            "Bu sıcaklığa ulaşılınca koruma devreye girer (işi yavaşlatır/duraklatır).",
            "Protection engages at this temperature (throttles/pauses work).",
        )
        .gelismis()
        .kelimeler("thermal sıcaklık termal watchdog"),
        // ── Gizlilik & Güvenlik ──
        mantik(
            "gizlilik.tamamen_yerel",
            K::Gizlilik,
            true,
            "Tamamen yerel çalış",
            "Fully local",
            "Hiçbir veri dışarı gönderilmez; tüm dış kanallar kapalı kalır (en güvenli).",
            "No data leaves the device; all external channels stay closed (most private).",
        )
        .kelimeler("privacy yerel local offline"),
        secim(
            "gizlilik.telemetri",
            K::Gizlilik,
            vec![
                sec("kapali", "Kapalı", "Off"),
                sec("minimal", "Minimal (anonim)", "Minimal (anonymous)"),
                sec("tam", "Tam", "Full"),
            ],
            "kapali",
            "Telemetri",
            "Telemetry",
            "Kullanım istatistiği paylaşımı. Varsayılan kapalı; PHI asla gönderilmez (İP-10).",
            "Usage statistics sharing. Off by default; PHI is never sent (İP-10).",
        )
        .kelimeler("telemetry istatistik analytics"),
        mantik(
            "gizlilik.ai_havuzu_katki",
            K::Gizlilik,
            false,
            "AI havuzuna katkı",
            "Contribute to AI pool",
            "Anonim verilerin ortak AI eğitim havuzuna katılması. Varsayılan kapalı.",
            "Sharing anonymized data with a common AI training pool. Off by default.",
        )
        .kelimeler("ai havuz pool katkı"),
        mantik(
            "gizlilik.her_dis_gonderim_onay",
            K::Gizlilik,
            true,
            "Her dış gönderimde onay iste",
            "Confirm every outbound send",
            "Veri dışarı gönderilmeden önce her seferinde onay iste (şeffaflık).",
            "Ask for confirmation before any data is sent out (transparency).",
        )
        .kelimeler("onay consent confirm"),
        // ── Kısayollar ──
        secim(
            "kisayol.tus_seti",
            K::Kisayollar,
            vec![sec("modern", "Modern", "Modern")],
            "modern",
            "Tuş seti profili",
            "Keymap profile",
            "Klavye kısayol seti. Tam özelleştirme + Vim/Emacs İP-13 ile gelir (şimdilik Modern).",
            "Keyboard shortcut set. Full customization + Vim/Emacs comes with İP-13 (Modern for now).",
        )
        .kelimeler("keymap vim emacs kısayol shortcut"),
        // ── AI ──
        mantik(
            "ai.etkin",
            K::Ai,
            false,
            "AI'ı etkinleştir",
            "Enable AI",
            "AI yüzeyini aç. Kapalıyken arayüz sadeleşir, uygulama tam çalışır (MK-48).",
            "Turn on the AI surface. When off the UI simplifies and the app works fully (MK-48).",
        )
        .kelimeler("ai yapay zeka"),
        mantik(
            "ai.token_sayaci_goster",
            K::Ai,
            false,
            "Token sayacını göster",
            "Show token counter",
            "Anlık token sayısı göstergesini durum çubuğunda göster (AI etkinken).",
            "Show the live token-count indicator in the status bar (when AI is on).",
        )
        .kelimeler("token sayaç counter gösterge"),
        mantik(
            "ai.maliyet_goster",
            K::Ai,
            false,
            "Maliyet göstergesini göster",
            "Show cost indicator",
            "Anlık tahmini maliyet göstergesini göster (AI etkinken).",
            "Show the live estimated-cost indicator (when AI is on).",
        )
        .kelimeler("cost maliyet ücret"),
        secim(
            "ai.saglayici",
            K::Ai,
            vec![sec("yapilandirilmadi", "Yapılandırılmadı", "Not configured")],
            "yapilandirilmadi",
            "AI sağlayıcı",
            "AI provider",
            "AI sağlayıcısı. Bu sürümde yapılandırılmadı; sağlayıcı/model İP-14 ile eklenir.",
            "AI provider. Not configured in this version; provider/model added with İP-14.",
        )
        .kelimeler("provider model sağlayıcı"),
        metin(
            "ai.api_anahtari",
            K::Ai,
            256,
            "",
            "API anahtarı",
            "API key",
            "AI sağlayıcı API anahtarı. Hassastır: profile/yedeğe AKTARILMAZ, OS kasasında saklanır.",
            "AI provider API key. Sensitive: NOT exported to profiles/backups, kept in the OS vault.",
        )
        .hassas()
        .kelimeler("api key anahtar secret"),
        // ── Eklentiler ──
        mantik(
            "eklenti.otomatik_guncelle",
            K::Eklentiler,
            true,
            "Eklentileri otomatik güncelle",
            "Auto-update plugins",
            "İmzalı eklenti güncellemelerini otomatik kur (MK-16; imzasız güncelleme reddedilir).",
            "Auto-install signed plugin updates (MK-16; unsigned updates are rejected).",
        )
        .kelimeler("plugin eklenti güncelleme update"),
        // ── Gelişmiş ──
        mantik(
            "gelismis.deneysel_ozellikler",
            K::Gelismis,
            false,
            "Deneysel özellikler",
            "Experimental features",
            "Yarım/kararsız özellikleri aç. Beklenmedik davranışlara yol açabilir — dikkatli olun.",
            "Enable half-baked/unstable features. May cause unexpected behavior — use with care.",
        )
        .gelismis()
        .kelimeler("experimental deneysel beta"),
        secim(
            "gelismis.gunluk_duzeyi",
            K::Gelismis,
            vec![
                sec("hata", "Hata", "Error"),
                sec("uyari", "Uyarı", "Warning"),
                sec("bilgi", "Bilgi", "Info"),
                sec("ayiklama", "Ayıklama", "Debug"),
            ],
            "bilgi",
            "Günlük düzeyi",
            "Log level",
            "Günlüğe yazılan ayrıntı düzeyi. Ayıklama, sorun bildirirken faydalıdır.",
            "Detail level written to the log. Debug is useful when reporting issues.",
        )
        .gelismis()
        .kelimeler("log günlük debug verbose"),
    ]
}

/// Bildirim ayrıntı düzeyi seçenekleri (tür bazında kullanılır).
fn bildirim_secenekleri() -> Vec<SecimSecenegi> {
    vec![
        sec("kapali", "Kapalı", "Off"),
        sec("sessiz", "Sessiz (rozet)", "Silent (badge)"),
        sec("normal", "Normal", "Normal"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yerlesik_tanimlar_dolu_ve_benzersiz() {
        let t = yerlesik_tanimlar();
        assert!(
            t.len() >= 30,
            "kapsamlı bir ayar seti bekleniyor: {}",
            t.len()
        );
        // Anahtarlar benzersiz olmalı (çakışma kalıcılığı bozar).
        let mut anahtarlar: Vec<&str> = t.iter().map(|d| d.anahtar.as_str()).collect();
        anahtarlar.sort_unstable();
        let onceki = anahtarlar.len();
        anahtarlar.dedup();
        assert_eq!(onceki, anahtarlar.len(), "yinelenen ayar anahtarı var");
    }

    #[test]
    fn her_ayarin_iki_dilde_basligi_ve_aciklamasi_var() {
        // Kabul kriteri: her ayarın açıklaması olmalı.
        for d in yerlesik_tanimlar() {
            assert!(
                !d.baslik(Dil::Tr).is_empty(),
                "TR başlık boş: {}",
                d.anahtar
            );
            assert!(
                !d.baslik(Dil::En).is_empty(),
                "EN başlık boş: {}",
                d.anahtar
            );
            assert!(
                !d.aciklama(Dil::Tr).is_empty(),
                "TR açıklama boş: {}",
                d.anahtar
            );
            assert!(
                !d.aciklama(Dil::En).is_empty(),
                "EN açıklama boş: {}",
                d.anahtar
            );
        }
    }

    #[test]
    fn varsayilanlar_kendi_tiplerine_gore_gecerli() {
        // Her varsayılan, gecerli_kil'den değişmeden geçmeli (zaten geçerli olmalı).
        for d in yerlesik_tanimlar() {
            let g = d.gecerli_kil(&d.varsayilan);
            assert_eq!(g, d.varsayilan, "varsayılan geçersiz: {}", d.anahtar);
        }
    }

    #[test]
    fn tam_sayi_araliga_sikistirilir() {
        let d = tam("x", AyarKategorisi::Editor, 2, 8, 1, 4, "b", "b", "a", "a");
        assert_eq!(
            d.gecerli_kil(&AyarDeger::TamSayi(99)),
            AyarDeger::TamSayi(8)
        );
        assert_eq!(
            d.gecerli_kil(&AyarDeger::TamSayi(-5)),
            AyarDeger::TamSayi(2)
        );
        // Tip uymazsa varsayılana düşer.
        assert_eq!(
            d.gecerli_kil(&AyarDeger::Mantik(true)),
            AyarDeger::TamSayi(4)
        );
    }

    #[test]
    fn ondalik_nan_varsayilana_duser() {
        let d = ondalik(
            "x",
            AyarKategorisi::Gorunum,
            0.0,
            2.0,
            0.1,
            1.0,
            "b",
            "b",
            "a",
            "a",
        );
        assert_eq!(
            d.gecerli_kil(&AyarDeger::Ondalik(f64::NAN)),
            AyarDeger::Ondalik(1.0)
        );
        assert_eq!(
            d.gecerli_kil(&AyarDeger::Ondalik(9.9)),
            AyarDeger::Ondalik(2.0)
        );
    }

    #[test]
    fn secim_taninmayan_anahtar_varsayilana_duser() {
        let d = secim(
            "x",
            AyarKategorisi::Gorunum,
            vec![sec("a", "A", "A"), sec("b", "B", "B")],
            "a",
            "b",
            "b",
            "a",
            "a",
        );
        assert_eq!(
            d.gecerli_kil(&AyarDeger::Secim("b".into())),
            AyarDeger::Secim("b".into())
        );
        // Tanınmayan seçenek → varsayılan "a".
        assert_eq!(
            d.gecerli_kil(&AyarDeger::Secim("zzz".into())),
            AyarDeger::Secim("a".into())
        );
    }

    #[test]
    fn metin_azami_uzunluga_kirpilir() {
        let d = metin("x", AyarKategorisi::Ai, 5, "", "b", "b", "a", "a");
        assert_eq!(
            d.gecerli_kil(&AyarDeger::Metin("merhaba".into())),
            AyarDeger::Metin("merha".into())
        );
    }

    #[test]
    fn hassas_ayar_isaretli() {
        let t = yerlesik_tanimlar();
        let api = t.iter().find(|d| d.anahtar == "ai.api_anahtari").unwrap();
        assert!(
            api.hassas,
            "API anahtarı hassas olmalı (profile aktarılmaz)"
        );
    }

    #[test]
    fn deger_serde_gidis_donus() {
        for v in [
            AyarDeger::Mantik(true),
            AyarDeger::TamSayi(42),
            AyarDeger::Ondalik(1.5),
            AyarDeger::Metin("x".into()),
            AyarDeger::Secim("koyu".into()),
        ] {
            let j = serde_json::to_string(&v).unwrap();
            let geri: AyarDeger = serde_json::from_str(&j).unwrap();
            assert_eq!(v, geri);
        }
    }
}
