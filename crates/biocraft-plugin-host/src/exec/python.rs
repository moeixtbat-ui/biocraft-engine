//! Out-of-process Python çalıştırma motoru (İP-06, MK-02).
//!
//! [`calistir_baslat`] kullanıcı kodunu **ayrı bir Python sürecinde** başlatır ve hemen bir
//! [`CalismaTutamac`] döner — **çağrı bloklamaz** (sonsuz döngü bile olsa anında döner).
//! Arayüz her karede [`CalismaTutamac::dene`] ile çıktı olaylarını **bloklamadan** toplar
//! (MK-07 kare bütçesi); böylece kötü kod arayüzü dondurmaz.  Kullanıcı durdurmak isterse
//! [`CalismaTutamac::durdur`] süreci **öldürür** (kill).
//!
//! ### Süreç yaşam döngüsü (üç iş parçacığı)
//! 1. **stdout okuyucu** — satırları [`CalismaOlay::Stdout`] olarak kanala yollar.
//! 2. **stderr okuyucu** — satırları [`CalismaOlay::Stderr`] olarak kanala yollar.
//! 3. **gözlemci** — durdur bayrağı / zaman aşımı / süreç çıkışını izler; **okuyucuları
//!    join eder** (tüm çıktı kanala düşsün diye), sonra **bitiş** olayını (`Bitti`/
//!    `Durduruldu`/`ZamanAsimi`) yollar.  Bu sıralama, çıktının bitiş olayından **önce**
//!    görünmesini garanti eder.

use crate::runtime::subprocess::python_yok_hatasi;
use biocraft_types::ErrorReport;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Çalıştırma kipi — ikisi de **ayrı süreçte** çalışır; fark anlamsaldır (UI/teşhis).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalismaModu {
    /// Tüm betiği baştan sona çalıştır.
    TamScript,
    /// Tek bir hücreyi (seçili kod parçası) çalıştır — Jupyter benzeri.
    Hucre,
}

impl CalismaModu {
    /// İnsan-okur kısa ad (teşhis/log).
    pub fn ad(self) -> &'static str {
        match self {
            CalismaModu::TamScript => "tam-script",
            CalismaModu::Hucre => "hücre",
        }
    }
}

/// Kod çalıştırma kaynak sınırları.
///
/// `bellek_bayt`/`cpu_yuzde` çocuğa ortam değişkeniyle bildirilen **işbirlikçi** tavandır;
/// **sert** OS-düzeyi sınır (Windows Job Object / Linux cgroup) İP-09 sertleştirme kancasıdır.
/// `zaman_asimi` ise gerçekten uygulanır (aşılırsa süreç öldürülür); `None` = otomatik sınır
/// yok (yalnız kullanıcı "Durdur" ile bitirir — uzun analiz işleri için).
#[derive(Debug, Clone, Copy)]
pub struct KodCalismaLimitleri {
    /// RAM tavanı (bayt) — işbirlikçi (ortam değişkeni).  Varsayılan 2 GiB.
    pub bellek_bayt: u64,
    /// CPU tavanı (%) — işbirlikçi.  Varsayılan 50.
    pub cpu_yuzde: u8,
    /// Üst zaman sınırı; aşılırsa süreç öldürülür.  Varsayılan 120 sn.
    pub zaman_asimi: Option<Duration>,
}

impl Default for KodCalismaLimitleri {
    fn default() -> Self {
        Self {
            bellek_bayt: 2 * 1024 * 1024 * 1024, // 2 GiB
            cpu_yuzde: 50,
            zaman_asimi: Some(Duration::from_secs(120)),
        }
    }
}

/// Çalışan süreçten arayüze akan tek bir olay.
#[derive(Debug, Clone)]
pub enum CalismaOlay {
    /// Standart çıkıştan bir satır.
    Stdout(String),
    /// Standart hatadan bir satır.
    Stderr(String),
    /// Süreç normal sonlandı (çıkış kodu; sinyalle ölürse `None`).
    Bitti { cikis_kodu: Option<i32> },
    /// Kullanıcı "Durdur" ile sonlandırdı.
    Durduruldu,
    /// Zaman aşımı tavanı aşıldı; süreç öldürüldü.
    ZamanAsimi,
    /// Süreç başlatılamadı / iletişim koptu (standart hata şeması).
    Hata(ErrorReport),
}

impl CalismaOlay {
    /// Bu olay çalışmanın **bitişini** mi imler (Bitti/Durduruldu/ZamanAsimi/Hata)?
    pub fn bitis_mi(&self) -> bool {
        matches!(
            self,
            CalismaOlay::Bitti { .. }
                | CalismaOlay::Durduruldu
                | CalismaOlay::ZamanAsimi
                | CalismaOlay::Hata(_)
        )
    }
}

