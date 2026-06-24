//! ÇE-11 — **Yayın kalitesi görsel dışa aktarma** (PNG yüksek-DPI + SVG vektör + PDF vektör).
//!
//! Genom tarayıcı / grafik görünümleri **render-bağımsız bir [`CizimListesi`]** (display list, MK-17)
//! derler; bu modül o listeyi **eklenti içinde** (motor/GPU olmadan, yeni dış bağımlılık olmadan)
//! yayın kalitesinde dosyaya çevirir.  Üç biçim, üçü de aynı çizim listesinden:
//!
//! * **PNG (yüksek DPI):** raster — [`GorselAyari::dpi`] ile **süper-örnekleme** (mantıksal boyut ×
//!   ölçek).  Ekran çözünürlüğüne kilitlenmez; 300 DPI varsayılan (yayın).  Metin **font gerektirir**
//!   → raster PNG'de çizilmez (geometri çizilir); etiketli yayın için **SVG/PDF** önerilir.
//! * **SVG (vektör):** çözünürlükten **bağımsız** tam vektör — dikdörtgen/çizgi/**metin** (etiketler)
//!   dâhil.  "PNG bulanık" sorununun temel çözümü (MK-58 yayın kalitesi).
//! * **PDF (vektör):** tek-sayfa, gömülü Helvetica ile **metin dâhil** vektör çıktı (sunum/dergi).
//!
//! **Arka plan seçimi** ([`ArkaPlan`]): beyaz (varsayılan), özel renk veya **saydam** (poster/slayt
//! üzerine yerleştirme).  Saydam SVG'de zemin yok; saydam PNG'de zemin pikselleri alfa=0.
//!
//! 3B yapı görünümü dolu küre/şerit içerdiğinden ([`crate::structure3d`]) kendi PNG/SVG yolunu
//! kullanır; bu modülün [`GorselAyari`] boyut/DPI hesapları ([`GorselAyari::raster_boyut`]) oradaki
//! `png_disa_aktar`/`svg_disa_aktar` çağrıları için de yeniden kullanılır (tek yayın-ayarı kaynağı).

use crate::genome_browser::cizim::{CizimListesi, MetinHiza, Primitif};
use crate::genome_browser::disa_aktar::{png_olustur, svg_olustur, Palet};

/// Ekran/yerleşim **temel** yoğunluğu (CSS pikseli ≈ 96 DPI).  Mantıksal boyutlar bu temele göredir;
/// dışa aktarma DPI'ı bunun katıdır (300 DPI → 3.125× süper-örnekleme).
pub const TEMEL_DPI: u32 = 96;

/// Yayın için makul **varsayılan** dışa aktarma yoğunluğu (dergi figürleri tipik 300 DPI).
pub const YAYIN_DPI: u32 = 300;

/// Görselin **mantıksal** (yerleşim) boyutu — piksel (96 DPI temel).  Raster çıktı boyutu
/// [`GorselAyari::dpi`] ile ölçeklenir; vektör (SVG/PDF) çözünürlükten bağımsızdır.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Boyut {
    /// Genişlik (mantıksal piksel).
    pub genislik: f32,
    /// Yükseklik (mantıksal piksel).
    pub yukseklik: f32,
}

impl Boyut {
    /// Mantıksal piksel boyut.
    pub fn yeni(genislik: f32, yukseklik: f32) -> Self {
        Self {
            genislik: genislik.max(1.0),
            yukseklik: yukseklik.max(1.0),
        }
    }

    /// **Fiziksel boyuttan** (inç) ve hedef DPI'dan mantıksal boyut üretir — örn. 6.5×4 inç @300 DPI
    /// bir dergi figürü.  Mantıksal boyut 96-DPI temele indirgenir; raster çıktı [`GorselAyari`] ile
    /// gerçek DPI'a ölçeklenir → "şu kadar santim/inç bas" deterministik olur.
    pub fn inçten(genislik_inc: f32, yukseklik_inc: f32) -> Self {
        Self::yeni(
            genislik_inc * TEMEL_DPI as f32,
            yukseklik_inc * TEMEL_DPI as f32,
        )
    }
}

