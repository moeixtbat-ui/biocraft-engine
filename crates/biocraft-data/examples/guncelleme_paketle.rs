//! **Release hattı imzalama aracı** — bir paket dosyasından imzalı güncelleme bildirimi üretir
//! (İP-20, MK-56).  `.github/workflows/release.yml` bunu her platform artifact'ı için çağırır.
//!
//! Çalıştır:
//! ```text
//! cargo run -p biocraft-data --example guncelleme_paketle -- \
//!     <paket_dosyasi> <surum> [--kanal kararli|beta|nightly] \
//!     [--changelog <dosya>] [--anahtar <hex_dosyasi>] [--delta <eski_paket>] \
//!     [--cikti <dizin>]
//! ```
//!
//! Üretir (çıktı dizinine):
//!   - `bildirim.json`     — imzalanan **kanonik** geniş bildirim (sürüm+özet+boyut+kanal+changelog).
//!   - `imza.hex`          — bildirimin Ed25519 imzası (64 bayt, onaltılık).
//!   - `acik_anahtar.hex`  — doğrulama (açık) anahtarı (32 bayt) — çekirdeğe gömülecek olan.
//!   - `paket.delta.json`  — (yalnız `--delta` verilirse) eski→yeni delta yaması.
//!
//! **İNSAN-ELİ / GÜVENLİK:** `--anahtar` ile verilen 32 baytlık **özel** imzalama anahtarı asla
//! repoda tutulmaz; CI'da gizli (secret) olarak gelir (`Hukuk-ve-Operasyon.md`).  `--anahtar`
//! verilmezse araç **deterministik bir TEST anahtarıyla** imzalar ve çıktıyı *dağıtılamaz test*
//! olarak işaretler — böylece imza zinciri/sertifika gelmeden de CI artifact üretir.

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::PathBuf;

use biocraft_data::update::{self, demo_imzala, SurumKanali};

fn to_hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

fn from_hex(s: &str) -> Option<Vec<u8>> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

fn kanaldan(s: &str) -> SurumKanali {
    match s.to_lowercase().as_str() {
        "beta" => SurumKanali::Beta,
        "nightly" | "gecelik" => SurumKanali::Nightly,
        _ => SurumKanali::Kararli,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!(
            "kullanım: guncelleme_paketle <paket> <surum> [--kanal k] [--changelog f] \
             [--anahtar f] [--delta eski] [--cikti d]"
        );
        std::process::exit(2);
    }
    let paket_yolu = PathBuf::from(&args[0]);
    let surum = args[1].clone();

    let mut kanal = SurumKanali::Kararli;
    let mut changelog = String::new();
    let mut anahtar_tohum: Option<[u8; 32]> = None;
    let mut delta_eski: Option<PathBuf> = None;
    let mut cikti = PathBuf::from("dist");

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--kanal" => {
                kanal = kanaldan(&args[i + 1]);
                i += 2;
            }
            "--changelog" => {
                changelog = fs::read_to_string(&args[i + 1]).unwrap_or_default();
                i += 2;
            }
            "--anahtar" => {
                let hx = fs::read_to_string(&args[i + 1]).expect("anahtar dosyası okunamadı");
                let bayt = from_hex(&hx).expect("anahtar geçerli hex değil");
                let arr: [u8; 32] = bayt.as_slice().try_into().expect("anahtar 32 bayt olmalı");
                anahtar_tohum = Some(arr);
                i += 2;
            }
            "--delta" => {
                delta_eski = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--cikti" => {
                cikti = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            other => {
                eprintln!("bilinmeyen argüman: {other}");
                std::process::exit(2);
            }
        }
    }

    let nihai = fs::read(&paket_yolu).expect("paket dosyası okunamadı");
    fs::create_dir_all(&cikti).expect("çıktı dizini oluşturulamadı");

    // İmzalama anahtarı: verilmişse gerçek (CI gizlisi), yoksa TEST.
    let (tohum, test_mi) = match anahtar_tohum {
        Some(t) => (t, false),
        None => ([0xA5u8; 32], true),
    };

    let (bildirim, imza, vk) = demo_imzala(&nihai, &surum, kanal, &changelog, tohum);

    fs::write(cikti.join("bildirim.json"), bildirim.kanonik()).unwrap();
    fs::write(cikti.join("imza.hex"), to_hex(&imza)).unwrap();
    fs::write(cikti.join("acik_anahtar.hex"), to_hex(&vk)).unwrap();

    if let Some(eski_yolu) = delta_eski {
        let eski = fs::read(&eski_yolu).expect("eski paket okunamadı");
        let yama = update::delta::uret(&eski, &nihai);
        fs::write(cikti.join("paket.delta.json"), yama.json()).unwrap();
        println!(
            "  delta: {} bayt taşınan / {} bayt yeni paket",
            yama.tasinan_bayt(),
            nihai.len()
        );
    }

    println!(
        "✓ {} sürüm {} ({:?}) için bildirim imzalandı → {}",
        if test_mi {
            "[TEST ANAHTARI — DAĞITILAMAZ]"
        } else {
            "[resmi anahtar]"
        },
        surum,
        kanal,
        cikti.display()
    );
    if test_mi {
        println!(
            "  UYARI: gerçek imza zinciri için --anahtar <gizli_hex> verin (CI secret); \
             bu artifact yalnız test/CI doğrulaması içindir."
        );
    }
}
