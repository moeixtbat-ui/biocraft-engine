//! Yapılandırılmış log kaydı + PII sanitizasyonu (İP-21, MK-57, MK-45).
//!
//! Bu modül **veri modelidir**: bir log olayının yapılandırılmış (alanlı) gösterimi,
//! OpenTelemetry-uyumlu önem (severity) eşlemesi ve **PII temizleme** kuralları.
//! Gerçek *sink* (konsol/dosya yazımı, seviye süzme, `log` köprüsü) üst katmandadır
//! (`biocraft-app/src/observability/`).  Modeli L0'da tutmak → her crate aynı yapılandırılmış
//! kaydı üretip test/golden'layabilir; sink'i tek noktadan değiştiririz.
//!
//! **MK-45 (gizlilik):** Hiçbir log satırı kişisel/hasta verisi (PHI/PII) içeremez.  Mesaj ve
//! alan değerleri [`pii_temizle`] süzgecinden geçer; ayrıca [`LogKaydi::hassas_alan`] ile işaretli
//! değerler **hiç yazılmaz** (yalnızca `<gizlendi>` görünür).

use serde::{Deserialize, Serialize};

use crate::{Timestamp, TraceContext};

/// Log önem seviyesi — OpenTelemetry `SeverityNumber` ile eşlenir (sayı + metin).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogSeverity {
    /// En ayrıntılı; geliştirici izleme (üretimde kapalı).
    Trace,
    /// Ayıklama ayrıntısı.
    Debug,
    /// Normal akış bilgisi.
    Info,
    /// Beklenmeyen ama kurtarılabilir durum.
    Warn,
    /// İşlem başarısız oldu.
    Error,
}

impl LogSeverity {
    /// OpenTelemetry `SeverityNumber` (1..24 ölçeği; her aralığın temsilcisi).
    pub fn otel_numarasi(&self) -> u8 {
        match self {
            LogSeverity::Trace => 1,
            LogSeverity::Debug => 5,
            LogSeverity::Info => 9,
            LogSeverity::Warn => 13,
            LogSeverity::Error => 17,
        }
    }

    /// OpenTelemetry `SeverityText` (standart İngilizce kısaltma).
    pub fn otel_metni(&self) -> &'static str {
        match self {
            LogSeverity::Trace => "TRACE",
            LogSeverity::Debug => "DEBUG",
            LogSeverity::Info => "INFO",
            LogSeverity::Warn => "WARN",
            LogSeverity::Error => "ERROR",
        }
    }
}

/// Tek bir yapılandırılmış log olayı (OTel "LogRecord" şekline yakın).
///
/// `to_json` çıktısı satır-başına-bir-JSON (NDJSON) biçimindedir; OTel toplayıcıya
/// olduğu gibi aktarılabilir.  Tüm metinler [`pii_temizle`]'den geçmiştir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogKaydi {
    /// Olay zamanı (UTC).
    pub zaman: Timestamp,
    /// Önem.
    pub seviye: LogSeverity,
    /// Kaynak modül/bileşen (ör. `biocraft_data::project`).
    pub hedef: String,
    /// Temizlenmiş mesaj gövdesi.
    pub mesaj: String,
    /// İz kimliği (W3C/OTel, 32 hex) — uzun iş/dış çağrı bağlamı varsa.
    pub trace_id: Option<String>,
    /// Adım kimliği (W3C/OTel, 16 hex).
    pub span_id: Option<String>,
    /// Yapılandırılmış alanlar (anahtar→temizlenmiş değer).
    pub alanlar: Vec<(String, String)>,
}

impl LogKaydi {
    /// Yeni kayıt; `mesaj` PII süzgecinden geçer, `zaman` = şimdi (UTC).
    pub fn yeni(seviye: LogSeverity, hedef: impl Into<String>, mesaj: impl AsRef<str>) -> Self {
        Self {
            zaman: chrono::Utc::now(),
            seviye,
            hedef: hedef.into(),
            mesaj: pii_temizle(mesaj.as_ref()),
            trace_id: None,
            span_id: None,
            alanlar: Vec::new(),
        }
    }

