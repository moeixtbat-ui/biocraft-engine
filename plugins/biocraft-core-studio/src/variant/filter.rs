//! ÇE-04 — **Filtreleme**: yapılandırılmış filtre + kullanıcı-dostu kurucu **ve** ham sorgu ifadesi
//! (gelişmiş) + **kaydedilebilir filtre setleri**.
//!
//! Filtre hem programatik (UI kurucu) kurulur hem de metinsel bir ifadeden ([`ayristir`])
//! ayrıştırılır; [`Filtre::ifade`] ters yönde metne çevirir → **kayıtlı setler** filtreyi ifade
//! metni olarak saklar (serde-basit; karmaşık tiplere serde bağımlılığı gerekmez).
//!
//! ## Desteklenen ham sorgu dili (MVP)
//! `ALAN OP DEĞER` ifadeleri `AND` ile bağlanır (büyük/küçük harf duyarsız).  Örnekler:
//! ```text
//! QUAL >= 30 AND FILTER = PASS
//! CHROM = chr1 AND POS >= 1000 AND POS <= 2000
//! TYPE = SNV AND ID ~ rs
//! INFO.DP > 10
//! ```
//! Alanlar: `QUAL`, `FILTER`(=PASS), `TYPE`(SNV/INS/DEL/VAR), `ID`(~/=), `CHROM`, `POS`,
//! `INFO.<anahtar>`.  Operatörler: `> >= < <= = != ~`(içerir).  `OR`/parantez **v1.x** (gelişmiş).

use serde::{Deserialize, Serialize};

use biocraft_sdk::biocraft_types::ErrorReport;

use crate::genome_browser::canvas::GenomBolge;
use crate::genome_browser::veri::VaryantTuru;

use super::query::VaryantSatiri;

/// Bölge ifadesinde üst sınır belirtilmediğinde kullanılan "sınırsız" konum (≈1 Gb; kromozom
/// uzunluklarının üstünde).
pub const BUYUK_KONUM: u64 = 1_000_000_000;

/// Karşılaştırma operatörü.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Karsilastirma {
    /// `>`
    Buyuk,
    /// `>=`
    BuyukEsit,
    /// `<`
    Kucuk,
    /// `<=`
    KucukEsit,
    /// `=`
    Esit,
    /// `!=`
    EsitDegil,
    /// `~` (içerir / substring)
    Icerir,
}

impl Karsilastirma {
    /// İfade metnindeki sembol.
    pub fn sembol(self) -> &'static str {
        match self {
            Karsilastirma::Buyuk => ">",
            Karsilastirma::BuyukEsit => ">=",
            Karsilastirma::Kucuk => "<",
            Karsilastirma::KucukEsit => "<=",
            Karsilastirma::Esit => "=",
            Karsilastirma::EsitDegil => "!=",
            Karsilastirma::Icerir => "~",
        }
    }

    fn coz(t: &str) -> Option<Karsilastirma> {
        Some(match t {
            ">" => Karsilastirma::Buyuk,
            ">=" => Karsilastirma::BuyukEsit,
            "<" => Karsilastirma::Kucuk,
            "<=" => Karsilastirma::KucukEsit,
            "=" => Karsilastirma::Esit,
            "!=" => Karsilastirma::EsitDegil,
            "~" => Karsilastirma::Icerir,
            _ => return None,
        })
    }

    /// İki sayıyı bu operatöre göre karşılaştırır.
    fn sayi_uygula(self, sol: f64, sag: f64) -> bool {
        match self {
            Karsilastirma::Buyuk => sol > sag,
            Karsilastirma::BuyukEsit => sol >= sag,
            Karsilastirma::Kucuk => sol < sag,
            Karsilastirma::KucukEsit => sol <= sag,
            Karsilastirma::Esit => sol == sag,
            Karsilastirma::EsitDegil => sol != sag,
            Karsilastirma::Icerir => false, // sayı için anlamsız
        }
    }
}

