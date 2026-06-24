//! ÇE-09 (Gün 40) — **Veritabanı erişimi demo** (çevrimdışı; sahte ulaştırıcı).
//!
//! Gerçek ağ yığını bu sürümde bağlı değildir (dürüst sınır); demo, [`SahteUlastirici`] ile
//! sentetik NCBI/BLAST yanıtları kullanarak çerçeveyi uçtan uca gösterir:
//! 1) gizlilik/onay sınırı, 2) NCBI araması + tek-tık getirme (provenance), 3) BLAST işi (Job),
//! 4) birleşik panel (kaynak rozeti + eylemler), 5) PHI engeli.
//!
//! Çalıştır: `cargo run -p biocraft-core-studio --example veritabani_demo`

use std::time::Duration;

use biocraft_core_studio::db_search::{
    AramaBaglami, BirlesikPanel, BlastKonektor, BlastProgram, DisVeri, Eylem, GizlilikKapisi,
    HassasiyetEtiketi, NcbiKonektor, SahteUlastirici, Sayfalama, Sorgu, VeritabaniKonektoru,
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

    println!("\n✔ ÇE-09 (1. kısım) demo tamam — gerçek ağ olmadan uçtan uca çerçeve.");
}
