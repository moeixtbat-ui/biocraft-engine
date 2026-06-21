//! UI uzantı **kayıt defteri** + çakışma/sıra yönetimi (İP-07 uzantı noktaları).
//!
//! Eklenti; panel/sekme/menü/komut/ayar (bkz. SDK [`UiKayit`]) ve düğüm
//! (bkz. SDK [`NodeTanimi`]) **kaydı** ilan eder; çekirdek bunları **güvenli alanlarda**
//! gösterir (MK-17).  Eklentiler birbirine bağlanmaz; kayıtlar burada toplanır.
//!
//! **Çakışma yönetimi (sessiz bozmaz):** İki eklenti aynı alanı (aynı `tur` + aynı
//! `kimlik` = "yuva") isterse hiçbiri **sessizce ezilmez.** Her kayıt sahibine göre
//! ad-uzaylanır (`{tur}/{sahip}/{kimlik}`), ikisi de görünür; sıra **öncelik** sonra
//! kimlik ile belirlenir ve **kullanıcı yeniden düzenleyebilir/gizleyebilir.** Aynı
//! yuvayı paylaşan kayıtlar [`KayitDefteri::cakismalar`] ile kullanıcıya bildirilir.

use biocraft_sdk::node::NodeTanimi;
use biocraft_sdk::ui::{UiKayit, UiUzantiTuru};
use biocraft_types::ErrorReport;
use std::collections::BTreeMap;

/// Çekirdeğin önemsediği bir kaydın benzersiz global anahtarı: `{tur}/{sahip}/{kimlik}`.
fn global_anahtar(tur: UiUzantiTuru, sahip: &str, kimlik: &str) -> String {
    format!("{}/{sahip}/{kimlik}", tur_etiketi(tur))
}

/// Bir UI uzantı türünün kararlı metin etiketi (anahtar üretiminde + UI gruplamada).
pub fn tur_etiketi(tur: UiUzantiTuru) -> &'static str {
    match tur {
        UiUzantiTuru::Panel => "panel",
        UiUzantiTuru::Sekme => "sekme",
        UiUzantiTuru::Menu => "menu",
        UiUzantiTuru::Komut => "komut",
        UiUzantiTuru::Ayar => "ayar",
    }
}

/// Bir eklentinin tek bir UI kaydı + çekirdek meta verisi (sahip + öncelik).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KayitliUzanti {
    /// Kaydı yapan eklentinin kimliği (`biocraft.<yayinci>.<eklenti>`).
    pub sahip: String,
    /// SDK kaydı (kimlik/başlık/tür).
    pub kayit: UiKayit,
    /// Sıralama önceliği (büyük = önce).  Resmi/çekirdek kayıtları daha yüksek alır.
    pub oncelik: i32,
}

impl KayitliUzanti {
    /// Bu kaydın global anahtarı.
    pub fn anahtar(&self) -> String {
        global_anahtar(self.kayit.tur, &self.sahip, &self.kayit.kimlik)
    }
}

/// Kullanıcının bir kayıt için verdiği yerel tercih (sıra geçersiz kılma + görünürlük).
///
/// Serileştirilebilir → oturumlar arası kalıcı (`biocraft-state` üzerinden saklanır).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KullaniciTercihi {
    /// Kullanıcının verdiği özel sıra (varsa önceliği ezer; küçük = önce).
    pub sira: Option<i32>,
    /// Kullanıcı bu kaydı gizledi mi?
    pub gizli: bool,
}

/// Aynı yuvayı (tur + kimlik) paylaşan birden çok eklentinin oluşturduğu çakışma.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cakisma {
    /// Uzantı türü.
    pub tur: UiUzantiTuru,
    /// Paylaşılan yuva kimliği.
    pub kimlik: String,
    /// Bu yuvayı isteyen eklenti sahipleri (öncelik sırasında).
    pub sahipler: Vec<String>,
}

