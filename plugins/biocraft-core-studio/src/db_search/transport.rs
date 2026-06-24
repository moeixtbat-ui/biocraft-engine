//! ÇE-09 — **Ağ taşıması soyutlaması** (HTTP GET/POST/PUT) + dürüst yer-tutucu + test ikizi.
//!
//! `data_io::remote` ile aynı **dürüst sınır** desenini izler (MK-48): gerçek HTTP istemcisi
//! (ureq/reqwest) bu sürümde **bağlı değildir**.  [`YapilandirilmamisUlastirici`] her isteği
//! "net istemcisi yapılandırılmadı" ile reddeder; [`SahteUlastirici`] kayıtlı (URL-deseni → yanıt)
//! eşlemesiyle konektörleri **çevrimdışı** test/demo eder.  Gerçek async adaptör ileride bu trait'i
//! uygular (`net` yetkili; eklenti/insan-eli) — çağıran (konektör/panel) değişmeden takılır.
//!
//! ## Neden Tokio/biocraft-net değil?
//! Görev "Tokio + biocraft-net" der; ancak **MK-17** eklentinin yalnızca `biocraft-sdk`'ya
//! bağlanmasına izin verir (`biocraft-net` bir motor crate'idir → doğrudan bağımlılık YASAK) ve
//! proje "yeni ağır bağımlılık ekleme" disiplinini tutar.  Uzun iş ([`super::connectors::blast`])
//! senkron-pull [`IsKulpu`](biocraft_sdk::biocraft_types::IsKulpu) ile modellenir (İP-21); gerçek
//! ağ yığını bu [`HttpUlastirici`] arkasına, çekirdek değişmeden, sonra eklenir.

use biocraft_sdk::biocraft_types::ErrorReport;

/// HTTP istek yöntemi (E-utilities GET; BLAST Put = PUT, sonuç sorgusu GET).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpYontem {
    /// Veri okuma (esearch/esummary/efetch, BLAST durum/sonuç).
    Get,
    /// Form gövdeli gönderim.
    Post,
    /// BLAST iş gönderimi (CMD=Put).
    Put,
}

impl HttpYontem {
    /// HTTP metin karşılığı.
    pub fn metni(&self) -> &'static str {
        match self {
            HttpYontem::Get => "GET",
            HttpYontem::Post => "POST",
            HttpYontem::Put => "PUT",
        }
    }
}

/// Tek bir HTTP isteği (taban URL + querystring parametreleri + opsiyonel gövde).
///
/// Parametreler ayrı tutulur → [`tam_url`](Self::tam_url) ile **şeffaflık** (kullanıcıya "ne
/// gönderiliyor" gösterimi) ve ileride önbellek anahtarı tek yerden üretilir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpIstek {
    /// İstek yöntemi.
    pub yontem: HttpYontem,
    /// Taban URL (querystring'siz).
    pub url: String,
    /// Querystring parametreleri (sırayla; değerler URL-kodlanır).
    pub sorgu: Vec<(String, String)>,
    /// Opsiyonel istek gövdesi (POST/PUT).
    pub govde: Option<String>,
}

impl HttpIstek {
    /// Bir GET isteği başlatır.
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            yontem: HttpYontem::Get,
            url: url.into(),
            sorgu: Vec::new(),
            govde: None,
        }
    }

    /// Bir PUT isteği başlatır (BLAST iş gönderimi).
    pub fn put(url: impl Into<String>) -> Self {
        Self {
            yontem: HttpYontem::Put,
            url: url.into(),
            sorgu: Vec::new(),
            govde: None,
        }
    }

    /// Bir querystring parametresi ekler (akıcı).
    pub fn param(mut self, ad: impl Into<String>, deger: impl Into<String>) -> Self {
        self.sorgu.push((ad.into(), deger.into()));
        self
    }

    /// İstek gövdesini ayarlar (akıcı).
    pub fn with_govde(mut self, govde: impl Into<String>) -> Self {
        self.govde = Some(govde.into());
        self
    }

    /// Taban + URL-kodlanmış querystring → tam URL (şeffaflık + önbellek anahtarı).
    pub fn tam_url(&self) -> String {
        if self.sorgu.is_empty() {
            return self.url.clone();
        }
        let mut s = String::with_capacity(self.url.len() + 16);
        s.push_str(&self.url);
        s.push('?');
        for (i, (ad, deger)) in self.sorgu.iter().enumerate() {
            if i > 0 {
                s.push('&');
            }
            s.push_str(&yuzde_kodla(ad));
            s.push('=');
            s.push_str(&yuzde_kodla(deger));
        }
        s
    }
}

