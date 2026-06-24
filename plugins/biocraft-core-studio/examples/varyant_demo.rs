//! `cargo run -p biocraft-core-studio --example varyant_demo`
//!
//! ÇE-04 (Gün 38) varyant incelemeyi uçtan uca gösterir: **VCF tablo** (out-of-core) + **filtre/ham
//! sorgu** + **sıralama** + **çok-örnekli genotip ızgarası** + **genom tarayıcıya "varyanta git"** +
//! **özet istatistik** + **filtreli dışa aktarma (CSV/VCF)** + **kayıtlı filtre seti**.
//! Tüm veriler **sentetiktir** (gerçek hasta verisi yok — CLAUDE.md §7).

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use biocraft_core_studio::variant::{
    DosyaKaynak, SafRustMotor, SiralamaAnahtari, VaryantGorunumu, VaryantKaynak,
};
use biocraft_sdk::biocraft_types::Capability;
use biocraft_sdk::YetkiKapisi;

const VCF: &[u8] = b"\
##fileformat=VCFv4.3
##contig=<ID=chr1,length=100000>
##contig=<ID=chr2,length=100000>
##INFO=<ID=DP,Number=1,Type=Integer,Description=\"Depth\">
##FORMAT=<ID=GT,Number=1,Type=String,Description=\"Genotype\">
#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1\tS2
chr1\t100\trs1\tA\tG\t50\tPASS\tDP=30\tGT\t0/1\t1/1
chr1\t250\t.\tG\tC,A\t60\tPASS\tDP=20\tGT\t1/2\t0/0
chr1\t400\trs2\tAC\tA\t35\tq10\tDP=12\tGT\t0/1\t0/1
chr2\t500\trs3\tT\tTGG\t70\tPASS\tDP=40\tGT\t0/0\t0/1
";

fn main() {
    println!("=== BioCraft Studio — ÇE-04 Varyant İnceleme Demosu (Gün 38) ===\n");

    // Sentetik VCF'i geçici dosyaya yaz.
    let mut yol = std::env::temp_dir();
    yol.push(format!("biocraft_varyant_demo_{}.vcf", std::process::id()));
    File::create(&yol).unwrap().write_all(VCF).unwrap();
    let yol: PathBuf = yol;

    // Motor: gerçek dosya kaynağı (out-of-core) + saf-Rust sorgu motoru ("DuckDB arayüzü").
    let kaynak = DosyaKaynak::ac(&yol, &YetkiKapisi::yeni([Capability::Fs])).unwrap();
    println!(
        "[0] Dosya açıldı — örnekler: {:?}, indeksli: {}, motor: saf-rust (DuckDB arayüzü hazır)",
        kaynak.basligi().ornekler,
        kaynak.basligi().indeksli
    );
    let ornekler = kaynak.basligi().ornekler.clone();
    let mut motor = SafRustMotor::yeni(kaynak);
    let mut g = VaryantGorunumu::yeni(ornekler);

    // 1) Filtresiz tablo (konum sıralı).
    g.yenile(&mut motor).unwrap();
    println!("\n[1] Tüm varyantlar ({} satır):", g.toplam());
    tablo_yaz(&g);

    // 2) Ham sorgu filtresi (QUAL + PASS).
    g.ham_sorgu_uygula("QUAL >= 50 AND FILTER = PASS", &mut motor)
        .unwrap();
    println!(
        "\n[2] Filtre 'QUAL >= 50 AND FILTER = PASS' → {} satır:",
        g.toplam()
    );
    tablo_yaz(&g);

    // 3) QUAL azalan sıralama.
    g.sirala(SiralamaAnahtari::Kalite, false, &mut motor)
        .unwrap();
    println!("\n[3] QUAL azalan sıralı:");
    tablo_yaz(&g);

    // 4) Özet istatistik.
    if let Some(ist) = g.istatistik() {
        println!(
            "\n[4] İstatistik: toplam={} SNV={} INS={} DEL={} PASS={} Ts/Tv={}",
            ist.toplam,
            ist.snv,
            ist.insersiyon,
            ist.delesyon,
            ist.pass,
            ist.ts_tv()
                .map(|v| format!("{v:.2}"))
                .unwrap_or_else(|| "-".into())
        );
    }

    // 5) Çok-örnekli genotip ızgarası (zigosite).
    println!("\n[5] Genotip ızgarası (zigosite):");
    if let Some(izgara) = g.genotip_izgara() {
        print!("    {:<14}", "VARYANT");
        for ad in izgara.ornekler() {
            print!("{ad:<12}");
        }
        println!();
        for satir in 0..izgara.satir_sayisi() {
            let v = g.satir(satir).unwrap();
            print!("    {:<14}", format!("{}:{}", v.kromozom(), v.konum()));
            for ornek in 0..izgara.ornek_sayisi() {
                let h = izgara.hucre(satir, ornek).unwrap();
                print!("{:<12}", format!("{} ({})", h.gt, h.zigosite.etiket()));
            }
            println!();
        }
    }

    // 6) Varyanta tıkla → genom tarayıcıya git (koordinat doğru, 1-tabanlı).
    g.sec(0);
    if let (Some(v), Some(hedef)) = (g.secili_satir(), g.tarayiciya_git()) {
        println!(
            "\n[6] Seçili {}:{} → genom tarayıcı hedefi {}:{}-{} (merkez={})",
            v.kromozom(),
            v.konum(),
            hedef.kromozom,
            hedef.baslangic,
            hedef.bitis,
            hedef.merkez()
        );
    }

    // 7) Kayıtlı filtre seti.
    g.filtre_kaydet("yüksek kalite + PASS");
    println!(
        "\n[7] Filtre seti kaydedildi: {} adet → {:?}",
        g.setler.sayi(),
        g.setler.liste().iter().map(|s| &s.ad).collect::<Vec<_>>()
    );

    // 8) Filtreli dışa aktarma (CSV + VCF).
    println!("\n[8] Filtreli dışa aktarma:");
    println!(
        "    --- CSV ---\n{}",
        g.csv_disa_aktar().unwrap().trim_end()
    );
    println!(
        "    --- VCF ---\n{}",
        g.vcf_disa_aktar().unwrap().trim_end()
    );

    let _ = std::fs::remove_file(&yol);
    println!("\n=== Demo tamam ===");
}

fn tablo_yaz(g: &VaryantGorunumu) {
    let basliklar = g.duzen.basliklar(g.ornekler());
    println!("    {}", basliklar.join(" | "));
    for satir in g.gorunur_satirlar(0, 100) {
        println!("    {}", g.duzen.satir_degerleri(satir).join(" | "));
    }
}