/// Dışa aktarılan görselin **arka planı** (ÇE-11 "arka plan seçimi").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArkaPlan {
    /// Beyaz (yayın varsayılanı).
    Beyaz,
    /// Özel düz renk (RGB).
    Ozel([u8; 3]),
    /// **Saydam** (şeffaf) — SVG'de zemin yok; PNG'de alfa=0.
    Saydam,
}

impl ArkaPlan {
    /// Bu arka planı somut bir [`Palet`]'e çevirir (zemin = renk veya saydam için `None`).
    pub fn palet(&self) -> Palet {
        match self {
            ArkaPlan::Beyaz => Palet::yayin(),
            ArkaPlan::Ozel(rgb) => {
                let mut p = Palet::yayin();
                p.zemin_ayarla(*rgb);
                p
            }
            ArkaPlan::Saydam => Palet::saydam(),
        }
    }
}

/// Dışa aktarma biçimi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GorselFormat {
    /// PNG raster (yüksek DPI süper-örnekleme).
    Png,
    /// SVG vektör (çözünürlükten bağımsız, etiketli).
    Svg,
    /// PDF vektör (tek sayfa, metin dâhil).
    Pdf,
}

impl GorselFormat {
    /// Dosya uzantısı (noktasız).
    pub fn uzanti(&self) -> &'static str {
        match self {
            GorselFormat::Png => "png",
            GorselFormat::Svg => "svg",
            GorselFormat::Pdf => "pdf",
        }
    }

    /// MIME türü.
    pub fn mime(&self) -> &'static str {
        match self {
            GorselFormat::Png => "image/png",
            GorselFormat::Svg => "image/svg+xml",
            GorselFormat::Pdf => "application/pdf",
        }
    }
}

/// Görsel dışa aktarma ayarı — **boyut + DPI + arka plan + biçim** (yayın kalitesi tek nokta).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GorselAyari {
    /// Mantıksal (yerleşim) boyut.
    pub boyut: Boyut,
    /// Raster dışa aktarma yoğunluğu (DPI); [`TEMEL_DPI`] altına düşmez.
    pub dpi: u32,
    /// Arka plan.
    pub arka_plan: ArkaPlan,
    /// Biçim.
    pub format: GorselFormat,
}

impl GorselAyari {
    /// Yayın varsayılanı: verilen boyut, 300 DPI, beyaz zemin, PNG.
    pub fn yayin(boyut: Boyut) -> Self {
        Self {
            boyut,
            dpi: YAYIN_DPI,
            arka_plan: ArkaPlan::Beyaz,
            format: GorselFormat::Png,
        }
    }

    /// DPI'ı ayarlar (akıcı; [`TEMEL_DPI`] altına kırpılır).
    pub fn with_dpi(mut self, dpi: u32) -> Self {
        self.dpi = dpi.max(TEMEL_DPI);
        self
    }

    /// Arka planı ayarlar (akıcı).
    pub fn with_arka_plan(mut self, arka_plan: ArkaPlan) -> Self {
        self.arka_plan = arka_plan;
        self
    }

    /// Biçimi ayarlar (akıcı).
    pub fn with_format(mut self, format: GorselFormat) -> Self {
        self.format = format;
        self
    }

    /// Raster süper-örnekleme ölçeği (`dpi / 96`).  Vektör çıktıda 1.0 sayılır (boyut bağımsız).
    pub fn olcek(&self) -> f32 {
        (self.dpi.max(TEMEL_DPI) as f32) / TEMEL_DPI as f32
    }

    /// Gerçek **raster piksel** boyutu (mantıksal boyut × ölçek) — yüksek DPI çıktı.  3B görünümün
    /// `png_disa_aktar`'ı da bu boyutu kullanır → tek yayın-ayarı kaynağı.
    pub fn raster_boyut(&self) -> (u32, u32) {
        let k = self.olcek();
        (
            (self.boyut.genislik * k).round().max(1.0) as u32,
            (self.boyut.yukseklik * k).round().max(1.0) as u32,
        )
    }
}

