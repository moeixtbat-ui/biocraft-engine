//! YZ-00 — **Sağlayıcı kayıt/keşif.**
//!
//! Sağlayıcılar (yerel/bulut/3. parti) İP-07 host'u üzerinden kaydolur; yüzey hangi sağlayıcının
//! mevcut olduğunu listeler.  **Hiç sağlayıcı yoksa durum [`SaglayiciDurumu::Yapilandirilmadi`]**
//! → yüzey "AI yapılandırılmadı" gösterir (MK-48; sahte işlev yok).  MVP'de gerçek motor
//! kaydolmaz; kayıt yalnızca demo/echo sağlayıcı ya da ileride eklenti motorlarıyla dolar.
// MK-48: motor yoksa açıkça "yapılandırılmadı".

use std::sync::Arc;

use crate::provider::{Provider, SaglayiciKimlik};

/// Yüzeyin gösterdiği genel AI durumu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaglayiciDurumu {
    /// Hiç sağlayıcı kayıtlı değil (MVP varsayılanı) — "yapılandırılmadı".
    Yapilandirilmadi,
    /// En az bir sağlayıcı hazır; seçili olanın kimliği.
    Hazir(SaglayiciKimlik),
}

impl SaglayiciDurumu {
    /// Yapılandırıldı mı (bir sağlayıcı seçili mi)?
    pub fn hazir_mi(&self) -> bool {
        matches!(self, SaglayiciDurumu::Hazir(_))
    }
}

/// **Sağlayıcı kayıt defteri.**  Sağlayıcılar `Arc<dyn Provider>` tutulur → arka plan thread'ine
/// klonlanıp taşınabilir (akış sırasında arayüz donmaz — MK-48).
#[derive(Default)]
pub struct SaglayiciKayit {
    saglayicilar: Vec<Arc<dyn Provider>>,
    /// Seçili sağlayıcının indeksi.
    secili: Option<usize>,
}

impl SaglayiciKayit {
    /// Boş kayıt (yapılandırılmadı).
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir sağlayıcı kaydeder; ilk kayıt otomatik seçili olur.  Eklenen indeksi döndürür.
    pub fn kaydet(&mut self, saglayici: Arc<dyn Provider>) -> usize {
        self.saglayicilar.push(saglayici);
        let idx = self.saglayicilar.len() - 1;
        if self.secili.is_none() {
            self.secili = Some(idx);
        }
        idx
    }

    /// Tüm sağlayıcıları kaldırır (kayıtlar boşalır → tekrar "yapılandırılmadı").
    pub fn temizle(&mut self) {
        self.saglayicilar.clear();
        self.secili = None;
    }

    /// Kayıtlı sağlayıcı sayısı.
    pub fn say(&self) -> usize {
        self.saglayicilar.len()
    }

    /// Hiç sağlayıcı yok mu?
    pub fn bos_mu(&self) -> bool {
        self.saglayicilar.is_empty()
    }

    /// Tüm sağlayıcıların kimliklerini sırayla döndürür (seçici/liste için).
    pub fn kimlikler(&self) -> Vec<&SaglayiciKimlik> {
        self.saglayicilar.iter().map(|s| s.kimlik()).collect()
    }

    /// Seçili sağlayıcının indeksi.
    pub fn secili_indeks(&self) -> Option<usize> {
        self.secili
    }

    /// Bir sağlayıcıyı seçer; geçersiz indeks yok sayılır.
    pub fn sec(&mut self, idx: usize) {
        if idx < self.saglayicilar.len() {
            self.secili = Some(idx);
        }
    }

    /// Seçili sağlayıcıya (Arc) erişim — arka plan thread'ine klonlanabilir.
    pub fn secili(&self) -> Option<Arc<dyn Provider>> {
        self.secili.and_then(|i| self.saglayicilar.get(i).cloned())
    }

    /// İndeksle bir sağlayıcıya erişim.
    pub fn al(&self, idx: usize) -> Option<Arc<dyn Provider>> {
        self.saglayicilar.get(idx).cloned()
    }

    /// Yüzeyin göstereceği genel durum (yapılandırılmadı / hazır).
    pub fn durum(&self) -> SaglayiciDurumu {
        match self.secili() {
            Some(s) => SaglayiciDurumu::Hazir(s.kimlik().clone()),
            None => SaglayiciDurumu::Yapilandirilmadi,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::EchoSaglayici;

    #[test]
    fn bos_kayit_yapilandirilmadi() {
        let k = SaglayiciKayit::yeni();
        assert!(k.bos_mu());
        assert_eq!(k.durum(), SaglayiciDurumu::Yapilandirilmadi);
        assert!(!k.durum().hazir_mi());
    }

    #[test]
    fn ilk_kayit_otomatik_secili() {
        let mut k = SaglayiciKayit::yeni();
        k.kaydet(Arc::new(EchoSaglayici::yeni()));
        assert_eq!(k.say(), 1);
        assert!(k.durum().hazir_mi());
        assert_eq!(k.secili_indeks(), Some(0));
    }

    #[test]
    fn temizle_tekrar_yapilandirilmadi() {
        let mut k = SaglayiciKayit::yeni();
        k.kaydet(Arc::new(EchoSaglayici::yeni()));
        k.temizle();
        assert_eq!(k.durum(), SaglayiciDurumu::Yapilandirilmadi);
    }
}
