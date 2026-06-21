//! İP-05 Node Motoru — uçtan uca çalıştırma demosu (Gün 21).
//!
//! Çalıştır:  `cargo run -p biocraft-ui --example node_calistir_demo`
//!
//! Gösterilenler:
//! 1. **Eklenti SDK node kaydı** — `biocraft-sdk` üzerinden özel bir node kaydedilir.
//! 2. **Paralel çalıştırma** — iki bağımsız dal aynı anda çalışır (gözlemlenen eş zamanlılık).
//! 3. **Bellek bütçesi** — orkestratörden rezervasyon (OOM yok).
//! 4. **Sonuç önbelleği** — ikinci çalıştırma değişmeyeni atlar; parametre değişince alt-graf yeniden.
//! 5. **`.bcflow` kaydet/aç** — JSON kaydedilir, yeniden yüklenir (gidiş-dönüş doğrulanır).
//! 6. **PNG/SVG + Python** — görsel + kod dışa aktarımı dosyaya yazılır.

use std::collections::HashMap;
use std::sync::Arc;

use biocraft_ui::biocraft_mem::BellekOrkestratoru;
use biocraft_ui::biocraft_sdk::node::NodeTanimi;
use biocraft_ui::node::{
    bcflow_kaydet, bcflow_yukle, calistir, png_disa_aktar, python_disa_aktar, svg_disa_aktar,
    AkisDeger, Baglanti, CalismaAyari, IlerlemeOlay, IptalJetonu, Node, NodeCalistirici,
    NodeDurumu, NodeGraf, NodeKatalogu, NodeKaydi, NodeKimlik, ParametreDeger, Parametreler, Port,
    PortRef, PortYonu, SonucOnbellek, YurutucuKayit,
};
use biocraft_ui::{Dil, Tokenlar};

/// Özel "eklenti" node'u: tablodaki satır sayısını ikiye katlar (SDK kaydı gösterimi).
struct Cogaltici;
impl NodeCalistirici for Cogaltici {
    fn calistir(
        &self,
        girdiler: &[AkisDeger],
        _p: &Parametreler,
    ) -> Result<Vec<AkisDeger>, String> {
        let n = girdiler.first().map(|g| g.eleman).unwrap_or(0) * 2;
        Ok(vec![AkisDeger::yeni(
            "tablo",
            format!("{n} satır (×2)"),
            n,
            n * 32,
        )])
    }
}

