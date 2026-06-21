//! Taşınabilir tek-dosya dışa aktarım: `.bcproj` (ZIP "stored", sıkıştırmasız) (MK-31).
//!
//! İP-02 taşınabilirlik: bir projeyi açık klasör + ek olarak **tek dosya** `.bcproj` olarak
//! paketle.  Biçim, herhangi bir ZIP aracıyla açılabilen **gerçek bir ZIP**'tir; ama sıkıştırma
//! yoktur (method = stored) → saf Rust, **yeni dış bağımlılık yok**, C derleyici gerekmez.
//!
//! Büyük veri **gömülmez** (referansla tutulur — `.bcproj` küçük kalır); yalnızca eşik altındaki
//! küçük dosyalar + manifest + meta + bütünlük mührü pakete girer (MK-09).  Hassas ayarlar
//! manifest filtresiyle dışarıda tutulur (madde 7; bkz. `manifest::disa_aktarim_icin_filtrele`).

use std::fs;
use std::path::Path;

use biocraft_types::ErrorReport;

use super::format;
use super::integrity::io_hatasi;

/// `.bcproj` dosya uzantısı.
pub const BCPROJ_UZANTI: &str = "bcproj";

/// Büyük dosya gömme eşiğinin varsayılanı (bayt).  Bu boyutun üstündeki dosyalar **gömülmez**;
/// referansla (checksum + yol) izlenir → `.bcproj` şişmez (MK-09).
pub const VARSAYILAN_GOMME_ESIGI: u64 = 5 * 1024 * 1024; // 5 MiB

/// Dışa aktarım seçenekleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisaAktarSecenekleri {
    /// Hassas (`[guvenlik]` + gerçek yol ipucu) ayarları pakete **dahil et** mi?
    /// Varsayılan `false` → hassas ayar sızmaz (madde 7; onaysız dışarı çıkmaz).
    pub hassas_dahil: bool,
    /// Bu boyutun (bayt) üstündeki dosyalar gömülmez; referansla kalır.
    pub gomme_esigi_bayt: u64,
}

impl Default for DisaAktarSecenekleri {
    fn default() -> Self {
        Self {
            hassas_dahil: false,
            gomme_esigi_bayt: VARSAYILAN_GOMME_ESIGI,
        }
    }
}

/// Başarılı bir dışa aktarımın özeti.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisaAktarRaporu {
    /// Pakete giren dosya sayısı.
    pub dosya_sayisi: usize,
    /// Hassas ayarlar hariç tutuldu mu?
    pub hassas_haric: bool,
    /// Üretilen `.bcproj` boyutu (bayt).
    pub boyut_bayt: u64,
}

/// Pakete eklenecek tek bir giriş (ZIP yolu + ham içerik).
pub struct Giris {
    /// ZIP içindeki yol (her zaman `/` ayıracı).
    pub yol: String,
    /// Ham bayt içeriği.
    pub veri: Vec<u8>,
}

