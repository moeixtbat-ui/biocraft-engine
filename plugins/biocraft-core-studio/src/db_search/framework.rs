//! ÇE-09 — **Veritabanı arama çerçevesi** (soyut konektör + birleşik sorgu/sonuç şeması).
//!
//! "Yeni veritabanı = yeni konektör" ilkesini uygular (MK-41, ÇE-09 *genişletilebilirlik*): her dış
//! bilimsel veritabanı [`VeritabaniKonektoru`] trait'ini uygular; çerçeve/çekirdek değişmeden yeni
//! kaynak eklenir (NCBI/BLAST bugün; PDB/UniProt/Ensembl/UCSC Gün 41).  Ağ taşıması soyuttur
//! ([`super::transport`]) ve gerçek istemci bu sürümde bağlı değildir (dürüst sınır) → tüm mantık
//! **çevrimdışı birim-testlenir**.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use biocraft_sdk::biocraft_types::ErrorReport;

use super::privacy::GizlilikKapisi;
use super::ratelimit::KaynakHizYoneticisi;
use super::transport::HttpUlastirici;
use crate::data_io::{HttpYapilandirma, Provenans};

/// Bir veritabanı kaydının türü (sonuç rozeti + tek-tık eylem hedefi).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KayitTuru {
    /// Nükleotid dizisi (DNA/RNA) — NCBI nucleotide, Ensembl…
    Nukleotid,
    /// Protein dizisi — NCBI protein, UniProt…
    Protein,
    /// Gen kaydı (lokus/öznitelik) — NCBI gene, Ensembl…
    Gen,
    /// 3B makromolekül yapısı (PDB/mmCIF) — RCSB PDB → ÇE-07 görüntüleyici.
    Yapi,
    /// Dizi hizalaması (BLAST sonucu).
    Hizalama,
    /// Genom izi / track (UCSC) — bir bölgeyle genom tarayıcıda (ÇE-02) gösterilir.
    Iz,
    /// Diğer/sınıflanmamış.
    Diger,
}

impl KayitTuru {
    /// İnsan-okur etiket.
    pub fn etiket(&self) -> &'static str {
        match self {
            KayitTuru::Nukleotid => "Nükleotid",
            KayitTuru::Protein => "Protein",
            KayitTuru::Gen => "Gen",
            KayitTuru::Yapi => "3B Yapı",
            KayitTuru::Hizalama => "Hizalama",
            KayitTuru::Iz => "Genom İzi",
            KayitTuru::Diger => "Diğer",
        }
    }

    /// Bu kayıt doğrudan 3B görüntüleyiciye (ÇE-07) yüklenebilir mi (tek-tık "yapıya bak")?
    pub fn yapiya_bakilabilir_mi(&self) -> bool {
        matches!(self, KayitTuru::Yapi)
    }
}

/// Bir sonucun genomik konumu (çapraz bağlantı: sonuç → genom tarayıcı ÇE-02 "bölgeye git").
///
/// `data_io`/`genome_browser`'ın `GenomBolge`'siyle aynı **1-tabanlı kapsayıcı** anlama sahiptir;
/// db_search çekirdek tiplere bağlanmadan (MK-17) hafif bir taşıyıcı tutar (host eşler).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Konum {
    /// Kromozom/kontig adı (örn. "17", "chr17").
    pub kromozom: String,
    /// 1-tabanlı başlangıç (kapsayıcı).
    pub baslangic: u64,
    /// 1-tabanlı bitiş (kapsayıcı).
    pub bitis: u64,
}

impl Konum {
    /// Yeni konum (1-tabanlı kapsayıcı).
    pub fn yeni(kromozom: impl Into<String>, baslangic: u64, bitis: u64) -> Self {
        Self {
            kromozom: kromozom.into(),
            baslangic,
            bitis,
        }
    }

    /// `kromozom:başlangıç-bitiş` biçimi (genom tarayıcıya "bölgeye git" girdisi).
    pub fn bolge_metni(&self) -> String {
        format!("{}:{}-{}", self.kromozom, self.baslangic, self.bitis)
    }
}

