//! **Kimlik / sır saklama** — API anahtarı, parola, şifreleme anahtarı OS güvenli deposunda (MK-44).
//!
//! Sırlar (şifreleme anahtarı, NCBI API anahtarı, parola) **asla** düz metin dosyaya veya kodun
//! içine yazılmaz; işletim sisteminin güvenli deposunda tutulur:
//! - **Windows:** Credential Manager
//! - **macOS:** Keychain
//! - **Linux:** kernel keyutils (D-Bus/secret-service gerekmez)
//!
//! Saklama bir [`KimlikDeposu`] trait'i arkasındadır (proje deseni — `KaliciDepo`/`DonanimSensoru`
//! gibi): gerçek OS deposu [`OsKimlikDeposu`], testler için bellek-içi [`BellekKimlikDeposu`].
//! Böylece mantık CI'da gerçek OS keychain'i olmadan da test edilir; gerçek depo çalışma-zamanında
//! doğrulanır (wgpu/keyring deseni).
//!
//! Sırlar OS deposuna **onaltılık (hex)** dize olarak yazılır → ham ikili anahtarlar (AES-256) güvenle
//! saklanır, ek bağımlılık (base64) gerekmez.

use std::collections::HashMap;
use std::sync::Mutex;

use biocraft_types::ErrorReport;

/// BioCraft'ın OS güvenli deposundaki "servis" adı (Credential Manager'da bu adla görünür).
pub const SERVIS: &str = "BioCraftEngine";

/// Sır saklama soyutlaması — gerçek OS deposu veya test için bellek-içi.
///
/// `hesap`, tek servis altında birden çok sırrı ayırır (örn. `"proje:<id>:anahtar"`,
/// `"api:ncbi"`).  Sırlar ham bayttır (şifreleme anahtarı veya UTF-8 parola).
pub trait KimlikDeposu {
    /// Bir sırrı saklar (varsa üzerine yazar).
    fn sakla(&self, hesap: &str, sir: &[u8]) -> Result<(), ErrorReport>;
    /// Bir sırrı okur; yoksa `Ok(None)` (hata değil — "henüz kurulmadı" normal durumdur).
    fn al(&self, hesap: &str) -> Result<Option<Vec<u8>>, ErrorReport>;
    /// Bir sırrı siler; yoksa sessizce başarılı (idempotent).
    fn sil(&self, hesap: &str) -> Result<(), ErrorReport>;
}

// ─── Gerçek OS deposu (keyring) ────────────────────────────────────────────────

/// İşletim sisteminin güvenli kimlik deposunu kullanan gerçek uygulama (MK-44).
///
/// Çalışma-zamanında kullanılır; sır OS tarafından şifreli/korumalı tutulur.  Birim testlerinde
/// [`BellekKimlikDeposu`] kullanılır (CI'da OS keychain garantisi yok).
#[derive(Debug, Clone, Default)]
pub struct OsKimlikDeposu;

impl OsKimlikDeposu {
    /// Yeni bir OS kimlik deposu handle'ı.
    pub fn yeni() -> Self {
        Self
    }

    fn girdi(hesap: &str) -> Result<keyring::Entry, ErrorReport> {
        keyring::Entry::new(SERVIS, hesap)
            .map_err(|e| keyring_hatasi("Kimlik girdisi açılamadı", &e))
    }
}

impl KimlikDeposu for OsKimlikDeposu {
    fn sakla(&self, hesap: &str, sir: &[u8]) -> Result<(), ErrorReport> {
        let girdi = Self::girdi(hesap)?;
        girdi
            .set_password(&hex_kodla(sir))
            .map_err(|e| keyring_hatasi("Sır OS deposuna yazılamadı", &e))
    }

    fn al(&self, hesap: &str) -> Result<Option<Vec<u8>>, ErrorReport> {
        let girdi = Self::girdi(hesap)?;
        match girdi.get_password() {
            Ok(hex) => Ok(Some(hex_coz(&hex)?)),
            // Sır yoksa "yok" döndür (hata değil).
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(keyring_hatasi("Sır OS deposundan okunamadı", &e)),
        }
    }

    fn sil(&self, hesap: &str) -> Result<(), ErrorReport> {
        let girdi = Self::girdi(hesap)?;
        match girdi.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(keyring_hatasi("Sır OS deposundan silinemedi", &e)),
        }
    }
}

// ─── Bellek-içi test deposu ────────────────────────────────────────────────────

/// Bellek-içi sahte kimlik deposu (testler/CI için; OS keychain'i gerektirmez).
#[derive(Debug, Default)]
pub struct BellekKimlikDeposu {
    icerik: Mutex<HashMap<String, Vec<u8>>>,
}

impl BellekKimlikDeposu {
    /// Boş bir bellek deposu.
    pub fn yeni() -> Self {
        Self::default()
    }
}

