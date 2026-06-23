//! ÇE-02 — **Gezinme**: bölge ayrıştırma (`chr:start-end` / `chr:pos` / gen adı), pan (kaydırma),
//! zoom (yakınlaştırma) ve **gezinme geçmişi** (geri/ileri).
//!
//! Tüm gezinme saf fonksiyonlardır (dosya/IO yok → birim test edilebilir).  Gen adı çözümü bir
//! [`GenAdiCozucu`] trait'i arkasındadır; tarayıcı bunu anotasyon iziyle (ileride) doldurur,
//! testte basit bir tablo ile doldurur.

use std::collections::HashMap;

use biocraft_sdk::biocraft_types::ErrorReport;

use super::canvas::GenomBolge;

/// Bir konum/bölge metnini bir gen/sembol adından [`GenomBolge`]'ye çözen kaynak.
///
/// Tarayıcı "bölgeye git" kutusuna `chr1:1000-2000` gibi koordinat **veya** `BRCA1` gibi bir ad
/// girilebilmesini ister.  Koordinat saf ayrıştırılır; ad bu trait'le çözülür (anotasyon/veritabanı).
pub trait GenAdiCozucu {
    /// Verilen adı (büyük/küçük harf duyarsız önerilir) bir bölgeye çözer; bulunamazsa `None`.
    fn coz(&self, ad: &str) -> Option<GenomBolge>;
}

/// Bellek-içi basit gen→bölge tablosu (anotasyondan doldurulur; test/varsayılan çözücü).
#[derive(Debug, Clone, Default)]
pub struct TabloCozucu {
    tablo: HashMap<String, GenomBolge>,
}

impl TabloCozucu {
    /// Boş tablo.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir gen adı→bölge eşlemesi ekler (ad küçük harfe indirilerek saklanır).
    pub fn ekle(&mut self, ad: impl Into<String>, bolge: GenomBolge) {
        self.tablo.insert(ad.into().to_lowercase(), bolge);
    }

    /// Tablodaki eşleme sayısı.
    pub fn sayi(&self) -> usize {
        self.tablo.len()
    }
}

impl GenAdiCozucu for TabloCozucu {
    fn coz(&self, ad: &str) -> Option<GenomBolge> {
        self.tablo.get(&ad.trim().to_lowercase()).cloned()
    }
}

/// Bir "bölgeye git" metnini çözer.
///
/// * Metinde `:` varsa **koordinat** (`chr:start-end` / `chr:pos`) olarak ayrıştırılır; geçersizse
///   hata (çıplak ad denenmez — kullanıcı koordinat yazmak istemiştir).
/// * `:` yoksa **çıplak ad**tır: önce `cozucu` ile **gen adı** denenir (örn. `BRCA1`), bulunamazsa
///   hata.  (Çıplak *kromozom* adına gidiş — `chr2` — kromozom listesini bilen üst katmandadır;
///   serbest fonksiyon, bilinmeyen bir adı sessizce "kromozom" sanmaz.)
///
/// `varsayilan_pencere` tek-konum (`chr:pos`) girişinde merkez çevresinde açılacak bp uzunluğudur.
pub fn bolgeye_git(
    metin: &str,
    cozucu: Option<&dyn GenAdiCozucu>,
    varsayilan_pencere: u64,
) -> Result<GenomBolge, ErrorReport> {
    let t = metin.trim();
    if t.is_empty() {
        return Err(bos_giris_hatasi());
    }
    if t.contains(':') {
        return koordinat_ayristir(t, varsayilan_pencere).ok_or_else(|| cozulemedi_hatasi(t));
    }
    // Çıplak ad → gen çözücü (kromozom adı belirsizliğini üst katman çözer).
    if let Some(c) = cozucu {
        if let Some(b) = c.coz(t) {
            return Ok(b);
        }
    }
    Err(cozulemedi_hatasi(t))
}

