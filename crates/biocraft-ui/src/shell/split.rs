//! Yan-yana bölme (split) — saf yön/oran mantığı + geometri (egui'siz, test edilebilir) — İP-03.
//!
//! Editör/Canvas alanı iki veriyi (örn. iki BAM) karşılaştırmak için **yatay** (sol|sağ) veya
//! **dikey** (üst/alt) bölünebilir.  Bu modül yalnızca *nasıl bölüneceğini* hesaplar: bir dikdörtgeni
//! verili oranda iki çocuğa ayırır ve aradaki tutamağı (gutter) ayırır.  Çizim ([`editor_area`])
//! bu geometriyi kullanır; böylece bölme oranı/kenar durumları egui'den bağımsız test edilir.
// MK-52: bu dosyada renk yok; yalnızca düzen geometrisi.

use biocraft_state::BolmeYonuSecimi;

/// Editör alanının bölme yönü (kalıcı [`BolmeYonuSecimi`]'nin UI tarafı karşılığı).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BolmeYonu {
    /// Bölme yok — tek grup (varsayılan).
    #[default]
    Yok,
    /// Yatay: iki grup yan-yana (sol | sağ).
    Yatay,
    /// Dikey: iki grup alt-alta (üst / alt).
    Dikey,
}

impl BolmeYonu {
    /// Görünümde döngüsel sonraki yön (Yok → Yatay → Dikey → Yok).
    pub fn sonraki(self) -> Self {
        match self {
            BolmeYonu::Yok => BolmeYonu::Yatay,
            BolmeYonu::Yatay => BolmeYonu::Dikey,
            BolmeYonu::Dikey => BolmeYonu::Yok,
        }
    }

    /// Bölme etkin mi (ikinci grup gösterilmeli mi)?
    pub fn bolundu_mu(self) -> bool {
        !matches!(self, BolmeYonu::Yok)
    }

    /// Yön düğmesi/menüsü için yerelleştirilmemiş kısa simge.
    pub fn ikon(self) -> &'static str {
        match self {
            BolmeYonu::Yok => "▢",
            BolmeYonu::Yatay => "▥",
            BolmeYonu::Dikey => "▤",
        }
    }

    /// Kalıcı (nötr) seçimden UI yönüne eşler (L2 → L4).
    pub fn secimden(s: BolmeYonuSecimi) -> Self {
        match s {
            BolmeYonuSecimi::Yok => BolmeYonu::Yok,
            BolmeYonuSecimi::Yatay => BolmeYonu::Yatay,
            BolmeYonuSecimi::Dikey => BolmeYonu::Dikey,
        }
    }

    /// UI yönünü kalıcı (nötr) seçime eşler (L4 → L2).
    pub fn secime(self) -> BolmeYonuSecimi {
        match self {
            BolmeYonu::Yok => BolmeYonuSecimi::Yok,
            BolmeYonu::Yatay => BolmeYonuSecimi::Yatay,
            BolmeYonu::Dikey => BolmeYonuSecimi::Dikey,
        }
    }
}

/// Bölme oranının izinli aralığı (her iki çocuk da görünür kalsın).
pub const ORAN_MIN: f32 = 0.1;
/// Bölme oranının izinli üst sınırı.
pub const ORAN_MAX: f32 = 0.9;
/// Çocuklar arasındaki tutamak (gutter) genişliği — mantıksal piksel.
pub const TUTAMAK: f32 = 6.0;

/// Bölme oranını `[ORAN_MIN, ORAN_MAX]`'a sıkıştırır (NaN → 0.5).
pub fn orani_sikistir(oran: f32) -> f32 {
    if !oran.is_finite() {
        0.5
    } else {
        oran.clamp(ORAN_MIN, ORAN_MAX)
    }
}

/// Bir `(genislik, yukseklik)` alanını, verili yön + oranla iki çocuğa böler.
///
/// Dönüş: `(birincil, ikincil)` — her biri `(genislik, yukseklik)`.  Aradaki [`TUTAMAK`] payı
/// düşülür; `birincil` payı = `oran`.  `Yok` yönünde ikinci çocuk sıfır boyutludur (kullanılmaz).
/// Saf fonksiyon: yalnızca aritmetik (egui `Rect`'e bağımlı değil → kolay test).
pub fn bol_boyut(
    yon: BolmeYonu,
    oran: f32,
    genislik: f32,
    yukseklik: f32,
) -> ((f32, f32), (f32, f32)) {
    let o = orani_sikistir(oran);
    match yon {
        BolmeYonu::Yok => ((genislik, yukseklik), (0.0, 0.0)),
        BolmeYonu::Yatay => {
            let kullanilabilir = (genislik - TUTAMAK).max(0.0);
            let sol = kullanilabilir * o;
            let sag = kullanilabilir - sol;
            ((sol, yukseklik), (sag, yukseklik))
        }
        BolmeYonu::Dikey => {
            let kullanilabilir = (yukseklik - TUTAMAK).max(0.0);
            let ust = kullanilabilir * o;
            let alt = kullanilabilir - ust;
            ((genislik, ust), (genislik, alt))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yon_dongusu_uc_durumu_gezer() {
        assert_eq!(BolmeYonu::Yok.sonraki(), BolmeYonu::Yatay);
        assert_eq!(BolmeYonu::Yatay.sonraki(), BolmeYonu::Dikey);
        assert_eq!(BolmeYonu::Dikey.sonraki(), BolmeYonu::Yok);
    }

    #[test]
    fn secim_gidis_donus_tutarli() {
        for y in [BolmeYonu::Yok, BolmeYonu::Yatay, BolmeYonu::Dikey] {
            assert_eq!(BolmeYonu::secimden(y.secime()), y);
        }
        assert!(!BolmeYonu::Yok.bolundu_mu());
        assert!(BolmeYonu::Yatay.bolundu_mu());
    }

    #[test]
    fn oran_araliga_zorlanir() {
        assert_eq!(orani_sikistir(0.0), ORAN_MIN);
        assert_eq!(orani_sikistir(1.0), ORAN_MAX);
        assert_eq!(orani_sikistir(0.5), 0.5);
        assert_eq!(orani_sikistir(f32::NAN), 0.5);
    }

    #[test]
    fn yatay_bolme_genisligi_paylastirir() {
        // 206 genişlik, tutamak 6 → 200 paylaşılır; oran 0.5 → 100|100.
        let ((sw, sh), (gw, gh)) = bol_boyut(BolmeYonu::Yatay, 0.5, 206.0, 400.0);
        assert!((sw - 100.0).abs() < 1e-3);
        assert!((gw - 100.0).abs() < 1e-3);
        assert_eq!(sh, 400.0);
        assert_eq!(gh, 400.0);
        // İki çocuk + tutamak = toplam genişlik.
        assert!((sw + gw + TUTAMAK - 206.0).abs() < 1e-3);
    }

    #[test]
    fn dikey_bolme_yuksekligi_paylastirir() {
        let ((bw, bh), (iw, ih)) = bol_boyut(BolmeYonu::Dikey, 0.25, 300.0, 206.0);
        assert!((bh - 50.0).abs() < 1e-3); // 200 * 0.25
        assert!((ih - 150.0).abs() < 1e-3);
        assert_eq!(bw, 300.0);
        assert_eq!(iw, 300.0);
    }

    #[test]
    fn bolunmemiste_ikincil_sifir() {
        let (birincil, ikincil) = bol_boyut(BolmeYonu::Yok, 0.5, 500.0, 400.0);
        assert_eq!(birincil, (500.0, 400.0));
        assert_eq!(ikincil, (0.0, 0.0));
    }
}
