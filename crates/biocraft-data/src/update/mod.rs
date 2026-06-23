//! **Güncelleme motoru** (İP-20, MK-56) — imzalı + delta + atomik + geri alınabilir auto-update'in
//! saf, test-edilebilir çekirdeği (L2; egui/ağ yok).
//!
//! Bu modül güncelleme akışının *karar ve bütünlük* mantığıdır; ağ indirme + TLS + "yeniden başlat"
//! akışı + kullanıcı onayı **üst katmanda** (`biocraft-app/src/update`) bunun üstüne kurulur.
//! Üç güvence iç içe çalışır:
//!
//! 1. **İmza (Ed25519):** Geniş bildirim (sürüm + özet + boyut + kanal + changelog) BioCraft'ın
//!    **resmi yayın anahtarıyla** imzalanmıştır.  Changelog/kanal da imzaya dâhildir → sahte
//!    "yenilikler" metni enjekte edilemez (İP-09 bütünlük ilkesiyle aynı çizgi).
//! 2. **Bütünlük (BLAKE3):** İndirilen/yeniden üretilen paket baytları bildirimdeki özetle birebir.
//! 3. **Atomiklik + geri alma:** Yeni sürüm staging'de doğrulanıp atomik takas edilir; yarıda
//!    kalırsa eski sürüm korunur, başarısızsa otomatik geri alınır ([`rollback`]).
//!
//! Ayrıca **delta** ([`delta`]) ile yalnızca değişen parça indirilir; taban uyuşmazsa motor tam
//! pakete düşer.  **Kanallar** ([`SurumKanali`]) kararlı kullanıcının beta/nightly güncellemesini
//! *görmemesini* sağlar.

pub mod delta;
pub mod rollback;

use std::str::FromStr;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use biocraft_types::{ErrorReport, Version};

pub use delta::{DeltaOp, DeltaYama};
pub use rollback::{AtomikKurulum, GeriAlSonuc};

// İmzalı temel bildirim İP-09'da (security::integrity) tanımlı; geniş bildirim onu sarmalar.
pub use crate::security::integrity::GuncellemeBildirimi;

/// Güncelleme yayın kanalı — kullanıcı bir kanala abone olur; başka kanalın güncellemesini görmez.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SurumKanali {
    /// Üretim — varsayılan; yalnız kararlı sürümler.
    #[default]
    Kararli,
    /// Erken erişim — kararlı + beta.
    Beta,
    /// Gecelik — tüm sürümler (en riskli).
    Nightly,
}

impl SurumKanali {
    /// İnsan-okur etiket (UI/changelog).
    pub fn etiket(&self, tr: bool) -> &'static str {
        match (self, tr) {
            (SurumKanali::Kararli, true) => "Kararlı",
            (SurumKanali::Kararli, false) => "Stable",
            (SurumKanali::Beta, true) => "Beta",
            (SurumKanali::Beta, false) => "Beta",
            (SurumKanali::Nightly, true) => "Gecelik",
            (SurumKanali::Nightly, false) => "Nightly",
        }
    }

    /// Bu kanala abone bir kullanıcı, `yayin` kanalında çıkan bir güncellemeyi **görür mü?**
    ///
    /// Kararlı yalnız kararlıyı; Beta kararlı+beta'yı; Gecelik hepsini görür (riziko sırası).
    pub fn gorur_mu(&self, yayin: SurumKanali) -> bool {
        match self {
            SurumKanali::Kararli => yayin == SurumKanali::Kararli,
            SurumKanali::Beta => matches!(yayin, SurumKanali::Kararli | SurumKanali::Beta),
            SurumKanali::Nightly => true,
        }
    }
}

/// İki sürüm metnini SemVer'e göre karşılaştırıp `aday > mevcut` mi söyler (downgrade tespiti için
/// [`surum_yonu`] kullanılır).
pub fn daha_yeni_mi(mevcut: &str, aday: &str) -> Result<bool, ErrorReport> {
    Ok(surum(aday)? > surum(mevcut)?)
}

/// Bir adayın mevcut sürüme göre yönü.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurumYon {
    /// Aday daha yeni — yükseltme.
    Yukseltme,
    /// Aday aynı sürüm — gereksiz.
    AyniSurum,
    /// Aday daha eski — downgrade (güvenlik ağı; izinli ama işaretlenir).
    Indirme,
}