/// Çalışan bir koda erişim tutamacı: olayları **bloklamadan** topla, istersen **durdur**.
pub struct CalismaTutamac {
    olaylar: Receiver<CalismaOlay>,
    durdur_bayrak: Arc<AtomicBool>,
    cocuk: Arc<Mutex<Option<Child>>>,
}

impl CalismaTutamac {
    /// Bir sonraki olayı **bloklamadan** dener (arayüz her karede çağırır → DONMAZ).
    pub fn dene(&self) -> Result<CalismaOlay, TryRecvError> {
        self.olaylar.try_recv()
    }

    /// Bekleyen tüm olayları bloklamadan boşaltır (UI'nin pratik kullanımı).
    pub fn tumunu_dene(&self) -> Vec<CalismaOlay> {
        let mut v = Vec::new();
        while let Ok(o) = self.olaylar.try_recv() {
            v.push(o);
        }
        v
    }

    /// Çalışmayı **durdurur** (süreci öldürür).  "Durdur" düğmesi bunu çağırır.
    ///
    /// Birden çok kez çağrılması zararsızdır; gözlemci iş parçacığı `Durduruldu` yollar.
    pub fn durdur(&self) {
        self.durdur_bayrak.store(true, Ordering::SeqCst);
        if let Ok(mut k) = self.cocuk.lock() {
            if let Some(child) = k.as_mut() {
                let _ = child.kill();
            }
        }
    }
}

impl Drop for CalismaTutamac {
    /// Tutamacın bırakılması = artık dinlenmiyor → süreç asılı kalmasın diye öldürülür.
    fn drop(&mut self) {
        self.durdur();
    }
}

/// Sürece ayrılan benzersiz geçici dizinler için sayaç (eşzamanlı çalıştırmalar çakışmasın).
static SAYAC: AtomicU64 = AtomicU64::new(0);

/// Kullanıcı kodunu **ayrı süreçte** başlatır; hemen bir tutamaç döner (bloklamaz).
///
/// Python keşfedilemezse **in-process'e DÖNÜLMEZ** (MK-02): standart "Python'u kur" hatası döner.
pub fn calistir_baslat(
    kod: &str,
    modu: CalismaModu,
    limitler: KodCalismaLimitleri,
) -> Result<CalismaTutamac, ErrorReport> {
    let python = crate::runtime::subprocess::python_bul().ok_or_else(python_yok_hatasi)?;
    calistir_baslat_ile(&python, kod, modu, limitler)
}

/// Belirli bir yorumlayıcıyla başlatır (test + ileride proje sanal ortamı — Gün 23).
pub fn calistir_baslat_ile(
    yorumlayici: &Path,
    kod: &str,
    _modu: CalismaModu,
    limitler: KodCalismaLimitleri,
) -> Result<CalismaTutamac, ErrorReport> {
    // 1) Kodu benzersiz bir geçici dizine yaz (arg uzunluğu/kaçış sorunlarından kaçınılır).
    let n = SAYAC.fetch_add(1, Ordering::Relaxed);
    let dizin = std::env::temp_dir().join(format!("biocraft_kod_{}_{}", std::process::id(), n));
    std::fs::create_dir_all(&dizin)
        .map_err(|e| io_hata("Geçici çalışma alanı oluşturulamadı", e))?;
    let betik = dizin.join("hucre.py");
    if let Err(e) = std::fs::write(&betik, kod) {
        let _ = std::fs::remove_dir_all(&dizin);
        return Err(io_hata("Kod geçici dosyaya yazılamadı", e));
    }

    // 2) Ayrı süreç başlat — `-u` = tamponsuz çıktı (satırlar anında akar; donmuş izlenim olmaz).
    //    stdin = null → kod stdin'den okursa asılı kalmaz.
    let mut komut = Command::new(yorumlayici);
    komut
        .arg("-u")
        .arg(&betik)
        .env("BIOCRAFT_MEM_LIMIT", limitler.bellek_bayt.to_string())
        .env("BIOCRAFT_CPU_LIMIT", limitler.cpu_yuzde.to_string())
        // Çıktıyı UTF-8'e zorla (özellikle Windows'ta boru kodlaması cp1252 olabilir →
        // Türkçe/Unicode çıktı bozulmasın).  Okuyucu yine de kayıpsız-decode eder (sağlamlık).
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match komut.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&dizin);
            return Err(ErrorReport::new(
                "Kod çalıştırılamadı",
                format!("'{}' süreci başlatılamadı", yorumlayici.display()),
                "Python kurulumunu doğrulayın; sorun sürerse yeniden deneyin",
            )
            .with_teknik_detay(e.to_string()));
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let cocuk = Arc::new(Mutex::new(Some(child)));
    let durdur_bayrak = Arc::new(AtomicBool::new(false));
    let (gonder, al) = mpsc::channel::<CalismaOlay>();

    // 3) stdout/stderr okuyucu iş parçacıkları — satır satır kanala yollar.
    let h_out = stdout.map(|s| {
        let g = gonder.clone();
        std::thread::spawn(move || oku_yolla(s, g, false))
    });
    let h_err = stderr.map(|s| {
        let g = gonder.clone();
        std::thread::spawn(move || oku_yolla(s, g, true))
    });

    // 4) Gözlemci iş parçacığı — durdur/zaman aşımı/çıkış izler, okuyucuları join eder, biter.
    let cocuk_g = Arc::clone(&cocuk);
    let bayrak_g = Arc::clone(&durdur_bayrak);
    let zaman_asimi = limitler.zaman_asimi;
    std::thread::spawn(move || {
        let baslangic = Instant::now();
        let bitis_olayi = loop {
            // a) Kullanıcı durdurdu mu?
            if bayrak_g.load(Ordering::SeqCst) {
                cocugu_oldur(&cocuk_g);
                break CalismaOlay::Durduruldu;
            }
            // b) Zaman aşımı?
            if let Some(zt) = zaman_asimi {
                if baslangic.elapsed() >= zt {
                    cocugu_oldur(&cocuk_g);
                    break CalismaOlay::ZamanAsimi;
                }
            }
            // c) Süreç kendiliğinden bitti mi? (reaped)
            let cikis = {
                match cocuk_g.lock() {
                    Ok(mut k) => match k.as_mut() {
                        Some(c) => c.try_wait().ok().flatten(),
                        None => None,
                    },
                    Err(_) => None,
                }
            };
            if let Some(durum) = cikis {
                break CalismaOlay::Bitti {
                    cikis_kodu: durum.code(),
                };
            }
            std::thread::sleep(Duration::from_millis(15));
        };

        // Tüm çıktının kanala düşmesini bekle (boruların kapanması = okuyucu EOF).
        if let Some(h) = h_out {
            let _ = h.join();
        }
        if let Some(h) = h_err {
            let _ = h.join();
        }
        // Geçici dizini temizle (kod artık çalışmıyor).
        if let Ok(mut k) = cocuk_g.lock() {
            if let Some(mut c) = k.take() {
                let _ = c.wait();
            }
        }
        let _ = gonder.send(bitis_olayi);
    });

    Ok(CalismaTutamac {
        olaylar: al,
        durdur_bayrak,
        cocuk,
    })
}