/// Skorlu (BLAST gibi) sonuçlarda hizalama kalitesi — sıralama/filtreleme için.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SonucSkoru {
    /// Bit skoru (yüksek = iyi).
    pub bit_skoru: f64,
    /// E-değeri (düşük = iyi).
    pub e_deger: f64,
    /// Özdeşlik yüzdesi (0–100).
    pub ozdeslik_yuzde: f64,
}

/// Birleşik sonuç satırı (kaynak rozetli; tüm konektörler aynı şemayı doldurur).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AramaSonucu {
    /// Kaynak rozeti (örn. "NCBI nucleotide", "BLAST", "PDB").
    pub kaynak: String,
    /// Erişim numarası / UID / RID (kaynak içi kimlik).
    pub kimlik: String,
    /// Başlık (tanım satırı).
    pub baslik: String,
    /// Organizma (varsa).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organizma: Option<String>,
    /// Dizi uzunluğu (bp/aa; varsa).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uzunluk: Option<u64>,
    /// Kayıt türü.
    pub tur: KayitTuru,
    /// Kısa açıklama/özet (önizleme).
    pub aciklama: String,
    /// Skor (BLAST gibi; aksi hâlde `None`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skor: Option<SonucSkoru>,
    /// Genomik konum (Ensembl/UCSC gibi konumlu sonuçlarda) → genom tarayıcı çapraz bağlantısı.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub konum: Option<Konum>,
}

impl AramaSonucu {
    /// Zorunlu alanlarla minimal sonuç (akıcı kurucularla zenginleştirilir).
    pub fn yeni(
        kaynak: impl Into<String>,
        kimlik: impl Into<String>,
        baslik: impl Into<String>,
        tur: KayitTuru,
    ) -> Self {
        Self {
            kaynak: kaynak.into(),
            kimlik: kimlik.into(),
            baslik: baslik.into(),
            organizma: None,
            uzunluk: None,
            tur,
            aciklama: String::new(),
            skor: None,
            konum: None,
        }
    }

    /// Organizma ekler.
    pub fn with_organizma(mut self, o: impl Into<String>) -> Self {
        self.organizma = Some(o.into());
        self
    }
    /// Uzunluk ekler.
    pub fn with_uzunluk(mut self, u: u64) -> Self {
        self.uzunluk = Some(u);
        self
    }
    /// Açıklama ekler.
    pub fn with_aciklama(mut self, a: impl Into<String>) -> Self {
        self.aciklama = a.into();
        self
    }
    /// Skor ekler.
    pub fn with_skor(mut self, s: SonucSkoru) -> Self {
        self.skor = Some(s);
        self
    }
    /// Genomik konum ekler (çapraz bağlantı: genom tarayıcıda "bölgeye git").
    pub fn with_konum(mut self, k: Konum) -> Self {
        self.konum = Some(k);
        self
    }
}

/// Arama sorgusu (serbest metin + tür ipucu).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sorgu {
    /// Serbest metin (gen adı / accession / anahtar kelime).  BLAST'ta = dizi.
    pub metin: String,
    /// Aranan kayıt türü ipucu (konektör destekliyorsa filtreler).
    pub tur: Option<KayitTuru>,
}

impl Sorgu {
    /// Yalnız metinden sorgu.
    pub fn metin(metin: impl Into<String>) -> Self {
        Self {
            metin: metin.into(),
            tur: None,
        }
    }
}

/// Sayfalama (binlerce sonuçta sayfalı/sanal liste — ÇE-09).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sayfalama {
    /// Atlanacak kayıt sayısı (esearch `retstart`).
    pub ofset: u64,
    /// Sayfa başına kayıt (esearch `retmax`).
    pub limit: u64,
}

impl Sayfalama {
    /// İlk sayfa: `ofset=0`, verilen `limit`.
    pub fn ilk(limit: u64) -> Self {
        Self {
            ofset: 0,
            limit: limit.max(1),
        }
    }

    /// Bir sonraki sayfa (ofset += limit).
    pub fn sonraki(&self) -> Self {
        Self {
            ofset: self.ofset + self.limit,
            limit: self.limit,
        }
    }
}

impl Default for Sayfalama {
    fn default() -> Self {
        Self::ilk(20)
    }
}

/// Bir sonuç sayfasının bilgisi (toplam + konum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SayfaBilgisi {
    /// Eşleşen toplam kayıt sayısı (sayfalamadan bağımsız).
    pub toplam: u64,
    /// Bu sayfanın ofseti.
    pub ofset: u64,
    /// Bu sayfanın limiti.
    pub limit: u64,
}

