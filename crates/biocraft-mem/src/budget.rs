//! Dosya-öncesi bütçe kontrolü — MK-22, MK-09.
//!
//! Bir dosya **açılmadan ÖNCE** boyutu + tahmini RAM ihtiyacı hesaplanır.  Sığıyorsa
//! normal açılır; sığmıyorsa kullanıcıya **"akış (stream) / cloud-burst (yer tutucu) /
//! iptal"** seçeneklerini içeren bir teklif döner.  Bu teklif yalnızca **veridir**;
//! görsel diyaloğu üst katman (UI) çizer — `biocraft-mem` (L2) egui'ye bağlanamaz (MK-40).
//!
//! Böylece 4 TB'lık bir dosyada bile "hepsini RAM'e yükle" yolu **hiç önerilmez** (MK-09).

use crate::birim::insan_bayt;
use crate::orchestrator::BellekOrkestratoru;

/// Bir dosyanın açılmadan önce tahmini RAM ihtiyacı.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DosyaTahmini {
    /// Dosyanın diskteki boyutu (bayt).
    pub dosya_bayt: u64,
    /// Ayrıştırılmış/bellek-içi temsil için çarpan (örn. metin→nesne ~3×).
    pub genisleme_kat: f64,
    /// Tahmini bellek-içi boyut (bayt) = dosya_bayt × genisleme_kat.
    pub tahmini_ram_bayt: u64,
}

impl DosyaTahmini {
    /// Dosya boyutu ve genişleme katından tahmin üretir.
    /// `genisleme_kat <= 0` ise 1.0 (kopya kadar) varsayılır.
    pub fn hesapla(dosya_bayt: u64, genisleme_kat: f64) -> Self {
        let kat = if genisleme_kat <= 0.0 {
            1.0
        } else {
            genisleme_kat
        };
        let ram = (dosya_bayt as f64 * kat).round() as u64;
        Self {
            dosya_bayt,
            genisleme_kat: kat,
            tahmini_ram_bayt: ram,
        }
    }
}

/// Kullanıcının bütçe diyaloğunda seçebileceği açma yolu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcmaSecenegi {
    /// Akış (stream) modunda aç — tüm dosya RAM'e alınmaz (MK-09, önerilen).
    AkisModu,
    /// Cloud-burst — ağır işi buluta taşı (MVP'de yer tutucu; `MVP-sonrasi.md` §7.3).
    CloudBurst,
    /// Vazgeç.
    Iptal,
}

impl AcmaSecenegi {
    /// Diyalog butonunda görünecek Türkçe etiket.
    pub fn etiket(&self) -> &'static str {
        match self {
            AcmaSecenegi::AkisModu => "Akış modunda aç",
            AcmaSecenegi::CloudBurst => "Bulutta işle (yakında)",
            AcmaSecenegi::Iptal => "İptal",
        }
    }

    /// Bu seçenek şu an gerçekten çalışıyor mu?  `CloudBurst` MVP'de yer tutucudur.
    pub fn etkin(&self) -> bool {
        !matches!(self, AcmaSecenegi::CloudBurst)
    }
}

/// Dosya bütçeye sığmadığında üretilen teklif (UI bunu diyalog olarak çizer).
#[derive(Debug, Clone, PartialEq)]
pub struct AkisTeklifi {
    /// Dosya boyutu (bayt).
    pub dosya_bayt: u64,
    /// Tahmini bellek-içi boyut (bayt).
    pub tahmini_ram_bayt: u64,
    /// Şu an boştaki bellek (bayt).
    pub bos_bayt: u64,
    /// Sunulan seçenekler (sırayla).
    pub secenekler: Vec<AcmaSecenegi>,
    /// Kullanıcıya gösterilecek sade açıklama.
    pub ozet: String,
}

/// Dosya-öncesi bütçe kontrolünün sonucu.
#[derive(Debug, Clone, PartialEq)]
pub enum ButceKarari {
    /// Tahmini RAM boştaki bütçeye sığıyor → normal aç.
    Sigar {
        /// Tahmini bellek-içi boyut (bayt).
        tahmini_ram_bayt: u64,
    },
    /// Sığmıyor → akış/cloud-burst/iptal diyaloğu göster.
    AkisOnerilir(AkisTeklifi),
}

impl ButceKarari {
    /// Sığıyor mu? (kısa yol — test ve UI için).
    pub fn sigar_mi(&self) -> bool {
        matches!(self, ButceKarari::Sigar { .. })
    }
}

