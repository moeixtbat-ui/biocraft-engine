//! ÇE-07 — **Render-bağımsız 3B sahne** + **yörünge kamera/projeksiyon** + **2B dışa aktarma**.
//!
//! Eklenti motorun GPU/render katmanına (wgpu) doğrudan bağlanamaz (MK-17).  Bu yüzden
//! görüntüleyici, çizilecek 3B geometriyi **anlamsal** ilkellere ([`Kure`]/[`Silindir`]/[`Serit`])
//! derler ([`Sahne3B`]); motorun render katmanı (biocraft-render, wgpu, Gün 6) bu sahneyi + kamera
//! parametrelerini alıp **instanced** küre/silindir shader'larıyla GPU'da çizer.
//!
//! Aynı sahne, **GPU yoksa CPU yedeği** (Gün 5 TDR kurtarmasının üstünde) ve **PNG/SVG dışa
//! aktarma** (Gün 42) için burada **saf-Rust** projeksiyonla 2B'ye düşürülür ([`Ekran2B`]):
//! perspektif kamera dünya noktalarını ekran pikseline eşler, ressam algoritmasıyla (uzak→yakın)
//! sıralanır.  Böylece tüm koordinat/kamera/projeksiyon mantığı GPU'dan bağımsız **birim-testlenir**.
//!
//! Renkler **anlamsal** anahtarlarla ([`Renk3B`]) verilir → motor tasarım jetonuna (Gün 31.2a) eşler;
//! dışa aktarma için [`Palet3B`] somut CPK/zincir/B-faktör RGB üretir (sabit RGB sahne kodunda yok).

use std::collections::HashMap;

// ─── Vektör cebiri (minimal; yeni dış bağımlılık yok) ─────────────────────────────

/// 3B vektör/nokta (Ångström).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    /// Yeni vektör.
    pub const fn yeni(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Sıfır vektör.
    pub const SIFIR: Self = Self::yeni(0.0, 0.0, 0.0);

    /// Toplama.
    pub fn topla(self, b: Vec3) -> Vec3 {
        Vec3::yeni(self.x + b.x, self.y + b.y, self.z + b.z)
    }

    /// Çıkarma (`self - b`).
    pub fn cikar(self, b: Vec3) -> Vec3 {
        Vec3::yeni(self.x - b.x, self.y - b.y, self.z - b.z)
    }

    /// Skalerle ölçekleme.
    pub fn olcekle(self, s: f32) -> Vec3 {
        Vec3::yeni(self.x * s, self.y * s, self.z * s)
    }

    /// Nokta (iç) çarpım.
    pub fn nokta(self, b: Vec3) -> f32 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }

    /// Çapraz (vektörel) çarpım.
    pub fn capraz(self, b: Vec3) -> Vec3 {
        Vec3::yeni(
            self.y * b.z - self.z * b.y,
            self.z * b.x - self.x * b.z,
            self.x * b.y - self.y * b.x,
        )
    }

    /// Uzunluk (norm).
    pub fn uzunluk(self) -> f32 {
        self.nokta(self).sqrt()
    }

    /// İki nokta arası Öklid uzaklığı (Ångström).
    pub fn uzaklik(self, b: Vec3) -> f32 {
        self.cikar(b).uzunluk()
    }

    /// Birim vektör (sıfır vektörde `(0,0,1)` döner — güvenli).
    pub fn normalize(self) -> Vec3 {
        let u = self.uzunluk();
        if u < 1e-6 {
            Vec3::yeni(0.0, 0.0, 1.0)
        } else {
            self.olcekle(1.0 / u)
        }
    }
}

// ─── Anlamsal renk + palet ────────────────────────────────────────────────────────

