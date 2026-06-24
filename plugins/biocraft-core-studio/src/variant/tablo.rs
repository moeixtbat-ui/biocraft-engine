//! ÇE-04 — **Tablo görünümü**: sütunlu VCF tablosu (CHROM/POS/REF/ALT/QUAL/FILTER + seçili
//! INFO/FORMAT) + **sütun seç/gizle/sırala** + **sanal kaydırma** (yalnız görünen satırlar) +
//! gruplama.
//!
//! Tablo, sorgu sonucunun ([`SorguSonuc`](super::query::SorguSonuc)) maddileştirilmiş satırları
//! üzerinde çalışır → sanal kaydırma penceresi O(1) dilimleme ile alınır (yeniden-sorgu yok).

use super::query::VaryantSatiri;

/// Tablo sütunu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sutun {
    /// CHROM.
    Kromozom,
    /// POS (1-tabanlı).
    Konum,
    /// ID/rsID.
    Kimlik,
    /// REF.
    Referans,
    /// ALT (virgülle).
    Alternatif,
    /// QUAL.
    Kalite,
    /// FILTER.
    Filtre,
    /// Türetilen tür (SNV/INS/DEL).
    Tur,
    /// Seçili bir INFO alanı (anahtar).
    Info(String),
    /// Belirli bir örneğin GT'si (örnek indeksi).
    OrnekGt(usize),
}

impl Sutun {
    /// Sütun başlığı (örnek adları INFO/GT başlıkları için gerekir).
    pub fn baslik(&self, ornekler: &[String]) -> String {
        match self {
            Sutun::Kromozom => "CHROM".into(),
            Sutun::Konum => "POS".into(),
            Sutun::Kimlik => "ID".into(),
            Sutun::Referans => "REF".into(),
            Sutun::Alternatif => "ALT".into(),
            Sutun::Kalite => "QUAL".into(),
            Sutun::Filtre => "FILTER".into(),
            Sutun::Tur => "TÜR".into(),
            Sutun::Info(anahtar) => format!("INFO.{anahtar}"),
            Sutun::OrnekGt(i) => ornekler
                .get(*i)
                .cloned()
                .unwrap_or_else(|| format!("örnek{i}")),
        }
    }

    /// Bu sütunun bir satırdaki metin değeri.
    pub fn deger(&self, satir: &VaryantSatiri) -> String {
        match self {
            Sutun::Kromozom => satir.kromozom().to_string(),
            Sutun::Konum => satir.konum().to_string(),
            Sutun::Kimlik => satir.kayit.kimlik.clone(),
            Sutun::Referans => satir.kayit.referans.clone(),
            Sutun::Alternatif => satir.alt_metni(),
            Sutun::Kalite => satir
                .kalite()
                .map(|q| format!("{q}"))
                .unwrap_or_else(|| ".".into()),
            Sutun::Filtre => {
                if satir.kayit.filtreler.is_empty() {
                    ".".into()
                } else {
                    satir.kayit.filtreler.join(";")
                }
            }
            Sutun::Tur => satir.tur.etiket().to_string(),
            Sutun::Info(anahtar) => satir
                .kayit
                .info
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(anahtar))
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| ".".into()),
            Sutun::OrnekGt(i) => satir
                .kayit
                .genotipler
                .get(*i)
                .cloned()
                .unwrap_or_else(|| ".".into()),
        }
    }
}

/// Bir sütunun düzendeki durumu (sütun + görünür mü).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SutunDurum {
    /// Sütun.
    pub sutun: Sutun,
    /// Görünür mü?
    pub gorunur: bool,
}

/// Tablo düzeni: sütun sırası + görünürlük (kullanıcı seç/gizle/sırala).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabloDuzeni {
    /// Sütunlar (sıralı).
    pub sutunlar: Vec<SutunDurum>,
}

