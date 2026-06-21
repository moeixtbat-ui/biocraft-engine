//! **Dinlenmede şifreleme (AES-256-GCM)** — hassas veri diske şifreli yazılır (MK-44, İP-09).
//!
//! Tasarım (açık/denetlenebilir — Kerckhoffs ilkesi, İP-09):
//! - **Algoritma:** AES-256-GCM (kimlik-doğrulamalı şifreleme; gizlilik **ve** bütünlük birlikte).
//!   256-bit anahtar; bozulmuş/değiştirilmiş şifreli metin çözmede **reddedilir** (GCM etiketi tutmaz).
//! - **Nonce (96-bit):** **her şifrelemede TAZE rastgele** üretilir (OS CSPRNG, [`getrandom`]) ve
//!   şifreli metnin başına eklenir.  Aynı anahtarla bir nonce'un **asla** tekrar etmemesi GCM'in
//!   güvenliği için kritiktir; sayaç/deterministik nonce yerine rastgele nonce kullanılır → tekrar yok.
//! - **Anahtar saklama:** Anahtar **kodun içinde DEĞİL**; rastgele üretilir ve OS güvenli deposunda
//!   tutulur ([`crate::security::credentials`]).  Anahtar baytları kullanım sonrası bellekten
//!   **sıfırlanır** ([`zeroize`]) — RAM dökümünde iz bırakmaz (MK-45).
//!
//! Bu modül salt kripto çekirdeğidir (anahtar + şifrele/çöz); anahtarın OS deposuna yazımı
//! `credentials.rs`'tedir, üst seviye "şifrele→sakla→çöz" akışı `mod.rs`'tedir.

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use biocraft_types::ErrorReport;

/// AES-256 anahtarının bayt uzunluğu (256 bit).
pub const ANAHTAR_BAYT: usize = 32;
/// AES-GCM nonce uzunluğu (96 bit — GCM standardı).
pub const NONCE_BAYT: usize = 12;

/// 256-bit bir veri şifreleme anahtarı.  Düşürüldüğünde baytlar bellekten **sıfırlanır** (MK-45).
///
/// `Debug`/`Clone` **bilinçli olarak türetilmedi**: anahtarın yanlışlıkla loglanması veya
/// gereksiz kopyalanması (RAM'de çoğalması) engellenir.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct VeriAnahtari([u8; ANAHTAR_BAYT]);

impl VeriAnahtari {
    /// OS CSPRNG'inden **rastgele** yeni bir 256-bit anahtar üretir (yeni veri/proje için).
    pub fn rastgele() -> Result<Self, ErrorReport> {
        let mut bayt = [0u8; ANAHTAR_BAYT];
        getrandom::getrandom(&mut bayt).map_err(|e| csprng_hatasi(&e.to_string()))?;
        Ok(Self(bayt))
    }

    /// Ham 32 baytlık bir anahtardan kurar (OS deposundan geri yükleme için).
    ///
    /// Uzunluk 32 değilse net hata döner (sessizce kırpmaz/doldurmaz).
    pub fn baytlardan(bayt: &[u8]) -> Result<Self, ErrorReport> {
        let dizi: [u8; ANAHTAR_BAYT] = bayt.try_into().map_err(|_| {
            ErrorReport::new(
                "Şifreleme anahtarı geçersiz",
                format!(
                    "Anahtar {} bayt olmalı ama {} bayt geldi — depo bozulmuş olabilir.",
                    ANAHTAR_BAYT,
                    bayt.len()
                ),
                "Anahtarı yeniden üretmeniz gerekebilir; bu, eski şifreli veriyi çözememe anlamına gelir.",
            )
        })?;
        Ok(Self(dizi))
    }

    /// Anahtar baytlarına ödünç erişim (OS deposuna yazmak için — `credentials.rs`).
    ///
    /// Dikkat: bu baytları kopyalayıp uzun süre tutmayın; iş bitince [`zeroize`]'layın.
    pub fn baytlar(&self) -> &[u8; ANAHTAR_BAYT] {
        &self.0
    }
}

