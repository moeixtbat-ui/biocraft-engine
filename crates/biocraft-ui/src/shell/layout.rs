//! Kabuk düzeni — saf boyut sabitleri + sınırlama mantığı (egui'siz, test edilebilir).
//!
//! 6-bölge düzeninin (0.9 tablosu, İP-03) ölçüleri burada tek yerde tanımlıdır; tüm bölge
//! çizicileri (title/activity/side/status) bu sabitleri kullanır.  Boyutlar **mantıksal piksel**
//! (egui "point") cinsindendir; DPI/4K ölçeklemesini egui `pixels_per_point` uygular, dolayısıyla
//! aynı sabitler hem 1080p hem 4K monitörde doğru fiziksel boyuta ölçeklenir (MK-51).
// MK-52: bu dosyada renk yoktur; yalnızca düzen ölçüleri.

use std::ops::RangeInclusive;

/// Title Bar (+ klasik menü) yüksekliği — 0.9 tablosu: 32 px.
pub const BASLIK_YUKSEKLIK: f32 = 32.0;

/// Activity Bar genişliği (sol şerit) — 0.9 tablosu: 48 px.
pub const AKTIVITE_GENISLIK: f32 = 48.0;

/// Status Bar yüksekliği (alt) — 0.9 tablosu: 22 px.
pub const DURUM_YUKSEKLIK: f32 = 22.0;

/// Side Panel asgari genişliği — 0.9 tablosu: 200 px.
pub const YAN_PANEL_MIN: f32 = 200.0;

/// Side Panel azami genişliği — 0.9 tablosu: 600 px.
pub const YAN_PANEL_MAX: f32 = 600.0;

/// Side Panel makul başlangıç genişliği (kayıtlı değer yoksa).
pub const YAN_PANEL_VARSAYILAN: f32 = 260.0;

/// Side Panel genişliğini izinli `[MIN, MAX]` aralığına sıkıştırır.
///
/// Hem kullanıcı sürüklerken (egui `width_range`) hem de kalıcı durumdan geri yüklerken kullanılır;
/// böylece bozuk/aralık-dışı bir kayıtlı değer bile güvenli bir genişliğe çekilir (MK-28: güvenli
/// varsayılana düş).  NaN gibi geçersiz değer varsayılana indirgenir.
pub fn yan_panel_sikistir(genislik: f32) -> f32 {
    if !genislik.is_finite() {
        return YAN_PANEL_VARSAYILAN;
    }
    genislik.clamp(YAN_PANEL_MIN, YAN_PANEL_MAX)
}

/// Side Panel için egui `width_range`'ine verilecek aralık (200..=600).
pub fn yan_panel_araligi() -> RangeInclusive<f32> {
    YAN_PANEL_MIN..=YAN_PANEL_MAX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bolge_olculeri_09_tablosuyla_uyumlu() {
        // 0.9 tablosundaki sabit ölçüler (regresyon koruması).
        assert_eq!(BASLIK_YUKSEKLIK, 32.0);
        assert_eq!(AKTIVITE_GENISLIK, 48.0);
        assert_eq!(DURUM_YUKSEKLIK, 22.0);
        assert_eq!(YAN_PANEL_MIN, 200.0);
        assert_eq!(YAN_PANEL_MAX, 600.0);
    }

    #[test]
    fn yan_panel_sikistir_araligi_uygular() {
        assert_eq!(
            yan_panel_sikistir(100.0),
            YAN_PANEL_MIN,
            "altı min'e çekilir"
        );
        assert_eq!(
            yan_panel_sikistir(900.0),
            YAN_PANEL_MAX,
            "üstü max'a çekilir"
        );
        assert_eq!(yan_panel_sikistir(300.0), 300.0, "aralık içi korunur");
    }

    #[test]
    fn yan_panel_sikistir_gecersizi_varsayilana_indirir() {
        assert_eq!(yan_panel_sikistir(f32::NAN), YAN_PANEL_VARSAYILAN);
        assert_eq!(yan_panel_sikistir(f32::INFINITY), YAN_PANEL_VARSAYILAN);
    }

    #[test]
    fn varsayilan_aralik_icinde() {
        assert!((YAN_PANEL_MIN..=YAN_PANEL_MAX).contains(&YAN_PANEL_VARSAYILAN));
    }
}