/// INFO alanı koşulu (`INFO.<anahtar> OP DEĞER`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoKosul {
    /// INFO anahtarı (DP, AF, …).
    pub anahtar: String,
    /// Operatör.
    pub op: Karsilastirma,
    /// Karşılaştırılacak değer (metin; sayısal op'larda sayıya çözülür).
    pub deger: String,
}

/// Yapılandırılmış varyant filtresi.  Boş ([`Filtre::default`]) = "hepsini geçir".
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Filtre {
    /// `QUAL >= bu` (varsa).
    pub kalite_min: Option<f32>,
    /// `QUAL <= bu` (varsa).
    pub kalite_max: Option<f32>,
    /// Yalnız `FILTER=PASS`.
    pub sadece_pass: bool,
    /// Yalnız bu türler (None = tümü).
    pub turler: Option<Vec<VaryantTuru>>,
    /// ID/rsID bu metni içersin (büyük/küçük harf duyarsız).
    pub kimlik_arama: Option<String>,
    /// Bölge (kromozom + konum aralığı) — out-of-core **pushdown** + yüklem.
    pub bolge: Option<GenomBolge>,
    /// INFO alanı koşulları (hepsi sağlanmalı — AND).
    pub info_kosullari: Vec<InfoKosul>,
}

impl Filtre {
    /// Bir satır bu filtreyi geçiyor mu? (tüm koşullar AND ile.)
    pub fn gecer(&self, satir: &VaryantSatiri) -> bool {
        if let Some(min) = self.kalite_min {
            match satir.kalite() {
                Some(q) if q >= min => {}
                _ => return false,
            }
        }
        if let Some(maks) = self.kalite_max {
            match satir.kalite() {
                Some(q) if q <= maks => {}
                _ => return false,
            }
        }
        if self.sadece_pass && !satir.pass_mi() {
            return false;
        }
        if let Some(turler) = &self.turler {
            if !turler.contains(&satir.tur) {
                return false;
            }
        }
        if let Some(arama) = &self.kimlik_arama {
            if !satir
                .kayit
                .kimlik
                .to_ascii_lowercase()
                .contains(&arama.to_ascii_lowercase())
            {
                return false;
            }
        }
        if let Some(b) = &self.bolge {
            let bas = satir.konum() as u64;
            let bit = bas + (satir.kayit.referans.len().max(1) as u64) - 1;
            if satir.kromozom() != b.kromozom || !b.ortusur(bas, bit) {
                return false;
            }
        }
        for kosul in &self.info_kosullari {
            if !info_kosul_gecer(satir, kosul) {
                return false;
            }
        }
        true
    }

    /// Filtreyi ham sorgu ifadesine çevirir ([`ayristir`]'ın tersi; round-trip eder).
    pub fn ifade(&self) -> String {
        let mut parcalar: Vec<String> = Vec::new();
        if let Some(b) = &self.bolge {
            parcalar.push(format!("CHROM = {}", b.kromozom));
            if b.baslangic > 1 {
                parcalar.push(format!("POS >= {}", b.baslangic));
            }
            if b.bitis < BUYUK_KONUM {
                parcalar.push(format!("POS <= {}", b.bitis));
            }
        }
        if let Some(v) = self.kalite_min {
            parcalar.push(format!("QUAL >= {v}"));
        }
        if let Some(v) = self.kalite_max {
            parcalar.push(format!("QUAL <= {v}"));
        }
        if self.sadece_pass {
            parcalar.push("FILTER = PASS".into());
        }
        if let Some(turler) = &self.turler {
            for t in turler {
                parcalar.push(format!("TYPE = {}", t.etiket()));
            }
        }
        if let Some(a) = &self.kimlik_arama {
            parcalar.push(format!("ID ~ {a}"));
        }
        for k in &self.info_kosullari {
            parcalar.push(format!("INFO.{} {} {}", k.anahtar, k.op.sembol(), k.deger));
        }
        parcalar.join(" AND ")
    }

    /// Filtrenin herhangi bir koşulu var mı? (boş = "hepsini geçir".)
    pub fn bos_mu(&self) -> bool {
        *self == Filtre::default()
    }
}

