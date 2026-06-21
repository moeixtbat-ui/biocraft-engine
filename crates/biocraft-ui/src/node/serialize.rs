//! Akış kaydı (`.bcflow`) + görsel dışa aktarma (SVG/PNG) — İP-05 "Kayıt".
//!
//! ## `.bcflow` (JSON)
//! Akış **insan-okunur JSON** olarak kaydedilir: **sürüm** + node id + bağlantı + parametre.
//! Böylece `git diff` ile değişiklik izlenebilir (RON yerine JSON: `serde_json` zaten ağaçta,
//! yeni dış bağımlılık yok).  Dosyaya **sürüm alanı** konur ve [`goc`] ile eski sürümler
//! yükseltilir (MK-59 — "eski sürüm açılmıyor" sorununun çözümü).
//!
//! ## Görsel dışa aktarma
//! - [`svg_disa_aktar`] — **tam vektör** (metin + renk + kablo); ölçeklenebilir, baskıya uygun.
//! - [`png_disa_aktar`] — **saf-Rust** raster küçük resim (kutular + durum + portlar + kablolar);
//!   dış bağımlılık olmadan geçerli PNG üretir (kendi minimal kodlayıcısı; bkz. [`png`] alt-modül).
//!   Glyph fontu gömülmez → metin etiketleri SVG'dedir; PNG yapısal önizlemedir.
// MK-52: renkler token'dan; MK-53: dışa aktarmada da yerel metin.

use std::collections::{BTreeMap, HashMap};

use egui::Color32;
use serde::{Deserialize, Serialize};

use biocraft_sdk::node::{ParametreDeger, Parametreler};
use biocraft_types::ErrorReport;

use super::graph::{
    Baglanti, BaglantiKimlik, Node, NodeDurumu, NodeGraf, NodeKimlik, NotKimlik, PortRef,
    YapiskanNot,
};
use super::port::{tur_renk_anahtari, Port, PortYonu, VeriTuru};
use crate::i18n::Dil;
use crate::tokens::Tokenlar;

/// `.bcflow` biçim sürümü.  Kırıcı şema değişikliğinde artırılır + [`goc`]'a bir adım eklenir.
pub const BCFLOW_SURUM: u32 = 1;

// ─── Serileştirme DTO'ları (graf egui'siz + serde'siz tutulduğu için ayrı tipler) ──

/// Bir portun kayıt biçimi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BcflowPort {
    /// Port adı.
    pub ad: String,
    /// Veri türü kimliği.
    pub tur: String,
}

/// Bir node'un kayıt biçimi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BcflowNode {
    /// Node kimliği (sayısal).
    pub kimlik: u64,
    /// Tür kimliği (`girdi.dizi_oku` vb.).
    pub tur: String,
    /// Başlık.
    pub baslik: String,
    /// Tuval konumu.
    pub konum: (f32, f32),
    /// Giriş portları.
    pub girisler: Vec<BcflowPort>,
    /// Çıkış portları.
    pub cikislar: Vec<BcflowPort>,
    /// Parametre değerleri (ad → değer).
    #[serde(default)]
    pub parametreler: BTreeMap<String, ParametreDeger>,
}

/// Bir bağlantının kayıt biçimi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BcflowBaglanti {
    /// Bağlantı kimliği.
    pub kimlik: u64,
    /// Kaynak node + çıkış port dizini.
    pub kaynak_node: u64,
    /// Kaynak çıkış port dizini.
    pub kaynak_port: usize,
    /// Hedef node + giriş port dizini.
    pub hedef_node: u64,
    /// Hedef giriş port dizini.
    pub hedef_port: usize,
}

/// Bir yapışkan notun kayıt biçimi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BcflowNot {
    /// Not kimliği.
    pub kimlik: u64,
    /// Metin.
    pub metin: String,
    /// Konum.
    pub konum: (f32, f32),
}

/// Tam `.bcflow` belgesi (sürümlü).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BcflowBelge {
    /// Biçim sürümü (MK-59 göç için).
    pub surum: u32,
    /// Grafiğin mantıksal kimliği (undo deposu sınırı).
    pub graf_kimlik: String,
    /// Node'lar.
    pub nodelar: Vec<BcflowNode>,
    /// Bağlantılar.
    pub baglantilar: Vec<BcflowBaglanti>,
    /// Yapışkan notlar.
    pub notlar: Vec<BcflowNot>,
}

