//! ÇE-09 — **NCBI E-utilities konektörü** (nucleotide / protein / gene).
//!
//! * `esearch` → eşleşen UID listesi + toplam sayı (sayfalama: `retstart`/`retmax`).
//! * `esummary` → her UID için başlık / organizma / uzunluk / accession (önizleme).
//! * `efetch` → ham içerik (FASTA, tek-tık yükleme → projeye ekle, provenance ile).
//!
//! Yanıtlar `retmode=json` ile alınır ve `serde_json` ile ayrıştırılır.  Gerçek istek
//! [`AramaBaglami::ulastirici`] üzerinden gider (bu sürümde dürüst yer-tutucu; test ikiziyle
//! çevrimdışı doğrulanır).  Dış istek ÖNCE gizlilik kapısından geçer (MK-41/42/43).

use biocraft_sdk::biocraft_types::ErrorReport;
use serde_json::Value;

use crate::data_io::tekrar_ile;

use super::super::framework::{
    AramaBaglami, AramaSonucu, GetirilenKayit, KayitTuru, SayfaBilgisi, Sayfalama, SonucListesi,
    Sorgu, VeritabaniKonektoru,
};
use super::super::privacy::{DisVeri, HassasiyetEtiketi};
use super::super::provenance::{db_provenansi, ncbi_lisans_atif};
use super::super::transport::HttpIstek;

/// E-utilities taban URL'i.
pub const EUTILS_TABAN: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils";
/// NCBI web (tarayıcıda aç) taban URL'i.
pub const NCBI_WEB_TABAN: &str = "https://www.ncbi.nlm.nih.gov";
/// E-utilities `tool` parametresi (NCBI kimlik önerisi).
pub const ARAC_ADI: &str = "BioCraftEngine";
/// Gizlilik özetinde gösterilen hedef.
const HEDEF: &str = "NCBI E-utilities (eutils.ncbi.nlm.nih.gov)";

/// NCBI'de aranabilen veritabanı (bu konektörün kapsamı).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NcbiVeritabani {
    /// Nükleotid dizileri (nuccore).
    Nukleotid,
    /// Protein dizileri.
    Protein,
    /// Gen kayıtları.
    Gen,
}

impl NcbiVeritabani {
    /// E-utilities `db` parametresi.
    pub fn eutils_db(&self) -> &'static str {
        match self {
            NcbiVeritabani::Nukleotid => "nucleotide",
            NcbiVeritabani::Protein => "protein",
            NcbiVeritabani::Gen => "gene",
        }
    }
    /// Web (tarayıcıda aç) yol bileşeni.
    pub fn web_yolu(&self) -> &'static str {
        match self {
            NcbiVeritabani::Nukleotid => "nuccore",
            NcbiVeritabani::Protein => "protein",
            NcbiVeritabani::Gen => "gene",
        }
    }
    /// Karşılık gelen birleşik kayıt türü.
    pub fn kayit_turu(&self) -> KayitTuru {
        match self {
            NcbiVeritabani::Nukleotid => KayitTuru::Nukleotid,
            NcbiVeritabani::Protein => KayitTuru::Protein,
            NcbiVeritabani::Gen => KayitTuru::Gen,
        }
    }
    /// Rozet/kaynak adı.
    pub fn kaynak_adi(&self) -> &'static str {
        match self {
            NcbiVeritabani::Nukleotid => "NCBI nucleotide",
            NcbiVeritabani::Protein => "NCBI protein",
            NcbiVeritabani::Gen => "NCBI gene",
        }
    }
}

/// NCBI E-utilities konektörü (tek bir veritabanına bağlı).
pub struct NcbiKonektor {
    veritabani: NcbiVeritabani,
    kaynak_adi: String,
    turler: [KayitTuru; 1],
}

