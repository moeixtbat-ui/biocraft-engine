//! ÇE-11 — **Veri / tablo dışa aktarma** (CSV/TSV + FASTA + VCF köprüsü) + **gizlilik filtresi**.
//!
//! Görünümlerdeki tablolar/seçimler ve diziler yayın/analiz için metin biçimlerine yazılır.  İki
//! disiplin korunur:
//! * **Filtre/seçim korunur:** dışa aktarma yalnızca **seçili** satırları (varsa) içerir; sıra korunur.
//! * **Gizlilik (Gün 18 / MK-42/43):** [`GizlilikSuzgeci`] **PHI/hassas** etiketli sütunları dışa
//!   aktarmadan **düşürür** (onaysız sızmaz).  Aynı sınıflandırma raporda da uygulanır ([`super::report`]).
//!
//! Varyant alt-kümesi VCF'i ÇE-04'ün [`crate::variant::disa_aktar::vcf_olustur`]'una köprülenir
//! (tek doğruluk kaynağı; tekrar yazılmaz).  Saf metin üretir; dosyaya yazma `fs`-kapılı çağırandadır.

use crate::db_search::HassasiyetEtiketi;

/// Varyant alt-kümesi VCF dışa aktarımı — ÇE-04 ile **aynı** üretici (köprü; seçim çağıranca süzülür).
pub use crate::variant::disa_aktar::vcf_olustur as varyant_vcf;

/// CSV/TSV ayracı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ayrac {
    /// Virgül (CSV).
    Virgul,
    /// Sekme (TSV).
    Sekme,
}

impl Ayrac {
    /// Ayraç karakteri.
    pub fn karakter(&self) -> char {
        match self {
            Ayrac::Virgul => ',',
            Ayrac::Sekme => '\t',
        }
    }

    /// Dosya uzantısı.
    pub fn uzanti(&self) -> &'static str {
        match self {
            Ayrac::Virgul => "csv",
            Ayrac::Sekme => "tsv",
        }
    }
}

/// Render-bağımsız basit **tablo** (başlık + satırlar) — herhangi bir görünümün dışa aktarılabilir
/// veri biçimi.  Her sütuna isteğe bağlı bir **hassasiyet etiketi** verilir (gizlilik filtresi için).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tablo {
    /// Sütun başlıkları.
    pub basliklar: Vec<String>,
    /// Satırlar (her biri başlıklarla aynı uzunlukta olmalı; kısa satır boş hücreyle tamamlanır).
    pub satirlar: Vec<Vec<String>>,
    /// Sütun başına hassasiyet etiketi (boşsa hepsi `Genel`).
    pub sutun_etiket: Vec<HassasiyetEtiketi>,
}

impl Tablo {
    /// Başlıklarla boş tablo (tüm sütunlar `Genel`).
    pub fn yeni(basliklar: Vec<String>) -> Self {
        let n = basliklar.len();
        Self {
            basliklar,
            satirlar: Vec::new(),
            sutun_etiket: vec![HassasiyetEtiketi::Genel; n],
        }
    }

    /// Bir satır ekler.
    pub fn satir_ekle(&mut self, satir: Vec<String>) -> &mut Self {
        self.satirlar.push(satir);
        self
    }

    /// Bir sütunun hassasiyet etiketini ayarlar (sınır dışıysa yok sayılır).
    pub fn sutun_etiketle(&mut self, sutun: usize, etiket: HassasiyetEtiketi) -> &mut Self {
        if self.sutun_etiket.len() < self.basliklar.len() {
            self.sutun_etiket
                .resize(self.basliklar.len(), HassasiyetEtiketi::Genel);
        }
        if let Some(e) = self.sutun_etiket.get_mut(sutun) {
            *e = etiket;
        }
        self
    }

    /// `sutun` indeksinin etiketi (tanımsızsa `Genel`).
    fn etiket(&self, sutun: usize) -> HassasiyetEtiketi {
        self.sutun_etiket
            .get(sutun)
            .copied()
            .unwrap_or(HassasiyetEtiketi::Genel)
    }
}

/// **Gizlilik filtresi** — dışa aktarmada hangi sütunların görünebileceğine karar verir (Gün 18).
///
/// * **PHI** sütunu **hiçbir koşulda** dışa aktarılmaz (onaylı olsa bile — MK-42/43).
/// * **Hassas** sütunu yalnız kullanıcı açıkça onayladıysa ([`onay`](Self::onay)) çıkar.
/// * **Genel** her zaman çıkar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GizlilikSuzgeci {
    /// Kullanıcı hassas (PHI olmayan) alanların dışa aktarımını onayladı mı?
    pub onay: bool,
}

impl GizlilikSuzgeci {
    /// Güvenli varsayılan: yalnız genel alanlar (hassas için açık onay gerek).
    pub fn yeni() -> Self {
        Self { onay: false }
    }

    /// Kullanıcının hassas alanları onayladığı filtre (PHI yine engellenir).
    pub fn onayli() -> Self {
        Self { onay: true }
    }

