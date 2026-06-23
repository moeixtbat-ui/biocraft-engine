//! ÇE-02 — **Anlık görüntü dışa aktarma** (Gün 37): görünümün **SVG** (yayın kalitesi vektör) ve
//! **PNG** (raster) olarak dışa aktarımı.
//!
//! Tarayıcı render-bağımsız bir [`CizimListesi`] derler (MK-17); bu modül o listeyi **eklenti
//! içinde** (motor/GPU olmadan, yeni dış bağımlılık olmadan) dosyaya çevirir:
//! * **SVG:** tam vektör — dikdörtgen/çizgi/**metin** (cetvel etiketleri, baz/aminoasit harfleri,
//!   gen adları) dâhil → yayın kalitesi, ölçeklenebilir, tam etiketli.
//! * **PNG:** saf-Rust kodlayıcı (zlib *stored* blok + CRC32 + Adler32) ile **geometriyi**
//!   (renkli baz hücreleri, read/kapsama/varyant) rasterleştirir.  Metin etiketleri yazı-tipi
//!   gerektirdiğinden raster PNG'de çizilmez; **yayın için tam etiketli SVG** önerilir (PNG hızlı
//!   önizleme).  Tam-yazıtipli yüksek-çözünürlük PNG ayrıca motorun GPU anlık görüntüsüyle alınır.
//!
//! Renkler bir [`Palet`] ile verilir: motor tasarım jetonlarından (Gün 31.2a) doldurabilir;
//! varsayılan **yayın** paleti (açık zemin, IGV-benzeri baz renkleri) sağlanır.

use std::collections::HashMap;

use super::cizim::{CizimListesi, CizimRengi, MetinHiza, Primitif};

/// Anlamsal renkten somut RGB'ye eşleme.  Varsayılan **yayın** paleti gelir; motor `ayarla` ile
/// kendi tasarım jetonlarını basabilir (dışa aktarılan dosya kurum temasına uyar).
#[derive(Debug, Clone, Default)]
pub struct Palet {
    ozel: HashMap<CizimRengi, [u8; 3]>,
    /// Tuval zemini (arka plan) rengi.
    pub zemin: Option<[u8; 3]>,
}

impl Palet {
    /// Yayın (publication) varsayılan paleti — açık zemin, okunaklı, IGV-benzeri baz renkleri.
    pub fn yayin() -> Self {
        Self {
            ozel: HashMap::new(),
            zemin: Some([255, 255, 255]),
        }
    }

    /// Bir anlamsal rengi geçersiz kılar (motor tasarım jetonu basabilir).
    pub fn ayarla(&mut self, renk: CizimRengi, rgb: [u8; 3]) -> &mut Self {
        self.ozel.insert(renk, rgb);
        self
    }

    /// Zemin rengini ayarlar.
    pub fn zemin_ayarla(&mut self, rgb: [u8; 3]) -> &mut Self {
        self.zemin = Some(rgb);
        self
    }

    /// Bir anlamsal rengin RGB karşılığı (özel ayar > yayın varsayılanı).
    pub fn rgb(&self, renk: CizimRengi) -> [u8; 3] {
        self.ozel
            .get(&renk)
            .copied()
            .unwrap_or_else(|| varsayilan_rgb(renk))
    }

    /// Zemin RGB'si (ayarsızsa beyaz).
    pub fn zemin_rgb(&self) -> [u8; 3] {
        self.zemin.unwrap_or([255, 255, 255])
    }
}

/// Yayın paletinin varsayılan değerleri (anlamsal renk → RGB).
fn varsayilan_rgb(renk: CizimRengi) -> [u8; 3] {
    match renk {
        CizimRengi::CetvelZemin => [238, 240, 242],
        CizimRengi::CetvelCizgi => [85, 85, 85],
        CizimRengi::CetvelMetin => [40, 40, 40],
        CizimRengi::IzZemin => [250, 250, 250],
        CizimRengi::IzEtiket => [40, 40, 40],
        CizimRengi::ReadIleri => [217, 123, 108],
        CizimRengi::ReadGeri => [108, 155, 217],
        CizimRengi::ReadDusuk => [200, 200, 200],
        CizimRengi::KapsamaCubuk => [136, 160, 184],
        CizimRengi::Gen => [68, 68, 68],
        CizimRengi::Ekson => [59, 110, 165],
        CizimRengi::AnotasyonMetin => [34, 34, 34],
        CizimRengi::OzetYogunluk => [154, 167, 181],
        CizimRengi::Secim => [255, 140, 0],
        // Bazlar (IGV standardı): A yeşil, C mavi, G turuncu/altın, T kırmızı.
        CizimRengi::BazA => [60, 179, 113],
        CizimRengi::BazC => [65, 105, 225],
        CizimRengi::BazG => [218, 165, 32],
        CizimRengi::BazT => [220, 20, 60],
        CizimRengi::BazN => [153, 153, 153],
        CizimRengi::BazMetin => [255, 255, 255],
        CizimRengi::AminoAsit => [180, 160, 200],
        CizimRengi::AminoAsitDur => [224, 92, 92],
        CizimRengi::VaryantSnv => [228, 87, 46],
        CizimRengi::VaryantIns => [46, 134, 171],
        CizimRengi::VaryantDel => [139, 58, 58],
        CizimRengi::OlcumCizgi => [214, 39, 40],
        CizimRengi::OlcumMetin => [214, 39, 40],
        CizimRengi::IsaretliBolge => [255, 243, 160],
        CizimRengi::OrnekA => [31, 119, 180],
        CizimRengi::OrnekB => [255, 127, 14],
        CizimRengi::OrnekC => [44, 160, 44],
        CizimRengi::OrnekD => [214, 39, 40],
    }
}

