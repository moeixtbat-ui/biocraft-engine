//! ÇE-01 — **PDB / mmCIF** 3B yapı okuma — **saf-Rust** (yeni dış bağımlılık yok).
//!
//! Atom/zincir/model çıkarılır → **Gün 39 (ÇE-07) 3B görüntüleyici** kullanır.
//!
//! * **PDB:** sabit-sütun metin; `MODEL`/`ENDMDL` modelleri, `ATOM`/`HETATM` atomları, `TER`
//!   zincir sonu.  Sütunlar PDB 3.x spesifikasyonuna göre dilimlenir.
//! * **mmCIF (PDBx):** `_atom_site` döngüsü (loop) ayrıştırılır; sütun adları başlıktan eşlenir,
//!   satırlar (tırnak-farkında) çözülür; `pdbx_PDB_model_num` ile modellere ayrılır.
//!
//! `pdbtbx` gibi bir crate **eklenmez**; atom_site/sabit-sütun ayrıştırma elle yazılabilir.
//! Tüm yapı belleğe alınır (3B görüntüleyicinin ihtiyacı budur) ama **bütçe** önce denetlenir.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use biocraft_sdk::biocraft_types::ErrorReport;

use super::budget::BellekButcesi;
use super::detect::{formati_belirle, VeriFormati};

/// Tek bir atom.
#[derive(Debug, Clone, PartialEq)]
pub struct Atom {
    /// Seri numarası.
    pub seri: i64,
    /// Atom adı (örn. "CA", "N").
    pub ad: String,
    /// Kalıntı (residue) adı (örn. "MET").
    pub kalinti: String,
    /// Zincir kimliği (örn. "A").
    pub zincir: String,
    /// Kalıntı sıra numarası.
    pub kalinti_no: i64,
    /// Koordinatlar (Ångström).
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Element sembolü (örn. "C", "N", "O").
    pub element: String,
    /// HETATM (heteroatom: ligand/su) mu?
    pub hetatm: bool,
}

/// Bir model (NMR yapılarında birden çok olabilir).
#[derive(Debug, Clone, PartialEq)]
pub struct YapiModeli {
    /// Model numarası (PDB MODEL / mmCIF pdbx_PDB_model_num; tek model → 1).
    pub model_no: i64,
    /// Modelin atomları.
    pub atomlar: Vec<Atom>,
}

/// Çözülmüş 3B yapı.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Yapi {
    /// Format etiketi (PDB/mmCIF).
    pub format: &'static str,
    /// Modeller.
    pub modeller: Vec<YapiModeli>,
}

impl Yapi {
    /// Bir yapı dosyasını açar (format otomatik tanınır; PDB/mmCIF olmalı).  `bütçe` atom
    /// belleğini sınırlar (İP-08).
    pub fn oku(yol: &Path, butce: &BellekButcesi) -> Result<Self, ErrorReport> {
        let format = formati_belirle(yol)?;
        match format {
            VeriFormati::Pdb => pdb_oku(yol, butce),
            VeriFormati::MmCif => mmcif_oku(yol, butce),
            _ => Err(ErrorReport::new(
                "Yapı dosyası değil",
                format!("'{}' bir PDB/mmCIF dosyası değil", yol.display()),
                "PDB (.pdb/.ent) veya mmCIF (.cif) uzantılı bir dosya seçin",
            )),
        }
    }

    /// Toplam atom sayısı (tüm modeller).
    pub fn atom_sayisi(&self) -> usize {
        self.modeller.iter().map(|m| m.atomlar.len()).sum()
    }

    /// Model sayısı.
    pub fn model_sayisi(&self) -> usize {
        self.modeller.len()
    }

    /// İlk modeldeki benzersiz zincir kimlikleri (görünme sırasıyla).
    pub fn zincirler(&self) -> Vec<String> {
        let mut gorulen = Vec::new();
        if let Some(m) = self.modeller.first() {
            for a in &m.atomlar {
                if !gorulen.contains(&a.zincir) {
                    gorulen.push(a.zincir.clone());
                }
            }
        }
        gorulen
    }
}

/// Bir atomun kaba bayt tahmini (bütçe muhasebesi).
fn atom_bayt(a: &Atom) -> u64 {
    (64 + a.ad.len() + a.kalinti.len() + a.zincir.len() + a.element.len()) as u64
}

// ─── PDB ────────────────────────────────────────────────────────────────────────