/// `chr:start-end` / `chr:pos` / `chr` koordinat biçimlerini ayrıştırır (gen adı denenmeden).
/// Sayılarda binlik ayraç (virgül/alt-çizgi/boşluk) ve uçlardaki boşluk hoş görülür.
pub fn koordinat_ayristir(metin: &str, varsayilan_pencere: u64) -> Option<GenomBolge> {
    let t = metin.trim();
    match t.rsplit_once(':') {
        None => {
            // Yalnız kromozom adı — boş/iki nokta yoksa.  Geçerli adsa baştan pencere.
            if t.is_empty() || t.contains(|c: char| c.is_whitespace()) {
                None
            } else {
                Some(GenomBolge {
                    kromozom: t.to_string(),
                    baslangic: 1,
                    bitis: varsayilan_pencere.max(1),
                })
            }
        }
        Some((krom, aralik)) => {
            let krom = krom.trim();
            if krom.is_empty() {
                return None;
            }
            let aralik = aralik.trim();
            match aralik.split_once('-') {
                Some((a, b)) => {
                    let bas = sayi_coz(a)?;
                    let bit = sayi_coz(b)?;
                    GenomBolge::yeni(krom, bas, bit).ok()
                }
                None => {
                    // Tek konum → merkez çevresinde varsayılan pencere.
                    let pos = sayi_coz(aralik)?;
                    Some(GenomBolge::merkezli(krom, pos, varsayilan_pencere))
                }
            }
        }
    }
}

/// Bir sayıyı binlik ayraçları (`,`/`_`/boşluk) yok sayarak çözer.
fn sayi_coz(s: &str) -> Option<u64> {
    let temiz: String = s
        .chars()
        .filter(|c| !matches!(c, ',' | '_' | ' '))
        .collect();
    if temiz.is_empty() {
        None
    } else {
        temiz.parse::<u64>().ok()
    }
}

// ─── Pan / Zoom ─────────────────────────────────────────────────────────────────

/// Bölgeyi `delta_bp` kadar **kaydırır** (pan).  Pozitif = sağa (daha büyük koordinata).
/// Sol kenar 1'in altına inmez (uzunluk korunur).  `kromozom_uzunlugu` verilirse sağdan da
/// sınırlanır.
pub fn kaydir(bolge: &GenomBolge, delta_bp: i64, kromozom_uzunlugu: Option<u64>) -> GenomBolge {
    let uzun = bolge.uzunluk();
    let yeni_bas = if delta_bp >= 0 {
        bolge.baslangic.saturating_add(delta_bp as u64)
    } else {
        bolge.baslangic.saturating_sub((-delta_bp) as u64)
    }
    .max(1);
    GenomBolge {
        kromozom: bolge.kromozom.clone(),
        baslangic: yeni_bas,
        bitis: yeni_bas + uzun - 1,
    }
    .sinirla(kromozom_uzunlugu)
}

/// En küçük görünür pencere (bp) — daha fazla yakınlaşılmaz (tek baz görünür düzey).
pub const ASGARI_PENCERE_BP: u64 = 40;

/// Bölgeyi `odak` pozisyonu sabit kalacak şekilde `faktor` ile yakınlaştırır/uzaklaştırır.
/// `faktor < 1` → yakınlaş (pencere küçülür); `faktor > 1` → uzaklaş.  Pencere
/// `[ASGARI_PENCERE_BP, kromozom_uzunlugu]` aralığına sıkıştırılır.
pub fn yakinlastir(
    bolge: &GenomBolge,
    faktor: f64,
    odak: u64,
    kromozom_uzunlugu: Option<u64>,
) -> GenomBolge {
    let eski_uzun = bolge.uzunluk();
    let faktor = faktor.max(1e-6);
    let mut yeni_uzun = ((eski_uzun as f64) * faktor).round() as u64;
    yeni_uzun = yeni_uzun.max(ASGARI_PENCERE_BP);
    if let Some(maks) = kromozom_uzunlugu {
        yeni_uzun = yeni_uzun.min(maks.max(1));
    }

    // Odağın pencere içindeki oranını koru (odak ekranda sabit kalır).
    let odak = odak.clamp(bolge.baslangic, bolge.bitis);
    let oran = (odak - bolge.baslangic) as f64 / (eski_uzun.max(1) as f64);
    let yeni_bas_i = odak as i64 - (oran * yeni_uzun as f64).round() as i64;
    let yeni_bas = yeni_bas_i.max(1) as u64;

    GenomBolge {
        kromozom: bolge.kromozom.clone(),
        baslangic: yeni_bas,
        bitis: yeni_bas + yeni_uzun - 1,
    }
    .sinirla(kromozom_uzunlugu)
}

