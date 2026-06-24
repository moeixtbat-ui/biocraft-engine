//! ÇE-09 (Gün 41) — **Veritabanı erişimini tamamla** entegrasyon testleri.
//!
//! Gerçek ağ kullanılmaz: [`SahteUlastirici`] kayıtlı sentetik PDB/UniProt/Ensembl/UCSC yanıtlarıyla
//! konektörleri **çevrimdışı** uçtan uca sürer (dürüst sınır).  Kontrol listesi (görev):
//! * PDB/UniProt/Ensembl/UCSC aramaları çalışır; **PDB sonucu 3B'de açılır** (KayitTuru::Yapi).
//! * Önbellek tekrar sorguyu hızlandırır + **çevrimdışı** sunar.
//! * Arama geçmişi + favori + tekrar; kaynak-başına hız sınırı çalışır.
//! * Her içe aktarımda **kaynak + tarih + atıf** provenance'a yazılır (MK-34).
//! * Çapraz bağlantı: Ensembl gen sonucu **genom tarayıcıda göster** (Konum) verir.
//! * Golden: birleşik (PDB+UniProt+Ensembl) sonuç tablosu.

use std::time::Duration;

use biocraft_core_studio::db_search::{
    AramaBaglami, AramaOnbellegi, BirlesikPanel, EnsemblKonektor, Eylem, GizlilikKapisi, KayitTuru,
    KaynakHizYoneticisi, Konum, PdbKonektor, SahteUlastirici, Sayfalama, Sorgu, UcscKonektor,
    UniprotKonektor, VeritabaniKonektoru,
};
use biocraft_sdk::biocraft_types::golden;

// ─── Sentetik yanıtlar ───────────────────────────────────────────────────────────

fn pdb_search() -> &'static str {
    r#"{"total_count":1,"result_set":[{"identifier":"1TUP","score":1.0}]}"#
}
fn pdb_entry() -> &'static str {
    r#"{"struct":{"title":"TUMOR SUPPRESSOR P53 COMPLEXED WITH DNA"},
        "exptl":[{"method":"X-RAY DIFFRACTION"}],
        "rcsb_entry_info":{"resolution_combined":[2.2],"deposited_polymer_monomer_count":393}}"#
}
fn pdb_file() -> &'static str {
    "HEADER    ANTITUMOR PROTEIN/DNA\nATOM      1  N   SER A  94\nEND\n"
}
fn uniprot_search() -> &'static str {
    r#"{"results":[{"primaryAccession":"P04637","uniProtkbId":"P53_HUMAN",
        "proteinDescription":{"recommendedName":{"fullName":{"value":"Cellular tumor antigen p53"}}},
        "organism":{"scientificName":"Homo sapiens"},"sequence":{"length":393}}]}"#
}
fn uniprot_fasta() -> &'static str {
    ">sp|P04637|P53_HUMAN Cellular tumor antigen p53\nMEEPQSDPSVEPPLSQETFS\n"
}
fn ensembl_xrefs() -> &'static str {
    r#"[{"id":"ENSG00000141510","type":"gene"}]"#
}
fn ensembl_lookup() -> &'static str {
    r#"{"id":"ENSG00000141510","display_name":"TP53","biotype":"protein_coding",
        "seq_region_name":"17","start":7661779,"end":7687550,"strand":-1,
        "description":"tumor protein p53","species":"homo_sapiens"}"#
}
fn ucsc_tracks() -> &'static str {
    r#"{"genome":"hg38","hg38":{
        "refGene":{"shortLabel":"RefSeq","longLabel":"NCBI RefSeq genes","type":"genePred"}}}"#
}
fn ucsc_getdata() -> &'static str {
    r#"{"genome":"hg38","chrom":"chr17","start":7668421,"end":7687550,"refGene":[]}"#
}

fn hizli_baglam<'a>(u: &'a SahteUlastirici, g: &'a GizlilikKapisi) -> AramaBaglami<'a> {
    let mut b = AramaBaglami::yeni(u, g);
    b.yapi.geri_cekilme_taban = Duration::ZERO;
    b
}

// ─── 1) PDB araması + tek-tık 3B açma ────────────────────────────────────────────

