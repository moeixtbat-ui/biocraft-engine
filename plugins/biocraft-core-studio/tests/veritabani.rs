//! ÇE-09 (Gün 40) — **Veritabanı erişimi: birleşik arama + konektörler** entegrasyon testleri.
//!
//! Gerçek ağ kullanılmaz: [`SahteUlastirici`] kayıtlı sentetik NCBI/BLAST yanıtlarıyla konektörleri
//! **çevrimdışı** uçtan uca sürer (dürüst sınır; `data_io::remote` deseni).  Kontrol listesi:
//! * NCBI nucleotide araması sonuç döndürür; sonuç projeye (provenance ile) eklenebilir.
//! * BLAST işi gönderilir, durumu izlenir (Job/ilerleme), sonuç gelir.
//! * Birleşik panel kaynak rozetli sonuç gösterir; "tarayıcıda aç" çalışır.
//! * Dış sorgu öncesi onay + PHI engeli çalışır.
//! * Golden: birleşik sonuç tablosu (MK-58).

use std::time::Duration;

use biocraft_core_studio::db_search::{
    AramaBaglami, AramaSonucu, BirlesikPanel, BlastDurum, BlastKonektor, BlastProgram, Eylem,
    GizlilikKapisi, HassasiyetEtiketi, KayitTuru, NcbiKonektor, SahteUlastirici, Sayfalama, Sorgu,
    VeritabaniKonektoru, YoklamaAyari,
};
use biocraft_sdk::biocraft_types::{golden, IsKulpu, JobStatus};

// ─── Sentetik yanıtlar ───────────────────────────────────────────────────────────

fn esearch_json() -> &'static str {
    r#"{"header":{},"esearchresult":{"count":"3","retmax":"2","retstart":"0",
        "idlist":["7157","672"]}}"#
}
fn esummary_json() -> &'static str {
    r#"{"header":{},"result":{"uids":["7157","672"],
        "7157":{"uid":"7157","caption":"NM_000546","title":"Homo sapiens tumor protein p53 (TP53), mRNA","slen":2591,"organism":"Homo sapiens"},
        "672":{"uid":"672","caption":"NM_007294","title":"Homo sapiens BRCA1 DNA repair associated (BRCA1), mRNA","slen":7088,"organism":"Homo sapiens"}}}"#
}
fn efetch_fasta() -> &'static str {
    ">NM_000546.6 Homo sapiens tumor protein p53 (TP53), mRNA\nGATCCACGTGACGTACGT\n"
}
fn blast_put() -> &'static str {
    "<!--QBlastInfoBegin\n    RID = TEST_RID_001\n    RTOE = 12\nQBlastInfoEnd-->"
}
fn blast_bekliyor() -> &'static str {
    "<!--QBlastInfoBegin\n    Status=WAITING\nQBlastInfoEnd-->"
}
fn blast_hazir() -> &'static str {
    "<!--QBlastInfoBegin\n    Status=READY\nQBlastInfoEnd-->"
}
fn blast_tabular() -> &'static str {
    "# blastn\n# Query: q\n# Database: nt\n# 2 hits found\n\
     q\tNM_000546.6\t100.00\t18\t0\t0\t1\t18\t1\t18\t2e-05\t36.2\n\
     q\tXM_011544981.1\t94.40\t18\t1\t0\t1\t18\t9\t26\t3e-03\t28.3"
}

fn hizli_baglam<'a>(u: &'a SahteUlastirici, g: &'a GizlilikKapisi) -> AramaBaglami<'a> {
    let mut b = AramaBaglami::yeni(u, g);
    b.yapi.geri_cekilme_taban = Duration::ZERO; // testte tekrar gecikmesi yok
    b
}

// ─── 1) NCBI araması + projeye ekleme (provenance) ───────────────────────────────

