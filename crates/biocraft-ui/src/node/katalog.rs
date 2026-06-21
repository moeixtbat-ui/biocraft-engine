//! Node kataloğu — palette/sürükle ile eklenebilir node türlerinin kaydı (İP-05).
//!
//! Çekirdek motor yalnızca **temel** node türlerini bilir; gerçek bilim node'larının çoğu
//! eklentilerden gelir (Gün 21 SDK kaydı — `biocraft-sdk`).  Bu katalog, sağ-tık paletinin ve
//! aranabilir node listesinin içeriğidir; her girdi bir [`NodeTanimi`] zenginleştirmesidir.
// MK-54: node'ların çoğu eklentilerden; burada motorun bildiği temel/demo türler.

use super::graph::{Node, NodeDurumu, NodeKimlik};
use super::port::{Donusturucu, DonusturucuKayit, Port, VeriTuru};

/// Paletteki tek bir node türü tanımı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeKatalogGirdisi {
    /// Node türünün kimliği (`donustur.varyant_tablo` vb.).
    pub tur_kimligi: String,
    /// Kullanıcıya görünen başlık.
    pub baslik: String,
    /// Palette gruplama için kategori ("Girdi", "İşle", "Çıktı", "Dönüştür").
    pub kategori: String,
    /// Kısa açıklama (palette ipucu).
    pub aciklama: String,
    /// Giriş portları.
    pub girisler: Vec<Port>,
    /// Çıkış portları.
    pub cikislar: Vec<Port>,
}

impl NodeKatalogGirdisi {
    /// Bu türden, verilen kimlik + konumla yeni bir node örneği üretir.
    pub fn ornekle(&self, kimlik: NodeKimlik, konum: (f32, f32)) -> Node {
        Node {
            kimlik,
            tur_kimligi: self.tur_kimligi.clone(),
            baslik: self.baslik.clone(),
            konum,
            girisler: self.girisler.clone(),
            cikislar: self.cikislar.clone(),
            durum: NodeDurumu::Bekliyor,
        }
    }
}

/// Eklenebilir node türlerinin kataloğu (palette içeriği + arama).
#[derive(Debug, Clone, Default)]
pub struct NodeKatalogu {
    girdiler: Vec<NodeKatalogGirdisi>,
}

impl NodeKatalogu {
    /// Boş katalog.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir tür ekler.
    pub fn ekle(&mut self, girdi: NodeKatalogGirdisi) {
        self.girdiler.push(girdi);
    }

    /// Tüm girdiler (salt-okunur).
    pub fn girdiler(&self) -> &[NodeKatalogGirdisi] {
        &self.girdiler
    }

    /// Tür kimliğinden girdi bulur.
    pub fn bul(&self, tur_kimligi: &str) -> Option<&NodeKatalogGirdisi> {
        self.girdiler.iter().find(|g| g.tur_kimligi == tur_kimligi)
    }

    /// Arama: başlık/kategori/açıklama içinde (harf-duyarsız) `sorgu` geçenleri döndürür.
    /// Boş sorgu → tüm girdiler.
    pub fn ara(&self, sorgu: &str) -> Vec<&NodeKatalogGirdisi> {
        let s = sorgu.trim().to_lowercase();
        if s.is_empty() {
            return self.girdiler.iter().collect();
        }
        self.girdiler
            .iter()
            .filter(|g| {
                g.baslik.to_lowercase().contains(&s)
                    || g.kategori.to_lowercase().contains(&s)
                    || g.aciklama.to_lowercase().contains(&s)
                    || g.tur_kimligi.to_lowercase().contains(&s)
            })
            .collect()
    }

