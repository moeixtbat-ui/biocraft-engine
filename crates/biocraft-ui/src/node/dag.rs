//! DAG (yönlü, döngüsüz grafik) kısıtı — döngü oluşturulamaz (İP-05).
//!
//! Node akışı bir **DAG** olmalıdır: veri çıkıştan girişe akar, geriye dönemez.  Bir bağlantı
//! eklenmeden **önce** döngü oluşup oluşmayacağı denetlenir; oluşacaksa bağlantı reddedilir
//! (anlık görsel uyarı — [`super::graph::NodeGraf::baglanti_kontrol`]).
//!
//! Kasti olarak `petgraph` gibi bir dış bağımlılık eklenmez: grafikler küçük-orta ölçeklidir ve
//! basit bir DFS/Kahn yeterlidir (bağımsız, denetlenebilir; proje "az bağımlılık" ilkesine uyar).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::graph::{NodeGraf, NodeKimlik};

/// Kenar yönünde (çıkış → giriş) `baslangic` node'undan `hedef` node'una bir yol var mı?
///
/// Bağlantı eklerken döngü denetimi için kullanılır: `out(A) → in(B)` kenarı, **B'den A'ya zaten
/// bir yol varsa** döngü oluşturur.  Kendi kendine kenar (A == B) de döngüdür.
pub fn yol_var_mi(graf: &NodeGraf, baslangic: NodeKimlik, hedef: NodeKimlik) -> bool {
    if baslangic == hedef {
        return true;
    }
    let komsu = komsuluk(graf);
    let mut ziyaret: BTreeSet<NodeKimlik> = BTreeSet::new();
    let mut yigin: Vec<NodeKimlik> = vec![baslangic];
    while let Some(d) = yigin.pop() {
        if !ziyaret.insert(d) {
            continue;
        }
        if let Some(sonrakiler) = komsu.get(&d) {
            for &s in sonrakiler {
                if s == hedef {
                    return true;
                }
                yigin.push(s);
            }
        }
    }
    false
}

/// Grafiğin döngü içerip içermediği (topolojik sıralanabilir mi?).
pub fn dongu_var_mi(graf: &NodeGraf) -> bool {
    topolojik_sira(graf).is_none()
}

/// Kahn algoritmasıyla topolojik sıralama; **döngü varsa `None`**.
///
/// Gün 21 çalıştırma motoru bunu kullanır: node'lar bağımlılık sırasında çalışır, bağımsız dallar
/// paralelleştirilebilir.  Bugün yalnızca DAG geçerliliğini doğrulamak + test için.
pub fn topolojik_sira(graf: &NodeGraf) -> Option<Vec<NodeKimlik>> {
    let komsu = komsuluk(graf);
    // Giriş derecesi (kaç kenar bu node'a giriyor).
    let mut derece: BTreeMap<NodeKimlik, usize> =
        graf.nodelar().iter().map(|n| (n.kimlik, 0usize)).collect();
    for sonrakiler in komsu.values() {
        for &s in sonrakiler {
            *derece.entry(s).or_insert(0) += 1;
        }
    }
    // Derecesi 0 olan node'lar kuyruğa.
    let mut kuyruk: VecDeque<NodeKimlik> = derece
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(&k, _)| k)
        .collect();
    let mut sira: Vec<NodeKimlik> = Vec::with_capacity(graf.nodelar().len());
    while let Some(n) = kuyruk.pop_front() {
        sira.push(n);
        if let Some(sonrakiler) = komsu.get(&n) {
            for &s in sonrakiler {
                let d = derece.get_mut(&s).expect("komşu derecede olmalı");
                *d -= 1;
                if *d == 0 {
                    kuyruk.push_back(s);
                }
            }
        }
    }
    // Tüm node'lar sıralandıysa DAG; aksi halde döngü var.
    if sira.len() == graf.nodelar().len() {
        Some(sira)
    } else {
        None
    }
}