/// Tüm eklentilerin UI/düğüm kayıtlarını toplayan, çakışma/sırayı yöneten kayıt defteri.
#[derive(Debug, Clone, Default)]
pub struct KayitDefteri {
    ui: Vec<KayitliUzanti>,
    nodelar: Vec<(String, NodeTanimi)>,
    // Kullanıcı tercihleri: global anahtar → tercih.
    tercihler: BTreeMap<String, KullaniciTercihi>,
}

impl KayitDefteri {
    /// Boş bir kayıt defteri.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir UI uzantısı kaydeder.
    ///
    /// Aynı eklenti **aynı (tur, kimlik)**'i iki kez kaydederse bu eklentinin kendi
    /// hatasıdır → reddedilir.  Farklı eklentiler aynı yuvayı isterse **ikisi de tutulur**
    /// (ad-uzaylı; sessiz bozma yok) ve [`cakismalar`] ile raporlanır.
    pub fn kaydet(
        &mut self,
        sahip: impl Into<String>,
        kayit: UiKayit,
        oncelik: i32,
    ) -> Result<(), ErrorReport> {
        let sahip = sahip.into();
        let anahtar = global_anahtar(kayit.tur, &sahip, &kayit.kimlik);
        if self.ui.iter().any(|u| u.anahtar() == anahtar) {
            return Err(ErrorReport::new(
                "Eklenti aynı kaydı iki kez yaptı",
                format!(
                    "'{sahip}' eklentisi '{}' türünde '{}' kimliğini birden çok kez kaydetti",
                    tur_etiketi(kayit.tur),
                    kayit.kimlik
                ),
                "Bu eklentinin bir hatasıdır; geliştiriciye bildirin",
            ));
        }
        self.ui.push(KayitliUzanti {
            sahip,
            kayit,
            oncelik,
        });
        Ok(())
    }

    /// Bir düğüm (node) türü kaydeder (İP-05 grafiği için).
    pub fn node_kaydet(&mut self, sahip: impl Into<String>, node: NodeTanimi) {
        self.nodelar.push((sahip.into(), node));
    }

