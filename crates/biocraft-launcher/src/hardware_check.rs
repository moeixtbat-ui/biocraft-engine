//! Donanım ön-kontrolü + yetenek matrisi — İP-01, MK-05.
//!
//! İlk açılışta sistem donanımı tespit edilir; **referans donanımın altındaysa** kullanıcı
//! *dışlanmaz*, **bilgilendirilir**: "ne yapılabilir / ne sınırlı / ne yapılamaz" şeffaf bir
//! yetenek matrisiyle gösterilir (MK-05).
//!
//! Donanım profili **yeniden kullanılır**: İP-08'de kurulan [`DonanimProfili`] (CPU çekirdek /
//! RAM / GPU) + [`DonanimSinifi`] burada tekrar üretilmez, doğrudan kullanılır (kod tekrarı yok).
//! Karar mantığı saftır → sahte profille test edilir; gerçek tespit host'ta (`sysinfo`).

// İP-08 donanım profili biocraft-mem'de; biocraft-ui onu yeniden dışa aktarır (L4→L2 zaten var).
use biocraft_ui::biocraft_mem::autotune::{DonanimProfili, DonanimSinifi};

const GB: u64 = 1024 * 1024 * 1024;

/// BioCraft Engine'in **referans (önerilen)** donanım tabanı.  Bunun altı = uyarı + sadeleşme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferansDonanim {
    /// Önerilen asgari mantıksal çekirdek sayısı.
    pub cekirdek: usize,
    /// Önerilen asgari RAM (bayt).
    pub ram_bayt: u64,
    /// Ayrık/yetenekli GPU öneriliyor mu?
    pub gpu_onerilir: bool,
}

impl Default for ReferansDonanim {
    /// Spec referansı: 4 çekirdek, 8 GB RAM, GPU önerilir.
    fn default() -> Self {
        Self {
            cekirdek: 4,
            ram_bayt: 8 * GB,
            gpu_onerilir: true,
        }
    }
}

/// Bir yeteneğin bu donanımda kullanılabilirlik durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YetenekDurumu {
    /// Tam çalışır.
    Tam,
    /// Sınırlı/sadeleşmiş çalışır (örn. düşük FPS, küçük veri).
    Sinirli,
    /// Bu donanımda kullanılamaz (alternatif önerilir).
    Yok,
}

impl YetenekDurumu {
    /// Görsel ikon (yeşil/sarı/kırmızı yerine sembolik; renk view'da token'dan).
    pub fn ikon(&self) -> &'static str {
        match self {
            YetenekDurumu::Tam => "✓",
            YetenekDurumu::Sinirli => "~",
            YetenekDurumu::Yok => "✕",
        }
    }

    /// Kısa, yerelleştirilmiş etiket.
    pub fn etiket(&self, tr: bool) -> &'static str {
        match (self, tr) {
            (YetenekDurumu::Tam, true) => "Tam",
            (YetenekDurumu::Tam, false) => "Full",
            (YetenekDurumu::Sinirli, true) => "Sınırlı",
            (YetenekDurumu::Sinirli, false) => "Limited",
            (YetenekDurumu::Yok, true) => "Yok",
            (YetenekDurumu::Yok, false) => "No",
        }
    }
}

/// Yetenek matrisindeki tek bir satır (ne yapılabilir + durum + açıklama).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yetenek {
    /// Yeteneğin adı (Türkçe/İngilizce çift; basitlik için view dile göre seçer).
    pub ad_tr: &'static str,
    /// Yeteneğin adı (İngilizce).
    pub ad_en: &'static str,
    /// Bu donanımdaki durumu.
    pub durum: YetenekDurumu,
    /// Kısa açıklama (Türkçe).
    pub aciklama_tr: String,
    /// Kısa açıklama (İngilizce).
    pub aciklama_en: String,
}

impl Yetenek {
    /// Dile göre ad.
    pub fn ad(&self, tr: bool) -> &str {
        if tr {
            self.ad_tr
        } else {
            self.ad_en
        }
    }

    /// Dile göre açıklama.
    pub fn aciklama(&self, tr: bool) -> &str {
        if tr {
            &self.aciklama_tr
        } else {
            &self.aciklama_en
        }
    }
}