/// Bir borudan satırları okuyup uygun olayla kanala yollar (okuyucu iş parçacığı gövdesi).
///
/// **Kayıpsız decode:** ham bayt okunur ve `from_utf8_lossy` ile çözülür — kullanıcı kodu
/// geçersiz UTF-8 (ör. Windows cp1252) yazsa bile satır DÜŞMEZ (yalnız bozuk bayt `�` olur).
fn oku_yolla<R: std::io::Read>(boru: R, gonder: mpsc::Sender<CalismaOlay>, hata_akisi: bool) {
    let mut okuyucu = BufReader::new(boru);
    let mut tampon: Vec<u8> = Vec::new();
    loop {
        tampon.clear();
        match okuyucu.read_until(b'\n', &mut tampon) {
            Ok(0) => break, // EOF
            Ok(_) => {
                while matches!(tampon.last(), Some(b'\n') | Some(b'\r')) {
                    tampon.pop();
                }
                let temiz = String::from_utf8_lossy(&tampon).into_owned();
                let olay = if hata_akisi {
                    CalismaOlay::Stderr(temiz)
                } else {
                    CalismaOlay::Stdout(temiz)
                };
                if gonder.send(olay).is_err() {
                    break; // dinleyici gitti
                }
            }
            Err(_) => break,
        }
    }
}

/// Çocuğu (varsa) öldürür — kilidi kısa tutar.
fn cocugu_oldur(cocuk: &Arc<Mutex<Option<Child>>>) {
    if let Ok(mut k) = cocuk.lock() {
        if let Some(c) = k.as_mut() {
            let _ = c.kill();
        }
    }
}

/// IO hatasını standart şemaya çevirir.
fn io_hata(ne: &str, e: std::io::Error) -> ErrorReport {
    ErrorReport::new(
        ne,
        "dosya/dizin işlemi başarısız",
        "Disk alanını ve geçici klasör iznini kontrol edin",
    )
    .with_teknik_detay(e.to_string())
}

