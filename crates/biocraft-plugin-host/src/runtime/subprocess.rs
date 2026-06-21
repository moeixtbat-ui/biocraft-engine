//! Tier-3 — **Python/R out-of-process köprüsü** (MK-02, İP-07).
//!
//! Eklenti **AYRI bir süreçte** çalışır — asla in-process değil (MK-02).  Böylece:
//! * eklenti çökerse çekirdeği düşürmez (izolasyon, MK-15),
//! * ağır/uzun iş arayüzü dondurmaz (ayrı süreç + zaman aşımı).
//!
//! **Kontrol kanalı:** satır-tabanlı JSON (host stdin'e bir istek satırı yazar,
//! eklenti stdout'a bir yanıt satırı yazar).  Bağımlılıksız, platformlar-arası, basit.
//! (Büyük veri taşıma = Arrow Flight, ileride — `MVP-sonrasi.md`.)
//!
//! **Kaynak sınırları (spec):** 2 GB RAM / %50 CPU / 30 s.  Zaman aşımı **gerçekten**
//! uygulanır (süre dolarsa süreç öldürülür).  RAM/CPU tavanı sürece ortam değişkeniyle
//! bildirilir (işbirlikçi kısma); **sert** OS-düzeyi sınır (Windows Job Object / Linux
//! cgroup) İP-09 sertleştirmesine bırakıldı (kanca).
//!
//! **Python yoksa in-process'e DÖNÜLMEZ** (MK-02): [`python_bul`] `None` dönerse host
//! kullanıcıya "Python'u kur" rehberi gösterir ve eklentiyi yüklemez.

use super::EklentiCalistirici;
use biocraft_ipc::{EklentiCagrisi, EklentiYaniti};
use biocraft_types::{CorrelationId, ErrorReport};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// Tier-3 alt-süreç kaynak sınırları (spec varsayılanları).
#[derive(Debug, Clone, Copy)]
pub struct AltSurecLimitleri {
    /// Çocuk sürecin RAM tavanı (bayt) — varsayılan 2 GiB.
    pub bellek_bayt: u64,
    /// CPU tavanı (%) — varsayılan 50.
    pub cpu_yuzde: u8,
    /// Tek çağrı zaman aşımı — varsayılan 30 s (aşılırsa süreç öldürülür).
    pub zaman_asimi: Duration,
    /// Host tarafı tampon/defter için orkestratörden rezerve edilecek bellek (bayt).
    /// Çocuğun RAM'i ayrı süreçtedir; bu yalnızca host-tarafı muhasebesidir (MK-22).
    pub host_rezervasyon: u64,
}

impl Default for AltSurecLimitleri {
    fn default() -> Self {
        Self {
            bellek_bayt: 2 * 1024 * 1024 * 1024, // 2 GiB
            cpu_yuzde: 50,
            zaman_asimi: Duration::from_secs(30),
            host_rezervasyon: 32 * 1024 * 1024, // 32 MiB
        }
    }
}

// ─── Satır protokolü (JSON) ───────────────────────────────────────────────────

/// Host → eklenti: tek bir fonksiyon çağrısı (JSON satırı).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KopruIstek {
    /// Çağrılacak fonksiyonun adı.
    pub fonksiyon: String,
}

/// Eklenti → host: çağrı sonucu (JSON satırı).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KopruYanit {
    /// İşlem başarılı mı?
    pub tamam: bool,
    /// Dönen sayısal değer (anlamı fonksiyona göre).
    #[serde(default)]
    pub donen: i64,
    /// Eklentinin günlüğe yazdığı satırlar.
    #[serde(default)]
    pub gunluk: Vec<String>,
    /// Başarısızsa hata açıklaması.
    #[serde(default)]
    pub hata: Option<String>,
}

// ─── Python keşfi ─────────────────────────────────────────────────────────────

/// Sistemde bir Python yorumlayıcısı arar (PATH üzerinde).
///
/// Bulunamazsa `None` — çağıran taraf **in-process'e dönmemeli**, "Python'u kur"
/// rehberi göstermelidir (MK-02).
pub fn python_bul() -> Option<PathBuf> {
    let adaylar: &[&str] = if cfg!(windows) {
        &["python.exe", "python3.exe", "py.exe"]
    } else {
        &["python3", "python"]
    };
    let path = std::env::var_os("PATH")?;
    dizinlerde_bul(std::env::split_paths(&path), adaylar)
}