/// Anlamsal 3B renk anahtarı — motor tasarım jetonuna eşler (sahne kodunda sabit RGB yok).
///
/// `Zincir(i)` ve `BFaktor(q)` sıralı/sürekli boyutları küçük tamsayı yük (`u8`) taşıyarak
/// `Hash`/`Eq`/`Copy` kalır → palet `HashMap` anahtarı olabilir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Renk3B {
    /// Arka plan (3B sahne zemini).
    Zemin,
    /// Element: karbon.
    ElemC,
    /// Element: azot.
    ElemN,
    /// Element: oksijen.
    ElemO,
    /// Element: kükürt.
    ElemS,
    /// Element: fosfor.
    ElemP,
    /// Element: hidrojen.
    ElemH,
    /// Element: halojen (F/Cl/Br/I).
    ElemHalojen,
    /// Element: metal/iyon (Fe/Mg/Zn/Ca/Na/K…).
    ElemMetal,
    /// Element: diğer/bilinmeyen.
    ElemDiger,
    /// Zincire göre renk (kimlik sırasına göre döngüsel palet indeksi).
    Zincir(u8),
    /// İkincil yapı: α-heliks.
    Heliks,
    /// İkincil yapı: β-yaprak (strand).
    Yaprak,
    /// İkincil yapı: ilmek/sarmal (coil).
    Sarmal,
    /// B-faktör (sıcaklık) rampası — `0`=düşük (mavi) … `255`=yüksek (kırmızı).
    BFaktor(u8),
    /// Seçili atom/kalıntı vurgusu.
    Secim,
    /// Ölçüm (mesafe/açı) çizgisi/etiketi.
    Olcum,
}

/// Anlamsal renkten somut RGB'ye eşleme (CPK + zincir paleti + ikincil yapı + B-faktör rampası).
/// Varsayılan **yayın** paleti gelir; motor `ayarla` ile kendi tasarım jetonlarını basabilir.
#[derive(Debug, Clone, Default)]
pub struct Palet3B {
    ozel: HashMap<Renk3B, [u8; 3]>,
    /// Arka plan rengi (None → koyu mavi-gri varsayılan).
    pub zemin: Option<[u8; 3]>,
}

/// Zincire göre renklendirmede döngüsel palet (görsel olarak ayrışan tonlar).
const ZINCIR_PALETI: [[u8; 3]; 8] = [
    [76, 175, 132],  // teal-yeşil
    [232, 168, 73],  // amber
    [120, 144, 230], // indigo
    [214, 109, 162], // magenta
    [124, 196, 110], // yeşil
    [220, 110, 90],  // mercan
    [150, 130, 220], // mor
    [110, 180, 200], // camgöbeği
];

impl Palet3B {
    /// Yayın (publication) varsayılan paleti.
    pub fn yayin() -> Self {
        Self {
            ozel: HashMap::new(),
            zemin: Some([18, 22, 30]),
        }
    }

    /// Bir anlamsal rengi geçersiz kılar (motor tasarım jetonu basabilir).
    pub fn ayarla(&mut self, renk: Renk3B, rgb: [u8; 3]) -> &mut Self {
        self.ozel.insert(renk, rgb);
        self
    }

    /// Arka plan rengini ayarlar.
    pub fn zemin_ayarla(&mut self, rgb: [u8; 3]) -> &mut Self {
        self.zemin = Some(rgb);
        self
    }

    /// Bir anlamsal rengin RGB karşılığı (özel ayar > varsayılan).
    pub fn rgb(&self, renk: Renk3B) -> [u8; 3] {
        self.ozel
            .get(&renk)
            .copied()
            .unwrap_or_else(|| varsayilan_rgb(renk))
    }

    /// Arka plan RGB'si.
    pub fn zemin_rgb(&self) -> [u8; 3] {
        self.zemin.unwrap_or([18, 22, 30])
    }
}

/// Yayın paletinin varsayılan değerleri (anlamsal renk → RGB).
fn varsayilan_rgb(renk: Renk3B) -> [u8; 3] {
    match renk {
        Renk3B::Zemin => [18, 22, 30],
        // CPK standardı.
        Renk3B::ElemC => [200, 200, 205],
        Renk3B::ElemN => [48, 80, 248],
        Renk3B::ElemO => [240, 60, 50],
        Renk3B::ElemS => [230, 200, 60],
        Renk3B::ElemP => [240, 150, 40],
        Renk3B::ElemH => [235, 235, 235],
        Renk3B::ElemHalojen => [60, 200, 120],
        Renk3B::ElemMetal => [225, 150, 160],
        Renk3B::ElemDiger => [170, 130, 200],
        Renk3B::Zincir(i) => ZINCIR_PALETI[(i as usize) % ZINCIR_PALETI.len()],
        // İkincil yapı (PyMOL/IGV benzeri): heliks kırmızı-magenta, yaprak sarı, ilmek camgöbeği.
        Renk3B::Heliks => [230, 70, 110],
        Renk3B::Yaprak => [230, 200, 60],
        Renk3B::Sarmal => [120, 200, 210],
        Renk3B::BFaktor(q) => bfaktor_rampa(q),
        Renk3B::Secim => [255, 145, 0],
        Renk3B::Olcum => [255, 90, 90],
    }
}

