//! Eklenti **imza doğrulama + bütünlük** (MK-16, İP-07).
//!
//! İki bağımsız güvence:
//! * **Bütünlük (BLAKE3):** paketin baytları beklenen özetle eşleşiyor mu? (bozulma,
//!   eksik/değişmiş bayt yakalanır — MK-33).
//! * **Köken (Ed25519 imza):** paketi gerçekten iddia edilen yayıncı mı imzaladı?
//!
//! İmza, paket baytlarının **BLAKE3 özeti** üzerine atılır (hash-then-sign).  Çekirdek
//! bir **güven deposu** ([`GuvenDeposu`]) tutar; imza bu depodaki bir anahtarla
//! doğrulanırsa eklenti **"doğrulanmış"**, BioCraft'ın **resmi** anahtarıysa **"resmi"**
//! rozeti alır.  İmzasız eklenti net uyarı verir; imzası **geçersiz** olan eklenti
//! (bozulma/sahtecilik) politikadan bağımsız **her zaman reddedilir.**
//!
//! Tasarım notu: imzalama/doğrulama RFC 8032 gereği **deterministiktir** ve anahtarlar
//! `from_bytes` ile kurulur → harici entropi / C derleyici gerekmez (CI'da sorunsuz).

use biocraft_types::ErrorReport;
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

// Anahtar/anahtar üretimi (test + demo + imzalama araçları) için yeniden dışa aktarım.
pub use ed25519_dalek::SigningKey;

/// Bir Ed25519 açık anahtarının bayt uzunluğu.
pub const ACIK_ANAHTAR_BAYT: usize = 32;
/// Bir Ed25519 imzasının bayt uzunluğu.
pub const IMZA_BAYT: usize = 64;

// ─── Ayrık (detached) imza ────────────────────────────────────────────────────

/// Bir pakete iliştirilen ayrık imza (yanında `imza.json` olarak veya `.bcext`
/// fragmanında taşınır).
///
/// Baytlar `Vec<u8>` olarak tutulur (serde, sabit-boy diziyi doğrudan desteklemez);
/// uzunluklar [`Imza::dogrula`] içinde denetlenir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Imza {
    /// İmzalayan yayıncının açık anahtarı (Ed25519, 32 bayt).
    pub acik_anahtar: Vec<u8>,
    /// Paket özetinin imzası (Ed25519, 64 bayt).
    pub imza: Vec<u8>,
}

impl Imza {
    /// Bir [`SigningKey`] ile `payload`'ı imzalayıp ayrık imza üretir.
    ///
    /// İmza, `payload`'ın **BLAKE3 özeti** üzerine atılır (hash-then-sign).
    pub fn olustur(payload: &[u8], imzalayici: &SigningKey) -> Self {
        let ozet = blake3::hash(payload);
        let sig: Signature = imzalayici.sign(ozet.as_bytes());
        Self {
            acik_anahtar: imzalayici.verifying_key().to_bytes().to_vec(),
            imza: sig.to_bytes().to_vec(),
        }
    }

    /// Açık anahtarı tipli forma çevirir (uzunluk/eğri denetimiyle).
    fn acik_anahtar_coz(&self) -> Result<VerifyingKey, ()> {
        let bayt: [u8; ACIK_ANAHTAR_BAYT] =
            self.acik_anahtar.as_slice().try_into().map_err(|_| ())?;
        VerifyingKey::from_bytes(&bayt).map_err(|_| ())
    }

    /// İmzayı tipli forma çevirir (uzunluk denetimiyle).
    fn imza_coz(&self) -> Result<Signature, ()> {
        let bayt: [u8; IMZA_BAYT] = self.imza.as_slice().try_into().map_err(|_| ())?;
        Ok(Signature::from_bytes(&bayt))
    }

    /// İmzanın `payload` üzerinde geçerli olup olmadığını döndürür (köken denetimi yok,
    /// yalnızca "bu anahtar bu içeriği imzaladı mı").
    pub fn dogrula(&self, payload: &[u8]) -> bool {
        let (Ok(vk), Ok(sig)) = (self.acik_anahtar_coz(), self.imza_coz()) else {
            return false;
        };
        let ozet = blake3::hash(payload);
        vk.verify(ozet.as_bytes(), &sig).is_ok()
    }
}

// ─── Güven deposu (trusted publishers) ────────────────────────────────────────

/// Bir imzanın çekirdekçe tanınma seviyesi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuvenSeviyesi {
    /// BioCraft'ın **resmi** yayıncı anahtarı.
    Resmi,
    /// Çekirdeğe eklenmiş, **güvenilen** üçüncü-parti yayıncı.
    Dogrulanmis,
}