/// Verilen girişleri **ZIP (stored)** biçiminde tek bir bayt tamponuna yazar.
///
/// Çıktı geçerli bir ZIP arşividir: her giriş için yerel başlık + veri, ardından merkezi dizin ve
/// son-kayıt.  Zaman damgası sabittir (1980-01-01) → **tekrarüretilebilir** çıktı.
pub fn zip_stored(girisler: &[Giris]) -> Vec<u8> {
    let mut govde: Vec<u8> = Vec::new();
    // Merkezi dizin kayıtları, gövde yazılırken biriktirilir.
    let mut merkez: Vec<u8> = Vec::new();
    let mut kayit_sayisi: u16 = 0;

    for g in girisler {
        let ad = g.yol.as_bytes();
        let crc = crc32(&g.veri);
        let boyut = g.veri.len() as u32;
        let yerel_ofset = govde.len() as u32;

        // ── Yerel dosya başlığı ──
        govde.extend_from_slice(&0x0403_4b50u32.to_le_bytes()); // imza
        govde.extend_from_slice(&20u16.to_le_bytes()); // çıkarmak için gereken sürüm
        govde.extend_from_slice(&0u16.to_le_bytes()); // genel amaç bayrağı
        govde.extend_from_slice(&0u16.to_le_bytes()); // yöntem = 0 (stored)
        govde.extend_from_slice(&0u16.to_le_bytes()); // değiştirme saati (sabit)
        govde.extend_from_slice(&0x0021u16.to_le_bytes()); // değiştirme tarihi = 1980-01-01
        govde.extend_from_slice(&crc.to_le_bytes());
        govde.extend_from_slice(&boyut.to_le_bytes()); // sıkıştırılmış = ham
        govde.extend_from_slice(&boyut.to_le_bytes()); // ham boyut
        govde.extend_from_slice(&(ad.len() as u16).to_le_bytes());
        govde.extend_from_slice(&0u16.to_le_bytes()); // ekstra alan yok
        govde.extend_from_slice(ad);
        govde.extend_from_slice(&g.veri);

        // ── Merkezi dizin kaydı ──
        merkez.extend_from_slice(&0x0201_4b50u32.to_le_bytes()); // imza
        merkez.extend_from_slice(&20u16.to_le_bytes()); // oluşturan sürüm
        merkez.extend_from_slice(&20u16.to_le_bytes()); // gereken sürüm
        merkez.extend_from_slice(&0u16.to_le_bytes()); // bayrak
        merkez.extend_from_slice(&0u16.to_le_bytes()); // yöntem = stored
        merkez.extend_from_slice(&0u16.to_le_bytes()); // saat
        merkez.extend_from_slice(&0x0021u16.to_le_bytes()); // tarih
        merkez.extend_from_slice(&crc.to_le_bytes());
        merkez.extend_from_slice(&boyut.to_le_bytes());
        merkez.extend_from_slice(&boyut.to_le_bytes());
        merkez.extend_from_slice(&(ad.len() as u16).to_le_bytes());
        merkez.extend_from_slice(&0u16.to_le_bytes()); // ekstra
        merkez.extend_from_slice(&0u16.to_le_bytes()); // yorum
        merkez.extend_from_slice(&0u16.to_le_bytes()); // disk no
        merkez.extend_from_slice(&0u16.to_le_bytes()); // iç öznitelik
        merkez.extend_from_slice(&0u32.to_le_bytes()); // dış öznitelik
        merkez.extend_from_slice(&yerel_ofset.to_le_bytes());
        merkez.extend_from_slice(ad);

        kayit_sayisi += 1;
    }

    let merkez_ofset = govde.len() as u32;
    let merkez_boyut = merkez.len() as u32;

    let mut cikti = govde;
    cikti.extend_from_slice(&merkez);

    // ── Merkezi dizin sonu kaydı ──
    cikti.extend_from_slice(&0x0605_4b50u32.to_le_bytes()); // imza
    cikti.extend_from_slice(&0u16.to_le_bytes()); // bu disk no
    cikti.extend_from_slice(&0u16.to_le_bytes()); // merkez başlangıç diski
    cikti.extend_from_slice(&kayit_sayisi.to_le_bytes()); // bu diskteki kayıt
    cikti.extend_from_slice(&kayit_sayisi.to_le_bytes()); // toplam kayıt
    cikti.extend_from_slice(&merkez_boyut.to_le_bytes());
    cikti.extend_from_slice(&merkez_ofset.to_le_bytes());
    cikti.extend_from_slice(&0u16.to_le_bytes()); // yorum yok

    cikti
}

/// `.bcproj` baytlarını diske atomik (geçici + rename) olarak yazar.
pub fn dosyaya_yaz(hedef: &Path, baytlar: &[u8]) -> Result<(), ErrorReport> {
    super::integrity::atomik_yaz(hedef, baytlar)
}

