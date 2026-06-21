//! Node motoru uçtan-uca davranış testleri (İP-05 kabul kriterleri — 1. kısım).
//!
//! Bu testler **modeli + undo/redo motorunu** doğrular; egui çizimi ayrıca headless test edilir
//! (`crate::tests` altındaki tuval testleri).  Kabul kriterleri:
//! - Node ekleme/bağlama/taşıma/silme + undo/redo çalışır.
//! - Tipsiz/döngüsel bağlantı engellenir; uyumlu portlar bağlanır.

use biocraft_state::GeriAlYigini;

use super::commands::{BaglantiEkleKomut, NodeEkleKomut, NodeSilKomut, NodeTasiKomut};
use super::dag::{dongu_var_mi, topolojik_sira};
use super::graph::{Baglanti, BaglantiKontrol, Node, NodeDurumu, NodeGraf, NodeKimlik, PortRef};
use super::katalog::{ornek_donusturucu_kayit, NodeKatalogu};
use super::port::{Port, PortYonu, VeriTuru};

/// Katalogdan bir node örnekler (id atayarak).
fn katalogtan(graf: &mut NodeGraf, katalog: &NodeKatalogu, tur: &str, konum: (f32, f32)) -> Node {
    let k = graf.yeni_node_kimlik();
    katalog.bul(tur).unwrap().ornekle(k, konum)
}

#[test]
fn uctan_uca_akis_kurulur_ve_geri_alinir() {
    // Senaryo: kullanıcı 3 node ekler, ikisini bağlar, taşır; her şey geri alınabilir.
    let katalog = NodeKatalogu::ornek();
    let mut g = NodeGraf::yeni("ana");
    let mut y: GeriAlYigini<NodeGraf> = GeriAlYigini::yeni();

    let oku = katalogtan(&mut g, &katalog, "girdi.dizi_oku", (0.0, 0.0));
    let oku_k = oku.kimlik;
    y.calistir(&mut g, Box::new(NodeEkleKomut::yeni("ana", oku)))
        .unwrap();
    let hizala = katalogtan(&mut g, &katalog, "isle.hizala", (200.0, 0.0));
    let hizala_k = hizala.kimlik;
    y.calistir(&mut g, Box::new(NodeEkleKomut::yeni("ana", hizala)))
        .unwrap();
    assert_eq!(g.nodelar().len(), 2);

    // Bağla (dizi → dizi uyumlu).
    let kaynak = PortRef::yeni(oku_k, PortYonu::Cikis, 0);
    let hedef = PortRef::yeni(hizala_k, PortYonu::Giris, 0);
    assert_eq!(
        g.baglanti_kontrol(kaynak, hedef, None),
        BaglantiKontrol::Uygun
    );
    let bk = g.yeni_baglanti_kimlik();
    y.calistir(
        &mut g,
        Box::new(BaglantiEkleKomut::yeni(
            "ana",
            Baglanti {
                kimlik: bk,
                kaynak,
                hedef,
            },
        )),
    )
    .unwrap();
    assert_eq!(g.baglantilar().len(), 1);

    // Taşı.
    y.calistir(
        &mut g,
        Box::new(NodeTasiKomut::yeni(
            "ana",
            hizala_k,
            (200.0, 0.0),
            (260.0, 40.0),
        )),
    )
    .unwrap();
    assert_eq!(g.node(hizala_k).unwrap().konum, (260.0, 40.0));

    // Tümünü geri al → boş grafik.
    while y.geri_al(&mut g).unwrap() {}
    assert_eq!(g.nodelar().len(), 0);
    assert_eq!(g.baglantilar().len(), 0);

    // Tümünü yinele → akış aynen geri gelir.
    while y.yinele(&mut g).unwrap() {}
    assert_eq!(g.nodelar().len(), 2);
    assert_eq!(g.baglantilar().len(), 1);
    assert_eq!(g.node(hizala_k).unwrap().konum, (260.0, 40.0));
}

#[test]
fn uyumsuz_baglanti_engellenir_donusturucu_onerilir() {
    let katalog = NodeKatalogu::ornek();
    let donusturucu = ornek_donusturucu_kayit();
    let mut g = NodeGraf::yeni("ana");
    let varyant = katalogtan(&mut g, &katalog, "isle.varyant_cagir", (0.0, 0.0));
    let vk = varyant.kimlik;
    g.node_ekle_ham(varyant);
    // "Özet İstatistik" girişi tablo bekler; varyant çıkışı uyumsuz.
    let ozet = katalogtan(&mut g, &katalog, "cikti.ozet", (200.0, 0.0));
    let ok = ozet.kimlik;
    g.node_ekle_ham(ozet);

    let k = g.baglanti_kontrol(
        PortRef::yeni(vk, PortYonu::Cikis, 0),
        PortRef::yeni(ok, PortYonu::Giris, 0),
        Some(&donusturucu),
    );
    match k {
        BaglantiKontrol::TipUyumsuz { donusturucu, .. } => {
            assert_eq!(donusturucu.as_deref(), Some("donustur.varyant_tablo"));
        }
        _ => panic!("varyant → tablo uyumsuz + dönüştürücü önerisi beklenir"),
    }
}

