//! Çalıştırma denetleyicisi (İP-06) — arayüz tarafı.
//!
//! Kullanıcı "Çalıştır"a basınca [`CalistirmaDurumu::baslat`] kodu **ayrı süreçte** başlatır
//! ([`biocraft_plugin_host::exec`], MK-02) ve bir tutamaç saklar.  Arayüz **her karede**
//! [`CalistirmaDurumu::yokla`] ile çıktıyı **bloklamadan** toplar (MK-07) → sonsuz döngü bile
//! olsa arayüz donmaz.  "Durdur" [`CalistirmaDurumu::durdur`] süreci öldürür.
//!
//! Çekirdek olay-uygulama mantığı ([`CalistirmaDurumu::olaylari_uygula`]) süreçten bağımsız
//! birim-testlenebilir tutulur (sentetik olaylarla).
// MK-02: çalıştırma daima ayrı süreçte; bu modül yalnız tutamacı sürer + çıktıyı biriktirir.

use biocraft_plugin_host::exec::{
    calistir_baslat, CalismaModu, CalismaOlay, CalismaTutamac, KodCalismaLimitleri,
};
use biocraft_types::ErrorReport;

/// Çıktının taşmaması için tutulan azami satır sayısı (eski satırlar atılır).
const AZAMI_CIKTI: usize = 5_000;

/// Çalıştırmanın o anki durumu.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Calisma {
    /// Çalışmıyor (henüz başlamadı veya temizlendi).
    #[default]
    Bosta,
    /// Şu an çalışıyor.
    Calisiyor,
    /// Normal bitti (çıkış kodu).
    Bitti(Option<i32>),
    /// Kullanıcı durdurdu.
    Durduruldu,
    /// Zaman aşımına uğradı.
    ZamanAsimi,
    /// Hata (başlatılamadı vb.).
    Hata,
}

impl Calisma {
    /// Arayüzde gösterilecek yerelleştirilmiş kısa etiket.
    pub fn etiket(&self, tr: bool) -> String {
        match self {
            Calisma::Bosta => if tr { "Hazır" } else { "Ready" }.into(),
            Calisma::Calisiyor => if tr { "Çalışıyor…" } else { "Running…" }.into(),
            Calisma::Bitti(Some(0)) => if tr {
                "Bitti (başarılı)"
            } else {
                "Done (ok)"
            }
            .into(),
            Calisma::Bitti(Some(k)) => {
                if tr {
                    format!("Bitti (çıkış {k})")
                } else {
                    format!("Done (exit {k})")
                }
            }
            Calisma::Bitti(None) => if tr { "Bitti" } else { "Done" }.into(),
            Calisma::Durduruldu => if tr { "Durduruldu" } else { "Stopped" }.into(),
            Calisma::ZamanAsimi => if tr { "Zaman aşımı" } else { "Timed out" }.into(),
            Calisma::Hata => if tr { "Hata" } else { "Error" }.into(),
        }
    }
}

/// Çıktı panelinde gösterilecek tek bir satır.
#[derive(Debug, Clone)]
pub struct CiktiSatiri {
    /// `true` ise stderr (hata akışı), `false` ise stdout.
    pub hata_akisi: bool,
    /// Satır metni.
    pub metin: String,
}

/// Bir editör belgesinin çalıştırma durumu + biriken çıktısı.
#[derive(Default)]
pub struct CalistirmaDurumu {
    /// O anki durum.
    pub durum: Calisma,
    /// Biriken çıktı satırları.
    pub cikti: Vec<CiktiSatiri>,
    /// Son çalıştırma kipi.
    pub modu: Option<CalismaModu>,
    /// Çalışan sürecin tutamacı (varsa).
    tutamac: Option<CalismaTutamac>,
}

impl std::fmt::Debug for CalistirmaDurumu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CalistirmaDurumu")
            .field("durum", &self.durum)
            .field("cikti_satir", &self.cikti.len())
            .field("calisiyor", &self.tutamac.is_some())
            .finish()
    }
}

