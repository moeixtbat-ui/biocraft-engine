//! ÇE-12 — **Doğruluk (golden) güvencesi**: referans araç parametre eşitleme + sayısal tolerans.
//!
//! MK-58: çekirdek eklentinin çıktısı bilinen-doğru araçlarla (IGV / samtools / bcftools / bedtools
//! / UCSC) **golden** karşılaştırılır.  Pratikte en sık golden sapması **parametre uyuşmazlığından**
//! gelir (bkz. görev "Muhtemel Hatalar": *koordinat tabanı / filtre / eşik farkı*).  Bu modül o
//! farkları **kaynağında** önler:
//! * [`KoordinatTabani`] — araç hangi tabanı kullanır (0-tabanlı yarı-açık mı, 1-tabanlı kapalı mı)?
//!   Dönüştürücülerle BioCraft'ın koordinatı referansla **aynı tabana** getirilir.
//! * [`yaklasik_esit`] — kayan-nokta golden (örn. Ts/Tv oranı) tam eşitlik yerine **tolerans**la
//!   kıyaslanır (kırılgan golden önlenir).
//! * [`DogrulukRaporu`] — bir karşılaştırmayı (araç + parametre + sonuç) **açıkça** belgeler →
//!   golden çıktısı yeniden-üretilebilir kalır.

use std::fmt::Write as _;

/// Golden karşılaştırmasında kullanılan referans araç.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferansArac {
    /// Integrative Genomics Viewer (görüntüleme; 1-tabanlı gösterim).
    Igv,
    /// samtools (BAM/SAM/CRAM; depth/view).
    Samtools,
    /// bcftools (VCF/BCF; query/filter/stats).
    Bcftools,
    /// bedtools (BED aralık işlemleri; 0-tabanlı yarı-açık).
    Bedtools,
    /// UCSC araçları (bigWig/2bit; 0-tabanlı çoğu yerde).
    UcscAraclari,
}

impl ReferansArac {
    /// İnsan-okunur ad (rapor/atıf).
    pub fn ad(&self) -> &'static str {
        match self {
            ReferansArac::Igv => "IGV",
            ReferansArac::Samtools => "samtools",
            ReferansArac::Bcftools => "bcftools",
            ReferansArac::Bedtools => "bedtools",
            ReferansArac::UcscAraclari => "UCSC araçları",
        }
    }

    /// Bu aracın **dosya/sorgu koordinat tabanı** (golden'ı eşitlemek için).
    ///
    /// - VCF/SAM/GFF/IGV gösterimi: **1-tabanlı kapalı**.
    /// - BED/BAM iç/UCSC ikili/bedtools: **0-tabanlı yarı-açık**.
    pub fn koordinat_tabani(&self) -> KoordinatTabani {
        match self {
            ReferansArac::Igv | ReferansArac::Samtools | ReferansArac::Bcftools => {
                KoordinatTabani::BirTabanliKapali
            }
            ReferansArac::Bedtools | ReferansArac::UcscAraclari => {
                KoordinatTabani::SifirTabanliYariAcik
            }
        }
    }
}

/// Genomik koordinat tabanı sözleşmesi.  Golden sapmasının en sık nedeni bunun karışmasıdır.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KoordinatTabani {
    /// 1-tabanlı, kapalı `[start, end]` (VCF/GFF/SAM/IGV).  İlk baz = 1.
    BirTabanliKapali,
    /// 0-tabanlı, yarı-açık `[start, end)` (BED/BAM-iç/UCSC).  İlk baz = 0.
    SifirTabanliYariAcik,
}

impl KoordinatTabani {
    /// BioCraft iç gösterimi **1-tabanlı kapalı**'dır (genom tarayıcı `GenomBolge`).  Bir aralığı
    /// bu tabandan hedef tabana çevirir → `(start, end)` (her ikisi de hedef sözleşmesinde).
    ///
    /// 1-tabanlı kapalı `[s, e]` ↔ 0-tabanlı yarı-açık `[s-1, e)`: başlangıç 1 azalır, bitiş aynı.
    pub fn bir_tabanli_kapaliden(&self, baslangic: u64, bitis: u64) -> (u64, u64) {
        match self {
            KoordinatTabani::BirTabanliKapali => (baslangic, bitis),
            KoordinatTabani::SifirTabanliYariAcik => (baslangic.saturating_sub(1), bitis),
        }
    }

