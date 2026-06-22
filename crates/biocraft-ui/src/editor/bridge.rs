//! Node ↔ Kod köprüsü + **ortak çalışma alanı (workspace)** (İP-06, 2. kısım — Gün 23).
//!
//! Görsel akış (node) ile yazılı kod (editör) arasında **tek yönlü dönüşüm değil**, **iki
//! yönlü veri paylaşımı** kurar:
//! * **"Bu node'u kod olarak aç"** — akış eşdeğer Python betiğine çevrilir
//!   ([`crate::node::python_disa_aktar`]) ve başına **node çıktılarını taşıyan bir önsöz**
//!   eklenir → üretilen kod, akışın ürettiği değerleri (`workspace[...]`) okuyabilir.
//! * **"Bu kodu node akışına ekle"** — kod, akışa tek bir **betik node'u** olarak sarılır
//!   ([`KodDugumTanimi`]); girişleri/çıkışları workspace değişkenlerinden türetilir.
//!
//! ## Ortak workspace — **tipli ve net** (veri kaybı yok)
//! Köprünün kalbi [`CalismaAlani`]'dır: ad → **tipli** değer ([`DegiskenDeger`]).  Değerler
//! ya doğrudan skalerdir (metin/sayı/mantık) ya da büyük veri için **tanıtım/özet**
//! taşıyan bir handle'dır ([`DegiskenDeger::Veri`]) — devasa tablo/dizi koda kopyalanmaz,
//! yalnız **tipli referansı** geçer (out-of-core, MK-09).  Böylece "köprüde veri kaybı"
//! olmaz: her değerin türü, özeti ve boyutu korunur.
//!
//! ## Veri akışı (node → kod → node)
//! 1. **node → kod:** [`node_ciktilarini_al`] çalıştırma sonucunu workspace'e koyar;
//!    [`python_onsoz`] bunu Python `workspace = {…}` sözlüğüne çevirir (önsöz).
//! 2. **kod → node:** üretilen koda [`python_sonsoz`] eklenir (çalışma sonunda `workspace`
//!    sözlüğünü bir **sentinel satırı** olarak basar); [`cikti_workspace_ayikla`] bu satırı
//!    bularak sonucu **tekrar tipli** workspace'e çözer → değer node tarafına döner.
//!
//! ⚠️ **Canlı çift yönlü senkron YOK** (`MVP-sonrasi.md`): tek aktif görünüm; köprü açık bir
//! kullanıcı eylemiyle çalışır (otomatik/sürekli senkron v1.x).  **Kod → Node tam dönüşüm**
//! (kodu ayrıştırıp gerçek node grafiğine çevirme) de v1.x'tir; buradaki yön kodu **bir node
//! olarak sarar**, ayrıştırmaz.
// MK-54: motor temelde; MK-09 büyük veri kopyalanmaz (tanıtım/özet). MK-02: üretilen kod ayrı süreçte çalışır.

use std::collections::{BTreeMap, HashMap};

use biocraft_sdk::node::{AkisDeger, ParametreDeger, Parametreler};

use crate::node::graph::{NodeGraf, NodeKimlik};
use crate::node::run::CalismaSonucu;

/// Üretilen kodun workspace sözlüğünü bastığı satırın **sentinel** öneki.
///
/// Çalışan betik en sonda `WORKSPACE_SENTINEL + json` basar; Rust tarafı çıktı satırlarında
/// bu öneki arayıp sonucu **tekrar tipli** workspace'e çözer (kod → node yönü).
pub const WORKSPACE_SENTINEL: &str = "__BIOCRAFT_WORKSPACE__:";

// ─── Tipli workspace değeri ────────────────────────────────────────────────────

