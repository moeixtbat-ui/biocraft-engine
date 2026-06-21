//! Çakışma tespiti — aynı dosya/proje iki yerde değişirse (İP-11 madde 18, MK-36/MK-37).
//!
//! **Sessiz ezme YOK.**  Bir dosyayı düzenlerken, onu *yüklediğimiz* andaki sürümünü (içerik
//! BLAKE3 özeti + zaman damgası) "taban sürüm" olarak hatırlarız.  Diske **yazmadan önce** diskteki
//! güncel sürümü tekrar damgalayıp tabanla karşılaştırırız:
//! - Disk hâlâ tabanla aynıysa → güvenle yazılır (kimse araya girmemiş).
//! - Disk **değişmişse** (başka bir pencere/araç/süreç yazmış) → **çakışma**: kullanıcıya iki sürüm
//!   sunulur ve hangisinin korunacağı sorulur.  Hiçbir değişiklik sessizce ezilmez.
//!
//! Bu modül **saf**tır: gerçek dosya okuma/yazma yapmaz, yalnızca sürüm damgalarını karşılaştırır.
//! Çağıran (uygulama) diskten okuyup [`damgala`] ile damga üretir; karşılaştırmayı buraya bırakır.
//! Böylece mantık tamamen test edilebilir kalır.  ([`crate::store`] zaten BLAKE3 bütünlük zarfı
//! tutar; burada amaç bütünlük değil **eşzamanlı değişiklik** tespitidir.)

use std::collections::BTreeMap;

use biocraft_types::{Blake3Hash, Timestamp};

/// Bir dosya/proje içeriğinin belirli bir andaki sürüm damgası (içerik özeti + görülme zamanı).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurumDamgasi {
    /// İçeriğin BLAKE3 özeti (eşzamanlı değişikliği belirleyen asıl ölçüt).
    pub hash: Blake3Hash,
    /// Bu sürümün görüldüğü/yazıldığı an (kullanıcıya "ne zaman değişmiş" göstermek için).
    pub zaman: Timestamp,
}

impl SurumDamgasi {
    /// İçerik baytlarından ve bir zaman damgasından sürüm damgası üretir.
    pub fn yeni(icerik: &[u8], zaman: Timestamp) -> Self {
        Self {
            hash: damgala(icerik),
            zaman,
        }
    }

    /// İki damga **aynı içeriği** mi gösteriyor? (zaman farkı önemli değil; ölçüt hash'tir).
    pub fn ayni_icerik(&self, diger: &SurumDamgasi) -> bool {
        self.hash == diger.hash
    }
}

/// İçerik baytlarının BLAKE3 özetini [`Blake3Hash`] olarak hesaplar.
pub fn damgala(icerik: &[u8]) -> Blake3Hash {
    Blake3Hash(*blake3::hash(icerik).as_bytes())
}

/// Diske yazmadan önce verilen karar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CakismaKarari {
    /// Disk, düzenlemeye başladığımız taban sürümle aynı → güvenle yazılabilir.
    GuvenliYaz,
    /// Disk, biz düzenlerken değişmiş → çakışma; kullanıcı sürüm seçmeli (sessiz ezme yok).
    Cakisma(CakismaBilgisi),
}

impl CakismaKarari {
    /// Bu bir çakışma mı? (UI'da uyarı gösterilip gösterilmeyeceği).
    pub fn cakisma_mi(&self) -> bool {
        matches!(self, CakismaKarari::Cakisma(_))
    }
}

/// Bir çakışmanın ayrıntıları: hangi dosya, hangi iki sürüm çatışıyor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CakismaBilgisi {
    /// Çakışan dosyanın/projenin yolu (kullanıcıya gösterilir).
    pub yol: String,
    /// Bizim düzenlememizin dayandığı taban sürüm (yüklediğimiz an).
    pub taban: SurumDamgasi,
    /// Diskteki güncel sürüm (başka bir yer tarafından değiştirilmiş).
    pub diskteki: SurumDamgasi,
}

/// Kullanıcının çakışmayı çözme seçimi (sessiz ezme yok — bilinçli seçim).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CozumSecimi {
    /// Bizim sürümümüzü yaz (diskteki değişikliğin üzerine — kullanıcı onayıyla).
    BizimkiniYaz,
    /// Diskteki sürümü koru (bizim değişikliğimizi at, diskten yeniden yükle).
    DiskiKoru,
    /// Şimdilik karar verme (yazma iptal; kullanıcı sonra birleştirebilir).  Birleştirme: sonra.
    Iptal,
}

/// Düzenlenen dosyaların **taban sürümlerini** izleyen ve yazma öncesi çakışma kontrolü yapan kayıt.
///
/// Kullanım: dosya yüklenince [`taban_kaydet`](Self::taban_kaydet); yazmadan önce
/// [`yazmadan_once`](Self::yazmadan_once); yazma/çözümden sonra tabanı yeni sürüme güncelle
/// (yine [`taban_kaydet`](Self::taban_kaydet)).
#[derive(Debug, Default)]
pub struct CakismaIzleyici {
    taban: BTreeMap<String, SurumDamgasi>,
}

impl CakismaIzleyici {
    /// Boş bir izleyici oluşturur.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir dosyanın taban sürümünü kaydeder/günceller (yükleme veya başarılı yazmadan sonra).
    pub fn taban_kaydet(&mut self, yol: impl Into<String>, damga: SurumDamgasi) {
        self.taban.insert(yol.into(), damga);
    }

    /// Bir dosyanın bilinen taban sürümü (varsa).
    pub fn taban(&self, yol: &str) -> Option<&SurumDamgasi> {
        self.taban.get(yol)
    }

    /// İzlemeyi bırak (dosya kapatıldığında).
    pub fn unut(&mut self, yol: &str) {
        self.taban.remove(yol);
    }

