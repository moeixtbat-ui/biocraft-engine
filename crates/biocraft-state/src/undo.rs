//! Geri-al / yinele yığını — çok-adımlı geçmiş (İP-11, MK-36).
//!
//! İki yığın tutulur:
//! - **geçmiş** (undo): uygulanmış komutlar; en sona uygulanan en üsttedir (LIFO).
//! - **ileri** (redo): geri alınmış komutlar; "yinele" buradan geri yüklenir.
//!
//! Kurallar:
//! - Yeni bir komut çalıştırıldığında **ileri yığın temizlenir** (geçmişten saparak yeni iş
//!   yapmak eski "yinele" dalını geçersiz kılar — standart editör davranışı).
//! - Geçmiş, bellek için bir **derinlik sınırıyla** tutulur; sınır aşılınca en eski komut atılır
//!   (artık geri alınamaz, ama uygulanmış kalır — veri kaybı yok).
//!
//! Motor hedef model `H` üzerinde geneldir; *ne* değiştiğini bilmez, yalnızca komutları yönetir.
//! Sonraki paketler (node/kod/ayar) kendi `H`'leriyle aynı motoru kullanır.

use std::collections::VecDeque;

use biocraft_types::ErrorReport;

use crate::command::Komut;

/// Varsayılan geçmiş derinliği (komut sayısı).  Bellek ile kullanışlılık dengesi.
pub const VARSAYILAN_DERINLIK: usize = 200;

/// Çok-adımlı geri-al / yinele motoru (tek hedef model `H` = tek mantıksal depo için).
pub struct GeriAlYigini<H> {
    gecmis: VecDeque<Box<dyn Komut<H>>>,
    ileri: Vec<Box<dyn Komut<H>>>,
    derinlik: usize,
}

impl<H> GeriAlYigini<H> {
    /// Varsayılan derinlikle ([`VARSAYILAN_DERINLIK`]) boş bir yığın kurar.
    pub fn yeni() -> Self {
        Self::derinlikle(VARSAYILAN_DERINLIK)
    }

    /// Belirtilen geçmiş derinliğiyle boş bir yığın kurar (en az 1).
    pub fn derinlikle(derinlik: usize) -> Self {
        Self {
            gecmis: VecDeque::new(),
            ileri: Vec::new(),
            derinlik: derinlik.max(1),
        }
    }

    /// Bir komutu çalıştırır: hedefe `uygula`, geçmişe ekler, **ileri (redo) yığınını temizler**.
    ///
    /// `uygula` başarısız olursa komut geçmişe **eklenmez** (hedef değişmediği varsayılır; komut
    /// kendi atomikliğinden sorumludur) ve hata döndürülür.
    pub fn calistir(
        &mut self,
        hedef: &mut H,
        mut komut: Box<dyn Komut<H>>,
    ) -> Result<(), ErrorReport> {
        komut.uygula(hedef)?;
        self.ileri.clear(); // yeni iş → eski "yinele" dalı geçersiz.
        self.gecmis.push_back(komut);
        // Derinlik sınırı: en eski komutu at (uygulanmış kalır, yalnızca geri alınamaz olur).
        while self.gecmis.len() > self.derinlik {
            self.gecmis.pop_front();
        }
        Ok(())
    }

    /// Son komutu geri alır (ters-işlem) ve "yinele" için ileri yığına taşır.
    ///
    /// Geri alınacak komut yoksa `Ok(false)`; geri alındıysa `Ok(true)`.
    pub fn geri_al(&mut self, hedef: &mut H) -> Result<bool, ErrorReport> {
        let Some(mut komut) = self.gecmis.pop_back() else {
            return Ok(false);
        };
        if let Err(e) = komut.geri_al(hedef) {
            // Geri alma başarısızsa komutu geçmişte bırak (tutarlı kal) ve hatayı bildir.
            self.gecmis.push_back(komut);
            return Err(e);
        }
        self.ileri.push(komut);
        Ok(true)
    }

