//! `cargo run -p biocraft-core-studio --example yapi3d_demo`
//!
//! ÇE-07 (Gün 39) 3B yapı görüntüleyiciyi uçtan uca gösterir: **PDB yükleme** → **gösterim modları**
//! (kartonet/top-çubuk/çubuk/dolgu) → **renklendirme** (zincir/ikincil yapı/eleman/B-faktör) →
//! **yörünge kamera** (döndür/yakınlaş/kaydır) → **atom seçimi + ölçüm** → **GPU çökmesi → CPU
//! yedeği** → **PNG/SVG anlık görüntü**.  Tüm veri **sentetiktir** (CLAUDE.md §7).

use std::fs::File;
use std::io::Write;

use biocraft_core_studio::data_io::{BellekButcesi, Yapi};
use biocraft_core_studio::structure3d::{
    GosterimModu, GpuDurumu, Palet3B, RenkSemasi, Yapi3BGorunumu,
};

const PDB: &[u8] = b"\
HEADER    SENTETIK MINI PROTEIN                   24-JUN-26   DEMO
ATOM      1  N   MET A   1       0.000   0.000   0.000  1.00 12.00           N
ATOM      2  CA  MET A   1       1.460   0.000   0.000  1.00 15.00           C
ATOM      3  C   MET A   1       2.000   1.400   0.000  1.00 18.00           C
ATOM      4  O   MET A   1       3.200   1.600   0.000  1.00 22.00           O
ATOM      5  CA  ALA A   2       5.260   0.000   0.000  1.00 28.00           C
ATOM      6  CA  GLY A   3       8.900   1.200   0.000  1.00 35.00           C
ATOM      7  CA  LEU B   1      12.000   0.000   0.000  1.00 40.00           C
HETATM    8 ZN    ZN C   1       6.000   3.000   1.000  1.00 10.00          ZN
HETATM    9  O   HOH A   4       9.000   9.000   9.000  1.00 50.00           O
END
";