impl TabloDuzeni {
    /// Çekirdek VCF sütunları + her örnek için GT sütunu (örnek GT sütunları başta gizli) ile
    /// varsayılan düzen.
    pub fn varsayilan(ornek_sayisi: usize) -> Self {
        let mut sutunlar: Vec<SutunDurum> = [
            Sutun::Kromozom,
            Sutun::Konum,
            Sutun::Kimlik,
            Sutun::Referans,
            Sutun::Alternatif,
            Sutun::Kalite,
            Sutun::Filtre,
            Sutun::Tur,
        ]
        .into_iter()
        .map(|sutun| SutunDurum {
            sutun,
            gorunur: true,
        })
        .collect();
        // Örnek GT sütunları — varsayılan gizli (genotip ızgarası birincil yüzey).
        for i in 0..ornek_sayisi {
            sutunlar.push(SutunDurum {
                sutun: Sutun::OrnekGt(i),
                gorunur: false,
            });
        }
        Self { sutunlar }
    }

    /// Bir INFO sütununu düzene ekler (yoksa).
    pub fn info_sutunu_ekle(&mut self, anahtar: impl Into<String>) {
        let s = Sutun::Info(anahtar.into());
        if !self.sutunlar.iter().any(|d| d.sutun == s) {
            self.sutunlar.push(SutunDurum {
                sutun: s,
                gorunur: true,
            });
        }
    }

    /// Görünür sütunlar (sırayla).
    pub fn gorunur_sutunlar(&self) -> Vec<&Sutun> {
        self.sutunlar
            .iter()
            .filter(|d| d.gorunur)
            .map(|d| &d.sutun)
            .collect()
    }

    /// `idx` sütununun görünürlüğünü değiştirir.
    pub fn gorunurluk_degistir(&mut self, idx: usize) {
        if let Some(d) = self.sutunlar.get_mut(idx) {
            d.gorunur = !d.gorunur;
        }
    }

    /// Sütunu `kaynak`'tan `hedef` konumuna taşır (yeniden sırala).
    pub fn tasi(&mut self, kaynak: usize, hedef: usize) {
        if kaynak < self.sutunlar.len() && hedef < self.sutunlar.len() && kaynak != hedef {
            let d = self.sutunlar.remove(kaynak);
            self.sutunlar.insert(hedef, d);
        }
    }

    /// Bir satırın görünür sütun değerleri (tablo satırı).
    pub fn satir_degerleri(&self, satir: &VaryantSatiri) -> Vec<String> {
        self.gorunur_sutunlar()
            .iter()
            .map(|s| s.deger(satir))
            .collect()
    }

    /// Görünür sütun başlıkları.
    pub fn basliklar(&self, ornekler: &[String]) -> Vec<String> {
        self.gorunur_sutunlar()
            .iter()
            .map(|s| s.baslik(ornekler))
            .collect()
    }
}

/// **Sanal kaydırma** penceresi: toplam `toplam` satırdan, `ilk`'ten başlayıp en çok `adet`
/// görünür satır → `[ilk, son)` aralığı (sınırlanmış).
pub fn gorunur_pencere(toplam: usize, ilk: usize, adet: usize) -> std::ops::Range<usize> {
    let ilk = ilk.min(toplam);
    let son = ilk.saturating_add(adet).min(toplam);
    ilk..son
}

/// Gruplama ölçütü.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gruplama {
    /// Gruplama yok.
    Yok,
    /// Kromozoma göre.
    Kromozom,
    /// Türe göre.
    Tur,
}

/// Bir grup bölümü: başlık + (sıralı sonuçtaki) satır indeksleri.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrupBolum {
    /// Grup başlığı (kromozom adı / tür etiketi).
    pub baslik: String,
    /// Bu gruba ait satır indeksleri (`satirlar` dizisindeki).
    pub satir_indeksleri: Vec<usize>,
}

