//! Node grafiği — saf, egui'siz veri modeli (İP-05).
//!
//! Bu modül node tabanlı akışın **veri çekirdeğidir**: node'lar, tipli portlar, bağlantılar ve
//! yapışkan notlar.  egui'den bağımsızdır → birim testlenir.  Çizim [`super::canvas`]'tadır;
//! geri-alınabilir düzenlemeler [`super::commands`]'tadır.
//!
//! ## Değişmezler (her zaman korunur)
//! - **Yön:** Bağlantı yalnızca bir **çıkıştan** bir **girişe** kurulur ([`PortYonu`]).
//! - **Giriş tekildir:** Bir giriş portunun en fazla **bir** bağlantısı olur (fan-in yok); çıkış
//!   çok node'a dağılabilir (fan-out — `MK-54`).
//! - **Tip uyumu:** Yalnızca [`VeriTuru::uyumlu_mu`] olan portlar bağlanır.
//! - **DAG:** Döngü oluşturan bağlantı reddedilir (bkz. [`super::dag`]).
// MK-54: node motoru çekirdeği; node'ların çoğu eklentilerden gelir, motor temelde.

use super::port::{Port, PortYonu, VeriTuru};

/// Bir node örneğinin oturum-içi benzersiz kimliği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeKimlik(pub u64);

/// Bir bağlantının oturum-içi benzersiz kimliği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BaglantiKimlik(pub u64);

/// Bir yapışkan notun oturum-içi benzersiz kimliği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NotKimlik(pub u64);

/// Bir node'un çalıştırma durumu — her node'da görsel durum halkası (İP-05 "Durum").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeDurumu {
    /// Henüz çalıştırılmadı / girdisi bekliyor.
    #[default]
    Bekliyor,
    /// Şu an çalışıyor (Gün 21 paralel çalıştırma).
    Calisiyor,
    /// Başarıyla tamamlandı.
    Bitti,
    /// Hata ile durdu (bu dal durur, bağımsız dallar devam eder).
    Hata,
}

/// Bir grafikteki tek node örneği.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Benzersiz kimlik.
    pub kimlik: NodeKimlik,
    /// Node türünün katalog kimliği (`biocraft.<yayinci>.<node>` veya çekirdek demo).
    pub tur_kimligi: String,
    /// Kullanıcıya görünen başlık.
    pub baslik: String,
    /// Tuval (mantıksal) koordinatında sol-üst köşe konumu.
    pub konum: (f32, f32),
    /// Giriş portları (sırası anlamlıdır).
    pub girisler: Vec<Port>,
    /// Çıkış portları (sırası anlamlıdır).
    pub cikislar: Vec<Port>,
    /// Çalıştırma durumu (durum halkası).
    pub durum: NodeDurumu,
}

impl Node {
    /// Verilen yöndeki portu indeksiyle döndürür.
    pub fn port(&self, yon: PortYonu, indeks: usize) -> Option<&Port> {
        match yon {
            PortYonu::Giris => self.girisler.get(indeks),
            PortYonu::Cikis => self.cikislar.get(indeks),
        }
    }

    /// Verilen yöndeki port sayısı.
    pub fn port_sayisi(&self, yon: PortYonu) -> usize {
        match yon {
            PortYonu::Giris => self.girisler.len(),
            PortYonu::Cikis => self.cikislar.len(),
        }
    }
}

/// Bir portu kesin konumlandıran referans (node + yön + indeks).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortRef {
    /// Hangi node.
    pub node: NodeKimlik,
    /// Giriş mi çıkış mı.
    pub yon: PortYonu,
    /// O yöndeki port dizini.
    pub indeks: usize,
}

impl PortRef {
    /// Kısa kurucu.
    pub fn yeni(node: NodeKimlik, yon: PortYonu, indeks: usize) -> Self {
        Self { node, yon, indeks }
    }
}

/// İki port arasındaki bağlantı (kaynak çıkış → hedef giriş).
#[derive(Debug, Clone, PartialEq)]
pub struct Baglanti {
    /// Benzersiz kimlik.
    pub kimlik: BaglantiKimlik,
    /// Kaynak uç (mutlaka [`PortYonu::Cikis`]).
    pub kaynak: PortRef,
    /// Hedef uç (mutlaka [`PortYonu::Giris`]).
    pub hedef: PortRef,
}

