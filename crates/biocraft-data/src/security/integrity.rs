//! **İmzalı güncelleme bütünlüğü** — sahte/değiştirilmiş güncelleme reddedilir (İP-09 / İP-18).
//!
//! Auto-updater iki bağımsız güvenceyle korunur (eklenti imzasıyla aynı fikir — `plugin-host`'taki
//! `signature.rs`):
//! 1. **BLAKE3 checksum:** indirilen paket baytları, imzalı bildirimde yazan özetle birebir eşleşmeli
//!    (eksik/değişmiş/bozuk indirme yakalanır — MK-33).
//! 2. **Ed25519 imza:** bildirimi gerçekten BioCraft'ın **resmi yayın anahtarı** mı imzaladı?
//!    (saldırgan paketi değiştirip checksum'ı güncelleyemez; imza tutmaz → **reddedilir**.)
//!
//! Bu modül salt **doğrulama mantığıdır** (saf, test-edilebilir).  Gerçek indirme + güvenli kanal
//! (TLS) + "yeniden başlat" akışı üst katmanda (`biocraft-app` auto-updater, İP-18) bunun üstüne kurulur;
//! ağ/TLS insan-eli/altyapı işidir (bkz. `Hukuk-ve-Operasyon.md`).  İmzalama/doğrulama RFC 8032 gereği
//! deterministiktir → CI'da harici entropi/C derleyici gerekmez.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use biocraft_types::ErrorReport;

/// Bir güncelleme paketini tanımlayan, **imzalanan** bildirim (manifest).
///
/// Bildirimin **kanonik JSON**'u imzalanır (`kanonik`).  `blake3_hex`, indirilen paket baytlarının
/// beklenen özetidir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuncellemeBildirimi {
    /// Güncellemenin sürümü (örn. "1.2.0").
    pub surum: String,
    /// Güncelleme paketinin (indirilecek baytların) BLAKE3 özeti (64 hane onaltılık).
    pub blake3_hex: String,
    /// Paket boyutu (bayt) — indirme öncesi disk/ön denetim için.
    pub boyut: u64,
}

impl GuncellemeBildirimi {
    /// İmzalanacak/doğrulanacak **kanonik** bayt temsili (alan sırası sabit → imza kararlı).
    pub fn kanonik(&self) -> Vec<u8> {
        // serde_json nesne alanlarını struct sırasıyla yazar; sürüm/özet/boyut sabit → kararlı.
        serde_json::to_vec(self).unwrap_or_default()
    }
}

/// Bir güncelleme doğrulamasının sonucu (UI'a anlamlı durum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuncellemeDurumu {
    /// İmza **ve** checksum geçti — güncelleme güvenle uygulanabilir.
    Gecerli,
}

/// İndirilen güncelleme paketini doğrular: **imza** (resmi anahtar) + **checksum** (BLAKE3).
///
/// * `paket_baytlari` — indirilen güncelleme paketinin ham içeriği.
/// * `bildirim`       — paketle gelen (imzalı) bildirim.
/// * `imza`           — bildirimin kanonik baytları üzerine Ed25519 imzası (64 bayt).
/// * `resmi_anahtar`  — çekirdeğe gömülü **resmi yayın açık anahtarı** (32 bayt).
///
/// Sıra (en ucuz/en kesin reddi önce): imza → checksum.  Herhangi biri tutmazsa **net hata** ve
/// güncelleme **uygulanmaz** (sahte/değiştirilmiş güncelleme reddi — kabul kriteri).
pub fn guncelleme_dogrula(
    paket_baytlari: &[u8],
    bildirim: &GuncellemeBildirimi,
    imza: &[u8],
    resmi_anahtar: &[u8],
) -> Result<GuncellemeDurumu, ErrorReport> {
    // 1) İmza: bildirimi resmi anahtar mı imzaladı?
    let vk_bayt: [u8; 32] = resmi_anahtar
        .try_into()
        .map_err(|_| imza_hatasi("Resmi anahtar geçersiz (32 bayt olmalı)"))?;
    let vk = VerifyingKey::from_bytes(&vk_bayt)
        .map_err(|_| imza_hatasi("Resmi anahtar bozuk (Ed25519 eğrisinde değil)"))?;
    let sig_bayt: [u8; 64] = imza
        .try_into()
        .map_err(|_| imza_hatasi("İmza geçersiz (64 bayt olmalı)"))?;
    let sig = Signature::from_bytes(&sig_bayt);
    vk.verify(&bildirim.kanonik(), &sig).map_err(|_| {
        imza_hatasi("Güncelleme imzası doğrulanamadı — paket sahte veya değiştirilmiş")
    })?;

    // 2) Checksum: indirilen baytlar bildirimdeki özetle eşleşiyor mu?
    let gercek = blake3::hash(paket_baytlari);
    if !gercek
        .to_hex()
        .as_str()
        .eq_ignore_ascii_case(bildirim.blake3_hex.trim())
    {
        return Err(checksum_hatasi());
    }

    Ok(GuncellemeDurumu::Gecerli)
}

