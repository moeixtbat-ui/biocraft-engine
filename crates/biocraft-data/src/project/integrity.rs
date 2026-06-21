//! Bütünlük (BLAKE3) + atomik yazma — proje formatının dayanıklılık temeli (MK-33/MK-34).
//!
//! İP-02: Manifest + meta + her veri referansı için BLAKE3 özeti tutulur ve **açılışta
//! doğrulanır**; bozuk/eksik dosya net bildirilir (sessiz açma yok — kabul kriteri).
//!
//! İki yapı taşı:
//! - **Atomik yazma:** geçici dosyaya yaz + `fsync` + hedefin üzerine atomik `rename`.  Yazma
//!   yarıda kesilse bile hedef ya eski tam hâli ya yeni tam hâli kalır — **asla yarım**.
//! - **Bütünlük zarfı (`BCP1`):** kendini doğrulayan metadata dosyaları için yükün BLAKE3 özeti
//!   başa yazılır; okumada yeniden hesaplanıp karşılaştırılır.  (`biocraft-state`'in `BCS1`
//!   zarfıyla aynı fikir; veri katmanı kendi içinde bağımsız kalsın diye burada ayrıca tutulur.)

use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use biocraft_types::{Blake3Hash, ErrorReport};

/// Kendini-doğrulayan metadata zarfının başlığı (sürüm 1).  Biçim: `BCP1 <64-hex>\n<yük>`.
const ZARF_ETIKET: &[u8] = b"BCP1 ";
/// BLAKE3 özetinin onaltılık uzunluğu.
const HEX_UZUNLUK: usize = 64;

/// Aynı süreçte üretilen geçici dosya adlarının çakışmamasını sağlayan sayaç.
static GECICI_SAYAC: AtomicU64 = AtomicU64::new(0);

/// Bir bayt diliminin BLAKE3 özetini [`Blake3Hash`] olarak hesaplar.
pub fn icerik_ozeti(veri: &[u8]) -> Blake3Hash {
    Blake3Hash(*blake3::hash(veri).as_bytes())
}

/// Bir dosyanın içeriğini okuyup BLAKE3 özetini döndürür.
///
/// Dosya yoksa/okunamıyorsa açıklayıcı [`ErrorReport`] döner (sessizce yutulmaz).
pub fn dosya_ozeti(yol: &Path) -> Result<Blake3Hash, ErrorReport> {
    let veri = fs::read(yol).map_err(|e| io_hatasi("Dosya bütünlük için okunamadı", yol, &e))?;
    Ok(icerik_ozeti(&veri))
}

/// Yükü `BCP1 <hex>\n<yük>` zarfına sarar (kendini-doğrulayan metadata için).
pub fn zarf_sar(yuk: &[u8]) -> Vec<u8> {
    let ozet = blake3::hash(yuk);
    let mut icerik = Vec::with_capacity(ZARF_ETIKET.len() + HEX_UZUNLUK + 1 + yuk.len());
    icerik.extend_from_slice(ZARF_ETIKET);
    icerik.extend_from_slice(ozet.to_hex().as_bytes());
    icerik.push(b'\n');
    icerik.extend_from_slice(yuk);
    icerik
}

/// `BCP1` zarfını çözer ve BLAKE3 özetini doğrular.  Bozuksa net [`ErrorReport`] döner.
pub fn zarf_coz(ham: &[u8], hedef: &Path) -> Result<Vec<u8>, ErrorReport> {
    let bozuk = || bozulma_hatasi(hedef);

    if !ham.starts_with(ZARF_ETIKET) {
        return Err(bozuk());
    }
    let govde = &ham[ZARF_ETIKET.len()..];
    let satir_sonu = govde.iter().position(|&b| b == b'\n').ok_or_else(bozuk)?;
    if satir_sonu != HEX_UZUNLUK {
        return Err(bozuk());
    }
    let hex = std::str::from_utf8(&govde[..satir_sonu]).map_err(|_| bozuk())?;
    let yuk = govde[satir_sonu + 1..].to_vec();

    let beklenen = blake3::hash(&yuk);
    if !beklenen.to_hex().as_str().eq_ignore_ascii_case(hex) {
        return Err(bozuk());
    }
    Ok(yuk)
}