    /// Belirli bir türdeki kayıtları **gösterim sırasında** döndürür (gizliler hariç).
    ///
    /// Sıra: (1) kullanıcı özel sırası (varsa), (2) öncelik (büyük önce),
    /// (3) sahip, (4) kimlik — tümüyle **belirleyici** (kararlı).
    pub fn alan(&self, tur: UiUzantiTuru) -> Vec<&KayitliUzanti> {
        let mut sonuc: Vec<&KayitliUzanti> = self
            .ui
            .iter()
            .filter(|u| u.kayit.tur == tur)
            .filter(|u| !self.tercih(u).gizli)
            .collect();
        sonuc.sort_by(|a, b| {
            let ta = self.tercih(a);
            let tb = self.tercih(b);
            // Kullanıcı sırası verilmişse önce o (küçük = önce); yoksa eşit say.
            match (ta.sira, tb.sira) {
                (Some(x), Some(y)) => x.cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
            // Sonra öncelik (büyük önce), sonra sahip, sonra kimlik.
            .then(b.oncelik.cmp(&a.oncelik))
            .then(a.sahip.cmp(&b.sahip))
            .then(a.kayit.kimlik.cmp(&b.kayit.kimlik))
        });
        sonuc
    }

    /// Kayıtlı düğüm türleri.
    pub fn nodelar(&self) -> &[(String, NodeTanimi)] {
        &self.nodelar
    }

    /// Aynı yuvayı (tur + kimlik) birden çok eklentinin paylaştığı çakışmalar.
    ///
    /// Kullanıcıya "şu iki eklenti aynı yere yerleşmek istiyor" diye gösterilir;
    /// hiçbiri kaybolmaz, kullanıcı sırayı/görünürlüğü ayarlayabilir.
    pub fn cakismalar(&self) -> Vec<Cakisma> {
        // (tur, kimlik) → sahipler (öncelik sırasında).
        let mut harita: BTreeMap<(UiUzantiTuru, String), Vec<&KayitliUzanti>> = BTreeMap::new();
        for u in &self.ui {
            harita
                .entry((u.kayit.tur, u.kayit.kimlik.clone()))
                .or_default()
                .push(u);
        }
        let mut sonuc = Vec::new();
        for ((tur, kimlik), mut grup) in harita {
            if grup.len() < 2 {
                continue;
            }
            grup.sort_by(|a, b| b.oncelik.cmp(&a.oncelik).then(a.sahip.cmp(&b.sahip)));
            sonuc.push(Cakisma {
                tur,
                kimlik,
                sahipler: grup.iter().map(|u| u.sahip.clone()).collect(),
            });
        }
        sonuc
    }

    /// Bir kaydı kullanıcı sırasına göre konumlandırır (özel sıra atar).
    pub fn kullanici_sirala(&mut self, anahtar: &str, sira: i32) {
        self.tercihler.entry(anahtar.to_string()).or_default().sira = Some(sira);
    }

    /// Bir kaydı gizler/gösterir.
    pub fn kullanici_gizle(&mut self, anahtar: &str, gizli: bool) {
        self.tercihler.entry(anahtar.to_string()).or_default().gizli = gizli;
    }

    /// Bir eklentinin **tüm** kayıtlarını kaldırır (kaldırma/çökme/devre-dışı bırakma).
    pub fn eklenti_kaldir(&mut self, sahip: &str) {
        self.ui.retain(|u| u.sahip != sahip);
        self.nodelar.retain(|(s, _)| s != sahip);
    }

    /// Toplam UI kaydı sayısı (teşhis).
    pub fn ui_sayisi(&self) -> usize {
        self.ui.len()
    }

    /// Kayıt tercihlerini (kalıcılık için) dışa verir.
    pub fn tercihleri(&self) -> &BTreeMap<String, KullaniciTercihi> {
        &self.tercihler
    }

    /// Kalıcı tercihleri geri yükler (oturum açılışında).
    pub fn tercihleri_yukle(&mut self, tercihler: BTreeMap<String, KullaniciTercihi>) {
        self.tercihler = tercihler;
    }

    fn tercih(&self, u: &KayitliUzanti) -> KullaniciTercihi {
        self.tercihler
            .get(&u.anahtar())
            .cloned()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kayit(kimlik: &str, baslik: &str, tur: UiUzantiTuru) -> UiKayit {
        UiKayit::yeni(kimlik, baslik, tur)
    }

    #[test]
    fn kayit_cekirdekte_gorunur() {
        let mut d = KayitDefteri::yeni();
        d.kaydet(
            "biocraft.acme.arac",
            kayit("panel.sonuc", "Sonuçlar", UiUzantiTuru::Panel),
            0,
        )
        .unwrap();
        d.kaydet(
            "biocraft.acme.arac",
            kayit("komut.calistir", "Çalıştır", UiUzantiTuru::Komut),
            0,
        )
        .unwrap();
        assert_eq!(d.alan(UiUzantiTuru::Panel).len(), 1);
        assert_eq!(d.alan(UiUzantiTuru::Komut).len(), 1);
        assert_eq!(d.alan(UiUzantiTuru::Sekme).len(), 0);
    }

    #[test]
    fn ayni_eklenti_ayni_kaydi_iki_kez_reddedilir() {
        let mut d = KayitDefteri::yeni();
        d.kaydet("biocraft.a.b", kayit("p1", "P", UiUzantiTuru::Panel), 0)
            .unwrap();
        assert!(d
            .kaydet(
                "biocraft.a.b",
                kayit("p1", "P tekrar", UiUzantiTuru::Panel),
                0
            )
            .is_err());
    }

    #[test]
    fn ayni_yuva_iki_eklenti_ikisi_de_tutulur() {
        // İki farklı eklenti aynı (Panel, "ana") yuvasını ister → sessiz bozma YOK.
        let mut d = KayitDefteri::yeni();
        d.kaydet(
            "biocraft.x.bir",
            kayit("ana", "Bir", UiUzantiTuru::Panel),
            5,
        )
        .unwrap();
        d.kaydet(
            "biocraft.y.iki",
            kayit("ana", "İki", UiUzantiTuru::Panel),
            10,
        )
        .unwrap();
        let alan = d.alan(UiUzantiTuru::Panel);
        assert_eq!(alan.len(), 2, "ikisi de görünmeli");
        // Yüksek öncelik (10) önce gelir.
        assert_eq!(alan[0].sahip, "biocraft.y.iki");
        // Çakışma raporlanır.
        let c = d.cakismalar();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].kimlik, "ana");
        assert_eq!(c[0].sahipler, vec!["biocraft.y.iki", "biocraft.x.bir"]);
    }