    /// Bu etiketli bir sütun dışa aktarılabilir mi?
    pub fn izinli(&self, etiket: HassasiyetEtiketi) -> bool {
        match etiket {
            HassasiyetEtiketi::Genel => true,
            HassasiyetEtiketi::Hassas => self.onay,
            HassasiyetEtiketi::Phi => false,
        }
    }

    /// Tabloyu süzer: izinsiz sütunları (başlık + tüm hücreler) **tamamen çıkarır**.
    pub fn temizle(&self, tablo: &Tablo) -> Tablo {
        let tut: Vec<usize> = (0..tablo.basliklar.len())
            .filter(|&i| self.izinli(tablo.etiket(i)))
            .collect();
        let basliklar = tut.iter().map(|&i| tablo.basliklar[i].clone()).collect();
        let sutun_etiket = tut.iter().map(|&i| tablo.etiket(i)).collect();
        let satirlar = tablo
            .satirlar
            .iter()
            .map(|satir| {
                tut.iter()
                    .map(|&i| satir.get(i).cloned().unwrap_or_default())
                    .collect()
            })
            .collect();
        Tablo {
            basliklar,
            satirlar,
            sutun_etiket,
        }
    }
}

impl Default for GizlilikSuzgeci {
    fn default() -> Self {
        Self::yeni()
    }
}

/// Tabloyu CSV/TSV olarak dışa aktarır.  `secili` verilirse **yalnız o satır indeksleri** (sırayla)
/// yazılır (seçim korunur); `None` ise tüm satırlar.  Gizlilik filtresi sütunları zaten süzmüş olmalı
/// (önce [`GizlilikSuzgeci::temizle`] çağrılır) — bu fonksiyon yalnız biçimlendirir.
pub fn tablo_disa_aktar(tablo: &Tablo, ayrac: Ayrac, secili: Option<&[usize]>) -> String {
    let sep = ayrac.karakter();
    let mut s = String::new();

    s.push_str(&satir_birlestir(&tablo.basliklar, sep));
    s.push('\n');

    match secili {
        Some(idx) => {
            for &i in idx {
                if let Some(satir) = tablo.satirlar.get(i) {
                    s.push_str(&satir_birlestir(satir, sep));
                    s.push('\n');
                }
            }
        }
        None => {
            for satir in &tablo.satirlar {
                s.push_str(&satir_birlestir(satir, sep));
                s.push('\n');
            }
        }
    }
    s
}

/// Bir alan listesini ayraçla birleştirir (her alan kaçışlı).
fn satir_birlestir(alanlar: &[String], sep: char) -> String {
    alanlar
        .iter()
        .map(|a| alan_kacis(a, sep))
        .collect::<Vec<_>>()
        .join(&sep.to_string())
}

/// Bir alanı kaçışlar: ayraç/tırnak/yeni-satır içeriyorsa çift tırnağa alır, iç tırnağı ikiler (RFC 4180).
fn alan_kacis(alan: &str, sep: char) -> String {
    if alan.contains([sep, '"', '\n', '\r']) {
        format!("\"{}\"", alan.replace('"', "\"\""))
    } else {
        alan.to_string()
    }
}

// ─── FASTA ─────────────────────────────────────────────────────────────────────────

/// Dışa aktarılacak bir dizi kaydı (FASTA).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastaKaydi {
    /// Tanımlayıcı (`>` sonrası ilk simge).
    pub ad: String,
    /// Opsiyonel açıklama (başlık satırında addan sonra).
    pub aciklama: Option<String>,
    /// Dizi (baz/kalıntı; büyük/küçük harf korunur).
    pub dizi: String,
}

impl FastaKaydi {
    /// Ad + dizi ile kayıt (açıklamasız).
    pub fn yeni(ad: impl Into<String>, dizi: impl Into<String>) -> Self {
        Self {
            ad: ad.into(),
            aciklama: None,
            dizi: dizi.into(),
        }
    }

    /// Açıklama ekler (akıcı).
    pub fn with_aciklama(mut self, aciklama: impl Into<String>) -> Self {
        self.aciklama = Some(aciklama.into());
        self
    }
}

/// Standart FASTA satır genişliği (NCBI/EMBL geleneği).
pub const FASTA_SATIR: usize = 60;

