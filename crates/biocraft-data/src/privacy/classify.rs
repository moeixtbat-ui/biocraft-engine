//! **Veri sınıflandırma çıkış kapısı** — gizliliğin çekirdek seviyesi koruması (MK-42/43).
//!
//! Bu modül, **tüm dış kanalların (P2P / dış-AI / dış-API / telemetri / paylaşım) önünde duran tek
//! kapıdır**.  Hasta/kişisel sağlık verisi (PHI) — ve genel olarak "dış paylaşıma yasak" sınıflar —
//! buradan **fiziksel olarak geçemez**.  Bu engel:
//!
//! 1. **Çekirdektedir (L2):** Dış kanal crate'leri (`biocraft-net` L3, `biocraft-ai-surface` L3) bu
//!    crate'e (L2) bağımlıdır; tersi MK-40 ile yasaktır.  Yani hiçbir dış kanal bu kapının
//!    *altına* inip onu atlayamaz — kapı, kanalların bağımlılık grafiğinde **altında** durur.
//! 2. **Eklentiye emanet edilmez:** Karar burada (çekirdek) verilir; eklenti yalnızca *istek* yapar,
//!    izni çekirdek verir/vermez.
//! 3. **Tek yol bile atlamamalı:** Her dış gönderim, gönderimden ÖNCE [`cikis_denetle`] çağırmalıdır.
//!
//! > **Kural (kod yazan için):** Yeni bir dış kanal eklerken (gerçek P2P, gerçek AI çağrısı, NCBI
//! > API'si, telemetri…), gönderim kodunun ilk satırı [`cikis_denetle`] olmalıdır.  Aksi hâlde
//! > sınıflandırma sınırı delinir (MK-43).

use serde::{Deserialize, Serialize};

use biocraft_types::{DataClassification, ErrorReport};

/// Verinin gönderilebileceği **dış** iletişim kanalları.
///
/// Yalnızca **dış** kanallar burada listelenir; tamamen yerel işlemler bu kapıdan geçmez (zaten
/// cihazdan çıkmaz).  Her yeni dış kanal bu enum'a eklenmeli ki sınıflandırma kapısı onu da kapsasın.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DisKanal {
    /// Dağıtık (P2P) ağ — yalnız metadata/sonuç; ham/PHI asla (MK-50).
    P2p,
    /// Dış yapay zekâ (bulut model / AI havuzuna katkı).
    DisAi,
    /// Dış API / uzak veritabanı sorgusu (örn. NCBI, Ensembl).
    DisApi,
    /// Telemetri / kullanım istatistiği (varsayılan kapalı, anonim).
    Telemetri,
    /// Kullanıcı paylaşımı (dosya/sonuç dışa gönderimi, e-posta/yükleme).
    Paylasim,
}

impl DisKanal {
    /// İnsan-okunur Türkçe ad (UI gösterimi üst katmanda i18n'lenebilir).
    pub fn ad(self) -> &'static str {
        match self {
            DisKanal::P2p => "Dağıtık ağ (P2P)",
            DisKanal::DisAi => "Dış yapay zekâ",
            DisKanal::DisApi => "Dış API / uzak veritabanı",
            DisKanal::Telemetri => "Telemetri",
            DisKanal::Paylasim => "Paylaşım",
        }
    }

    /// Tüm dış kanallar (denetim/test için sabit sıra).
    pub const TUMU: [DisKanal; 5] = [
        DisKanal::P2p,
        DisKanal::DisAi,
        DisKanal::DisApi,
        DisKanal::Telemetri,
        DisKanal::Paylasim,
    ];
}

/// Bir sınıfın **kısıtlama seviyesi** (büyük = daha hassas).  En kısıtlayıcıyı bulmak için kullanılır.
pub fn kisit_seviyesi(sinif: DataClassification) -> u8 {
    match sinif {
        DataClassification::Sentetik => 0, // yapay veri: en az kısıt
        DataClassification::Normal => 1,
        DataClassification::HasasPhi => 2, // PHI: en yüksek kısıt
    }
}

/// İnsan-okunur sınıf adı (UI'de "Sınıf görünür" gereği için — İP-10).
pub fn sinif_ad(sinif: DataClassification) -> &'static str {
    match sinif {
        DataClassification::Normal => "Normal",
        DataClassification::HasasPhi => "Hassas / PHI",
        DataClassification::Sentetik => "Sentetik",
    }
}

/// Bu sınıf **hiçbir koşulda** dış kanala gönderilebilir mi?  PHI için `false` (mutlak engel).
///
/// MK-42/43: PHI sınırı çekirdekte sabittir; ne profil ne onay ne eklenti bunu açabilir.  Anonimleştirme
/// (bkz. [`super::anonymize`]) ile sınıf düşürülürse ayrı bir (anonim) veri üretilir; PHI verinin
/// kendisi yine de çıkamaz.
pub fn dis_gonderime_uygun(sinif: DataClassification) -> bool {
    !matches!(sinif, DataClassification::HasasPhi)
}

/// Bir küme verinin **en kısıtlayıcı** sınıfı (paket içindeki en hassas öğeye göre denetlenir).
///
/// **Fail-closed:** küme **boşsa** en kısıtlayıcı (PHI) sayılır → boş/bilinmeyen paketler güvenli
/// tarafta bloklanır.  Böylece çağıran sınıf listesini doldurmayı *unutursa* veri sızmaz, gönderim durur.
pub fn en_kisitlayici<I>(siniflar: I) -> DataClassification
where
    I: IntoIterator<Item = DataClassification>,
{
    siniflar
        .into_iter()
        .max_by_key(|&s| kisit_seviyesi(s))
        .unwrap_or(DataClassification::HasasPhi)
}

