//! ÇE-12 (Gün 43) entegrasyon testi — **çekirdek eklenti cilası**: performans (büyük veri akıcılığı +
//! bellek bütçesi), erişilebilirlik (klavye/ekran okuyucu/renk körü palet), doğruluk (bcftools/
//! samtools-eşdeğeri golden), edge-case (boş/tek/büyük/bozuk + Unicode) ve standart hata şeması +
//! correlation_id (MK-58, MK-52, Bölüm 0.12, İP-21).
//!
//! Tüm veriler **sentetiktir** (gerçek hasta verisi repoya girmez — CLAUDE.md §7).  Testler
//! **render-bağımsızdır** (egui/wgpu/GPU gerekmez; saf-mantık çekirdeği — MK-17).

use std::collections::BTreeMap;

use biocraft_core_studio::data_io::VaryantKaydi;
use biocraft_core_studio::genome_browser::CizimRengi;
use biocraft_core_studio::genome_browser::{
    GenomBolge, GenomTarayici, Iz, IzTuru, IzVeri, OkumaParcasi, Serit,
};
use biocraft_core_studio::perf::accessibility::{
    self, ErisilebilirlikAyari, RenkGormeTuru, AYIRT_ESIGI,
};
use biocraft_core_studio::perf::budget::{detay_sec, detay_uyarisi, Detay, KareButcesi};
use biocraft_core_studio::perf::correctness::{DogrulukRaporu, KoordinatTabani, ReferansArac};
use biocraft_core_studio::perf::edge::{self, VeriDurumu};
use biocraft_core_studio::perf::keyboard::{erisilebilirlik_denetimi, OdakHalkasi, OdakOgesi, Rol};
use biocraft_core_studio::variant::{
    ayristir, BellekKaynak, SafRustMotor, Sorgu, VaryantSatiri, VaryantSorguMotoru,
};
use biocraft_sdk::biocraft_types::ErrorReport;
use biocraft_sdk::biocraft_types::{golden, Capability};
use biocraft_sdk::YetkiKapisi;

// ─── Yardımcılar (sentetik veri üreteçleri) ───────────────────────────────────

/// Tek-nükleotit varyant kaydı kurar (deterministik; bcftools eşdeğeri sayım için).
fn snv(konum: usize, kalite: f32, pass: bool, alt: &str) -> VaryantSatiri {
    VaryantSatiri::yeni(VaryantKaydi {
        kromozom: "chr1".into(),
        konum,
        kimlik: ".".into(),
        referans: "A".into(),
        alternatifler: vec![alt.into()],
        kalite: Some(kalite),
        filtreler: vec![if pass { "PASS".into() } else { "q10".into() }],
        info: vec![],
        ornek_sayisi: 0,
        format_anahtarlari: vec![],
        genotipler: vec![],
    })
}

/// `n` deterministik SNV üretir (kalite 0..99 döngüsel, her 3'te bir PASS).
fn varyant_kumesi(n: usize) -> Vec<VaryantSatiri> {
    (0..n)
        .map(|i| {
            let kalite = (i % 100) as f32;
            let pass = i % 3 == 0;
            let alt = ["G", "C", "T"][i % 3]; // A>G geçiş; A>C / A>T geçişsizlik
            snv(100 + i * 10, kalite, pass, alt)
        })
        .collect()
}

/// `n` okuma (read) — bölgeye yayılmış, yarısı ileri/yarısı geri şerit.
fn okuma_kumesi(n: usize, bolge_uzunluk: u64) -> Vec<OkumaParcasi> {
    (0..n)
        .map(|i| {
            let bas = 1 + (i as u64 * bolge_uzunluk / n as u64);
            OkumaParcasi {
                ad: format!("r{i}"),
                bas,
                bit: bas + 99, // 100 bp okuma
                serit: if i % 2 == 0 {
                    Serit::Ileri
                } else {
                    Serit::Geri
                },
                mapq: Some(60),
            }
        })
        .collect()
}

// ─── 1) Performans: büyük VCF akıcı + bellek bütçesi ──────────────────────────

