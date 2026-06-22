//! Node tuvali — egui çizimi + etkileşim (İP-05, MK-54).
//!
//! Unreal Blueprint mantığında görsel akış tuvali:
//! - **Tuval:** pan (boş alanı / orta tuş ile sürükle), zoom (tekerlek; imleç sabit), "tümünü sığdır",
//!   minimap (genel bakış + görünür alan + tıkla-ortala), arka plan ızgarası.
//! - **Node ekleme:** sağ-tık → aranabilir palet; araç çubuğundan **＋Node**.
//! - **Portlar:** tipli & renkli; çıkıştan sürükleyerek bağla; sürükleme sırasında **uyumlu girişler
//!   vurgulanır**; uyumsuz bağlantı **reddedilir** (anlık uyarı + varsa dönüştürücü önerisi).
//! - **DAG:** döngü oluşturan bağlantı reddedilir ([`super::dag`]).
//! - **Durum halkası:** her node bekliyor/çalışıyor/bitti/hata rengiyle çevrelenir.
//! - **Undo/redo:** ekleme/bağlama/taşıma/silme [`biocraft_state::GeriAlYigini`] ile geri alınır.
//!
//! ## Performans (MK-04 / İP-04)
//! Görünür alan dışındaki node'lar **çizilmez** (culling); düşük zoom'da ayrıntı azaltılır (LOD:
//! port/metin atlanır) → büyük grafikte bile kare bütçesi korunur, kasma olmaz.
// MK-52: tüm renkler token'dan.  MK-53: tüm metinler i18n'den / yerel.

use std::collections::HashMap;
use std::sync::mpsc;

use biocraft_mem::BellekOrkestratoru;
use biocraft_sdk::node::Parametreler;
use biocraft_state::command::Komut;
use biocraft_state::GeriAlYigini;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use crate::i18n::Dil;
use crate::tokens::Tokenlar;

use super::cache::SonucOnbellek;
use super::commands::{
    BaglantiEkleKomut, BaglantiSilKomut, NodeEkleKomut, NodeSilKomut, NodeTasiKomut, NotEkleKomut,
    NotSilKomut,
};
use super::graph::{
    Baglanti, BaglantiKimlik, BaglantiKontrol, NodeDurumu, NodeGraf, NodeKimlik, NotKimlik,
    PortRef, YapiskanNot,
};
use super::katalog::{ornek_donusturucu_kayit, NodeKatalogu};
use super::kod::python_disa_aktar;
use super::port::{tur_renk_anahtari, DonusturucuKayit, PortYonu};
use super::run::{self, CalismaAyari, CalismaSonucu, IlerlemeOlay, IptalJetonu, YurutucuKayit};
use super::serialize;

/// Canvas'ın varsayılan bellek bütçesi (1 GiB) — app kendi orkestratörünü enjekte edebilir.
const VARSAYILAN_BUTCE: u64 = 1024 * 1024 * 1024;

/// Arka planda süren bir çalıştırma işi (arayüz donmaz — MK-07: pull-based, kare bütçeli).
struct CalismaIsi {
    ilerleme_rx: mpsc::Receiver<IlerlemeOlay>,
    sonuc_rx: mpsc::Receiver<(CalismaSonucu, SonucOnbellek)>,
    iptal: IptalJetonu,
    toplam: usize,
    tamamlanan: usize,
    // Thread tutamağı: bırakılınca thread ayrılır (sonuç kanaldan gelir).
    _handle: std::thread::JoinHandle<()>,
}

// ── Mantıksal (zoom'dan bağımsız) ölçüler ──────────────────────────────────────
const NODE_GEN: f32 = 170.0;
const BASLIK_YUK: f32 = 26.0;
const PORT_SATIR: f32 = 22.0;
const PORT_R: f32 = 5.0;
const PAD: f32 = 8.0;
const IZGARA: f32 = 40.0;
const NOT_GEN: f32 = 150.0;
const NOT_YUK: f32 = 90.0;

const ZOOM_MIN: f32 = 0.25;
const ZOOM_MAX: f32 = 2.5;
/// Bu zoom'un altında ayrıntı (port/metin) çizilmez (LOD).
const LOD_ESIK: f32 = 0.55;
/// Uyarı şeridinin ekranda kalma süresi (sn).
const UYARI_SURE: f64 = 3.5;

/// Sağ-tık paletinin durumu.
#[derive(Debug, Clone)]
struct PaletDurum {
    /// Node'un ekleneceği mantıksal konum.
    mantiksal: (f32, f32),
    /// Arama metni.
    arama: String,
}

/// Çizim sırasında toplanan, çizimden **sonra** uygulanan geri-alınabilir eylemler.
///
/// (egui immediate-mode'da grafiği okurken aynı anda undo-komutu çalıştırmamak için ertelenir.)
enum Eylem {
    NodeEkle {
        tur_kimligi: String,
        konum: (f32, f32),
    },
    NodeSil(NodeKimlik),
    NodeTasi {
        kimlik: NodeKimlik,
        eski: (f32, f32),
        yeni: (f32, f32),
    },
    Baglan {
        kaynak: PortRef,
        hedef: PortRef,
    },
    BaglantiSil(BaglantiKimlik),
    NotEkle {
        konum: (f32, f32),
        metin: String,
    },
    NotSil(NotKimlik),
}

/// Node tabanlı görsel akış tuvali (tek grafik + kendi undo geçmişi).
pub struct NodeTuvali {
    /// Düzenlenen grafik.
    pub graf: NodeGraf,
    gecmis: GeriAlYigini<NodeGraf>,
    katalog: NodeKatalogu,
    donusturucu: DonusturucuKayit,

    // Görünüm dönüşümü.
    pan: Vec2,
    zoom: f32,

    // Etkileşim durumu.
    secili_node: Option<NodeKimlik>,
    /// Taşınan node + taşımaya başlarkenki (komut için) özgün konumu.
    surukle: Option<(NodeKimlik, (f32, f32))>,
    /// Taşınan not + özgün konumu.
    not_surukle: Option<(NotKimlik, (f32, f32))>,
    /// Bağlantı sürükleniyorsa, sürüklemenin başladığı çıkış portu.
    baglanti_kaynak: Option<PortRef>,
    /// Açık sağ-tık paleti.
    palet: Option<PaletDurum>,
    /// Anlık uyarı (metin + ne zaman ayarlandığı).
    uyari: Option<(String, f64)>,

    // ── Çalıştırma (Gün 21) ────────────────────────────────────────────────
    /// Node başına parametre değerleri (`.bcflow`'da saklanır, çalıştırmada okunur).
    parametreler: HashMap<NodeKimlik, Parametreler>,
    /// Tür kimliğinden çalıştırıcıya eşleme (eklenti SDK node'ları buraya kaydolur).
    kayit: YurutucuKayit,
    /// Sonuç önbelleği (değişmeyen node yeniden hesaplanmaz).
    onbellek: SonucOnbellek,
    /// Bellek bütçesi orkestratörü (paralel çalıştırma rezervasyonu — OOM koruması).
    orkestrator: BellekOrkestratoru,
    /// Canlı mod: yapısal değişiklikte otomatik çalıştır (ağır node varsa uyarı).
    canli_mod: bool,
    /// Süren arka plan çalıştırması (varsa).
    is: Option<CalismaIsi>,
    /// Son çalıştırmanın tam sonucu (kablo önizlemesi + özet).
    son_sonuc: Option<CalismaSonucu>,
    /// Önizleme için seçili kablo.
    secili_baglanti: Option<BaglantiKimlik>,
    /// Canlı modda yeniden çalıştırma gereği (yapı değişti).
    canli_kirli: bool,
}

impl Default for NodeTuvali {
    fn default() -> Self {
        Self::yeni("ana")
    }
}

impl NodeTuvali {
    /// Boş tuval (verilen grafik kimliğiyle).
    pub fn yeni(graf_kimlik: impl Into<String>) -> Self {
        Self {
            graf: NodeGraf::yeni(graf_kimlik),
            gecmis: GeriAlYigini::yeni(),
            katalog: NodeKatalogu::ornek(),
            donusturucu: ornek_donusturucu_kayit(),
            pan: Vec2::new(40.0, 40.0),
            zoom: 1.0,
            secili_node: None,
            surukle: None,
            not_surukle: None,
            baglanti_kaynak: None,
            palet: None,
            uyari: None,
            parametreler: HashMap::new(),
            kayit: YurutucuKayit::ornek(),
            onbellek: SonucOnbellek::yeni(),
            orkestrator: BellekOrkestratoru::yeni(VARSAYILAN_BUTCE),
            canli_mod: false,
            is: None,
            son_sonuc: None,
            secili_baglanti: None,
            canli_kirli: false,
        }
    }

    /// Örnek (demo) tuval: küçük bir genomik akış + bir not, ilk açılışta dolu görünür.
    pub fn ornek() -> Self {
        let mut t = Self::yeni("ana");
        let oku = t.node_ekle_dogrudan("girdi.dizi_oku", (40.0, 60.0));
        let hizala = t.node_ekle_dogrudan("isle.hizala", (260.0, 60.0));
        let varyant = t.node_ekle_dogrudan("isle.varyant_cagir", (480.0, 60.0));
        // Uyumlu zincir: dizi → hizalama → varyant.
        t.baglanti_dogrudan(oku, 0, hizala, 0);
        t.baglanti_dogrudan(hizala, 0, varyant, 0);
        let nk = t.graf.yeni_not_kimlik();
        t.graf.not_ekle_ham(YapiskanNot {
            kimlik: nk,
            metin: "Akışı buradan başlatın:\nDizi → Hizala → Varyant".into(),
            konum: (40.0, 220.0),
        });
        // Demo durum halkaları (görsel): ilk node bitti.
        t.graf.durum_ayarla(oku, NodeDurumu::Bitti);
        t
    }

