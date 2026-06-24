//! ÇE-04 — **Varyant izi**: filtreli varyantların genomik konumda gösterimi + tür (SNV/indel)
//! **görsel ayrımı**.
//!
//! Çizim, ÇE-02 genom tarayıcısının varyant iziyle (render-bağımsız [`VaryantParcasi`]) yapılır;
//! bu modül **filtreli alt-kümeyi** o ize besler ve tür→renk anlamsal eşlemesini + bir efsane
//! (legend) sağlar.  Böylece tablo/filtre ile genom tarayıcı **aynı** varyant kümesini gösterir.

// Türü ve çizim-parçasını ÇE-02'den yeniden kullan (tek tanım → tutarlılık).
pub use crate::genome_browser::veri::{VaryantParcasi, VaryantTuru};

use super::query::VaryantSatiri;

/// Varyant türünün anlamsal renk jetonu (motor tasarım jetonuna eşlenir; sabit RGB yok).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaryantIzRengi {
    /// SNV/SNP (tek-nükleotit).
    Snv,
    /// İnsersiyon.
    Insersiyon,
    /// Delesyon.
    Delesyon,
    /// Diğer/karmaşık.
    Diger,
}

/// Tür → renk jetonu.
pub fn tur_rengi(tur: VaryantTuru) -> VaryantIzRengi {
    match tur {
        VaryantTuru::Snv => VaryantIzRengi::Snv,
        VaryantTuru::Insersiyon => VaryantIzRengi::Insersiyon,
        VaryantTuru::Delesyon => VaryantIzRengi::Delesyon,
        VaryantTuru::Diger => VaryantIzRengi::Diger,
    }
}

/// Filtreli satırları genom tarayıcı varyant izine besler (her satır → [`VaryantParcasi`]).
pub fn iz_parcalari(satirlar: &[VaryantSatiri]) -> Vec<VaryantParcasi> {
    satirlar
        .iter()
        .map(|s| VaryantParcasi::kayittan(&s.kayit))
        .collect()
}

/// Efsane (legend): tür → etiket — UI'da renk açıklaması için.
pub fn efsane() -> Vec<(VaryantTuru, &'static str)> {
    vec![
        (VaryantTuru::Snv, VaryantTuru::Snv.etiket()),
        (VaryantTuru::Insersiyon, VaryantTuru::Insersiyon.etiket()),
        (VaryantTuru::Delesyon, VaryantTuru::Delesyon.etiket()),
        (VaryantTuru::Diger, VaryantTuru::Diger.etiket()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_io::VaryantKaydi;

    fn satir(pos: usize, r: &str, a: &str) -> VaryantSatiri {
        VaryantSatiri::yeni(VaryantKaydi {
            kromozom: "chr1".into(),
            konum: pos,
            kimlik: ".".into(),
            referans: r.into(),
            alternatifler: vec![a.into()],
            kalite: Some(50.0),
            filtreler: vec!["PASS".into()],
            info: vec![],
            ornek_sayisi: 0,
            format_anahtarlari: vec![],
            genotipler: vec![],
        })
    }

    #[test]
    fn iz_parcalari_turleri_korur() {
        let satirlar = vec![satir(100, "A", "G"), satir(200, "A", "ACGT")];
        let parcalar = iz_parcalari(&satirlar);
        assert_eq!(parcalar.len(), 2);
        assert_eq!(parcalar[0].tur, VaryantTuru::Snv);
        assert_eq!(parcalar[1].tur, VaryantTuru::Insersiyon);
        assert_eq!(parcalar[0].bas, 100);
    }

    #[test]
    fn tur_rengi_eslemesi() {
        assert_eq!(tur_rengi(VaryantTuru::Snv), VaryantIzRengi::Snv);
        assert_eq!(tur_rengi(VaryantTuru::Delesyon), VaryantIzRengi::Delesyon);
        assert_eq!(efsane().len(), 4);
    }
}