#[test]
fn pdb_arama_ve_3b_acma() {
    let u = SahteUlastirici::yeni()
        .ekle("rcsbsearch", 200, pdb_search())
        .ekle("core/entry/1TUP", 200, pdb_entry())
        .ekle("1TUP.pdb", 200, pdb_file());
    let g = GizlilikKapisi::onayli();
    let baglam = hizli_baglam(&u, &g);

    let mut panel = BirlesikPanel::yeni();
    panel.konektor_ekle(Box::new(PdbKonektor::yeni()));
    panel.sorgu_metni = "p53".into();
    panel.ara(Sayfalama::ilk(20), &baglam).unwrap();

    let sonuc = &panel.sonuclar()[0];
    assert_eq!(sonuc.kimlik, "1TUP");
    assert_eq!(sonuc.tur, KayitTuru::Yapi);

    // PDB sonucu → "yapıya bak" eylemi (ÇE-07 3B görüntüleyici).
    let eylemler = panel.eylemler(sonuc);
    assert!(eylemler
        .iter()
        .any(|e| matches!(e, Eylem::YapiyaBak(k) if k == "1TUP")));

    // Tek-tık getir → PDB metni (3B görüntüleyiciye/projeye) + provenance (CC0 + atıf).
    let kayit = panel.projeye_ekle(sonuc, &baglam).unwrap();
    assert_eq!(kayit.format_ipucu, "pdb");
    assert!(kayit.icerik.starts_with(b"HEADER"));
    let p = &kayit.provenans;
    assert!(p.kaynak.contains("RCSB PDB"));
    assert_eq!(p.blake3.len(), 64);
    let la = p.lisans_atif.as_ref().unwrap();
    assert!(la.lisans.contains("CC0"));
    assert!(la.atif.contains("Berman"));
}

// ─── 2) UniProt + Ensembl (çapraz bağlantı) ──────────────────────────────────────

#[test]
fn uniprot_ve_ensembl_capraz_baglanti() {
    let u = SahteUlastirici::yeni()
        .ekle("uniprotkb/search", 200, uniprot_search())
        .ekle("P04637.fasta", 200, uniprot_fasta())
        .ekle("xrefs/symbol", 200, ensembl_xrefs())
        .ekle("lookup/id/ENSG00000141510", 200, ensembl_lookup());
    let g = GizlilikKapisi::onayli();
    let baglam = hizli_baglam(&u, &g);

    // UniProt protein.
    let uni = UniprotKonektor::yeni();
    let liste = uni
        .ara(&Sorgu::metin("TP53"), Sayfalama::ilk(20), &baglam)
        .unwrap();
    assert_eq!(liste.sonuclar[0].kimlik, "P04637");
    assert_eq!(liste.sonuclar[0].tur, KayitTuru::Protein);
    let kayit = uni.getir("P04637", &baglam).unwrap();
    assert!(kayit
        .provenans
        .lisans_atif
        .as_ref()
        .unwrap()
        .lisans
        .contains("CC BY 4.0"));

    // Ensembl gen + koordinat (genom tarayıcı çapraz bağlantısı).
    let ens = EnsemblKonektor::insan();
    let gen = ens
        .ara(&Sorgu::metin("TP53"), Sayfalama::ilk(20), &baglam)
        .unwrap();
    let g0 = &gen.sonuclar[0];
    assert_eq!(g0.tur, KayitTuru::Gen);
    let konum = g0.konum.as_ref().unwrap();
    assert_eq!(konum.bolge_metni(), "17:7661779-7687550");

    // Panel: konumlu sonuç → "genom tarayıcıda göster".
    let mut panel = BirlesikPanel::yeni();
    panel.konektor_ekle(Box::new(EnsemblKonektor::insan()));
    let eylemler = panel.eylemler(g0);
    assert!(eylemler
        .iter()
        .any(|e| matches!(e, Eylem::GenomdaGoster(k) if k.kromozom == "17")));
}

// ─── 3) UCSC iz listesi + bölgesel veri ──────────────────────────────────────────