    /// Katalogtan, geçmişe yazmadan doğrudan bir node ekler (demo/seed kurulumu için).
    fn node_ekle_dogrudan(&mut self, tur_kimligi: &str, konum: (f32, f32)) -> NodeKimlik {
        let kimlik = self.graf.yeni_node_kimlik();
        if let Some(g) = self.katalog.bul(tur_kimligi) {
            self.graf.node_ekle_ham(g.ornekle(kimlik, konum));
        }
        kimlik
    }

    /// Demo bağlantısı (doğrulamasız, seed için).
    fn baglanti_dogrudan(&mut self, kaynak: NodeKimlik, ci: usize, hedef: NodeKimlik, gi: usize) {
        let bk = self.graf.yeni_baglanti_kimlik();
        self.graf.baglanti_ekle_ham(Baglanti {
            kimlik: bk,
            kaynak: PortRef::yeni(kaynak, PortYonu::Cikis, ci),
            hedef: PortRef::yeni(hedef, PortYonu::Giris, gi),
        });
    }

    // ── Geçmiş (undo/redo) erişimi ─────────────────────────────────────────

    /// Geri alınabilir bir işlem var mı?
    pub fn geri_alinabilir_mi(&self) -> bool {
        self.gecmis.geri_alinabilir_mi()
    }
    /// Yinelenebilir bir işlem var mı?
    pub fn yinelenebilir_mi(&self) -> bool {
        self.gecmis.yinelenebilir_mi()
    }
    /// Son işlemi geri alır.
    pub fn geri_al(&mut self) {
        let _ = self.gecmis.geri_al(&mut self.graf);
    }
    /// Son geri alınan işlemi yineler.
    pub fn yinele(&mut self) {
        let _ = self.gecmis.yinele(&mut self.graf);
    }

    fn komut(&mut self, k: Box<dyn Komut<NodeGraf>>) {
        // Komutlar yalnızca önceden doğrulanmış işlemler için kurulur; hata beklenmez.
        let _ = self.gecmis.calistir(&mut self.graf, k);
    }

    fn uyar(&mut self, ctx: &egui::Context, metin: impl Into<String>) {
        let now = ctx.input(|i| i.time);
        self.uyari = Some((metin.into(), now));
    }

    // ── Çalıştırma (paralel + bütçeli + önbellekli; arayüz donmaz) ─────────

    /// Bir eklenti/çekirdek node kaydı ekler (eklenti SDK entegrasyon noktası).
    pub fn node_kaydet(&mut self, kayit: biocraft_sdk::node::NodeKaydi) {
        self.kayit.kaydet(kayit);
    }

    /// App'in paylaşılan bellek orkestratörünü enjekte eder (yoksa varsayılan kullanılır).
    pub fn orkestrator_ayarla(&mut self, orkestrator: BellekOrkestratoru) {
        self.orkestrator = orkestrator;
    }

    /// Canlı mod açık mı?
    pub fn canli_mod(&self) -> bool {
        self.canli_mod
    }

    /// Bir çalıştırma şu an sürüyor mu?
    pub fn calisiyor_mu(&self) -> bool {
        self.is.is_some()
    }

    /// Akışta ağır (pahalı) node var mı? (Canlı mod uyarısı için.)
    fn agir_node_var(&self) -> bool {
        self.graf.nodelar().iter().any(|n| {
            self.kayit
                .al(&n.tur_kimligi)
                .map(|k| k.yurutucu.agir_mi())
                .unwrap_or(false)
        })
    }

