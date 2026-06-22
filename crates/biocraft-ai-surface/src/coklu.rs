//! YZ — **Opsiyonel çok-AI çapraz kontrol** (İP-18 kancası; MK-47).
//!
//! Kullanıcı bir çıktıyı **isteğe bağlı olarak** birden çok sağlayıcıda çapraz kontrol ettirebilir;
//! sistem uyum/uyuşmazlığı işaretler ("kaynaklar hemfikir / ayrışıyor").
//!
//! **EN ÖNEMLİ — dürüstlük (MK-47, 0-AI.5/4):** Bu bir **doğruluk garantisi DEĞİLDİR.** AI'lar ortak
//! eğitim önyargısı paylaşır ve birlikte **emin bir şekilde yanılabilir**; sonuç yalnızca "daha
//! yüksek **güven sinyali**" olarak sunulur, "kesin doğru" olarak değil.  Bilimsel sonuç yine
//! kullanıcı tarafından doğrulanmalıdır.  Bu yüzden [`CokluAiSonuc::garanti_degil`] **her zaman
//! `true`** döner ve [`CokluAiSonuc::uyari`] her sonuçta taşınır.
//!
//! **Kanca, motor değil:** Burada yalnızca koordinasyon mantığı (her sağlayıcıya sor → uyumu özetle)
//! vardır; gerçek çoklu-sağlayıcı **motoru** (paralel ağ çağrıları, oylama, gelişmiş benzerlik)
//! İP-07 host'unda bir **eklenti** olarak gelir ve bu kancayı kullanır.  Saf/senkron tutulur;
//! arayüz donmaması için üst katman bunu bir arka plan thread'inde çalıştırır (`Provider: Send+Sync`).

use std::sync::Arc;

use crate::context::AiBaglam;
use crate::contract::CokluAiUyum;
use crate::provider::{Provider, SaglayiciKimlik};

/// Bir sağlayıcının çapraz kontroldeki tek yanıtı.
#[derive(Debug, Clone)]
pub struct CokluAiYanit {
    /// Yanıtı veren sağlayıcının kimliği.
    pub kimlik: SaglayiciKimlik,
    /// Üretilen metin (başarısızsa boş).
    pub metin: String,
    /// Sağlayıcı yanıt üretebildi mi?
    pub basarili: bool,
    /// Başarısızsa kısa hata özeti (PII'siz).
    pub hata: Option<String>,
}

/// Çapraz kontrol sonucundaki genel uyum seviyesi — **yalnızca sinyal**, garanti değil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UyumSeviyesi {
    /// İkiden az sağlayıcı yanıt verdi → anlamlı çapraz kontrol yapılamaz.
    Yetersiz,
    /// Yanıtlar belirgin biçimde ayrışıyor (çoğunluk yok).
    Ayrisiyor,
    /// Yanıtların bir kısmı hemfikir (zayıf sinyal).
    KismenHemfikir,
    /// Yanıtların tamamı (yanıt verenler) hemfikir (güçlü sinyal — yine garanti değil).
    Hemfikir,
}

impl UyumSeviyesi {
    /// İki dilli kısa etiket.
    pub fn etiket(self, tr: bool) -> &'static str {
        match (self, tr) {
            (UyumSeviyesi::Yetersiz, true) => "Yetersiz (çapraz kontrol için en az 2 sağlayıcı)",
            (UyumSeviyesi::Yetersiz, false) => "Insufficient (need ≥2 providers)",
            (UyumSeviyesi::Ayrisiyor, true) => "Kaynaklar ayrışıyor",
            (UyumSeviyesi::Ayrisiyor, false) => "Sources disagree",
            (UyumSeviyesi::KismenHemfikir, true) => "Kısmen hemfikir",
            (UyumSeviyesi::KismenHemfikir, false) => "Partial agreement",
            (UyumSeviyesi::Hemfikir, true) => "Kaynaklar hemfikir",
            (UyumSeviyesi::Hemfikir, false) => "Sources agree",
        }
    }
}

/// Çok-AI çapraz kontrolün özeti.  **Güven sinyali, garanti değil** (MK-47).
#[derive(Debug, Clone)]
pub struct CokluAiSonuc {
    /// Her sağlayıcının yanıtı (şeffaflık — kullanıcı tek tek görebilir).
    pub yanitlar: Vec<CokluAiYanit>,
    /// Sayısal uyum özeti (kaç sağlayıcı / kaçı hemfikir).
    pub uyum: CokluAiUyum,
    /// Genel uyum seviyesi.
    pub seviye: UyumSeviyesi,
}

impl CokluAiSonuc {
    /// **Doğruluk garantisi değildir** — sözleşme bunu sabitler (her zaman `true`).
    ///
    /// AI'lar ortak önyargı paylaşıp birlikte yanılabilir; uyum yalnızca bir güven sinyalidir.
    pub fn garanti_degil(&self) -> bool {
        true
    }

