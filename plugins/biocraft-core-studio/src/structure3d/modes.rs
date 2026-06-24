//! ÇE-07 — **Gösterim modları + renklendirme**: bir [`YapiModeli`]'ni render-bağımsız bir
//! [`Sahne3B`]'ye derler.
//!
//! * **Gösterim modları:** kartonet (cartoon/omurga izi), top-çubuk (ball-and-stick), çubuk (stick),
//!   dolgu (space-filling/VDW).  (Gerçek çözücü-dışlanan **yüzey** [SES] v1.x — `MVP-sonrasi.md`.)
//! * **Bağ çıkarımı:** mesafe + **kovalent yarıçap** toplamı ölçütüyle (uzaysal ızgara → ~O(n)).
//! * **İkincil yapı:** Cα izinden **P-SEA mesafe ölçütleri** ile heliks/yaprak/ilmek (yaklaşık;
//!   gerçek DSSP H-bağı analizi değil — dürüst sınır).
//! * **Renklendirme:** zincire / ikincil yapıya / elemana (CPK) / B-faktöre göre.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::data_io::{Atom, YapiModeli};

use super::render::{Kure, Renk3B, Sahne3B, Serit, Silindir, Vec3};

/// Bir atomun 3B konumu.
fn konum(a: &Atom) -> Vec3 {
    Vec3::yeni(a.x, a.y, a.z)
}

/// Su molekülü (gösterim dışı bırakılabilir).
fn su_mu(a: &Atom) -> bool {
    matches!(a.kalinti.as_str(), "HOH" | "WAT" | "DOD" | "H2O")
}

// ─── Gösterim modu + renk şeması ──────────────────────────────────────────────────

/// 3B gösterim biçimi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GosterimModu {
    /// Kartonet/karikatür — omurga (Cα/P) izi (tüp).  Büyük yapıda hızlı, genel kıvrımı gösterir.
    Kartonet,
    /// Top-çubuk (ball-and-stick) — atom küreleri + kovalent bağ silindirleri.
    TopCubuk,
    /// Çubuk (stick) — yalnız bağlar (uçlarda küçük küre).
    Cubuk,
    /// Dolgu (space-filling/VDW) — Van der Waals küreleri.
    Dolgu,
}

impl GosterimModu {
    /// Kullanıcıya görünen ad.
    pub fn etiket(self) -> &'static str {
        match self {
            GosterimModu::Kartonet => "Kartonet (omurga)",
            GosterimModu::TopCubuk => "Top-Çubuk",
            GosterimModu::Cubuk => "Çubuk",
            GosterimModu::Dolgu => "Dolgu (VDW)",
        }
    }
}

/// Renklendirme şeması.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenkSemasi {
    /// Zincire göre (her zincir farklı renk).
    Zincir,
    /// İkincil yapıya göre (heliks/yaprak/ilmek).
    IkincilYapi,
    /// Elemana göre (CPK).
    Eleman,
    /// B-faktöre göre (mavi→kırmızı rampası).
    BFaktor,
}

impl RenkSemasi {
    /// Kullanıcıya görünen ad.
    pub fn etiket(self) -> &'static str {
        match self {
            RenkSemasi::Zincir => "Zincir",
            RenkSemasi::IkincilYapi => "İkincil yapı",
            RenkSemasi::Eleman => "Eleman (CPK)",
            RenkSemasi::BFaktor => "B-faktör",
        }
    }
}

/// İkincil yapı sınıfı (yaklaşık).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IkincilYapi {
    /// α-heliks.
    Heliks,
    /// β-yaprak (strand).
    Yaprak,
    /// İlmek/sarmal (coil).
    Sarmal,
}

impl IkincilYapi {
    fn renk(self) -> Renk3B {
        match self {
            IkincilYapi::Heliks => Renk3B::Heliks,
            IkincilYapi::Yaprak => Renk3B::Yaprak,
            IkincilYapi::Sarmal => Renk3B::Sarmal,
        }
    }
}

// ─── Element tabloları (CPK rengi + kovalent/VDW yarıçap) ─────────────────────────