    /// **Akışı arka planda çalıştırır** (arayüz donmaz).  Zaten süren iş varsa yok sayar.
    pub fn calistir_baslat(&mut self) {
        if self.is.is_some() {
            return;
        }
        self.canli_kirli = false;
        // Başlangıçta tüm durumları "bekliyor" yap (görsel sıfırlama).
        let kimlikler: Vec<NodeKimlik> = self.graf.nodelar().iter().map(|n| n.kimlik).collect();
        for k in &kimlikler {
            self.graf.durum_ayarla(*k, NodeDurumu::Bekliyor);
        }

        let graf = self.graf.clone();
        let kayit = self.kayit.clone();
        let pars = self.parametreler.clone();
        let ork = self.orkestrator.clone();
        let mut onb = self.onbellek.clone();
        let iptal = IptalJetonu::yeni();
        let iptal_thr = iptal.clone();
        let toplam = graf.nodelar().len();
        let ayar = CalismaAyari {
            canli_mod: self.canli_mod,
            ..CalismaAyari::default()
        };
        let (itx, irx) = mpsc::channel();
        let (stx, srx) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            let mut ilerleme = |o: IlerlemeOlay| {
                let _ = itx.send(o);
            };
            let sonuc = run::calistir(
                &graf,
                &kayit,
                &pars,
                &ork,
                &mut onb,
                &ayar,
                &iptal_thr,
                &mut ilerleme,
            );
            let _ = stx.send((sonuc, onb));
        });
        self.is = Some(CalismaIsi {
            ilerleme_rx: irx,
            sonuc_rx: srx,
            iptal,
            toplam,
            tamamlanan: 0,
            _handle: handle,
        });
    }

    /// **Akışı senkron çalıştırır** (arka plan thread'siz) — basit kullanım + testler için.
    /// Sonucu grafiğe uygular ve döndürür.  İş sürüyorsa bir şey yapmaz.
    pub fn calistir_simdi(&mut self) -> &Option<CalismaSonucu> {
        if self.is.is_some() {
            return &self.son_sonuc;
        }
        let ayar = CalismaAyari {
            canli_mod: self.canli_mod,
            ..CalismaAyari::default()
        };
        let mut ilerleme = |_o: IlerlemeOlay| {};
        let sonuc = run::calistir(
            &self.graf,
            &self.kayit,
            &self.parametreler,
            &self.orkestrator,
            &mut self.onbellek,
            &ayar,
            &IptalJetonu::yeni(),
            &mut ilerleme,
        );
        sonuc.durumu_uygula(&mut self.graf);
        self.son_sonuc = Some(sonuc);
        self.canli_kirli = false;
        &self.son_sonuc
    }

    /// Bir node'un parametre değerini ayarlar (çalıştırmayı + önbelleği etkiler).
    pub fn parametre_ayarla(
        &mut self,
        node: NodeKimlik,
        ad: impl Into<String>,
        deger: biocraft_sdk::node::ParametreDeger,
    ) {
        self.parametreler.entry(node).or_default().ayarla(ad, deger);
        self.canli_kirli = true;
    }

    /// Süren çalıştırmayı iptal eder.
    pub fn calistirmayi_iptal(&mut self) {
        if let Some(is) = &self.is {
            is.iptal.iptal_et();
        }
    }

    /// Arka plan işini yoklar: ilerleme olaylarını uygular, bitince sonucu alır (kare başına).
    fn calistirmayi_yokla(&mut self, ctx: &egui::Context) {
        let mut olaylar: Vec<(NodeKimlik, NodeDurumu)> = Vec::new();
        let mut bitti: Option<(CalismaSonucu, SonucOnbellek)> = None;
        if let Some(is) = self.is.as_mut() {
            while let Ok(o) = is.ilerleme_rx.try_recv() {
                is.tamamlanan = o.tamamlanan;
                olaylar.push((o.node, o.durum));
            }
            if let Ok(r) = is.sonuc_rx.try_recv() {
                bitti = Some(r);
            }
        }
        for (node, durum) in olaylar {
            self.graf.durum_ayarla(node, durum);
        }
        if let Some((sonuc, onb)) = bitti {
            sonuc.durumu_uygula(&mut self.graf);
            self.onbellek = onb;
            self.son_sonuc = Some(sonuc);
            self.is = None;
        }
        if self.is.is_some() {
            ctx.request_repaint(); // iş sürerken kareleri zorla (pull-based ilerleme).
        }
    }

    // ── Dışa aktarma (kaydet/görsel) ──────────────────────────────────────

    /// Akışı `.bcflow` (JSON) metni olarak döndürür.
    pub fn bcflow_metni(&self) -> String {
        serialize::bcflow_kaydet(&self.graf, &self.parametreler)
    }

    /// `.bcflow` metninden akışı yükler (mevcut grafı + parametreleri + geçmişi değiştirir).
    pub fn bcflow_yukle(&mut self, metin: &str) -> Result<(), biocraft_types::ErrorReport> {
        let (graf, pars) = serialize::bcflow_yukle(metin)?;
        self.graf = graf;
        self.parametreler = pars;
        self.gecmis = GeriAlYigini::yeni();
        self.onbellek.temizle();
        self.son_sonuc = None;
        self.secili_node = None;
        self.secili_baglanti = None;
        Ok(())
    }

    /// Akışın SVG (vektör) dışa aktarımı.
    pub fn svg_metni(&self, tok: &Tokenlar, dil: Dil) -> String {
        serialize::svg_disa_aktar(&self.graf, tok, dil)
    }

    /// Akışın PNG (raster) dışa aktarımı.
    pub fn png_baytlari(&self, tok: &Tokenlar, olcek: f32) -> Vec<u8> {
        serialize::png_disa_aktar(&self.graf, tok, olcek)
    }

    /// Akışın eşdeğer Python betiği.
    pub fn python_metni(&self) -> String {
        python_disa_aktar(&self.graf, &self.parametreler)
    }

    /// Node parametreleri (node ↔ kod köprüsü için — "akışı kod olarak aç").
    pub fn parametreler(&self) -> &HashMap<NodeKimlik, Parametreler> {
        &self.parametreler
    }

    /// Son çalıştırma sonucu (varsa) — köprü, node çıktılarını workspace'e taşır.
    pub fn son_sonuc(&self) -> Option<&CalismaSonucu> {
        self.son_sonuc.as_ref()
    }

    /// Pointer'a en yakın kabloyu (eşik içinde) döndürür — kablo önizlemesi için.
    fn kablo_vurus(&self, rect: Rect, pointer: Option<Pos2>) -> Option<BaglantiKimlik> {
        let p = pointer?;
        let mut en_yakin: Option<(BaglantiKimlik, f32)> = None;
        for b in self.graf.baglantilar() {
            let (Some(ns), Some(nh)) =
                (self.graf.node(b.kaynak.node), self.graf.node(b.hedef.node))
            else {
                continue;
            };
            let a = self.l2s(
                rect,
                Self::port_konum(ns.konum, b.kaynak.yon, b.kaynak.indeks),
            );
            let z = self.l2s(
                rect,
                Self::port_konum(nh.konum, b.hedef.yon, b.hedef.indeks),
            );
            let dx = (z.x - a.x).abs().max(40.0) * 0.5;
            let c1 = Pos2::new(a.x + dx, a.y);
            let c2 = Pos2::new(z.x - dx, z.y);
            // Bezier'i örnekle, en yakın segment mesafesini bul.
            let mut onceki = a;
            for i in 1..=20 {
                let t = i as f32 / 20.0;
                let u = 1.0 - t;
                let nx = u * u * u * a.x
                    + 3.0 * u * u * t * c1.x
                    + 3.0 * u * t * t * c2.x
                    + t * t * t * z.x;
                let ny = u * u * u * a.y
                    + 3.0 * u * u * t * c1.y
                    + 3.0 * u * t * t * c2.y
                    + t * t * t * z.y;
                let nokta = Pos2::new(nx, ny);
                let d = nokta_segment_mesafe(p, onceki, nokta);
                if d < 6.0 && en_yakin.map(|(_, ed)| d < ed).unwrap_or(true) {
                    en_yakin = Some((b.kimlik, d));
                }
                onceki = nokta;
            }
        }
        en_yakin.map(|(k, _)| k)
    }

    // ── Koordinat dönüşümleri ──────────────────────────────────────────────

    fn origin(&self, rect: Rect) -> Pos2 {
        rect.min + self.pan
    }
    fn l2s(&self, rect: Rect, p: (f32, f32)) -> Pos2 {
        let o = self.origin(rect);
        Pos2::new(o.x + p.0 * self.zoom, o.y + p.1 * self.zoom)
    }
    fn s2l(&self, rect: Rect, s: Pos2) -> (f32, f32) {
        let o = self.origin(rect);
        ((s.x - o.x) / self.zoom, (s.y - o.y) / self.zoom)
    }

    fn node_yuksekligi(kimlik: NodeKimlik, graf: &NodeGraf) -> f32 {
        let n = match graf.node(kimlik) {
            Some(n) => n,
            None => return BASLIK_YUK,
        };
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

    fn durum_renk(durum: NodeDurumu, tok: &Tokenlar) -> Color32 {
        match durum {
            NodeDurumu::Bekliyor => tok.renk.metin_soluk,
            NodeDurumu::Calisiyor => tok.renk.bilgi,
            NodeDurumu::Bitti => tok.renk.basari,
            NodeDurumu::Hata => tok.renk.hata,
        }
    }

    /// Tüm node'ları/notları görünür kılacak şekilde pan+zoom ayarlar ("tümünü sığdır").
    pub fn tumunu_sigdir(&mut self, rect: Rect) {
        let Some((min, max)) = self.icerik_siniri() else {
            // Boş grafik → varsayılan görünüm.
            self.zoom = 1.0;
            self.pan = Vec2::new(40.0, 40.0);
            return;
        };
        let bw = (max.0 - min.0).max(1.0);
        let bh = (max.1 - min.1).max(1.0);
        let kenar = 40.0;
        let zx = (rect.width() - 2.0 * kenar) / bw;
        let zy = (rect.height() - 2.0 * kenar) / bh;
        self.zoom = zx.min(zy).clamp(ZOOM_MIN, ZOOM_MAX);
        // İçeriği ekrana ortala.
        let icerik_orta = ((min.0 + max.0) * 0.5, (min.1 + max.1) * 0.5);
        let ekran_orta = rect.center();
        self.pan = Vec2::new(
            (ekran_orta.x - rect.min.x) - icerik_orta.0 * self.zoom,
            (ekran_orta.y - rect.min.y) - icerik_orta.1 * self.zoom,
        );
    }

    /// Tüm node + notların mantıksal sınır kutusu (min, max).  İçerik yoksa `None`.
    fn icerik_siniri(&self) -> Option<((f32, f32), (f32, f32))> {
        let mut var = false;
        let (mut nx, mut ny) = (f32::MAX, f32::MAX);
        let (mut xx, mut xy) = (f32::MIN, f32::MIN);
        for n in self.graf.nodelar() {
            var = true;
            let h = Self::node_yuksekligi(n.kimlik, &self.graf);
            nx = nx.min(n.konum.0);
            ny = ny.min(n.konum.1);
            xx = xx.max(n.konum.0 + NODE_GEN);
            xy = xy.max(n.konum.1 + h);
        }
        for not in self.graf.notlar() {
            var = true;
            nx = nx.min(not.konum.0);
            ny = ny.min(not.konum.1);
            xx = xx.max(not.konum.0 + NOT_GEN);
            xy = xy.max(not.konum.1 + NOT_YUK);
        }
        if var {
            Some(((nx, ny), (xx, xy)))
        } else {
            None
        }
    }
}

// ─── Çizim + etkileşim ──────────────────────────────────────────────────────────

