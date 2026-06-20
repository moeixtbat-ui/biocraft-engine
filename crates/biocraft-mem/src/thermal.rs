//! Termal eşik tablosu + kademeli yük azaltma (Zero-Impact) — İP-08, MK-24.
//!
//! **Uygulama kullanıcının donanımına asla bilinçli zarar vermez.**  GPU/CPU/NVMe
//! sıcaklığı yükseldikçe iş yükü **kademeli** azaltılır; kritik sıcaklıkta tamamen
//! durdurulur.  Bu modül tablonun **saf** (egui'siz, donanımsız) çekirdeğidir: bir
//! sıcaklık verir, alınacak [`TermalAksiyon`]'u döner.  Gerçek sensör okuma + watchdog
//! thread'i [`crate::hardware_guard`]'dadır.
//!
//! Tablo (İP-08 spec):
//!
//! | Parça | Sıcaklık | Aksiyon |
//! | --- | --- | --- |
//! | GPU | <70°C | Tam kapasite |
//! | GPU | 70–75°C | Yük ~%75 |
//! | GPU | 75–80°C | Yük ~%50 |
//! | GPU | 80–85°C | Duraklat (checkpoint + soğumayı bekle) |
//! | GPU | >85°C | Acil durdur |
//! | CPU | <75°C | Tam kapasite |
//! | CPU | 75–85°C | Yük ~%75 |
//! | CPU | 85–95°C | Duraklat |
//! | CPU | >95°C | Acil durdur |
//! | NVMe | <60°C | Tam I/O |
//! | NVMe | 60–70°C | I/O ~%50 |
//! | NVMe | >70°C | I/O duraklat |
//!
//! Eşikler [`TermalEsikler`] ile **ince ayarlanabilir** (spec: "ayardan ince ayarlanabilir").
//! Duraklamadan çıkış için **histerezis** vardır: bir parça eşiği aşıp duraklatıldıysa,
//! yalnızca eşiğin `histerezis` derece altına soğuyunca yeniden hızlanır (titreşim/yo-yo önlenir).

/// Termal koruma için izlenen donanım parçası.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DonanimParca {
    /// Ekran kartı (GPU).
    Gpu,
    /// İşlemci (CPU).
    Cpu,
    /// NVMe SSD (depolama).
    Nvme,
}

impl DonanimParca {
    /// Durum panelinde gösterilecek kısa ad.
    pub fn ad(&self) -> &'static str {
        match self {
            DonanimParca::Gpu => "GPU",
            DonanimParca::Cpu => "CPU",
            DonanimParca::Nvme => "NVMe",
        }
    }
}

/// Sıcaklığa karşı alınacak **kademeli** aksiyon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermalAksiyon {
    /// Tam kapasite — sıcaklık güvenli aralıkta.
    TamKapasite,
    /// Yükü hedef yüzdeye düşür (örn. 75, 50) — kademeli kısma.
    YukAzalt(u8),
    /// İşi **duraklat**, checkpoint al, soğumayı bekle (veri kaybı yok).
    Duraklat,
    /// **Acil durdur** — kritik sıcaklık; donanımı korumak için iş hemen kesilir.
    AcilDurdur,
}

impl TermalAksiyon {
    /// İzin verilen yük oranı (0.0–1.0).  Duraklat/AcilDurdur → 0.0 (iş ilerlemez).
    pub fn yuk_orani(&self) -> f32 {
        match self {
            TermalAksiyon::TamKapasite => 1.0,
            TermalAksiyon::YukAzalt(p) => (*p as f32 / 100.0).clamp(0.0, 1.0),
            TermalAksiyon::Duraklat | TermalAksiyon::AcilDurdur => 0.0,
        }
    }

    /// İş bu aksiyonda **duraklar mı?** (Duraklat veya AcilDurdur).  Bu kenar tetiklendiğinde
    /// checkpoint alınır (watchdog).
    pub fn duraklatir(&self) -> bool {
        matches!(self, TermalAksiyon::Duraklat | TermalAksiyon::AcilDurdur)
    }

    /// Bu **acil durdurma** mı? (en kritik durum).
    pub fn acil_mi(&self) -> bool {
        matches!(self, TermalAksiyon::AcilDurdur)
    }