fn eleman_norm(el: &str) -> String {
    el.trim().to_ascii_uppercase()
}

/// Element → anlamsal CPK rengi.
pub fn eleman_rengi(el: &str) -> Renk3B {
    match eleman_norm(el).as_str() {
        "C" => Renk3B::ElemC,
        "N" => Renk3B::ElemN,
        "O" => Renk3B::ElemO,
        "S" => Renk3B::ElemS,
        "P" => Renk3B::ElemP,
        "H" | "D" => Renk3B::ElemH,
        "F" | "CL" | "BR" | "I" => Renk3B::ElemHalojen,
        "NA" | "K" | "MG" | "CA" | "FE" | "ZN" | "MN" | "CU" | "NI" | "CO" => Renk3B::ElemMetal,
        _ => Renk3B::ElemDiger,
    }
}

/// Kovalent yarıçap (Ångström) — bağ çıkarımı için.
fn kovalent_yaricap(el: &str) -> f32 {
    match eleman_norm(el).as_str() {
        "H" | "D" => 0.31,
        "C" => 0.76,
        "N" => 0.71,
        "O" => 0.66,
        "S" => 1.05,
        "P" => 1.07,
        "F" => 0.57,
        "CL" => 1.02,
        "BR" => 1.20,
        "I" => 1.39,
        "FE" => 1.32,
        "ZN" => 1.22,
        "MG" => 1.41,
        "CA" => 1.76,
        "NA" => 1.66,
        "K" => 2.03,
        _ => 0.77,
    }
}

/// Van der Waals yarıçap (Ångström) — dolgu (space-filling) gösterimi için.
fn vdw_yaricap(el: &str) -> f32 {
    match eleman_norm(el).as_str() {
        "H" | "D" => 1.10,
        "C" => 1.70,
        "N" => 1.55,
        "O" => 1.52,
        "S" => 1.80,
        "P" => 1.80,
        "F" => 1.47,
        "CL" => 1.75,
        "BR" => 1.85,
        "I" => 1.98,
        "FE" | "ZN" | "MG" | "CA" | "NA" | "K" | "MN" | "CU" | "NI" | "CO" => 1.90,
        _ => 1.70,
    }
}

// ─── Bağ çıkarımı (uzaysal ızgara) ────────────────────────────────────────────────

/// İki atomun bağlı sayılma toleransı (Ångström).
const BAG_TOLERANS: f32 = 0.45;

/// Görünür atom kümesi için kovalent bağları (model atom indeks çiftleri) çıkarır.
/// Uzaysal ızgara: hücre = 4 Å; her atom 27 komşu hücreyi tarar → tipik yapıda ~O(n).
pub fn baglari_bul(model: &YapiModeli, gorunur: &[usize]) -> Vec<(usize, usize)> {
    const HUC: f32 = 4.0;
    let hucre = |p: Vec3| {
        (
            (p.x / HUC).floor() as i32,
            (p.y / HUC).floor() as i32,
            (p.z / HUC).floor() as i32,
        )
    };
    let mut izgara: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
    for &i in gorunur {
        izgara
            .entry(hucre(konum(&model.atomlar[i])))
            .or_default()
            .push(i);
    }

    let mut baglar = Vec::new();
    for &i in gorunur {
        let ai = &model.atomlar[i];
        let pi = konum(ai);
        let ri = kovalent_yaricap(&ai.element);
        let (cx, cy, cz) = hucre(pi);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let Some(liste) = izgara.get(&(cx + dx, cy + dy, cz + dz)) else {
                        continue;
                    };
                    for &j in liste {
                        if j <= i {
                            continue; // her çifti bir kez (ve kendiyle bağ yok)
                        }
                        let aj = &model.atomlar[j];
                        let d = pi.uzaklik(konum(aj));
                        let esik = ri + kovalent_yaricap(&aj.element) + BAG_TOLERANS;
                        if d > 0.4 && d <= esik {
                            baglar.push((i, j));
                        }
                    }
                }
            }
        }
    }
    baglar
}

// ─── İkincil yapı (P-SEA mesafe ölçütleri — yaklaşık) ─────────────────────────────

