//! Mağaza **arama / kategori / filtre / sıralama** — saf mantık (egui'siz, testlenir).
//!
//! VS Code standardı: aranabilir + kategoriye göre + türe/fiyata göre filtre + sıralama
//! (popüler/yeni/puan/ad).  Arama anlık olmalı → basit, tahsisatsız alt-dizge eşleşmesi
//! (önceden hesaplı saman; İP-12 ayar araması ile aynı çizgi).  `nucleo` gibi ağır bir bağımlılık
//! eklenmez (proje "yeni dış bağımlılık YOK" ilkesi).

use biocraft_net::{Kategori, OgeTuru, PazarOgesi};

/// Mağaza sıralama ölçütü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Siralama {
    /// En çok indirilen önce (popüler).
    #[default]
    Populer,
    /// En son güncellenen önce (yeni).
    Yeni,
    /// En yüksek puan önce.
    Puan,
    /// Ada göre (A→Z).
    Ad,
}

impl Siralama {
    /// Tüm sıralama ölçütleri (seçici için).
    pub const TUMU: &'static [Siralama] = &[
        Siralama::Populer,
        Siralama::Yeni,
        Siralama::Puan,
        Siralama::Ad,
    ];

    /// İki dilli etiket.
    pub fn etiket(self, tr: bool) -> &'static str {
        match (self, tr) {
            (Siralama::Populer, true) => "Popüler",
            (Siralama::Populer, false) => "Popular",
            (Siralama::Yeni, true) => "Yeni",
            (Siralama::Yeni, false) => "Newest",
            (Siralama::Puan, true) => "Puan",
            (Siralama::Puan, false) => "Rating",
            (Siralama::Ad, true) => "Ad (A→Z)",
            (Siralama::Ad, false) => "Name (A→Z)",
        }
    }
}

/// Mağaza filtre durumu (arama çubuğu + sol kategoriler + tür/fiyat/doğrulama süzgeçleri).
#[derive(Debug, Clone, Default)]
pub struct PazarSuzgec {
    /// Arama sorgusu (çoklu-sözcük AND alt-dizge).
    pub sorgu: String,
    /// Seçili kategori (None = tümü).
    pub kategori: Option<Kategori>,
    /// Seçili tür (None = tümü).
    pub tur: Option<OgeTuru>,
    /// Yalnızca ücretsiz/açık kaynak göster.
    pub yalniz_ucretsiz: bool,
    /// Yalnızca güven rozetli (resmi/doğrulanmış yayıncı) göster.
    pub yalniz_dogrulanmis: bool,
}

impl PazarSuzgec {
    /// Herhangi bir filtre etkin mi (arama dahil)?  Boş-durum mesajı için yardımcı.
    pub fn etkin_mi(&self) -> bool {
        !self.sorgu.trim().is_empty()
            || self.kategori.is_some()
            || self.tur.is_some()
            || self.yalniz_ucretsiz
            || self.yalniz_dogrulanmis
    }

    /// Tek bir öğe bu filtreden geçiyor mu?
    fn gecer_mi(&self, oge: &PazarOgesi, tr: bool) -> bool {
        if let Some(k) = self.kategori {
            if oge.kategori != k {
                return false;
            }
        }
        if let Some(t) = self.tur {
            if oge.tur != t {
                return false;
            }
        }
        if self.yalniz_ucretsiz && !oge.fiyat.ucretsiz_mi() {
            return false;
        }
        if self.yalniz_dogrulanmis && !oge.dogrulama.guven_rozeti_mi() {
            return false;
        }
        // Çoklu-sözcük AND: her sözcük samanda geçmeli.
        let saman = oge.saman(tr);
        self.sorgu
            .split_whitespace()
            .all(|kelime| saman.contains(&kelime.to_lowercase()))
    }
}

/// Öğeleri filtreleyip sıralar; sonucu **orijinal dizideki indeksler** olarak döner
/// (çağıran öğelere indeksle erişir → klon yok).
pub fn suz_ve_sirala(
    ogeler: &[PazarOgesi],
    suzgec: &PazarSuzgec,
    siralama: Siralama,
    tr: bool,
) -> Vec<usize> {
    let mut indeksler: Vec<usize> = ogeler
        .iter()
        .enumerate()
        .filter(|(_, o)| suzgec.gecer_mi(o, tr))
        .map(|(i, _)| i)
        .collect();

    indeksler.sort_by(|&a, &b| {
        let (oa, ob) = (&ogeler[a], &ogeler[b]);
        match siralama {
            Siralama::Populer => ob.indirme.cmp(&oa.indirme),
            // ISO tarih dizgesi (YYYY-MM-DD) sözlük sırası = kronolojik sıra → en yeni önce.
            Siralama::Yeni => ob.son_guncelleme.cmp(&oa.son_guncelleme),
            Siralama::Puan => ob
                .puan
                .partial_cmp(&oa.puan)
                .unwrap_or(std::cmp::Ordering::Equal),
            Siralama::Ad => oa.ad.to_lowercase().cmp(&ob.ad.to_lowercase()),
        }
        // Kararlı ikincil sıra: ada göre (eşitlikte deterministik).
        .then_with(|| oa.ad.cmp(&ob.ad))
    });
    indeksler
}

#[cfg(test)]
mod tests {
    use super::*;
    use biocraft_net::{DogrulamaDurumu, Fiyat};

