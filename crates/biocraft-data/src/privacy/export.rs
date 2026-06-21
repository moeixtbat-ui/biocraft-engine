//! **Veri sahibi hakları** (KVKK/GDPR) — tam ihraç (taşınabilirlik), güvenli silme (unutulma),
//! erişim/görüntüleme (İP-10).
//!
//! - **Tam ihraç** ([`tam_veri_ihrac`]): projenin **tüm** verisini (manifest dâhil, filtresiz) tek
//!   `.zip`'e paketler — taşınabilirlik hakkı.  `.bcproj`'dan farkı: hassas ayarları **çıkarmaz**
//!   (bu, *sahibinin kendi* verisidir).
//! - **Güvenli silme** ([`guvenli_sil`]): unutulma hakkı.  İsteğe bağlı **üzerine yazma** + klasörü
//!   tümüyle kaldırma.  *Dürüst not:* SSD/journaling dosya sistemlerinde üzerine-yazma kurtarmaya
//!   karşı garanti vermez; **gerçek** garanti, at-rest şifreleme + anahtar imhasıdır (kripto-shred —
//!   İP-09).  Bu fonksiyon en iyi-çaba + İP-09 kancasıdır.
//! - **Erişim/görüntüleme** ([`veri_envanteri`]): kullanıcının elindeki tüm veriyi (yol + boyut)
//!   listeler — "neyim var" sorusu.

use std::fs;
use std::path::Path;

use biocraft_types::ErrorReport;

use crate::project::export::{dosyaya_yaz, kucuk_dosyalari_topla, zip_stored, Giris};
use crate::project::integrity::io_hatasi;
use crate::project::{ac, format};

// ─── Erişim / görüntüleme (envanter) ──────────────────────────────────────────

/// Envanterdeki tek bir veri öğesi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvanterOge {
    /// Proje köküne göreli yol (`/` ayraçlı).
    pub yol: String,
    /// Dosya boyutu (bayt).
    pub boyut: u64,
}

/// Bir projenin tuttuğu veriyi listeleyen envanter (erişim/görüntüleme hakkı).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Envanter {
    /// Veri öğeleri (data/flows/scripts altındaki tüm dosyalar).
    pub ogeler: Vec<EnvanterOge>,
    /// Toplam boyut (bayt).
    pub toplam_bayt: u64,
}

/// Projenin veri envanterini çıkarır (kullanıcı "neyim var" diye görebilsin).
pub fn veri_envanteri(kok: &Path) -> Result<Envanter, ErrorReport> {
    let mut ogeler = Vec::new();
    let mut toplam = 0u64;
    for alt in [
        format::VERI_DIZIN,
        format::FLOWS_DIZIN,
        format::SCRIPTS_DIZIN,
    ] {
        let dizin = kok.join(alt);
        if dizin.is_dir() {
            envanter_gez(kok, &dizin, &mut ogeler, &mut toplam)?;
        }
    }
    ogeler.sort_by(|a, b| a.yol.cmp(&b.yol));
    Ok(Envanter {
        ogeler,
        toplam_bayt: toplam,
    })
}

fn envanter_gez(
    kok: &Path,
    dizin: &Path,
    cikti: &mut Vec<EnvanterOge>,
    toplam: &mut u64,
) -> Result<(), ErrorReport> {
    for girdi in fs::read_dir(dizin).map_err(|e| io_hatasi("Klasör gezilemedi", dizin, &e))? {
        let girdi = girdi.map_err(|e| io_hatasi("Klasör girdisi okunamadı", dizin, &e))?;
        let yol = girdi.path();
        if yol.is_dir() {
            envanter_gez(kok, &yol, cikti, toplam)?;
        } else if yol.is_file() {
            let boyut = girdi.metadata().map(|m| m.len()).unwrap_or(0);
            *toplam += boyut;
            let goreli = yol
                .strip_prefix(kok)
                .unwrap_or(&yol)
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/");
            cikti.push(EnvanterOge { yol: goreli, boyut });
        }
    }
    Ok(())
}

// ─── Tam ihraç (taşınabilirlik) ───────────────────────────────────────────────

/// Tam ihraç seçenekleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IhracSecenekleri {
    /// Büyük dosyalar da pakete gömülsün mü?  Taşınabilirlik için varsayılan **Evet** (tüm veri).
    pub buyuk_dahil: bool,
}

impl Default for IhracSecenekleri {
    fn default() -> Self {
        Self { buyuk_dahil: true }
    }
}

/// Tam ihracın özeti.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IhracRaporu {
    /// Pakete giren dosya sayısı.
    pub dosya_sayisi: usize,
    /// Üretilen `.zip` boyutu (bayt).
    pub boyut_bayt: u64,
}