fn yakin(v: f32, hedef: f32, tol: f32) -> bool {
    (v - hedef).abs() <= tol
}

/// Cα izinden ikincil yapıyı (yaklaşık) atar.  Anahtar `(zincir, kalıntı_no)`.
///
/// P-SEA'nın Cα mesafe ölçütleri (d2/d3/d4 = `Cα(i)`–`Cα(i+2/3/4)`):
/// heliks ≈ (5.5, 5.3, 6.4); yaprak ≈ (6.7, 9.9, 12.4).  Gerçek DSSP **değil** (yaklaşık).
pub fn ikincil_yapi(model: &YapiModeli) -> HashMap<(String, i64), IkincilYapi> {
    let mut zincir_ca: BTreeMap<String, Vec<(i64, Vec3)>> = BTreeMap::new();
    for a in &model.atomlar {
        if !a.hetatm && a.ad == "CA" {
            zincir_ca
                .entry(a.zincir.clone())
                .or_default()
                .push((a.kalinti_no, konum(a)));
        }
    }

    let mut harita: HashMap<(String, i64), IkincilYapi> = HashMap::new();
    for (zincir, mut ca) in zincir_ca {
        ca.sort_by_key(|(no, _)| *no);
        if ca.len() < 5 {
            continue;
        }
        for i in 0..ca.len() - 4 {
            let d2 = ca[i].1.uzaklik(ca[i + 2].1);
            let d3 = ca[i].1.uzaklik(ca[i + 3].1);
            let d4 = ca[i].1.uzaklik(ca[i + 4].1);
            let sinif = if yakin(d2, 5.5, 0.5) && yakin(d3, 5.3, 0.5) && yakin(d4, 6.4, 0.7) {
                Some(IkincilYapi::Heliks)
            } else if yakin(d2, 6.7, 0.6) && yakin(d3, 9.9, 0.9) && yakin(d4, 12.4, 1.1) {
                Some(IkincilYapi::Yaprak)
            } else {
                None
            };
            if let Some(s) = sinif {
                // Pencere kalıntılarını işaretle (heliks yaprağı ezmesin: yalnız boşları doldur).
                for (no, _) in &ca[i..=i + 4] {
                    harita.entry((zincir.clone(), *no)).or_insert(s);
                }
            }
        }
    }
    harita
}

// ─── Sahne derleme ────────────────────────────────────────────────────────────────

/// Sahne derleme ayarı.
#[derive(Debug, Clone, Copy)]
pub struct SahneAyar {
    pub mod_: GosterimModu,
    pub sema: RenkSemasi,
    /// Su (HOH/WAT) molekülleri gösterilsin mi?
    pub su_goster: bool,
    /// Büyük yapı sadeleştirmesi: yalnız omurga izi çiz (fallback — performans + uyarı).
    pub yalniz_omurga: bool,
}

impl Default for SahneAyar {
    fn default() -> Self {
        Self {
            mod_: GosterimModu::Kartonet,
            sema: RenkSemasi::Zincir,
            su_goster: false,
            yalniz_omurga: false,
        }
    }
}

