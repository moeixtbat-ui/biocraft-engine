//! Gözlemlenebilirlik + kalite altyapısı demosu (İP-21, MK-57, MK-58).
//!
//! Çalıştır:  `cargo run -p biocraft-data --example gozlem_demo`
//!
//! Beş bölüm: (1) iz bağlamı (W3C correlation_id), (2) yapılandırılmış log + PII temizleme,
//! (3) ilerleme/iptal'li uzun iş (Job), (4) edge-case eşikleri, (5) golden normalizasyon.

use biocraft_data::biocraft_types::esikler::{DiskDurumu, Gericekilme};
use biocraft_data::biocraft_types::golden::Normalize;
use biocraft_data::biocraft_types::{Ilerleme, IsKulpu, LogKaydi, LogSeverity, TraceContext};

fn baslik(n: u8, ad: &str) {
    println!("\n=== {n}) {ad} ===");
}

fn main() {
    // 1) İz bağlamı: correlation_id ile W3C traceparent aynı kimliktir.
    baslik(1, "İz bağlamı (W3C Trace Context = correlation_id)");
    let iz = TraceContext::kok();
    println!("traceparent : {}", iz.traceparent());
    println!("correlation : {}", iz.correlation_id().kisa());
    let alt = iz.cocuk();
    println!(
        "alt-adım    : aynı trace, yeni span → {}",
        alt.span_id_hex()
    );

    // 2) Yapılandırılmış log + PII temizleme (MK-45).
    baslik(2, "Yapılandırılmış log (NDJSON) + PII temizleme");
    let kayit = LogKaydi::yeni(
        LogSeverity::Info,
        "biocraft_data::project",
        // Mesajda kasıtlı PII: yol kullanıcı adı + e-posta + uzun numara → maskelenir.
        r"Proje açıldı C:\Users\Furkan\veri için a@b.com, kimlik 123456789012",
    )
    .with_iz(&iz)
    .alan("dosya", "ornek.vcf")
    .hassas_alan("tam_yol"); // değer asla yazılmaz
    println!("konsol: {}", kayit.satir());
    println!("ndjson: {}", kayit.to_json());

    // 3) Uzun iş: ilerleme bildirimi + iptal (Job; Gün 4/7 ile tutarlı).
    baslik(3, "Uzun iş — ilerleme + iptal (Job)");
    let is = IsKulpu::yeni("FASTA indeksleme", Some(iz));
    println!("başlangıç durum: {:?}", is.durum());
    for adim in 1..=5 {
        if is.iptal_mi() {
            is.iptal_tamam();
            break;
        }
        is.ilerleme_bildir(Ilerleme::Adim {
            tamam: adim,
            toplam: 5,
        });
        println!("  ilerleme: %{:?}", is.ilerleme().yuzde().unwrap());
        if adim == 3 {
            // Arayüzden "Durdur"a basıldığını taklit et.
            println!("  (kullanıcı iptal etti)");
            is.iptal_et();
        }
    }
    if is.iptal_mi() {
        is.iptal_tamam();
    } else {
        is.tamamla();
    }
    println!("son durum: {:?}", is.durum());

    // 4) Edge-case eşikleri (0.12).
    baslik(4, "Edge-case eşikleri (disk / ağ geri çekilme)");
    for bos in [25.0, 5.0, 1.0] {
        let d = DiskDurumu::siniflandir(bos);
        println!(
            "  disk %{bos:>4} boş → {d:?} (yazılabilir: {})",
            d.yazilabilir()
        );
    }
    let mut geri = Gericekilme::yeni();
    print!("  ağ yeniden deneme gecikmeleri (s): ");
    while geri.devam_eder_mi() {
        print!("{} ", geri.gecikme_saniye());
        geri.ilerle();
    }
    println!("→ sonra kalıcı hata");

    // 5) Golden normalizasyon (gürültülü alanları sabitler → kırılgan değil).
    baslik(5, "Golden normalizasyon");
    let ham = format!(
        "olay {} trace={}",
        chrono::Utc::now().to_rfc3339(),
        iz.trace_id_hex()
    );
    let norm = Normalize::yeni(ham.clone())
        .zaman_damgalari()
        .kimlikler()
        .bitir();
    println!("ham        : {ham}");
    println!("normalize  : {norm}");

    println!("\nTüm bölümler tamam. Üretimde bu loglar `<veri>/logs/biocraft-*.ndjson` dosyasına yazılır.");
}
