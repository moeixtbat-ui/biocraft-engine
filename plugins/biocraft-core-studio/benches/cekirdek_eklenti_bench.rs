//! Çekirdek eklenti mikro-benchmark'ları — performans regresyon koruması (ÇE-12, İP-21, MK-58).
//!
//! Çalıştır:  `cargo bench -p biocraft-core-studio --bench cekirdek_eklenti_bench`
//!
//! **Neden criterion değil?** Proje "yeni dış bağımlılık yok" disiplinindedir; çekirdek bench
//! (`crates/biocraft-types/benches/cekirdek_bench.rs`) gibi saf-Rust ince bir harness kullanılır.
//! Her ölçüm bir **kalibrasyon** işine **oranlanır** → CI regresyon denetimi (`scripts/check-bench.py`)
//! farklı runner'larda da anlamlı kalır (makineden ~bağımsız).
//!
//! Ölçülen sıcak yollar (büyük veri akıcılığının belkemiği):
//!  * `tarayici_derle` — genom tarayıcının çok-iz LOD derlemesi (büyük BAM akıcılığı).
//!  * `varyant_sorgu`  — VCF filtre + sıralama sorgusu (büyük VCF akıcılığı).
//!  * `cvd_simule`     — renk körü (dichromat) simülasyonu (erişilebilirlik palet güvenliği).

use std::collections::BTreeMap;
use std::hint::black_box;
use std::io::Write;
use std::time::Instant;

use biocraft_core_studio::data_io::VaryantKaydi;
use biocraft_core_studio::genome_browser::{
    GenomBolge, GenomTarayici, Iz, IzTuru, IzVeri, OkumaParcasi, Serit,
};
use biocraft_core_studio::perf::accessibility::{simule_et, RenkGormeTuru, OKABE_ITO};
use biocraft_core_studio::variant::{
    ayristir, BellekKaynak, SafRustMotor, Sorgu, VaryantSatiri, VaryantSorguMotoru,
};

/// Bir işi ölçer: ısınma + `parti` kez `iter` tekrar; en iyi (min) ns/iter döner.
fn olc<F: FnMut()>(mut f: F, iter: u64, parti: u32) -> u64 {
    for _ in 0..(iter / 10 + 1) {
        f();
    }
    let mut en_iyi = u64::MAX;
    for _ in 0..parti {
        let t = Instant::now();
        for _ in 0..iter {
            f();
        }
        let ns = (t.elapsed().as_nanos() as u64).max(1) / iter.max(1);
        en_iyi = en_iyi.min(ns);
    }
    en_iyi.max(1)
}

/// Kalibrasyon işi: sabit miktarda iş (makine hızı birimi).
fn kalibrasyon() -> u64 {
    olc(
        || {
            let mut s: u64 = 0;
            for i in 0..256u64 {
                s = s.wrapping_add(black_box(i).wrapping_mul(2654435761));
            }
            black_box(s);
        },
        20_000,
        20,
    )
}

struct Olcum {
    ad: &'static str,
    ns_iter: u64,
    oran: f64,
}

/// 5000 okuma, 1 Mb bölge, yarısı ileri/geri şerit.
fn okuma_kumesi(n: usize, bolge_uzunluk: u64) -> Vec<OkumaParcasi> {
    (0..n)
        .map(|i| {
            let bas = 1 + (i as u64 * bolge_uzunluk / n as u64);
            OkumaParcasi {
                ad: format!("r{i}"),
                bas,
                bit: bas + 99,
                serit: if i % 2 == 0 {
                    Serit::Ileri
                } else {
                    Serit::Geri
                },
                mapq: Some(60),
            }
        })
        .collect()
}