// ─── SVG (yayın kalitesi vektör) ─────────────────────────────────────────────────

/// Çizim listesini bir **SVG** belgesine (tam vektör, metin dâhil) çevirir.
pub fn svg_olustur(liste: &CizimListesi, genislik: f32, yukseklik: f32, palet: &Palet) -> String {
    let w = genislik.max(1.0);
    let h = yukseklik.max(1.0);
    let mut s = String::with_capacity(1024 + liste.primitifler.len() * 80);
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.0}\" height=\"{h:.0}\" \
viewBox=\"0 0 {w:.0} {h:.0}\" font-family=\"sans-serif\">\n"
    ));
    // Zemin.
    s.push_str(&format!(
        "  <rect x=\"0\" y=\"0\" width=\"{w:.0}\" height=\"{h:.0}\" fill=\"{}\"/>\n",
        hex(palet.zemin_rgb())
    ));

    for p in &liste.primitifler {
        match p {
            Primitif::Dikdortgen {
                x,
                y,
                gen,
                yuk,
                renk,
            } => {
                let opaklik = if *renk == CizimRengi::IsaretliBolge {
                    " fill-opacity=\"0.3\""
                } else {
                    ""
                };
                s.push_str(&format!(
                    "  <rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{:.2}\" height=\"{:.2}\" \
fill=\"{}\"{opaklik}/>\n",
                    gen.max(0.0),
                    yuk.max(0.0),
                    hex(palet.rgb(*renk))
                ));
            }
            Primitif::Cizgi {
                x1,
                y1,
                x2,
                y2,
                renk,
                kalinlik,
            } => {
                s.push_str(&format!(
                    "  <line x1=\"{x1:.2}\" y1=\"{y1:.2}\" x2=\"{x2:.2}\" y2=\"{y2:.2}\" \
stroke=\"{}\" stroke-width=\"{kalinlik:.2}\"/>\n",
                    hex(palet.rgb(*renk))
                ));
            }
            Primitif::Metin {
                x,
                y,
                icerik,
                renk,
                boyut,
                hiza,
            } => {
                let anchor = match hiza {
                    MetinHiza::Sol => "start",
                    MetinHiza::Orta => "middle",
                };
                s.push_str(&format!(
                    "  <text x=\"{x:.2}\" y=\"{y:.2}\" fill=\"{}\" font-size=\"{boyut:.1}\" \
text-anchor=\"{anchor}\" dominant-baseline=\"hanging\">{}</text>\n",
                    hex(palet.rgb(*renk)),
                    xml_kacis(icerik)
                ));
            }
        }
    }
    s.push_str("</svg>\n");
    s
}

/// RGB → `#rrggbb`.
fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

/// XML metin kaçışı (`&`, `<`, `>`, `"`).
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

// ─── PNG (saf-Rust raster) ───────────────────────────────────────────────────────

/// Çizim listesinin **geometrisini** (dikdörtgen + çizgi) rasterleştirip **PNG** baytları üretir.
/// Metin (etiket) raster PNG'de çizilmez — yayın için [`svg_olustur`] kullanın.
pub fn png_olustur(liste: &CizimListesi, genislik: u32, yukseklik: u32, palet: &Palet) -> Vec<u8> {
    let w = genislik.max(1) as usize;
    let h = yukseklik.max(1) as usize;
    let mut tuval = RasterTuval::yeni(w, h, palet.zemin_rgb());

    for p in &liste.primitifler {
        match p {
            Primitif::Dikdortgen {
                x,
                y,
                gen,
                yuk,
                renk,
            } => {
                tuval.dikdortgen(*x, *y, *gen, *yuk, palet.rgb(*renk));
            }
            Primitif::Cizgi {
                x1,
                y1,
                x2,
                y2,
                renk,
                kalinlik,
            } => {
                tuval.cizgi(*x1, *y1, *x2, *y2, *kalinlik, palet.rgb(*renk));
            }
            // Metin raster PNG'de atlanır (yazı-tipi gerektirir; SVG tam etiketlidir).
            Primitif::Metin { .. } => {}
        }
    }
    tuval.png()
}

