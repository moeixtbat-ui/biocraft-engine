//! SDK — Node (düğüm) grafiği uzantı **kontratı** (veri tanımları + çalıştırma arayüzü).
//!
//! Eklenti, node tabanlı iş akışına (İP-05) yeni düğüm türleri ekleyebilir.
//! Bu modül düğümün **arayüz tanımını** (kimlik + portlar + parametre şeması) ve
//! **çalıştırma kontratını** ([`NodeCalistirici`]) taşır; grafik tuvali ve paralel
//! çalıştırma motoru çekirdek/İP-05 tarafındadır (`biocraft-ui`).  Eklentiler birbirine
//! değil, yalnızca bu kontrata bağlanır (MK-17).
//!
//! ## Akış değeri ([`AkisDeger`])
//! Node'lar arasında kablolarda **akış değeri** taşınır.  Çekirdek motorun her veriyi
//! anlaması gerekmez: değer **tipli + özetli + boyutludur** (`bayt`).  Boyut bilgisi
//! bellek bütçesi (İP-08) rezervasyonu içindir; özet, kabloya tıklayınca ara veri
//! önizlemesi içindir.  Değer serileştirilebilir → ileride süreç sınırını (IPC) geçebilir.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Bir port (bağlantı ucu) yönü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortYonu {
    /// Düğüme veri giren uç.
    Giris,
    /// Düğümden veri çıkan uç.
    Cikis,
}

/// Bir düğüm portunun tanımı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortTanimi {
    /// Port adı (düğüm içinde benzersiz).
    pub ad: String,
    /// Giriş mi çıkış mı.
    pub yon: PortYonu,
    /// Taşıdığı veri türünün etiketi (örn. "dizi", "tablo", "hizalama").
    pub veri_turu: String,
}

// ─── Parametre şeması ─────────────────────────────────────────────────────────

/// Bir düğüm parametresinin türü (arayüzde nasıl düzenleneceğini belirler).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParametreTuru {
    /// Serbest metin.
    Metin,
    /// Tam sayı.
    TamSayi,
    /// Ondalık sayı.
    OndalikSayi,
    /// Açık/kapalı.
    Mantik,
    /// Sınırlı seçenek kümesinden biri (açılır liste).
    Secim(Vec<String>),
}

/// Bir düğüm parametresinin **değeri** (çalıştırmada okunur, `.bcflow`'da saklanır).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParametreDeger {
    /// Metin değeri.
    Metin(String),
    /// Tam sayı değeri.
    TamSayi(i64),
    /// Ondalık değeri.
    OndalikSayi(f64),
    /// Mantıksal değer.
    Mantik(bool),
}

impl ParametreDeger {
    /// Metinse içeriği.
    pub fn metin(&self) -> Option<&str> {
        match self {
            ParametreDeger::Metin(s) => Some(s),
            _ => None,
        }
    }
    /// Tam sayıysa değeri.
    pub fn tam_sayi(&self) -> Option<i64> {
        match self {
            ParametreDeger::TamSayi(n) => Some(*n),
            _ => None,
        }
    }
    /// Mantıksalsa değeri.
    pub fn mantik(&self) -> Option<bool> {
        match self {
            ParametreDeger::Mantik(b) => Some(*b),
            _ => None,
        }
    }

    /// **Kararlı** dize imzası — önbellek anahtarı için (locale/precision'dan bağımsız).
    pub fn imza(&self) -> String {
        match self {
            ParametreDeger::Metin(s) => format!("m:{s}"),
            ParametreDeger::TamSayi(n) => format!("t:{n}"),
            // f64 bit deseni → her makinede/çağrıda aynı imza.
            ParametreDeger::OndalikSayi(f) => format!("o:{:016x}", f.to_bits()),
            ParametreDeger::Mantik(b) => format!("b:{b}"),
        }
    }
}

/// Bir düğüm parametresinin **şema tanımı** (ad + tür + varsayılan).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParametreTanimi {
    /// Kanonik anahtar (kodda/serileştirmede kullanılır).
    pub ad: String,
    /// Kullanıcıya görünen etiket.
    pub etiket: String,
    /// Parametre türü.
    pub tur: ParametreTuru,
    /// Varsayılan değer.
    pub varsayilan: ParametreDeger,
}