/// Ortak çalışma alanındaki **tek bir tipli değer** (net — tür/özet korunur, kayıp yok).
///
/// Skaler değerler doğrudan taşınır; büyük bilim verisi ([`DegiskenDeger::Veri`]) **kopyalanmaz**,
/// yalnız tipli bir **tanıtım** (tür + özet + eleman + bayt) taşınır (out-of-core, MK-09).
#[derive(Debug, Clone, PartialEq)]
pub enum DegiskenDeger {
    /// Metin.
    Metin(String),
    /// Tam sayı.
    TamSayi(i64),
    /// Ondalık sayı.
    Ondalik(f64),
    /// Mantıksal (açık/kapalı).
    Mantik(bool),
    /// Büyük veri **tanıtımı** (kopyalanmaz; node kablosundaki [`AkisDeger`] ile aynı bilgi).
    Veri {
        /// Veri türü kimliği (port türüyle eşleşir).
        veri_turu: String,
        /// İnsan-okur kısa özet.
        ozet: String,
        /// Eleman/satır sayısı (bilinmiyorsa 0).
        eleman: u64,
        /// Tahmini bayt boyutu.
        bayt: u64,
    },
}

impl DegiskenDeger {
    /// Türün kısa adı (teşhis/UI).
    pub fn tur_adi(&self) -> &'static str {
        match self {
            DegiskenDeger::Metin(_) => "metin",
            DegiskenDeger::TamSayi(_) => "tam sayı",
            DegiskenDeger::Ondalik(_) => "ondalık",
            DegiskenDeger::Mantik(_) => "mantık",
            DegiskenDeger::Veri { .. } => "veri",
        }
    }

    /// Kısa, insan-okur özet (UI'da değişken satırı).
    pub fn ozet(&self) -> String {
        match self {
            DegiskenDeger::Metin(s) => {
                if s.len() > 40 {
                    format!("\"{}…\"", kes(s, 40))
                } else {
                    format!("\"{s}\"")
                }
            }
            DegiskenDeger::TamSayi(n) => n.to_string(),
            DegiskenDeger::Ondalik(f) => format!("{f}"),
            DegiskenDeger::Mantik(b) => b.to_string(),
            DegiskenDeger::Veri {
                veri_turu,
                ozet,
                eleman,
                ..
            } => format!("{veri_turu}: {ozet} ({eleman} eleman)"),
        }
    }

    /// Bir [`AkisDeger`]'i (node kablosu) workspace değerine çevirir — **tip korunur**.
    pub fn akistan(a: &AkisDeger) -> Self {
        DegiskenDeger::Veri {
            veri_turu: a.veri_turu.clone(),
            ozet: a.ozet.clone(),
            eleman: a.eleman,
            bayt: a.bayt,
        }
    }

    /// Bir parametre değerini (UI alanı) workspace değerine çevirir.
    pub fn parametreden(p: &ParametreDeger) -> Self {
        match p {
            ParametreDeger::Metin(s) => DegiskenDeger::Metin(s.clone()),
            ParametreDeger::TamSayi(n) => DegiskenDeger::TamSayi(*n),
            ParametreDeger::OndalikSayi(f) => DegiskenDeger::Ondalik(*f),
            ParametreDeger::Mantik(b) => DegiskenDeger::Mantik(*b),
        }
    }

    /// Workspace değerini node kablosuna (**kod → node**) çevirir.  Skaler değerler küçük bir
    /// [`AkisDeger`] tanıtımına; [`DegiskenDeger::Veri`] aslına döner (tür/özet/boyut korunur).
    pub fn akisa(&self) -> AkisDeger {
        match self {
            DegiskenDeger::Metin(s) => AkisDeger::yeni("metin", s.clone(), 1, s.len() as u64),
            DegiskenDeger::TamSayi(n) => AkisDeger::yeni("tam_sayi", n.to_string(), 1, 8),
            DegiskenDeger::Ondalik(f) => AkisDeger::yeni("ondalik", format!("{f}"), 1, 8),
            DegiskenDeger::Mantik(b) => AkisDeger::yeni("mantik", b.to_string(), 1, 1),
            DegiskenDeger::Veri {
                veri_turu,
                ozet,
                eleman,
                bayt,
            } => AkisDeger::yeni(veri_turu.clone(), ozet.clone(), *eleman, *bayt),
        }
    }

    /// Bu değeri bir **Python literali** olarak yazar (önsöz `workspace` sözlüğü için).
    fn python_literal(&self) -> String {
        match self {
            DegiskenDeger::Metin(s) => python_metin_literal(s),
            DegiskenDeger::TamSayi(n) => n.to_string(),
            DegiskenDeger::Ondalik(f) => {
                if f.is_finite() {
                    format!("{f}")
                } else {
                    format!("float('{f}')")
                }
            }
            DegiskenDeger::Mantik(b) => if *b { "True" } else { "False" }.to_string(),
            // Büyük veri kopyalanmaz: kod tarafına tipli bir **tanıtım sözlüğü** geçer.
            DegiskenDeger::Veri {
                veri_turu,
                ozet,
                eleman,
                bayt,
            } => format!(
                "{{\"_tur\": \"veri\", \"veri_turu\": {}, \"ozet\": {}, \"eleman\": {}, \"bayt\": {}}}",
                python_metin_literal(veri_turu),
                python_metin_literal(ozet),
                eleman,
                bayt
            ),
        }
    }

    /// Bir JSON değerinden (`kod → node` sentinel'i) **tekrar tipli** workspace değeri çözer.
    ///
    /// Tanınmayan biçim **kaybolmaz**: metinsel gösterimi `Metin` olarak saklanır (net).
    pub fn jsondan(v: &serde_json::Value) -> Self {
        match v {
            serde_json::Value::String(s) => DegiskenDeger::Metin(s.clone()),
            serde_json::Value::Bool(b) => DegiskenDeger::Mantik(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    DegiskenDeger::TamSayi(i)
                } else {
                    DegiskenDeger::Ondalik(n.as_f64().unwrap_or(0.0))
                }
            }
            serde_json::Value::Object(m)
                if m.get("_tur").and_then(|t| t.as_str()) == Some("veri") =>
            {
                DegiskenDeger::Veri {
                    veri_turu: m
                        .get("veri_turu")
                        .and_then(|x| x.as_str())
                        .unwrap_or("veri")
                        .to_string(),
                    ozet: m
                        .get("ozet")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    eleman: m.get("eleman").and_then(|x| x.as_u64()).unwrap_or(0),
                    bayt: m.get("bayt").and_then(|x| x.as_u64()).unwrap_or(0),
                }
            }
            // Dizi/iç içe nesne vb.: net metinsel gösterim (kayıp yok).
            diger => DegiskenDeger::Metin(diger.to_string()),
        }
    }
}