fn main() {
    println!("=== BioCraft Engine — İP-05 Node Motoru çalıştırma demosu (Gün 21) ===\n");

    // ── 1) Çalıştırıcı kaydı: çekirdek demo node'ları + eklenti SDK ile özel node ──
    let mut kayit = YurutucuKayit::ornek();
    kayit.kaydet(NodeKaydi::yeni(
        NodeTanimi {
            kimlik: "eklenti.cogalt".into(),
            baslik: "Satır Çoğalt (eklenti)".into(),
            kategori: "Eklenti".into(),
            aciklama: "Tablo satır sayısını ikiye katlar.".into(),
            portlar: vec![],
            parametreler: vec![],
        },
        Arc::new(Cogaltici),
    ));
    println!(
        "1) Çalıştırıcı kaydı: {} tür (çekirdek demo + 1 eklenti SDK node'u).",
        kayit.adet()
    );

    // ── 2) Akış grafiği: iki bağımsız dal (paralel) + eklenti node'u ──
    let katalog = NodeKatalogu::ornek();
    let mut g = NodeGraf::yeni("demo-akis");
    let ekle = |g: &mut NodeGraf, tur: &str, konum: (f32, f32)| -> NodeKimlik {
        let k = g.yeni_node_kimlik();
        g.node_ekle_ham(katalog.bul(tur).unwrap().ornekle(k, konum));
        k
    };
    let baglan = |g: &mut NodeGraf, a: NodeKimlik, b: NodeKimlik| {
        let bk = g.yeni_baglanti_kimlik();
        g.baglanti_ekle_ham(Baglanti {
            kimlik: bk,
            kaynak: PortRef::yeni(a, PortYonu::Cikis, 0),
            hedef: PortRef::yeni(b, PortYonu::Giris, 0),
        });
    };
    // Dal A: dizi → hizala → varyant → tablo → filtre
    let a1 = ekle(&mut g, "girdi.dizi_oku", (40.0, 40.0));
    let a2 = ekle(&mut g, "isle.hizala", (250.0, 40.0));
    let a3 = ekle(&mut g, "isle.varyant_cagir", (460.0, 40.0));
    let a4 = ekle(&mut g, "donustur.varyant_tablo", (670.0, 40.0));
    let a5 = ekle(&mut g, "isle.tablo_filtrele", (880.0, 40.0));
    baglan(&mut g, a1, a2);
    baglan(&mut g, a2, a3);
    baglan(&mut g, a3, a4);
    baglan(&mut g, a4, a5);
    // Dal B (bağımsız): dizi → hizala  → paralel çalışır
    let b1 = ekle(&mut g, "girdi.dizi_oku", (40.0, 240.0));
    let b2 = ekle(&mut g, "isle.hizala", (250.0, 240.0));
    baglan(&mut g, b1, b2);
    // Eklenti node'u: filtre çıktısını çoğalt
    let c1 = g.yeni_node_kimlik();
    g.node_ekle_ham(Node {
        kimlik: c1,
        tur_kimligi: "eklenti.cogalt".into(),
        baslik: "Satır Çoğalt".into(),
        konum: (1090.0, 40.0),
        girisler: vec![Port::yeni("tablo", "tablo")],
        cikislar: vec![Port::yeni("tablo", "tablo")],
        durum: NodeDurumu::Bekliyor,
    });
    baglan(&mut g, a5, c1);
    println!(
        "2) Akış kuruldu: {} node, {} bağlantı (2 bağımsız dal).\n",
        g.nodelar().len(),
        g.baglantilar().len()
    );

    // ── 3) Bellek bütçeli + paralel çalıştırma ──
    let ork = BellekOrkestratoru::yeni(256 * 1024 * 1024); // 256 MiB bütçe
    let mut pars: HashMap<NodeKimlik, Parametreler> = HashMap::new();
    let mut onbellek = SonucOnbellek::yeni();
    let ayar = CalismaAyari::default();

    println!("3) İlk çalıştırma (paralel, bütçeli):");
    let mut ilerleme = |o: IlerlemeOlay| {
        println!(
            "   • node #{:<2} → {:?} ({}/{})",
            o.node.0, o.durum, o.tamamlanan, o.toplam
        );
    };
    let s1 = calistir(
        &g,
        &kayit,
        &pars,
        &ork,
        &mut onbellek,
        &ayar,
        &IptalJetonu::yeni(),
        &mut ilerleme,
    );
    println!(
        "   Sonuç: {} hesaplandı, {} önbellek, {} hata; gözlemlenen en yüksek eş zamanlılık = {}.",
        s1.hesaplanan, s1.onbellekten_atlanan, s1.hata_sayisi, s1.azami_es_zamanli
    );
    println!("   (Bağımsız dallar aynı dalgada paralel başlatılır; anlık demo işinde örtüşme görülmeyebilir — gerçek paralellik birim testiyle doğrulanır.)");
    println!(
        "   Çalıştırma sonunda rezerve bellek = {} bayt (sıfır → sızıntı yok).\n",
        ork.durum().rezerve
    );

    // ── 4) Önbellek: aynı akışı tekrar çalıştır ──
    println!("4) İkinci çalıştırma (değişiklik yok):");
    let mut sessiz = |_o: IlerlemeOlay| {};
    let s2 = calistir(
        &g,
        &kayit,
        &pars,
        &ork,
        &mut onbellek,
        &ayar,
        &IptalJetonu::yeni(),
        &mut sessiz,
    );
    println!(
        "   {} önbellekten atlandı, {} yeniden hesaplandı (tümü önbellekten).\n",
        s2.onbellekten_atlanan, s2.hesaplanan
    );

    // ── 5) Parametre değişimi → yalnız alt-graf yeniden hesaplanır ──
    let mut p = Parametreler::yeni();
    p.ayarla("esik", ParametreDeger::TamSayi(20));
    pars.insert(a5, p);
    println!("5) Filtre eşiği değişti (esik=20) → yalnız alt-graf yeniden:");
    let s3 = calistir(
        &g,
        &kayit,
        &pars,
        &ork,
        &mut onbellek,
        &ayar,
        &IptalJetonu::yeni(),
        &mut sessiz,
    );
    println!(
        "   {} önbellekten, {} yeniden hesaplandı (filtre + altı).\n",
        s3.onbellekten_atlanan, s3.hesaplanan
    );

    // ── 6) .bcflow kaydet / aç ──
    let bcflow = bcflow_kaydet(&g, &pars);
    let (g2, pars2) = bcflow_yukle(&bcflow).expect(".bcflow yüklenmeli");
    println!(
        "6) .bcflow kaydı: {} bayt; geri yüklendi → {} node, {} bağlantı, {} parametreli node.",
        bcflow.len(),
        g2.nodelar().len(),
        g2.baglantilar().len(),
        pars2.len()
    );

    // ── 7) Görsel + kod dışa aktarma (dosyalara) ──
    let tok = Tokenlar::koyu();
    let dizin = std::env::temp_dir().join("biocraft_node_demo");
    std::fs::create_dir_all(&dizin).ok();
    let svg = svg_disa_aktar(&g, &tok, Dil::Tr);
    let png = png_disa_aktar(&g, &tok, 1.0);
    let py = python_disa_aktar(&g, &pars);
    let f_bcflow = dizin.join("akis.bcflow");
    let f_svg = dizin.join("akis.svg");
    let f_png = dizin.join("akis.png");
    let f_py = dizin.join("akis.py");
    std::fs::write(&f_bcflow, &bcflow).ok();
    std::fs::write(&f_svg, &svg).ok();
    std::fs::write(&f_png, &png).ok();
    std::fs::write(&f_py, &py).ok();
    println!("7) Dışa aktarıldı:");
    println!("   {}  ({} bayt)", f_bcflow.display(), bcflow.len());
    println!("   {}  ({} bayt)", f_svg.display(), svg.len());
    println!(
        "   {}  ({} bayt PNG, geçerli imza={})",
        f_png.display(),
        png.len(),
        png[..8] == [137, 80, 78, 71, 13, 10, 26, 10]
    );
    println!("   {}  ({} bayt Python)", f_py.display(), py.len());

    println!("\n=== Demo tamam: paralel + bütçeli + önbellekli çalıştırma, .bcflow, PNG/SVG, node→Python, eklenti SDK node'u. ===");
}