    /// Her sonuçta gösterilmesi gereken dürüstlük uyarısı (iki dilli).
    pub fn uyari(&self, tr: bool) -> &'static str {
        if tr {
            "Bu bir doğruluk garantisi DEĞİLDİR. Birden çok AI hemfikir olsa bile ortak önyargıyla \
             birlikte yanılabilir; sonuç yalnızca bir güven sinyalidir, kesin doğru değil. Bilimsel \
             sonucu yine de kendiniz doğrulayın."
        } else {
            "This is NOT a correctness guarantee. Even if multiple AIs agree, they can be wrong \
             together due to shared bias; this is only a confidence signal, not certainty. Verify \
             the scientific result yourself."
        }
    }

    /// Başarıyla yanıt veren sağlayıcı sayısı.
    pub fn yanit_veren(&self) -> usize {
        self.yanitlar.iter().filter(|y| y.basarili).count()
    }
}

/// Çok-AI çapraz kontrol koordinatörü (kanca).
pub struct CokluAiKontrol;

impl CokluAiKontrol {
    /// Aynı bağlamı verilen tüm sağlayıcılara sorar ve uyumu özetler.
    ///
    /// Senkron + saf: her sağlayıcının [`Provider::uret`]'i sırayla çağrılır.  Üst katman bunu bir
    /// arka plan thread'inde çalıştırmalıdır (arayüz donmasın — sağlayıcılar `Send + Sync`).
    ///
    /// **Not:** Bu kanca dış kanal denetimi (PHI çıkış kapısı) **yapmaz**; çağırandan önce
    /// [`crate::guard::baglam_denetle`] ile her sağlayıcı türü için kapı uygulanmalıdır.
    pub fn calistir(saglayicilar: &[Arc<dyn Provider>], baglam: &AiBaglam) -> CokluAiSonuc {
        let mut yanitlar = Vec::with_capacity(saglayicilar.len());
        for s in saglayicilar {
            match s.uret(baglam) {
                Ok(c) => yanitlar.push(CokluAiYanit {
                    kimlik: s.kimlik().clone(),
                    metin: c.metin,
                    basarili: true,
                    hata: None,
                }),
                Err(e) => yanitlar.push(CokluAiYanit {
                    kimlik: s.kimlik().clone(),
                    metin: String::new(),
                    basarili: false,
                    hata: Some(e.ne_oldu),
                }),
            }
        }
        let (uyum, seviye) = uyumu_olc(&yanitlar);
        CokluAiSonuc {
            yanitlar,
            uyum,
            seviye,
        }
    }
}

/// Yanıtları normalleştirip en büyük "hemfikir" grubunu bulur → uyum özeti + seviye.
///
/// Heuristik (kanca): metinler küçük-harf + boşluk-normalleştirme sonrası **aynıysa** hemfikir
/// sayılır.  Gerçek anlamsal benzerlik (gömme/oylama) eklenti motorunun işidir; burada kasıtlı
/// olarak basit ve **abartısız** tutulur (yanlış "kesin" izlenimi vermemek için).
fn uyumu_olc(yanitlar: &[CokluAiYanit]) -> (CokluAiUyum, UyumSeviyesi) {
    let basarili: Vec<String> = yanitlar
        .iter()
        .filter(|y| y.basarili)
        .map(|y| normalle(&y.metin))
        .collect();
    let toplam = basarili.len() as u32;

    if toplam < 2 {
        return (
            CokluAiUyum {
                saglayici_sayisi: toplam,
                hemfikir: toplam.min(1),
            },
            UyumSeviyesi::Yetersiz,
        );
    }

    // En sık görülen normalleştirilmiş metnin frekansı = en büyük hemfikir grubu.
    let mut en_buyuk = 1u32;
    for (i, a) in basarili.iter().enumerate() {
        let mut sayi = 1u32;
        for (j, b) in basarili.iter().enumerate() {
            if i != j && a == b {
                sayi += 1;
            }
        }
        en_buyuk = en_buyuk.max(sayi);
    }

    let seviye = if en_buyuk == toplam {
        UyumSeviyesi::Hemfikir
    } else if en_buyuk * 2 > toplam {
        // Çoğunluk hemfikir (yarıdan fazlası aynı) → kısmen hemfikir.
        UyumSeviyesi::KismenHemfikir
    } else {
        UyumSeviyesi::Ayrisiyor
    };

    (
        CokluAiUyum {
            saglayici_sayisi: toplam,
            hemfikir: en_buyuk,
        },
        seviye,
    )
}