/// Komşuluk listesi: node → kendisinden (çıkışından) çıkan kenarların hedef node'ları.
fn komsuluk(graf: &NodeGraf) -> BTreeMap<NodeKimlik, Vec<NodeKimlik>> {
    let mut komsu: BTreeMap<NodeKimlik, Vec<NodeKimlik>> = BTreeMap::new();
    for n in graf.nodelar() {
        komsu.entry(n.kimlik).or_default();
    }
    for b in graf.baglantilar() {
        komsu.entry(b.kaynak.node).or_default().push(b.hedef.node);
    }
    komsu
}

#[cfg(test)]
mod tests {
    use super::super::graph::{Baglanti, BaglantiKontrol, Node, NodeDurumu, PortRef};
    use super::super::port::{Port, PortYonu};
    use super::*;

    /// dizi→dizi geçişli basit node (zincir kurmak için).
    fn dugum(k: u64) -> Node {
        Node {
            kimlik: NodeKimlik(k),
            tur_kimligi: "n".into(),
            baslik: "n".into(),
            konum: (0.0, 0.0),
            girisler: vec![Port::yeni("g", "dizi")],
            cikislar: vec![Port::yeni("c", "dizi")],
            durum: NodeDurumu::Bekliyor,
        }
    }

    fn baglan(g: &mut NodeGraf, a: u64, b: u64) {
        let bk = g.yeni_baglanti_kimlik();
        g.baglanti_ekle_ham(Baglanti {
            kimlik: bk,
            kaynak: PortRef::yeni(NodeKimlik(a), PortYonu::Cikis, 0),
            hedef: PortRef::yeni(NodeKimlik(b), PortYonu::Giris, 0),
        });
    }

    #[test]
    fn zincirde_yol_bulunur() {
        let mut g = NodeGraf::yeni("ana");
        for k in 1..=3 {
            g.node_ekle_ham(dugum(k));
        }
        baglan(&mut g, 1, 2);
        baglan(&mut g, 2, 3);
        assert!(
            yol_var_mi(&g, NodeKimlik(1), NodeKimlik(3)),
            "1→2→3 yolu var"
        );
        assert!(
            !yol_var_mi(&g, NodeKimlik(3), NodeKimlik(1)),
            "ters yön yok"
        );
    }

    #[test]
    fn dongu_olusturan_baglanti_reddedilir() {
        let mut g = NodeGraf::yeni("ana");
        for k in 1..=3 {
            g.node_ekle_ham(dugum(k));
        }
        baglan(&mut g, 1, 2);
        baglan(&mut g, 2, 3);
        // 3 → 1 eklemek döngü yapar; baglanti_kontrol bunu yakalamalı.
        let k = g.baglanti_kontrol(
            PortRef::yeni(NodeKimlik(3), PortYonu::Cikis, 0),
            PortRef::yeni(NodeKimlik(1), PortYonu::Giris, 0),
            None,
        );
        assert_eq!(k, BaglantiKontrol::DonguOlur);
    }

    #[test]
    fn topolojik_sira_dag_de_var_donguda_yok() {
        let mut g = NodeGraf::yeni("ana");
        for k in 1..=3 {
            g.node_ekle_ham(dugum(k));
        }
        baglan(&mut g, 1, 2);
        baglan(&mut g, 2, 3);
        let sira = topolojik_sira(&g).expect("DAG topolojik sıralanmalı");
        // 1, 2'den; 2, 3'ten önce gelmeli.
        let poz = |k: u64| sira.iter().position(|&n| n == NodeKimlik(k)).unwrap();
        assert!(poz(1) < poz(2) && poz(2) < poz(3));
        assert!(!dongu_var_mi(&g));

        // Ham döngü kur (kontrolü atlayarak) → topolojik sıra None.
        baglan(&mut g, 3, 1);
        assert!(topolojik_sira(&g).is_none());
        assert!(dongu_var_mi(&g));
    }
}