// ─── Ortak çalışma alanı (workspace) ───────────────────────────────────────────

/// Node ile kod arasında paylaşılan **adlandırılmış, tipli** değişkenler kümesi.
///
/// `BTreeMap` → kararlı/sıralı (üretilen önsöz deterministik; testlenebilir).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CalismaAlani {
    degerler: BTreeMap<String, DegiskenDeger>,
}

impl CalismaAlani {
    /// Boş çalışma alanı.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir değişkeni ayarlar/günceller (ad ASCII-temiz tutulur → geçerli Python anahtarı).
    pub fn ayarla(&mut self, ad: impl AsRef<str>, deger: DegiskenDeger) {
        self.degerler.insert(temiz_ad(ad.as_ref()), deger);
    }

    /// Bir değişken (varsa).
    pub fn al(&self, ad: &str) -> Option<&DegiskenDeger> {
        self.degerler.get(ad)
    }

    /// Tüm değişkenler (sıralı, salt-okunur).
    pub fn tumu(&self) -> &BTreeMap<String, DegiskenDeger> {
        &self.degerler
    }

    /// Değişken sayısı.
    pub fn len(&self) -> usize {
        self.degerler.len()
    }

    /// Boş mu?
    pub fn is_empty(&self) -> bool {
        self.degerler.is_empty()
    }