/// Dışa aktarılmış görsel çıktısı (biçime göre bayt veya metin).
#[derive(Debug, Clone, PartialEq)]
pub enum GorselCikti {
    /// PNG baytları.
    Png(Vec<u8>),
    /// SVG belgesi (UTF-8 metin).
    Svg(String),
    /// PDF baytları.
    Pdf(Vec<u8>),
}

impl GorselCikti {
    /// Çıktıyı ham baytlara çevirir (dosyaya yazma çağıran tarafta, `fs`-kapılı — MK-13).
    pub fn baytlar(&self) -> Vec<u8> {
        match self {
            GorselCikti::Png(b) | GorselCikti::Pdf(b) => b.clone(),
            GorselCikti::Svg(s) => s.clone().into_bytes(),
        }
    }

    /// Çıktının uygun dosya uzantısı.
    pub fn uzanti(&self) -> &'static str {
        match self {
            GorselCikti::Png(_) => "png",
            GorselCikti::Svg(_) => "svg",
            GorselCikti::Pdf(_) => "pdf",
        }
    }

    /// Bayt cinsinden boyut (ilerleme/özet için).
    pub fn boyut_bayt(&self) -> usize {
        match self {
            GorselCikti::Png(b) | GorselCikti::Pdf(b) => b.len(),
            GorselCikti::Svg(s) => s.len(),
        }
    }
}

/// Bir [`CizimListesi`]'ni (genom tarayıcı / grafik) [`GorselAyari`]'na göre dışa aktarır.
pub fn cizimi_disa_aktar(liste: &CizimListesi, ayari: &GorselAyari) -> GorselCikti {
    let palet = ayari.arka_plan.palet();
    match ayari.format {
        GorselFormat::Png => {
            // Yüksek DPI: hem piksel boyutu hem de geometri ölçeğini büyüt (süper-örnekleme).
            let olcekli = olcekli_liste(liste, ayari.olcek());
            let (w, h) = ayari.raster_boyut();
            GorselCikti::Png(png_olustur(&olcekli, w, h, &palet))
        }
        GorselFormat::Svg => GorselCikti::Svg(svg_olustur(
            liste,
            ayari.boyut.genislik,
            ayari.boyut.yukseklik,
            &palet,
        )),
        GorselFormat::Pdf => GorselCikti::Pdf(pdf_olustur(
            liste,
            ayari.boyut.genislik,
            ayari.boyut.yukseklik,
            &palet,
        )),
    }
}

/// Çizim listesinin koordinat/boyut/font/kalınlık değerlerini `k` ile ölçekler (raster süper-örnekleme).
/// İsabet bölgeleri dışa aktarmada gerekmez (etkileşim yok) → atlanır.
fn olcekli_liste(liste: &CizimListesi, k: f32) -> CizimListesi {
    if (k - 1.0).abs() < f32::EPSILON {
        return liste.clone();
    }
    let primitifler = liste
        .primitifler
        .iter()
        .map(|p| match p {
            Primitif::Dikdortgen {
                x,
                y,
                gen,
                yuk,
                renk,
            } => Primitif::Dikdortgen {
                x: x * k,
                y: y * k,
                gen: gen * k,
                yuk: yuk * k,
                renk: *renk,
            },
            Primitif::Cizgi {
                x1,
                y1,
                x2,
                y2,
                renk,
                kalinlik,
            } => Primitif::Cizgi {
                x1: x1 * k,
                y1: y1 * k,
                x2: x2 * k,
                y2: y2 * k,
                renk: *renk,
                kalinlik: kalinlik * k,
            },
            Primitif::Metin {
                x,
                y,
                icerik,
                renk,
                boyut,
                hiza,
            } => Primitif::Metin {
                x: x * k,
                y: y * k,
                icerik: icerik.clone(),
                renk: *renk,
                boyut: boyut * k,
                hiza: *hiza,
            },
        })
        .collect();
    CizimListesi {
        primitifler,
        isabetler: Vec::new(),
    }
}

