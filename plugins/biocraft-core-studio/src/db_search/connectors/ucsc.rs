//! ÇE-09 (Gün 41) — **UCSC Genom Tarayıcı konektörü** (genom derlemesi + iz/track).
//!
//! UCSC API (`api.genome.ucsc.edu`) tam-metin arama değil, **derleme + iz + bölge** verir:
//! * **Arama:** `/list/tracks?genome={derleme}` → derlemenin izlerini listeler; sorgu, iz adı /
//!   kısa / uzun etiketinde **anahtar kelime süzgeci** olarak uygulanır → eşleşen izler
//!   ([`KayitTuru::Iz`]).
//! * **Bölgesel veri:** [`track_verisi`](UcscKonektor::track_verisi) → `/getData/track` ile bir
//!   **bölge** için iz verisini (JSON) çeker → genom tarayıcıya (ÇE-02) yüklenir / projeye eklenir.
//! * **Tek-tık getir:** Bir iz tek başına bir dosya değildir (bölge gerekir) → [`getir`] dürüstçe
//!   reddeder ve genom tarayıcı/bölge seçimine yönlendirir (NCBI gene deseni).
//!
//! Çapraz bağlantı: iz "tarayıcıda aç" ile UCSC'de açılır; bölgeli veri genom tarayıcıya gider.

use biocraft_sdk::biocraft_types::ErrorReport;
use serde_json::Value;

use crate::data_io::tekrar_ile;

use super::super::framework::{
    AramaBaglami, AramaSonucu, GetirilenKayit, KayitTuru, Konum, SayfaBilgisi, Sayfalama,
    SonucListesi, Sorgu, VeritabaniKonektoru,
};
use super::super::privacy::{DisVeri, HassasiyetEtiketi};
use super::super::provenance::{db_provenansi, ucsc_lisans_atif};
use super::super::transport::HttpIstek;

/// UCSC API tabanı.
pub const API_TABAN: &str = "https://api.genome.ucsc.edu";
/// UCSC web (iz arayüzü).
pub const WEB_TABAN: &str = "https://genome.ucsc.edu/cgi-bin/hgTrackUi";
const KAYNAK_ADI: &str = "UCSC";
const HEDEF: &str = "UCSC Genome Browser API (api.genome.ucsc.edu)";

/// UCSC konektörü (tek bir genom derlemesine bağlı).
pub struct UcscKonektor {
    genom: String,
    turler: [KayitTuru; 1],
}

impl UcscKonektor {
    /// Belirli bir derleme için konektör (örn. `hg38`).
    pub fn yeni(genom: impl Into<String>) -> Self {
        Self {
            genom: genom.into(),
            turler: [KayitTuru::Iz],
        }
    }

    /// İnsan GRCh38 (`hg38`) — varsayılan.
    pub fn hg38() -> Self {
        Self::yeni("hg38")
    }

    /// İnsan GRCh37 (`hg19`).
    pub fn hg19() -> Self {
        Self::yeni("hg19")
    }

    /// Bu konektörün derleme adı.
    pub fn genom(&self) -> &str {
        &self.genom
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

    /// Bir **bölge** için iz verisini (JSON) çeker → genom tarayıcıya/projeye (provenance ile).
    /// `konum` 1-tabanlı kapsayıcı; UCSC 0-tabanlı yarı-açık olduğundan `start=baslangic-1`.
    pub fn track_verisi(
        &self,
        track: &str,
        konum: &Konum,
        baglam: &AramaBaglami,
    ) -> Result<GetirilenKayit, ErrorReport> {
        baglam.gizlilik.dis_gonderim(
            KAYNAK_ADI,
            HEDEF,
            DisVeri::Metin(&format!("{track} @ {}", konum.bolge_metni())),
            HassasiyetEtiketi::Genel,
        )?;
        let krom = ucsc_krom(&konum.kromozom);
        let start0 = konum.baslangic.saturating_sub(1);
        let istek = HttpIstek::get(format!("{API_TABAN}/getData/track"))
            .param("genome", &self.genom)
            .param("track", track)
            .param("chrom", &krom)
            .param("start", start0.to_string())
            .param("end", konum.bitis.to_string());
        let icerik = self.gonder(&istek, baglam)?.into_bytes();

        let provenans = db_provenansi(
            format!("{}_{}_{}.json", self.genom, track, konum.bolge_metni()),
            format!("UCSC {} ({} track, getData)", self.genom, track),
            "JSON",
            &icerik,
            Some(ucsc_lisans_atif()),
        );
        Ok(GetirilenKayit {
            kimlik: track.to_string(),
            format_ipucu: "json".to_string(),
            icerik,
            provenans,
        })
    }
}

impl Default for UcscKonektor {
    fn default() -> Self {
        Self::hg38()
    }
}

impl VeritabaniKonektoru for UcscKonektor {
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
            Some(format!("{WEB_TABAN}?db={}&g={k}", self.genom))
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