impl BcflowBelge {
    /// Bir grafik + parametre kümesinden belge üretir.
    pub fn graften(graf: &NodeGraf, parametreler: &HashMap<NodeKimlik, Parametreler>) -> Self {
        let nodelar = graf
            .nodelar()
            .iter()
            .map(|n| BcflowNode {
                kimlik: n.kimlik.0,
                tur: n.tur_kimligi.clone(),
                baslik: n.baslik.clone(),
                konum: n.konum,
                girisler: n.girisler.iter().map(port_dto).collect(),
                cikislar: n.cikislar.iter().map(port_dto).collect(),
                parametreler: parametreler
                    .get(&n.kimlik)
                    .map(|p| p.tumu().clone())
                    .unwrap_or_default(),
            })
            .collect();
        let baglantilar = graf
            .baglantilar()
            .iter()
            .map(|b| BcflowBaglanti {
                kimlik: b.kimlik.0,
                kaynak_node: b.kaynak.node.0,
                kaynak_port: b.kaynak.indeks,
                hedef_node: b.hedef.node.0,
                hedef_port: b.hedef.indeks,
            })
            .collect();
        let notlar = graf
            .notlar()
            .iter()
            .map(|n| BcflowNot {
                kimlik: n.kimlik.0,
                metin: n.metin.clone(),
                konum: n.konum,
            })
            .collect();
        Self {
            surum: BCFLOW_SURUM,
            graf_kimlik: graf.kimlik.clone(),
            nodelar,
            baglantilar,
            notlar,
        }
    }

    /// Belgeyi bir grafik + parametre kümesine geri çevirir.
    pub fn grafa(&self) -> (NodeGraf, HashMap<NodeKimlik, Parametreler>) {
        let mut g = NodeGraf::yeni(self.graf_kimlik.clone());
        let mut parametreler: HashMap<NodeKimlik, Parametreler> = HashMap::new();
        for bn in &self.nodelar {
            let node = Node {
                kimlik: NodeKimlik(bn.kimlik),
                tur_kimligi: bn.tur.clone(),
                baslik: bn.baslik.clone(),
                konum: bn.konum,
                girisler: bn.girisler.iter().map(port_modeli).collect(),
                cikislar: bn.cikislar.iter().map(port_modeli).collect(),
                durum: NodeDurumu::Bekliyor,
            };
            g.node_ekle_ham(node);
            if !bn.parametreler.is_empty() {
                let mut p = Parametreler::yeni();
                for (ad, deg) in &bn.parametreler {
                    p.ayarla(ad.clone(), deg.clone());
                }
                parametreler.insert(NodeKimlik(bn.kimlik), p);
            }
        }
        for bb in &self.baglantilar {
            g.baglanti_ekle_ham(Baglanti {
                kimlik: BaglantiKimlik(bb.kimlik),
                kaynak: PortRef::yeni(NodeKimlik(bb.kaynak_node), PortYonu::Cikis, bb.kaynak_port),
                hedef: PortRef::yeni(NodeKimlik(bb.hedef_node), PortYonu::Giris, bb.hedef_port),
            });
        }
        for bn in &self.notlar {
            g.not_ekle_ham(YapiskanNot {
                kimlik: NotKimlik(bn.kimlik),
                metin: bn.metin.clone(),
                konum: bn.konum,
            });
        }
        (g, parametreler)
    }
}

fn port_dto(p: &Port) -> BcflowPort {
    BcflowPort {
        ad: p.ad.clone(),
        tur: p.veri_turu.kimlik.clone(),
    }
}
fn port_modeli(p: &BcflowPort) -> Port {
    Port {
        ad: p.ad.clone(),
        veri_turu: VeriTuru::yeni(p.tur.clone()),
    }
}

/// Eski sürümlü bir belgeyi güncel sürüme yükseltir (MK-59).
///
/// Her sürüm atlaması için burada bir adım eklenir; bilinmeyen/daha yeni sürüm reddedilir
/// ("dosyayı bu sürüm açamaz" net hata — sessiz/bozuk yükleme yok).
fn goc(mut belge: BcflowBelge) -> Result<BcflowBelge, ErrorReport> {
    if belge.surum > BCFLOW_SURUM {
        return Err(ErrorReport::new(
            "Akış dosyası bu sürümle açılamıyor",
            format!(
                "Dosya sürümü {} bu uygulamanın desteklediği en yüksek sürümden ({}) yeni.",
                belge.surum, BCFLOW_SURUM
            ),
            "BioCraft Engine'i güncelleyip tekrar deneyin.",
        ));
    }
    // Gelecekte: while belge.surum < BCFLOW_SURUM { belge = goc_adimi(belge)?; }
    // Şimdilik tek sürüm var; sürüm alanını güncelle (eski 0 → 1 gibi durumlar için).
    belge.surum = BCFLOW_SURUM;
    Ok(belge)
}

