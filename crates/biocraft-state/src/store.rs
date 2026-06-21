//! Kalıcı depo (durable store) — MK-37 "tek mantıksal depo" + atomik yazma + BLAKE3 bütünlük.
//!
//! Durum altyapısının dayanıklılığı buradan gelir.  Üç güvence:
//! - **Atomik yazma (MK-28 kural 1):** Önce geçici dosyaya yazılır + `fsync` edilir, sonra hedefin
//!   üzerine **atomik yer değiştirme** (`rename`) yapılır.  Yazma yarıda kesilse bile (güç gitse,
//!   süreç öldürülse) hedef dosya ya eski tam hâliyle ya yeni tam hâliyle kalır — **asla yarım**.
//! - **Bütünlük (BLAKE3):** Her kaydın başına yükün BLAKE3 özeti yazılır.  Okumada özet yeniden
//!   hesaplanıp karşılaştırılır; bit bozulması/yarım dosya tespit edilir (sessiz okuma yok).
//! - **Tek mantıksal depo (MK-37):** Her `yaz` çağrısı TEK bir anahtara (tek dosyaya) dokunur.
//!   "Çok-depoda tek atomik işlem" (saga/2PC) **vaat edilmez**; tutarlılık tek depo sınırında garanti.
//!
//! Depo, gerçek konumu (klasör) bilmez; çağıran verir.  Böylece çekirdek mantık platformdan
//! bağımsız ve testte geçici klasörle tamamen denetlenebilir kalır.  İleride aynı `KaliciDepo`
//! sözleşmesinin arkasına SQLite/RocksDB tabanlı bir depo da konabilir (spec teknolojisi) —
//! MVP için atomik-dosya deposu tüm kabul kriterlerini (kalıcılık/atomiklik/bütünlük) karşılar.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use biocraft_types::ErrorReport;

/// Bütünlük zarfının başlık etiketi (sürüm 1).  Format: `BCS1 <64-hex-blake3>\n<yük baytları>`.
const BASLIK_ETIKET: &[u8] = b"BCS1 ";
/// BLAKE3 özetinin onaltılık (hex) uzunluğu.
const HEX_UZUNLUK: usize = 64;

/// Aynı süreçte üretilen geçici dosya adlarının benzersizliğini garanti eden sayaç.
static GECICI_SAYAC: AtomicU64 = AtomicU64::new(0);

/// Bir anahtar→bayt kalıcı deposunun sözleşmesi.
///
/// MK-37: Her metot TEK mantıksal depoya dokunur; birden çok anahtarı tek atomik işlemde
/// birleştirme **garantisi verilmez**.  Yazma atomik ve bütünlük-denetimlidir.
pub trait KaliciDepo {
    /// `anahtar` altına `veri`'yi atomik ve bütünlük-mühürlü olarak yazar (üzerine yazar).
    fn yaz(&self, anahtar: &str, veri: &[u8]) -> Result<(), ErrorReport>;

    /// `anahtar`'ı okur.  Yoksa `Ok(None)`; bozuksa `Err` (sessizce yutmaz).
    fn oku(&self, anahtar: &str) -> Result<Option<Vec<u8>>, ErrorReport>;

    /// `anahtar`'ı siler (yoksa sorun değil).
    fn sil(&self, anahtar: &str) -> Result<(), ErrorReport>;

    /// `anahtar` için bir kayıt dosyası mevcut mu (içeriği doğrulamaz).
    fn var_mi(&self, anahtar: &str) -> bool;
}

/// Disk üzerinde atomik-dosya tabanlı `KaliciDepo`.  Her anahtar `<kok>/<anahtar>.bcs` dosyasıdır.
pub struct DosyaDepo {
    kok: PathBuf,
}

impl DosyaDepo {
    /// Verilen kök klasörü kullanan bir depo açar; klasörü (yoksa) oluşturmaya çalışır.
    ///
    /// Klasör burada oluşturulamazsa hata bastırılmaz; ilk `yaz`'da anlaşılır `ErrorReport` döner.
    pub fn yeni(kok: impl Into<PathBuf>) -> Self {
        let kok = kok.into();
        let _ = fs::create_dir_all(&kok); // en iyi çaba; gerçek hata yazma anında raporlanır.
        Self { kok }
    }

    /// Deponun kök klasörü.
    pub fn kok(&self) -> &Path {
        &self.kok
    }

    /// Bir anahtarın dosya yolunu üretir.  Anahtarlar iç sabitlerdir; yine de yol ayıracı
    /// içeren bir anahtar alt klasöre kaçamasın diye sadeleştirilir.
    fn dosya_yolu(&self, anahtar: &str) -> PathBuf {
        let guvenli: String = anahtar
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.kok.join(format!("{guvenli}.bcs"))
    }
}

impl KaliciDepo for DosyaDepo {
    fn yaz(&self, anahtar: &str, veri: &[u8]) -> Result<(), ErrorReport> {
        if let Err(e) = fs::create_dir_all(&self.kok) {
            return Err(io_hatasi("Durum klasörü oluşturulamadı", &self.kok, &e));
        }
        let hedef = self.dosya_yolu(anahtar);

        // Bütünlük zarfı: "BCS1 <hex>\n" + ham yük.
        let ozet = blake3::hash(veri);
        let mut icerik = Vec::with_capacity(BASLIK_ETIKET.len() + HEX_UZUNLUK + 1 + veri.len());
        icerik.extend_from_slice(BASLIK_ETIKET);
        icerik.extend_from_slice(ozet.to_hex().as_bytes());
        icerik.push(b'\n');
        icerik.extend_from_slice(veri);

        atomik_yaz(&hedef, &icerik)
    }

