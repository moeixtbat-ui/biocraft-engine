//! ÇE-12 — **Klavye gezinme + ekran okuyucu** modeli (erişilebilirlik; Bölüm 0 / MK-52).
//!
//! Eklenti egui/winit'e MK-17 gereği dokunamaz; bu yüzden klavye odağı ve ekran okuyucu
//! anlatımı **render-bağımsız** bir model olarak tutulur (motor bunu gerçek odak halkasına +
//! platform erişilebilirlik API'sine — Windows UIA / AT-SPI — bağlar).  Buradaki kanıt:
//! 1. **Her etkileşimli öğeye klavyeyle ulaşılır** ([`OdakHalkasi`] tab sırası; sarmalı, devre-dışı
//!    atlanır) → "klavye ile her yere ulaşılamıyor" hatası yapısal olarak önlenir.
//! 2. **Her etkileşimli öğenin ekran okuyucu etiketi vardır** ([`OdakOgesi::etiket`] boş olamaz) →
//!    [`erisilebilirlik_denetimi`] eksik etiketi yakalar.
//! 3. **Modal/diyalog odak tuzağı** ([`OdakHalkasi::tuzak`]) → odak diyalog dışına kaçmaz.

use crate::genome_browser::cizim::CizimRengi;
use crate::perf::accessibility::{serit_sekli, varyant_sekli, SekilIpucu};

/// Ekran okuyucuya bildirilen **rol** (öğe tipi).  Platform API'sinin rolüne (UIA ControlType /
/// ARIA role) eşlenir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rol {
    /// Tıklanabilir buton.
    Buton,
    /// Geçiş düğmesi (açık/kapalı).
    Anahtar,
    /// Liste kabı.
    Liste,
    /// Liste öğesi.
    ListeOgesi,
    /// Çizim tuvali (genom/3B) — ok tuşlarıyla gezinilir.
    Tuval,
    /// Kaydırıcı/değer (zoom vb.).
    Kaydirici,
    /// Sekme.
    Sekme,
    /// Metin giriş alanı.
    GirisAlani,
    /// Salt-okunur etiket/metin (etkileşimli değil — tab sırasına girmez).
    Etiket,
}

impl Rol {
    /// Bu rol etkileşimli mi? (Etkileşimli olanlar tab sırasına girer + etiket zorunludur.)
    pub fn etkilesimli(&self) -> bool {
        !matches!(self, Rol::Etiket)
    }

    /// Ekran okuyucu anlatımında kullanılan rol adı.
    pub fn ad(&self) -> &'static str {
        match self {
            Rol::Buton => "buton",
            Rol::Anahtar => "anahtar",
            Rol::Liste => "liste",
            Rol::ListeOgesi => "liste öğesi",
            Rol::Tuval => "tuval",
            Rol::Kaydirici => "kaydırıcı",
            Rol::Sekme => "sekme",
            Rol::GirisAlani => "giriş alanı",
            Rol::Etiket => "etiket",
        }
    }
}

/// Bir odaklanabilir/anlatılabilir arayüz öğesi (render-bağımsız tanım).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OdakOgesi {
    /// Benzersiz kimlik (komut/eleman kimliğiyle eşleşir).
    pub kimlik: String,
    /// Ekran okuyucu etiketi (boş olamaz — [`erisilebilirlik_denetimi`] zorlar).
    pub etiket: String,
    /// Öğe rolü.
    pub rol: Rol,
    /// Klavye kısayolu (varsa, örn. "Ctrl+F").
    pub kisayol: Option<String>,
    /// Geçici olarak devre dışı (tab sırasında atlanır ama gösterilir).
    pub devre_disi: bool,
}

impl OdakOgesi {
    /// Yeni etkileşimli öğe.
    pub fn yeni(kimlik: impl Into<String>, etiket: impl Into<String>, rol: Rol) -> Self {
        Self {
            kimlik: kimlik.into(),
            etiket: etiket.into(),
            rol,
            kisayol: None,
            devre_disi: false,
        }
    }

