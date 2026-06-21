//! Proje sihirbazının **saf** veri modeli + doğrulama (İP-02).
//!
//! Bu modülde egui YOKTUR: adım tipleri, alan yapıları, doğrulama kuralları ve sonuç/taslak
//! tipleri burada tanımlanır → **sahte veriyle birim-testlenir** (görsel katman olmadan).  egui
//! adaptörü [`super::ProjeSihirbazi::ciz`]'dedir.
//!
//! Veri sınıflandırma (Normal / Hassas-PHI / Sentetik) **zorunludur** (MK-42, İP-02): seçilmeden
//! özet/oluştur adımına geçilemez.  Gizliliğin temeli budur.

use std::path::PathBuf;

use biocraft_types::DataClassification;

// ─── Adım 1: Şablon / tür ─────────────────────────────────────────────────────

/// Yeni projenin başlangıç şablonu/türü.  Seçim, hangi panel/eklentilerin **ön-kurulu**
/// geleceğini belirler (gerçek ön-kurulum İP-07 eklenti host'u + ÇE çekirdek eklentiyle gelir;
/// burada yalnızca niyet kaydedilir).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjeSablonu {
    /// Genomik: genom tarayıcı + varyant + hizalama odaklı.
    Genomik,
    /// Proteomik: protein yapı/dizilim odaklı.
    Proteomik,
    /// CRISPR / gen düzenleme: kılavuz RNA + hedef-dışı analiz odaklı.
    CrisprGenDuzenleme,
    /// Boş: hiçbir şey ön-kurulmaz; kullanıcı kendi panel/eklentisini ekler.
    Bos,
}

impl ProjeSablonu {
    /// Sihirbazda gösterilecek tüm şablonlar (sıra sabit).
    pub const TUMU: &'static [ProjeSablonu] = &[
        ProjeSablonu::Genomik,
        ProjeSablonu::Proteomik,
        ProjeSablonu::CrisprGenDuzenleme,
        ProjeSablonu::Bos,
    ];

    /// Şablon ikonu (renk view'da token'dan).
    pub fn ikon(&self) -> &'static str {
        match self {
            ProjeSablonu::Genomik => "🧬",
            ProjeSablonu::Proteomik => "🔬",
            ProjeSablonu::CrisprGenDuzenleme => "✂",
            ProjeSablonu::Bos => "📄",
        }
    }

    /// Dile göre kısa ad.
    pub fn ad(&self, tr: bool) -> &'static str {
        match (self, tr) {
            (ProjeSablonu::Genomik, true) => "Genomik",
            (ProjeSablonu::Genomik, false) => "Genomics",
            (ProjeSablonu::Proteomik, true) => "Proteomik",
            (ProjeSablonu::Proteomik, false) => "Proteomics",
            (ProjeSablonu::CrisprGenDuzenleme, true) => "CRISPR / Gen Düzenleme",
            (ProjeSablonu::CrisprGenDuzenleme, false) => "CRISPR / Gene Editing",
            (ProjeSablonu::Bos, true) => "Boş Proje",
            (ProjeSablonu::Bos, false) => "Empty Project",
        }
    }

    /// Dile göre tek cümlelik açıklama.
    pub fn aciklama(&self, tr: bool) -> &'static str {
        match (self, tr) {
            (ProjeSablonu::Genomik, true) => {
                "Genom tarayıcı, varyant ve hizalama panelleriyle başlar."
            }
            (ProjeSablonu::Genomik, false) => {
                "Starts with genome browser, variant and alignment panels."
            }
            (ProjeSablonu::Proteomik, true) => {
                "Protein yapı görüntüleyici ve dizilim hizalama ile başlar."
            }
            (ProjeSablonu::Proteomik, false) => {
                "Starts with protein structure viewer and sequence alignment."
            }
            (ProjeSablonu::CrisprGenDuzenleme, true) => {
                "Kılavuz RNA tasarımı ve hedef-dışı analiz panelleriyle başlar."
            }
            (ProjeSablonu::CrisprGenDuzenleme, false) => {
                "Starts with guide RNA design and off-target analysis panels."
            }
            (ProjeSablonu::Bos, true) => "Hiçbir panel ön-kurulmaz; her şeyi kendiniz eklersiniz.",
            (ProjeSablonu::Bos, false) => "No panels pre-installed; you add everything yourself.",
        }
    }
}

// ─── Adım 3: Veri ayarları ────────────────────────────────────────────────────