/// Atomik dosya yazımı: geçici dosyaya yaz + `fsync` + hedefin üzerine `rename`.
///
/// `rename` aynı klasör içinde atomik yer değiştirmedir (Windows'ta da mevcut dosyanın üzerine
/// yazar).  Okuyucu ya eski ya yeni tam içeriği görür; yarım dosya hedefte hiç görünmez.
pub fn atomik_yaz(hedef: &Path, icerik: &[u8]) -> Result<(), ErrorReport> {
    if let Some(ana) = hedef.parent() {
        if let Err(e) = fs::create_dir_all(ana) {
            return Err(io_hatasi("Hedef klasör oluşturulamadı", ana, &e));
        }
    }

    let pid = std::process::id();
    let n = GECICI_SAYAC.fetch_add(1, Ordering::Relaxed);
    let temel = hedef
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "proje".to_string());
    let gecici = hedef.with_file_name(format!("{temel}.tmp-{pid}-{n}"));

    {
        let mut f = match fs::File::create(&gecici) {
            Ok(f) => f,
            Err(e) => return Err(io_hatasi("Geçici dosya oluşturulamadı", &gecici, &e)),
        };
        if let Err(e) = f.write_all(icerik) {
            let _ = fs::remove_file(&gecici);
            return Err(io_hatasi("Veri diske yazılamadı", &gecici, &e));
        }
        if let Err(e) = f.sync_all() {
            let _ = fs::remove_file(&gecici);
            return Err(io_hatasi("Veri diske işlenemedi (fsync)", &gecici, &e));
        }
    }

    if let Err(e) = fs::rename(&gecici, hedef) {
        let _ = fs::remove_file(&gecici);
        return Err(io_hatasi("Dosya yerine konamadı", hedef, &e));
    }
    Ok(())
}

/// IO hatasını standart `ErrorReport` şemasına (ne/neden/çözüm) çevirir (İP-16).
pub fn io_hatasi(ne_oldu: &str, yol: &Path, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        ne_oldu.to_string(),
        format!("Disk erişiminde sorun: {} ({})", e, yol.display()),
        "Diskte yer/izin olduğundan emin olun; sorun sürerse uygulamayı yeniden başlatın.",
    )
    .with_teknik_detay(format!("{e:?} — {}", yol.display()))
}

/// Bütünlük doğrulaması başarısız olan (bozuk/yarım) metadata için standart hata.
fn bozulma_hatasi(yol: &Path) -> ErrorReport {
    ErrorReport::new(
        "Proje dosyası bozuk",
        "Dosyanın bütünlük (BLAKE3) doğrulaması tutmadı; dosya yarım yazılmış veya değişmiş olabilir.",
        "Projenin yedeğinden geri yükleyin; yedek yoksa bu dosya elle onarılmalı.",
    )
    .with_teknik_detay(format!("BLAKE3 zarf doğrulama hatası: {}", yol.display()))
}

/// Bir manifest/meta dosyasının beklenen özetle uyuşmadığını bildiren net hata (açılış denetimi).
pub fn uyusmazlik_hatasi(ad: &str, yol: &Path) -> ErrorReport {
    ErrorReport::new(
        format!("{ad} dosyası değişmiş veya bozuk"),
        format!(
            "{ad} dosyasının BLAKE3 özeti, projeyle birlikte kaydedilen beklenen özetle uyuşmuyor."
        ),
        "Dosyayı projenin güvenilir bir yedeğinden geri yükleyin (sessiz açma yapılmaz).",
    )
    .with_teknik_detay(format!("Bütünlük uyuşmazlığı: {}", yol.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zarf_gidis_donus() {
        let yuk = b"merhaba biocraft";
        let sarili = zarf_sar(yuk);
        let cozulen = zarf_coz(&sarili, Path::new("x")).unwrap();
        assert_eq!(cozulen, yuk);
    }

    #[test]
    fn zarf_bozulma_yakalanir() {
        let mut sarili = zarf_sar(b"veri");
        let son = sarili.len() - 1;
        sarili[son] ^= 0xFF; // yükü boz
        assert!(zarf_coz(&sarili, Path::new("x")).is_err());
    }

    #[test]
    fn icerik_ozeti_kararli() {
        assert_eq!(icerik_ozeti(b"abc"), icerik_ozeti(b"abc"));
        assert_ne!(icerik_ozeti(b"abc"), icerik_ozeti(b"abd"));
    }
}