// ─── PDF (tek-sayfa vektör; gömülü Helvetica) ──────────────────────────────────────

/// Çizim listesini tek-sayfa **vektör PDF**'e çevirir (dikdörtgen + çizgi + metin).  Saf-Rust,
/// yeni bağımlılık yok; PDF koordinat sistemi alttan-yukarı olduğundan y ekseni çevrilir.
pub fn pdf_olustur(liste: &CizimListesi, genislik: f32, yukseklik: f32, palet: &Palet) -> Vec<u8> {
    let w = genislik.max(1.0);
    let h = yukseklik.max(1.0);
    let icerik = pdf_icerik(liste, w, h, palet);
    pdf_belge(&icerik, w, h)
}

/// Çok-satırlı **metin** içeriğini tek-sayfa bir PDF'e dizer (Helvetica; en üstten aşağıya).  Temel
/// rapor PDF'i ([`super::report::Rapor::pdf`]) bunu kullanır.  `punto` font boyutu; sayfa, satır
/// sayısına göre boyutlanır (tek sayfa, taşma kırpılmaz — temel MVP rapor).
pub fn metin_pdf(satirlar: &[String], punto: f32) -> Vec<u8> {
    let punto = punto.max(4.0);
    let satir_yuksekligi = punto * 1.4;
    let kenar = 36.0; // ~0.5 inç kenar boşluğu
    let w = 612.0; // US Letter genişliği (pt)
    let h = (kenar * 2.0 + satir_yuksekligi * satirlar.len().max(1) as f32).max(792.0);

    let mut icerik = String::with_capacity(64 + satirlar.len() * 48);
    icerik.push_str(&format!("BT /F1 {punto:.1} Tf 0 0 0 rg\n"));
    // İlk satırın taban çizgisi üst kenardan punto kadar aşağıda.
    let mut y = h - kenar - punto;
    for satir in satirlar {
        icerik.push_str(&format!(
            "1 0 0 1 {kenar:.1} {y:.1} Tm ({}) Tj\n",
            pdf_metin_kacis(satir)
        ));
        y -= satir_yuksekligi;
    }
    icerik.push_str("ET\n");
    pdf_belge(&icerik, w, h)
}

/// Bir içerik akışını tek-sayfa, gömülü Helvetica'lı geçerli bir PDF belgesine sarar (xref + trailer).
fn pdf_belge(icerik: &str, w: f32, h: f32) -> Vec<u8> {
    // PDF nesne gövdeleri (1-tabanlı; içerik akışı 4, font 5).
    let nesneler = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w:.0} {h:.0}] \
/Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{icerik}\nendstream",
            icerik.len()
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];

    let mut pdf: Vec<u8> = Vec::with_capacity(512 + icerik.len());
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let mut ofsetler = Vec::with_capacity(nesneler.len());
    for (i, govde) in nesneler.iter().enumerate() {
        ofsetler.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{govde}\nendobj\n", i + 1).as_bytes());
    }
    let xref_ofset = pdf.len();
    let say = nesneler.len() + 1; // + serbest (0) nesne
    pdf.extend_from_slice(format!("xref\n0 {say}\n").as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for o in &ofsetler {
        pdf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size {say} /Root 1 0 R >>\nstartxref\n{xref_ofset}\n%%EOF\n")
            .as_bytes(),
    );
    pdf
}

