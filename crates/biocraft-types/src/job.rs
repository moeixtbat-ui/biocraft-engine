//! İş (Job) modeli — uzun işlemlerin **ilerleme / iptal / durum** soyutlaması
//! (İP-21; Gün 4/7 donanım-watchdog ve bellek-bütçe işleriyle tutarlı, MK-11/MK-07).
//!
//! Her uzun iş bir [`IsKulpu`] döndürür.  Arka plan görevi ilerlemeyi **iter**;
//! arayüz kare başına durumu **okur** (pull-based, MK-07 — GPU thread'i bloklanmaz).
//! Her iş bir [`TraceContext`] taşır → tüm logları ve hatası tek **iz** altında izlenebilir
//! (İP-21: "her uzun iş ... ZORUNLU correlation_id").

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::{JobStatus, TraceContext};

/// İptal jetonu — "Durdur" butonu bunu işaretler; iş döngüsü her parçada denetler (MK-11).
///
/// `Clone` ucuzdur (paylaşılan `Arc`); üreten ve tüketen aynı jetonu paylaşır.
#[derive(Debug, Clone, Default)]
pub struct IptalJetonu(Arc<AtomicBool>);

impl IptalJetonu {
    /// Temiz (iptal edilmemiş) jeton.
    pub fn yeni() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// İptali işaretler (geri alınamaz).
    pub fn iptal_et(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// İptal istendi mi?
    pub fn iptal_mi(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Bir işin ilerlemesi.  Belirsiz (toplamı bilinmeyen akış) veya ölçülebilir olabilir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ilerleme {
    /// Toplam bilinmiyor (ör. akış okuma) — yalnızca "çalışıyor" göster.
    Belirsiz,
    /// Yüzde (0–100).
    Yuzde(u8),
    /// Adım sayacı (ör. 3/10 dosya).
    Adim {
        /// Tamamlanan adım.
        tamam: u64,
        /// Toplam adım.
        toplam: u64,
    },
}

impl Ilerleme {
    /// 0–100 yüzdeye normalize eder (belirsizse `None`).
    pub fn yuzde(&self) -> Option<u8> {
        match self {
            Ilerleme::Belirsiz => None,
            Ilerleme::Yuzde(p) => Some((*p).min(100)),
            Ilerleme::Adim { tamam, toplam } => {
                if *toplam == 0 {
                    Some(100)
                } else {
                    Some(
                        ((*tamam as f64 / *toplam as f64) * 100.0)
                            .round()
                            .min(100.0) as u8,
                    )
                }
            }
        }
    }
}

/// İşin paylaşılan iç durumu (üretici yazar, tüketici okur).
#[derive(Debug)]
struct IsDurumu {
    status: JobStatus,
    ilerleme: Ilerleme,
}

/// Uzun bir işin kulpu — ilerleme bildirimi, iptal ve durum sorgusu sağlar.
///
/// `Clone` paylaşımlıdır: arka plan görevi bir kopyayı tutar (ilerleme **bildirir**),
/// arayüz başka bir kopyayı tutar (durumu **okur**, gerekirse **iptal eder**).
#[derive(Debug, Clone)]
pub struct IsKulpu {
    ad: Arc<str>,
    iz: TraceContext,
    iptal: IptalJetonu,
    durum: Arc<Mutex<IsDurumu>>,
}

impl IsKulpu {
    /// Yeni bir iş kulpu kurar; başlangıç durumu `Bekliyor`, ilerleme `Belirsiz`.
    /// `iz` verilmezse [`TraceContext::kok`] ile **yeni bir iz** başlatılır (her iş izlenir).
    pub fn yeni(ad: impl Into<String>, iz: Option<TraceContext>) -> Self {
        Self {
            ad: Arc::from(ad.into().as_str()),
            iz: iz.unwrap_or_else(TraceContext::kok),
            iptal: IptalJetonu::yeni(),
            durum: Arc::new(Mutex::new(IsDurumu {
                status: JobStatus::Bekliyor,
                ilerleme: Ilerleme::Belirsiz,
            })),
        }
    }

    /// İşin adı.
    pub fn ad(&self) -> &str {
        &self.ad
    }

    /// İşin iz bağlamı (loglar bunu taşır).
    pub fn iz(&self) -> &TraceContext {
        &self.iz
    }

    /// İptal jetonu (arka plan döngüsüne verilir).
    pub fn iptal_jetonu(&self) -> IptalJetonu {
        self.iptal.clone()
    }

    /// İptal istendi mi? (arka plan döngüsü her parçada çağırır)
    pub fn iptal_mi(&self) -> bool {
        self.iptal.iptal_mi()
    }

    /// İşi iptal et (arayüzden "Durdur").
    pub fn iptal_et(&self) {
        self.iptal.iptal_et();
    }

    /// İlerleme bildir; durumu `Calisiyor`a geçirir (zaten bitmemişse).
    pub fn ilerleme_bildir(&self, ilerleme: Ilerleme) {
        let mut d = self.durum.lock().unwrap();
        d.ilerleme = ilerleme;
        if matches!(d.status, JobStatus::Bekliyor | JobStatus::Calisiyor { .. }) {
            d.status = JobStatus::Calisiyor {
                ilerleme: ilerleme.yuzde(),
            };
        }
    }

    /// İşi başarıyla tamamlandı olarak işaretle (ilerleme %100).
    pub fn tamamla(&self) {
        let mut d = self.durum.lock().unwrap();
        d.ilerleme = Ilerleme::Yuzde(100);
        d.status = JobStatus::Bitti;
    }

    /// İşi hatayla sonlandır (kullanıcıya gösterilebilir mesaj).
    pub fn basarisiz(&self, mesaj: impl Into<String>) {
        let mut d = self.durum.lock().unwrap();
        d.status = JobStatus::Hata {
            mesaj: mesaj.into(),
        };
    }

    /// İptal edildi olarak işaretle (iş döngüsü iptali fark edip durduğunda).
    pub fn iptal_tamam(&self) {
        let mut d = self.durum.lock().unwrap();
        d.status = JobStatus::Hata {
            mesaj: "İş kullanıcı tarafından iptal edildi.".to_string(),
        };
    }

    /// Anlık durum (kopya).
    pub fn durum(&self) -> JobStatus {
        self.durum.lock().unwrap().status.clone()
    }

    /// Anlık ilerleme.
    pub fn ilerleme(&self) -> Ilerleme {
        self.durum.lock().unwrap().ilerleme
    }

    /// İş bitti mi (başarı veya hata)?
    pub fn bitti_mi(&self) -> bool {
        matches!(
            self.durum.lock().unwrap().status,
            JobStatus::Bitti | JobStatus::Hata { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yeni_is_bekliyor_durumunda_iz_tasir() {
        let is = IsKulpu::yeni("FASTA indeksleme", None);
        assert_eq!(is.durum(), JobStatus::Bekliyor);
        assert!(!is.iptal_mi());
        // Her iş bir iz (correlation) taşır.
        assert_ne!(is.iz().trace_id, [0u8; 16]);
    }

    #[test]
    fn ilerleme_bildirimi_calisiyora_gecirir() {
        let is = IsKulpu::yeni("hizalama", None);
        is.ilerleme_bildir(Ilerleme::Adim {
            tamam: 3,
            toplam: 10,
        });
        match is.durum() {
            JobStatus::Calisiyor { ilerleme } => assert_eq!(ilerleme, Some(30)),
            d => panic!("beklenen Calisiyor, gelen {d:?}"),
        }
    }

    #[test]
    fn tamamla_bitti_yuzde_100() {
        let is = IsKulpu::yeni("x", None);
        is.tamamla();
        assert_eq!(is.durum(), JobStatus::Bitti);
        assert_eq!(is.ilerleme().yuzde(), Some(100));
        assert!(is.bitti_mi());
    }

    #[test]
    fn iptal_paylasilan_jeton_uzerinden_gorunur() {
        let is = IsKulpu::yeni("uzun iş", None);
        let jeton = is.iptal_jetonu(); // arka plana verilir
        assert!(!jeton.iptal_mi());
        is.iptal_et(); // arayüzden durdur
        assert!(jeton.iptal_mi());
    }

    #[test]
    fn basarisiz_hata_durumu_tasir() {
        let is = IsKulpu::yeni("x", None);
        is.basarisiz("disk dolu");
        match is.durum() {
            JobStatus::Hata { mesaj } => assert!(mesaj.contains("disk")),
            d => panic!("beklenen Hata, gelen {d:?}"),
        }
        assert!(is.bitti_mi());
    }

    #[test]
    fn ilerleme_yuzde_normalize() {
        assert_eq!(Ilerleme::Belirsiz.yuzde(), None);
        assert_eq!(Ilerleme::Yuzde(150).yuzde(), Some(100));
        assert_eq!(
            Ilerleme::Adim {
                tamam: 1,
                toplam: 4
            }
            .yuzde(),
            Some(25)
        );
        assert_eq!(
            Ilerleme::Adim {
                tamam: 0,
                toplam: 0
            }
            .yuzde(),
            Some(100)
        );
    }

    #[test]
    fn verilen_iz_korunur() {
        let iz = TraceContext::kok();
        let is = IsKulpu::yeni("x", Some(iz));
        assert_eq!(is.iz().trace_id, iz.trace_id);
    }
}