fn pdb_oku(yol: &Path, butce: &BellekButcesi) -> Result<Yapi, ErrorReport> {
    let dosya = File::open(yol).map_err(|e| io_hatasi(yol, &e))?;
    let okuyucu = BufReader::new(dosya);

    let mut modeller: Vec<YapiModeli> = Vec::new();
    let mut aktif: Option<YapiModeli> = None;
    let mut tahmini: u64 = 0;

    for satir_sonuc in okuyucu.lines() {
        let satir = satir_sonuc.map_err(|e| io_hatasi(yol, &e))?;
        let kayit = satir.get(..6).unwrap_or("").trim_end();

        match kayit {
            "MODEL" => {
                if let Some(m) = aktif.take() {
                    modeller.push(m);
                }
                let no = satir
                    .get(10..14)
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or((modeller.len() + 1) as i64);
                aktif = Some(YapiModeli {
                    model_no: no,
                    atomlar: Vec::new(),
                });
            }
            "ENDMDL" => {
                if let Some(m) = aktif.take() {
                    modeller.push(m);
                }
            }
            "ATOM" | "HETATM" => {
                let atom = pdb_atom(&satir, kayit == "HETATM", yol)?;
                tahmini += atom_bayt(&atom);
                butce.kontrol(tahmini)?;
                aktif
                    .get_or_insert_with(|| YapiModeli {
                        model_no: 1,
                        atomlar: Vec::new(),
                    })
                    .atomlar
                    .push(atom);
            }
            _ => {} // TER/HEADER/REMARK/… atlanır
        }
    }
    if let Some(m) = aktif.take() {
        modeller.push(m);
    }

    if modeller.iter().all(|m| m.atomlar.is_empty()) {
        return Err(bos_yapi(yol, "PDB"));
    }
    Ok(Yapi {
        format: "PDB",
        modeller,
    })
}

/// Bir PDB ATOM/HETATM satırını ayrıştırır (sabit sütun).
fn pdb_atom(satir: &str, hetatm: bool, yol: &Path) -> Result<Atom, ErrorReport> {
    let alan = |a: usize, b: usize| satir.get(a..b).unwrap_or("").trim().to_string();
    let koord = |a: usize, b: usize| -> Result<f32, ErrorReport> {
        satir
            .get(a..b)
            .unwrap_or("")
            .trim()
            .parse::<f32>()
            .map_err(|_| bozuk_satir(yol, "PDB koordinatı"))
    };

    let seri = alan(6, 11).parse().unwrap_or(0);
    let ad = alan(12, 16);
    let kalinti = alan(17, 20);
    let zincir = {
        let z = alan(21, 22);
        if z.is_empty() {
            "A".to_string()
        } else {
            z
        }
    };
    let kalinti_no = alan(22, 26).parse().unwrap_or(0);
    let x = koord(30, 38)?;
    let y = koord(38, 46)?;
    let z = koord(46, 54)?;
    // Element sütunu (77-78) yoksa atom adının baş harfinden tahmin et.
    let element = {
        let e = alan(76, 78);
        if e.is_empty() {
            ad.chars()
                .find(|c| c.is_ascii_alphabetic())
                .map(|c| c.to_ascii_uppercase().to_string())
                .unwrap_or_default()
        } else {
            e
        }
    };

    Ok(Atom {
        seri,
        ad,
        kalinti,
        zincir,
        kalinti_no,
        x,
        y,
        z,
        element,
        hetatm,
    })
}

// ─── mmCIF ──────────────────────────────────────────────────────────────────────

