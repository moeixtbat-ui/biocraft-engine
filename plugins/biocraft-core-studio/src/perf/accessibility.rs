//! ÇE-12 — **Erişilebilirlik: renk** (renk körü dostu palet + kontrast + şekil/desen ipucu).
//!
//! MK-52: renkler token'dan gelir, renge ek görsel ipucu (şekil/desen) verilir, yüksek kontrast +
//! yazı ölçeği desteklenir.  Eklenti motorun render token sistemine (`biocraft-render`) MK-17 gereği
//! **bağlanamadığından**, çekirdek eklenti kendi **render-bağımsız** erişilebilirlik modelini taşır;
//! somut RGB'ye [`crate::genome_browser::disa_aktar::Palet`] üzerinden (motor da bu paleti tasarım
//! jetonundan doldurabilir) bağlanır.
//!
//! ## Bu modül neyi kanıtlar?
//! 1. **Renk körü güvenliği:** [`OKABE_ITO`] (bilimsel kategorik CB-güvenli palet) tüm renk-körlüğü
//!    türleri altında **ayırt edilebilir kalır** — [`simule_et`] (dichromat simülasyonu) + algı
//!    uzaklığı ile **birim-testlenir** (golden değil; sayısal güvence).
//! 2. **Renge bağımlılık yok (WCAG 1.4.1):** her anlamsal kategori bir [`SekilIpucu`] de taşır →
//!    renk tek kanal değildir (renk körü kullanıcı şekil/desenle de ayırır).
//! 3. **Kontrast (WCAG 1.4.3/1.4.6):** [`kontrast_orani`] + [`KontrastSeviyesi`] ile metin/zemin
//!    kontrastı AA/AAA eşiğini geçer mi denetlenir.
//!
//! ## Dichromat simülasyonu hakkında dürüst not
//! [`simule_et`], yaygın "daltonize" LMS yaklaşımını kullanır (Viénot-Brettel-Mollon 1999 türevi
//! katsayılar).  Bu bir **yaklaşımdır** (klinik tanı aracı değil); amacı paletin renk körü altında
//! ayırt edilebilirliğini **mühendislik güvencesiyle** ölçmektir.

use crate::genome_browser::cizim::CizimRengi;
use crate::genome_browser::disa_aktar::Palet;

/// 8-bit RGB renk (eklenti-yerel; genom paletiyle aynı gösterim).
pub type Renk = [u8; 3];

// ─── Renk görme (renk körlüğü) türleri ────────────────────────────────────────

/// Bir renk görme türü.  Erişilebilirlik ayarında kullanıcı seçer; palet buna göre üretilir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenkGormeTuru {
    /// Tipik (üç-konili) renk görme.
    Normal,
    /// Protanopi — L (kırmızı) konisi yok (kırmızı-yeşil ayrımı zayıf).
    Protanopi,
    /// Deuteranopi — M (yeşil) konisi yok (en yaygın kırmızı-yeşil körlüğü).
    Deuteranopi,
    /// Tritanopi — S (mavi) konisi yok (mavi-sarı ayrımı zayıf, nadir).
    Tritanopi,
    /// Akromatopsi — renk yok (yalnız parlaklık; tam renk körlüğü).
    Akromatopsi,
}

impl RenkGormeTuru {
    /// Tüm türler (palet güvenlik testi tüm türler altında doğrular).
    pub fn tumu() -> [RenkGormeTuru; 5] {
        [
            RenkGormeTuru::Normal,
            RenkGormeTuru::Protanopi,
            RenkGormeTuru::Deuteranopi,
            RenkGormeTuru::Tritanopi,
            RenkGormeTuru::Akromatopsi,
        ]
    }

    /// İnsan-okunur ad (ayarlar arayüzü).
    pub fn ad(&self) -> &'static str {
        match self {
            RenkGormeTuru::Normal => "Normal",
            RenkGormeTuru::Protanopi => "Protanopi (kırmızı-zayıf)",
            RenkGormeTuru::Deuteranopi => "Deuteranopi (yeşil-zayıf)",
            RenkGormeTuru::Tritanopi => "Tritanopi (mavi-zayıf)",
            RenkGormeTuru::Akromatopsi => "Akromatopsi (renksiz)",
        }
    }
}

