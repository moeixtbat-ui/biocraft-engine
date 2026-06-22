//! Bilim Pazarı **veri modeli** — İP-18 (salt-okur içerik akışının taşıdığı tipler).
//!
//! Bu modül yalnızca **veridir, davranış değil** (MK ruhu: dış içerik komut değil veridir).
//! Mağaza öğeleri (eklenti/şablon/veri seti), haber/makale kartları, yorumlar ve raporlama
//! sebepleri burada tanımlanır; akış [`crate::feed`] bunları küratörlü bir uzak kaynaktan
//! (MVP'de yerel/sentetik karşılığı) getirir.  Hepsi `serde` ile serileştirilebilir
//! (çevrimdışı önbellek + gelecekteki uzak JSON/RSS şeması).
//!
//! **Dürüstlük (MK-47/MK-48 ruhu):** [`DogrulamaDurumu`] alanları **abartısızdır** —
//! "doğrulandı" iddiası yalnızca kriptografik imza/resmi kaynağa (MK-16) bağlanır; topluluk
//! içeriği için durum açıkça **"doğrulama: beklemede"** kalır.  Hayalî "3 AI + insan onayı"
//! hattı bu sürümde **çalışmaz**; yalnızca bir **durum etiketi** olarak gösterilir (vizyon).

use biocraft_types::Timestamp;
use serde::{Deserialize, Serialize};

/// Bir mağaza öğesinin türü (mağaza bunu kategoriden ayrı bir filtre olarak kullanır).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OgeTuru {
    /// Çalıştırılabilir eklenti (`.bcext`; İP-07 host'unda çalışır).
    Eklenti,
    /// İş akışı şablonu (hazır node/akış düzeni; İP-17 ile aynı çizgi).
    Sablon,
    /// Veri seti (referans/örnek; köken + lisans şeffaf).
    VeriSeti,
}

impl OgeTuru {
    /// Tüm türler (filtre çubuğu için).
    pub const TUMU: &'static [OgeTuru] = &[OgeTuru::Eklenti, OgeTuru::Sablon, OgeTuru::VeriSeti];

    /// İki dilli kısa etiket.
    pub fn etiket(self, tr: bool) -> &'static str {
        match (self, tr) {
            (OgeTuru::Eklenti, true) => "Eklenti",
            (OgeTuru::Eklenti, false) => "Plugin",
            (OgeTuru::Sablon, true) => "Şablon",
            (OgeTuru::Sablon, false) => "Template",
            (OgeTuru::VeriSeti, true) => "Veri Seti",
            (OgeTuru::VeriSeti, false) => "Dataset",
        }
    }
}

/// Mağaza kategorisi (VS Code standardı: kategoriye göre gezinme + filtre).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kategori {
    /// Analiz / hesaplama.
    Analiz,
    /// Görselleştirme.
    Gorsellestirme,
    /// Veritabanı / erişim (BLAST/PDB/NCBI…).
    Veritabani,
    /// Yapay zeka / model.
    Ai,
    /// Genel araç / yardımcı.
    Arac,
    /// Eğitim / öğretici.
    Egitim,
    /// Diğer / sınıflandırılmamış.
    Diger,
}

impl Kategori {
    /// Tüm kategoriler (sol gezinme listesi için).
    pub const TUMU: &'static [Kategori] = &[
        Kategori::Analiz,
        Kategori::Gorsellestirme,
        Kategori::Veritabani,
        Kategori::Ai,
        Kategori::Arac,
        Kategori::Egitim,
        Kategori::Diger,
    ];

    /// İki dilli etiket.
    pub fn etiket(self, tr: bool) -> &'static str {
        match (self, tr) {
            (Kategori::Analiz, true) => "Analiz",
            (Kategori::Analiz, false) => "Analysis",
            (Kategori::Gorsellestirme, true) => "Görselleştirme",
            (Kategori::Gorsellestirme, false) => "Visualization",
            (Kategori::Veritabani, true) => "Veritabanı",
            (Kategori::Veritabani, false) => "Database",
            (Kategori::Ai, true) => "Yapay Zeka",
            (Kategori::Ai, false) => "AI",
            (Kategori::Arac, true) => "Araç",
            (Kategori::Arac, false) => "Tool",
            (Kategori::Egitim, true) => "Eğitim",
            (Kategori::Egitim, false) => "Education",
            (Kategori::Diger, true) => "Diğer",
            (Kategori::Diger, false) => "Other",
        }
    }
}

