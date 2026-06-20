//! Donanım profili + otomatik ayar (Eco/Bio) — İP-08, MK-26.
//!
//! Başlangıçta donanım profili (CPU çekirdek / RAM / GPU) çıkarılır ve ayarlar otomatik
//! uyarlanır: **düşük donanımda sadeleşme + uyarı**, çok düşükte **30 FPS hedefi** (spec).
//! `Eco` modu güç tasarrufu/sadeleştirme, `Bio` modu tam görsel kalitedir.
//!
//! `--emulate-min` (geliştirici bayrağı) düşük-donanım profilini **taklit eder** → sadeleşme
//! ve uyarı yolunu gerçek zayıf makine olmadan test etmeyi sağlar.
//!
//! Karar mantığı **saf**tır ([`OtoAyar::hesapla`]); gerçek profil [`profil_cikar`]'dadır (sysinfo).

use biocraft_types::ErrorReport;

const GB: u64 = 1024 * 1024 * 1024;

/// Çıkarılan donanım profili.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DonanimProfili {
    /// Mantıksal CPU çekirdek sayısı.
    pub cekirdek: usize,
    /// Toplam RAM (bayt).
    pub ram_bayt: u64,
    /// Ayrık/entegre bir GPU kullanılabiliyor mu?
    pub gpu_var: bool,
}

impl DonanimProfili {
    /// `--emulate-min`: zayıf bir makineyi taklit eder (2 çekirdek, 2 GB RAM, GPU yok).
    pub fn asgari_emulasyon() -> Self {
        Self {
            cekirdek: 2,
            ram_bayt: 2 * GB,
            gpu_var: false,
        }
    }

    /// Donanım sınıfını türetir.
    pub fn sinif(&self) -> DonanimSinifi {
        // Çok düşük: ≤2 çekirdek veya <4 GB RAM → sadeleşme + 30 FPS.
        if self.cekirdek <= 2 || self.ram_bayt < 4 * GB {
            DonanimSinifi::Dusuk
        } else if self.cekirdek >= 8 && self.ram_bayt >= 16 * GB && self.gpu_var {
            DonanimSinifi::Yuksek
        } else {
            DonanimSinifi::Orta
        }
    }
}

/// Donanım gücü sınıfı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DonanimSinifi {
    /// Zayıf makine → sadeleşme + 30 FPS hedefi + uyarı.
    Dusuk,
    /// Orta makine → tam özellik, 60 FPS.
    Orta,
    /// Güçlü makine → tam kalite (Bio), 60 FPS.
    Yuksek,
}

impl DonanimSinifi {
    /// Durum panelinde gösterilecek kısa ad.
    pub fn ad(&self) -> &'static str {
        match self {
            DonanimSinifi::Dusuk => "Düşük",
            DonanimSinifi::Orta => "Orta",
            DonanimSinifi::Yuksek => "Yüksek",
        }
    }
}

/// Performans/enerji modu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PerformansModu {
    /// Güç tasarrufu + görsel sadeleştirme (düşük donanım / pil).
    Eco,
    /// Tam görsel kalite (varsayılan, yeterli donanım).
    #[default]
    Bio,
}

impl PerformansModu {
    /// Durum panelinde gösterilecek kısa ad.
    pub fn ad(&self) -> &'static str {
        match self {
            PerformansModu::Eco => "Eco (tasarruf)",
            PerformansModu::Bio => "Bio (tam kalite)",
        }
    }
}

/// Donanıma göre otomatik ayar sonucu.
#[derive(Debug, Clone, PartialEq)]
pub struct OtoAyar {
    /// Donanım sınıfı.
    pub sinif: DonanimSinifi,
    /// Önerilen mod (Eco/Bio).
    pub mod_: PerformansModu,
    /// Hedef kare hızı (FPS): düşük donanımda 30, aksi halde 60.
    pub hedef_fps: u32,
    /// Görsel sadeleştirme uygulanmalı mı? (ağır efektler kapalı).
    pub sadelesme: bool,
    /// Önerilen arka plan worker sayısı (≥1).
    pub onerilen_worker: usize,
    /// Düşük donanım uyarısı (TDA) — yeterli donanımda `None`.
    pub uyari: Option<ErrorReport>,
}