        let istek = HttpIstek::get(format!("{API_TABAN}/list/tracks")).param("genome", &self.genom);
        let govde = self.gonder(&istek, baglam)?;
        let mut izler = izleri_ayristir(&govde, &self.genom)?;

        // Anahtar kelime süzgeci (boş sorgu → hepsi; ad/kısa/uzun etiket + iz tipi).
        let q = sorgu.metin.trim().to_ascii_lowercase();
        if !q.is_empty() {
            izler.retain(|iz| {
                iz.ad.to_ascii_lowercase().contains(&q)
                    || iz.kisa.to_ascii_lowercase().contains(&q)
                    || iz.uzun.to_ascii_lowercase().contains(&q)
                    || iz.tip.to_ascii_lowercase().contains(&q)
            });
        }
        let toplam = izler.len() as u64;

        let sonuclar = izler
            .into_iter()
            .skip(sayfalama.ofset as usize)
            .take(sayfalama.limit as usize)
            .map(|iz| iz.sonuc())
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
        // İz listesinden ilgili izi bulup özet döndür.
        let liste = self.ara(&Sorgu::metin(kimlik), Sayfalama::ilk(50), baglam)?;
        liste
            .sonuclar
            .into_iter()
            .find(|s| s.kimlik == kimlik)
            .ok_or_else(|| {
                ErrorReport::new(
                    "UCSC izi bulunamadı",
                    format!("'{kimlik}' izi '{}' derlemesinde listede yok", self.genom),
                    "İz adını doğrulayın veya derlemeyi değiştirin",
                )
            })
    }

    fn getir(&self, _kimlik: &str, _baglam: &AramaBaglami) -> Result<GetirilenKayit, ErrorReport> {
        Err(ErrorReport::new(
            "UCSC izi doğrudan dosya olarak indirilemez",
            "bir UCSC izi bir bölge gerektirir (tüm genom indirilmez)",
            "Genom tarayıcıda bir bölge açıp izi oradan yükleyin (track_verisi ile bölgesel veri)",
        )
        .with_eylem("Genom tarayıcıda aç"))
    }
}

// ─── İz ayrıştırma ───────────────────────────────────────────────────────────────

/// Listeden bir iz (track) özeti.
struct Iz {
    ad: String,
    kisa: String,
    uzun: String,
    tip: String,
}

impl Iz {
    fn sonuc(&self) -> AramaSonucu {
        let baslik = if self.kisa.is_empty() {
            self.ad.clone()
        } else {
            self.kisa.clone()
        };
        let mut aciklama = self.uzun.clone();
        if !self.tip.is_empty() {
            if !aciklama.is_empty() {
                aciklama.push_str(" · ");
            }
            aciklama.push_str(&self.tip);
        }
        AramaSonucu::yeni(KAYNAK_ADI, &self.ad, baslik, KayitTuru::Iz).with_aciklama(aciklama)
    }
}

/// `/list/tracks` yanıtından (genom anahtarı altındaki) izleri çıkarır.
fn izleri_ayristir(govde: &str, genom: &str) -> Result<Vec<Iz>, ErrorReport> {
    let v: Value = serde_json::from_str(govde).map_err(json_hatasi)?;
    let genom_obj = v
        .get(genom)
        .and_then(|x| x.as_object())
        .ok_or_else(|| sema_hatasi(genom))?;

    let mut izler = Vec::new();
    for (ad, deger) in genom_obj {
        let Some(obj) = deger.as_object() else {
            continue;
        };
        // Bir iz nesnesi shortLabel taşır (kompozit alt-izleri sığ geçilir).
        let kisa = obj
            .get("shortLabel")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let uzun = obj
            .get("longLabel")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let tip = obj
            .get("type")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if kisa.is_empty() && uzun.is_empty() && tip.is_empty() {
            continue; // iz gibi görünmüyor (meta alan)
        }
        izler.push(Iz {
            ad: ad.clone(),
            kisa,
            uzun,
            tip,
        });
    }
    // Deterministik sıralama (ad).
    izler.sort_by(|a, b| a.ad.cmp(&b.ad));
    Ok(izler)
}

/// UCSC kromozom adı (gerekirse "chr" öneki ekler: "1" → "chr1"; "chr1" değişmez).
fn ucsc_krom(krom: &str) -> String {
    if krom.starts_with("chr") {
        krom.to_string()
    } else {
        format!("chr{krom}")
    }
}