/// Projenin **tüm** verisini (manifest dâhil, filtresiz) tek bir `.zip`'e ihraç eder (taşınabilirlik).
///
/// `.bcproj`'dan farkı: sahibinin kendi tam kopyası olduğundan hassas ayar **çıkarılmaz**.  Önce proje
/// doğrulanır (bozuksa ihraç edilmez).
pub fn tam_veri_ihrac(
    kok: &Path,
    hedef_zip: &Path,
    secenek: &IhracSecenekleri,
) -> Result<IhracRaporu, ErrorReport> {
    let _acilan = ac(kok)?; // bütünlüğü doğrula; bozuksa ihraç etme.

    // data/flows/scripts/provenance altındaki kullanıcı dosyaları (köken + onay defterleri dâhil).
    let esik = if secenek.buyuk_dahil {
        u64::MAX
    } else {
        crate::project::export::VARSAYILAN_GOMME_ESIGI
    };
    let mut girisler = kucuk_dosyalari_topla(kok, esik)?;

    // Çekirdek format dosyaları (manifest FİLTRESİZ — bu sahibinin kendi verisi).
    girisler.push(format_giris(
        kok,
        &format::manifest_yolu(kok),
        format::MANIFEST_DOSYA,
    )?);
    girisler.push(format_giris(
        kok,
        &format::meta_yolu(kok),
        &format!("{}/{}", format::META_DIZIN, format::META_DOSYA),
    )?);
    girisler.push(format_giris(
        kok,
        &format::butunluk_yolu(kok),
        &format!("{}/{}", format::META_DIZIN, format::BUTUNLUK_DOSYA),
    )?);

    let zip = zip_stored(&girisler);
    dosyaya_yaz(hedef_zip, &zip)?;

    Ok(IhracRaporu {
        dosya_sayisi: girisler.len(),
        boyut_bayt: zip.len() as u64,
    })
}

/// Bir format dosyasını okuyup ZIP girişi yapar.
fn format_giris(_kok: &Path, yol: &Path, zip_yolu: &str) -> Result<Giris, ErrorReport> {
    let veri = fs::read(yol).map_err(|e| io_hatasi("Dosya okunamadı", yol, &e))?;
    Ok(Giris {
        yol: zip_yolu.to_string(),
        veri,
    })
}

// ─── Güvenli silme (unutulma hakkı) ───────────────────────────────────────────

/// Güvenli silme seçenekleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SilmeSecenekleri {
    /// Silmeden önce dosya içeriklerinin üzerine yaz (en iyi-çaba; SSD'de garanti değil — bkz. modül notu).
    pub uzerine_yaz: bool,
}

impl Default for SilmeSecenekleri {
    fn default() -> Self {
        Self { uzerine_yaz: true }
    }
}

/// Güvenli silmenin raporu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SilmeRaporu {
    /// Silinen dosya sayısı.
    pub silinen_dosya: usize,
    /// Üzerine yazılan toplam bayt.
    pub uzerine_yazilan_bayt: u64,
}

/// Bir projeyi **güvenli** siler (unutulma hakkı): isteğe bağlı üzerine yazma + klasörü tümüyle kaldırma.
///
/// **Geri döndürülemez** — çağıran (UI) önce onay almalıdır (TDA madde 7).  Hedef bir BioCraft projesi
/// değilse hata döner (yanlış klasörü silmeyi önler — güvenlik).
pub fn guvenli_sil(kok: &Path, secenek: &SilmeSecenekleri) -> Result<SilmeRaporu, ErrorReport> {
    if !format::proje_mi(kok) {
        return Err(ErrorReport::new(
            "Bu klasör bir BioCraft projesi değil",
            format!(
                "'{}' içinde manifest yok; yanlış klasör silinmesin diye işlem durduruldu.",
                kok.display()
            ),
            "Silmek istediğiniz geçerli proje klasörünü seçin.",
        ));
    }

    let mut silinen = 0usize;
    let mut yazilan = 0u64;
    if secenek.uzerine_yaz {
        uzerine_yaz_gez(kok, &mut silinen, &mut yazilan)?;
    }

    fs::remove_dir_all(kok).map_err(|e| io_hatasi("Proje klasörü silinemedi", kok, &e))?;

    Ok(SilmeRaporu {
        silinen_dosya: if secenek.uzerine_yaz {
            silinen
        } else {
            // Üzerine yazma yoksa dosya sayısını yine de raporla (bilgi amaçlı saymıyoruz → 0).
            0
        },
        uzerine_yazilan_bayt: yazilan,
    })
}

