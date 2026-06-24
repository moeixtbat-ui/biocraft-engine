//! ÇE-07 (Gün 39) entegrasyon testi — **3B PDB/mmCIF yapı görüntüleyici**.
//!
//! PDB → yapı yükleme, gösterim modları (kartonet + top-çubuk), yörünge kamera (döndür/yakınlaş/
//! kaydır), zincir/atom seçimi, ölçüm, renklendirme, GPU çökmesi → CPU yedeği, yüksek çözünürlüklü
//! PNG/SVG dışa aktarma — uçtan uca + **golden** (top-çubuk sahne dökümü: bağ çıkarımı + CPK renk).
//! Test verileri **sentetiktir** (CLAUDE.md §7) — gerçek hasta/yapı verisi repoya girmez.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use biocraft_core_studio::data_io::{BellekButcesi, Yapi};
use biocraft_core_studio::structure3d::{
    GosterimModu, GpuDurumu, Palet3B, Renk3B, RenkSemasi, Sahne3B, Vec3, Yapi3BGorunumu,
};
use biocraft_sdk::biocraft_types::golden;

/// Her çağrı için benzersiz geçici ad (paralel testler aynı dosyaya çakışmasın).
static SAYAC: AtomicU64 = AtomicU64::new(0);

fn gecici(ad: &str) -> PathBuf {
    let n = SAYAC.fetch_add(1, Ordering::Relaxed);
    let mut yol = std::env::temp_dir();
    yol.push(format!("biocraft_yapi3d_{}_{n}_{ad}", std::process::id()));
    yol
}

fn yaz(yol: &Path, icerik: &[u8]) {
    File::create(yol).unwrap().write_all(icerik).unwrap();
}

/// Sentetik küçük PDB: A zinciri res1 omurgası (N-CA-C-O, kovalent bağlı) + res2 Cα (kartonet izi
/// 2 Cα ister) + B zinciri tek Cα + bir su.  Sütun hizalaması gerçek PDB 3.x'e uygundur (B-faktör
/// sütunu dâhil).
const PDB: &[u8] = b"\
HEADER    SENTETIK TEST                           24-JUN-26   TST1
ATOM      1  N   ALA A   1       0.000   0.000   0.000  1.00 15.00           N
ATOM      2  CA  ALA A   1       1.460   0.000   0.000  1.00 18.00           C
ATOM      3  C   ALA A   1       2.000   1.400   0.000  1.00 20.00           C
ATOM      4  O   ALA A   1       3.200   1.600   0.000  1.00 25.00           O
ATOM      5  CA  ALA A   2       5.260   0.000   0.000  1.00 22.00           C
ATOM      6  CA  GLY B   1      10.000   0.000   0.000  1.00 30.00           C
HETATM    7  O   HOH A   3       5.000   5.000   5.000  1.00 40.00           O
END
";

fn yapiyi_yukle() -> Yapi {
    let yol = gecici("a.pdb");
    yaz(&yol, PDB);
    let y = Yapi::oku(&yol, &BellekButcesi::sinirsiz()).unwrap();
    let _ = std::fs::remove_file(&yol);
    y
}

/// Sahneyi deterministik metne döker (golden — bağ çıkarımı + renk; trig/projeksiyon İÇERMEZ).
fn sahne_dok(s: &Sahne3B) -> String {
    let mut c = String::new();
    c.push_str(&format!(
        "kure={} silindir={} serit={}\n",
        s.kureler.len(),
        s.silindirler.len(),
        s.seritler.len()
    ));
    for k in &s.kureler {
        c.push_str(&format!(
            "KURE ({:.3},{:.3},{:.3}) r={:.2} {:?}\n",
            k.merkez.x, k.merkez.y, k.merkez.z, k.yaricap, k.renk
        ));
    }
    for b in &s.silindirler {
        c.push_str(&format!(
            "BAG ({:.3},{:.3},{:.3})->({:.3},{:.3},{:.3}) r={:.2} {:?}\n",
            b.bas.x, b.bas.y, b.bas.z, b.son.x, b.son.y, b.son.z, b.yaricap, b.renk
        ));
    }
    c
}

#[test]
fn pdb_yuklenir_zincirler_atomlar() {
    let g = Yapi3BGorunumu::yeni(yapiyi_yukle(), GpuDurumu::Var);
    assert_eq!(g.atom_sayisi(), 7);
    assert_eq!(g.zincirler(), vec!["A", "B"]);
    assert_eq!(g.model_sayisi(), 1);
}

#[test]
fn kartonet_ve_topcubuk_cizilir() {
    let mut g = Yapi3BGorunumu::yeni(yapiyi_yukle(), GpuDurumu::Var);

    // Kartonet: A zinciri 4 omurga atomu (N/CA/C/O) içerir → en az bir şerit; su gizli.
    g.mod_ = GosterimModu::Kartonet;
    let kartonet = g.sahne();
    assert!(
        !kartonet.seritler.is_empty(),
        "kartonet omurga izi üretmeli"
    );

    // Top-çubuk: küreler + bağ silindirleri.
    g.mod_ = GosterimModu::TopCubuk;
    let ts = g.sahne();
    assert!(!ts.kureler.is_empty());
    assert!(!ts.silindirler.is_empty(), "kovalent bağlar bulunmalı");
}