    /// Çekirdek/demo node türleri (genomik akış örneği) — palet ilk açıldığında dolu olsun.
    pub fn ornek() -> Self {
        let mut k = Self::yeni();
        k.ekle(NodeKatalogGirdisi {
            tur_kimligi: "girdi.dizi_oku".into(),
            baslik: "Dizi Oku (FASTA)".into(),
            kategori: "Girdi".into(),
            aciklama: "Bir FASTA dosyasından dizileri okur.".into(),
            girisler: vec![],
            cikislar: vec![Port::yeni("diziler", "dizi")],
        });
        k.ekle(NodeKatalogGirdisi {
            tur_kimligi: "isle.hizala".into(),
            baslik: "Hizala".into(),
            kategori: "İşle".into(),
            aciklama: "Dizileri referansa hizalar.".into(),
            girisler: vec![Port::yeni("diziler", "dizi")],
            cikislar: vec![Port::yeni("hizalama", "hizalama")],
        });
        k.ekle(NodeKatalogGirdisi {
            tur_kimligi: "isle.varyant_cagir".into(),
            baslik: "Varyant Çağır".into(),
            kategori: "İşle".into(),
            aciklama: "Hizalamadan varyantları çağırır.".into(),
            girisler: vec![Port::yeni("hizalama", "hizalama")],
            cikislar: vec![Port::yeni("varyantlar", "varyant")],
        });
        k.ekle(NodeKatalogGirdisi {
            tur_kimligi: "donustur.varyant_tablo".into(),
            baslik: "Varyant → Tablo".into(),
            kategori: "Dönüştür".into(),
            aciklama: "Varyantları tabloya dönüştürür (köprü).".into(),
            girisler: vec![Port::yeni("varyantlar", "varyant")],
            cikislar: vec![Port::yeni("tablo", "tablo")],
        });
        k.ekle(NodeKatalogGirdisi {
            tur_kimligi: "isle.tablo_filtrele".into(),
            baslik: "Tablo Filtrele".into(),
            kategori: "İşle".into(),
            aciklama: "Tabloyu ölçüte göre süzer.".into(),
            girisler: vec![Port::yeni("tablo", "tablo")],
            cikislar: vec![Port::yeni("tablo", "tablo")],
        });
        k.ekle(NodeKatalogGirdisi {
            tur_kimligi: "cikti.ozet".into(),
            baslik: "Özet İstatistik".into(),
            kategori: "Çıktı".into(),
            aciklama: "Tablodan özet metin üretir.".into(),
            girisler: vec![Port::yeni("tablo", "tablo")],
            cikislar: vec![Port::yeni("ozet", "metin")],
        });
        k.ekle(NodeKatalogGirdisi {
            tur_kimligi: "cikti.uc_boyut".into(),
            baslik: "3B Görüntüle".into(),
            kategori: "Çıktı".into(),
            aciklama: "Bir 3B yapıyı sahnede gösterir.".into(),
            girisler: vec![Port::yeni("yapi", "3b_yapi")],
            cikislar: vec![],
        });
        k
    }
}

/// Örnek katalogla tutarlı dönüştürücü kaydı (otomatik öneri için).
pub fn ornek_donusturucu_kayit() -> DonusturucuKayit {
    let mut d = DonusturucuKayit::yeni();
    d.ekle(Donusturucu {
        kaynak: VeriTuru::yeni("varyant"),
        hedef: VeriTuru::yeni("tablo"),
        node_tur_kimligi: "donustur.varyant_tablo".into(),
        baslik: "Varyant → Tablo".into(),
    });
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ornek_katalog_dolu_ve_aranabilir() {
        let k = NodeKatalogu::ornek();
        assert!(k.girdiler().len() >= 5);
        // Boş sorgu hepsini döndürür.
        assert_eq!(k.ara("").len(), k.girdiler().len());
        // "hizala" araması Hizala node'unu bulur.
        let sonuc = k.ara("hizala");
        assert!(sonuc.iter().any(|g| g.tur_kimligi == "isle.hizala"));
        // Kategoriden de aranır.
        assert!(!k.ara("girdi").is_empty());
    }

    #[test]
    fn girdiden_node_ornekle() {
        let k = NodeKatalogu::ornek();
        let g = k.bul("isle.hizala").unwrap();
        let n = g.ornekle(NodeKimlik(7), (100.0, 40.0));
        assert_eq!(n.kimlik, NodeKimlik(7));
        assert_eq!(n.konum, (100.0, 40.0));
        assert_eq!(n.girisler.len(), 1);
        assert_eq!(n.cikislar.len(), 1);
        assert_eq!(n.tur_kimligi, "isle.hizala");
    }

    #[test]
    fn ornek_donusturucu_varyant_tablo() {
        let d = ornek_donusturucu_kayit();
        assert!(d
            .oner(&VeriTuru::yeni("varyant"), &VeriTuru::yeni("tablo"))
            .is_some());
    }
}
