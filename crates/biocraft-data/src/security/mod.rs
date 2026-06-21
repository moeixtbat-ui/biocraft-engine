//! **Çekirdek güvenlik katmanı** (İP-09) — şifreleme, kimlik saklama, bütünlük, güvenli silme.
//!
//! **Açık/kapalı ilkesi (önemli):** Bu modüldeki tüm veri-koruma kodu **açık ve denetlenebilirdir**
//! (Kerckhoffs ilkesi — MK-20).  Hassas veri emanet eden kullanıcı için güven, güvenliğin
//! görülebilir/denetlenebilir olmasından gelir.  *Kapalı olan* yalnızca lisans/aktivasyon ve
//! anti-tamper katmanıdır (ticari koruma) — o ayrı paketlenir, bu açık koddan ayrıdır ve bu MVP'de
//! yer almaz.
//!
//! Alt modüller:
//! - [`crypto`] — AES-256-GCM dinlenmede şifreleme (taze nonce, anahtar zeroize) — MK-44.
//! - [`credentials`] — sır/anahtar OS güvenli deposunda (Credential Manager/Keychain) — MK-44.
//! - [`secure_delete`] — güvenli silme (üzerine yazma + kaldırma; kripto-shred notu) — MK-45.
//! - [`integrity`] — imzalı güncelleme bütünlüğü (BLAKE3 + Ed25519; sahte güncelleme reddi).
//! - [`sanitize`] — log/PII temizleme (loglar hassas veri içermez) — MK-45.
//!
//! Üst seviye "şeffaf şifreleme" akışı ([`anahtar_kur_veya_yukle`] + [`crypto::sifrele`]): her proje
//! için rastgele bir veri anahtarı OS deposunda tutulur; hassas veri bu anahtarla şifrelenir
//! (TDA madde 9: şifreleme otomatik/şeffaf, kullanıcı sürtünmesi yok).

pub mod credentials;
pub mod crypto;
pub mod integrity;
pub mod sanitize;
pub mod secure_delete;

pub use credentials::{BellekKimlikDeposu, KimlikDeposu, OsKimlikDeposu};
pub use crypto::{coz, sifrele, SifreliVeri, VeriAnahtari};
pub use integrity::{guncelleme_dogrula, GuncellemeBildirimi, GuncellemeDurumu};
pub use sanitize::pii_temizle;
pub use secure_delete::{guvenli_dosya_sil, UzerineYazSecenek};

use biocraft_types::ErrorReport;

/// Bir hesap için OS deposundaki veri anahtarını yükler; yoksa **rastgele üretip saklar** ve döndürür.
///
/// Şeffaf şifrelemenin temeli: her proje/bağlam kendi `hesap` anahtarıyla (örn. `"proje:<id>:veri"`)
/// kalıcı bir 256-bit anahtara sahip olur.  İlk çağrıda üretilir + OS deposuna yazılır; sonraki
/// çağrılarda aynı anahtar geri yüklenir → eski şifreli veri çözülebilir kalır.
///
/// Anahtar **asla** kodda/düz metinde tutulmaz; yalnızca OS güvenli deposundadır (MK-44).
pub fn anahtar_kur_veya_yukle(
    depo: &dyn KimlikDeposu,
    hesap: &str,
) -> Result<VeriAnahtari, ErrorReport> {
    if let Some(bayt) = depo.al(hesap)? {
        return VeriAnahtari::baytlardan(&bayt);
    }
    let anahtar = VeriAnahtari::rastgele()?;
    depo.sakla(hesap, anahtar.baytlar())?;
    Ok(anahtar)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anahtar_kur_veya_yukle_kalici() {
        let depo = BellekKimlikDeposu::yeni();
        let hesap = "proje:test:veri";

        // İlk çağrı: anahtar üretilir + saklanır.
        let a1 = anahtar_kur_veya_yukle(&depo, hesap).unwrap();
        // İkinci çağrı: AYNI anahtar geri yüklenir (baytlar eşit).
        let a2 = anahtar_kur_veya_yukle(&depo, hesap).unwrap();
        assert_eq!(
            a1.baytlar(),
            a2.baytlar(),
            "anahtar oturumlar arası kalıcı olmalı"
        );
    }

    #[test]
    fn sakli_anahtarla_sifrele_coz_uctan_uca() {
        // Şeffaf şifreleme akışı: anahtar OS deposundan → şifrele → (yeniden yükle) → çöz.
        let depo = BellekKimlikDeposu::yeni();
        let hesap = "proje:hasta:veri";
        let mesaj = b"PHI: hasta dogum tarihi 1990-01-01, tani C50.9";

        let anahtar = anahtar_kur_veya_yukle(&depo, hesap).unwrap();
        let zarf = sifrele(&anahtar, mesaj).unwrap();

        // Yeni bir "oturum": anahtarı depodan tekrar yükle, çöz.
        let anahtar2 = anahtar_kur_veya_yukle(&depo, hesap).unwrap();
        assert_eq!(coz(&anahtar2, &zarf).unwrap(), mesaj);
    }
}