/// B-faktör rampası: mavi (düşük/rijit) → beyaz → kırmızı (yüksek/esnek).
fn bfaktor_rampa(q: u8) -> [u8; 3] {
    let t = q as f32 / 255.0;
    if t < 0.5 {
        let k = t / 0.5; // mavi→beyaz
        let v = (255.0 * k) as u8;
        [v, v, 255]
    } else {
        let k = (t - 0.5) / 0.5; // beyaz→kırmızı
        let v = (255.0 * (1.0 - k)) as u8;
        [255, v, v]
    }
}

// ─── 3B sahne ilkelleri ─────────────────────────────────────────────────────────

/// Küre (atom).
#[derive(Debug, Clone, PartialEq)]
pub struct Kure {
    pub merkez: Vec3,
    pub yaricap: f32,
    pub renk: Renk3B,
    /// Hangi atomu temsil eder (model içi indeks) — seçim/picking için.
    pub atom_indeksi: Option<usize>,
}

/// Silindir (kovalent bağ / kartonet omurga segmenti).
#[derive(Debug, Clone, PartialEq)]
pub struct Silindir {
    pub bas: Vec3,
    pub son: Vec3,
    pub yaricap: f32,
    pub renk: Renk3B,
}

/// Şerit/omurga izi (kartonet) — ardışık noktalar + nokta başına renk.
#[derive(Debug, Clone, PartialEq)]
pub struct Serit {
    pub noktalar: Vec<Vec3>,
    pub renkler: Vec<Renk3B>,
    pub yaricap: f32,
}

/// Render-bağımsız 3B sahne — motorun GPU katmanına teslim edilen geometri.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sahne3B {
    pub kureler: Vec<Kure>,
    pub silindirler: Vec<Silindir>,
    pub seritler: Vec<Serit>,
}

impl Sahne3B {
    /// Boş sahne.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Toplam ilkel (küre + silindir + şerit segmenti) sayısı — bütçe/LOD teşhisi.
    pub fn ilkel_sayisi(&self) -> usize {
        self.kureler.len()
            + self.silindirler.len()
            + self
                .seritler
                .iter()
                .map(|s| s.noktalar.len().saturating_sub(1))
                .sum::<usize>()
    }

    /// Tüm geometriyi kapsayan eksen-hizalı sınır kutusu (`min`, `max`).  Boşsa `None`.
    pub fn sinir_kutusu(&self) -> Option<(Vec3, Vec3)> {
        let mut noktalar = self.kureler.iter().map(|k| k.merkez);
        let ilk = noktalar.next()?;
        let mut min = ilk;
        let mut max = ilk;
        let hesaba_kat = |min: &mut Vec3, max: &mut Vec3, p: Vec3| {
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            min.z = min.z.min(p.z);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
            max.z = max.z.max(p.z);
        };
        for p in noktalar {
            hesaba_kat(&mut min, &mut max, p);
        }
        for s in &self.silindirler {
            hesaba_kat(&mut min, &mut max, s.bas);
            hesaba_kat(&mut min, &mut max, s.son);
        }
        for s in &self.seritler {
            for &p in &s.noktalar {
                hesaba_kat(&mut min, &mut max, p);
            }
        }
        Some((min, max))
    }
}

// ─── Kamera + perspektif projeksiyon ──────────────────────────────────────────────

/// Perspektif kamera (yörünge kamerası [`super::interact::YorungeKamera`] tarafından üretilir).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Kamera {
    /// Göz (kamera) konumu.
    pub goz: Vec3,
    /// Bakılan hedef nokta.
    pub hedef: Vec3,
    /// Yukarı yön (genelde `(0,1,0)`).
    pub yukari: Vec3,
    /// Görüş açısı (dikey FOV, derece).
    pub fov_derece: f32,
    /// Yakın düzlem (bunun önündeki noktalar kırpılır).
    pub yakin: f32,
}

