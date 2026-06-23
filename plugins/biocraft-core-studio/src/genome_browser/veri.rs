//! ÇE-02 — **İz verisi köprüsü**: `data_io` okuyucu kayıtlarını tarayıcı çizim-parçalarına
//! çevirir ve **yalnız görünen pencereyi** (out-of-core, MK-09) yükler.
//!
//! Tarayıcı çekirdeği (cetvel/yerleşim/LOD/çizim) bu sade parça tipleri üzerinde çalışır; gerçek
//! BAM/GFF okuma `data_io` (noodles) ile yapılır.  Böylece çizim mantığı dosyadan bağımsız
//! birim-testlenir, okuma katmanı ayrı kalır.

use biocraft_sdk::biocraft_types::ErrorReport;

use super::canvas::GenomBolge;
use super::lod::Konumlu;
use crate::data_io::{
    AnotasyonKaydi, AnotasyonOkuyucu, BellekButcesi, HizalamaKaydi, HizalamaOkuyucu,
};

/// DNA şeridi (read/özellik yönü).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Serit {
    /// İleri (+).
    Ileri,
    /// Geri (-).
    Geri,
    /// Belirsiz/yok (.).
    Yok,
}

impl Serit {
    /// SAM/BAM bayrağından (`0x10` = ters şerit) şerit.
    pub fn bayraktan(bayrak: u16) -> Serit {
        if bayrak & 0x10 != 0 {
            Serit::Geri
        } else {
            Serit::Ileri
        }
    }

    /// GFF şerit karakterinden.
    pub fn karakterden(c: char) -> Serit {
        match c {
            '+' => Serit::Ileri,
            '-' => Serit::Geri,
            _ => Serit::Yok,
        }
    }

    /// İşaret karakteri.
    pub fn isaret(self) -> char {
        match self {
            Serit::Ileri => '+',
            Serit::Geri => '-',
            Serit::Yok => '.',
        }
    }
}

/// Hizalama izinde tek bir okuma (read) — çizim için sadeleştirilmiş ayak izi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OkumaParcasi {
    /// Read adı (QNAME).
    pub ad: String,
    /// 1-tabanlı hizalama başlangıcı.
    pub bas: u64,
    /// 1-tabanlı hizalama bitişi (kapsayıcı; dizi uzunluğundan tahmini — kesin CIGAR span ÇE-03).
    pub bit: u64,
    /// Şerit.
    pub serit: Serit,
    /// Eşleme kalitesi (MAPQ).
    pub mapq: Option<u8>,
}

impl Konumlu for OkumaParcasi {
    fn bas(&self) -> u64 {
        self.bas
    }
    fn bit(&self) -> u64 {
        self.bit
    }
}

impl OkumaParcasi {
    /// Düşük eşleme kalitesi eşiği (altı görsel olarak soluk gösterilir).
    pub const DUSUK_MAPQ: u8 = 10;

    /// Bir `HizalamaKaydi`'ndan parça üretir; eşlenmemiş (konum yok) okuma `None`.
    pub fn kayittan(k: &HizalamaKaydi) -> Option<OkumaParcasi> {
        let bas = k.konum? as u64;
        // Ayak izi ≈ dizi uzunluğu (indel'siz yaklaşım; kesin span ÇE-03'te CIGAR ile).
        let uzun = (k.dizi_uzunlugu.max(1)) as u64;
        Some(OkumaParcasi {
            ad: k.ad.clone(),
            bas,
            bit: bas + uzun - 1,
            serit: Serit::bayraktan(k.bayrak),
            mapq: k.mapq,
        })
    }

    /// Düşük kaliteli mi (MAPQ < eşik)?
    pub fn dusuk_kalite(&self) -> bool {
        self.mapq.is_some_and(|q| q < Self::DUSUK_MAPQ)
    }

    /// Üzerine gelince gösterilecek kısa ipucu (tooltip; Gün 4 şeması — sade).
    pub fn ipucu(&self) -> String {
        let q = self
            .mapq
            .map(|q| q.to_string())
            .unwrap_or_else(|| "—".into());
        format!(
            "{} ({}) {}-{} • MAPQ {}",
            self.ad,
            self.serit.isaret(),
            self.bas,
            self.bit,
            q
        )
    }

    /// Seçildiğinde içerik (inspector) panelinde gösterilecek çok-satırlı detay.
    pub fn detay(&self) -> String {
        format!(
            "Okuma: {}\nKonum: {}-{} ({} bp)\nŞerit: {}\nMAPQ: {}",
            self.ad,
            self.bas,
            self.bit,
            self.bit - self.bas + 1,
            self.serit.isaret(),
            self.mapq
                .map(|q| q.to_string())
                .unwrap_or_else(|| "—".into()),
        )
    }
}

/// Anotasyon izinde tek bir özellik (gen/transkript/ekson) — çizim için sade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OzellikParcasi {
    /// Ad/kimlik (varsa).
    pub ad: Option<String>,
    /// 1-tabanlı başlangıç.
    pub bas: u64,
    /// 1-tabanlı bitiş (kapsayıcı).
    pub bit: u64,
    /// Şerit.
    pub serit: Serit,
    /// Özellik türü (gene/exon/transcript/region…).
    pub tur: String,
}