/// Proje verisinin nerede tutulacağı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VeriYerlesimi {
    /// Tüm veri proje klasörünün içinde (yerel).
    Yerel,
    /// Veri dış konumlarda; projede yalnızca bağlantı/referans tutulur.
    Baglantili,
}

impl VeriYerlesimi {
    /// Dile göre ad.
    pub fn ad(&self, tr: bool) -> &'static str {
        match (self, tr) {
            (VeriYerlesimi::Yerel, true) => "Yerel (proje klasöründe)",
            (VeriYerlesimi::Yerel, false) => "Local (inside project folder)",
            (VeriYerlesimi::Baglantili, true) => "Bağlantılı (dış konumlara referans)",
            (VeriYerlesimi::Baglantili, false) => "Linked (reference to external locations)",
        }
    }
}

/// Büyük dosyaların projeye nasıl dahil edileceği.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuyukVeriStratejisi {
    /// Referansla tut (50 GB BAM klasöre kopyalanmaz; yol + BLAKE3 ile izlenir — MK-09).
    Referans,
    /// Gerçekten kopyala/göm (taşınabilir ama proje şişer).
    Gomulu,
}

impl BuyukVeriStratejisi {
    /// Dile göre ad.
    pub fn ad(&self, tr: bool) -> &'static str {
        match (self, tr) {
            (BuyukVeriStratejisi::Referans, true) => "Referansla tut (önerilen)",
            (BuyukVeriStratejisi::Referans, false) => "Keep by reference (recommended)",
            (BuyukVeriStratejisi::Gomulu, true) => "Projeye göm (kopyala)",
            (BuyukVeriStratejisi::Gomulu, false) => "Embed into project (copy)",
        }
    }
}

/// Adım 3 verisi: yerleşim + büyük-veri stratejisi + akış modu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VeriAyarlari {
    /// Verinin yerleşimi.
    pub yerlesim: VeriYerlesimi,
    /// Büyük veri stratejisi.
    pub buyuk_veri: BuyukVeriStratejisi,
    /// Akış (stream/out-of-core) modu — düşük RAM'de akıllı varsayılan olarak açık (MK-09).
    pub akis_modu: bool,
}

// ─── Adım 4: Veri sınıflandırma + gizlilik + güvenlik ─────────────────────────

/// Sihirbazda sunulan sınıflandırma seçenekleri (sıra sabit).
pub const SINIFLANDIRMALAR: &[DataClassification] = &[
    DataClassification::Normal,
    DataClassification::HasasPhi,
    DataClassification::Sentetik,
];

/// Bir sınıflandırmanın dile göre adı.
pub fn siniflandirma_ad(c: DataClassification, tr: bool) -> &'static str {
    match (c, tr) {
        (DataClassification::Normal, true) => "Normal",
        (DataClassification::Normal, false) => "Normal",
        (DataClassification::HasasPhi, true) => "Hassas / PHI (hasta verisi)",
        (DataClassification::HasasPhi, false) => "Sensitive / PHI (patient data)",
        (DataClassification::Sentetik, true) => "Sentetik (yapay)",
        (DataClassification::Sentetik, false) => "Synthetic (artificial)",
    }
}

/// Bir sınıflandırmanın dile göre açıklaması (sade dilde).
pub fn siniflandirma_aciklama(c: DataClassification, tr: bool) -> &'static str {
    match (c, tr) {
        (DataClassification::Normal, true) => "Kısıtlama gerektirmeyen genel araştırma verisi.",
        (DataClassification::Normal, false) => "General research data with no restrictions.",
        (DataClassification::HasasPhi, true) => {
            "Hasta/kişisel sağlık bilgisi. Asla otomatik dışarı, P2P'ye veya dış AI'a gönderilmez \
             (çekirdek seviyesinde engel — MK-42)."
        }
        (DataClassification::HasasPhi, false) => {
            "Patient/personal health info. Never sent out automatically, to P2P, or to external AI \
             (blocked at the core — MK-42)."
        }
        (DataClassification::Sentetik, true) => "Test ve kıyaslama için üretilmiş yapay veri.",
        (DataClassification::Sentetik, false) => {
            "Artificial data generated for testing/benchmarking."
        }
    }
}

