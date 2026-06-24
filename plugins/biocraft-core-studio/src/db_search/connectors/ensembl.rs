//! ÇE-09 (Gün 41) — **Ensembl konektörü** (gen / transkript / anotasyon + koordinat çapraz bağlantı).
//!
//! Ensembl REST (`rest.ensembl.org`) tam-metin arama sunmaz; sorgu bir **gen sembolü** kabul edilir:
//! * `xrefs/symbol/{tür}/{sembol}` → eşleşen Ensembl kimlikleri.
//! * `lookup/id/{id}` → gen ayrıntısı (ad / biyotip / **kromozom+başlangıç+bitiş** / açıklama).
//!   Koordinat [`Konum`] olarak sonuca işlenir → panel **"genom tarayıcıda göster"** (ÇE-02) verir.
//! * `sequence/id/{id}` (FASTA) → tek-tık dizi yükleme (provenance: Ensembl).
//!
//! Tür (species) varsayılan `homo_sapiens`; [`with_tur`](EnsemblKonektor::with_tur) ile değişir.

use biocraft_sdk::biocraft_types::ErrorReport;
use serde_json::Value;

use crate::data_io::tekrar_ile;

use super::super::framework::{
    AramaBaglami, AramaSonucu, GetirilenKayit, KayitTuru, Konum, SayfaBilgisi, Sayfalama,
    SonucListesi, Sorgu, VeritabaniKonektoru,
};
use super::super::privacy::{DisVeri, HassasiyetEtiketi};
use super::super::provenance::{db_provenansi, ensembl_lisans_atif};
use super::super::transport::HttpIstek;

/// Ensembl REST tabanı.
pub const REST_TABAN: &str = "https://rest.ensembl.org";
/// Ensembl web (tarayıcıda aç).
pub const WEB_TABAN: &str = "https://www.ensembl.org";
const KAYNAK_ADI: &str = "Ensembl";
const HEDEF: &str = "Ensembl REST (rest.ensembl.org)";

/// Ensembl konektörü (tek bir tür/species'e bağlı).
pub struct EnsemblKonektor {
    tur_adi: String,
    turler: [KayitTuru; 1],
}

impl EnsemblKonektor {
    /// Belirli bir tür için konektör (örn. `homo_sapiens`).
    pub fn yeni(tur_adi: impl Into<String>) -> Self {
        Self {
            tur_adi: tur_adi.into(),
            turler: [KayitTuru::Gen],
        }
    }

    /// İnsan (`homo_sapiens`) konektörü — varsayılan.
    pub fn insan() -> Self {
        Self::yeni("homo_sapiens")
    }

    /// Türü değiştirir (akıcı).
    pub fn with_tur(mut self, tur_adi: impl Into<String>) -> Self {
        self.tur_adi = tur_adi.into();
        self
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

    /// Bir Ensembl kimliği için ayrıntı (lookup) → sonuç satırı (koordinatlı).  Hata → `None`.
    fn lookup(&self, id: &str, baglam: &AramaBaglami) -> Option<AramaSonucu> {
        let istek = HttpIstek::get(format!("{REST_TABAN}/lookup/id/{id}"))
            .param("content-type", "application/json");
        let govde = self.gonder(&istek, baglam).ok()?;
        let v: Value = serde_json::from_str(&govde).ok()?;
        Some(self.satir_olustur(id, &v))
    }

    /// Web (tarayıcıda aç) için tür adını "Homo_sapiens" biçimine getirir.
    fn web_tur(&self) -> String {
        let mut c = self.tur_adi.chars();
        match c.next() {
            Some(ilk) => ilk.to_uppercase().collect::<String>() + c.as_str(),
            None => self.tur_adi.clone(),
        }
    }

    fn satir_olustur(&self, id: &str, v: &Value) -> AramaSonucu {
        let ad = v
            .get("display_name")
            .and_then(|x| x.as_str())
            .unwrap_or(id)
            .to_string();
        let biyotip = v.get("biotype").and_then(|x| x.as_str());
        let aciklama_metni = v.get("description").and_then(|x| x.as_str());
        let organizma = v.get("species").and_then(|x| x.as_str());

        let mut aciklama = String::new();
        if let Some(b) = biyotip {
            aciklama.push_str(b);
        }
        if let Some(d) = aciklama_metni {
            if !aciklama.is_empty() {
                aciklama.push_str(" · ");
            }
            aciklama.push_str(d);
        }

        let mut s = AramaSonucu::yeni(KAYNAK_ADI, id, ad, KayitTuru::Gen).with_aciklama(aciklama);
        if let Some(o) = organizma {
            s = s.with_organizma(o.replace('_', " "));
        }
        // Koordinat → genom tarayıcı çapraz bağlantısı (1-tabanlı; Ensembl uçları kapsayıcı).
        if let (Some(krom), Some(bas), Some(bit)) = (
            v.get("seq_region_name").and_then(|x| x.as_str()),
            v.get("start").and_then(|x| x.as_u64()),
            v.get("end").and_then(|x| x.as_u64()),
        ) {
            s = s.with_konum(Konum::yeni(krom, bas, bit));
            s = s.with_uzunluk(bit.saturating_sub(bas) + 1);
        }
        s
    }
}

impl Default for EnsemblKonektor {
    fn default() -> Self {
        Self::insan()
    }
}

impl VeritabaniKonektoru for EnsemblKonektor {
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
            Some(format!("{WEB_TABAN}/{}/Gene/Summary?g={k}", self.web_tur()))
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