/// Bir öğenin fiyat/lisans modeli.  **Bio-kredi yalnızca yer tutucudur** (gerçek ödeme/ekonomi
/// `Hukuk-ve-Operasyon.md` + sonra; kripto/blockchain DEĞİL — ARCHITECTURE §13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fiyat {
    /// Ücretsiz.
    Ucretsiz,
    /// Açık kaynak (ücretsiz + kaynak açık).
    AcikKaynak,
    /// Ücretli — Bio-kredi **yer tutucu** miktarı (gerçek ödeme MVP'de yok).
    Ucretli {
        /// Yer tutucu Bio-kredi miktarı (gösterim amaçlı).
        bio_kredi: u32,
    },
}

impl Fiyat {
    /// Ücretsiz mi (açık kaynak dahil)?
    pub fn ucretsiz_mi(self) -> bool {
        matches!(self, Fiyat::Ucretsiz | Fiyat::AcikKaynak)
    }

    /// İki dilli kısa etiket (ücretli ise Bio-kredi yer tutucu gösterilir).
    pub fn etiket(self, tr: bool) -> String {
        match (self, tr) {
            (Fiyat::Ucretsiz, true) => "Ücretsiz".into(),
            (Fiyat::Ucretsiz, false) => "Free".into(),
            (Fiyat::AcikKaynak, true) => "Açık kaynak".into(),
            (Fiyat::AcikKaynak, false) => "Open source".into(),
            (Fiyat::Ucretli { bio_kredi }, true) => format!("{bio_kredi} Bio-kredi (yer tutucu)"),
            (Fiyat::Ucretli { bio_kredi }, false) => {
                format!("{bio_kredi} Bio-credits (placeholder)")
            }
        }
    }
}

/// Bir içeriğin **doğrulama durumu** — abartısız, dürüst.
///
/// "Doğrulandı" yalnızca kriptografik imzaya/resmi kaynağa (MK-16) dayanır.  Topluluk içeriği
/// için durum **"doğrulama: beklemede"** kalır: gelecekteki "3 AI + insan onayı" hattı bu
/// sürümde **çalışmaz**, yalnızca bir durum etiketi olarak gösterilir (vizyon — sahte
/// "doğrulandı" iddiası ÜRETİLMEZ, MK-48).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DogrulamaDurumu {
    /// Resmi (BioCraft) — imza ile doğrulanır (MK-16; kurulumda host teyit eder).
    Resmi,
    /// Doğrulanmış yayıncı — güven deposundaki bir 3. parti yayıncının imzası (MK-16).
    DogrulanmisYayinci,
    /// Küratörlü — BioCraft tarafından seçilmiş ama **biçimsel doğrulama değil** (yalnızca seçki).
    Kuratorlu,
    /// Doğrulama beklemede — topluluk içeriği; gelecekteki inceleme hattına aday (vizyon).
    IncelemeBekliyor,
}

impl DogrulamaDurumu {
    /// Bu durum bir **güven rozeti** (resmi/doğrulanmış yayıncı) mı?  Yalnızca imzaya dayananlar.
    pub fn guven_rozeti_mi(self) -> bool {
        matches!(
            self,
            DogrulamaDurumu::Resmi | DogrulamaDurumu::DogrulanmisYayinci
        )
    }

    /// İki dilli **dürüst** etiket — sahte "doğrulandı" iddiası içermez.
    pub fn etiket(self, tr: bool) -> &'static str {
        match (self, tr) {
            (DogrulamaDurumu::Resmi, true) => "Resmi",
            (DogrulamaDurumu::Resmi, false) => "Official",
            (DogrulamaDurumu::DogrulanmisYayinci, true) => "Doğrulanmış yayıncı",
            (DogrulamaDurumu::DogrulanmisYayinci, false) => "Verified publisher",
            (DogrulamaDurumu::Kuratorlu, true) => "Küratörlü",
            (DogrulamaDurumu::Kuratorlu, false) => "Curated",
            (DogrulamaDurumu::IncelemeBekliyor, true) => "Doğrulama: beklemede",
            (DogrulamaDurumu::IncelemeBekliyor, false) => "Verification: pending",
        }
    }
}

