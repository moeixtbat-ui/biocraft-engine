//! **Veri paylaşım sözleşmesi** — P2P'ye ne çıkabileceğinin tip-seviyesi sınırı (MK-42/43/50).
//!
//! Bu modül, dağıtık ağa (P2P) gönderilebilecek tek veri tipini ([`P2pYuku`]) tanımlar.  İki katmanlı
//! koruma sağlar:
//!
//! 1. **İçerik türü kısıtı (derleme-zamanı):** [`P2pIcerikTuru`]'nde yalnızca **metadata / sonuç /
//!    eklenti** vardır.  "Ham veri" / "hassas veri" için **varyant yoktur** → ham/hassas veri taşıyan
//!    bir yük *yazılamaz bile* (MK-50: "P2P sadece metadata/sonuç/eklenti").
//! 2. **Sınıflandırma kapısı (çalışma-zamanı):** [`P2pYuku`]'nun **tek** kurucusu
//!    [`P2pYuku::olustur`], her yükü çekirdek çıkış kapısından
//!    (`biocraft_data::privacy::classify::cikis_denetle(sinif, DisKanal::P2p)`) geçirir.  PHI/hassas
//!    sınıf → `Err` → yük **hiç oluşmaz**.  Böylece "PHI asla ağa çıkmaz" sınırı, kancanın *arayüzüne*
//!    gömülüdür: kapıyı atlayan bir yük inşa etmenin public bir yolu yoktur.
//!
//! > **Kural (kod yazan için):** P2P'ye giden her şey bir `P2pYuku` olmalıdır; bu tipi `olustur`
//! > dışında kuramazsınız ve `olustur` çekirdek kapısını çağırır.  Sınır eklentiye emanet edilmez.

use serde::{Deserialize, Serialize};

use biocraft_data::privacy::classify::{cikis_denetle, CikisKarari, DisKanal};
use biocraft_types::{DataClassification, ErrorReport};

/// P2P ağına çıkabilecek **içerik türleri** (MK-50 — yalnızca bunlar).
///
/// **Bilinçli olarak** ham/hassas veri için varyant içermez: dağıtık ağ ham veri taşıyamaz; yalnızca
/// üst-düzey meta veri, hesaplama sonuçları ve eklenti yükleri paylaşılabilir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum P2pIcerikTuru {
    /// Üst-düzey meta veri (iş tanımı, parametre özeti, yetenek ilanı).  Ham veri DEĞİL.
    Metadata,
    /// Hesaplama **sonucu** (özet/çıktı) — kaynak ham veriyi içermez.
    Sonuc,
    /// Eklenti/kod paketi (imzalı; İP-07 host doğrular).
    Eklenti,
}

impl P2pIcerikTuru {
    /// İnsan-okunur Türkçe ad (UI gösterimi üst katmanda i18n'lenebilir).
    pub fn ad(self) -> &'static str {
        match self {
            P2pIcerikTuru::Metadata => "Meta veri",
            P2pIcerikTuru::Sonuc => "Sonuç",
            P2pIcerikTuru::Eklenti => "Eklenti",
        }
    }
}

/// Dağıtık ağa gönderilebilen **tek** yük tipi — çekirdek çıkış kapısından geçmeden kurulamaz.
///
/// Alanlar `pub` değildir: bir `P2pYuku` ele geçiren her kod, onun sınıflandırma kapısından geçmiş
/// olduğuna güvenebilir (MK-43).  `sinif` daima dış-paylaşıma uygun (Normal/Sentetik); PHI buraya
/// hiç ulaşamaz çünkü [`P2pYuku::olustur`] PHI'de `Err` döner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2pYuku {
    icerik_turu: P2pIcerikTuru,
    /// Yükün veri sınıfı — kapıdan geçtiği için yalnızca Normal/Sentetik olabilir.
    sinif: DataClassification,
    /// İnsan-okunur kısa açıklama (denetim/şeffaflık için; ham içerik değil).
    aciklama: String,
    /// Taşınan baytlar (serileştirilmiş meta/sonuç/eklenti).  Ham hasta verisi BURAYA giremez —
    /// içerik türü + sınıflandırma kapısı bunu engeller.
    bayt: Vec<u8>,
}