/// Bir düğümün **parametre değerleri kümesi** (ad → değer).
///
/// `BTreeMap` ile saklanır → imza üretimi sıralı/kararlıdır (önbellek tutarlılığı).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Parametreler {
    degerler: BTreeMap<String, ParametreDeger>,
}

impl Parametreler {
    /// Boş parametre kümesi.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Şema tanımlarından varsayılan değerlerle bir küme kurar.
    pub fn varsayilanlardan(tanimlar: &[ParametreTanimi]) -> Self {
        let mut p = Self::yeni();
        for t in tanimlar {
            p.degerler.insert(t.ad.clone(), t.varsayilan.clone());
        }
        p
    }

    /// Bir parametreyi ayarlar/günceller.
    pub fn ayarla(&mut self, ad: impl Into<String>, deger: ParametreDeger) {
        self.degerler.insert(ad.into(), deger);
    }

    /// Bir parametre değeri (varsa).
    pub fn al(&self, ad: &str) -> Option<&ParametreDeger> {
        self.degerler.get(ad)
    }

    /// Metin parametresi (yoksa/uyumsuzsa `None`).
    pub fn metin(&self, ad: &str) -> Option<&str> {
        self.al(ad).and_then(|d| d.metin())
    }
    /// Tam sayı parametresi.
    pub fn tam_sayi(&self, ad: &str) -> Option<i64> {
        self.al(ad).and_then(|d| d.tam_sayi())
    }
    /// Mantıksal parametre.
    pub fn mantik(&self, ad: &str) -> Option<bool> {
        self.al(ad).and_then(|d| d.mantik())
    }

    /// Tüm değerler (salt-okunur, sıralı).
    pub fn tumu(&self) -> &BTreeMap<String, ParametreDeger> {
        &self.degerler
    }

    /// **Kararlı imza** — önbellek anahtarının parametre bileşeni.
    pub fn imza(&self) -> String {
        let mut s = String::new();
        for (ad, deg) in &self.degerler {
            s.push_str(ad);
            s.push('=');
            s.push_str(&deg.imza());
            s.push(';');
        }
        s
    }
}

/// Bir eklentinin ilan ettiği düğüm türü (palet + çalıştırma için arayüz).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeTanimi {
    /// Düğüm türü kimliği.
    pub kimlik: String,
    /// Kullanıcıya görünen başlık.
    pub baslik: String,
    /// Giriş/çıkış portları.
    pub portlar: Vec<PortTanimi>,
    /// Palet gruplaması için kategori (örn. "Girdi", "İşle", "Çıktı").
    #[serde(default)]
    pub kategori: String,
    /// Kısa açıklama (palet ipucu).
    #[serde(default)]
    pub aciklama: String,
    /// Parametre şeması (yapılandırılabilir alanlar).
    #[serde(default)]
    pub parametreler: Vec<ParametreTanimi>,
}

impl NodeTanimi {
    /// Yalnızca giriş portlarını döndürür (sırasıyla).
    pub fn girisler(&self) -> impl Iterator<Item = &PortTanimi> {
        self.portlar.iter().filter(|p| p.yon == PortYonu::Giris)
    }
    /// Yalnızca çıkış portlarını döndürür (sırasıyla).
    pub fn cikislar(&self) -> impl Iterator<Item = &PortTanimi> {
        self.portlar.iter().filter(|p| p.yon == PortYonu::Cikis)
    }
}

// ─── Akış değeri (kablolarda taşınan veri) ─────────────────────────────────────

/// Node çıkışından bir sonraki node girişine, kabloda akan **değer**.
///
/// Çekirdek motor değerin *içeriğini* yorumlamaz; yalnızca **tip** (uyum denetimi),
/// **özet** (kablo önizlemesi) ve **bayt** (bellek bütçesi rezervasyonu) bilgilerini kullanır.
/// Gerçek bilim verisi (diziler, tablolar) eklenti tarafında tutulur; burada hafif bir
/// **tanıtım/özet** taşınır (out-of-core ilkesiyle uyumlu — MK-09).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AkisDeger {
    /// Değerin veri türü kimliği (port türüyle eşleşir).
    pub veri_turu: String,
    /// Kullanıcıya görünen kısa özet (kablo önizlemesi).
    pub ozet: String,
    /// Eleman/satır sayısı (tablo/dizi için anlamlı; bilinmiyorsa 0).
    pub eleman: u64,
    /// Tahmini bayt boyutu (bellek bütçesi için).
    pub bayt: u64,
}

