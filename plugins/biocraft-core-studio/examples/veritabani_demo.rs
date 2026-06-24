//! ÇE-09 (Gün 40-41) — **Veritabanı erişimi demo** (çevrimdışı; sahte ulaştırıcı).
//!
//! Gerçek ağ yığını bu sürümde bağlı değildir (dürüst sınır); demo, [`SahteUlastirici`] ile
//! sentetik yanıtlar kullanarak çerçeveyi uçtan uca gösterir:
//! 1) gizlilik/onay sınırı, 2) NCBI araması + tek-tık getirme (provenance), 3) BLAST işi (Job),
//! 4) birleşik panel (kaynak rozeti + eylemler), 5) PHI engeli,
//! 6) **Gün 41:** PDB/UniProt/Ensembl/UCSC + çapraz bağlantı, 7) önbellek (hız + çevrimdışı),
//! 8) arama geçmişi + favori.
//!
//! Çalıştır: `cargo run -p biocraft-core-studio --example veritabani_demo`

use std::time::Duration;

use biocraft_core_studio::db_search::{
    AramaBaglami, AramaOnbellegi, BirlesikPanel, BlastKonektor, BlastProgram, DisVeri,
    EnsemblKonektor, Eylem, GizlilikKapisi, HassasiyetEtiketi, Konum, NcbiKonektor, PdbKonektor,
    SahteUlastirici, Sayfalama, Sorgu, UcscKonektor, UniprotKonektor, VeritabaniKonektoru,
    YoklamaAyari,
};
use biocraft_sdk::biocraft_types::IsKulpu;

fn baslik(no: u8, ad: &str) {
    println!("\n══════ {no}. {ad} ══════");
}

fn sahte_ag() -> SahteUlastirici {
    SahteUlastirici::yeni()
        .ekle(
            "esearch.fcgi",
            200,
            r#"{"esearchresult":{"count":"2","idlist":["7157","672"]}}"#,
        )
        .ekle(
            "esummary.fcgi",
            200,
            r#"{"result":{"uids":["7157","672"],
              "7157":{"caption":"NM_000546","title":"Homo sapiens TP53 mRNA","slen":2591,"organism":"Homo sapiens"},
              "672":{"caption":"NM_007294","title":"Homo sapiens BRCA1 mRNA","slen":7088,"organism":"Homo sapiens"}}}"#,
        )
        .ekle("efetch.fcgi", 200, ">NM_000546.6 TP53\nGATCCACGTGACGT\n")
        .ekle(
            "CMD=Put",
            200,
            "<!--QBlastInfoBegin\n RID = DEMO_RID\n RTOE = 9\nQBlastInfoEnd-->",
        )
        .ekle(
            "FORMAT_OBJECT=SearchInfo",
            200,
            "<!--QBlastInfoBegin\n Status=READY\nQBlastInfoEnd-->",
        )
        .ekle(
            "FORMAT_TYPE=Tabular",
            200,
            "q\tNM_000546.6\t100.00\t14\t0\t0\t1\t14\t1\t14\t1e-04\t28.0",
        )
        // ── Gün 41 kaynakları ──
        .ekle(
            "rcsbsearch",
            200,
            r#"{"total_count":1,"result_set":[{"identifier":"1TUP","score":1.0}]}"#,
        )
        .ekle(
            "core/entry/1TUP",
            200,
            r#"{"struct":{"title":"TUMOR SUPPRESSOR P53 COMPLEXED WITH DNA"},
                "exptl":[{"method":"X-RAY DIFFRACTION"}],
                "rcsb_entry_info":{"resolution_combined":[2.2],"deposited_polymer_monomer_count":393}}"#,
        )
        .ekle("1TUP.pdb", 200, "HEADER    ANTITUMOR PROTEIN/DNA\nATOM ...\nEND\n")
        .ekle(
            "uniprotkb/search",
            200,
            r#"{"results":[{"primaryAccession":"P04637","uniProtkbId":"P53_HUMAN",
                "proteinDescription":{"recommendedName":{"fullName":{"value":"Cellular tumor antigen p53"}}},
                "organism":{"scientificName":"Homo sapiens"},"sequence":{"length":393}}]}"#,
        )
        .ekle(
            "xrefs/symbol",
            200,
            r#"[{"id":"ENSG00000141510","type":"gene"}]"#,
        )
        .ekle(
            "lookup/id/ENSG00000141510",
            200,
            r#"{"id":"ENSG00000141510","display_name":"TP53","biotype":"protein_coding",
                "seq_region_name":"17","start":7661779,"end":7687550,"strand":-1,
                "description":"tumor protein p53","species":"homo_sapiens"}"#,
        )
        .ekle(
            "list/tracks",
            200,
            r#"{"genome":"hg38","hg38":{"refGene":{"shortLabel":"RefSeq","longLabel":"NCBI RefSeq genes","type":"genePred"}}}"#,
        )
        .ekle("getData/track", 200, r#"{"genome":"hg38","chrom":"chr17","refGene":[]}"#)
}

