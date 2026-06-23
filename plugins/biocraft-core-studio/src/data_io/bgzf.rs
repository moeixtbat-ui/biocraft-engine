//! ÇE-01 — **BGZF blok-farkında okuma** (MK-32).
//!
//! BGZF (Blocked GZIP Format), her biri bağımsız bir gzip üyesi olan **64 KiB sınırlı bloklardan**
//! oluşur.  BAM/VCF.gz/… bu formatta saklanır.  **Ham baytlardan rastgele kesme YASAKTIR**: bir
//! bloğun ortasından bölmek, takip eden tüm blokların açılamamasına yol açar.  Bunun yerine veri
//! **blok sınırından** çözülür; rastgele erişim **sanal ofset** (`blok_baslangic << 16 | blok_ici`)
//! ile yapılır — indeks (.bai/.csi/.crai) bu sanal ofsetleri saklar.
//!
//! noodles'ın BAM/CRAM/indeksli okuyucuları BGZF'i **içeride** blok-farkında çözer; bu modül o
//! da  güvenceyi **açıkça** kullanan bir yardımcı sunar: bir BGZF dosyasını blok blok çözer,
//! asla ham bayttan kesmez.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use noodles::bgzf;

use biocraft_sdk::biocraft_types::ErrorReport;

/// Bir BGZF dosyasının çözülmüş içeriğinin özeti (blok-farkında okumanın kanıtı).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BgzfOzet {
    /// Çözülen toplam bayt (sabit bellekle, akışlı — tüm dosya RAM'e alınmaz).
    pub cozulen_bayt: u64,
    /// Son okumada ulaşılan sıkıştırılmış (kaynak) ofset — blok sınırına denk gelir.
    pub son_sikistirilmis_ofset: u64,
}

/// Okuma parça boyutu (64 KiB = BGZF blok üst sınırı) — sabit bellek (MK-09).
const PARCA: usize = 1 << 16;

/// Bir BGZF dosyasını **blok sınırından** baştan sona çözer; toplam çözülmüş baytı döndürür.
/// Hiçbir aşamada ham bayt parçalanmaz (MK-32); bellek dosya boyutundan bağımsız sabittir.
pub fn coz_ve_olc(yol: &Path) -> Result<BgzfOzet, ErrorReport> {
    let f = File::open(yol).map_err(|e| io_hatasi(yol, &e))?;
    let mut okuyucu = bgzf::io::Reader::new(f);
    let mut tampon = vec![0u8; PARCA];
    let mut cozulen: u64 = 0;
    loop {
        let n = okuyucu.read(&mut tampon).map_err(|e| {
            ErrorReport::new(
                "BGZF dosyası çözülemedi",
                format!(
                    "'{}' bir BGZF bloğu açılırken bozuldu (eksik/hatalı sıkıştırma)",
                    yol.display()
                ),
                "Dosyayı yeniden indirin; BGZF ile sıkıştırıldığından emin olun (bgzip)",
            )
            .with_eylem("Yeniden indir")
            .with_teknik_detay(e.to_string())
        })?;
        if n == 0 {
            break;
        }
        cozulen += n as u64;
    }
    // Sanal konumun sıkıştırılmış bileşeni = ulaşılan blok başlangıcı (>>16).
    let son_sikistirilmis_ofset = u64::from(okuyucu.virtual_position()) >> 16;
    Ok(BgzfOzet {
        cozulen_bayt: cozulen,
        son_sikistirilmis_ofset,
    })
}

fn io_hatasi(yol: &Path, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "Dosya açılamadı",
        format!("'{}' BGZF okuma için açılamadı", yol.display()),
        "Dosya yolunu ve okuma iznini kontrol edin",
    )
    .with_teknik_detay(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use noodles::bgzf::io::Writer;
    use std::io::Write;

    fn bgzf_yaz(ad: &str, icerik: &[u8]) -> std::path::PathBuf {
        let mut yol = std::env::temp_dir();
        yol.push(format!("biocraft_bgzf_{}_{ad}", std::process::id()));
        let mut w = Writer::new(File::create(&yol).unwrap());
        w.write_all(icerik).unwrap();
        w.finish().unwrap();
        yol
    }

    #[test]
    fn blok_farkinda_cozer_ve_olcer() {
        let icerik = b"Merhaba BioCraft BGZF blok-farkinda okuma testi.\n".repeat(100);
        let yol = bgzf_yaz("a.bgzf", &icerik);
        let ozet = coz_ve_olc(&yol).unwrap();
        assert_eq!(ozet.cozulen_bayt, icerik.len() as u64);
        let _ = std::fs::remove_file(&yol);
    }
}
