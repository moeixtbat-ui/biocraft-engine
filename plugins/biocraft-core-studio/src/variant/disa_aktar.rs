//! ÇE-04 — **Dışa aktarma**: filtrelenen varyant alt-kümesini **VCF** / **CSV** olarak yazar
//! (ÇE-11 dışa aktarma ile uyumlu).  Ayrıca golden doğrulama için **bcftools `query -f`**
//! eşdeğeri bir TSV üretir (Gün 32/43 golden çerçevesi → doğruluk kanıtı).
//!
//! Saf metin üretir (dosyaya yazma `fs`-kapılı çağıran tarafta yapılır — MK-13); yeni bağımlılık yok.

use super::filter::deger_sadelestir;
use super::query::VaryantSatiri;
use super::tablo::TabloDuzeni;

/// Filtreli satırları, **seçili (görünür) sütunlarla** CSV olarak dışa aktarır (başlık + satırlar).
pub fn csv_olustur(satirlar: &[VaryantSatiri], duzen: &TabloDuzeni, ornekler: &[String]) -> String {
    let mut s = String::new();
    let basliklar = duzen.basliklar(ornekler);
    s.push_str(
        &basliklar
            .iter()
            .map(|b| csv_kacis(b))
            .collect::<Vec<_>>()
            .join(","),
    );
    s.push('\n');
    for satir in satirlar {
        let degerler = duzen.satir_degerleri(satir);
        s.push_str(
            &degerler
                .iter()
                .map(|d| csv_kacis(d))
                .collect::<Vec<_>>()
                .join(","),
        );
        s.push('\n');
    }
    s
}

/// Filtreli satırları **minimal geçerli VCF** olarak dışa aktarır (çekirdek sütunlar + INFO +
/// varsa FORMAT/GT).  INFO değerleri kaynak Debug sarmalından sadeleştirilir (en iyi çaba).
pub fn vcf_olustur(satirlar: &[VaryantSatiri], ornekler: &[String]) -> String {
    let ornekli = !ornekler.is_empty() && satirlar.iter().any(|s| !s.kayit.genotipler.is_empty());

    let mut s = String::new();
    s.push_str("##fileformat=VCFv4.3\n");
    s.push_str("##source=BioCraftStudio-CE04\n");
    s.push_str("#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO");
    if ornekli {
        s.push_str("\tFORMAT");
        for ad in ornekler {
            s.push('\t');
            s.push_str(ad);
        }
    }
    s.push('\n');

    for satir in satirlar {
        let k = &satir.kayit;
        let id = if k.kimlik.is_empty() { "." } else { &k.kimlik };
        let alt = satir.alt_metni();
        let qual = k
            .kalite
            .map(|q| format!("{q}"))
            .unwrap_or_else(|| ".".into());
        let filter = if k.filtreler.is_empty() {
            ".".to_string()
        } else {
            k.filtreler.join(";")
        };
        let info = info_metni(&k.info);
        s.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            k.kromozom, k.konum, id, k.referans, alt, qual, filter, info
        ));
        if ornekli {
            s.push_str("\tGT");
            for i in 0..ornekler.len() {
                s.push('\t');
                let gt = k.genotipler.get(i).map(|g| g.as_str()).unwrap_or(".");
                s.push_str(if gt.is_empty() { "." } else { gt });
            }
        }
        s.push('\n');
    }
    s
}

/// **bcftools `query -f '%CHROM\t%POS\t%REF\t%ALT\t%QUAL\t%FILTER\n'`** eşdeğeri TSV (golden).
pub fn bcftools_query(satirlar: &[VaryantSatiri]) -> String {
    let mut s = String::new();
    for satir in satirlar {
        let k = &satir.kayit;
        let qual = k
            .kalite
            .map(|q| format!("{q}"))
            .unwrap_or_else(|| ".".into());
        let filter = if k.filtreler.is_empty() {
            ".".to_string()
        } else {
            k.filtreler.join(";")
        };
        s.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            k.kromozom,
            k.konum,
            k.referans,
            satir.alt_metni(),
            qual,
            filter
        ));
    }
    s
}

/// INFO çiftlerini `key=val;key2=val2` metnine çevirir (boşsa ".").
fn info_metni(info: &[(String, String)]) -> String {
    if info.is_empty() {
        return ".".to_string();
    }
    info.iter()
        .map(|(k, v)| {
            let sade = deger_sadelestir(v);
            if sade.is_empty() {
                k.clone()
            } else {
                format!("{k}={sade}")
            }
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Bir CSV alanını kaçışlar (virgül/tırnak/yeni-satır varsa tırnağa alır, iç tırnağı ikiler).
fn csv_kacis(alan: &str) -> String {
    if alan.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", alan.replace('"', "\"\""))
    } else {
        alan.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_io::VaryantKaydi;
    use crate::variant::query::VaryantSatiri;

    fn satir(
        kr: &str,
        pos: usize,
        id: &str,
        r: &str,
        a: &str,
        q: f32,
        filt: &str,
    ) -> VaryantSatiri {
        VaryantSatiri::yeni(VaryantKaydi {
            kromozom: kr.into(),
            konum: pos,
            kimlik: id.into(),
            referans: r.into(),
            alternatifler: a.split(',').map(|s| s.to_string()).collect(),
            kalite: Some(q),
            filtreler: vec![filt.into()],
            info: vec![("DP".into(), "Integer(30)".into())],
            ornek_sayisi: 1,
            format_anahtarlari: vec!["GT".into()],
            genotipler: vec!["0/1".into()],
        })
    }

    #[test]
    fn csv_baslik_ve_satir() {
        let duzen = TabloDuzeni::varsayilan(0);
        let csv = csv_olustur(
            &[satir("chr1", 100, "rs1", "A", "G", 50.0, "PASS")],
            &duzen,
            &[],
        );
        let satirlar: Vec<&str> = csv.lines().collect();
        assert!(satirlar[0].starts_with("CHROM,POS,ID,REF,ALT,QUAL,FILTER"));
        assert!(satirlar[1].starts_with("chr1,100,rs1,A,G,50,PASS"));
    }

    #[test]
    fn vcf_cikti_gecerli_basliklı() {
        let vcf = vcf_olustur(
            &[satir("chr1", 100, "rs1", "A", "G", 50.0, "PASS")],
            &["S1".into()],
        );
        assert!(vcf.starts_with("##fileformat=VCFv4.3"));
        assert!(vcf.contains("#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1"));
        assert!(vcf.contains("chr1\t100\trs1\tA\tG\t50\tPASS\tDP=30\tGT\t0/1"));
    }

    #[test]
    fn vcf_orneksiz_format_sutunu_yok() {
        let vcf = vcf_olustur(&[satir("chr1", 100, "rs1", "A", "G", 50.0, "PASS")], &[]);
        assert!(vcf.contains("#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n"));
        assert!(!vcf.contains("FORMAT"));
    }

    #[test]
    fn bcftools_query_formati() {
        let tsv = bcftools_query(&[
            satir("chr1", 100, "rs1", "A", "G", 50.0, "PASS"),
            satir("chr1", 150, ".", "G", "C,A", 60.0, "PASS"),
        ]);
        assert_eq!(
            tsv,
            "chr1\t100\tA\tG\t50\tPASS\nchr1\t150\tG\tC,A\t60\tPASS\n"
        );
    }

    #[test]
    fn csv_kacis_virgul() {
        assert_eq!(csv_kacis("a,b"), "\"a,b\"");
        assert_eq!(csv_kacis("düz"), "düz");
    }
}
