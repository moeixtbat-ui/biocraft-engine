//! ÇE-11 (Gün 42) entegrasyon testi — **Dışa Aktarma ve Oturum**.
//!
//! Uçtan uca: görsel (yüksek-DPI PNG + vektör SVG + vektör PDF) + veri (CSV/TSV/FASTA/VCF, seçim
//! korunur) + görünüm oturumu (kaydet/yükle, eksik alana güvenli varsayılan) + eklenti içi geçmiş +
//! temel rapor (köken/atıf/parametre + gizlilik) + **golden** (deterministik dışa aktarma dökümü).
//! Veriler **sentetiktir** (CLAUDE.md §7) — gerçek hasta verisi repoya girmez.

use biocraft_core_studio::data_io::{LisansAtif, Provenans};
use biocraft_core_studio::db_search::HassasiyetEtiketi;
use biocraft_core_studio::export::{
    cizimi_disa_aktar, fasta_olustur, tablo_disa_aktar, varyant_vcf, ArkaPlan, Ayrac, Boyut,
    FastaKaydi, Gecmis, GecmisTuru, GizlilikSuzgeci, GorselAyari, GorselCikti, GorselFormat,
    OturumDurumu, Rapor, Tablo,
};
use biocraft_core_studio::genome_browser::{CizimListesi, CizimRengi, MetinHiza, Primitif};
use biocraft_sdk::biocraft_types::golden;