/// Bir model + ayar + gizli zincirlerden render-bağımsız [`Sahne3B`] üretir.
pub fn sahne_olustur(
    model: &YapiModeli,
    ayar: SahneAyar,
    gizli_zincirler: &BTreeSet<String>,
) -> Sahne3B {
    // Görünür atom indeksleri (gizli zincir / su filtresi).
    let gorunur: Vec<usize> = model
        .atomlar
        .iter()
        .enumerate()
        .filter(|(_, a)| !gizli_zincirler.contains(&a.zincir))
        .filter(|(_, a)| ayar.su_goster || !su_mu(a))
        .map(|(i, _)| i)
        .collect();

    // Zincir → indeks (zincir rengi için, görünme sırasına göre).
    let mut zincir_idx: HashMap<String, usize> = HashMap::new();
    for &i in &gorunur {
        let z = &model.atomlar[i].zincir;
        if !zincir_idx.contains_key(z) {
            let n = zincir_idx.len();
            zincir_idx.insert(z.clone(), n);
        }
    }

    let ss = if ayar.sema == RenkSemasi::IkincilYapi || ayar.mod_ == GosterimModu::Kartonet {
        ikincil_yapi(model)
    } else {
        HashMap::new()
    };

    // B-faktör aralığı (rampa normalizasyonu).
    let (b_min, b_max) = b_araligi(model, &gorunur);
    let renk = |a: &Atom| -> Renk3B { atom_rengi(a, ayar.sema, &zincir_idx, &ss, b_min, b_max) };

    let mut sahne = Sahne3B::yeni();

    // Büyük yapı sadeleştirmesi: her modda yalnız omurga izi (Serit).
    if ayar.yalniz_omurga {
        omurga_seritleri(model, &gorunur, &renk, &mut sahne);
        return sahne;
    }

    match ayar.mod_ {
        GosterimModu::Dolgu => {
            for &i in &gorunur {
                let a = &model.atomlar[i];
                sahne.kureler.push(Kure {
                    merkez: konum(a),
                    yaricap: vdw_yaricap(&a.element),
                    renk: renk(a),
                    atom_indeksi: Some(i),
                });
            }
        }
        GosterimModu::TopCubuk => {
            for &i in &gorunur {
                let a = &model.atomlar[i];
                sahne.kureler.push(Kure {
                    merkez: konum(a),
                    yaricap: 0.38,
                    renk: renk(a),
                    atom_indeksi: Some(i),
                });
            }
            bag_silindirleri(
                model,
                &baglari_bul(model, &gorunur),
                &renk,
                0.16,
                &mut sahne,
            );
        }
        GosterimModu::Cubuk => {
            // Uçlarda küçük küre (tek atomlar görünür kalsın) + bağ silindirleri.
            for &i in &gorunur {
                let a = &model.atomlar[i];
                sahne.kureler.push(Kure {
                    merkez: konum(a),
                    yaricap: 0.20,
                    renk: renk(a),
                    atom_indeksi: Some(i),
                });
            }
            bag_silindirleri(
                model,
                &baglari_bul(model, &gorunur),
                &renk,
                0.20,
                &mut sahne,
            );
        }
        GosterimModu::Kartonet => {
            omurga_seritleri(model, &gorunur, &renk, &mut sahne);
            // Ligandlar (su olmayan HETATM) top-çubuk olarak görünür.
            let ligand: Vec<usize> = gorunur
                .iter()
                .copied()
                .filter(|&i| model.atomlar[i].hetatm)
                .collect();
            for &i in &ligand {
                let a = &model.atomlar[i];
                sahne.kureler.push(Kure {
                    merkez: konum(a),
                    yaricap: 0.38,
                    renk: renk(a),
                    atom_indeksi: Some(i),
                });
            }
            bag_silindirleri(model, &baglari_bul(model, &ligand), &renk, 0.16, &mut sahne);
        }
    }
    sahne
}

/// Her bağı orta noktadan ikiye bölüp uçların rengiyle iki silindir ekler (iki renkli çubuk).
fn bag_silindirleri(
    model: &YapiModeli,
    baglar: &[(usize, usize)],
    renk: &impl Fn(&Atom) -> Renk3B,
    yaricap: f32,
    sahne: &mut Sahne3B,
) {
    for &(i, j) in baglar {
        let ai = &model.atomlar[i];
        let aj = &model.atomlar[j];
        let pi = konum(ai);
        let pj = konum(aj);
        let orta = pi.topla(pj).olcekle(0.5);
        sahne.silindirler.push(Silindir {
            bas: pi,
            son: orta,
            yaricap,
            renk: renk(ai),
        });
        sahne.silindirler.push(Silindir {
            bas: orta,
            son: pj,
            yaricap,
            renk: renk(aj),
        });
    }
}