/// **Dosya açmadan önce bütçeyi kontrol et (MK-22).**
///
/// `dosya_bayt`: dosya boyutu; `genisleme_kat`: bellek-içi temsil çarpanı.
/// Boştaki bütçeye sığıyorsa [`ButceKarari::Sigar`], değilse akış teklifini döner.
pub fn dosya_butce_kontrol(
    dosya_bayt: u64,
    genisleme_kat: f64,
    ork: &BellekOrkestratoru,
) -> ButceKarari {
    let tahmin = DosyaTahmini::hesapla(dosya_bayt, genisleme_kat);
    let bos = ork.bos();

    if tahmin.tahmini_ram_bayt <= bos {
        ButceKarari::Sigar {
            tahmini_ram_bayt: tahmin.tahmini_ram_bayt,
        }
    } else {
        let ozet = format!(
            "Bu dosya diskte {} yer kaplıyor; tamamını açmak tahminen {} bellek ister, \
             ama şu an boş bellek yalnızca {}. Tümünü belleğe almak yerine akış (stream) \
             modunu öneriyoruz — dosya parça parça işlenir, çökme olmaz.",
            insan_bayt(dosya_bayt),
            insan_bayt(tahmin.tahmini_ram_bayt),
            insan_bayt(bos),
        );
        ButceKarari::AkisOnerilir(AkisTeklifi {
            dosya_bayt,
            tahmini_ram_bayt: tahmin.tahmini_ram_bayt,
            bos_bayt: bos,
            secenekler: vec![
                AcmaSecenegi::AkisModu,
                AcmaSecenegi::CloudBurst,
                AcmaSecenegi::Iptal,
            ],
            ozet,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::{BellekBileseni, BellekOrkestratoru};

    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * MB;

    #[test]
    fn tahmin_genisleme_kati_uygulanir() {
        let t = DosyaTahmini::hesapla(100 * MB, 3.0);
        assert_eq!(t.tahmini_ram_bayt, 300 * MB);
    }

    #[test]
    fn tahmin_sifir_kat_kopya_kadar_sayilir() {
        let t = DosyaTahmini::hesapla(100 * MB, 0.0);
        assert_eq!(t.tahmini_ram_bayt, 100 * MB);
    }

    #[test]
    fn kucuk_dosya_butceye_sigar() {
        let ork = BellekOrkestratoru::yeni(GB);
        let karar = dosya_butce_kontrol(10 * MB, 3.0, &ork);
        assert!(karar.sigar_mi());
        match karar {
            ButceKarari::Sigar { tahmini_ram_bayt } => assert_eq!(tahmini_ram_bayt, 30 * MB),
            _ => panic!("Sığar bekleniyordu"),
        }
    }

    #[test]
    fn buyuk_dosya_akis_diyalogu_uretir() {
        // MK-22/MK-09: büyük dosya → "load all" değil, stream/iptal teklifi.
        let ork = BellekOrkestratoru::yeni(GB);
        let karar = dosya_butce_kontrol(800 * MB, 3.0, &ork); // ~2.4 GB > 1 GB
        assert!(!karar.sigar_mi());
        match karar {
            ButceKarari::AkisOnerilir(teklif) => {
                assert_eq!(teklif.secenekler[0], AcmaSecenegi::AkisModu);
                assert!(teklif.secenekler.contains(&AcmaSecenegi::Iptal));
                assert!(teklif.tahmini_ram_bayt > teklif.bos_bayt);
                assert!(!teklif.ozet.is_empty());
            }
            _ => panic!("Akış teklifi bekleniyordu"),
        }
    }

    #[test]
    fn dort_tb_dosya_asla_sigmaz_akis_onerilir() {
        // MK-09: 4 TB dosyada bile "hepsini yükle" yok.
        let ork = BellekOrkestratoru::yeni(16 * GB);
        let dort_tb = 4 * 1024 * GB;
        let karar = dosya_butce_kontrol(dort_tb, 1.0, &ork);
        assert!(!karar.sigar_mi());
    }

    #[test]
    fn dolu_butcede_kucuk_dosya_bile_akis_onerir() {
        let ork = BellekOrkestratoru::yeni(100 * MB);
        let _h = ork.rezerve_et(BellekBileseni::Arayuz, 95 * MB).unwrap();
        // 10 MB × 3 = 30 MB ama boşta sadece 5 MB → akış önerilir.
        let karar = dosya_butce_kontrol(10 * MB, 3.0, &ork);
        assert!(!karar.sigar_mi());
    }

    #[test]
    fn cloud_burst_yer_tutucu_etkin_degil() {
        assert!(!AcmaSecenegi::CloudBurst.etkin());
        assert!(AcmaSecenegi::AkisModu.etkin());
    }
}