impl NcbiKonektor {
    /// Belirli bir NCBI veritabanı için konektör.
    pub fn yeni(veritabani: NcbiVeritabani) -> Self {
        Self {
            veritabani,
            kaynak_adi: veritabani.kaynak_adi().to_string(),
            turler: [veritabani.kayit_turu()],
        }
    }
    /// nucleotide konektörü.
    pub fn nukleotid() -> Self {
        Self::yeni(NcbiVeritabani::Nukleotid)
    }
    /// protein konektörü.
    pub fn protein() -> Self {
        Self::yeni(NcbiVeritabani::Protein)
    }
    /// gene konektörü.
    pub fn gen() -> Self {
        Self::yeni(NcbiVeritabani::Gen)
    }

    /// Ortak querystring (db + retmode=json + tool + opsiyonel api_key).
    fn temel_istek(&self, fcgi: &str, baglam: &AramaBaglami) -> HttpIstek {
        let mut istek = HttpIstek::get(format!("{EUTILS_TABAN}/{fcgi}"))
            .param("db", self.veritabani.eutils_db())
            .param("retmode", "json")
            .param("tool", ARAC_ADI);
        if let Some(anahtar) = &baglam.api_anahtari {
            istek = istek.param("api_key", anahtar.clone());
        }
        istek
    }

    /// İsteği gönderir (tekrar/geri-çekilme ile) → başarılı gövde.  HTTP hata durumu → tekrar tetikler.
    fn gonder(&self, istek: &HttpIstek, baglam: &AramaBaglami) -> Result<String, ErrorReport> {
        baglam.hiz_bekle();
        let yanit = tekrar_ile(&baglam.yapi, |_| {
            let y = baglam.ulastirici.gonder(istek)?;
            y.metin()?; // 2xx değilse Err → yeniden dene (rate-limit/sunucu hatası)
            Ok(y)
        })?;
        Ok(yanit.govde)
    }
}

impl VeritabaniKonektoru for NcbiKonektor {
    fn kaynak_adi(&self) -> &str {
        &self.kaynak_adi
    }

    fn turler(&self) -> &[KayitTuru] {
        &self.turler
    }

    fn tarayici_url(&self, kimlik: &str) -> Option<String> {
        if kimlik.trim().is_empty() {
            return None;
        }
        Some(format!(
            "{NCBI_WEB_TABAN}/{}/{}",
            self.veritabani.web_yolu(),
            kimlik.trim()
        ))
    }

    fn ara(
        &self,
        sorgu: &Sorgu,
        sayfalama: Sayfalama,
        baglam: &AramaBaglami,
    ) -> Result<SonucListesi, ErrorReport> {
        // Dış gönderim denetimi (onay + PHI; arama metni kamuya açık = Genel).
        baglam.gizlilik.dis_gonderim(
            &self.kaynak_adi,
            HEDEF,
            DisVeri::Metin(&sorgu.metin),
            HassasiyetEtiketi::Genel,
        )?;

        // 1) esearch → toplam + UID listesi.
        let esearch = self
            .temel_istek("esearch.fcgi", baglam)
            .param("term", &sorgu.metin)
            .param("retmax", sayfalama.limit.to_string())
            .param("retstart", sayfalama.ofset.to_string());
        let govde = self.gonder(&esearch, baglam)?;
        let (toplam, idler) = esearch_ayristir(&govde)?;

        let sayfa = SayfaBilgisi {
            toplam,
            ofset: sayfalama.ofset,
            limit: sayfalama.limit,
        };
        if idler.is_empty() {
            return Ok(SonucListesi {
                sonuclar: Vec::new(),
                sayfa,
            });
        }

        // 2) esummary → her UID için özet.
        let esummary = self
            .temel_istek("esummary.fcgi", baglam)
            .param("id", idler.join(","));
        let ozet_govde = self.gonder(&esummary, baglam)?;
        let sonuclar = self.esummary_satirlar(&ozet_govde)?;

        Ok(SonucListesi { sonuclar, sayfa })
    }