/// Zincir başına omurga (Cα ya da P) izini bir [`Serit`] olarak ekler (kalıntı sırasına göre).
fn omurga_seritleri(
    model: &YapiModeli,
    gorunur: &[usize],
    renk: &impl Fn(&Atom) -> Renk3B,
    sahne: &mut Sahne3B,
) {
    let gorunur_set: BTreeSet<usize> = gorunur.iter().copied().collect();
    let mut zincir_omurga: BTreeMap<String, Vec<(i64, usize)>> = BTreeMap::new();
    for &i in gorunur {
        let a = &model.atomlar[i];
        if !a.hetatm && (a.ad == "CA" || a.ad == "P") {
            zincir_omurga
                .entry(a.zincir.clone())
                .or_default()
                .push((a.kalinti_no, i));
        }
    }
    for (_zincir, mut omurga) in zincir_omurga {
        omurga.sort_by_key(|(no, _)| *no);
        let noktalar: Vec<Vec3> = omurga
            .iter()
            .filter(|(_, i)| gorunur_set.contains(i))
            .map(|(_, i)| konum(&model.atomlar[*i]))
            .collect();
        let renkler: Vec<Renk3B> = omurga
            .iter()
            .filter(|(_, i)| gorunur_set.contains(i))
            .map(|(_, i)| renk(&model.atomlar[*i]))
            .collect();
        if noktalar.len() >= 2 {
            sahne.seritler.push(Serit {
                noktalar,
                renkler,
                yaricap: 0.45,
            });
        }
    }
}

fn b_araligi(model: &YapiModeli, gorunur: &[usize]) -> (f32, f32) {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for &i in gorunur {
        let b = model.atomlar[i].b_faktor;
        min = min.min(b);
        max = max.max(b);
    }
    if !min.is_finite() || (max - min).abs() < 1e-6 {
        (0.0, 1.0)
    } else {
        (min, max)
    }
}