/// Sınıflandırma kapısının kararı (yalnızca **sınıf** ekseni; profil/onay [`super::consent`]'te).
#[derive(Debug, Clone)]
pub enum CikisKarari {
    /// Sınıflandırma izin veriyor (gönderim için ayrıca **onay** gerekir — [`super::consent`]).
    Izinli,
    /// Çekirdek tarafından **engellendi** (PHI/hassas dış kanala çıkamaz).  Atlanamaz.
    Engellendi(Box<ErrorReport>),
}

impl CikisKarari {
    /// Engellendi mi?
    pub fn engellendi_mi(&self) -> bool {
        matches!(self, CikisKarari::Engellendi(_))
    }
}

/// **Çıkış kapısı:** verilen sınıfın verisi verilen dış kanala çıkabilir mi?
///
/// Bu, her dış gönderimin geçmesi gereken çekirdek denetimidir (MK-43).  PHI/hassas için **mutlak**
/// `Engellendi` döner — hiçbir üst katman ayarı bunu geçersiz kılamaz.
pub fn cikis_denetle(sinif: DataClassification, kanal: DisKanal) -> CikisKarari {
    if dis_gonderime_uygun(sinif) {
        CikisKarari::Izinli
    } else {
        CikisKarari::Engellendi(Box::new(phi_engel_hatasi(sinif, kanal)))
    }
}

/// Bir veri kümesini (en kısıtlayıcı sınıfına göre) tek seferde denetler.
pub fn kume_denetle<I>(siniflar: I, kanal: DisKanal) -> CikisKarari
where
    I: IntoIterator<Item = DataClassification>,
{
    cikis_denetle(en_kisitlayici(siniflar), kanal)
}

/// PHI/hassas verinin dış kanala çıkışını reddeden standart hata raporu (İP-16 şeması).
pub fn phi_engel_hatasi(sinif: DataClassification, kanal: DisKanal) -> ErrorReport {
    ErrorReport::new(
        "Hassas veri dışarı gönderilemez",
        format!(
            "'{}' sınıfı veri '{}' kanalına gönderilemez. Hasta/kişisel sağlık verisi (PHI) \
             çekirdek tarafından tüm dış kanallara karşı korunur; bu engel kapatılamaz (MK-42/43).",
            sinif_ad(sinif),
            kanal.ad()
        ),
        "Paylaşmak istiyorsanız önce veriyi anonimleştirin (geri-tanımlanamaz hâle getirin) ya da \
         yalnızca sentetik/özet sonuçları gönderin. Gerçek PHI için bu sınır kaldırılmaz.",
    )
    .with_eylem("Anonimleştir")
    .with_teknik_detay(format!(
        "cikis_denetle: sinif={:?}, kanal={:?} → Engellendi",
        sinif, kanal
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phi_her_dis_kanala_engellenir() {
        // MK-43: PHI hiçbir dış kanaldan çıkamaz — tek yol bile atlamamalı.
        for kanal in DisKanal::TUMU {
            let karar = cikis_denetle(DataClassification::HasasPhi, kanal);
            assert!(
                karar.engellendi_mi(),
                "PHI {kanal:?} kanalından çıkabildi — sınır delindi!"
            );
        }
    }

    #[test]
    fn normal_ve_sentetik_sinif_olarak_izinli() {
        // Sınıf izin verir (gönderim için ayrıca onay gerekir; o consent'te denetlenir).
        for kanal in DisKanal::TUMU {
            assert!(!cikis_denetle(DataClassification::Normal, kanal).engellendi_mi());
            assert!(!cikis_denetle(DataClassification::Sentetik, kanal).engellendi_mi());
        }
    }

    #[test]
    fn en_kisitlayici_phi_iceren_kumeyi_phi_sayar() {
        let kume = [
            DataClassification::Sentetik,
            DataClassification::Normal,
            DataClassification::HasasPhi, // bir tane PHI → tüm paket PHI gibi denetlenir
        ];
        assert_eq!(en_kisitlayici(kume), DataClassification::HasasPhi);
        assert!(kume_denetle(kume, DisKanal::DisAi).engellendi_mi());
    }

    #[test]
    fn bos_kume_fail_closed_phi_sayilir() {
        // Boş/bilinmeyen paket en kısıtlayıcı sayılır (güvenli taraf).
        let bos: Vec<DataClassification> = Vec::new();
        assert_eq!(en_kisitlayici(bos.clone()), DataClassification::HasasPhi);
        assert!(kume_denetle(bos, DisKanal::Paylasim).engellendi_mi());
    }

    #[test]
    fn normal_kume_izinli() {
        let kume = [DataClassification::Normal, DataClassification::Sentetik];
        assert_eq!(en_kisitlayici(kume), DataClassification::Normal);
        assert!(!kume_denetle(kume, DisKanal::DisApi).engellendi_mi());
    }

    #[test]
    fn engel_hatasi_standart_sema() {
        let h = phi_engel_hatasi(DataClassification::HasasPhi, DisKanal::DisAi);
        assert!(!h.ne_oldu.is_empty());
        assert!(!h.neden.is_empty());
        assert!(!h.nasil_cozulur.is_empty());
        assert_eq!(h.eylem_etiketi.as_deref(), Some("Anonimleştir"));
    }

    #[test]
    fn kisit_seviyesi_siralamasi() {
        assert!(
            kisit_seviyesi(DataClassification::Sentetik)
                < kisit_seviyesi(DataClassification::Normal)
        );
        assert!(
            kisit_seviyesi(DataClassification::Normal)
                < kisit_seviyesi(DataClassification::HasasPhi)
        );
    }
}