    /// "Soğutuluyor" rozeti gösterilmeli mi?  Yalnızca [`TermalAksiyon::Duraklat`]'ta:
    /// iş geçici durdu, soğuyunca **otomatik devam** edecek.  Acil durdurma kalıcı bir
    /// güvenlik kesintisidir, "soğutuluyor" değil.
    pub fn sogutuluyor_mu(&self) -> bool {
        matches!(self, TermalAksiyon::Duraklat)
    }

    /// Ciddiyet ağırlığı — birden çok parçanın aksiyonu arasından **en kötüsünü** seçmek için.
    /// Büyük = daha ciddi.  Aynı kademede daha düşük yük oranı daha ciddidir.
    pub fn agirlik(&self) -> u32 {
        match self {
            TermalAksiyon::TamKapasite => 0,
            // 75% → 25, 50% → 50 (düşük yük = daha ciddi); aralık 1..=100.
            TermalAksiyon::YukAzalt(p) => 100u32.saturating_sub(*p as u32).max(1),
            TermalAksiyon::Duraklat => 1_000,
            TermalAksiyon::AcilDurdur => 2_000,
        }
    }

    /// Kullanıcıya gösterilecek kısa Türkçe açıklama.
    pub fn ad(&self) -> String {
        match self {
            TermalAksiyon::TamKapasite => "Tam kapasite".to_string(),
            TermalAksiyon::YukAzalt(p) => format!("Yük %{p}'e düşürüldü"),
            TermalAksiyon::Duraklat => "Duraklatıldı (soğutuluyor)".to_string(),
            TermalAksiyon::AcilDurdur => "Acil durduruldu (kritik sıcaklık)".to_string(),
        }
    }
}

/// Termal eşik tablosu — parça başına °C eşikleri.  Varsayılan [`TermalEsikler::default`]
/// İP-08 spec tablosudur; kullanıcı ayarlardan ince ayarlayabilir.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TermalEsikler {
    /// GPU: bu sıcaklıktan itibaren yük %75'e düşer.
    pub gpu_azalt75: f32,
    /// GPU: bu sıcaklıktan itibaren yük %50'ye düşer.
    pub gpu_azalt50: f32,
    /// GPU: bu sıcaklıktan itibaren iş duraklatılır.
    pub gpu_duraklat: f32,
    /// GPU: bu sıcaklıktan itibaren acil durdurma.
    pub gpu_acil: f32,
    /// CPU: bu sıcaklıktan itibaren yük %75'e düşer.
    pub cpu_azalt75: f32,
    /// CPU: bu sıcaklıktan itibaren iş duraklatılır.
    pub cpu_duraklat: f32,
    /// CPU: bu sıcaklıktan itibaren acil durdurma.
    pub cpu_acil: f32,
    /// NVMe: bu sıcaklıktan itibaren I/O %50'ye düşer.
    pub nvme_azalt50: f32,
    /// NVMe: bu sıcaklıktan itibaren I/O duraklatılır.
    pub nvme_duraklat: f32,
    /// Histerezis (°C): duraklamadan çıkmak için eşiğin bu kadar altına soğumak gerekir.
    pub histerezis: f32,
}

impl Default for TermalEsikler {
    fn default() -> Self {
        // İP-08 spec tablosu.
        Self {
            gpu_azalt75: 70.0,
            gpu_azalt50: 75.0,
            gpu_duraklat: 80.0,
            gpu_acil: 85.0,
            cpu_azalt75: 75.0,
            cpu_duraklat: 85.0,
            cpu_acil: 95.0,
            nvme_azalt50: 60.0,
            nvme_duraklat: 70.0,
            histerezis: 5.0,
        }
    }
}

impl TermalEsikler {
    /// **Tablodan stateless aksiyon** — verilen parça + sıcaklık için kademeli aksiyon.
    pub fn aksiyon(&self, parca: DonanimParca, sicaklik_c: f32) -> TermalAksiyon {
        match parca {
            DonanimParca::Gpu => {
                if sicaklik_c >= self.gpu_acil {
                    TermalAksiyon::AcilDurdur
                } else if sicaklik_c >= self.gpu_duraklat {
                    TermalAksiyon::Duraklat
                } else if sicaklik_c >= self.gpu_azalt50 {
                    TermalAksiyon::YukAzalt(50)
                } else if sicaklik_c >= self.gpu_azalt75 {
                    TermalAksiyon::YukAzalt(75)
                } else {
                    TermalAksiyon::TamKapasite
                }
            }
            DonanimParca::Cpu => {
                if sicaklik_c >= self.cpu_acil {
                    TermalAksiyon::AcilDurdur
                } else if sicaklik_c >= self.cpu_duraklat {
                    TermalAksiyon::Duraklat
                } else if sicaklik_c >= self.cpu_azalt75 {
                    TermalAksiyon::YukAzalt(75)
                } else {
                    TermalAksiyon::TamKapasite
                }
            }
            DonanimParca::Nvme => {
                if sicaklik_c >= self.nvme_duraklat {
                    TermalAksiyon::Duraklat
                } else if sicaklik_c >= self.nvme_azalt50 {
                    TermalAksiyon::YukAzalt(50)
                } else {
                    TermalAksiyon::TamKapasite
                }
            }
        }
    }