/// Akışı `.bcflow` (JSON) metnine çevirir (git-diff alınabilir, okunaklı).
pub fn bcflow_kaydet(graf: &NodeGraf, parametreler: &HashMap<NodeKimlik, Parametreler>) -> String {
    let belge = BcflowBelge::graften(graf, parametreler);
    // pretty: satır-tabanlı → küçük değişiklik = küçük diff.
    serde_json::to_string_pretty(&belge).unwrap_or_else(|_| "{}".to_string())
}

/// `.bcflow` metnini yükler (sürüm denetimi + göç + grafiğe çevirme).
pub fn bcflow_yukle(
    metin: &str,
) -> Result<(NodeGraf, HashMap<NodeKimlik, Parametreler>), ErrorReport> {
    let belge: BcflowBelge = serde_json::from_str(metin).map_err(|e| {
        ErrorReport::new(
            "Akış dosyası okunamadı",
            "Dosya geçerli bir .bcflow (JSON) değil ya da bozulmuş.",
            "Dosyanın doğru olduğundan emin olun; yedeğiniz varsa onu açın.",
        )
        .with_teknik_detay(e.to_string())
    })?;
    let belge = goc(belge)?;
    Ok(belge.grafa())
}

// ─── Ortak yerleşim (canvas ile aynı mantıksal ölçüler) ────────────────────────

const NODE_GEN: f32 = 170.0;
const BASLIK_YUK: f32 = 26.0;
const PORT_SATIR: f32 = 22.0;
const PAD: f32 = 8.0;
const NOT_GEN: f32 = 150.0;
const NOT_YUK: f32 = 90.0;
const KENAR: f32 = 30.0;

fn node_yuksekligi(n: &Node) -> f32 {
    let satir = n.girisler.len().max(n.cikislar.len()).max(1) as f32;
    BASLIK_YUK + PAD + satir * PORT_SATIR + PAD
}

fn port_konum(node_konum: (f32, f32), yon: PortYonu, indeks: usize) -> (f32, f32) {
    let y = node_konum.1 + BASLIK_YUK + PAD + (indeks as f32 + 0.5) * PORT_SATIR;
    let x = match yon {
        PortYonu::Giris => node_konum.0,
        PortYonu::Cikis => node_konum.0 + NODE_GEN,
    };
    (x, y)
}

/// Grafiğin içerik sınır kutusu (min, max) — boşsa makul varsayılan.
fn sinir(graf: &NodeGraf) -> ((f32, f32), (f32, f32)) {
    let mut var = false;
    let (mut nx, mut ny) = (f32::MAX, f32::MAX);
    let (mut xx, mut xy) = (f32::MIN, f32::MIN);
    for n in graf.nodelar() {
        var = true;
        let h = node_yuksekligi(n);
        nx = nx.min(n.konum.0);
        ny = ny.min(n.konum.1);
        xx = xx.max(n.konum.0 + NODE_GEN);
        xy = xy.max(n.konum.1 + h);
    }
    for not in graf.notlar() {
        var = true;
        nx = nx.min(not.konum.0);
        ny = ny.min(not.konum.1);
        xx = xx.max(not.konum.0 + NOT_GEN);
        xy = xy.max(not.konum.1 + NOT_YUK);
    }
    if var {
        ((nx, ny), (xx, xy))
    } else {
        ((0.0, 0.0), (400.0, 200.0))
    }
}

fn durum_renk(durum: NodeDurumu, tok: &Tokenlar) -> Color32 {
    match durum {
        NodeDurumu::Bekliyor => tok.renk.metin_soluk,
        NodeDurumu::Calisiyor => tok.renk.bilgi,
        NodeDurumu::Bitti => tok.renk.basari,
        NodeDurumu::Hata => tok.renk.hata,
    }
}