#[test]
fn buyuk_vcf_sorgu_akici_ve_butce_uyumlu() {
    let n = 50_000;
    let satirlar = varyant_kumesi(n);

    // Beklenen (bcftools `view -i 'QUAL>=50 && FILTER=PASS'` semantiği): aynı 1-tabanlı koordinat +
    // aynı filtre → BioCraft sayımı literal yüklemle birebir olmalı.
    let beklenen = (0..n).filter(|i| (i % 100) >= 50 && i % 3 == 0).count();

    let mut motor = SafRustMotor::yeni(BellekKaynak::yeni(vec![], satirlar));
    let filtre = ayristir("QUAL >= 50 AND FILTER = PASS").unwrap();
    let sonuc = motor.calistir(&Sorgu::yeni(filtre)).unwrap();
    assert_eq!(
        sonuc.toplam_eslesme, beklenen,
        "filtre sayımı bcftools-eşdeğeri olmalı"
    );
    assert!(!sonuc.kesildi, "varsayılan üst sınırda 50k tam taranmalı");

    // Bellek bütçesi: ham kümeyi 50k öğe için detay seyrelir (TDA 11) — akıcılık için.
    let detay = detay_sec(
        n,
        KareButcesi::fps60(),
        16.666, /* µs/öğe → ~1000 sığar */
    );
    assert_ne!(detay, Detay::Tam, "50k öğe tam çizilemez; seyreltilmeli");
    assert!(
        detay_uyarisi(n, detay).is_some(),
        "sadeleştirme şeffaf bildirilmeli"
    );
}

#[test]
fn dar_bellek_butcesi_guvenli_red_standart_sema() {
    // Düşük bellek (out-of-core dosya yolu): çok küçük bütçe → tarama OOM yerine NET hata (panik
    // yok); standart şema + correlation_id.  Gerçek bütçe enforcement DosyaKaynak'ta (akışlı okuma).
    use biocraft_core_studio::data_io::BellekButcesi;
    use biocraft_core_studio::variant::DosyaKaynak;
    use std::io::Write;

    let mut vcf = String::from(
        "##fileformat=VCFv4.3\n##contig=<ID=chr1,length=100000>\n\
         #CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n",
    );
    for i in 0..200 {
        vcf.push_str(&format!("chr1\t{}\t.\tA\tG\t60\tPASS\t.\n", 100 + i * 10));
    }
    let yol =
        std::env::temp_dir().join(format!("biocraft_cilala_butce_{}.vcf", std::process::id()));
    std::fs::File::create(&yol)
        .unwrap()
        .write_all(vcf.as_bytes())
        .unwrap();

    let kaynak = DosyaKaynak::ac(&yol, &YetkiKapisi::yeni([Capability::Fs])).unwrap();
    let mut motor = SafRustMotor::yeni(kaynak).butce_ile(BellekButcesi::yeni(64)); // 64 bayt: yetersiz
    let rapor: ErrorReport = motor
        .calistir(&Sorgu::yeni(ayristir("QUAL >= 0").unwrap()))
        .unwrap_err();
    // Gün 4 şeması: üç alan dolu + correlation_id (8 hane kısa kimlik).
    assert!(
        !rapor.ne_oldu.is_empty() && !rapor.neden.is_empty() && !rapor.nasil_cozulur.is_empty()
    );
    assert_eq!(rapor.correlation_id.kisa().len(), 8);
    let _ = std::fs::remove_file(&yol);
}

// ─── 1b) Performans: büyük BAM (genom tarayıcı LOD) akıcı ─────────────────────

#[test]
fn buyuk_bam_tarayici_lod_ile_sinirli_cizim() {
    // 20.000 okuma, 1 Mb bölge, 1000 px → LOD/seyreltme devreye girer: çizilen ilkellerin sayısı
    // okuma sayısıyla DEĞİL, ekran genişliğiyle ölçeklenir (akıcılık garantisi).
    let okumalar = okuma_kumesi(20_000, 1_000_000);
    let mut tarayici = GenomTarayici::yeni(1000.0, GenomBolge::yeni("chr1", 1, 1_000_000).unwrap());
    tarayici.iz_ekle(Iz::yeni("kapsama", "Kapsama", IzTuru::Kapsama));
    tarayici.iz_ekle(Iz::yeni("reads", "Okumalar", IzTuru::Hizalama));

    let mut harita: BTreeMap<String, IzVeri> = BTreeMap::new();
    harita.insert("kapsama".into(), IzVeri::Kapsama(okumalar.clone()));
    harita.insert("reads".into(), IzVeri::Hizalama(okumalar));
    let liste = tarayici.derle(&harita);

    // İlkel sayısı sınırlı kalmalı (20k×... değil) → büyük dosyada da kare bütçesi korunur.
    assert!(
        liste.primitifler.len() < 12_000,
        "LOD devrede değil: {} ilkel (sınırsız çizim akıcılığı bozar)",
        liste.primitifler.len()
    );
}

