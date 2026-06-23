//! ÇE-01 — **Bütünlük denetimi (BLAKE3)** (MK-34).
//!
//! Dosya özeti **akışlı** hesaplanır: dosya 1 MiB'lık parçalarla okunur, hasher parça parça
//! güncellenir → **tüm dosya asla belleğe alınmaz** (MK-09).  4 TB BAM bile sabit bellekle
//! özetlenir.  Özet uyuşmazlığı = **karantina** (sessiz yükleme yok; net hata + öneri).

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use biocraft_sdk::biocraft_types::ErrorReport;

/// Akışlı okuma parça boyutu (1 MiB) — out-of-core: bellek dosya boyutundan bağımsız sabittir.
const PARCA: usize = 1 << 20;

/// Bir dosyanın BLAKE3 özetini (hex) **akışlı** hesaplar (tüm dosyayı RAM'e almadan — MK-09).
pub fn blake3_dosya(yol: &Path) -> Result<String, ErrorReport> {
    let f = File::open(yol).map_err(|e| {
        ErrorReport::new(
            "Dosya açılamadı",
            format!("'{}' bütünlük denetimi için açılamadı", yol.display()),
            "Dosya yolunu ve okuma iznini kontrol edin",
        )
        .with_teknik_detay(e.to_string())
    })?;
    let mut okuyucu = BufReader::new(f);
    let mut hasher = blake3::Hasher::new();
    let mut tampon = vec![0u8; PARCA];
    loop {
        let n = okuyucu.read(&mut tampon).map_err(|e| {
            ErrorReport::new(
                "Dosya okunurken hata",
                format!("'{}' okunurken G/Ç hatası oluştu", yol.display()),
                "Disk/ağ bağlantısını kontrol edip yeniden deneyin",
            )
            .with_teknik_detay(e.to_string())
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&tampon[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// Dosyayı **beklenen** BLAKE3'e karşı doğrular.  Uyuşmazlık → karantina hatası (çözüm önerir).
pub fn dogrula_blake3(yol: &Path, beklenen_hex: &str) -> Result<(), ErrorReport> {
    let gercek = blake3_dosya(yol)?;
    if gercek.eq_ignore_ascii_case(beklenen_hex) {
        Ok(())
    } else {
        Err(ErrorReport::new(
            "Dosya bütünlüğü doğrulanamadı",
            "dosyanın BLAKE3 özeti beklenenle eşleşmiyor (bozulmuş, eksik indirilmiş veya değiştirilmiş olabilir)",
            "Dosyayı yeniden indirin/edinin; sorun sürerse kaynağı kontrol edin (dosya karantinaya alındı)",
        )
        .with_eylem("Yeniden indir")
        .with_teknik_detay(format!("beklenen={beklenen_hex} gerçek={gercek}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn gecici(ad: &str, icerik: &[u8]) -> std::path::PathBuf {
        let mut yol = std::env::temp_dir();
        yol.push(format!("biocraft_integ_{}_{ad}", std::process::id()));
        let mut f = File::create(&yol).unwrap();
        f.write_all(icerik).unwrap();
        yol
    }

    #[test]
    fn blake3_bilinen_vektor() {
        // BLAKE3("abc") = 6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85
        let yol = gecici("abc.bin", b"abc");
        let h = blake3_dosya(&yol).unwrap();
        assert_eq!(
            h,
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn dogrulama_eslesir_ve_uyusmazlikta_reddeder() {
        let yol = gecici("d.bin", b"abc");
        let h = blake3_dosya(&yol).unwrap();
        assert!(dogrula_blake3(&yol, &h).is_ok());
        assert!(dogrula_blake3(&yol, &h.to_uppercase()).is_ok()); // hex büyük/küçük harf duyarsız
        let hata = dogrula_blake3(&yol, "00").unwrap_err();
        assert_eq!(hata.ne_oldu, "Dosya bütünlüğü doğrulanamadı");
        let _ = std::fs::remove_file(&yol);
    }
}
