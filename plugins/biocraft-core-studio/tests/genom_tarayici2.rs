//! ÇE-02 (Gün 37) entegrasyon testi — **genom tarayıcı tamamlama (2. kısım)**.
//!
//! Referans dizi izi + kodon/aminoasit çevirisi (out-of-core FASTA), ileri LOD ("tam göster" +
//! önemli read koruma), çoklu örnek **senkron** karşılaştırma, ölçüm/yer imi araçları, varyant
//! (mismatch/indel) vurgusu ve **PNG/SVG** dışa aktarma — uçtan uca + **golden** (çeviri 3
//! çerçeve doğruluğu, SVG anlık görüntü).  Test verileri **sentetiktir** (CLAUDE.md §7).

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use biocraft_core_studio::data_io::{fai_olustur, BellekButcesi, FastaOkuyucu, VaryantOkuyucu};
use biocraft_core_studio::genome_browser::{
    self as gb, cevir, gorunur_referans_fasta, referans_gerekli, CeviriDurumu, CizimRengi,
    GenomBolge, GenomTarayici, Iz, IzTuru, IzVeri, KarsilastirmaModu, Ornek, OrnekKatman, Palet,
    Primitif, Serit,
};
use biocraft_sdk::biocraft_types::golden;

fn gecici(ad: &str) -> PathBuf {
    let mut yol = std::env::temp_dir();
    yol.push(format!("biocraft_gb37_{}_{ad}", std::process::id()));
    yol
}

fn yaz(yol: &Path, icerik: &[u8]) {
    File::create(yol).unwrap().write_all(icerik).unwrap();
}

fn okuma(ad: &str, bas: u64, bit: u64, serit: Serit) -> gb::OkumaParcasi {
    gb::OkumaParcasi {
        ad: ad.into(),
        bas,
        bit,
        serit,
        mapq: Some(60),
    }
}

// ─── Referans dizi izi + çeviri (out-of-core FASTA) ─────────────────────────────

#[test]
fn referans_dizi_ve_ceviri_out_of_core() {
    // ATG AAA TTT GGG TAA → M K F G *  (ileri çerçeve 0).
    let fa = gecici("ref.fasta");
    yaz(&fa, b">chr1\nATGAAATTTGGGTAACCCGGG\n");
    let _ = fai_olustur(&fa).unwrap();
    let okuyucu = FastaOkuyucu::ac(&fa).unwrap();

    // chr1:1-21, 1000 px → çok yakın → referans gerekli (Baz LOD).
    let mut t = GenomTarayici::yeni(1000.0, GenomBolge::yeni("chr1", 1, 21).unwrap());
    t.kromozom_uzunluklari_ayarla([("chr1".to_string(), 21u64)]);
    t.iz_ekle(Iz::yeni("ref", "Referans", IzTuru::Referans));
    t.iz_yukseklik("ref", 80.0); // baz + 3 çerçeve sığsın
    t.ceviri = CeviriDurumu {
        goster: true,
        serit: Serit::Ileri,
    };

    assert!(
        referans_gerekli(t.tuval()),
        "yakınlaşmada referans yüklenmeli"
    );
    let referans = gorunur_referans_fasta(&okuyucu, t.bolge(), &BellekButcesi::sinirsiz()).unwrap();
    assert_eq!(
        referans.bazlar.len(),
        21,
        "yalnız görünen pencere (out-of-core)"
    );

    let mut veri: BTreeMap<String, IzVeri> = BTreeMap::new();
    veri.insert("ref".into(), IzVeri::Referans(referans.clone()));
    let liste = t.derle(&veri);

    // Renkli bazlar (A,T,G,C hepsi var) + harfler.
    for renk in [
        CizimRengi::BazA,
        CizimRengi::BazT,
        CizimRengi::BazG,
        CizimRengi::BazC,
    ] {
        assert!(
            liste
                .primitifler
                .iter()
                .any(|p| matches!(p, Primitif::Dikdortgen { renk: r, .. } if *r == renk)),
            "baz rengi yok: {renk:?}"
        );
    }
    // Çeviri çerçevesi 0'da dur kodonu (TAA) → AminoAsitDur.
    assert!(liste.primitifler.iter().any(|p| matches!(
        p,
        Primitif::Dikdortgen {
            renk: CizimRengi::AminoAsitDur,
            ..
        }
    )));

    let _ = std::fs::remove_file(&fa);
    let _ = std::fs::remove_file(gecici("ref.fasta").with_extension("fasta.fai"));
}