// ─── Gezinme geçmişi (geri / ileri) ─────────────────────────────────────────────

/// Tarayıcı gibi geri/ileri yığını.  `git` ileri dalını budar; `geri`/`ileri` imleci kaydırır.
#[derive(Debug, Clone, PartialEq)]
pub struct GezinmeGecmisi {
    gecmis: Vec<GenomBolge>,
    imlec: usize,
}

impl GezinmeGecmisi {
    /// Bir başlangıç bölgesiyle başlar (imleç o bölgede).
    pub fn yeni(baslangic: GenomBolge) -> Self {
        Self {
            gecmis: vec![baslangic],
            imlec: 0,
        }
    }

    /// Şu anki bölge.
    pub fn simdiki(&self) -> &GenomBolge {
        &self.gecmis[self.imlec]
    }

    /// Yeni bir bölgeye gider: imleçten sonrasını budar, ekler, imleci sona alır.  Aynı bölgeye
    /// gidiş (tekrar) yok sayılır (gürültü olmaz).
    pub fn git(&mut self, bolge: GenomBolge) {
        if self.gecmis[self.imlec] == bolge {
            return;
        }
        self.gecmis.truncate(self.imlec + 1);
        self.gecmis.push(bolge);
        self.imlec = self.gecmis.len() - 1;
    }

    /// Geri gidilebilir mi?
    pub fn geri_var_mi(&self) -> bool {
        self.imlec > 0
    }

    /// İleri gidilebilir mi?
    pub fn ileri_var_mi(&self) -> bool {
        self.imlec + 1 < self.gecmis.len()
    }

    /// Bir adım geri; yeni şu anki bölgeyi döner (yoksa `None`).
    pub fn geri(&mut self) -> Option<&GenomBolge> {
        if self.geri_var_mi() {
            self.imlec -= 1;
            Some(&self.gecmis[self.imlec])
        } else {
            None
        }
    }

    /// Bir adım ileri; yeni şu anki bölgeyi döner (yoksa `None`).
    pub fn ileri(&mut self) -> Option<&GenomBolge> {
        if self.ileri_var_mi() {
            self.imlec += 1;
            Some(&self.gecmis[self.imlec])
        } else {
            None
        }
    }

    /// Geçmişteki toplam giriş sayısı.
    pub fn sayi(&self) -> usize {
        self.gecmis.len()
    }
}

// ─── Hatalar ────────────────────────────────────────────────────────────────────

fn bos_giris_hatasi() -> ErrorReport {
    ErrorReport::new(
        "Boş bölge girişi",
        "gidilecek bölge/gen adı boş",
        "Bir koordinat (chr1:1000-2000) veya gen adı (BRCA1) girin",
    )
}