/// Kayıtları FASTA metnine çevirir (`satir_uzunlugu` = her dizi satırının sarma genişliği, 0 → sarmasız).
pub fn fasta_olustur(kayitlar: &[FastaKaydi], satir_uzunlugu: usize) -> String {
    let mut s = String::new();
    for k in kayitlar {
        s.push('>');
        s.push_str(&k.ad);
        if let Some(a) = &k.aciklama {
            s.push(' ');
            s.push_str(a);
        }
        s.push('\n');
        if satir_uzunlugu == 0 {
            s.push_str(&k.dizi);
            s.push('\n');
        } else {
            let karakterler: Vec<char> = k.dizi.chars().collect();
            for parca in karakterler.chunks(satir_uzunlugu) {
                s.extend(parca.iter());
                s.push('\n');
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ornek_tablo() -> Tablo {
        let mut t = Tablo::yeni(vec!["CHROM".into(), "POS".into(), "GEN".into()]);
        t.satir_ekle(vec!["chr1".into(), "100".into(), "BRCA1".into()]);
        t.satir_ekle(vec!["chr2".into(), "200".into(), "TP53".into()]);
        t.satir_ekle(vec!["chr3".into(), "300".into(), "EGFR".into()]);
        t
    }

    #[test]
    fn csv_baslik_ve_tum_satirlar() {
        let csv = tablo_disa_aktar(&ornek_tablo(), Ayrac::Virgul, None);
        let satirlar: Vec<&str> = csv.lines().collect();
        assert_eq!(satirlar[0], "CHROM,POS,GEN");
        assert_eq!(satirlar.len(), 4); // başlık + 3
        assert_eq!(satirlar[1], "chr1,100,BRCA1");
    }

    #[test]
    fn tsv_ayraci_sekme() {
        let tsv = tablo_disa_aktar(&ornek_tablo(), Ayrac::Sekme, None);
        assert!(tsv.lines().next().unwrap().contains('\t'));
        assert_eq!(Ayrac::Sekme.uzanti(), "tsv");
    }

    #[test]
    fn secim_korunur() {
        // Yalnız 0. ve 2. satır (seçim) — sırayla.
        let csv = tablo_disa_aktar(&ornek_tablo(), Ayrac::Virgul, Some(&[2, 0]));
        let satirlar: Vec<&str> = csv.lines().collect();
        assert_eq!(satirlar.len(), 3); // başlık + 2 seçili
        assert_eq!(satirlar[1], "chr3,300,EGFR"); // önce 2. indeks
        assert_eq!(satirlar[2], "chr1,100,BRCA1");
    }

    #[test]
    fn csv_kacis_virgul_ve_tirnak() {
        assert_eq!(alan_kacis("a,b", ','), "\"a,b\"");
        assert_eq!(alan_kacis("a\"b", ','), "\"a\"\"b\"");
        // Sekme ayracında virgül kaçışsız.
        assert_eq!(alan_kacis("a,b", '\t'), "a,b");
    }

    #[test]
    fn gizlilik_phi_sutunu_dusurulur() {
        let mut t = ornek_tablo();
        // 2. sütun (GEN) hassas, ayrıca bir PHI sütunu ekleyelim.
        t.basliklar.push("HASTA_ID".into());
        t.sutun_etiket.push(HassasiyetEtiketi::Phi);
        for (i, satir) in t.satirlar.iter_mut().enumerate() {
            satir.push(format!("P{i}"));
        }
        // PHI sütunu onaylı filtrede bile düşer.
        let suzulmus = GizlilikSuzgeci::onayli().temizle(&t);
        assert!(!suzulmus.basliklar.contains(&"HASTA_ID".to_string()));
        assert_eq!(suzulmus.basliklar.len(), 3);
        let csv = tablo_disa_aktar(&suzulmus, Ayrac::Virgul, None);
        assert!(!csv.contains("P0"));
        assert!(!csv.contains("HASTA_ID"));
    }

    #[test]
    fn gizlilik_hassas_onaysiz_dusurulur_onayli_kalir() {
        let mut t = ornek_tablo();
        t.sutun_etiketle(2, HassasiyetEtiketi::Hassas); // GEN = hassas
                                                        // Onaysız: hassas sütun çıkmaz.
        let onaysiz = GizlilikSuzgeci::yeni().temizle(&t);
        assert_eq!(onaysiz.basliklar, vec!["CHROM", "POS"]);
        // Onaylı: hassas çıkar.
        let onayli = GizlilikSuzgeci::onayli().temizle(&t);
        assert!(onayli.basliklar.contains(&"GEN".to_string()));
    }

    #[test]
    fn fasta_sarma_ve_baslik() {
        let kayitlar = vec![
            FastaKaydi::yeni("seq1", "ACGTACGTAC").with_aciklama("örnek"),
            FastaKaydi::yeni("seq2", "TTTT"),
        ];
        let fasta = fasta_olustur(&kayitlar, 4);
        let satirlar: Vec<&str> = fasta.lines().collect();
        assert_eq!(satirlar[0], ">seq1 örnek");
        assert_eq!(satirlar[1], "ACGT"); // 4'lük sarma
        assert_eq!(satirlar[2], "ACGT");
        assert_eq!(satirlar[3], "AC");
        assert_eq!(satirlar[4], ">seq2");
        assert_eq!(satirlar[5], "TTTT");
    }

    #[test]
    fn fasta_sarmasiz() {
        let fasta = fasta_olustur(&[FastaKaydi::yeni("s", "ACGTACGT")], 0);
        assert_eq!(fasta, ">s\nACGTACGT\n");
    }

    #[test]
    fn varyant_vcf_koprusu_calisir() {
        // Köprü ÇE-04 üreticisine bağlı — boş girdi geçerli başlık üretir.
        let vcf = varyant_vcf(&[], &[]);
        assert!(vcf.starts_with("##fileformat=VCFv4.3"));
    }
}
