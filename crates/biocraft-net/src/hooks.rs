//! **Pasif kanca kayıt defteri** (İP-15) — dağıtık ağın çekirdek tarafı.
//!
//! [`DagitikAg`], dağıtık-ağ eklentisinin bağlanacağı tek noktadır.  Tasarımın kalbi **pasifliktir**:
//!
//! - **Eklenti yokken sıfır maliyet (MK-50):** Kayıt defteri yalnızca bir `Option<Arc<dyn …>>`'tür.
//!   Eklenti kayıtlı değilken `None`'dır → arka plan görevi yok, soket yok, ayrılan bellek yok.  Tüm
//!   ağ yolları `None`'da kısa devre yapar ("eklenti gerekli — [İndir]") ve hiçbir iş yapmaz.
//! - **Varsayılan KAPALI (MK-50):** Eklenti kayıtlı olsa bile ağ, kullanıcı açıkça
//!   [`DagitikAg::etkinlestir`] demedikçe **kapalıdır** → kullanıcı izni olmadan ağ etkinliği olmaz.
//! - **Veri sınırı son denetimi (MK-43):** Gönderim anında, eklentiye devretmeden ÖNCE her iş yükü
//!   tekrar çıkış kapısından geçirilir (savunma katmanı — "tek yol bile atlamamalı").

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use biocraft_types::ErrorReport;

use crate::job::{Is, IsDurumu, IsKimlik, IsSonucu};
use crate::limits::KaynakSiniri;

/// Dağıtık-ağ eklentisinin (kurulu değilse) indirme yönlendirme adresi (İP-15).
///
/// Sihirbazdaki (`biocraft-ui` İP-02) `DAGITIK_AG_EKLENTI_URL` ile **aynı** olmalıdır (tutarlı [İndir]).
pub const DAGITIK_AG_EKLENTI_URL: &str = "https://biocraftengine.com/eklentiler/dagitik-ag";

/// Dağıtık-ağ sağlayıcısının (eklenti) kimliği.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaglayiciKimlik {
    /// Eklenti kimliği (örn. `biocraft.<yayinci>.dagitik-ag`).
    pub kimlik: String,
    /// İnsan-okunur ad.
    pub ad: String,
    /// Sürüm.
    pub surum: String,
}

/// Dağıtık ağın **durumu** — UI bunu rozet/yönlendirme olarak gösterir (İP-15/İP-16).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgDurumu {
    /// Eklenti kurulu değil → [İndir] yönlendirmesi gösterilir.
    EklentiYok {
        /// İndirme adresi.
        indir_url: String,
    },
    /// Eklenti kurulu ama ağ **kapalı** (varsayılan) — kullanıcı açabilir.
    Kapali(SaglayiciKimlik),
    /// Eklenti kurulu ve ağ **açık** — iş gönderilebilir.
    Hazir(SaglayiciKimlik),
}

/// **Dağıtık-ağ sağlayıcı arayüzü** (eklenti uygular) — iş gönderme/sonuç toplama + kaynak sınırı.
///
/// `Send + Sync`: çekirdek bunu `Arc` ardında tutar, arka plan runtime'ından erişilebilir.  Gerçek
/// P2P/Iroh/iş dağıtımı bu trait'in implementasyonunda (eklentide) yaşar.  MVP'de hiçbir
/// implementasyon kayıtlı değildir (sıfır maliyet, MK-50).
pub trait DagitikAgSaglayici: Send + Sync {
    /// Eklenti kimliği.
    fn kimlik(&self) -> SaglayiciKimlik;

    /// Bir işi ağa gönderir.
    fn is_gonder(&self, is: Is) -> Result<IsKimlik, Box<ErrorReport>>;

    /// Bir işin durumunu sorgular.
    fn is_durumu(&self, is: &IsKimlik) -> Result<IsDurumu, Box<ErrorReport>>;

    /// Bir işin (kısmi olabilen) sonuçlarını toplar.
    fn sonuclari_topla(&self, is: &IsKimlik) -> Result<Vec<IsSonucu>, Box<ErrorReport>>;

    /// Bu makinenin paylaşacağı kaynak sınırını ayarlar (opt-in).
    fn kaynak_siniri_ayarla(&self, sinir: KaynakSiniri) -> Result<(), Box<ErrorReport>>;
}