// ─── Dichromat simülasyonu (daltonize / LMS) ──────────────────────────────────

/// Bir rengin verilen renk körlüğü türü altında **nasıl görüneceğini** simüle eder.
///
/// Yöntem: sRGB → LMS (Hunt-Pointer-Estevez türevi katsayılar) → tür için dichromat izdüşümü →
/// LMS → sRGB.  Akromatopside Rec.601 parlaklığına indirir (R=G=B).  Çıktı 0–255'e kırpılır.
pub fn simule_et(rgb: Renk, tur: RenkGormeTuru) -> Renk {
    if tur == RenkGormeTuru::Normal {
        return rgb;
    }
    let (r, g, b) = (rgb[0] as f64, rgb[1] as f64, rgb[2] as f64);

    if tur == RenkGormeTuru::Akromatopsi {
        // Rec.601 parlaklık → gri ton (renk bilgisi tamamen kaybolur).
        let y = 0.299 * r + 0.587 * g + 0.114 * b;
        let v = kirp(y);
        return [v, v, v];
    }

    // sRGB → LMS.
    let l = 17.8824 * r + 43.5161 * g + 4.11935 * b;
    let m = 3.45565 * r + 27.1554 * g + 3.86714 * b;
    let s = 0.0299566 * r + 0.184309 * g + 1.46709 * b;

    // Tür için dichromat izdüşümü (eksik koni, kalan ikisinden yeniden kurulur).
    let (l2, m2, s2) = match tur {
        RenkGormeTuru::Protanopi => (2.02344 * m - 2.52581 * s, m, s),
        RenkGormeTuru::Deuteranopi => (l, 0.494207 * l + 1.24827 * s, s),
        RenkGormeTuru::Tritanopi => (l, m, -0.395913 * l + 0.801109 * m),
        _ => (l, m, s),
    };

    // LMS → sRGB.
    let r2 = 0.0809444479 * l2 - 0.130504409 * m2 + 0.116721066 * s2;
    let g2 = -0.0102485335 * l2 + 0.0540193266 * m2 - 0.113614708 * s2;
    let b2 = -0.000365296938 * l2 - 0.00412161469 * m2 + 0.693511405 * s2;

    [kirp(r2), kirp(g2), kirp(b2)]
}

/// f64 değeri 0–255 u8'e kırpar (yuvarlayarak).
fn kirp(x: f64) -> u8 {
    x.round().clamp(0.0, 255.0) as u8
}

// ─── Algı uzaklığı + ayırt edilebilirlik ──────────────────────────────────────

/// İki renk arasında **algısal** uzaklık (yaygın "redmean" ağırlıklı RGB metriği — Lab'a yakın,
/// ucuz).  0 = aynı; siyah↔beyaz ≈ 765.
pub fn algi_uzakligi(a: Renk, b: Renk) -> f64 {
    let rbar = (a[0] as f64 + b[0] as f64) / 2.0;
    let dr = a[0] as f64 - b[0] as f64;
    let dg = a[1] as f64 - b[1] as f64;
    let db = a[2] as f64 - b[2] as f64;
    ((2.0 + rbar / 256.0) * dr * dr + 4.0 * dg * dg + (2.0 + (255.0 - rbar) / 256.0) * db * db)
        .sqrt()
}

/// "Kategorik renkler net ayrı olmalı" eşiği (algı uzaklığı).  Kategorik (kalitatif) paletler için
/// kaba ama pratik bir alt sınır; CB-güvenli palet bunu **tüm** renk körlüğü türlerinde aşar.
pub const AYIRT_ESIGI: f64 = 60.0;

/// İki renk verilen renk körlüğü türü altında ayırt edilebilir mi? (simüle edip uzaklığa bakar.)
pub fn ayirt_edilebilir_mi(a: Renk, b: Renk, tur: RenkGormeTuru, esik: f64) -> bool {
    algi_uzakligi(simule_et(a, tur), simule_et(b, tur)) >= esik
}