impl NodeTuvali {
    /// Tuvali çizer ve tüm etkileşimi işler (tek karelik immediate-mode).
    pub fn ciz(&mut self, ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) {
        let ctx = ui.ctx().clone();
        // Süren arka plan çalıştırmasını yokla (ilerleme/sonuç) — arayüz donmadan.
        self.calistirmayi_yokla(&ctx);
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = resp.rect;
        painter.rect_filled(rect, egui::Rounding::ZERO, tok.renk.zemin);

        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let birakildi = ctx.input(|i| i.pointer.any_released());
        let mut eylemler: Vec<Eylem> = Vec::new();

        // ── Zoom (imleç sabit kalır) ──────────────────────────────────────
        if resp.hovered() {
            let scroll = ctx.input(|i| i.raw_scroll_delta.y);
            if scroll.abs() > 0.0 {
                if let Some(p) = pointer {
                    let eski_z = self.zoom;
                    let yeni_z = (eski_z * (1.0 + scroll * 0.0015)).clamp(ZOOM_MIN, ZOOM_MAX);
                    if (yeni_z - eski_z).abs() > f32::EPSILON {
                        let o = self.origin(rect);
                        let l = ((p.x - o.x) / eski_z, (p.y - o.y) / eski_z);
                        self.zoom = yeni_z;
                        let yeni_origin = Pos2::new(p.x - l.0 * yeni_z, p.y - l.1 * yeni_z);
                        self.pan = yeni_origin - rect.min;
                    }
                }
            }
        }

        // ── Arka plan ızgarası + mevcut bağlantılar (node'ların ardında) ──
        self.izgara_ciz(&painter, rect, tok);
        self.baglantilari_ciz(&painter, rect, tok);

        // ── Sürüklenen bağlantı önizlemesi ────────────────────────────────
        if let (Some(kaynak), Some(p)) = (self.baglanti_kaynak, pointer) {
            if let Some(n) = self.graf.node(kaynak.node) {
                let kp = Self::port_konum(n.konum, kaynak.yon, kaynak.indeks);
                let renk = self.port_renk(kaynak, tok);
                self.kablo_ciz(&painter, self.l2s(rect, kp), p, renk);
            }
        }

        // ── Node'lar + portlar + etkileşim ────────────────────────────────
        let mut hedef_aday: Option<PortRef> = None;
        let node_kimlikler: Vec<NodeKimlik> =
            self.graf.nodelar().iter().map(|n| n.kimlik).collect();
        for nk in node_kimlikler {
            self.node_ciz(
                ui,
                &painter,
                rect,
                nk,
                dil,
                tok,
                &mut eylemler,
                &mut hedef_aday,
                pointer,
            );
        }

        // ── Bağlantı sürüklemesi bırakıldı: doğrula → bağla / uyar ─────────
        if self.baglanti_kaynak.is_some() && birakildi {
            let kaynak = self.baglanti_kaynak.take().unwrap();
            if let Some(hedef) = hedef_aday {
                match self
                    .graf
                    .baglanti_kontrol(kaynak, hedef, Some(&self.donusturucu))
                {
                    BaglantiKontrol::Uygun => {
                        eylemler.push(Eylem::Baglan { kaynak, hedef });
                    }
                    diger => {
                        let m = baglanti_uyari_metni(&diger, dil);
                        self.uyar(&ctx, m);
                    }
                }
            }
        }

        // ── Yapışkan notlar ───────────────────────────────────────────────
        let not_kimlikler: Vec<NotKimlik> = self.graf.notlar().iter().map(|n| n.kimlik).collect();
        for notk in not_kimlikler {
            self.not_ciz(ui, rect, notk, tok, &mut eylemler);
        }

        // ── Minimap ───────────────────────────────────────────────────────
        let mm_consumed = self.minimap_ciz(ui, &painter, rect, tok);

        // ── Pan (boş alan sürükle / orta tuş) ─────────────────────────────
        let pan_uygun = self.surukle.is_none()
            && self.not_surukle.is_none()
            && self.baglanti_kaynak.is_none()
            && self.palet.is_none()
            && !mm_consumed;
        if resp.dragged_by(egui::PointerButton::Middle) || (pan_uygun && resp.dragged()) {
            self.pan += resp.drag_delta();
        }

        // Boş alana sol tık → kablo varsa önizleme seç, yoksa seçim temizle.  Sağ tık → palet.
        if resp.clicked() {
            if let Some(bk) = self.kablo_vurus(rect, pointer) {
                self.secili_baglanti = Some(bk);
                self.secili_node = None;
            } else {
                self.secili_node = None;
                self.secili_baglanti = None;
            }
        }
        if resp.secondary_clicked() {
            if let Some(p) = pointer {
                self.palet = Some(PaletDurum {
                    mantiksal: self.s2l(rect, p),
                    arama: String::new(),
                });
            }
        }

        // Delete/Backspace → seçili node'u sil.
        if let Some(sec) = self.secili_node {
            let sil = ctx
                .input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace));
            if sil {
                eylemler.push(Eylem::NodeSil(sec));
            }
        }

        // ── Overlay'ler ───────────────────────────────────────────────────
        self.baglanti_onizleme_ciz(&ctx, &painter, rect, dil, tok);
        self.arac_cubugu_ciz(&ctx, rect, dil, tok, &mut eylemler);
        self.palet_ciz(&ctx, rect, dil, tok, &mut eylemler);
        self.uyari_ciz(&ctx, rect, dil, tok);

        // ── Toplanan eylemleri geçmişe yaz ────────────────────────────────
        self.eylemleri_uygula(eylemler);

        // F5 → çalıştır (klavye kısayolu).
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
            self.calistir_baslat();
        }
        // Canlı mod: yapı değiştiyse ve iş sürmüyorsa otomatik yeniden çalıştır.
        if self.canli_mod && self.canli_kirli && self.is.is_none() {
            self.calistir_baslat();
        }
    }

    fn izgara_ciz(&self, painter: &egui::Painter, rect: Rect, tok: &Tokenlar) {
        if self.zoom < LOD_ESIK {
            return; // Çok küçük: ızgara çizilmez (LOD).
        }
        let adim = IZGARA * self.zoom;
        let o = self.origin(rect);
        let stroke = Stroke::new(1.0, tok.renk.kenarlik.gamma_multiply(0.4));
        let mut kx = ((rect.min.x - o.x) / adim).ceil() as i32;
        loop {
            let sx = o.x + kx as f32 * adim;
            if sx > rect.max.x {
                break;
            }
            painter.line_segment(
                [Pos2::new(sx, rect.min.y), Pos2::new(sx, rect.max.y)],
                stroke,
            );
            kx += 1;
        }
        let mut ky = ((rect.min.y - o.y) / adim).ceil() as i32;
        loop {
            let sy = o.y + ky as f32 * adim;
            if sy > rect.max.y {
                break;
            }
            painter.line_segment(
                [Pos2::new(rect.min.x, sy), Pos2::new(rect.max.x, sy)],
                stroke,
            );
            ky += 1;
        }
    }

    fn baglantilari_ciz(&self, painter: &egui::Painter, rect: Rect, tok: &Tokenlar) {
        for b in self.graf.baglantilar() {
            let (Some(ns), Some(nh)) =
                (self.graf.node(b.kaynak.node), self.graf.node(b.hedef.node))
            else {
                continue;
            };
            let a = self.l2s(
                rect,
                Self::port_konum(ns.konum, b.kaynak.yon, b.kaynak.indeks),
            );
            let z = self.l2s(
                rect,
                Self::port_konum(nh.konum, b.hedef.yon, b.hedef.indeks),
            );
            let renk = self.port_renk(b.kaynak, tok);
            self.kablo_ciz(painter, a, z, renk);
        }
    }

    fn kablo_ciz(&self, painter: &egui::Painter, a: Pos2, z: Pos2, renk: Color32) {
        let dx = (z.x - a.x).abs().max(40.0) * 0.5;
        let c1 = Pos2::new(a.x + dx, a.y);
        let c2 = Pos2::new(z.x - dx, z.y);
        let stroke = Stroke::new(2.0, renk);
        let bez = egui::epaint::CubicBezierShape::from_points_stroke(
            [a, c1, c2, z],
            false,
            Color32::TRANSPARENT,
            stroke,
        );
        painter.add(bez);
    }

    fn port_renk(&self, r: PortRef, tok: &Tokenlar) -> Color32 {
        self.graf
            .port_coz(r)
            .map(|p| tok.anahtar_renk(tur_renk_anahtari(&p.veri_turu)))
            .unwrap_or(tok.renk.metin_soluk)
    }

    #[allow(clippy::too_many_arguments)]
    fn node_ciz(
        &mut self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        rect: Rect,
        nk: NodeKimlik,
        dil: Dil,
        tok: &Tokenlar,
        eylemler: &mut Vec<Eylem>,
        hedef_aday: &mut Option<PortRef>,
        pointer: Option<Pos2>,
    ) {
        let tr = matches!(dil, Dil::Tr);
        let konum0 = match self.graf.node(nk) {
            Some(n) => n.konum,
            None => return,
        };
        let h = Self::node_yuksekligi(nk, &self.graf);
        let tl0 = self.l2s(rect, konum0);
        let nrect0 = Rect::from_min_size(tl0, Vec2::new(NODE_GEN * self.zoom, h * self.zoom));
        if !rect.intersects(nrect0) {
            return; // Görünür alan dışında → çizme (culling, MK-04).
        }

        // Gövde etkileşimi (taşıma/seçme/sağ-tık menü).
        let br = ui.interact(
            nrect0,
            egui::Id::new(("node-body", nk.0)),
            Sense::click_and_drag(),
        );
        if br.drag_started() {
            self.surukle = Some((nk, konum0));
            self.secili_node = Some(nk);
        }
        if let Some((sn, _)) = self.surukle {
            if sn == nk && br.dragged() {
                let d = br.drag_delta();
                if d != Vec2::ZERO {
                    if let Some(n) = self.graf.node_mut(nk) {
                        n.konum.0 += d.x / self.zoom;
                        n.konum.1 += d.y / self.zoom;
                    }
                }
            }
        }
        if br.drag_stopped() {
            if let Some((sn, eski)) = self.surukle {
                if sn == nk {
                    self.surukle = None;
                    let yeni = self.graf.node(nk).map(|n| n.konum).unwrap_or(eski);
                    if yeni != eski {
                        eylemler.push(Eylem::NodeTasi {
                            kimlik: nk,
                            eski,
                            yeni,
                        });
                    }
                }
            }
        }
        if br.clicked() {
            self.secili_node = Some(nk);
        }
        br.context_menu(|ui| {
            if ui.button(if tr { "🗑 Sil" } else { "🗑 Delete" }).clicked() {
                eylemler.push(Eylem::NodeSil(nk));
                ui.close_menu();
            }
        });

        // Taşımadan sonra güncel konumla çiz.
        let (konum, durum, baslik, girisler, cikislar) = match self.graf.node(nk) {
            Some(n) => (
                n.konum,
                n.durum,
                n.baslik.clone(),
                n.girisler.clone(),
                n.cikislar.clone(),
            ),
            None => return,
        };
        let tl = self.l2s(rect, konum);
        let nrect = Rect::from_min_size(tl, Vec2::new(NODE_GEN * self.zoom, h * self.zoom));
        let secili = self.secili_node == Some(nk);
        let yuvarlak = egui::Rounding::same(6.0 * self.zoom.clamp(0.4, 1.2));
        let durum_renk = Self::durum_renk(durum, tok);

        painter.rect_filled(nrect, yuvarlak, tok.renk.yuzey);
        let hrect =
            Rect::from_min_size(nrect.min, Vec2::new(nrect.width(), BASLIK_YUK * self.zoom));
        painter.rect_filled(hrect, yuvarlak, tok.renk.yuzey_alt);
        // Durum halkası = node kenarlığı durum renginde.
        painter.rect_stroke(nrect, yuvarlak, Stroke::new(2.0, durum_renk));
        if secili {
            painter.rect_stroke(
                nrect.expand(2.0),
                yuvarlak,
                Stroke::new(2.0, tok.renk.vurgu),
            );
        }
        // Durum noktası.
        let dot = Pos2::new(nrect.min.x + 12.0 * self.zoom, hrect.center().y);
        painter.circle_filled(dot, 4.0 * self.zoom.max(0.5), durum_renk);

        if self.zoom >= LOD_ESIK {
            let bf = (12.0 * self.zoom).clamp(8.0, 20.0);
            painter.text(
                Pos2::new(nrect.min.x + 24.0 * self.zoom, hrect.center().y),
                Align2::LEFT_CENTER,
                &baslik,
                FontId::proportional(bf),
                tok.renk.metin,
            );

            // Çıkış portları.
            for (i, port) in cikislar.iter().enumerate() {
                let sp = self.l2s(rect, Self::port_konum(konum, PortYonu::Cikis, i));
                let renk = tok.anahtar_renk(tur_renk_anahtari(&port.veri_turu));
                painter.circle_filled(sp, PORT_R * self.zoom, renk);
                painter.circle_stroke(sp, PORT_R * self.zoom, Stroke::new(1.0, tok.renk.zemin));
                painter.text(
                    Pos2::new(sp.x - 9.0 * self.zoom, sp.y),
                    Align2::RIGHT_CENTER,
                    &port.ad,
                    FontId::proportional((10.0 * self.zoom).clamp(7.0, 16.0)),
                    tok.renk.metin_soluk,
                );
                let irect = Rect::from_center_size(sp, Vec2::splat(16.0));
                let pr = ui.interact(
                    irect,
                    egui::Id::new(("port-c", nk.0, i)),
                    Sense::click_and_drag(),
                );
                if pr.drag_started() {
                    self.baglanti_kaynak = Some(PortRef::yeni(nk, PortYonu::Cikis, i));
                }
            }

            // Giriş portları.
            for (i, port) in girisler.iter().enumerate() {
                let hedef = PortRef::yeni(nk, PortYonu::Giris, i);
                let sp = self.l2s(rect, Self::port_konum(konum, PortYonu::Giris, i));
                // Sürükleme sırasında uyumlu girişi vurgula.
                if let Some(kaynak) = self.baglanti_kaynak {
                    if self
                        .graf
                        .baglanti_kontrol(kaynak, hedef, Some(&self.donusturucu))
                        .uygun_mu()
                    {
                        painter.circle_stroke(
                            sp,
                            (PORT_R + 3.0) * self.zoom,
                            Stroke::new(2.0, tok.renk.basari),
                        );
                    }
                }
                let renk = tok.anahtar_renk(tur_renk_anahtari(&port.veri_turu));
                painter.circle_filled(sp, PORT_R * self.zoom, renk);
                painter.circle_stroke(sp, PORT_R * self.zoom, Stroke::new(1.0, tok.renk.zemin));
                painter.text(
                    Pos2::new(sp.x + 9.0 * self.zoom, sp.y),
                    Align2::LEFT_CENTER,
                    &port.ad,
                    FontId::proportional((10.0 * self.zoom).clamp(7.0, 16.0)),
                    tok.renk.metin_soluk,
                );
                let irect = Rect::from_center_size(sp, Vec2::splat(16.0));
                let pr = ui.interact(
                    irect,
                    egui::Id::new(("port-g", nk.0, i)),
                    Sense::click_and_drag(),
                );
                // Bırakma hedefi: imleç bu giriş portunun üzerindeyse aday yap.
                if self.baglanti_kaynak.is_some() {
                    if let Some(p) = pointer {
                        if irect.contains(p) {
                            *hedef_aday = Some(hedef);
                        }
                    }
                }
                // Bağlı bir girişe tıklamak bağlantıyı söker.
                if pr.clicked() {
                    if let Some(b) = self.graf.baglantilar().iter().find(|b| b.hedef == hedef) {
                        eylemler.push(Eylem::BaglantiSil(b.kimlik));
                    }
                }
            }
        }
    }

    fn not_ciz(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        notk: NotKimlik,
        tok: &Tokenlar,
        eylemler: &mut Vec<Eylem>,
    ) {
        let konum0 = match self.graf.notlar().iter().find(|n| n.kimlik == notk) {
            Some(n) => n.konum,
            None => return,
        };
        let tl0 = self.l2s(rect, konum0);
        let nrect0 = Rect::from_min_size(tl0, Vec2::new(NOT_GEN * self.zoom, NOT_YUK * self.zoom));
        if !rect.intersects(nrect0) {
            return;
        }
        // Üst şerit = taşıma tutamağı.
        let strip = Rect::from_min_size(nrect0.min, Vec2::new(nrect0.width(), 16.0 * self.zoom));
        let sr = ui.interact(strip, egui::Id::new(("not-strip", notk.0)), Sense::drag());
        if sr.drag_started() {
            self.not_surukle = Some((notk, konum0));
        }
        if let Some((sn, _)) = self.not_surukle {
            if sn == notk && sr.dragged() {
                let d = sr.drag_delta();
                if let Some(n) = self.graf.not_mut(notk) {
                    n.konum.0 += d.x / self.zoom;
                    n.konum.1 += d.y / self.zoom;
                }
            }
        }
        if sr.drag_stopped() {
            self.not_surukle = None;
        }

        // Güncel konumla çiz.
        let konum = self
            .graf
            .notlar()
            .iter()
            .find(|n| n.kimlik == notk)
            .map(|n| n.konum)
            .unwrap_or(konum0);
        let tl = self.l2s(rect, konum);
        let nrect = Rect::from_min_size(tl, Vec2::new(NOT_GEN * self.zoom, NOT_YUK * self.zoom));
        let yuvarlak = egui::Rounding::same(4.0);
        ui.painter()
            .rect_filled(nrect, yuvarlak, tok.renk.uyari_zemin);
        ui.painter()
            .rect_stroke(nrect, yuvarlak, Stroke::new(1.0, tok.renk.uyari));
        let strip = Rect::from_min_size(nrect.min, Vec2::new(nrect.width(), 16.0 * self.zoom));
        ui.painter()
            .rect_filled(strip, yuvarlak, tok.renk.uyari.gamma_multiply(0.35));

        if self.zoom >= LOD_ESIK {
            // Sil butonu (sağ-üst).
            let bsize = 16.0;
            let brect = Rect::from_min_size(
                Pos2::new(nrect.max.x - bsize - 2.0, nrect.min.y + 1.0),
                Vec2::splat(bsize),
            );
            if ui
                .put(brect, egui::Button::new("✕").small().frame(false))
                .clicked()
            {
                eylemler.push(Eylem::NotSil(notk));
            }
            // Düzenlenebilir metin.
            let trect = Rect::from_min_max(
                Pos2::new(nrect.min.x + 4.0, nrect.min.y + 18.0 * self.zoom),
                Pos2::new(nrect.max.x - 4.0, nrect.max.y - 4.0),
            );
            if let Some(n) = self.graf.not_mut(notk) {
                ui.put(
                    trect,
                    egui::TextEdit::multiline(&mut n.metin)
                        .frame(false)
                        .font(FontId::proportional(11.0)),
                );
            }
        } else {
            // Düşük zoom: metni salt-okunur, kısaltarak çiz (LOD).
            let metin = self
                .graf
                .notlar()
                .iter()
                .find(|n| n.kimlik == notk)
                .map(|n| n.metin.clone())
                .unwrap_or_default();
            ui.painter().text(
                nrect.shrink(4.0).min,
                Align2::LEFT_TOP,
                metin.lines().next().unwrap_or(""),
                FontId::proportional(9.0),
                tok.renk.metin,
            );
        }
    }

    /// Minimap çizer; kullanıcı minimap'le etkileşince (tıkla/sürükle ortala) `true` döner.
    fn minimap_ciz(
        &mut self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        rect: Rect,
        tok: &Tokenlar,
    ) -> bool {
        if rect.width() < 280.0 || rect.height() < 200.0 {
            return false;
        }
        let mm = Rect::from_min_size(
            Pos2::new(rect.max.x - 180.0, rect.min.y + 12.0),
            Vec2::new(168.0, 112.0),
        );
        painter.rect_filled(
            mm,
            egui::Rounding::same(4.0),
            tok.renk.yuzey.gamma_multiply(0.92),
        );
        painter.rect_stroke(
            mm,
            egui::Rounding::same(4.0),
            Stroke::new(1.0, tok.renk.kenarlik),
        );
        let Some((min, max)) = self.icerik_siniri() else {
            return false;
        };
        let bw = (max.0 - min.0).max(1.0);
        let bh = (max.1 - min.1).max(1.0);
        let inner = mm.shrink(6.0);
        let scale = (inner.width() / bw).min(inner.height() / bh);
        let off = Vec2::new(
            (inner.width() - bw * scale) * 0.5,
            (inner.height() - bh * scale) * 0.5,
        );
        let map = |lp: (f32, f32)| {
            Pos2::new(
                inner.min.x + off.x + (lp.0 - min.0) * scale,
                inner.min.y + off.y + (lp.1 - min.1) * scale,
            )
        };
        for n in self.graf.nodelar() {
            let hh = Self::node_yuksekligi(n.kimlik, &self.graf);
            let a = map(n.konum);
            let b = map((n.konum.0 + NODE_GEN, n.konum.1 + hh));
            painter.rect_filled(
                Rect::from_two_pos(a, b),
                egui::Rounding::same(1.0),
                Self::durum_renk(n.durum, tok),
            );
        }
        // Görünür alanı (viewport) çiz.
        let v0 = self.s2l(rect, rect.min);
        let v1 = self.s2l(rect, rect.max);
        painter.rect_stroke(
            Rect::from_two_pos(map(v0), map(v1)),
            egui::Rounding::ZERO,
            Stroke::new(1.0, tok.renk.vurgu),
        );
        // Etkileşim: minimap'e tıkla/sürükle → o noktayı ortala.
        let mr = ui.interact(mm, egui::Id::new("node-minimap"), Sense::click_and_drag());
        if mr.clicked() || mr.dragged() {
            if let Some(p) = ui.input(|i| i.pointer.interact_pos()) {
                if mm.contains(p) {
                    let lx = min.0 + (p.x - inner.min.x - off.x) / scale;
                    let ly = min.1 + (p.y - inner.min.y - off.y) / scale;
                    self.pan = Vec2::new(
                        (rect.center().x - rect.min.x) - lx * self.zoom,
                        (rect.center().y - rect.min.y) - ly * self.zoom,
                    );
                }
            }
            return true;
        }
        false
    }

    fn arac_cubugu_ciz(
        &mut self,
        ctx: &egui::Context,
        rect: Rect,
        dil: Dil,
        tok: &Tokenlar,
        eylemler: &mut Vec<Eylem>,
    ) {
        let tr = matches!(dil, Dil::Tr);
        egui::Area::new(egui::Id::new("node-arac"))
            .fixed_pos(rect.min + Vec2::splat(8.0))
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(tok.renk.yuzey)
                    .stroke(Stroke::new(1.0, tok.renk.kenarlik))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::same(6.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .button(if tr {
                                    "⤢ Tümünü Sığdır"
                                } else {
                                    "⤢ Fit All"
                                })
                                .clicked()
                            {
                                self.tumunu_sigdir(rect);
                            }
                            if ui.button("＋ Node").clicked() {
                                self.palet = Some(PaletDurum {
                                    mantiksal: self.s2l(rect, rect.center()),
                                    arama: String::new(),
                                });
                            }
                            if ui.button(if tr { "＋ Not" } else { "＋ Note" }).clicked() {
                                eylemler.push(Eylem::NotEkle {
                                    konum: self.s2l(rect, rect.center()),
                                    metin: if tr {
                                        "Yeni not".into()
                                    } else {
                                        "New note".into()
                                    },
                                });
                            }
                            ui.separator();
                            if ui
                                .add_enabled(self.geri_alinabilir_mi(), egui::Button::new("↶"))
                                .on_hover_text(if tr { "Geri al" } else { "Undo" })
                                .clicked()
                            {
                                self.geri_al();
                            }
                            if ui
                                .add_enabled(self.yinelenebilir_mi(), egui::Button::new("↷"))
                                .on_hover_text(if tr { "Yinele" } else { "Redo" })
                                .clicked()
                            {
                                self.yinele();
                            }
                            ui.separator();
                            ui.label(
                                egui::RichText::new(format!("{:.0}%", self.zoom * 100.0))
                                    .color(tok.renk.metin_soluk),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "· {} node · {} {}",
                                    self.graf.nodelar().len(),
                                    self.graf.baglantilar().len(),
                                    if tr { "bağlantı" } else { "links" }
                                ))
                                .small()
                                .color(tok.renk.metin_soluk),
                            );
                        });
                        ui.separator();
                        // ── Çalıştırma satırı (Çalıştır / İptal / ilerleme / canlı / dışa aktar) ──
                        ui.horizontal(|ui| {
                            self.calistirma_kontrolleri(ui, ctx, dil, tok);
                        });
                    });
            });
    }

    /// Araç çubuğunun çalıştırma kontrolleri (Çalıştır/İptal/ilerleme/canlı mod/dışa aktarma).
    fn calistirma_kontrolleri(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        dil: Dil,
        tok: &Tokenlar,
    ) {
        let tr = matches!(dil, Dil::Tr);
        // Çalıştır / İptal.
        if self.is.is_some() {
            let (tamam, toplam) = self
                .is
                .as_ref()
                .map(|i| (i.tamamlanan, i.toplam))
                .unwrap_or((0, 0));
            let oran = if toplam > 0 {
                tamam as f32 / toplam as f32
            } else {
                0.0
            };
            ui.add(
                egui::ProgressBar::new(oran)
                    .desired_width(120.0)
                    .text(format!("{tamam}/{toplam}"))
                    .fill(tok.renk.vurgu),
            );
            if ui.button(if tr { "⏹ İptal" } else { "⏹ Cancel" }).clicked() {
                self.calistirmayi_iptal();
            }
        } else if ui
            .button(
                egui::RichText::new(if tr { "▶ Çalıştır" } else { "▶ Run" }).color(tok.renk.basari),
            )
            .on_hover_text(if tr {
                "Akışı çalıştır (bağımsız dallar paralel, sonuç önbelleğinden)"
            } else {
                "Run the flow (independent branches in parallel, results cached)"
            })
            .clicked()
        {
            self.calistir_baslat();
        }

        // Canlı mod.
        let onceki_canli = self.canli_mod;
        ui.checkbox(&mut self.canli_mod, if tr { "Canlı" } else { "Live" })
            .on_hover_text(if tr {
                "Değişiklikte otomatik çalıştır"
            } else {
                "Auto-run on change"
            });
        if self.canli_mod && !onceki_canli && self.agir_node_var() {
            self.uyar(
                ctx,
                if tr {
                    "Canlı mod açık: akışta ağır node var; her değişiklikte yeniden hesaplanır."
                } else {
                    "Live mode on: flow has heavy nodes; recomputed on every change."
                },
            );
        }

        // Dışa aktarma menüsü (rfd dosya seçici ertelendi → panoya kopyala).
        ui.menu_button(if tr { "⬇ Dışa Aktar" } else { "⬇ Export" }, |ui| {
            if ui
                .button(if tr {
                    "Akış (.bcflow) → pano"
                } else {
                    "Flow (.bcflow) → clipboard"
                })
                .clicked()
            {
                let s = self.bcflow_metni();
                ui.output_mut(|o| o.copied_text = s);
                self.uyar(
                    ctx,
                    if tr {
                        "Akış panoya kopyalandı (.bcflow)."
                    } else {
                        "Flow copied to clipboard (.bcflow)."
                    },
                );
                ui.close_menu();
            }
            if ui
                .button(if tr {
                    "SVG → pano"
                } else {
                    "SVG → clipboard"
                })
                .clicked()
            {
                let s = self.svg_metni(tok, dil);
                ui.output_mut(|o| o.copied_text = s);
                self.uyar(
                    ctx,
                    if tr {
                        "SVG panoya kopyalandı."
                    } else {
                        "SVG copied to clipboard."
                    },
                );
                ui.close_menu();
            }
            if ui
                .button(if tr {
                    "Python betiği → pano"
                } else {
                    "Python script → clipboard"
                })
                .clicked()
            {
                let s = self.python_metni();
                ui.output_mut(|o| o.copied_text = s);
                self.uyar(
                    ctx,
                    if tr {
                        "Python betiği panoya kopyalandı."
                    } else {
                        "Python script copied to clipboard."
                    },
                );
                ui.close_menu();
            }
        });

        // Son çalıştırma özeti.
        if let Some(s) = &self.son_sonuc {
            let metin = if tr {
                format!(
                    "✓ {} hesaplandı · {} önbellek · {} hata",
                    s.hesaplanan, s.onbellekten_atlanan, s.hata_sayisi
                )
            } else {
                format!(
                    "✓ {} computed · {} cached · {} errors",
                    s.hesaplanan, s.onbellekten_atlanan, s.hata_sayisi
                )
            };
            let renk = if s.hata_sayisi > 0 {
                tok.renk.uyari
            } else {
                tok.renk.metin_soluk
            };
            ui.label(egui::RichText::new(metin).small().color(renk));
        }
    }

    /// Seçili kabloyu vurgular + üstünde ara veri önizlemesi gösterir (kabloya tıkla → önizleme).
    fn baglanti_onizleme_ciz(
        &self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        rect: Rect,
        dil: Dil,
        tok: &Tokenlar,
    ) {
        let tr = matches!(dil, Dil::Tr);
        let Some(bk) = self.secili_baglanti else {
            return;
        };
        let Some(b) = self.graf.baglantilar().iter().find(|x| x.kimlik == bk) else {
            return;
        };
        let (Some(ns), Some(nh)) = (self.graf.node(b.kaynak.node), self.graf.node(b.hedef.node))
        else {
            return;
        };
        let a = self.l2s(
            rect,
            Self::port_konum(ns.konum, b.kaynak.yon, b.kaynak.indeks),
        );
        let z = self.l2s(
            rect,
            Self::port_konum(nh.konum, b.hedef.yon, b.hedef.indeks),
        );
        // Vurgulu (kalın) kablo.
        let dx = (z.x - a.x).abs().max(40.0) * 0.5;
        let bez = egui::epaint::CubicBezierShape::from_points_stroke(
            [a, Pos2::new(a.x + dx, a.y), Pos2::new(z.x - dx, z.y), z],
            false,
            Color32::TRANSPARENT,
            Stroke::new(4.0, tok.renk.vurgu),
        );
        painter.add(bez);

        // Önizleme metni (çalıştırma sonucundan).
        let metin = match self
            .son_sonuc
            .as_ref()
            .and_then(|s| s.baglanti_onizleme.get(&bk))
        {
            Some(d) => {
                if tr {
                    format!("◆ {} · {} öğe · {}", d.veri_turu, d.eleman, d.ozet)
                } else {
                    format!("◆ {} · {} items · {}", d.veri_turu, d.eleman, d.ozet)
                }
            }
            None => {
                if tr {
                    "◆ Ara veri yok — önce ▶ Çalıştır'a basın.".to_string()
                } else {
                    "◆ No data yet — press ▶ Run first.".to_string()
                }
            }
        };
        let mid = Pos2::new((a.x + z.x) * 0.5, (a.y + z.y) * 0.5);
        let anchor = Pos2::new(
            mid.x.clamp(rect.min.x, rect.max.x - 240.0),
            mid.y.clamp(rect.min.y + 8.0, rect.max.y - 40.0),
        );
        egui::Area::new(egui::Id::new("node-kablo-onizleme"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(tok.renk.yuzey)
                    .stroke(Stroke::new(1.0, tok.renk.vurgu))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                    .show(ui, |ui| {
                        ui.set_max_width(240.0);
                        ui.label(egui::RichText::new(metin).color(tok.renk.metin).small());
                    });
            });
    }

    fn palet_ciz(
        &mut self,
        ctx: &egui::Context,
        rect: Rect,
        dil: Dil,
        tok: &Tokenlar,
        eylemler: &mut Vec<Eylem>,
    ) {
        let Some(mut p) = self.palet.take() else {
            return;
        };
        let tr = matches!(dil, Dil::Tr);
        let mut kapat = false;
        let mut secilen: Option<String> = None;
        // Çapayı ekran içinde tut.
        let ham = self.l2s(rect, p.mantiksal);
        let anchor = Pos2::new(
            ham.x.clamp(rect.min.x, rect.max.x - 250.0),
            ham.y.clamp(rect.min.y, rect.max.y - 280.0),
        );
        let katalog = &self.katalog;
        egui::Area::new(egui::Id::new("node-palet"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(tok.renk.yuzey)
                    .stroke(Stroke::new(1.0, tok.renk.vurgu))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.set_max_width(240.0);
                        ui.label(
                            egui::RichText::new(if tr { "Node Ekle" } else { "Add Node" })
                                .strong()
                                .color(tok.renk.metin),
                        );
                        let te = ui.add(
                            egui::TextEdit::singleline(&mut p.arama)
                                .hint_text(if tr { "Ara…" } else { "Search…" })
                                .desired_width(224.0),
                        );
                        te.request_focus();
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical()
                            .max_height(240.0)
                            .show(ui, |ui| {
                                for g in katalog.ara(&p.arama) {
                                    let etiket = g.baslik.clone();
                                    if ui
                                        .add(
                                            egui::Button::new(etiket)
                                                .min_size(Vec2::new(224.0, 0.0)),
                                        )
                                        .on_hover_text(format!("{} · {}", g.kategori, g.aciklama))
                                        .clicked()
                                    {
                                        secilen = Some(g.tur_kimligi.clone());
                                    }
                                }
                            });
                        ui.add_space(4.0);
                        if ui.button(if tr { "Kapat" } else { "Close" }).clicked() {
                            kapat = true;
                        }
                    });
            });

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            kapat = true;
        }
        if let Some(t) = secilen {
            eylemler.push(Eylem::NodeEkle {
                tur_kimligi: t,
                konum: p.mantiksal,
            });
            kapat = true;
        }
        if !kapat {
            self.palet = Some(p);
        }
    }

    fn uyari_ciz(&mut self, ctx: &egui::Context, rect: Rect, dil: Dil, tok: &Tokenlar) {
        let _ = dil;
        let Some((metin, t0)) = self.uyari.clone() else {
            return;
        };
        let now = ctx.input(|i| i.time);
        if now - t0 > UYARI_SURE {
            self.uyari = None;
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(120));
        let pos = Pos2::new(rect.center().x - 200.0, rect.max.y - 52.0);
        egui::Area::new(egui::Id::new("node-uyari"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(tok.renk.uyari_zemin)
                    .stroke(Stroke::new(1.0, tok.renk.uyari))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.set_max_width(400.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new("⚠").color(tok.renk.uyari));
                            ui.label(egui::RichText::new(metin).color(tok.renk.metin));
                        });
                    });
            });
    }

    fn eylemleri_uygula(&mut self, eylemler: Vec<Eylem>) {
        for e in eylemler {
            match e {
                Eylem::NodeEkle { tur_kimligi, konum } => {
                    if let Some(g) = self.katalog.bul(&tur_kimligi).cloned() {
                        let kimlik = self.graf.yeni_node_kimlik();
                        let node = g.ornekle(kimlik, konum);
                        let gk = self.graf.kimlik.clone();
                        self.komut(Box::new(NodeEkleKomut::yeni(&gk, node)));
                        self.secili_node = Some(kimlik);
                        self.canli_kirli = true;
                    }
                }
                Eylem::NodeSil(k) => {
                    if let Some(c) = NodeSilKomut::yeni(&self.graf, k) {
                        self.komut(Box::new(c));
                        if self.secili_node == Some(k) {
                            self.secili_node = None;
                        }
                        self.parametreler.remove(&k);
                        self.onbellek.gecersiz_kil(k);
                        self.canli_kirli = true;
                    }
                }
                Eylem::NodeTasi { kimlik, eski, yeni } => {
                    let gk = self.graf.kimlik.clone();
                    self.komut(Box::new(NodeTasiKomut::yeni(&gk, kimlik, eski, yeni)));
                    // Taşıma sonucu değiştirmez → önbellek/canlı tetik gerekmez.
                }
                Eylem::Baglan { kaynak, hedef } => {
                    if self
                        .graf
                        .baglanti_kontrol(kaynak, hedef, Some(&self.donusturucu))
                        .uygun_mu()
                    {
                        let bk = self.graf.yeni_baglanti_kimlik();
                        let b = Baglanti {
                            kimlik: bk,
                            kaynak,
                            hedef,
                        };
                        let gk = self.graf.kimlik.clone();
                        self.komut(Box::new(BaglantiEkleKomut::yeni(&gk, b)));
                        self.canli_kirli = true;
                    }
                }
                Eylem::BaglantiSil(k) => {
                    if let Some(c) = BaglantiSilKomut::yeni(&self.graf, k) {
                        self.komut(Box::new(c));
                        if self.secili_baglanti == Some(k) {
                            self.secili_baglanti = None;
                        }
                        self.canli_kirli = true;
                    }
                }
                Eylem::NotEkle { konum, metin } => {
                    let nk = self.graf.yeni_not_kimlik();
                    let not = YapiskanNot {
                        kimlik: nk,
                        metin,
                        konum,
                    };
                    let gk = self.graf.kimlik.clone();
                    self.komut(Box::new(NotEkleKomut::yeni(&gk, not)));
                }
                Eylem::NotSil(k) => {
                    if let Some(c) = NotSilKomut::yeni(&self.graf, k) {
                        self.komut(Box::new(c));
                    }
                }
            }
        }
    }
}