/// Bir mağaza öğesi (eklenti / şablon / veri seti).
///
/// Görsel/ekran-görüntüsü gerçek ikili veri olarak taşınmaz (MVP); yalnızca **etiketler**
/// ([`ekran_etiketleri`](Self::ekran_etiketleri)) gösterilir → güvenli/hafif (ham görsel indirme yok).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PazarOgesi {
    /// Kararlı kimlik (`biocraft.<yayinci>.<eklenti>`).
    pub kimlik: String,
    /// İnsan-okunur ad.
    pub ad: String,
    /// Yayıncı (geliştirici kimliği).
    pub yayinci: String,
    /// Kısa özet (liste kartında gösterilir — düz metin, HTML değil).
    pub ozet: String,
    /// Uzun açıklama (detay görünümünde — düz metin, HTML değil).
    pub aciklama: String,
    /// Öğe türü.
    pub tur: OgeTuru,
    /// Kategori.
    pub kategori: Kategori,
    /// Sürüm (SemVer metni).
    pub surum: String,
    /// Fiyat/lisans modeli.
    pub fiyat: Fiyat,
    /// Lisans adı ("MIT", "Apache-2.0", "CC-BY-4.0"…).
    pub lisans: String,
    /// Opsiyonel atıf metni (yayın/kaynak — bilimsel içerik için).
    #[serde(default)]
    pub atif: Option<String>,
    /// Ortalama puan (0.0–5.0).
    pub puan: f32,
    /// Puan veren sayısı.
    pub puan_sayisi: u32,
    /// İndirme/kurulum sayısı.
    pub indirme: u64,
    /// Son güncelleme (insan-okur tarih: "2026-06-20").
    pub son_guncelleme: String,
    /// Doğrulama durumu (dürüst; bkz. [`DogrulamaDurumu`]).
    pub dogrulama: DogrulamaDurumu,
    /// Ekran görüntüsü **etiketleri** (gerçek görsel değil; güvenli/hafif önizleme yer tutucu).
    #[serde(default)]
    pub ekran_etiketleri: Vec<String>,
    /// Kullanıcı yorumları (salt-okur; MVP'de yayınlama yok).
    #[serde(default)]
    pub yorumlar: Vec<Yorum>,
    /// Kurulabilir `.bcext` paketi (varsa) — İP-07 host'u ile gerçek kurulum için.
    ///
    /// MVP'de küratörlü kaynak birkaç öğe için **sentetik** paket gömer; ileride uzak indirme bunu
    /// doldurur.  Önbellek/serileştirmede yer kaplamasın diye boşsa atlanır (paketler yeniden çekilir).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paket: Option<Vec<u8>>,
}

impl PazarOgesi {
    /// Hızlı test/örnek kurucu (zorunlu alanlarla; opsiyoneller boş).
    pub fn yeni(
        kimlik: impl Into<String>,
        ad: impl Into<String>,
        yayinci: impl Into<String>,
        tur: OgeTuru,
        kategori: Kategori,
    ) -> Self {
        Self {
            kimlik: kimlik.into(),
            ad: ad.into(),
            yayinci: yayinci.into(),
            ozet: String::new(),
            aciklama: String::new(),
            tur,
            kategori,
            surum: "1.0.0".into(),
            fiyat: Fiyat::Ucretsiz,
            lisans: "MIT".into(),
            atif: None,
            puan: 0.0,
            puan_sayisi: 0,
            indirme: 0,
            son_guncelleme: String::new(),
            dogrulama: DogrulamaDurumu::IncelemeBekliyor,
            ekran_etiketleri: Vec::new(),
            yorumlar: Vec::new(),
            paket: None,
        }
    }

    /// Arama için birleşik (küçük-harf) saman dizgesi: ad + yayıncı + özet + kategori.
    pub fn saman(&self, tr: bool) -> String {
        format!(
            "{} {} {} {} {}",
            self.ad,
            self.yayinci,
            self.ozet,
            self.kategori.etiket(tr),
            self.tur.etiket(tr),
        )
        .to_lowercase()
    }
}

/// Bir haber/makale kartının türü (rozet/filtre).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HaberTuru {
    /// Bilimsel makale.
    Makale,
    /// Bilim/araştırma haberi.
    Haber,
    /// Veri seti duyurusu.
    VeriSeti,
    /// Şirket/topluluk duyurusu.
    Duyuru,
    /// Sürüm notu.
    SurumNotu,
}