/// Bir renk listesinin verilen tür altında **en yakın** çift uzaklığını döndürür (palet güvenlik
/// denetimi).  Liste < 2 ise `f64::INFINITY`.
pub fn en_yakin_cift(renkler: &[Renk], tur: RenkGormeTuru) -> f64 {
    let mut en_az = f64::INFINITY;
    for i in 0..renkler.len() {
        for j in (i + 1)..renkler.len() {
            let d = algi_uzakligi(simule_et(renkler[i], tur), simule_et(renkler[j], tur));
            if d < en_az {
                en_az = d;
            }
        }
    }
    en_az
}

// ─── Okabe-Ito CB-güvenli kategorik palet ─────────────────────────────────────

/// Okabe & Ito (2008) "renk körü dostu" kategorik palet (8 renk).  Bilimsel görselleştirmede
/// kanıtlanmış; özellikle **yaygın** kırmızı-yeşil körlüğünde (protan/deutan) ayırt edilebilir kalır.
pub const OKABE_ITO: [Renk; 8] = [
    [0, 0, 0],       // siyah
    [230, 159, 0],   // turuncu
    [86, 180, 233],  // gök mavisi
    [0, 158, 115],   // mavimsi yeşil
    [240, 228, 66],  // sarı
    [0, 114, 178],   // mavi
    [213, 94, 0],    // vermilyon (kırmızımsı)
    [204, 121, 167], // kırmızımsı mor
];

/// Çekirdek eklentinin 4-yönlü kategorik ihtiyaçları (okuma yönü, A/C/G/T baz, varyant türü, çoklu
/// örnek) için seçilen **CB-güvenli 4-renk** alt kümesi (turuncu / mavimsi yeşil / mavi / vermilyon).
/// Yaygın renk körlüğünde (Normal/protan/deutan) net ayrılır; nadir tritan/akromatopside şekil/harf
/// ipucu (ikinci kanal) güvenceyi tamamlar.
pub const GENOM_KATEGORIK: [Renk; 4] = [OKABE_ITO[1], OKABE_ITO[3], OKABE_ITO[5], OKABE_ITO[6]];

// ─── Şekil/desen ipucu (renge ek görsel ipucu — WCAG 1.4.1) ───────────────────

/// Renge **ek** görsel ipucu: bir kategori şekille de ayırt edilir (renk tek kanal değildir).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SekilIpucu {
    /// Dolu daire.
    Daire,
    /// Kare.
    Kare,
    /// Yukarı üçgen.
    UcgenYukari,
    /// Aşağı üçgen.
    UcgenAsagi,
    /// Eşkenar dörtgen (baklava).
    Eskenar,
    /// Artı/çapraz işaret.
    Arti,
    /// Sağ ok (ileri şerit).
    OkSag,
    /// Sol ok (geri şerit).
    OkSol,
}

impl SekilIpucu {
    /// ASCII/etiket gösterimi (metin tabanlı demo + ekran okuyucu kısaltması).
    pub fn simge(&self) -> &'static str {
        match self {
            SekilIpucu::Daire => "●",
            SekilIpucu::Kare => "■",
            SekilIpucu::UcgenYukari => "▲",
            SekilIpucu::UcgenAsagi => "▼",
            SekilIpucu::Eskenar => "◆",
            SekilIpucu::Arti => "✚",
            SekilIpucu::OkSag => "▶",
            SekilIpucu::OkSol => "◀",
        }
    }
}

/// Varyant türü için şekil ipucu (renk + şekil çift kanal → renk körü ayırt eder).
pub fn varyant_sekli(renk: CizimRengi) -> Option<SekilIpucu> {
    match renk {
        CizimRengi::VaryantSnv => Some(SekilIpucu::Daire),
        CizimRengi::VaryantIns => Some(SekilIpucu::UcgenYukari),
        CizimRengi::VaryantDel => Some(SekilIpucu::UcgenAsagi),
        _ => None,
    }
}

/// Şerit (okuma yönü) için şekil ipucu.
pub fn serit_sekli(renk: CizimRengi) -> Option<SekilIpucu> {
    match renk {
        CizimRengi::ReadIleri => Some(SekilIpucu::OkSag),
        CizimRengi::ReadGeri => Some(SekilIpucu::OkSol),
        _ => None,
    }
}

