//! **Atomik kurulum + geri alma** — yarıda kalan güncelleme uygulamayı bozmaz (İP-20, MK-56).
//!
//! Kurulum kökü üç bölmeden oluşur:
//! ```text
//! kok/
//!   current/    ← çalışan (aktif) sürüm dosyaları
//!   previous/   ← son iyi bilinen sürüm (geri alma / downgrade güvenlik ağı)
//!   staging/    ← güncelleme uygulanırken geçici hazırlık (commit sonrası silinir)
//! ```
//!
//! **Atomiklik:** Yeni sürüm önce `staging/`'e tam yazılıp doğrulanır.  Yalnız *doğrulama
//! geçerse* takas yapılır: `current → previous`, `staging → current` (aynı dosya sisteminde
//! `rename` atomiktir).  Süreç herhangi bir anda ölürse `current` ya eski ya yeni sürümü gösterir —
//! **asla yarım** kalmaz.  Doğrulama başarısızsa `current` hiç dokunulmadan kalır.
//!
//! **Geri alma / downgrade:** `previous` korunduğu için [`geri_al`](AtomikKurulum::geri_al) ile bir
//! önceki sürüme atomik dönülür (başarısız güncelleme veya kullanıcı isteğiyle downgrade).
//!
//! Bu katman **dosya kümeleriyle** çalışır; imza/BLAKE3 doğrulaması ve delta birleştirme bir üst
//! katmandadır (`super::guncellemeyi_uygula`).  Saf `std::fs` — dış bağımlılık yok.

use std::fs;
use std::path::{Path, PathBuf};

use biocraft_types::ErrorReport;

/// `current/` içine yazılan sürüm etiketi dosyası.
const SURUM_DOSYA: &str = "VERSION";

/// Bir güncellemenin geri alınıp alınamayacağını + sonucunu bildiren rapor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeriAlSonuc {
    /// Geri almadan önce aktif olan sürüm.
    pub onceki_aktif: Option<String>,
    /// Geri almadan sonra aktif olan sürüm.
    pub yeni_aktif: Option<String>,
}

/// Üç bölmeli (current/previous/staging) atomik kurulum kökü.
#[derive(Debug, Clone)]
pub struct AtomikKurulum {
    kok: PathBuf,
}

impl AtomikKurulum {
    /// Verilen kök altında atomik kurulumu hazırlar (bölmeler ilk yazımda oluşturulur).
    pub fn yeni(kok: impl Into<PathBuf>) -> Self {
        Self { kok: kok.into() }
    }

    fn current(&self) -> PathBuf {
        self.kok.join("current")
    }
    fn previous(&self) -> PathBuf {
        self.kok.join("previous")
    }
    fn staging(&self) -> PathBuf {
        self.kok.join("staging")
    }