/// Basit RGB raster tuvali (alfa yok; opak boyama).
struct RasterTuval {
    gen: usize,
    yuk: usize,
    /// RGB piksel verisi (gen*yuk*3).
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

    fn dikdortgen(&mut self, x: f32, y: f32, gen: f32, yuk: f32, renk: [u8; 3]) {
        let x0 = x.floor() as i64;
        let y0 = y.floor() as i64;
        let x1 = (x + gen.max(0.0)).ceil() as i64;
        let y1 = (y + yuk.max(0.0)).ceil() as i64;
        for yy in y0..y1 {
            for xx in x0..x1 {
                self.nokta(xx, yy, renk);
            }
        }
    }

    /// Çizgi: eksen-hizalı (yatay/dikey) ise kalın bant; aksi halde Bresenham (1 px).
    fn cizgi(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, kalinlik: f32, renk: [u8; 3]) {
        let k = kalinlik.max(1.0);
        if (x1 - x2).abs() < 0.5 {
            // Dikey.
            let yy0 = y1.min(y2);
            let yuk = (y1 - y2).abs().max(1.0);
            self.dikdortgen(x1 - k / 2.0, yy0, k, yuk, renk);
        } else if (y1 - y2).abs() < 0.5 {
            // Yatay.
            let xx0 = x1.min(x2);
            let gen = (x1 - x2).abs().max(1.0);
            self.dikdortgen(xx0, y1 - k / 2.0, gen, k, renk);
        } else {
            // Eğik: Bresenham.
            let (mut x, mut y) = (x1.round() as i64, y1.round() as i64);
            let (xb, yb) = (x2.round() as i64, y2.round() as i64);
            let dx = (xb - x).abs();
            let dy = -(yb - y).abs();
            let sx = if x < xb { 1 } else { -1 };
            let sy = if y < yb { 1 } else { -1 };
            let mut hata = dx + dy;
            loop {
                self.nokta(x, y, renk);
                if x == xb && y == yb {
                    break;
                }
                let e2 = 2 * hata;
                if e2 >= dy {
                    hata += dy;
                    x += sx;
                }
                if e2 <= dx {
                    hata += dx;
                    y += sy;
                }
            }
        }
    }

    /// RGB tuvali RGBA PNG baytlarına kodlar (saf-Rust; zlib *stored*).
    fn png(&self) -> Vec<u8> {
        // Filtre baytı (0=None) önekli RGBA scanline'lar.
        let mut ham = Vec::with_capacity(self.yuk * (1 + self.gen * 4));
        for y in 0..self.yuk {
            ham.push(0u8); // filtre: None
            for x in 0..self.gen {
                let i = (y * self.gen + x) * 3;
                ham.push(self.piksel[i]);
                ham.push(self.piksel[i + 1]);
                ham.push(self.piksel[i + 2]);
                ham.push(255); // alfa opak
            }
        }
        let zlib = zlib_stored(&ham);

        let mut png = Vec::new();
        png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]); // imza
                                                                   // IHDR
        let mut ihdr = Vec::with_capacity(13);
        ihdr.extend_from_slice(&(self.gen as u32).to_be_bytes());
        ihdr.extend_from_slice(&(self.yuk as u32).to_be_bytes());
        ihdr.push(8); // bit derinliği
        ihdr.push(6); // renk tipi: RGBA
        ihdr.push(0); // sıkıştırma
        ihdr.push(0); // filtre
        ihdr.push(0); // interlace
        chunk_yaz(&mut png, b"IHDR", &ihdr);
        chunk_yaz(&mut png, b"IDAT", &zlib);
        chunk_yaz(&mut png, b"IEND", &[]);
        png
    }
}

/// Bir PNG chunk'ı yazar: uzunluk(BE) + tür + veri + CRC32(BE).
fn chunk_yaz(cikti: &mut Vec<u8>, tur: &[u8; 4], veri: &[u8]) {
    cikti.extend_from_slice(&(veri.len() as u32).to_be_bytes());
    cikti.extend_from_slice(tur);
    cikti.extend_from_slice(veri);
    let mut crc_girdi = Vec::with_capacity(4 + veri.len());
    crc_girdi.extend_from_slice(tur);
    crc_girdi.extend_from_slice(veri);
    cikti.extend_from_slice(&crc32(&crc_girdi).to_be_bytes());
}