/// Metni karşılaştırma için normalleştirir (küçük-harf + boşlukları tekilleştir + kırp).
fn normalle(metin: &str) -> String {
    metin
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::AiCikti;
    use crate::provider::{SaglayiciTuru, SaglayiciYetenekleri};
    use biocraft_types::ErrorReport;

    /// Sabit bir metin döndüren test sağlayıcısı (veya hata).
    struct SabitSaglayici {
        kimlik: SaglayiciKimlik,
        cevap: Result<String, ()>,
    }

    impl SabitSaglayici {
        fn yeni(kimlik: &str, cevap: Result<&str, ()>) -> Arc<dyn Provider> {
            Arc::new(Self {
                kimlik: SaglayiciKimlik {
                    kimlik: kimlik.into(),
                    ad: kimlik.into(),
                    tur: SaglayiciTuru::Yerel,
                    model: None,
                    aciklama: String::new(),
                },
                cevap: cevap.map(|s| s.to_string()),
            })
        }
    }

    impl Provider for SabitSaglayici {
        fn kimlik(&self) -> &SaglayiciKimlik {
            &self.kimlik
        }
        fn yetenekler(&self) -> SaglayiciYetenekleri {
            SaglayiciYetenekleri::default()
        }
        fn uret(&self, _baglam: &AiBaglam) -> Result<AiCikti, ErrorReport> {
            match &self.cevap {
                Ok(m) => Ok(AiCikti::oneri(m.clone())),
                Err(()) => Err(ErrorReport::new("hata", "yok", "tekrar")),
            }
        }
    }

    fn baglam() -> AiBaglam {
        AiBaglam::sorgudan("varyant nedir?")
    }

    #[test]
    fn garanti_degil_her_zaman_dogru() {
        // MK-47: çok-AI uyumu asla kesin doğruluk garantisi değildir.
        let s = vec![
            SabitSaglayici::yeni("a", Ok("Aynı yanıt")),
            SabitSaglayici::yeni("b", Ok("Aynı yanıt")),
        ];
        let sonuc = CokluAiKontrol::calistir(&s, &baglam());
        assert!(sonuc.garanti_degil(), "her zaman true olmalı");
        assert!(!sonuc.uyari(true).is_empty());
        assert!(sonuc.uyari(true).contains("garanti"));
        assert!(sonuc.uyari(false).to_lowercase().contains("not"));
    }

    #[test]
    fn ayni_yanitlar_hemfikir() {
        // ASCII metin: boşluk + harf normalleştirmesi üçünü de aynı gruba toplar.
        // (Türkçe İ/ı casing'i `to_lowercase()` ile kasıtlı olarak farklı kalır — bkz. diğer test.)
        let s = vec![
            SabitSaglayici::yeni("a", Ok("It is a variant")),
            SabitSaglayici::yeni("b", Ok("it is  a variant")), // boşluk farkı normalleşir
            SabitSaglayici::yeni("c", Ok("IT IS A VARIANT")),  // büyük-harf normalleşir
        ];
        let sonuc = CokluAiKontrol::calistir(&s, &baglam());
        assert_eq!(sonuc.seviye, UyumSeviyesi::Hemfikir);
        assert_eq!(sonuc.uyum.saglayici_sayisi, 3);
        assert_eq!(sonuc.uyum.hemfikir, 3);
    }

    #[test]
    fn farkli_yanitlar_ayrisiyor() {
        let s = vec![
            SabitSaglayici::yeni("a", Ok("Cevap bir")),
            SabitSaglayici::yeni("b", Ok("Cevap iki")),
            SabitSaglayici::yeni("c", Ok("Cevap üç")),
            SabitSaglayici::yeni("d", Ok("Cevap dört")),
        ];
        let sonuc = CokluAiKontrol::calistir(&s, &baglam());
        assert_eq!(sonuc.seviye, UyumSeviyesi::Ayrisiyor);
        assert_eq!(sonuc.uyum.hemfikir, 1);
    }

    #[test]
    fn cogunluk_kismen_hemfikir() {
        let s = vec![
            SabitSaglayici::yeni("a", Ok("Aynı")),
            SabitSaglayici::yeni("b", Ok("Aynı")),
            SabitSaglayici::yeni("c", Ok("Farklı")),
        ];
        let sonuc = CokluAiKontrol::calistir(&s, &baglam());
        assert_eq!(sonuc.seviye, UyumSeviyesi::KismenHemfikir);
        assert_eq!(sonuc.uyum.hemfikir, 2);
    }

    #[test]
    fn tek_saglayici_yetersiz() {
        let s = vec![SabitSaglayici::yeni("a", Ok("Tek"))];
        let sonuc = CokluAiKontrol::calistir(&s, &baglam());
        assert_eq!(sonuc.seviye, UyumSeviyesi::Yetersiz);
    }

    #[test]
    fn hata_veren_saglayici_yanit_verene_dahil_degil() {
        let s = vec![
            SabitSaglayici::yeni("a", Ok("Aynı")),
            SabitSaglayici::yeni("b", Ok("Aynı")),
            SabitSaglayici::yeni("c", Err(())), // başarısız → hemfikir sayımına girmez
        ];
        let sonuc = CokluAiKontrol::calistir(&s, &baglam());
        assert_eq!(sonuc.yanit_veren(), 2);
        assert_eq!(
            sonuc.uyum.saglayici_sayisi, 2,
            "yalnız yanıt verenler sayılır"
        );
        assert_eq!(sonuc.seviye, UyumSeviyesi::Hemfikir);
        // Başarısız yanıt şeffaflık için listede kalır (hata özetiyle).
        assert!(sonuc
            .yanitlar
            .iter()
            .any(|y| !y.basarili && y.hata.is_some()));
    }
}