/// Bir INFO koşulunu satıra karşı değerlendirir.
fn info_kosul_gecer(satir: &VaryantSatiri, kosul: &InfoKosul) -> bool {
    let Some((_, ham)) = satir
        .kayit
        .info
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(&kosul.anahtar))
    else {
        // Alan yoksa: yalnız "eşit değil" geçer (yokluk ≠ değer); diğerleri başarısız.
        return kosul.op == Karsilastirma::EsitDegil;
    };
    let deger = deger_sadelestir(ham);
    match kosul.op {
        Karsilastirma::Icerir => deger
            .to_ascii_lowercase()
            .contains(&kosul.deger.to_ascii_lowercase()),
        Karsilastirma::Esit => {
            if let (Some(a), Some(b)) = (sayi_cek(&deger), sayi_cek(&kosul.deger)) {
                a == b
            } else {
                deger.eq_ignore_ascii_case(&kosul.deger)
            }
        }
        Karsilastirma::EsitDegil => {
            if let (Some(a), Some(b)) = (sayi_cek(&deger), sayi_cek(&kosul.deger)) {
                a != b
            } else {
                !deger.eq_ignore_ascii_case(&kosul.deger)
            }
        }
        _ => match (sayi_cek(&deger), sayi_cek(&kosul.deger)) {
            (Some(a), Some(b)) => kosul.op.sayi_uygula(a, b),
            _ => false,
        },
    }
}

/// noodles INFO değerinin Debug sarmalını sadeleştirir (`Integer(30)`→`30`, `String("x")`→`x`).
pub(crate) fn deger_sadelestir(ham: &str) -> String {
    let s = ham.trim();
    if let (Some(ac), Some(_)) = (s.find('('), s.rfind(')')) {
        let ic = &s[ac + 1..s.rfind(')').unwrap()];
        return ic.trim().trim_matches('"').to_string();
    }
    s.trim_matches('"').to_string()
}

/// Bir metinden ilk sayısal değeri çeker (`"DP=30"`→30, `"0.5"`→0.5).
fn sayi_cek(s: &str) -> Option<f64> {
    if let Ok(v) = s.trim().parse::<f64>() {
        return Some(v);
    }
    s.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e'))
        .find(|t| !t.is_empty() && t.chars().any(|c| c.is_ascii_digit()))
        .and_then(|t| t.parse::<f64>().ok())
}

// ─── Ham sorgu ayrıştırıcı ───────────────────────────────────────────────────