fn json_hatasi(e: serde_json::Error) -> ErrorReport {
    ErrorReport::new(
        "UCSC yanıtı çözümlenemedi",
        "UCSC'den gelen JSON beklenen biçimde değil",
        "Daha sonra yeniden deneyin; sorun sürerse konektör güncellemesi gerekebilir",
    )
    .with_teknik_detay(e.to_string())
}

fn sema_hatasi(genom: &str) -> ErrorReport {
    ErrorReport::new(
        "UCSC yanıtı beklenen derlemeyi içermiyor",
        format!("yanıtta '{genom}' derleme anahtarı yok"),
        "Derleme adını doğrulayın (örn. hg38/hg19) veya yeniden deneyin",
    )
    .with_teknik_detay(format!("eksik_anahtar={genom}"))
}

#[cfg(test)]
mod tests {
    use super::super::super::privacy::GizlilikKapisi;
    use super::super::super::transport::SahteUlastirici;
    use super::*;
    use std::time::Duration;

    fn tracks_json() -> &'static str {
        r#"{"downloadTime":"x","genome":"hg38","hg38":{
            "knownGene":{"shortLabel":"GENCODE V44","longLabel":"GENCODE V44 Comprehensive","type":"genePred"},
            "refGene":{"shortLabel":"RefSeq Genes","longLabel":"NCBI RefSeq genes","type":"genePred"},
            "snp151":{"shortLabel":"dbSNP 151","longLabel":"Short Genetic Variants dbSNP 151","type":"bigDbSnp"}
        }}"#
    }
    fn getdata_json() -> &'static str {
        r#"{"genome":"hg38","chrom":"chr17","start":7668421,"end":7687550,
            "refGene":[{"name":"NM_000546","chromStart":7668421,"chromEnd":7687550}]}"#
    }

    fn baglam_kur<'a>(u: &'a SahteUlastirici, g: &'a GizlilikKapisi) -> AramaBaglami<'a> {
        let mut b = AramaBaglami::yeni(u, g);
        b.yapi.geri_cekilme_taban = Duration::ZERO;
        b
    }

    #[test]
    fn ara_iz_suzgec_uygular() {
        let u = SahteUlastirici::yeni().ekle("list/tracks", 200, tracks_json());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = UcscKonektor::hg38();
        // "genePred" tipi → knownGene + refGene (snp151 = bigDbSnp dışarıda).
        let liste = kon
            .ara(&Sorgu::metin("genePred"), Sayfalama::ilk(20), &baglam)
            .unwrap();
        assert_eq!(liste.sayfa.toplam, 2);
        assert!(liste.sonuclar.iter().all(|s| s.tur == KayitTuru::Iz));
        let adlar: Vec<&str> = liste.sonuclar.iter().map(|s| s.kimlik.as_str()).collect();
        assert!(adlar.contains(&"knownGene"));
        assert!(adlar.contains(&"refGene"));
        assert!(!adlar.contains(&"snp151"));
    }

    #[test]
    fn bos_sorgu_tum_izler() {
        let u = SahteUlastirici::yeni().ekle("list/tracks", 200, tracks_json());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = UcscKonektor::hg38();
        let liste = kon
            .ara(&Sorgu::metin(""), Sayfalama::ilk(20), &baglam)
            .unwrap();
        assert_eq!(liste.sayfa.toplam, 3);
    }

    #[test]
    fn track_verisi_bolge_cevirir_ve_provenans() {
        let u = SahteUlastirici::yeni().ekle("getData/track", 200, getdata_json());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = UcscKonektor::hg38();
        let konum = Konum::yeni("17", 7_668_422, 7_687_550); // 1-tabanlı
        let kayit = kon.track_verisi("refGene", &konum, &baglam).unwrap();
        assert_eq!(kayit.format_ipucu, "json");
        assert!(kayit.provenans.kaynak.contains("UCSC hg38"));
        assert!(kayit
            .provenans
            .lisans_atif
            .as_ref()
            .unwrap()
            .atif
            .contains("Kent"));
    }

    #[test]
    fn getir_durust_reddeder() {
        let u = SahteUlastirici::yeni();
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = UcscKonektor::hg38();
        let hata = kon.getir("refGene", &baglam).err().unwrap();
        assert!(hata.ne_oldu.contains("doğrudan dosya olarak indirilemez"));
    }

    #[test]
    fn ucsc_krom_chr_onek_ekler() {
        assert_eq!(ucsc_krom("1"), "chr1");
        assert_eq!(ucsc_krom("chr1"), "chr1");
        assert_eq!(ucsc_krom("X"), "chrX");
    }

    #[test]
    fn tarayici_url() {
        let kon = UcscKonektor::hg38();
        assert_eq!(
            kon.tarayici_url("refGene").as_deref(),
            Some("https://genome.ucsc.edu/cgi-bin/hgTrackUi?db=hg38&g=refGene")
        );
    }
}