/// Tuvale serbest yerleştirilen açıklama notu (sticky note / etiket — İP-05 "Tuval").
#[derive(Debug, Clone, PartialEq)]
pub struct YapiskanNot {
    /// Benzersiz kimlik.
    pub kimlik: NotKimlik,
    /// Not metni (kullanıcı düzenler).
    pub metin: String,
    /// Tuval (mantıksal) koordinatında sol-üst köşe.
    pub konum: (f32, f32),
}

/// Bir bağlantı kurma denemesinin sonucu — neden uygun/uygun değil (anlık görsel geri bildirim).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaglantiKontrol {
    /// Bağlanabilir.
    Uygun,
    /// Yön hatası (çıkış→giriş değil; örn. giriş→giriş ya da çıkış→çıkış).
    YonHatasi,
    /// Bir node kendine bağlanamaz.
    AyniNode,
    /// Tür uyumsuz; varsa önerilen dönüştürücü node türü kimliği taşınır.
    TipUyumsuz {
        /// Kaynak portun türü.
        kaynak: VeriTuru,
        /// Hedef portun türü.
        hedef: VeriTuru,
        /// Otomatik dönüştürücü önerisi (varsa).
        donusturucu: Option<String>,
    },
    /// Hedef giriş portu zaten dolu (giriş tekildir).
    GirisDolu,
    /// Aynı bağlantı zaten var.
    ZatenVar,
    /// Döngü oluşturur (DAG ihlali).
    DonguOlur,
    /// Belirtilen port bulunamadı (geçersiz referans).
    PortYok,
}

impl BaglantiKontrol {
    /// Bağlantı kurulabilir mi?
    pub fn uygun_mu(&self) -> bool {
        matches!(self, BaglantiKontrol::Uygun)
    }
}

/// Node tabanlı akışın veri grafiği (tek mantıksal depo — MK-37).
#[derive(Debug, Clone)]
pub struct NodeGraf {
    /// Bu grafiğin mantıksal depo kimliği (undo/redo MK-37 sınırı; örn. "ana").
    pub kimlik: String,
    nodelar: Vec<Node>,
    baglantilar: Vec<Baglanti>,
    notlar: Vec<YapiskanNot>,
    sonraki_node: u64,
    sonraki_baglanti: u64,
    sonraki_not: u64,
}

impl Default for NodeGraf {
    fn default() -> Self {
        Self::yeni("ana")
    }
}

impl NodeGraf {
    /// Boş grafik.
    pub fn yeni(kimlik: impl Into<String>) -> Self {
        Self {
            kimlik: kimlik.into(),
            nodelar: Vec::new(),
            baglantilar: Vec::new(),
            notlar: Vec::new(),
            sonraki_node: 1,
            sonraki_baglanti: 1,
            sonraki_not: 1,
        }
    }

    // ── Salt-okunur erişim ─────────────────────────────────────────────────

    /// Tüm node'lar.
    pub fn nodelar(&self) -> &[Node] {
        &self.nodelar
    }

    /// Tüm bağlantılar.
    pub fn baglantilar(&self) -> &[Baglanti] {
        &self.baglantilar
    }

    /// Tüm yapışkan notlar.
    pub fn notlar(&self) -> &[YapiskanNot] {
        &self.notlar
    }

    /// Kimlikten node (salt-okunur).
    pub fn node(&self, kimlik: NodeKimlik) -> Option<&Node> {
        self.nodelar.iter().find(|n| n.kimlik == kimlik)
    }

    /// Kimlikten node (değiştirilebilir).
    pub fn node_mut(&mut self, kimlik: NodeKimlik) -> Option<&mut Node> {
        self.nodelar.iter_mut().find(|n| n.kimlik == kimlik)
    }

    /// Kimlikten not (değiştirilebilir).
    pub fn not_mut(&mut self, kimlik: NotKimlik) -> Option<&mut YapiskanNot> {
        self.notlar.iter_mut().find(|n| n.kimlik == kimlik)
    }

    /// Bir port referansının gösterdiği portu döndürür (geçersizse `None`).
    pub fn port_coz(&self, r: PortRef) -> Option<&Port> {
        self.node(r.node).and_then(|n| n.port(r.yon, r.indeks))
    }

    /// Bir giriş portunun zaten bir bağlantısı var mı? (Giriş tekildir.)
    pub fn giris_dolu_mu(&self, hedef: PortRef) -> bool {
        self.baglantilar.iter().any(|b| b.hedef == hedef)
    }

    // ── Kimlik ayırma ──────────────────────────────────────────────────────

