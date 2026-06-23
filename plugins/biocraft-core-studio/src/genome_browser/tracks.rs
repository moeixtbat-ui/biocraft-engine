//! ÇE-02 — **İz (track) modeli** ve **dikey yerleşim**.
//!
//! Tarayıcı çok-izlidir: referans, anotasyon (gen/ekson), hizalama (read), kapsama (coverage),
//! varyant.  Kullanıcı izleri **yeniden sıralar / açıp-kapatır / yüksekliğini değiştirir**.
//! Yerleşim saf bir fonksiyondur (golden/birim test edilebilir): görünür izleri, cetvel
//! altından başlayarak dikey olarak istifler ve her birine bir `[y_ust, yukseklik]` dilimi verir.

/// İz türü — hangi veri/çizim modelinin kullanılacağını belirler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IzTuru {
    /// Referans dizi (A/C/G/T) — Gün 37'de zenginleşir (şimdilik yer tutucu).
    Referans,
    /// Anotasyon: gen/transkript/ekson kutuları (BED/GFF/GTF).
    Anotasyon,
    /// Hizalama: read yığını (BAM/SAM/CRAM).
    Hizalama,
    /// Kapsama (coverage) histogramı (hizalamadan türetilir).
    Kapsama,
    /// Varyant işareti (VCF) — ÇE-04'te zenginleşir.
    Varyant,
}

impl IzTuru {
    /// Bu tür için makul varsayılan yükseklik (piksel).
    pub fn varsayilan_yukseklik(self) -> f32 {
        match self {
            IzTuru::Referans => 22.0,
            IzTuru::Anotasyon => 40.0,
            IzTuru::Hizalama => 160.0,
            IzTuru::Kapsama => 60.0,
            IzTuru::Varyant => 30.0,
        }
    }

    /// Kısa Türkçe ad (UI/teşhis).
    pub fn ad(self) -> &'static str {
        match self {
            IzTuru::Referans => "Referans",
            IzTuru::Anotasyon => "Anotasyon",
            IzTuru::Hizalama => "Hizalama",
            IzTuru::Kapsama => "Kapsama",
            IzTuru::Varyant => "Varyant",
        }
    }
}

/// Tek bir iz (görünüm tarafı durumu — veri kaynağı ayrı tutulur).
#[derive(Debug, Clone, PartialEq)]
pub struct Iz {
    /// Benzersiz kimlik (oturum/proje kaydı için kararlı anahtar).
    pub kimlik: String,
    /// Kullanıcıya görünen ad.
    pub ad: String,
    /// İz türü.
    pub tur: IzTuru,
    /// Görünür mü? (aç-kapa)
    pub gorunur: bool,
    /// Yükseklik (piksel; en az [`Iz::ASGARI_YUKSEKLIK`]).
    pub yukseklik_px: f32,
}

impl Iz {
    /// Bir izin en küçük yüksekliği (piksel) — altına inilmez (okunabilirlik).
    pub const ASGARI_YUKSEKLIK: f32 = 16.0;

    /// Tür için varsayılan yükseklikle görünür bir iz kurar.
    pub fn yeni(kimlik: impl Into<String>, ad: impl Into<String>, tur: IzTuru) -> Self {
        Self {
            kimlik: kimlik.into(),
            ad: ad.into(),
            tur,
            gorunur: true,
            yukseklik_px: tur.varsayilan_yukseklik(),
        }
    }
}

/// Sıralı iz listesi + düzenleme işlemleri.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IzListesi {
    izler: Vec<Iz>,
}

impl IzListesi {
    /// Boş liste.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Sona iz ekler (aynı kimlik varsa **ezmez**, `false` döner — sessiz çakışma yok).
    pub fn ekle(&mut self, iz: Iz) -> bool {
        if self.izler.iter().any(|i| i.kimlik == iz.kimlik) {
            return false;
        }
        self.izler.push(iz);
        true
    }

    /// Tüm izler (sıralı).
    pub fn tumu(&self) -> &[Iz] {
        &self.izler
    }

    /// Yalnızca görünür izler (yerleşim/çizim bunu kullanır).
    pub fn gorunur_izler(&self) -> impl Iterator<Item = &Iz> {
        self.izler.iter().filter(|i| i.gorunur)
    }

    /// İz sayısı.
    pub fn sayi(&self) -> usize {
        self.izler.len()
    }

    /// Kimliğe göre iz (salt-okur).
    pub fn bul(&self, kimlik: &str) -> Option<&Iz> {
        self.izler.iter().find(|i| i.kimlik == kimlik)
    }

    /// Görünürlüğü değiştirir (aç-kapa).  Kimlik yoksa `false`.
    pub fn gorunurluk_degistir(&mut self, kimlik: &str) -> bool {
        if let Some(iz) = self.izler.iter_mut().find(|i| i.kimlik == kimlik) {
            iz.gorunur = !iz.gorunur;
            true
        } else {
            false
        }
    }

    /// Yüksekliği ayarlar (asgari sınıra sıkıştırılır).  Kimlik yoksa `false`.
    pub fn yukseklik_ayarla(&mut self, kimlik: &str, yukseklik_px: f32) -> bool {
        if let Some(iz) = self.izler.iter_mut().find(|i| i.kimlik == kimlik) {
            iz.yukseklik_px = yukseklik_px.max(Iz::ASGARI_YUKSEKLIK);
            true
        } else {
            false
        }
    }

    /// `kaynak` indeksindeki izi `hedef` indeksine taşır (yeniden sırala).  Geçersiz indekste
    /// `false`.
    pub fn tasi(&mut self, kaynak: usize, hedef: usize) -> bool {
        if kaynak >= self.izler.len() || hedef >= self.izler.len() {
            return false;
        }
        let iz = self.izler.remove(kaynak);
        self.izler.insert(hedef, iz);
        true
    }
}

