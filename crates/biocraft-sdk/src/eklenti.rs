//! SDK — Eklenti **aktivasyon kontratı** (MK-17, MK-13).
//!
//! Bir eklenti yüklendiğinde host onun aktivasyon giriş noktasını çağırır; eklenti
//! karşılığında iki şey üretir/kullanır:
//!
//! * [`Aktivasyon`] — eklentinin host'a sunduğu **kayıtlar** (panel/sekme/menü/komut/ayar
//!   [`UiKayit`] + düğüm [`NodeTanimi`]).  Host bunları güvenli alanlarda gösterir.
//! * [`YetkiKapisi`] — host'un eklentiye verdiği **çalışma-zamanı yetki kapısı.**  Eklenti
//!   her yetenek (dosya/ağ/…) kullanımından önce buradan geçer; **ilan etmediği/onaylanmamış**
//!   bir yeteneği kullanamaz (MK-13, en az yetki).
//!
//! Bu modül yalnızca **kontratı** tanımlar (L1 → egui'ye/host'a bağlanamaz).  Yetki kümesini
//! hesaplayan otorite host'tadır (`biocraft-plugin-host::YetkiKumesi`); host hesapladığı
//! `istenen ∩ onaylanan` kümeyi bir [`YetkiKapisi`]'ye koyup eklentiye verir.  Böylece eklenti
//! host'a doğrudan bağlanmaz; yalnızca bu sınırı görür.

use std::collections::BTreeSet;

use biocraft_types::{Capability, ErrorReport};

use crate::node::NodeTanimi;
use crate::ui::{UiKayit, UiUzantiTuru};

// ─── Yetki kapısı (çalışma-zamanı capability denetimi) ────────────────────────

/// Host'un eklentiye verdiği **fiilen kullanılabilir** yetkilerin kapısı.
///
/// Host, manifest'te ilan edilen ile kullanıcının onayladığı yetkilerin **kesişimini**
/// (`istenen ∩ onaylanan`) buraya koyar.  Eklenti yetki-kapılı her işlemden önce
/// [`iste`](Self::iste) (Result) veya [`var_mi`](Self::var_mi) (sessiz sorgu) çağırır.
/// İlan edilmemiş/onaylanmamış bir yetki **asla** verilmez (MK-13).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct YetkiKapisi {
    verilen: BTreeSet<Capability>,
}

impl YetkiKapisi {
    /// Verilen yetki kümesinden bir kapı kurar (host bunu `istenen ∩ onaylanan`'dan üretir).
    pub fn yeni(verilen: impl IntoIterator<Item = Capability>) -> Self {
        Self {
            verilen: verilen.into_iter().collect(),
        }
    }

    /// Hiç yetki içermeyen kapı (varsayılan = en az yetki).
    pub fn bos() -> Self {
        Self::default()
    }

    /// Bu kapı verilen yetkiyi içeriyor mu? (sessiz sorgu — özellik ifşa kararı için.)
    pub fn var_mi(&self, cap: Capability) -> bool {
        self.verilen.contains(&cap)
    }

    /// Çalışma-zamanı denetimi: yetki yoksa açıklayıcı [`ErrorReport`] döner.
    ///
    /// Eklentinin yetki-kapılı her işlemi (dosya/ağ/veritabanı/…) çağrı başında bunu çağırır;
    /// host tarafı da kendi sınırında (defansif derinlik) aynı denetimi tekrarlar.
    pub fn iste(&self, cap: Capability) -> Result<(), ErrorReport> {
        if self.var_mi(cap) {
            Ok(())
        } else {
            let ad = crate::yetenek_metni(cap);
            Err(ErrorReport::new(
                "Eklenti erişimi reddedildi",
                format!("eklenti '{ad}' yetkisini kullanmaya çalıştı ama bu yetki verilmemiş"),
                format!("Eklentiye '{ad}' iznini vermek için eklenti ayarlarından yetkilerini onaylayın"),
            )
            .with_eylem("İzinleri yönet"))
        }
    }

    /// Verilen yetkilerin sıralı listesi (UI/teşhis için).
    pub fn liste(&self) -> Vec<Capability> {
        self.verilen.iter().copied().collect()
    }

    /// Verilen yetki sayısı.
    pub fn sayi(&self) -> usize {
        self.verilen.len()
    }
}

// ─── Aktivasyon (eklentinin host'a sunduğu kayıtlar) ──────────────────────────

/// Bir eklentinin aktivasyonda host'a sunduğu kayıtların toplamı.
///
/// Eklenti, alt-modüllerinin kayıtlarını [`birlestir`](Self::birlestir) ile toplar; host
/// sonuçta UI kayıtlarını uzantı noktalarına, düğümleri node paletine ekler.  Hiçbir kayıt
/// **sessizce ezilmez** (çakışma yönetimi host kayıt defterindedir).
///
/// `Eq` türetilmez: [`NodeTanimi`] ondalık parametre (f64) taşıyabildiği için yalnızca
/// `PartialEq`'dir; aktivasyon eşitliği de bu yüzden `PartialEq` ile yapılır.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Aktivasyon {
    /// UI uzantı kayıtları (panel/sekme/menü/komut/ayar).
    pub ui: Vec<UiKayit>,
    /// Düğüm (node) tür kayıtları (İP-05 grafiği için).
    pub nodelar: Vec<NodeTanimi>,
}

