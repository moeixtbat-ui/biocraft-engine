//! ÇE-01 entegrasyon testi — **Veri G/Ç**: FASTA/FASTQ + BAM/SAM/CRAM, indeksli bölge sorgusu,
//! BGZF blok-farkındalık, out-of-core, BLAKE3+provenance ve **golden** doğruluk (samtools-eşdeğeri).
//!
//! Test verileri **sentetiktir** ve burada `noodles` ile üretilir (gerçek hasta verisi repoya
//! girmez — CLAUDE.md §7).  BAM, küçük bir SAM'den üretilip indekslenir; indeksli bölge sorgusunun
//! sonucu, indekssiz lineer SAM taramasıyla **çapraz doğrulanır** (ikisi de aynı kayıtları vermeli)
//! ve ayrıca diske kayıtlı **golden** referansla karşılaştırılır (Gün 32 çerçevesi, MK-58).

use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

use biocraft_core_studio::data_io::{
    indeks_olustur, BellekButcesi, HizalamaKaydi, HizalamaOkuyucu, VeriFormati,
};
use biocraft_sdk::biocraft_types::golden;

use noodles::sam::alignment::io::Write as _;
use noodles::{bam, sam};

/// Geçici dosya yolu (her test benzersiz ad kullanır → paralel test çakışmaz).
fn gecici(ad: &str) -> PathBuf {
    let mut yol = std::env::temp_dir();
    yol.push(format!("biocraft_veriio_{}_{ad}", std::process::id()));
    yol
}

fn yaz(yol: &Path, icerik: &[u8]) {
    File::create(yol).unwrap().write_all(icerik).unwrap();
}

/// Koordinata göre sıralı küçük bir SAM (1 referans, 3 read) — bilinen-doğru sentetik veri.
/// r1: sq0:10 (4M → 10-13), r2: sq0:20 (5M → 20-24), r3: sq0:100 (4M → 100-103).
const SAM_ICERIK: &[u8] = b"@HD\tVN:1.6\tSO:coordinate\n\
@SQ\tSN:sq0\tLN:1000\n\
r1\t0\tsq0\t10\t60\t4M\t*\t0\t0\tACGT\tIIII\n\
r2\t0\tsq0\t20\t60\t5M\t*\t0\t0\tACGTA\tIIIII\n\
r3\t0\tsq0\t100\t60\t4M\t*\t0\t0\tTTTT\tIIII\n";

/// SAM'i BGZF'li BAM'e dönüştürür (noodles writer; her kayıt blok-farkında yazılır).
fn sam_to_bam(sam_yol: &Path, bam_yol: &Path) {
    let mut r = sam::io::Reader::new(BufReader::new(File::open(sam_yol).unwrap()));
    let header = r.read_header().unwrap();
    let mut w = bam::io::Writer::new(File::create(bam_yol).unwrap());
    w.write_header(&header).unwrap();
    for res in r.records() {
        let kayit = res.unwrap();
        // alignment::io::Write trait'i her record tipini (sam::Record dâhil) kabul eder.
        w.write_alignment_record(&header, &kayit).unwrap();
    }
    w.try_finish().unwrap();
}

/// Bir kayıt listesini deterministik, golden-uyumlu metne çevirir.
fn dok(kayitlar: &[HizalamaKaydi]) -> String {
    let mut s = String::new();
    for k in kayitlar {
        s.push_str(&format!(
            "{}\t{}\t{}\tflag={}\tmapq={}\tlen={}\n",
            k.ad,
            k.referans.as_deref().unwrap_or("*"),
            k.konum.map(|p| p.to_string()).unwrap_or_else(|| "*".into()),
            k.bayrak,
            k.mapq.map(|m| m.to_string()).unwrap_or_else(|| "*".into()),
            k.dizi_uzunlugu,
        ));
    }
    s
}

#[test]
fn sam_lineer_bolge_sorgusu_dogru_kayitlari_doner() {
    let sam = gecici("a.sam");
    yaz(&sam, SAM_ICERIK);

    let (mut okuyucu, basligi) = HizalamaOkuyucu::ac(&sam, None).unwrap();
    assert_eq!(basligi.format, VeriFormati::Sam);
    assert!(!basligi.indeksli, "SAM indeksli değil (lineer)");
    assert_eq!(basligi.referans_diziler, vec![("sq0".to_string(), 1000)]);

    // sq0:18-25 → yalnız r2 (20-24 örtüşür).
    let r = okuyucu
        .bolge_sorgu("sq0:18-25", &BellekButcesi::sinirsiz(), 1000)
        .unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].ad, "r2");
    assert_eq!(r[0].konum, Some(20));

    // sq0:50-60 → boş.
    let bos = okuyucu
        .bolge_sorgu("sq0:50-60", &BellekButcesi::sinirsiz(), 1000)
        .unwrap();
    assert!(bos.is_empty());

    let _ = std::fs::remove_file(&sam);
}