/// Adım 4 verisi: zorunlu sınıflandırma + gizlilik profili + güvenlik.
///
/// `siniflandirma` **`None` ise adım geçersizdir** (MK-42): kullanıcı bir sınıf seçmeden
/// özet/oluştur adımına geçemez.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GizlilikGuvenlik {
    /// Zorunlu veri sınıflandırması (seçilene kadar `None`).
    pub siniflandirma: Option<DataClassification>,
    /// Tamamen yerel çalışma (varsayılan: açık).
    pub tamamen_yerel: bool,
    /// Anonimleştirilmiş sonuçları AI havuzuna katkı (varsayılan: **Hayır**).
    pub ai_havuzu_katki: bool,
    /// Yerel şifreleme (varsayılan: açık — "şifreli-yerel").
    pub sifreleme: bool,
}

impl GizlilikGuvenlik {
    /// Spec varsayılanları: sınıf seçilmemiş, tamamen yerel, AI havuzu Hayır, şifreli.
    pub fn varsayilan() -> Self {
        Self {
            siniflandirma: None,
            tamamen_yerel: true,
            ai_havuzu_katki: false,
            sifreleme: true,
        }
    }

    /// PHI seçiliyse gizlilik kilitlenir mi?  PHI'de tamamen-yerel + şifreleme zorunlu açık,
    /// AI havuzu katkısı zorunlu kapalıdır (MK-42 güvenli sınır kullanıcı hatasıyla aşılamaz).
    pub fn phi_kilitli(&self) -> bool {
        self.siniflandirma == Some(DataClassification::HasasPhi)
    }

    /// PHI seçildiğinde güvenli varsayılanları zorlar (geri alınamaz kilit, view bunları pasifler).
    pub fn phi_kilitlerini_uygula(&mut self) {
        if self.phi_kilitli() {
            self.tamamen_yerel = true;
            self.sifreleme = true;
            self.ai_havuzu_katki = false;
        }
    }
}

// ─── Adım 2: Proje bilgisi (girdi tamponları, ham) ────────────────────────────

/// Adım 2 verisi: kimlik alanları.  Etiket/ORCID **ham metin** olarak tutulur; doğrulama +
/// ayrıştırma taslağa dönüştürülürken yapılır (egui `TextEdit` doğrudan bu alanlara yazar).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjeBilgisi {
    /// Proje adı (zorunlu, geçerli dosya adı).
    pub ad: String,
    /// Proje konumu — ham metin (üst klasör; gerçek dosya seçici `rfd` ile İP-02 sonrası).
    pub konum: String,
    /// Açıklama (opsiyonel).
    pub aciklama: String,
    /// Kurum (opsiyonel).
    pub kurum: String,
    /// Etiketler — ham, virgülle ayrılmış (opsiyonel).
    pub etiketler_ham: String,
    /// ORCID — ham (opsiyonel; doluysa biçim doğrulanır).
    pub orcid_ham: String,
}

// ─── Adım 5: Dağıtık ağ ───────────────────────────────────────────────────────

/// Adım 5 verisi: dağıtık ağ eklenti durumu + tercih.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DagitikAg {
    /// Dağıtık ağ eklentisi kurulu mu (host bildirir; İP-15 ile tutarlı)?
    pub eklenti_kurulu: bool,
    /// Kullanıcı dağıtık ağı bu proje için etkinleştirdi mi (yalnızca kuruluysa anlamlı)?
    pub etkin: bool,
}

/// Dağıtık ağ eklentisinin (kurulu değilse) indirme yönlendirme adresi (İP-15).
pub const DAGITIK_AG_EKLENTI_URL: &str = "https://biocraftengine.com/eklentiler/dagitik-ag";

// ─── Adımlar ──────────────────────────────────────────────────────────────────

/// Sihirbazın altı adımı (sıra sabit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SihirbazAdim {
    /// 1) Şablon/tür.
    Sablon,
    /// 2) Proje bilgisi (ad/konum/açıklama/kurum/etiket/ORCID).
    Bilgi,
    /// 3) Veri ayarları.
    Veri,
    /// 4) Veri sınıflandırma (zorunlu) + gizlilik + güvenlik.
    Gizlilik,
    /// 5) Dağıtık ağ.
    Dagitik,
    /// 6) Özet + oluştur.
    Ozet,
}

