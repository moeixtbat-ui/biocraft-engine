//! ÇE-01 (Gün 35) entegrasyon testi — **varyant + anotasyon + GenBank + 2bit + yapı + uzak/karantina**.
//!
//! Bölge sorgusu (linear/out-of-core) + format otomatik tanıma + **golden** doğruluk (bcftools/
//! bedtools-eşdeğeri bilinen-doğru çıktı, Gün 32 çerçevesi, MK-58) + uzak bayt-aralığı + MK-33
//! karantina.  Test verileri **sentetiktir** (gerçek hasta verisi repoya girmez — CLAUDE.md §7).

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use biocraft_core_studio::data_io::{
    dogrula_veya_karantina, formati_belirle, genbank_ilk_kayit, AnotasyonKaydi, AnotasyonOkuyucu,
    BellekButcesi, TwoBitOkuyucu, UzakOkuyucu, VaryantKaydi, VaryantOkuyucu, VeriFormati, Yapi,
    YerelBaytAralik,
};
use biocraft_sdk::biocraft_types::golden;

fn gecici(ad: &str) -> PathBuf {
    let mut yol = std::env::temp_dir();
    yol.push(format!("biocraft_veriio2_{}_{ad}", std::process::id()));
    yol
}

fn yaz(yol: &Path, icerik: &[u8]) {
    File::create(yol).unwrap().write_all(icerik).unwrap();
}

// ─── Varyant (VCF) ───────────────────────────────────────────────────────────────

const VCF: &[u8] = b"\
##fileformat=VCFv4.3
##contig=<ID=chr1,length=1000>
##INFO=<ID=DP,Number=1,Type=Integer,Description=\"Depth\">
##INFO=<ID=AF,Number=A,Type=Float,Description=\"Allele Frequency\">
##FORMAT=<ID=GT,Number=1,Type=String,Description=\"Genotype\">
##FORMAT=<ID=DP,Number=1,Type=Integer,Description=\"Depth\">
#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1\tS2
chr1\t100\trs1\tA\tT\t50\tPASS\tDP=30;AF=0.5\tGT:DP\t0/1:15\t1/1:20
chr1\t250\t.\tG\tC,A\t60\tPASS\tDP=20\tGT:DP\t1/2:10\t0/0:8
chr1\t900\trs3\tT\tG\t40\tq10\tDP=10\tGT:DP\t0/0:5\t0/1:6
";

/// Varyant kayıtlarını deterministik golden metne çevirir.
fn vcf_dok(kayitlar: &[VaryantKaydi]) -> String {
    let mut s = String::new();
    for k in kayitlar {
        s.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\tqual={}\tfilt={}\tinfo={}\tornek={}\tfmt={}\n",
            k.kromozom,
            k.konum,
            k.kimlik,
            k.referans,
            k.alternatifler.join(","),
            k.kalite
                .map(|q| format!("{q:.1}"))
                .unwrap_or_else(|| "-".into()),
            k.filtreler.join(";"),
            k.info
                .iter()
                .map(|(a, _)| a.as_str())
                .collect::<Vec<_>>()
                .join(","),
            k.ornek_sayisi,
            k.format_anahtarlari.join(":"),
        ));
    }
    s
}

#[test]
fn vcf_bolge_sorgusu_golden_ve_alanlar() {
    let p = gecici("a.vcf");
    yaz(&p, VCF);
    let (mut okuyucu, basligi) = VaryantOkuyucu::ac(&p).unwrap();
    assert_eq!(basligi.format, VeriFormati::Vcf);
    assert_eq!(basligi.ornekler, vec!["S1", "S2"]);

    // chr1:1-500 → ilk iki varyant (100, 250); 900 hariç.
    let kayitlar = okuyucu
        .bolge_sorgu("chr1:1-500", &BellekButcesi::sinirsiz(), 1000)
        .unwrap();
    assert_eq!(kayitlar.len(), 2);
    assert_eq!(kayitlar[0].ornek_sayisi, 2);
    assert_eq!(kayitlar[0].format_anahtarlari, vec!["GT", "DP"]);
    assert_eq!(kayitlar[1].alternatifler, vec!["C", "A"]);

    // Golden (bcftools view -r chr1:1-500 eşdeğeri özet).
    golden::dogrula("ce01_vcf_chr1_1_500", &vcf_dok(&kayitlar));
    let _ = std::fs::remove_file(&p);
}

