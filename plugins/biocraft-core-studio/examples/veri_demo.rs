//! `cargo run -p biocraft-core-studio --example veri_demo`
//!
//! ÇE-01 (Gün 35) veri okumayı uçtan uca gösterir: **varyant (VCF)**, **anotasyon (BED/GFF)**,
//! **GenBank** (dizi + özellik), **2bit** (referans), **yapı (PDB)** ve **uzak bayt-aralığı +
//! KARANTİNA** (MK-33).  Tüm veriler **sentetiktir** (gerçek hasta verisi yok — CLAUDE.md §7).

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use biocraft_core_studio::data_io::{
    dogrula_veya_karantina, genbank_ilk_kayit, AnotasyonOkuyucu, BellekButcesi, TwoBitOkuyucu,
    UzakOkuyucu, VaryantOkuyucu, Yapi, YerelBaytAralik,
};

fn yaz(ad: &str, icerik: &[u8]) -> PathBuf {
    let mut yol = std::env::temp_dir();
    yol.push(format!("biocraft_veridemo_{}_{ad}", std::process::id()));
    File::create(&yol).unwrap().write_all(icerik).unwrap();
    yol
}

fn sil(yol: &Path) {
    let _ = std::fs::remove_file(yol);
}

fn main() {
    println!("=== BioCraft Studio — ÇE-01 Veri Okuma Demosu (Gün 35) ===\n");
    let butce = BellekButcesi::varsayilan();

    // 1) VARYANT (VCF) — bölge sorgusu + INFO/FORMAT.
    println!("[1] Varyant (VCF) — chr1:1-500 bölgesi:");
    let vcf = yaz(
        "a.vcf",
        b"##fileformat=VCFv4.3\n##contig=<ID=chr1,length=1000>\n\
##INFO=<ID=DP,Number=1,Type=Integer,Description=\"d\">\n\
##FORMAT=<ID=GT,Number=1,Type=String,Description=\"g\">\n\
#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1\n\
chr1\t100\trs1\tA\tT\t50\tPASS\tDP=30\tGT\t0/1\n\
chr1\t250\t.\tG\tC,A\t60\tPASS\tDP=20\tGT\t1/2\n\
chr1\t900\trs3\tT\tG\t40\tPASS\tDP=10\tGT\t0/0\n",
    );
    let (mut vo, vb) = VaryantOkuyucu::ac(&vcf).unwrap();
    println!("    örnekler: {:?}, indeksli: {}", vb.ornekler, vb.indeksli);
    for k in vo.bolge_sorgu("chr1:1-500", &butce, 100).unwrap() {
        println!(
            "      · {}:{} {}>{} (QUAL {:?}, INFO {:?}, FORMAT {:?})",
            k.kromozom,
            k.konum,
            k.referans,
            k.alternatifler.join(","),
            k.kalite,
            k.info.iter().map(|(a, _)| a).collect::<Vec<_>>(),
            k.format_anahtarlari,
        );
    }
    sil(&vcf);

    // 2) ANOTASYON (GFF) — gen modeli.
    println!("\n[2] Anotasyon (GFF3) — chr1:1-6000:");
    let gff = yaz(
        "a.gff3",
        b"##gff-version 3\n\
chr1\tHAVANA\tgene\t1000\t5000\t.\t+\t.\tID=geneX;Name=FOO\n\
chr1\tHAVANA\texon\t1000\t1200\t.\t+\t.\tID=ex1;Parent=geneX\n",
    );
    let (ao, _) = AnotasyonOkuyucu::ac(&gff).unwrap();
    for k in ao.bolge_sorgu("chr1:1-6000", &butce, 100).unwrap() {
        println!(
            "      · {} {}:{}-{} {} ({})",
            k.tur,
            k.kromozom,
            k.baslangic,
            k.bitis,
            k.serit,
            k.ad.as_deref().unwrap_or("-"),
        );
    }
    sil(&gff);

    // 3) GENBANK — dizi + özellik (F1).
    println!("\n[3] GenBank — dizi + özellik:");
    // NOT: GenBank özellik satırı 5 boşlukla başlamalı; Rust string `\`-devamı baştaki boşluğu
    // kırptığından her satır `\x20` (boşluk kaçışı) ile sabitlenir (sütun hizası korunur).
    let gb = yaz(
        "a.gb",
        b"LOCUS       SYN001                  12 bp    DNA     linear   SYN 23-JUN-2026\n\
DEFINITION  Mini sentetik.\n\
FEATURES             Location/Qualifiers\n\
\x20    gene            1..12\n\
\x20                    /gene=\"g1\"\n\
ORIGIN\n\
\x20       1 acgtacgtac gt\n\
//\n",
    );
    let kayit = genbank_ilk_kayit(&gb).unwrap();
    println!(
        "      LOCUS {} ({} bp), dizi: {}, özellik sayısı: {}",
        kayit.locus,
        kayit.uzunluk,
        String::from_utf8_lossy(&kayit.dizi),
        kayit.ozellikler.len(),
    );
    sil(&gb);

    // 4) 2bit — referans dizi (out-of-core bölge).
    println!("\n[4] 2bit — referans dizi bölgesi:");
    let mut v: Vec<u8> = Vec::new();
    v.extend_from_slice(&0x1A41_2743u32.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&1u32.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.push(2);
    v.extend_from_slice(b"s1");
    v.extend_from_slice(&23u32.to_le_bytes());
    v.extend_from_slice(&8u32.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.push(0x9C);
    v.push(0x9C);
    let tb = yaz("a.2bit", &v);
    let r2 = TwoBitOkuyucu::ac(&tb).unwrap();
    let parca = r2.bolge("s1:1-8", &butce).unwrap();
    println!("      s1:1-8 = {}", String::from_utf8_lossy(&parca.diziler));
    sil(&tb);

    // 5) YAPI (PDB) — atom/zincir.
    println!("\n[5] Yapı (PDB) — atom/zincir:");
    let pdb = yaz(
        "a.pdb",
        b"ATOM      1  N   MET A   1      11.104  13.207  10.567  1.00 20.00           N\n\
ATOM      2  CA  MET A   1      12.560  13.000  10.420  1.00 20.00           C\n\
HETATM    3  O   HOH B   2       5.000   5.000   5.000  1.00 30.00           O\nEND\n",
    );
    let yapi = Yapi::oku(&pdb, &butce).unwrap();
    println!(
        "      {} atom, {} model, zincirler: {:?}",
        yapi.atom_sayisi(),
        yapi.model_sayisi(),
        yapi.zincirler(),
    );
    sil(&pdb);

    // 6) UZAK + KARANTİNA (MK-33).
    println!("\n[6] Uzak bayt-aralığı + karantina (MK-33):");
    let veri: Vec<u8> = (0..1000u32).map(|i| (i % 256) as u8).collect();
    let uzak = yaz("uzak.bin", &veri);
    let backend = YerelBaytAralik::yeni(&uzak);
    let bolge = backend.bayt_araligi(100, 10).unwrap();
    println!(
        "      uzak dosya {} bayt; yalnız [100,110) çekildi → {:?} (tümü indirilmedi)",
        backend.boyut().unwrap(),
        bolge,
    );
    // Bozuk indirme → karantina.
    let bozuk = yaz("bozuk.dat", b"bozuk icerik");
    let karantina = std::env::temp_dir().join(format!("biocraft_kar_{}", std::process::id()));
    match dogrula_veya_karantina(&bozuk, &"0".repeat(64), &karantina) {
        Ok(()) => println!("      (beklenmiyordu: sağlam)"),
        Err(e) => println!(
            "      bozuk dosya → {} → KARANTİNA (sessiz açma yok)",
            e.ne_oldu
        ),
    }
    sil(&uzak);
    let _ = std::fs::remove_dir_all(&karantina);

    println!("\n=== Tüm formatlar okundu; out-of-core bölge erişimi + karantina çalıştı. ===");
}