// ─── 2) Erişilebilirlik: klavye + ekran okuyucu + renk körü palet ─────────────

#[test]
fn genom_tarayici_tam_klavye_ve_ekran_okuyucu() {
    // Gerçekçi tarayıcı yüzeyinin odak halkası: hepsi etiketli + klavyeyle ulaşılır olmalı.
    let halka = OdakHalkasi::ogelerden(vec![
        OdakOgesi::yeni("ara", "Bölgeye git", Rol::GirisAlani).kisayol("Ctrl+G"),
        OdakOgesi::yeni("geri", "Geri", Rol::Buton).kisayol("Alt+Sol"),
        OdakOgesi::yeni("ileri", "İleri", Rol::Buton).kisayol("Alt+Sağ"),
        OdakOgesi::yeni("izler", "İz listesi", Rol::Liste),
        OdakOgesi::yeni("tuval", "Genom tuvali", Rol::Tuval).kisayol("Ok tuşları"),
        OdakOgesi::yeni("baslik", "chr1", Rol::Etiket), // etkileşimsiz
    ]);
    let denetim = erisilebilirlik_denetimi(halka.ogeler());
    assert!(denetim.temiz(), "tarayıcı erişilemez: {denetim:?}");

    // Tab ile tüm etkileşimli öğeler bir tur içinde ziyaret edilir (klavyeyle her yere ulaşılır).
    let mut h = halka;
    let mut gorulen = std::collections::HashSet::new();
    gorulen.insert(h.odakli().unwrap().kimlik.clone());
    for _ in 0..10 {
        gorulen.insert(h.sonraki().unwrap().kimlik.clone());
    }
    for k in ["ara", "geri", "ileri", "izler", "tuval"] {
        assert!(gorulen.contains(k), "{k} klavyeyle erişilemiyor");
    }
    assert!(
        !gorulen.contains("baslik"),
        "etkileşimsiz etiket odak almamalı"
    );

    // Ekran okuyucu anlatımı etiket + rol + kısayol içerir.
    let mut h2 =
        OdakHalkasi::ogelerden(vec![
            OdakOgesi::yeni("ara", "Bölgeye git", Rol::GirisAlani).kisayol("Ctrl+G")
        ]);
    assert_eq!(
        h2.odakli().unwrap().ekran_okuyucu_metni(),
        "Bölgeye git, giriş alanı, kısayol Ctrl+G"
    );
    let _ = h2.sonraki();
}

#[test]
fn renk_koru_palet_deuteranopide_ayirt_edilir() {
    // Deuteranopi (en yaygın) altında CB-güvenli palet: okuma yönü + A/T baz çiftleri ayrılır.
    let p =
        accessibility::genom_paleti(&ErisilebilirlikAyari::renk_koru(RenkGormeTuru::Deuteranopi));
    let ileri = p.rgb(CizimRengi::ReadIleri);
    let geri = p.rgb(CizimRengi::ReadGeri);
    assert!(accessibility::ayirt_edilebilir_mi(
        ileri,
        geri,
        RenkGormeTuru::Deuteranopi,
        AYIRT_ESIGI
    ));
    let baz_a = p.rgb(CizimRengi::BazA);
    let baz_t = p.rgb(CizimRengi::BazT);
    assert!(accessibility::ayirt_edilebilir_mi(
        baz_a,
        baz_t,
        RenkGormeTuru::Deuteranopi,
        AYIRT_ESIGI
    ));

    // Yüksek kontrast modu: metin/zemin AAA.
    let yk = accessibility::genom_paleti(&ErisilebilirlikAyari {
        yuksek_kontrast: true,
        ..Default::default()
    });
    assert!(accessibility::kontrast_gecer(
        yk.rgb(CizimRengi::CetvelMetin),
        yk.zemin_rgb(),
        accessibility::KontrastSeviyesi::Aaa
    ));
}

// ─── 3) Doğruluk: bcftools / samtools-eşdeğeri golden ─────────────────────────