impl CalistirmaDurumu {
    /// Boş durum.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Şu an bir süreç çalışıyor mu?
    pub fn calisiyor(&self) -> bool {
        matches!(self.durum, Calisma::Calisiyor)
    }

    /// Kodu **ayrı süreçte** başlatır (varsayılan limitlerle).  Çıktı temizlenir.
    pub fn baslat(&mut self, kod: &str, modu: CalismaModu) {
        self.baslat_limitli(kod, modu, KodCalismaLimitleri::default());
    }

    /// Kodu verili limitlerle başlatır.
    pub fn baslat_limitli(&mut self, kod: &str, modu: CalismaModu, limitler: KodCalismaLimitleri) {
        // Önceki çalışma varsa durdur (yeni başlatma temiz olsun).
        self.durdur();
        self.cikti.clear();
        self.modu = Some(modu);
        match calistir_baslat(kod, modu, limitler) {
            Ok(t) => {
                self.tutamac = Some(t);
                self.durum = Calisma::Calisiyor;
                self.bilgi_satiri(modu);
            }
            Err(r) => {
                self.tutamac = None;
                self.durum = Calisma::Hata;
                self.hata_satirlari(&r);
            }
        }
    }

    /// Çalışmayı durdurur (süreci öldürür).  Çalışmıyorsa zararsız.
    pub fn durdur(&mut self) {
        if let Some(t) = &self.tutamac {
            t.durdur();
        }
    }

    /// Her karede çağrılır: bekleyen çıktı olaylarını **bloklamadan** uygular (MK-07).
    pub fn yokla(&mut self) {
        let olaylar = match &self.tutamac {
            Some(t) => t.tumunu_dene(),
            None => return,
        };
        if !olaylar.is_empty() {
            self.olaylari_uygula(olaylar);
        }
    }

    /// Bir olay grubunu duruma/çıktıya uygular (süreçten bağımsız; birim-testlenebilir).
    pub fn olaylari_uygula(&mut self, olaylar: impl IntoIterator<Item = CalismaOlay>) {
        for olay in olaylar {
            match olay {
                CalismaOlay::Stdout(s) => self.satir_ekle(false, s),
                CalismaOlay::Stderr(s) => self.satir_ekle(true, s),
                CalismaOlay::Bitti { cikis_kodu } => {
                    self.durum = Calisma::Bitti(cikis_kodu);
                    self.tutamac = None;
                }
                CalismaOlay::Durduruldu => {
                    self.durum = Calisma::Durduruldu;
                    self.tutamac = None;
                }
                CalismaOlay::ZamanAsimi => {
                    self.durum = Calisma::ZamanAsimi;
                    self.tutamac = None;
                }
                CalismaOlay::Hata(r) => {
                    self.hata_satirlari(&r);
                    self.durum = Calisma::Hata;
                    self.tutamac = None;
                }
            }
        }
    }

    /// Çıktıyı temizler (durumu Bosta'ya çekmez — yalnız satırları siler).
    pub fn temizle(&mut self) {
        self.cikti.clear();
    }

    // ─── iç yardımcılar ─────────────────────────────────────────────────────

    fn satir_ekle(&mut self, hata_akisi: bool, metin: String) {
        self.cikti.push(CiktiSatiri { hata_akisi, metin });
        // Taşmayı önle: en eski satırları at (büyük/uzun çıktıda bellek koruması).
        if self.cikti.len() > AZAMI_CIKTI {
            let fazla = self.cikti.len() - AZAMI_CIKTI;
            self.cikti.drain(0..fazla);
        }
    }

    fn bilgi_satiri(&mut self, modu: CalismaModu) {
        self.satir_ekle(
            false,
            format!("▶ {} çalıştırılıyor (ayrı süreç)…", modu.ad()),
        );
    }

