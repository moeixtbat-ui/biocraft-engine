//! ÇE-02 (Gün 36) entegrasyon testi — **genom tarayıcı tuvali (1. kısım)**.
//!
//! Uçtan uca: sentetik SAM (hizalama) + GFF3 (anotasyon) → **out-of-core** görünen pencere
//! yükleme (yalnız bölgeyle örtüşen kayıtlar) → çok-iz derleme (cetvel + kapsama + hizalama +
//! anotasyon) → tooltip/seçim → **cetvel golden** (bp/kb/Mb ölçek doğruluğu).  Test verileri
//! **sentetiktir** (gerçek hasta verisi repoya girmez — CLAUDE.md §7).

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use biocraft_core_studio::data_io::{AnotasyonOkuyucu, BellekButcesi, HizalamaOkuyucu};
use biocraft_core_studio::genome_browser::{
    veri, GenomBolge, GenomTarayici, Iz, IzTuru, IzVeri, Primitif, Tuval,
};
use biocraft_sdk::biocraft_types::golden;

fn gecici(ad: &str) -> PathBuf {
    let mut yol = std::env::temp_dir();
    yol.push(format!("biocraft_gb36_{}_{ad}", std::process::id()));
    yol
}

fn yaz(yol: &Path, icerik: &[u8]) {
    File::create(yol).unwrap().write_all(icerik).unwrap();
}

/// Bir SAM hizalama satırı (50M, SEQ uzunluğu 50).
fn sam_satir(ad: &str, bayrak: u16, pos: u64) -> String {
    let seq = "A".repeat(50);
    let qual = "I".repeat(50);
    format!("{ad}\t{bayrak}\tchr1\t{pos}\t60\t50M\t*\t0\t0\t{seq}\t{qual}\n")
}

fn sam_dosyasi() -> PathBuf {
    let mut s = String::from("@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:5000\n");
    s.push_str(&sam_satir("read1", 0, 100)); // ileri 100-149
    s.push_str(&sam_satir("read2", 16, 150)); // ters 150-199 (read1 ile çakışır)
    s.push_str(&sam_satir("read3", 0, 2000)); // ileri 2000-2049 (görünen pencere dışı)
    let p = gecici("a.sam");
    yaz(&p, s.as_bytes());
    p
}

fn gff_dosyasi() -> PathBuf {
    let icerik = b"##gff-version 3\n\
chr1\ttest\tgene\t100\t1500\t.\t+\t.\tID=GENE1;Name=BRCA\n\
chr1\ttest\texon\t100\t300\t.\t+\t.\tID=exon1;Parent=GENE1\n\
chr1\ttest\texon\t1200\t1500\t.\t+\t.\tID=exon2;Parent=GENE1\n\
chr2\ttest\tgene\t50\t90\t.\t-\t.\tID=GENE2;Name=FOO\n";
    let p = gecici("a.gff3");
    yaz(&p, icerik);
    p
}

// ─── Out-of-core görünen pencere yükleme ────────────────────────────────────────

#[test]
fn gorunen_pencere_out_of_core() {
    let sam = sam_dosyasi();
    let (mut hiz, basligi) = HizalamaOkuyucu::ac(&sam, None).unwrap();
    // Başlık kromozom uzunluğunu taşır.
    assert_eq!(basligi.referans_diziler, vec![("chr1".to_string(), 5000)]);

    let butce = BellekButcesi::sinirsiz();

    // chr1:1-1000 → yalnız read1 + read2 (read3 pencere dışı, YÜKLENMEZ).
    let pencere1 = GenomBolge::yeni("chr1", 1, 1000).unwrap();
    let okumalar = veri::gorunur_okumalar(&mut hiz, &pencere1, &butce, 10_000).unwrap();
    assert_eq!(okumalar.len(), 2, "yalnız görünen pencere yüklenir (MK-09)");
    assert!(okumalar.iter().all(|o| o.bas < 1000));

    // Farklı pencere chr1:1900-2100 → yalnız read3.
    let pencere2 = GenomBolge::yeni("chr1", 1900, 2100).unwrap();
    let okumalar2 = veri::gorunur_okumalar(&mut hiz, &pencere2, &butce, 10_000).unwrap();
    assert_eq!(okumalar2.len(), 1);
    assert_eq!(okumalar2[0].ad, "read3");

    let _ = std::fs::remove_file(&sam);
}

// ─── Uçtan uca derleme: cetvel + 3 iz + tooltip + seçim ─────────────────────────