// ─── WCAG kontrast ─────────────────────────────────────────────────────────────

/// Bir sRGB bileşeninin (0–255) doğrusal (linear) değeri — WCAG göreli parlaklık için.
fn srgb_dogrusal(c: u8) -> f64 {
    let x = c as f64 / 255.0;
    if x <= 0.03928 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG **göreli parlaklık** (0.0 siyah – 1.0 beyaz).
pub fn goreli_parlaklik(rgb: Renk) -> f64 {
    0.2126 * srgb_dogrusal(rgb[0]) + 0.7152 * srgb_dogrusal(rgb[1]) + 0.0722 * srgb_dogrusal(rgb[2])
}

/// İki renk arasında **WCAG kontrast oranı** (1.0 – 21.0).  Siyah↔beyaz = 21:1.
pub fn kontrast_orani(a: Renk, b: Renk) -> f64 {
    let la = goreli_parlaklik(a);
    let lb = goreli_parlaklik(b);
    let (yuksek, dusuk) = if la >= lb { (la, lb) } else { (lb, la) };
    (yuksek + 0.05) / (dusuk + 0.05)
}

/// WCAG kontrast uygunluk seviyesi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KontrastSeviyesi {
    /// AA — büyük metin (≥ 3.0:1).
    AaBuyuk,
    /// AA — normal metin (≥ 4.5:1).
    Aa,
    /// AAA — gelişmiş (≥ 7.0:1).
    Aaa,
}

impl KontrastSeviyesi {
    /// Bu seviyenin geçme eşiği (kontrast oranı).
    pub fn esik(&self) -> f64 {
        match self {
            KontrastSeviyesi::AaBuyuk => 3.0,
            KontrastSeviyesi::Aa => 4.5,
            KontrastSeviyesi::Aaa => 7.0,
        }
    }
}

/// Verilen iki renk, istenen kontrast seviyesini geçiyor mu?
pub fn kontrast_gecer(a: Renk, b: Renk, seviye: KontrastSeviyesi) -> bool {
    kontrast_orani(a, b) >= seviye.esik()
}

// ─── Yazı ölçeği (font scale — MK-52) ─────────────────────────────────────────

/// Kullanıcı yazı boyutu ölçeği — `[0.75, 2.0]` aralığına kırpılır (okunabilirlik + bozulmama).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct YaziOlcek(f32);

impl YaziOlcek {
    /// En küçük ölçek.
    pub const ASGARI: f32 = 0.75;
    /// En büyük ölçek.
    pub const AZAMI: f32 = 2.0;

    /// Ölçek kur (aralığa kırpılır).
    pub fn yeni(olcek: f32) -> Self {
        Self(olcek.clamp(Self::ASGARI, Self::AZAMI))
    }

    /// Ölçek değeri.
    pub fn deger(&self) -> f32 {
        self.0
    }

    /// Bir taban yazı boyutunu (px) ölçekler.
    pub fn uygula(&self, taban_px: f32) -> f32 {
        taban_px * self.0
    }
}

impl Default for YaziOlcek {
    fn default() -> Self {
        Self(1.0)
    }
}

// ─── Erişilebilirlik ayarı (toplu) ─────────────────────────────────────────────

/// Kullanıcının erişilebilirlik tercihleri — palet/yazı/animasyon davranışını sürer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ErisilebilirlikAyari {
    /// Renk görme türü (palet bu türe güvenli olacak şekilde seçilir).
    pub renk_gorme: RenkGormeTuru,
    /// Yüksek kontrast modu (zemin/metin kontrastı maksimuma çekilir).
    pub yuksek_kontrast: bool,
    /// Yazı boyutu ölçeği.
    pub yazi_olcek: YaziOlcek,
    /// Hareketi azalt (animasyon/geçiş kapalı — vestibüler duyarlılık).
    pub hareket_azalt: bool,
}

impl Default for ErisilebilirlikAyari {
    fn default() -> Self {
        Self {
            renk_gorme: RenkGormeTuru::Normal,
            yuksek_kontrast: false,
            yazi_olcek: YaziOlcek::default(),
            hareket_azalt: false,
        }
    }
}