    /// Bu parçanın **duraklatma eşiği** (°C) — histerezis hesabı için.
    pub fn duraklat_esigi(&self, parca: DonanimParca) -> f32 {
        match parca {
            DonanimParca::Gpu => self.gpu_duraklat,
            DonanimParca::Cpu => self.cpu_duraklat,
            DonanimParca::Nvme => self.nvme_duraklat,
        }
    }

    /// **Histerezisli aksiyon.**  Parça hâlihazırda duraklatılmışsa (`duraklatildi_mi`),
    /// yalnızca duraklatma eşiğinin `histerezis` derece **altına** soğuyunca normale döner;
    /// aksi halde duraklamada kalır (yo-yo/titreşim önlenir → "soğuyunca otomatik devam").
    pub fn aksiyon_histerezisli(
        &self,
        parca: DonanimParca,
        sicaklik_c: f32,
        duraklatildi_mi: bool,
    ) -> TermalAksiyon {
        let ham = self.aksiyon(parca, sicaklik_c);
        if duraklatildi_mi && !ham.duraklatir() && !ham.acil_mi() {
            let cozulme = self.duraklat_esigi(parca) - self.histerezis;
            if sicaklik_c > cozulme {
                return TermalAksiyon::Duraklat;
            }
        }
        ham
    }
}

