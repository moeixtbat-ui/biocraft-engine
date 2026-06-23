//! ÇE-01 — **GenBank** (`.gb`/`.gbk`) okuma — **dizi + özellik (feature)** (F1).
//!
//! GenBank düz-metin bir kayıt formatıdır; `noodles`'ta yoktur, bu yüzden **saf-Rust** (yeni dış
//! bağımlılık eklemeden) ayrıştırılır.  Bir kayıt üç bölümden oluşur:
//!
//! 1. **Başlık:** `LOCUS` (ad/uzunluk/molekül/topoloji), `DEFINITION`, `ACCESSION`, …
//! 2. **FEATURES:** her özellik bir **tür** (gene/CDS/source…), bir **konum** (`1..5028`,
//!    `complement(...)`, `join(...)`) ve `/anahtar="değer"` **nitelikleri** taşır.
//! 3. **ORIGIN:** dizi (numara+boşluk arındırılır → saf nükleotit/aminoasit).
//!
//! Kayıt `//` ile biter.
//!
//! Akışlıdır (MK-09): dosya satır satır okunur; her tamamlanan kayıt çağırana verilir, tüm dosya
//! tek seferde belleğe alınmaz.  (Tek kaydın dizisi doğal olarak bellekte tutulur — kullanıcının
//! istediği veri budur.)  Yaygın GenBank kayıtları hedeflenir; egzotik yapılar sadeleştirilir.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use biocraft_sdk::biocraft_types::ErrorReport;

/// Tek bir GenBank kaydı (F1: dizi + özellikler).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GenBankKaydi {
    /// LOCUS adı (örn. "SCU49845").
    pub locus: String,
    /// LOCUS'ta bildirilen dizi uzunluğu (bp/aa).
    pub uzunluk: usize,
    /// Molekül tipi (DNA/RNA/protein…; boşsa bilinmiyor).
    pub molekul: String,
    /// Topoloji (linear/circular; boşsa bilinmiyor).
    pub topoloji: String,
    /// DEFINITION satırı (açıklama).
    pub tanim: String,
    /// ACCESSION (erişim numarası).
    pub erisim: String,
    /// Özellikler (gene/CDS/exon/source…).
    pub ozellikler: Vec<GenBankOzellik>,
    /// Dizi baytları (ORIGIN; numara/boşluk arındırılmış).
    pub dizi: Vec<u8>,
}

/// Bir GenBank özelliği (feature).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GenBankOzellik {
    /// Özellik türü (gene/CDS/exon/source/mRNA…).
    pub tur: String,
    /// Konum ifadesi (örn. "1..5028", "complement(100..200)", "join(1..5,10..15)").
    pub konum: String,
    /// `/anahtar="değer"` nitelikleri (sırayla).
    pub nitelikler: Vec<(String, String)>,
}

#[derive(PartialEq)]
enum Bolge {
    Bas,
    Ozellikler,
    Dizi,
}

/// GenBank dosyasını **akışlı** okur; her tamamlanan kayıt için `gozlemci(&GenBankKaydi)` çağrılır.
/// Toplam kayıt sayısını döndürür.
pub fn genbank_akis<F>(yol: &Path, mut gozlemci: F) -> Result<usize, ErrorReport>
where
    F: FnMut(&GenBankKaydi),
{
    let dosya = File::open(yol).map_err(|e| io_hatasi(yol, &e))?;
    let okuyucu = BufReader::new(dosya);

    let mut sayi = 0;
    let mut kayit = GenBankKaydi::default();
    let mut bolge = Bolge::Bas;
    // Çok-satırlı nitelik değeri biriktirme (kapanmamış tırnak).
    let mut acik_nitelik: Option<(String, String)> = None;

    for satir_sonuc in okuyucu.lines() {
        let satir = satir_sonuc.map_err(|e| io_hatasi(yol, &e))?;

        // Kayıt sonu.
        if satir.starts_with("//") {
            nitelik_kapat(&mut kayit, &mut acik_nitelik);
            gozlemci(&kayit);
            sayi += 1;
            kayit = GenBankKaydi::default();
            bolge = Bolge::Bas;
            continue;
        }

        // Bölüm geçişleri (sütun 0'dan başlayan anahtar sözcükler).
        if satir.starts_with("FEATURES") {
            bolge = Bolge::Ozellikler;
            continue;
        }
        if satir.starts_with("ORIGIN") {
            nitelik_kapat(&mut kayit, &mut acik_nitelik);
            bolge = Bolge::Dizi;
            continue;
        }

        match bolge {
            Bolge::Bas => baslik_satiri(&satir, &mut kayit),
            Bolge::Ozellikler => {
                ozellik_satiri(&satir, &mut kayit, &mut acik_nitelik);
            }
            Bolge::Dizi => dizi_satiri(&satir, &mut kayit),
        }
    }

    // Dosya `//` olmadan bittiyse son kaydı yine de teslim et (yalnız anlamlıysa).
    if !kayit.locus.is_empty() || !kayit.dizi.is_empty() || !kayit.ozellikler.is_empty() {
        nitelik_kapat(&mut kayit, &mut acik_nitelik);
        gozlemci(&kayit);
        sayi += 1;
    }

    Ok(sayi)
}

