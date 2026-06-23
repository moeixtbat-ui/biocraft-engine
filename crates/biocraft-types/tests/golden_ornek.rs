//! Golden çerçevesi örnek testleri (İP-21, MK-58).
//!
//! İki örnek: (1) yapılandırılmış log satırının (NDJSON) **şekli**, (2) sentetik bir bilimsel
//! özet çıktısı.  Gerçek IGV/samtools/bcftools karşılaştırması çekirdek eklentide (ÇE) bu
//! çerçeveyle yapılacaktır; bugün **çerçeve + örnek**.
//!
//! Referansları (yeniden) üretmek için:  `BIOCRAFT_GOLDEN_UPDATE=1 cargo test -p biocraft-types`

use biocraft_types::golden::{dogrula, Normalize};
use biocraft_types::{LogKaydi, LogSeverity, TraceContext};

/// Sabit (deterministik) iz bağlamı — golden için (alanlar `pub`, doğrudan kurulabilir).
fn sabit_iz() -> TraceContext {
    TraceContext {
        trace_id: [0x11; 16],
        span_id: [0x22; 8],
        flags: 1,
    }
}

#[test]
fn golden_yapilandirilmis_log_satiri() {
    // Zaman damgası volatildir → normalize edilir; trace/span sabittir (deterministik bağlam).
    let kayit = LogKaydi::yeni(
        LogSeverity::Warn,
        "biocraft_data::project",
        "manifest doğrulandı",
    )
    .with_iz(&sabit_iz())
    .alan("dosya", "ornek.vcf")
    .alan("kayit_sayisi", "3");

    let normalize = Normalize::yeni(kayit.to_json()).zaman_damgalari().bitir();
    dogrula("log_satiri", &normalize);
}

#[test]
fn golden_bilimsel_ozet() {
    // Sentetik bir "varyant özeti" — bilimsel çıktının golden ile sabitlenmesini gösterir.
    // (Gerçek bcftools karşılaştırması ÇE'de; bu, çerçevenin çok-satırlı çıktıda çalıştığını gösterir.)
    let ozet = sentetik_varyant_ozeti();
    dogrula("varyant_ozeti", &ozet);
}

/// Deterministik sahte varyant özeti (sıralı, sabit) — golden örneği için.
fn sentetik_varyant_ozeti() -> String {
    let varyantlar = [
        ("chr1", 10_177, "AC", "A", "PASS"),
        ("chr1", 10_352, "T", "TA", "PASS"),
        ("chr7", 117_559_590, "ATCT", "A", "LowQual"),
        ("chrX", 31_224_010, "C", "T", "PASS"),
    ];
    let mut s = String::from("# Varyant Özeti (sentetik)\n");
    s.push_str(&format!("toplam\t{}\n", varyantlar.len()));
    let gecen = varyantlar.iter().filter(|v| v.4 == "PASS").count();
    s.push_str(&format!("pass\t{gecen}\n"));
    s.push_str("--- kayıtlar ---\n");
    for (kromozom, konum, refb, altb, filtre) in varyantlar {
        s.push_str(&format!("{kromozom}\t{konum}\t{refb}\t{altb}\t{filtre}\n"));
    }
    s
}