/// Çekirdeğin tanıdığı yayıncı açık anahtarları.
///
/// `resmi` listesindeki anahtarlar **"resmi"** rozeti verir (BioCraft'ın kendi
/// eklentileri); `dogrulanmis` listesindekiler **"doğrulanmış"** (incelenmiş 3. parti).
#[derive(Debug, Clone, Default)]
pub struct GuvenDeposu {
    resmi: Vec<(String, VerifyingKey)>,
    dogrulanmis: Vec<(String, VerifyingKey)>,
}

impl GuvenDeposu {
    /// Boş bir güven deposu (hiçbir yayıncı tanınmaz → her imza "bilinmeyen").
    pub fn bos() -> Self {
        Self::default()
    }

    /// Resmi (BioCraft) bir yayıncı anahtarı ekler.
    pub fn resmi_ekle(&mut self, ad: impl Into<String>, vk: VerifyingKey) {
        self.resmi.push((ad.into(), vk));
    }

    /// Güvenilen üçüncü-parti bir yayıncı anahtarı ekler.
    pub fn yayinci_ekle(&mut self, ad: impl Into<String>, vk: VerifyingKey) {
        self.dogrulanmis.push((ad.into(), vk));
    }

    /// Verilen anahtarın depoda tanınıp tanınmadığını döndürür `(seviye, yayıncı adı)`.
    pub fn bul(&self, vk: &VerifyingKey) -> Option<(GuvenSeviyesi, String)> {
        if let Some((ad, _)) = self.resmi.iter().find(|(_, k)| k == vk) {
            return Some((GuvenSeviyesi::Resmi, ad.clone()));
        }
        if let Some((ad, _)) = self.dogrulanmis.iter().find(|(_, k)| k == vk) {
            return Some((GuvenSeviyesi::Dogrulanmis, ad.clone()));
        }
        None
    }
}

// ─── İmza durumu + rozet ──────────────────────────────────────────────────────

/// Bir eklentinin imza/bütünlük denetiminden sonra aldığı durum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImzaDurumu {
    /// BioCraft'ın resmi anahtarıyla doğrulandı.
    Resmi { yayinci: String },
    /// Güvenilen bir 3. parti yayıncıyla doğrulandı.
    Dogrulanmis { yayinci: String },
    /// İmza geçerli ama anahtar güven deposunda **yok** (köken bilinmiyor).
    ImzaliBilinmeyen,
    /// İmza **yok** (kullanıcıya net uyarı gösterilmeli).
    Imzasiz,
    /// İmza var ama **doğrulanamadı** (bozulma/sahtecilik) — daima reddedilir.
    Gecersiz { neden: String },
}

impl ImzaDurumu {
    /// Bir `payload` + opsiyonel imzayı güven deposuna karşı değerlendirir.
    pub fn degerlendir(payload: &[u8], imza: Option<&Imza>, depo: &GuvenDeposu) -> Self {
        let Some(imza) = imza else {
            return ImzaDurumu::Imzasiz;
        };
        // Önce anahtar/imza tipsel olarak çözülebiliyor mu?
        let (Ok(vk), Ok(sig)) = (imza.acik_anahtar_coz(), imza.imza_coz()) else {
            return ImzaDurumu::Gecersiz {
                neden: "imza veya açık anahtar biçimi bozuk".into(),
            };
        };
        // İmza içeriğe uyuyor mu? (bütünlük + köken birlikte)
        let ozet = blake3::hash(payload);
        if vk.verify(ozet.as_bytes(), &sig).is_err() {
            return ImzaDurumu::Gecersiz {
                neden: "imza paket içeriğiyle eşleşmiyor (paket değişmiş veya imza sahte olabilir)"
                    .into(),
            };
        }
        // Geçerli imza; yayıncı tanınıyor mu?
        match depo.bul(&vk) {
            Some((GuvenSeviyesi::Resmi, ad)) => ImzaDurumu::Resmi { yayinci: ad },
            Some((GuvenSeviyesi::Dogrulanmis, ad)) => ImzaDurumu::Dogrulanmis { yayinci: ad },
            None => ImzaDurumu::ImzaliBilinmeyen,
        }
    }

    /// Resmi (BioCraft) bir eklenti mi?
    pub fn resmi_mi(&self) -> bool {
        matches!(self, ImzaDurumu::Resmi { .. })
    }

    /// Güven deposunca tanınan (resmi veya doğrulanmış) bir imza mı?
    pub fn guvenilir_mi(&self) -> bool {
        matches!(
            self,
            ImzaDurumu::Resmi { .. } | ImzaDurumu::Dogrulanmis { .. }
        )
    }

    /// Geçersiz (bozuk/sahte) imza mı? (Politikadan bağımsız reddedilir.)
    pub fn gecersiz_mi(&self) -> bool {
        matches!(self, ImzaDurumu::Gecersiz { .. })
    }