        // 1) Sembol → Ensembl kimlikleri (gen).
        let istek = HttpIstek::get(format!(
            "{REST_TABAN}/xrefs/symbol/{}/{}",
            self.tur_adi,
            sorgu.metin.trim()
        ))
        .param("content-type", "application/json");
        let govde = self.gonder(&istek, baglam)?;
        let tum_idler = xrefs_ayristir(&govde)?;
        let toplam = tum_idler.len() as u64;

        // 2) Sayfayı dilimle (client-side) + her kimliği lookup ile zenginleştir.
        let dilim: Vec<String> = tum_idler
            .into_iter()
            .skip(sayfalama.ofset as usize)
            .take(sayfalama.limit as usize)
            .collect();
        let sonuclar = dilim
            .into_iter()
            .map(|id| {
                self.lookup(&id, baglam)
                    .unwrap_or_else(|| AramaSonucu::yeni(KAYNAK_ADI, &id, &id, KayitTuru::Gen))
            })
            .collect();

        Ok(SonucListesi {
            sonuclar,
            sayfa: SayfaBilgisi {
                toplam,
                ofset: sayfalama.ofset,
                limit: sayfalama.limit,
            },
        })
    }

    fn detay(&self, kimlik: &str, baglam: &AramaBaglami) -> Result<AramaSonucu, ErrorReport> {
        baglam.gizlilik.dis_gonderim(
            KAYNAK_ADI,
            HEDEF,
            DisVeri::Metin(kimlik),
            HassasiyetEtiketi::Genel,
        )?;
        self.lookup(kimlik, baglam).ok_or_else(|| {
            ErrorReport::new(
                "Ensembl kaydı bulunamadı",
                format!("'{kimlik}' için lookup boş döndü"),
                "Ensembl kimliğini doğrulayın veya yeniden arayın",
            )
        })
    }