#[test]
fn dongusel_baglanti_dag_korur() {
    let mut g = NodeGraf::yeni("ana");
    // dizi→dizi geçişli 3 node zinciri.
    for k in 1..=3u64 {
        g.node_ekle_ham(Node {
            kimlik: NodeKimlik(k),
            tur_kimligi: "n".into(),
            baslik: format!("n{k}"),
            konum: (0.0, 0.0),
            girisler: vec![Port::yeni("g", "dizi")],
            cikislar: vec![Port::yeni("c", "dizi")],
            durum: NodeDurumu::Bekliyor,
        });
    }
    let baglan = |g: &mut NodeGraf, a: u64, b: u64| {
        let bk = g.yeni_baglanti_kimlik();
        g.baglanti_ekle_ham(Baglanti {
            kimlik: bk,
            kaynak: PortRef::yeni(NodeKimlik(a), PortYonu::Cikis, 0),
            hedef: PortRef::yeni(NodeKimlik(b), PortYonu::Giris, 0),
        });
    };
    baglan(&mut g, 1, 2);
    baglan(&mut g, 2, 3);
    assert!(!dongu_var_mi(&g));
    // 3 → 1 döngü yapar → kontrol reddetmeli (graf temiz kalır).
    assert_eq!(
        g.baglanti_kontrol(
            PortRef::yeni(NodeKimlik(3), PortYonu::Cikis, 0),
            PortRef::yeni(NodeKimlik(1), PortYonu::Giris, 0),
            None,
        ),
        BaglantiKontrol::DonguOlur
    );
    // Reddedildiği için graf hâlâ DAG.
    assert!(topolojik_sira(&g).is_some());
}

#[test]
fn node_silinince_baglantilar_da_gider_ve_geri_yuklenir() {
    let katalog = NodeKatalogu::ornek();
    let mut g = NodeGraf::yeni("ana");
    let a = katalogtan(&mut g, &katalog, "girdi.dizi_oku", (0.0, 0.0));
    let ak = a.kimlik;
    g.node_ekle_ham(a);
    let b = katalogtan(&mut g, &katalog, "isle.hizala", (200.0, 0.0));
    let bk_node = b.kimlik;
    g.node_ekle_ham(b);
    let bk = g.yeni_baglanti_kimlik();
    g.baglanti_ekle_ham(Baglanti {
        kimlik: bk,
        kaynak: PortRef::yeni(ak, PortYonu::Cikis, 0),
        hedef: PortRef::yeni(bk_node, PortYonu::Giris, 0),
    });

    let mut y: GeriAlYigini<NodeGraf> = GeriAlYigini::yeni();
    let komut = NodeSilKomut::yeni(&g, ak).unwrap();
    y.calistir(&mut g, Box::new(komut)).unwrap();
    assert_eq!(g.nodelar().len(), 1);
    assert_eq!(g.baglantilar().len(), 0, "node ile kenar da silinir");

    y.geri_al(&mut g).unwrap();
    assert_eq!(g.nodelar().len(), 2);
    assert_eq!(g.baglantilar().len(), 1, "geri-al kenarı da geri yükler");
}

#[test]
fn joker_port_her_turle_baglanir() {
    let mut g = NodeGraf::yeni("ana");
    g.node_ekle_ham(Node {
        kimlik: NodeKimlik(1),
        tur_kimligi: "kaynak".into(),
        baslik: "kaynak".into(),
        konum: (0.0, 0.0),
        girisler: vec![],
        cikislar: vec![Port::yeni("c", "varyant")],
        durum: NodeDurumu::Bekliyor,
    });
    g.node_ekle_ham(Node {
        kimlik: NodeKimlik(2),
        tur_kimligi: "genel".into(),
        baslik: "genel".into(),
        konum: (200.0, 0.0),
        girisler: vec![Port {
            ad: "girdi".into(),
            veri_turu: VeriTuru::her(),
        }],
        cikislar: vec![],
        durum: NodeDurumu::Bekliyor,
    });
    // Joker giriş varyant çıkışını kabul eder.
    assert_eq!(
        g.baglanti_kontrol(
            PortRef::yeni(NodeKimlik(1), PortYonu::Cikis, 0),
            PortRef::yeni(NodeKimlik(2), PortYonu::Giris, 0),
            None,
        ),
        BaglantiKontrol::Uygun
    );
}