/// Küçük, deterministik bir genom-tarayıcı çizim listesi (cetvel çizgisi + ekson kutusu + gen etiketi).
fn ornek_cizim() -> CizimListesi {
    let mut l = CizimListesi::yeni();
    l.primitifler.push(Primitif::Cizgi {
        x1: 0.0,
        y1: 10.0,
        x2: 200.0,
        y2: 10.0,
        renk: CizimRengi::CetvelCizgi,
        kalinlik: 1.0,
    });
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

// ─── 1) Görsel: yüksek-DPI PNG + vektör SVG + vektör PDF ────────────────────────────

#[test]
fn gorsel_png_yuksek_dpi() {
    let l = ornek_cizim();
    let ayari = GorselAyari::yayin(Boyut::yeni(200.0, 100.0)); // 300 DPI
    let cikti = cizimi_disa_aktar(&l, &ayari);
    match cikti {
        GorselCikti::Png(b) => {
            // Geçerli PNG imzası.
            assert_eq!(&b[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
            // 300 DPI → 200×3.125 = 625 px genişlik IHDR'de yazılı olmalı.
            // IHDR veri ilk 4 baytı = genişlik (imza 8 + uzunluk 4 + "IHDR" 4 = 16. ofsetten).
            let gen = u32::from_be_bytes([b[16], b[17], b[18], b[19]]);
            assert_eq!(gen, 625, "yüksek DPI raster genişliği");
        }
        _ => panic!("PNG bekleniyordu"),
    }
}

#[test]
fn gorsel_svg_vektor_etiketli() {
    let l = ornek_cizim();
    let ayari = GorselAyari::yayin(Boyut::yeni(200.0, 100.0)).with_format(GorselFormat::Svg);
    if let GorselCikti::Svg(s) = cizimi_disa_aktar(&l, &ayari) {
        assert!(s.starts_with("<svg"));
        assert!(s.contains("BRCA1")); // metin etiketi vektörde (yayın kalitesi)
        assert!(s.contains("<rect")); // ekson kutusu
        assert!(s.contains("<line")); // cetvel çizgisi
    } else {
        panic!("SVG bekleniyordu");
    }
}

#[test]
fn gorsel_pdf_vektor() {
    let l = ornek_cizim();
    let ayari = GorselAyari::yayin(Boyut::yeni(200.0, 100.0)).with_format(GorselFormat::Pdf);
    if let GorselCikti::Pdf(b) = cizimi_disa_aktar(&l, &ayari) {
        let m = String::from_utf8_lossy(&b);
        assert!(m.starts_with("%PDF-1.4"));
        assert!(m.contains("(BRCA1) Tj"));
        assert!(m.trim_end().ends_with("%%EOF"));
    } else {
        panic!("PDF bekleniyordu");
    }
}

#[test]
fn gorsel_saydam_arka_plan() {
    let l = ornek_cizim();
    let svg = cizimi_disa_aktar(
        &l,
        &GorselAyari::yayin(Boyut::yeni(200.0, 100.0))
            .with_format(GorselFormat::Svg)
            .with_arka_plan(ArkaPlan::Saydam),
    );
    if let GorselCikti::Svg(s) = svg {
        // 200×100 zemin dikdörtgeni saydamda yok.
        assert!(!s.contains("width=\"200\" height=\"100\" fill=\"#"));
    } else {
        panic!("SVG bekleniyordu");
    }
}

// ─── 2) Veri: CSV/TSV (seçim korunur) + FASTA + VCF ─────────────────────────────────

fn ornek_tablo() -> Tablo {
    let mut t = Tablo::yeni(vec!["CHROM".into(), "POS".into(), "GEN".into()]);
    t.satir_ekle(vec!["chr1".into(), "100".into(), "BRCA1".into()]);
    t.satir_ekle(vec!["chr17".into(), "7579472".into(), "TP53".into()]);
    t.satir_ekle(vec!["chr7".into(), "55191822".into(), "EGFR".into()]);
    t
}

#[test]
fn veri_csv_tsv_secim_korunur() {
    let t = ornek_tablo();
    // Tüm satırlar CSV.
    let csv = tablo_disa_aktar(&t, Ayrac::Virgul, None);
    assert_eq!(csv.lines().count(), 4); // başlık + 3

    // Yalnız seçili 2 satır (sıra korunur) TSV.
    let tsv = tablo_disa_aktar(&t, Ayrac::Sekme, Some(&[2, 0]));
    let satirlar: Vec<&str> = tsv.lines().collect();
    assert_eq!(satirlar.len(), 3); // başlık + 2 seçili
    assert!(satirlar[1].starts_with("chr7")); // önce 2. indeks
    assert!(satirlar[1].contains('\t'));
}

#[test]
fn veri_fasta() {
    let kayitlar = vec![
        FastaKaydi::yeni("seq1", "ACGTACGTACGTAC").with_aciklama("sentetik"),
        FastaKaydi::yeni("seq2", "TTTTGGGG"),
    ];
    let fasta = fasta_olustur(&kayitlar, 10);
    assert!(fasta.starts_with(">seq1 sentetik\n"));
    assert!(fasta.contains(">seq2\nTTTTGGGG\n"));
}

#[test]
fn veri_vcf_koprusu() {
    // Köprü ÇE-04 üreticisine bağlı; boş girdi geçerli VCF başlığı üretir.
    let vcf = varyant_vcf(&[], &[]);
    assert!(vcf.starts_with("##fileformat=VCFv4.3"));
}

#[test]
fn veri_gizlilik_phi_sutunu_dusurulur() {
    let mut t = ornek_tablo();
    t.basliklar.push("HASTA_ID".into());
    t.sutun_etiket.push(HassasiyetEtiketi::Phi);
    for (i, s) in t.satirlar.iter_mut().enumerate() {
        s.push(format!("PID{i}"));
    }
    // Onaylı filtrede bile PHI sütunu çıkmaz.
    let suzulmus = GizlilikSuzgeci::onayli().temizle(&t);
    let csv = tablo_disa_aktar(&suzulmus, Ayrac::Virgul, None);
    assert!(!csv.contains("HASTA_ID"));
    assert!(!csv.contains("PID0"));
}

// ─── 3) Görünüm oturumu: kaydet/yükle + eksik alan güvenli varsayılan ───────────────

#[test]
fn oturum_round_trip() {
    let mut o = OturumDurumu::yeni();
    o.dosya_ekle("ornek.bam")
        .iz_ekle("hizalama", true, 80.0)
        .ayar("tema", "koyu");
    o.bolge = Some("chr1:1000-2000".into());

    let json = o.to_json();
    let geri = OturumDurumu::yukle(&json).unwrap();
    assert_eq!(o, geri);
    assert_eq!(geri.bolge.as_deref(), Some("chr1:1000-2000"));
}

#[test]
fn oturum_eksik_alan_bozulmaz() {
    // Eski/eksik oturum — diğer alanlar güvenli varsayılana düşer, panik yok.
    let o = OturumDurumu::yukle(r#"{ "bolge": "chr2:5-9" }"#).unwrap();
    assert_eq!(o.bolge.as_deref(), Some("chr2:5-9"));
    assert!(o.izler.is_empty());
    assert!(o.acik_dosyalar.is_empty());
}

#[test]
fn oturum_bozuk_json_net_hata() {
    let hata = OturumDurumu::yukle("{ bozuk ").err().unwrap();
    assert_eq!(hata.ne_oldu, "Oturum dosyası okunamadı");
    assert!(!hata.nasil_cozulur.is_empty());
}

// ─── 4) Eklenti içi geçmiş ──────────────────────────────────────────────────────────

#[test]
fn gecmis_tekillestir_kapasite_rehber() {
    let mut g = Gecmis::azami_ile(3);
    assert!(g.rehber().is_some()); // boş → rehber

    g.ekle(GecmisTuru::Dosya, "a.bam");
    g.ekle(GecmisTuru::Bolge, "chr1:1-100");
    g.ekle(GecmisTuru::Dosya, "a.bam"); // tekrar → öne
    assert_eq!(g.say(), 2);
    assert_eq!(g.son(1)[0].etiket, "a.bam");
    assert!(g.rehber().is_none());

    // JSON round-trip.
    let geri = Gecmis::from_json(&g.to_json());
    assert_eq!(g, geri);
}

// ─── 5) Temel rapor: köken/atıf/parametre + gizlilik ────────────────────────────────

fn ncbi_kaynak() -> Provenans {
    Provenans {
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
    }
}

#[test]
fn rapor_koken_atif_ve_gizlilik() {
    let r = Rapor::yeni("BRCA1 Analizi")
        .with_ozet("BRCA1 bölgesinde varyant taraması.")
        .bolum_ekle("Bulgular", "2 patojenik varyant.")
        .kaynak_ekle(ncbi_kaynak())
        .parametre_ekle("QUAL eşiği", "30")
        .parametre_ekle_etiketli("hasta_id", "X-9", HassasiyetEtiketi::Phi)
        .gorsel_ekle("sekil1.svg", "Genom tarayıcı görünümü");

    let md = r.markdown();
    assert!(md.contains("# BRCA1 Analizi"));
    assert!(md.contains("## Yöntem ve Materyaller"));
    assert!(md.contains("NM_007294.fasta"));
    assert!(md.contains("NCBI, NLM, E-utilities")); // atıf
    assert!(md.contains("| QUAL eşiği | 30 |"));
    // PHI parametre rapora SIZMAZ.
    assert!(!md.contains("hasta_id"));
    assert!(!md.contains("X-9"));

    // HTML + PDF de üretilir.
    let html = r.html();
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(!html.contains("X-9"));
    let pdf = r.pdf();
    assert!(String::from_utf8_lossy(&pdf).starts_with("%PDF-1.4"));
}

// ─── 6) Golden: deterministik dışa aktarma dökümü ───────────────────────────────────

#[test]
fn golden_disa_aktarma_dokumu() {
    let mut metin = String::new();

    // SVG (DPI'dan bağımsız; deterministik).
    metin.push_str("=== SVG ===\n");
    if let GorselCikti::Svg(s) = cizimi_disa_aktar(
        &ornek_cizim(),
        &GorselAyari::yayin(Boyut::yeni(200.0, 100.0)).with_format(GorselFormat::Svg),
    ) {
        metin.push_str(&s);
    }

    // CSV tablo (gizlilik uygulanmış).
    metin.push_str("\n=== CSV ===\n");
    metin.push_str(&tablo_disa_aktar(
        &GizlilikSuzgeci::yeni().temizle(&ornek_tablo()),
        Ayrac::Virgul,
        None,
    ));

    // FASTA.
    metin.push_str("=== FASTA ===\n");
    metin.push_str(&fasta_olustur(
        &[FastaKaydi::yeni("seq1", "ACGTACGTAC").with_aciklama("test")],
        60,
    ));

    golden::dogrula("ce11_disa_aktarma", &metin);
}
