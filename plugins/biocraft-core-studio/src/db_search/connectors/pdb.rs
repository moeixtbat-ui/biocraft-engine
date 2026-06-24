//! ÇE-09 (Gün 41) — **RCSB PDB konektörü** (3B makromolekül yapıları → ÇE-07 görüntüleyici).
//!
//! * **Arama:** RCSB Search API (`search.rcsb.org`) tam-metin sorgusu → eşleşen PDB kimlikleri +
//!   toplam sayı (sayfalama: `paginate.start/rows`).
//! * **Özet/önizleme:** RCSB Data API (`data.rcsb.org/.../core/entry/{id}`) → başlık / yöntem /
//!   çözünürlük (her kimlik için; bir özet alınamazsa o satır temel hâlde kalır — federe dayanıklılık).
//! * **Getir (tek-tık):** `files.rcsb.org/download/{id}.pdb` → ham PDB metni → [`KayitTuru::Yapi`]
//!   olduğundan panel **"yapıya bak"** ile doğrudan ÇE-07 3B görüntüleyiciye yükler.
//!
//! PDB ana arşiv verisi kamuya açıktır (CC0); yine de dış sorgu önce gizlilik kapısından geçer
//! (MK-41; PDB araması kamuya açık anahtar kelime = `Genel`).

use biocraft_sdk::biocraft_types::ErrorReport;
use serde_json::{json, Value};

use crate::data_io::tekrar_ile;

use super::super::framework::{
    AramaBaglami, AramaSonucu, GetirilenKayit, KayitTuru, SayfaBilgisi, Sayfalama, SonucListesi,
    Sorgu, VeritabaniKonektoru,
};
use super::super::privacy::{DisVeri, HassasiyetEtiketi};
use super::super::provenance::{db_provenansi, pdb_lisans_atif};
use super::super::transport::HttpIstek;

/// RCSB Search API uç noktası (tam-metin sorgu; POST JSON).
pub const SEARCH_URL: &str = "https://search.rcsb.org/rcsbsearch/v2/query";
/// RCSB Data API taban (giriş özeti).
pub const DATA_TABAN: &str = "https://data.rcsb.org/rest/v1/core/entry";
/// Yapı dosyası indirme tabanı.
pub const FILES_TABAN: &str = "https://files.rcsb.org/download";
/// RCSB web (tarayıcıda aç).
pub const WEB_TABAN: &str = "https://www.rcsb.org/structure";
const KAYNAK_ADI: &str = "PDB";
const HEDEF: &str = "RCSB PDB (search.rcsb.org / data.rcsb.org)";

/// RCSB PDB konektörü (yapı arama + indirme).
pub struct PdbKonektor {
    turler: [KayitTuru; 1],
}

impl PdbKonektor {
    /// Yeni PDB konektörü.
    pub fn yeni() -> Self {
        Self {
            turler: [KayitTuru::Yapi],
        }
    }

    /// İsteği gönderir (kaynak-başına hız + tekrar/geri-çekilme + HTTP durum denetimi) → gövde.
    fn gonder(&self, istek: &HttpIstek, baglam: &AramaBaglami) -> Result<String, ErrorReport> {
        baglam.hiz_bekle_kaynak(KAYNAK_ADI);
        let yanit = tekrar_ile(&baglam.yapi, |_| {
            let y = baglam.ulastirici.gonder(istek)?;
            y.metin()?;
            Ok(y)
        })?;
        Ok(yanit.govde)
    }

    /// Bir PDB girişi için özet (başlık/yöntem/çözünürlük) — Data API.  Hata olursa `None`
    /// (çağıran temel satırla devam eder; tek bir kayıt yüzünden arama düşmez).
    fn ozet(&self, id: &str, baglam: &AramaBaglami) -> Option<AramaSonucu> {
        let istek = HttpIstek::get(format!("{DATA_TABAN}/{id}"));
        let govde = self.gonder(&istek, baglam).ok()?;
        let v: Value = serde_json::from_str(&govde).ok()?;
        Some(satir_olustur(id, &v))
    }
}

impl Default for PdbKonektor {
    fn default() -> Self {
        Self::yeni()
    }
}

