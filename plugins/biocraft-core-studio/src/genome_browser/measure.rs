//! ÇE-02 — **Ölçüm ve işaretleme araçları** (Gün 37): iki nokta arası bp mesafesi, pozisyon
//! kopyalama, bölge işaretleme ve **yer imleri** (bookmark; geri-alınabilir).
//!
//! Hepsi saf veri/fonksiyondur (egui/IO yok → birim test edilebilir); çizimleri
//! [`super::cizim`] modülü render-bağımsız ilkellere derler.

use super::canvas::{GenomBolge, Tuval};
use super::ruler::{pozisyon_etiketle, Olcek};

/// İki genom konumu arasında **ölçüm** (cetvel/mesafe aracı).  Konumlar 1-tabanlıdır; sıralı
/// değilse `sol`/`sag` ile normalize edilir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Olcum {
    /// Birinci tıklanan konum (1-tabanlı).
    pub a: u64,
    /// İkinci tıklanan konum (1-tabanlı).
    pub b: u64,
}

impl Olcum {
    /// İki konumdan ölçüm kurar.
    pub fn yeni(a: u64, b: u64) -> Self {
        Self { a, b }
    }

    /// Sol (küçük) uç.
    pub fn sol(self) -> u64 {
        self.a.min(self.b)
    }

    /// Sağ (büyük) uç.
    pub fn sag(self) -> u64 {
        self.a.max(self.b)
    }

    /// İki nokta arasındaki **kapsayıcı** bp mesafesi (`|b−a| + 1`) — IGV "span" davranışı:
    /// aynı baza iki kez tıklanırsa 1 bp.
    pub fn mesafe_bp(self) -> u64 {
        self.sag() - self.sol() + 1
    }

    /// İnsan-okur ölçüm etiketi (ör. "1,234 bp", "12.5 kb").
    pub fn etiket(self) -> String {
        let m = self.mesafe_bp();
        pozisyon_etiketle(m, Olcek::sec(m))
    }
}

/// Bir ekran x'indeki genom konumunu **kopyalanabilir** metne çevirir (ör. `chr1:12,345`).
pub fn konum_metni(tuval: &Tuval) -> impl Fn(f32) -> String + '_ {
    move |x: f32| {
        let pos = tuval.ekran_to_genom(x);
        format!("{}:{}", tuval.bolge.kromozom, binlik(pos))
    }
}

/// Bir bölgeyi kopyalanabilir metne çevirir (ör. `chr1:1,000-2,000`).
pub fn bolge_metni(bolge: &GenomBolge) -> String {
    format!(
        "{}:{}-{}",
        bolge.kromozom,
        binlik(bolge.baslangic),
        binlik(bolge.bitis)
    )
}

/// Kullanıcının kaydettiği bir bölge (yer imi / bookmark).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yerimi {
    /// Kullanıcı adı/notu.
    pub ad: String,
    /// İşaretlenen bölge.
    pub bolge: GenomBolge,
}

impl Yerimi {
    /// Bir yer imi kurar.
    pub fn yeni(ad: impl Into<String>, bolge: GenomBolge) -> Self {
        Self {
            ad: ad.into(),
            bolge,
        }
    }
}

/// Yer imleri listesi — ekle / sil (geri-alınabilir) / listele.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct YerimleriListesi {
    ogeler: Vec<Yerimi>,
}

impl YerimleriListesi {
    /// Boş liste.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Yer imi ekler (sona); eklenen indeksi döner.
    pub fn ekle(&mut self, y: Yerimi) -> usize {
        self.ogeler.push(y);
        self.ogeler.len() - 1
    }

    /// Bir indeksteki yer imini siler ve **geri döndürür** (TDA: geri-alınabilir → UI tekrar
    /// `ekle` ile geri koyabilir).  Geçersiz indekste `None`.
    pub fn sil(&mut self, indeks: usize) -> Option<Yerimi> {
        if indeks < self.ogeler.len() {
            Some(self.ogeler.remove(indeks))
        } else {
            None
        }
    }

    /// Tüm yer imleri (sıralı, salt-okur).
    pub fn tumu(&self) -> &[Yerimi] {
        &self.ogeler
    }

    /// Yer imi sayısı.
    pub fn sayi(&self) -> usize {
        self.ogeler.len()
    }

    /// Bir indeksteki yer imi.
    pub fn al(&self, indeks: usize) -> Option<&Yerimi> {
        self.ogeler.get(indeks)
    }
}

/// Bir sayıya binlik ayraç ekler (`12345` → `12,345`).
fn binlik(n: u64) -> String {
    let s = n.to_string();
    let uzun = s.len();
    let mut c = String::with_capacity(uzun + uzun / 3);
    for (i, ch) in s.chars().enumerate() {
        if i != 0 && (uzun - i) % 3 == 0 {
            c.push(',');
        }
        c.push(ch);
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn olcum_mesafe_ve_etiket() {
        // 1000..2000 → 1001 bp (kapsayıcı).
        let o = Olcum::yeni(2000, 1000);
        assert_eq!(o.sol(), 1000);
        assert_eq!(o.sag(), 2000);
        assert_eq!(o.mesafe_bp(), 1001);
        // Aynı baz → 1 bp.
        assert_eq!(Olcum::yeni(500, 500).mesafe_bp(), 1);
        // Etiket ölçeği.
        assert_eq!(Olcum::yeni(1, 250).etiket(), "250 bp");
    }

    #[test]
    fn konum_ve_bolge_metni() {
        let t = Tuval::yeni(1000.0, GenomBolge::yeni("chr1", 1000, 1999).unwrap());
        let f = konum_metni(&t);
        // x=0 → ilk konum (1000).
        assert_eq!(f(0.0), "chr1:1,000");
        assert_eq!(
            bolge_metni(&GenomBolge::yeni("chr2", 1_000_000, 2_000_000).unwrap()),
            "chr2:1,000,000-2,000,000"
        );
    }

    #[test]
    fn yerimleri_ekle_sil_geri_al() {
        let mut y = YerimleriListesi::yeni();
        let i = y.ekle(Yerimi::yeni(
            "MYC",
            GenomBolge::yeni("chr8", 127_700_000, 127_800_000).unwrap(),
        ));
        assert_eq!(y.sayi(), 1);
        assert_eq!(y.al(i).unwrap().ad, "MYC");

        // Sil → öğeyi döndürür (geri-alma için).
        let silinen = y.sil(0).unwrap();
        assert_eq!(y.sayi(), 0);
        assert_eq!(silinen.ad, "MYC");

        // Geri al = yeniden ekle.
        y.ekle(silinen);
        assert_eq!(y.sayi(), 1);

        // Geçersiz indeks → None.
        assert!(y.sil(9).is_none());
    }
}