    /// Diske yazmadan önce çakışma kontrolü: taban sürüm ile **diskteki güncel** sürümü karşılaştırır.
    ///
    /// - Taban bilinmiyorsa (ilk kez yazılıyor) → [`CakismaKarari::GuvenliYaz`].
    /// - Disk taban ile aynı içerikteyse → [`CakismaKarari::GuvenliYaz`].
    /// - Disk değişmişse → [`CakismaKarari::Cakisma`] (kullanıcı sürüm seçmeli).
    ///
    /// `diskteki`: çağıranın **şu an** diskten okuyup [`SurumDamgasi::yeni`] ile ürettiği güncel damga.
    pub fn yazmadan_once(&self, yol: &str, diskteki: &SurumDamgasi) -> CakismaKarari {
        match self.taban.get(yol) {
            None => CakismaKarari::GuvenliYaz, // ilk yazım; karşılaştırılacak taban yok.
            Some(taban) if taban.ayni_icerik(diskteki) => CakismaKarari::GuvenliYaz,
            Some(taban) => CakismaKarari::Cakisma(CakismaBilgisi {
                yol: yol.to_string(),
                taban: taban.clone(),
                diskteki: diskteki.clone(),
            }),
        }
    }
}

// ─── Testler ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use biocraft_types::Timestamp;

    /// Sabit zaman damgası (saniyeden) — testlerde belirlilik.
    fn t(saniye: i64) -> Timestamp {
        chrono::DateTime::from_timestamp(saniye, 0).expect("geçerli zaman")
    }

    #[test]
    fn damgala_belirli_ve_iceriğe_duyarli() {
        assert_eq!(damgala(b"abc"), damgala(b"abc"), "aynı içerik aynı hash");
        assert_ne!(
            damgala(b"abc"),
            damgala(b"abd"),
            "farklı içerik farklı hash"
        );
    }

    #[test]
    fn taban_yokken_guvenli_yaz() {
        let izleyici = CakismaIzleyici::yeni();
        let diskteki = SurumDamgasi::yeni(b"yeni dosya", t(100));
        assert_eq!(
            izleyici.yazmadan_once("yeni.txt", &diskteki),
            CakismaKarari::GuvenliYaz,
            "ilk yazımda taban yok → güvenli"
        );
    }

    #[test]
    fn disk_degismediyse_guvenli_yaz() {
        let mut izleyici = CakismaIzleyici::yeni();
        let icerik = b"v1 icerik";
        izleyici.taban_kaydet("a.txt", SurumDamgasi::yeni(icerik, t(100)));
        // Disk hâlâ aynı içerik (zaman farklı olsa bile ölçüt hash).
        let diskteki = SurumDamgasi::yeni(icerik, t(200));
        assert_eq!(
            izleyici.yazmadan_once("a.txt", &diskteki),
            CakismaKarari::GuvenliYaz,
            "disk içeriği değişmemiş → güvenli yaz"
        );
    }

    #[test]
    fn ayni_dosya_iki_yerde_degisince_cakisma() {
        // Kabul kriteri: aynı dosya iki yerde değişirse tespit + uyarı (sessiz ezme YOK).
        let mut izleyici = CakismaIzleyici::yeni();
        izleyici.taban_kaydet("a.txt", SurumDamgasi::yeni(b"v1", t(100)));
        // Biz v1 üzerinde düzenlerken başka bir yer diske v2 yazmış.
        let diskteki = SurumDamgasi::yeni(b"v2 baska yerden", t(150));
        let karar = izleyici.yazmadan_once("a.txt", &diskteki);
        assert!(karar.cakisma_mi(), "disk değişmiş → çakışma sunulmalı");
        match karar {
            CakismaKarari::Cakisma(bilgi) => {
                assert_eq!(bilgi.yol, "a.txt");
                assert_eq!(bilgi.taban.hash, damgala(b"v1"));
                assert_eq!(bilgi.diskteki.hash, damgala(b"v2 baska yerden"));
                assert_ne!(
                    bilgi.taban.hash, bilgi.diskteki.hash,
                    "iki sürüm gerçekten farklı"
                );
            }
            _ => panic!("çakışma bekleniyordu"),
        }
    }

    #[test]
    fn cozumden_sonra_taban_guncellenince_cakisma_biter() {
        let mut izleyici = CakismaIzleyici::yeni();
        izleyici.taban_kaydet("a.txt", SurumDamgasi::yeni(b"v1", t(100)));
        let diskteki_v2 = SurumDamgasi::yeni(b"v2", t(150));
        assert!(izleyici.yazmadan_once("a.txt", &diskteki_v2).cakisma_mi());
        // Kullanıcı "bizimkini yaz" dedi → yeni taban v2 (diskle aynı) olarak güncellenir.
        izleyici.taban_kaydet("a.txt", SurumDamgasi::yeni(b"v2", t(160)));
        assert_eq!(
            izleyici.yazmadan_once("a.txt", &diskteki_v2),
            CakismaKarari::GuvenliYaz,
            "taban güncellendi → artık çakışma yok"
        );
    }

    #[test]
    fn unut_izlemeyi_birakir() {
        let mut izleyici = CakismaIzleyici::yeni();
        izleyici.taban_kaydet("a.txt", SurumDamgasi::yeni(b"v1", t(100)));
        assert!(izleyici.taban("a.txt").is_some());
        izleyici.unut("a.txt");
        assert!(izleyici.taban("a.txt").is_none());
    }

    #[test]
    fn cozum_secimleri_ayri() {
        assert_ne!(CozumSecimi::BizimkiniYaz, CozumSecimi::DiskiKoru);
        assert_ne!(CozumSecimi::DiskiKoru, CozumSecimi::Iptal);
    }
}