    #[test]
    fn kullanici_sirayi_ezebilir() {
        let mut d = KayitDefteri::yeni();
        d.kaydet(
            "biocraft.x.bir",
            kayit("ana", "Bir", UiUzantiTuru::Panel),
            5,
        )
        .unwrap();
        d.kaydet(
            "biocraft.y.iki",
            kayit("ana", "İki", UiUzantiTuru::Panel),
            10,
        )
        .unwrap();
        // Öncelik 'iki'yi öne koyardı; kullanıcı 'bir'i en öne çeksin.
        let bir_anahtar = global_anahtar(UiUzantiTuru::Panel, "biocraft.x.bir", "ana");
        d.kullanici_sirala(&bir_anahtar, -100);
        assert_eq!(d.alan(UiUzantiTuru::Panel)[0].sahip, "biocraft.x.bir");
    }

    #[test]
    fn kullanici_gizleyebilir() {
        let mut d = KayitDefteri::yeni();
        d.kaydet(
            "biocraft.x.bir",
            kayit("ana", "Bir", UiUzantiTuru::Panel),
            0,
        )
        .unwrap();
        let anahtar = global_anahtar(UiUzantiTuru::Panel, "biocraft.x.bir", "ana");
        d.kullanici_gizle(&anahtar, true);
        assert_eq!(d.alan(UiUzantiTuru::Panel).len(), 0);
        d.kullanici_gizle(&anahtar, false);
        assert_eq!(d.alan(UiUzantiTuru::Panel).len(), 1);
    }

    #[test]
    fn eklenti_kaldirinca_kayitlari_silinir() {
        let mut d = KayitDefteri::yeni();
        d.kaydet("biocraft.x.bir", kayit("p", "P", UiUzantiTuru::Panel), 0)
            .unwrap();
        d.kaydet("biocraft.y.iki", kayit("p2", "P2", UiUzantiTuru::Panel), 0)
            .unwrap();
        d.eklenti_kaldir("biocraft.x.bir");
        let alan = d.alan(UiUzantiTuru::Panel);
        assert_eq!(alan.len(), 1);
        assert_eq!(alan[0].sahip, "biocraft.y.iki");
    }

    #[test]
    fn cakisma_yoksa_bos() {
        let mut d = KayitDefteri::yeni();
        d.kaydet("biocraft.x.bir", kayit("a", "A", UiUzantiTuru::Panel), 0)
            .unwrap();
        d.kaydet("biocraft.x.bir", kayit("b", "B", UiUzantiTuru::Komut), 0)
            .unwrap();
        assert!(d.cakismalar().is_empty());
    }

    #[test]
    fn node_kaydi_tutulur() {
        let mut d = KayitDefteri::yeni();
        d.node_kaydet(
            "biocraft.x.bir",
            NodeTanimi {
                kimlik: "node.hizala".into(),
                baslik: "Hizala".into(),
                portlar: vec![],
                kategori: String::new(),
                aciklama: String::new(),
                parametreler: vec![],
            },
        );
        assert_eq!(d.nodelar().len(), 1);
        d.eklenti_kaldir("biocraft.x.bir");
        assert_eq!(d.nodelar().len(), 0);
    }
}