#[test]
fn yorunge_kamera_dondur_yakinlas_kaydir() {
    let mut g = Yapi3BGorunumu::yeni(yapiyi_yukle(), GpuDurumu::Var);
    let yaw0 = g.kamera.yaw;
    let mesafe0 = g.kamera.mesafe;
    let hedef0 = g.kamera.hedef;

    g.dondur(0.3, 0.1);
    assert!((g.kamera.yaw - yaw0).abs() > 1e-6);

    g.yakinlastir(0.5);
    assert!(g.kamera.mesafe < mesafe0);

    g.kaydir(20.0, 10.0);
    assert!(g.kamera.hedef != hedef0);

    // Odakla → yeniden çerçeveler.
    g.odakla();
    assert!(g.kamera.mesafe > 0.0);
}

#[test]
fn atom_secimi_bilgi_verir() {
    let mut g = Yapi3BGorunumu::yeni(yapiyi_yukle(), GpuDurumu::Var);
    g.mod_ = GosterimModu::Dolgu;
    // İlk atomun (N, ALA A1) ekran izdüşümüne tıkla.
    let kamera = g.kamera.kamera();
    let p = kamera
        .hazirla(400.0, 400.0)
        .projekte(Vec3::yeni(0.0, 0.0, 0.0))
        .unwrap();
    let secilen = g.sec_ekrandan(p.x, p.y, 400.0, 400.0);
    assert!(secilen.is_some());
    let bilgi = g.secili_bilgi().unwrap();
    assert!(bilgi.contains("Kalıntı"));
    assert!(bilgi.contains("B-faktör"));
}

#[test]
fn zincir_gizle_goster() {
    let mut g = Yapi3BGorunumu::yeni(yapiyi_yukle(), GpuDurumu::Var);
    g.mod_ = GosterimModu::Dolgu;
    g.su_goster = false;
    let once = g.sahne().kureler.len(); // 5 (su hariç)
    g.zincir_gizle("B");
    assert!(g.sahne().kureler.len() < once);
    g.zincir_goster("B");
    assert_eq!(g.sahne().kureler.len(), once);
}

#[test]
fn olcum_mesafe_aci_geri_al() {
    let mut g = Yapi3BGorunumu::yeni(yapiyi_yukle(), GpuDurumu::Var);
    // N(0) - CA(1): tam 1.460 Å.
    let d = g.mesafe_ekle(0, 1).unwrap();
    assert!((d - 1.460).abs() < 1e-3);
    // N(0)-CA(1)-C(2) açısı (tepe CA).
    let _ = g.aci_ekle(0, 1, 2).unwrap();
    assert_eq!(g.olcumler().len(), 2);
    g.olcum_geri_al();
    assert_eq!(g.olcumler().len(), 1);
}

#[test]
fn renklendirme_semalari() {
    let mut g = Yapi3BGorunumu::yeni(yapiyi_yukle(), GpuDurumu::Var);
    g.mod_ = GosterimModu::Dolgu;

    g.sema = RenkSemasi::Zincir;
    assert!(g
        .sahne()
        .kureler
        .iter()
        .any(|k| matches!(k.renk, Renk3B::Zincir(_))));

    g.sema = RenkSemasi::Eleman;
    assert!(g
        .sahne()
        .kureler
        .iter()
        .any(|k| k.renk == Renk3B::ElemN || k.renk == Renk3B::ElemO));

    g.sema = RenkSemasi::BFaktor;
    assert!(g
        .sahne()
        .kureler
        .iter()
        .any(|k| matches!(k.renk, Renk3B::BFaktor(_))));
}

#[test]
fn gpu_cokmesi_cpu_yedegi_ve_png() {
    let mut g = Yapi3BGorunumu::yeni(yapiyi_yukle(), GpuDurumu::Var);
    g.mod_ = GosterimModu::TopCubuk;
    assert!(g.uyari().is_none());

    // Cihaz kaybı (TDR/DeviceLost) → CPU yedeği; yapı kaybolmaz.
    g.gpu_ayarla(GpuDurumu::cihaz_kaybi());
    assert_eq!(g.gpu(), GpuDurumu::Yok);
    assert!(g.uyari().is_some());

    // Yüksek çözünürlüklü PNG anlık görüntü (CPU rasteri).
    let png = g.png_disa_aktar(800, 600, &Palet3B::yayin());
    assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    assert!(png.len() > 100);

    // SVG anlık görüntü (yayın kalitesi).
    let svg = g.svg_disa_aktar(800.0, 600.0, &Palet3B::yayin());
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<circle") || svg.contains("<line"));
}

#[test]
fn golden_topcubuk_sahne() {
    let mut g = Yapi3BGorunumu::yeni(yapiyi_yukle(), GpuDurumu::Var);
    g.mod_ = GosterimModu::TopCubuk;
    g.sema = RenkSemasi::Eleman;
    g.su_goster = true; // su atomu da dahil (bağsız → küre)
                        // Sahne 3B geometri dökümü (deterministik; projeksiyon/trig içermez).
    golden::dogrula("ce07_topcubuk_sahne", &sahne_dok(&g.sahne()));
}