fn imza_hatasi(detay: &str) -> ErrorReport {
    ErrorReport::new(
        "Güncelleme reddedildi (imza)",
        "Güncelleme paketinin imzası BioCraft'ın resmi anahtarıyla doğrulanamadı; paket sahte veya \
         indirilirken değiştirilmiş olabilir.",
        "Güncellemeyi yalnızca resmi kaynaktan indirin; sorun sürerse bu güncellemeyi atlayın.",
    )
    .with_teknik_detay(detay.to_string())
}

fn checksum_hatasi() -> ErrorReport {
    ErrorReport::new(
        "Güncelleme reddedildi (bütünlük)",
        "İndirilen güncelleme paketinin BLAKE3 özeti beklenen değerle uyuşmuyor; indirme eksik/bozuk \
         veya değiştirilmiş.",
        "İnternet bağlantınızı kontrol edip güncellemeyi yeniden indirin.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    /// Deterministik bir test anahtar çifti (harici entropi yok).
    fn anahtar_cifti() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn bildirim_imzala(
        paket: &[u8],
        surum: &str,
        anahtar: &SigningKey,
    ) -> (GuncellemeBildirimi, Vec<u8>, Vec<u8>) {
        let bildirim = GuncellemeBildirimi {
            surum: surum.to_string(),
            blake3_hex: blake3::hash(paket).to_hex().to_string(),
            boyut: paket.len() as u64,
        };
        let imza: Signature = anahtar.sign(&bildirim.kanonik());
        (
            bildirim,
            imza.to_bytes().to_vec(),
            anahtar.verifying_key().to_bytes().to_vec(),
        )
    }

    #[test]
    fn gecerli_guncelleme_kabul() {
        let paket = b"BioCraft Engine 1.2.0 ikili icerigi...";
        let k = anahtar_cifti();
        let (bildirim, imza, vk) = bildirim_imzala(paket, "1.2.0", &k);
        assert_eq!(
            guncelleme_dogrula(paket, &bildirim, &imza, &vk).unwrap(),
            GuncellemeDurumu::Gecerli
        );
    }

    #[test]
    fn degistirilmis_paket_reddedilir() {
        // Saldırgan paketi değiştirdi ama checksum bildirimde sabit → checksum tutmaz.
        let paket = b"orijinal paket";
        let k = anahtar_cifti();
        let (bildirim, imza, vk) = bildirim_imzala(paket, "1.0.0", &k);
        let sahte = b"kotu amacli paket!!";
        let hata = guncelleme_dogrula(sahte, &bildirim, &imza, &vk).unwrap_err();
        assert!(hata.ne_oldu.contains("bütünlük"));
    }

    #[test]
    fn yanlis_anahtarla_imza_reddedilir() {
        // Saldırgan kendi anahtarıyla imzaladı ama resmi anahtar farklı → imza tutmaz.
        let paket = b"paket";
        let saldirgan = SigningKey::from_bytes(&[9u8; 32]);
        let (bildirim, imza, _) = bildirim_imzala(paket, "1.0.0", &saldirgan);
        let resmi = anahtar_cifti().verifying_key().to_bytes().to_vec();
        let hata = guncelleme_dogrula(paket, &bildirim, &imza, &resmi).unwrap_err();
        assert!(hata.ne_oldu.contains("imza"));
    }

    #[test]
    fn kurcalanmis_bildirim_reddedilir() {
        // Bildirim (sürüm) değiştirilirse kanonik baytlar değişir → imza tutmaz.
        let paket = b"paket";
        let k = anahtar_cifti();
        let (mut bildirim, imza, vk) = bildirim_imzala(paket, "1.0.0", &k);
        bildirim.surum = "9.9.9".to_string(); // imzadan sonra değiştir
        assert!(guncelleme_dogrula(paket, &bildirim, &imza, &vk).is_err());
    }

    #[test]
    fn bozuk_uzunluklar_net_hata() {
        let paket = b"x";
        let k = anahtar_cifti();
        let (bildirim, imza, vk) = bildirim_imzala(paket, "1.0.0", &k);
        assert!(guncelleme_dogrula(paket, &bildirim, &imza[..10], &vk).is_err());
        assert!(guncelleme_dogrula(paket, &bildirim, &imza, &vk[..10]).is_err());
    }
}