    /// Yeni bir node kimliği ayırır (komutlar kararlı kimlik için önceden ayırır).
    pub fn yeni_node_kimlik(&mut self) -> NodeKimlik {
        let k = NodeKimlik(self.sonraki_node);
        self.sonraki_node += 1;
        k
    }

    /// Yeni bir bağlantı kimliği ayırır.
    pub fn yeni_baglanti_kimlik(&mut self) -> BaglantiKimlik {
        let k = BaglantiKimlik(self.sonraki_baglanti);
        self.sonraki_baglanti += 1;
        k
    }

    /// Yeni bir not kimliği ayırır.
    pub fn yeni_not_kimlik(&mut self) -> NotKimlik {
        let k = NotKimlik(self.sonraki_not);
        self.sonraki_not += 1;
        k
    }

    // ── Ham mutasyonlar (komutlar tarafından kullanılır) ───────────────────
    //
    // Bu "ham" işlemler doğrulama YAPMAZ; çağıran (komut) geçerliliği önceden garanti eder.
    // Kullanıcı-yüzeyli doğrulama [`baglanti_kontrol`] + [`super::canvas`]'tadır.

    /// Bir node'u doğrudan ekler (kimlik zaten atanmış olmalı).
    pub fn node_ekle_ham(&mut self, node: Node) {
        // Kimlik sayacını ileri al (yeniden kullanılmasın).
        if node.kimlik.0 >= self.sonraki_node {
            self.sonraki_node = node.kimlik.0 + 1;
        }
        self.nodelar.push(node);
    }

    /// Bir node'u (ve ona bağlı tüm bağlantıları) kaldırır; kaldırılan node + bağlantıları döner.
    ///
    /// Geri-al için kullanılır: dönen değer node'u **tam** geri yüklemeye yeter.
    pub fn node_kaldir(&mut self, kimlik: NodeKimlik) -> Option<(Node, Vec<Baglanti>)> {
        let idx = self.nodelar.iter().position(|n| n.kimlik == kimlik)?;
        let node = self.nodelar.remove(idx);
        // Bu node'a değen bağlantıları topla + kaldır.
        let (kalan, dusen): (Vec<_>, Vec<_>) = self
            .baglantilar
            .drain(..)
            .partition(|b| b.kaynak.node != kimlik && b.hedef.node != kimlik);
        self.baglantilar = kalan;
        Some((node, dusen))
    }

    /// Bir bağlantıyı doğrudan ekler (kimlik atanmış; doğrulama çağırana ait).
    pub fn baglanti_ekle_ham(&mut self, baglanti: Baglanti) {
        if baglanti.kimlik.0 >= self.sonraki_baglanti {
            self.sonraki_baglanti = baglanti.kimlik.0 + 1;
        }
        self.baglantilar.push(baglanti);
    }

    /// Bir bağlantıyı kaldırır; kaldırılan bağlantıyı döner.
    pub fn baglanti_kaldir(&mut self, kimlik: BaglantiKimlik) -> Option<Baglanti> {
        let idx = self.baglantilar.iter().position(|b| b.kimlik == kimlik)?;
        Some(self.baglantilar.remove(idx))
    }

    /// Bir yapışkan notu doğrudan ekler.
    pub fn not_ekle_ham(&mut self, not: YapiskanNot) {
        if not.kimlik.0 >= self.sonraki_not {
            self.sonraki_not = not.kimlik.0 + 1;
        }
        self.notlar.push(not);
    }

    /// Bir yapışkan notu kaldırır; kaldırılanı döner.
    pub fn not_kaldir(&mut self, kimlik: NotKimlik) -> Option<YapiskanNot> {
        let idx = self.notlar.iter().position(|n| n.kimlik == kimlik)?;
        Some(self.notlar.remove(idx))
    }

    /// Bir node'u yeni konuma taşır; eski konumu döner (geri-al için).  Node yoksa `None`.
    pub fn node_tasi(&mut self, kimlik: NodeKimlik, konum: (f32, f32)) -> Option<(f32, f32)> {
        let n = self.node_mut(kimlik)?;
        let eski = n.konum;
        n.konum = konum;
        Some(eski)
    }

    /// Bir notu yeni konuma taşır; eski konumu döner.  Not yoksa `None`.
    pub fn not_tasi(&mut self, kimlik: NotKimlik, konum: (f32, f32)) -> Option<(f32, f32)> {
        let n = self.not_mut(kimlik)?;
        let eski = n.konum;
        n.konum = konum;
        Some(eski)
    }