    fn hata_satirlari(&mut self, r: &ErrorReport) {
        self.satir_ekle(true, format!("✖ {}", r.ne_oldu));
        self.satir_ekle(true, format!("  {}", r.neden));
        self.satir_ekle(true, format!("  → {}", r.nasil_cozulur));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn olaylar_stdout_stderr_biriktirir_ve_biter() {
        let mut d = CalistirmaDurumu::yeni();
        d.durum = Calisma::Calisiyor;
        d.olaylari_uygula([
            CalismaOlay::Stdout("satır1".into()),
            CalismaOlay::Stderr("uyarı".into()),
            CalismaOlay::Bitti {
                cikis_kodu: Some(0),
            },
        ]);
        assert_eq!(d.cikti.len(), 2);
        assert!(!d.cikti[0].hata_akisi);
        assert!(d.cikti[1].hata_akisi);
        assert_eq!(d.durum, Calisma::Bitti(Some(0)));
        assert!(!d.calisiyor());
    }

    #[test]
    fn durduruldu_olayi_durumu_gunceller() {
        let mut d = CalistirmaDurumu::yeni();
        d.durum = Calisma::Calisiyor;
        d.olaylari_uygula([CalismaOlay::Durduruldu]);
        assert_eq!(d.durum, Calisma::Durduruldu);
    }

    #[test]
    fn zaman_asimi_olayi_durumu_gunceller() {
        let mut d = CalistirmaDurumu::yeni();
        d.olaylari_uygula([CalismaOlay::ZamanAsimi]);
        assert_eq!(d.durum, Calisma::ZamanAsimi);
    }

    #[test]
    fn cikti_azami_siniri_asilmaz() {
        let mut d = CalistirmaDurumu::yeni();
        let cok: Vec<CalismaOlay> = (0..AZAMI_CIKTI + 200)
            .map(|i| CalismaOlay::Stdout(format!("s{i}")))
            .collect();
        d.olaylari_uygula(cok);
        assert!(d.cikti.len() <= AZAMI_CIKTI);
        // En yeni satır korunur, en eskisi atılır.
        assert_eq!(
            d.cikti.last().unwrap().metin,
            format!("s{}", AZAMI_CIKTI + 199)
        );
    }

    #[test]
    fn hata_olayi_uc_satir_yazar() {
        let mut d = CalistirmaDurumu::yeni();
        let r = ErrorReport::new("ne", "neden", "çözüm");
        d.olaylari_uygula([CalismaOlay::Hata(r)]);
        assert_eq!(d.durum, Calisma::Hata);
        assert_eq!(d.cikti.len(), 3);
        assert!(d.cikti.iter().all(|s| s.hata_akisi));
    }

    #[test]
    fn etiketler_iki_dilde() {
        assert_ne!(
            Calisma::Calisiyor.etiket(true),
            Calisma::Calisiyor.etiket(false)
        );
        assert!(Calisma::Bitti(Some(2)).etiket(true).contains('2'));
    }

    /// Uçtan uca: Python varsa gerçek bir betik çalıştırılıp çıktı toplanır (donmadan).
    #[test]
    fn python_varsa_gercek_calistirma_uctan_uca() {
        if biocraft_plugin_host::python_bul().is_none() {
            eprintln!("Python yok → test atlandı");
            return;
        }
        let mut d = CalistirmaDurumu::yeni();
        d.baslat("print('selam editör')", CalismaModu::TamScript);
        assert!(d.calisiyor());
        // Bitene kadar yokla (donmadan, kısa uykularla).
        let basla = std::time::Instant::now();
        while d.calisiyor() && basla.elapsed() < std::time::Duration::from_secs(20) {
            d.yokla();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!d.calisiyor(), "süreç bitmeliydi");
        assert!(
            d.cikti.iter().any(|s| s.metin.contains("selam editör")),
            "çıktı 'selam editör' içermeli: {:?}",
            d.cikti
        );
    }
}