    /// Kısayol ekler (akıcı API).
    pub fn kisayol(mut self, k: impl Into<String>) -> Self {
        self.kisayol = Some(k.into());
        self
    }

    /// Devre dışı işaretler.
    pub fn devre_disi(mut self) -> Self {
        self.devre_disi = true;
        self
    }

    /// **Ekran okuyucu anlatımı** — "Etiket, rol[, kısayol Ctrl+F][, devre dışı]".
    /// Motor bunu platform erişilebilirlik API'sine (UIA/AT-SPI) Name olarak verir.
    pub fn ekran_okuyucu_metni(&self) -> String {
        let mut s = format!("{}, {}", self.etiket, self.rol.ad());
        if let Some(k) = &self.kisayol {
            s.push_str(&format!(", kısayol {k}"));
        }
        if self.devre_disi {
            s.push_str(", devre dışı");
        }
        s
    }
}

/// Sıralı **odak halkası** (tab sırası).  Tab/Shift+Tab ile sarmalı gezinir; devre-dışı öğeleri
/// atlar; modal için odak tuzağı kurulabilir.
#[derive(Debug, Clone, Default)]
pub struct OdakHalkasi {
    ogeler: Vec<OdakOgesi>,
    odak: Option<usize>,
    tuzak: bool,
}

impl OdakHalkasi {
    /// Boş halka.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Öğelerden halka kurar; ilk **etkin etkileşimli** öğeye odaklanır.
    pub fn ogelerden(ogeler: Vec<OdakOgesi>) -> Self {
        let mut h = Self {
            ogeler,
            odak: None,
            tuzak: false,
        };
        h.odak = h.ilk_odaklanabilir();
        h
    }

    /// Öğe ekler (sona).  İlk eklenen etkin öğe otomatik odaklanır.
    pub fn ekle(&mut self, oge: OdakOgesi) -> &mut Self {
        self.ogeler.push(oge);
        if self.odak.is_none() {
            self.odak = self.ilk_odaklanabilir();
        }
        self
    }

    /// Odak tuzağını (modal) açar/kapatır.  (Model düzeyinde tuzak her zaman halka içi sarmaladığı
    /// için bayrak belgeleyicidir; gerçek fark, motorun bu halkayı modal kapsamı yapmasıdır.)
    pub fn tuzak_ayarla(&mut self, acik: bool) -> &mut Self {
        self.tuzak = acik;
        self
    }

    /// Tuzak açık mı?
    pub fn tuzakli(&self) -> bool {
        self.tuzak
    }

    /// Şu an odaklı öğe.
    pub fn odakli(&self) -> Option<&OdakOgesi> {
        self.odak.and_then(|i| self.ogeler.get(i))
    }

    /// Tüm öğeler (denetim/anlatım için).
    pub fn ogeler(&self) -> &[OdakOgesi] {
        &self.ogeler
    }

    /// Bir öğe, klavye odağı **alabilir** mi? (etkileşimli + etkin)
    fn odaklanabilir(oge: &OdakOgesi) -> bool {
        oge.rol.etkilesimli() && !oge.devre_disi
    }

    fn ilk_odaklanabilir(&self) -> Option<usize> {
        self.ogeler.iter().position(Self::odaklanabilir)
    }

    /// **Tab** — bir sonraki odaklanabilir öğeye sarmalı geçer; döndürür.
    pub fn sonraki(&mut self) -> Option<&OdakOgesi> {
        self.gez(true)
    }

    /// **Shift+Tab** — bir önceki odaklanabilir öğeye sarmalı geçer; döndürür.
    pub fn onceki(&mut self) -> Option<&OdakOgesi> {
        self.gez(false)
    }

    /// Odağı belirli kimliğe taşır (örn. fare tıklaması/komut palet sonrası senkron).
    pub fn odakla(&mut self, kimlik: &str) -> bool {
        if let Some(i) = self
            .ogeler
            .iter()
            .position(|o| o.kimlik == kimlik && Self::odaklanabilir(o))
        {
            self.odak = Some(i);
            true
        } else {
            false
        }
    }