/// Dağıtık ağ kanca kayıt defteri — **pasif** (eklenti yokken sıfır maliyet).
///
/// `Default` = eklenti yok + kapalı.  Sadece bir `Option` + iki düz alan tutar → boştayken hiçbir
/// kaynak kullanmaz.
#[derive(Default)]
pub struct DagitikAg {
    /// Kayıtlı eklenti sağlayıcı (yoksa `None` = sıfır maliyet).
    saglayici: Option<Arc<dyn DagitikAgSaglayici>>,
    /// Ağ kullanıcı tarafından etkinleştirildi mi?  Varsayılan **false** (KAPALI, MK-50).
    etkin: bool,
    /// Bu makinenin kaynak paylaşım sınırı (opt-in; varsayılan paylaşım yok).
    kaynak_siniri: KaynakSiniri,
}

impl DagitikAg {
    /// Boş (pasif) bir kayıt defteri kurar — eklenti yok, ağ kapalı.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Dağıtık-ağ eklenti sağlayıcısını kaydeder (host eklentiyi yükleyince çağırır).
    ///
    /// Kayıt **ağı açmaz**: varsayılan hâlâ KAPALI'dır; kullanıcı [`etkinlestir`](Self::etkinlestir)
    /// demelidir (MK-50).
    pub fn saglayici_kaydet(&mut self, saglayici: Arc<dyn DagitikAgSaglayici>) {
        self.saglayici = Some(saglayici);
    }

    /// Eklenti kayıtlı mı?
    pub fn eklenti_var_mi(&self) -> bool {
        self.saglayici.is_some()
    }

    /// Ağ açık mı?  (Eklenti var **ve** kullanıcı etkinleştirmiş.)
    pub fn etkin_mi(&self) -> bool {
        self.etkin && self.eklenti_var_mi()
    }

    /// Ağı etkinleştirir (yalnızca eklenti varsa anlamlı).
    pub fn etkinlestir(&mut self) {
        self.etkin = true;
    }

    /// Ağı devre dışı bırakır.
    pub fn devre_disi_birak(&mut self) {
        self.etkin = false;
    }

    /// Güncel ağ durumu (UI rozeti/yönlendirmesi için).
    pub fn durum(&self) -> AgDurumu {
        match &self.saglayici {
            None => AgDurumu::EklentiYok {
                indir_url: DAGITIK_AG_EKLENTI_URL.to_string(),
            },
            Some(s) if self.etkin => AgDurumu::Hazir(s.kimlik()),
            Some(s) => AgDurumu::Kapali(s.kimlik()),
        }
    }

    /// Mevcut kaynak paylaşım sınırı.
    pub fn kaynak_siniri(&self) -> KaynakSiniri {
        self.kaynak_siniri
    }

    /// Kaynak paylaşım sınırını ayarlar; eklenti varsa ona da iletir.
    pub fn kaynak_siniri_ayarla(&mut self, sinir: KaynakSiniri) -> Result<(), Box<ErrorReport>> {
        let sinir = sinir.gecerli_kil();
        self.kaynak_siniri = sinir;
        match &self.saglayici {
            Some(s) => s.kaynak_siniri_ayarla(sinir),
            None => Ok(()), // eklenti yok → yalnızca tercih saklanır (pasif)
        }
    }

    /// Bir işi ağa gönderir.  Kapılar sırayla:
    /// 1. Eklenti yoksa → [İndir] hatası (pasif, hiç ağ etkinliği yok).
    /// 2. Ağ kapalıysa → "ağı açın" hatası (varsayılan KAPALI).
    /// 3. Herhangi bir yük çıkış kapısından geçmiyorsa → **engellenir** (MK-43 son denetim).
    /// 4. Hepsi tamamsa → eklentiye devredilir.
    pub fn is_gonder(&self, is: Is) -> Result<IsKimlik, Box<ErrorReport>> {
        let saglayici = match &self.saglayici {
            None => return Err(Box::new(eklenti_yok_hatasi())),
            Some(s) => s,
        };
        if !self.etkin {
            return Err(Box::new(ag_kapali_hatasi()));
        }
        // MK-43 savunma katmanı: eklentiye devretmeden önce sınırı bir kez daha uygula.
        if !is.tum_yukler_kapidan_gecer_mi() {
            return Err(Box::new(sinir_ihlali_hatasi()));
        }
        saglayici.is_gonder(is)
    }

