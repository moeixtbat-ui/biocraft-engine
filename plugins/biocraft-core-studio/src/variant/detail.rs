//! ÇE-04 — **Detay**: seçili varyantın INFO/FORMAT tam dökümü + genom tarayıcıya **"varyanta git"**
//! hedefi + rsID/dış anotasyon bağlantısı (dürüst: dış bağlantı `net` + kullanıcı onayı ister).

use biocraft_sdk::biocraft_types::ErrorReport;

use crate::genome_browser::canvas::GenomBolge;

use super::query::VaryantSatiri;

/// "Varyanta git" hedefini varyantın çevresine açarken kullanılan varsayılan pencere (bp).
pub const VARSAYILAN_BAGLAM_BP: u64 = 80;

/// Seçili varyantın anahtar/değer detay listesi (İçerik paneli için).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaryantDetay {
    /// (etiket, değer) çiftleri — sırayla gösterilir.
    pub satirlar: Vec<(String, String)>,
}

/// Bir varyant satırından detay paneli üretir (çekirdek alanlar + INFO + örnek GT'leri).
pub fn detay(satir: &VaryantSatiri, ornekler: &[String]) -> VaryantDetay {
    let k = &satir.kayit;
    let mut s: Vec<(String, String)> = Vec::new();
    s.push(("Konum".into(), format!("{}:{}", k.kromozom, k.konum)));
    s.push(("ID".into(), k.kimlik.clone()));
    s.push(("REF".into(), k.referans.clone()));
    s.push(("ALT".into(), satir.alt_metni()));
    s.push((
        "QUAL".into(),
        k.kalite
            .map(|q| format!("{q}"))
            .unwrap_or_else(|| ".".into()),
    ));
    s.push((
        "FILTER".into(),
        if k.filtreler.is_empty() {
            ".".into()
        } else {
            k.filtreler.join(";")
        },
    ));
    s.push(("Tür".into(), satir.tur.etiket().into()));

    // INFO alanları.
    for (anahtar, deger) in &k.info {
        s.push((format!("INFO.{anahtar}"), deger.clone()));
    }

    // Örnek genotipleri (FORMAT GT).
    for (i, gt) in k.genotipler.iter().enumerate() {
        let ad = ornekler
            .get(i)
            .cloned()
            .unwrap_or_else(|| format!("örnek{i}"));
        s.push((format!("GT[{ad}]"), gt.clone()));
    }

    VaryantDetay { satirlar: s }
}

/// Genom tarayıcıda varyantı ortalayan **"varyanta git"** bölge hedefi (Gün 36-37 tarayıcısı).
/// Koordinat tabanı **1-tabanlı** (VCF POS = tarayıcı `GenomBolge` ile birebir → kayma yok).
pub fn tarayici_hedefi(satir: &VaryantSatiri, baglam_bp: u64) -> GenomBolge {
    GenomBolge::merkezli(
        satir.kromozom().to_string(),
        satir.konum() as u64,
        baglam_bp.max(1),
    )
}

/// rsID için dbSNP bağlantısı (yalnız `rs<rakam>` kimliklerinde).  **Dış kaynak**: açmadan önce
/// kullanıcı onayı + `net` yetkisi gerekir (çekirdek veri sınırı korunur — bu yalnız URL üretir).
pub fn dbsnp_baglantisi(satir: &VaryantSatiri) -> Option<String> {
    let id = satir.kayit.kimlik.trim();
    let govde = id.strip_prefix("rs").or_else(|| id.strip_prefix("RS"))?;
    if !govde.is_empty() && govde.chars().all(|c| c.is_ascii_digit()) {
        Some(format!("https://www.ncbi.nlm.nih.gov/snp/rs{govde}"))
    } else {
        None
    }
}

/// Bir genom tarayıcı hedefini insan-okur bölge metnine çevirir (teşhis/log).
pub fn hedef_metni(bolge: &GenomBolge) -> Result<String, ErrorReport> {
    Ok(format!(
        "{}:{}-{}",
        bolge.kromozom, bolge.baslangic, bolge.bitis
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_io::VaryantKaydi;

    fn satir(kr: &str, pos: usize, id: &str) -> VaryantSatiri {
        VaryantSatiri::yeni(VaryantKaydi {
            kromozom: kr.into(),
            konum: pos,
            kimlik: id.into(),
            referans: "A".into(),
            alternatifler: vec!["G".into()],
            kalite: Some(50.0),
            filtreler: vec!["PASS".into()],
            info: vec![("DP".into(), "Integer(30)".into())],
            ornek_sayisi: 1,
            format_anahtarlari: vec!["GT".into()],
            genotipler: vec!["0/1".into()],
        })
    }

    #[test]
    fn detay_alanlari_icerir() {
        let d = detay(&satir("chr1", 100, "rs5"), &["S1".into()]);
        assert!(d
            .satirlar
            .iter()
            .any(|(k, v)| k == "Konum" && v == "chr1:100"));
        assert!(d.satirlar.iter().any(|(k, _)| k == "INFO.DP"));
        assert!(d.satirlar.iter().any(|(k, v)| k == "GT[S1]" && v == "0/1"));
    }

    #[test]
    fn tarayici_hedefi_varyanti_ortalar() {
        let b = tarayici_hedefi(&satir("chr1", 1000, "rs5"), 80);
        assert_eq!(b.kromozom, "chr1");
        assert!(b.kapsar(1000));
        assert_eq!(b.uzunluk(), 80);
    }

    #[test]
    fn dbsnp_yalniz_rs_kimliklerinde() {
        assert_eq!(
            dbsnp_baglantisi(&satir("chr1", 100, "rs123")),
            Some("https://www.ncbi.nlm.nih.gov/snp/rs123".into())
        );
        assert_eq!(dbsnp_baglantisi(&satir("chr1", 100, ".")), None);
        assert_eq!(dbsnp_baglantisi(&satir("chr1", 100, "COSM1")), None);
    }
}