/// Veriyi zlib *stored* (sıkıştırmasız) akışına sarar: 2 bayt başlık + deflate stored bloklar +
/// 4 bayt Adler-32.
fn zlib_stored(veri: &[u8]) -> Vec<u8> {
    let mut z = Vec::with_capacity(veri.len() + veri.len() / 65535 * 5 + 6);
    z.push(0x78); // CMF
    z.push(0x01); // FLG (stored, kontrol uyumlu: 0x7801 % 31 == 0)
    let mut kalan = veri;
    loop {
        let n = kalan.len().min(65535);
        let son = n == kalan.len();
        z.push(if son { 1 } else { 0 }); // BFINAL, BTYPE=00
        z.extend_from_slice(&(n as u16).to_le_bytes()); // LEN
        z.extend_from_slice(&(!(n as u16)).to_le_bytes()); // NLEN
        z.extend_from_slice(&kalan[..n]);
        kalan = &kalan[n..];
        if son {
            break;
        }
    }
    z.extend_from_slice(&adler32(veri).to_be_bytes());
    z
}

/// Adler-32 sağlama.
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

/// CRC-32 (IEEE 802.3) — PNG chunk sağlaması.
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
    use super::super::canvas::{GenomBolge, Tuval};
    use super::super::cizim::cetvel_ciz;
    use super::super::ruler::cetvel;
    use super::*;

    fn ornek_liste() -> CizimListesi {
        let t = Tuval::yeni(200.0, GenomBolge::yeni("chr1", 1, 200).unwrap());
        let c = cetvel(&t, 5);
        let mut l = CizimListesi::yeni();
        cetvel_ciz(&mut l, &c, t.genislik_px, 20.0);
        l
    }

    #[test]
    fn svg_gecerli_ve_renk_icerir() {
        let l = ornek_liste();
        let svg = svg_olustur(&l, 200.0, 100.0, &Palet::yayin());
        assert!(svg.starts_with("<svg"));
        assert!(svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("<rect"));
        assert!(svg.contains("<text"));
        // Cetvel metni (etiketi) SVG'de var (yayın etiketleri).
        assert!(svg.contains("text-anchor"));
    }

    #[test]
    fn svg_metin_kacisi() {
        let mut l = CizimListesi::yeni();
        // Metin içinde özel XML karakteri — kaçırılmalı.
        l.primitifler.push(Primitif::Metin {
            x: 0.0,
            y: 0.0,
            icerik: "a<b>&\"c".into(),
            renk: CizimRengi::CetvelMetin,
            boyut: 10.0,
            hiza: MetinHiza::Sol,
        });
        let svg = svg_olustur(&l, 50.0, 20.0, &Palet::yayin());
        assert!(svg.contains("a&lt;b&gt;&amp;&quot;c"));
        assert!(!svg.contains("a<b>"));
    }

    #[test]
    fn crc_ve_adler_bilinen_degerler() {
        // CRC32("123456789") = 0xCBF43926 (standart test vektörü).
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        // Adler32("Wikipedia") = 0x11E60398.
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn png_gecerli_yapi_ve_crc() {
        let l = ornek_liste();
        let png = png_olustur(&l, 200, 100, &Palet::yayin());
        // İmza.
        assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);

        // Chunk'ları yürü + CRC doğrula; IHDR boyutları + IEND sonu kontrol.
        let mut i = 8;
        let mut turler: Vec<String> = Vec::new();
        let mut ihdr_gen = 0u32;
        let mut ihdr_yuk = 0u32;
        while i + 8 <= png.len() {
            let uzun = u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]) as usize;
            let tur = &png[i + 4..i + 8];
            let veri = &png[i + 8..i + 8 + uzun];
            let crc = u32::from_be_bytes(png[i + 8 + uzun..i + 12 + uzun].try_into().unwrap());
            let mut crc_girdi = tur.to_vec();
            crc_girdi.extend_from_slice(veri);
            assert_eq!(crc, crc32(&crc_girdi), "chunk CRC tutmuyor");
            if tur == b"IHDR" {
                ihdr_gen = u32::from_be_bytes(veri[0..4].try_into().unwrap());
                ihdr_yuk = u32::from_be_bytes(veri[4..8].try_into().unwrap());
            }
            turler.push(String::from_utf8_lossy(tur).into_owned());
            i += 12 + uzun;
        }
        assert_eq!(ihdr_gen, 200);
        assert_eq!(ihdr_yuk, 100);
        assert_eq!(turler.first().unwrap(), "IHDR");
        assert_eq!(turler.last().unwrap(), "IEND");
        assert!(turler.iter().any(|t| t == "IDAT"));
        assert_eq!(i, png.len(), "chunk'lar dosyayı tam kaplamalı");
    }
}
