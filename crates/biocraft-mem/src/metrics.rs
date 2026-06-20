//! Performans metrikleri — tepe/ortalama toplayıcı (İP-08).
//!
//! Kare süresi, bellek tepe/ortalama, sıcaklık gibi büyüklükler için **saf**, ucuz bir
//! toplayıcı.  Durum çubuğunda metrik şeffaflığı (TDA) ve CI'de regresyon benchmark'ı
//! (spec: "performans regresyon eşik aşımı") için ortak temel.

/// Bir büyüklüğün tepe (max) + çalışan ortalamasını biriktirir.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TepeOrtalama {
    /// Eklenen örnek sayısı.
    adet: u64,
    /// Örneklerin toplamı (ortalama için).
    toplam: f64,
    /// Görülen en büyük örnek.
    tepe: f64,
}

impl TepeOrtalama {
    /// Boş bir toplayıcı.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir örnek ekler (negatifler 0'a kırpılır — süre/bayt negatif olamaz).
    pub fn ekle(&mut self, deger: f64) {
        let d = deger.max(0.0);
        self.adet += 1;
        self.toplam += d;
        if d > self.tepe {
            self.tepe = d;
        }
    }

    /// Şimdiye kadarki en büyük örnek (hiç örnek yoksa 0).
    pub fn tepe(&self) -> f64 {
        self.tepe
    }

    /// Çalışan ortalama (hiç örnek yoksa 0).
    pub fn ortalama(&self) -> f64 {
        if self.adet == 0 {
            0.0
        } else {
            self.toplam / self.adet as f64
        }
    }

    /// Eklenen örnek sayısı.
    pub fn adet(&self) -> u64 {
        self.adet
    }

    /// Verilen eşiği aşan bir tepe görüldü mü? (CI regresyon kontrolü için).
    pub fn tepe_asti_mi(&self, esik: f64) -> bool {
        self.tepe > esik
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bos_toplayici_sifir_dondurur() {
        let t = TepeOrtalama::yeni();
        assert_eq!(t.tepe(), 0.0);
        assert_eq!(t.ortalama(), 0.0);
        assert_eq!(t.adet(), 0);
    }

    #[test]
    fn tepe_ve_ortalama_dogru() {
        let mut t = TepeOrtalama::yeni();
        for d in [10.0, 20.0, 30.0] {
            t.ekle(d);
        }
        assert_eq!(t.tepe(), 30.0);
        assert_eq!(t.ortalama(), 20.0);
        assert_eq!(t.adet(), 3);
    }

    #[test]
    fn negatif_orneck_sifira_kirpilir() {
        let mut t = TepeOrtalama::yeni();
        t.ekle(-5.0);
        assert_eq!(t.tepe(), 0.0);
        assert_eq!(t.ortalama(), 0.0);
    }

    #[test]
    fn regresyon_esigi_yakalanir() {
        let mut t = TepeOrtalama::yeni();
        t.ekle(16.0); // ~16 ms kare
        t.ekle(50.0); // sıçrama
        assert!(t.tepe_asti_mi(33.0), "50 ms tepe 33 ms eşiğini aşmalı");
        assert!(!t.tepe_asti_mi(100.0));
    }
}