/// Çeviriyi (3 ileri + 3 geri çerçeve) deterministik metne döker (frame/strand doğruluğu golden).
fn ceviri_dok(referans: &gb::ReferansDizi) -> String {
    let mut s = String::new();
    for (etiket, serit) in [("ileri", Serit::Ileri), ("geri", Serit::Geri)] {
        for cerceve in 0u8..3 {
            let kodonlar = cevir(referans, cerceve, serit);
            let aminolar: String = kodonlar.iter().map(|k| k.amino).collect();
            let ilk = kodonlar
                .first()
                .map(|k| format!("{}-{}", k.bas, k.bit))
                .unwrap_or_else(|| "-".into());
            s.push_str(&format!(
                "{etiket} çerçeve{cerceve}: {aminolar} (ilk kodon {ilk})\n"
            ));
        }
    }
    s
}

#[test]
fn ceviri_3cerceve_golden() {
    let referans = gb::ReferansDizi {
        kromozom: "chr1".into(),
        baslangic: 100,
        bazlar: b"ATGAAATTTGGGTAACCCGGG".to_vec(),
    };
    golden::dogrula("ce02_ceviri_3cerceve", &ceviri_dok(&referans));
}

// ─── Çoklu örnek senkron karşılaştırma ──────────────────────────────────────────

#[test]
fn coklu_ornek_senkron_kaydirma() {
    let mut t = GenomTarayici::yeni(1000.0, GenomBolge::yeni("chr1", 1, 1000).unwrap());
    t.kromozom_uzunluklari_ayarla([("chr1".to_string(), 100_000u64)]);

    let ornekler = vec![
        Ornek::yeni("vaka", "Vaka"),
        Ornek::yeni("kontrol", "Kontrol"),
    ];
    t.ornekleri_ekle(&ornekler, KarsilastirmaModu::YanYana);
    // 2 örnek × (kapsama + reads) = 4 iz.
    assert_eq!(t.izler().sayi(), 4);

    // Aynı okumalar her iki örnekte (senkron kanıtı: aynı bölge → aynı x).
    let okumalar = vec![okuma("r1", 100, 200, Serit::Ileri)];
    let mut veri: BTreeMap<String, IzVeri> = BTreeMap::new();
    veri.insert("vaka.kapsama".into(), IzVeri::Kapsama(okumalar.clone()));
    veri.insert("kontrol.kapsama".into(), IzVeri::Kapsama(okumalar.clone()));

    // Pan + zoom (senkron: tek tuval); sonra derle.
    t.pan_bp(50);
    let liste = t.derle(&veri);

    let yer = |k: &str| t.yerlesim().into_iter().find(|y| y.kimlik == k).unwrap();
    let vaka_y = yer("vaka.kapsama");
    let kontrol_y = yer("kontrol.kapsama");

    // Her lane'deki kapsama çubuğu x'lerini topla.
    let bar_x = |lane: &gb::IzYer| -> Vec<i64> {
        liste
            .primitifler
            .iter()
            .filter_map(|p| match p {
                Primitif::Dikdortgen {
                    x,
                    y,
                    renk: CizimRengi::KapsamaCubuk,
                    ..
                } if *y >= lane.y_ust - 0.5 && *y < lane.y_alt() + 0.5 => Some((x * 100.0) as i64),
                _ => None,
            })
            .collect()
    };
    let vaka_bar = bar_x(&vaka_y);
    let kontrol_bar = bar_x(&kontrol_y);
    assert!(!vaka_bar.is_empty());
    assert_eq!(
        vaka_bar, kontrol_bar,
        "örnekler senkron: aynı bölge tüm örneklerde aynı x'te"
    );
}