impl HaberTuru {
    /// İki dilli kısa etiket.
    pub fn etiket(self, tr: bool) -> &'static str {
        match (self, tr) {
            (HaberTuru::Makale, true) => "Makale",
            (HaberTuru::Makale, false) => "Article",
            (HaberTuru::Haber, true) => "Haber",
            (HaberTuru::Haber, false) => "News",
            (HaberTuru::VeriSeti, true) => "Veri",
            (HaberTuru::VeriSeti, false) => "Data",
            (HaberTuru::Duyuru, true) => "Duyuru",
            (HaberTuru::Duyuru, false) => "Announcement",
            (HaberTuru::SurumNotu, true) => "Sürüm",
            (HaberTuru::SurumNotu, false) => "Release",
        }
    }
}

/// Bilim pazarı haber/makale kartı (salt-okur akış).
///
/// **Güvenli render (spec/Dikkat):** tüm alanlar **düz metindir**; HTML çalıştırılmaz.  `baglanti`
/// bir dış URL'dir ve **açmadan önce kullanıcı onayı** istenir (üst katman uygular).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HaberKarti {
    /// Başlık (düz metin).
    pub baslik: String,
    /// Kısa özet (düz metin).
    pub ozet: String,
    /// Kaynak adı ("Nature", "BioCraft", "NCBI"…).
    pub kaynak: String,
    /// İnsan-okur tarih ("2026-06-20").
    pub tarih: String,
    /// Opsiyonel dış bağlantı (açmadan önce onay istenir).
    #[serde(default)]
    pub baglanti: Option<String>,
    /// Kaynak **küratörlü/güvenilir** mi (rozet)?  Bu, *kaynağın* seçili olduğunu söyler;
    /// içeriğin bağımsız doğruluğunu **garanti etmez** (dürüstlük).
    #[serde(default)]
    pub dogrulanmis: bool,
    /// Kart türü.
    pub tur: HaberTuru,
}

impl HaberKarti {
    /// Arama için birleşik (küçük-harf) saman dizgesi.
    pub fn saman(&self) -> String {
        format!("{} {} {}", self.baslik, self.ozet, self.kaynak).to_lowercase()
    }
}

/// Bir kullanıcı yorumu (salt-okur; MVP'de yazma/yayınlama yok — yalnızca gösterim).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Yorum {
    /// Yorumu yazan (görünen ad).
    pub yazar: String,
    /// 1–5 yıldız.
    pub puan: u8,
    /// Yorum metni (düz metin).
    pub metin: String,
    /// İnsan-okur tarih.
    pub tarih: String,
}

impl Yorum {
    /// Yeni bir yorum (puanı 1–5 aralığına sıkıştırır).
    pub fn yeni(
        yazar: impl Into<String>,
        puan: u8,
        metin: impl Into<String>,
        tarih: impl Into<String>,
    ) -> Self {
        Self {
            yazar: yazar.into(),
            puan: puan.clamp(1, 5),
            metin: metin.into(),
            tarih: tarih.into(),
        }
    }
}

/// Bir içeriği **raporlama** sebebi (moderasyon/raporlama temeli — İP-18).
///
/// İçerik sorumluluğunun (yanlış bilgi/telif/takedown) **hukuki** çerçevesi
/// `Hukuk-ve-Operasyon.md`'dedir; burada yalnızca kullanıcı-tarafı sinyal toplama temeli vardır.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaporSebebi {
    /// Spam / alakasız.
    Spam,
    /// Yanlış / yanıltıcı bilgi.
    YanlisBilgi,
    /// Telif / lisans ihlali.
    TelifIhlali,
    /// Zararlı / kötü amaçlı içerik.
    ZararliIcerik,
    /// Diğer.
    Diger,
}

impl RaporSebebi {
    /// Tüm sebepler (raporlama menüsü).
    pub const TUMU: &'static [RaporSebebi] = &[
        RaporSebebi::Spam,
        RaporSebebi::YanlisBilgi,
        RaporSebebi::TelifIhlali,
        RaporSebebi::ZararliIcerik,
        RaporSebebi::Diger,
    ];

    /// İki dilli etiket.
    pub fn etiket(self, tr: bool) -> &'static str {
        match (self, tr) {
            (RaporSebebi::Spam, true) => "Spam / alakasız",
            (RaporSebebi::Spam, false) => "Spam / irrelevant",
            (RaporSebebi::YanlisBilgi, true) => "Yanlış bilgi",
            (RaporSebebi::YanlisBilgi, false) => "Misinformation",
            (RaporSebebi::TelifIhlali, true) => "Telif ihlali",
            (RaporSebebi::TelifIhlali, false) => "Copyright violation",
            (RaporSebebi::ZararliIcerik, true) => "Zararlı içerik",
            (RaporSebebi::ZararliIcerik, false) => "Harmful content",
            (RaporSebebi::Diger, true) => "Diğer",
            (RaporSebebi::Diger, false) => "Other",
        }
    }
}