    /// Hedef tabandaki bir aralığı BioCraft iç tabanına (1-tabanlı kapalı) çevirir.
    pub fn bir_tabanli_kapaliya(&self, baslangic: u64, bitis: u64) -> (u64, u64) {
        match self {
            KoordinatTabani::BirTabanliKapali => (baslangic, bitis),
            KoordinatTabani::SifirTabanliYariAcik => (baslangic + 1, bitis),
        }
    }

    /// Bu tabandaki bir aralığın uzunluğu (baz sayısı).
    pub fn uzunluk(&self, baslangic: u64, bitis: u64) -> u64 {
        match self {
            // 1-tabanlı kapalı: e - s + 1 (10..10 = 1 baz).
            KoordinatTabani::BirTabanliKapali => bitis.saturating_sub(baslangic) + 1,
            // 0-tabanlı yarı-açık: e - s (0..10 = 10 baz).
            KoordinatTabani::SifirTabanliYariAcik => bitis.saturating_sub(baslangic),
        }
    }
}

/// İki kayan-nokta değeri tolerans içinde eşit mi? (kırılgan golden önleme; mutlak **veya** göreli.)
pub fn yaklasik_esit(a: f64, b: f64, tolerans: f64) -> bool {
    let fark = (a - b).abs();
    if fark <= tolerans {
        return true;
    }
    let olcek = a.abs().max(b.abs());
    fark <= tolerans * olcek
}

/// Bir golden karşılaştırmasının **belgelenmiş** sonucu (araç + parametre + eşleşme).  Determinist
/// metne ([`DogrulukRaporu::metin`]) çevrilir → golden referans dosyası olarak saklanır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DogrulukRaporu {
    /// Karşılaştırılan referans araç.
    pub arac: ReferansArac,
    /// Neyin karşılaştırıldığı (örn. "QUAL>=50 && PASS varyant sayısı").
    pub olcut: String,
    /// Eşitlenen parametreler (araç parametresiyle bire bir — sapmayı önleyen kayıt).
    pub parametreler: Vec<(String, String)>,
    /// BioCraft sonucu (metin).
    pub biocraft: String,
    /// Referans (beklenen) sonucu (metin).
    pub referans: String,
}

impl DogrulukRaporu {
    /// Yeni rapor.
    pub fn yeni(arac: ReferansArac, olcut: impl Into<String>) -> Self {
        Self {
            arac,
            olcut: olcut.into(),
            parametreler: Vec::new(),
            biocraft: String::new(),
            referans: String::new(),
        }
    }

    /// Eşitlenen bir parametre ekler (akıcı API).
    pub fn parametre(mut self, ad: impl Into<String>, deger: impl Into<String>) -> Self {
        self.parametreler.push((ad.into(), deger.into()));
        self
    }

    /// Sonuçları kaydeder.
    pub fn sonuc(mut self, biocraft: impl Into<String>, referans: impl Into<String>) -> Self {
        self.biocraft = biocraft.into();
        self.referans = referans.into();
        self
    }

    /// BioCraft ve referans sonucu **eşleşti** mi? (doğruluk geçti mi)
    pub fn eslesti(&self) -> bool {
        self.biocraft == self.referans
    }

    /// Deterministik, golden'lanabilir metin (parametreler ada göre sıralı → kararlı).
    pub fn metin(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "araç: {}", self.arac.ad());
        let _ = writeln!(s, "ölçüt: {}", self.olcut);
        let mut params = self.parametreler.clone();
        params.sort();
        for (ad, deger) in &params {
            let _ = writeln!(s, "param {ad} = {deger}");
        }
        let _ = writeln!(s, "biocraft: {}", self.biocraft);
        let _ = writeln!(s, "referans: {}", self.referans);
        let _ = writeln!(
            s,
            "eşleşti: {}",
            if self.eslesti() { "evet" } else { "HAYIR" }
        );
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arac_koordinat_tabanlari_dogru() {
        // VCF/SAM araçları 1-tabanlı; BED/UCSC 0-tabanlı (sık golden sapma kaynağı).
        assert_eq!(
            ReferansArac::Bcftools.koordinat_tabani(),
            KoordinatTabani::BirTabanliKapali
        );
        assert_eq!(
            ReferansArac::Samtools.koordinat_tabani(),
            KoordinatTabani::BirTabanliKapali
        );
        assert_eq!(
            ReferansArac::Bedtools.koordinat_tabani(),
            KoordinatTabani::SifirTabanliYariAcik
        );
    }