fn mmcif_oku(yol: &Path, butce: &BellekButcesi) -> Result<Yapi, ErrorReport> {
    let dosya = File::open(yol).map_err(|e| io_hatasi(yol, &e))?;
    let okuyucu = BufReader::new(dosya);

    // _atom_site döngüsünün sütun adları → indeks.
    let mut sutunlar: Vec<String> = Vec::new();
    let mut loop_icinde = false; // `loop_` görüldü
    let mut atom_site_loop = false; // bu loop _atom_site mı
    let mut basliklari_okuyor = false;

    let mut modeller: Vec<YapiModeli> = Vec::new();
    let mut model_index: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
    let mut tahmini: u64 = 0;

    for satir_sonuc in okuyucu.lines() {
        let satir = satir_sonuc.map_err(|e| io_hatasi(yol, &e))?;
        let kirp = satir.trim();

        if kirp == "loop_" {
            loop_icinde = true;
            atom_site_loop = false;
            basliklari_okuyor = false;
            sutunlar.clear();
            continue;
        }

        if loop_icinde && kirp.starts_with("_atom_site.") {
            atom_site_loop = true;
            basliklari_okuyor = true;
            sutunlar.push(kirp.trim_start_matches("_atom_site.").to_string());
            continue;
        }

        // Başka bir kategori başlığı / başka loop → atom_site döngüsünden çık.
        if kirp.starts_with('_') && !kirp.starts_with("_atom_site.") {
            loop_icinde = false;
            atom_site_loop = false;
            basliklari_okuyor = false;
            continue;
        }

        if atom_site_loop && basliklari_okuyor {
            // Başlıklardan sonraki ilk veri satırı.
            if kirp.is_empty() || kirp.starts_with('#') {
                continue;
            }
            basliklari_okuyor = false;
        }

        if atom_site_loop && !basliklari_okuyor {
            if kirp.is_empty() || kirp == "#" || kirp.starts_with("data_") {
                if kirp.starts_with("data_") {
                    atom_site_loop = false;
                    loop_icinde = false;
                }
                continue;
            }
            if kirp.starts_with("loop_") || kirp.starts_with('_') {
                // Sonraki kategori; bu döngü bitti.
                atom_site_loop = false;
                loop_icinde = false;
                continue;
            }
            // Veri satırı.
            let degerler = mmcif_jetonla(kirp);
            if degerler.len() < sutunlar.len() {
                continue; // eksik satır → atla (savunmacı)
            }
            if let Some(atom) = mmcif_atom(&sutunlar, &degerler) {
                let model_no = atom.0;
                let a = atom.1;
                tahmini += atom_bayt(&a);
                butce.kontrol(tahmini)?;
                let idx = *model_index.entry(model_no).or_insert_with(|| {
                    modeller.push(YapiModeli {
                        model_no,
                        atomlar: Vec::new(),
                    });
                    modeller.len() - 1
                });
                modeller[idx].atomlar.push(a);
            }
        }
    }

    if modeller.is_empty() || modeller.iter().all(|m| m.atomlar.is_empty()) {
        return Err(bos_yapi(yol, "mmCIF"));
    }
    Ok(Yapi {
        format: "mmCIF",
        modeller,
    })
}

/// Bir mmCIF veri satırını jetonlara böler (basit tek/çift tırnak farkındalığı).
fn mmcif_jetonla(satir: &str) -> Vec<String> {
    let mut jetonlar = Vec::new();
    let mut gecerli = String::new();
    let mut tirnak: Option<char> = None;

    for c in satir.chars() {
        match tirnak {
            Some(t) => {
                if c == t {
                    tirnak = None;
                    jetonlar.push(std::mem::take(&mut gecerli));
                } else {
                    gecerli.push(c);
                }
            }
            None => {
                if c == '\'' || c == '"' {
                    tirnak = Some(c);
                } else if c.is_whitespace() {
                    if !gecerli.is_empty() {
                        jetonlar.push(std::mem::take(&mut gecerli));
                    }
                } else {
                    gecerli.push(c);
                }
            }
        }
    }
    if !gecerli.is_empty() {
        jetonlar.push(gecerli);
    }
    jetonlar
}

/// Sütun adlarına göre bir mmCIF atom satırını çözer → (model_no, Atom).  Eksik/geçersizse `None`.
fn mmcif_atom(sutunlar: &[String], degerler: &[String]) -> Option<(i64, Atom)> {
    let al = |ad: &str| -> Option<&str> {
        sutunlar
            .iter()
            .position(|s| s == ad)
            .and_then(|i| degerler.get(i))
            .map(|s| s.as_str())
    };
    let al_ilk = |adlar: &[&str]| -> Option<&str> { adlar.iter().find_map(|a| al(a)) };

    let grup = al("group_PDB").unwrap_or("ATOM");
    let x = al("Cartn_x")?.parse().ok()?;
    let y = al("Cartn_y")?.parse().ok()?;
    let z = al("Cartn_z")?.parse().ok()?;
    let ad = al_ilk(&["label_atom_id", "auth_atom_id"])
        .unwrap_or("")
        .trim_matches('"')
        .to_string();
    let kalinti = al_ilk(&["label_comp_id", "auth_comp_id"])
        .unwrap_or("")
        .to_string();
    let zincir = al_ilk(&["label_asym_id", "auth_asym_id"])
        .unwrap_or("A")
        .to_string();
    let kalinti_no = al_ilk(&["label_seq_id", "auth_seq_id"])
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let element = al("type_symbol").unwrap_or("").to_string();
    let seri = al("id").and_then(|s| s.parse().ok()).unwrap_or(0);
    let model_no = al("pdbx_PDB_model_num")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    Some((
        model_no,
        Atom {
            seri,
            ad,
            kalinti,
            zincir,
            kalinti_no,
            x,
            y,
            z,
            element,
            hetatm: grup == "HETATM",
        },
    ))
}

// ─── Hatalar ────────────────────────────────────────────────────────────────────

fn io_hatasi(yol: &Path, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "Yapı dosyası okunamadı",
        format!("'{}' okunurken hata oluştu", yol.display()),
        "Dosya yolunu ve okuma iznini kontrol edin",
    )
    .with_teknik_detay(e.to_string())
}

