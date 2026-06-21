//! Son projeler listesi — İP-01 (saf, test-edilebilir model).
//!
//! Launcher'ın kalbi: kullanıcının son açtığı projeleri ad/yol/tarih/önizleme ile tutar.
//! Sabitleme (pin), kaldırma, arama ve **taşınmış proje kurtarma** (TDA madde 19) buradadır.
//! Liste boşsa view katmanı "Henüz proje yok — Yeni Proje oluştur" rehberini gösterir (madde 5).
//!
//! Mantık tamamen saftır: zaman damgası ([`Timestamp`]) **dışarıdan** verilir (host
//! `biocraft_state::simdi()` ile enjekte eder) ve dosya varlığı kontrolü bir **closure** ile
//! enjekte edilir → gerçek diske dokunmadan, sahte saat/sahte dosya sistemiyle test edilir.
//! Kalıcılık [`SonProjelerListesi::serde_yaz`]/[`serde_oku`] ile JSON; host bunu atomik +
//! BLAKE3 bütünlüklü [`biocraft_ui::biocraft_state::DosyaDepo`]'ya yazar.

use std::path::{Path, PathBuf};

use biocraft_types::{ErrorReport, Timestamp};
use serde::{Deserialize, Serialize};

/// Listede tutulan en fazla proje sayısı (sabitlenenler bu sınırın dışında korunur).
pub const MAKS_PROJE: usize = 30;

/// Son açılan tek bir projenin kaydı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SonProje {
    /// Görünen ad (proje manifestinden; yoksa klasör adı).
    pub ad: String,
    /// Diskteki tam yol (`.bcproj` klasörü/dosyası — İP-02'de kesinleşir).
    pub yol: PathBuf,
    /// En son ne zaman açıldığı (UTC).  Sıralama bu damgaya göredir.
    pub son_acilma: Timestamp,
    /// Kullanıcı bu projeyi sabitledi mi (pin)?  Sabitler listenin başında, sınırın dışında kalır.
    #[serde(default)]
    pub sabit: bool,
    /// Kısa önizleme/açıklama metni (manifestten; opsiyonel — küçük önizleme yerine).
    #[serde(default)]
    pub ozet: Option<String>,
}

impl SonProje {
    /// Bu projenin kararlı kimliği (egui satır kimliği / arama için) — yolun metin hâli.
    ///
    /// Yol benzersizdir (bir konumda tek proje); taşınınca [`SonProjelerListesi::yeniden_bagla`]
    /// yolu günceller, böylece kimlik yeni konuma taşınır.
    pub fn kimlik(&self) -> String {
        self.yol.to_string_lossy().to_string()
    }
}

/// Taşınmış/silinmiş proje tespiti (TDA madde 19): yol diskte hâlâ var mı?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjeDurumu {
    /// Yol mevcut → doğrudan açılabilir.
    Mevcut,
    /// Yol bulunamadı (taşınmış/silinmiş) → "yeniden bağla" sunulur, sessizce kaybolmaz.
    Bulunamadi,
}

/// Bir projenin disk durumunu, varlık kontrolünü **enjekte ederek** belirler (test-edilebilir).
///
/// Host gerçek kontrolü `|p| p.exists()` ile geçer; test sahte bir closure verir.
pub fn proje_durumu(yol: &Path, mevcut_mu: impl Fn(&Path) -> bool) -> ProjeDurumu {
    if mevcut_mu(yol) {
        ProjeDurumu::Mevcut
    } else {
        ProjeDurumu::Bulunamadi
    }
}

/// Son projeler listesi — kalıcı, sıralanabilir, aranabilir.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SonProjelerListesi {
    /// Ham kayıtlar (sıralama [`sirali`](Self::sirali) ile hesaplanır; depolama sırası önemsiz).
    #[serde(default)]
    pub projeler: Vec<SonProje>,
}

impl SonProjelerListesi {
    /// Boş bir liste.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Liste boş mu (view boş-durum rehberini gösterir — TDA madde 5)?
    pub fn bos_mu(&self) -> bool {
        self.projeler.is_empty()
    }

    /// Kayıtlı proje sayısı.
    pub fn sayi(&self) -> usize {
        self.projeler.len()
    }

    /// Bir proje açıldı: varsa damgasını/adını günceller (öne taşınır), yoksa ekler.
    ///
    /// Eklemeden sonra liste [`MAKS_PROJE`] sınırına kırpılır (sabitler **korunur**).
    pub fn acildi(&mut self, yol: impl Into<PathBuf>, ad: impl Into<String>, now: Timestamp) {
        let yol = yol.into();
        if let Some(p) = self.projeler.iter_mut().find(|p| p.yol == yol) {
            p.son_acilma = now;
            p.ad = ad.into();
        } else {
            self.projeler.push(SonProje {
                ad: ad.into(),
                yol,
                son_acilma: now,
                sabit: false,
                ozet: None,
            });
        }
        self.kirp();
    }