impl Kamera {
    /// Bir görüş kutusu (genişlik/yükseklik piksel) için projeksiyon hazırlığı — kamera tabanını
    /// (sağ/yukarı/ileri ortonormal) ve odak/ölçeği bir kez hesaplar.
    pub fn hazirla(&self, gen: f32, yuk: f32) -> KameraGoruntu {
        let ileri = self.hedef.cikar(self.goz).normalize();
        let sag = ileri.capraz(self.yukari).normalize();
        let yukari = sag.capraz(ileri); // ortonormal düzeltme
        let fov = self.fov_derece.to_radians().clamp(0.05, 3.0);
        let odak = 1.0 / (fov * 0.5).tan();
        KameraGoruntu {
            goz: self.goz,
            sag,
            yukari,
            ileri,
            odak,
            yari: 0.5 * gen.max(1.0).min(yuk.max(1.0)),
            merkez_x: gen * 0.5,
            merkez_y: yuk * 0.5,
            yakin: self.yakin,
        }
    }
}

/// Bir görüş kutusu için ön-hesaplanmış kamera projeksiyonu.
#[derive(Debug, Clone, Copy)]
pub struct KameraGoruntu {
    goz: Vec3,
    sag: Vec3,
    yukari: Vec3,
    ileri: Vec3,
    odak: f32,
    yari: f32,
    merkez_x: f32,
    merkez_y: f32,
    yakin: f32,
}

/// Ekrana izdüşmüş bir nokta (piksel) + kamera derinliği (ressam sıralaması için).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nokta2B {
    pub x: f32,
    pub y: f32,
    /// Kamera ileri yönündeki uzaklık (Ångström); büyük = uzak.
    pub derinlik: f32,
}

impl KameraGoruntu {
    /// Bir dünya noktasını ekran pikseline projekte eder.  Kameranın arkasında/çok yakınsa `None`.
    pub fn projekte(&self, p: Vec3) -> Option<Nokta2B> {
        let pe = p.cikar(self.goz);
        let cz = pe.nokta(self.ileri);
        if cz <= self.yakin {
            return None;
        }
        let cx = pe.nokta(self.sag);
        let cy = pe.nokta(self.yukari);
        let oran = self.odak * self.yari / cz;
        Some(Nokta2B {
            x: self.merkez_x + cx * oran,
            // Ekran y aşağı doğru artar → işaret ters.
            y: self.merkez_y - cy * oran,
            derinlik: cz,
        })
    }

    /// Belirli derinlikte bir Ångström yarıçapın ekran piksel yarıçapı.
    pub fn yaricap_px(&self, yaricap: f32, derinlik: f32) -> f32 {
        if derinlik <= self.yakin {
            0.0
        } else {
            (yaricap * self.odak * self.yari / derinlik).max(0.4)
        }
    }
}

// ─── 2B ekran listesi (CPU yedeği + dışa aktarma) ─────────────────────────────────

/// İzdüşürülmüş 2B çizim parçası (derinlik sıralı; ressam algoritması).
#[derive(Debug, Clone, PartialEq)]
pub enum Parca2B {
    /// Dolu daire (izdüşmüş küre/atom).
    Daire {
        x: f32,
        y: f32,
        r: f32,
        renk: Renk3B,
        derinlik: f32,
    },
    /// Çizgi (izdüşmüş bağ/şerit segmenti).
    Cizgi {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        renk: Renk3B,
        kalinlik: f32,
        derinlik: f32,
    },
}

impl Parca2B {
    fn derinlik(&self) -> f32 {
        match self {
            Parca2B::Daire { derinlik, .. } | Parca2B::Cizgi { derinlik, .. } => *derinlik,
        }
    }
}

/// Bir metin etiketi (ölçüm/seçim) — yalnızca SVG'de çizilir (PNG yazı-tipi gerektirir).
#[derive(Debug, Clone, PartialEq)]
pub struct Etiket2B {
    pub x: f32,
    pub y: f32,
    pub icerik: String,
}

/// CPU yedeği/dışa aktarma için derinlik-sıralı 2B sahne.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ekran2B {
    pub parcalar: Vec<Parca2B>,
    pub etiketler: Vec<Etiket2B>,
}