/// Bir HTTP yanıtı (durum kodu + gövde metni).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpYanit {
    /// HTTP durum kodu (200, 404…).
    pub durum_kodu: u16,
    /// Yanıt gövdesi (UTF-8 metin).
    pub govde: String,
}

impl HttpYanit {
    /// Yeni yanıt.
    pub fn yeni(durum_kodu: u16, govde: impl Into<String>) -> Self {
        Self {
            durum_kodu,
            govde: govde.into(),
        }
    }

    /// 2xx başarı durumu mu?
    pub fn basarili_mi(&self) -> bool {
        (200..300).contains(&self.durum_kodu)
    }

    /// Başarılıysa gövdeyi döndürür; değilse standart hata (durum kodu teknik detayda).
    pub fn metin(&self) -> Result<&str, ErrorReport> {
        if self.basarili_mi() {
            Ok(&self.govde)
        } else {
            Err(durum_hatasi(self.durum_kodu))
        }
    }
}

/// Bir HTTP isteğini gönderebilen taşıma katmanı (gerçek istemci ileride; bugün yer-tutucu/ikiz).
pub trait HttpUlastirici {
    /// İsteği gönderir, yanıtı döndürür (ağ/transport hatası `Err`).
    fn gonder(&self, istek: &HttpIstek) -> Result<HttpYanit, ErrorReport>;

    /// İstemci şu an çevrimiçi mi (çevrimdışı durumunu UI net göstersin diye — ÇE-09)?
    fn cevrimici_mi(&self) -> bool {
        true
    }
}

/// **Dürüst yer-tutucu** (MK-48): gerçek HTTP istemcisi bağlı değil → her isteği net hatayla
/// reddeder.  Gerçek `net` adaptörü bağlanınca bu tip değiştirilir (trait aynı kalır).
pub struct YapilandirilmamisUlastirici;

impl HttpUlastirici for YapilandirilmamisUlastirici {
    fn gonder(&self, istek: &HttpIstek) -> Result<HttpYanit, ErrorReport> {
        Err(net_yapilandirilmadi(&istek.tam_url()))
    }
    fn cevrimici_mi(&self) -> bool {
        false
    }
}

/// **Test/demo ikizi**: kayıtlı (URL-parçası → yanıt) eşlemesiyle konektörleri çevrimdışı sürer.
///
/// `gonder`, isteğin [`tam_url`](HttpIstek::tam_url)'inde **ilk eşleşen** parçayı bulur → o yanıtı
/// döndürür (ekleme sırası önemlidir: daha özel parçayı önce ekleyin).  Hiçbir eşleşme yoksa net
/// "eşleşme yok" hatası → konektör testi sahte veriyi unuttuğunda sessiz geçmez.
pub struct SahteUlastirici {
    yanitlar: Vec<(String, HttpYanit)>,
    cevrimici: bool,
}

impl SahteUlastirici {
    /// Boş (çevrimiçi) sahte ulaştırıcı.
    pub fn yeni() -> Self {
        Self {
            yanitlar: Vec::new(),
            cevrimici: true,
        }
    }

    /// Çevrimdışı ikiz — her istek "çevrimiçi değil" hatası verir (offline durumu testi).
    pub fn cevrimdisi() -> Self {
        Self {
            yanitlar: Vec::new(),
            cevrimici: false,
        }
    }

    /// `url_parcasi` tam URL'de geçtiğinde `govde`/`durum` döndüren bir kural ekler (akıcı).
    pub fn ekle(
        mut self,
        url_parcasi: impl Into<String>,
        durum_kodu: u16,
        govde: impl Into<String>,
    ) -> Self {
        self.yanitlar
            .push((url_parcasi.into(), HttpYanit::yeni(durum_kodu, govde)));
        self
    }
}

impl Default for SahteUlastirici {
    fn default() -> Self {
        Self::yeni()
    }
}

impl HttpUlastirici for SahteUlastirici {
    fn gonder(&self, istek: &HttpIstek) -> Result<HttpYanit, ErrorReport> {
        if !self.cevrimici {
            return Err(cevrimdisi_hatasi());
        }
        let tam = istek.tam_url();
        for (parca, yanit) in &self.yanitlar {
            if tam.contains(parca.as_str()) {
                return Ok(yanit.clone());
            }
        }
        Err(eslesme_yok(&tam))
    }
    fn cevrimici_mi(&self) -> bool {
        self.cevrimici
    }
}

// ─── URL kodlama (RFC 3986 unreserved dışında %XX) ───────────────────────────────