    /// Bir projenin önizleme/özet metnini ayarlar (manifest okununca).
    pub fn ozet_ayarla(&mut self, yol: &Path, ozet: Option<String>) {
        if let Some(p) = self.projeler.iter_mut().find(|p| p.yol == yol) {
            p.ozet = ozet;
        }
    }

    /// Sabitleme durumunu tersine çevirir (pin ↔ unpin).  Döndürür: yeni sabit durumu (bulunursa).
    pub fn sabit_degistir(&mut self, yol: &Path) -> Option<bool> {
        let p = self.projeler.iter_mut().find(|p| p.yol == yol)?;
        p.sabit = !p.sabit;
        Some(p.sabit)
    }

    /// Bir projeyi listeden kaldırır (diskten **silmez**, yalnızca son-projeler listesinden).
    pub fn kaldir(&mut self, yol: &Path) -> bool {
        let onceki = self.projeler.len();
        self.projeler.retain(|p| p.yol != yol);
        self.projeler.len() != onceki
    }

    /// Taşınmış projeyi yeni konuma yeniden bağlar (TDA madde 19): eski yolu yenisiyle değiştirir.
    ///
    /// Yeni yol zaten listedeyse çift kayıt oluşmaz (eski kayıt kaldırılır).  Damga korunur.
    pub fn yeniden_bagla(&mut self, eski: &Path, yeni: impl Into<PathBuf>) -> bool {
        let yeni = yeni.into();
        if !self.projeler.iter().any(|p| p.yol == eski) {
            return false;
        }
        // Hedef yolu tutan farklı bir kayıt varsa kaldır (çift kayıt önle); `eski` kaydı korunur.
        self.projeler.retain(|p| p.yol == eski || p.yol != yeni);
        if let Some(p) = self.projeler.iter_mut().find(|p| p.yol == eski) {
            p.yol = yeni;
            true
        } else {
            false
        }
    }

    /// Görüntüleme sırası: önce sabitler, sonra en son açılan en üstte (madde: pin + tarih).
    pub fn sirali(&self) -> Vec<&SonProje> {
        let mut v: Vec<&SonProje> = self.projeler.iter().collect();
        v.sort_by(|a, b| {
            b.sabit
                .cmp(&a.sabit)
                .then(b.son_acilma.cmp(&a.son_acilma))
                .then(a.ad.to_lowercase().cmp(&b.ad.to_lowercase()))
        });
        v
    }

