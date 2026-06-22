//! YZ-00 — **Sağlayıcı soyutlaması (MK-46).**  Tüm AI altyapısının bel kemiği.
//!
//! Yerel / bulut / özel sağlayıcı **hepsi aynı sözleşmeyi** uygular: `uret` (generate),
//! `akis` (stream), `gom` (embed), `yetenekler` (capabilities), `maliyet` (cost).  Üst katman
//! (yüzey) hangi sağlayıcının takılı olduğunu bilmez; yeni sağlayıcı = yeni eklenti, çekirdek
//! değişmeden (0-AI.4).  **MVP'de bağlı gerçek motor YOKtur**; gerçek motorlar İP-07 host'unda
//! eklenti olarak gelir ve bu trait'i uygular.
// MK-46: sağlayıcı-bağımsız tek sözleşme.  MK-48: işlemler asenkron (UI donmaz).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use biocraft_types::ErrorReport;

use crate::context::AiBaglam;
use crate::contract::{AiCikti, Kullanim};
use crate::cost::Maliyet;

/// Sağlayıcı türü — **dış-AI sınırı** bu ayrıma göre uygulanır (YZ-08).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaglayiciTuru {
    /// Cihazda çalışan yerel model (veri cihazdan çıkmaz; PHI'de bile çalışabilir — 0-AI.5/1).
    Yerel,
    /// Bulut API (OpenAI/Anthropic vb.) — **dış kanal**, PHI gönderilemez.
    Bulut,
    /// Özel/self-hosted uç nokta — güvenli tarafta **dış kabul edilir** (fail-closed); yerel
    /// gizlilik isteniyorsa [`SaglayiciTuru::Yerel`] kullanılmalıdır.
    Ozel,
}

impl SaglayiciTuru {
    /// Bu tür **dış AI** mı? (PHI sınırı yalnızca dış sağlayıcılara uygulanır.)
    pub fn dis_mi(self) -> bool {
        matches!(self, SaglayiciTuru::Bulut | SaglayiciTuru::Ozel)
    }

    /// İki dilli kısa etiket.
    pub fn etiket(self, tr: bool) -> &'static str {
        match (self, tr) {
            (SaglayiciTuru::Yerel, true) => "Yerel",
            (SaglayiciTuru::Yerel, false) => "Local",
            (SaglayiciTuru::Bulut, true) => "Bulut",
            (SaglayiciTuru::Bulut, false) => "Cloud",
            (SaglayiciTuru::Ozel, true) => "Özel",
            (SaglayiciTuru::Ozel, false) => "Custom",
        }
    }
}

/// Bir sağlayıcının kimliği (yüzey bunu listeler/seçtirir).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaglayiciKimlik {
    /// Kararlı kimlik (ör. `biocraft.ai.local`, `biocraft.demo.echo`).
    pub kimlik: String,
    /// İnsan-okunur ad.
    pub ad: String,
    /// Sağlayıcı türü.
    pub tur: SaglayiciTuru,
    /// Aktif model adı (opsiyonel).
    pub model: Option<String>,
    /// Kısa açıklama (yüzeyde gösterilir).
    pub aciklama: String,
}

/// Sağlayıcının yetenekleri (capabilities).  Yüzey bunlara göre UI'yı uyarlar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SaglayiciYetenekleri {
    /// Akışlı (streaming) yanıt destekler mi?
    pub akis: bool,
    /// Gömme (embedding) üretebilir mi?
    pub gomme: bool,
    /// Görü (vision/multimodal) destekler mi?
    pub goru: bool,
    /// Maksimum bağlam jetonu (opsiyonel).
    pub maks_baglam_jeton: Option<u64>,
}

/// İptal bayrağı — "Durdur" butonu bunu işaretler; akış sağlayıcısı her parçada denetler (MK-11).
#[derive(Debug, Clone, Default)]
pub struct IptalBayragi(Arc<AtomicBool>);