/// Akışın taşıdığı **tüm pazar verisi** (önbelleğe yazılan birim).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PazarVerisi {
    /// Mağaza öğeleri (eklenti/şablon/veri seti).
    pub ogeler: Vec<PazarOgesi>,
    /// Haber/makale kartları.
    pub haberler: Vec<HaberKarti>,
    /// Bu akışın en son ne zaman çekildiği (önbellek tazeliği göstergesi).
    #[serde(default)]
    pub son_guncelleme: Option<Timestamp>,
}

impl PazarVerisi {
    /// Hem mağaza hem haber boş mu?
    pub fn bos_mu(&self) -> bool {
        self.ogeler.is_empty() && self.haberler.is_empty()
    }

    /// Kimliğe göre bir öğe bulur.
    pub fn oge(&self, kimlik: &str) -> Option<&PazarOgesi> {
        self.ogeler.iter().find(|o| o.kimlik == kimlik)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dogrulama_durumu_durust_etiketler() {
        // "Doğrulandı" sahte iddiası YOK: topluluk içeriği "beklemede" der.
        assert_eq!(
            DogrulamaDurumu::IncelemeBekliyor.etiket(true),
            "Doğrulama: beklemede"
        );
        assert!(!DogrulamaDurumu::IncelemeBekliyor.guven_rozeti_mi());
        assert!(!DogrulamaDurumu::Kuratorlu.guven_rozeti_mi());
        // Yalnız imzaya dayalı olanlar güven rozeti taşır.
        assert!(DogrulamaDurumu::Resmi.guven_rozeti_mi());
        assert!(DogrulamaDurumu::DogrulanmisYayinci.guven_rozeti_mi());
    }

    #[test]
    fn fiyat_ucretsiz_ayrimi() {
        assert!(Fiyat::Ucretsiz.ucretsiz_mi());
        assert!(Fiyat::AcikKaynak.ucretsiz_mi());
        assert!(!Fiyat::Ucretli { bio_kredi: 5 }.ucretsiz_mi());
        // Bio-kredi yer tutucu metinde açıkça "yer tutucu" geçer.
        assert!(Fiyat::Ucretli { bio_kredi: 5 }
            .etiket(true)
            .contains("yer tutucu"));
    }

    #[test]
    fn yorum_puani_sikistirilir() {
        assert_eq!(Yorum::yeni("a", 9, "x", "t").puan, 5);
        assert_eq!(Yorum::yeni("a", 0, "x", "t").puan, 1);
    }

    #[test]
    fn oge_saman_alanlari_icerir() {
        let o = PazarOgesi::yeni(
            "biocraft.x.y",
            "Süper Araç",
            "Acme",
            OgeTuru::Eklenti,
            Kategori::Analiz,
        );
        let s = o.saman(true);
        assert!(s.contains("süper araç"));
        assert!(s.contains("acme"));
        assert!(s.contains("analiz"));
    }

    #[test]
    fn pazar_verisi_kimlikle_bulur() {
        let mut v = PazarVerisi::default();
        v.ogeler.push(PazarOgesi::yeni(
            "biocraft.a.b",
            "Ad",
            "Y",
            OgeTuru::Sablon,
            Kategori::Arac,
        ));
        assert!(v.oge("biocraft.a.b").is_some());
        assert!(v.oge("yok").is_none());
        assert!(!v.bos_mu());
    }

    #[test]
    fn paket_baytlari_serdede_atlanir() {
        // Paket bytes önbellekte yer kaplamasın diye boşsa serileştirmede atlanır.
        let o = PazarOgesi::yeni("biocraft.a.b", "Ad", "Y", OgeTuru::Eklenti, Kategori::Ai);
        let j = serde_json::to_string(&o).unwrap();
        assert!(!j.contains("paket"), "boş paket alanı serileştirilmemeli");
        // Gidiş-dönüş korunur.
        let geri: PazarOgesi = serde_json::from_str(&j).unwrap();
        assert_eq!(o, geri);
    }
}