#[test]
fn vcf_dar_bolge_yalniz_ortusen() {
    let p = gecici("b.vcf");
    yaz(&p, VCF);
    let (mut okuyucu, _) = VaryantOkuyucu::ac(&p).unwrap();
    let kayitlar = okuyucu
        .bolge_sorgu("chr1:880-1000", &BellekButcesi::sinirsiz(), 1000)
        .unwrap();
    assert_eq!(kayitlar.len(), 1);
    assert_eq!(kayitlar[0].konum, 900);
    assert_eq!(kayitlar[0].filtreler, vec!["q10"]);
    let _ = std::fs::remove_file(&p);
}

// ─── Anotasyon (BED / GFF) ─────────────────────────────────────────────────────

/// Anotasyon kayıtlarını deterministik golden metne çevirir.
fn annot_dok(kayitlar: &[AnotasyonKaydi]) -> String {
    let mut s = String::new();
    for k in kayitlar {
        s.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            k.kromozom,
            k.baslangic,
            k.bitis,
            k.tur,
            k.ad.as_deref().unwrap_or("-"),
            k.serit,
        ));
    }
    s
}

#[test]
fn bed_bolge_sorgusu_golden() {
    // bedtools intersect eşdeğeri: chr1:150-650 ile örtüşen BED özellikleri.
    let p = gecici("a.bed");
    yaz(
        &p,
        b"chr1\t100\t200\tgeneA\nchr1\t300\t400\tgeneB\nchr1\t600\t700\tgeneC\nchr2\t10\t20\tgeneD\n",
    );
    let (okuyucu, _) = AnotasyonOkuyucu::ac(&p).unwrap();
    let kayitlar = okuyucu
        .bolge_sorgu("chr1:150-650", &BellekButcesi::sinirsiz(), 1000)
        .unwrap();
    // geneA (100-200), geneB (300-400), geneC (600-700) örtüşür; geneD (chr2) hariç.
    assert_eq!(kayitlar.len(), 3);
    golden::dogrula("ce01_bed_chr1_150_650", &annot_dok(&kayitlar));
    let _ = std::fs::remove_file(&p);
}

#[test]
fn gff_gen_transkript_ekson_golden() {
    let p = gecici("a.gff3");
    yaz(
        &p,
        b"##gff-version 3\n\
chr1\tHAVANA\tgene\t1000\t5000\t.\t+\t.\tID=geneX;Name=FOO\n\
chr1\tHAVANA\tmRNA\t1000\t5000\t.\t+\t.\tID=tx1;Parent=geneX\n\
chr1\tHAVANA\texon\t1000\t1200\t.\t+\t.\tID=ex1;Parent=tx1\n\
chr1\tHAVANA\texon\t4800\t5000\t.\t+\t.\tID=ex2;Parent=tx1\n",
    );
    let (okuyucu, _) = AnotasyonOkuyucu::ac(&p).unwrap();
    let kayitlar = okuyucu
        .bolge_sorgu("chr1:1-6000", &BellekButcesi::sinirsiz(), 1000)
        .unwrap();
    assert_eq!(kayitlar.len(), 4);
    let turler: Vec<&str> = kayitlar.iter().map(|k| k.tur.as_str()).collect();
    assert_eq!(turler, vec!["gene", "mRNA", "exon", "exon"]);
    golden::dogrula("ce01_gff_chr1_genemodel", &annot_dok(&kayitlar));
    let _ = std::fs::remove_file(&p);
}

// ─── GenBank (F1: dizi + özellik) ──────────────────────────────────────────────

