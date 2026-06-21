//! Proje sihirbazı testleri — saf mantık (gezinme/doğrulama/taslak) + headless egui dumanı.

use super::*;
use crate::i18n::Dil;
use crate::tokens::Tokenlar;
use biocraft_types::DataClassification;

/// Boş bağlamla bir sihirbaz kurar (varsayılan konum ön-doldurulmuş → konum geçerli).
fn sihirbaz() -> ProjeSihirbazi {
    ProjeSihirbazi::yeni(SihirbazBaglam {
        dusuk_ram: false,
        dagitik_eklenti_kurulu: false,
        varsayilan_konum: "/projeler".into(),
    })
}

/// Tüm zorunlu alanları dolu, geçerli bir sihirbaz (Özet'e hazır).
fn gecerli_sihirbaz() -> ProjeSihirbazi {
    let mut w = sihirbaz();
    w.bilgi.ad = "Genom Çalışması".to_string();
    w.bilgi.konum = "/projeler/genom".to_string();
    w.siniflandirma_sec(DataClassification::Normal);
    w
}

// ── Adım navigasyonu ──────────────────────────────────────────────────────────

#[test]
fn adim_indeksleri_ve_toplam_dogru() {
    assert_eq!(SihirbazAdim::toplam(), 6);
    assert_eq!(SihirbazAdim::Sablon.indeks(), 0);
    assert_eq!(SihirbazAdim::Ozet.indeks(), 5);
}

#[test]
fn sonraki_onceki_uclarda_none() {
    assert_eq!(SihirbazAdim::Sablon.onceki(), None);
    assert_eq!(SihirbazAdim::Ozet.sonraki(), None);
    assert_eq!(SihirbazAdim::Sablon.sonraki(), Some(SihirbazAdim::Bilgi));
    assert_eq!(SihirbazAdim::Ozet.onceki(), Some(SihirbazAdim::Dagitik));
}

#[test]
fn ileri_geri_gezinir() {
    let mut w = gecerli_sihirbaz();
    assert_eq!(w.adim, SihirbazAdim::Sablon);
    w.ileri(); // Şablon her zaman geçerli.
    assert_eq!(w.adim, SihirbazAdim::Bilgi);
    w.geri();
    assert_eq!(w.adim, SihirbazAdim::Sablon);
    w.geri(); // İlk adımda geri pasif.
    assert_eq!(w.adim, SihirbazAdim::Sablon);
}

// ── Doğrulama ─────────────────────────────────────────────────────────────────

#[test]
fn sablon_adimi_her_zaman_gecerli() {
    let w = sihirbaz();
    assert!(w.adim_gecerli(SihirbazAdim::Sablon));
}

#[test]
fn bos_ad_engellenir() {
    let mut w = sihirbaz();
    w.bilgi.ad = "  ".to_string();
    let h = w.adim_hatalari(SihirbazAdim::Bilgi);
    assert!(h.contains(&DogrulamaHatasi::AdBos));
}

#[test]
fn gecersiz_ad_karakteri_engellenir() {
    let mut w = sihirbaz();
    w.bilgi.ad = "genom/çalışma".to_string();
    let h = w.adim_hatalari(SihirbazAdim::Bilgi);
    assert!(h.contains(&DogrulamaHatasi::AdGecersizKarakter));
}

#[test]
fn bos_konum_engellenir() {
    let mut w = sihirbaz();
    w.bilgi.ad = "X".to_string();
    w.bilgi.konum = "   ".to_string();
    let h = w.adim_hatalari(SihirbazAdim::Bilgi);
    assert!(h.contains(&DogrulamaHatasi::KonumBos));
}

#[test]
fn gecersiz_orcid_engellenir_bos_orcid_serbest() {
    let mut w = sihirbaz();
    w.bilgi.ad = "X".to_string();
    // Boş ORCID = geçerli (opsiyonel).
    assert!(!w
        .adim_hatalari(SihirbazAdim::Bilgi)
        .contains(&DogrulamaHatasi::OrcidGecersiz));
    // Bozuk ORCID = hata.
    w.bilgi.orcid_ham = "1234".to_string();
    assert!(w
        .adim_hatalari(SihirbazAdim::Bilgi)
        .contains(&DogrulamaHatasi::OrcidGecersiz));
    // Geçerli ORCID = hata yok.
    w.bilgi.orcid_ham = "0000-0002-1825-0097".to_string();
    assert!(!w
        .adim_hatalari(SihirbazAdim::Bilgi)
        .contains(&DogrulamaHatasi::OrcidGecersiz));
}