/// `mevcut` → `aday` yönünü hesaplar.
pub fn surum_yonu(mevcut: &str, aday: &str) -> Result<SurumYon, ErrorReport> {
    let (m, a) = (surum(mevcut)?, surum(aday)?);
    Ok(match a.cmp(&m) {
        std::cmp::Ordering::Greater => SurumYon::Yukseltme,
        std::cmp::Ordering::Equal => SurumYon::AyniSurum,
        std::cmp::Ordering::Less => SurumYon::Indirme,
    })
}

fn surum(s: &str) -> Result<Version, ErrorReport> {
    Version::from_str(s).map_err(|e| {
        ErrorReport::new(
            "Geçersiz sürüm numarası",
            format!("'{s}' geçerli bir SemVer (ana.alt.yama) değil: {e}"),
            "Güncelleme bildirimini resmi kaynaktan yeniden alın.",
        )
    })
}

/// **İmzalanan** geniş güncelleme bildirimi: temel (sürüm/özet/boyut) + kanal + changelog.
///
/// `kanonik()`'in tamamı imzalanır → changelog ve kanal da kurcalamaya karşı korunur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenisBildirim {
    /// İmzalı temel bütünlük bildirimi (sürüm + nihai paketin BLAKE3 özeti + boyut) — İP-09.
    pub temel: GuncellemeBildirimi,
    /// Bu sürümün yayınlandığı kanal (kullanıcının kanalı bunu görmüyorsa güncelleme sunulmaz).
    #[serde(default)]
    pub kanal: SurumKanali,
    /// Kullanıcı-dilinde "Yenilikler" metni (launcher + güncelleme onayında gösterilir).
    #[serde(default)]
    pub changelog: String,
    /// Minimum kurulu sürüm (delta bu sürümden itibaren geçerli); bilgi amaçlı.
    #[serde(default)]
    pub asgari_surum: Option<String>,
}

impl GenisBildirim {
    /// İmzalanacak/doğrulanacak **kanonik** baytlar (alan sırası sabit → imza kararlı).
    pub fn kanonik(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Bildirimdeki sürüm.
    pub fn surum(&self) -> &str {
        &self.temel.surum
    }
}

/// Bir güncellemenin taşıdığı yük: tam paket ya da delta yaması.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuncellemePaketi {
    /// Tüm paket baytları (delta yok ya da taban uyuşmadı).
    Tam(Vec<u8>),
    /// Yalnız değişen parça — kurulu paketin baytlarına uygulanır.
    Delta(DeltaYama),
}

/// Başarılı bir güncellemenin sonucu (UI raporu).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuncellemeRaporu {
    /// Güncellemeden önceki aktif sürüm.
    pub eski_surum: String,
    /// Güncellemeden sonraki aktif sürüm.
    pub yeni_surum: String,
    /// Yön (yükseltme/aynı/downgrade).
    pub yon: SurumYon,
    /// Delta mı kullanıldı (false = tam paket).
    pub delta_mi: bool,
    /// Nihai paketin boyutu (bayt).
    pub paket_boyut: u64,
}