/// Donanım değerlendirmesinin sonucu (view bunu kart olarak gösterir).
#[derive(Debug, Clone)]
pub struct DonanimDegerlendirme {
    /// Tespit edilen profil.
    pub profil: DonanimProfili,
    /// Türetilen güç sınıfı.
    pub sinif: DonanimSinifi,
    /// Referans donanımın altında mı (uyarı gösterilir)?
    pub referans_alti: bool,
    /// Yetenek matrisi (ne yapılabilir/yapılamaz).
    pub matris: Vec<Yetenek>,
}

impl DonanimDegerlendirme {
    /// Profili referansa göre değerlendirir + yetenek matrisini üretir (saf).
    pub fn degerlendir(profil: DonanimProfili, referans: ReferansDonanim) -> Self {
        let referans_alti = profil.cekirdek < referans.cekirdek
            || profil.ram_bayt < referans.ram_bayt
            || (referans.gpu_onerilir && !profil.gpu_var);
        let sinif = profil.sinif();
        let matris = matris_uret(profil, sinif);
        Self {
            profil,
            sinif,
            referans_alti,
            matris,
        }
    }

    /// Kısa özet metni (RAM/çekirdek/GPU) — durum kartı başlığı için.
    pub fn ozet(&self, tr: bool) -> String {
        let gb = self.profil.ram_bayt as f64 / GB as f64;
        let gpu = if self.profil.gpu_var {
            if tr {
                "GPU var"
            } else {
                "GPU yes"
            }
        } else if tr {
            "GPU yok"
        } else {
            "no GPU"
        };
        format!(
            "{} {} · {:.0} GB RAM · {} {} · {}",
            self.profil.cekirdek,
            if tr { "çekirdek" } else { "cores" },
            gb,
            if tr { "sınıf" } else { "class" },
            self.sinif.ad(),
            gpu,
        )
    }
}

/// Sınıfa/profile göre yetenek matrisini kurar.
fn matris_uret(profil: DonanimProfili, sinif: DonanimSinifi) -> Vec<Yetenek> {
    // 3B görselleştirme GPU'ya ve sınıfa bağlı.
    let uc_boyut = match (profil.gpu_var, sinif) {
        (true, DonanimSinifi::Yuksek) | (true, DonanimSinifi::Orta) => YetenekDurumu::Tam,
        (true, DonanimSinifi::Dusuk) => YetenekDurumu::Sinirli,
        (false, _) => YetenekDurumu::Sinirli, // CPU fallback (WARP) ile sınırlı çalışır.
    };
    // GPU hızlandırma doğrudan GPU varlığına bağlı.
    let gpu_hiz = if profil.gpu_var {
        YetenekDurumu::Tam
    } else {
        YetenekDurumu::Yok
    };
    // Çok büyük veri (out-of-core akış) HER donanımda çalışır (MK-09) — sadece hız değişir.
    let buyuk_veri = match sinif {
        DonanimSinifi::Dusuk => YetenekDurumu::Sinirli,
        _ => YetenekDurumu::Tam,
    };

    vec![
        Yetenek {
            ad_tr: "Genom tarayıcı & 2B grafikler",
            ad_en: "Genome browser & 2D plots",
            durum: YetenekDurumu::Tam, // egui/CPU; her yerde çalışır.
            aciklama_tr: "Tüm donanımlarda akıcı çalışır.".into(),
            aciklama_en: "Runs smoothly on all hardware.".into(),
        },
        Yetenek {
            ad_tr: "3B genom/molekül görselleştirme",
            ad_en: "3D genome/molecule view",
            durum: uc_boyut,
            aciklama_tr: match uc_boyut {
                YetenekDurumu::Tam => "GPU hızlandırmalı, tam kalite.".into(),
                YetenekDurumu::Sinirli => {
                    "GPU yok/zayıf → yazılım (CPU) ile sade ve daha yavaş çalışır.".into()
                }
                YetenekDurumu::Yok => "Kullanılamaz.".into(),
            },
            aciklama_en: match uc_boyut {
                YetenekDurumu::Tam => "GPU-accelerated, full quality.".into(),
                YetenekDurumu::Sinirli => {
                    "No/weak GPU → runs simplified & slower on CPU (software).".into()
                }
                YetenekDurumu::Yok => "Unavailable.".into(),
            },
        },
        Yetenek {
            ad_tr: "GPU hızlandırma (büyük hesap)",
            ad_en: "GPU acceleration (heavy compute)",
            durum: gpu_hiz,
            aciklama_tr: if profil.gpu_var {
                "Ayrık/entegre GPU bulundu.".into()
            } else {
                "GPU bulunamadı; hesaplar CPU'da yapılır (daha yavaş).".into()
            },
            aciklama_en: if profil.gpu_var {
                "Discrete/integrated GPU found.".into()
            } else {
                "No GPU found; compute runs on CPU (slower).".into()
            },
        },
        Yetenek {
            ad_tr: "Çok büyük dosyalar (out-of-core)",
            ad_en: "Very large files (out-of-core)",
            durum: buyuk_veri,
            aciklama_tr: match buyuk_veri {
                YetenekDurumu::Tam => "Akış/mmap ile RAM'den büyük veri işlenir.".into(),
                _ => "Düşük RAM → daha küçük pencerelerle, daha yavaş işlenir.".into(),
            },
            aciklama_en: match buyuk_veri {
                YetenekDurumu::Tam => "Streams/mmaps data larger than RAM.".into(),
                _ => "Low RAM → smaller windows, slower processing.".into(),
            },
        },
    ]
}