/// Bir ham sorgu ifadesini [`Filtre`]'ye ayrıştırır.  Boş ifade → boş filtre (hepsini geçir).
pub fn ayristir(ifade: &str) -> Result<Filtre, ErrorReport> {
    let tokenler = tokenle(ifade);
    let mut cumleler: Vec<Vec<String>> = vec![Vec::new()];
    for t in tokenler {
        if t.eq_ignore_ascii_case("and") {
            cumleler.push(Vec::new());
        } else {
            cumleler.last_mut().unwrap().push(t);
        }
    }

    let mut filtre = Filtre::default();
    let mut kromozom: Option<String> = None;
    let mut konum_bas: Option<u64> = None;
    let mut konum_bit: Option<u64> = None;

    for cumle in &cumleler {
        if cumle.is_empty() {
            continue; // baştaki/sondaki/çift AND'i hoş gör
        }
        if cumle.len() != 3 {
            return Err(sorgu_hatasi(&format!(
                "'{}' → beklenen biçim: ALAN OP DEĞER (ör. QUAL >= 30)",
                cumle.join(" ")
            )));
        }
        let alan = cumle[0].to_ascii_uppercase();
        let op = Karsilastirma::coz(&cumle[1])
            .ok_or_else(|| sorgu_hatasi(&format!("bilinmeyen operatör '{}'", cumle[1])))?;
        let deger = &cumle[2];

        match alan.as_str() {
            "QUAL" => {
                let v: f32 = deger
                    .parse()
                    .map_err(|_| sorgu_hatasi(&format!("QUAL sayısal olmalı: '{deger}'")))?;
                match op {
                    Karsilastirma::Buyuk | Karsilastirma::BuyukEsit => filtre.kalite_min = Some(v),
                    Karsilastirma::Kucuk | Karsilastirma::KucukEsit => filtre.kalite_max = Some(v),
                    Karsilastirma::Esit => {
                        filtre.kalite_min = Some(v);
                        filtre.kalite_max = Some(v);
                    }
                    _ => return Err(sorgu_hatasi("QUAL için > >= < <= = kullanın")),
                }
            }
            "FILTER" => {
                if op != Karsilastirma::Esit || !deger.eq_ignore_ascii_case("PASS") {
                    return Err(sorgu_hatasi("yalnız 'FILTER = PASS' desteklenir"));
                }
                filtre.sadece_pass = true;
            }
            "TYPE" | "TUR" | "TÜR" => {
                let t = tur_coz(deger).ok_or_else(|| {
                    sorgu_hatasi(&format!("bilinmeyen tür '{deger}' (SNV/INS/DEL/VAR)"))
                })?;
                filtre.turler.get_or_insert_with(Vec::new).push(t);
            }
            "ID" => {
                if !matches!(op, Karsilastirma::Esit | Karsilastirma::Icerir) {
                    return Err(sorgu_hatasi("ID için = veya ~ kullanın"));
                }
                filtre.kimlik_arama = Some(deger.clone());
            }
            "CHROM" | "CHR" => {
                if op != Karsilastirma::Esit {
                    return Err(sorgu_hatasi("CHROM için = kullanın"));
                }
                kromozom = Some(deger.clone());
            }
            "POS" => {
                let v: u64 = deger
                    .parse()
                    .map_err(|_| sorgu_hatasi(&format!("POS tam sayı olmalı: '{deger}'")))?;
                match op {
                    Karsilastirma::Buyuk | Karsilastirma::BuyukEsit => konum_bas = Some(v),
                    Karsilastirma::Kucuk | Karsilastirma::KucukEsit => konum_bit = Some(v),
                    Karsilastirma::Esit => {
                        konum_bas = Some(v);
                        konum_bit = Some(v);
                    }
                    _ => return Err(sorgu_hatasi("POS için > >= < <= = kullanın")),
                }
            }
            _ if alan.starts_with("INFO.") => {
                let anahtar = alan["INFO.".len()..].to_string();
                if anahtar.is_empty() {
                    return Err(sorgu_hatasi("INFO. sonrası anahtar gerekir (ör. INFO.DP)"));
                }
                filtre.info_kosullari.push(InfoKosul {
                    anahtar,
                    op,
                    deger: deger.clone(),
                });
            }
            _ => return Err(sorgu_hatasi(&format!("bilinmeyen alan '{}'", cumle[0]))),
        }
    }

    // Bölge: POS sınırı verildiyse CHROM gerekir.
    if (konum_bas.is_some() || konum_bit.is_some()) && kromozom.is_none() {
        return Err(sorgu_hatasi(
            "POS filtresi için CHROM da belirtin (ör. CHROM = chr1)",
        ));
    }
    if let Some(kr) = kromozom {
        let bas = konum_bas.unwrap_or(1).max(1);
        let bit = konum_bit.unwrap_or(BUYUK_KONUM);
        filtre.bolge = Some(GenomBolge::yeni(kr, bas, bit)?);
    }

    Ok(filtre)
}

/// Tür metnini çözer (SNV/SNP, INS, DEL, VAR/OTHER).
fn tur_coz(s: &str) -> Option<VaryantTuru> {
    match s.to_ascii_uppercase().as_str() {
        "SNV" | "SNP" => Some(VaryantTuru::Snv),
        "INS" | "INSERTION" => Some(VaryantTuru::Insersiyon),
        "DEL" | "DELETION" => Some(VaryantTuru::Delesyon),
        "VAR" | "OTHER" | "DIGER" | "DİĞER" => Some(VaryantTuru::Diger),
        _ => None,
    }
}

