//! Geri-alınabilir node düzenleme komutları — undo/redo entegrasyonu (İP-05 ↔ İP-11, MK-36/37).
//!
//! Node ekleme/bağlama/taşıma/silme + not işlemleri, Gün 10'da kurulan **genel** [`Komut`] motoru
//! ([`biocraft_state::GeriAlYigini`]) üzerinden geri alınabilir.  Her komut **tek mantıksal depoya**
//! (bu node grafiği) dokunur (MK-37) → `depo()` grafiğin kimliğini taşır.
//!
//! ## Kararlı "yinele" (redo) güvencesi
//! Komutlar **önceden ayrılmış kimlikler** (node/bağlantı/not) ile kurulur.  Böylece bir işlem geri
//! alınıp yeniden uygulandığında (yinele) **aynı kimlik** geri gelir; sonraki komutların referansları
//! (ör. "şu node'a bağlan") kopmaz.  Silme komutları, geri-yükleme için silinen **tam veriyi** saklar.

use biocraft_state::command::{DepoKimligi, Komut};
use biocraft_types::ErrorReport;

use super::graph::{Baglanti, BaglantiKimlik, Node, NodeGraf, NodeKimlik, NotKimlik, YapiskanNot};

/// Bu node grafiğinin undo deposu kimliğini üretir (MK-37 tek-depo sınırı).
fn depo_kimligi(graf_kimlik: &str) -> DepoKimligi {
    DepoKimligi::yeni(format!("node-graf:{graf_kimlik}"))
}

/// Hedef node grafiğin yokluğunda dönecek standart hata (program hatası — olmamalı).
fn yok_hatasi(ne: &str) -> ErrorReport {
    ErrorReport::new(
        "Node işlemi uygulanamadı",
        format!("Beklenen öğe ({ne}) grafikte bulunamadı."),
        "İşlemi tekrar deneyin; sorun sürerse akışı yeniden açın.",
    )
}

// ─── Node ekle ────────────────────────────────────────────────────────────────

/// Önceden kimliği atanmış bir node'u ekler (geri-al: kaldırır).
pub struct NodeEkleKomut {
    depo: String,
    node: Node,
}

impl NodeEkleKomut {
    /// Kimliği zaten ayrılmış bir node için ekleme komutu kurar.
    pub fn yeni(graf_kimlik: &str, node: Node) -> Self {
        Self {
            depo: graf_kimlik.to_string(),
            node,
        }
    }
}

impl Komut<NodeGraf> for NodeEkleKomut {
    fn uygula(&mut self, hedef: &mut NodeGraf) -> Result<(), ErrorReport> {
        hedef.node_ekle_ham(self.node.clone());
        Ok(())
    }
    fn geri_al(&mut self, hedef: &mut NodeGraf) -> Result<(), ErrorReport> {
        hedef
            .node_kaldir(self.node.kimlik)
            .map(|_| ())
            .ok_or_else(|| yok_hatasi("node"))
    }
    fn aciklama(&self) -> String {
        format!("Node ekle: {}", self.node.baslik)
    }
    fn depo(&self) -> DepoKimligi {
        depo_kimligi(&self.depo)
    }
}

// ─── Node sil ─────────────────────────────────────────────────────────────────

/// Bir node'u (ve ona bağlı bağlantıları) siler (geri-al: node + bağlantıları geri yükler).
pub struct NodeSilKomut {
    depo: String,
    node: Node,
    baglantilar: Vec<Baglanti>,
}

impl NodeSilKomut {
    /// Grafikten okuyarak silme komutu kurar (node + bağlı kenarları yakalar).  Node yoksa `None`.
    pub fn yeni(graf: &NodeGraf, kimlik: NodeKimlik) -> Option<Self> {
        let node = graf.node(kimlik)?.clone();
        let baglantilar: Vec<Baglanti> = graf
            .baglantilar()
            .iter()
            .filter(|b| b.kaynak.node == kimlik || b.hedef.node == kimlik)
            .cloned()
            .collect();
        Some(Self {
            depo: graf.kimlik.clone(),
            node,
            baglantilar,
        })
    }
}