/// Tüm dosyaların içeriğini üzerine yazıp fsync'ler (en iyi-çaba) — İP-09 silme primitifini kullanır.
///
/// Düşük seviye üzerine-yazma `security::secure_delete`'te tek yerde tutulur (DRY); buradaki gezgin
/// onu klasör ağacına uygular.  Üzerine yazma başarısız olsa bile silmeye devam edilir (en iyi-çaba).
fn uzerine_yaz_gez(
    dizin: &Path,
    silinen: &mut usize,
    yazilan: &mut u64,
) -> Result<(), ErrorReport> {
    use crate::security::secure_delete::{dosya_uzerine_yaz, UzerineYazSecenek};
    for girdi in fs::read_dir(dizin).map_err(|e| io_hatasi("Klasör gezilemedi", dizin, &e))? {
        let girdi = girdi.map_err(|e| io_hatasi("Klasör girdisi okunamadı", dizin, &e))?;
        let yol = girdi.path();
        if yol.is_dir() {
            uzerine_yaz_gez(&yol, silinen, yazilan)?;
        } else if yol.is_file() {
            // En iyi-çaba: üzerine yazma başarısızsa bile (kilit/izin) silmeye devam edilir.
            *yazilan += dosya_uzerine_yaz(&yol, &UzerineYazSecenek::default()).unwrap_or(0);
            *silinen += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{olustur, ProjeKurulumGirdisi};
    use biocraft_types::{DataClassification, Version};

    fn kur(etiket: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let konum = std::env::temp_dir().join(format!(
            "bc_priv_export_{}_{}_{}",
            etiket,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&konum);
        fs::create_dir_all(&konum).unwrap();
        let g = ProjeKurulumGirdisi::yeni(
            "Hak Projesi",
            &konum,
            "genomik",
            DataClassification::HasasPhi,
            Version::new(0, 1, 0),
        );
        let kurulan = olustur(&g).unwrap();
        (konum, kurulan.kok)
    }

    #[test]
    fn envanter_dosyalari_listeler() {
        let (konum, kok) = kur("env");
        fs::write(kok.join("data/inputs/a.txt"), b"on bayt!!!").unwrap();
        fs::write(kok.join("data/inputs/b.txt"), b"bes").unwrap();

        let env = veri_envanteri(&kok).unwrap();
        assert_eq!(env.ogeler.len(), 2);
        assert_eq!(env.toplam_bayt, 10 + 3);
        assert!(env.ogeler.iter().any(|o| o.yol == "data/inputs/a.txt"));

        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn tam_ihrac_manifesti_filtresiz_icerir() {
        let (konum, kok) = kur("ihrac");
        fs::write(kok.join("data/inputs/veri.txt"), b"kullanici verisi").unwrap();

        let hedef = konum.join("ihrac.zip");
        let rapor = tam_veri_ihrac(&kok, &hedef, &IhracSecenekleri::default()).unwrap();
        assert!(hedef.is_file());
        assert!(rapor.dosya_sayisi >= 4); // veri + manifest + meta + butunluk

        // Tam ihraçta hassas [guvenlik] bölümü KORUNUR (sahibinin kendi kopyası).
        let icerik = fs::read(&hedef).unwrap();
        assert!(
            icerik.windows(10).any(|w| w == b"[guvenlik]"),
            "tam ihraç manifesti filtresiz olmalı"
        );
        assert!(icerik.windows(16).any(|w| w == b"kullanici verisi"));

        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn guvenli_sil_projeyi_kaldirir() {
        let (konum, kok) = kur("sil");
        fs::write(kok.join("data/inputs/gizli.txt"), b"hassas veri burada").unwrap();
        assert!(kok.exists());

        let rapor = guvenli_sil(&kok, &SilmeSecenekleri::default()).unwrap();
        assert!(!kok.exists(), "proje klasörü silinmeli");
        assert!(rapor.silinen_dosya > 0);
        assert!(rapor.uzerine_yazilan_bayt > 0);

        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn guvenli_sil_proje_olmayan_klasoru_reddeder() {
        let bos = std::env::temp_dir().join(format!("bc_priv_notproj_{}", std::process::id()));
        let _ = fs::remove_dir_all(&bos);
        fs::create_dir_all(&bos).unwrap();
        fs::write(bos.join("onemli.txt"), b"kullanici dosyasi").unwrap();

        let hata = guvenli_sil(&bos, &SilmeSecenekleri::default()).unwrap_err();
        assert!(hata.ne_oldu.contains("değil"));
        // Klasör + kullanıcı dosyası KORUNMUŞ olmalı (yanlış silme önlendi).
        assert!(bos.join("onemli.txt").is_file());

        let _ = fs::remove_dir_all(&bos);
    }
}
