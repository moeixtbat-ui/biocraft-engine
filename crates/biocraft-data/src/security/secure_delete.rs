//! **Güvenli silme** — "sildim" gerçekten siler; hassas veri için üzerine yazma opsiyonu (MK-45).
//!
//! İki kademe:
//! - **Üzerine yazma:** Silmeden önce dosya içeriğinin üzerine (sıfır veya rastgele) yazılır ve
//!   `fsync`'lenir → basit kurtarma araçlarına karşı en iyi-çaba savunma.
//! - **Kaldırma:** Dosya/klasör dosya sisteminden silinir.
//!
//! **Dürüst not (önemli):** SSD/journaling/COW dosya sistemlerinde (wear-leveling, kopyala-yaz)
//! üzerine yazma fiziksel kurtarmaya karşı **garanti vermez** — eski bloklar başka yerde kalabilir.
//! **Gerçek** garanti, veriyi en baştan **dinlenmede şifrelemek** ve silerken **anahtarı imha
//! etmektir** (kripto-shred → [`crate::security::crypto`]).  Bu modül en iyi-çaba + kripto-shred'in
//! tamamlayıcısıdır.

use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

use biocraft_types::ErrorReport;

use crate::project::integrity::io_hatasi;

/// Üzerine yazma davranışı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UzerineYazSecenek {
    /// Kaç geçiş üzerine yazılsın (0 = üzerine yazma, doğrudan sil).  En iyi-çaba; 1 genelde yeterli.
    pub gecis: u8,
    /// Sıfır yerine rastgele bayt yaz (biraz daha güçlü; varsayılan sıfır = hızlı/deterministik test).
    pub rastgele: bool,
}

impl Default for UzerineYazSecenek {
    fn default() -> Self {
        Self {
            gecis: 1,
            rastgele: false,
        }
    }
}

impl UzerineYazSecenek {
    /// Hiç üzerine yazma — yalnızca kaldır (büyük/hassas-olmayan veri için hızlı).
    pub fn yok() -> Self {
        Self {
            gecis: 0,
            rastgele: false,
        }
    }
}

/// Tek bir dosyanın içeriğinin üzerine yazar (silmeden).  Yazılan toplam baytı döndürür.
///
/// En iyi-çaba: yazma başarısız olursa hata döner ama dosya yine de silinebilir (çağıran karar verir).
pub fn dosya_uzerine_yaz(yol: &Path, secenek: &UzerineYazSecenek) -> Result<u64, ErrorReport> {
    if secenek.gecis == 0 {
        return Ok(0);
    }
    let boyut = fs::metadata(yol)
        .map_err(|e| io_hatasi("Dosya bilgisi alınamadı", yol, &e))?
        .len();
    if boyut == 0 {
        return Ok(0);
    }

    let mut f = fs::OpenOptions::new()
        .write(true)
        .open(yol)
        .map_err(|e| io_hatasi("Dosya üzerine yazma için açılamadı", yol, &e))?;

    let mut toplam = 0u64;
    // En çok 1 MiB'lik tampon — devasa dosyada belleği şişirmemek için pencere pencere yaz.
    let tampon_boyut = boyut.min(1024 * 1024) as usize;
    for _ in 0..secenek.gecis {
        let mut tampon = vec![0u8; tampon_boyut];
        if secenek.rastgele {
            getrandom::getrandom(&mut tampon).map_err(|e| rastgele_hatasi(&e.to_string()))?;
        }
        f.seek(SeekFrom::Start(0))
            .map_err(|e| io_hatasi("Dosya başına gidilemedi", yol, &e))?;
        let mut kalan = boyut;
        while kalan > 0 {
            let n = kalan.min(tampon.len() as u64) as usize;
            f.write_all(&tampon[..n])
                .map_err(|e| io_hatasi("Üzerine yazılamadı", yol, &e))?;
            kalan -= n as u64;
            toplam += n as u64;
        }
        f.flush()
            .map_err(|e| io_hatasi("Tampon boşaltılamadı", yol, &e))?;
        f.sync_all()
            .map_err(|e| io_hatasi("Diske işlenemedi (fsync)", yol, &e))?;
    }
    Ok(toplam)
}