    /// Son geri alınan komutu yeniden uygular (yinele) ve geçmişe geri taşır.
    ///
    /// Yinelenecek komut yoksa `Ok(false)`; yinelendiyse `Ok(true)`.
    pub fn yinele(&mut self, hedef: &mut H) -> Result<bool, ErrorReport> {
        let Some(mut komut) = self.ileri.pop() else {
            return Ok(false);
        };
        if let Err(e) = komut.uygula(hedef) {
            self.ileri.push(komut);
            return Err(e);
        }
        self.gecmis.push_back(komut);
        Ok(true)
    }

    /// Geri alınabilecek bir komut var mı?
    pub fn geri_alinabilir_mi(&self) -> bool {
        !self.gecmis.is_empty()
    }

    /// Yinelenebilecek bir komut var mı?
    pub fn yinelenebilir_mi(&self) -> bool {
        !self.ileri.is_empty()
    }

    /// Sıradaki "geri al"ın açıklaması (menü etiketi: "Geri Al: Tema değiştir").  Yoksa `None`.
    pub fn sonraki_geri_al(&self) -> Option<String> {
        self.gecmis.back().map(|k| k.aciklama())
    }

    /// Sıradaki "yinele"nin açıklaması.  Yoksa `None`.
    pub fn sonraki_yinele(&self) -> Option<String> {
        self.ileri.last().map(|k| k.aciklama())
    }

    /// Geçmişteki komutların açıklamaları (eskiden yeniye) — UI'da geçmiş listesi için.
    pub fn gecmis_aciklamalari(&self) -> Vec<String> {
        self.gecmis.iter().map(|k| k.aciklama()).collect()
    }

    /// Geçmişteki (geri-alınabilir) komut sayısı.
    pub fn gecmis_uzunlugu(&self) -> usize {
        self.gecmis.len()
    }

    /// İleri (yinelenebilir) komut sayısı.
    pub fn ileri_uzunlugu(&self) -> usize {
        self.ileri.len()
    }

    /// Tüm geçmişi ve ileri yığını siler (komutların etkisi hedefte kalır; yalnızca geçmiş silinir).
    pub fn temizle(&mut self) {
        self.gecmis.clear();
        self.ileri.clear();
    }
}

impl<H> Default for GeriAlYigini<H> {
    fn default() -> Self {
        Self::yeni()
    }
}