#[test]
fn coklu_ornek_overlay_karsilastirma() {
    let mut t = GenomTarayici::yeni(1000.0, GenomBolge::yeni("chr1", 1, 1000).unwrap());
    let ornekler = vec![
        Ornek::yeni("vaka", "Vaka"),
        Ornek::yeni("kontrol", "Kontrol"),
    ];
    t.ornekleri_ekle(&ornekler, KarsilastirmaModu::UstUste);
    assert_eq!(t.izler().sayi(), 1, "üst üste → tek overlay lane");

    let katmanlar = vec![
        OrnekKatman {
            ad: "Vaka".into(),
            renk: CizimRengi::OrnekA,
            okumalar: vec![okuma("v", 100, 300, Serit::Ileri)],
        },
        OrnekKatman {
            ad: "Kontrol".into(),
            renk: CizimRengi::OrnekB,
            okumalar: vec![okuma("k", 200, 400, Serit::Ileri)],
        },
    ];
    let mut veri: BTreeMap<String, IzVeri> = BTreeMap::new();
    veri.insert(
        gb::multisample::OVERLAY_KIMLIK.into(),
        IzVeri::KapsamaCokOrnek(katmanlar),
    );
    let liste = t.derle(&veri);
    for renk in [CizimRengi::OrnekA, CizimRengi::OrnekB] {
        assert!(liste
            .primitifler
            .iter()
            .any(|p| matches!(p, Primitif::Dikdortgen { renk: r, .. } if *r == renk)));
    }
}

// ─── Ölçüm + pozisyon kopyalama + yer imleri ────────────────────────────────────

#[test]
fn olcum_kopyalama_ve_yerimleri() {
    let mut t = GenomTarayici::yeni(1000.0, GenomBolge::yeni("chr1", 1000, 1999).unwrap());
    t.kromozom_uzunluklari_ayarla([("chr1".to_string(), 100_000u64)]);

    // Ölçüm: ekranda iki nokta → bp mesafe.
    t.olcum_ekrandan(0.0, 500.0); // x=0 → 1000, x=500 → 1500
    let o = t.olcum().unwrap();
    assert_eq!(o.sol(), 1000);
    assert_eq!(o.mesafe_bp(), 501); // kapsayıcı 1000..1500

    // Pozisyon kopyalama (clipboard metni).
    assert_eq!(t.konum_kopyala(0.0), "chr1:1,000");
    assert_eq!(t.bolge_kopyala(), "chr1:1,000-1,999");

    // Yer imi: ekle → git → sil (geri-alınabilir).
    t.bolgeye_git("chr1:5000-6000").unwrap();
    let i = t.yerimi_ekle("İlgi bölgesi");
    assert_eq!(t.yerimleri().sayi(), 1);
    t.bolgeye_git("chr1:1-1000").unwrap();
    assert!(t.yerimine_git(i));
    assert_eq!(t.bolge().baslangic, 5000);
    let silinen = t.yerimi_sil(i).unwrap();
    assert_eq!(silinen.ad, "İlgi bölgesi");
    assert_eq!(t.yerimleri().sayi(), 0);

    // Ölçüm çizimde görünür.
    let liste = t.derle(&BTreeMap::new());
    assert!(liste.primitifler.iter().any(|p| matches!(
        p,
        Primitif::Metin {
            renk: CizimRengi::OlcumMetin,
            ..
        }
    )));
}

// ─── Varyant izi + önemli read koruma (out-of-core VCF) ─────────────────────────

const VCF: &[u8] = b"\
##fileformat=VCFv4.3
##contig=<ID=chr1,length=100000>
##INFO=<ID=DP,Number=1,Type=Integer,Description=\"Depth\">
##FORMAT=<ID=GT,Number=1,Type=String,Description=\"Genotype\">
#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1
chr1\t300\trs9\tA\tG\t50\tPASS\tDP=30\tGT\t0/1
chr1\t5000\trsX\tAC\tA\t40\tPASS\tDP=20\tGT\t0/1
";