fn hex(c: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

fn xml_kacis(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\n', " ")
}

// ─── SVG dışa aktarma (tam vektör + metin) ─────────────────────────────────────

/// Akışı **SVG** (ölçeklenebilir vektör) olarak dışa aktarır — metin, renk ve kablolar dahil.
pub fn svg_disa_aktar(graf: &NodeGraf, tok: &Tokenlar, dil: Dil) -> String {
    let ((minx, miny), (maxx, maxy)) = sinir(graf);
    let w = (maxx - minx) + 2.0 * KENAR;
    let h = (maxy - miny) + 2.0 * KENAR;
    let ofs = |p: (f32, f32)| (p.0 - minx + KENAR, p.1 - miny + KENAR);

    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.0}\" height=\"{h:.0}\" viewBox=\"0 0 {w:.0} {h:.0}\">\n"
    ));
    s.push_str(&format!(
        "  <rect x=\"0\" y=\"0\" width=\"{w:.0}\" height=\"{h:.0}\" fill=\"{}\"/>\n",
        hex(tok.renk.zemin)
    ));

    // Kablolar (node'ların ardında).
    for b in graf.baglantilar() {
        let (Some(ns), Some(nh)) = (graf.node(b.kaynak.node), graf.node(b.hedef.node)) else {
            continue;
        };
        let a = ofs(port_konum(ns.konum, PortYonu::Cikis, b.kaynak.indeks));
        let z = ofs(port_konum(nh.konum, PortYonu::Giris, b.hedef.indeks));
        let renk = port_renk(graf, b.kaynak, tok);
        let dx = ((z.0 - a.0).abs().max(40.0)) * 0.5;
        s.push_str(&format!(
            "  <path d=\"M {:.1} {:.1} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>\n",
            a.0, a.1, a.0 + dx, a.1, z.0 - dx, z.1, z.0, z.1, hex(renk)
        ));
    }

    // Notlar.
    for not in graf.notlar() {
        let (x, y) = ofs(not.konum);
        s.push_str(&format!(
            "  <rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{NOT_GEN:.0}\" height=\"{NOT_YUK:.0}\" rx=\"4\" fill=\"{}\" stroke=\"{}\"/>\n",
            hex(tok.renk.uyari_zemin), hex(tok.renk.uyari)
        ));
        for (i, satir) in not.metin.lines().take(4).enumerate() {
            s.push_str(&format!(
                "  <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"sans-serif\" font-size=\"11\" fill=\"{}\">{}</text>\n",
                x + 6.0,
                y + 18.0 + i as f32 * 14.0,
                hex(tok.renk.metin),
                xml_kacis(satir)
            ));
        }
    }

    // Node'lar.
    for n in graf.nodelar() {
        let (x, y) = ofs(n.konum);
        let hgt = node_yuksekligi(n);
        let dr = durum_renk(n.durum, tok);
        s.push_str(&format!(
            "  <rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{NODE_GEN:.0}\" height=\"{hgt:.1}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>\n",
            hex(tok.renk.yuzey), hex(dr)
        ));
        s.push_str(&format!(
            "  <rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{NODE_GEN:.0}\" height=\"{BASLIK_YUK:.0}\" rx=\"6\" fill=\"{}\"/>\n",
            hex(tok.renk.yuzey_alt)
        ));
        s.push_str(&format!(
            "  <circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"4\" fill=\"{}\"/>\n",
            x + 12.0,
            y + BASLIK_YUK / 2.0,
            hex(dr)
        ));
        s.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"sans-serif\" font-size=\"12\" font-weight=\"bold\" fill=\"{}\">{}</text>\n",
            x + 24.0,
            y + BASLIK_YUK / 2.0 + 4.0,
            hex(tok.renk.metin),
            xml_kacis(&n.baslik)
        ));
        // Portlar + adlar.
        for (i, p) in n.cikislar.iter().enumerate() {
            let (px, py) = ofs(port_konum(n.konum, PortYonu::Cikis, i));
            let renk = tok.anahtar_renk(tur_renk_anahtari(&p.veri_turu));
            s.push_str(&format!(
                "  <circle cx=\"{px:.1}\" cy=\"{py:.1}\" r=\"5\" fill=\"{}\"/>\n",
                hex(renk)
            ));
            s.push_str(&format!(
                "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" font-family=\"sans-serif\" font-size=\"10\" fill=\"{}\">{}</text>\n",
                px - 9.0, py + 3.0, hex(tok.renk.metin_soluk), xml_kacis(&p.ad)
            ));
        }
        for (i, p) in n.girisler.iter().enumerate() {
            let (px, py) = ofs(port_konum(n.konum, PortYonu::Giris, i));
            let renk = tok.anahtar_renk(tur_renk_anahtari(&p.veri_turu));
            s.push_str(&format!(
                "  <circle cx=\"{px:.1}\" cy=\"{py:.1}\" r=\"5\" fill=\"{}\"/>\n",
                hex(renk)
            ));
            s.push_str(&format!(
                "  <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"sans-serif\" font-size=\"10\" fill=\"{}\">{}</text>\n",
                px + 9.0, py + 3.0, hex(tok.renk.metin_soluk), xml_kacis(&p.ad)
            ));
        }
    }

    let _ = dil; // metinler doğrudan node başlıklarından (zaten yerel); ileride yerel başlık eki.
    s.push_str("</svg>\n");
    s
}