/// Şifreli bir veri zarfı: `nonce` (açık) + `sifreli_metin` (GCM etiketi dâhil).
///
/// Nonce gizli değildir (yalnızca tekrarsız olmalı); zarfla birlikte saklanır.  Serde ile
/// diske/db'ye yazılabilir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SifreliVeri {
    /// Bu şifrelemeye özgü 96-bit nonce (her çağrıda taze).
    pub nonce: [u8; NONCE_BAYT],
    /// AES-256-GCM şifreli metin (sonunda 16 baytlık kimlik-doğrulama etiketi).
    pub sifreli_metin: Vec<u8>,
}

impl SifreliVeri {
    /// Zarfı tek bir bayt dizisine düzleştirir: `nonce || sifreli_metin` (diske yazım için).
    pub fn duz_bayt(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(NONCE_BAYT + self.sifreli_metin.len());
        v.extend_from_slice(&self.nonce);
        v.extend_from_slice(&self.sifreli_metin);
        v
    }

    /// `nonce || sifreli_metin` düz baytlarından zarfı geri kurar.
    pub fn duz_bayttan(ham: &[u8]) -> Result<Self, ErrorReport> {
        if ham.len() < NONCE_BAYT {
            return Err(bozuk_sifreli_hatasi("Şifreli veri çok kısa (nonce eksik)"));
        }
        let mut nonce = [0u8; NONCE_BAYT];
        nonce.copy_from_slice(&ham[..NONCE_BAYT]);
        Ok(Self {
            nonce,
            sifreli_metin: ham[NONCE_BAYT..].to_vec(),
        })
    }
}

/// `duz_metin`'i AES-256-GCM ile şifreler; **taze rastgele nonce** üretip zarfa koyar (MK-44).
pub fn sifrele(anahtar: &VeriAnahtari, duz_metin: &[u8]) -> Result<SifreliVeri, ErrorReport> {
    // Her şifreleme için BENZERSİZ nonce — aynı anahtarla tekrar ASLA olmamalı (GCM güvenliği).
    let mut nonce_bayt = [0u8; NONCE_BAYT];
    getrandom::getrandom(&mut nonce_bayt).map_err(|e| csprng_hatasi(&e.to_string()))?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(anahtar.baytlar()));
    let nonce = Nonce::from_slice(&nonce_bayt);
    let sifreli_metin = cipher
        .encrypt(nonce, duz_metin)
        .map_err(|_| sifreleme_hatasi())?;

    Ok(SifreliVeri {
        nonce: nonce_bayt,
        sifreli_metin,
    })
}

/// Bir [`SifreliVeri`] zarfını çözer.  Anahtar yanlış **veya** veri değiştirilmişse (GCM etiketi
/// tutmaz) net hata döner — bozuk/sahte şifreli metin **sessizce kabul edilmez** (MK-44 bütünlük).
pub fn coz(anahtar: &VeriAnahtari, zarf: &SifreliVeri) -> Result<Vec<u8>, ErrorReport> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(anahtar.baytlar()));
    let nonce = Nonce::from_slice(&zarf.nonce);
    cipher
        .decrypt(nonce, zarf.sifreli_metin.as_ref())
        .map_err(|_| cozme_hatasi())
}

// ─── Hata yardımcıları (İP-16 standart şema) ───────────────────────────────────

fn csprng_hatasi(detay: &str) -> ErrorReport {
    ErrorReport::new(
        "Güvenli rastgelelik alınamadı",
        "İşletim sisteminin rastgele sayı üreteci okunamadı; şifreleme güvenli yapılamaz.",
        "Uygulamayı yeniden başlatın; sorun sürerse sistem güncellemelerini kontrol edin.",
    )
    .with_teknik_detay(format!("getrandom: {detay}"))
}

fn sifreleme_hatasi() -> ErrorReport {
    ErrorReport::new(
        "Veri şifrelenemedi",
        "Şifreleme işlemi başarısız oldu (beklenmeyen iç durum).",
        "Tekrar deneyin; sorun sürerse veriyi yedekleyip geliştiriciye bildirin.",
    )
}