impl Komut<NodeGraf> for NodeSilKomut {
    fn uygula(&mut self, hedef: &mut NodeGraf) -> Result<(), ErrorReport> {
        hedef
            .node_kaldir(self.node.kimlik)
            .map(|_| ())
            .ok_or_else(|| yok_hatasi("node"))
    }
    fn geri_al(&mut self, hedef: &mut NodeGraf) -> Result<(), ErrorReport> {
        hedef.node_ekle_ham(self.node.clone());
        for b in &self.baglantilar {
            hedef.baglanti_ekle_ham(b.clone());
        }
        Ok(())
    }
    fn aciklama(&self) -> String {
        format!("Node sil: {}", self.node.baslik)
    }
    fn depo(&self) -> DepoKimligi {
        depo_kimligi(&self.depo)
    }
}

// ─── Node taşı ──────────────────────────────────────────────────────────────

/// Bir node'u taşır (simetrik: eski + yeni konum saklı → yinele kararlı).
pub struct NodeTasiKomut {
    depo: String,
    kimlik: NodeKimlik,
    eski: (f32, f32),
    yeni: (f32, f32),
}

impl NodeTasiKomut {
    /// Eski ve yeni konumla taşıma komutu kurar.
    pub fn yeni(graf_kimlik: &str, kimlik: NodeKimlik, eski: (f32, f32), yeni: (f32, f32)) -> Self {
        Self {
            depo: graf_kimlik.to_string(),
            kimlik,
            eski,
            yeni,
        }
    }
}

impl Komut<NodeGraf> for NodeTasiKomut {
    fn uygula(&mut self, hedef: &mut NodeGraf) -> Result<(), ErrorReport> {
        hedef
            .node_tasi(self.kimlik, self.yeni)
            .map(|_| ())
            .ok_or_else(|| yok_hatasi("node"))
    }
    fn geri_al(&mut self, hedef: &mut NodeGraf) -> Result<(), ErrorReport> {
        hedef
            .node_tasi(self.kimlik, self.eski)
            .map(|_| ())
            .ok_or_else(|| yok_hatasi("node"))
    }
    fn aciklama(&self) -> String {
        "Node taşı".to_string()
    }
    fn depo(&self) -> DepoKimligi {
        depo_kimligi(&self.depo)
    }
}

// ─── Bağlantı ekle ──────────────────────────────────────────────────────────

/// Önceden kimliği atanmış bir bağlantıyı ekler (geri-al: kaldırır).  Geçerlilik çağırana ait.
pub struct BaglantiEkleKomut {
    depo: String,
    baglanti: Baglanti,
}

impl BaglantiEkleKomut {
    /// Kimliği ayrılmış geçerli bir bağlantı için ekleme komutu kurar.
    pub fn yeni(graf_kimlik: &str, baglanti: Baglanti) -> Self {
        Self {
            depo: graf_kimlik.to_string(),
            baglanti,
        }
    }
}

impl Komut<NodeGraf> for BaglantiEkleKomut {
    fn uygula(&mut self, hedef: &mut NodeGraf) -> Result<(), ErrorReport> {
        hedef.baglanti_ekle_ham(self.baglanti.clone());
        Ok(())
    }
    fn geri_al(&mut self, hedef: &mut NodeGraf) -> Result<(), ErrorReport> {
        hedef
            .baglanti_kaldir(self.baglanti.kimlik)
            .map(|_| ())
            .ok_or_else(|| yok_hatasi("bağlantı"))
    }
    fn aciklama(&self) -> String {
        "Bağlantı ekle".to_string()
    }
    fn depo(&self) -> DepoKimligi {
        depo_kimligi(&self.depo)
    }
}

// ─── Bağlantı sil ───────────────────────────────────────────────────────────

/// Bir bağlantıyı siler (geri-al: geri yükler).
pub struct BaglantiSilKomut {
    depo: String,
    baglanti: Baglanti,
}

impl BaglantiSilKomut {
    /// Grafikten okuyarak silme komutu kurar.  Bağlantı yoksa `None`.
    pub fn yeni(graf: &NodeGraf, kimlik: BaglantiKimlik) -> Option<Self> {
        let baglanti = graf
            .baglantilar()
            .iter()
            .find(|b| b.kimlik == kimlik)?
            .clone();
        Some(Self {
            depo: graf.kimlik.clone(),
            baglanti,
        })
    }
}

