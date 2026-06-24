//! ÇE-11 (Gün 42) demo — **Dışa Aktarma ve Oturum**.
//!
//! `cargo run -p biocraft-core-studio --example disa_aktarma_demo`
//!
//! Çalıştırma: yayın kalitesi görsel (PNG/SVG/PDF), veri (CSV/FASTA/VCF), görünüm oturumu kaydet/yükle,
//! eklenti içi geçmiş ve köken/atıf içeren temel rapor — uçtan uca, dosya yazmadan (saf metin/bayt).

use biocraft_core_studio::data_io::{LisansAtif, Provenans};
use biocraft_core_studio::export::{
    cizimi_disa_aktar, fasta_olustur, tablo_disa_aktar, ArkaPlan, Ayrac, Boyut, FastaKaydi, Gecmis,
    GecmisTuru, GorselAyari, GorselFormat, OturumDurumu, Rapor, Tablo,
};
use biocraft_core_studio::genome_browser::{CizimListesi, CizimRengi, MetinHiza, Primitif};

fn cizim() -> CizimListesi {
    let mut l = CizimListesi::yeni();
    l.primitifler.push(Primitif::Dikdortgen {
        x: 40.0,
        y: 30.0,
        gen: 80.0,
        yuk: 14.0,
        renk: CizimRengi::Ekson,
    });
    l.primitifler.push(Primitif::Metin {
        x: 40.0,
        y: 48.0,
        icerik: "BRCA1".into(),
        renk: CizimRengi::AnotasyonMetin,
        boyut: 11.0,
        hiza: MetinHiza::Sol,
    });
    l
}

fn main() {
    println!("=== BioCraft Studio — ÇE-11 Dışa Aktarma ve Oturum demosu ===\n");

    // 1) Görsel: aynı görünüm, üç biçim + yüksek DPI.
    println!("[1] Görsel dışa aktarma (yayın kalitesi)");
    let l = cizim();
    let boyut = Boyut::yeni(400.0, 200.0);
    let png = cizimi_disa_aktar(&l, &GorselAyari::yayin(boyut).with_dpi(96));
    let png300 = cizimi_disa_aktar(&l, &GorselAyari::yayin(boyut)); // 300 DPI
    let svg = cizimi_disa_aktar(
        &l,
        &GorselAyari::yayin(boyut).with_format(GorselFormat::Svg),
    );
    let pdf = cizimi_disa_aktar(
        &l,
        &GorselAyari::yayin(boyut).with_format(GorselFormat::Pdf),
    );
    let saydam = cizimi_disa_aktar(
        &l,
        &GorselAyari::yayin(boyut)
            .with_format(GorselFormat::Svg)
            .with_arka_plan(ArkaPlan::Saydam),
    );
    println!("    PNG  96 DPI : {} bayt (400×200 px)", png.boyut_bayt());
    println!(
        "    PNG 300 DPI : {} bayt ({:?} px) — yüksek DPI",
        png300.boyut_bayt(),
        GorselAyari::yayin(boyut).raster_boyut()
    );
    println!(
        "    SVG (vektör): {} bayt (etiketli, ölçeklenebilir)",
        svg.boyut_bayt()
    );
    println!("    PDF (vektör): {} bayt", pdf.boyut_bayt());
    println!(
        "    SVG saydam  : {} bayt (arka plan yok)\n",
        saydam.boyut_bayt()
    );

    // 2) Veri: tablo (seçim) + FASTA.
    println!("[2] Veri dışa aktarma");
    let mut t = Tablo::yeni(vec!["CHROM".into(), "POS".into(), "GEN".into()]);
    t.satir_ekle(vec!["chr1".into(), "100".into(), "BRCA1".into()]);
    t.satir_ekle(vec!["chr17".into(), "7579472".into(), "TP53".into()]);
    println!("    CSV (tümü):");
    for satir in tablo_disa_aktar(&t, Ayrac::Virgul, None).lines() {
        println!("      {satir}");
    }
    let fasta = fasta_olustur(&[FastaKaydi::yeni("seq1", "ACGTACGTACGT")], 60);
    println!(
        "    FASTA:\n      {}\n",
        fasta.replace('\n', "\n      ").trim_end()
    );

    // 3) Görünüm oturumu: kaydet → yükle.
    println!("[3] Görünüm oturumu (kaldığın görünüme dön)");
    let mut o = OturumDurumu::yeni();
    o.dosya_ekle("ornek.bam")
        .iz_ekle("hizalama", true, 80.0)
        .ayar("tema", "koyu");
    o.bolge = Some("chr17:43044295-43125483".into());
    let json = o.to_json();
    let geri = OturumDurumu::yukle(&json).unwrap();
    println!(
        "    Kaydedildi → bölge={:?}, iz={}",
        geri.bolge,
        geri.izler.len()
    );
    println!("    Eksik alanlı eski oturum da güvenle yüklenir (varsayılana düşer).\n");

    // 4) Geçmiş.
    println!("[4] Eklenti içi geçmiş");
    let mut g = Gecmis::yeni();
    g.ekle(GecmisTuru::Dosya, "ornek.bam");
    g.ekle(GecmisTuru::Bolge, "chr17:43044295-43125483");
    g.ekle(GecmisTuru::Islem, "QUAL>=30 filtresi + CSV dışa aktarma");
    for girdi in g.son(5) {
        println!("    [{}] {}", girdi.tur.etiket(), girdi.etiket);
    }
    println!();

    // 5) Rapor: köken + atıf + parametre.
    println!("[5] Temel rapor (köken + atıf + Yöntem ve Materyaller)");
    let kaynak = Provenans {
        veri_kimligi: "NM_007294.fasta".into(),
        kaynak: "NCBI nucleotide (efetch)".into(),
        format: "FASTA".into(),
        surum: String::new(),
        tarih: chrono::Utc::now(),
        blake3: "a".repeat(64),
        boyut_bayt: 1234,
        lisans_atif: Some(LisansAtif {
            lisans: "Public Domain".into(),
            atif: "NCBI, NLM, E-utilities".into(),
            url: Some("https://www.ncbi.nlm.nih.gov".into()),
        }),
    };
    let rapor = Rapor::yeni("BRCA1 Varyant Analizi")
        .with_ozet("chr17 BRCA1 bölgesinde varyant taraması.")
        .bolum_ekle("Bulgular", "2 patojenik varyant saptandı.")
        .kaynak_ekle(kaynak)
        .parametre_ekle("QUAL eşiği", "30")
        .gorsel_ekle("sekil1.svg", "BRCA1 genom tarayıcı görünümü");
    println!("    --- Markdown rapor ---");
    for satir in rapor.markdown().lines() {
        println!("    {satir}");
    }
    println!(
        "\n    (HTML: {} bayt, PDF: {} bayt da üretilir)",
        rapor.html().len(),
        rapor.pdf().len()
    );

    println!(
        "\n=== Demo tamam — tüm çıktılar gizlilik sınırına uyar (hassas alan onaysız sızmaz). ==="
    );
}