// ── ZORUNLU veri sınıflandırma (MK-42) ────────────────────────────────────────

#[test]
fn siniflandirma_secilmeden_gizlilik_adimi_gecersiz() {
    let w = sihirbaz();
    assert!(w.gizlilik.siniflandirma.is_none());
    let h = w.adim_hatalari(SihirbazAdim::Gizlilik);
    assert!(h.contains(&DogrulamaHatasi::SiniflandirmaSecilmedi));
    assert!(!w.adim_gecerli(SihirbazAdim::Gizlilik));
}

#[test]
fn siniflandirma_secilmeden_ileri_gidilemez() {
    let mut w = sihirbaz();
    w.bilgi.ad = "X".to_string();
    // Gizlilik adımına ilerle.
    w.adim = SihirbazAdim::Gizlilik;
    assert!(
        !w.ileri_aktif(),
        "sınıflandırma seçilmeden İleri pasif olmalı"
    );
    w.ileri();
    assert_eq!(w.adim, SihirbazAdim::Gizlilik, "İleri çalışmamalı");
    // Sınıflandırma seçilince ilerler.
    w.siniflandirma_sec(DataClassification::Sentetik);
    assert!(w.ileri_aktif());
    w.ileri();
    assert_eq!(w.adim, SihirbazAdim::Dagitik);
}

#[test]
fn ozet_tum_onceki_adimlari_dener() {
    // Ad boş + sınıflandırma yok → özet geçersiz, iki hata da görünür.
    let w = sihirbaz();
    let h = w.adim_hatalari(SihirbazAdim::Ozet);
    assert!(h.contains(&DogrulamaHatasi::AdBos));
    assert!(h.contains(&DogrulamaHatasi::SiniflandirmaSecilmedi));
    assert!(!w.tumden_gecerli());
}

// ── Gizlilik varsayılanları ───────────────────────────────────────────────────

#[test]
fn gizlilik_varsayilanlari_dogru() {
    let w = sihirbaz();
    assert!(w.gizlilik.tamamen_yerel, "varsayılan: tamamen yerel");
    assert!(!w.gizlilik.ai_havuzu_katki, "varsayılan: AI havuzu Hayır");
    assert!(w.gizlilik.sifreleme, "varsayılan: şifreli-yerel");
}

#[test]
fn phi_secimi_guvenli_kilitleri_zorlar() {
    let mut w = sihirbaz();
    // Önce kullanıcı tehlikeli ayar yapsın.
    w.gizlilik.sifreleme = false;
    w.gizlilik.ai_havuzu_katki = true;
    w.gizlilik.tamamen_yerel = false;
    // PHI seçilince hepsi güvenli değere kilitlenir (MK-42).
    w.siniflandirma_sec(DataClassification::HasasPhi);
    assert!(w.gizlilik.phi_kilitli());
    assert!(w.gizlilik.sifreleme);
    assert!(!w.gizlilik.ai_havuzu_katki);
    assert!(w.gizlilik.tamamen_yerel);
}

// ── Akıllı varsayılan (donanım) ───────────────────────────────────────────────

#[test]
fn dusuk_ram_akis_modunu_acar() {
    let dusuk = ProjeSihirbazi::yeni(SihirbazBaglam {
        dusuk_ram: true,
        dagitik_eklenti_kurulu: false,
        varsayilan_konum: "/p".into(),
    });
    assert!(
        dusuk.veri.akis_modu,
        "düşük RAM → akış modu varsayılan açık"
    );

    let guclu = sihirbaz();
    assert!(!guclu.veri.akis_modu, "güçlü makinede akış modu kapalı");
}

// ── Dağıtık ağ eklenti durumu ─────────────────────────────────────────────────

#[test]
fn dagitik_eklenti_durumu_baglamdan_gelir() {
    let yok = sihirbaz();
    assert!(!yok.dagitik.eklenti_kurulu);
    let var = ProjeSihirbazi::yeni(SihirbazBaglam {
        dusuk_ram: false,
        dagitik_eklenti_kurulu: true,
        varsayilan_konum: "/p".into(),
    });
    assert!(var.dagitik.eklenti_kurulu);
}

// ── Taslak üretimi ────────────────────────────────────────────────────────────

#[test]
fn taslak_gecersizken_none() {
    let w = sihirbaz(); // ad boş + sınıf yok.
    assert!(w.taslak_uret().is_none());
}