/// Deterministik SNV kümesi.
fn varyant_kumesi(n: usize) -> Vec<VaryantSatiri> {
    (0..n)
        .map(|i| {
            VaryantSatiri::yeni(VaryantKaydi {
                kromozom: "chr1".into(),
                konum: 100 + i * 10,
                kimlik: ".".into(),
                referans: "A".into(),
                alternatifler: vec![["G", "C", "T"][i % 3].into()],
                kalite: Some((i % 100) as f32),
                filtreler: vec![if i % 3 == 0 {
                    "PASS".into()
                } else {
                    "q10".into()
                }],
                info: vec![],
                ornek_sayisi: 0,
                format_anahtarlari: vec![],
                genotipler: vec![],
            })
        })
        .collect()
}

fn main() {
    let kal = kalibrasyon();
    let mut olcumler = Vec::new();
    let mut ekle = |ad: &'static str, ns: u64| {
        olcumler.push(Olcum {
            ad,
            ns_iter: ns,
            oran: ns as f64 / kal as f64,
        });
    };

    // 1) Genom tarayıcı LOD derlemesi (5000 okuma → ilkel listesi).
    let okumalar = okuma_kumesi(5_000, 1_000_000);
    let mut tarayici = GenomTarayici::yeni(1000.0, GenomBolge::yeni("chr1", 1, 1_000_000).unwrap());
    tarayici.iz_ekle(Iz::yeni("kapsama", "Kapsama", IzTuru::Kapsama));
    tarayici.iz_ekle(Iz::yeni("reads", "Okumalar", IzTuru::Hizalama));
    let mut harita: BTreeMap<String, IzVeri> = BTreeMap::new();
    harita.insert("kapsama".into(), IzVeri::Kapsama(okumalar.clone()));
    harita.insert("reads".into(), IzVeri::Hizalama(okumalar));
    ekle(
        "tarayici_derle",
        olc(
            || {
                black_box(tarayici.derle(black_box(&harita)));
            },
            200,
            8,
        ),
    );

    // 2) Varyant filtre + sıralama sorgusu (5000 SNV).
    let mut motor = SafRustMotor::yeni(BellekKaynak::yeni(vec![], varyant_kumesi(5_000)));
    let sorgu = Sorgu::yeni(ayristir("QUAL >= 50 AND FILTER = PASS").unwrap());
    ekle(
        "varyant_sorgu",
        olc(
            || {
                black_box(motor.calistir(black_box(&sorgu)).unwrap());
            },
            500,
            8,
        ),
    );

    // 3) Renk körü (dichromat) simülasyonu — erişilebilirlik palet güvenlik hesabı.
    ekle(
        "cvd_simule",
        olc(
            || {
                for &renk in &OKABE_ITO {
                    black_box(simule_et(black_box(renk), RenkGormeTuru::Deuteranopi));
                }
            },
            50_000,
            12,
        ),
    );

    // JSON üret (manuel — ek bağımlılık yok; check-bench.py ile aynı şema).
    let mut json = String::from("{\n  \"kalibrasyon_ns\": ");
    json.push_str(&kal.to_string());
    json.push_str(",\n  \"olcumler\": {\n");
    for (i, o) in olcumler.iter().enumerate() {
        json.push_str(&format!(
            "    \"{}\": {{ \"ns_iter\": {}, \"oran\": {:.4} }}{}\n",
            o.ad,
            o.ns_iter,
            o.oran,
            if i + 1 < olcumler.len() { "," } else { "" }
        ));
    }
    json.push_str("  }\n}\n");

    eprintln!("kalibrasyon: {kal} ns/iter");
    for o in &olcumler {
        eprintln!(
            "  {:<18} {:>10} ns/iter  (oran {:.3})",
            o.ad, o.ns_iter, o.oran
        );
    }
    let yol = std::env::var("BENCH_OUT").unwrap_or_else(|_| "bench-core-studio.json".to_string());
    if let Ok(mut f) = std::fs::File::create(&yol) {
        let _ = f.write_all(json.as_bytes());
        eprintln!("Sonuçlar yazıldı: {yol}");
    }
    print!("{json}");
}