/// Bir 3B sahneyi bir kamerayla 2B ekran listesine projekte eder (uzak→yakın sıralı).
pub fn projeksiyon(sahne: &Sahne3B, kamera: &Kamera, gen: f32, yuk: f32) -> Ekran2B {
    let g = kamera.hazirla(gen, yuk);
    let mut parcalar: Vec<Parca2B> = Vec::new();

    for k in &sahne.kureler {
        if let Some(n) = g.projekte(k.merkez) {
            parcalar.push(Parca2B::Daire {
                x: n.x,
                y: n.y,
                r: g.yaricap_px(k.yaricap, n.derinlik),
                renk: k.renk,
                derinlik: n.derinlik,
            });
        }
    }
    for s in &sahne.silindirler {
        cizgi_ekle(&mut parcalar, &g, s.bas, s.son, s.renk, s.yaricap);
    }
    for s in &sahne.seritler {
        for i in 0..s.noktalar.len().saturating_sub(1) {
            let renk = s.renkler.get(i).copied().unwrap_or(Renk3B::Sarmal);
            cizgi_ekle(
                &mut parcalar,
                &g,
                s.noktalar[i],
                s.noktalar[i + 1],
                renk,
                s.yaricap,
            );
        }
    }

    // Ressam algoritması: uzak (büyük derinlik) önce çizilir.
    parcalar.sort_by(|a, b| b.derinlik().total_cmp(&a.derinlik()));
    Ekran2B {
        parcalar,
        etiketler: Vec::new(),
    }
}

fn cizgi_ekle(
    parcalar: &mut Vec<Parca2B>,
    g: &KameraGoruntu,
    bas: Vec3,
    son: Vec3,
    renk: Renk3B,
    yaricap: f32,
) {
    if let (Some(a), Some(b)) = (g.projekte(bas), g.projekte(son)) {
        let orta = (a.derinlik + b.derinlik) * 0.5;
        parcalar.push(Parca2B::Cizgi {
            x1: a.x,
            y1: a.y,
            x2: b.x,
            y2: b.y,
            renk,
            kalinlik: g.yaricap_px(yaricap, orta).max(1.0),
            derinlik: orta,
        });
    }
}

// ─── SVG dışa aktarma ─────────────────────────────────────────────────────────────

/// 2B ekran listesini bir **SVG** belgesine (daire + çizgi + etiket) çevirir (yayın kalitesi).
pub fn svg_olustur(ekran: &Ekran2B, gen: f32, yuk: f32, palet: &Palet3B) -> String {
    let w = gen.max(1.0);
    let h = yuk.max(1.0);
    let mut s = String::with_capacity(1024 + ekran.parcalar.len() * 80);
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.0}\" height=\"{h:.0}\" \
viewBox=\"0 0 {w:.0} {h:.0}\" font-family=\"sans-serif\">\n"
    ));
    s.push_str(&format!(
        "  <rect x=\"0\" y=\"0\" width=\"{w:.0}\" height=\"{h:.0}\" fill=\"{}\"/>\n",
        hex(palet.zemin_rgb())
    ));
    for p in &ekran.parcalar {
        match p {
            Parca2B::Daire { x, y, r, renk, .. } => {
                s.push_str(&format!(
                    "  <circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"{:.2}\" fill=\"{}\"/>\n",
                    r.max(0.1),
                    hex(palet.rgb(*renk))
                ));
            }
            Parca2B::Cizgi {
                x1,
                y1,
                x2,
                y2,
                renk,
                kalinlik,
                ..
            } => {
                s.push_str(&format!(
                    "  <line x1=\"{x1:.2}\" y1=\"{y1:.2}\" x2=\"{x2:.2}\" y2=\"{y2:.2}\" \
stroke=\"{}\" stroke-width=\"{kalinlik:.2}\" stroke-linecap=\"round\"/>\n",
                    hex(palet.rgb(*renk))
                ));
            }
        }
    }
    for e in &ekran.etiketler {
        s.push_str(&format!(
            "  <text x=\"{:.2}\" y=\"{:.2}\" fill=\"{}\" font-size=\"12\">{}</text>\n",
            e.x,
            e.y,
            hex(palet.rgb(Renk3B::Olcum)),
            xml_kacis(&e.icerik)
        ));
    }
    s.push_str("</svg>\n");
    s
}