    /// Arama: ad veya yolda (büyük/küçük harf duyarsız) `sorgu` geçen projeler, sıralı.
    ///
    /// Boş sorgu = tüm liste (sıralı).
    pub fn ara(&self, sorgu: &str) -> Vec<&SonProje> {
        let q = sorgu.trim().to_lowercase();
        self.sirali()
            .into_iter()
            .filter(|p| {
                q.is_empty()
                    || p.ad.to_lowercase().contains(&q)
                    || p.yol.to_string_lossy().to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Liste [`MAKS_PROJE`]'yi aşarsa en eski **sabit olmayan** kayıtları çıkarır (sabitler korunur).
    fn kirp(&mut self) {
        if self.projeler.len() <= MAKS_PROJE {
            return;
        }
        // Sabit olmayanları en eskiden yeniye sırala; sınırı aşan kadarını sil.
        loop {
            if self.projeler.len() <= MAKS_PROJE {
                break;
            }
            // En eski sabit-olmayanı bul.
            let Some((idx, _)) = self
                .projeler
                .iter()
                .enumerate()
                .filter(|(_, p)| !p.sabit)
                .min_by(|(_, a), (_, b)| a.son_acilma.cmp(&b.son_acilma))
            else {
                break; // hepsi sabit → daha fazla kırpma yapma.
            };
            self.projeler.remove(idx);
        }
    }

    /// JSON serileştirme (host bunu atomik + bütünlüklü depoya yazar).
    pub fn serde_yaz(&self) -> Result<Vec<u8>, ErrorReport> {
        serde_json::to_vec_pretty(self).map_err(|e| {
            ErrorReport::new(
                "Son projeler kaydedilemedi",
                format!("Liste JSON'a çevrilemedi: {e}"),
                "Sorun sürerse uygulamayı yeniden başlatın; liste bir sonraki açılışta yeniden kurulur.",
            )
            .with_teknik_detay(format!("{e:?}"))
        })
    }

    /// JSON çözme.  Bozuk/eksik veri okunamazsa standart hata döner (host varsayılana düşer).
    pub fn serde_oku(baytlar: &[u8]) -> Result<Self, ErrorReport> {
        serde_json::from_slice(baytlar).map_err(|e| {
            ErrorReport::new(
                "Son projeler listesi okunamadı",
                format!("Kayıt çözülemedi: {e}"),
                "Endişelenmeyin: boş listeyle açılır; projeleri yeniden açtıkça liste dolar.",
            )
            .with_teknik_detay(format!("{e:?}"))
        })
    }
}

// ─── Testler ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn an(saniye: i64) -> Timestamp {
        // Sabit bir referans an + ofset (deterministik sıralama testleri için).
        Utc::now() + Duration::seconds(saniye)
    }

    #[test]
    fn bos_liste_rehber_durumu() {
        let l = SonProjelerListesi::yeni();
        assert!(l.bos_mu());
        assert_eq!(l.sayi(), 0);
        assert!(l.sirali().is_empty());
    }

    #[test]
    fn acilan_proje_eklenir_ve_one_tasinir() {
        let mut l = SonProjelerListesi::yeni();
        l.acildi("/p/a", "A", an(0));
        l.acildi("/p/b", "B", an(10));
        // En son açılan (B) en üstte.
        assert_eq!(l.sirali()[0].ad, "B");
        // A'yı tekrar aç → öne geçer.
        l.acildi("/p/a", "A", an(20));
        assert_eq!(l.sirali()[0].ad, "A");
        assert_eq!(l.sayi(), 2, "tekrar açmak yeni kayıt eklemez");
    }

    #[test]
    fn sabit_proje_basta_ve_korunur() {
        let mut l = SonProjelerListesi::yeni();
        l.acildi("/p/eski", "Eski", an(0));
        l.acildi("/p/yeni", "Yeni", an(100));
        // Eski'yi sabitle → Yeni daha güncel olsa da Eski başta.
        assert_eq!(l.sabit_degistir(Path::new("/p/eski")), Some(true));
        assert_eq!(l.sirali()[0].ad, "Eski");
        // Tekrar tıkla → sabit kalkar.
        assert_eq!(l.sabit_degistir(Path::new("/p/eski")), Some(false));
        assert_eq!(l.sirali()[0].ad, "Yeni");
    }

    #[test]
    fn kaldirma_calisir() {
        let mut l = SonProjelerListesi::yeni();
        l.acildi("/p/a", "A", an(0));
        assert!(l.kaldir(Path::new("/p/a")));
        assert!(l.bos_mu());
        assert!(!l.kaldir(Path::new("/p/a")), "olmayanı kaldırmak false");
    }

    #[test]
    fn arama_ad_ve_yolda_calisir() {
        let mut l = SonProjelerListesi::yeni();
        l.acildi("/genom/insan", "İnsan Genomu", an(0));
        l.acildi("/proteomik/test", "Protein", an(10));
        assert_eq!(l.ara("genom").len(), 1);
        assert_eq!(l.ara("genom")[0].ad, "İnsan Genomu");
        assert_eq!(l.ara("test").len(), 1, "yolda da arar");
        assert_eq!(l.ara("").len(), 2, "boş sorgu = tümü");
        assert_eq!(l.ara("PROTEIN").len(), 1, "büyük/küçük harf duyarsız");
    }

    #[test]
    fn tasinmis_proje_tespit_ve_yeniden_bagla() {
        let mut l = SonProjelerListesi::yeni();
        l.acildi("/eski/yol", "Taşınan", an(0));
        // Sahte varlık kontrolü: yalnızca /yeni/yol mevcut.
        let mevcut = |p: &Path| p == Path::new("/yeni/yol");
        assert_eq!(
            proje_durumu(Path::new("/eski/yol"), mevcut),
            ProjeDurumu::Bulunamadi
        );
        // Yeniden bağla → yol güncellenir, artık mevcut.
        assert!(l.yeniden_bagla(Path::new("/eski/yol"), "/yeni/yol"));
        assert_eq!(l.sirali()[0].yol, PathBuf::from("/yeni/yol"));
        assert_eq!(
            proje_durumu(l.sirali()[0].yol.as_path(), mevcut),
            ProjeDurumu::Mevcut
        );
    }

    #[test]
    fn maks_sinir_sabitleri_korur() {
        let mut l = SonProjelerListesi::yeni();
        // Bir sabit + MAKS_PROJE kadar normal ekle → sabit kalmalı, en eski normal düşmeli.
        l.acildi("/sabit", "Sabit", an(0));
        l.sabit_degistir(Path::new("/sabit"));
        for i in 0..MAKS_PROJE as i64 {
            l.acildi(format!("/n/{i}"), format!("N{i}"), an(100 + i));
        }
        assert!(
            l.projeler
                .iter()
                .any(|p| p.yol.as_path() == Path::new("/sabit")),
            "sabit proje kırpmada korunmalı"
        );
        assert!(l.sayi() <= MAKS_PROJE + 1, "sabit hariç MAKS sınırı");
    }

    #[test]
    fn serde_gidis_donus() {
        let mut l = SonProjelerListesi::yeni();
        l.acildi("/p/a", "A", an(0));
        l.sabit_degistir(Path::new("/p/a"));
        l.ozet_ayarla(Path::new("/p/a"), Some("12 örnek, 3 varyant".into()));
        let baytlar = l.serde_yaz().unwrap();
        let geri = SonProjelerListesi::serde_oku(&baytlar).unwrap();
        assert_eq!(geri.sayi(), 1);
        assert!(geri.projeler[0].sabit);
        assert_eq!(
            geri.projeler[0].ozet.as_deref(),
            Some("12 örnek, 3 varyant")
        );
    }

    #[test]
    fn bozuk_kayit_hata_doner() {
        assert!(SonProjelerListesi::serde_oku(b"{ bozuk json").is_err());
    }
}