fn cozme_hatasi() -> ErrorReport {
    ErrorReport::new(
        "Şifreli veri çözülemedi",
        "Anahtar yanlış veya veri değiştirilmiş/bozulmuş olabilir (kimlik doğrulama tutmadı).",
        "Doğru anahtarla açtığınızdan emin olun; veri bozuksa yedekten geri yükleyin.",
    )
    .with_teknik_detay("AES-256-GCM kimlik-doğrulama etiketi doğrulanamadı".to_string())
}

fn bozuk_sifreli_hatasi(neden: &str) -> ErrorReport {
    ErrorReport::new(
        "Şifreli veri bozuk",
        neden.to_string(),
        "Veriyi güvenilir bir yedekten geri yükleyin.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sifrele_coz_gidis_donus() {
        let anahtar = VeriAnahtari::rastgele().unwrap();
        let mesaj = b"hassas hasta verisi: BRCA1 c.68_69delAG";
        let zarf = sifrele(&anahtar, mesaj).unwrap();
        // Şifreli metin düz metinden farklı olmalı (gerçekten şifrelendi).
        assert_ne!(zarf.sifreli_metin.as_slice(), mesaj.as_slice());
        let cozulen = coz(&anahtar, &zarf).unwrap();
        assert_eq!(cozulen, mesaj);
    }

    #[test]
    fn ayni_mesaj_farkli_nonce_farkli_ciktilar() {
        // Taze nonce → aynı düz metin iki kez şifrelenince şifreli metinler FARKLI olmalı.
        let anahtar = VeriAnahtari::rastgele().unwrap();
        let a = sifrele(&anahtar, b"tekrar eden mesaj").unwrap();
        let b = sifrele(&anahtar, b"tekrar eden mesaj").unwrap();
        assert_ne!(
            a.nonce, b.nonce,
            "nonce her çağrıda taze olmalı (tekrar yok)"
        );
        assert_ne!(a.sifreli_metin, b.sifreli_metin);
    }

    #[test]
    fn yanlis_anahtar_cozemez() {
        let a1 = VeriAnahtari::rastgele().unwrap();
        let a2 = VeriAnahtari::rastgele().unwrap();
        let zarf = sifrele(&a1, b"gizli").unwrap();
        assert!(coz(&a2, &zarf).is_err(), "yanlış anahtar çözmemeli");
    }

    #[test]
    fn degistirilmis_sifreli_metin_reddedilir() {
        // GCM kimlik doğrulaması: tek bit değişse bile çözme reddedilmeli (bütünlük).
        let anahtar = VeriAnahtari::rastgele().unwrap();
        let mut zarf = sifrele(&anahtar, b"degistirilemez veri").unwrap();
        zarf.sifreli_metin[0] ^= 0x01;
        assert!(
            coz(&anahtar, &zarf).is_err(),
            "kurcalanmış veri reddedilmeli"
        );
    }

    #[test]
    fn duz_bayt_gidis_donus() {
        let anahtar = VeriAnahtari::rastgele().unwrap();
        let zarf = sifrele(&anahtar, b"duzlestirme testi").unwrap();
        let ham = zarf.duz_bayt();
        let geri = SifreliVeri::duz_bayttan(&ham).unwrap();
        assert_eq!(zarf, geri);
        assert_eq!(coz(&anahtar, &geri).unwrap(), b"duzlestirme testi");
    }

    #[test]
    fn kisa_bayt_nonce_eksik_hata() {
        assert!(SifreliVeri::duz_bayttan(&[0u8; 5]).is_err());
    }

    #[test]
    fn anahtar_baytlardan_yanlis_uzunluk_hata() {
        assert!(VeriAnahtari::baytlardan(&[0u8; 10]).is_err());
        assert!(VeriAnahtari::baytlardan(&[7u8; ANAHTAR_BAYT]).is_ok());
    }
}