    /// Bir işin durumunu sorgular (eklenti yoksa/ağ kapalıysa hata).
    pub fn is_durumu(&self, is: &IsKimlik) -> Result<IsDurumu, Box<ErrorReport>> {
        self.aktif_saglayici()?.is_durumu(is)
    }

    /// Bir işin sonuçlarını toplar (eklenti yoksa/ağ kapalıysa hata).
    pub fn sonuclari_topla(&self, is: &IsKimlik) -> Result<Vec<IsSonucu>, Box<ErrorReport>> {
        self.aktif_saglayici()?.sonuclari_topla(is)
    }

    /// Aktif (kayıtlı + etkin) sağlayıcıyı döndürür ya da uygun hatayı verir.
    fn aktif_saglayici(&self) -> Result<&Arc<dyn DagitikAgSaglayici>, Box<ErrorReport>> {
        match &self.saglayici {
            None => Err(Box::new(eklenti_yok_hatasi())),
            Some(_) if !self.etkin => Err(Box::new(ag_kapali_hatasi())),
            Some(s) => Ok(s),
        }
    }
}

/// "Dağıtık ağ eklentisi gerekli — [İndir]" standart hatası (İP-15/İP-16 şeması).
pub fn eklenti_yok_hatasi() -> ErrorReport {
    ErrorReport::new(
        "Dağıtık ağ için eklenti gerekli",
        "Dağıtık (P2P) hesaplama bu sürümde bir eklenti ile gelir ve şu an kurulu değil. \
         Eklenti kurulu olmadığından hiçbir ağ etkinliği yapılmadı.",
        "Dağıtık ağ eklentisini indirip kurun; sonra ağı ayarlardan açabilirsiniz.",
    )
    .with_eylem("İndir")
    .with_teknik_detay(format!(
        "DagitikAg: saglayici=None → EklentiYok ({DAGITIK_AG_EKLENTI_URL})"
    ))
}

/// "Ağ kapalı — açın" standart hatası (varsayılan KAPALI, MK-50).
pub fn ag_kapali_hatasi() -> ErrorReport {
    ErrorReport::new(
        "Dağıtık ağ kapalı",
        "Eklenti kurulu ancak dağıtık ağ varsayılan olarak kapalı. İzniniz olmadan ağ etkinliği başlatılmaz.",
        "Dağıtık ağı kullanmak için ayarlardan etkinleştirin.",
    )
    .with_eylem("Ağı aç")
    .with_teknik_detay("DagitikAg: etkin=false → Kapali".to_string())
}