    fn oku(&self, anahtar: &str) -> Result<Option<Vec<u8>>, ErrorReport> {
        let hedef = self.dosya_yolu(anahtar);
        let ham = match fs::read(&hedef) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io_hatasi("Durum dosyası okunamadı", &hedef, &e)),
        };
        zarf_coz(&ham, &hedef).map(Some)
    }

    fn sil(&self, anahtar: &str) -> Result<(), ErrorReport> {
        let hedef = self.dosya_yolu(anahtar);
        match fs::remove_file(&hedef) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_hatasi("Durum dosyası silinemedi", &hedef, &e)),
        }
    }

    fn var_mi(&self, anahtar: &str) -> bool {
        self.dosya_yolu(anahtar).exists()
    }
}

/// Atomik dosya yazımı: geçici dosyaya yaz + `fsync` + hedefin üzerine `rename`.
///
/// `rename` aynı klasör (aynı dosya sistemi) içinde atomik yer değiştirmedir (Windows'ta da
/// `MoveFileEx` ile mevcut dosyanın üzerine yazar).  Böylece okuyucu ya eski ya yeni tam içeriği
/// görür; **yarım yazılmış dosya hiçbir zaman hedefte görünmez**.
fn atomik_yaz(hedef: &Path, icerik: &[u8]) -> Result<(), ErrorReport> {
    let pid = std::process::id();
    let n = GECICI_SAYAC.fetch_add(1, Ordering::Relaxed);
    let temel = hedef
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "durum".to_string());
    let gecici = hedef.with_file_name(format!("{temel}.tmp-{pid}-{n}"));

    // 1) Geçici dosyaya tam içeriği yaz ve diske zorla (fsync).
    {
        let mut f = match fs::File::create(&gecici) {
            Ok(f) => f,
            Err(e) => {
                return Err(io_hatasi(
                    "Geçici durum dosyası oluşturulamadı",
                    &gecici,
                    &e,
                ))
            }
        };
        if let Err(e) = f.write_all(icerik) {
            let _ = fs::remove_file(&gecici);
            return Err(io_hatasi("Durum diske yazılamadı", &gecici, &e));
        }
        if let Err(e) = f.sync_all() {
            let _ = fs::remove_file(&gecici);
            return Err(io_hatasi("Durum diske işlenemedi (fsync)", &gecici, &e));
        }
    }

    // 2) Atomik yer değiştirme.  Başarısız olursa geçici dosyayı temizle (çöp bırakma).
    if let Err(e) = fs::rename(&gecici, hedef) {
        let _ = fs::remove_file(&gecici);
        return Err(io_hatasi("Durum dosyası yerine konamadı", hedef, &e));
    }
    Ok(())
}

/// Bütünlük zarfını çözer ve BLAKE3 özetini doğrular.  Bozuksa anlaşılır `ErrorReport` döner.
fn zarf_coz(ham: &[u8], hedef: &Path) -> Result<Vec<u8>, ErrorReport> {
    // Başlık etiketi + en az hex + yeni satır olmalı.
    let bozuk = || bozulma_hatasi(hedef);

    if !ham.starts_with(BASLIK_ETIKET) {
        return Err(bozuk());
    }
    let govde = &ham[BASLIK_ETIKET.len()..];
    let satir_sonu = govde.iter().position(|&b| b == b'\n').ok_or_else(bozuk)?;
    if satir_sonu != HEX_UZUNLUK {
        return Err(bozuk());
    }
    let hex = std::str::from_utf8(&govde[..satir_sonu]).map_err(|_| bozuk())?;
    let yuk = govde[satir_sonu + 1..].to_vec();

    // Özet yeniden hesaplanıp dosyadakiyle karşılaştırılır.
    let beklenen = blake3::hash(&yuk);
    if !beklenen.to_hex().as_str().eq_ignore_ascii_case(hex) {
        return Err(bozuk());
    }
    Ok(yuk)
}

/// IO hatasını standart `ErrorReport` şemasına (ne/neden/çözüm) çevirir (İP-16).
fn io_hatasi(ne_oldu: &str, yol: &Path, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        ne_oldu.to_string(),
        format!("Disk erişiminde sorun: {} ({})", e, yol.display()),
        "Diskte yer/izin olduğundan emin olun; sorun sürerse uygulamayı yeniden başlatın.",
    )
    .with_teknik_detay(format!("{e:?} — {}", yol.display()))
}

/// Bütünlük doğrulaması başarısız olan (bozuk/yarım) kayıt için standart hata.
fn bozulma_hatasi(yol: &Path) -> ErrorReport {
    ErrorReport::new(
        "Kayıtlı durum dosyası bozuk",
        "Dosyanın bütünlük (BLAKE3) doğrulaması tutmadı; dosya yarım yazılmış veya değişmiş olabilir.",
        "Endişelenmeyin: uygulama güvenli varsayılan durumla açılır; bozuk kayıt yok sayılır.",
    )
    .with_teknik_detay(format!("BLAKE3 doğrulama hatası: {}", yol.display()))
}