    /// Olay zamanını sabitler (golden/deterministik test için).
    pub fn with_zaman(mut self, zaman: Timestamp) -> Self {
        self.zaman = zaman;
        self
    }

    /// İz bağlamını (trace_id/span_id) iliştirir — uzun iş/dış çağrı ZORUNLU bunu taşır.
    pub fn with_iz(mut self, iz: &TraceContext) -> Self {
        self.trace_id = Some(iz.trace_id_hex());
        self.span_id = Some(iz.span_id_hex());
        self
    }

    /// Yapılandırılmış bir alan ekler; değer PII süzgecinden geçer.
    pub fn alan(mut self, anahtar: impl Into<String>, deger: impl AsRef<str>) -> Self {
        self.alanlar
            .push((anahtar.into(), pii_temizle(deger.as_ref())));
        self
    }

    /// **Hassas** bir alan: değer ASLA yazılmaz, yalnızca `<gizlendi>` görünür (MK-45).
    /// Yol/parola/hasta-kimliği gibi alanlar için kullanılır.
    pub fn hassas_alan(mut self, anahtar: impl Into<String>) -> Self {
        self.alanlar.push((anahtar.into(), GIZLENDI.to_string()));
        self
    }

    /// NDJSON satırı (OTel-uyumlu alanlar).  `serde_json` ile kaçışlanır → enjeksiyon güvenli.
    pub fn to_json(&self) -> String {
        let mut nesne = serde_json::Map::new();
        nesne.insert(
            "timestamp".into(),
            serde_json::Value::String(self.zaman.to_rfc3339()),
        );
        nesne.insert(
            "severity_text".into(),
            serde_json::Value::String(self.seviye.otel_metni().into()),
        );
        nesne.insert(
            "severity_number".into(),
            serde_json::Value::from(self.seviye.otel_numarasi()),
        );
        nesne.insert(
            "target".into(),
            serde_json::Value::String(self.hedef.clone()),
        );
        nesne.insert("body".into(), serde_json::Value::String(self.mesaj.clone()));
        if let Some(t) = &self.trace_id {
            nesne.insert("trace_id".into(), serde_json::Value::String(t.clone()));
        }
        if let Some(s) = &self.span_id {
            nesne.insert("span_id".into(), serde_json::Value::String(s.clone()));
        }
        if !self.alanlar.is_empty() {
            let mut oznitelik = serde_json::Map::new();
            for (k, v) in &self.alanlar {
                oznitelik.insert(k.clone(), serde_json::Value::String(v.clone()));
            }
            nesne.insert("attributes".into(), serde_json::Value::Object(oznitelik));
        }
        serde_json::Value::Object(nesne).to_string()
    }

    /// İnsan-okur tek satır (konsol için): `LVL [trace8] hedef: mesaj {alanlar}`.
    pub fn satir(&self) -> String {
        let iz = self
            .trace_id
            .as_deref()
            .map(|t| format!(" [{}]", &t[..t.len().min(8)]))
            .unwrap_or_default();
        let alan = if self.alanlar.is_empty() {
            String::new()
        } else {
            let cift: Vec<String> = self
                .alanlar
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            format!(" {{{}}}", cift.join(", "))
        };
        format!(
            "{:<5}{} {}: {}{}",
            self.seviye.otel_metni(),
            iz,
            self.hedef,
            self.mesaj,
            alan
        )
    }
}

/// Hassas değerlerin yerine yazılan sabit.
pub const GIZLENDI: &str = "<gizlendi>";

/// Bir metni loglanabilir hâle getirir: bilinen PII/PHI desenlerini maskeler (MK-45).
///
/// Maskelenen desenler (muhafazakâr; yanlış-pozitifi sınırlı tutar):
/// - **E-posta adresleri** → `<eposta>`
/// - **Kullanıcı ana dizini adı** (`C:\Users\<ad>\…`, `/home/<ad>/…`, `/Users/<ad>/…`) → `<kullanici>`
/// - **Uzun rakam dizileri** (≥9 hane; olası hasta/TC/telefon kimliği) → `<numara>`
///
/// Not: Bu, *son savunma katmanıdır*.  Asıl ilke, PHI'yi loga **hiç koymamaktır**
/// ([`LogKaydi::hassas_alan`]).  Süzgeç, kazara sızıntıyı yakalamak içindir.
pub fn pii_temizle(girdi: &str) -> String {
    let mut s = girdi.to_string();
    s = eposta_maskele(&s);
    s = ana_dizin_maskele(&s);
    s = uzun_rakam_maskele(&s);
    s
}