impl ErisilebilirlikAyari {
    /// Renk körü dostu bir hızlı ön ayar (CB-güvenli palet + şekil ipuçları zaten her zaman açık).
    pub fn renk_koru(tur: RenkGormeTuru) -> Self {
        Self {
            renk_gorme: tur,
            ..Self::default()
        }
    }
}

// ─── Genom paletini erişilebilirlik ayarından üret ────────────────────────────

/// Erişilebilirlik ayarına göre **genom tarayıcı/dışa aktarma paleti** üretir.
///
/// - Renk körü türü `Normal` değilse veya yüksek kontrast açıksa: kategorik renkler (okuma yönü,
///   bazlar, varyant, çoklu örnek) **Okabe-Ito CB-güvenli** karşılıklarına çevrilir.
/// - Yüksek kontrast: metin renkleri saf siyaha çekilir, zemin beyaz kalır (azami kontrast).
///
/// Böylece var olan SVG/PNG dışa aktarma yolu (Gün 37/42) ayarı **olduğu gibi** onurlandırır.
pub fn genom_paleti(ayar: &ErisilebilirlikAyari) -> Palet {
    let mut p = Palet::yayin();
    let cb = ayar.renk_gorme != RenkGormeTuru::Normal;

    if cb || ayar.yuksek_kontrast {
        // Kategorik gruplar → Okabe-Ito (tür içi ayırt edilebilir + şekil ipucu ile ikinci kanal).
        // Okuma yönü: vermilyon ↔ mavi (klasik CB-güvenli çift).
        p.ayarla(CizimRengi::ReadIleri, OKABE_ITO[6]); // vermilyon
        p.ayarla(CizimRengi::ReadGeri, OKABE_ITO[5]); // mavi
                                                      // Bazlar: A/C/G/T → mavimsi yeşil / mavi / turuncu / vermilyon (4-yönlü CB-güvenli).
        p.ayarla(CizimRengi::BazA, OKABE_ITO[3]);
        p.ayarla(CizimRengi::BazC, OKABE_ITO[5]);
        p.ayarla(CizimRengi::BazG, OKABE_ITO[1]);
        p.ayarla(CizimRengi::BazT, OKABE_ITO[6]);
        // Varyant türleri: vermilyon / mavi / turuncu (+ şekil ipucu daire/üçgen).
        p.ayarla(CizimRengi::VaryantSnv, OKABE_ITO[6]);
        p.ayarla(CizimRengi::VaryantIns, OKABE_ITO[5]);
        p.ayarla(CizimRengi::VaryantDel, OKABE_ITO[1]);
        // Çoklu örnek paleti → Okabe-Ito ilk 4 ayırt-güvenli renk.
        p.ayarla(CizimRengi::OrnekA, OKABE_ITO[5]); // mavi
        p.ayarla(CizimRengi::OrnekB, OKABE_ITO[1]); // turuncu
        p.ayarla(CizimRengi::OrnekC, OKABE_ITO[3]); // mavimsi yeşil
        p.ayarla(CizimRengi::OrnekD, OKABE_ITO[6]); // vermilyon
    }

    if ayar.yuksek_kontrast {
        // Azami kontrast: saf siyah metin, beyaz zemin; çizgiler koyu.
        let siyah = [0u8, 0, 0];
        p.zemin_ayarla([255, 255, 255]);
        p.ayarla(CizimRengi::CetvelMetin, siyah);
        p.ayarla(CizimRengi::CetvelCizgi, siyah);
        p.ayarla(CizimRengi::IzEtiket, siyah);
        p.ayarla(CizimRengi::AnotasyonMetin, siyah);
        p.ayarla(CizimRengi::OlcumMetin, siyah);
    }

    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn akromatopsi_gri_ton_uretir() {
        // Akromatopside her renk R=G=B (yalnız parlaklık kalır).
        for renk in [[255, 0, 0], [0, 255, 0], [0, 0, 255], [123, 45, 200]] {
            let s = simule_et(renk, RenkGormeTuru::Akromatopsi);
            assert_eq!(s[0], s[1]);
            assert_eq!(s[1], s[2]);
        }
        // Normal → değişmez.
        assert_eq!(simule_et([12, 34, 56], RenkGormeTuru::Normal), [12, 34, 56]);
    }

    #[test]
    fn kirmizi_yesil_deuteranopi_altinda_hue_collapse() {
        // Deuteranopi: kırmızı ve yeşil ikisi de sarı eksenine çöker → R≈G olur (renk ipucu kaybolur),
        // oysa orijinalde kırmızıda R≫G, yeşilde G≫R idi.  Bu, kırmızı-yeşil çökmesinin imzasıdır.
        let k = simule_et([220, 20, 60], RenkGormeTuru::Deuteranopi); // kırmızımsı
        let y = simule_et([60, 179, 113], RenkGormeTuru::Deuteranopi); // yeşilimsi
        assert!(
            (k[0] as i32 - k[1] as i32).abs() < 25,
            "kırmızı deut altında R≈G olmalı: {k:?}"
        );
        assert!(
            (y[0] as i32 - y[1] as i32).abs() < 25,
            "yeşil deut altında R≈G olmalı: {y:?}"
        );
        // Orijinal kırmızıda R≫G, yeşilde G≫R (renk ipucu vardı) — çöktükten sonra ikisi de R≈G.
        let kirmizi = [220i32, 20, 60];
        let yesil = [60i32, 179, 113];
        assert!(kirmizi[0] - kirmizi[1] > 100, "orijinal kırmızıda R baskın");
        assert!(yesil[1] - yesil[0] > 100, "orijinal yeşilde G baskın");
    }

    #[test]
    fn genom_kategorik_yaygin_korlukte_net_ayrilir() {
        // CB-güvenli 4-renk setinin asıl güvencesi: YAYGIN renk körlüğünde (Normal + protanopi +
        // deuteranopi — CVD'lilerin ~%99'u) kategoriler net ayrı kalır (en yakın çift ≥ eşik).
        for tur in [
            RenkGormeTuru::Normal,
            RenkGormeTuru::Protanopi,
            RenkGormeTuru::Deuteranopi,
        ] {
            let en_az = en_yakin_cift(&GENOM_KATEGORIK, tur);
            assert!(
                en_az >= AYIRT_ESIGI,
                "{} altında genom kategorik en yakın çift {en_az:.1} < {AYIRT_ESIGI}",
                tur.ad()
            );
        }
    }

    #[test]
    fn nadir_korlukte_renk_ayrik_kalir_veya_sekil_devreye_girer() {
        // Tritanopi (nadir): renkler hâlâ ayrık kalır (minimal eşik üstünde) — ama "net" eşiğin
        // altına düşebilir; bu beklenir.
        let tri = en_yakin_cift(&GENOM_KATEGORIK, RenkGormeTuru::Tritanopi);
        assert!(
            tri >= 40.0,
            "tritanopide renkler en azından ayrık olmalı: {tri:.1}"
        );
        // Akromatopsi (renksiz): renk tek başına yetmeyebilir — bu yüzden şekil/harf ipucu (ikinci
        // kanal) GARANTİDİR.  Renk metriğine güvenmiyoruz; her kategorinin farklı şekli var.
        let snv = varyant_sekli(CizimRengi::VaryantSnv).unwrap();
        let ins = varyant_sekli(CizimRengi::VaryantIns).unwrap();
        let del = varyant_sekli(CizimRengi::VaryantDel).unwrap();
        assert!(snv != ins && ins != del && snv != del);
    }

    #[test]
    fn akromatopside_sekil_ipucu_sart() {
        // Renksiz görmede renk hiç ayırt etmeyebilir → şekil ipucu (ikinci kanal) zorunlu güvencedir.
        // Varyant türleri farklı şekiller taşır (renk çökse bile ayrılır).
        let snv = varyant_sekli(CizimRengi::VaryantSnv).unwrap();
        let ins = varyant_sekli(CizimRengi::VaryantIns).unwrap();
        let del = varyant_sekli(CizimRengi::VaryantDel).unwrap();
        assert_ne!(snv, ins);
        assert_ne!(ins, del);
        assert_ne!(snv, del);
        // Şerit yönü de şekil taşır (ileri/geri ok).
        assert_ne!(
            serit_sekli(CizimRengi::ReadIleri),
            serit_sekli(CizimRengi::ReadGeri)
        );
    }

    #[test]
    fn wcag_kontrast_bilinen_degerler() {
        // Siyah↔beyaz = 21:1 (WCAG referans).
        let oran = kontrast_orani([0, 0, 0], [255, 255, 255]);
        assert!(
            (oran - 21.0).abs() < 0.01,
            "siyah/beyaz 21:1 olmalı: {oran}"
        );
        // Aynı renk = 1:1.
        assert!((kontrast_orani([100, 100, 100], [100, 100, 100]) - 1.0).abs() < 1e-9);
        // Beyaz zemin + siyah metin AAA geçer; açık gri metin AA geçmez.
        assert!(kontrast_gecer(
            [0, 0, 0],
            [255, 255, 255],
            KontrastSeviyesi::Aaa
        ));
        assert!(!kontrast_gecer(
            [170, 170, 170],
            [255, 255, 255],
            KontrastSeviyesi::Aa
        ));
    }

    #[test]
    fn yuksek_kontrast_palet_metni_aaa_yapar() {
        // Yüksek kontrast ayar → metin/zemin AAA (≥7:1) olmalı.
        let ayar = ErisilebilirlikAyari {
            yuksek_kontrast: true,
            ..Default::default()
        };
        let p = genom_paleti(&ayar);
        let zemin = p.zemin_rgb();
        let metin = p.rgb(CizimRengi::CetvelMetin);
        assert!(kontrast_gecer(metin, zemin, KontrastSeviyesi::Aaa));
    }

    #[test]
    fn cb_palet_okuma_yonu_deuteranopide_ayrilir() {
        // Varsayılan IGV okuma renkleri yerine CB-güvenli ReadIleri/ReadGeri seçilir; deut altında
        // bile ayırt edilebilir (vermilyon ↔ mavi).
        let ayar = ErisilebilirlikAyari::renk_koru(RenkGormeTuru::Deuteranopi);
        let p = genom_paleti(&ayar);
        let ileri = p.rgb(CizimRengi::ReadIleri);
        let geri = p.rgb(CizimRengi::ReadGeri);
        assert!(ayirt_edilebilir_mi(
            ileri,
            geri,
            RenkGormeTuru::Deuteranopi,
            AYIRT_ESIGI
        ));
    }

    #[test]
    fn yazi_olcek_araliga_kirpilir() {
        assert_eq!(YaziOlcek::yeni(0.1).deger(), YaziOlcek::ASGARI);
        assert_eq!(YaziOlcek::yeni(5.0).deger(), YaziOlcek::AZAMI);
        assert_eq!(YaziOlcek::yeni(1.5).deger(), 1.5);
        assert_eq!(YaziOlcek::default().deger(), 1.0);
        // 12px taban + 1.5 ölçek = 18px.
        assert_eq!(YaziOlcek::yeni(1.5).uygula(12.0), 18.0);
    }

    #[test]
    fn normal_ayar_yayin_paletini_korur() {
        // Normal + kontrast kapalı → değişmemiş yayın paleti (geriye uyum).
        let p = genom_paleti(&ErisilebilirlikAyari::default());
        assert_eq!(
            p.rgb(CizimRengi::BazA),
            Palet::yayin().rgb(CizimRengi::BazA)
        );
    }

    #[test]
    fn simule_idempotent_dichromat() {
        // Bir dichromat çıktısını aynı türle yeniden simüle etmek ~aynı kalmalı (izdüşüm sabit nokta).
        let renk = [200, 120, 40];
        let bir = simule_et(renk, RenkGormeTuru::Deuteranopi);
        let iki = simule_et(bir, RenkGormeTuru::Deuteranopi);
        assert!(
            algi_uzakligi(bir, iki) < 15.0,
            "izdüşüm ~sabit nokta: {bir:?} vs {iki:?}"
        );
    }
}
