//! 2B çizim (plot) temeli — coverage/çizgi/scatter (İP-04).
//!
//! Saf veri modeli + veri→ekran koordinat dönüşümü (egui/wgpu'dan bağımsız, MK-40).  UI katmanı
//! bu modelin ürettiği [`CizimKomut`] listesini egui `Painter` ile çizer; ağır/çok-yoğun
//! çizimler ileride wgpu shader'a taşınır.  Görünür-alan **culling** + **LOD seyreltme**
//! (bkz. [`crate::lod`]) burada uygulanır → çok büyük seride bile kare bütçesi korunur (MK-04).

use crate::lod::{seyrelt, Dortgen};

/// Bir veri noktası (x, y).  Bilimsel veri için f64 (sayısal hassasiyet, İP-04 "gerektiğinde f64").
pub type Nokta = [f64; 2];

/// Bir serinin nasıl çizileceği.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriTur {
    /// Ardışık noktaları çizgiyle bağla (coverage/çizgi grafiği).
    Cizgi,
    /// Her noktayı ayrı işaretle (scatter/dağılım).
    Sacilim,
}

/// Tek bir veri serisi.  Rengi doğrudan değil, **token anahtarıyla** taşır (MK-52).
#[derive(Debug, Clone)]
pub struct Seri {
    /// Açıklama/gösterge adı.
    pub ad: String,
    /// Çizim türü.
    pub tur: SeriTur,
    /// Veri noktaları (x artan sırada beklenir; culling bunu varsayar).
    pub noktalar: Vec<Nokta>,
    /// Çizgi kalınlığı / nokta yarıçapı (mantıksal px).
    pub kalinlik: f32,
    /// Rengin geleceği tasarım-token anahtarı (örn. "accent.primary", "info").
    pub renk_anahtari: String,
}

impl Seri {
    /// Bir çizgi serisi.
    pub fn cizgi(
        ad: impl Into<String>,
        noktalar: Vec<Nokta>,
        renk_anahtari: impl Into<String>,
    ) -> Self {
        Self {
            ad: ad.into(),
            tur: SeriTur::Cizgi,
            noktalar,
            kalinlik: 1.5,
            renk_anahtari: renk_anahtari.into(),
        }
    }

    /// Bir scatter serisi.
    pub fn sacilim(
        ad: impl Into<String>,
        noktalar: Vec<Nokta>,
        renk_anahtari: impl Into<String>,
    ) -> Self {
        Self {
            ad: ad.into(),
            tur: SeriTur::Sacilim,
            noktalar,
            kalinlik: 3.0,
            renk_anahtari: renk_anahtari.into(),
        }
    }
}

/// Bir eksenin veri aralığı (min, max).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aralik {
    /// Alt sınır.
    pub min: f64,
    /// Üst sınır.
    pub max: f64,
}

impl Aralik {
    /// Yeni aralık (min ≤ max güvence altına alınır).
    pub fn yeni(min: f64, max: f64) -> Self {
        if min <= max {
            Self { min, max }
        } else {
            Self { min: max, max: min }
        }
    }

    /// Aralık genişliği (0'a karşı korunur).
    pub fn genislik(&self) -> f64 {
        (self.max - self.min).max(f64::EPSILON)
    }
}

/// Tek bir çizim ilkel komutu (ekran/piksel uzayında, token anahtarıyla renklenir).
#[derive(Debug, Clone, PartialEq)]
pub enum Sekil {
    /// İki nokta arası çizgi parçası.
    Cizgi {
        /// Başlangıç (ekran px).
        a: [f32; 2],
        /// Bitiş (ekran px).
        b: [f32; 2],
    },
    /// Dolu işaretçi (scatter noktası).
    Nokta {
        /// Merkez (ekran px).
        merkez: [f32; 2],
        /// Yarıçap (px).
        yaricap: f32,
    },
}

/// Renklendirilmiş çizim komutu (hangi token anahtarıyla, ne kalınlıkta).
#[derive(Debug, Clone, PartialEq)]
pub struct CizimKomut {
    /// Çizilecek şekil.
    pub sekil: Sekil,
    /// Rengin geleceği token anahtarı.
    pub renk_anahtari: String,
    /// Çizgi/çevre kalınlığı (px).
    pub kalinlik: f32,
}