/// RGB → `#rrggbb`.
fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

/// XML metin kaçışı.
fn xml_kacis(s: &str) -> String {
    let mut c = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => c.push_str("&amp;"),
            '<' => c.push_str("&lt;"),
            '>' => c.push_str("&gt;"),
            '"' => c.push_str("&quot;"),
            _ => c.push(ch),
        }
    }
    c
}

// ─── PNG dışa aktarma (saf-Rust raster; dolu daire + çizgi) ───────────────────────

/// 2B ekran listesinin **geometrisini** (dolu daire + çizgi) rasterleştirip **PNG** baytı üretir.
/// Metin etiketleri (yazı-tipi gerektirir) PNG'de çizilmez — yayın için [`svg_olustur`] kullanın.
pub fn png_olustur(ekran: &Ekran2B, gen: u32, yuk: u32, palet: &Palet3B) -> Vec<u8> {
    let w = gen.max(1) as usize;
    let h = yuk.max(1) as usize;
    let mut t = RasterTuval::yeni(w, h, palet.zemin_rgb());
    for p in &ekran.parcalar {
        match p {
            Parca2B::Daire { x, y, r, renk, .. } => t.daire(*x, *y, *r, palet.rgb(*renk)),
            Parca2B::Cizgi {
                x1,
                y1,
                x2,
                y2,
                renk,
                kalinlik,
                ..
            } => t.cizgi(*x1, *y1, *x2, *y2, *kalinlik, palet.rgb(*renk)),
        }
    }
    t.png()
}

struct RasterTuval {
    gen: usize,
    yuk: usize,
    piksel: Vec<u8>,
}

impl RasterTuval {
    fn yeni(gen: usize, yuk: usize, zemin: [u8; 3]) -> Self {
        let mut piksel = Vec::with_capacity(gen * yuk * 3);
        for _ in 0..gen * yuk {
            piksel.extend_from_slice(&zemin);
        }
        Self { gen, yuk, piksel }
    }

    fn nokta(&mut self, x: i64, y: i64, renk: [u8; 3]) {
        if x < 0 || y < 0 || x as usize >= self.gen || y as usize >= self.yuk {
            return;
        }
        let i = (y as usize * self.gen + x as usize) * 3;
        self.piksel[i] = renk[0];
        self.piksel[i + 1] = renk[1];
        self.piksel[i + 2] = renk[2];
    }

    fn daire(&mut self, cx: f32, cy: f32, r: f32, renk: [u8; 3]) {
        let r = r.max(0.5);
        let r2 = r * r;
        let x0 = (cx - r).floor() as i64;
        let x1 = (cx + r).ceil() as i64;
        let y0 = (cy - r).floor() as i64;
        let y1 = (cy + r).ceil() as i64;
        for yy in y0..=y1 {
            for xx in x0..=x1 {
                let dx = xx as f32 + 0.5 - cx;
                let dy = yy as f32 + 0.5 - cy;
                if dx * dx + dy * dy <= r2 {
                    self.nokta(xx, yy, renk);
                }
            }
        }
    }

    /// Kalın çizgi (yuvarlak uçlu): her adımda küçük daire bas (Bresenham yörüngesinde).
    fn cizgi(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, kalinlik: f32, renk: [u8; 3]) {
        let yari = (kalinlik * 0.5).max(0.5);
        let uzun = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt().max(1.0);
        let adim = (uzun.ceil() as i64).max(1);
        for i in 0..=adim {
            let t = i as f32 / adim as f32;
            let x = x1 + (x2 - x1) * t;
            let y = y1 + (y2 - y1) * t;
            self.daire(x, y, yari, renk);
        }
    }