fn cozulemedi_hatasi(metin: &str) -> ErrorReport {
    ErrorReport::new(
        "Bölge çözülemedi",
        format!("'{metin}' bir koordinata veya bilinen bir gen adına çözülemedi"),
        "Koordinatı 'chr1:1000-2000' biçiminde yazın ya da anotasyon izi yükleyerek gen adıyla arayın",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn koordinat_bicimleri() {
        // chr:start-end
        let b = koordinat_ayristir("chr1:1000-2000", 100).unwrap();
        assert_eq!(
            (b.kromozom.as_str(), b.baslangic, b.bitis),
            ("chr1", 1000, 2000)
        );

        // Binlik ayraçlı.
        let b2 = koordinat_ayristir("chr2:1,000,000-1,000,500", 100).unwrap();
        assert_eq!((b2.baslangic, b2.bitis), (1_000_000, 1_000_500));

        // chr:pos → merkez çevresinde pencere (varsayılan 100 → 51 bp yarıçap).
        let b3 = koordinat_ayristir("chrX:5000", 101).unwrap();
        assert_eq!(b3.merkez(), 5000);
        assert_eq!(b3.uzunluk(), 101);

        // Yalnız kromozom → baştan pencere.
        let b4 = koordinat_ayristir("chr3", 1000).unwrap();
        assert_eq!((b4.baslangic, b4.bitis), (1, 1000));

        // Geçersiz aralık (bitiş < başlangıç) → None.
        assert!(koordinat_ayristir("chr1:200-100", 100).is_none());
    }

    #[test]
    fn gen_adi_cozumu() {
        let mut tablo = TabloCozucu::yeni();
        tablo.ekle(
            "BRCA1",
            GenomBolge::yeni("chr17", 43044295, 43125483).unwrap(),
        );
        assert_eq!(tablo.sayi(), 1);

        // Koordinat değil ama gen adı → çözülür (büyük/küçük harf duyarsız).
        let b = bolgeye_git("brca1", Some(&tablo), 100).unwrap();
        assert_eq!(b.kromozom, "chr17");

        // Çözülemeyen ad → hata.
        let hata = bolgeye_git("YOKGEN", Some(&tablo), 100).unwrap_err();
        assert_eq!(hata.ne_oldu, "Bölge çözülemedi");

        // Boş giriş → hata.
        assert!(bolgeye_git("  ", Some(&tablo), 100).is_err());
    }

    #[test]
    fn pan_sol_kenari_korur() {
        let b = GenomBolge::yeni("chr1", 100, 199).unwrap();
        let sag = kaydir(&b, 50, None);
        assert_eq!((sag.baslangic, sag.bitis), (150, 249));
        let sol = kaydir(&b, -200, None); // 100-200 = taşar → 1'e sabit
        assert_eq!(sol.baslangic, 1);
        assert_eq!(sol.uzunluk(), 100, "pan uzunluğu korur");

        // Kromozom uzunluğu sınırı (sağ).
        let snr = kaydir(&b, 10_000, Some(500));
        assert_eq!(snr.bitis, 500);
        assert_eq!(snr.uzunluk(), 100);
    }

    #[test]
    fn zoom_odagi_sabit_tutar() {
        // 1000 bp pencere (1..=1000), odak 250'de; 0.5 ile yakınlaş → 500 bp.
        let b = GenomBolge::yeni("chr1", 1, 1000).unwrap();
        let y = yakinlastir(&b, 0.5, 250, None);
        assert_eq!(y.uzunluk(), 500);
        // Odak oranı (~0.249) korunur → odak hâlâ pencere içinde ve benzer oranda.
        let oran = (250 - y.baslangic) as f64 / y.uzunluk() as f64;
        assert!((oran - 0.249).abs() < 0.02, "odak oranı korunmalı: {oran}");

        // Asgari pencere sınırı.
        let kucuk = yakinlastir(&b, 0.0001, 500, None);
        assert_eq!(kucuk.uzunluk(), ASGARI_PENCERE_BP);

        // Uzaklaşma kromozom uzunluğuyla sınırlı.
        let genis = yakinlastir(&b, 100.0, 500, Some(2000));
        assert_eq!(genis.uzunluk(), 2000);
    }

    #[test]
    fn gecmis_geri_ileri_ve_budama() {
        let a = GenomBolge::yeni("chr1", 1, 100).unwrap();
        let mut g = GezinmeGecmisi::yeni(a.clone());
        assert!(!g.geri_var_mi() && !g.ileri_var_mi());

        let b = GenomBolge::yeni("chr1", 200, 300).unwrap();
        let c = GenomBolge::yeni("chr1", 400, 500).unwrap();
        g.git(b.clone());
        g.git(c.clone());
        assert_eq!(g.sayi(), 3);
        assert!(g.geri_var_mi() && !g.ileri_var_mi());

        // Aynı bölgeye tekrar gidiş yok sayılır.
        g.git(c.clone());
        assert_eq!(g.sayi(), 3);

        // Geri → b, sonra ileri → c.
        assert_eq!(g.geri().unwrap(), &b);
        assert!(g.ileri_var_mi());
        assert_eq!(g.ileri().unwrap(), &c);

        // Ortadayken yeni bölgeye gidince ileri dalı budanır.
        g.geri(); // b
        let d = GenomBolge::yeni("chr1", 600, 700).unwrap();
        g.git(d.clone());
        assert_eq!(g.simdiki(), &d);
        assert!(!g.ileri_var_mi(), "ileri dalı budanmalı");
        assert_eq!(g.sayi(), 3); // a, b, d
    }
}