    fn detay(&self, kimlik: &str, baglam: &AramaBaglami) -> Result<AramaSonucu, ErrorReport> {
        let esummary = self
            .temel_istek("esummary.fcgi", baglam)
            .param("id", kimlik);
        let govde = self.gonder(&esummary, baglam)?;
        self.esummary_satirlar(&govde)?
            .into_iter()
            .next()
            .ok_or_else(|| {
                ErrorReport::new(
                    "Kayıt bulunamadı",
                    format!("'{kimlik}' için NCBI özeti boş döndü"),
                    "Kimliği (UID) doğrulayın veya yeniden arayın",
                )
            })
    }

    fn getir(&self, kimlik: &str, baglam: &AramaBaglami) -> Result<GetirilenKayit, ErrorReport> {
        if matches!(self.veritabani, NcbiVeritabani::Gen) {
            return Err(ErrorReport::new(
                "Gen kaydı doğrudan dizi olarak indirilemez",
                "NCBI gene kaydı bir FASTA dizisi değildir (lokus/öznitelik kaydıdır)",
                "Bağlı nükleotid/protein kaydını açın ya da 'tarayıcıda aç' ile inceleyin",
            )
            .with_eylem("Tarayıcıda aç"));
        }

        // efetch'e gönderilen sadece accession/UID'dir (kamuya açık = Genel).
        baglam.gizlilik.dis_gonderim(
            &self.kaynak_adi,
            HEDEF,
            DisVeri::Metin(kimlik),
            HassasiyetEtiketi::Genel,
        )?;

        let efetch = self
            .temel_istek("efetch.fcgi", baglam)
            .param("id", kimlik)
            .param("rettype", "fasta")
            .param("retmode", "text");
        let icerik = self.gonder(&efetch, baglam)?.into_bytes();

        let provenans = db_provenansi(
            format!("{kimlik}.fasta"),
            format!("{} (efetch)", self.kaynak_adi),
            "FASTA",
            &icerik,
            Some(ncbi_lisans_atif()),
        );
        Ok(GetirilenKayit {
            kimlik: kimlik.to_string(),
            format_ipucu: "fasta".to_string(),
            icerik,
            provenans,
        })
    }
}

impl NcbiKonektor {
    /// esummary JSON'unu birleşik sonuç satırlarına çevirir.
    fn esummary_satirlar(&self, govde: &str) -> Result<Vec<AramaSonucu>, ErrorReport> {
        let v: Value = serde_json::from_str(govde).map_err(json_hatasi)?;
        let result = v.get("result").ok_or_else(|| sema_hatasi("result"))?;
        let uids = result
            .get("uids")
            .and_then(|x| x.as_array())
            .ok_or_else(|| sema_hatasi("result.uids"))?;

        let mut satirlar = Vec::with_capacity(uids.len());
        for uid_v in uids {
            let Some(uid) = deger_str(Some(uid_v)) else {
                continue;
            };
            let Some(obj) = result.get(&uid) else {
                continue;
            };
            satirlar.push(self.satir_olustur(&uid, obj));
        }
        Ok(satirlar)
    }

    /// Tek bir esummary nesnesinden birleşik sonuç satırı kurar.
    fn satir_olustur(&self, uid: &str, obj: &Value) -> AramaSonucu {
        let baslik = deger_str(obj.get("title"))
            .or_else(|| {
                let ad = deger_str(obj.get("name"));
                let aciklama = deger_str(obj.get("description"));
                match (ad, aciklama) {
                    (Some(a), Some(d)) if !d.is_empty() => Some(format!("{a} — {d}")),
                    (Some(a), _) => Some(a),
                    (_, Some(d)) => Some(d),
                    _ => None,
                }
            })
            .unwrap_or_default();

        let accession = deger_str(obj.get("caption")).filter(|s| !s.is_empty());
        let organizma = deger_organizma(obj.get("organism"));
        let uzunluk = deger_u64(obj.get("slen"));

        let mut s = AramaSonucu::yeni(&self.kaynak_adi, uid, baslik, self.veritabani.kayit_turu());
        if let Some(o) = organizma {
            s = s.with_organizma(o);
        }
        if let Some(u) = uzunluk {
            s = s.with_uzunluk(u);
        }
        if let Some(acc) = accession {
            s = s.with_aciklama(format!("Erişim: {acc}"));
        }
        s
    }
}