/// `kullanici@alan.tld` benzeri belirteçleri `<eposta>` ile değiştirir.
fn eposta_maskele(girdi: &str) -> String {
    let mut cikti = String::with_capacity(girdi.len());
    for kelime in ayir_koruyarak(girdi) {
        if eposta_mi(&kelime) {
            cikti.push_str("<eposta>");
        } else {
            cikti.push_str(&kelime);
        }
    }
    cikti
}

/// Bir belirtecin e-posta olup olmadığını kabaca denetler (tek `@`, öncesi/sonrası dolu, `.` var).
fn eposta_mi(k: &str) -> bool {
    let cekirdek = k.trim_matches(|c: char| !c.is_alphanumeric());
    let parca: Vec<&str> = cekirdek.split('@').collect();
    if parca.len() != 2 {
        return false;
    }
    let (yerel, alan) = (parca[0], parca[1]);
    !yerel.is_empty()
        && alan.contains('.')
        && alan.split('.').all(|p| !p.is_empty())
        && cekirdek
            .chars()
            .all(|c| c.is_alphanumeric() || "@.-_+".contains(c))
}

/// `…\Users\AD\…`, `…/home/AD/…`, `…/Users/AD/…` içindeki AD'yi `<kullanici>` yapar.
fn ana_dizin_maskele(girdi: &str) -> String {
    // Hem `\` hem `/` ayraçlarını ele al; yol parçaları üzerinde yürü.
    let mut sonuc = girdi.to_string();
    for belirtec in ["Users", "home"] {
        sonuc = ana_dizin_belirteci_maskele(&sonuc, belirtec);
    }
    sonuc
}

fn ana_dizin_belirteci_maskele(girdi: &str, belirtec: &str) -> String {
    let kucuk = girdi.to_lowercase();
    let hedef = belirtec.to_lowercase();
    let mut sonuc = String::with_capacity(girdi.len());
    let baytlar: Vec<char> = girdi.chars().collect();
    let alt: Vec<char> = kucuk.chars().collect();
    let mut i = 0;
    while i < baytlar.len() {
        // Ayraç + belirtec + ayraç + AD desenini ara.
        let ayrac = baytlar[i] == '/' || baytlar[i] == '\\';
        if ayrac && pencere_eslesir(&alt, i + 1, &hedef) {
            let bel_son = i + 1 + hedef.len();
            if bel_son < baytlar.len() && (baytlar[bel_son] == '/' || baytlar[bel_son] == '\\') {
                // Ayraç + belirtec + ayraç'ı olduğu gibi yaz.
                sonuc.push(baytlar[i]);
                sonuc.extend(&baytlar[i + 1..bel_son + 1]);
                // Sonraki yol parçası = kullanıcı adı → maskele.
                let mut j = bel_son + 1;
                let ad_basi = j;
                while j < baytlar.len() && baytlar[j] != '/' && baytlar[j] != '\\' {
                    j += 1;
                }
                if j > ad_basi {
                    sonuc.push_str("<kullanici>");
                }
                i = j;
                continue;
            }
        }
        sonuc.push(baytlar[i]);
        i += 1;
    }
    sonuc
}

fn pencere_eslesir(alt: &[char], basla: usize, hedef: &str) -> bool {
    let h: Vec<char> = hedef.chars().collect();
    if basla + h.len() > alt.len() {
        return false;
    }
    alt[basla..basla + h.len()] == h[..]
}

/// ≥9 ardışık rakamı `<numara>` ile değiştirir (olası kimlik/telefon/TC).
fn uzun_rakam_maskele(girdi: &str) -> String {
    let mut cikti = String::with_capacity(girdi.len());
    let mut tampon = String::new();
    let bosalt = |cikti: &mut String, tampon: &mut String| {
        if tampon.len() >= 9 {
            cikti.push_str("<numara>");
        } else {
            cikti.push_str(tampon);
        }
        tampon.clear();
    };
    for c in girdi.chars() {
        if c.is_ascii_digit() {
            tampon.push(c);
        } else {
            bosalt(&mut cikti, &mut tampon);
            cikti.push(c);
        }
    }
    bosalt(&mut cikti, &mut tampon);
    cikti
}