impl VeritabaniKonektoru for PdbKonektor {
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
            Some(format!("{WEB_TABAN}/{}", k.to_ascii_uppercase()))
        }
    }

    fn ara(
        &self,
        sorgu: &Sorgu,
        sayfalama: Sayfalama,
        baglam: &AramaBaglami,
    ) -> Result<SonucListesi, ErrorReport> {
        // Dış gönderim denetimi (anahtar kelime = kamuya açık).
        baglam.gizlilik.dis_gonderim(
            KAYNAK_ADI,
            HEDEF,
            DisVeri::Metin(&sorgu.metin),
            HassasiyetEtiketi::Genel,
        )?;

        // 1) Arama (POST JSON) → kimlikler + toplam.
        let govde = json!({
            "query": {
                "type": "terminal",
                "service": "full_text",
                "parameters": { "value": sorgu.metin }
            },
            "return_type": "entry",
            "request_options": {
                "paginate": { "start": sayfalama.ofset, "rows": sayfalama.limit }
            }
        })
        .to_string();
        let istek = HttpIstek::post(SEARCH_URL).with_govde(govde);
        let yanit = self.gonder(&istek, baglam)?;
        let (toplam, idler) = arama_ayristir(&yanit)?;

        let sayfa = SayfaBilgisi {
            toplam,
            ofset: sayfalama.ofset,
            limit: sayfalama.limit,
        };

        // 2) Her kimlik için özet (federe dayanıklılık: özet gelmezse temel satır).
        let sonuclar = idler
            .into_iter()
            .map(|id| {
                self.ozet(&id, baglam)
                    .unwrap_or_else(|| AramaSonucu::yeni(KAYNAK_ADI, &id, &id, KayitTuru::Yapi))
            })
            .collect();

        Ok(SonucListesi { sonuclar, sayfa })
    }

    fn detay(&self, kimlik: &str, baglam: &AramaBaglami) -> Result<AramaSonucu, ErrorReport> {
        baglam.gizlilik.dis_gonderim(
            KAYNAK_ADI,
            HEDEF,
            DisVeri::Metin(kimlik),
            HassasiyetEtiketi::Genel,
        )?;
        let istek = HttpIstek::get(format!("{DATA_TABAN}/{kimlik}"));
        let govde = self.gonder(&istek, baglam)?;
        let v: Value = serde_json::from_str(&govde).map_err(json_hatasi)?;
        Ok(satir_olustur(kimlik, &v))
    }

    fn getir(&self, kimlik: &str, baglam: &AramaBaglami) -> Result<GetirilenKayit, ErrorReport> {
        baglam.gizlilik.dis_gonderim(
            KAYNAK_ADI,
            HEDEF,
            DisVeri::Metin(kimlik),
            HassasiyetEtiketi::Genel,
        )?;
        let id = kimlik.trim().to_ascii_uppercase();
        let istek = HttpIstek::get(format!("{FILES_TABAN}/{id}.pdb"));
        let icerik = self.gonder(&istek, baglam)?.into_bytes();

        let provenans = db_provenansi(
            format!("{id}.pdb"),
            "RCSB PDB (files.rcsb.org)",
            "PDB",
            &icerik,
            Some(pdb_lisans_atif()),
        );
        Ok(GetirilenKayit {
            kimlik: id,
            format_ipucu: "pdb".to_string(),
            icerik,
            provenans,
        })
    }
}

// ─── JSON ayrıştırma ─────────────────────────────────────────────────────────────

/// RCSB search yanıtından (toplam, kimlik listesi) çıkarır.
fn arama_ayristir(govde: &str) -> Result<(u64, Vec<String>), ErrorReport> {
    let v: Value = serde_json::from_str(govde).map_err(json_hatasi)?;
    // Boş sonuç: RCSB 204 yerine bazen boş gövde döndürebilir → 0 sonuç.
    if govde.trim().is_empty() {
        return Ok((0, Vec::new()));
    }
    let toplam = v.get("total_count").and_then(|x| x.as_u64()).unwrap_or(0);
    let idler = v
        .get("result_set")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|e| e.get("identifier").and_then(|i| i.as_str()))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();
    Ok((toplam, idler))
}

/// Bir Data API giriş JSON'undan birleşik sonuç satırı kurar.
fn satir_olustur(id: &str, v: &Value) -> AramaSonucu {
    let baslik = v
        .get("struct")
        .and_then(|s| s.get("title"))
        .and_then(|t| t.as_str())
        .unwrap_or(id)
        .to_string();

    let yontem = v
        .get("exptl")
        .and_then(|e| e.as_array())
        .and_then(|a| a.first())
        .and_then(|m| m.get("method"))
        .and_then(|x| x.as_str());

    let cozunurluk = v
        .get("rcsb_entry_info")
        .and_then(|i| i.get("resolution_combined"))
        .and_then(|r| r.as_array())
        .and_then(|a| a.first())
        .and_then(|x| x.as_f64());

    let uzunluk = v
        .get("rcsb_entry_info")
        .and_then(|i| i.get("deposited_polymer_monomer_count"))
        .and_then(|x| x.as_u64());

    let mut aciklama = String::new();
    if let Some(y) = yontem {
        aciklama.push_str(y);
    }
    if let Some(c) = cozunurluk {
        if !aciklama.is_empty() {
            aciklama.push_str(" · ");
        }
        aciklama.push_str(&format!("{c:.2} Å"));
    }

    let mut s = AramaSonucu::yeni(KAYNAK_ADI, id.to_ascii_uppercase(), baslik, KayitTuru::Yapi)
        .with_aciklama(aciklama);
    if let Some(u) = uzunluk {
        s = s.with_uzunluk(u);
    }
    s
}

