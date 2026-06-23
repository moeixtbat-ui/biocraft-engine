//! Yapılandırılmış log sink'i — `log` cephesini L0 [`LogKaydi`]'na bağlar (İP-21, MK-57, MK-45).
//!
//! - **Konsol:** insan-okur tek satır (geliştirici/destek için).
//! - **Dosya:** NDJSON (satır başına bir JSON), OTel-uyumlu alanlar → toplayıcıya aktarılabilir.
//! - **İz bağlamı:** thread-local "kapsam" yığını; bir uzun iş/dış çağrı [`kapsam`] içinde
//!   çalışırken o thread'deki **tüm** log satırları o izin `trace_id/span_id`'sini taşır.
//! - **PII:** her mesaj/alan [`biocraft_types::pii_temizle`]'den geçer (MK-45).

use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use biocraft_types::{LogKaydi, LogSeverity, TraceContext};

// ─── İz bağlamı: thread-local kapsam yığını ───────────────────────────────────

thread_local! {
    static AMBIYANS: RefCell<Vec<TraceContext>> = const { RefCell::new(Vec::new()) };
}

/// Bir iz bağlamını **mevcut thread'de** etkin kılan RAII koruyucu.
///
/// Yaşadığı sürece, bu thread'deki tüm `log::*` satırları bu izin kimliklerini taşır.
/// `Drop` ile bağlam yığından çıkar → iç içe kapsamlar doğru sırada geri alınır.
#[must_use = "Kapsam düştüğünde iz bağlamı kalkar; değişkene bağlayın"]
pub struct Kapsam {
    _gizli: (),
}

impl Drop for Kapsam {
    fn drop(&mut self) {
        AMBIYANS.with(|y| {
            y.borrow_mut().pop();
        });
    }
}

/// Verilen iz bağlamını bu thread için etkin kılar (uzun iş/dış çağrı başında).
pub fn kapsam(iz: TraceContext) -> Kapsam {
    AMBIYANS.with(|y| y.borrow_mut().push(iz));
    Kapsam { _gizli: () }
}

/// Yeni bir **kök iz** açıp döndüren kısayol (sık kullanılan kalıp).
pub fn with_iz() -> (Kapsam, TraceContext) {
    let iz = TraceContext::kok();
    (kapsam(iz), iz)
}

fn gecerli_iz() -> Option<TraceContext> {
    AMBIYANS.with(|y| y.borrow().last().copied())
}

// ─── Seviye süzgeci (env_logger benzeri yönerge ayrıştırma) ───────────────────

/// `info,wgpu_core=warn,naga=warn` gibi bir yönergeyi ayrıştıran seviye süzgeci.
#[derive(Debug, Clone)]
struct SeviyeSuzgec {
    varsayilan: log::LevelFilter,
    hedefler: Vec<(String, log::LevelFilter)>,
}

impl SeviyeSuzgec {
    fn ayristir(yonerge: &str) -> Self {
        let mut varsayilan = log::LevelFilter::Info;
        let mut hedefler = Vec::new();
        for parca in yonerge.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            if let Some((hedef, seviye)) = parca.split_once('=') {
                if let Some(lf) = seviye_coz(seviye.trim()) {
                    hedefler.push((hedef.trim().to_string(), lf));
                }
            } else if let Some(lf) = seviye_coz(parca) {
                varsayilan = lf;
            }
        }
        // En uzun önek önce eşleşsin diye uzunluğa göre azalan sırala.
        hedefler.sort_by_key(|(on, _)| std::cmp::Reverse(on.len()));
        Self {
            varsayilan,
            hedefler,
        }
    }

    /// Bu hedef + seviye loglanmalı mı?
    fn izin(&self, hedef: &str, seviye: log::Level) -> bool {
        let esik = self
            .hedefler
            .iter()
            .find(|(on, _)| hedef.starts_with(on.as_str()))
            .map(|(_, lf)| *lf)
            .unwrap_or(self.varsayilan);
        seviye <= esik
    }
}

fn seviye_coz(s: &str) -> Option<log::LevelFilter> {
    match s.to_ascii_lowercase().as_str() {
        "off" => Some(log::LevelFilter::Off),
        "error" => Some(log::LevelFilter::Error),
        "warn" => Some(log::LevelFilter::Warn),
        "info" => Some(log::LevelFilter::Info),
        "debug" => Some(log::LevelFilter::Debug),
        "trace" => Some(log::LevelFilter::Trace),
        _ => None,
    }
}

fn seviye_cevir(s: log::Level) -> LogSeverity {
    match s {
        log::Level::Error => LogSeverity::Error,
        log::Level::Warn => LogSeverity::Warn,
        log::Level::Info => LogSeverity::Info,
        log::Level::Debug => LogSeverity::Debug,
        log::Level::Trace => LogSeverity::Trace,
    }
}

// ─── Sink ─────────────────────────────────────────────────────────────────────

/// Gözlemlenebilirlik başlatma ayarı.
#[derive(Debug, Clone)]
pub struct GozlemAyari {
    /// Seviye yönergesi (env `BIOCRAFT_LOG` yoksa bu).
    pub yonerge: String,
    /// NDJSON log dosyalarının yazılacağı klasör (`None` → yalnız konsol).
    pub log_dizini: Option<PathBuf>,
}