    /// Tümünü temizler.
    pub fn temizle(&mut self) {
        self.degerler.clear();
    }
}

// ─── node → kod ────────────────────────────────────────────────────────────────

/// Bir node çalıştırma sonucunu workspace'e çevirir: her node'un çıktısı bir değişken olur.
///
/// Değişken adı: tek çıkışta `n<id>`, çok çıkışta `n<id>_<port>` — üretilen kodun node
/// değişken adlandırmasıyla (bkz. `python_disa_aktar`) hizalı, böylece kod sezgisel okur.
pub fn node_ciktilarini_al(graf: &NodeGraf, sonuc: &CalismaSonucu) -> CalismaAlani {
    let mut alan = CalismaAlani::yeni();
    for (kimlik, ns) in &sonuc.node_sonuclari {
        if ns.ciktilar.is_empty() {
            continue;
        }
        let node = graf.node(*kimlik);
        for (i, cikti) in ns.ciktilar.iter().enumerate() {
            let port_ad = node
                .and_then(|n| n.cikislar.get(i))
                .map(|p| temiz_ad(&p.ad))
                .filter(|s| !s.is_empty());
            let ad = match (ns.ciktilar.len(), port_ad) {
                (1, _) => degisken_adi(*kimlik),
                (_, Some(p)) => format!("{}_{p}", degisken_adi(*kimlik)),
                (_, None) => format!("{}_{i}", degisken_adi(*kimlik)),
            };
            alan.ayarla(ad, DegiskenDeger::akistan(cikti));
        }
    }
    alan
}

/// `workspace = {…}` Python önsözü — üretilen kod node çıktılarını buradan okur.
pub fn python_onsoz(alan: &CalismaAlani) -> String {
    let mut s = String::new();
    s.push_str("# ── Ortak çalışma alanı (node → kod köprüsü) ──\n");
    s.push_str("# Bu sözlük, görsel akışın ürettiği değerleri taşır (büyük veri = tanıtım).\n");
    if alan.is_empty() {
        s.push_str("workspace = {}\n\n");
        return s;
    }
    s.push_str("workspace = {\n");
    for (ad, deg) in alan.tumu() {
        s.push_str(&format!(
            "    {}: {},\n",
            python_metin_literal(ad),
            deg.python_literal()
        ));
    }
    s.push_str("}\n\n");
    s
}

/// `workspace`'i çalışma sonunda **sentinel satırı** olarak basan Python sonsözü (kod → node).
pub fn python_sonsoz() -> String {
    let mut s = String::new();
    s.push('\n');
    s.push_str("# ── Sonucu node tarafına geri ver (kod → node köprüsü) ──\n");
    s.push_str("import json as _bc_json\n");
    s.push_str(&format!(
        "print({} + _bc_json.dumps(workspace, default=str))\n",
        python_metin_literal(WORKSPACE_SENTINEL)
    ));
    s
}

/// **"Bu node'u kod olarak aç":** akışı eşdeğer Python'a çevirir; başına node çıktılarını
/// taşıyan workspace önsözü, sonuna kod→node sonsözü ekler (tam köprülü betik).
pub fn node_olarak_kod(
    graf: &NodeGraf,
    parametreler: &HashMap<NodeKimlik, Parametreler>,
    sonuc: Option<&CalismaSonucu>,
) -> String {
    let alan = match sonuc {
        Some(s) => node_ciktilarini_al(graf, s),
        None => CalismaAlani::yeni(),
    };
    let mut metin = String::new();
    metin.push_str("#!/usr/bin/env python3\n# -*- coding: utf-8 -*-\n");
    metin.push_str("# BioCraft Engine — node ↔ kod köprüsü ile üretildi (İP-06).\n\n");
    metin.push_str(&python_onsoz(&alan));
    metin.push_str(&crate::node::python_disa_aktar(graf, parametreler));
    metin.push_str(&python_sonsoz());
    metin
}

