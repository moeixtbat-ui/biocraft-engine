//! ÇE-04 — **Genotip ızgarası**: çok-örnekli genotip matrisi + **zigosite** (hom/het/ref)
//! renklendirme + **sanal liste** (binlerce satır akıcı).
//!
//! Izgara satırları = varyantlar, sütunları = örnekler.  Hücre, [`VaryantKaydi.genotipler`]
//! (örnek başına GT metni) üzerinden çözülür; yalnız **görünen pencere** maddileştirilir (sanal
//! kaydırma → büyük matriste de akıcı).

use super::query::VaryantSatiri;

/// Bir örnek-varyant hücresinin zigositesi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zigosite {
    /// Homozigot referans (0/0).
    HomRef,
    /// Heterozigot (0/1, 1/2 …).
    Het,
    /// Homozigot alternatif (1/1, 2/2 …).
    HomAlt,
    /// Eksik/çağrılmamış (./.).
    Eksik,
    /// Çözümlenemeyen/diğer.
    Diger,
}

impl Zigosite {
    /// Kısa etiket.
    pub fn etiket(self) -> &'static str {
        match self {
            Zigosite::HomRef => "hom-ref",
            Zigosite::Het => "het",
            Zigosite::HomAlt => "hom-alt",
            Zigosite::Eksik => "eksik",
            Zigosite::Diger => "diğer",
        }
    }
}

/// Bir GT metnini (`0/1`, `1|1`, `./.`) zigositeye çözer.
pub fn zigosite_coz(gt: &str) -> Zigosite {
    let t = gt.trim();
    if t.is_empty() || t == "." {
        return Zigosite::Eksik;
    }
    let aleller: Vec<&str> = t.split(['/', '|']).collect();
    if aleller.is_empty() {
        return Zigosite::Diger;
    }
    if aleller.contains(&".") {
        return Zigosite::Eksik;
    }
    // Tüm aleller sayıya çözülmeli.
    let sayilar: Option<Vec<u32>> = aleller.iter().map(|a| a.parse::<u32>().ok()).collect();
    let Some(sayilar) = sayilar else {
        return Zigosite::Diger;
    };
    if sayilar.is_empty() {
        return Zigosite::Diger;
    }
    let hepsi_ayni = sayilar.iter().all(|&n| n == sayilar[0]);
    if hepsi_ayni {
        if sayilar[0] == 0 {
            Zigosite::HomRef
        } else {
            Zigosite::HomAlt
        }
    } else {
        Zigosite::Het
    }
}

/// Izgaranın tek bir hücresi: GT metni + zigosite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenotipHucre {
    /// Ham GT metni (`0/1` …).
    pub gt: String,
    /// Çözülmüş zigosite (renklendirme için).
    pub zigosite: Zigosite,
}

/// Çok-örnekli genotip ızgarası — örnek adları + (filtreli) varyant satırları üzerinde.
pub struct GenotipIzgara<'a> {
    ornekler: &'a [String],
    satirlar: &'a [VaryantSatiri],
}

impl<'a> GenotipIzgara<'a> {
    /// Izgarayı kurar (sahiplik almaz; yalnız görüntüler).
    pub fn yeni(ornekler: &'a [String], satirlar: &'a [VaryantSatiri]) -> Self {
        Self { ornekler, satirlar }
    }

    /// Satır (varyant) sayısı.
    pub fn satir_sayisi(&self) -> usize {
        self.satirlar.len()
    }

    /// Sütun (örnek) sayısı.
    pub fn ornek_sayisi(&self) -> usize {
        self.ornekler.len()
    }

    /// Örnek adları.
    pub fn ornekler(&self) -> &[String] {
        self.ornekler
    }

    /// Tek bir hücre (sınır dışıysa `None`).
    pub fn hucre(&self, satir: usize, ornek: usize) -> Option<GenotipHucre> {
        let s = self.satirlar.get(satir)?;
        if ornek >= self.ornekler.len() {
            return None;
        }
        let gt = s
            .kayit
            .genotipler
            .get(ornek)
            .cloned()
            .unwrap_or_else(|| ".".to_string());
        let zigosite = zigosite_coz(&gt);
        Some(GenotipHucre { gt, zigosite })
    }

    /// **Sanal kaydırma** penceresi: `[ilk_satir, ilk_satir+adet)` satırlarının tüm örnek
    /// hücreleri (yalnız görünenler maddileştirilir).
    pub fn pencere(&self, ilk_satir: usize, adet: usize) -> Vec<Vec<GenotipHucre>> {
        let son = (ilk_satir + adet).min(self.satirlar.len());
        let mut sonuc = Vec::new();
        for satir in ilk_satir..son {
            let mut hucreler = Vec::with_capacity(self.ornekler.len());
            for ornek in 0..self.ornekler.len() {
                if let Some(h) = self.hucre(satir, ornek) {
                    hucreler.push(h);
                }
            }
            sonuc.push(hucreler);
        }
        sonuc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_io::VaryantKaydi;

    fn satir(gts: &[&str]) -> VaryantSatiri {
        VaryantSatiri::yeni(VaryantKaydi {
            kromozom: "chr1".into(),
            konum: 100,
            kimlik: ".".into(),
            referans: "A".into(),
            alternatifler: vec!["G".into()],
            kalite: Some(50.0),
            filtreler: vec!["PASS".into()],
            info: vec![],
            ornek_sayisi: gts.len(),
            format_anahtarlari: vec!["GT".into()],
            genotipler: gts.iter().map(|s| s.to_string()).collect(),
        })
    }

    #[test]
    fn zigosite_cozumleri() {
        assert_eq!(zigosite_coz("0/0"), Zigosite::HomRef);
        assert_eq!(zigosite_coz("0/1"), Zigosite::Het);
        assert_eq!(zigosite_coz("1|1"), Zigosite::HomAlt);
        assert_eq!(zigosite_coz("1/2"), Zigosite::Het);
        assert_eq!(zigosite_coz("./."), Zigosite::Eksik);
        assert_eq!(zigosite_coz("."), Zigosite::Eksik);
        assert_eq!(zigosite_coz(""), Zigosite::Eksik);
        assert_eq!(zigosite_coz("2/2"), Zigosite::HomAlt);
    }

    #[test]
    fn izgara_hucre_ve_pencere() {
        let ornekler = vec!["S1".to_string(), "S2".to_string(), "S3".to_string()];
        let satirlar = vec![satir(&["0/0", "0/1", "1/1"]), satir(&["./.", "0/0", "0/1"])];
        let izgara = GenotipIzgara::yeni(&ornekler, &satirlar);
        assert_eq!(izgara.satir_sayisi(), 2);
        assert_eq!(izgara.ornek_sayisi(), 3);

        let h = izgara.hucre(0, 1).unwrap();
        assert_eq!(h.gt, "0/1");
        assert_eq!(h.zigosite, Zigosite::Het);

        // Sanal pencere: yalnız 1. satır.
        let pencere = izgara.pencere(1, 1);
        assert_eq!(pencere.len(), 1);
        assert_eq!(pencere[0][0].zigosite, Zigosite::Eksik);
    }

    #[test]
    fn eksik_genotip_nokta() {
        let ornekler = vec!["S1".to_string(), "S2".to_string()];
        // Yalnız 1 GT verili → 2. örnek "." (eksik).
        let satirlar = vec![satir(&["0/1"])];
        let izgara = GenotipIzgara::yeni(&ornekler, &satirlar);
        assert_eq!(izgara.hucre(0, 1).unwrap().gt, ".");
        assert_eq!(izgara.hucre(0, 1).unwrap().zigosite, Zigosite::Eksik);
    }
}
