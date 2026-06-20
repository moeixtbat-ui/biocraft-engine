//! Ölçeklenebilirlik altyapısı — görünür-alan **culling** + **LOD** (Level of Detail) API'si.
//!
//! Büyük genom/node tuvali ve 3B yapılar (milyonlarca öğe) için: ekranda görünmeyen nesneler
//! çizilmeden elenir (culling), görünenler uzaklık/ekran-boyutuna göre kabalaştırılır (LOD).
//! Bu, MK-04'ün "kare bütçesi korunur" güvencesinin veri tarafıdır — GPU'ya yalnızca gerçekten
//! görünen ve gerektiği kadar ayrıntılı iş gider.
//!
//! Saf geometri/matematik (egui/wgpu'dan bağımsız, MK-40); hem 2B plot hem 3B sahne kullanır.
//!
//! Statik ekranda FPS düşürme (Eco) kare bütçesi tarafındadır — bkz. [`crate::frame_budget`].
// MK-04: görünür-alan culling + LOD; kare bütçesini korur.

/// Eksen-hizalı dikdörtgen (2B kutu).  Hem ekran bölgesi hem nesne sınır-kutusu (AABB) olur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dortgen {
    /// Sol kenar.
    pub x: f32,
    /// Üst kenar.
    pub y: f32,
    /// Genişlik (≥ 0).
    pub w: f32,
    /// Yükseklik (≥ 0).
    pub h: f32,
}

impl Dortgen {
    /// Köşe + boyuttan dikdörtgen.
    pub const fn yeni(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Sağ kenar (x + w).
    pub fn sag(&self) -> f32 {
        self.x + self.w
    }

    /// Alt kenar (y + h).
    pub fn alt(&self) -> f32 {
        self.y + self.h
    }

    /// Bu dikdörtgen `diger` ile kesişiyor mu (görünür-alan testi).
    pub fn kesisiyor(&self, diger: &Dortgen) -> bool {
        self.x < diger.sag() && self.sag() > diger.x && self.y < diger.alt() && self.alt() > diger.y
    }

    /// Bir noktayı içeriyor mu.
    pub fn icerir_nokta(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.sag() && py >= self.y && py <= self.alt()
    }

    /// Ekran-uzayı (piksel) alanı — LOD seçiminde "nesne ne kadar büyük görünüyor" ölçütü.
    pub fn alan(&self) -> f32 {
        (self.w.max(0.0)) * (self.h.max(0.0))
    }
}

/// Bir nesnenin ne kadar ayrıntılı çizileceği.  Görünür-alandaki ekran boyutuna göre seçilir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LodKademe {
    /// Görünmüyor → hiç çizme (culled).
    Gizli,
    /// Çok küçük/uzak → en kaba temsil (tek nokta/kutu).
    Kaba,
    /// Orta → azaltılmış ayrıntı.
    Orta,
    /// Yakın/büyük → tam ayrıntı.
    Tam,
}

impl LodKademe {
    /// Çizilebilir mi (Gizli değilse).
    pub fn cizilir(&self) -> bool {
        !matches!(self, LodKademe::Gizli)
    }
}

/// Görünür-alan (viewport) + LOD eşiklerini tutan seçici.
#[derive(Debug, Clone, Copy)]
pub struct LodSecici {
    /// Görünür ekran bölgesi (bunun dışındaki her şey elenir).
    pub gorunur: Dortgen,
    /// Bu ekran-piksel-alanının **altında** kalan nesne `Kaba` çizilir.
    pub kaba_esik: f32,
    /// Bu ekran-piksel-alanının **üstündeki** nesne `Tam` çizilir; arası `Orta`.
    pub tam_esik: f32,
}

impl LodSecici {
    /// Bir görünür bölge için makul varsayılan eşiklerle seçici (kaba < 64 px², tam > 1024 px²).
    pub fn yeni(gorunur: Dortgen) -> Self {
        Self {
            gorunur,
            kaba_esik: 64.0,
            tam_esik: 1024.0,
        }
    }

    /// Bir nesnenin sınır-kutusuna göre kademesini seçer (önce culling, sonra LOD).
    pub fn kademe(&self, nesne: &Dortgen) -> LodKademe {
        if !self.gorunur.kesisiyor(nesne) {
            return LodKademe::Gizli;
        }
        let alan = nesne.alan();
        if alan < self.kaba_esik {
            LodKademe::Kaba
        } else if alan >= self.tam_esik {
            LodKademe::Tam
        } else {
            LodKademe::Orta
        }
    }