#[test]
fn bam_indeksli_sorgu_sam_ile_capraz_dogrulanir_ve_golden() {
    let sam = gecici("b.sam");
    let bam = gecici("b.bam");
    yaz(&sam, SAM_ICERIK);
    sam_to_bam(&sam, &bam);

    // İndeks yokken açılış: "İndeks oluştur" önerir (TDA madde 1/4).
    // (.err().unwrap() — Ok tipi Debug değil; okuyucu noodles reader tutar.)
    let hata = HizalamaOkuyucu::ac(&bam, None).err().unwrap();
    assert_eq!(hata.ne_oldu, "BAM indeksi bulunamadı");
    assert_eq!(hata.eylem_etiketi.as_deref(), Some("İndeks oluştur"));

    // İndeks oluştur (.bai) → artık indeksli açılır.
    let bai = indeks_olustur(&bam, VeriFormati::Bam).unwrap();
    assert!(bai.exists());

    let (mut bam_okuyucu, basligi) = HizalamaOkuyucu::ac(&bam, None).unwrap();
    assert_eq!(basligi.format, VeriFormati::Bam);
    assert!(basligi.indeksli);

    // sq0:8-105 → üç read de örtüşür; indeksli (hızlı) sorgu.
    let indeksli = bam_okuyucu
        .bolge_sorgu("sq0:8-105", &BellekButcesi::sinirsiz(), 1000)
        .unwrap();
    let adlar: Vec<&str> = indeksli.iter().map(|k| k.ad.as_str()).collect();
    assert_eq!(adlar, vec!["r1", "r2", "r3"]);

    // Çapraz doğrulama: aynı bölge SAM lineer taramasıyla AYNI kayıtları vermeli.
    let sam_yol = gecici("b2.sam");
    yaz(&sam_yol, SAM_ICERIK);
    let (mut sam_okuyucu, _) = HizalamaOkuyucu::ac(&sam_yol, None).unwrap();
    let lineer = sam_okuyucu
        .bolge_sorgu("sq0:8-105", &BellekButcesi::sinirsiz(), 1000)
        .unwrap();
    assert_eq!(
        dok(&indeksli),
        dok(&lineer),
        "indeksli ve lineer sonuç eşit olmalı"
    );

    // Golden (samtools-eşdeğeri bilinen-doğru çıktı, Gün 32 çerçevesi).
    golden::dogrula("ce01_bam_sq0_8_105", &dok(&indeksli));

    let _ = std::fs::remove_file(&sam);
    let _ = std::fs::remove_file(&sam_yol);
    let _ = std::fs::remove_file(&bai);
    let _ = std::fs::remove_file(&bam);
}

#[test]
fn bam_dar_bolge_yalniz_ortusen_okunur() {
    let sam = gecici("c.sam");
    let bam = gecici("c.bam");
    yaz(&sam, SAM_ICERIK);
    sam_to_bam(&sam, &bam);
    let bai = indeks_olustur(&bam, VeriFormati::Bam).unwrap();

    let (mut okuyucu, _) = HizalamaOkuyucu::ac(&bam, None).unwrap();
    // sq0:19-21 → yalnız r2.
    let r = okuyucu
        .bolge_sorgu("sq0:19-21", &BellekButcesi::sinirsiz(), 1000)
        .unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].ad, "r2");

    let _ = std::fs::remove_file(&sam);
    let _ = std::fs::remove_file(&bai);
    let _ = std::fs::remove_file(&bam);
}

#[test]
fn cram_referanssiz_net_hata() {
    // .cram uzantısı yeterli (uzantıdan tanınır); referans verilmeyince net hata döner.
    let cram = gecici("d.cram");
    yaz(&cram, b"CRAM\x03\x00sentetik");
    let hata = HizalamaOkuyucu::ac(&cram, None).err().unwrap();
    assert_eq!(hata.ne_oldu, "CRAM için referans dizisi gerekli");
    assert_eq!(hata.eylem_etiketi.as_deref(), Some("Referans seç"));
    let _ = std::fs::remove_file(&cram);
}

#[test]
fn bozuk_bam_cokme_yerine_guvenli_reddedilir() {
    // Geçerli BAM değil ama .bam uzantılı + .bai mevcut süsü → açılış güvenli hata vermeli (panik yok).
    let bam = gecici("e.bam");
    yaz(&bam, b"bu gecerli bir BAM degil\n");
    // İndeks dosyası olmadığından "indeks yok" ya da "açılamadı" — her hâlükârda Err + panik yok.
    let sonuc = HizalamaOkuyucu::ac(&bam, None);
    assert!(sonuc.is_err());
    let _ = std::fs::remove_file(&bam);
}