/// PDF içerik akışı (çizim operatörleri).  Renkler 0..1; y çevrilir (PDF alttan-yukarı).
fn pdf_icerik(liste: &CizimListesi, w: f32, h: f32, palet: &Palet) -> String {
    let mut s = String::with_capacity(256 + liste.primitifler.len() * 48);

    // Zemin (saydam paletse atlanır).
    if let Some(z) = palet.zemin {
        let (r, g, b) = rgb01(z);
        s.push_str(&format!("{r:.3} {g:.3} {b:.3} rg 0 0 {w:.2} {h:.2} re f\n"));
    }

    for p in &liste.primitifler {
        match p {
            Primitif::Dikdortgen {
                x,
                y,
                gen,
                yuk,
                renk,
            } => {
                let (r, g, b) = rgb01(palet.rgb(*renk));
                let yy = h - y - yuk.max(0.0); // sol-üst → PDF sol-alt
                s.push_str(&format!(
                    "{r:.3} {g:.3} {b:.3} rg {x:.2} {yy:.2} {:.2} {:.2} re f\n",
                    gen.max(0.0),
                    yuk.max(0.0)
                ));
            }
            Primitif::Cizgi {
                x1,
                y1,
                x2,
                y2,
                renk,
                kalinlik,
            } => {
                let (r, g, b) = rgb01(palet.rgb(*renk));
                s.push_str(&format!(
                    "{r:.3} {g:.3} {b:.3} RG {:.2} w {x1:.2} {:.2} m {x2:.2} {:.2} l S\n",
                    kalinlik.max(0.1),
                    h - y1,
                    h - y2
                ));
            }
            Primitif::Metin {
                x,
                y,
                icerik,
                renk,
                boyut,
                hiza,
            } => {
                let (r, g, b) = rgb01(palet.rgb(*renk));
                // Ortalama: kaba genişlik tahmini (Helvetica ≈ 0.5em/karakter) ile sola kaydır.
                let kayma = match hiza {
                    MetinHiza::Sol => 0.0,
                    MetinHiza::Orta => icerik.chars().count() as f32 * boyut * 0.25,
                };
                let tx = (x - kayma).max(0.0);
                let ty = (h - y - boyut).max(0.0); // hanging baseline yaklaşığı
                s.push_str(&format!(
                    "BT /F1 {boyut:.1} Tf {r:.3} {g:.3} {b:.3} rg {tx:.2} {ty:.2} Td ({}) Tj ET\n",
                    pdf_metin_kacis(icerik)
                ));
            }
        }
    }
    s
}

/// RGB bayt → 0..1 üçlüsü.
fn rgb01(rgb: [u8; 3]) -> (f32, f32, f32) {
    (
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
    )
}