impl IptalBayragi {
    /// Temiz (iptal edilmemiş) bayrak.
    pub fn yeni() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// İptali işaretler.
    pub fn iptal_et(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// İptal istendi mi?
    pub fn iptal_mi(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Akış (streaming) olayı — arka plan sağlayıcısından arayüze kare-başı kanalla gelir.
#[derive(Debug, Clone)]
pub enum AkisOlay {
    /// Bir metin parçası (token chunk) — kısmi yanıta eklenir.
    Parca(String),
    /// Akış tamamlandı; **tam zengin çıktı** (kaynak/güven/doğrulama/maliyet ile).
    Tamamlandi(Box<AiCikti>),
    /// Kullanıcı durdurdu (iptal).
    Durduruldu,
    /// Hata oluştu.
    Hata(Box<ErrorReport>),
}

/// **Sağlayıcı sözleşmesi (MK-46).**  Yerel/bulut/özel + 3. parti eklentiler bunu uygular.
///
/// `Send + Sync`: sağlayıcılar arka plan thread'inde çalışabilir (out-of-process motorlar için
/// köprü; arayüz 60 FPS kalır — MK-48).
pub trait Provider: Send + Sync {
    /// Sağlayıcının kimliği.
    fn kimlik(&self) -> &SaglayiciKimlik;

    /// Sağlayıcının yetenekleri.
    fn yetenekler(&self) -> SaglayiciYetenekleri;

    /// **generate** — tam yanıtı senkron üretir (bloklayan; akış istenmiyorsa).
    fn uret(&self, baglam: &AiBaglam) -> Result<AiCikti, ErrorReport>;

    /// **stream** — yanıtı parça parça üretir; her parçada `iptal` denetlenir, sonuç `gonder`
    /// ile yayınlanır.  Varsayılan uygulama [`Provider::uret`]'i çağırıp tek parçada yayınlar.
    fn akis(&self, baglam: &AiBaglam, iptal: &IptalBayragi, gonder: &mut dyn FnMut(AkisOlay)) {
        if iptal.iptal_mi() {
            gonder(AkisOlay::Durduruldu);
            return;
        }
        match self.uret(baglam) {
            Ok(cikti) => {
                gonder(AkisOlay::Parca(cikti.metin.clone()));
                gonder(AkisOlay::Tamamlandi(Box::new(cikti)));
            }
            Err(e) => gonder(AkisOlay::Hata(Box::new(e))),
        }
    }

    /// **embed** — metni gömme vektörüne çevirir.  Varsayılan: desteklenmiyor.
    fn gom(&self, _metin: &str) -> Result<Vec<f32>, ErrorReport> {
        Err(gomme_desteklenmiyor(self.kimlik()))
    }

    /// **cost** — bir kullanımın maliyetini hesaplar.  Varsayılan: yerel=0, dış=jeton(bedel yok).
    fn maliyet(&self, kullanim: &Kullanim) -> Maliyet {
        if matches!(self.kimlik().tur, SaglayiciTuru::Yerel) {
            Maliyet::yerel(kullanim.toplam())
        } else {
            Maliyet::yok(kullanim.toplam())
        }
    }
}

/// "Gömme desteklenmiyor" standart hatası (İP-16 şeması).
pub fn gomme_desteklenmiyor(kimlik: &SaglayiciKimlik) -> ErrorReport {
    ErrorReport::new(
        format!(
            "'{}' sağlayıcısı gömme (embedding) desteklemiyor.",
            kimlik.ad
        ),
        "Bu sağlayıcının yetenekleri arasında gömme yok.",
        "Gömme destekleyen bir sağlayıcı seçin (RAG için yerel embedding modeli — YZ-04).",
    )
    .with_teknik_detay(format!("kimlik={}", kimlik.kimlik))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dis_sinir_dogru() {
        assert!(!SaglayiciTuru::Yerel.dis_mi(), "yerel dış değil");
        assert!(SaglayiciTuru::Bulut.dis_mi(), "bulut dış");
        assert!(SaglayiciTuru::Ozel.dis_mi(), "özel fail-closed → dış");
    }

    #[test]
    fn iptal_bayragi_calisir() {
        let b = IptalBayragi::yeni();
        assert!(!b.iptal_mi());
        b.iptal_et();
        assert!(b.iptal_mi());
    }
}