#[test]
fn taslak_gecerli_alanlari_dogru_tasir() {
    let mut w = gecerli_sihirbaz();
    w.sablon = ProjeSablonu::CrisprGenDuzenleme;
    w.bilgi.kurum = "  BioLab  ".to_string();
    w.bilgi.etiketler_ham = "genom, crispr, genom".to_string(); // tekrar elenmeli
    w.bilgi.orcid_ham = "0000-0002-1825-0097".to_string();
    w.siniflandirma_sec(DataClassification::Sentetik);

    let t = w.taslak_uret().expect("geçerli sihirbaz taslak üretmeli");
    assert_eq!(t.sablon, ProjeSablonu::CrisprGenDuzenleme);
    assert_eq!(t.ad, "Genom Çalışması");
    assert_eq!(t.konum, std::path::PathBuf::from("/projeler/genom"));
    assert_eq!(t.kurum, "BioLab"); // trim'lenmiş
    assert_eq!(t.etiketler, vec!["genom", "crispr"]); // dedup
    assert_eq!(t.orcid.as_deref(), Some("0000-0002-1825-0097"));
    assert_eq!(t.siniflandirma, DataClassification::Sentetik);
    // Gizlilik varsayılanları taslağa geçti.
    assert!(t.tamamen_yerel && t.sifreleme && !t.ai_havuzu_katki);
}

#[test]
fn taslak_bos_orcid_none_olur() {
    let w = gecerli_sihirbaz();
    let t = w.taslak_uret().unwrap();
    assert_eq!(t.orcid, None);
}

#[test]
fn taslak_dagitik_eklenti_yoksa_etkin_olamaz() {
    let mut w = gecerli_sihirbaz();
    // Eklenti yok ama kullanıcı 'etkin' işaretlese bile taslakta false olmalı.
    w.dagitik.etkin = true;
    let t = w.taslak_uret().unwrap();
    assert!(
        !t.dagitik_ag_etkin,
        "eklenti kurulu değilken dağıtık ağ etkinleşemez"
    );
}

// ── Saf yardımcılar ───────────────────────────────────────────────────────────

#[test]
fn ad_gecerli_kontrolu() {
    assert!(ad_gecerli("İnsan Genomu"));
    assert!(!ad_gecerli(""));
    assert!(!ad_gecerli("  "));
    assert!(!ad_gecerli("a/b"));
    assert!(!ad_gecerli("c:dosya"));
    assert!(!ad_gecerli("x?y"));
}

#[test]
fn orcid_gecerli_kontrolu() {
    assert!(orcid_gecerli("0000-0002-1825-0097"));
    assert!(orcid_gecerli("0000-0001-2345-678X")); // son hane X
    assert!(!orcid_gecerli("0000000218250097")); // tiresiz
    assert!(!orcid_gecerli("0000-0002-1825-009")); // kısa
    assert!(!orcid_gecerli("000A-0002-1825-0097")); // harf
    assert!(!orcid_gecerli(""));
}

#[test]
fn etiket_ayristirma_temizler() {
    assert_eq!(
        etiketleri_ayristir(" a, b ,c,"),
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
    assert_eq!(etiketleri_ayristir("x, x, x"), vec!["x".to_string()]); // dedup
    assert!(etiketleri_ayristir("   ").is_empty());
}

// ── Headless egui dumanı ──────────────────────────────────────────────────────

/// Bir kareyi headless egui'de çizer ve sonucu döndürür (panik = test başarısız).
fn kare_ciz(w: &mut ProjeSihirbazi, esc: bool) -> Option<SihirbazSonucu> {
    let ctx = egui::Context::default();
    let mut input = egui::RawInput::default();
    if esc {
        input.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        });
    }
    let mut sonuc = None;
    let _ = ctx.run(input, |ctx| {
        sonuc = w.ciz(ctx, Dil::Tr, &Tokenlar::acik());
    });
    sonuc
}

#[test]
fn her_adim_panik_olmadan_cizilir() {
    for &a in SihirbazAdim::TUMU {
        let mut w = gecerli_sihirbaz();
        w.adim = a;
        let _ = kare_ciz(&mut w, false);
        // Diğer dilde de çiz (i18n yolu).
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            w.ciz(ctx, Dil::En, &Tokenlar::koyu());
        });
    }
}

#[test]
fn esc_iptal_dondurur() {
    let mut w = gecerli_sihirbaz();
    assert_eq!(kare_ciz(&mut w, true), Some(SihirbazSonucu::Iptal));
}

#[test]
fn dagitik_adimi_eklenti_yokken_cizilir() {
    // Eklenti yok → [İndir] yönlendirmesi içeren kart panik olmadan çizilmeli.
    let mut w = gecerli_sihirbaz();
    w.adim = SihirbazAdim::Dagitik;
    let _ = kare_ciz(&mut w, false);
}
