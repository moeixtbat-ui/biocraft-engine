//! **Dış iletişim onayı** — her dış gönderim açık onay ister; ne gönderildiği şeffaftır (İP-10).
//!
//! Bir dış gönderim **üç kapıdan** geçer (sıra önemlidir — en kısıtlayıcı önce):
//! 1. **Sınıf kapısı** ([`super::classify::cikis_denetle`]): PHI/hassas **mutlak** engellenir.
//! 2. **Profil kapısı** ([`super::profile::GizlilikProfili::dis_kanal_etkin_mi`]): kanal açık mı?
//! 3. **Onay kapısı** (bu modül): kullanıcı, **ne/nereye/ne kadar** gönderileceğini görüp onaylar mı?
//!
//! [`gonderim_degerlendir`] ilk iki kapıyı uygular ve sonucu döndürür; onay kararı kullanıcıdadır
//! ([`OnayKarari`]).  Onaylanan her gönderim, şeffaflık/denetim için [`OnayKaydi`] olarak deftere
//! işlenir (bkz. [`super::provenance::onay_ekle`]).

use serde::{Deserialize, Serialize};

use biocraft_types::{DataClassification, ErrorReport, Timestamp};

use super::classify::{self, DisKanal};
use super::profile::GizlilikProfili;

/// **Şeffaflık özeti** — bir dış gönderimde tam olarak *ne*nin gideceği.
///
/// Kullanıcı onaylamadan önce bunu görür: kaç öğe, hangi sınıflar, ne kadar veri, hangi alanlar.
/// "Kara kutu" gönderim yoktur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GonderimOzeti {
    /// Gönderilecek öğe (kayıt/dosya) sayısı.
    pub oge_sayisi: usize,
    /// Pakette bulunan sınıflandırmalar (en kısıtlayıcısı denetlenir).
    pub siniflar: Vec<DataClassification>,
    /// Toplam veri boyutu (bayt).
    pub boyut_bayt: u64,
    /// Gönderilecek alan/sütun adları (şeffaflık — hangi bilgiler gidiyor).
    #[serde(default)]
    pub alanlar: Vec<String>,
}

impl GonderimOzeti {
    /// Bu paketin **en kısıtlayıcı** sınıfı (boşsa fail-closed → PHI).
    pub fn en_kisitlayici(&self) -> DataClassification {
        classify::en_kisitlayici(self.siniflar.iter().copied())
    }

    /// İnsan-okunur tek satır özet (loga/onay defterine yazmak için).
    pub fn ozet_satiri(&self) -> String {
        format!(
            "{} öğe, {} bayt, sınıf={}",
            self.oge_sayisi,
            self.boyut_bayt,
            classify::sinif_ad(self.en_kisitlayici())
        )
    }
}

/// Bir dış gönderim **onay talebi** — kullanıcıya sunulan eksiksiz bağlam.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnayTalebi {
    /// Hedef kanal.
    pub kanal: DisKanal,
    /// Hedefin insan-okunur tanımı (örn. "NCBI BLAST", "AI havuzu", "iş arkadaşı@kurum").
    pub hedef: String,
    /// Ne için / neden gönderiliyor (sade dil).
    pub aciklama: String,
    /// Şeffaflık özeti (ne gidiyor).
    pub ozet: GonderimOzeti,
}

impl OnayTalebi {
    /// Yeni bir onay talebi kurar.
    pub fn yeni(
        kanal: DisKanal,
        hedef: impl Into<String>,
        aciklama: impl Into<String>,
        ozet: GonderimOzeti,
    ) -> Self {
        Self {
            kanal,
            hedef: hedef.into(),
            aciklama: aciklama.into(),
            ozet,
        }
    }
}

/// Bir gönderim talebinin (sınıf + profil kapıları sonrası) durumu.
#[derive(Debug, Clone)]
pub enum GonderimDurumu {
    /// Çekirdek tarafından **engellendi** (PHI/hassas) — atlanamaz; gönderim olmaz.
    Engellendi(Box<ErrorReport>),
    /// Profil bu kanalı **kapatmış** — kullanıcı önce ayarlardan açmalı.
    KanalKapali(Box<ErrorReport>),
    /// Sınıf + profil izin verdi; **kullanıcı onayı bekleniyor** (talep döndürülür).
    OnayBekliyor(Box<OnayTalebi>),
}

impl GonderimDurumu {
    /// Gönderim (onaydan önce) prensipte mümkün mü?
    pub fn onay_bekliyor_mu(&self) -> bool {
        matches!(self, GonderimDurumu::OnayBekliyor(_))
    }
}

/// **Sınıf** ve **profil** kapılarını uygular; sonucu döndürür.  Onay kararı çağırana (kullanıcıya) aittir.
///
/// Sıra: PHI engeli (mutlak) → profil kanal denetimi → onay talebi.  Profil ayarı **asla** PHI engelini
/// geçersiz kılamaz (engel önce denetlenir).
pub fn gonderim_degerlendir(profil: &GizlilikProfili, talep: OnayTalebi) -> GonderimDurumu {
    let sinif = talep.ozet.en_kisitlayici();

    // 1) Sınıf kapısı — PHI/hassas mutlak engel (profil bunu açamaz).
    if let classify::CikisKarari::Engellendi(rapor) = classify::cikis_denetle(sinif, talep.kanal) {
        return GonderimDurumu::Engellendi(rapor);
    }

    // 2) Profil kapısı — kanal açık mı?
    if !profil.dis_kanal_etkin_mi(talep.kanal) {
        return GonderimDurumu::KanalKapali(Box::new(kanal_kapali_hatasi(talep.kanal)));
    }

    // 3) Onay kapısı — kullanıcıya talep sun.
    GonderimDurumu::OnayBekliyor(Box::new(talep))
}