#[test]
fn ncbi_arama_sonuc_ve_projeye_ekleme() {
    let u = SahteUlastirici::yeni()
        .ekle("esearch.fcgi", 200, esearch_json())
        .ekle("esummary.fcgi", 200, esummary_json())
        .ekle("efetch.fcgi", 200, efetch_fasta());
    let g = GizlilikKapisi::onayli();
    let baglam = hizli_baglam(&u, &g);
    let kon = NcbiKonektor::nukleotid();

    // Arama → sonuç listesi (toplam 3, bu sayfada 2).
    let liste = kon
        .ara(&Sorgu::metin("p53"), Sayfalama::ilk(2), &baglam)
        .unwrap();
    assert_eq!(liste.sayfa.toplam, 3);
    assert!(liste.sayfa.sonraki_var());
    assert_eq!(liste.sonuclar.len(), 2);
    assert_eq!(liste.sonuclar[0].kimlik, "7157");
    assert_eq!(liste.sonuclar[0].uzunluk, Some(2591));

    // Tek-tık yükleme: kaydı getir → içerik + provenance (kaynak/erişim tarihi/BLAKE3 + lisans).
    let kayit = kon.getir("7157", &baglam).unwrap();
    assert_eq!(kayit.format_ipucu, "fasta");
    assert!(kayit.icerik.starts_with(b">NM_000546"));
    let p = &kayit.provenans;
    assert!(p.kaynak.contains("NCBI nucleotide"));
    assert_eq!(p.blake3.len(), 64);
    assert!(p
        .lisans_atif
        .as_ref()
        .unwrap()
        .lisans
        .contains("Public Domain"));
}

// ─── 2) BLAST: gönder + durum izleme (Job/ilerleme) + sonuç ──────────────────────

#[test]
fn blast_gonder_durum_izle_sonuc() {
    // İlk yoklama BEKLİYOR, ikinci HAZIR → durum izleme akışı.
    let u = SahteUlastirici::yeni()
        .ekle("CMD=Put", 200, blast_put())
        .ekle("FORMAT_OBJECT=SearchInfo", 200, blast_hazir())
        .ekle("FORMAT_TYPE=Tabular", 200, blast_tabular());
    let g = GizlilikKapisi::onayli();
    let baglam = hizli_baglam(&u, &g);
    let kon = BlastKonektor::varsayilan(BlastProgram::Blastn).with_yoklama(YoklamaAyari {
        azami_yoklama: 5,
        aralik: Duration::ZERO,
    });

    // Düşük seviye: gönder → RID, durum yokla → HAZIR.
    let isi = kon
        .gonder("GATCCACGTG", HassasiyetEtiketi::Genel, &baglam)
        .unwrap();
    assert_eq!(isi.rid, "TEST_RID_001");
    assert_eq!(isi.tahmini_sure_sn, 12);
    assert_eq!(
        kon.durum_yokla(&isi.rid, &baglam).unwrap(),
        BlastDurum::Hazir
    );

    // Tam akış: Job kulpu ile ilerleme/iptal; sonuç hizalamaları.
    let is = IsKulpu::yeni("BLAST test", None);
    let hizalamalar = kon
        .blast_calistir("GATCCACGTG", HassasiyetEtiketi::Genel, &baglam, &is)
        .unwrap();
    assert_eq!(hizalamalar.len(), 2);
    assert_eq!(hizalamalar[0].konu_kimligi, "NM_000546.6");
    assert_eq!(hizalamalar[0].ozdeslik_yuzde, 100.0);
    assert_eq!(is.durum(), JobStatus::Bitti);
}

#[test]
fn blast_bekleyen_durum_izlenir() {
    let u = SahteUlastirici::yeni().ekle("FORMAT_OBJECT=SearchInfo", 200, blast_bekliyor());
    let g = GizlilikKapisi::onayli();
    let baglam = hizli_baglam(&u, &g);
    let kon = BlastKonektor::varsayilan(BlastProgram::Blastn);
    assert_eq!(kon.durum_yokla("R", &baglam).unwrap(), BlastDurum::Bekliyor);
}

// ─── 3) Birleşik panel: kaynak rozeti + "tarayıcıda aç" ──────────────────────────