    fn ornek() -> Vec<PazarOgesi> {
        let mut a = PazarOgesi::yeni(
            "a",
            "Alfa Analiz",
            "Acme",
            OgeTuru::Eklenti,
            Kategori::Analiz,
        );
        a.indirme = 100;
        a.puan = 4.0;
        a.son_guncelleme = "2026-01-01".into();
        a.fiyat = Fiyat::Ucretli { bio_kredi: 5 };
        a.dogrulama = DogrulamaDurumu::IncelemeBekliyor;

        let mut b = PazarOgesi::yeni(
            "b",
            "Beta Görsel",
            "BioCraft",
            OgeTuru::Sablon,
            Kategori::Gorsellestirme,
        );
        b.indirme = 500;
        b.puan = 4.9;
        b.son_guncelleme = "2026-06-01".into();
        b.fiyat = Fiyat::Ucretsiz;
        b.dogrulama = DogrulamaDurumu::Resmi;

        let mut c = PazarOgesi::yeni(
            "c",
            "Gama Veri",
            "Acme",
            OgeTuru::VeriSeti,
            Kategori::Veritabani,
        );
        c.indirme = 300;
        c.puan = 4.5;
        c.son_guncelleme = "2026-03-15".into();
        c.fiyat = Fiyat::AcikKaynak;
        c.dogrulama = DogrulamaDurumu::Kuratorlu;
        vec![a, b, c]
    }

    #[test]
    fn populer_siralama_indirmeye_gore() {
        let o = ornek();
        let s = suz_ve_sirala(&o, &PazarSuzgec::default(), Siralama::Populer, true);
        assert_eq!(o[s[0]].kimlik, "b"); // 500
        assert_eq!(o[s[1]].kimlik, "c"); // 300
        assert_eq!(o[s[2]].kimlik, "a"); // 100
    }

    #[test]
    fn yeni_siralama_tarihe_gore() {
        let o = ornek();
        let s = suz_ve_sirala(&o, &PazarSuzgec::default(), Siralama::Yeni, true);
        assert_eq!(o[s[0]].kimlik, "b"); // 2026-06-01
        assert_eq!(o[s[2]].kimlik, "a"); // 2026-01-01
    }

    #[test]
    fn puan_siralama() {
        let o = ornek();
        let s = suz_ve_sirala(&o, &PazarSuzgec::default(), Siralama::Puan, true);
        assert_eq!(o[s[0]].kimlik, "b"); // 4.9
    }

    #[test]
    fn ad_siralama_alfabetik() {
        let o = ornek();
        let s = suz_ve_sirala(&o, &PazarSuzgec::default(), Siralama::Ad, true);
        assert_eq!(o[s[0]].ad, "Alfa Analiz");
        assert_eq!(o[s[2]].ad, "Gama Veri");
    }

    #[test]
    fn arama_alt_dizge_bulur() {
        let o = ornek();
        let f = PazarSuzgec {
            sorgu: "acme".into(),
            ..Default::default()
        };
        let s = suz_ve_sirala(&o, &f, Siralama::Ad, true);
        assert_eq!(s.len(), 2); // a + c (yayıncı Acme)
    }

    #[test]
    fn coklu_sozcuk_and() {
        let o = ornek();
        let f = PazarSuzgec {
            sorgu: "alfa acme".into(),
            ..Default::default()
        };
        let s = suz_ve_sirala(&o, &f, Siralama::Ad, true);
        assert_eq!(s.len(), 1);
        assert_eq!(o[s[0]].kimlik, "a");
    }

    #[test]
    fn kategori_filtresi() {
        let o = ornek();
        let f = PazarSuzgec {
            kategori: Some(Kategori::Gorsellestirme),
            ..Default::default()
        };
        let s = suz_ve_sirala(&o, &f, Siralama::Ad, true);
        assert_eq!(s.len(), 1);
        assert_eq!(o[s[0]].kimlik, "b");
    }

    #[test]
    fn tur_filtresi() {
        let o = ornek();
        let f = PazarSuzgec {
            tur: Some(OgeTuru::VeriSeti),
            ..Default::default()
        };
        let s = suz_ve_sirala(&o, &f, Siralama::Ad, true);
        assert_eq!(s.len(), 1);
        assert_eq!(o[s[0]].kimlik, "c");
    }

    #[test]
    fn yalniz_ucretsiz_filtresi() {
        let o = ornek();
        let f = PazarSuzgec {
            yalniz_ucretsiz: true,
            ..Default::default()
        };
        let s = suz_ve_sirala(&o, &f, Siralama::Ad, true);
        // a ücretli → dışarıda; b + c ücretsiz/açık kaynak.
        assert_eq!(s.len(), 2);
        assert!(s.iter().all(|&i| o[i].kimlik != "a"));
    }

    #[test]
    fn yalniz_dogrulanmis_filtresi() {
        let o = ornek();
        let f = PazarSuzgec {
            yalniz_dogrulanmis: true,
            ..Default::default()
        };
        let s = suz_ve_sirala(&o, &f, Siralama::Ad, true);
        // Yalnız b (Resmi) güven rozetli; c (Küratörlü) ve a (Beklemede) değil.
        assert_eq!(s.len(), 1);
        assert_eq!(o[s[0]].kimlik, "b");
    }

    #[test]
    fn etkin_mi_dogru() {
        assert!(!PazarSuzgec::default().etkin_mi());
        let f = PazarSuzgec {
            sorgu: "x".into(),
            ..Default::default()
        };
        assert!(f.etkin_mi());
    }
}