// ─── kod → node ──────────────────────────────────────────────────────────────

/// **"Bu kodu node akışına ekle":** kodu tek bir **betik node'u** olarak saran tanım.
///
/// Tam tersine-çevirme (kodu ayrıştırıp gerçek graf üretme) **v1.x**'tir; bu tanım kodu
/// olduğu gibi sarar — girişleri/çıkışları workspace değişkenlerinden türetilir.
#[derive(Debug, Clone, PartialEq)]
pub struct KodDugumTanimi {
    /// Node başlığı.
    pub baslik: String,
    /// Sarılan kaynak kod.
    pub kod: String,
    /// Giriş portu adları (workspace'ten okunacak değişkenler).
    pub girisler: Vec<String>,
    /// Çıkış portu adları (workspace'e yazılacak değişkenler).
    pub cikislar: Vec<String>,
}

impl KodDugumTanimi {
    /// Bir kod parçasını + o anki workspace'i sararak betik node'u tanımı üretir.
    ///
    /// Workspace'teki her değişken bir **giriş** portu sayılır (kod onları okuyabilir);
    /// `cikislar` başlangıçta `sonuc` tek çıkışıdır (kod en az bir değer üretir varsayımı).
    pub fn koddan(baslik: impl Into<String>, kod: impl Into<String>, alan: &CalismaAlani) -> Self {
        let girisler: Vec<String> = alan.tumu().keys().cloned().collect();
        Self {
            baslik: baslik.into(),
            kod: kod.into(),
            girisler,
            cikislar: vec!["sonuc".to_string()],
        }
    }
}

// ─── kod çıktısı → workspace (sentinel ayıklama) ───────────────────────────────

/// Çalışan kodun çıktı satırlarında **sentinel**'i arar; bulursa workspace'i tipli çözer.
///
/// Birden çok sentinel varsa **sonuncusu** (en güncel) kullanılır.  Yoksa `None` (kod
/// workspace basmadı — örn. köprüsüz çalıştırma).
pub fn cikti_workspace_ayikla<'a, I>(satirlar: I) -> Option<CalismaAlani>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut son: Option<&str> = None;
    for s in satirlar {
        if let Some(rest) = s.strip_prefix(WORKSPACE_SENTINEL) {
            son = Some(rest);
        }
    }
    let json = son?.trim();
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let obj = v.as_object()?;
    let mut alan = CalismaAlani::yeni();
    for (ad, deger) in obj {
        alan.ayarla(ad, DegiskenDeger::jsondan(deger));
    }
    Some(alan)
}

// ─── Saf yardımcılar ───────────────────────────────────────────────────────────

/// Node kimliğinden kararlı Python değişken adı (`python_disa_aktar` ile aynı kalıp).
fn degisken_adi(k: NodeKimlik) -> String {
    format!("n{}", k.0)
}

/// Bir adı geçerli, ASCII Python tanımlayıcısına indirger (Türkçe harf çevrilir).
fn temiz_ad(s: &str) -> String {
    let cevir = |c: char| -> char {
        match c {
            'ç' => 'c',
            'ş' => 's',
            'ğ' => 'g',
            'ı' => 'i',
            'ö' => 'o',
            'ü' => 'u',
            'İ' => 'i',
            'Ç' => 'C',
            'Ş' => 'S',
            'Ğ' => 'G',
            'Ö' => 'O',
            'Ü' => 'U',
            d => d,
        }
    };
    let mut out = String::new();
    for c in s.chars() {
        let c = cevir(c);
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        out.insert(0, '_');
    }
    out
}

/// `s`'i en fazla `n` bayta, **char sınırına** saygılı keser (panik yok).
fn kes(s: &str, n: usize) -> &str {
    let mut b = n.min(s.len());
    while b > 0 && !s.is_char_boundary(b) {
        b -= 1;
    }
    &s[..b]
}