#[test]
fn ce12_dogruluk_golden_bcftools_samtools() {
    // (a) Varyant filtre sayımı bcftools ile aynı parametrede karşılaştırılır.
    let n = 1_000;
    let beklenen = (0..n).filter(|i| (i % 100) >= 50 && i % 3 == 0).count();
    let mut motor = SafRustMotor::yeni(BellekKaynak::yeni(vec![], varyant_kumesi(n)));
    let sonuc = motor
        .calistir(&Sorgu::yeni(
            ayristir("QUAL >= 50 AND FILTER = PASS").unwrap(),
        ))
        .unwrap();
    let rapor_vcf = DogrulukRaporu::yeni(ReferansArac::Bcftools, "QUAL>=50 && FILTER=PASS sayısı")
        .parametre("koordinat", "1-tabanlı kapalı")
        .parametre("filtre", "QUAL>=50 && FILTER=PASS")
        .sonuc(sonuc.toplam_eslesme.to_string(), beklenen.to_string());
    assert!(rapor_vcf.eslesti());

    // (b) Kapsama bölge uzunluğu samtools (1-tabanlı) ile bedtools (0-tabanlı) eşitlendiğinde aynı.
    // chr1:100-199 (1-tabanlı kapalı) = 100 baz; bedtools 99-199 (0-tabanlı) = 100 baz.
    let samtools_uzunluk = ReferansArac::Samtools.koordinat_tabani().uzunluk(100, 199);
    let (bb, be) = KoordinatTabani::SifirTabanliYariAcik.bir_tabanli_kapaliden(100, 199);
    let bedtools_uzunluk = ReferansArac::Bedtools.koordinat_tabani().uzunluk(bb, be);
    assert_eq!(samtools_uzunluk, 100);
    assert_eq!(
        bedtools_uzunluk, 100,
        "koordinat tabanı eşitlendiğinde uzunluk aynı"
    );
    let rapor_cov =
        DogrulukRaporu::yeni(ReferansArac::Samtools, "chr1:100-199 bölge uzunluğu (baz)")
            .parametre("koordinat", "1-tabanlı kapalı")
            .sonuc(samtools_uzunluk.to_string(), bedtools_uzunluk.to_string());
    assert!(rapor_cov.eslesti());

    // Golden: iki raporun deterministik metni (gürültüsüz → normalize gerekmez).
    let cikti = format!("{}\n{}", rapor_vcf.metin(), rapor_cov.metin());
    golden::dogrula("ce12_dogruluk", &cikti);
}

// ─── 4) Edge-case: boş / tek / bozuk / Unicode → güvenli ──────────────────────

#[test]
fn bos_kaynak_guvenli_ve_rehberli() {
    // Boş kaynak: sorgu boş döner (panik/çökme yok) + boş-durum rehberi (TDA 5).
    let mut motor = SafRustMotor::yeni(BellekKaynak::yeni(vec![], vec![]));
    let sonuc = motor
        .calistir(&Sorgu::yeni(ayristir("QUAL >= 0").unwrap()))
        .unwrap();
    assert_eq!(sonuc.toplam_eslesme, 0);
    assert_eq!(VeriDurumu::siniflandir(0), VeriDurumu::Bos);
    assert!(VeriDurumu::Bos.bos_rehberi("Varyant").is_some());
}

#[test]
fn tek_kayit_ve_buyuk_siniflama() {
    let mut motor = SafRustMotor::yeni(BellekKaynak::yeni(vec![], varyant_kumesi(1)));
    let sonuc = motor
        .calistir(&Sorgu::yeni(ayristir("QUAL >= 0").unwrap()))
        .unwrap();
    assert_eq!(sonuc.toplam_eslesme, 1);
    assert_eq!(VeriDurumu::siniflandir(1), VeriDurumu::TekKayit);
    assert_eq!(VeriDurumu::siniflandir(200_000), VeriDurumu::Buyuk);
}

#[test]
fn bozuk_dosya_acmak_guvenli_red() {
    // Bozuk/var olmayan VCF'i açmak panik değil; standart şema (correlation_id'li) döner.
    use biocraft_core_studio::variant::DosyaKaynak;
    let yetki = YetkiKapisi::yeni([Capability::Fs]);
    let yok = std::env::temp_dir().join(format!("biocraft_yok_{}.vcf", std::process::id()));
    let sonuc = DosyaKaynak::ac(&yok, &yetki);
    assert!(
        sonuc.is_err(),
        "var olmayan dosya Err dönmeli (panik değil)"
    );

    // Eklenti-yerel bozuk-dosya hatası da standart şema + correlation_id taşır.
    let h = edge::bozuk_dosya("ornek.bam", Some(7), "beklenmedik EOF");
    assert_eq!(h.correlation_id.kisa().len(), 8);
    assert!(h.eylem_etiketi.is_some());
}

#[test]
fn unicode_ornek_adi_korunur() {
    // Türkçe/CJK/emoji örnek adı bozulmadan taşınır (panik/� yok).
    assert_eq!(
        edge::guvenli_ad("hasta_çalışması_基因🧬", 64),
        "hasta_çalışması_基因🧬"
    );
}