    /// Ortak gezinme: ileri/geri yönde, devre-dışı/etkileşimsizi atlayarak, en çok bir tur döner.
    fn gez(&mut self, ileri: bool) -> Option<&OdakOgesi> {
        let n = self.ogeler.len();
        if n == 0 {
            return None;
        }
        let bas = self.odak.unwrap_or(0);
        for adim in 1..=n {
            let i = if ileri {
                (bas + adim) % n
            } else {
                (bas + n - (adim % n)) % n
            };
            if Self::odaklanabilir(&self.ogeler[i]) {
                self.odak = Some(i);
                return self.ogeler.get(i);
            }
        }
        None // hiç odaklanabilir öğe yok
    }
}

/// Bir öğe kümesinin erişilebilirlik denetimi: **her etkileşimli öğenin boş olmayan etiketi**
/// olmalı + en az bir öğe klavyeyle odaklanabilir olmalı.  Eksikleri (kimlik listesi) döndürür.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErisilebilirlikDenetimi {
    /// Etiketi boş olan etkileşimli öğelerin kimlikleri (ekran okuyucu "isimsiz öğe" der → hata).
    pub etiketsiz: Vec<String>,
    /// Hiç klavye-odaklanabilir öğe yok mu? (tuzak/çıkmaz panel riski)
    pub klavye_erisimsiz: bool,
}

impl ErisilebilirlikDenetimi {
    /// Denetim tamamen temiz mi?
    pub fn temiz(&self) -> bool {
        self.etiketsiz.is_empty() && !self.klavye_erisimsiz
    }
}

/// Öğe listesini denetler (yukarıdaki iki kural).
pub fn erisilebilirlik_denetimi(ogeler: &[OdakOgesi]) -> ErisilebilirlikDenetimi {
    let etiketsiz: Vec<String> = ogeler
        .iter()
        .filter(|o| o.rol.etkilesimli() && o.etiket.trim().is_empty())
        .map(|o| o.kimlik.clone())
        .collect();
    let klavye_erisimsiz = !ogeler.iter().any(|o| o.rol.etkilesimli() && !o.devre_disi);
    ErisilebilirlikDenetimi {
        etiketsiz,
        klavye_erisimsiz,
    }
}

