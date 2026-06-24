//! ÇE-09 (Gün 41) — **UniProt konektörü** (protein bilgisi + dizi).
//!
//! * **Arama:** UniProtKB REST (`rest.uniprot.org/uniprotkb/search?query=...&format=json`) →
//!   protein girişleri (accession / ad / organizma / uzunluk).
//! * **Getir (tek-tık):** `rest.uniprot.org/uniprotkb/{accession}.fasta` → FASTA dizi → projeye
//!   eklenir (provenance: UniProt CC BY 4.0).
//!
//! > **Sayfalama notu:** UniProt derin sayfalamayı *cursor* (Link başlığı) ile yapar; bu sürüm
//! > HTTP başlığı okumadığından **ilk sayfa** (`size=limit`) döndürülür → sonraki sayfalar (cursor)
//! > v1.x.  Sonuçlar yine de kaynak/atıf provenance'ıyla doğru taşınır.

use biocraft_sdk::biocraft_types::ErrorReport;
use serde_json::Value;

use crate::data_io::tekrar_ile;

use super::super::framework::{
    AramaBaglami, AramaSonucu, GetirilenKayit, KayitTuru, SayfaBilgisi, Sayfalama, SonucListesi,
    Sorgu, VeritabaniKonektoru,
};
use super::super::privacy::{DisVeri, HassasiyetEtiketi};
use super::super::provenance::{db_provenansi, uniprot_lisans_atif};
use super::super::transport::HttpIstek;

/// UniProtKB REST tabanı.
pub const REST_TABAN: &str = "https://rest.uniprot.org/uniprotkb";
/// UniProt web (tarayıcıda aç).
pub const WEB_TABAN: &str = "https://www.uniprot.org/uniprotkb";
const KAYNAK_ADI: &str = "UniProt";
const HEDEF: &str = "UniProt (rest.uniprot.org)";

/// UniProt konektörü.
pub struct UniprotKonektor {
    turler: [KayitTuru; 1],
}

impl UniprotKonektor {
    /// Yeni UniProt konektörü.
    pub fn yeni() -> Self {
        Self {
            turler: [KayitTuru::Protein],
        }
    }

    fn gonder(&self, istek: &HttpIstek, baglam: &AramaBaglami) -> Result<String, ErrorReport> {
        baglam.hiz_bekle_kaynak(KAYNAK_ADI);
        let yanit = tekrar_ile(&baglam.yapi, |_| {
            let y = baglam.ulastirici.gonder(istek)?;
            y.metin()?;
            Ok(y)
        })?;
        Ok(yanit.govde)
    }
}

impl Default for UniprotKonektor {
    fn default() -> Self {
        Self::yeni()
    }
}

impl VeritabaniKonektoru for UniprotKonektor {
    fn kaynak_adi(&self) -> &str {
        KAYNAK_ADI
    }

    fn turler(&self) -> &[KayitTuru] {
        &self.turler
    }

    fn tarayici_url(&self, kimlik: &str) -> Option<String> {
        let k = kimlik.trim();
        if k.is_empty() {
            None
        } else {
            Some(format!("{WEB_TABAN}/{k}/entry"))
        }
    }

    fn ara(
        &self,
        sorgu: &Sorgu,
        sayfalama: Sayfalama,
        baglam: &AramaBaglami,
    ) -> Result<SonucListesi, ErrorReport> {
        baglam.gizlilik.dis_gonderim(
            KAYNAK_ADI,
            HEDEF,
            DisVeri::Metin(&sorgu.metin),
            HassasiyetEtiketi::Genel,
        )?;

        let istek = HttpIstek::get(format!("{REST_TABAN}/search"))
            .param("query", &sorgu.metin)
            .param("format", "json")
            .param("size", sayfalama.limit.to_string());
        let govde = self.gonder(&istek, baglam)?;
        let sonuclar = sonuclari_ayristir(&govde)?;

        // Cursor sayfalama yok → bu sayfa = tüm görünen sonuç (toplam = bu sayfa).
        let sayfa = SayfaBilgisi {
            toplam: sonuclar.len() as u64,
            ofset: 0,
            limit: sayfalama.limit,
        };
        Ok(SonucListesi { sonuclar, sayfa })
    }