/// Bir izin tuval üzerindeki dikey dilimi.
#[derive(Debug, Clone, PartialEq)]
pub struct IzYer {
    /// İz kimliği.
    pub kimlik: String,
    /// İz türü (çizim modeli seçimi için).
    pub tur: IzTuru,
    /// Üst kenarın y'si (piksel; cetvelin altından itibaren).
    pub y_ust: f32,
    /// İzin yüksekliği (piksel).
    pub yukseklik: f32,
}

impl IzYer {
    /// Alt kenarın y'si.
    pub fn y_alt(&self) -> f32 {
        self.y_ust + self.yukseklik
    }

    /// Bir ekran y'si bu izin dikey aralığında mı? (isabet testi.)
    pub fn icerir_y(&self, y: f32) -> bool {
        y >= self.y_ust && y < self.y_alt()
    }
}

/// Görünür izleri `cetvel_yuksekligi`'nin altından başlayarak `izler_arasi` boşlukla istifler.
/// Dönen dilimler çizim ve dikey isabet testi için kullanılır.
pub fn dikey_yerlesim(liste: &IzListesi, cetvel_yuksekligi: f32, izler_arasi: f32) -> Vec<IzYer> {
    let mut yerler = Vec::new();
    let mut y = cetvel_yuksekligi;
    for iz in liste.gorunur_izler() {
        yerler.push(IzYer {
            kimlik: iz.kimlik.clone(),
            tur: iz.tur,
            y_ust: y,
            yukseklik: iz.yukseklik_px,
        });
        y += iz.yukseklik_px + izler_arasi;
    }
    yerler
}

/// Tüm görünür izlerin + cetvelin kapladığı toplam içerik yüksekliği (dikey kaydırma çubuğu için).
pub fn toplam_yukseklik(liste: &IzListesi, cetvel_yuksekligi: f32, izler_arasi: f32) -> f32 {
    let gorunur: Vec<&Iz> = liste.gorunur_izler().collect();
    if gorunur.is_empty() {
        return cetvel_yuksekligi;
    }
    let izler: f32 = gorunur.iter().map(|i| i.yukseklik_px).sum();
    let bosluklar = izler_arasi * (gorunur.len() as f32 - 1.0);
    cetvel_yuksekligi + izler + bosluklar
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ornek() -> IzListesi {
        let mut l = IzListesi::yeni();
        assert!(l.ekle(Iz::yeni("kapsama", "Kapsama", IzTuru::Kapsama)));
        assert!(l.ekle(Iz::yeni("reads", "Okumalar", IzTuru::Hizalama)));
        assert!(l.ekle(Iz::yeni("genler", "Genler", IzTuru::Anotasyon)));
        l
    }

    #[test]
    fn ekleme_cakismayi_ezmez() {
        let mut l = ornek();
        assert_eq!(l.sayi(), 3);
        assert!(!l.ekle(Iz::yeni("reads", "Tekrar", IzTuru::Hizalama)));
        assert_eq!(l.sayi(), 3, "aynı kimlik ezilmemeli");
    }

    #[test]
    fn gorunurluk_ve_yukseklik() {
        let mut l = ornek();
        assert!(l.gorunurluk_degistir("reads"));
        assert!(!l.bul("reads").unwrap().gorunur);
        assert_eq!(l.gorunur_izler().count(), 2);

        // Asgari sınır altına inilmez.
        assert!(l.yukseklik_ayarla("genler", 5.0));
        assert_eq!(l.bul("genler").unwrap().yukseklik_px, Iz::ASGARI_YUKSEKLIK);

        assert!(!l.gorunurluk_degistir("yok"));
    }

    #[test]
    fn yeniden_siralama() {
        let mut l = ornek();
        // İlk (kapsama) en sona.
        assert!(l.tasi(0, 2));
        let sira: Vec<&str> = l.tumu().iter().map(|i| i.kimlik.as_str()).collect();
        assert_eq!(sira, ["reads", "genler", "kapsama"]);
        assert!(!l.tasi(0, 9)); // geçersiz hedef
    }

    #[test]
    fn dikey_yerlesim_istifler() {
        let l = ornek(); // kapsama(60) + reads(160) + genler(40)
        let cetvel_h = 24.0;
        let bosluk = 4.0;
        let yerler = dikey_yerlesim(&l, cetvel_h, bosluk);
        assert_eq!(yerler.len(), 3);
        assert_eq!(yerler[0].y_ust, 24.0);
        assert_eq!(yerler[0].y_alt(), 84.0); // 24 + 60
        assert_eq!(yerler[1].y_ust, 88.0); // 84 + 4
        assert_eq!(yerler[1].y_alt(), 248.0); // 88 + 160
        assert_eq!(yerler[2].y_ust, 252.0); // 248 + 4

        // İsabet testi: reads izinin içindeki bir y.
        assert!(yerler[1].icerir_y(100.0));
        assert!(!yerler[1].icerir_y(300.0));

        // Toplam yükseklik = 24 + (60+160+40) + 2*4 = 292.
        assert_eq!(toplam_yukseklik(&l, cetvel_h, bosluk), 292.0);
    }

    #[test]
    fn gizli_iz_yerlesimde_yok() {
        let mut l = ornek();
        l.gorunurluk_degistir("reads");
        let yerler = dikey_yerlesim(&l, 24.0, 4.0);
        assert_eq!(yerler.len(), 2);
        assert!(yerler.iter().all(|y| y.kimlik != "reads"));
    }
}