impl SayfaBilgisi {
    /// Daha fazla sonuç var mı (sonraki sayfaya geçilebilir)?
    pub fn sonraki_var(&self) -> bool {
        self.ofset + self.limit < self.toplam
    }
    /// Önceki sayfa var mı?
    pub fn onceki_var(&self) -> bool {
        self.ofset > 0
    }
}

/// Bir aramanın sonucu (satırlar + sayfa bilgisi).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SonucListesi {
    /// Bu sayfadaki sonuç satırları.
    pub sonuclar: Vec<AramaSonucu>,
    /// Sayfa bilgisi (toplam/ofset/limit).
    pub sayfa: SayfaBilgisi,
}

/// İndirilmiş, projeye eklenmeye hazır bir kayıt (tek-tık yükleme çıktısı).
#[derive(Debug, Clone, PartialEq)]
pub struct GetirilenKayit {
    /// Kaynak içi kimlik (accession/UID).
    pub kimlik: String,
    /// İçerik formatı ipucu (`data_io::detect` ile aynı sözlük: "fasta", "pdb"…).
    pub format_ipucu: String,
    /// Ham içerik (FASTA/PDB metni vb.).
    pub icerik: Vec<u8>,
    /// Köken kaydı (kaynak/erişim tarihi/BLAKE3 + lisans/atıf — İP-10, ÇE-09).
    pub provenans: Provenans,
}

/// Bir arama çağrısının **bağlamı**: ağ taşıması + gizlilik kapısı + HTTP yapılandırma + (opsiyonel)
/// API anahtarı / hız sınırlayıcı.
///
/// Bağlamın struct olması, Gün 41 alanlarının (önbellek, kaynak-başına kota) [`VeritabaniKonektoru`]
/// **trait imzasını değiştirmeden** eklenmesini sağlar (ileri-uyum / dürüst-sınır deseni).
pub struct AramaBaglami<'a> {
    /// Ağ taşıması (gerçek istemci ileride; bugün yer-tutucu/test ikizi).
    pub ulastirici: &'a dyn HttpUlastirici,
    /// Dış gönderim onayı + PHI sınırı (MK-41/42/43).
    pub gizlilik: &'a GizlilikKapisi,
    /// Zaman aşımı + tekrar/geri-çekilme (`data_io::remote` ile paylaşılan tip).
    pub yapi: HttpYapilandirma,
    /// Opsiyonel API anahtarı (NCBI: hız limiti yükselir; kimlik İP-09 ile şifreli saklanır — Gün 41).
    pub api_anahtari: Option<String>,
    /// Opsiyonel **tek** istek hız sınırlayıcı (eski; tüm konektörler aynı kovayı paylaşır).
    pub hiz: Option<&'a HizSinirlayici>,
    /// Opsiyonel **kaynak-başına** hız yöneticisi (Gün 41): her kaynağın kendi kovası + kuyruğu
    /// (ileri-uyum; varsa [`hiz_bekle_kaynak`](Self::hiz_bekle_kaynak) bunu kullanır).
    pub hiz_yoneticisi: Option<&'a KaynakHizYoneticisi>,
}

impl<'a> AramaBaglami<'a> {
    /// Taşıma + gizlilik kapısıyla varsayılan bağlam.
    pub fn yeni(ulastirici: &'a dyn HttpUlastirici, gizlilik: &'a GizlilikKapisi) -> Self {
        Self {
            ulastirici,
            gizlilik,
            yapi: HttpYapilandirma::default(),
            api_anahtari: None,
            hiz: None,
            hiz_yoneticisi: None,
        }
    }

    /// API anahtarı ekler (akıcı).
    pub fn with_api_anahtari(mut self, anahtar: impl Into<String>) -> Self {
        self.api_anahtari = Some(anahtar.into());
        self
    }

    /// Tek hız sınırlayıcı ekler (akıcı; eski/ortak kova).
    pub fn with_hiz(mut self, hiz: &'a HizSinirlayici) -> Self {
        self.hiz = Some(hiz);
        self
    }