fn port_renk(graf: &NodeGraf, r: PortRef, tok: &Tokenlar) -> Color32 {
    graf.port_coz(r)
        .map(|p| tok.anahtar_renk(tur_renk_anahtari(&p.veri_turu)))
        .unwrap_or(tok.renk.metin_soluk)
}

// ─── PNG dışa aktarma (saf-Rust raster + minimal kodlayıcı) ────────────────────

/// Akışı **PNG** (raster küçük resim) olarak dışa aktarır — kutular, durum halkaları, portlar
/// ve kablolar.  Glyph fontu gömmez (metin etiketleri SVG'dedir); yapısal/durum önizlemesidir.
/// Dış bağımlılık YOK: kendi minimal PNG kodlayıcısını kullanır ([`png`]).
pub fn png_disa_aktar(graf: &NodeGraf, tok: &Tokenlar, olcek: f32) -> Vec<u8> {
    let ((minx, miny), (maxx, maxy)) = sinir(graf);
    let olcek = olcek.clamp(0.3, 3.0);
    let w = (((maxx - minx) + 2.0 * KENAR) * olcek).ceil().max(16.0) as usize;
    let h = (((maxy - miny) + 2.0 * KENAR) * olcek).ceil().max(16.0) as usize;
    let mut r = Raster::yeni(w.min(8192), h.min(8192));
    r.temizle(tok.renk.zemin);

    let ofs = |p: (f32, f32)| ((p.0 - minx + KENAR) * olcek, (p.1 - miny + KENAR) * olcek);

    // Kablolar.
    for b in graf.baglantilar() {
        let (Some(ns), Some(nh)) = (graf.node(b.kaynak.node), graf.node(b.hedef.node)) else {
            continue;
        };
        let a = ofs(port_konum(ns.konum, PortYonu::Cikis, b.kaynak.indeks));
        let z = ofs(port_konum(nh.konum, PortYonu::Giris, b.hedef.indeks));
        let renk = port_renk(graf, b.kaynak, tok);
        let dx = ((z.0 - a.0).abs().max(40.0 * olcek)) * 0.5;
        r.bezier(a, (a.0 + dx, a.1), (z.0 - dx, z.1), z, renk);
    }

    // Notlar.
    for not in graf.notlar() {
        let (x, y) = ofs(not.konum);
        r.dortgen_dolu(x, y, NOT_GEN * olcek, NOT_YUK * olcek, tok.renk.uyari_zemin);
        r.dortgen_kenar(x, y, NOT_GEN * olcek, NOT_YUK * olcek, tok.renk.uyari);
    }

    // Node'lar.
    for n in graf.nodelar() {
        let (x, y) = ofs(n.konum);
        let nw = NODE_GEN * olcek;
        let nh = node_yuksekligi(n) * olcek;
        let dr = durum_renk(n.durum, tok);
        r.dortgen_dolu(x, y, nw, nh, tok.renk.yuzey);
        r.dortgen_dolu(x, y, nw, BASLIK_YUK * olcek, tok.renk.yuzey_alt);
        r.dortgen_kenar(x, y, nw, nh, dr); // durum halkası
        r.daire_dolu(
            x + 12.0 * olcek,
            y + BASLIK_YUK * 0.5 * olcek,
            4.0 * olcek,
            dr,
        );
        for (i, p) in n.cikislar.iter().enumerate() {
            let (px, py) = ofs(port_konum(n.konum, PortYonu::Cikis, i));
            r.daire_dolu(
                px,
                py,
                5.0 * olcek,
                tok.anahtar_renk(tur_renk_anahtari(&p.veri_turu)),
            );
        }
        for (i, p) in n.girisler.iter().enumerate() {
            let (px, py) = ofs(port_konum(n.konum, PortYonu::Giris, i));
            r.daire_dolu(
                px,
                py,
                5.0 * olcek,
                tok.anahtar_renk(tur_renk_anahtari(&p.veri_turu)),
            );
        }
    }

    png::kodla(&r)
}