/// Bir noktanın bir doğru parçasına (a→b) en kısa uzaklığı (kablo vuruş testi için).
fn nokta_segment_mesafe(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let uzunluk2 = ab.x * ab.x + ab.y * ab.y;
    if uzunluk2 <= f32::EPSILON {
        return (p - a).length();
    }
    let t = (((p - a).x * ab.x + (p - a).y * ab.y) / uzunluk2).clamp(0.0, 1.0);
    let proj = Pos2::new(a.x + ab.x * t, a.y + ab.y * t);
    (p - proj).length()
}

/// Reddedilen bir bağlantı için kullanıcıya gösterilecek anlık uyarı metni (yerelleştirilmiş).
fn baglanti_uyari_metni(k: &BaglantiKontrol, dil: Dil) -> String {
    let tr = matches!(dil, Dil::Tr);
    match k {
        BaglantiKontrol::TipUyumsuz {
            kaynak,
            hedef,
            donusturucu,
        } => {
            let temel = if tr {
                format!(
                    "Uyumsuz tür: {} → {} bağlanamaz.",
                    kaynak.ad(dil),
                    hedef.ad(dil)
                )
            } else {
                format!(
                    "Incompatible types: {} → {} can't connect.",
                    kaynak.ad(dil),
                    hedef.ad(dil)
                )
            };
            match donusturucu {
                Some(d) if tr => format!("{temel} Dönüştürücü öneriliyor: {d}"),
                Some(d) => format!("{temel} Suggested converter: {d}"),
                None => temel,
            }
        }
        BaglantiKontrol::DonguOlur => {
            if tr {
                "Döngü oluşur: bağlantı reddedildi (akış döngüsüz olmalı).".into()
            } else {
                "Would create a cycle: connection rejected (flow must be acyclic).".into()
            }
        }
        BaglantiKontrol::GirisDolu => {
            if tr {
                "Bu giriş zaten bağlı (giriş tek bağlantı alır).".into()
            } else {
                "This input is already connected (single connection).".into()
            }
        }
        BaglantiKontrol::AyniNode => {
            if tr {
                "Bir node kendine bağlanamaz.".into()
            } else {
                "A node can't connect to itself.".into()
            }
        }
        BaglantiKontrol::ZatenVar => {
            if tr {
                "Bu bağlantı zaten var.".into()
            } else {
                "This connection already exists.".into()
            }
        }
        _ => {
            if tr {
                "Bağlantı kurulamadı.".into()
            } else {
                "Could not connect.".into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ornek_tuval_dolu_ve_dag() {
        let t = NodeTuvali::ornek();
        assert!(t.graf.nodelar().len() >= 3);
        assert!(t.graf.baglantilar().len() >= 2);
        assert!(!t.graf.notlar().is_empty());
        // Demo akış döngüsüz olmalı.
        assert!(super::super::dag::topolojik_sira(&t.graf).is_some());
    }

    #[test]
    fn fit_bos_grafikte_varsayilana_doner() {
        let mut t = NodeTuvali::yeni("ana");
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        t.tumunu_sigdir(rect);
        assert_eq!(t.zoom, 1.0);
    }

    #[test]
    fn fit_dolu_grafikte_icerigi_kapsar() {
        let mut t = NodeTuvali::ornek();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        t.tumunu_sigdir(rect);
        assert!(t.zoom >= ZOOM_MIN && t.zoom <= ZOOM_MAX);
        // İçerik sınırı hesaplanabilmeli.
        assert!(t.icerik_siniri().is_some());
    }

    #[test]
    fn uyari_metni_dongu_ve_tip_yerellesir() {
        let m = baglanti_uyari_metni(&BaglantiKontrol::DonguOlur, Dil::Tr);
        assert!(m.contains("Döngü"));
        let tip = BaglantiKontrol::TipUyumsuz {
            kaynak: super::super::port::VeriTuru::yeni("dizi"),
            hedef: super::super::port::VeriTuru::yeni("tablo"),
            donusturucu: Some("donustur.x".into()),
        };
        let mt = baglanti_uyari_metni(&tip, Dil::Tr);
        assert!(mt.contains("Uyumsuz tür") && mt.contains("Dönüştürücü"));
    }

    // ── Headless egui dumanı (çizim + etkileşim panik yapmamalı) ──────────────

    /// Tuvali headless egui'de bir kare çizer (panik = test başarısız).
    fn kare_ciz(t: &mut NodeTuvali, dil: Dil, tok: &Tokenlar, input: egui::RawInput) {
        let ctx = egui::Context::default();
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                t.ciz(ui, dil, tok);
            });
        });
    }

    #[test]
    fn cizim_iki_dil_iki_temada_panik_yapmaz() {
        let mut t = NodeTuvali::ornek();
        kare_ciz(
            &mut t,
            Dil::Tr,
            &Tokenlar::koyu(),
            egui::RawInput::default(),
        );
        kare_ciz(
            &mut t,
            Dil::En,
            &Tokenlar::acik(),
            egui::RawInput::default(),
        );
        kare_ciz(
            &mut t,
            Dil::Tr,
            &Tokenlar::yuksek_kontrast(),
            egui::RawInput::default(),
        );
    }

    #[test]
    fn bos_tuval_ve_dusuk_zoom_lod_cizilir() {
        // LOD yolu (port/metin atlanır) ve boş grafik panik yapmamalı.
        let mut t = NodeTuvali::yeni("ana");
        t.zoom = 0.3; // LOD eşiğinin altı
        kare_ciz(
            &mut t,
            Dil::Tr,
            &Tokenlar::koyu(),
            egui::RawInput::default(),
        );
        // Yüksek zoomda dolu tuval.
        let mut d = NodeTuvali::ornek();
        d.zoom = 2.0;
        kare_ciz(
            &mut d,
            Dil::En,
            &Tokenlar::acik(),
            egui::RawInput::default(),
        );
    }

    #[test]
    fn palet_acikken_cizilir() {
        // Sağ-tık paleti açık durumda da kare panik yapmadan çizilmeli.
        let mut t = NodeTuvali::ornek();
        t.palet = Some(PaletDurum {
            mantiksal: (50.0, 50.0),
            arama: "hizala".into(),
        });
        kare_ciz(
            &mut t,
            Dil::Tr,
            &Tokenlar::koyu(),
            egui::RawInput::default(),
        );
    }

    #[test]
    fn arac_undo_redo_calisir() {
        // Programatik olarak bir node ekle (eylem yolu) → geçmiş dolar; geri al/yinele çalışır.
        let mut t = NodeTuvali::yeni("ana");
        t.eylemleri_uygula(vec![Eylem::NodeEkle {
            tur_kimligi: "isle.hizala".into(),
            konum: (10.0, 10.0),
        }]);
        assert_eq!(t.graf.nodelar().len(), 1);
        assert!(t.geri_alinabilir_mi());
        t.geri_al();
        assert_eq!(t.graf.nodelar().len(), 0);
        assert!(t.yinelenebilir_mi());
        t.yinele();
        assert_eq!(t.graf.nodelar().len(), 1);
    }

    // ── Çalıştırma (Gün 21) ───────────────────────────────────────────────

    #[test]
    fn senkron_calistirma_durumu_uygular() {
        let mut t = NodeTuvali::ornek(); // dizi_oku → hizala → varyant
        t.calistir_simdi();
        {
            let sonuc = t.son_sonuc.as_ref().unwrap();
            assert_eq!(sonuc.hesaplanan, 3);
            assert_eq!(sonuc.hata_sayisi, 0);
            // Kablo önizlemesi dolu.
            assert_eq!(sonuc.baglanti_onizleme.len(), 2);
        }
        // Grafiğe uygulandı → hepsi Bitti.
        assert!(t
            .graf
            .nodelar()
            .iter()
            .all(|n| n.durum == NodeDurumu::Bitti));
    }

    #[test]
    fn ikinci_calistirma_onbellekten() {
        let mut t = NodeTuvali::ornek();
        t.calistir_simdi();
        t.calistir_simdi();
        let s2 = t.son_sonuc.as_ref().unwrap();
        assert_eq!(
            s2.onbellekten_atlanan, 3,
            "değişmeyen akış önbellekten gelir"
        );
        assert_eq!(s2.hesaplanan, 0);
    }

    #[test]
    fn arka_plan_calistirma_tamamlanir() {
        // Arka plan thread + kare-başı yoklama → iş donmadan tamamlanmalı.
        let mut t = NodeTuvali::ornek();
        t.calistir_baslat();
        assert!(t.calisiyor_mu());
        let ctx = egui::Context::default();
        let baslangic = std::time::Instant::now();
        while t.calisiyor_mu() && baslangic.elapsed().as_secs() < 5 {
            t.calistirmayi_yokla(&ctx);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(!t.calisiyor_mu(), "arka plan işi tamamlanmalı");
        let sonuc = t.son_sonuc.as_ref().unwrap();
        assert_eq!(sonuc.hesaplanan, 3);
    }

    #[test]
    fn eklenti_sdk_node_kaydi_calisir() {
        use biocraft_sdk::node::{AkisDeger, NodeCalistirici, NodeKaydi, NodeTanimi, Parametreler};
        // Bir "eklenti" node'u SDK üzerinden kaydeder.
        struct EklentiNode;
        impl NodeCalistirici for EklentiNode {
            fn calistir(
                &self,
                _g: &[AkisDeger],
                _p: &Parametreler,
            ) -> Result<Vec<AkisDeger>, String> {
                Ok(vec![AkisDeger::yeni("ozel", "eklenti çıktısı", 42, 100)])
            }
        }
        let mut t = NodeTuvali::yeni("ana");
        t.node_kaydet(NodeKaydi::yeni(
            NodeTanimi {
                kimlik: "eklenti.ozel".into(),
                baslik: "Özel Eklenti".into(),
                kategori: "Eklenti".into(),
                aciklama: String::new(),
                portlar: vec![],
                parametreler: vec![],
            },
            std::sync::Arc::new(EklentiNode),
        ));
        // Grafa o türden bir node ekle (katalogda olmasa da çalıştırıcı kayıtlı).
        let k = t.graf.yeni_node_kimlik();
        t.graf.node_ekle_ham(super::super::graph::Node {
            kimlik: k,
            tur_kimligi: "eklenti.ozel".into(),
            baslik: "Özel".into(),
            konum: (0.0, 0.0),
            girisler: vec![],
            cikislar: vec![crate::node::port::Port::yeni("c", "ozel")],
            durum: NodeDurumu::Bekliyor,
        });
        t.calistir_simdi();
        let sonuc = t.son_sonuc.as_ref().unwrap();
        assert_eq!(sonuc.hesaplanan, 1);
        assert_eq!(sonuc.node_sonuclari[&k].ciktilar[0].eleman, 42);
    }

    #[test]
    fn bcflow_kaydet_yukle_tuval_uzerinden() {
        let t = NodeTuvali::ornek();
        let metin = t.bcflow_metni();
        let mut t2 = NodeTuvali::yeni("baska");
        t2.bcflow_yukle(&metin).unwrap();
        assert_eq!(t2.graf.nodelar().len(), t.graf.nodelar().len());
        assert_eq!(t2.graf.baglantilar().len(), t.graf.baglantilar().len());
    }

    #[test]
    fn disa_aktarmalar_uretilir() {
        let t = NodeTuvali::ornek();
        let svg = t.svg_metni(&Tokenlar::koyu(), Dil::Tr);
        assert!(svg.contains("</svg>"));
        let png = t.png_baytlari(&Tokenlar::koyu(), 1.0);
        assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        let py = t.python_metni();
        assert!(py.contains("def calistir():"));
    }

    #[test]
    fn calistirma_kontrollu_kare_panik_yapmaz() {
        // Araç çubuğu çalıştırma satırı + sonuç özeti çizilirken panik olmamalı.
        let mut t = NodeTuvali::ornek();
        let _ = t.calistir_simdi();
        t.secili_baglanti = t.graf.baglantilar().first().map(|b| b.kimlik);
        kare_ciz(
            &mut t,
            Dil::Tr,
            &Tokenlar::koyu(),
            egui::RawInput::default(),
        );
        kare_ciz(
            &mut t,
            Dil::En,
            &Tokenlar::acik(),
            egui::RawInput::default(),
        );
    }
}