    fn detay(&self, kimlik: &str, baglam: &AramaBaglami) -> Result<AramaSonucu, ErrorReport> {
        baglam.gizlilik.dis_gonderim(
            KAYNAK_ADI,
            HEDEF,
            DisVeri::Metin(kimlik),
            HassasiyetEtiketi::Genel,
        )?;
        let istek = HttpIstek::get(format!("{REST_TABAN}/{kimlik}")).param("format", "json");
        let govde = self.gonder(&istek, baglam)?;
        let v: Value = serde_json::from_str(&govde).map_err(json_hatasi)?;
        Ok(satir_olustur(&v)
            .unwrap_or_else(|| AramaSonucu::yeni(KAYNAK_ADI, kimlik, kimlik, KayitTuru::Protein)))
    }

    fn getir(&self, kimlik: &str, baglam: &AramaBaglami) -> Result<GetirilenKayit, ErrorReport> {
        baglam.gizlilik.dis_gonderim(
            KAYNAK_ADI,
            HEDEF,
            DisVeri::Metin(kimlik),
            HassasiyetEtiketi::Genel,
        )?;
        let istek = HttpIstek::get(format!("{REST_TABAN}/{kimlik}.fasta"));
        let icerik = self.gonder(&istek, baglam)?.into_bytes();

        let provenans = db_provenansi(
            format!("{kimlik}.fasta"),
            "UniProt (rest.uniprot.org)",
            "FASTA",
            &icerik,
            Some(uniprot_lisans_atif()),
        );
        Ok(GetirilenKayit {
            kimlik: kimlik.to_string(),
            format_ipucu: "fasta".to_string(),
            icerik,
            provenans,
        })
    }
}

// ─── JSON ayrıştırma ─────────────────────────────────────────────────────────────

/// UniProt arama yanıtının `results` dizisinden sonuç satırları üretir.
fn sonuclari_ayristir(govde: &str) -> Result<Vec<AramaSonucu>, ErrorReport> {
    let v: Value = serde_json::from_str(govde).map_err(json_hatasi)?;
    let results = v
        .get("results")
        .and_then(|x| x.as_array())
        .ok_or_else(|| sema_hatasi("results"))?;
    Ok(results.iter().filter_map(satir_olustur).collect())
}

/// Tek bir UniProt giriş nesnesinden sonuç satırı (accession/ad/organizma/uzunluk).
fn satir_olustur(obj: &Value) -> Option<AramaSonucu> {
    let accession = obj.get("primaryAccession").and_then(|x| x.as_str())?;
    let ad = protein_adi(obj).unwrap_or_else(|| {
        obj.get("uniProtkbId")
            .and_then(|x| x.as_str())
            .unwrap_or(accession)
            .to_string()
    });
    let organizma = obj
        .get("organism")
        .and_then(|o| o.get("scientificName"))
        .and_then(|x| x.as_str());
    let uzunluk = obj
        .get("sequence")
        .and_then(|s| s.get("length"))
        .and_then(|x| x.as_u64());

    let mut s = AramaSonucu::yeni(KAYNAK_ADI, accession, ad, KayitTuru::Protein);
    if let Some(o) = organizma {
        s = s.with_organizma(o);
    }
    if let Some(u) = uzunluk {
        s = s.with_uzunluk(u);
    }
    if let Some(id) = obj.get("uniProtkbId").and_then(|x| x.as_str()) {
        s = s.with_aciklama(format!("UniProtKB: {id}"));
    }
    Some(s)
}