/// Bir dosyayı **güvenli** siler: önce (seçenekse) üzerine yazar, sonra kaldırır.
///
/// Hedef dosya değilse (klasör/yok) hata döner.
pub fn guvenli_dosya_sil(yol: &Path, secenek: &UzerineYazSecenek) -> Result<u64, ErrorReport> {
    if !yol.is_file() {
        return Err(ErrorReport::new(
            "Silinecek dosya bulunamadı",
            format!("'{}' bir dosya değil veya yok.", yol.display()),
            "Geçerli bir dosya yolu verin.",
        ));
    }
    let yazilan = dosya_uzerine_yaz(yol, secenek)?;
    fs::remove_file(yol).map_err(|e| io_hatasi("Dosya silinemedi", yol, &e))?;
    Ok(yazilan)
}

fn rastgele_hatasi(detay: &str) -> ErrorReport {
    ErrorReport::new(
        "Güvenli rastgelelik alınamadı",
        "Üzerine yazma için rastgele bayt üretilemedi.",
        "Sıfır-geçişli üzerine yazma kullanın veya uygulamayı yeniden başlatın.",
    )
    .with_teknik_detay(format!("getrandom: {detay}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gecici_dosya(ad: &str, icerik: &[u8]) -> std::path::PathBuf {
        let yol = std::env::temp_dir().join(format!(
            "bc_sec_del_{}_{}_{}",
            ad,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::write(&yol, icerik).unwrap();
        yol
    }

    #[test]
    fn guvenli_sil_dosyayi_kaldirir() {
        let yol = gecici_dosya("kaldir", b"hassas icerik buradaydi");
        assert!(yol.is_file());
        let yazilan = guvenli_dosya_sil(&yol, &UzerineYazSecenek::default()).unwrap();
        assert!(!yol.exists(), "dosya silinmeli");
        assert_eq!(yazilan, b"hassas icerik buradaydi".len() as u64);
    }

    #[test]
    fn uzerine_yazma_icerigi_degistirir() {
        // Üzerine yazmadan ÖNCE içeriği oku → sonra dosya silinmeden üzerine yaz → değişmiş olmalı.
        let yol = gecici_dosya("uzerineyaz", b"AAAAAAAAAAAAAAAA");
        dosya_uzerine_yaz(&yol, &UzerineYazSecenek::default()).unwrap();
        let sonra = fs::read(&yol).unwrap();
        assert_eq!(sonra, vec![0u8; 16], "içerik sıfırlanmış olmalı");
        let _ = fs::remove_file(&yol);
    }

    #[test]
    fn sifir_gecis_sadece_siler() {
        let yol = gecici_dosya("nogecis", b"veri");
        let yazilan = guvenli_dosya_sil(&yol, &UzerineYazSecenek::yok()).unwrap();
        assert_eq!(yazilan, 0, "üzerine yazma yapılmamalı");
        assert!(!yol.exists());
    }

    #[test]
    fn olmayan_dosya_hata() {
        let yok = std::env::temp_dir().join("bc_sec_del_yok_xyz_12345");
        let _ = fs::remove_file(&yok);
        assert!(guvenli_dosya_sil(&yok, &UzerineYazSecenek::default()).is_err());
    }

    #[test]
    fn rastgele_gecis_calisir() {
        let yol = gecici_dosya("rastgele", b"0123456789");
        let secenek = UzerineYazSecenek {
            gecis: 2,
            rastgele: true,
        };
        let yazilan = dosya_uzerine_yaz(&yol, &secenek).unwrap();
        assert_eq!(yazilan, 20, "2 geçiş × 10 bayt");
        let _ = fs::remove_file(&yol);
    }
}