impl Default for GozlemAyari {
    fn default() -> Self {
        Self {
            // MK-08: kendi katmanlarımız "info"; wgpu/naga arka plan gürültüsü kısılır.
            yonerge: "info,wgpu_core=warn,wgpu_hal=error,naga=warn,wgpu=warn".to_string(),
            log_dizini: None,
        }
    }
}

struct YapilandirilmisLoglayici {
    suzgec: SeviyeSuzgec,
    dosya: Option<Mutex<File>>,
}

impl log::Log for YapilandirilmisLoglayici {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.suzgec.izin(metadata.target(), metadata.level())
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        // L0 yapılandırılmış kayıt — mesaj burada PII süzgecinden geçer (MK-45).
        let mut kayit = LogKaydi::yeni(
            seviye_cevir(record.level()),
            record.target().to_string(),
            record.args().to_string(),
        );
        if let Some(iz) = gecerli_iz() {
            kayit = kayit.with_iz(&iz);
        }

        // Konsol: insan-okur satır (stderr).
        eprintln!("{}", kayit.satir());

        // Dosya: NDJSON.
        if let Some(kilit) = &self.dosya {
            if let Ok(mut f) = kilit.lock() {
                let _ = writeln!(f, "{}", kayit.to_json());
            }
        }
    }

    fn flush(&self) {
        if let Some(kilit) = &self.dosya {
            if let Ok(mut f) = kilit.lock() {
                let _ = f.flush();
            }
        }
    }
}

/// Gözlemlenebilirliği başlatır: `log` cephesine yapılandırılmış sink'i kurar.
///
/// `BIOCRAFT_LOG` ortam değişkeni ayarlıysa seviye yönergesini ondan alır (geliştirici geçersiz
/// kılma).  Dosya açılamazsa (ör. salt-okunur disk) sessizce yalnız-konsola düşer — loglama
/// asla uygulamayı çökertmemelidir.  İkinci kez çağrılırsa (test) sessizce yok sayılır.
pub fn init(ayar: GozlemAyari) {
    let yonerge = std::env::var("BIOCRAFT_LOG").unwrap_or(ayar.yonerge);
    let suzgec = SeviyeSuzgec::ayristir(&yonerge);

    let dosya = ayar.log_dizini.as_ref().and_then(|d| log_dosyasi_ac(d));

    let loglayici = YapilandirilmisLoglayici { suzgec, dosya };
    // En yüksek seviyeyi cepheye bildir; ince süzme bizdedir.
    log::set_max_level(log::LevelFilter::Trace);
    // İkinci çağrı (örn. testte) hata döndürür → yok say.
    let _ = log::set_boxed_logger(Box::new(loglayici));
}

/// `<dizin>/biocraft-YYYYMMDD.ndjson` dosyasını ekleme (append) kipinde açar.
fn log_dosyasi_ac(dizin: &Path) -> Option<Mutex<File>> {
    if std::fs::create_dir_all(dizin).is_err() {
        return None;
    }
    let tarih = chrono::Utc::now().format("%Y%m%d");
    let yol = dizin.join(format!("biocraft-{tarih}.ndjson"));
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(yol)
        .ok()
        .map(Mutex::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suzgec_varsayilan_ve_hedef() {
        let s = SeviyeSuzgec::ayristir("info,wgpu_core=warn,naga=error");
        // Kendi katman info geçer.
        assert!(s.izin("biocraft_data::project", log::Level::Info));
        assert!(!s.izin("biocraft_data::project", log::Level::Debug));
        // wgpu_core yalnız warn ve üstü.
        assert!(s.izin("wgpu_core::device", log::Level::Warn));
        assert!(!s.izin("wgpu_core::device", log::Level::Info));
        // naga yalnız error.
        assert!(s.izin("naga::front", log::Level::Error));
        assert!(!s.izin("naga::front", log::Level::Warn));
    }

    #[test]
    fn en_uzun_onek_kazanir() {
        let s = SeviyeSuzgec::ayristir("warn,wgpu=warn,wgpu_hal=error");
        // wgpu_hal, wgpu önekinden uzun → error eşiği uygulanır.
        assert!(!s.izin("wgpu_hal::vulkan", log::Level::Warn));
        assert!(s.izin("wgpu_hal::vulkan", log::Level::Error));
        // Sadece wgpu eşiği warn.
        assert!(s.izin("wgpu::core", log::Level::Warn));
    }

    #[test]
    fn kapsam_thread_local_iz_saglar() {
        assert!(gecerli_iz().is_none());
        let iz = TraceContext::kok();
        {
            let _k = kapsam(iz);
            assert_eq!(gecerli_iz().unwrap().trace_id, iz.trace_id);
            // İç içe kapsam.
            let ic = iz.cocuk();
            let _k2 = kapsam(ic);
            assert_eq!(gecerli_iz().unwrap().span_id, ic.span_id);
        }
        // Kapsamlar düştü → ambiyans temizlendi.
        assert!(gecerli_iz().is_none());
    }

    #[test]
    fn seviye_cevirisi() {
        assert_eq!(seviye_cevir(log::Level::Error), LogSeverity::Error);
        assert_eq!(seviye_cevir(log::Level::Info), LogSeverity::Info);
    }
}