/// Önerilen ad → gönderilen ad → `None` sırasıyla protein adını çözer.
fn protein_adi(obj: &Value) -> Option<String> {
    let pd = obj.get("proteinDescription")?;
    if let Some(ad) = pd
        .get("recommendedName")
        .and_then(|r| r.get("fullName"))
        .and_then(|f| f.get("value"))
        .and_then(|x| x.as_str())
    {
        return Some(ad.to_string());
    }
    pd.get("submissionNames")
        .and_then(|x| x.as_array())
        .and_then(|a| a.first())
        .and_then(|n| n.get("fullName"))
        .and_then(|f| f.get("value"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
}

fn json_hatasi(e: serde_json::Error) -> ErrorReport {
    ErrorReport::new(
        "UniProt yanıtı çözümlenemedi",
        "UniProt'tan gelen JSON beklenen biçimde değil",
        "Daha sonra yeniden deneyin; sorun sürerse konektör güncellemesi gerekebilir",
    )
    .with_teknik_detay(e.to_string())
}

fn sema_hatasi(alan: &str) -> ErrorReport {
    ErrorReport::new(
        "UniProt yanıtı beklenen alanı içermiyor",
        format!("yanıtta '{alan}' alanı yok (API değişmiş olabilir)"),
        "Konektörü güncelleyin veya daha sonra yeniden deneyin",
    )
    .with_teknik_detay(format!("eksik_alan={alan}"))
}

#[cfg(test)]
mod tests {
    use super::super::super::privacy::GizlilikKapisi;
    use super::super::super::transport::SahteUlastirici;
    use super::*;
    use std::time::Duration;

    fn search_json() -> &'static str {
        r#"{"results":[
            {"primaryAccession":"P04637","uniProtkbId":"P53_HUMAN",
             "proteinDescription":{"recommendedName":{"fullName":{"value":"Cellular tumor antigen p53"}}},
             "organism":{"scientificName":"Homo sapiens"},
             "sequence":{"length":393}},
            {"primaryAccession":"P38634","uniProtkbId":"P53_RAT",
             "proteinDescription":{"recommendedName":{"fullName":{"value":"Cellular tumor antigen p53"}}},
             "organism":{"scientificName":"Rattus norvegicus"},
             "sequence":{"length":391}}
        ]}"#
    }

    fn baglam_kur<'a>(u: &'a SahteUlastirici, g: &'a GizlilikKapisi) -> AramaBaglami<'a> {
        let mut b = AramaBaglami::yeni(u, g);
        b.yapi.geri_cekilme_taban = Duration::ZERO;
        b
    }

    #[test]
    fn arama_protein_satirlari() {
        let u = SahteUlastirici::yeni().ekle("uniprotkb/search", 200, search_json());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = UniprotKonektor::yeni();
        let liste = kon
            .ara(&Sorgu::metin("p53"), Sayfalama::ilk(20), &baglam)
            .unwrap();
        assert_eq!(liste.sonuclar.len(), 2);
        let s0 = &liste.sonuclar[0];
        assert_eq!(s0.kimlik, "P04637");
        assert_eq!(s0.tur, KayitTuru::Protein);
        assert!(s0.baslik.contains("p53"));
        assert_eq!(s0.organizma.as_deref(), Some("Homo sapiens"));
        assert_eq!(s0.uzunluk, Some(393));
    }

    #[test]
    fn getir_fasta_ve_provenans() {
        let fasta = ">sp|P04637|P53_HUMAN Cellular tumor antigen p53\nMEEPQSDPSV\n";
        let u = SahteUlastirici::yeni().ekle("P04637.fasta", 200, fasta);
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = UniprotKonektor::yeni();
        let kayit = kon.getir("P04637", &baglam).unwrap();
        assert_eq!(kayit.format_ipucu, "fasta");
        assert!(kayit.icerik.starts_with(b">sp|P04637"));
        assert!(kayit
            .provenans
            .lisans_atif
            .as_ref()
            .unwrap()
            .lisans
            .contains("CC BY 4.0"));
    }

    #[test]
    fn tarayici_url() {
        let kon = UniprotKonektor::yeni();
        assert_eq!(
            kon.tarayici_url("P04637").as_deref(),
            Some("https://www.uniprot.org/uniprotkb/P04637/entry")
        );
    }

    #[test]
    fn gonderilen_ad_yedek_kullanilir() {
        // recommendedName yoksa submissionNames kullanılır.
        let js = r#"{"results":[{"primaryAccession":"A0A1","uniProtkbId":"X_Y",
            "proteinDescription":{"submissionNames":[{"fullName":{"value":"Uncharacterized protein"}}]},
            "sequence":{"length":100}}]}"#;
        let satirlar = sonuclari_ayristir(js).unwrap();
        assert_eq!(satirlar[0].baslik, "Uncharacterized protein");
    }
}