/// Bir 2B çizim: seriler + eksen aralıkları + görünür x-aralığı (yatay zoom/pan).
#[derive(Debug, Clone, Default)]
pub struct Plot2B {
    /// Çizilecek seriler.
    pub seriler: Vec<Seri>,
    /// Açık x-aralığı (None → verilerden otomatik).
    pub x_aralik: Option<Aralik>,
    /// Açık y-aralığı (None → verilerden otomatik).
    pub y_aralik: Option<Aralik>,
    /// Görünür x-aralığı (yatay viewport; None → tüm x).  Bunun dışındaki noktalar elenir.
    pub gorunur_x: Option<Aralik>,
}

impl Plot2B {
    /// Boş bir çizim.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir seri ekler.
    pub fn seri_ekle(mut self, seri: Seri) -> Self {
        self.seriler.push(seri);
        self
    }

    /// Tüm serilerden x ve y aralığını otomatik hesaplar (açıkça verilmemişse).
    pub fn otomatik_aralik(&self) -> (Aralik, Aralik) {
        if let (Some(x), Some(y)) = (self.x_aralik, self.y_aralik) {
            return (x, y);
        }
        let (mut xmin, mut xmax) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);
        for s in &self.seriler {
            for &[x, y] in &s.noktalar {
                xmin = xmin.min(x);
                xmax = xmax.max(x);
                ymin = ymin.min(y);
                ymax = ymax.max(y);
            }
        }
        if !xmin.is_finite() {
            // Veri yok → birim kare.
            xmin = 0.0;
            xmax = 1.0;
            ymin = 0.0;
            ymax = 1.0;
        }
        let x = self.x_aralik.unwrap_or(Aralik::yeni(xmin, xmax));
        // y'ye küçük bir tepe boşluğu (%5) ekle ki çizim kenara yapışmasın.
        let pay = (ymax - ymin).max(1.0) * 0.05;
        let y = self
            .y_aralik
            .unwrap_or(Aralik::yeni(ymin - pay, ymax + pay));
        (x, y)
    }

    /// Veri noktasını ekran (piksel) koordinatına eşler.  y ekranda ters (yukarı = küçük y).
    fn ekrana(nokta: Nokta, x: &Aralik, y: &Aralik, alan: &Dortgen) -> [f32; 2] {
        let tx = ((nokta[0] - x.min) / x.genislik()) as f32;
        let ty = ((nokta[1] - y.min) / y.genislik()) as f32;
        [alan.x + tx * alan.w, alan.alt() - ty * alan.h]
    }

    /// Verilen ekran dikdörtgenine çizim komutlarını üretir.
    ///
    /// **Culling:** görünür x-aralığı dışındaki noktalar atlanır.  **LOD:** görünür nokta
    /// sayısı, alanın piksel genişliğinin ~2 katını aşarsa seri seyreltilir (decimation) —
    /// böylece milyonlarca noktalı coverage bile kare bütçesini bozmadan çizilir (MK-04).
    pub fn ciz_komutlari(&self, alan: Dortgen) -> Vec<CizimKomut> {
        let (x_aralik, y_aralik) = self.otomatik_aralik();
        // Piksel başına ~2 örnekten fazlasını çizmenin görsel faydası yok.
        let azami_nokta = ((alan.w.max(1.0)) * 2.0) as usize;
        let mut komutlar = Vec::new();

        for seri in &self.seriler {
            // 1) Culling: yalnızca görünür x-aralığındaki noktaların indekslerini topla.
            let gorunur: Vec<usize> = seri
                .noktalar
                .iter()
                .enumerate()
                .filter(|(_, &[x, _])| {
                    self.gorunur_x
                        .map(|g| x >= g.min && x <= g.max)
                        .unwrap_or(true)
                })
                .map(|(i, _)| i)
                .collect();

            // 2) LOD: çok yoğunsa eşit aralıkla seyrelt.
            let secili: Vec<usize> = if gorunur.len() > azami_nokta {
                seyrelt(gorunur.len(), azami_nokta)
                    .into_iter()
                    .map(|k| gorunur[k])
                    .collect()
            } else {
                gorunur
            };

            // 3) Şekilleri üret.
            match seri.tur {
                SeriTur::Cizgi => {
                    for pencere in secili.windows(2) {
                        let a =
                            Self::ekrana(seri.noktalar[pencere[0]], &x_aralik, &y_aralik, &alan);
                        let b =
                            Self::ekrana(seri.noktalar[pencere[1]], &x_aralik, &y_aralik, &alan);
                        komutlar.push(CizimKomut {
                            sekil: Sekil::Cizgi { a, b },
                            renk_anahtari: seri.renk_anahtari.clone(),
                            kalinlik: seri.kalinlik,
                        });
                    }
                }
                SeriTur::Sacilim => {
                    for &i in &secili {
                        let m = Self::ekrana(seri.noktalar[i], &x_aralik, &y_aralik, &alan);
                        komutlar.push(CizimKomut {
                            sekil: Sekil::Nokta {
                                merkez: m,
                                yaricap: seri.kalinlik,
                            },
                            renk_anahtari: seri.renk_anahtari.clone(),
                            kalinlik: seri.kalinlik,
                        });
                    }
                }
            }
        }
        komutlar
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alan() -> Dortgen {
        Dortgen::yeni(0.0, 0.0, 200.0, 100.0)
    }

    #[test]
    fn otomatik_aralik_verilerden_hesaplanir() {
        let p = Plot2B::yeni().seri_ekle(Seri::cizgi(
            "a",
            vec![[0.0, 0.0], [10.0, 5.0], [20.0, 2.0]],
            "accent.primary",
        ));
        let (x, _y) = p.otomatik_aralik();
        assert_eq!(x.min, 0.0);
        assert_eq!(x.max, 20.0);
    }

    #[test]
    fn ekran_esleme_kose_noktalari_dogru() {
        // x=min,y=min → sol-alt köşe; x=max,y=max → sağ-üst köşe.
        let p = Plot2B {
            x_aralik: Some(Aralik::yeni(0.0, 10.0)),
            y_aralik: Some(Aralik::yeni(0.0, 10.0)),
            ..Default::default()
        };
        let (x, y) = p.otomatik_aralik();
        let sol_alt = Plot2B::ekrana([0.0, 0.0], &x, &y, &alan());
        let sag_ust = Plot2B::ekrana([10.0, 10.0], &x, &y, &alan());
        assert_eq!(sol_alt, [0.0, 100.0]); // alt kenar
        assert_eq!(sag_ust, [200.0, 0.0]); // üst kenar
    }

    #[test]
    fn cizgi_serisi_n_eksi_bir_segment_uretir() {
        let p = Plot2B::yeni().seri_ekle(Seri::cizgi(
            "a",
            vec![[0.0, 0.0], [1.0, 1.0], [2.0, 0.0], [3.0, 1.0]],
            "accent.primary",
        ));
        let k = p.ciz_komutlari(alan());
        // 4 nokta → 3 çizgi segmenti.
        assert_eq!(k.len(), 3);
        assert!(matches!(k[0].sekil, Sekil::Cizgi { .. }));
        assert_eq!(k[0].renk_anahtari, "accent.primary");
    }

    #[test]
    fn scatter_serisi_nokta_basina_komut() {
        let p = Plot2B::yeni().seri_ekle(Seri::sacilim(
            "s",
            vec![[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]],
            "info",
        ));
        let k = p.ciz_komutlari(alan());
        assert_eq!(k.len(), 3);
        assert!(matches!(k[0].sekil, Sekil::Nokta { .. }));
    }

    #[test]
    fn gorunur_x_disindaki_noktalar_elenir() {
        // 0..100 noktalar; yalnızca 40..60 görünür → çizilen segment sayısı belirgin azalır.
        let noktalar: Vec<Nokta> = (0..=100).map(|i| [i as f64, (i % 7) as f64]).collect();
        let mut p = Plot2B::yeni().seri_ekle(Seri::cizgi("c", noktalar, "accent.primary"));
        p.gorunur_x = Some(Aralik::yeni(40.0, 60.0));
        let k = p.ciz_komutlari(alan());
        // 21 görünür nokta → en çok 20 segment (culling çalıştı; 100 değil).
        assert!(k.len() <= 20, "culling sonrası segment sayısı: {}", k.len());
        assert!(k.len() >= 19);
    }

    #[test]
    fn lod_cok_yogun_seriyi_seyreltir() {
        // 100k nokta, 200 px genişlik → en çok ~400 örnek; segment sayısı buna sınırlı.
        let noktalar: Vec<Nokta> = (0..100_000).map(|i| [i as f64, (i % 13) as f64]).collect();
        let p = Plot2B::yeni().seri_ekle(Seri::cizgi("yogun", noktalar, "accent.primary"));
        let k = p.ciz_komutlari(alan()); // alan genişliği 200 → azami ~400 nokta
        assert!(
            k.len() <= 400,
            "LOD seyreltme sonrası: {} (kare bütçesi korunur)",
            k.len()
        );
        assert!(
            k.len() > 100,
            "yine de anlamlı çözünürlük kalmalı: {}",
            k.len()
        );
    }
}