/// Tek-kayıtlı GenBank dosyaları için kolaylık: **ilk** kaydı döndürür (yoksa net hata).
pub fn ilk_kayit(yol: &Path) -> Result<GenBankKaydi, ErrorReport> {
    let mut bulunan: Option<GenBankKaydi> = None;
    genbank_akis(yol, |k| {
        if bulunan.is_none() {
            bulunan = Some(k.clone());
        }
    })?;
    bulunan.ok_or_else(|| {
        ErrorReport::new(
            "GenBank kaydı bulunamadı",
            format!(
                "'{}' içinde hiçbir GenBank kaydı ayrıştırılamadı",
                yol.display()
            ),
            "Dosyanın geçerli bir GenBank (LOCUS…//) olduğundan emin olun",
        )
    })
}

// ─── Satır ayrıştırıcıları ──────────────────────────────────────────────────────

fn baslik_satiri(satir: &str, kayit: &mut GenBankKaydi) {
    if let Some(geri) = satir.strip_prefix("LOCUS") {
        let alanlar: Vec<&str> = geri.split_whitespace().collect();
        if let Some(ad) = alanlar.first() {
            kayit.locus = ad.to_string();
        }
        // "<ad> <uzunluk> bp/aa <molekül> <topoloji> ..." — uzunluk = 'bp'/'aa'tan önceki sayı.
        for (i, a) in alanlar.iter().enumerate() {
            if (*a == "bp" || *a == "aa") && i > 0 {
                kayit.uzunluk = alanlar[i - 1].parse().unwrap_or(0);
                // molekül (varsa) bir sonraki alan.
                if let Some(mol) = alanlar.get(i + 1) {
                    if *mol == "linear" || *mol == "circular" {
                        kayit.topoloji = mol.to_string();
                    } else {
                        kayit.molekul = mol.to_string();
                    }
                }
                if let Some(top) = alanlar.get(i + 2) {
                    if *top == "linear" || *top == "circular" {
                        kayit.topoloji = top.to_string();
                    }
                }
                break;
            }
        }
    } else if let Some(geri) = satir.strip_prefix("DEFINITION") {
        kayit.tanim = geri.trim().to_string();
    } else if let Some(geri) = satir.strip_prefix("ACCESSION") {
        kayit.erisim = geri.split_whitespace().next().unwrap_or("").to_string();
    }
}

/// FEATURES bölümünde bir satır: yeni özellik (sütun 5'te tür) / nitelik (`/k=v`) / devam.
fn ozellik_satiri(
    satir: &str,
    kayit: &mut GenBankKaydi,
    acik_nitelik: &mut Option<(String, String)>,
) {
    let baytlar = satir.as_bytes();
    // Yeni özellik: ilk 5 sütun boşluk, 6. sütun (index 5) boşluk DEĞİL.
    let yeni_ozellik = baytlar.len() > 5 && satir[..5].trim().is_empty() && baytlar[5] != b' ';

    if yeni_ozellik {
        nitelik_kapat(kayit, acik_nitelik);
        let geri = satir[5..].trim_end();
        let mut parcalar = geri.splitn(2, char::is_whitespace);
        let tur = parcalar.next().unwrap_or("").to_string();
        let konum = parcalar.next().unwrap_or("").trim().to_string();
        kayit.ozellikler.push(GenBankOzellik {
            tur,
            konum,
            nitelikler: Vec::new(),
        });
        return;
    }

    let govde = satir.trim_start();
    if let Some(nitelik) = govde.strip_prefix('/') {
        // Önceki açık niteliği kapat, yenisini başlat.
        nitelik_kapat(kayit, acik_nitelik);
        if let Some((anahtar, deger)) = nitelik.split_once('=') {
            let deger = deger.trim();
            if let Some(ic) = deger.strip_prefix('"') {
                if let Some(tam) = ic.strip_suffix('"') {
                    // Tek satırlık tırnaklı değer.
                    nitelik_ekle(kayit, anahtar, tam);
                } else {
                    // Açık tırnak → sonraki satırlarda devam eder.
                    *acik_nitelik = Some((anahtar.to_string(), ic.to_string()));
                }
            } else {
                // Tırnaksız değer (örn. /codon_start=1).
                nitelik_ekle(kayit, anahtar, deger);
            }
        } else {
            // Değersiz nitelik (örn. /pseudo).
            nitelik_ekle(kayit, nitelik.trim(), "");
        }
    } else if let Some((_, deger)) = acik_nitelik.as_mut() {
        // Açık (çok-satırlı) nitelik değeri devam ediyor.
        let parca = govde.trim_end();
        if let Some(kapali) = parca.strip_suffix('"') {
            deger.push(' ');
            deger.push_str(kapali);
            nitelik_kapat(kayit, acik_nitelik);
        } else {
            deger.push(' ');
            deger.push_str(parca);
        }
    } else if let Some(son) = kayit.ozellikler.last_mut() {
        // Çok-satırlı konum devamı.
        son.konum.push_str(govde.trim_end());
    }
}