/// Basit RGBA8 yazılım rasterizer'ı (alfa harmanlamalı çizim ilkelleri).
struct Raster {
    w: usize,
    h: usize,
    /// RGBA, satır satır (row-major).
    piksel: Vec<[u8; 4]>,
}

impl Raster {
    fn yeni(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            piksel: vec![[0, 0, 0, 255]; w * h],
        }
    }
    fn temizle(&mut self, c: Color32) {
        let p = [c.r(), c.g(), c.b(), 255];
        for px in &mut self.piksel {
            *px = p;
        }
    }
    fn nokta(&mut self, x: i32, y: i32, c: Color32) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        let idx = y as usize * self.w + x as usize;
        // Opak çizim (token renkleri tam opak; alfa harmanlama gerekmez).
        self.piksel[idx] = [c.r(), c.g(), c.b(), 255];
    }
    fn dortgen_dolu(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color32) {
        let x0 = x.round() as i32;
        let y0 = y.round() as i32;
        let x1 = (x + w).round() as i32;
        let y1 = (y + h).round() as i32;
        for yy in y0..y1 {
            for xx in x0..x1 {
                self.nokta(xx, yy, c);
            }
        }
    }
    fn dortgen_kenar(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color32) {
        let x0 = x.round() as i32;
        let y0 = y.round() as i32;
        let x1 = (x + w).round() as i32;
        let y1 = (y + h).round() as i32;
        for xx in x0..x1 {
            self.nokta(xx, y0, c);
            self.nokta(xx, y0 + 1, c);
            self.nokta(xx, y1 - 1, c);
            self.nokta(xx, y1 - 2, c);
        }
        for yy in y0..y1 {
            self.nokta(x0, yy, c);
            self.nokta(x0 + 1, yy, c);
            self.nokta(x1 - 1, yy, c);
            self.nokta(x1 - 2, yy, c);
        }
    }
    fn daire_dolu(&mut self, cx: f32, cy: f32, r: f32, c: Color32) {
        let r = r.max(1.0);
        let r2 = r * r;
        let x0 = (cx - r).floor() as i32;
        let x1 = (cx + r).ceil() as i32;
        let y0 = (cy - r).floor() as i32;
        let y1 = (cy + r).ceil() as i32;
        for yy in y0..=y1 {
            for xx in x0..=x1 {
                let dx = xx as f32 + 0.5 - cx;
                let dy = yy as f32 + 0.5 - cy;
                if dx * dx + dy * dy <= r2 {
                    self.nokta(xx, yy, c);
                }
            }
        }
    }
    fn cizgi(&mut self, a: (f32, f32), b: (f32, f32), c: Color32) {
        // Kalın çizgi için 2×2 fırça ile Bresenham benzeri örnekleme.
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let adim = dx.abs().max(dy.abs()).max(1.0).ceil() as i32;
        for i in 0..=adim {
            let t = i as f32 / adim as f32;
            let x = a.0 + dx * t;
            let y = a.1 + dy * t;
            let xi = x.round() as i32;
            let yi = y.round() as i32;
            self.nokta(xi, yi, c);
            self.nokta(xi + 1, yi, c);
            self.nokta(xi, yi + 1, c);
        }
    }
    fn bezier(
        &mut self,
        p0: (f32, f32),
        p1: (f32, f32),
        p2: (f32, f32),
        p3: (f32, f32),
        c: Color32,
    ) {
        const N: usize = 28;
        let mut onceki = p0;
        for i in 1..=N {
            let t = i as f32 / N as f32;
            let u = 1.0 - t;
            let x = u * u * u * p0.0
                + 3.0 * u * u * t * p1.0
                + 3.0 * u * t * t * p2.0
                + t * t * t * p3.0;
            let y = u * u * u * p0.1
                + 3.0 * u * u * t * p1.1
                + 3.0 * u * t * t * p2.1
                + t * t * t * p3.1;
            self.cizgi(onceki, (x, y), c);
            onceki = (x, y);
        }
    }
}