#[test]
fn ucsc_iz_ve_bolge_verisi() {
    let u = SahteUlastirici::yeni()
        .ekle("list/tracks", 200, ucsc_tracks())
        .ekle("getData/track", 200, ucsc_getdata());
    let g = GizlilikKapisi::onayli();
    let baglam = hizli_baglam(&u, &g);
    let kon = UcscKonektor::hg38();

    let liste = kon
        .ara(&Sorgu::metin("refseq"), Sayfalama::ilk(20), &baglam)
        .unwrap();
    assert_eq!(liste.sonuclar[0].kimlik, "refGene");
    assert_eq!(liste.sonuclar[0].tur, KayitTuru::Iz);

    // Bölgesel iz verisi → provenance (UCSC atıf).
    let konum = Konum::yeni("17", 7_668_422, 7_687_550);
    let kayit = kon.track_verisi("refGene", &konum, &baglam).unwrap();
    assert_eq!(kayit.format_ipucu, "json");
    assert!(kayit
        .provenans
        .lisans_atif
        .as_ref()
        .unwrap()
        .atif
        .contains("Kent"));
}

// ─── 4) Önbellek: tekrar sorgu hızı + çevrimdışı ─────────────────────────────────

#[test]
fn onbellek_tekrar_ve_cevrimdisi() {
    let dizin = std::env::temp_dir().join(format!("biocraft_ce09_ob_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dizin);

    // İlk: ağ var → önbelleğe yaz.
    {
        let u = SahteUlastirici::yeni()
            .ekle("uniprotkb/search", 200, uniprot_search())
            .ekle("P04637.fasta", 200, uniprot_fasta());
        let g = GizlilikKapisi::onayli();
        let baglam = hizli_baglam(&u, &g);
        let ob = AramaOnbellegi::varsayilan(&dizin).unwrap();
        let mut panel = BirlesikPanel::yeni().with_onbellek(ob);
        panel.konektor_ekle(Box::new(UniprotKonektor::yeni()));
        panel.sorgu_metni = "TP53".into();
        panel.ara(Sayfalama::ilk(20), &baglam).unwrap();
        assert_eq!(panel.sonuclar().len(), 1);
        assert!(!panel.onbellekten());

        // Veriyi de indir → veri önbelleğine (provenance ile) yazılır.
        let sonuc = panel.sonuclar()[0].clone();
        let _ = panel.projeye_ekle(&sonuc, &baglam).unwrap();
    }

    // İkinci: ÇEVRİMDIŞI → sonuç önbellekten, veri önbellekten (provenance korunur).
    {
        let u = SahteUlastirici::cevrimdisi();
        let g = GizlilikKapisi::onayli();
        let baglam = hizli_baglam(&u, &g);
        let ob = AramaOnbellegi::varsayilan(&dizin).unwrap();
        let mut panel = BirlesikPanel::yeni().with_onbellek(ob);
        panel.konektor_ekle(Box::new(UniprotKonektor::yeni()));
        panel.sorgu_metni = "TP53".into();
        panel.ara(Sayfalama::ilk(20), &baglam).unwrap();
        assert_eq!(panel.sonuclar().len(), 1);
        assert!(panel.onbellekten());

        // Çevrimdışı veri getirme önbellekten gelir + provenance (atıf) korunmuş.
        let sonuc = panel.sonuclar()[0].clone();
        let kayit = panel.projeye_ekle(&sonuc, &baglam).unwrap();
        assert!(kayit.icerik.starts_with(b">sp|P04637"));
        assert!(kayit
            .provenans
            .lisans_atif
            .as_ref()
            .unwrap()
            .lisans
            .contains("CC BY 4.0"));
    }
    let _ = std::fs::remove_dir_all(&dizin);
}

// ─── 5) Geçmiş + favori + tekrar çalıştır ────────────────────────────────────────

#[test]
fn gecmis_favori_ve_tekrar() {
    let u = SahteUlastirici::yeni().ekle("uniprotkb/search", 200, uniprot_search());
    let g = GizlilikKapisi::onayli();
    let baglam = hizli_baglam(&u, &g);
    let mut panel = BirlesikPanel::yeni();
    panel.konektor_ekle(Box::new(UniprotKonektor::yeni()));

    panel.sorgu_metni = "TP53".into();
    panel.ara(Sayfalama::ilk(20), &baglam).unwrap();
    panel.sorgu_metni = "BRCA1".into();
    panel.ara(Sayfalama::ilk(20), &baglam).unwrap();

    assert_eq!(panel.gecmis().len(), 2);
    assert_eq!(panel.gecmis().girdiler()[0].sorgu, "BRCA1");

    // "TP53" aramasını favori yap.
    panel.gecmis_mut().favori_degistir(1);
    assert_eq!(panel.gecmis().favoriler().len(), 1);
    assert_eq!(panel.gecmis().favoriler()[0].sorgu, "TP53");

    // Tekrar çalıştır → sorgu kutusu dolar.
    let sorgu = panel.gecmisten_yukle(1).unwrap();
    assert_eq!(sorgu.metin, "TP53");
}

// ─── 6) Kaynak-başına hız sınırı bağlama ─────────────────────────────────────────

#[test]
fn kaynak_basina_hiz_yoneticisi_baglanir() {
    let u = SahteUlastirici::yeni().ekle("uniprotkb/search", 200, uniprot_search());
    let g = GizlilikKapisi::onayli();
    let yonetici = KaynakHizYoneticisi::yeni();
    let mut baglam = AramaBaglami::yeni(&u, &g).with_hiz_yoneticisi(&yonetici);
    baglam.yapi.geri_cekilme_taban = Duration::ZERO;

    let kon = UniprotKonektor::yeni();
    kon.ara(&Sorgu::metin("TP53"), Sayfalama::ilk(5), &baglam)
        .unwrap();
    // Konektör bir istek yolladığından UniProt kovası oluşturuldu.
    assert!(yonetici.kaynak_sayisi() >= 1);
}

// ─── 7) Onaysız dış sorgu + PHI sınırı (yeni konektörlerde de) ────────────────────

#[test]
fn yeni_konektorlerde_de_onay_sinirir() {
    let u = SahteUlastirici::yeni().ekle("rcsbsearch", 200, pdb_search());
    let g = GizlilikKapisi::yeni(); // onaylanmadı
    let baglam = hizli_baglam(&u, &g);
    let hata = PdbKonektor::yeni()
        .ara(&Sorgu::metin("p53"), Sayfalama::ilk(20), &baglam)
        .err()
        .unwrap();
    assert_eq!(hata.ne_oldu, "Dış sorgu onayı gerekli");
}

// ─── 8) Golden: birleşik (PDB+UniProt+Ensembl) sonuç tablosu ──────────────────────

#[test]
fn golden_gun41_birlesik_tablo() {
    let u = SahteUlastirici::yeni()
        .ekle("rcsbsearch", 200, pdb_search())
        .ekle("core/entry/1TUP", 200, pdb_entry())
        .ekle("uniprotkb/search", 200, uniprot_search())
        .ekle("xrefs/symbol", 200, ensembl_xrefs())
        .ekle("lookup/id/ENSG00000141510", 200, ensembl_lookup());
    let g = GizlilikKapisi::onayli();
    let baglam = hizli_baglam(&u, &g);

    let mut panel = BirlesikPanel::yeni();
    panel.konektor_ekle(Box::new(PdbKonektor::yeni()));
    panel.konektor_ekle(Box::new(UniprotKonektor::yeni()));
    panel.konektor_ekle(Box::new(EnsemblKonektor::insan()));
    panel.sorgu_metni = "TP53".to_string();
    panel.ara(Sayfalama::ilk(20), &baglam).unwrap();
    assert!(panel.son_hatalar().is_empty());

    let mut metin = String::from("KAYNAK | KİMLİK | TÜR | UZUNLUK | ORGANİZMA | KONUM | BAŞLIK\n");
    for s in panel.sonuclar() {
        metin.push_str(&format!(
            "{} | {} | {} | {} | {} | {} | {}\n",
            s.kaynak,
            s.kimlik,
            s.tur.etiket(),
            s.uzunluk
                .map(|u| u.to_string())
                .unwrap_or_else(|| "-".into()),
            s.organizma.as_deref().unwrap_or("-"),
            s.konum
                .as_ref()
                .map(|k| k.bolge_metni())
                .unwrap_or_else(|| "-".into()),
            s.baslik,
        ));
    }
    golden::dogrula("ce09_gun41_birlesik", &metin);
}
