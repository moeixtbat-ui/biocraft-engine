//! ÇE-02 — **Koordinat cetveli** (üst şerit): kromozom + 1-tabanlı pozisyon, otomatik bp/kb/Mb
//! ölçek ve "yuvarlak" (1·2·5 × 10ⁿ) işaret aralıkları.
//!
//! Cetvel işaretleri [`Tuval`] dönüşümünden geçtiği için veri izleriyle **aynı** koordinatı
//! gösterir (koordinat kayması olmaz).  Çıktı saf veri olduğundan golden test edilebilir.

use super::canvas::Tuval;

/// Cetvelin görüntü ölçeği (etiket birimi).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Olcek {
    /// Baz çifti (kısa bölge).
    Bp,
    /// Kilobaz (1.000 bp).
    Kb,
    /// Megabaz (1.000.000 bp).
    Mb,
}

impl Olcek {
    /// Görünen bp uzunluğuna göre uygun ölçeği seçer.
    pub fn sec(uzunluk_bp: u64) -> Olcek {
        if uzunluk_bp >= 1_000_000 {
            Olcek::Mb
        } else if uzunluk_bp >= 1_000 {
            Olcek::Kb
        } else {
            Olcek::Bp
        }
    }

    /// Birim kısaltması.
    pub fn birim(self) -> &'static str {
        match self {
            Olcek::Bp => "bp",
            Olcek::Kb => "kb",
            Olcek::Mb => "Mb",
        }
    }
}

/// Cetvel üzerindeki tek bir işaret.
#[derive(Debug, Clone, PartialEq)]
pub struct CetvelIsareti {
    /// 1-tabanlı genom pozisyonu.
    pub pos: u64,
    /// İşaretin ekran x'i (piksel).
    pub x_px: f32,
    /// Ölçeğe göre biçimlenmiş etiket (ör. "12.5 kb").
    pub etiket: String,
    /// Büyük (etiketli, uzun çizgi) mi yoksa küçük (ara, kısa çizgi) mi?
    pub buyuk: bool,
}

/// Hesaplanmış cetvel: ölçek + işaretler.
#[derive(Debug, Clone, PartialEq)]
pub struct Cetvel {
    /// Seçilen görüntü ölçeği.
    pub olcek: Olcek,
    /// Soldan sağa sıralı işaretler (büyük + küçük).
    pub isaretler: Vec<CetvelIsareti>,
}

/// 1·2·5 × 10ⁿ ailesinden, `hedef_isaret` adede en yakın **eşit-veya-büyük** yuvarlak adımı seçer.
/// Sonuç en az 1'dir.
pub fn yuvarlak_adim(uzunluk_bp: u64, hedef_isaret: u32) -> u64 {
    let hedef = hedef_isaret.max(1) as f64;
    let ham = (uzunluk_bp as f64 / hedef).max(1.0);
    let us = ham.log10().floor();
    let taban = 10f64.powf(us);
    let oran = ham / taban; // [1, 10)
    let carpan = if oran <= 1.0 {
        1.0
    } else if oran <= 2.0 {
        2.0
    } else if oran <= 5.0 {
        5.0
    } else {
        10.0
    };
    ((carpan * taban).round() as u64).max(1)
}

/// Bir pozisyonu verilen ölçekte biçimler (ör. Kb → "12.5 kb", Bp → "1,234 bp").
pub fn pozisyon_etiketle(pos: u64, olcek: Olcek) -> String {
    match olcek {
        Olcek::Bp => format!("{} bp", binlik_ayrac(pos)),
        Olcek::Kb => format!("{} kb", ondalikli(pos, 1_000.0)),
        Olcek::Mb => format!("{} Mb", ondalikli(pos, 1_000_000.0)),
    }
}

/// Bir tuval için cetvel işaretlerini üretir.  `hedef_isaret` ekranda istenen yaklaşık büyük
/// işaret sayısıdır (genişliğe göre tipik 8–12).  Büyük işaretler `adim` katlarında ve etiketli;
/// her büyük işaretin ortasına bir küçük (etiketsiz) işaret eklenir.
pub fn cetvel(tuval: &Tuval, hedef_isaret: u32) -> Cetvel {
    let uzunluk = tuval.bolge.uzunluk();
    let olcek = Olcek::sec(uzunluk);
    let adim = yuvarlak_adim(uzunluk, hedef_isaret);

    let bas = tuval.bolge.baslangic;
    let bit = tuval.bolge.bitis;
    // Görünür ilk büyük işaret = bas'tan ≥ olan ilk adim katı.
    let ilk = bas.div_ceil(adim) * adim;

    let mut isaretler = Vec::new();
    let mut p = ilk;
    let yari = adim / 2;
    while p <= bit {
        isaretler.push(CetvelIsareti {
            pos: p,
            x_px: tuval.genom_to_ekran(p),
            etiket: pozisyon_etiketle(p, olcek),
            buyuk: true,
        });
        // Bir sonraki büyük işaretten önce bir küçük işaret (yarı adımda), bölge içindeyse.
        if yari > 0 {
            let kucuk = p + yari;
            if kucuk <= bit {
                isaretler.push(CetvelIsareti {
                    pos: kucuk,
                    x_px: tuval.genom_to_ekran(kucuk),
                    etiket: String::new(),
                    buyuk: false,
                });
            }
        }
        // Taşma koruması (çok büyük adımda p += adim sonsuza gitmesin).
        match p.checked_add(adim) {
            Some(y) => p = y,
            None => break,
        }
    }

    Cetvel { olcek, isaretler }
}