/// Bir dizgeyi güvenli bir Python (JSON-uyumlu) metin literaline çevirir.
fn python_metin_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            d => out.push(d),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::graph::{Baglanti, NodeGraf};
    use crate::node::katalog::NodeKatalogu;
    use crate::node::port::PortYonu;
    use crate::node::run::{CalismaSonucu, NodeSonuc};
    use crate::node::NodeDurumu;
    use crate::node::PortRef;

    fn akis() -> (NodeGraf, NodeKimlik, NodeKimlik) {
        let katalog = NodeKatalogu::ornek();
        let mut g = NodeGraf::yeni("ana");
        let ekle = |g: &mut NodeGraf, tur: &str| -> NodeKimlik {
            let k = g.yeni_node_kimlik();
            g.node_ekle_ham(katalog.bul(tur).unwrap().ornekle(k, (0.0, 0.0)));
            k
        };
        let oku = ekle(&mut g, "girdi.dizi_oku");
        let hiz = ekle(&mut g, "isle.hizala");
        let bk = g.yeni_baglanti_kimlik();
        g.baglanti_ekle_ham(Baglanti {
            kimlik: bk,
            kaynak: PortRef::yeni(oku, PortYonu::Cikis, 0),
            hedef: PortRef::yeni(hiz, PortYonu::Giris, 0),
        });
        (g, oku, hiz)
    }

    fn sonuc_ile(k: NodeKimlik, cikti: AkisDeger) -> CalismaSonucu {
        let mut s = CalismaSonucu::default();
        let ns = NodeSonuc {
            durum: NodeDurumu::Bitti,
            ciktilar: vec![cikti],
            hata: None,
            onbellekten: false,
            atlandi: false,
        };
        s.node_sonuclari.insert(k, ns);
        s
    }

    #[test]
    fn akis_deger_tipli_korunur() {
        let a = AkisDeger::yeni("tablo", "120 satır", 120, 960);
        let d = DegiskenDeger::akistan(&a);
        // İleri-geri: tür/özet/boyut korunur (kayıp yok).
        let geri = d.akisa();
        assert_eq!(geri.veri_turu, "tablo");
        assert_eq!(geri.eleman, 120);
        assert_eq!(geri.bayt, 960);
    }

    #[test]
    fn node_ciktilari_workspace_olur() {
        let (g, oku, _) = akis();
        let sonuc = sonuc_ile(oku, AkisDeger::yeni("dizi", "1000 okuma", 1000, 8000));
        let alan = node_ciktilarini_al(&g, &sonuc);
        // Tek çıkışlı node → n<id> adıyla görünür.
        let ad = format!("n{}", oku.0);
        assert!(alan.al(&ad).is_some(), "workspace: {:?}", alan.tumu());
        match alan.al(&ad).unwrap() {
            DegiskenDeger::Veri { eleman, .. } => assert_eq!(*eleman, 1000),
            d => panic!("Veri bekleniyordu: {d:?}"),
        }
    }

    #[test]
    fn onsoz_python_workspace_uretir() {
        let mut alan = CalismaAlani::yeni();
        alan.ayarla("esik", DegiskenDeger::TamSayi(30));
        alan.ayarla("ad", DegiskenDeger::Metin("deney".into()));
        alan.ayarla(
            "tablo",
            DegiskenDeger::Veri {
                veri_turu: "tablo".into(),
                ozet: "10 satır".into(),
                eleman: 10,
                bayt: 80,
            },
        );
        let py = python_onsoz(&alan);
        assert!(py.contains("workspace = {"));
        assert!(py.contains("\"esik\": 30"));
        assert!(py.contains("\"ad\": \"deney\""));
        // Büyük veri kopyalanmaz → tanıtım sözlüğü.
        assert!(py.contains("\"_tur\": \"veri\""));
        assert!(py.contains("\"eleman\": 10"));
    }

    #[test]
    fn node_olarak_kod_kopru_uretir() {
        let (g, oku, _) = akis();
        let sonuc = sonuc_ile(oku, AkisDeger::yeni("dizi", "1000 okuma", 1000, 8000));
        let kod = node_olarak_kod(&g, &HashMap::new(), Some(&sonuc));
        // Önsöz + akış + sonsöz hepsi var.
        assert!(kod.contains("workspace = {"));
        assert!(kod.contains("def calistir():"));
        assert!(kod.contains(WORKSPACE_SENTINEL));
        assert!(kod.contains("_bc_json.dumps(workspace"));
    }

    #[test]
    fn sentinel_workspace_geri_cozulur() {
        // Kod, sentinel satırını basar (kod → node).  Tipli çözülmeli.
        let satirlar = [
            "merhaba".to_string(),
            format!(
                "{}{}",
                WORKSPACE_SENTINEL,
                r#"{"esik": 42, "oran": 0.5, "acik": true, "etiket": "x", "tablo": {"_tur":"veri","veri_turu":"tablo","ozet":"5 satır","eleman":5,"bayt":40}}"#
            ),
        ];
        let alan = cikti_workspace_ayikla(satirlar.iter().map(|s| s.as_str())).unwrap();
        assert_eq!(alan.al("esik"), Some(&DegiskenDeger::TamSayi(42)));
        assert_eq!(alan.al("oran"), Some(&DegiskenDeger::Ondalik(0.5)));
        assert_eq!(alan.al("acik"), Some(&DegiskenDeger::Mantik(true)));
        assert_eq!(alan.al("etiket"), Some(&DegiskenDeger::Metin("x".into())));
        match alan.al("tablo").unwrap() {
            DegiskenDeger::Veri {
                eleman, veri_turu, ..
            } => {
                assert_eq!(*eleman, 5);
                assert_eq!(veri_turu, "tablo");
            }
            d => panic!("Veri bekleniyordu: {d:?}"),
        }
    }

    #[test]
    fn sentinel_yoksa_none() {
        let satirlar = ["normal çıktı", "başka satır"];
        assert!(cikti_workspace_ayikla(satirlar.iter().copied()).is_none());
    }

    #[test]
    fn kod_dugumu_workspace_girisleri_alir() {
        let mut alan = CalismaAlani::yeni();
        alan.ayarla("a", DegiskenDeger::TamSayi(1));
        alan.ayarla("b", DegiskenDeger::Metin("x".into()));
        let dugum = KodDugumTanimi::koddan("Betik", "print(a)", &alan);
        assert_eq!(dugum.girisler, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(dugum.cikislar, vec!["sonuc".to_string()]);
        assert!(dugum.kod.contains("print(a)"));
    }

    #[test]
    fn temiz_ad_turkce_ve_rakam_duzeltir() {
        assert_eq!(temiz_ad("Çıktı Özet"), "cikti_ozet");
        assert_eq!(temiz_ad("3sonuc"), "_3sonuc");
    }

    #[test]
    fn round_trip_tipli_kayipsiz() {
        // Sayı/metin: python_literal **geçerli JSON** üretir → doğrudan geri çözülür.
        for d in [
            DegiskenDeger::TamSayi(7),
            DegiskenDeger::Ondalik(1.5),
            DegiskenDeger::Metin("merhaba".into()),
        ] {
            let lit = d.python_literal();
            let v: serde_json::Value = serde_json::from_str(&lit).unwrap();
            assert_eq!(DegiskenDeger::jsondan(&v), d);
        }
        // Mantık: python_literal **Python** True/False üretir (JSON değil); gerçek round-trip
        // Python json.dumps üzerinden olur (true/false). jsondan JSON bool'u doğru çözer.
        assert_eq!(DegiskenDeger::Mantik(true).python_literal(), "True");
        assert_eq!(DegiskenDeger::Mantik(false).python_literal(), "False");
        assert_eq!(
            DegiskenDeger::jsondan(&serde_json::Value::Bool(true)),
            DegiskenDeger::Mantik(true)
        );
    }
}