// ─── JSON ayrıştırma yardımcıları ────────────────────────────────────────────────

/// esearch JSON'undan (toplam, UID listesi) çıkarır.  `ERROR` alanı varsa hata döndürür.
fn esearch_ayristir(govde: &str) -> Result<(u64, Vec<String>), ErrorReport> {
    let v: Value = serde_json::from_str(govde).map_err(json_hatasi)?;
    let er = v
        .get("esearchresult")
        .ok_or_else(|| sema_hatasi("esearchresult"))?;

    if let Some(mesaj) = deger_str(er.get("ERROR")) {
        return Err(ErrorReport::new(
            "NCBI arama hatası",
            mesaj,
            "Sorgu ifadesini kontrol edip yeniden deneyin",
        ));
    }

    let toplam = deger_u64(er.get("count")).unwrap_or(0);
    let idler = er
        .get("idlist")
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|x| deger_str(Some(x))).collect())
        .unwrap_or_default();
    Ok((toplam, idler))
}

/// Bir JSON değerini (String veya Number) dizgeye çevirir.
fn deger_str(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Bir JSON değerini u64'e çevirir (Number veya sayısal String).
fn deger_u64(v: Option<&Value>) -> Option<u64> {
    match v? {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().parse::<u64>().ok(),
        _ => None,
    }
}

/// Organizma alanını çözer: ya düz String ya da `{scientificname: ...}` nesnesi (gene db).
fn deger_organizma(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Object(_) => deger_str(v?.get("scientificname")),
        _ => None,
    }
}

fn json_hatasi(e: serde_json::Error) -> ErrorReport {
    ErrorReport::new(
        "NCBI yanıtı çözümlenemedi",
        "sunucudan gelen JSON beklenen biçimde değil",
        "Daha sonra yeniden deneyin; sorun sürerse konektör güncellemesi gerekebilir",
    )
    .with_teknik_detay(e.to_string())
}