    fn getir(&self, kimlik: &str, baglam: &AramaBaglami) -> Result<GetirilenKayit, ErrorReport> {
        baglam.gizlilik.dis_gonderim(
            KAYNAK_ADI,
            HEDEF,
            DisVeri::Metin(kimlik),
            HassasiyetEtiketi::Genel,
        )?;
        let istek = HttpIstek::get(format!("{REST_TABAN}/sequence/id/{kimlik}"))
            .param("content-type", "text/x-fasta");
        let icerik = self.gonder(&istek, baglam)?.into_bytes();

        let provenans = db_provenansi(
            format!("{kimlik}.fasta"),
            format!("Ensembl {} (sequence/id)", self.tur_adi),
            "FASTA",
            &icerik,
            Some(ensembl_lisans_atif()),
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

/// xrefs/symbol yanıtından (gen) Ensembl kimliklerini çıkarır.
fn xrefs_ayristir(govde: &str) -> Result<Vec<String>, ErrorReport> {
    let v: Value = serde_json::from_str(govde).map_err(json_hatasi)?;
    let dizi = v.as_array().ok_or_else(|| {
        ErrorReport::new(
            "Ensembl yanıtı beklenen biçimde değil",
            "xrefs/symbol bir JSON dizisi döndürmedi",
            "Sembolü kontrol edip yeniden deneyin",
        )
    })?;
    Ok(dizi
        .iter()
        .filter(|e| {
            // Yalnız gen tipindeki çapraz referansları al (tipi yoksa yine kabul).
            e.get("type")
                .and_then(|t| t.as_str())
                .map(|t| t == "gene")
                .unwrap_or(true)
        })
        .filter_map(|e| e.get("id").and_then(|i| i.as_str()))
        .map(|s| s.to_string())
        .collect())
}

fn json_hatasi(e: serde_json::Error) -> ErrorReport {
    ErrorReport::new(
        "Ensembl yanıtı çözümlenemedi",
        "Ensembl'den gelen JSON beklenen biçimde değil",
        "Daha sonra yeniden deneyin; sorun sürerse konektör güncellemesi gerekebilir",
    )
    .with_teknik_detay(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::super::super::privacy::GizlilikKapisi;
    use super::super::super::transport::SahteUlastirici;
    use super::*;
    use std::time::Duration;

    fn xrefs_json() -> &'static str {
        r#"[{"id":"ENSG00000141510","type":"gene","display_id":"TP53"}]"#
    }
    fn lookup_json() -> &'static str {
        r#"{"id":"ENSG00000141510","display_name":"TP53","biotype":"protein_coding",
            "seq_region_name":"17","start":7661779,"end":7687550,"strand":-1,
            "description":"tumor protein p53","species":"homo_sapiens","assembly_name":"GRCh38"}"#
    }

    fn baglam_kur<'a>(u: &'a SahteUlastirici, g: &'a GizlilikKapisi) -> AramaBaglami<'a> {
        let mut b = AramaBaglami::yeni(u, g);
        b.yapi.geri_cekilme_taban = Duration::ZERO;
        b
    }

    #[test]
    fn ara_gen_koordinatli_sonuc() {
        let u = SahteUlastirici::yeni()
            .ekle("xrefs/symbol", 200, xrefs_json())
            .ekle("lookup/id/ENSG00000141510", 200, lookup_json());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = EnsemblKonektor::insan();
        let liste = kon
            .ara(&Sorgu::metin("TP53"), Sayfalama::ilk(20), &baglam)
            .unwrap();
        assert_eq!(liste.sayfa.toplam, 1);
        assert_eq!(liste.sonuclar.len(), 1);
        let s = &liste.sonuclar[0];
        assert_eq!(s.kimlik, "ENSG00000141510");
        assert_eq!(s.baslik, "TP53");
        assert_eq!(s.tur, KayitTuru::Gen);
        assert!(s.aciklama.contains("protein_coding"));
        // Koordinat → çapraz bağlantı (genom tarayıcı).
        let k = s.konum.as_ref().unwrap();
        assert_eq!(k.kromozom, "17");
        assert_eq!(k.bolge_metni(), "17:7661779-7687550");
        assert_eq!(s.uzunluk, Some(7687550 - 7661779 + 1));
    }

    #[test]
    fn getir_fasta_ve_provenans() {
        let fasta = ">ENSG00000141510\nACGTACGT\n";
        let u = SahteUlastirici::yeni().ekle("sequence/id/ENSG00000141510", 200, fasta);
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = EnsemblKonektor::insan();
        let kayit = kon.getir("ENSG00000141510", &baglam).unwrap();
        assert_eq!(kayit.format_ipucu, "fasta");
        assert!(kayit.icerik.starts_with(b">ENSG"));
        assert!(kayit.provenans.kaynak.contains("Ensembl"));
        assert!(kayit.provenans.lisans_atif.is_some());
    }

    #[test]
    fn tarayici_url_tur_buyuk_harfli() {
        let kon = EnsemblKonektor::insan();
        assert_eq!(
            kon.tarayici_url("ENSG00000141510").as_deref(),
            Some("https://www.ensembl.org/Homo_sapiens/Gene/Summary?g=ENSG00000141510")
        );
    }

    #[test]
    fn xrefs_yalniz_gen_alir() {
        let js = r#"[{"id":"ENSG1","type":"gene"},{"id":"ENST1","type":"transcript"}]"#;
        let idler = xrefs_ayristir(js).unwrap();
        assert_eq!(idler, vec!["ENSG1".to_string()]);
    }
}