// ─── Testler ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::DepoKimligi;

    /// Listenin sonuna değer ekleyen basit test komutu (geri-al: son değeri çıkarır).
    struct Ekle(i32);
    impl Komut<Vec<i32>> for Ekle {
        fn uygula(&mut self, hedef: &mut Vec<i32>) -> Result<(), ErrorReport> {
            hedef.push(self.0);
            Ok(())
        }
        fn geri_al(&mut self, hedef: &mut Vec<i32>) -> Result<(), ErrorReport> {
            hedef.pop();
            Ok(())
        }
        fn aciklama(&self) -> String {
            format!("ekle {}", self.0)
        }
        fn depo(&self) -> DepoKimligi {
            DepoKimligi::yeni("test")
        }
    }

    #[test]
    fn cok_adimli_geri_al_yinele() {
        let mut y = GeriAlYigini::yeni();
        let mut h = Vec::new();
        y.calistir(&mut h, Box::new(Ekle(1))).unwrap();
        y.calistir(&mut h, Box::new(Ekle(2))).unwrap();
        y.calistir(&mut h, Box::new(Ekle(3))).unwrap();
        assert_eq!(h, vec![1, 2, 3]);

        // İki adım geri al.
        assert!(y.geri_al(&mut h).unwrap());
        assert!(y.geri_al(&mut h).unwrap());
        assert_eq!(h, vec![1], "iki geri-al sonrası");

        // Bir adım yinele.
        assert!(y.yinele(&mut h).unwrap());
        assert_eq!(h, vec![1, 2], "bir yinele sonrası");

        // Kalan bir adımı da yinele.
        assert!(y.yinele(&mut h).unwrap());
        assert_eq!(h, vec![1, 2, 3], "tam yinele → başlangıç sonucu");
    }

    #[test]
    fn yinele_dogru_sonuc_verir() {
        // "Redo bazen yanlış sonuç veriyor" riskine karşı: geri-al + yinele dizisi her zaman
        // ilk uygulama ile AYNI sonucu üretmeli.
        let mut y = GeriAlYigini::yeni();
        let mut h = Vec::new();
        y.calistir(&mut h, Box::new(Ekle(10))).unwrap();
        y.calistir(&mut h, Box::new(Ekle(20))).unwrap();
        let beklenen = h.clone();
        for _ in 0..5 {
            y.geri_al(&mut h).unwrap();
            y.geri_al(&mut h).unwrap();
            y.yinele(&mut h).unwrap();
            y.yinele(&mut h).unwrap();
        }
        assert_eq!(h, beklenen, "tekrarlı geri-al/yinele aynı sonucu vermeli");
    }

    #[test]
    fn yeni_is_yinele_yigininini_temizler() {
        let mut y = GeriAlYigini::yeni();
        let mut h = Vec::new();
        y.calistir(&mut h, Box::new(Ekle(1))).unwrap();
        y.calistir(&mut h, Box::new(Ekle(2))).unwrap();
        y.geri_al(&mut h).unwrap(); // h = [1]; ileri yığında Ekle(2) var.
        assert!(y.yinelenebilir_mi());
        // Geçmişten sapıp yeni iş yap → eski "yinele" dalı geçersiz olmalı.
        y.calistir(&mut h, Box::new(Ekle(9))).unwrap();
        assert!(!y.yinelenebilir_mi(), "yeni iş ileri yığını temizlemeli");
        assert_eq!(h, vec![1, 9]);
    }

    #[test]
    fn bos_yiginda_geri_al_yinele_false() {
        let mut y: GeriAlYigini<Vec<i32>> = GeriAlYigini::yeni();
        let mut h = Vec::new();
        assert!(!y.geri_alinabilir_mi());
        assert!(!y.yinelenebilir_mi());
        assert!(!y.geri_al(&mut h).unwrap(), "boş geçmişte geri-al false");
        assert!(!y.yinele(&mut h).unwrap(), "boş ileride yinele false");
    }

    #[test]
    fn derinlik_siniri_en_eskiyi_atar() {
        let mut y = GeriAlYigini::derinlikle(2);
        let mut h = Vec::new();
        y.calistir(&mut h, Box::new(Ekle(1))).unwrap();
        y.calistir(&mut h, Box::new(Ekle(2))).unwrap();
        y.calistir(&mut h, Box::new(Ekle(3))).unwrap();
        // Derinlik 2 → yalnızca son 2 komut geri alınabilir; en eski (Ekle(1)) düştü.
        assert_eq!(y.gecmis_uzunlugu(), 2);
        assert!(y.geri_al(&mut h).unwrap()); // 3 geri.
        assert!(y.geri_al(&mut h).unwrap()); // 2 geri.
        assert!(!y.geri_al(&mut h).unwrap(), "1 artık geçmişte değil");
        assert_eq!(h, vec![1], "Ekle(1)'in etkisi kalır (veri kaybı yok)");
    }

    #[test]
    fn aciklamalar_gorunur() {
        let mut y = GeriAlYigini::yeni();
        let mut h = Vec::new();
        y.calistir(&mut h, Box::new(Ekle(7))).unwrap();
        assert_eq!(y.sonraki_geri_al().as_deref(), Some("ekle 7"));
        assert_eq!(y.gecmis_aciklamalari(), vec!["ekle 7".to_string()]);
        y.geri_al(&mut h).unwrap();
        assert_eq!(y.sonraki_yinele().as_deref(), Some("ekle 7"));
    }
}