// ─── Biçimleme yardımcıları ───────────────────────────────────────────────────────

/// Tam sayıya binlik ayraç ekler (ör. 1234567 → "1,234,567").  Sağdan 3'lü gruplar.
fn binlik_ayrac(n: u64) -> String {
    let s = n.to_string();
    let uzun = s.len();
    let mut cikti = String::with_capacity(uzun + uzun / 3);
    for (i, ch) in s.chars().enumerate() {
        if i != 0 && (uzun - i) % 3 == 0 {
            cikti.push(',');
        }
        cikti.push(ch);
    }
    cikti
}

/// `n / bolen`'i en çok bir ondalıkla, gereksiz ".0" olmadan biçimler (ör. 12500/1000 → "12.5",
/// 2000/1000 → "2").
fn ondalikli(n: u64, bolen: f64) -> String {
    let deger = n as f64 / bolen;
    let yuvarlanmis = (deger * 10.0).round() / 10.0;
    if (yuvarlanmis.fract()).abs() < 1e-9 {
        format!("{}", yuvarlanmis as i64)
    } else {
        format!("{yuvarlanmis:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::super::canvas::GenomBolge;
    use super::*;

    #[test]
    fn olcek_secimi() {
        assert_eq!(Olcek::sec(500), Olcek::Bp);
        assert_eq!(Olcek::sec(1_000), Olcek::Kb);
        assert_eq!(Olcek::sec(50_000), Olcek::Kb);
        assert_eq!(Olcek::sec(1_000_000), Olcek::Mb);
        assert_eq!(Olcek::sec(250_000_000), Olcek::Mb);
    }

    #[test]
    fn yuvarlak_adim_1_2_5_ailesi() {
        // ~10 işaret hedefiyle 1000 bp → adım 100.
        assert_eq!(yuvarlak_adim(1_000, 10), 100);
        // 100 bp / 10 = 10 → adım 10.
        assert_eq!(yuvarlak_adim(100, 10), 10);
        // 3000 / 10 = 300 → bir sonraki yuvarlak 500.
        assert_eq!(yuvarlak_adim(3_000, 10), 500);
        // Çok küçük bölge → en az 1.
        assert_eq!(yuvarlak_adim(5, 10), 1);
    }

    #[test]
    fn binlik_ve_ondalik_bicimleme() {
        assert_eq!(binlik_ayrac(0), "0");
        assert_eq!(binlik_ayrac(999), "999");
        assert_eq!(binlik_ayrac(1_000), "1,000");
        assert_eq!(binlik_ayrac(1_234_567), "1,234,567");

        assert_eq!(pozisyon_etiketle(1_234, Olcek::Bp), "1,234 bp");
        assert_eq!(pozisyon_etiketle(12_500, Olcek::Kb), "12.5 kb");
        assert_eq!(pozisyon_etiketle(2_000, Olcek::Kb), "2 kb");
        assert_eq!(pozisyon_etiketle(1_500_000, Olcek::Mb), "1.5 Mb");
    }

    #[test]
    fn cetvel_isaretleri_bolge_icinde_ve_sirali() {
        let t = Tuval::yeni(1000.0, GenomBolge::yeni("chr1", 1, 1000).unwrap());
        let c = cetvel(&t, 10);
        assert_eq!(c.olcek, Olcek::Kb);
        assert!(!c.isaretler.is_empty());
        // Tüm işaretler bölge içinde ve x soldan sağa artar.
        let mut onceki_x = f32::MIN;
        for m in &c.isaretler {
            assert!(t.bolge.kapsar(m.pos), "işaret bölge dışı: {}", m.pos);
            assert!(m.x_px >= onceki_x, "işaretler sıralı olmalı");
            onceki_x = m.x_px;
        }
        // En az bir büyük (etiketli) işaret var.
        assert!(c.isaretler.iter().any(|m| m.buyuk && !m.etiket.is_empty()));
    }

    #[test]
    fn cetvel_buyuk_isaret_adim_katinda() {
        let t = Tuval::yeni(1000.0, GenomBolge::yeni("chr1", 100, 1099).unwrap());
        let c = cetvel(&t, 10);
        let adim = yuvarlak_adim(1000, 10); // 100
        for m in c.isaretler.iter().filter(|m| m.buyuk) {
            assert_eq!(m.pos % adim, 0, "büyük işaret adım katı olmalı");
        }
    }
}