/// Verilen dizinlerde aday yürütülebilirleri arar (test edilebilir çekirdek).
fn dizinlerde_bul(
    dizinler: impl IntoIterator<Item = PathBuf>,
    adaylar: &[&str],
) -> Option<PathBuf> {
    for dizin in dizinler {
        for ad in adaylar {
            let aday = dizin.join(ad);
            if aday.is_file() {
                return Some(aday);
            }
        }
    }
    None
}

/// Python bulunamadığında gösterilecek standart "kur" rehberi.
pub fn python_yok_hatasi() -> ErrorReport {
    ErrorReport::new(
        "Python bulunamadı",
        "Bu eklenti Python gerektiriyor ama sistemde bir Python yorumlayıcısı bulunamadı",
        "python.org adresinden Python 3'ü kurun ve PATH'e ekleyin, sonra yeniden deneyin",
    )
    .with_eylem("Python kurulum rehberi")
}

// ─── Alt-süreç çalıştırıcı ────────────────────────────────────────────────────

/// Bir Python/R eklentisini **ayrı süreçte** çalıştıran köprü (MK-02).
#[derive(Debug, Clone)]
pub struct AltSurecCalistirici {
    yorumlayici: PathBuf,
    betik: PathBuf,
    limitler: AltSurecLimitleri,
}

impl AltSurecCalistirici {
    /// Yorumlayıcı + betik yolu + limitlerle yeni bir köprü kurar.
    pub fn yeni(
        yorumlayici: impl Into<PathBuf>,
        betik: impl Into<PathBuf>,
        limitler: AltSurecLimitleri,
    ) -> Self {
        Self {
            yorumlayici: yorumlayici.into(),
            betik: betik.into(),
            limitler,
        }
    }

    /// Uygulanan limitler.
    pub fn limitler(&self) -> &AltSurecLimitleri {
        &self.limitler
    }

    /// Tek bir çağrıyı ayrı süreçte çalıştırır; zaman aşımını uygular.
    fn calistir(&self, istek: &KopruIstek, kid: CorrelationId) -> EklentiYaniti {
        let hata = |ne: &str, neden: String, detay: Option<String>| {
            let mut r = ErrorReport::new(
                ne,
                neden,
                "Eklenti günlüğünü inceleyin veya yeniden deneyin",
            )
            .with_correlation_id(kid);
            if let Some(d) = detay {
                r = r.with_teknik_detay(d);
            }
            EklentiYaniti::Hata(r)
        };

        let istek_satiri = match serde_json::to_string(istek) {
            Ok(s) => s,
            Err(e) => {
                return hata(
                    "İç hata",
                    "istek serileştirilemedi".into(),
                    Some(e.to_string()),
                )
            }
        };

        // Ayrı süreç başlat (MK-02 — asla in-process).
        let mut cocuk = match Command::new(&self.yorumlayici)
            .arg(&self.betik)
            // Kaynak tavanını işbirlikçi kısma için ortamla bildir.
            .env("BIOCRAFT_MEM_LIMIT", self.limitler.bellek_bayt.to_string())
            .env("BIOCRAFT_CPU_LIMIT", self.limitler.cpu_yuzde.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                // Süreç başlatılamadı; in-process'e DÖNME — net hata.
                return hata(
                    "Python eklentisi başlatılamadı",
                    format!("'{}' süreci başlatılamadı", self.yorumlayici.display()),
                    Some(e.to_string()),
                );
            }
        };

        // İstek satırını stdin'e yaz ve kapat (eklenti tek satır okur).
        if let Some(mut stdin) = cocuk.stdin.take() {
            let _ = writeln!(stdin, "{istek_satiri}");
            // stdin burada düşer → kapanır.
        }