/// Bir proje klasöründeki kullanıcı veri klasörlerini (`data/`, `flows/`, `scripts/`,
/// `provenance/`) gezer; **eşik altındaki** dosyaları `(zip_yolu, içerik)` olarak toplar.
///
/// Eşik üstündeki büyük dosyalar **atlanır** (referansla tutulur — `.bcproj` şişmez, MK-09).
pub fn kucuk_dosyalari_topla(kok: &Path, esik_bayt: u64) -> Result<Vec<Giris>, ErrorReport> {
    let mut girisler = Vec::new();
    for alt in [
        format::VERI_DIZIN,
        format::FLOWS_DIZIN,
        format::SCRIPTS_DIZIN,
        format::PROVENANS_DIZIN,
    ] {
        let dizin = kok.join(alt);
        if dizin.is_dir() {
            gez(kok, &dizin, esik_bayt, &mut girisler)?;
        }
    }
    girisler.sort_by(|a, b| a.yol.cmp(&b.yol));
    Ok(girisler)
}

/// Bir klasörü özyineli gezer (eşik altı dosyaları toplar).
fn gez(kok: &Path, dizin: &Path, esik: u64, cikti: &mut Vec<Giris>) -> Result<(), ErrorReport> {
    let girdiler = fs::read_dir(dizin).map_err(|e| io_hatasi("Klasör gezilemedi", dizin, &e))?;
    for girdi in girdiler {
        let girdi = girdi.map_err(|e| io_hatasi("Klasör girdisi okunamadı", dizin, &e))?;
        let yol = girdi.path();
        if yol.is_dir() {
            gez(kok, &yol, esik, cikti)?;
        } else if yol.is_file() {
            let boyut = girdi.metadata().map(|m| m.len()).unwrap_or(0);
            if boyut > esik {
                continue; // büyük dosya: referansla tut, gömme.
            }
            let veri = fs::read(&yol).map_err(|e| io_hatasi("Dosya okunamadı", &yol, &e))?;
            let zip_yolu = zip_yolu_uret(kok, &yol);
            cikti.push(Giris {
                yol: zip_yolu,
                veri,
            });
        }
    }
    Ok(())
}

/// Proje köküne göreli, `/` ayıraçlı ZIP yolu üretir.
fn zip_yolu_uret(kok: &Path, dosya: &Path) -> String {
    let goreli = dosya.strip_prefix(kok).unwrap_or(dosya);
    goreli
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

// ─── CRC-32 (IEEE 802.3, yansıtılmış 0xEDB88320) ──────────────────────────────

/// Bir bayt diliminin CRC-32 (IEEE) sağlama toplamı — ZIP girişleri için.
pub fn crc32(veri: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in veri {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_bilinen_deger() {
        // "123456789" için CRC-32 referans değeri 0xCBF43926'dır.
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn zip_stored_imza_ve_eocd_iceriyor() {
        let girisler = vec![Giris {
            yol: "a.txt".to_string(),
            veri: b"merhaba".to_vec(),
        }];
        let zip = zip_stored(&girisler);
        // Yerel başlık imzası baştadır.
        assert_eq!(&zip[0..4], &0x0403_4b50u32.to_le_bytes());
        // EOCD imzası sonlardadır (yorum yok → son 22 baytın başı).
        let eocd = &zip[zip.len() - 22..zip.len() - 18];
        assert_eq!(eocd, &0x0605_4b50u32.to_le_bytes());
    }

    #[test]
    fn zip_stored_veriyi_ham_gomer() {
        let girisler = vec![Giris {
            yol: "x".to_string(),
            veri: b"AAA".to_vec(),
        }];
        let zip = zip_stored(&girisler);
        // Stored olduğundan ham veri ("AAA") arşivde aynen bulunur.
        assert!(zip.windows(3).any(|w| w == b"AAA"));
    }
}