    /// Görünür kalan (Gizli olmayan) nesnelerin indekslerini döndürür (culling sonucu).
    pub fn gorunenler<'a>(
        &'a self,
        nesneler: &'a [Dortgen],
    ) -> impl Iterator<Item = (usize, LodKademe)> + 'a {
        nesneler
            .iter()
            .enumerate()
            .map(|(i, d)| (i, self.kademe(d)))
            .filter(|(_, k)| k.cizilir())
    }
}

/// Bir nokta dizisini en fazla `hedef` öğeye **eşit aralıkla** seyreltir (decimation/LOD).
///
/// Çok yoğun veride (örn. piksel başına düşen birden fazla nokta) GPU'ya gönderilen iş azalır;
/// görsel olarak fark edilmez ama kare bütçesi korunur.  `hedef == 0` ise boş döner; dizi
/// zaten `hedef`'ten küçükse tüm indeksler döner.
pub fn seyrelt(toplam: usize, hedef: usize) -> Vec<usize> {
    if hedef == 0 || toplam == 0 {
        return Vec::new();
    }
    if toplam <= hedef {
        return (0..toplam).collect();
    }
    let mut cikti = Vec::with_capacity(hedef);
    for i in 0..hedef {
        // İlk ve son nokta korunur; aradakiler eşit aralıkla örneklenir.
        let idx = (i * (toplam - 1)) / (hedef - 1).max(1);
        cikti.push(idx);
    }
    cikti.dedup();
    cikti
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kesisim_ve_icerme_dogru() {
        let a = Dortgen::yeni(0.0, 0.0, 10.0, 10.0);
        assert!(a.kesisiyor(&Dortgen::yeni(5.0, 5.0, 10.0, 10.0)));
        assert!(!a.kesisiyor(&Dortgen::yeni(20.0, 20.0, 5.0, 5.0)));
        assert!(a.icerir_nokta(5.0, 5.0));
        assert!(!a.icerir_nokta(11.0, 5.0));
    }

    #[test]
    fn gorunur_alan_disindaki_nesne_elenir() {
        let s = LodSecici::yeni(Dortgen::yeni(0.0, 0.0, 100.0, 100.0));
        // Ekran dışı → Gizli (culled).
        assert_eq!(
            s.kademe(&Dortgen::yeni(500.0, 500.0, 50.0, 50.0)),
            LodKademe::Gizli
        );
        assert!(!LodKademe::Gizli.cizilir());
    }

    #[test]
    fn lod_kademesi_ekran_boyutuna_gore_secilir() {
        let s = LodSecici::yeni(Dortgen::yeni(0.0, 0.0, 1000.0, 1000.0));
        // Küçük (5x5=25 px² < 64) → Kaba.
        assert_eq!(
            s.kademe(&Dortgen::yeni(10.0, 10.0, 5.0, 5.0)),
            LodKademe::Kaba
        );
        // Orta (20x20=400) → Orta.
        assert_eq!(
            s.kademe(&Dortgen::yeni(10.0, 10.0, 20.0, 20.0)),
            LodKademe::Orta
        );
        // Büyük (50x50=2500 > 1024) → Tam.
        assert_eq!(
            s.kademe(&Dortgen::yeni(10.0, 10.0, 50.0, 50.0)),
            LodKademe::Tam
        );
    }

    #[test]
    fn culling_yalnizca_gorunenleri_birakir() {
        let s = LodSecici::yeni(Dortgen::yeni(0.0, 0.0, 100.0, 100.0));
        let nesneler = vec![
            Dortgen::yeni(10.0, 10.0, 20.0, 20.0), // görünür
            Dortgen::yeni(500.0, 0.0, 10.0, 10.0), // ekran dışı
            Dortgen::yeni(50.0, 50.0, 30.0, 30.0), // görünür
        ];
        let gorunen: Vec<usize> = s.gorunenler(&nesneler).map(|(i, _)| i).collect();
        assert_eq!(gorunen, vec![0, 2]);
    }

    #[test]
    fn seyreltme_hedef_sayiya_indirir_uclari_korur() {
        let idx = seyrelt(1000, 100);
        assert!(idx.len() <= 100);
        assert_eq!(*idx.first().unwrap(), 0);
        assert_eq!(*idx.last().unwrap(), 999); // son nokta korunur
                                               // Zaten küçükse hepsi döner.
        assert_eq!(seyrelt(5, 100).len(), 5);
        assert!(seyrelt(100, 0).is_empty());
    }
}