        // stdout'u ayrı bir thread'de oku (zaman aşımı için ana thread bloklanmaz).
        let stdout = cocuk.stdout.take();
        let (gonder, al) = mpsc::channel::<Option<String>>();
        if let Some(stdout) = stdout {
            std::thread::spawn(move || {
                let mut okuyucu = BufReader::new(stdout);
                let mut satir = String::new();
                let sonuc = match okuyucu.read_line(&mut satir) {
                    Ok(0) => None,        // EOF, yanıt yok
                    Ok(_) => Some(satir), // bir satır geldi
                    Err(_) => None,
                };
                let _ = gonder.send(sonuc);
            });
        }

        // Zaman aşımıyla yanıtı bekle.
        let yanit_satiri = match al.recv_timeout(self.limitler.zaman_asimi) {
            Ok(Some(s)) => {
                // Yanıt alındı; süreç hâlâ yaşıyorsa temiz kapat.
                let _ = cocuk.kill();
                let _ = cocuk.wait();
                s
            }
            Ok(None) => {
                // Süreç yanıt vermeden bitti (çökme) — yalıtıldı.
                let _ = cocuk.kill();
                let stderr = self.stderr_oku(&mut cocuk);
                return hata(
                    "Python eklentisi çöktü",
                    "eklenti süreci bir yanıt üretmeden sonlandı (çekirdek etkilenmedi)".into(),
                    stderr,
                );
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Zaman aşımı → süreci öldür (arayüz donmaz).
                let _ = cocuk.kill();
                let _ = cocuk.wait();
                return hata(
                    "Python eklentisi zaman aşımına uğradı",
                    format!(
                        "eklenti {} sn içinde yanıt vermedi ve durduruldu",
                        self.limitler.zaman_asimi.as_secs()
                    ),
                    None,
                );
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = cocuk.kill();
                let _ = cocuk.wait();
                return hata(
                    "Python eklentisi yanıt vermedi",
                    "eklenti süreciyle iletişim koptu".into(),
                    None,
                );
            }
        };