    /// UI'da gösterilecek rozet.
    pub fn rozet(&self) -> Rozet {
        match self {
            ImzaDurumu::Resmi { .. } => Rozet::Resmi,
            ImzaDurumu::Dogrulanmis { .. } => Rozet::Dogrulanmis,
            ImzaDurumu::ImzaliBilinmeyen => Rozet::Bilinmeyen,
            ImzaDurumu::Imzasiz => Rozet::Imzasiz,
            ImzaDurumu::Gecersiz { .. } => Rozet::Tehlike,
        }
    }
}

/// Kullanıcıya gösterilecek imza rozeti (renk/önem seviyesi UI'da eşlenir).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rozet {
    /// ✓ Resmi (BioCraft).
    Resmi,
    /// ✓ Doğrulanmış yayıncı.
    Dogrulanmis,
    /// ⚠ İmzalı ama yayıncı bilinmiyor.
    Bilinmeyen,
    /// ⚠ İmzasız.
    Imzasiz,
    /// ⛔ Geçersiz imza.
    Tehlike,
}

impl Rozet {
    /// Kısa Türkçe etiket (UI i18n L4'te yapılır; bu L3 için varsayılan metin).
    pub fn etiket(&self) -> &'static str {
        match self {
            Rozet::Resmi => "Resmi",
            Rozet::Dogrulanmis => "Doğrulanmış",
            Rozet::Bilinmeyen => "İmzalı (bilinmeyen yayıncı)",
            Rozet::Imzasiz => "İmzasız",
            Rozet::Tehlike => "Geçersiz imza",
        }
    }

    /// Kullanıcı uyarısı gerektirir mi? (imzasız/bilinmeyen/geçersiz)
    pub fn uyari_mi(&self) -> bool {
        !matches!(self, Rozet::Resmi | Rozet::Dogrulanmis)
    }
}

// ─── İmza politikası (kurulum/yükleme kapısı) ─────────────────────────────────

/// Kurulum/yükleme sırasında uygulanacak imza politikası.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImzaPolitikasi {
    /// Yalnızca **resmi** (BioCraft) eklentilere izin ver (en katı; kurumsal/kiosk).
    SadeceResmi,
    /// Güven deposunca tanınan (resmi veya doğrulanmış) imza **zorunlu.**
    GuvenilirGerekli,
    /// İmzasız/bilinmeyene de izin ver (uyarıyla) — **varsayılan** (açık ekosistem).
    /// Geçersiz imza yine de reddedilir.
    #[default]
    ImzasizaIzinVer,
}

impl ImzaPolitikasi {
    /// Verilen imza durumu bu politikayla yüklenebilir mi? Değilse açıklayıcı hata.
    pub fn denetle(&self, durum: &ImzaDurumu) -> Result<(), ErrorReport> {
        // Geçersiz imza her politikada reddedilir (savunma önce).
        if let ImzaDurumu::Gecersiz { neden } = durum {
            return Err(ErrorReport::new(
                "Eklenti imzası geçersiz",
                format!("eklentinin imzası doğrulanamadı: {neden}"),
                "Eklentiyi resmi mağazadan yeniden indirin; sorun sürerse yayıncıya bildirin",
            )
            .with_eylem("Yeniden indir"));
        }
        let izin = match self {
            ImzaPolitikasi::SadeceResmi => durum.resmi_mi(),
            ImzaPolitikasi::GuvenilirGerekli => durum.guvenilir_mi(),
            ImzaPolitikasi::ImzasizaIzinVer => true,
        };
        if izin {
            Ok(())
        } else {
            Err(ErrorReport::new(
                "Eklenti güvenlik politikasını karşılamıyor",
                format!(
                    "geçerli politika ({}) bu eklentinin imza durumuna ({}) izin vermiyor",
                    self.aciklama(),
                    durum.rozet().etiket()
                ),
                "Eklentiyi yalnızca güveniyorsanız kurun veya güvenlik politikasını ayarlardan gevşetin",
            )
            .with_eylem("Güvenlik ayarları"))
        }
    }