impl Komut<NodeGraf> for BaglantiSilKomut {
    fn uygula(&mut self, hedef: &mut NodeGraf) -> Result<(), ErrorReport> {
        hedef
            .baglanti_kaldir(self.baglanti.kimlik)
            .map(|_| ())
            .ok_or_else(|| yok_hatasi("bağlantı"))
    }
    fn geri_al(&mut self, hedef: &mut NodeGraf) -> Result<(), ErrorReport> {
        hedef.baglanti_ekle_ham(self.baglanti.clone());
        Ok(())
    }
    fn aciklama(&self) -> String {
        "Bağlantı sil".to_string()
    }
    fn depo(&self) -> DepoKimligi {
        depo_kimligi(&self.depo)
    }
}

// ─── Yapışkan not ekle / sil / taşı ─────────────────────────────────────────

/// Bir yapışkan not ekler (geri-al: kaldırır).
pub struct NotEkleKomut {
    depo: String,
    not: YapiskanNot,
}

impl NotEkleKomut {
    /// Kimliği ayrılmış bir not için ekleme komutu kurar.
    pub fn yeni(graf_kimlik: &str, not: YapiskanNot) -> Self {
        Self {
            depo: graf_kimlik.to_string(),
            not,
        }
    }
}

impl Komut<NodeGraf> for NotEkleKomut {
    fn uygula(&mut self, hedef: &mut NodeGraf) -> Result<(), ErrorReport> {
        hedef.not_ekle_ham(self.not.clone());
        Ok(())
    }
    fn geri_al(&mut self, hedef: &mut NodeGraf) -> Result<(), ErrorReport> {
        hedef
            .not_kaldir(self.not.kimlik)
            .map(|_| ())
            .ok_or_else(|| yok_hatasi("not"))
    }
    fn aciklama(&self) -> String {
        "Not ekle".to_string()
    }
    fn depo(&self) -> DepoKimligi {
        depo_kimligi(&self.depo)
    }
}

/// Bir yapışkan notu siler (geri-al: geri yükler).
pub struct NotSilKomut {
    depo: String,
    not: YapiskanNot,
}

impl NotSilKomut {
    /// Grafikten okuyarak silme komutu kurar.  Not yoksa `None`.
    pub fn yeni(graf: &NodeGraf, kimlik: NotKimlik) -> Option<Self> {
        let not = graf.notlar().iter().find(|n| n.kimlik == kimlik)?.clone();
        Some(Self {
            depo: graf.kimlik.clone(),
            not,
        })
    }
}

impl Komut<NodeGraf> for NotSilKomut {
    fn uygula(&mut self, hedef: &mut NodeGraf) -> Result<(), ErrorReport> {
        hedef
            .not_kaldir(self.not.kimlik)
            .map(|_| ())
            .ok_or_else(|| yok_hatasi("not"))
    }
    fn geri_al(&mut self, hedef: &mut NodeGraf) -> Result<(), ErrorReport> {
        hedef.not_ekle_ham(self.not.clone());
        Ok(())
    }
    fn aciklama(&self) -> String {
        "Not sil".to_string()
    }
    fn depo(&self) -> DepoKimligi {
        depo_kimligi(&self.depo)
    }
}

#[cfg(test)]
mod tests {
    use super::super::graph::{NodeDurumu, PortRef};
    use super::super::port::{Port, PortYonu};
    use super::*;
    use biocraft_state::GeriAlYigini;

    fn node(k: u64, cikis: bool, giris: bool) -> Node {
        Node {
            kimlik: NodeKimlik(k),
            tur_kimligi: "n".into(),
            baslik: format!("n{k}"),
            konum: (10.0, 10.0),
            girisler: if giris {
                vec![Port::yeni("g", "dizi")]
            } else {
                vec![]
            },
            cikislar: if cikis {
                vec![Port::yeni("c", "dizi")]
            } else {
                vec![]
            },
            durum: NodeDurumu::Bekliyor,
        }
    }