impl SihirbazAdim {
    /// Adımlar (sıra sabit).
    pub const TUMU: &'static [SihirbazAdim] = &[
        SihirbazAdim::Sablon,
        SihirbazAdim::Bilgi,
        SihirbazAdim::Veri,
        SihirbazAdim::Gizlilik,
        SihirbazAdim::Dagitik,
        SihirbazAdim::Ozet,
    ];

    /// 0-tabanlı sıra indeksi.
    pub fn indeks(&self) -> usize {
        Self::TUMU.iter().position(|a| a == self).unwrap_or(0)
    }

    /// Toplam adım sayısı.
    pub fn toplam() -> usize {
        Self::TUMU.len()
    }

    /// Bir sonraki adım (son adımdaysa `None`).
    pub fn sonraki(&self) -> Option<SihirbazAdim> {
        Self::TUMU.get(self.indeks() + 1).copied()
    }

    /// Bir önceki adım (ilk adımdaysa `None`).
    pub fn onceki(&self) -> Option<SihirbazAdim> {
        let i = self.indeks();
        if i == 0 {
            None
        } else {
            Self::TUMU.get(i - 1).copied()
        }
    }

    /// Dile göre adım başlığı.
    pub fn baslik(&self, tr: bool) -> &'static str {
        match (self, tr) {
            (SihirbazAdim::Sablon, true) => "Şablon / Tür",
            (SihirbazAdim::Sablon, false) => "Template / Type",
            (SihirbazAdim::Bilgi, true) => "Proje Bilgisi",
            (SihirbazAdim::Bilgi, false) => "Project Info",
            (SihirbazAdim::Veri, true) => "Veri Ayarları",
            (SihirbazAdim::Veri, false) => "Data Settings",
            (SihirbazAdim::Gizlilik, true) => "Sınıflandırma & Gizlilik",
            (SihirbazAdim::Gizlilik, false) => "Classification & Privacy",
            (SihirbazAdim::Dagitik, true) => "Dağıtık Ağ",
            (SihirbazAdim::Dagitik, false) => "Distributed Network",
            (SihirbazAdim::Ozet, true) => "Özet & Oluştur",
            (SihirbazAdim::Ozet, false) => "Summary & Create",
        }
    }
}

// ─── Doğrulama ────────────────────────────────────────────────────────────────

/// Bir adımın doğrulamasından dönen tipli hata (i18n view'da, mantık saf).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DogrulamaHatasi {
    /// Proje adı boş.
    AdBos,
    /// Proje adında geçersiz karakter var (dosya adı kuralı).
    AdGecersizKarakter,
    /// Proje konumu boş.
    KonumBos,
    /// ORCID dolu ama biçimi geçersiz.
    OrcidGecersiz,
    /// Veri sınıflandırması seçilmedi (ZORUNLU — MK-42).
    SiniflandirmaSecilmedi,
}

impl DogrulamaHatasi {
    /// Dile göre kullanıcı mesajı.
    pub fn mesaj(&self, tr: bool) -> &'static str {
        match (self, tr) {
            (DogrulamaHatasi::AdBos, true) => "Proje adı boş olamaz.",
            (DogrulamaHatasi::AdBos, false) => "Project name cannot be empty.",
            (DogrulamaHatasi::AdGecersizKarakter, true) => {
                "Proje adı şu karakterleri içeremez: / \\ : * ? \" < > |"
            }
            (DogrulamaHatasi::AdGecersizKarakter, false) => {
                "Project name cannot contain: / \\ : * ? \" < > |"
            }
            (DogrulamaHatasi::KonumBos, true) => "Bir konum seçin.",
            (DogrulamaHatasi::KonumBos, false) => "Choose a location.",
            (DogrulamaHatasi::OrcidGecersiz, true) => {
                "ORCID biçimi geçersiz (örn. 0000-0002-1825-0097)."
            }
            (DogrulamaHatasi::OrcidGecersiz, false) => {
                "Invalid ORCID format (e.g. 0000-0002-1825-0097)."
            }
            (DogrulamaHatasi::SiniflandirmaSecilmedi, true) => {
                "Devam etmek için bir veri sınıflandırması seçin (zorunlu)."
            }
            (DogrulamaHatasi::SiniflandirmaSecilmedi, false) => {
                "Choose a data classification to continue (required)."
            }
        }
    }
}

/// Bir proje adının geçerli (dosya adı olarak güvenli) olup olmadığını söyler.
pub fn ad_gecerli(ad: &str) -> bool {
    let t = ad.trim();
    !t.is_empty() && !t.chars().any(gecersiz_dosya_karakteri)
}

/// Dosya adında yasak olan karakterler (Windows + Unix ortak güvenli alt küme).
fn gecersiz_dosya_karakteri(c: char) -> bool {
    matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control()
}