#[test]
fn birlesik_panel_rozet_ve_tarayicida_ac() {
    let u = SahteUlastirici::yeni()
        .ekle("esearch.fcgi", 200, esearch_json())
        .ekle("esummary.fcgi", 200, esummary_json())
        .ekle("CMD=Put", 200, blast_put())
        .ekle("FORMAT_OBJECT=SearchInfo", 200, blast_hazir())
        .ekle("FORMAT_TYPE=Tabular", 200, blast_tabular());
    let g = GizlilikKapisi::onayli();
    let baglam = hizli_baglam(&u, &g);

    let mut panel = BirlesikPanel::yeni();
    panel.konektor_ekle(Box::new(NcbiKonektor::nukleotid()));
    panel.konektor_ekle(Box::new(
        BlastKonektor::varsayilan(BlastProgram::Blastn).with_yoklama(YoklamaAyari {
            azami_yoklama: 3,
            aralik: Duration::ZERO,
        }),
    ));
    panel.sorgu_metni = "TP53".to_string();

    panel.ara(Sayfalama::ilk(2), &baglam).unwrap();
    assert_eq!(panel.sonuclar().len(), 4); // 2 NCBI + 2 BLAST
    assert!(panel.son_hatalar().is_empty());

    // Kaynak rozetleri birleşik listede görünür.
    let rozetler: std::collections::BTreeSet<&str> =
        panel.sonuclar().iter().map(|s| s.kaynak.as_str()).collect();
    assert!(rozetler.contains("NCBI nucleotide"));
    assert!(rozetler.contains("BLAST blastn"));

    // "Tarayıcıda aç" eylemi NCBI sonucu için doğru URL üretir.
    let ncbi_sonuc = panel
        .sonuclar()
        .iter()
        .find(|s| s.kaynak == "NCBI nucleotide")
        .unwrap();
    let eylemler = panel.eylemler(ncbi_sonuc);
    assert!(eylemler.iter().any(|e| matches!(
        e,
        Eylem::TarayicidaAc(u) if u.contains("ncbi.nlm.nih.gov/nuccore/")
    )));
}

#[test]
fn yapi_sonucu_yapiya_bak_eylemi() {
    // PDB konektörü Gün 41'de gelecek; eylem yönlendirmesi bugün hazır (KayitTuru::Yapi → ÇE-07).
    let panel = BirlesikPanel::yeni();
    let yapi = AramaSonucu::yeni("PDB", "1TUP", "p53 core domain", KayitTuru::Yapi);
    let eylemler = panel.eylemler(&yapi);
    assert!(eylemler
        .iter()
        .any(|e| matches!(e, Eylem::YapiyaBak(k) if k == "1TUP")));
}

// ─── 4) Dış sorgu onayı + PHI engeli ─────────────────────────────────────────────

#[test]
fn onaysiz_dis_sorgu_gonderilmez() {
    // Onaylanmamış kapı → arama dış isteği yollamadan reddedilir (sessiz gönderim yok).
    let u = SahteUlastirici::yeni().ekle("esearch.fcgi", 200, esearch_json());
    let g = GizlilikKapisi::yeni(); // onaylanmadı
    let baglam = hizli_baglam(&u, &g);
    let hata = NcbiKonektor::nukleotid()
        .ara(&Sorgu::metin("p53"), Sayfalama::ilk(2), &baglam)
        .err()
        .unwrap();
    assert_eq!(hata.ne_oldu, "Dış sorgu onayı gerekli");
}

#[test]
fn phi_dizi_blast_a_gonderilemez() {
    // Onaylı kapı bile PHI dizisini dışarı çıkaramaz (MK-42/43, İP-10).
    let u = SahteUlastirici::yeni().ekle("CMD=Put", 200, blast_put());
    let g = GizlilikKapisi::onayli();
    let baglam = hizli_baglam(&u, &g);
    let hata = BlastKonektor::varsayilan(BlastProgram::Blastn)
        .gonder("HASTAYADIZISI", HassasiyetEtiketi::Phi, &baglam)
        .err()
        .unwrap();
    assert_eq!(hata.ne_oldu, "Hassas/PHI veri dış sorguya gönderilemez");
}

// ─── 5) Golden: birleşik sonuç tablosu ───────────────────────────────────────────

#[test]
fn golden_birlesik_sonuc_tablosu() {
    let u = SahteUlastirici::yeni()
        .ekle("esearch.fcgi", 200, esearch_json())
        .ekle("esummary.fcgi", 200, esummary_json());
    let g = GizlilikKapisi::onayli();
    let baglam = hizli_baglam(&u, &g);

    let mut panel = BirlesikPanel::yeni();
    panel.konektor_ekle(Box::new(NcbiKonektor::nukleotid()));
    panel.sorgu_metni = "p53".to_string();
    panel.ara(Sayfalama::ilk(2), &baglam).unwrap();

    let mut metin = String::from("KAYNAK | KİMLİK | TÜR | UZUNLUK | ORGANİZMA | BAŞLIK\n");
    for s in panel.sonuclar() {
        metin.push_str(&format!(
            "{} | {} | {} | {} | {} | {}\n",
            s.kaynak,
            s.kimlik,
            s.tur.etiket(),
            s.uzunluk
                .map(|u| u.to_string())
                .unwrap_or_else(|| "-".into()),
            s.organizma.as_deref().unwrap_or("-"),
            s.baslik,
        ));
    }
    golden::dogrula("ce09_birlesik_sonuc", &metin);
}