    #[test]
    fn ekle_geri_al_yinele() {
        let mut g = NodeGraf::yeni("ana");
        let mut y = GeriAlYigini::yeni();
        let k = g.yeni_node_kimlik();
        let n = node(k.0, true, false);
        y.calistir(&mut g, Box::new(NodeEkleKomut::yeni("ana", n)))
            .unwrap();
        assert_eq!(g.nodelar().len(), 1);
        // Geri al → boş.
        y.geri_al(&mut g).unwrap();
        assert_eq!(g.nodelar().len(), 0);
        // Yinele → aynı kimlikle geri gelir.
        y.yinele(&mut g).unwrap();
        assert_eq!(g.nodelar().len(), 1);
        assert_eq!(g.nodelar()[0].kimlik, k, "yinele kararlı kimlik");
    }

    #[test]
    fn node_sil_baglantilari_da_geri_yukler() {
        let mut g = NodeGraf::yeni("ana");
        g.node_ekle_ham(node(1, true, false));
        g.node_ekle_ham(node(2, false, true));
        let bk = g.yeni_baglanti_kimlik();
        g.baglanti_ekle_ham(Baglanti {
            kimlik: bk,
            kaynak: PortRef::yeni(NodeKimlik(1), PortYonu::Cikis, 0),
            hedef: PortRef::yeni(NodeKimlik(2), PortYonu::Giris, 0),
        });
        let mut y = GeriAlYigini::yeni();
        let komut = NodeSilKomut::yeni(&g, NodeKimlik(2)).unwrap();
        y.calistir(&mut g, Box::new(komut)).unwrap();
        assert_eq!(g.nodelar().len(), 1);
        assert_eq!(g.baglantilar().len(), 0, "node ile kenar da gitti");
        // Geri al → node + kenar geri gelir.
        y.geri_al(&mut g).unwrap();
        assert_eq!(g.nodelar().len(), 2);
        assert_eq!(g.baglantilar().len(), 1, "kenar geri yüklendi");
    }

    #[test]
    fn tasi_geri_al_eski_konuma_doner() {
        let mut g = NodeGraf::yeni("ana");
        g.node_ekle_ham(node(1, true, false));
        let mut y = GeriAlYigini::yeni();
        let komut = NodeTasiKomut::yeni("ana", NodeKimlik(1), (10.0, 10.0), (200.0, 50.0));
        y.calistir(&mut g, Box::new(komut)).unwrap();
        assert_eq!(g.node(NodeKimlik(1)).unwrap().konum, (200.0, 50.0));
        y.geri_al(&mut g).unwrap();
        assert_eq!(g.node(NodeKimlik(1)).unwrap().konum, (10.0, 10.0));
    }

    #[test]
    fn baglanti_ekle_sil_geri_alinir() {
        let mut g = NodeGraf::yeni("ana");
        g.node_ekle_ham(node(1, true, false));
        g.node_ekle_ham(node(2, false, true));
        let mut y = GeriAlYigini::yeni();
        let bk = g.yeni_baglanti_kimlik();
        let b = Baglanti {
            kimlik: bk,
            kaynak: PortRef::yeni(NodeKimlik(1), PortYonu::Cikis, 0),
            hedef: PortRef::yeni(NodeKimlik(2), PortYonu::Giris, 0),
        };
        y.calistir(&mut g, Box::new(BaglantiEkleKomut::yeni("ana", b)))
            .unwrap();
        assert_eq!(g.baglantilar().len(), 1);
        y.geri_al(&mut g).unwrap();
        assert_eq!(g.baglantilar().len(), 0);
        y.yinele(&mut g).unwrap();
        assert_eq!(g.baglantilar().len(), 1);
    }

    #[test]
    fn hepsi_ayni_depoya_dokunur() {
        // MK-37: tüm node komutları aynı mantıksal depoya dokunur.
        let n = node(1, true, false);
        let ekle = NodeEkleKomut::yeni("ana", n);
        let tasi = NodeTasiKomut::yeni("ana", NodeKimlik(1), (0.0, 0.0), (1.0, 1.0));
        assert_eq!(ekle.depo(), tasi.depo());
        assert_eq!(ekle.depo(), DepoKimligi::yeni("node-graf:ana"));
    }
}