// ─── Testler ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn profil(cekirdek: usize, ram_gb: u64, gpu: bool) -> DonanimProfili {
        DonanimProfili {
            cekirdek,
            ram_bayt: ram_gb * GB,
            gpu_var: gpu,
        }
    }

    #[test]
    fn guclu_donanim_referans_ustu() {
        let d = DonanimDegerlendirme::degerlendir(profil(16, 32, true), ReferansDonanim::default());
        assert!(!d.referans_alti);
        assert_eq!(d.sinif, DonanimSinifi::Yuksek);
        // 3B tam çalışır.
        let uc = &d.matris[1];
        assert_eq!(uc.durum, YetenekDurumu::Tam);
    }

    #[test]
    fn zayif_donanim_referans_alti_uyari() {
        // İP-08 --emulate-min profili (2 çekirdek, 2 GB, GPU yok).
        let d = DonanimDegerlendirme::degerlendir(
            DonanimProfili::asgari_emulasyon(),
            ReferansDonanim::default(),
        );
        assert!(d.referans_alti, "zayıf makine referans altı → uyarı");
        assert_eq!(d.sinif, DonanimSinifi::Dusuk);
        // GPU hızlandırma yok, ama 2B/genom hâlâ tam (kullanıcı DIŞLANMAZ — MK-05).
        assert_eq!(d.matris[0].durum, YetenekDurumu::Tam);
        assert_eq!(d.matris[2].durum, YetenekDurumu::Yok); // GPU hızlandırma
    }

    #[test]
    fn gpu_yok_ucboyut_sinirli_calisir() {
        // GPU yok ama yeterli CPU/RAM → 3B yazılım (CPU) ile SINIRLI, "Yok" değil.
        let d = DonanimDegerlendirme::degerlendir(profil(8, 16, false), ReferansDonanim::default());
        assert!(d.referans_alti, "GPU önerilir ama yok → referans altı");
        assert_eq!(d.matris[1].durum, YetenekDurumu::Sinirli);
    }

    #[test]
    fn buyuk_veri_her_donanimda_calisir() {
        // Out-of-core (MK-09): zayıf makinede bile en az SINIRLI; asla Yok değil.
        let zayif =
            DonanimDegerlendirme::degerlendir(profil(2, 2, false), ReferansDonanim::default());
        assert_ne!(zayif.matris[3].durum, YetenekDurumu::Yok);
    }

    #[test]
    fn ozet_metni_makul() {
        let d = DonanimDegerlendirme::degerlendir(profil(8, 16, true), ReferansDonanim::default());
        let s = d.ozet(true);
        assert!(s.contains("çekirdek"));
        assert!(s.contains("GB RAM"));
    }
}