    /// Politikanın kısa açıklaması.
    pub fn aciklama(&self) -> &'static str {
        match self {
            ImzaPolitikasi::SadeceResmi => "yalnızca resmi",
            ImzaPolitikasi::GuvenilirGerekli => "güvenilir imza zorunlu",
            ImzaPolitikasi::ImzasizaIzinVer => "imzasıza izin ver",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministik test anahtarı (sabit tohum → CI'da entropi gerekmez).
    fn anahtar(tohum: u8) -> SigningKey {
        SigningKey::from_bytes(&[tohum; 32])
    }

    #[test]
    fn dogru_imza_dogrulanir() {
        let sk = anahtar(1);
        let payload = b"merhaba eklenti paketi";
        let imza = Imza::olustur(payload, &sk);
        assert!(imza.dogrula(payload));
    }

    #[test]
    fn degisen_icerik_imzayi_bozar() {
        let sk = anahtar(2);
        let imza = Imza::olustur(b"orijinal", &sk);
        assert!(
            !imza.dogrula(b"degistirilmis"),
            "değişen içerik doğrulanmamalı"
        );
    }

    #[test]
    fn imzasiz_durum() {
        let durum = ImzaDurumu::degerlendir(b"x", None, &GuvenDeposu::bos());
        assert_eq!(durum, ImzaDurumu::Imzasiz);
        assert!(durum.rozet().uyari_mi());
    }

    #[test]
    fn resmi_anahtar_resmi_rozeti() {
        let sk = anahtar(3);
        let mut depo = GuvenDeposu::bos();
        depo.resmi_ekle("BioCraft", sk.verifying_key());
        let payload = b"resmi eklenti";
        let imza = Imza::olustur(payload, &sk);
        let durum = ImzaDurumu::degerlendir(payload, Some(&imza), &depo);
        assert!(matches!(durum, ImzaDurumu::Resmi { .. }));
        assert!(durum.resmi_mi());
        assert_eq!(durum.rozet(), Rozet::Resmi);
        assert!(!durum.rozet().uyari_mi());
    }

    #[test]
    fn guvenilir_yayinci_dogrulanmis_rozeti() {
        let sk = anahtar(4);
        let mut depo = GuvenDeposu::bos();
        depo.yayinci_ekle("Acme Bio", sk.verifying_key());
        let payload = b"3. parti eklenti";
        let imza = Imza::olustur(payload, &sk);
        let durum = ImzaDurumu::degerlendir(payload, Some(&imza), &depo);
        assert!(matches!(durum, ImzaDurumu::Dogrulanmis { .. }));
        assert!(durum.guvenilir_mi());
    }

    #[test]
    fn bilinmeyen_anahtar_imzali_bilinmeyen() {
        let sk = anahtar(5);
        let payload = b"taninmayan yayinci";
        let imza = Imza::olustur(payload, &sk);
        // Depo boş → anahtar tanınmaz ama imza geçerli.
        let durum = ImzaDurumu::degerlendir(payload, Some(&imza), &GuvenDeposu::bos());
        assert_eq!(durum, ImzaDurumu::ImzaliBilinmeyen);
        assert!(!durum.guvenilir_mi());
    }

    #[test]
    fn sahte_imza_gecersiz() {
        let sk = anahtar(6);
        let mut imza = Imza::olustur(b"icerik", &sk);
        // İmza baytlarını boz.
        imza.imza[0] ^= 0xFF;
        let durum = ImzaDurumu::degerlendir(b"icerik", Some(&imza), &GuvenDeposu::bos());
        assert!(durum.gecersiz_mi());
        assert_eq!(durum.rozet(), Rozet::Tehlike);
    }

    #[test]
    fn bozuk_uzunluk_gecersiz() {
        let imza = Imza {
            acik_anahtar: vec![0; 10], // yanlış uzunluk
            imza: vec![0; 64],
        };
        let durum = ImzaDurumu::degerlendir(b"x", Some(&imza), &GuvenDeposu::bos());
        assert!(durum.gecersiz_mi());
    }

    #[test]
    fn politika_gecersizi_daima_reddeder() {
        let durum = ImzaDurumu::Gecersiz {
            neden: "test".into(),
        };
        for pol in [
            ImzaPolitikasi::SadeceResmi,
            ImzaPolitikasi::GuvenilirGerekli,
            ImzaPolitikasi::ImzasizaIzinVer,
        ] {
            assert!(pol.denetle(&durum).is_err(), "{pol:?} geçersizi reddetmeli");
        }
    }

    #[test]
    fn politika_imzasiza_izin_ver() {
        assert!(ImzaPolitikasi::ImzasizaIzinVer
            .denetle(&ImzaDurumu::Imzasiz)
            .is_ok());
        assert!(ImzaPolitikasi::GuvenilirGerekli
            .denetle(&ImzaDurumu::Imzasiz)
            .is_err());
        assert!(ImzaPolitikasi::SadeceResmi
            .denetle(&ImzaDurumu::Dogrulanmis {
                yayinci: "x".into()
            })
            .is_err());
    }

    #[test]
    fn varsayilan_politika_imzasiza_izin() {
        assert_eq!(ImzaPolitikasi::default(), ImzaPolitikasi::ImzasizaIzinVer);
    }
}