impl OtoAyar {
    /// **Profilden otomatik ayar hesapla (MK-26).**
    pub fn hesapla(profil: &DonanimProfili) -> Self {
        let sinif = profil.sinif();
        // En az 1 worker; çekirdeklerin tamamını değil, arayüze pay bırakacak şekilde kullan.
        let onerilen_worker = profil.cekirdek.saturating_sub(1).max(1);

        match sinif {
            DonanimSinifi::Dusuk => {
                let uyari = ErrorReport::new(
                    "Düşük donanım algılandı — sadeleştirildi",
                    format!(
                        "Bu makinede {} çekirdek ve {} RAM görüldü. Akıcı kalmak için görsel \
                         efektler sadeleştirildi ve 30 FPS hedeflendi.",
                        profil.cekirdek,
                        crate::birim::insan_bayt(profil.ram_bayt),
                    ),
                    "Daha akıcı bir deneyim için ağır 3B sahneleri kapalı tutun; mümkünse RAM \
                     yükseltin. İsterseniz ayarlardan tam kaliteye (Bio) geçebilirsiniz.",
                )
                .with_eylem("Ayarları aç");
                Self {
                    sinif,
                    mod_: PerformansModu::Eco,
                    hedef_fps: 30,
                    sadelesme: true,
                    onerilen_worker,
                    uyari: Some(uyari),
                }
            }
            DonanimSinifi::Orta => Self {
                sinif,
                mod_: PerformansModu::Bio,
                hedef_fps: 60,
                sadelesme: false,
                onerilen_worker,
                uyari: None,
            },
            DonanimSinifi::Yuksek => Self {
                sinif,
                mod_: PerformansModu::Bio,
                hedef_fps: 60,
                sadelesme: false,
                onerilen_worker,
                uyari: None,
            },
        }
    }
}

/// **Gerçek donanım profilini çıkar (sysinfo).**  `gpu_var`: render katmanı bir GPU backend'i
/// seçebildiyse `true` (CPU fallback'te `false`).  Çekirdek sayısı std'den, RAM sysinfo'dan.
pub fn profil_cikar(gpu_var: bool) -> DonanimProfili {
    let cekirdek = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let ram_bayt = sys.total_memory();
    DonanimProfili {
        cekirdek,
        ram_bayt,
        gpu_var,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dusuk_donanim_sadelesir_ve_uyarir() {
        // MK-26 / --emulate-min: zayıf makine → sadeleşme + 30 FPS + uyarı.
        let p = DonanimProfili::asgari_emulasyon();
        let ayar = OtoAyar::hesapla(&p);
        assert_eq!(ayar.sinif, DonanimSinifi::Dusuk);
        assert!(ayar.sadelesme, "Düşük donanımda sadeleşmeli");
        assert_eq!(ayar.hedef_fps, 30, "Çok düşükte 30 FPS hedeflenmeli");
        assert_eq!(ayar.mod_, PerformansModu::Eco);
        assert!(ayar.uyari.is_some(), "Düşük donanımda kullanıcı uyarılmalı");
    }

    #[test]
    fn orta_donanim_tam_ozellik() {
        let p = DonanimProfili {
            cekirdek: 4,
            ram_bayt: 8 * GB,
            gpu_var: true,
        };
        let ayar = OtoAyar::hesapla(&p);
        assert_eq!(ayar.sinif, DonanimSinifi::Orta);
        assert!(!ayar.sadelesme);
        assert_eq!(ayar.hedef_fps, 60);
        assert!(ayar.uyari.is_none());
    }

    #[test]
    fn guclu_donanim_yuksek_sinif_bio() {
        let p = DonanimProfili {
            cekirdek: 16,
            ram_bayt: 32 * GB,
            gpu_var: true,
        };
        let ayar = OtoAyar::hesapla(&p);
        assert_eq!(ayar.sinif, DonanimSinifi::Yuksek);
        assert_eq!(ayar.mod_, PerformansModu::Bio);
        assert_eq!(ayar.hedef_fps, 60);
    }

    #[test]
    fn az_ram_dusuk_sinifa_dusurur() {
        // Çok çekirdek ama az RAM → yine düşük (RAM darboğazı).
        let p = DonanimProfili {
            cekirdek: 16,
            ram_bayt: 3 * GB,
            gpu_var: true,
        };
        assert_eq!(p.sinif(), DonanimSinifi::Dusuk);
    }

    #[test]
    fn worker_en_az_bir_ve_arayuze_pay_birakir() {
        let p = DonanimProfili {
            cekirdek: 1,
            ram_bayt: 8 * GB,
            gpu_var: false,
        };
        let ayar = OtoAyar::hesapla(&p);
        assert_eq!(ayar.onerilen_worker, 1, "En az 1 worker");

        let p8 = DonanimProfili {
            cekirdek: 8,
            ram_bayt: 16 * GB,
            gpu_var: true,
        };
        assert_eq!(
            OtoAyar::hesapla(&p8).onerilen_worker,
            7,
            "Arayüze 1 çekirdek pay"
        );
    }
}