    #[test]
    fn koordinat_donusumu_gidip_gelme() {
        // BioCraft (1-tabanlı kapalı) chr:100-200 → BED (0-tabanlı): 99-200.
        let bed = KoordinatTabani::SifirTabanliYariAcik;
        assert_eq!(bed.bir_tabanli_kapaliden(100, 200), (99, 200));
        // Geri: BED 99-200 → 1-tabanlı 100-200 (gidip gelme korunur).
        assert_eq!(bed.bir_tabanli_kapaliya(99, 200), (100, 200));
        // 1-tabanlı araçta dönüşüm yok.
        let vcf = KoordinatTabani::BirTabanliKapali;
        assert_eq!(vcf.bir_tabanli_kapaliden(100, 200), (100, 200));
    }

    #[test]
    fn koordinat_uzunlugu_taban_farki() {
        // Aynı "10..10" iki tabanda farklı uzunluk → golden sapmasının klasik kaynağı.
        // 1-tabanlı kapalı [10,10] = 1 baz; 0-tabanlı yarı-açık [10,10) = 0 baz.
        assert_eq!(KoordinatTabani::BirTabanliKapali.uzunluk(10, 10), 1);
        assert_eq!(KoordinatTabani::SifirTabanliYariAcik.uzunluk(10, 10), 0);
        // chr1:1-1000 (1-tabanlı) = 1000 baz; BED 0-1000 = 1000 baz (eşitlendiğinde aynı).
        assert_eq!(KoordinatTabani::BirTabanliKapali.uzunluk(1, 1000), 1000);
        assert_eq!(KoordinatTabani::SifirTabanliYariAcik.uzunluk(0, 1000), 1000);
    }

    #[test]
    fn yaklasik_esit_tolerans() {
        // Ts/Tv gibi oranlar tam eşit olmayabilir → tolerans.
        assert!(yaklasik_esit(2.10, 2.1000001, 1e-6));
        assert!(!yaklasik_esit(2.1, 2.2, 1e-6));
        // Göreli tolerans büyük değerlerde.
        assert!(yaklasik_esit(1_000_000.0, 1_000_000.5, 1e-6));
        assert!(yaklasik_esit(0.0, 0.0, 0.0));
    }

    #[test]
    fn dogruluk_raporu_eslesme_ve_metin() {
        let r = DogrulukRaporu::yeni(ReferansArac::Bcftools, "QUAL>=50 && PASS sayısı")
            .parametre("min_qual", "50")
            .parametre("filter", "PASS")
            .parametre("koordinat", "1-tabanlı")
            .sonuc("3", "3");
        assert!(r.eslesti());
        let m = r.metin();
        assert!(m.contains("araç: bcftools"));
        assert!(m.contains("param filter = PASS"));
        assert!(m.contains("eşleşti: evet"));
        // Parametreler sıralı (kararlı golden): filter < koordinat < min_qual.
        let i_filter = m.find("param filter").unwrap();
        let i_koord = m.find("param koordinat").unwrap();
        let i_qual = m.find("param min_qual").unwrap();
        assert!(i_filter < i_koord && i_koord < i_qual);
    }

    #[test]
    fn dogruluk_raporu_sapma_yakalar() {
        let r = DogrulukRaporu::yeni(ReferansArac::Samtools, "kapsama @ chr1:100")
            .parametre("koordinat", "1-tabanlı")
            .sonuc("42", "41"); // off-by-one sapması
        assert!(!r.eslesti());
        assert!(r.metin().contains("eşleşti: HAYIR"));
    }
}