impl AkisDeger {
    /// Yeni bir akış değeri.
    pub fn yeni(
        veri_turu: impl Into<String>,
        ozet: impl Into<String>,
        eleman: u64,
        bayt: u64,
    ) -> Self {
        Self {
            veri_turu: veri_turu.into(),
            ozet: ozet.into(),
            eleman,
            bayt,
        }
    }

    /// **Kararlı imza** — önbellek anahtarının girdi bileşeni.
    pub fn imza(&self) -> String {
        format!("{}#{}#{}", self.veri_turu, self.eleman, self.bayt)
    }
}

// ─── Çalıştırma kontratı ───────────────────────────────────────────────────────

/// Bir düğüm türünün **çalıştırma davranışı** (eklenti veya çekirdek sağlar).
///
/// `Send + Sync`: motor bağımsız dalları **paralel** çalıştırır (İP-05); çalıştırıcı
/// thread'ler arasında paylaşılabilir olmalıdır.  Çalıştırma **saf** olmalıdır: aynı
/// girdi + parametre → aynı çıktı (önbellek bunun üzerine kuruludur).
pub trait NodeCalistirici: Send + Sync {
    /// Girdileri (yukarı akış çıktıları, port sırasında) ve parametreleri alıp çıktı üretir.
    /// Hata, bu dalı durdurur (bağımsız dallar devam eder); mesaj kullanıcıya gösterilir.
    fn calistir(
        &self,
        girdiler: &[AkisDeger],
        parametreler: &Parametreler,
    ) -> Result<Vec<AkisDeger>, String>;

    /// Bu düğüm **ağır** mı? (Canlı modda uyarı için — pahalı node her tuşta çalışmasın.)
    fn agir_mi(&self) -> bool {
        false
    }

    /// Çalıştırmadan **önce** tahmini bellek ihtiyacı (bayt) — orkestratör rezervasyonu için.
    /// Varsayılan: girdilerin toplam boyutu (çoğu işlem girdisi kadar yer ister).
    fn tahmini_bayt(&self, girdiler: &[AkisDeger]) -> u64 {
        girdiler.iter().map(|g| g.bayt).sum::<u64>().max(1)
    }
}

/// Bir eklentinin **node kaydı**: arayüz tanımı + çalıştırma davranışı.
///
/// Eklenti `biocraft-sdk` üzerinden bunu üretir; çekirdek motor (`biocraft-ui`) onu
/// kataloğa (palet) + çalıştırıcı kaydına ekler.  `Arc` ile çalıştırıcı, paralel
/// çalıştırmada thread'lere ucuzca klonlanır.
#[derive(Clone)]
pub struct NodeKaydi {
    /// Düğümün arayüz tanımı (portlar + parametre şeması).
    pub tanim: NodeTanimi,
    /// Çalıştırma davranışı.
    pub yurutucu: std::sync::Arc<dyn NodeCalistirici>,
}

impl NodeKaydi {
    /// Tanım + çalıştırıcıdan kayıt kurar.
    pub fn yeni(tanim: NodeTanimi, yurutucu: std::sync::Arc<dyn NodeCalistirici>) -> Self {
        Self { tanim, yurutucu }
    }
}