/// Çıkış kapısını geçemeyen yük içeren iş için standart hata (MK-43 son denetim).
pub fn sinir_ihlali_hatasi() -> ErrorReport {
    ErrorReport::new(
        "İş gönderilemedi: hassas veri sınırı",
        "İşin içindeki en az bir yük dış-paylaşıma uygun değil (PHI/hassas). Çekirdek sınır, bu işin \
         ağa çıkmasını engelledi (MK-42/43).",
        "Yalnızca meta veri/özet sonuç gönderin ya da veriyi anonimleştirin; gerçek PHI ağa çıkamaz.",
    )
    .with_eylem("Anonimleştir")
    .with_teknik_detay("DagitikAg::is_gonder: tum_yukler_kapidan_gecer_mi=false → engellendi".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::P2pYuku;
    use biocraft_types::DataClassification;

    /// İşleri yutan basit sahte sağlayıcı (gerçek ağ yok — test için).
    struct SahteSaglayici {
        gonderilen: std::sync::Mutex<Vec<Is>>,
    }

    impl SahteSaglayici {
        fn yeni() -> Self {
            Self {
                gonderilen: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    impl DagitikAgSaglayici for SahteSaglayici {
        fn kimlik(&self) -> SaglayiciKimlik {
            SaglayiciKimlik {
                kimlik: "biocraft.test.dagitik".into(),
                ad: "Sahte Dağıtık Ağ".into(),
                surum: "0.0.0".into(),
            }
        }
        fn is_gonder(&self, is: Is) -> Result<IsKimlik, Box<ErrorReport>> {
            self.gonderilen.lock().unwrap().push(is);
            Ok(IsKimlik::yeni("is-1"))
        }
        fn is_durumu(&self, _is: &IsKimlik) -> Result<IsDurumu, Box<ErrorReport>> {
            Ok(IsDurumu::Beklemede)
        }
        fn sonuclari_topla(&self, _is: &IsKimlik) -> Result<Vec<IsSonucu>, Box<ErrorReport>> {
            Ok(Vec::new())
        }
        fn kaynak_siniri_ayarla(&self, _sinir: KaynakSiniri) -> Result<(), Box<ErrorReport>> {
            Ok(())
        }
    }

    fn ornek_is() -> Is {
        let y = P2pYuku::metadata(DataClassification::Normal, "param", vec![1, 2]).unwrap();
        Is::yeni("hizalama", vec![y])
    }

    #[test]
    fn eklenti_yokken_sifir_maliyet_ve_indir() {
        // MK-50: eklenti yok → durum EklentiYok([İndir]); gönderim hiç ağ etkinliği yapmadan hata döner.
        let ag = DagitikAg::yeni();
        assert!(!ag.eklenti_var_mi());
        assert!(!ag.etkin_mi());
        match ag.durum() {
            AgDurumu::EklentiYok { indir_url } => assert_eq!(indir_url, DAGITIK_AG_EKLENTI_URL),
            d => panic!("eklenti yokken EklentiYok beklenir, bulundu: {d:?}"),
        }
        let r = ag.is_gonder(ornek_is());
        assert!(r.is_err(), "eklenti yokken iş gönderilememeli");
        assert_eq!(r.unwrap_err().eylem_etiketi.as_deref(), Some("İndir"));
    }

    #[test]
    fn kayit_defteri_sadece_option_kadar_yer_tutar() {
        // "Sıfır maliyet" göstergesi: pasif kayıt defteri yalnızca küçük, yığın-dışı (heap'siz) bir
        // yapıdır — boştayken hiçbir tahsisat yapmaz.
        let ag = DagitikAg::yeni();
        assert!(ag.saglayici.is_none(), "boşta None — arka plan kaynağı yok");
        assert_eq!(ag.kaynak_siniri(), KaynakSiniri::default());
    }

    #[test]
    fn eklenti_var_ama_varsayilan_kapali() {
        // MK-50: kayıt ağı AÇMAZ → varsayılan KAPALI; gönderim "ağı aç" hatası verir.
        let mut ag = DagitikAg::yeni();
        ag.saglayici_kaydet(Arc::new(SahteSaglayici::yeni()));
        assert!(ag.eklenti_var_mi());
        assert!(!ag.etkin_mi(), "kayıt ağı açmaz — varsayılan kapalı");
        assert!(matches!(ag.durum(), AgDurumu::Kapali(_)));
        let r = ag.is_gonder(ornek_is());
        assert_eq!(r.unwrap_err().eylem_etiketi.as_deref(), Some("Ağı aç"));
    }

    #[test]
    fn etkinlestirilince_is_eklentiye_gider() {
        let mut ag = DagitikAg::yeni();
        let s = Arc::new(SahteSaglayici::yeni());
        ag.saglayici_kaydet(s.clone());
        ag.etkinlestir();
        assert!(ag.etkin_mi());
        assert!(matches!(ag.durum(), AgDurumu::Hazir(_)));
        let kimlik = ag.is_gonder(ornek_is()).expect("açıkken normal iş geçmeli");
        assert_eq!(kimlik, IsKimlik::yeni("is-1"));
        assert_eq!(
            s.gonderilen.lock().unwrap().len(),
            1,
            "iş eklentiye iletildi"
        );
    }

    #[test]
    fn devre_disi_birakinca_tekrar_kapali() {
        let mut ag = DagitikAg::yeni();
        ag.saglayici_kaydet(Arc::new(SahteSaglayici::yeni()));
        ag.etkinlestir();
        ag.devre_disi_birak();
        assert!(!ag.etkin_mi());
        assert!(ag.is_gonder(ornek_is()).is_err());
    }

    #[test]
    fn kaynak_siniri_eklentisiz_saklanir() {
        // Eklenti yokken bile tercih saklanır (pasif); yüzde aralığa sıkıştırılır.
        let mut ag = DagitikAg::yeni();
        ag.kaynak_siniri_ayarla(KaynakSiniri {
            etkin: true,
            azami_cpu_yuzde: 200,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(ag.kaynak_siniri().azami_cpu_yuzde, 100);
    }
}