/// Bir anlamsal çizim rengi için **ekran okuyucu/şekil** kısaltması (renk + şekil ipucu birlikte
/// anlatılır → renk körü kullanıcı şekli "duyar/görür").  Şekli olmayan renkler `None`.
pub fn renk_sekil_ipucu(renk: CizimRengi) -> Option<SekilIpucu> {
    varyant_sekli(renk).or_else(|| serit_sekli(renk))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ornek_halka() -> OdakHalkasi {
        OdakHalkasi::ogelerden(vec![
            OdakOgesi::yeni("ara", "Bölgeye git", Rol::GirisAlani).kisayol("Ctrl+G"),
            OdakOgesi::yeni("geri", "Geri", Rol::Buton).devre_disi(), // başlangıçta geçmiş yok
            OdakOgesi::yeni("ileri", "İleri", Rol::Buton),
            OdakOgesi::yeni("baslik", "Hizalama izi", Rol::Etiket), // etkileşimsiz
            OdakOgesi::yeni("tuval", "Genom tuvali", Rol::Tuval).kisayol("Ok tuşları"),
        ])
    }

    #[test]
    fn tab_sarmali_gezer_ve_devre_disi_atlar() {
        let mut h = ornek_halka();
        // İlk odak: ilk etkin etkileşimli (ara).
        assert_eq!(h.odakli().unwrap().kimlik, "ara");
        // Tab: geri (devre dışı) ve baslik (etiket) atlanır → ileri.
        assert_eq!(h.sonraki().unwrap().kimlik, "ileri");
        // Tab: tuval.
        assert_eq!(h.sonraki().unwrap().kimlik, "tuval");
        // Tab: sarmalı başa (ara).
        assert_eq!(h.sonraki().unwrap().kimlik, "ara");
    }

    #[test]
    fn shift_tab_geri_sarmali() {
        let mut h = ornek_halka();
        assert_eq!(h.odakli().unwrap().kimlik, "ara");
        // Shift+Tab: geriye sarmalı → tuval (son odaklanabilir).
        assert_eq!(h.onceki().unwrap().kimlik, "tuval");
        assert_eq!(h.onceki().unwrap().kimlik, "ileri");
    }

    #[test]
    fn etkilesimsiz_ve_devre_disi_asla_odaklanmaz() {
        let mut h = ornek_halka();
        // 10 tab boyunca etiket/devre-dışı öğeye hiç odaklanılmamalı.
        for _ in 0..10 {
            let o = h.sonraki().unwrap();
            assert!(
                o.rol.etkilesimli() && !o.devre_disi,
                "odak yanlış öğede: {o:?}"
            );
        }
    }

    #[test]
    fn odakla_kimlikle_tasir() {
        let mut h = ornek_halka();
        assert!(h.odakla("tuval"));
        assert_eq!(h.odakli().unwrap().kimlik, "tuval");
        // Devre-dışı öğeye odaklanılamaz.
        assert!(!h.odakla("geri"));
        // Olmayan kimlik.
        assert!(!h.odakla("yok"));
    }

    #[test]
    fn ekran_okuyucu_metni_etiket_rol_kisayol() {
        let o = OdakOgesi::yeni("ara", "Bölgeye git", Rol::GirisAlani).kisayol("Ctrl+G");
        assert_eq!(
            o.ekran_okuyucu_metni(),
            "Bölgeye git, giriş alanı, kısayol Ctrl+G"
        );
        let d = OdakOgesi::yeni("geri", "Geri", Rol::Buton).devre_disi();
        assert_eq!(d.ekran_okuyucu_metni(), "Geri, buton, devre dışı");
    }

    #[test]
    fn denetim_etiketsiz_etkilesimli_yakalar() {
        let ogeler = vec![
            OdakOgesi::yeni("a", "Tamam", Rol::Buton),
            OdakOgesi::yeni("b", "   ", Rol::Buton), // boş etiket → hata
            OdakOgesi::yeni("c", "", Rol::Etiket),   // etiket rolü: etkileşimsiz, sorun değil
        ];
        let d = erisilebilirlik_denetimi(&ogeler);
        assert!(!d.temiz());
        assert_eq!(d.etiketsiz, vec!["b".to_string()]);
        assert!(!d.klavye_erisimsiz);
    }

    #[test]
    fn denetim_temiz_gercek_panel() {
        // Gerçekçi panel: hepsi etiketli + en az biri odaklanabilir → temiz.
        let h = ornek_halka();
        let d = erisilebilirlik_denetimi(h.ogeler());
        assert!(d.temiz(), "panel erişilebilir olmalı: {d:?}");
    }

    #[test]
    fn klavye_erisimsiz_panel_yakalanir() {
        // Yalnız etiketlerden oluşan panel → klavyeyle ulaşılacak hiçbir şey yok (hata).
        let ogeler = vec![OdakOgesi::yeni("x", "Bilgi", Rol::Etiket)];
        let d = erisilebilirlik_denetimi(&ogeler);
        assert!(d.klavye_erisimsiz);
        assert!(!d.temiz());
        // Boş halkada gezinme panik yapmaz, None döner.
        let mut h = OdakHalkasi::ogelerden(ogeler);
        assert!(h.sonraki().is_none());
        assert!(h.odakli().is_none());
    }

    #[test]
    fn tuzak_bayragi_modal() {
        let mut h = ornek_halka();
        assert!(!h.tuzakli());
        h.tuzak_ayarla(true);
        assert!(h.tuzakli());
    }
}