#[test]
fn varyant_vurgu_ve_onemli_read_koruma() {
    let vcf = gecici("v.vcf");
    yaz(&vcf, VCF);
    let (mut vok, _) = VaryantOkuyucu::ac(&vcf).unwrap();
    let bolge = GenomBolge::yeni("chr1", 1, 1000).unwrap();
    let varyantlar =
        gb::veri::gorunur_varyantlar(&mut vok, &bolge, &BellekButcesi::sinirsiz(), 1000).unwrap();
    assert_eq!(varyantlar.len(), 1, "yalnız chr1:1-1000 varyantı (300)");
    assert_eq!(varyantlar[0].tur, gb::VaryantTuru::Snv);

    // Yoğun read seti (özet LOD'u tetikler) + biri varyantı (300) örter.
    let okumalar: Vec<gb::OkumaParcasi> = (1..=1000)
        .map(|i| okuma(&format!("r{i}"), i, i + 50, Serit::Ileri))
        .collect();
    // 300'ü örten read'ler: bas ≤ 300 ≤ bit → i ∈ [250, 300] = 51 read.
    let ortusen = okumalar
        .iter()
        .filter(|o| o.bas <= 300 && o.bit >= 300)
        .count();
    assert_eq!(ortusen, 51);

    // Dar tuval → bütçe küçük → 1000 read özet moduna düşer.
    let mut t = GenomTarayici::yeni(100.0, bolge.clone());
    t.iz_ekle(Iz::yeni("reads", "Okumalar", IzTuru::Hizalama));
    t.iz_ekle(Iz::yeni("varyant", "Varyantlar", IzTuru::Varyant));

    let mut veri: BTreeMap<String, IzVeri> = BTreeMap::new();
    veri.insert("reads".into(), IzVeri::Hizalama(okumalar.clone()));
    veri.insert("varyant".into(), IzVeri::Varyant(varyantlar.clone()));
    let liste = t.derle(&veri);

    // Özet çubuğu (yoğunluk) + korunan read'lerin tek tek isabeti + varyant işareti.
    assert!(liste.primitifler.iter().any(|p| matches!(
        p,
        Primitif::Dikdortgen {
            renk: CizimRengi::OzetYogunluk,
            ..
        }
    )));
    assert!(liste.primitifler.iter().any(|p| matches!(
        p,
        Primitif::Dikdortgen {
            renk: CizimRengi::VaryantSnv,
            ..
        }
    )));
    // Read isabetleri (varyantı örtenler) + 1 varyant isabeti.
    let read_isabet = liste
        .isabetler
        .iter()
        .filter(|i| i.iz_kimlik == "reads")
        .count();
    assert_eq!(read_isabet, 51, "yoğun bölgede varyant read'leri korunur");
    assert!(liste.isabetler.iter().any(|i| i.iz_kimlik == "varyant"));

    let _ = std::fs::remove_file(&vcf);
}

// ─── Dışa aktarma: SVG (golden) + PNG (geçerli) ─────────────────────────────────

#[test]
fn svg_golden_ve_png_gecerli() {
    // Deterministik küçük görünüm: cetvel + bir read + bir ekson.
    let mut t = GenomTarayici::yeni(400.0, GenomBolge::yeni("chr1", 1, 200).unwrap());
    t.iz_ekle(Iz::yeni("reads", "Okumalar", IzTuru::Hizalama));
    t.iz_ekle(Iz::yeni("genler", "Genler", IzTuru::Anotasyon));
    let mut veri: BTreeMap<String, IzVeri> = BTreeMap::new();
    veri.insert(
        "reads".into(),
        IzVeri::Hizalama(vec![okuma("r1", 20, 80, Serit::Ileri)]),
    );
    veri.insert(
        "genler".into(),
        IzVeri::Anotasyon(vec![gb::OzellikParcasi {
            ad: Some("GENE1".into()),
            bas: 10,
            bit: 150,
            serit: Serit::Ileri,
            tur: "gene".into(),
        }]),
    );
    let liste = t.derle(&veri);
    let yuk = t.toplam_yukseklik();

    // SVG: yayın kalitesi vektör (golden).
    let svg = gb::svg_olustur(&liste, 400.0, yuk, &Palet::yayin());
    golden::dogrula("ce02_svg_snapshot", &svg);

    // PNG: geçerli imza + makul boyut.
    let png = gb::png_olustur(&liste, 400, yuk as u32, &Palet::yayin());
    assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    assert!(png.len() > 100, "PNG boş olmamalı");
}

// ─── "Tam göster" (LOD bypass) ───────────────────────────────────────────────────

#[test]
fn tam_goster_lod_atlar() {
    let okumalar: Vec<gb::OkumaParcasi> = (1..=1000)
        .map(|i| okuma(&format!("r{i}"), i, i + 30, Serit::Ileri))
        .collect();
    let mut veri: BTreeMap<String, IzVeri> = BTreeMap::new();
    veri.insert("reads".into(), IzVeri::Hizalama(okumalar));

    let mut t = GenomTarayici::yeni(100.0, GenomBolge::yeni("chr1", 1, 1100).unwrap());
    t.iz_ekle(Iz::yeni("reads", "Okumalar", IzTuru::Hizalama));

    // Varsayılan: özet (akıcı) → tek tek isabet yok.
    let ozet = t.derle(&veri);
    assert!(ozet.isabetler.is_empty());

    // Tam göster: tüm read'ler tek tek → isabetler var.
    t.tam_goster = true;
    let tam = t.derle(&veri);
    assert!(
        !tam.isabetler.is_empty(),
        "tam göster: tüm read'ler çizilir"
    );
}
