//! İP-06 (2. kısım, Gün 23) demosu — **node↔kod köprüsü + temel LSP + izole ortam**.
//!
//! Çalıştırma:
//! ```text
//! cargo run -p biocraft-ui --example kod_kopru_demo
//! ```
//! Hepsi **arayüzsüz** (headless) çalışır; gerçek pencere açmaz.  jedi/venv adımları
//! sistemde Python varsa canlı, yoksa atlanır (kabul kriterleri uçtan uca görünür).

use biocraft_plugin_host::exec::{
    jedi_var_mi, lsp_durumu, onek_al, ortam, tamamla_async, temel_tamamla, LspDurumu,
    PaketGereksinimi, SanalOrtam, TamamlamaIstegi, TamamlamaYaniti,
};
use biocraft_ui::editor::bridge::{cikti_workspace_ayikla, node_olarak_kod, WORKSPACE_SENTINEL};
use biocraft_ui::NodeTuvali;

fn baslik(s: &str) {
    println!("\n========== {s} ==========");
}

fn main() {
    println!(
        "BioCraft Engine — İP-06 2. kısım demosu (node↔kod köprüsü + temel LSP + izole ortam)"
    );

    // ─── 1) NODE → KOD köprüsü (ortak workspace) ───────────────────────────────
    baslik("1) Node → Kod köprüsü (ortak workspace)");
    let mut tuval = NodeTuvali::ornek();
    // Akışı senkron çalıştır → node çıktıları üretilir (workspace'i besler).
    let sonuc = tuval.calistir_simdi().clone();
    let n_sonuc = sonuc.as_ref();
    println!(
        "Akış çalıştırıldı: {} node hesaplandı.",
        n_sonuc.map(|s| s.hesaplanan).unwrap_or(0)
    );
    let kod = node_olarak_kod(&tuval.graf, tuval.parametreler(), n_sonuc);
    println!("\n--- Üretilen köprülü Python (ilk satırlar) ---");
    for satir in kod.lines().take(16) {
        println!("  {satir}");
    }
    println!("  …");
    println!(
        "\n✔ Önsöz 'workspace = {{…}}' içeriyor: {}",
        kod.contains("workspace = {")
    );
    println!(
        "✔ Sonsöz workspace'i geri basıyor (kod→node): {}",
        kod.contains(WORKSPACE_SENTINEL)
    );

    // ─── 2) KOD → NODE köprüsü (sentinel ile tipli geri dönüş) ──────────────────
    baslik("2) Kod → Node köprüsü (sentinel → tipli workspace)");
    let sahte_cikti = [
        "normal çıktı satırı".to_string(),
        format!(
            "{}{}",
            WORKSPACE_SENTINEL,
            r#"{"toplam": 45, "oran": 0.8, "bitti": true, "ozet": "9 satır işlendi"}"#
        ),
    ];
    match cikti_workspace_ayikla(sahte_cikti.iter().map(|s| s.as_str())) {
        Some(alan) => {
            println!("Kod çıktısından çözülen tipli değişkenler:");
            for (ad, deg) in alan.tumu() {
                println!("  {ad} : {} = {}", deg.tur_adi(), deg.ozet());
            }
        }
        None => println!("(sentinel bulunamadı)"),
    }

    // ─── 3) Temel LSP (saf-Rust her zaman; jedi out-of-process opsiyonel) ───────
    baslik("3) Temel Python tamamlama (LSP)");
    let kod_metni = "import os\ndef hesapla_toplam(x):\n    return x\nhesap";
    let onek = onek_al("hesap", 5);
    println!("Önek '{onek}' için saf-Rust temel öneriler:");
    for t in temel_tamamla(kod_metni, &onek).iter().take(6) {
        println!("  {} ({:?})", t.etiket, t.tur);
    }
    let py = biocraft_plugin_host::python_bul();
    println!("\nLSP durumu: {:?}", lsp_durumu(py.as_deref()));
    if let Some(py) = &py {
        if jedi_var_mi(py) {
            println!("jedi kurulu → 'os.' için bağlam-duyarlı öneri (AYRI SÜREÇTE):");
            let tutamac = tamamla_async(
                py,
                TamamlamaIstegi {
                    kod: "import os\nos.".into(),
                    satir: 1,
                    sutun: 3,
                },
            );
            // Bloklamadan yokla (arayüz donmaz mantığı).
            let basla = std::time::Instant::now();
            loop {
                if let Some(y) = tutamac.dene() {
                    match y {
                        TamamlamaYaniti::Hazir(v) => {
                            for t in v.iter().take(6) {
                                println!("  {} ({:?})", t.etiket, t.tur);
                            }
                        }
                        diger => println!("  (jedi: {diger:?})"),
                    }
                    break;
                }
                if basla.elapsed() > std::time::Duration::from_secs(15) {
                    println!("  (jedi zamanında dönmedi)");
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        } else {
            println!("jedi kurulu DEĞİL → [Kur] yönlendirmesi (temel tamamlama yine de çalışır).");
            println!(
                "  → {}",
                biocraft_plugin_host::jedi_kur_rehberi().nasil_cozulur
            );
        }
    }
    if matches!(lsp_durumu(py.as_deref()), LspDurumu::Hazir) {
        println!("\n(Not: tam dil zekâsı — tanı/hover/diğer diller — v1.x'e ertelendi.)");
    }

    // ─── 4) İzole ortam: sürüm kilidi + eksik paket tespiti (model) ─────────────
    baslik("4) İzole ortam — sürüm kilidi + [Kur] (eksik paket)");
    let demo_dizin = std::env::temp_dir().join("biocraft_ortam_demo");
    let _ = std::fs::create_dir_all(&demo_dizin);
    let o = SanalOrtam::yeni(&demo_dizin);
    println!("Proje izole ortamı: {}", o.venv_dizini().display());
    println!("Yorumlayıcı (venv): {}", o.yorumlayici().display());
    let kilit = vec![
        PaketGereksinimi::yeni("numpy", Some("1.26.0".into())),
        PaketGereksinimi::yeni("biopython", Some("1.83".into())),
        PaketGereksinimi::yeni("jedi", None),
    ];
    o.kilit_yaz(&kilit).unwrap();
    println!("\nSürüm kilidi yazıldı ({}):", ortam::KILIT_DOSYA);
    for g in o.kilit_oku().unwrap() {
        println!("  {}", g.kilit_satiri());
    }
    // Diyelim numpy doğru, biopython yanlış sürüm, jedi hiç yok → ikisi "eksik" → [Kur].
    let kurulu = vec![
        biocraft_plugin_host::KuruluPaket {
            ad: "numpy".into(),
            surum: "1.26.0".into(),
        },
        biocraft_plugin_host::KuruluPaket {
            ad: "biopython".into(),
            surum: "1.80".into(),
        },
    ];
    let eksik = ortam::eksikleri_bul(&o.kilit_oku().unwrap(), &kurulu);
    println!("\nEksik/yanlış sürümlü paketler → [Kur] gösterilir:");
    for g in &eksik {
        println!("  [Kur] {}", g.pip_argumani());
    }
    let _ = std::fs::remove_dir_all(&demo_dizin);

    // Gerçek venv kurulumu yavaş + ağ ister → BIOCRAFT_VENV_TEST=1 ile birim testte (opsiyonel).
    println!(
        "\n(Gerçek 'python -m venv' + 'pip install' canlı testi: \
         BIOCRAFT_VENV_TEST=1 cargo test -p biocraft-plugin-host venv_uctan_uca)"
    );

    baslik("Demo tamam");
    println!("Köprü (iki yön) + temel LSP (out-of-process) + izole ortam/kilit uçtan uca çalıştı.");
}