    /// Kaynak-başına hız yöneticisi ekler (akıcı; Gün 41).
    pub fn with_hiz_yoneticisi(mut self, yoneticisi: &'a KaynakHizYoneticisi) -> Self {
        self.hiz_yoneticisi = Some(yoneticisi);
        self
    }

    /// Bir dış istek yollamadan önce çağrılır: tek (ortak) hız sınırı varsa bekletir.
    pub fn hiz_bekle(&self) {
        if let Some(h) = self.hiz {
            h.bekle();
        }
    }

    /// Bir dış istek yollamadan önce **kaynağa özgü** hız sınırını uygular (Gün 41).
    /// Kaynak-başına yönetici varsa onu (her kaynağın kendi kovası) kullanır; yoksa ortak
    /// sınırlayıcıya ([`hiz_bekle`](Self::hiz_bekle)) düşer (geriye-uyum).
    pub fn hiz_bekle_kaynak(&self, kaynak: &str) {
        if let Some(y) = self.hiz_yoneticisi {
            y.bekle(kaynak);
        } else {
            self.hiz_bekle();
        }
    }
}

/// Basit **istek hız sınırlayıcı** (NCBI: API anahtarsız ≤3 istek/sn, anahtarla ≤10).
///
/// Son istek zamanını tutar; `talep_et` bir sonraki isteğe izin vermeden önce beklenmesi gereken
/// süreyi döndürür **ve** iç zamanı ilerletir (saf-test edilebilir).  `bekle` bu süreyi uygular.
/// **Gün 41** kaynak-başına kova + kuyruk ile derinleştirir.
pub struct HizSinirlayici {
    asgari_aralik: Duration,
    sonraki_uygun: Mutex<Option<Instant>>,
}

impl HizSinirlayici {
    /// İstekler arası asgari aralıkla yeni sınırlayıcı.
    pub fn yeni(asgari_aralik: Duration) -> Self {
        Self {
            asgari_aralik,
            sonraki_uygun: Mutex::new(None),
        }
    }

    /// NCBI E-utilities için uygun sınır (anahtar varsa 10/sn = 100 ms, yoksa 3/sn ≈ 334 ms).
    pub fn ncbi(api_anahtari_var: bool) -> Self {
        let ms = if api_anahtari_var { 100 } else { 334 };
        Self::yeni(Duration::from_millis(ms))
    }

    /// Bir sonraki isteğe izin vermeden önce **beklenmesi gereken süre**; iç zamanı ilerletir.
    pub fn talep_et(&self) -> Duration {
        let simdi = Instant::now();
        let mut kilit = self.sonraki_uygun.lock().unwrap();
        match *kilit {
            Some(uygun) if uygun > simdi => {
                let bekle = uygun - simdi;
                *kilit = Some(uygun + self.asgari_aralik);
                bekle
            }
            _ => {
                *kilit = Some(simdi + self.asgari_aralik);
                Duration::ZERO
            }
        }
    }

    /// Gerekiyorsa bekler (gerçek hız sınırlama).
    pub fn bekle(&self) {
        let d = self.talep_et();
        if !d.is_zero() {
            std::thread::sleep(d);
        }
    }
}

/// Bir dış bilimsel veritabanı konektörü.  **Yeni veritabanı = bu trait'i uygulayan yeni tip**
/// (çerçeve/çekirdek değişmez — MK-41, ÇE-09 genişletilebilirlik).
pub trait VeritabaniKonektoru {
    /// Sonuç rozetinde görünen kaynak adı (örn. "NCBI nucleotide").
    fn kaynak_adi(&self) -> &str;

    /// Bu konektörün döndürebileceği kayıt türleri.
    fn turler(&self) -> &[KayitTuru];

    /// Bir kaydın kaynak web sayfası URL'i ("tarayıcıda aç" — dış erişim, onay + `net` gerekir).
    fn tarayici_url(&self, kimlik: &str) -> Option<String>;

    /// **Arama**: sorgu + sayfalama → sonuç listesi.
    ///
    /// Dış istek yollamadan ÖNCE [`baglam.gizlilik`](AramaBaglami::gizlilik) ile denetlenir (PHI
    /// engeli + onay; MK-41/42/43) ve hız sınırı uygulanır.
    fn ara(
        &self,
        sorgu: &Sorgu,
        sayfalama: Sayfalama,
        baglam: &AramaBaglami,
    ) -> Result<SonucListesi, ErrorReport>;