/// Minimal, bağımlılıksız PNG kodlayıcısı (8-bit RGBA, sıkıştırmasız "stored" deflate).
///
/// `.bcproj` için yazılan saf-Rust minimal ZIP "stored" yazıcısıyla aynı ruh (İP-02): standart
/// biçimi kendi başımıza, denetlenebilir biçimde üretiriz (CRC-32 + Adler-32 + zlib zarfı).
mod png {
    use super::Raster;

    const IMZA: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    /// Raster'ı geçerli bir PNG bayt dizisine kodlar.
    pub fn kodla(r: &Raster) -> Vec<u8> {
        let mut cikti = Vec::new();
        cikti.extend_from_slice(&IMZA);

        // IHDR
        let mut ihdr = Vec::with_capacity(13);
        ihdr.extend_from_slice(&(r.w as u32).to_be_bytes());
        ihdr.extend_from_slice(&(r.h as u32).to_be_bytes());
        ihdr.push(8); // bit derinliği
        ihdr.push(6); // renk türü: RGBA
        ihdr.push(0); // sıkıştırma: deflate
        ihdr.push(0); // filtre: standart
        ihdr.push(0); // interlace: yok
        parca_yaz(&mut cikti, b"IHDR", &ihdr);

        // IDAT: ham görüntü verisi (her satır başında filtre baytı 0) → zlib(stored).
        let mut ham = Vec::with_capacity(r.h * (1 + r.w * 4));
        for y in 0..r.h {
            ham.push(0); // filtre: None
            for x in 0..r.w {
                ham.extend_from_slice(&r.piksel[y * r.w + x]);
            }
        }
        let zlib = zlib_stored(&ham);
        parca_yaz(&mut cikti, b"IDAT", &zlib);

        // IEND
        parca_yaz(&mut cikti, b"IEND", &[]);
        cikti
    }

    fn parca_yaz(cikti: &mut Vec<u8>, tur: &[u8; 4], veri: &[u8]) {
        cikti.extend_from_slice(&(veri.len() as u32).to_be_bytes());
        cikti.extend_from_slice(tur);
        cikti.extend_from_slice(veri);
        let mut crc_girdi = Vec::with_capacity(4 + veri.len());
        crc_girdi.extend_from_slice(tur);
        crc_girdi.extend_from_slice(veri);
        cikti.extend_from_slice(&crc32(&crc_girdi).to_be_bytes());
    }