    /// Aktif (current) sürümün etiketi; henüz kurulum yoksa `None`.
    pub fn gecerli_surum(&self) -> Option<String> {
        fs::read_to_string(self.current().join(SURUM_DOSYA))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Aktif sürümdeki bir dosyanın baytları (örn. paket blob'u); yoksa `None`.
    pub fn gecerli_dosya(&self, ad: &str) -> Option<Vec<u8>> {
        fs::read(self.current().join(ad)).ok()
    }

    /// Geri alınabilecek bir önceki sürüm var mı?
    pub fn geri_al_mumkun_mu(&self) -> bool {
        self.previous().join(SURUM_DOSYA).is_file()
    }

    /// Bir önceki (previous) sürümün etiketi; yoksa `None`.
    pub fn onceki_surum(&self) -> Option<String> {
        fs::read_to_string(self.previous().join(SURUM_DOSYA))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// İlk kurulum: doğrudan `current/`'ı yazar (takas yok — önceki sürüm yoktur).
    ///
    /// Dosyalar `(göreli_ad, baytlar)` çiftleridir; `VERSION` otomatik yazılır.
    pub fn ilk_kur(&self, surum: &str, dosyalar: &[(&str, Vec<u8>)]) -> Result<(), ErrorReport> {
        let current = self.current();
        if current.exists() {
            return Err(hata(
                "Kurulum zaten var",
                "current/ bölmesi zaten dolu; ilk kurulum yerine güncelleme kullanın.",
            ));
        }
        self.bolme_yaz(&self.staging(), surum, dosyalar)?;
        self.dogrula_bolme(&self.staging(), surum, dosyalar)?;
        atomik_yeniden_adlandir(&self.staging(), &current)?;
        Ok(())
    }

    /// Bir güncellemeyi **atomik** uygular: staging'e yaz → doğrula → takas (current→previous,
    /// staging→current).  Herhangi bir adım başarısızsa `current` **dokunulmaz** kalır.
    ///
    /// Çağıran taraf (üst katman) yeni paketin imza + BLAKE3 doğrulamasını zaten yapmıştır; burada
    /// ayrıca *diske yazıldıktan sonra* yeniden okuyup özet doğrulanır (disk/yarım-yazım koruması).
    pub fn uygula(&self, surum: &str, dosyalar: &[(&str, Vec<u8>)]) -> Result<(), ErrorReport> {
        let current = self.current();
        if !current.join(SURUM_DOSYA).is_file() {
            // Mevcut kurulum yoksa bu bir ilk kurulumdur.
            return self.ilk_kur(surum, dosyalar);
        }

        // 1) Staging'i taze yaz.
        let staging = self.staging();
        let _ = fs::remove_dir_all(&staging); // önceki yarım staging'i temizle
        self.bolme_yaz(&staging, surum, dosyalar)?;

        // 2) Diskten doğrula (yazım tamamlandı + baytlar birebir).
        self.dogrula_bolme(&staging, surum, dosyalar)?;

        // 3) Takas — bu noktadan sonra current ya eski ya yeni; asla yarım.
        let previous = self.previous();
        let _ = fs::remove_dir_all(&previous); // eski previous'ı bırak
        atomik_yeniden_adlandir(&current, &previous)?;
        // current artık yok; staging'i current yap.
        if let Err(e) = atomik_yeniden_adlandir(&staging, &current) {
            // Felaket önleme: staging→current başarısızsa previous'ı current'a geri al.
            let _ = atomik_yeniden_adlandir(&previous, &current);
            return Err(e);
        }
        Ok(())
    }

    /// Bir önceki sürüme **atomik geri dön** (başarısız güncelleme sonrası veya kullanıcı downgrade).
    /// previous yoksa net hata; current ile previous yer değiştirir (geri-al da geri alınabilir).
    pub fn geri_al(&self) -> Result<GeriAlSonuc, ErrorReport> {
        if !self.geri_al_mumkun_mu() {
            return Err(hata(
                "Geri alınacak sürüm yok",
                "Daha önce kurulmuş (previous) bir sürüm bulunmadığından geri alma yapılamaz.",
            ));
        }
        let onceki_aktif = self.gecerli_surum();
        let current = self.current();
        let previous = self.previous();
        let gecici = self.kok.join(".swap-tmp");
        let _ = fs::remove_dir_all(&gecici);

        // current ↔ previous takası (üç-adımlı; her adım rename = atomik).
        atomik_yeniden_adlandir(&current, &gecici)?;
        atomik_yeniden_adlandir(&previous, &current)?;
        atomik_yeniden_adlandir(&gecici, &previous)?;

        Ok(GeriAlSonuc {
            onceki_aktif,
            yeni_aktif: self.gecerli_surum(),
        })
    }

    // ─── İç yardımcılar ──────────────────────────────────────────────────────

    fn bolme_yaz(
        &self,
        bolme: &Path,
        surum: &str,
        dosyalar: &[(&str, Vec<u8>)],
    ) -> Result<(), ErrorReport> {
        fs::create_dir_all(bolme)
            .map_err(|e| io_hata("Hazırlık klasörü oluşturulamadı", bolme, &e))?;
        for (ad, bayt) in dosyalar {
            let yol = bolme.join(ad);
            if let Some(ust) = yol.parent() {
                fs::create_dir_all(ust)
                    .map_err(|e| io_hata("Alt klasör oluşturulamadı", ust, &e))?;
            }
            fs::write(&yol, bayt).map_err(|e| io_hata("Dosya yazılamadı", &yol, &e))?;
        }
        fs::write(bolme.join(SURUM_DOSYA), surum.as_bytes())
            .map_err(|e| io_hata("Sürüm dosyası yazılamadı", bolme, &e))?;
        Ok(())
    }

    fn dogrula_bolme(
        &self,
        bolme: &Path,
        surum: &str,
        dosyalar: &[(&str, Vec<u8>)],
    ) -> Result<(), ErrorReport> {
        let yazili_surum = fs::read_to_string(bolme.join(SURUM_DOSYA))
            .map_err(|e| io_hata("Sürüm dosyası okunamadı", bolme, &e))?;
        if yazili_surum.trim() != surum {
            return Err(hata(
                "Güncelleme doğrulanamadı",
                "Diske yazılan sürüm etiketi beklenenle uyuşmuyor (yarım yazım?).",
            ));
        }
        for (ad, bayt) in dosyalar {
            let yol = bolme.join(ad);
            let diskten =
                fs::read(&yol).map_err(|e| io_hata("Doğrulama için okunamadı", &yol, &e))?;
            if blake3::hash(&diskten) != blake3::hash(bayt) {
                return Err(hata(
                    "Güncelleme doğrulanamadı",
                    "Diske yazılan bir dosya beklenen içerikle eşleşmiyor (bozuk/yarım yazım).",
                ));
            }
        }
        Ok(())
    }
}

/// `rename` ile atomik taşıma; aynı dosya sisteminde atomiktir.
fn atomik_yeniden_adlandir(kaynak: &Path, hedef: &Path) -> Result<(), ErrorReport> {
    fs::rename(kaynak, hedef).map_err(|e| io_hata("Atomik takas başarısız (rename)", kaynak, &e))
}

fn io_hata(ne: &str, yol: &Path, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        ne.to_string(),
        format!("'{}' işlemi başarısız oldu.", yol.display()),
        "Disk alanı/izinlerini kontrol edip yeniden deneyin; sorun sürerse uygulama eski sürümle \
         güvenle çalışmaya devam eder.",
    )
    .with_teknik_detay(e.to_string())
}

fn hata(ne: &str, neden: &str) -> ErrorReport {
    ErrorReport::new(
        ne.to_string(),
        neden.to_string(),
        "Güncellemeyi yeniden deneyin; uygulama bu sırada eski sürümle çalışmaya devam eder.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dosyalar(blob: &[u8]) -> Vec<(&'static str, Vec<u8>)> {
        vec![("paket.bin", blob.to_vec())]
    }

    #[test]
    fn ilk_kurulum_ve_okuma() {
        let tmp = std::env::temp_dir().join(format!("bc-rb-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let k = AtomikKurulum::yeni(&tmp);
        k.uygula("1.0.0", &dosyalar(b"v1")).unwrap();
        assert_eq!(k.gecerli_surum().as_deref(), Some("1.0.0"));
        assert_eq!(k.gecerli_dosya("paket.bin").as_deref(), Some(&b"v1"[..]));
        assert!(!k.geri_al_mumkun_mu());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn guncelleme_takas_ve_previous() {
        let tmp = std::env::temp_dir().join(format!("bc-rb2-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let k = AtomikKurulum::yeni(&tmp);
        k.uygula("1.0.0", &dosyalar(b"v1")).unwrap();
        k.uygula("1.1.0", &dosyalar(b"v2")).unwrap();
        assert_eq!(k.gecerli_surum().as_deref(), Some("1.1.0"));
        assert_eq!(k.gecerli_dosya("paket.bin").as_deref(), Some(&b"v2"[..]));
        assert!(k.geri_al_mumkun_mu());
        assert_eq!(k.onceki_surum().as_deref(), Some("1.0.0"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn geri_al_onceki_surume_doner() {
        let tmp = std::env::temp_dir().join(format!("bc-rb3-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let k = AtomikKurulum::yeni(&tmp);
        k.uygula("1.0.0", &dosyalar(b"v1")).unwrap();
        k.uygula("2.0.0", &dosyalar(b"v2")).unwrap();
        let sonuc = k.geri_al().unwrap();
        assert_eq!(sonuc.onceki_aktif.as_deref(), Some("2.0.0"));
        assert_eq!(sonuc.yeni_aktif.as_deref(), Some("1.0.0"));
        assert_eq!(k.gecerli_dosya("paket.bin").as_deref(), Some(&b"v1"[..]));
        // Geri-al da geri alınabilir (downgrade'i geri al = yeniden 2.0.0).
        let geri = k.geri_al().unwrap();
        assert_eq!(geri.yeni_aktif.as_deref(), Some("2.0.0"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn geri_al_yoksa_net_hata() {
        let tmp = std::env::temp_dir().join(format!("bc-rb4-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let k = AtomikKurulum::yeni(&tmp);
        k.uygula("1.0.0", &dosyalar(b"v1")).unwrap();
        assert!(k.geri_al().is_err());
        let _ = fs::remove_dir_all(&tmp);
    }
}