impl Aktivasyon {
    /// Boş aktivasyon.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir UI kaydı ekler (akıcı zincir için `&mut self` döndürür).
    pub fn ui_ekle(&mut self, kayit: UiKayit) -> &mut Self {
        self.ui.push(kayit);
        self
    }

    /// Bir **panel** (yan/alt panel; Activity Bar girişiyle eşleşir) kaydı ekler.
    pub fn panel(&mut self, kimlik: impl Into<String>, baslik: impl Into<String>) -> &mut Self {
        self.ui_ekle(UiKayit::yeni(kimlik, baslik, UiUzantiTuru::Panel))
    }

    /// Bir **sekme** (editör sekmesi) kaydı ekler.
    pub fn sekme(&mut self, kimlik: impl Into<String>, baslik: impl Into<String>) -> &mut Self {
        self.ui_ekle(UiKayit::yeni(kimlik, baslik, UiUzantiTuru::Sekme))
    }

    /// Bir **komut** (komut paleti) kaydı ekler.
    pub fn komut(&mut self, kimlik: impl Into<String>, baslik: impl Into<String>) -> &mut Self {
        self.ui_ekle(UiKayit::yeni(kimlik, baslik, UiUzantiTuru::Komut))
    }

    /// Bir **menü** öğesi kaydı ekler.
    pub fn menu(&mut self, kimlik: impl Into<String>, baslik: impl Into<String>) -> &mut Self {
        self.ui_ekle(UiKayit::yeni(kimlik, baslik, UiUzantiTuru::Menu))
    }

    /// Bir **ayar** sayfası kaydı ekler.
    pub fn ayar(&mut self, kimlik: impl Into<String>, baslik: impl Into<String>) -> &mut Self {
        self.ui_ekle(UiKayit::yeni(kimlik, baslik, UiUzantiTuru::Ayar))
    }

    /// Bir **düğüm** (node) türü kaydı ekler.
    pub fn node(&mut self, tanim: NodeTanimi) -> &mut Self {
        self.nodelar.push(tanim);
        self
    }

    /// Başka bir aktivasyonun kayıtlarını bu aktivasyona katar (alt-modül toplama).
    pub fn birlestir(&mut self, diger: Aktivasyon) -> &mut Self {
        self.ui.extend(diger.ui);
        self.nodelar.extend(diger.nodelar);
        self
    }

    /// Belirli bir türdeki UI kayıtlarının sayısı (teşhis/test için).
    pub fn ui_say(&self, tur: UiUzantiTuru) -> usize {
        self.ui.iter().filter(|k| k.tur == tur).count()
    }

    /// Belirli bir türdeki UI kayıtlarını döndürür (host'a bağlamada kullanılır).
    pub fn ui_turden(&self, tur: UiUzantiTuru) -> impl Iterator<Item = &UiKayit> {
        self.ui.iter().filter(move |k| k.tur == tur)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kapi_verilen_yetkiye_izin_verir() {
        let k = YetkiKapisi::yeni([Capability::Fs, Capability::Db]);
        assert!(k.var_mi(Capability::Fs));
        assert!(k.iste(Capability::Db).is_ok());
        assert_eq!(k.sayi(), 2);
    }

    #[test]
    fn kapi_ilan_edilmeyen_yetkiyi_reddeder() {
        // Eklenti fs/db ister; net İSTEMEZ → host net'i vermez → kapı reddeder.
        let k = YetkiKapisi::yeni([Capability::Fs, Capability::Db]);
        let hata = k.iste(Capability::Net).unwrap_err();
        assert_eq!(hata.ne_oldu, "Eklenti erişimi reddedildi");
        assert!(hata.neden.contains("net"));
        assert!(!k.var_mi(Capability::Net));
    }

    #[test]
    fn bos_kapi_her_seyi_reddeder() {
        let k = YetkiKapisi::bos();
        assert!(k.iste(Capability::Fs).is_err());
        assert_eq!(k.sayi(), 0);
    }

    #[test]
    fn yetki_kapisi_kume_tekille_ve_sirali() {
        // Tekrarlı girdi tekilleşir; liste sıralı (kararlı) döner.
        let k = YetkiKapisi::yeni([Capability::Db, Capability::Fs, Capability::Db]);
        assert_eq!(k.sayi(), 2);
        let l = k.liste();
        assert_eq!(l.len(), 2);
        let mut sirali = l.clone();
        sirali.sort();
        assert_eq!(l, sirali, "liste kararlı/sıralı olmalı");
    }

    #[test]
    fn aktivasyon_kayit_toplar() {
        let mut a = Aktivasyon::yeni();
        a.panel("p.studio", "BioCraft Studio")
            .komut("k.hakkinda", "Hakkında")
            .komut("k.merhaba", "Hoş Geldin")
            .ayar("a.studio", "BioCraft Studio");
        assert_eq!(a.ui_say(UiUzantiTuru::Panel), 1);
        assert_eq!(a.ui_say(UiUzantiTuru::Komut), 2);
        assert_eq!(a.ui_say(UiUzantiTuru::Ayar), 1);
        assert_eq!(a.nodelar.len(), 0);
    }

    #[test]
    fn aktivasyon_birlestirir() {
        let mut ana = Aktivasyon::yeni();
        ana.panel("p", "P");
        let mut alt = Aktivasyon::yeni();
        alt.komut("k", "K");
        ana.birlestir(alt);
        assert_eq!(ana.ui.len(), 2);
        assert_eq!(ana.ui_say(UiUzantiTuru::Komut), 1);
    }
}