/// PDF metin dizgisi kaçışı (`(`, `)`, `\`).  Non-ASCII karakterler temel Helvetica kodlamasına
/// bırakılır (MVP "temel" rapor; tam Unicode gömülü font v1.x).
fn pdf_metin_kacis(s: &str) -> String {
    let mut c = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '(' => c.push_str("\\("),
            ')' => c.push_str("\\)"),
            '\\' => c.push_str("\\\\"),
            '\n' | '\r' => c.push(' '),
            _ => c.push(ch),
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome_browser::cizim::CizimRengi;

    fn ornek_liste() -> CizimListesi {
        let mut l = CizimListesi::yeni();
        l.primitifler.push(Primitif::Dikdortgen {
            x: 10.0,
            y: 10.0,
            gen: 40.0,
            yuk: 20.0,
            renk: CizimRengi::Ekson,
        });
        l.primitifler.push(Primitif::Cizgi {
            x1: 0.0,
            y1: 5.0,
            x2: 100.0,
            y2: 5.0,
            renk: CizimRengi::CetvelCizgi,
            kalinlik: 1.0,
        });
        l.primitifler.push(Primitif::Metin {
            x: 12.0,
            y: 12.0,
            icerik: "BRCA1 (gen)".into(),
            renk: CizimRengi::AnotasyonMetin,
            boyut: 10.0,
            hiza: MetinHiza::Sol,
        });
        l
    }

    #[test]
    fn raster_boyut_dpi_ile_olceklenir() {
        let ayari = GorselAyari::yayin(Boyut::yeni(200.0, 100.0)); // 300 DPI
        assert!((ayari.olcek() - 3.125).abs() < 1e-6);
        assert_eq!(ayari.raster_boyut(), (625, 313)); // 200*3.125=625, 100*3.125=312.5→313
                                                      // 96 DPI → 1:1.
        let temel = ayari.with_dpi(96);
        assert_eq!(temel.raster_boyut(), (200, 100));
    }

    #[test]
    fn dpi_temel_altina_kirpilir() {
        let ayari = GorselAyari::yayin(Boyut::yeni(10.0, 10.0)).with_dpi(10);
        assert_eq!(ayari.dpi, TEMEL_DPI); // 10 < 96 → 96
    }

    #[test]
    fn inçten_boyut() {
        let b = Boyut::inçten(6.5, 4.0);
        assert_eq!(b.genislik, 6.5 * 96.0);
        assert_eq!(b.yukseklik, 4.0 * 96.0);
    }

    #[test]
    fn png_yuksek_dpi_daha_buyuk_bayt() {
        let l = ornek_liste();
        let dusuk = cizimi_disa_aktar(
            &l,
            &GorselAyari::yayin(Boyut::yeni(200.0, 100.0)).with_dpi(96),
        );
        let yuksek = cizimi_disa_aktar(&l, &GorselAyari::yayin(Boyut::yeni(200.0, 100.0)));
        // 300 DPI çıktı 96 DPI'dan (çok) daha fazla piksel → daha büyük.
        assert!(yuksek.boyut_bayt() > dusuk.boyut_bayt());
        assert_eq!(yuksek.uzanti(), "png");
    }

    #[test]
    fn svg_vektor_metni_icerir() {
        let l = ornek_liste();
        let cikti = cizimi_disa_aktar(
            &l,
            &GorselAyari::yayin(Boyut::yeni(200.0, 100.0)).with_format(GorselFormat::Svg),
        );
        match cikti {
            GorselCikti::Svg(s) => {
                assert!(s.starts_with("<svg"));
                assert!(s.contains("BRCA1 (gen)")); // metin etiketi vektörde var
                assert!(s.contains("<rect")); // dikdörtgen
            }
            _ => panic!("SVG bekleniyordu"),
        }
    }

    #[test]
    fn svg_saydam_zemin_yok() {
        let l = ornek_liste();
        let cikti = cizimi_disa_aktar(
            &l,
            &GorselAyari::yayin(Boyut::yeni(50.0, 50.0))
                .with_format(GorselFormat::Svg)
                .with_arka_plan(ArkaPlan::Saydam),
        );
        if let GorselCikti::Svg(s) = cikti {
            // 50×50 zemin dikdörtgeni yok.
            assert!(!s.contains("width=\"50\" height=\"50\" fill=\"#"));
        } else {
            panic!("SVG bekleniyordu");
        }
    }

    #[test]
    fn pdf_gecerli_iskelet() {
        let l = ornek_liste();
        let cikti = cizimi_disa_aktar(
            &l,
            &GorselAyari::yayin(Boyut::yeni(200.0, 100.0)).with_format(GorselFormat::Pdf),
        );
        if let GorselCikti::Pdf(b) = cikti {
            let metin = String::from_utf8_lossy(&b);
            assert!(metin.starts_with("%PDF-1.4"));
            assert!(metin.contains("/Type /Catalog"));
            assert!(metin.contains("/BaseFont /Helvetica"));
            assert!(metin.contains("stream"));
            assert!(metin.contains("(BRCA1 \\(gen\\)) Tj")); // metin + parantez kaçışı
            assert!(metin.trim_end().ends_with("%%EOF"));
            // xref tablosu nesne sayısıyla tutarlı (0 + 5 nesne = 6).
            assert!(metin.contains("xref\n0 6\n"));
        } else {
            panic!("PDF bekleniyordu");
        }
    }

    #[test]
    fn pdf_metin_kacis_parantez() {
        assert_eq!(pdf_metin_kacis("a(b)c\\d"), "a\\(b\\)c\\\\d");
    }

    #[test]
    fn ozel_arka_plan_paleti() {
        let p = ArkaPlan::Ozel([10, 20, 30]).palet();
        assert_eq!(p.zemin, Some([10, 20, 30]));
        assert!(ArkaPlan::Saydam.palet().zemin.is_none());
    }
}