/// Bir querystring bileşenini yüzde-kodlar (yalnız `ALPHA/DIGIT/-._~` aynen kalır).
/// Harici bağımlılık yok — küçük ve deterministik.
pub fn yuzde_kodla(s: &str) -> String {
    let mut cikti = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                cikti.push(b as char);
            }
            _ => {
                cikti.push('%');
                cikti.push(onaltili(b >> 4));
                cikti.push(onaltili(b & 0x0F));
            }
        }
    }
    cikti
}

fn onaltili(yarim: u8) -> char {
    match yarim {
        0..=9 => (b'0' + yarim) as char,
        _ => (b'A' + (yarim - 10)) as char,
    }
}

// ─── Hatalar ─────────────────────────────────────────────────────────────────────

fn net_yapilandirilmadi(url: &str) -> ErrorReport {
    ErrorReport::new(
        "Ağ istemcisi yapılandırılmadı",
        format!("'{url}' isteği için gerçek HTTP istemcisi bu sürümde bağlı değil (dürüst sınır)"),
        "Dış veritabanı erişimi, 'net' yetkili bir ağ adaptörü bağlanınca etkin olur",
    )
    .with_eylem("Daha sonra")
}

fn cevrimdisi_hatasi() -> ErrorReport {
    ErrorReport::new(
        "Çevrimdışı",
        "ağ bağlantısı yok; dış veritabanına ulaşılamıyor",
        "Bağlantı gelince yeniden deneyin; önbellekteki sonuçlar gösterilmeye devam eder",
    )
    .with_eylem("Yeniden dene")
}

fn eslesme_yok(url: &str) -> ErrorReport {
    ErrorReport::new(
        "Sahte ulaştırıcıda eşleşen yanıt yok",
        format!("'{url}' için kayıtlı bir test yanıtı bulunamadı"),
        "Testte bu URL için `SahteUlastirici::ekle(...)` ile yanıt tanımlayın",
    )
}

fn durum_hatasi(kod: u16) -> ErrorReport {
    ErrorReport::new(
        "Sunucu hata durumu döndürdü",
        format!("HTTP {kod}"),
        "İstek hızını düşürün veya daha sonra yeniden deneyin (rate-limit/sunucu hatası olabilir)",
    )
    .with_eylem("Yeniden dene")
    .with_teknik_detay(format!("durum_kodu={kod}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tam_url_parametreleri_kodlar() {
        let istek = HttpIstek::get("https://e.org/esearch.fcgi")
            .param("db", "nucleotide")
            .param("term", "BRCA1[gene] AND human");
        let url = istek.tam_url();
        assert!(url.starts_with("https://e.org/esearch.fcgi?db=nucleotide&term="));
        // Boşluk + köşeli parantez kodlanır.
        assert!(url.contains("BRCA1%5Bgene%5D%20AND%20human"));
    }

    #[test]
    fn yapilandirilmamis_durust_reddeder() {
        let u = YapilandirilmamisUlastirici;
        assert!(!u.cevrimici_mi());
        let hata = u.gonder(&HttpIstek::get("https://x")).err().unwrap();
        assert_eq!(hata.ne_oldu, "Ağ istemcisi yapılandırılmadı");
    }

    #[test]
    fn sahte_kayitli_yaniti_dondurur() {
        let u = SahteUlastirici::yeni()
            .ekle("esearch.fcgi", 200, r#"{"esearchresult":{}}"#)
            .ekle("efetch.fcgi", 200, ">sq\nACGT");
        let y = u
            .gonder(&HttpIstek::get("https://e/efetch.fcgi").param("id", "5"))
            .unwrap();
        assert!(y.basarili_mi());
        assert_eq!(y.govde, ">sq\nACGT");
    }

    #[test]
    fn sahte_eslesme_yoksa_hata() {
        let u = SahteUlastirici::yeni().ekle("esearch.fcgi", 200, "{}");
        let hata = u.gonder(&HttpIstek::get("https://e/efetch.fcgi")).err();
        assert!(hata.is_some());
    }

    #[test]
    fn cevrimdisi_ikiz_hata_verir() {
        let u = SahteUlastirici::cevrimdisi();
        assert!(!u.cevrimici_mi());
        let hata = u.gonder(&HttpIstek::get("https://e")).err().unwrap();
        assert_eq!(hata.ne_oldu, "Çevrimdışı");
    }

    #[test]
    fn durum_kodu_hatasi_metinde_yansir() {
        let y = HttpYanit::yeni(429, "rate limited");
        assert!(!y.basarili_mi());
        assert!(y.metin().is_err());
    }
}