    /// RGB tuvali RGBA PNG baytlarına kodlar (saf-Rust; zlib *stored*).
    fn png(&self) -> Vec<u8> {
        let mut ham = Vec::with_capacity(self.yuk * (1 + self.gen * 4));
        for y in 0..self.yuk {
            ham.push(0u8); // filtre: None
            for x in 0..self.gen {
                let i = (y * self.gen + x) * 3;
                ham.push(self.piksel[i]);
                ham.push(self.piksel[i + 1]);
                ham.push(self.piksel[i + 2]);
                ham.push(255);
            }
        }
        let zlib = zlib_stored(&ham);

        let mut png = Vec::new();
        png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
        let mut ihdr = Vec::with_capacity(13);
        ihdr.extend_from_slice(&(self.gen as u32).to_be_bytes());
        ihdr.extend_from_slice(&(self.yuk as u32).to_be_bytes());
        ihdr.push(8); // bit derinliği
        ihdr.push(6); // RGBA
        ihdr.push(0);
        ihdr.push(0);
        ihdr.push(0);
        chunk_yaz(&mut png, b"IHDR", &ihdr);
        chunk_yaz(&mut png, b"IDAT", &zlib);
        chunk_yaz(&mut png, b"IEND", &[]);
        png
    }
}

fn chunk_yaz(cikti: &mut Vec<u8>, tur: &[u8; 4], veri: &[u8]) {
    cikti.extend_from_slice(&(veri.len() as u32).to_be_bytes());
    cikti.extend_from_slice(tur);
    cikti.extend_from_slice(veri);
    let mut crc_girdi = Vec::with_capacity(4 + veri.len());
    crc_girdi.extend_from_slice(tur);
    crc_girdi.extend_from_slice(veri);
    cikti.extend_from_slice(&crc32(&crc_girdi).to_be_bytes());
}

fn zlib_stored(veri: &[u8]) -> Vec<u8> {
    let mut z = Vec::with_capacity(veri.len() + veri.len() / 65535 * 5 + 6);
    z.push(0x78);
    z.push(0x01);
    let mut kalan = veri;
    loop {
        let n = kalan.len().min(65535);
        let son = n == kalan.len();
        z.push(if son { 1 } else { 0 });
        z.extend_from_slice(&(n as u16).to_le_bytes());
        z.extend_from_slice(&(!(n as u16)).to_le_bytes());
        z.extend_from_slice(&kalan[..n]);
        kalan = &kalan[n..];
        if son {
            break;
        }
    }
    z.extend_from_slice(&adler32(veri).to_be_bytes());
    z
}

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

