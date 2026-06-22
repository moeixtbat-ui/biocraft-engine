//! YZ-00 — AI **çıktı sözleşmesi** (MK-47): zengin, doğrulanabilir, kör-güvene kapalı.
//!
//! Her AI çıktısı yalnızca düz metin değildir; **kaynak/atıf + güven göstergesi + "doğrulanmalı"
//! uyarısı + token/maliyet** taşır.  Bu alanlar MVP'de iskelet (değerler boş/demo) olsa bile
//! **baştan** vardır — gelecekteki açıklanabilirlik motoru (eski "beş katmanlı güvenilirlik")
//! sancısız bağlanabilsin diye (0-AI.6).  Tipler `serde` ile serileştirilebilir (kalıcılık + WIT).
// MK-47: çıktı = öneri; kör güven YOK.  MK-49: klinik değil.

use serde::{Deserialize, Serialize};

/// Bir AI çağrısının **jeton (token) kullanımı**.  Yerel sağlayıcıda bedel=0, yalnızca kaynaktır.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kullanim {
    /// İstem (prompt) için harcanan jeton.
    pub girdi_jeton: u64,
    /// Yanıt için üretilen jeton.
    pub cikti_jeton: u64,
}

impl Kullanim {
    /// Girdi + çıktı jetonuyla kurar.
    pub fn yeni(girdi_jeton: u64, cikti_jeton: u64) -> Self {
        Self {
            girdi_jeton,
            cikti_jeton,
        }
    }

    /// Toplam jeton (girdi + çıktı).
    pub fn toplam(&self) -> u64 {
        self.girdi_jeton.saturating_add(self.cikti_jeton)
    }
}

/// Güven seviyesi — **kesin doğruluk değil**, yalnızca bir sinyal (0-AI.5/4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GuvenSeviyesi {
    /// Henüz bir motor yok / hesaplanmadı (MVP varsayılanı).
    #[default]
    Bilinmiyor,
    /// Düşük güven sinyali.
    Dusuk,
    /// Orta güven sinyali.
    Orta,
    /// Yüksek güven sinyali (yine de doğrulanmalı).
    Yuksek,
}

impl GuvenSeviyesi {
    /// İki dilli kısa etiket.
    pub fn etiket(self, tr: bool) -> &'static str {
        match (self, tr) {
            (GuvenSeviyesi::Bilinmiyor, true) => "Bilinmiyor",
            (GuvenSeviyesi::Bilinmiyor, false) => "Unknown",
            (GuvenSeviyesi::Dusuk, true) => "Düşük",
            (GuvenSeviyesi::Dusuk, false) => "Low",
            (GuvenSeviyesi::Orta, true) => "Orta",
            (GuvenSeviyesi::Orta, false) => "Medium",
            (GuvenSeviyesi::Yuksek, true) => "Yüksek",
            (GuvenSeviyesi::Yuksek, false) => "High",
        }
    }

    /// 0..=1 aralığında kaba bir oran (ilerleme çubuğu için).
    pub fn oran(self) -> f32 {
        match self {
            GuvenSeviyesi::Bilinmiyor => 0.0,
            GuvenSeviyesi::Dusuk => 0.33,
            GuvenSeviyesi::Orta => 0.66,
            GuvenSeviyesi::Yuksek => 1.0,
        }
    }
}

/// Güven göstergesi.  **Çok-AI uyumu garanti değildir** (AI'lar ortak önyargı paylaşıp
/// birlikte yanılabilir); birden çok sağlayıcı hemfikirse bu yalnızca "daha yüksek güven
/// sinyali"dir, "kesin doğru" değil (0-AI.5/4, MK-47).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Guven {
    /// Seviye sinyali.
    pub seviye: GuvenSeviyesi,
    /// İnsan-okunur kısa gerekçe (opsiyonel).
    pub aciklama: Option<String>,
    /// Kaç sağlayıcı çapraz kontrol etti (opsiyonel; çok-AI çerçevesi — İP-18 ile aynı çizgi).
    /// `None` = çapraz kontrol yapılmadı.
    pub coklu_ai_uyumu: Option<CokluAiUyum>,
}

/// Opsiyonel çok-AI çapraz kontrol özeti — **güven sinyali**, garanti değil.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CokluAiUyum {
    /// Çapraz kontrol eden sağlayıcı sayısı.
    pub saglayici_sayisi: u32,
    /// Hemfikir olan sağlayıcı sayısı.
    pub hemfikir: u32,
}

/// Kaynak / atıf — çıktının dayanağı (MVP'de boş olabilir).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kaynak {
    /// Kaynağın başlığı/adı.
    pub baslik: String,
    /// Opsiyonel URL.
    pub url: Option<String>,
    /// Opsiyonel atıf metni (yayın/lisans).
    pub atif: Option<String>,
}