impl P2pYuku {
    /// **Tek kurucu** — yükü çekirdek çıkış kapısından geçirir (MK-43).
    ///
    /// `cikis_denetle(sinif, DisKanal::P2p)` `Engellendi` dönerse (PHI/hassas) yük **oluşmaz**;
    /// standart İP-16 hatası döner.  Normal/Sentetik için yük kurulur.
    pub fn olustur(
        icerik_turu: P2pIcerikTuru,
        sinif: DataClassification,
        aciklama: impl Into<String>,
        bayt: Vec<u8>,
    ) -> Result<Self, Box<ErrorReport>> {
        match cikis_denetle(sinif, DisKanal::P2p) {
            CikisKarari::Izinli => Ok(Self {
                icerik_turu,
                sinif,
                aciklama: aciklama.into(),
                bayt,
            }),
            CikisKarari::Engellendi(hata) => Err(hata),
        }
    }

    /// Bir **metadata** yükü kurar (en sık kullanılan; ham veri değildir).
    pub fn metadata(
        sinif: DataClassification,
        aciklama: impl Into<String>,
        bayt: Vec<u8>,
    ) -> Result<Self, Box<ErrorReport>> {
        Self::olustur(P2pIcerikTuru::Metadata, sinif, aciklama, bayt)
    }

    /// Bir **sonuç** yükü kurar (hesaplama çıktısı; kaynak ham veriyi içermez).
    pub fn sonuc(
        sinif: DataClassification,
        aciklama: impl Into<String>,
        bayt: Vec<u8>,
    ) -> Result<Self, Box<ErrorReport>> {
        Self::olustur(P2pIcerikTuru::Sonuc, sinif, aciklama, bayt)
    }

    /// İçerik türü.
    pub fn icerik_turu(&self) -> P2pIcerikTuru {
        self.icerik_turu
    }

    /// Veri sınıfı (daima dış-paylaşıma uygun — kapıdan geçti).
    pub fn sinif(&self) -> DataClassification {
        self.sinif
    }

    /// Kısa açıklama.
    pub fn aciklama(&self) -> &str {
        &self.aciklama
    }

    /// Taşınan bayt sayısı (bütçe/şeffaflık için).
    pub fn bayt_sayisi(&self) -> usize {
        self.bayt.len()
    }

    /// Taşınan baytlara salt-okunur erişim (eklenti gönderim sırasında okur).
    pub fn baytlar(&self) -> &[u8] {
        &self.bayt
    }

    /// **Savunma katmanı (defense-in-depth):** yükün hâlâ çıkış kapısına uygun olduğunu yeniden
    /// doğrular.  Public kurucu zaten kapıdan geçirir; bu, gönderim anında "tek yol bile atlamamalı"
    /// (MK-43) ilkesini bir kez daha uygular (örn. serileştirme/deserialize sonrası emniyet).
    pub fn kapidan_gecer_mi(&self) -> bool {
        !cikis_denetle(self.sinif, DisKanal::P2p).engellendi_mi()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phi_yuku_olusturulamaz() {
        // MK-42/43: PHI'den P2P yükü inşa edilemez — kapı engeller, yük hiç oluşmaz.
        let sonuc = P2pYuku::olustur(
            P2pIcerikTuru::Sonuc,
            DataClassification::HasasPhi,
            "hasta sonucu",
            vec![1, 2, 3],
        );
        assert!(
            sonuc.is_err(),
            "PHI'den P2P yükü oluşturulabildi — sınır delindi!"
        );
    }

    #[test]
    fn normal_ve_sentetik_yuk_olusur() {
        for sinif in [DataClassification::Normal, DataClassification::Sentetik] {
            let y =
                P2pYuku::metadata(sinif, "meta", vec![0u8; 8]).expect("normal/sentetik geçmeli");
            assert_eq!(y.sinif(), sinif);
            assert_eq!(y.bayt_sayisi(), 8);
            assert!(y.kapidan_gecer_mi());
        }
    }

    #[test]
    fn sonuc_ve_metadata_kisayollari() {
        let m = P2pYuku::metadata(DataClassification::Normal, "m", vec![]).unwrap();
        assert_eq!(m.icerik_turu(), P2pIcerikTuru::Metadata);
        let s = P2pYuku::sonuc(DataClassification::Sentetik, "s", vec![]).unwrap();
        assert_eq!(s.icerik_turu(), P2pIcerikTuru::Sonuc);
    }

    #[test]
    fn yuk_serilesir_ve_geri_okunur() {
        // Eklenti köprüsü için serde gidiş-dönüş çalışmalı.
        let y = P2pYuku::sonuc(DataClassification::Normal, "ozet", vec![9, 8, 7]).unwrap();
        let json = serde_json::to_string(&y).unwrap();
        let geri: P2pYuku = serde_json::from_str(&json).unwrap();
        assert_eq!(geri.aciklama(), "ozet");
        assert_eq!(geri.baytlar(), &[9, 8, 7]);
        // Deserialize sonrası da kapı emniyeti tutar (Normal → geçer).
        assert!(geri.kapidan_gecer_mi());
    }
}