/// İfadeyi token'lara böler; operatörleri (`>= <= != > < = ~`) boşluksuz da ayırır.
fn tokenle(ifade: &str) -> Vec<String> {
    let karakterler: Vec<char> = ifade.chars().collect();
    let mut tokenler = Vec::new();
    let mut cur = String::new();
    let mut i = 0;
    while i < karakterler.len() {
        let c = karakterler[i];
        if c.is_whitespace() {
            if !cur.is_empty() {
                tokenler.push(std::mem::take(&mut cur));
            }
            i += 1;
        } else if matches!(c, '>' | '<' | '=' | '!' | '~') {
            if !cur.is_empty() {
                tokenler.push(std::mem::take(&mut cur));
            }
            if i + 1 < karakterler.len() && karakterler[i + 1] == '=' {
                tokenler.push(format!("{c}="));
                i += 2;
            } else {
                tokenler.push(c.to_string());
                i += 1;
            }
        } else {
            cur.push(c);
            i += 1;
        }
    }
    if !cur.is_empty() {
        tokenler.push(cur);
    }
    tokenler
}

fn sorgu_hatasi(detay: &str) -> ErrorReport {
    ErrorReport::new(
        "Sorgu ayrıştırılamadı",
        format!("Filtre ifadesi geçersiz: {detay}"),
        "Biçim: ALAN OP DEĞER (AND ile bağlanır). Örn: QUAL >= 30 AND FILTER = PASS",
    )
}

// ─── Kayıtlı filtre setleri ────────────────────────────────────────────────────

/// Adlandırılmış, kaydedilmiş bir filtre (ham ifade olarak saklanır → serde-basit, kalıcı).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KayitliFiltreSeti {
    /// Kullanıcının verdiği ad.
    pub ad: String,
    /// Filtrenin ham ifade metni ([`Filtre::ifade`]).
    pub sorgu: String,
}

/// Kaydedilmiş filtre setleri koleksiyonu (proje/oturumla birlikte kalıcılaştırılabilir).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FiltreSetleri {
    setler: Vec<KayitliFiltreSeti>,
}

impl FiltreSetleri {
    /// Boş koleksiyon.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir filtreyi adla kaydeder (aynı ad varsa üzerine yazar).
    pub fn kaydet(&mut self, ad: impl Into<String>, filtre: &Filtre) {
        let ad = ad.into();
        let sorgu = filtre.ifade();
        if let Some(mevcut) = self.setler.iter_mut().find(|s| s.ad == ad) {
            mevcut.sorgu = sorgu;
        } else {
            self.setler.push(KayitliFiltreSeti { ad, sorgu });
        }
    }

    /// Adlı seti siler; bulunduysa `true`.
    pub fn sil(&mut self, ad: &str) -> bool {
        let onceki = self.setler.len();
        self.setler.retain(|s| s.ad != ad);
        self.setler.len() != onceki
    }

    /// Adlı setin filtresini çözer (ham ifadeyi yeniden ayrıştırır).
    pub fn getir(&self, ad: &str) -> Option<Result<Filtre, ErrorReport>> {
        self.setler
            .iter()
            .find(|s| s.ad == ad)
            .map(|s| ayristir(&s.sorgu))
    }

    /// Tüm kayıtlı setler.
    pub fn liste(&self) -> &[KayitliFiltreSeti] {
        &self.setler
    }