impl Kaynak {
    /// Yalnızca başlıkla bir kaynak kurar.
    pub fn baslikla(baslik: impl Into<String>) -> Self {
        Self {
            baslik: baslik.into(),
            url: None,
            atif: None,
        }
    }
}

/// Önerilen eylemin türü (UI bunu ikon/onay diline çevirir).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EylemTuru {
    /// Koda parça ekle.
    KodEkle,
    /// Bir filtre/parametre uygula.
    FiltreUygula,
    /// Dosya yükle/oluştur.
    DosyaYukle,
    /// Bir araç/komut çalıştır.
    AracCalistir,
    /// Diğer.
    Diger,
}

/// Eylem önerisi — **her zaman kullanıcı onayına tabi** (MK-47/YZ-07); yıkıcı/geri-döndürülemez
/// eylem asla otomatik uygulanmaz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EylemOnerisi {
    /// Eylemin insan-okunur açıklaması.
    pub aciklama: String,
    /// Eylemin türü.
    pub tur: EylemTuru,
    /// Geri alınabilir mi? (İP-11 — geri alınamaz eylem ekstra dikkat ister.)
    pub geri_alinabilir: bool,
}

impl EylemOnerisi {
    /// Geri alınabilir bir eylem önerisi kurar.
    pub fn yeni(aciklama: impl Into<String>, tur: EylemTuru) -> Self {
        Self {
            aciklama: aciklama.into(),
            tur,
            geri_alinabilir: true,
        }
    }

    /// Onay her zaman gereklidir — sözleşme bunu sabitler (kör otomasyon yok).
    pub fn onay_gerekli(&self) -> bool {
        true
    }
}

/// **Zengin AI çıktısı (MK-47).**  Metin tek başına asla yeterli değildir; öneri/eylem/kaynak/
/// güven/doğrulama/maliyet birlikte gelir.  Klinik karar üretmez (MK-49).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiCikti {
    /// Ana yanıt metni (akışla dolar).
    pub metin: String,
    /// Kısa metinsel öneriler (madde madde).
    pub oneriler: Vec<String>,
    /// Onaya tabi eylem önerileri.
    pub eylem_onerileri: Vec<EylemOnerisi>,
    /// Kaynak / atıf listesi.
    pub kaynaklar: Vec<Kaynak>,
    /// Güven göstergesi.
    pub guven: Guven,
    /// **"Bu çıktı doğrulanmalı"** uyarısı — her çıktıda bulunur (kör güven yok).
    pub dogrulama_uyarisi: String,
    /// Bu sürüm klinik/tanısal karar üretmez (MK-49) — UI bunu etiketler.
    pub klinik_degil: bool,
    /// Jeton kullanımı.
    pub kullanim: Kullanim,
}

impl AiCikti {
    /// Standart "doğrulanmalı" uyarısı (iki dilli; UI dile göre seçer).
    pub const DOGRULAMA_TR: &'static str =
        "Bu bir AI önerisidir; bilimsel sonuç olarak kullanmadan önce doğrulayın. Klinik/tanısal karar değildir.";
    /// İngilizce karşılığı.
    pub const DOGRULAMA_EN: &'static str =
        "This is an AI suggestion; verify before using as a scientific result. Not a clinical/diagnostic decision.";

    /// Metinden, **doğrulama uyarısı + klinik-değil etiketi otomatik eklenmiş** bir öneri çıktısı
    /// kurar (kör güven yok — sözleşme bunu garanti eder).
    pub fn oneri(metin: impl Into<String>) -> Self {
        Self {
            metin: metin.into(),
            oneriler: Vec::new(),
            eylem_onerileri: Vec::new(),
            kaynaklar: Vec::new(),
            guven: Guven::default(),
            dogrulama_uyarisi: Self::DOGRULAMA_TR.to_string(),
            klinik_degil: true,
            kullanim: Kullanim::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oneri_dogrulama_uyarisi_hep_var() {
        // MK-47: her çıktı doğrulama uyarısı + klinik-değil etiketi taşır.
        let c = AiCikti::oneri("merhaba");
        assert!(
            !c.dogrulama_uyarisi.is_empty(),
            "doğrulama uyarısı boş olamaz"
        );
        assert!(c.klinik_degil, "çıktı klinik değil etiketli olmalı (MK-49)");
    }

    #[test]
    fn eylem_onerisi_hep_onay_ister() {
        let e = EylemOnerisi::yeni("Filtreyi uygula", EylemTuru::FiltreUygula);
        assert!(e.onay_gerekli(), "her eylem önerisi onaya tabidir (MK-47)");
    }

    #[test]
    fn kullanim_toplam_dogru() {
        assert_eq!(Kullanim::yeni(10, 25).toplam(), 35);
    }

    #[test]
    fn cikti_serde_gidis_donus() {
        let c = AiCikti::oneri("x");
        let j = serde_json::to_string(&c).unwrap();
        let g: AiCikti = serde_json::from_str(&j).unwrap();
        assert_eq!(c, g);
    }
}
