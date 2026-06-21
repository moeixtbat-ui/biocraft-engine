//! Sonuç önbelleği — değişmeyen node'u yeniden hesaplamadan atlama (İP-05 "Çalıştırma").
//!
//! Her node için son başarılı çalıştırmanın **imzası** + **çıktıları** saklanır.  Yeni bir
//! çalıştırmada motor her node'un güncel imzasını hesaplar ([`super::run`]):
//!
//! > `imza(node) = tür_kimliği + parametre imzası + tüm girdi (yukarı-akış çıktı) imzaları`
//!
//! İmza **aynıysa** node yeniden hesaplanmaz; önbellekteki çıktı kullanılır.  Bir node'un
//! parametresi **veya** bir yukarı-akış çıktısı değişirse imzası değişir → o node yeniden
//! hesaplanır; çıktısı da değişeceği için **alt-graf boyunca** geçersizleşme kendiliğinden
//! yayılır (bayat sonuç olmaz — "Muhtemel Hata: önbellek bayat sonuç" çözümü).
//!
//! Bağımsız (imzası değişmeyen) dallar dokunulmadan kalır → yalnızca **değişen alt-graf**
//! yeniden hesaplanır.

use std::collections::HashMap;

use biocraft_sdk::node::AkisDeger;

use super::graph::NodeKimlik;

/// Tek bir node'un önbellek kaydı.
#[derive(Debug, Clone)]
struct OnbellekGirdisi {
    /// Bu çıktıyı üreten girdi+parametre kümesinin imzası.
    imza: u64,
    /// Saklanan çıktı değerleri (çıkış portu sırasında).
    ciktilar: Vec<AkisDeger>,
}

/// Node çalıştırma sonuçlarının önbelleği (çalıştırmalar arası kalıcı; tuvalde tutulur).
#[derive(Debug, Clone, Default)]
pub struct SonucOnbellek {
    girdiler: HashMap<NodeKimlik, OnbellekGirdisi>,
}

impl SonucOnbellek {
    /// Boş önbellek.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Verilen imzayla **taze** önbellek kaydı varsa çıktıları döndürür (cache hit).
    ///
    /// İmza eşleşmezse `None` (cache miss) — node yeniden hesaplanmalı.
    pub fn al(&self, node: NodeKimlik, imza: u64) -> Option<&[AkisDeger]> {
        self.girdiler
            .get(&node)
            .filter(|g| g.imza == imza)
            .map(|g| g.ciktilar.as_slice())
    }

    /// Bir node'un çıktısını imzasıyla saklar (önceki kaydın üzerine yazar).
    pub fn yaz(&mut self, node: NodeKimlik, imza: u64, ciktilar: Vec<AkisDeger>) {
        self.girdiler
            .insert(node, OnbellekGirdisi { imza, ciktilar });
    }

    /// Bir node'un önbelleğini açıkça geçersiz kılar (ör. node silindi/parametre düzenlendi).
    pub fn gecersiz_kil(&mut self, node: NodeKimlik) {
        self.girdiler.remove(&node);
    }

    /// Artık grafikte olmayan node'ların kayıtlarını temizler (bellek sızıntısını önler).
    pub fn buda(&mut self, mevcut: impl Fn(NodeKimlik) -> bool) {
        self.girdiler.retain(|k, _| mevcut(*k));
    }

    /// Tüm önbelleği temizler (tam yeniden hesaplama zorlar).
    pub fn temizle(&mut self) {
        self.girdiler.clear();
    }

    /// Önbellekteki node sayısı.
    pub fn adet(&self) -> usize {
        self.girdiler.len()
    }

    /// Önbellek boş mu?
    pub fn bos_mu(&self) -> bool {
        self.girdiler.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deger(n: u64) -> AkisDeger {
        AkisDeger::yeni("tablo", format!("{n} satır"), n, n * 8)
    }

    #[test]
    fn ayni_imza_hit_farkli_imza_miss() {
        let mut o = SonucOnbellek::yeni();
        let k = NodeKimlik(1);
        o.yaz(k, 42, vec![deger(10)]);
        // Aynı imza → hit.
        assert!(o.al(k, 42).is_some());
        assert_eq!(o.al(k, 42).unwrap()[0].eleman, 10);
        // Farklı imza → miss (bayat sonuç verilmez).
        assert!(o.al(k, 43).is_none());
        // Olmayan node → miss.
        assert!(o.al(NodeKimlik(2), 42).is_none());
    }

    #[test]
    fn uzerine_yazma_ve_gecersiz_kilma() {
        let mut o = SonucOnbellek::yeni();
        let k = NodeKimlik(1);
        o.yaz(k, 1, vec![deger(5)]);
        o.yaz(k, 2, vec![deger(9)]); // yeni imza üzerine yazar
        assert!(o.al(k, 1).is_none(), "eski imza artık geçersiz");
        assert_eq!(o.al(k, 2).unwrap()[0].eleman, 9);
        o.gecersiz_kil(k);
        assert!(o.al(k, 2).is_none());
    }

    #[test]
    fn budama_olmayan_nodelari_atar() {
        let mut o = SonucOnbellek::yeni();
        o.yaz(NodeKimlik(1), 1, vec![deger(1)]);
        o.yaz(NodeKimlik(2), 1, vec![deger(2)]);
        o.yaz(NodeKimlik(3), 1, vec![deger(3)]);
        // Yalnızca 1 ve 3 hâlâ grafikte.
        o.buda(|k| k == NodeKimlik(1) || k == NodeKimlik(3));
        assert_eq!(o.adet(), 2);
        assert!(o.al(NodeKimlik(2), 1).is_none());
        assert!(o.al(NodeKimlik(1), 1).is_some());
    }
}