fn json_hatasi(e: serde_json::Error) -> ErrorReport {
    ErrorReport::new(
        "PDB yanıtı çözümlenemedi",
        "RCSB'den gelen JSON beklenen biçimde değil",
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

    fn search_json() -> &'static str {
        r#"{"query_id":"x","result_type":"entry","total_count":2,
            "result_set":[{"identifier":"1TUP","score":1.0},{"identifier":"4HHB","score":0.8}]}"#
    }
    fn entry_1tup() -> &'static str {
        r#"{"struct":{"title":"TUMOR SUPPRESSOR P53 COMPLEXED WITH DNA"},
            "exptl":[{"method":"X-RAY DIFFRACTION"}],
            "rcsb_entry_info":{"resolution_combined":[2.2],"deposited_polymer_monomer_count":393}}"#
    }
    fn entry_4hhb() -> &'static str {
        r#"{"struct":{"title":"THE CRYSTAL STRUCTURE OF HUMAN DEOXYHAEMOGLOBIN"},
            "exptl":[{"method":"X-RAY DIFFRACTION"}],
            "rcsb_entry_info":{"resolution_combined":[1.74],"deposited_polymer_monomer_count":574}}"#
    }

    fn baglam_kur<'a>(u: &'a SahteUlastirici, g: &'a GizlilikKapisi) -> AramaBaglami<'a> {
        let mut b = AramaBaglami::yeni(u, g);
        b.yapi.geri_cekilme_taban = Duration::ZERO;
        b
    }

    #[test]
    fn arama_ayristirma_toplam_ve_idler() {
        let (toplam, idler) = arama_ayristir(search_json()).unwrap();
        assert_eq!(toplam, 2);
        assert_eq!(idler, vec!["1TUP".to_string(), "4HHB".to_string()]);
    }

    #[test]
    fn ara_yapilari_ozetle_dondurur() {
        let u = SahteUlastirici::yeni()
            .ekle("rcsbsearch", 200, search_json())
            .ekle("core/entry/1TUP", 200, entry_1tup())
            .ekle("core/entry/4HHB", 200, entry_4hhb());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = PdbKonektor::yeni();

        let liste = kon
            .ara(&Sorgu::metin("p53"), Sayfalama::ilk(20), &baglam)
            .unwrap();
        assert_eq!(liste.sayfa.toplam, 2);
        assert_eq!(liste.sonuclar.len(), 2);
        let s0 = &liste.sonuclar[0];
        assert_eq!(s0.kimlik, "1TUP");
        assert_eq!(s0.tur, KayitTuru::Yapi);
        assert!(s0.baslik.contains("P53"));
        assert!(s0.aciklama.contains("X-RAY"));
        assert!(s0.aciklama.contains("2.20 Å"));
        assert_eq!(s0.uzunluk, Some(393));
        // Yapı → "yapıya bakılabilir" (ÇE-07 tek-tık).
        assert!(s0.tur.yapiya_bakilabilir_mi());
    }

    #[test]
    fn ozet_gelmezse_temel_satir() {
        // Arama döner ama giriş özeti kayıtlı değil → temel satır (arama düşmez).
        let u = SahteUlastirici::yeni().ekle("rcsbsearch", 200, search_json());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = PdbKonektor::yeni();
        let liste = kon
            .ara(&Sorgu::metin("p53"), Sayfalama::ilk(20), &baglam)
            .unwrap();
        assert_eq!(liste.sonuclar.len(), 2);
        // Özet yok → başlık = kimlik.
        assert_eq!(liste.sonuclar[0].baslik, "1TUP");
    }

    #[test]
    fn getir_pdb_dosyasi_ve_provenans() {
        let pdb = "HEADER    TUMOR SUPPRESSOR\nATOM      1  N   ...\nEND\n";
        let u = SahteUlastirici::yeni().ekle("1TUP.pdb", 200, pdb);
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = PdbKonektor::yeni();
        let kayit = kon.getir("1tup", &baglam).unwrap();
        assert_eq!(kayit.kimlik, "1TUP");
        assert_eq!(kayit.format_ipucu, "pdb");
        assert!(kayit.icerik.starts_with(b"HEADER"));
        assert!(kayit
            .provenans
            .lisans_atif
            .as_ref()
            .unwrap()
            .lisans
            .contains("CC0"));
        assert_eq!(kayit.provenans.blake3.len(), 64);
    }

    #[test]
    fn tarayici_url_buyuk_harfli() {
        let kon = PdbKonektor::yeni();
        assert_eq!(
            kon.tarayici_url("1tup").as_deref(),
            Some("https://www.rcsb.org/structure/1TUP")
        );
        assert!(kon.tarayici_url("  ").is_none());
    }

    #[test]
    fn onaysiz_baglam_reddeder() {
        let u = SahteUlastirici::yeni().ekle("rcsbsearch", 200, search_json());
        let g = GizlilikKapisi::yeni();
        let baglam = baglam_kur(&u, &g);
        let hata = PdbKonektor::yeni()
            .ara(&Sorgu::metin("p53"), Sayfalama::ilk(20), &baglam)
            .err()
            .unwrap();
        assert_eq!(hata.ne_oldu, "Dış sorgu onayı gerekli");
    }
}