#[test]
fn uctan_uca_derleme_ve_etkilesim() {
    let sam = sam_dosyasi();
    let gff = gff_dosyasi();

    let (mut hiz, basligi) = HizalamaOkuyucu::ac(&sam, None).unwrap();
    let (annot, _) = AnotasyonOkuyucu::ac(&gff).unwrap();
    let butce = BellekButcesi::sinirsiz();

    // Tarayıcı: chr1:1-1000, 1000 px.
    let mut tarayici = GenomTarayici::yeni(1000.0, GenomBolge::yeni("chr1", 1, 1000).unwrap());
    tarayici.kromozom_uzunluklari_ayarla(
        basligi
            .referans_diziler
            .iter()
            .map(|(ad, uzun)| (ad.clone(), *uzun as u64)),
    );
    tarayici.iz_ekle(Iz::yeni("kapsama", "Kapsama", IzTuru::Kapsama));
    tarayici.iz_ekle(Iz::yeni("reads", "Okumalar", IzTuru::Hizalama));
    tarayici.iz_ekle(Iz::yeni("genler", "Genler", IzTuru::Anotasyon));

    // Görünen pencereyi yükle (out-of-core).
    let bolge = tarayici.bolge().clone();
    let okumalar = veri::gorunur_okumalar(&mut hiz, &bolge, &butce, 10_000).unwrap();
    let ozellikler = veri::gorunur_ozellikler(&annot, &bolge, &butce, 10_000).unwrap();
    assert_eq!(okumalar.len(), 2);
    // gene(100-1500) + exon1(100-300) örtüşür; exon2(1200-1500) chr1:1-1000 dışı.
    assert_eq!(ozellikler.len(), 2);

    // Gen adıyla gezinme için çözücüyü anotasyondan doldur.
    for o in &ozellikler {
        if o.tur == "gene" {
            if let Some(ad) = &o.ad {
                tarayici
                    .gen_cozucu_mut()
                    .ekle(ad.clone(), GenomBolge::yeni("chr1", o.bas, o.bit).unwrap());
            }
        }
    }

    // Çok-iz veri haritası.
    let mut harita: BTreeMap<String, IzVeri> = BTreeMap::new();
    harita.insert("kapsama".into(), IzVeri::Kapsama(okumalar.clone()));
    harita.insert("reads".into(), IzVeri::Hizalama(okumalar.clone()));
    harita.insert("genler".into(), IzVeri::Anotasyon(ozellikler.clone()));

    let liste = tarayici.derle(&harita);

    // Cetvel + kapsama çubuğu + read kutusu + ekson kutusu + gen çizgisi hepsi var.
    assert!(liste
        .primitifler
        .iter()
        .any(|p| matches!(p, Primitif::Dikdortgen { .. })));
    // Hizalama izinde 2 read + anotasyon izinde 2 özellik → en az 4 isabet (tooltip noktası).
    assert!(liste.isabetler.len() >= 4, "read + özellik isabetleri");

    // Tooltip: read1 kutusunun üstüne gel.
    let yer = tarayici
        .yerlesim()
        .into_iter()
        .find(|y| y.kimlik == "reads")
        .unwrap();
    let (sol, _) = tarayici.tuval().aralik_ekran(100, 149);
    let ipucu = tarayici.tooltip(&liste, sol + 1.0, yer.y_ust + 1.0);
    assert!(ipucu.is_some());
    assert!(ipucu.unwrap().contains("read"));

    // Seçim → inspector detayı.
    assert!(tarayici.sec(&liste, sol + 1.0, yer.y_ust + 1.0));
    let detay = &tarayici.secili().unwrap().detay;
    assert!(detay.contains("Okuma:"));
    assert!(detay.contains("Şerit"));

    // Gen adıyla "bölgeye git" (anotasyon okuyucu ID'yi ad alır → GENE1).
    assert!(tarayici.bolgeye_git("GENE1").is_ok());
    assert_eq!(tarayici.bolge().baslangic, 100);

    let _ = std::fs::remove_file(&sam);
    let _ = std::fs::remove_file(&gff);
}

// ─── Cetvel golden: bp / kb / Mb ölçek doğruluğu ────────────────────────────────

/// Bir tuvalin cetvelini (yalnız büyük/etiketli işaretler) deterministik metne dök.
fn cetvel_metni(baslik: &str, t: &Tuval) -> String {
    let c = biocraft_core_studio::genome_browser::ruler::cetvel(t, 10);
    let mut s = format!("=== {baslik} ===\nÖlçek: {}\n", c.olcek.birim());
    for m in c.isaretler.iter().filter(|m| m.buyuk) {
        s.push_str(&format!("{} = {}\n", m.pos, m.etiket));
    }
    s
}

#[test]
fn cetvel_golden_bp_kb_mb() {
    let bp = Tuval::yeni(1000.0, GenomBolge::yeni("chr1", 1, 500).unwrap());
    let kb = Tuval::yeni(1000.0, GenomBolge::yeni("chr1", 1, 100_000).unwrap());
    let mb = Tuval::yeni(1000.0, GenomBolge::yeni("chr1", 1, 5_000_000).unwrap());

    let metin = format!(
        "{}\n{}\n{}",
        cetvel_metni("chr1:1-500 (bp)", &bp),
        cetvel_metni("chr1:1-100000 (kb)", &kb),
        cetvel_metni("chr1:1-5000000 (Mb)", &mb),
    );
    golden::dogrula("ce02_cetvel_bp_kb_mb", &metin);
}

// ─── Pan/zoom/geri-ileri akıcılığı (durum doğruluğu) ────────────────────────────

#[test]
fn gezinme_gecmisi_uctan_uca() {
    let mut t = GenomTarayici::yeni(1000.0, GenomBolge::yeni("chr1", 1, 1000).unwrap());
    t.kromozom_uzunluklari_ayarla([("chr1".to_string(), 1_000_000u64)]);

    t.bolgeye_git("chr1:5000-6000").unwrap();
    t.bolgeye_git("chr1:50000-60000").unwrap();
    assert!(t.geri_var_mi() && !t.ileri_var_mi());

    // Pan + zoom (ayrık jest → geçmişe kaydet).
    t.pan_bp(1000);
    t.gecmise_kaydet();
    t.yakinlastir_merkez(0.5);
    assert!(t.bolge().uzunluk() < 10_000);

    // Geri zinciri tutarlı.
    let onceki = t.bolge().clone();
    assert!(t.geri());
    assert_ne!(t.bolge(), &onceki);
    assert!(t.ileri());
    assert_eq!(t.bolge(), &onceki);
}