/// Boşluk sınırlarını koruyarak böler (maskeleme sonrası boşlukları yeniden kurmak için).
fn ayir_koruyarak(girdi: &str) -> Vec<String> {
    let mut parcalar = Vec::new();
    let mut gecerli = String::new();
    let mut bosluk = false;
    for c in girdi.chars() {
        if c.is_whitespace() {
            if !bosluk && !gecerli.is_empty() {
                parcalar.push(std::mem::take(&mut gecerli));
            }
            gecerli.push(c);
            bosluk = true;
        } else {
            if bosluk && !gecerli.is_empty() {
                parcalar.push(std::mem::take(&mut gecerli));
            }
            gecerli.push(c);
            bosluk = false;
        }
    }
    if !gecerli.is_empty() {
        parcalar.push(gecerli);
    }
    parcalar
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otel_severity_eslemesi() {
        assert_eq!(LogSeverity::Info.otel_numarasi(), 9);
        assert_eq!(LogSeverity::Error.otel_metni(), "ERROR");
        assert!(LogSeverity::Error > LogSeverity::Warn);
    }

    #[test]
    fn eposta_maskelenir() {
        let t = pii_temizle("kullanici tr.computer000@gmail.com ile giriş yaptı");
        assert!(!t.contains("gmail"));
        assert!(t.contains("<eposta>"));
        assert!(t.contains("giriş yaptı"));
    }

    #[test]
    fn ana_dizin_kullanici_adi_maskelenir() {
        let t = pii_temizle(r"Proje açıldı: C:\Users\Furkan\Desktop\proje");
        assert!(!t.contains("Furkan"));
        assert!(t.contains("<kullanici>"));
        assert!(t.contains("Desktop"));
        let u = pii_temizle("dosya /home/furkan/veri.vcf okunamadı");
        assert!(!u.contains("furkan"));
        assert!(u.contains("<kullanici>"));
        assert!(u.contains("veri.vcf"));
    }

    #[test]
    fn uzun_rakam_maskelenir_kisa_kalir() {
        let t = pii_temizle("hasta kimliği 123456789012 kaydedildi, sürüm 42");
        assert!(t.contains("<numara>"));
        assert!(!t.contains("123456789012"));
        // Kısa sayılar (sürüm 42) korunur.
        assert!(t.contains("42"));
    }

    #[test]
    fn hassas_alan_deger_yazmaz() {
        let k = LogKaydi::yeni(LogSeverity::Info, "test", "iş").hassas_alan("dosya_yolu");
        let j = k.to_json();
        assert!(j.contains("<gizlendi>"));
    }

    #[test]
    fn json_otel_alanlari_icerir() {
        let iz = TraceContext::kok();
        let k = LogKaydi::yeni(LogSeverity::Warn, "biocraft_data::project", "uyarı")
            .with_iz(&iz)
            .alan("dosya", "ornek.vcf");
        let j = k.to_json();
        assert!(j.contains("\"severity_text\":\"WARN\""));
        assert!(j.contains("\"severity_number\":13"));
        assert!(j.contains("\"trace_id\""));
        assert!(j.contains(&iz.trace_id_hex()));
        assert!(j.contains("\"attributes\""));
    }

    #[test]
    fn mesaj_ins_yapilandirilmis_pii_temizler() {
        let k = LogKaydi::yeni(
            LogSeverity::Error,
            "x",
            "hata: C:\\Users\\Gizli\\a.txt için e-posta a@b.com",
        );
        assert!(!k.mesaj.contains("Gizli"));
        assert!(!k.mesaj.contains("a@b.com"));
    }

    #[test]
    fn temiz_metin_degismeden_kalir() {
        let t = pii_temizle("GPU başlatıldı: backend=Vulkan, kare=16ms");
        assert_eq!(t, "GPU başlatıldı: backend=Vulkan, kare=16ms");
    }
}