fn bozuk_satir(yol: &Path, baglam: &str) -> ErrorReport {
    ErrorReport::new(
        "Yapı satırı ayrıştırılamadı",
        format!(
            "'{}' içinde {baglam} okunamadı (bozuk biçim)",
            yol.display()
        ),
        "Dosyanın geçerli bir PDB/mmCIF olduğundan emin olun",
    )
}

fn bos_yapi(yol: &Path, format: &str) -> ErrorReport {
    ErrorReport::new(
        format!("{format} içinde atom bulunamadı"),
        format!("'{}' hiçbir ATOM/HETATM kaydı içermiyor", yol.display()),
        "Dosyanın koordinat (atom) verisi içerdiğinden emin olun",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    fn yaz(ad: &str, icerik: &[u8]) -> PathBuf {
        let mut yol = std::env::temp_dir();
        yol.push(format!("biocraft_struct_{}_{ad}", std::process::id()));
        File::create(&yol).unwrap().write_all(icerik).unwrap();
        yol
    }

    // Gerçek PDB sütun hizalı (2 atom, zincir A + ligand).
    const PDB: &[u8] = b"\
HEADER    TEST                                    23-JUN-26   TEST
ATOM      1  N   MET A   1      11.104  13.207  10.567  1.00 20.00           N
ATOM      2  CA  MET A   1      12.560  13.000  10.420  1.00 20.00           C
HETATM    3  O   HOH B   2       5.000   5.000   5.000  1.00 30.00           O
END
";

    #[test]
    fn pdb_atom_zincir_okur() {
        let p = yaz("a.pdb", PDB);
        let y = Yapi::oku(&p, &BellekButcesi::sinirsiz()).unwrap();
        assert_eq!(y.format, "PDB");
        assert_eq!(y.model_sayisi(), 1);
        assert_eq!(y.atom_sayisi(), 3);
        let m = &y.modeller[0];
        assert_eq!(m.atomlar[0].ad, "N");
        assert_eq!(m.atomlar[0].kalinti, "MET");
        assert_eq!(m.atomlar[0].zincir, "A");
        assert_eq!(m.atomlar[0].element, "N");
        assert!((m.atomlar[0].x - 11.104).abs() < 1e-3);
        assert!(m.atomlar[2].hetatm);
        assert_eq!(y.zincirler(), vec!["A", "B"]);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn pdb_coklu_model() {
        let pdb = b"\
MODEL        1
ATOM      1  CA  ALA A   1       0.000   0.000   0.000  1.00  0.00           C
ENDMDL
MODEL        2
ATOM      1  CA  ALA A   1       1.000   1.000   1.000  1.00  0.00           C
ENDMDL
";
        let p = yaz("m.pdb", pdb);
        let y = Yapi::oku(&p, &BellekButcesi::sinirsiz()).unwrap();
        assert_eq!(y.model_sayisi(), 2);
        assert_eq!(y.modeller[1].model_no, 2);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn mmcif_atom_site_okur() {
        let cif = b"\
data_TEST
#
loop_
_atom_site.group_PDB
_atom_site.id
_atom_site.type_symbol
_atom_site.label_atom_id
_atom_site.label_comp_id
_atom_site.label_asym_id
_atom_site.label_seq_id
_atom_site.Cartn_x
_atom_site.Cartn_y
_atom_site.Cartn_z
_atom_site.pdbx_PDB_model_num
ATOM 1 N N MET A 1 11.104 13.207 10.567 1
ATOM 2 C CA MET A 1 12.560 13.000 10.420 1
HETATM 3 O O HOH B 2 5.000 5.000 5.000 1
#
";
        let p = yaz("a.cif", cif);
        let y = Yapi::oku(&p, &BellekButcesi::sinirsiz()).unwrap();
        assert_eq!(y.format, "mmCIF");
        assert_eq!(y.atom_sayisi(), 3);
        assert_eq!(y.modeller[0].atomlar[0].kalinti, "MET");
        assert_eq!(y.modeller[0].atomlar[0].element, "N");
        assert!((y.modeller[0].atomlar[1].x - 12.560).abs() < 1e-3);
        assert!(y.modeller[0].atomlar[2].hetatm);
        assert_eq!(y.zincirler(), vec!["A", "B"]);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn butce_asiminda_reddeder() {
        let p = yaz("b.pdb", PDB);
        let hata = Yapi::oku(&p, &BellekButcesi::yeni(10)).err().unwrap();
        assert_eq!(hata.ne_oldu, "Bellek bütçesi aşıldı");
        let _ = std::fs::remove_file(&p);
    }
}
