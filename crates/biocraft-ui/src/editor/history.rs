//! Editör **yerel geçmişi** — kod düzenlemelerine bağlı anlık görüntüler (İP-06 / İP-11).
//!
//! egui `TextEdit` zaten karakter-düzeyi **geri al/yinele** (Ctrl+Z/Y) verir.  Bu modül onun
//! **üstüne**, kaba taneli bir **yerel geçmiş** koyar: önemli anlarda (çalıştırma, kaydet,
//! elle anlık görüntü) tüm belgenin bir kopyası saklanır → kullanıcı saatler önceki bir
//! sürüme **geri dönebilir** (TDA madde 2/10 — değişiklikler güvenle geri alınır).
//!
//! Saf model (egui'siz) → birim-testlenebilir.  Bellek koruması: en fazla [`YerelGecmis::azami`]
//! kayıt; en eskisi atılır.  Ardışık **aynı** metin tekrar saklanmaz (gürültü yok).
// MK-09 uyumlu: yalnız sınırlı sayıda anlık görüntü RAM'de; büyük geçmiş diske (v1.x).

/// Tek bir geçmiş anlık görüntüsü.
#[derive(Debug, Clone, PartialEq)]
pub struct GecmisKaydi {
    /// Kaydın kısa etiketi (ör. "çalıştırma", "kaydet", "elle").
    pub etiket: String,
    /// O andaki tam belge metni.
    pub metin: String,
    /// Oluşturma sırası (artan; en yeni en büyük) — UI sıralaması/teşhis.
    pub sira: u64,
}

/// Bir belgenin yerel geçmişi (kaba taneli anlık görüntüler).
#[derive(Debug, Clone)]
pub struct YerelGecmis {
    kayitlar: Vec<GecmisKaydi>,
    azami: usize,
    sayac: u64,
}

impl Default for YerelGecmis {
    fn default() -> Self {
        Self::yeni()
    }
}

impl YerelGecmis {
    /// Varsayılan kapasiteyle (50 kayıt) boş geçmiş.
    pub fn yeni() -> Self {
        Self::kapasiteli(50)
    }

    /// Belirli kapasiteyle boş geçmiş.
    pub fn kapasiteli(azami: usize) -> Self {
        Self {
            kayitlar: Vec::new(),
            azami: azami.max(1),
            sayac: 0,
        }
    }

    /// Bir anlık görüntü saklar.  Son kayıtla **aynı metin** ise saklanmaz (`false` döner).
    pub fn anlik_al(&mut self, etiket: impl Into<String>, metin: &str) -> bool {
        if self.kayitlar.last().map(|k| k.metin.as_str()) == Some(metin) {
            return false;
        }
        self.sayac += 1;
        self.kayitlar.push(GecmisKaydi {
            etiket: etiket.into(),
            metin: metin.to_string(),
            sira: self.sayac,
        });
        // Kapasite aşımı: en eskiyi at.
        if self.kayitlar.len() > self.azami {
            let fazla = self.kayitlar.len() - self.azami;
            self.kayitlar.drain(0..fazla);
        }
        true
    }

    /// Tüm kayıtlar (en eskiden en yeniye, salt-okunur).
    pub fn kayitlar(&self) -> &[GecmisKaydi] {
        &self.kayitlar
    }

    /// Kayıt sayısı.
    pub fn len(&self) -> usize {
        self.kayitlar.len()
    }

    /// Boş mu?
    pub fn is_empty(&self) -> bool {
        self.kayitlar.is_empty()
    }

    /// `idx`. kaydın metni (en eskiden en yeniye sıra; yoksa `None`).
    pub fn metin(&self, idx: usize) -> Option<&str> {
        self.kayitlar.get(idx).map(|k| k.metin.as_str())
    }

    /// Tümünü temizler.
    pub fn temizle(&mut self) {
        self.kayitlar.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anlik_eklenir_ve_okunur() {
        let mut g = YerelGecmis::yeni();
        assert!(g.anlik_al("ilk", "a = 1"));
        assert!(g.anlik_al("ikinci", "a = 2"));
        assert_eq!(g.len(), 2);
        assert_eq!(g.metin(0), Some("a = 1"));
        assert_eq!(g.metin(1), Some("a = 2"));
    }

    #[test]
    fn ardisik_ayni_metin_saklanmaz() {
        let mut g = YerelGecmis::yeni();
        assert!(g.anlik_al("x", "ayni"));
        assert!(!g.anlik_al("y", "ayni")); // değişmedi → saklanmaz
        assert_eq!(g.len(), 1);
    }

    #[test]
    fn kapasite_asiminda_en_eski_atilir() {
        let mut g = YerelGecmis::kapasiteli(3);
        for i in 0..5 {
            g.anlik_al("k", &format!("s{i}"));
        }
        assert_eq!(g.len(), 3);
        // En eski (s0, s1) atıldı; en yenisi s4.
        assert_eq!(g.metin(0), Some("s2"));
        assert_eq!(g.kayitlar().last().unwrap().metin, "s4");
    }

    #[test]
    fn sira_artar() {
        let mut g = YerelGecmis::yeni();
        g.anlik_al("a", "1");
        g.anlik_al("b", "2");
        assert!(g.kayitlar()[1].sira > g.kayitlar()[0].sira);
    }
}