fn sema_hatasi(alan: &str) -> ErrorReport {
    ErrorReport::new(
        "NCBI yanıtı beklenen alanı içermiyor",
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

    fn esearch_json() -> &'static str {
        r#"{"header":{},"esearchresult":{"count":"2","retmax":"2","retstart":"0",
            "idlist":["7157","672"]}}"#
    }
    fn esummary_json() -> &'static str {
        r#"{"header":{},"result":{"uids":["7157","672"],
            "7157":{"uid":"7157","caption":"NM_000546","title":"Homo sapiens tumor protein p53 (TP53), mRNA",
                    "slen":2591,"organism":"Homo sapiens"},
            "672":{"uid":"672","caption":"NM_007294","title":"Homo sapiens BRCA1 DNA repair associated (BRCA1), mRNA",
                    "slen":7088,"organism":"Homo sapiens"}}}"#
    }

    fn baglam_kur<'a>(u: &'a SahteUlastirici, g: &'a GizlilikKapisi) -> AramaBaglami<'a> {
        let mut b = AramaBaglami::yeni(u, g);
        // Testte tekrar gecikmesi olmasın.
        b.yapi.geri_cekilme_taban = std::time::Duration::ZERO;
        b
    }

    #[test]
    fn esearch_ayristirma_toplam_ve_idler() {
        let (toplam, idler) = esearch_ayristir(esearch_json()).unwrap();
        assert_eq!(toplam, 2);
        assert_eq!(idler, vec!["7157".to_string(), "672".to_string()]);
    }

    #[test]
    fn esearch_error_alani_hata_dondurur() {
        let js = r#"{"esearchresult":{"ERROR":"Invalid term"}}"#;
        assert!(esearch_ayristir(js).is_err());
    }

    #[test]
    fn ara_birlesik_sonuc_dondurur() {
        let u = SahteUlastirici::yeni()
            .ekle("esearch.fcgi", 200, esearch_json())
            .ekle("esummary.fcgi", 200, esummary_json());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = NcbiKonektor::nukleotid();

        let liste = kon
            .ara(&Sorgu::metin("p53"), Sayfalama::ilk(20), &baglam)
            .unwrap();
        assert_eq!(liste.sayfa.toplam, 2);
        assert_eq!(liste.sonuclar.len(), 2);
        let s0 = &liste.sonuclar[0];
        assert_eq!(s0.kaynak, "NCBI nucleotide");
        assert_eq!(s0.kimlik, "7157");
        assert!(s0.baslik.contains("TP53"));
        assert_eq!(s0.organizma.as_deref(), Some("Homo sapiens"));
        assert_eq!(s0.uzunluk, Some(2591));
        assert!(s0.aciklama.contains("NM_000546"));
    }

    #[test]
    fn onaysiz_baglam_disari_gondermez() {
        let u = SahteUlastirici::yeni().ekle("esearch.fcgi", 200, esearch_json());
        let g = GizlilikKapisi::yeni(); // onaylanmadı
        let baglam = baglam_kur(&u, &g);
        let kon = NcbiKonektor::nukleotid();
        let hata = kon
            .ara(&Sorgu::metin("p53"), Sayfalama::ilk(20), &baglam)
            .err()
            .unwrap();
        assert_eq!(hata.ne_oldu, "Dış sorgu onayı gerekli");
    }

    #[test]
    fn getir_fasta_ve_provenans_uretir() {
        let fasta = ">NM_000546 Homo sapiens TP53\nACGTACGTACGT\n";
        let u = SahteUlastirici::yeni().ekle("efetch.fcgi", 200, fasta);
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = NcbiKonektor::nukleotid();

        let kayit = kon.getir("7157", &baglam).unwrap();
        assert_eq!(kayit.format_ipucu, "fasta");
        assert_eq!(kayit.icerik, fasta.as_bytes());
        assert!(kayit.provenans.kaynak.contains("NCBI nucleotide"));
        assert!(kayit.provenans.lisans_atif.is_some());
        assert_eq!(kayit.provenans.blake3.len(), 64);
    }

    #[test]
    fn gen_getir_durust_reddeder() {
        let u = SahteUlastirici::yeni();
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = NcbiKonektor::gen();
        let hata = kon.getir("672", &baglam).err().unwrap();
        assert!(hata.ne_oldu.contains("doğrudan dizi olarak indirilemez"));
    }

    #[test]
    fn gene_db_organizma_nesnesini_cozer() {
        let kon = NcbiKonektor::gen();
        let obj: Value = serde_json::from_str(
            r#"{"uid":"672","name":"BRCA1","description":"BRCA1 DNA repair associated",
                "organism":{"scientificname":"Homo sapiens","taxid":9606}}"#,
        )
        .unwrap();
        let s = kon.satir_olustur("672", &obj);
        assert!(s.baslik.contains("BRCA1"));
        assert_eq!(s.organizma.as_deref(), Some("Homo sapiens"));
        assert_eq!(s.tur, KayitTuru::Gen);
    }

    #[test]
    fn tarayici_url_uretir() {
        let kon = NcbiKonektor::protein();
        assert_eq!(
            kon.tarayici_url("NP_000537").as_deref(),
            Some("https://www.ncbi.nlm.nih.gov/protein/NP_000537")
        );
        assert!(kon.tarayici_url("  ").is_none());
    }
}