fn main() {
    println!("BioCraft Studio — ÇE-09 Veritabanı Erişimi (Gün 40, çevrimdışı demo)");
    let u = sahte_ag();

    // ── 1) Gizlilik / onay sınırı (MK-41) ───────────────────────────────────────
    baslik(1, "Dış sorgu onayı (şeffaflık)");
    let onaysiz = GizlilikKapisi::yeni();
    let onizleme = onaysiz.onizleme(
        "NCBI nucleotide",
        "eutils.ncbi.nlm.nih.gov",
        DisVeri::Metin("TP53"),
    );
    println!(
        "Gönderilecek: {}  →  {}  (onay gerekli: {})",
        onizleme.gonderilen_ozet, onizleme.hedef_aciklama, onizleme.onay_gerekli
    );
    println!("Kullanıcı onaylıyor → dış sorgular etkin.");
    let g = GizlilikKapisi::onayli();
    let mut baglam = AramaBaglami::yeni(&u, &g);
    baglam.yapi.geri_cekilme_taban = Duration::ZERO;

    // ── 2) NCBI araması + tek-tık getirme (provenance) ──────────────────────────
    baslik(2, "NCBI nucleotide araması + tek-tık yükleme");
    let ncbi = NcbiKonektor::nukleotid();
    let liste = ncbi
        .ara(&Sorgu::metin("TP53"), Sayfalama::ilk(10), &baglam)
        .unwrap();
    println!(
        "Toplam {} eşleşme; bu sayfada {}:",
        liste.sayfa.toplam,
        liste.sonuclar.len()
    );
    for s in &liste.sonuclar {
        println!(
            "  [{}] {} — {} ({} bp, {})",
            s.kaynak,
            s.kimlik,
            s.baslik,
            s.uzunluk.unwrap_or(0),
            s.organizma.as_deref().unwrap_or("-")
        );
    }
    let kayit = ncbi.getir("7157", &baglam).unwrap();
    println!(
        "Getirildi: {} ({} bayt, {})  → köken: {}",
        kayit.kimlik,
        kayit.icerik.len(),
        kayit.format_ipucu,
        kayit.provenans.kaynak
    );
    println!(
        "  Lisans/atıf: {}",
        kayit.provenans.lisans_atif.as_ref().unwrap().atif
    );

    // ── 3) BLAST işi (gönder → durum → sonuç; Job/ilerleme) ─────────────────────
    baslik(3, "BLAST (uzun iş: gönder → durum izle → sonuç)");
    let blast = BlastKonektor::varsayilan(BlastProgram::Blastn).with_yoklama(YoklamaAyari {
        azami_yoklama: 5,
        aralik: Duration::ZERO,
    });
    let isi = blast
        .gonder("GATCCACGTG", HassasiyetEtiketi::Genel, &baglam)
        .unwrap();
    println!(
        "İş gönderildi: RID={} (tahmini {} sn)",
        isi.rid, isi.tahmini_sure_sn
    );
    let is = IsKulpu::yeni("BLAST demo", None);
    let hizalamalar = blast
        .blast_calistir("GATCCACGTG", HassasiyetEtiketi::Genel, &baglam, &is)
        .unwrap();
    println!("Durum: {:?}; {} hizalama:", is.durum(), hizalamalar.len());
    for h in &hizalamalar {
        println!(
            "  {} — %{:.1} özdeşlik, E={:.0e}, bit={}",
            h.konu_kimligi, h.ozdeslik_yuzde, h.e_deger, h.bit_skoru
        );
    }

    // ── 4) Birleşik panel (kaynak rozeti + eylemler) ────────────────────────────
    baslik(4, "Birleşik panel (kaynak rozetli sonuç + eylemler)");
    let mut panel = BirlesikPanel::yeni();
    panel.konektor_ekle(Box::new(NcbiKonektor::nukleotid()));
    panel.konektor_ekle(Box::new(
        BlastKonektor::varsayilan(BlastProgram::Blastn).with_yoklama(YoklamaAyari {
            azami_yoklama: 3,
            aralik: Duration::ZERO,
        }),
    ));
    panel.sorgu_metni = "TP53".to_string();
    panel.ara(Sayfalama::ilk(10), &baglam).unwrap();
    println!("Birleşik {} sonuç:", panel.sonuclar().len());
    for s in panel.sonuclar() {
        let eylemler: Vec<&str> = panel
            .eylemler(s)
            .iter()
            .map(|e| match e {
                Eylem::TarayicidaAc(_) => "tarayıcıda aç",
                Eylem::YapiyaBak(_) => "yapıya bak",
                Eylem::GenomdaGoster(_) => "genom tarayıcıda göster",
                Eylem::ProjeyeEkle(_) => "projeye ekle",
            })
            .collect();
        println!(
            "  [{}] {} → eylemler: {}",
            s.kaynak,
            s.kimlik,
            eylemler.join(", ")
        );
    }

    // ── 5) PHI engeli (MK-42/43) ────────────────────────────────────────────────
    baslik(5, "PHI/hassas engeli (cihazdan çıkamaz)");
    match blast.gonder("HASTADIZISI", HassasiyetEtiketi::Phi, &baglam) {
        Ok(_) => println!("BEKLENMEDİK: PHI dizi gönderildi!"),
        Err(e) => println!("Engellendi ✔  {} — {}", e.ne_oldu, e.nasil_cozulur),
    }

    // ── 6) Gün 41: PDB/UniProt/Ensembl/UCSC + çapraz bağlantı ───────────────────
    baslik(
        6,
        "Yeni kaynaklar (PDB/UniProt/Ensembl/UCSC) + çapraz bağlantı",
    );
    let pdb = PdbKonektor::yeni();
    let yapi = pdb
        .ara(&Sorgu::metin("p53"), Sayfalama::ilk(5), &baglam)
        .unwrap();
    let y0 = &yapi.sonuclar[0];
    println!("PDB: {} — {} ({})", y0.kimlik, y0.baslik, y0.aciklama);
    let pdb_kayit = pdb.getir(&y0.kimlik, &baglam).unwrap();
    println!(
        "  → 3B'ye aç ({}): {} bayt, lisans {}",
        pdb_kayit.format_ipucu,
        pdb_kayit.icerik.len(),
        pdb_kayit.provenans.lisans_atif.as_ref().unwrap().lisans
    );

    let uni = UniprotKonektor::yeni();
    let prot = uni
        .ara(&Sorgu::metin("TP53"), Sayfalama::ilk(5), &baglam)
        .unwrap();
    println!(
        "UniProt: {} — {} ({} aa, {})",
        prot.sonuclar[0].kimlik,
        prot.sonuclar[0].baslik,
        prot.sonuclar[0].uzunluk.unwrap_or(0),
        prot.sonuclar[0].organizma.as_deref().unwrap_or("-")
    );

    let ens = EnsemblKonektor::insan();
    let genler = ens
        .ara(&Sorgu::metin("TP53"), Sayfalama::ilk(5), &baglam)
        .unwrap();
    let gen = &genler.sonuclar[0];
    println!(
        "Ensembl: {} — {} → çapraz bağlantı: genom tarayıcıda {}",
        gen.kimlik,
        gen.baslik,
        gen.konum
            .as_ref()
            .map(|k| k.bolge_metni())
            .unwrap_or_default()
    );

    let ucsc = UcscKonektor::hg38();
    let izler = ucsc
        .ara(&Sorgu::metin("refseq"), Sayfalama::ilk(5), &baglam)
        .unwrap();
    println!(
        "UCSC ({}): iz '{}' bulundu",
        ucsc.genom(),
        izler.sonuclar[0].kimlik
    );
    let iz_verisi = ucsc
        .track_verisi("refGene", &Konum::yeni("17", 7_668_422, 7_687_550), &baglam)
        .unwrap();
    println!(
        "  → bölgesel iz verisi: {} bayt (atıf: {})",
        iz_verisi.icerik.len(),
        iz_verisi.provenans.lisans_atif.as_ref().unwrap().atif
    );

    // ── 7) Önbellek (hız + çevrimdışı) ──────────────────────────────────────────
    baslik(7, "Akıllı önbellek (tekrar hızlı + çevrimdışı sunar)");
    let ob_dizin = std::env::temp_dir().join(format!("biocraft_demo_ob_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&ob_dizin);
    {
        let ob = AramaOnbellegi::varsayilan(&ob_dizin).unwrap();
        let mut panel = BirlesikPanel::yeni().with_onbellek(ob);
        panel.konektor_ekle(Box::new(UniprotKonektor::yeni()));
        panel.sorgu_metni = "TP53".into();
        panel.ara(Sayfalama::ilk(5), &baglam).unwrap();
        println!(
            "İlk arama (ağ): {} sonuç, önbellekten={}",
            panel.sonuclar().len(),
            panel.onbellekten()
        );
    }
    {
        // Çevrimdışı ulaştırıcı — yalnız önbellekten sunulur.
        let cevrimdisi = SahteUlastirici::cevrimdisi();
        let baglam2 = AramaBaglami::yeni(&cevrimdisi, &g);
        let ob = AramaOnbellegi::varsayilan(&ob_dizin).unwrap();
        let mut panel = BirlesikPanel::yeni().with_onbellek(ob);
        panel.konektor_ekle(Box::new(UniprotKonektor::yeni()));
        panel.sorgu_metni = "TP53".into();
        panel.ara(Sayfalama::ilk(5), &baglam2).unwrap();
        println!(
            "İkinci arama (ÇEVRİMDIŞI): {} sonuç, önbellekten={} ✔",
            panel.sonuclar().len(),
            panel.onbellekten()
        );
    }
    let _ = std::fs::remove_dir_all(&ob_dizin);

    // ── 8) Arama geçmişi + favori ───────────────────────────────────────────────
    baslik(8, "Arama geçmişi (tekrar çalıştır + favori)");
    let mut panel = BirlesikPanel::yeni();
    panel.konektor_ekle(Box::new(NcbiKonektor::nukleotid()));
    for q in ["TP53", "BRCA1", "EGFR"] {
        panel.sorgu_metni = q.into();
        let _ = panel.ara(Sayfalama::ilk(5), &baglam);
    }
    println!("Geçmiş ({} kayıt):", panel.gecmis().len());
    for (i, gir) in panel.gecmis().girdiler().iter().enumerate() {
        println!("  [{i}] {} ({} sonuç)", gir.sorgu, gir.sonuc_sayisi);
    }
    panel.gecmis_mut().favori_degistir(2); // en eski (TP53) favori
    println!(
        "Favori sayısı: {}; '{}' tekrar çalıştırılabilir.",
        panel.gecmis().favoriler().len(),
        panel
            .gecmisten_yukle(2)
            .map(|s| s.metin)
            .unwrap_or_default()
    );

    println!("\n✔ ÇE-09 (Gün 40-41) demo tamam — gerçek ağ olmadan uçtan uca çerçeve.");
}