/// Satırları gruplara böler (komşu eşit-anahtarlı satırlar; sonuç zaten sıralı varsayılır).
pub fn gruplandir(satirlar: &[VaryantSatiri], g: Gruplama) -> Vec<GrupBolum> {
    if g == Gruplama::Yok {
        return vec![GrupBolum {
            baslik: String::new(),
            satir_indeksleri: (0..satirlar.len()).collect(),
        }];
    }
    let anahtar = |s: &VaryantSatiri| -> String {
        match g {
            Gruplama::Kromozom => s.kromozom().to_string(),
            Gruplama::Tur => s.tur.etiket().to_string(),
            Gruplama::Yok => String::new(),
        }
    };
    let mut bolumler: Vec<GrupBolum> = Vec::new();
    for (i, s) in satirlar.iter().enumerate() {
        let a = anahtar(s);
        match bolumler.last_mut() {
            Some(son) if son.baslik == a => son.satir_indeksleri.push(i),
            _ => bolumler.push(GrupBolum {
                baslik: a,
                satir_indeksleri: vec![i],
            }),
        }
    }
    bolumler
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_io::VaryantKaydi;

    fn satir(kr: &str, pos: usize, r: &str, a: &str) -> VaryantSatiri {
        VaryantSatiri::yeni(VaryantKaydi {
            kromozom: kr.into(),
            konum: pos,
            kimlik: "rs1".into(),
            referans: r.into(),
            alternatifler: vec![a.into()],
            kalite: Some(42.0),
            filtreler: vec!["PASS".into()],
            info: vec![("DP".into(), "Integer(30)".into())],
            ornek_sayisi: 1,
            format_anahtarlari: vec!["GT".into()],
            genotipler: vec!["0/1".into()],
        })
    }

    #[test]
    fn varsayilan_duzen_ve_degerler() {
        let duzen = TabloDuzeni::varsayilan(1);
        let basliklar = duzen.basliklar(&["S1".into()]);
        assert_eq!(basliklar[0], "CHROM");
        assert!(basliklar.contains(&"QUAL".to_string()));
        // GT sütunu varsayılan gizli → görünür başlıklarda yok.
        assert!(!basliklar.contains(&"S1".to_string()));

        let degerler = duzen.satir_degerleri(&satir("chr1", 100, "A", "G"));
        assert_eq!(degerler[0], "chr1");
        assert_eq!(degerler[1], "100");
    }

    #[test]
    fn sutun_gizle_goster_ve_info_ekle() {
        let mut duzen = TabloDuzeni::varsayilan(0);
        let onceki = duzen.gorunur_sutunlar().len();
        duzen.gorunurluk_degistir(0); // CHROM gizle
        assert_eq!(duzen.gorunur_sutunlar().len(), onceki - 1);

        duzen.info_sutunu_ekle("DP");
        let basliklar = duzen.basliklar(&[]);
        assert!(basliklar.contains(&"INFO.DP".to_string()));
        let degerler = duzen.satir_degerleri(&satir("chr1", 100, "A", "G"));
        assert!(degerler.iter().any(|d| d.contains("30")));
    }

    #[test]
    fn sutun_tasima() {
        let mut duzen = TabloDuzeni::varsayilan(0);
        let ilk = duzen.sutunlar[0].sutun.clone();
        duzen.tasi(0, 2);
        assert_eq!(duzen.sutunlar[2].sutun, ilk);
    }

    #[test]
    fn sanal_pencere_sinirlanir() {
        assert_eq!(gorunur_pencere(100, 90, 20), 90..100);
        assert_eq!(gorunur_pencere(5, 10, 3), 5..5); // ilk > toplam
        assert_eq!(gorunur_pencere(50, 0, 10), 0..10);
    }

    #[test]
    fn gruplama_kromozoma_gore() {
        let satirlar = vec![
            satir("chr1", 100, "A", "G"),
            satir("chr1", 200, "A", "G"),
            satir("chr2", 300, "A", "G"),
        ];
        let gruplar = gruplandir(&satirlar, Gruplama::Kromozom);
        assert_eq!(gruplar.len(), 2);
        assert_eq!(gruplar[0].baslik, "chr1");
        assert_eq!(gruplar[0].satir_indeksleri, vec![0, 1]);
        assert_eq!(gruplar[1].baslik, "chr2");

        let yok = gruplandir(&satirlar, Gruplama::Yok);
        assert_eq!(yok.len(), 1);
        assert_eq!(yok[0].satir_indeksleri.len(), 3);
    }
}