    /// **Önizleme/detay**: tek bir kayıt için özet (uzunluk/organizma/açıklama).
    fn detay(&self, kimlik: &str, baglam: &AramaBaglami) -> Result<AramaSonucu, ErrorReport>;

    /// **Getir** (tek-tık yükleme): kaydı indirir → projeye eklenmeye hazır içerik + provenance.
    fn getir(&self, kimlik: &str, baglam: &AramaBaglami) -> Result<GetirilenKayit, ErrorReport>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sayfalama_sonraki_ofseti_ilerletir() {
        let s = Sayfalama::ilk(20);
        assert_eq!(s.ofset, 0);
        let s2 = s.sonraki();
        assert_eq!(s2.ofset, 20);
        assert_eq!(s2.limit, 20);
    }

    #[test]
    fn sayfa_bilgisi_sonraki_onceki() {
        let b = SayfaBilgisi {
            toplam: 100,
            ofset: 0,
            limit: 20,
        };
        assert!(b.sonraki_var());
        assert!(!b.onceki_var());
        let son = SayfaBilgisi {
            toplam: 100,
            ofset: 80,
            limit: 20,
        };
        assert!(!son.sonraki_var());
        assert!(son.onceki_var());
    }

    #[test]
    fn kayit_turu_yapiya_bakilabilir() {
        assert!(KayitTuru::Yapi.yapiya_bakilabilir_mi());
        assert!(!KayitTuru::Nukleotid.yapiya_bakilabilir_mi());
        assert!(!KayitTuru::Iz.yapiya_bakilabilir_mi());
        assert_eq!(KayitTuru::Iz.etiket(), "Genom İzi");
    }

    #[test]
    fn konum_bolge_metni_ve_serde() {
        let k = Konum::yeni("17", 7_668_421, 7_687_550);
        assert_eq!(k.bolge_metni(), "17:7668421-7687550");
        // AramaSonucu konumuyla serde round-trip (önbellek için).
        let s = AramaSonucu::yeni("Ensembl", "ENSG00000141510", "TP53", KayitTuru::Gen)
            .with_konum(k.clone());
        let js = serde_json::to_string(&s).unwrap();
        let geri: AramaSonucu = serde_json::from_str(&js).unwrap();
        assert_eq!(geri.konum, Some(k));
    }

    #[test]
    fn sonuc_listesi_serde_round_trip() {
        let liste = SonucListesi {
            sonuclar: vec![AramaSonucu::yeni("PDB", "1TUP", "p53", KayitTuru::Yapi)],
            sayfa: SayfaBilgisi {
                toplam: 1,
                ofset: 0,
                limit: 20,
            },
        };
        let js = serde_json::to_string(&liste).unwrap();
        let geri: SonucListesi = serde_json::from_str(&js).unwrap();
        assert_eq!(geri, liste);
    }

    #[test]
    fn hiz_sinirlayici_ardisik_istegi_bekletir() {
        let h = HizSinirlayici::yeni(Duration::from_millis(200));
        // İlk talep beklemesiz.
        assert_eq!(h.talep_et(), Duration::ZERO);
        // Hemen ardından gelen talep beklemeli (pozitif süre).
        let bekle = h.talep_et();
        assert!(bekle > Duration::ZERO);
        assert!(bekle <= Duration::from_millis(200));
    }

    #[test]
    fn ncbi_hiz_anahtarla_daha_yuksek() {
        // Anahtarsız aralık (≈334ms) > anahtarlı (100ms) → anahtarla daha hızlı.
        let anahtarsiz = HizSinirlayici::ncbi(false);
        let anahtarli = HizSinirlayici::ncbi(true);
        assert!(anahtarsiz.asgari_aralik > anahtarli.asgari_aralik);
    }

    #[test]
    fn arama_sonucu_akici_kurucular() {
        let s = AramaSonucu::yeni("NCBI", "NM_1", "BRCA1", KayitTuru::Nukleotid)
            .with_organizma("Homo sapiens")
            .with_uzunluk(320);
        assert_eq!(s.organizma.as_deref(), Some("Homo sapiens"));
        assert_eq!(s.uzunluk, Some(320));
    }
}
