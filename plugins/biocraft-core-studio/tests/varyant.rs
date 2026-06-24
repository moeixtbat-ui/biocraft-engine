//! ÇE-04 (Gün 38) entegrasyon testi — **Varyant inceleme**: gerçek dosyadan (out-of-core) VCF
//! tablo + filtre/sorgu + sıralama + genom tarayıcı bağlantısı (koordinat doğruluğu) + filtreli
//! dışa aktarma + **bcftools-eşdeğeri golden** (MK-58, Gün 32/43 çerçevesi).
//!
//! Test verileri **sentetiktir** (gerçek hasta verisi repoya girmez — CLAUDE.md §7).

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use biocraft_core_studio::genome_browser::GenomBolge;
use biocraft_core_studio::variant::disa_aktar::bcftools_query;
use biocraft_core_studio::variant::{
    ayristir, DosyaKaynak, Filtre, SafRustMotor, SiralamaAnahtari, VaryantGorunumu, VaryantKaynak,
    Zigosite,
};
use biocraft_sdk::biocraft_types::{golden, Capability};
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

fn gecici(ad: &str) -> PathBuf {
    let mut yol = std::env::temp_dir();
    yol.push(format!("biocraft_varyant_{}_{ad}", std::process::id()));
    yol
}

fn yaz(yol: &Path, icerik: &[u8]) {
    File::create(yol).unwrap().write_all(icerik).unwrap();
}

fn yetki() -> YetkiKapisi {
    YetkiKapisi::yeni([Capability::Fs])
}

/// Gerçek VCF dosyasından bir saf-Rust motor + görünüm kurar (örnek adları başlıktan).
fn gorunum_ac(yol: &Path) -> (VaryantGorunumu, SafRustMotor<DosyaKaynak>) {
    let kaynak = DosyaKaynak::ac(yol, &yetki()).unwrap();
    let ornekler = kaynak.basligi().ornekler.clone();
    let motor = SafRustMotor::yeni(kaynak);
    (VaryantGorunumu::yeni(ornekler), motor)
}

#[test]
fn dosya_tablo_filtre_ve_siralama() {
    let p = gecici("a.vcf");
    yaz(&p, VCF);
    let (mut g, mut m) = gorunum_ac(&p);

    // Filtresiz: 4 varyant, konum sıralı (chr1:100,250,400, chr2:500).
    g.yenile(&mut m).unwrap();
    assert_eq!(g.toplam(), 4);
    let ilk = g.satir(0).unwrap();
    assert_eq!(ilk.kromozom(), "chr1");
    assert_eq!(ilk.konum(), 100);

    // QUAL azalan sıralama → 70,60,50,35.
    g.sirala(SiralamaAnahtari::Kalite, false, &mut m).unwrap();
    assert_eq!(g.satir(0).unwrap().kalite(), Some(70.0));
    assert_eq!(g.satir(3).unwrap().kalite(), Some(35.0));

    let _ = std::fs::remove_file(&p);
}

#[test]
fn out_of_core_bolge_pushdown_yalniz_o_bolge() {
    let p = gecici("b.vcf");
    yaz(&p, VCF);
    let mut kaynak = DosyaKaynak::ac(&p, &yetki()).unwrap();

    // chr1:1-300 → yalnız chr1:100 ve chr1:250 OKUNUR (chr1:400 ve chr2:500 dosyadan getirilmez).
    let bolge = GenomBolge::yeni("chr1", 1, 300).unwrap();
    let satirlar = kaynak
        .tara(
            Some(&bolge),
            &biocraft_core_studio::data_io::BellekButcesi::sinirsiz(),
            1000,
        )
        .unwrap();
    assert_eq!(satirlar.len(), 2);
    assert_eq!(satirlar[0].konum(), 100);
    assert_eq!(satirlar[1].konum(), 250);

    let _ = std::fs::remove_file(&p);
}