    /// zlib akışı: başlık (0x78 0x01) + sıkıştırmasız deflate blokları + Adler-32.
    fn zlib_stored(veri: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(0x78); // CMF: 32K pencere, deflate
        out.push(0x01); // FLG: en hızlı, sıkıştırma yok
                        // Deflate "stored" blokları (her biri ≤ 65535 bayt).
        let mut i = 0;
        if veri.is_empty() {
            // Tek boş son blok.
            out.push(0x01);
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&(!0u16).to_le_bytes());
        }
        while i < veri.len() {
            let kalan = veri.len() - i;
            let blok = kalan.min(0xFFFF);
            let son = i + blok >= veri.len();
            out.push(if son { 0x01 } else { 0x00 }); // BFINAL + BTYPE(00 stored)
            out.extend_from_slice(&(blok as u16).to_le_bytes());
            out.extend_from_slice(&(!(blok as u16)).to_le_bytes());
            out.extend_from_slice(&veri[i..i + blok]);
            i += blok;
        }
        out.extend_from_slice(&adler32(veri).to_be_bytes());
        out
    }

    /// Standart CRC-32 (IEEE, PNG/ZIP ile aynı; poli 0xEDB88320).
    fn crc32(veri: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFF_FFFF;
        for &b in veri {
            crc ^= b as u32;
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
        !crc
    }

    /// Adler-32 (zlib bütünlük damgası).
    fn adler32(veri: &[u8]) -> u32 {
        const MOD: u32 = 65521;
        let mut a: u32 = 1;
        let mut b: u32 = 0;
        for &x in veri {
            a = (a + x as u32) % MOD;
            b = (b + a) % MOD;
        }
        (b << 16) | a
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn crc32_bilinen_deger() {
            // "IEND" + boş veri CRC'si bilinen sabit (0xAE426082).
            assert_eq!(crc32(b"IEND"), 0xAE42_6082);
        }

        #[test]
        fn adler32_bilinen_deger() {
            // Adler-32("") = 1.
            assert_eq!(adler32(b""), 1);
            // Adler-32("abc") = 0x024D0127.
            assert_eq!(adler32(b"abc"), 0x024D_0127);
        }

        #[test]
        fn kucuk_raster_gecerli_png() {
            let r = Raster::yeni(3, 2);
            let png = kodla(&r);
            assert_eq!(&png[..8], &IMZA);
            // IHDR/IDAT/IEND parça türleri bulunmalı.
            let pencere = |adet: &[u8]| png.windows(4).any(|w| w == adet);
            assert!(pencere(b"IHDR"));
            assert!(pencere(b"IDAT"));
            assert!(pencere(b"IEND"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::katalog::NodeKatalogu;

    fn ornek_graf() -> NodeGraf {
        let katalog = NodeKatalogu::ornek();
        let mut g = NodeGraf::yeni("ana");
        let ekle = |g: &mut NodeGraf, tur: &str, konum: (f32, f32)| -> NodeKimlik {
            let k = g.yeni_node_kimlik();
            g.node_ekle_ham(katalog.bul(tur).unwrap().ornekle(k, konum));
            k
        };
        let a = ekle(&mut g, "girdi.dizi_oku", (40.0, 60.0));
        let b = ekle(&mut g, "isle.hizala", (260.0, 60.0));
        let bk = g.yeni_baglanti_kimlik();
        g.baglanti_ekle_ham(Baglanti {
            kimlik: bk,
            kaynak: PortRef::yeni(a, PortYonu::Cikis, 0),
            hedef: PortRef::yeni(b, PortYonu::Giris, 0),
        });
        let nk = g.yeni_not_kimlik();
        g.not_ekle_ham(YapiskanNot {
            kimlik: nk,
            metin: "Demo akış".into(),
            konum: (40.0, 200.0),
        });
        g
    }

    #[test]
    fn bcflow_gidis_donus_korur() {
        let g = ornek_graf();
        let mut pars: HashMap<NodeKimlik, Parametreler> = HashMap::new();
        let mut p = Parametreler::yeni();
        p.ayarla("esik", ParametreDeger::TamSayi(12));
        pars.insert(g.nodelar()[1].kimlik, p);

        let metin = bcflow_kaydet(&g, &pars);
        assert!(metin.contains("\"surum\""));
        let (g2, pars2) = bcflow_yukle(&metin).unwrap();
        assert_eq!(g2.nodelar().len(), g.nodelar().len());
        assert_eq!(g2.baglantilar().len(), g.baglantilar().len());
        assert_eq!(g2.notlar().len(), g.notlar().len());
        // Parametre korunmalı.
        let k = g2.nodelar()[1].kimlik;
        assert_eq!(pars2[&k].tam_sayi("esik"), Some(12));
        // Tür/başlık/konum korunmalı.
        assert_eq!(g2.nodelar()[0].tur_kimligi, g.nodelar()[0].tur_kimligi);
        assert_eq!(g2.nodelar()[0].konum, g.nodelar()[0].konum);
    }

    #[test]
    fn daha_yeni_surum_reddedilir() {
        let g = ornek_graf();
        let mut metin = bcflow_kaydet(&g, &HashMap::new());
        // Sürümü elle yapay olarak yükselt.
        metin = metin.replace("\"surum\": 1", "\"surum\": 9999");
        let sonuc = bcflow_yukle(&metin);
        assert!(sonuc.is_err(), "daha yeni sürüm açılmamalı (MK-59)");
    }

    #[test]
    fn bozuk_bcflow_net_hata() {
        let sonuc = bcflow_yukle("bu json değil {");
        assert!(sonuc.is_err());
    }

    #[test]
    fn svg_temel_ogeleri_icerir() {
        let g = ornek_graf();
        let svg = svg_disa_aktar(&g, &Tokenlar::koyu(), Dil::Tr);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        // Node başlığı + kablo yolu + port dairesi içermeli.
        assert!(svg.contains("Hizala"));
        assert!(svg.contains("<path"));
        assert!(svg.contains("<circle"));
    }

    #[test]
    fn png_gecerli_imza_ve_makul_boyut() {
        let g = ornek_graf();
        let png = png_disa_aktar(&g, &Tokenlar::koyu(), 1.0);
        assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        assert!(png.len() > 100, "boş olmayan PNG");
    }

    #[test]
    fn bos_graf_dahi_dısa_aktarılır() {
        let g = NodeGraf::yeni("bos");
        let svg = svg_disa_aktar(&g, &Tokenlar::acik(), Dil::En);
        assert!(svg.contains("</svg>"));
        let png = png_disa_aktar(&g, &Tokenlar::acik(), 1.0);
        assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }
}