    /// Kayıtlı set sayısı.
    pub fn sayi(&self) -> usize {
        self.setler.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_io::VaryantKaydi;

    fn satir(
        pos: usize,
        id: &str,
        r: &str,
        a: &str,
        q: f32,
        filt: &str,
        info: &[(&str, &str)],
    ) -> VaryantSatiri {
        VaryantSatiri::yeni(VaryantKaydi {
            kromozom: "chr1".into(),
            konum: pos,
            kimlik: id.into(),
            referans: r.into(),
            alternatifler: a.split(',').map(|s| s.to_string()).collect(),
            kalite: Some(q),
            filtreler: vec![filt.into()],
            info: info
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            ornek_sayisi: 0,
            format_anahtarlari: vec![],
            genotipler: vec![],
        })
    }

    #[test]
    fn bos_filtre_hepsini_gecer() {
        let f = Filtre::default();
        assert!(f.bos_mu());
        assert!(f.gecer(&satir(100, "rs1", "A", "G", 50.0, "PASS", &[])));
    }

    #[test]
    fn kalite_ve_pass_birlikte() {
        let f = Filtre {
            kalite_min: Some(30.0),
            sadece_pass: true,
            ..Filtre::default()
        };
        assert!(f.gecer(&satir(100, "rs1", "A", "G", 50.0, "PASS", &[])));
        assert!(!f.gecer(&satir(100, "rs1", "A", "G", 50.0, "q10", &[]))); // PASS değil
        assert!(!f.gecer(&satir(100, "rs1", "A", "G", 20.0, "PASS", &[]))); // QUAL düşük
    }

    #[test]
    fn tur_filtresi() {
        let f = Filtre {
            turler: Some(vec![VaryantTuru::Snv]),
            ..Filtre::default()
        };
        assert!(f.gecer(&satir(100, ".", "A", "G", 50.0, "PASS", &[]))); // SNV
        assert!(!f.gecer(&satir(100, ".", "A", "ACGT", 50.0, "PASS", &[]))); // INS
    }

    #[test]
    fn info_sayisal_kosul() {
        let f = Filtre {
            info_kosullari: vec![InfoKosul {
                anahtar: "DP".into(),
                op: Karsilastirma::Buyuk,
                deger: "20".into(),
            }],
            ..Filtre::default()
        };
        assert!(f.gecer(&satir(
            100,
            ".",
            "A",
            "G",
            50.0,
            "PASS",
            &[("DP", "Integer(30)")]
        )));
        assert!(!f.gecer(&satir(
            100,
            ".",
            "A",
            "G",
            50.0,
            "PASS",
            &[("DP", "Integer(10)")]
        )));
        // Alan yoksa sayısal koşul başarısız.
        assert!(!f.gecer(&satir(100, ".", "A", "G", 50.0, "PASS", &[])));
    }

    #[test]
    fn ayristir_temel() {
        let f = ayristir("QUAL >= 30 AND FILTER = PASS").unwrap();
        assert_eq!(f.kalite_min, Some(30.0));
        assert!(f.sadece_pass);
    }

    #[test]
    fn ayristir_bolge_ve_bosluksuz_operator() {
        let f = ayristir("CHROM=chr1 AND POS>=100 AND POS<=200").unwrap();
        let b = f.bolge.unwrap();
        assert_eq!(b.kromozom, "chr1");
        assert_eq!((b.baslangic, b.bitis), (100, 200));
    }

    #[test]
    fn ayristir_pos_chrom_olmadan_hata() {
        assert!(ayristir("POS >= 100").is_err());
    }

    #[test]
    fn ayristir_bilinmeyen_alan_hata() {
        let h = ayristir("FOO = 1").unwrap_err();
        assert_eq!(h.ne_oldu, "Sorgu ayrıştırılamadı");
    }

    #[test]
    fn ifade_round_trip() {
        let f1 = ayristir("CHROM = chr2 AND POS >= 500 AND QUAL >= 40 AND FILTER = PASS AND TYPE = SNV AND ID ~ rs AND INFO.DP > 10").unwrap();
        let metin = f1.ifade();
        let f2 = ayristir(&metin).unwrap();
        assert_eq!(f1, f2);
    }

    #[test]
    fn bos_ifade_bos_filtre() {
        assert!(ayristir("").unwrap().bos_mu());
        assert!(ayristir("   ").unwrap().bos_mu());
    }

    #[test]
    fn filtre_setleri_kaydet_getir_sil() {
        let mut setler = FiltreSetleri::yeni();
        let f = Filtre {
            kalite_min: Some(30.0),
            sadece_pass: true,
            ..Filtre::default()
        };
        setler.kaydet("yüksek kalite", &f);
        assert_eq!(setler.sayi(), 1);
        let geri = setler.getir("yüksek kalite").unwrap().unwrap();
        assert_eq!(geri, f);
        assert!(setler.sil("yüksek kalite"));
        assert_eq!(setler.sayi(), 0);
    }
}