impl Konumlu for OzellikParcasi {
    fn bas(&self) -> u64 {
        self.bas
    }
    fn bit(&self) -> u64 {
        self.bit
    }
}

impl OzellikParcasi {
    /// Bir `AnotasyonKaydi`'ndan parça üretir.
    pub fn kayittan(k: &AnotasyonKaydi) -> OzellikParcasi {
        OzellikParcasi {
            ad: k.ad.clone(),
            bas: k.baslangic as u64,
            bit: k.bitis as u64,
            serit: Serit::karakterden(k.serit),
            tur: k.tur.clone(),
        }
    }

    /// Bu özellik bir ekson mu (farklı çizilir)?
    pub fn ekson_mu(&self) -> bool {
        self.tur.eq_ignore_ascii_case("exon") || self.tur.eq_ignore_ascii_case("CDS")
    }

    /// Görünen ad (yoksa tür).
    pub fn gorunen_ad(&self) -> &str {
        self.ad.as_deref().unwrap_or(&self.tur)
    }

    /// Üzerine gelince ipucu.
    pub fn ipucu(&self) -> String {
        format!(
            "{} [{}] ({}) {}-{}",
            self.gorunen_ad(),
            self.tur,
            self.serit.isaret(),
            self.bas,
            self.bit
        )
    }

    /// Seçim detayı.
    pub fn detay(&self) -> String {
        format!(
            "Özellik: {}\nTür: {}\nKonum: {}-{} ({} bp)\nŞerit: {}",
            self.gorunen_ad(),
            self.tur,
            self.bas,
            self.bit,
            self.bit - self.bas + 1,
            self.serit.isaret()
        )
    }
}

// ─── Out-of-core yükleyiciler (yalnız görünen pencere — MK-09) ──────────────────

/// Görünen bölgedeki okumaları yükler.  `data_io` bölge sorgusu **indeksliyse** (BAM/CRAM) yalnız
/// o bölgenin blokları okunur (MK-09); SAM lineer taranır.  Eşlenmemiş okumalar atlanır.
pub fn gorunur_okumalar(
    okuyucu: &mut HizalamaOkuyucu,
    bolge: &GenomBolge,
    butce: &BellekButcesi,
    max_kayit: usize,
) -> Result<Vec<OkumaParcasi>, ErrorReport> {
    let kayitlar = okuyucu.bolge_sorgu(&bolge.etiket(), butce, max_kayit)?;
    Ok(kayitlar.iter().filter_map(OkumaParcasi::kayittan).collect())
}

/// Görünen bölgedeki anotasyon özelliklerini yükler (linear/out-of-core bölge süzme).
pub fn gorunur_ozellikler(
    okuyucu: &AnotasyonOkuyucu,
    bolge: &GenomBolge,
    butce: &BellekButcesi,
    max_kayit: usize,
) -> Result<Vec<OzellikParcasi>, ErrorReport> {
    let kayitlar = okuyucu.bolge_sorgu(&bolge.etiket(), butce, max_kayit)?;
    Ok(kayitlar.iter().map(OzellikParcasi::kayittan).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn okuma_kayittan_ve_ipucu() {
        let k = HizalamaKaydi {
            ad: "read1".into(),
            bayrak: 0x10, // ters şerit
            referans: Some("chr1".into()),
            konum: Some(100),
            mapq: Some(60),
            dizi_uzunlugu: 50,
        };
        let p = OkumaParcasi::kayittan(&k).unwrap();
        assert_eq!((p.bas, p.bit), (100, 149)); // 100 + 50 - 1
        assert_eq!(p.serit, Serit::Geri);
        assert!(!p.dusuk_kalite());
        assert!(p.ipucu().contains("read1"));
        assert!(p.detay().contains("50 bp"));

        // Eşlenmemiş (konum yok) → None.
        let unmapped = HizalamaKaydi {
            ad: "u".into(),
            bayrak: 0x4,
            referans: None,
            konum: None,
            mapq: None,
            dizi_uzunlugu: 50,
        };
        assert!(OkumaParcasi::kayittan(&unmapped).is_none());
    }

    #[test]
    fn ozellik_kayittan_ve_ekson() {
        let k = AnotasyonKaydi {
            kromozom: "chr1".into(),
            baslangic: 1000,
            bitis: 1200,
            tur: "exon".into(),
            ad: Some("exon1".into()),
            serit: '+',
            kaynak: Some("HAVANA".into()),
        };
        let p = OzellikParcasi::kayittan(&k);
        assert_eq!((p.bas, p.bit), (1000, 1200));
        assert!(p.ekson_mu());
        assert_eq!(p.gorunen_ad(), "exon1");
        assert_eq!(p.serit, Serit::Ileri);
        assert!(p.ipucu().contains("exon1"));
    }

    #[test]
    fn dusuk_kalite_esigi() {
        let mut k = HizalamaKaydi {
            ad: "r".into(),
            bayrak: 0,
            referans: Some("c".into()),
            konum: Some(1),
            mapq: Some(5),
            dizi_uzunlugu: 10,
        };
        assert!(OkumaParcasi::kayittan(&k).unwrap().dusuk_kalite());
        k.mapq = Some(30);
        assert!(!OkumaParcasi::kayittan(&k).unwrap().dusuk_kalite());
    }
}