#[test]
fn varyanta_tikla_genom_tarayici_dogru_pozisyon() {
    // Koordinat tabanı (1-tabanlı) tutarlılığı: VCF POS = tarayıcı GenomBolge merkezi → kayma yok.
    let p = gecici("c.vcf");
    yaz(&p, VCF);
    let (mut g, mut m) = gorunum_ac(&p);
    g.yenile(&mut m).unwrap();

    // chr1:250 varyantını seç (konum sıralı → indeks 1).
    g.sec(1);
    let secili = g.secili_satir().unwrap();
    assert_eq!(secili.konum(), 250);

    let hedef = g.tarayiciya_git().unwrap();
    assert_eq!(hedef.kromozom, "chr1");
    // Hedef bölge varyantın TAM konumunu kapsamalı (off-by-one yok).
    assert!(hedef.kapsar(250));
    assert_eq!(hedef.merkez(), 250);

    let _ = std::fs::remove_file(&p);
}

#[test]
fn genotip_izgara_gercek_dosyadan() {
    let p = gecici("d.vcf");
    yaz(&p, VCF);
    let (mut g, mut m) = gorunum_ac(&p);
    g.yenile(&mut m).unwrap();

    assert_eq!(g.ornekler(), &["S1".to_string(), "S2".to_string()]);
    let izgara = g.genotip_izgara().unwrap();
    // chr1:100 → S1=0/1 (het), S2=1/1 (hom-alt).
    assert_eq!(izgara.hucre(0, 0).unwrap().zigosite, Zigosite::Het);
    assert_eq!(izgara.hucre(0, 1).unwrap().zigosite, Zigosite::HomAlt);
    // chr1:250 → S2=0/0 (hom-ref).
    assert_eq!(izgara.hucre(1, 1).unwrap().zigosite, Zigosite::HomRef);

    let _ = std::fs::remove_file(&p);
}

#[test]
fn filtreli_disa_aktarma_csv_ve_vcf() {
    let p = gecici("e.vcf");
    yaz(&p, VCF);
    let (mut g, mut m) = gorunum_ac(&p);

    // QUAL>=50 ve PASS → chr1:100, chr1:250, chr2:500 (chr1:400 q10/35 hariç).
    g.ham_sorgu_uygula("QUAL >= 50 AND FILTER = PASS", &mut m)
        .unwrap();
    assert_eq!(g.toplam(), 3);

    let csv = g.csv_disa_aktar().unwrap();
    assert_eq!(csv.lines().count(), 4); // başlık + 3
    let vcf = g.vcf_disa_aktar().unwrap();
    assert_eq!(vcf.lines().filter(|l| !l.starts_with('#')).count(), 3);
    assert!(vcf.contains("chr2\t500\trs3\tT\tTGG\t70\tPASS"));

    let _ = std::fs::remove_file(&p);
}

#[test]
fn golden_bcftools_query_esdegeri() {
    // bcftools view -i 'QUAL>=50 && FILTER="PASS"' | query -f '%CHROM\t%POS\t%REF\t%ALT\t%QUAL\t%FILTER\n'
    let p = gecici("f.vcf");
    yaz(&p, VCF);
    let (mut g, mut m) = gorunum_ac(&p);
    g.ham_sorgu_uygula("QUAL >= 50 AND FILTER = PASS", &mut m)
        .unwrap();

    let tsv = bcftools_query(&g.sonuc().unwrap().satirlar);
    golden::dogrula("ce04_qual50_pass", &tsv);

    let _ = std::fs::remove_file(&p);
}

#[test]
fn dosya_kaynak_fs_yetkisi_ister() {
    let p = gecici("g.vcf");
    yaz(&p, VCF);
    // fs yetkisi yoksa açılmaz (MK-13).  (DosyaKaynak Debug değil → match ile çöz.)
    let hata = match DosyaKaynak::ac(&p, &YetkiKapisi::bos()) {
        Ok(_) => panic!("fs yetkisi yokken dosya açılmamalıydı"),
        Err(e) => e,
    };
    assert_eq!(hata.ne_oldu, "Eklenti erişimi reddedildi");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn ham_sorgu_hatasi_kullaniciya_donen_sema() {
    let p = gecici("h.vcf");
    yaz(&p, VCF);
    let (mut g, mut m) = gorunum_ac(&p);
    // Geçersiz alan → standart hata şeması (panik yok).
    let hata = g.ham_sorgu_uygula("FOO = 1", &mut m).unwrap_err();
    assert_eq!(hata.ne_oldu, "Sorgu ayrıştırılamadı");
    let _ = std::fs::remove_file(&p);

    // ayristir doğrudan da aynı hatayı verir.
    assert!(ayristir("POS >= 5").is_err()); // CHROM yok
    let _ = Filtre::default();
}