/// Bir güncellemeyi **uçtan uca** uygular: imza + bütünlük doğrula → (delta ise) birleştir →
/// atomik takas + (gerekirse) geri alma.
///
/// * `kurulum`        — atomik kurulum kökü (current/previous/staging).
/// * `paket`          — tam paket veya delta yaması.
/// * `bildirim`       — imzalı geniş bildirim (nihai paketin özeti/boyutu burada).
/// * `imza`           — `bildirim.kanonik()` üzerine Ed25519 imzası (64 bayt).
/// * `resmi_anahtar`  — çekirdeğe gömülü resmi yayın açık anahtarı (32 bayt).
/// * `kanal`          — kullanıcının abone olduğu kanal (yayın kanalı görünmüyorsa reddedilir).
///
/// **Atomik garanti:** Bu fonksiyon `Err` dönerse aktif kurulum (current) **değişmemiştir**;
/// kullanıcı eski sürümle çalışmaya devam eder.
pub fn guncellemeyi_uygula(
    kurulum: &AtomikKurulum,
    paket: GuncellemePaketi,
    bildirim: &GenisBildirim,
    imza: &[u8],
    resmi_anahtar: &[u8],
    kanal: SurumKanali,
) -> Result<GuncellemeRaporu, ErrorReport> {
    // 0) Kanal kapısı: kullanıcı bu yayın kanalını görmüyorsa güncelleme sunulmaz.
    if !kanal.gorur_mu(bildirim.kanal) {
        return Err(ErrorReport::new(
            "Güncelleme bu kanalda sunulmuyor",
            format!(
                "Güncelleme '{}' kanalında yayınlandı; siz '{}' kanalındasınız.",
                bildirim.kanal.etiket(true),
                kanal.etiket(true)
            ),
            "Daha erken sürümler için Ayarlar'dan kanalı değiştirebilirsiniz (önerilmez).",
        ));
    }

    // 1) İmza: geniş bildirimi resmi anahtar mı imzaladı? (changelog/kanal dâhil korunur.)
    imza_dogrula(bildirim, imza, resmi_anahtar)?;

    // 2) Nihai paket baytlarını elde et (delta ise kurulu paketten birleştir).
    let (nihai, delta_mi) = match paket {
        GuncellemePaketi::Tam(b) => (b, false),
        GuncellemePaketi::Delta(yama) => {
            let mevcut = kurulum.gecerli_dosya("paket.bin").ok_or_else(|| {
                ErrorReport::new(
                    "Delta uygulanamadı (kurulu paket yok)",
                    "Delta güncelleme için tabanı oluşturan kurulu paket bulunamadı.",
                    "Tam (delta olmayan) güncelleme paketini indirin.",
                )
            })?;
            (delta::uygula(&mevcut, &yama)?, true)
        }
    };

    // 3) Bütünlük: nihai paket bildirimdeki özet + boyutla birebir mi?
    if nihai.len() as u64 != bildirim.temel.boyut {
        return Err(ErrorReport::new(
            "Güncelleme reddedildi (boyut)",
            "İndirilen paketin boyutu imzalı bildirimdekiyle uyuşmuyor.",
            "Güncellemeyi yeniden indirin.",
        ));
    }
    let ozet = blake3::hash(&nihai);
    if !ozet
        .to_hex()
        .as_str()
        .eq_ignore_ascii_case(bildirim.temel.blake3_hex.trim())
    {
        return Err(ErrorReport::new(
            "Güncelleme reddedildi (bütünlük)",
            "İndirilen paketin BLAKE3 özeti imzalı bildirimle uyuşmuyor; paket eksik/bozuk veya \
             değiştirilmiş.",
            "İnternet bağlantınızı kontrol edip güncellemeyi yeniden indirin.",
        ));
    }

    // 4) Yön (downgrade işaretlenir ama izinlidir — güvenlik ağı / kullanıcı isteği).
    let eski_surum = kurulum
        .gecerli_surum()
        .unwrap_or_else(|| "0.0.0".to_string());
    let yon = surum_yonu(&eski_surum, bildirim.surum())?;

    // 5) Atomik uygula (staging→doğrula→takas; başarısızsa current dokunulmaz).
    let dosyalar: Vec<(&str, Vec<u8>)> = vec![
        ("paket.bin", nihai.clone()),
        ("CHANGELOG.txt", bildirim.changelog.clone().into_bytes()),
    ];
    kurulum.uygula(bildirim.surum(), &dosyalar)?;

    Ok(GuncellemeRaporu {
        eski_surum,
        yeni_surum: bildirim.surum().to_string(),
        yon,
        delta_mi,
        paket_boyut: nihai.len() as u64,
    })
}

/// `bildirim.kanonik()` üzerine Ed25519 imzasını resmi anahtara karşı doğrular.
pub fn imza_dogrula(
    bildirim: &GenisBildirim,
    imza: &[u8],
    resmi_anahtar: &[u8],
) -> Result<(), ErrorReport> {
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
    })
}

/// **Yalnız demo/test için:** bir nihai paketi verilen tohum anahtarla imzalar; `(geniş bildirim,
/// imza[64], açık anahtar[32])` döndürür.  Gerçek yayında imzalama insan-eli/CI gizli anahtarıyla
/// yapılır (`Hukuk-ve-Operasyon.md`); bu yardımcı app'in `--update-demo` yüzeyi + testleri içindir →
/// böylece üst katmanların doğrudan kripto bağımlılığı olmaz.
pub fn demo_imzala(
    nihai: &[u8],
    surum: &str,
    kanal: SurumKanali,
    changelog: &str,
    anahtar_tohum: [u8; 32],
) -> (GenisBildirim, Vec<u8>, Vec<u8>) {
    use ed25519_dalek::{Signer, SigningKey};
    let k = SigningKey::from_bytes(&anahtar_tohum);
    let bildirim = GenisBildirim {
        temel: GuncellemeBildirimi {
            surum: surum.to_string(),
            blake3_hex: blake3::hash(nihai).to_hex().to_string(),
            boyut: nihai.len() as u64,
        },
        kanal,
        changelog: changelog.to_string(),
        asgari_surum: None,
    };
    let imza = k.sign(&bildirim.kanonik()).to_bytes().to_vec();
    (bildirim, imza, k.verifying_key().to_bytes().to_vec())
}