/// Var olmayan bir yorumlayıcı yolu — test/teşhis için (asla in-process'e dönmez).
pub fn gecersiz_yorumlayici() -> PathBuf {
    PathBuf::from("___biocraft_olmayan_python___")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::subprocess::python_bul;

    /// Bir tutamaçtan, bir bitiş olayı görene kadar (veya zaman aşımına dek) olayları toplar.
    fn topla(tutamac: &CalismaTutamac, azami: Duration) -> Vec<CalismaOlay> {
        let baslangic = Instant::now();
        let mut hepsi = Vec::new();
        loop {
            match tutamac.dene() {
                Ok(o) => {
                    let bitti = o.bitis_mi();
                    hepsi.push(o);
                    if bitti {
                        break;
                    }
                }
                Err(TryRecvError::Empty) => {
                    if baslangic.elapsed() > azami {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(TryRecvError::Disconnected) => break,
            }
        }
        hepsi
    }

    #[test]
    fn limit_varsayilanlari_makul() {
        let l = KodCalismaLimitleri::default();
        assert_eq!(l.bellek_bayt, 2 * 1024 * 1024 * 1024);
        assert_eq!(l.cpu_yuzde, 50);
        assert_eq!(l.zaman_asimi, Some(Duration::from_secs(120)));
    }

    #[test]
    fn gecersiz_yorumlayici_in_process_donmez() {
        // Var olmayan yorumlayıcı → net Hata (asla panik/in-process — MK-02).
        let sonuc = calistir_baslat_ile(
            &gecersiz_yorumlayici(),
            "print(1)",
            CalismaModu::TamScript,
            KodCalismaLimitleri::default(),
        );
        match sonuc {
            Err(r) => assert_eq!(r.ne_oldu, "Kod çalıştırılamadı"),
            Ok(_) => panic!("Hata bekleniyordu"),
        }
    }

    #[test]
    fn print_ciktisi_akar_ve_biter() {
        let Some(py) = python_bul() else {
            eprintln!("Python yok → test atlandı");
            return;
        };
        let t = calistir_baslat_ile(
            &py,
            "print('merhaba bio')",
            CalismaModu::TamScript,
            KodCalismaLimitleri::default(),
        )
        .unwrap();
        let olaylar = topla(&t, Duration::from_secs(20));
        assert!(
            olaylar
                .iter()
                .any(|o| matches!(o, CalismaOlay::Stdout(s) if s.contains("merhaba bio"))),
            "stdout'ta 'merhaba bio' bekleniyordu: {olaylar:?}"
        );
        assert!(
            matches!(
                olaylar.last(),
                Some(CalismaOlay::Bitti {
                    cikis_kodu: Some(0)
                })
            ),
            "son olay Bitti(0) olmalı: {olaylar:?}"
        );
    }

    #[test]
    fn hatali_kod_stderr_ve_sifirsiz_cikis() {
        let Some(py) = python_bul() else {
            return;
        };
        let t = calistir_baslat_ile(
            &py,
            "import sys\nsys.exit(3)\n",
            CalismaModu::TamScript,
            KodCalismaLimitleri::default(),
        )
        .unwrap();
        let olaylar = topla(&t, Duration::from_secs(20));
        assert!(
            matches!(
                olaylar.last(),
                Some(CalismaOlay::Bitti {
                    cikis_kodu: Some(3)
                })
            ),
            "çıkış kodu 3 olmalı: {olaylar:?}"
        );
    }

    #[test]
    fn sonsuz_dongu_bloklamaz_ve_durdurulabilir() {
        let Some(py) = python_bul() else {
            return;
        };
        // KRİTİK kabul kriteri: sonsuz döngü arayüzü dondurmaz.
        let basla = Instant::now();
        let t = calistir_baslat_ile(
            &py,
            "while True:\n    pass\n",
            CalismaModu::TamScript,
            KodCalismaLimitleri {
                zaman_asimi: None, // yalnız "Durdur" ile bitecek
                ..Default::default()
            },
        )
        .unwrap();
        // calistir_baslat ANINDA döndü (sonsuz döngüye rağmen) → arayüz bloklanmaz.
        assert!(
            basla.elapsed() < Duration::from_secs(3),
            "başlatma bloklamamalı (arayüz donmaz)"
        );
        // Biraz çalışsın, sonra durdur.
        std::thread::sleep(Duration::from_millis(150));
        t.durdur();
        let olaylar = topla(&t, Duration::from_secs(10));
        assert!(
            matches!(olaylar.last(), Some(CalismaOlay::Durduruldu)),
            "durdurma sonrası son olay Durduruldu olmalı: {olaylar:?}"
        );
    }

    #[test]
    fn zaman_asimi_uygulanir() {
        let Some(py) = python_bul() else {
            return;
        };
        let t = calistir_baslat_ile(
            &py,
            "while True:\n    pass\n",
            CalismaModu::TamScript,
            KodCalismaLimitleri {
                zaman_asimi: Some(Duration::from_millis(300)),
                ..Default::default()
            },
        )
        .unwrap();
        let olaylar = topla(&t, Duration::from_secs(10));
        assert!(
            matches!(olaylar.last(), Some(CalismaOlay::ZamanAsimi)),
            "zaman aşımıyla bitmeli: {olaylar:?}"
        );
    }
}