fn dizi_satiri(satir: &str, kayit: &mut GenBankKaydi) {
    for &b in satir.as_bytes() {
        if b.is_ascii_alphabetic() {
            kayit.dizi.push(b);
        }
    }
}

// ─── Nitelik yardımcıları ───────────────────────────────────────────────────────

fn nitelik_ekle(kayit: &mut GenBankKaydi, anahtar: &str, deger: &str) {
    if let Some(son) = kayit.ozellikler.last_mut() {
        son.nitelikler
            .push((anahtar.trim().to_string(), deger.to_string()));
    }
}

fn nitelik_kapat(kayit: &mut GenBankKaydi, acik: &mut Option<(String, String)>) {
    if let Some((anahtar, deger)) = acik.take() {
        if let Some(son) = kayit.ozellikler.last_mut() {
            son.nitelikler.push((anahtar, deger));
        }
    }
}

fn io_hatasi(yol: &Path, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "GenBank dosyası okunamadı",
        format!("'{}' okunurken hata oluştu", yol.display()),
        "Dosya yolunu, okuma iznini ve dosyanın geçerli GenBank olduğunu kontrol edin",
    )
    .with_teknik_detay(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    fn yaz(ad: &str, icerik: &[u8]) -> PathBuf {
        let mut yol = std::env::temp_dir();
        yol.push(format!("biocraft_gb_{}_{ad}", std::process::id()));
        File::create(&yol).unwrap().write_all(icerik).unwrap();
        yol
    }

    const GB: &[u8] = b"\
LOCUS       TESTSEQ                 28 bp    DNA     linear   SYN 23-JUN-2026
DEFINITION  Sentetik test dizisi.
ACCESSION   TEST001
VERSION     TEST001.1
FEATURES             Location/Qualifiers
     source          1..28
                     /organism=\"synthetic construct\"
     gene            1..28
                     /gene=\"foo\"
     CDS             1..28
                     /gene=\"foo\"
                     /product=\"hypothetical protein with a very
                     long name spanning lines\"
                     /codon_start=1
ORIGIN
        1 acgtacgtac gtacgtacgt acgtacgt
//
";

    #[test]
    fn locus_dizi_ve_ozellik_okunur() {
        let p = yaz("a.gb", GB);
        let kayit = ilk_kayit(&p).unwrap();
        assert_eq!(kayit.locus, "TESTSEQ");
        assert_eq!(kayit.uzunluk, 28);
        assert_eq!(kayit.molekul, "DNA");
        assert_eq!(kayit.topoloji, "linear");
        assert_eq!(kayit.tanim, "Sentetik test dizisi.");
        assert_eq!(kayit.erisim, "TEST001");
        // Dizi: 28 baz, numara/boşluk arındırılmış.
        assert_eq!(kayit.dizi.len(), 28);
        assert_eq!(&kayit.dizi[..4], b"acgt");
        // Özellikler: source, gene, CDS.
        assert_eq!(kayit.ozellikler.len(), 3);
        assert_eq!(kayit.ozellikler[0].tur, "source");
        assert_eq!(kayit.ozellikler[0].konum, "1..28");
        assert_eq!(kayit.ozellikler[1].tur, "gene");
        assert_eq!(
            kayit.ozellikler[1].nitelikler,
            vec![("gene".to_string(), "foo".to_string())]
        );
        // CDS çok-satırlı /product birleştirilir.
        let cds = &kayit.ozellikler[2];
        assert_eq!(cds.tur, "CDS");
        let urun = cds
            .nitelikler
            .iter()
            .find(|(k, _)| k == "product")
            .map(|(_, v)| v.as_str());
        assert_eq!(
            urun,
            Some("hypothetical protein with a very long name spanning lines")
        );
        // Tırnaksız nitelik.
        assert!(cds
            .nitelikler
            .iter()
            .any(|(k, v)| k == "codon_start" && v == "1"));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn coklu_kayit_akisi() {
        let mut iki = GB.to_vec();
        iki.extend_from_slice(GB);
        let p = yaz("multi.gb", &iki);
        let mut adlar = vec![];
        let sayi = genbank_akis(&p, |k| adlar.push(k.locus.clone())).unwrap();
        assert_eq!(sayi, 2);
        assert_eq!(adlar, vec!["TESTSEQ", "TESTSEQ"]);
        let _ = std::fs::remove_file(&p);
    }
}