#[test]
fn genbank_dizi_ve_ozellik() {
    let p = gecici("a.gb");
    yaz(
        &p,
        b"\
LOCUS       SYN001                  12 bp    DNA     linear   SYN 23-JUN-2026
DEFINITION  Mini sentetik.
ACCESSION   SYN001
FEATURES             Location/Qualifiers
     source          1..12
                     /organism=\"synthetic\"
     gene            1..12
                     /gene=\"g1\"
ORIGIN
        1 acgtacgtac gt
//
",
    );
    let kayit = genbank_ilk_kayit(&p).unwrap();
    assert_eq!(kayit.locus, "SYN001");
    assert_eq!(kayit.dizi, b"acgtacgtacgt");
    assert_eq!(kayit.ozellikler.len(), 2);
    assert_eq!(kayit.ozellikler[1].tur, "gene");
    assert_eq!(
        kayit.ozellikler[1].nitelikler[0],
        ("gene".into(), "g1".into())
    );
    let _ = std::fs::remove_file(&p);
}

// ─── 2bit (referans dizi) ──────────────────────────────────────────────────────

#[test]
fn twobit_bolge_okuma() {
    // İki dizi: s1="ACGTACGT" (8), s2="TTTTGGGG" (8).
    let mut v: Vec<u8> = Vec::new();
    v.extend_from_slice(&0x1A41_2743u32.to_le_bytes()); // imza
    v.extend_from_slice(&0u32.to_le_bytes()); // version
    v.extend_from_slice(&2u32.to_le_bytes()); // dizi sayısı
    v.extend_from_slice(&0u32.to_le_bytes()); // reserved
                                              // İndeks: başlık(16)+ index iki giriş = (1+2+4)*2 = 14 → kayıtlar 30'dan başlar.
    v.push(2);
    v.extend_from_slice(b"s1");
    v.extend_from_slice(&30u32.to_le_bytes());
    v.push(2);
    v.extend_from_slice(b"s2");
    v.extend_from_slice(&(30u32 + 18).to_le_bytes()); // s1 kaydı 18 bayt (16 başlık + 2 DNA)
                                                      // s1 kaydı @30.
    v.extend_from_slice(&8u32.to_le_bytes()); // dnaSize
    v.extend_from_slice(&0u32.to_le_bytes()); // nBlock
    v.extend_from_slice(&0u32.to_le_bytes()); // maskBlock
    v.extend_from_slice(&0u32.to_le_bytes()); // reserved
    v.push(0x9C); // ACGT
    v.push(0x9C); // ACGT
                  // s2 kaydı @48. "TTTT" = T(00)x4 = 0x00; "GGGG" = G(11)x4 = 0xFF.
    v.extend_from_slice(&8u32.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.push(0x00); // TTTT
    v.push(0xFF); // GGGG

    let p = gecici("a.2bit");
    yaz(&p, &v);
    let r = TwoBitOkuyucu::ac(&p).unwrap();
    assert_eq!(r.dizi_adlari(), vec!["s1", "s2"]);
    assert_eq!(
        r.bolge("s1:1-8", &BellekButcesi::sinirsiz())
            .unwrap()
            .diziler,
        b"ACGTACGT"
    );
    assert_eq!(
        r.bolge("s2:1-8", &BellekButcesi::sinirsiz())
            .unwrap()
            .diziler,
        b"TTTTGGGG"
    );
    let _ = std::fs::remove_file(&p);
}

// ─── Yapı (PDB / mmCIF) ────────────────────────────────────────────────────────

#[test]
fn pdb_ve_mmcif_ayni_yapiyi_verir() {
    let pdb = gecici("a.pdb");
    yaz(
        &pdb,
        b"ATOM      1  N   MET A   1      11.104  13.207  10.567  1.00 20.00           N\n\
ATOM      2  CA  MET A   1      12.560  13.000  10.420  1.00 20.00           C\n\
END\n",
    );
    let y = Yapi::oku(&pdb, &BellekButcesi::sinirsiz()).unwrap();
    assert_eq!(y.atom_sayisi(), 2);
    assert_eq!(y.zincirler(), vec!["A"]);

    let cif = gecici("a.cif");
    yaz(
        &cif,
        b"data_T\nloop_\n_atom_site.group_PDB\n_atom_site.id\n_atom_site.type_symbol\n\
_atom_site.label_atom_id\n_atom_site.label_comp_id\n_atom_site.label_asym_id\n\
_atom_site.label_seq_id\n_atom_site.Cartn_x\n_atom_site.Cartn_y\n_atom_site.Cartn_z\n\
_atom_site.pdbx_PDB_model_num\n\
ATOM 1 N N MET A 1 11.104 13.207 10.567 1\n\
ATOM 2 C CA MET A 1 12.560 13.000 10.420 1\n#\n",
    );
    let y2 = Yapi::oku(&cif, &BellekButcesi::sinirsiz()).unwrap();
    assert_eq!(y2.atom_sayisi(), 2);
    // PDB ve mmCIF aynı koordinatları verir.
    assert!((y.modeller[0].atomlar[0].x - y2.modeller[0].atomlar[0].x).abs() < 1e-3);
    let _ = std::fs::remove_file(&pdb);
    let _ = std::fs::remove_file(&cif);
}

// ─── Format otomatik tanıma süpürmesi ──────────────────────────────────────────

#[test]
fn format_otomatik_tanima_supurmesi() {
    let cases: &[(&str, &[u8], VeriFormati)] = &[
        ("x.vcf", b"##fileformat=VCFv4.3\n", VeriFormati::Vcf),
        ("x.bed", b"chr1\t1\t2\n", VeriFormati::Bed),
        ("x.gff3", b"##gff-version 3\n", VeriFormati::Gff),
        (
            "x.gtf",
            b"chr1\ta\tgene\t1\t2\t.\t+\t.\tgene_id \"x\";\n",
            VeriFormati::Gtf,
        ),
        ("x.gb", b"LOCUS x\n", VeriFormati::GenBank),
        ("x.pdb", b"ATOM      1  N\n", VeriFormati::Pdb),
        ("x.cif", b"data_x\n", VeriFormati::MmCif),
    ];
    for (ad, icerik, beklenen) in cases {
        let p = gecici(ad);
        yaz(&p, icerik);
        assert_eq!(formati_belirle(&p).unwrap(), *beklenen, "{ad}");
        let _ = std::fs::remove_file(&p);
    }
}

// ─── Uzak bayt-aralığı + KARANTİNA (MK-33) ─────────────────────────────────────

#[test]
fn uzak_bolge_tumunu_indirmeden_okur() {
    // 2000 baytlık "uzak" dosya; yalnızca [500,520) çekilir.
    let veri: Vec<u8> = (0..2000u32).map(|i| (i % 256) as u8).collect();
    let p = gecici("uzak.bin");
    yaz(&p, &veri);
    let backend = YerelBaytAralik::yeni(&p);
    assert_eq!(backend.boyut().unwrap(), 2000);
    let parca = backend.bayt_araligi(500, 20).unwrap();
    assert_eq!(parca.len(), 20);
    assert_eq!(parca, &veri[500..520]);
    let _ = std::fs::remove_file(&p);
}

#[test]
fn bozuk_indirme_karantinaya_alinir_sessiz_acma_yok() {
    let p = gecici("indirilen.dat");
    yaz(&p, b"bozuk/eksik indirilmis icerik");
    let karantina = gecici("karantina_dizin");
    // Yanlış BLAKE3 → karantina + hata; dosya yerinde KALMAZ (sessiz açma yok — MK-33).
    let hata = dogrula_veya_karantina(&p, &"a".repeat(64), &karantina)
        .err()
        .unwrap();
    assert_eq!(hata.ne_oldu, "Dosya bütünlüğü doğrulanamadı");
    assert_eq!(hata.eylem_etiketi.as_deref(), Some("Yeniden indir"));
    assert!(!p.exists(), "bozuk dosya karantinaya taşınmalı");
    assert_eq!(std::fs::read_dir(&karantina).unwrap().count(), 1);
    let _ = std::fs::remove_dir_all(&karantina);
}