/// Kullanıcının onay talebine verdiği karar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnayKarari {
    /// Kullanıcı gönderimi onayladı.
    Onaylandi,
    /// Kullanıcı reddetti.
    Reddedildi,
}

/// Onay defterine yazılan **denetim kaydı** (her dış gönderim girişimi şeffaflık için saklanır).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnayKaydi {
    /// Karar zamanı (UTC).
    pub zaman: Timestamp,
    /// Kanal.
    pub kanal_ad: String,
    /// Hedef.
    pub hedef: String,
    /// Ne gönderildiğinin özeti.
    pub ozet: String,
    /// Kullanıcının kararı.
    pub karar: OnayKarari,
}

impl OnayKaydi {
    /// Bir talep + karardan denetim kaydı üretir (zaman = şimdi).
    pub fn yeni(talep: &OnayTalebi, karar: OnayKarari) -> Self {
        Self {
            zaman: chrono::Utc::now(),
            kanal_ad: talep.kanal.ad().to_string(),
            hedef: talep.hedef.clone(),
            ozet: talep.ozet.ozet_satiri(),
            karar,
        }
    }
}

/// Profil bir kanalı kapatmışken oluşan açıklayıcı hata.
pub fn kanal_kapali_hatasi(kanal: DisKanal) -> ErrorReport {
    ErrorReport::new(
        "Bu kanal kapalı",
        format!(
            "Gizlilik ayarlarınız '{}' kanalını kapalı tutuyor (varsayılan: tamamen yerel).",
            kanal.ad()
        ),
        "Bu kanalı kullanmak istiyorsanız Ayarlar → Gizlilik'ten açıkça etkinleştirin.",
    )
    .with_eylem("Gizlilik ayarları")
}

#[cfg(test)]
mod tests {
    use super::super::profile::TelemetriDuzeyi;
    use super::*;

    fn ozet(sinif: DataClassification) -> GonderimOzeti {
        GonderimOzeti {
            oge_sayisi: 3,
            siniflar: vec![sinif],
            boyut_bayt: 1024,
            alanlar: vec!["gen".into(), "varyant".into()],
        }
    }

    fn acik_profil() -> GizlilikProfili {
        GizlilikProfili {
            tamamen_yerel: false,
            ai_havuzu_katki: true,
            telemetri: TelemetriDuzeyi::MinimalAnonim,
            dagitik_ag_etkin: true,
            her_dis_gonderim_onay: true,
        }
    }

    #[test]
    fn phi_kanal_acik_olsa_bile_engellenir() {
        // Profil tüm kanalları açmış olsa bile PHI engeli mutlaktır.
        let talep = OnayTalebi::yeni(
            DisKanal::DisAi,
            "AI havuzu",
            "Model eğitimine katkı",
            ozet(DataClassification::HasasPhi),
        );
        let durum = gonderim_degerlendir(&acik_profil(), talep);
        assert!(matches!(durum, GonderimDurumu::Engellendi(_)));
    }

    #[test]
    fn varsayilan_yerel_profil_dis_gonderimi_durdurur() {
        // Varsayılan (tamamen yerel) profil: Normal veri bile kanal kapalı olduğundan gönderilemez.
        let global = GizlilikProfili::varsayilan_global();
        let talep = OnayTalebi::yeni(
            DisKanal::DisApi,
            "NCBI",
            "Dizi araması",
            ozet(DataClassification::Normal),
        );
        let durum = gonderim_degerlendir(&global, talep);
        assert!(matches!(durum, GonderimDurumu::KanalKapali(_)));
    }

    #[test]
    fn normal_veri_acik_kanalda_onay_bekler() {
        let talep = OnayTalebi::yeni(
            DisKanal::DisApi,
            "NCBI",
            "Dizi araması",
            ozet(DataClassification::Normal),
        );
        let durum = gonderim_degerlendir(&acik_profil(), talep);
        assert!(durum.onay_bekliyor_mu());
    }

    #[test]
    fn onay_kaydi_kararı_tasir() {
        let talep = OnayTalebi::yeni(
            DisKanal::Paylasim,
            "ekip",
            "sonuç paylaşımı",
            ozet(DataClassification::Sentetik),
        );
        let kayit = OnayKaydi::yeni(&talep, OnayKarari::Onaylandi);
        assert_eq!(kayit.karar, OnayKarari::Onaylandi);
        assert!(kayit.ozet.contains("Sentetik"));
        assert_eq!(kayit.kanal_ad, DisKanal::Paylasim.ad());
    }

    #[test]
    fn ozet_en_kisitlayici_phi_secer() {
        let o = GonderimOzeti {
            oge_sayisi: 2,
            siniflar: vec![DataClassification::Normal, DataClassification::HasasPhi],
            boyut_bayt: 10,
            alanlar: vec![],
        };
        assert_eq!(o.en_kisitlayici(), DataClassification::HasasPhi);
    }
}