/// Bir dizi (parça, aksiyon) arasından **en kötü** olanı seçer; tetikleyen parçayı da döner.
/// Hiç sıcaklık yoksa `(TamKapasite, None)`.
pub fn en_kotu_aksiyon(
    girdiler: impl IntoIterator<Item = (DonanimParca, TermalAksiyon)>,
) -> (TermalAksiyon, Option<DonanimParca>) {
    let mut en_kotu = TermalAksiyon::TamKapasite;
    let mut tetik = None;
    for (parca, aksiyon) in girdiler {
        if aksiyon.agirlik() > en_kotu.agirlik() {
            en_kotu = aksiyon;
            tetik = Some(parca);
        }
    }
    (en_kotu, tetik)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_tablosu_speclere_uyar() {
        let e = TermalEsikler::default();
        assert_eq!(
            e.aksiyon(DonanimParca::Gpu, 50.0),
            TermalAksiyon::TamKapasite
        );
        assert_eq!(
            e.aksiyon(DonanimParca::Gpu, 72.0),
            TermalAksiyon::YukAzalt(75)
        );
        assert_eq!(
            e.aksiyon(DonanimParca::Gpu, 77.0),
            TermalAksiyon::YukAzalt(50)
        );
        assert_eq!(e.aksiyon(DonanimParca::Gpu, 82.0), TermalAksiyon::Duraklat);
        assert_eq!(
            e.aksiyon(DonanimParca::Gpu, 90.0),
            TermalAksiyon::AcilDurdur
        );
    }

    #[test]
    fn gpu_esik_sinirlari_alt_dahil() {
        let e = TermalEsikler::default();
        // Tam eşik değeri bir üst kademeye dahildir (70 → %75 başlar).
        assert_eq!(
            e.aksiyon(DonanimParca::Gpu, 70.0),
            TermalAksiyon::YukAzalt(75)
        );
        assert_eq!(
            e.aksiyon(DonanimParca::Gpu, 75.0),
            TermalAksiyon::YukAzalt(50)
        );
        assert_eq!(e.aksiyon(DonanimParca::Gpu, 80.0), TermalAksiyon::Duraklat);
        assert_eq!(
            e.aksiyon(DonanimParca::Gpu, 85.0),
            TermalAksiyon::AcilDurdur
        );
    }

    #[test]
    fn cpu_tablosu_speclere_uyar() {
        let e = TermalEsikler::default();
        assert_eq!(
            e.aksiyon(DonanimParca::Cpu, 60.0),
            TermalAksiyon::TamKapasite
        );
        assert_eq!(
            e.aksiyon(DonanimParca::Cpu, 80.0),
            TermalAksiyon::YukAzalt(75)
        );
        assert_eq!(e.aksiyon(DonanimParca::Cpu, 90.0), TermalAksiyon::Duraklat);
        assert_eq!(
            e.aksiyon(DonanimParca::Cpu, 96.0),
            TermalAksiyon::AcilDurdur
        );
    }

    #[test]
    fn nvme_tablosu_speclere_uyar() {
        let e = TermalEsikler::default();
        assert_eq!(
            e.aksiyon(DonanimParca::Nvme, 50.0),
            TermalAksiyon::TamKapasite
        );
        assert_eq!(
            e.aksiyon(DonanimParca::Nvme, 65.0),
            TermalAksiyon::YukAzalt(50)
        );
        assert_eq!(e.aksiyon(DonanimParca::Nvme, 75.0), TermalAksiyon::Duraklat);
    }

    #[test]
    fn yuk_orani_kademeli_azalir() {
        assert_eq!(TermalAksiyon::TamKapasite.yuk_orani(), 1.0);
        assert_eq!(TermalAksiyon::YukAzalt(75).yuk_orani(), 0.75);
        assert_eq!(TermalAksiyon::YukAzalt(50).yuk_orani(), 0.5);
        assert_eq!(TermalAksiyon::Duraklat.yuk_orani(), 0.0);
        assert_eq!(TermalAksiyon::AcilDurdur.yuk_orani(), 0.0);
    }

    #[test]
    fn agirlik_dusuk_yuku_daha_ciddi_sayar() {
        // %50 yük, %75 yükten daha ciddi (daha çok kısma) olmalı.
        assert!(TermalAksiyon::YukAzalt(50).agirlik() > TermalAksiyon::YukAzalt(75).agirlik());
        assert!(TermalAksiyon::Duraklat.agirlik() > TermalAksiyon::YukAzalt(50).agirlik());
        assert!(TermalAksiyon::AcilDurdur.agirlik() > TermalAksiyon::Duraklat.agirlik());
    }

    #[test]
    fn en_kotu_aksiyon_en_ciddiyi_secer() {
        let (aksiyon, tetik) = en_kotu_aksiyon([
            (DonanimParca::Gpu, TermalAksiyon::YukAzalt(75)),
            (DonanimParca::Cpu, TermalAksiyon::Duraklat),
            (DonanimParca::Nvme, TermalAksiyon::TamKapasite),
        ]);
        assert_eq!(aksiyon, TermalAksiyon::Duraklat);
        assert_eq!(tetik, Some(DonanimParca::Cpu));
    }

    #[test]
    fn en_kotu_aksiyon_bos_girdi_tam_kapasite() {
        let (aksiyon, tetik) = en_kotu_aksiyon(std::iter::empty());
        assert_eq!(aksiyon, TermalAksiyon::TamKapasite);
        assert_eq!(tetik, None);
    }

    #[test]
    fn histerezis_sicaklik_dususe_kadar_duraklamada_tutar() {
        let e = TermalEsikler::default();
        // GPU 80°C'de duraklatıldı; 78°C'ye düştü ama histerezis (80-5=75) üstünde → hâlâ duraklı.
        let a = e.aksiyon_histerezisli(DonanimParca::Gpu, 78.0, true);
        assert_eq!(
            a,
            TermalAksiyon::Duraklat,
            "Histerezis altına inmeden devam etmemeli"
        );
        // 74°C → 75'in altına indi → normale (kademeli) döner.
        let b = e.aksiyon_histerezisli(DonanimParca::Gpu, 74.0, true);
        assert!(
            !b.duraklatir(),
            "Histerezis altına inince otomatik devam etmeli"
        );
        assert_eq!(b, TermalAksiyon::YukAzalt(75));
    }

    #[test]
    fn histerezis_acil_durumu_engellemez() {
        let e = TermalEsikler::default();
        // Duraklı iken sıcaklık kritiğe fırlarsa histerezis acil durdurmayı geciktirmemeli.
        let a = e.aksiyon_histerezisli(DonanimParca::Gpu, 90.0, true);
        assert_eq!(a, TermalAksiyon::AcilDurdur);
    }
}