fn main() {
    println!("=== BioCraft Studio — ÇE-07 3B Yapı Görüntüleyici Demosu (Gün 39) ===\n");

    // 1) Sentetik PDB'yi geçici dosyaya yaz + yükle.
    let mut yol = std::env::temp_dir();
    yol.push(format!("biocraft_yapi3d_demo_{}.pdb", std::process::id()));
    File::create(&yol).unwrap().write_all(PDB).unwrap();
    let yapi = Yapi::oku(&yol, &BellekButcesi::sinirsiz()).expect("PDB okunmalı");

    println!("── 1. Yükleme (PDB → atom/zincir/model) ─────────────────────────");
    let mut g = Yapi3BGorunumu::yeni(yapi, GpuDurumu::Var);
    println!(
        "  format=PDB  atom={}  zincir={:?}  model={}",
        g.atom_sayisi(),
        g.zincirler(),
        g.model_sayisi()
    );

    // 2) Gösterim modları.
    println!("\n── 2. Gösterim modları (sahne ilkelleri) ────────────────────────");
    for m in [
        GosterimModu::Kartonet,
        GosterimModu::TopCubuk,
        GosterimModu::Cubuk,
        GosterimModu::Dolgu,
    ] {
        g.mod_ = m;
        let s = g.sahne();
        println!(
            "  {:<16} küre={:<3} silindir={:<3} şerit={}",
            m.etiket(),
            s.kureler.len(),
            s.silindirler.len(),
            s.seritler.len()
        );
    }

    // 3) Renklendirme şemaları.
    println!("\n── 3. Renklendirme şemaları ─────────────────────────────────────");
    g.mod_ = GosterimModu::Dolgu;
    for sema in [
        RenkSemasi::Zincir,
        RenkSemasi::IkincilYapi,
        RenkSemasi::Eleman,
        RenkSemasi::BFaktor,
    ] {
        g.sema = sema;
        let ilk = g.sahne().kureler.first().map(|k| format!("{:?}", k.renk));
        println!(
            "  {:<16} ilk atom rengi = {:?}",
            sema.etiket(),
            ilk.unwrap()
        );
    }

    // 4) Yörünge kamera.
    println!("\n── 4. Yörünge kamera (döndür / yakınlaş / kaydır) ───────────────");
    println!(
        "  başlangıç: yaw={:.2} pitch={:.2} mesafe={:.1}",
        g.kamera.yaw, g.kamera.pitch, g.kamera.mesafe
    );
    g.dondur(0.5, 0.2);
    g.yakinlastir(0.7);
    g.kaydir(15.0, -8.0);
    println!(
        "  sonra:     yaw={:.2} pitch={:.2} mesafe={:.1}  hedef=({:.1},{:.1},{:.1})",
        g.kamera.yaw,
        g.kamera.pitch,
        g.kamera.mesafe,
        g.kamera.hedef.x,
        g.kamera.hedef.y,
        g.kamera.hedef.z
    );

    // 5) Seçim + ölçüm.
    println!("\n── 5. Atom seçimi + ölçüm (mesafe/açı) ──────────────────────────");
    g.mod_ = GosterimModu::TopCubuk;
    let kamera = g.kamera.kamera();
    if let Some(p) =
        kamera
            .hazirla(600.0, 600.0)
            .projekte(biocraft_core_studio::structure3d::Vec3::yeni(
                1.460, 0.0, 0.0,
            ))
    {
        if let Some(idx) = g.sec_ekrandan(p.x, p.y, 600.0, 600.0) {
            println!("  seçildi (atom {idx}):");
            for satir in g.secili_bilgi().unwrap().lines() {
                println!("    {satir}");
            }
        }
    }
    let d = g.mesafe_ekle(0, 1).unwrap();
    let a = g.aci_ekle(0, 1, 2).unwrap();
    println!(
        "  N-CA mesafe = {d:.2} Å   N-CA-C açı = {a:.1}°   (ölçüm sayısı={})",
        g.olcumler().len()
    );
    g.olcum_geri_al();
    println!(
        "  son ölçüm geri alındı → ölçüm sayısı={}",
        g.olcumler().len()
    );

    // 6) Zincir gizle/göster.
    println!("\n── 6. Zincir görünürlüğü ────────────────────────────────────────");
    g.mod_ = GosterimModu::Dolgu;
    println!("  tüm küre = {}", g.sahne().kureler.len());
    g.zincir_gizle("B");
    g.zincir_gizle("C");
    println!("  B+C gizli → küre = {}", g.sahne().kureler.len());
    g.zincir_goster("B");
    g.zincir_goster("C");

    // 7) GPU çökmesi → CPU yedeği + uyarı.
    println!("\n── 7. GPU çökmesi (TDR/DeviceLost) → CPU yedeği ─────────────────");
    println!("  GPU var → uyarı: {:?}", g.uyari());
    g.gpu_ayarla(GpuDurumu::cihaz_kaybi());
    println!("  cihaz kaybı → GPU={:?}", g.gpu());
    println!("  uyarı: {}", g.uyari().unwrap());

    // 8) PNG / SVG anlık görüntü.
    println!("\n── 8. Yüksek çözünürlüklü anlık görüntü ─────────────────────────");
    g.gpu_ayarla(GpuDurumu::Var);
    g.mod_ = GosterimModu::TopCubuk;
    g.sema = RenkSemasi::Eleman;
    let palet = Palet3B::yayin();
    let png = g.png_disa_aktar(1200, 900, &palet);
    let svg = g.svg_disa_aktar(1200.0, 900.0, &palet);
    let png_yol = std::env::temp_dir().join("biocraft_yapi3d_demo.png");
    let svg_yol = std::env::temp_dir().join("biocraft_yapi3d_demo.svg");
    File::create(&png_yol).unwrap().write_all(&png).unwrap();
    File::create(&svg_yol)
        .unwrap()
        .write_all(svg.as_bytes())
        .unwrap();
    println!("  PNG: {} bayt → {}", png.len(), png_yol.display());
    println!("  SVG: {} bayt → {}", svg.len(), svg_yol.display());

    let _ = std::fs::remove_file(&yol);
    println!("\n=== Demo tamam: PDB → 3B sahne → kamera → seçim/ölçüm → CPU yedeği → PNG/SVG ===");
}