fn imza_hatasi(detay: &str) -> ErrorReport {
    ErrorReport::new(
        "Güncelleme reddedildi (imza)",
        "Güncelleme bildiriminin imzası BioCraft'ın resmi anahtarıyla doğrulanamadı; paket sahte \
         veya indirilirken değiştirilmiş olabilir.",
        "Güncellemeyi yalnızca resmi kaynaktan indirin; sorun sürerse bu güncellemeyi atlayın.",
    )
    .with_teknik_detay(detay.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn anahtar() -> SigningKey {
        SigningKey::from_bytes(&[5u8; 32])
    }

    fn imzala(
        nihai: &[u8],
        surum: &str,
        kanal: SurumKanali,
        changelog: &str,
        k: &SigningKey,
    ) -> (GenisBildirim, Vec<u8>, Vec<u8>) {
        let bildirim = GenisBildirim {
            temel: GuncellemeBildirimi {
                surum: surum.to_string(),
                blake3_hex: blake3::hash(nihai).to_hex().to_string(),
                boyut: nihai.len() as u64,
            },
            kanal,
            changelog: changelog.to_string(),
            asgari_surum: None,
        };
        let sig: Signature = k.sign(&bildirim.kanonik());
        (
            bildirim,
            sig.to_bytes().to_vec(),
            k.verifying_key().to_bytes().to_vec(),
        )
    }

    fn kurulum() -> (AtomikKurulum, std::path::PathBuf) {
        let tmp = std::env::temp_dir().join(format!(
            "bc-upd-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        (AtomikKurulum::yeni(&tmp), tmp)
    }

    #[test]
    fn semver_karsilastirma() {
        assert!(daha_yeni_mi("1.0.0", "1.0.1").unwrap());
        assert!(daha_yeni_mi("1.9.0", "1.10.0").unwrap());
        assert!(!daha_yeni_mi("2.0.0", "1.9.9").unwrap());
        assert_eq!(surum_yonu("1.0.0", "1.0.0").unwrap(), SurumYon::AyniSurum);
        assert_eq!(surum_yonu("2.0.0", "1.0.0").unwrap(), SurumYon::Indirme);
    }

    #[test]
    fn kanal_gorunurlugu() {
        assert!(SurumKanali::Kararli.gorur_mu(SurumKanali::Kararli));
        assert!(!SurumKanali::Kararli.gorur_mu(SurumKanali::Beta));
        assert!(SurumKanali::Beta.gorur_mu(SurumKanali::Kararli));
        assert!(SurumKanali::Nightly.gorur_mu(SurumKanali::Nightly));
    }

    #[test]
    fn tam_paket_gecerli_uygulanir() {
        let (kur, tmp) = kurulum();
        kur.uygula("1.0.0", &[("paket.bin", b"surum-1".to_vec())])
            .unwrap();
        let yeni = b"BioCraft 1.1.0 paket icerigi".to_vec();
        let k = anahtar();
        let (bildirim, imza, vk) =
            imzala(&yeni, "1.1.0", SurumKanali::Kararli, "- Yeni özellik", &k);
        let rapor = guncellemeyi_uygula(
            &kur,
            GuncellemePaketi::Tam(yeni.clone()),
            &bildirim,
            &imza,
            &vk,
            SurumKanali::Kararli,
        )
        .unwrap();
        assert_eq!(rapor.yeni_surum, "1.1.0");
        assert_eq!(rapor.yon, SurumYon::Yukseltme);
        assert!(!rapor.delta_mi);
        assert_eq!(kur.gecerli_surum().as_deref(), Some("1.1.0"));
        assert_eq!(kur.gecerli_dosya("paket.bin").as_deref(), Some(&yeni[..]));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn delta_paket_birlestirilir() {
        let (kur, tmp) = kurulum();
        let eski = b"BioCraft tabani ".repeat(200); // ~3 KB ortak gövde
        kur.uygula("1.0.0", &[("paket.bin", eski.clone())]).unwrap();
        let mut yeni = eski.clone();
        yeni.extend_from_slice(b"<< 1.1.0 yeni kuyruk >>");
        let yama = delta::uret(&eski, &yeni);
        // Delta kazancı: taşınan bayt yeni paketten çok küçük olmalı.
        assert!(yama.tasinan_bayt() < yeni.len());
        let k = anahtar();
        let (bildirim, imza, vk) = imzala(&yeni, "1.1.0", SurumKanali::Kararli, "delta", &k);
        let rapor = guncellemeyi_uygula(
            &kur,
            GuncellemePaketi::Delta(yama),
            &bildirim,
            &imza,
            &vk,
            SurumKanali::Kararli,
        )
        .unwrap();
        assert!(rapor.delta_mi);
        assert_eq!(kur.gecerli_dosya("paket.bin").as_deref(), Some(&yeni[..]));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sahte_imza_reddedilir_current_korunur() {
        let (kur, tmp) = kurulum();
        kur.uygula("1.0.0", &[("paket.bin", b"v1".to_vec())])
            .unwrap();
        let yeni = b"kotu paket".to_vec();
        let saldirgan = SigningKey::from_bytes(&[9u8; 32]);
        let (bildirim, imza, _) = imzala(&yeni, "1.1.0", SurumKanali::Kararli, "x", &saldirgan);
        let resmi = anahtar().verifying_key().to_bytes().to_vec();
        let hata = guncellemeyi_uygula(
            &kur,
            GuncellemePaketi::Tam(yeni),
            &bildirim,
            &imza,
            &resmi,
            SurumKanali::Kararli,
        )
        .unwrap_err();
        assert!(hata.ne_oldu.contains("imza"));
        // Reddedildi → aktif kurulum dokunulmadı.
        assert_eq!(kur.gecerli_surum().as_deref(), Some("1.0.0"));
        assert_eq!(kur.gecerli_dosya("paket.bin").as_deref(), Some(&b"v1"[..]));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn kurcalanmis_changelog_reddedilir() {
        let (kur, tmp) = kurulum();
        kur.uygula("1.0.0", &[("paket.bin", b"v1".to_vec())])
            .unwrap();
        let yeni = b"paket".to_vec();
        let k = anahtar();
        let (mut bildirim, imza, vk) =
            imzala(&yeni, "1.1.0", SurumKanali::Kararli, "gerçek changelog", &k);
        // Saldırgan imzadan sonra changelog'u değiştirdi → kanonik değişir → imza tutmaz.
        bildirim.changelog = "sahte: tıkla http://kotu".to_string();
        assert!(guncellemeyi_uygula(
            &kur,
            GuncellemePaketi::Tam(yeni),
            &bildirim,
            &imza,
            &vk,
            SurumKanali::Kararli
        )
        .is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn yanlis_kanal_reddedilir() {
        let (kur, tmp) = kurulum();
        kur.uygula("1.0.0", &[("paket.bin", b"v1".to_vec())])
            .unwrap();
        let yeni = b"beta paket".to_vec();
        let k = anahtar();
        let (bildirim, imza, vk) = imzala(&yeni, "1.1.0", SurumKanali::Beta, "beta", &k);
        // Kararlı kullanıcı beta yayınını görmez.
        let hata = guncellemeyi_uygula(
            &kur,
            GuncellemePaketi::Tam(yeni),
            &bildirim,
            &imza,
            &vk,
            SurumKanali::Kararli,
        )
        .unwrap_err();
        assert!(hata.ne_oldu.contains("kanal"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn downgrade_isaretlenir_ama_uygulanir() {
        let (kur, tmp) = kurulum();
        kur.uygula("2.0.0", &[("paket.bin", b"v2".to_vec())])
            .unwrap();
        let yeni = b"v1 paket".to_vec();
        let k = anahtar();
        let (bildirim, imza, vk) = imzala(&yeni, "1.0.0", SurumKanali::Kararli, "downgrade", &k);
        let rapor = guncellemeyi_uygula(
            &kur,
            GuncellemePaketi::Tam(yeni),
            &bildirim,
            &imza,
            &vk,
            SurumKanali::Kararli,
        )
        .unwrap();
        assert_eq!(rapor.yon, SurumYon::Indirme);
        assert_eq!(kur.gecerli_surum().as_deref(), Some("1.0.0"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