        // Yanıtı çöz.
        match serde_json::from_str::<KopruYanit>(yanit_satiri.trim()) {
            Ok(y) if y.tamam => EklentiYaniti::Basari {
                donen: y.donen,
                gunluk: y.gunluk,
            },
            Ok(y) => hata(
                "Python eklentisi hata döndürdü",
                y.hata.unwrap_or_else(|| "eklenti bir hata bildirdi".into()),
                None,
            ),
            Err(e) => hata(
                "Python eklentisinden geçersiz yanıt",
                "eklenti beklenen JSON satırını üretmedi".into(),
                Some(format!("{e}: {}", yanit_satiri.trim())),
            ),
        }
    }

    /// Çocuk sürecin stderr'ini (varsa) okur — teşhis için.
    fn stderr_oku(&self, cocuk: &mut std::process::Child) -> Option<String> {
        let mut s = String::new();
        if let Some(mut err) = cocuk.stderr.take() {
            let _ = err.read_to_string(&mut s);
        }
        let _ = cocuk.wait();
        if s.trim().is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

impl EklentiCalistirici for AltSurecCalistirici {
    fn cagir(&self, cagri: EklentiCagrisi) -> EklentiYaniti {
        let kid = cagri.correlation_id;
        let istek = KopruIstek {
            fonksiyon: cagri.fonksiyon,
        };
        self.calistir(&istek, kid)
    }
}

/// Bir betik yolunun var olup okunabilir olduğunu doğrular (yükleme öncesi).
pub fn betik_dogrula(betik: &Path) -> Result<(), ErrorReport> {
    if betik.is_file() {
        Ok(())
    } else {
        Err(ErrorReport::new(
            "Eklenti betiği bulunamadı",
            format!("'{}' bir dosya değil", betik.display()),
            "Eklenti paketinin eksiksiz kurulduğundan emin olun",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_varsayilanlari_spec_ile_uyumlu() {
        let l = AltSurecLimitleri::default();
        assert_eq!(l.bellek_bayt, 2 * 1024 * 1024 * 1024); // 2 GiB
        assert_eq!(l.cpu_yuzde, 50);
        assert_eq!(l.zaman_asimi, Duration::from_secs(30));
    }

    #[test]
    fn protokol_serde_gidis_donus() {
        let istek = KopruIstek {
            fonksiyon: "merhaba".into(),
        };
        let s = serde_json::to_string(&istek).unwrap();
        let geri: KopruIstek = serde_json::from_str(&s).unwrap();
        assert_eq!(geri.fonksiyon, "merhaba");

        let yanit = KopruYanit {
            tamam: true,
            donen: 16,
            gunluk: vec!["Merhaba".into()],
            hata: None,
        };
        let s = serde_json::to_string(&yanit).unwrap();
        let geri: KopruYanit = serde_json::from_str(&s).unwrap();
        assert!(geri.tamam);
        assert_eq!(geri.donen, 16);
    }

    #[test]
    fn yanit_eksik_alanlar_varsayilan() {
        // Eklenti yalnızca {"tamam": true} dönerse donen=0, gunluk=[].
        let y: KopruYanit = serde_json::from_str(r#"{"tamam": true}"#).unwrap();
        assert!(y.tamam);
        assert_eq!(y.donen, 0);
        assert!(y.gunluk.is_empty());
        assert!(y.hata.is_none());
    }

    #[test]
    fn dizinlerde_bul_dosyayi_bulur() {
        let dizin = std::env::temp_dir().join(format!("biocraft_pybul_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dizin);
        let sahte = dizin.join("python3");
        std::fs::write(&sahte, b"#!/bin/sh\n").unwrap();
        let bulunan = dizinlerde_bul([dizin.clone()], &["python3", "python"]);
        assert_eq!(bulunan, Some(sahte));
        // Olmayan ad → None.
        assert!(dizinlerde_bul([dizin.clone()], &["kesinlikle_yok_xyz"]).is_none());
        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn betik_yoksa_hata() {
        assert!(betik_dogrula(Path::new("___olmayan_betik___.py")).is_err());
    }

    #[test]
    fn baslatilamayan_yorumlayici_in_process_donmez() {
        // Var olmayan yorumlayıcı → net Hata (asla panik/in-process).
        let c = AltSurecCalistirici::yeni(
            "___kesinlikle_yok_yorumlayici___",
            "betik.py",
            AltSurecLimitleri::default(),
        );
        let yanit = c.cagir(EklentiCagrisi::yeni("merhaba"));
        match yanit {
            EklentiYaniti::Hata(r) => assert_eq!(r.ne_oldu, "Python eklentisi başlatılamadı"),
            diger => panic!("Hata bekleniyordu: {diger:?}"),
        }
    }

    /// Python varsa: örnek betik AYRI süreçte çalışır, sonuç doğru döner.
    /// Python yoksa test atlanır (CI'da Python olmasa bile yeşil kalır).
    #[test]
    fn python_varsa_ayri_surecte_calisir() {
        let Some(python) = python_bul() else {
            eprintln!("Python bulunamadı → test atlandı");
            return;
        };
        // Protokolü uygulayan minimal bir betik yaz.
        let dizin = std::env::temp_dir().join(format!("biocraft_pytest_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dizin);
        let betik = dizin.join("eklenti.py");
        std::fs::write(
            &betik,
            r#"
import sys, json, os
satir = sys.stdin.readline()
istek = json.loads(satir)
yanit = {"tamam": True, "donen": 16,
         "gunluk": ["ayri surecte calisti pid=%d" % os.getpid()], "hata": None}
sys.stdout.write(json.dumps(yanit) + "\n")
sys.stdout.flush()
"#,
        )
        .unwrap();

        let c = AltSurecCalistirici::yeni(python, &betik, AltSurecLimitleri::default());
        let benim_pid = std::process::id();
        match c.cagir(EklentiCagrisi::yeni("merhaba")) {
            EklentiYaniti::Basari { donen, gunluk } => {
                assert_eq!(donen, 16);
                assert!(gunluk.iter().any(|s| s.contains("ayri surecte")));
                // Eklentinin PID'i host'unkinden farklı → gerçekten ayrı süreç (MK-02).
                let pid_satiri = gunluk.iter().find(|s| s.contains("pid=")).unwrap();
                assert!(!pid_satiri.contains(&format!("pid={benim_pid}")));
            }
            diger => panic!("Başarı bekleniyordu: {diger:?}"),
        }
        let _ = std::fs::remove_dir_all(&dizin);
    }
}
