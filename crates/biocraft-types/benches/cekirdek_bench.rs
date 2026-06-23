//! Çekirdek mikro-benchmark'lar — performans regresyon koruması (İP-21, MK-58).
//!
//! Çalıştır:  `cargo bench -p biocraft-types`
//!
//! **Neden criterion değil?** criterion büyük bir bağımlılık ağacı (rayon, plotters, clap…)
//! getirir; bu projenin "yeni dış bağımlılık yok" disiplini gereği saf-Rust ince bir harness
//! kullanıyoruz (criterion ileride bir kanca).  Gürültüye karşı: **ısınma** + birden çok parti +
//! **en iyi (min)** örnek alınır (zamanlayıcı/planlayıcı gürültüsünü eler).
//!
//! **Makineden bağımsızlık:** Mutlak ns değerleri makineye bağlıdır; bu yüzden her ölçüm bir
//! **kalibrasyon** işine **oranlanır** (`oran`).  CI regresyon denetimi bu oranı `baseline.json`
//! ile karşılaştırır (bkz. `scripts/check-bench.py`) → farklı runner'larda da anlamlı kalır.

use std::hint::black_box;
use std::io::Write;
use std::time::Instant;

use biocraft_types::golden::Normalize;
use biocraft_types::{pii_temizle, LogKaydi, LogSeverity, TraceContext};

/// Bir işi ölçer: ısınma + `parti` kez `iter` tekrar; en iyi (min) ns/iter döner.
fn olc<F: FnMut()>(mut f: F, iter: u64, parti: u32) -> u64 {
    // Isınma (önbellek/branch predictor).
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

/// Kalibrasyon işi: sabit miktarda iş (makine hızı birimi).  Derleyici elemesin diye `black_box`.
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

    // 1) İz/correlation_id üretimi (her uzun iş/dış çağrı bunu yapar).
    ekle(
        "iz_olustur",
        olc(
            || {
                black_box(TraceContext::kok());
            },
            50_000,
            15,
        ),
    );

    // 2) traceparent biçimleme (W3C başlık).
    let iz = TraceContext::kok();
    ekle(
        "traceparent",
        olc(
            || {
                black_box(iz.traceparent());
            },
            50_000,
            15,
        ),
    );

    // 3) Yapılandırılmış log kaydı + NDJSON serileştirme (sık yol).
    ekle(
        "log_json",
        olc(
            || {
                let k = LogKaydi::yeni(LogSeverity::Info, "biocraft_data::project", "manifest ok")
                    .with_iz(&iz)
                    .alan("dosya", "ornek.vcf");
                black_box(k.to_json());
            },
            20_000,
            12,
        ),
    );

    // 4) PII temizleme (her log satırı geçer; MK-45).
    let ornek = r"Proje C:\Users\Furkan\veri için a@b.com kimlik 123456789012 açıldı";
    ekle(
        "pii_temizle",
        olc(
            || {
                black_box(pii_temizle(black_box(ornek)));
            },
            20_000,
            12,
        ),
    );

    // 5) Golden normalizasyon (test altyapısı sıcak yolu).
    let ham = "olay 2026-06-23T12:00:00+00:00 id=550e8400-e29b-41d4-a716-446655440000";
    ekle(
        "golden_normalize",
        olc(
            || {
                black_box(Normalize::yeni(ham).zaman_damgalari().kimlikler().bitir());
            },
            20_000,
            12,
        ),
    );

    // JSON üret (manuel — ek bağımlılık yok).
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

    // İnsan-okur özet (stderr) + JSON (dosya + stdout).
    eprintln!("kalibrasyon: {kal} ns/iter");
    for o in &olcumler {
        eprintln!(
            "  {:<18} {:>8} ns/iter  (oran {:.3})",
            o.ad, o.ns_iter, o.oran
        );
    }
    let yol = std::env::var("BENCH_OUT").unwrap_or_else(|_| "bench-results.json".to_string());
    if let Ok(mut f) = std::fs::File::create(&yol) {
        let _ = f.write_all(json.as_bytes());
        eprintln!("Sonuçlar yazıldı: {yol}");
    }
    print!("{json}");
}