/// Bir ORCID dizgesinin biçimsel olarak geçerli olup olmadığını söyler (regex'siz, saf).
///
/// Biçim: `dddd-dddd-dddd-dddc` (16 hane, tireler 4-8-13. konumda; son hane `0-9` veya `X`).
pub fn orcid_gecerli(s: &str) -> bool {
    let t = s.trim();
    if t.len() != 19 {
        return false;
    }
    for (i, ch) in t.chars().enumerate() {
        match i {
            4 | 9 | 14 => {
                if ch != '-' {
                    return false;
                }
            }
            18 => {
                // Son kontrol hanesi 0-9 veya X olabilir (ORCID/ISNI kuralı).
                if !ch.is_ascii_digit() && ch != 'X' {
                    return false;
                }
            }
            _ => {
                if !ch.is_ascii_digit() {
                    return false;
                }
            }
        }
    }
    true
}

/// Ham etiket metnini ("a, b ,c,") temiz bir listeye ayrıştırır (boşları/tekrarları atar).
pub fn etiketleri_ayristir(ham: &str) -> Vec<String> {
    let mut sonuc: Vec<String> = Vec::new();
    for parca in ham.split(',') {
        let t = parca.trim();
        if !t.is_empty() && !sonuc.iter().any(|x| x == t) {
            sonuc.push(t.to_string());
        }
    }
    sonuc
}

// ─── Bağlam + sonuç + taslak ──────────────────────────────────────────────────

/// Sihirbazın akıllı varsayılanları için host'tan gelen bağlam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SihirbazBaglam {
    /// Düşük RAM tespit edildi mi (İP-08 donanım sınıfı) → akış modu varsayılan açık.
    pub dusuk_ram: bool,
    /// Dağıtık ağ eklentisi kurulu mu (İP-15)?
    pub dagitik_eklenti_kurulu: bool,
    /// Konum alanı için ön-doldurulan varsayılan klasör.
    pub varsayilan_konum: PathBuf,
}

impl Default for SihirbazBaglam {
    fn default() -> Self {
        Self {
            dusuk_ram: false,
            dagitik_eklenti_kurulu: false,
            varsayilan_konum: PathBuf::new(),
        }
    }
}

/// Sihirbaz tamamlandığında üretilen **taslak** (henüz dosya yok).
///
/// Gün 17'de `biocraft-data` bu taslağı alıp gerçek klasör + `biocraft.toml` + BLAKE3 bütünlük
/// üretecek.  Katman kuralı (MK-40): `biocraft-data` (L1) `biocraft-ui`'ye (L4) bağlanamaz; bu
/// nedenle taslak host (L5) tarafından `biocraft-data`'nın manifest tipine köprülenir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjeTaslagi {
    /// Seçilen şablon.
    pub sablon: ProjeSablonu,
    /// Proje adı (doğrulanmış).
    pub ad: String,
    /// Proje konumu.
    pub konum: PathBuf,
    /// Açıklama.
    pub aciklama: String,
    /// Kurum.
    pub kurum: String,
    /// Ayrıştırılmış etiketler.
    pub etiketler: Vec<String>,
    /// Geçerliyse normalize ORCID, yoksa `None`.
    pub orcid: Option<String>,
    /// Veri ayarları.
    pub veri: VeriAyarlari,
    /// Zorunlu veri sınıflandırması (taslakta artık kesin seçilmiştir).
    pub siniflandirma: DataClassification,
    /// Tamamen yerel mi?
    pub tamamen_yerel: bool,
    /// AI havuzuna katkı?
    pub ai_havuzu_katki: bool,
    /// Yerel şifreleme?
    pub sifreleme: bool,
    /// Dağıtık ağ etkin mi?
    pub dagitik_ag_etkin: bool,
}

/// Sihirbazın bir karede ürettiği üst-düzey sonuç (host uygular).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SihirbazSonucu {
    /// "Oluştur" → taslak hazır; host projeyi kurar (Gün 17 `biocraft-data`).
    Olustur(Box<ProjeTaslagi>),
    /// "İptal" → temiz çık; hiçbir kalıntı bırakılmaz (bugün dosya da yazılmaz).
    Iptal,
    /// Dağıtık ağ eklentisi için [İndir] tıklandı — sihirbaz açık kalır, host indirmeyi yönetir.
    EklentiIndir(String),
}