    /// Bir node'un durumunu ayarlar (durum halkası — çalıştırma motoru Gün 21).
    pub fn durum_ayarla(&mut self, kimlik: NodeKimlik, durum: NodeDurumu) {
        if let Some(n) = self.node_mut(kimlik) {
            n.durum = durum;
        }
    }

    // ── Bağlantı doğrulama (kullanıcı-yüzeyli) ─────────────────────────────

    /// `kaynak` (çıkış) → `hedef` (giriş) bağlantısının **kurulabilir olup olmadığını** denetler.
    ///
    /// Tuval bunu hem **sürükleme sırasında** (uyumlu portları vurgulamak) hem de **bırakırken**
    /// (geçersizse reddetmek + anlık uyarı) kullanır.  `donusturucu` kaydı verilirse, tip uyumsuz
    /// olduğunda otomatik dönüştürücü önerisi doldurulur.
    pub fn baglanti_kontrol(
        &self,
        kaynak: PortRef,
        hedef: PortRef,
        donusturucu: Option<&super::port::DonusturucuKayit>,
    ) -> BaglantiKontrol {
        // Yön: kaynak çıkış, hedef giriş olmalı.
        if kaynak.yon != PortYonu::Cikis || hedef.yon != PortYonu::Giris {
            return BaglantiKontrol::YonHatasi;
        }
        // Aynı node'a (kendine) bağlanamaz.
        if kaynak.node == hedef.node {
            return BaglantiKontrol::AyniNode;
        }
        // Portlar var mı?
        let (Some(p_kaynak), Some(p_hedef)) = (self.port_coz(kaynak), self.port_coz(hedef)) else {
            return BaglantiKontrol::PortYok;
        };
        // Tip uyumu.
        if !p_kaynak.veri_turu.uyumlu_mu(&p_hedef.veri_turu) {
            let oneri = donusturucu
                .and_then(|d| d.oner(&p_kaynak.veri_turu, &p_hedef.veri_turu))
                .map(|d| d.node_tur_kimligi.clone());
            return BaglantiKontrol::TipUyumsuz {
                kaynak: p_kaynak.veri_turu.clone(),
                hedef: p_hedef.veri_turu.clone(),
                donusturucu: oneri,
            };
        }
        // Aynı bağlantı zaten var mı?
        if self
            .baglantilar
            .iter()
            .any(|b| b.kaynak == kaynak && b.hedef == hedef)
        {
            return BaglantiKontrol::ZatenVar;
        }
        // Giriş tekildir.
        if self.giris_dolu_mu(hedef) {
            return BaglantiKontrol::GirisDolu;
        }
        // Döngü oluşturur mu? (hedef node zaten kaynak node'a ulaşıyorsa, yeni kenar döngü yapar.)
        if super::dag::yol_var_mi(self, hedef.node, kaynak.node) {
            return BaglantiKontrol::DonguOlur;
        }
        BaglantiKontrol::Uygun
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test için tek girişli + tek çıkışlı bir node üretir.
    fn test_node(kimlik: u64, tur_kimligi: &str, giris: &str, cikis: &str) -> Node {
        Node {
            kimlik: NodeKimlik(kimlik),
            tur_kimligi: tur_kimligi.into(),
            baslik: tur_kimligi.into(),
            konum: (0.0, 0.0),
            girisler: if giris.is_empty() {
                vec![]
            } else {
                vec![Port::yeni("girdi", giris)]
            },
            cikislar: if cikis.is_empty() {
                vec![]
            } else {
                vec![Port::yeni("sonuc", cikis)]
            },
            durum: NodeDurumu::Bekliyor,
        }
    }

    #[test]
    fn node_ekle_kaldir_baglanti_dusurur() {
        let mut g = NodeGraf::yeni("ana");
        g.node_ekle_ham(test_node(1, "oku", "", "dizi"));
        g.node_ekle_ham(test_node(2, "hizala", "dizi", "hizalama"));
        let bk = g.yeni_baglanti_kimlik();
        g.baglanti_ekle_ham(Baglanti {
            kimlik: bk,
            kaynak: PortRef::yeni(NodeKimlik(1), PortYonu::Cikis, 0),
            hedef: PortRef::yeni(NodeKimlik(2), PortYonu::Giris, 0),
        });
        assert_eq!(g.baglantilar().len(), 1);
        // Node 1 kaldırılınca bağlı kenar da düşer; kaldırılanlar döner.
        let (node, dusen) = g.node_kaldir(NodeKimlik(1)).unwrap();
        assert_eq!(node.kimlik, NodeKimlik(1));
        assert_eq!(dusen.len(), 1, "bağlı kenar node ile birlikte kaldırılmalı");
        assert_eq!(g.baglantilar().len(), 0);
    }

    #[test]
    fn uyumsuz_tip_baglanmaz_uyumlu_baglanir() {
        let mut g = NodeGraf::yeni("ana");
        g.node_ekle_ham(test_node(1, "oku", "", "dizi"));
        g.node_ekle_ham(test_node(2, "tablo", "tablo", "tablo"));
        // dizi → tablo: uyumsuz.
        let k = g.baglanti_kontrol(
            PortRef::yeni(NodeKimlik(1), PortYonu::Cikis, 0),
            PortRef::yeni(NodeKimlik(2), PortYonu::Giris, 0),
            None,
        );
        assert!(matches!(k, BaglantiKontrol::TipUyumsuz { .. }));

        // dizi → dizi: uyumlu.
        g.node_ekle_ham(test_node(3, "hizala", "dizi", "hizalama"));
        let k2 = g.baglanti_kontrol(
            PortRef::yeni(NodeKimlik(1), PortYonu::Cikis, 0),
            PortRef::yeni(NodeKimlik(3), PortYonu::Giris, 0),
            None,
        );
        assert_eq!(k2, BaglantiKontrol::Uygun);
    }

    #[test]
    fn yon_hatasi_ve_ayni_node() {
        let mut g = NodeGraf::yeni("ana");
        g.node_ekle_ham(test_node(1, "n", "dizi", "dizi"));
        // çıkış→çıkış = yön hatası.
        let k = g.baglanti_kontrol(
            PortRef::yeni(NodeKimlik(1), PortYonu::Cikis, 0),
            PortRef::yeni(NodeKimlik(1), PortYonu::Cikis, 0),
            None,
        );
        assert_eq!(k, BaglantiKontrol::YonHatasi);
        // çıkış→giriş ama aynı node = AyniNode.
        let k2 = g.baglanti_kontrol(
            PortRef::yeni(NodeKimlik(1), PortYonu::Cikis, 0),
            PortRef::yeni(NodeKimlik(1), PortYonu::Giris, 0),
            None,
        );
        assert_eq!(k2, BaglantiKontrol::AyniNode);
    }

    #[test]
    fn giris_tekildir() {
        let mut g = NodeGraf::yeni("ana");
        g.node_ekle_ham(test_node(1, "a", "", "dizi"));
        g.node_ekle_ham(test_node(2, "b", "", "dizi"));
        g.node_ekle_ham(test_node(3, "c", "dizi", ""));
        let bk = g.yeni_baglanti_kimlik();
        g.baglanti_ekle_ham(Baglanti {
            kimlik: bk,
            kaynak: PortRef::yeni(NodeKimlik(1), PortYonu::Cikis, 0),
            hedef: PortRef::yeni(NodeKimlik(3), PortYonu::Giris, 0),
        });
        // 3'ün girişi dolu → 2'den ikinci bağlantı reddedilir.
        let k = g.baglanti_kontrol(
            PortRef::yeni(NodeKimlik(2), PortYonu::Cikis, 0),
            PortRef::yeni(NodeKimlik(3), PortYonu::Giris, 0),
            None,
        );
        assert_eq!(k, BaglantiKontrol::GirisDolu);
    }

    #[test]
    fn donusturucu_onerisi_tasinir() {
        let mut g = NodeGraf::yeni("ana");
        g.node_ekle_ham(test_node(1, "a", "", "varyant"));
        g.node_ekle_ham(test_node(2, "b", "tablo", ""));
        let mut kayit = super::super::port::DonusturucuKayit::yeni();
        kayit.ekle(super::super::port::Donusturucu {
            kaynak: VeriTuru::yeni("varyant"),
            hedef: VeriTuru::yeni("tablo"),
            node_tur_kimligi: "donustur.varyant_tablo".into(),
            baslik: "Varyant → Tablo".into(),
        });
        let k = g.baglanti_kontrol(
            PortRef::yeni(NodeKimlik(1), PortYonu::Cikis, 0),
            PortRef::yeni(NodeKimlik(2), PortYonu::Giris, 0),
            Some(&kayit),
        );
        match k {
            BaglantiKontrol::TipUyumsuz { donusturucu, .. } => {
                assert_eq!(donusturucu.as_deref(), Some("donustur.varyant_tablo"));
            }
            _ => panic!("tip uyumsuz + dönüştürücü önerisi beklenir"),
        }
    }
}