fn atom_rengi(
    a: &Atom,
    sema: RenkSemasi,
    zincir_idx: &HashMap<String, usize>,
    ss: &HashMap<(String, i64), IkincilYapi>,
    b_min: f32,
    b_max: f32,
) -> Renk3B {
    match sema {
        RenkSemasi::Zincir => Renk3B::Zincir(zincir_idx.get(&a.zincir).copied().unwrap_or(0) as u8),
        RenkSemasi::Eleman => eleman_rengi(&a.element),
        RenkSemasi::IkincilYapi => ss
            .get(&(a.zincir.clone(), a.kalinti_no))
            .map(|s| s.renk())
            .unwrap_or(Renk3B::Sarmal),
        RenkSemasi::BFaktor => {
            let t = ((a.b_faktor - b_min) / (b_max - b_min)).clamp(0.0, 1.0);
            Renk3B::BFaktor((t * 255.0) as u8)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_io::Atom;

    fn atom(ad: &str, el: &str, zincir: &str, no: i64, x: f32, y: f32, z: f32) -> Atom {
        Atom {
            seri: 0,
            ad: ad.into(),
            kalinti: "ALA".into(),
            zincir: zincir.into(),
            kalinti_no: no,
            x,
            y,
            z,
            element: el.into(),
            b_faktor: 0.0,
            hetatm: false,
        }
    }

    #[test]
    fn baglari_mesafeden_bulur() {
        // C-C ~1.5 Å (bağlı), C…C ~3.0 Å (bağsız).
        let model = YapiModeli {
            model_no: 1,
            atomlar: vec![
                atom("C1", "C", "A", 1, 0.0, 0.0, 0.0),
                atom("C2", "C", "A", 1, 1.5, 0.0, 0.0),
                atom("C3", "C", "A", 1, 4.5, 0.0, 0.0),
            ],
        };
        let baglar = baglari_bul(&model, &[0, 1, 2]);
        assert_eq!(baglar, vec![(0, 1)]); // yalnız 0-1 bağlı
    }

    #[test]
    fn dolgu_modu_vdw_kureleri() {
        let model = YapiModeli {
            model_no: 1,
            atomlar: vec![atom("O", "O", "A", 1, 0.0, 0.0, 0.0)],
        };
        let sahne = sahne_olustur(
            &model,
            SahneAyar {
                mod_: GosterimModu::Dolgu,
                sema: RenkSemasi::Eleman,
                ..Default::default()
            },
            &BTreeSet::new(),
        );
        assert_eq!(sahne.kureler.len(), 1);
        assert!((sahne.kureler[0].yaricap - 1.52).abs() < 1e-3); // O VDW
        assert_eq!(sahne.kureler[0].renk, Renk3B::ElemO);
    }

    #[test]
    fn topcubuk_kure_ve_bag() {
        let model = YapiModeli {
            model_no: 1,
            atomlar: vec![
                atom("C1", "C", "A", 1, 0.0, 0.0, 0.0),
                atom("C2", "C", "A", 1, 1.5, 0.0, 0.0),
            ],
        };
        let sahne = sahne_olustur(
            &model,
            SahneAyar {
                mod_: GosterimModu::TopCubuk,
                sema: RenkSemasi::Eleman,
                ..Default::default()
            },
            &BTreeSet::new(),
        );
        assert_eq!(sahne.kureler.len(), 2);
        // Bir bağ → iki yarım silindir.
        assert_eq!(sahne.silindirler.len(), 2);
    }

    #[test]
    fn gizli_zincir_atlanir() {
        let model = YapiModeli {
            model_no: 1,
            atomlar: vec![
                atom("CA", "C", "A", 1, 0.0, 0.0, 0.0),
                atom("CA", "C", "B", 1, 5.0, 0.0, 0.0),
            ],
        };
        let mut gizli = BTreeSet::new();
        gizli.insert("B".to_string());
        let sahne = sahne_olustur(
            &model,
            SahneAyar {
                mod_: GosterimModu::Dolgu,
                ..Default::default()
            },
            &gizli,
        );
        assert_eq!(sahne.kureler.len(), 1); // yalnız A
    }

    #[test]
    fn kartonet_omurga_serit_uretir() {
        // 5 ardışık Cα (heliksimsi) → bir şerit.
        let mut atomlar = Vec::new();
        for i in 0..5 {
            atomlar.push(atom("CA", "C", "A", i + 1, i as f32 * 2.0, 0.0, 0.0));
        }
        let model = YapiModeli {
            model_no: 1,
            atomlar,
        };
        let sahne = sahne_olustur(
            &model,
            SahneAyar {
                mod_: GosterimModu::Kartonet,
                ..Default::default()
            },
            &BTreeSet::new(),
        );
        assert_eq!(sahne.seritler.len(), 1);
        assert_eq!(sahne.seritler[0].noktalar.len(), 5);
    }

    #[test]
    fn ikincil_yapi_heliks_atar() {
        // İdeal α-heliks Cα koordinatları (sarmal): SS ataması heliks içermeli.
        let mut atomlar = Vec::new();
        for i in 0..10 {
            let t = i as f32;
            let aci = t * 100.0_f32.to_radians(); // tur başına 100° (3.6 kalıntı/tur)
            atomlar.push(atom(
                "CA",
                "C",
                "A",
                i + 1,
                2.3 * aci.cos(),
                2.3 * aci.sin(),
                1.5 * t, // 1.5 Å/kalıntı yükselme
            ));
        }
        let model = YapiModeli {
            model_no: 1,
            atomlar,
        };
        let ss = ikincil_yapi(&model);
        assert!(
            ss.values().any(|&s| s == IkincilYapi::Heliks),
            "ideal heliks koordinatları heliks olarak sınıflanmalı"
        );
    }

    #[test]
    fn bfaktor_semasi_rampaya_eslestirir() {
        let mut a = atom("O", "O", "A", 1, 0.0, 0.0, 0.0);
        a.b_faktor = 50.0;
        let mut b = atom("O", "O", "A", 2, 2.0, 0.0, 0.0);
        b.b_faktor = 100.0;
        let model = YapiModeli {
            model_no: 1,
            atomlar: vec![a, b],
        };
        let sahne = sahne_olustur(
            &model,
            SahneAyar {
                mod_: GosterimModu::Dolgu,
                sema: RenkSemasi::BFaktor,
                ..Default::default()
            },
            &BTreeSet::new(),
        );
        // Düşük B → 0 (mavi uç), yüksek B → 255 (kırmızı uç).
        assert_eq!(sahne.kureler[0].renk, Renk3B::BFaktor(0));
        assert_eq!(sahne.kureler[1].renk, Renk3B::BFaktor(255));
    }
}