impl KimlikDeposu for BellekKimlikDeposu {
    fn sakla(&self, hesap: &str, sir: &[u8]) -> Result<(), ErrorReport> {
        self.icerik
            .lock()
            .map_err(|_| kilit_hatasi())?
            .insert(hesap.to_string(), sir.to_vec());
        Ok(())
    }

    fn al(&self, hesap: &str) -> Result<Option<Vec<u8>>, ErrorReport> {
        Ok(self
            .icerik
            .lock()
            .map_err(|_| kilit_hatasi())?
            .get(hesap)
            .cloned())
    }

    fn sil(&self, hesap: &str) -> Result<(), ErrorReport> {
        self.icerik
            .lock()
            .map_err(|_| kilit_hatasi())?
            .remove(hesap);
        Ok(())
    }
}

// ─── Hex kodlama (ham anahtar baytlarını OS deposunda dize olarak saklamak için) ───

/// Baytları küçük-harf onaltılık dizeye çevirir (ek bağımlılık olmadan).
fn hex_kodla(bayt: &[u8]) -> String {
    let mut s = String::with_capacity(bayt.len() * 2);
    for b in bayt {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0x0f) as u32, 16).unwrap());
    }
    s
}

/// Onaltılık dizeyi baytlara çevirir; biçim bozuksa net hata (depo bozulması).
fn hex_coz(hex: &str) -> Result<Vec<u8>, ErrorReport> {
    let h = hex.trim();
    if h.len() % 2 != 0 {
        return Err(hex_hatasi());
    }
    let mut bayt = Vec::with_capacity(h.len() / 2);
    let ch: Vec<char> = h.chars().collect();
    for ikili in ch.chunks(2) {
        let yuksek = ikili[0].to_digit(16).ok_or_else(hex_hatasi)?;
        let dusuk = ikili[1].to_digit(16).ok_or_else(hex_hatasi)?;
        bayt.push(((yuksek << 4) | dusuk) as u8);
    }
    Ok(bayt)
}

// ─── Hata yardımcıları ─────────────────────────────────────────────────────────

fn keyring_hatasi(ne: &str, e: &keyring::Error) -> ErrorReport {
    ErrorReport::new(
        ne.to_string(),
        "İşletim sisteminin güvenli kimlik deposuna erişilemedi.",
        "Sistem kimlik yöneticisinin (Credential Manager/Keychain) çalıştığından emin olun.",
    )
    .with_teknik_detay(format!("keyring: {e}"))
}

fn hex_hatasi() -> ErrorReport {
    ErrorReport::new(
        "Kayıtlı sır bozuk",
        "OS deposundaki sır onaltılık biçimde değil; depo bozulmuş olabilir.",
        "Sırrı (ör. şifreleme anahtarını) yeniden kurmanız gerekebilir.",
    )
}

fn kilit_hatasi() -> ErrorReport {
    ErrorReport::new(
        "Kimlik deposu kilidi alınamadı",
        "Bellek-içi depo kilidi zehirlendi (eşzamanlılık hatası).",
        "Uygulamayı yeniden başlatın.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bellek_deposu_sakla_al_sil() {
        let depo = BellekKimlikDeposu::yeni();
        assert_eq!(depo.al("api:ncbi").unwrap(), None, "başta yok");

        depo.sakla("api:ncbi", b"gizli-anahtar-1234").unwrap();
        assert_eq!(
            depo.al("api:ncbi").unwrap().as_deref(),
            Some(&b"gizli-anahtar-1234"[..])
        );

        depo.sil("api:ncbi").unwrap();
        assert_eq!(depo.al("api:ncbi").unwrap(), None, "silindikten sonra yok");
    }

    #[test]
    fn olmayan_sir_silmek_hata_degil() {
        let depo = BellekKimlikDeposu::yeni();
        assert!(depo.sil("yok").is_ok(), "idempotent silme");
    }

    #[test]
    fn ikili_anahtar_korunur() {
        // 256-bit ham anahtar (hex gidiş-dönüşü tüm baytları korumalı).
        let depo = BellekKimlikDeposu::yeni();
        let anahtar: Vec<u8> = (0..32).map(|i| i as u8 ^ 0xA5).collect();
        depo.sakla("proje:1:anahtar", &anahtar).unwrap();
        assert_eq!(depo.al("proje:1:anahtar").unwrap().unwrap(), anahtar);
    }

    #[test]
    fn hex_gidis_donus_tum_baytlar() {
        let veri: Vec<u8> = (0..=255u16).map(|i| i as u8).collect();
        let kod = hex_kodla(&veri);
        assert_eq!(kod.len(), 512);
        assert_eq!(hex_coz(&kod).unwrap(), veri);
    }

    #[test]
    fn hex_bozuk_reddedilir() {
        assert!(hex_coz("xyz").is_err()); // tek sayı + geçersiz karakter
        assert!(hex_coz("0g").is_err()); // geçersiz onaltılık
    }
}