fn crc32(veri: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &x in veri {
        crc ^= x as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec3_temel_islemler() {
        let a = Vec3::yeni(1.0, 0.0, 0.0);
        let b = Vec3::yeni(0.0, 1.0, 0.0);
        assert_eq!(a.capraz(b), Vec3::yeni(0.0, 0.0, 1.0));
        assert!((a.nokta(b)).abs() < 1e-6);
        assert!((a.uzaklik(b) - 2.0_f32.sqrt()).abs() < 1e-5);
        assert!((Vec3::yeni(3.0, 4.0, 0.0).uzunluk() - 5.0).abs() < 1e-5);
    }

    #[test]
    fn projeksiyon_merkez_ve_derinlik() {
        // Hedefte (orijin) duran nokta tam ekran merkezine düşmeli.
        let kam = Kamera {
            goz: Vec3::yeni(0.0, 0.0, 10.0),
            hedef: Vec3::SIFIR,
            yukari: Vec3::yeni(0.0, 1.0, 0.0),
            fov_derece: 45.0,
            yakin: 0.1,
        };
        let g = kam.hazirla(200.0, 100.0);
        let n = g.projekte(Vec3::SIFIR).unwrap();
        assert!((n.x - 100.0).abs() < 1e-3);
        assert!((n.y - 50.0).abs() < 1e-3);
        assert!((n.derinlik - 10.0).abs() < 1e-3);
        // Kameranın arkası → None.
        assert!(g.projekte(Vec3::yeni(0.0, 0.0, 20.0)).is_none());
    }

    #[test]
    fn projeksiyon_yakin_uzak_derinlik_sirasi() {
        let kam = Kamera {
            goz: Vec3::yeni(0.0, 0.0, 10.0),
            hedef: Vec3::SIFIR,
            yukari: Vec3::yeni(0.0, 1.0, 0.0),
            fov_derece: 45.0,
            yakin: 0.1,
        };
        let mut sahne = Sahne3B::yeni();
        // Uzak küre (z=-5 → derinlik 15), yakın küre (z=5 → derinlik 5).
        sahne.kureler.push(Kure {
            merkez: Vec3::yeni(0.0, 0.0, -5.0),
            yaricap: 1.0,
            renk: Renk3B::ElemO,
            atom_indeksi: Some(0),
        });
        sahne.kureler.push(Kure {
            merkez: Vec3::yeni(0.0, 0.0, 5.0),
            yaricap: 1.0,
            renk: Renk3B::ElemN,
            atom_indeksi: Some(1),
        });
        let ekran = projeksiyon(&sahne, &kam, 200.0, 200.0);
        // Ressam: ilk çizilen uzak olmalı (büyük derinlik), yakın küre daha büyük yarıçaplı.
        if let (
            Parca2B::Daire {
                derinlik: d0,
                r: r0,
                ..
            },
            Parca2B::Daire {
                derinlik: d1,
                r: r1,
                ..
            },
        ) = (&ekran.parcalar[0], &ekran.parcalar[1])
        {
            assert!(d0 > d1, "uzak parça önce gelmeli");
            assert!(r1 > r0, "yakın küre daha büyük izdüşmeli");
        } else {
            panic!("iki daire bekleniyordu");
        }
    }

    #[test]
    fn sinir_kutusu_kureleri_kapsar() {
        let mut sahne = Sahne3B::yeni();
        for (i, p) in [Vec3::yeni(-1.0, -2.0, -3.0), Vec3::yeni(4.0, 5.0, 6.0)]
            .into_iter()
            .enumerate()
        {
            sahne.kureler.push(Kure {
                merkez: p,
                yaricap: 1.0,
                renk: Renk3B::ElemC,
                atom_indeksi: Some(i),
            });
        }
        let (min, max) = sahne.sinir_kutusu().unwrap();
        assert_eq!(min, Vec3::yeni(-1.0, -2.0, -3.0));
        assert_eq!(max, Vec3::yeni(4.0, 5.0, 6.0));
    }

    #[test]
    fn png_gecerli_imza_ve_crc() {
        let mut ekran = Ekran2B::default();
        ekran.parcalar.push(Parca2B::Daire {
            x: 25.0,
            y: 25.0,
            r: 10.0,
            renk: Renk3B::ElemO,
            derinlik: 5.0,
        });
        let png = png_olustur(&ekran, 50, 50, &Palet3B::yayin());
        assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        // Chunk'ları yürü + CRC doğrula.
        let mut i = 8;
        let mut turler = Vec::new();
        while i + 8 <= png.len() {
            let uzun = u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]) as usize;
            let tur = &png[i + 4..i + 8];
            let veri = &png[i + 8..i + 8 + uzun];
            let crc = u32::from_be_bytes(png[i + 8 + uzun..i + 12 + uzun].try_into().unwrap());
            let mut g = tur.to_vec();
            g.extend_from_slice(veri);
            assert_eq!(crc, crc32(&g));
            turler.push(String::from_utf8_lossy(tur).into_owned());
            i += 12 + uzun;
        }
        assert_eq!(turler.first().unwrap(), "IHDR");
        assert_eq!(turler.last().unwrap(), "IEND");
        assert_eq!(i, png.len());
    }

    #[test]
    fn crc_adler_test_vektorleri() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn svg_daire_ve_cizgi_icerir() {
        let mut ekran = Ekran2B::default();
        ekran.parcalar.push(Parca2B::Daire {
            x: 5.0,
            y: 5.0,
            r: 2.0,
            renk: Renk3B::ElemC,
            derinlik: 3.0,
        });
        ekran.parcalar.push(Parca2B::Cizgi {
            x1: 0.0,
            y1: 0.0,
            x2: 10.0,
            y2: 10.0,
            renk: Renk3B::Heliks,
            kalinlik: 2.0,
            derinlik: 3.0,
        });
        let svg = svg_olustur(&ekran, 20.0, 20.0, &Palet3B::yayin());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("<line"));
        assert!(svg.trim_end().ends_with("</svg>"));
    }

    #[test]
    fn bfaktor_rampasi_uclar() {
        assert_eq!(bfaktor_rampa(0), [0, 0, 255]); // mavi
        assert_eq!(bfaktor_rampa(255), [255, 0, 0]); // kırmızı
    }
}