impl std::fmt::Debug for NodeKaydi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `dyn NodeCalistirici` Debug değil; tanım kimliğini gösteririz.
        f.debug_struct("NodeKaydi")
            .field("tanim", &self.tanim.kimlik)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_tanimi_serde_gidis_donus() {
        let n = NodeTanimi {
            kimlik: "node.hizala".into(),
            baslik: "Hizalama".into(),
            kategori: "İşle".into(),
            aciklama: "Dizileri hizalar.".into(),
            portlar: vec![
                PortTanimi {
                    ad: "girdi".into(),
                    yon: PortYonu::Giris,
                    veri_turu: "dizi".into(),
                },
                PortTanimi {
                    ad: "sonuc".into(),
                    yon: PortYonu::Cikis,
                    veri_turu: "hizalama".into(),
                },
            ],
            parametreler: vec![ParametreTanimi {
                ad: "duyarlilik".into(),
                etiket: "Duyarlılık".into(),
                tur: ParametreTuru::OndalikSayi,
                varsayilan: ParametreDeger::OndalikSayi(0.8),
            }],
        };
        let json = serde_json::to_string(&n).unwrap();
        let geri: NodeTanimi = serde_json::from_str(&json).unwrap();
        assert_eq!(n, geri);
    }

    #[test]
    fn eski_node_tanimi_yeni_alansiz_okunur() {
        // Geriye uyum: kategori/açıklama/parametreler olmadan da çözülmeli (serde default).
        let eski = r#"{"kimlik":"x","baslik":"X","portlar":[]}"#;
        let n: NodeTanimi = serde_json::from_str(eski).unwrap();
        assert_eq!(n.kimlik, "x");
        assert!(n.parametreler.is_empty());
        assert_eq!(n.kategori, "");
    }

    #[test]
    fn parametre_imzasi_kararli_ve_ondalik_bit_tabanli() {
        let mut a = Parametreler::yeni();
        a.ayarla("k", ParametreDeger::OndalikSayi(0.1));
        a.ayarla("a", ParametreDeger::TamSayi(3));
        let mut b = Parametreler::yeni();
        // Ekleme sırası farklı ama imza aynı olmalı (BTreeMap → sıralı).
        b.ayarla("a", ParametreDeger::TamSayi(3));
        b.ayarla("k", ParametreDeger::OndalikSayi(0.1));
        assert_eq!(a.imza(), b.imza());
        // Değer değişince imza değişir.
        b.ayarla("a", ParametreDeger::TamSayi(4));
        assert_ne!(a.imza(), b.imza());
    }

    #[test]
    fn varsayilanlardan_kurar() {
        let tanimlar = vec![
            ParametreTanimi {
                ad: "esik".into(),
                etiket: "Eşik".into(),
                tur: ParametreTuru::TamSayi,
                varsayilan: ParametreDeger::TamSayi(30),
            },
            ParametreTanimi {
                ad: "ad".into(),
                etiket: "Ad".into(),
                tur: ParametreTuru::Metin,
                varsayilan: ParametreDeger::Metin("varsayilan".into()),
            },
        ];
        let p = Parametreler::varsayilanlardan(&tanimlar);
        assert_eq!(p.tam_sayi("esik"), Some(30));
        assert_eq!(p.metin("ad"), Some("varsayilan"));
    }

    /// Basit bir çalıştırıcı: girdileri sayar, tek çıktı üretir.
    struct Toplayici;
    impl NodeCalistirici for Toplayici {
        fn calistir(
            &self,
            girdiler: &[AkisDeger],
            _p: &Parametreler,
        ) -> Result<Vec<AkisDeger>, String> {
            let toplam: u64 = girdiler.iter().map(|g| g.eleman).sum();
            Ok(vec![AkisDeger::yeni(
                "tablo",
                format!("{toplam} satır"),
                toplam,
                toplam * 8,
            )])
        }
    }

    #[test]
    fn node_kaydi_calistirir() {
        let kayit = NodeKaydi::yeni(
            NodeTanimi {
                kimlik: "topla".into(),
                baslik: "Topla".into(),
                kategori: "İşle".into(),
                aciklama: String::new(),
                portlar: vec![],
                parametreler: vec![],
            },
            std::sync::Arc::new(Toplayici),
        );
        let girdi = vec![AkisDeger::yeni("dizi", "5", 5, 40)];
        let cikti = kayit
            .yurutucu
            .calistir(&girdi, &Parametreler::yeni())
            .unwrap();
        assert_eq!(cikti[0].eleman, 5);
        // Varsayılan bayt tahmini = girdi toplamı.
        assert_eq!(kayit.yurutucu.tahmini_bayt(&girdi), 40);
    }
}
