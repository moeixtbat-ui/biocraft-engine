//! Determinizm bayrağı (kanca) — MK-29.
//!
//! Bir proje/iş **"tekrarüretilebilir (bilimsel)"** olarak işaretliyse, hesap yolu
//! deterministik moda **hazırlanır**: sabit RNG tohumu, sıralı (deterministik) paralel
//! indirgeme, gerektiğinde worker sayısının sabitlenmesi.  **"Hızlı keşif"** modunda hız
//! önceliklidir; bit-bit tekrar garanti edilmez.
//!
//! ⚠️ **Bu yalnızca bir kancadır.**  Gerçek **bit-bit** tekrarüretilebilirlik garantisi v1.x
//! kapsamındadır (`docs/specs/MVP-sonrasi.md` §9.1).  Burada hesap katmanlarının okuyup
//! uyacağı **niyet + parametreler** taşınır; motor bu bayrağı henüz zorlamaz.

/// Determinizm modu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeterminizmModu {
    /// Hızlı keşif (varsayılan): hız önce; bit-bit tekrar garanti edilmez.
    #[default]
    HizliKesif,
    /// Tekrarüretilebilir (bilimsel): deterministik hesap yoluna hazırlan.
    TekrarUretilebilir,
}

impl DeterminizmModu {
    /// Durum panelinde gösterilecek kısa ad.
    pub fn ad(&self) -> &'static str {
        match self {
            DeterminizmModu::HizliKesif => "Hızlı keşif",
            DeterminizmModu::TekrarUretilebilir => "Tekrarüretilebilir",
        }
    }
}

/// Determinizm bayrağı — mod + (tekrarüretilebilir modda) sabit RNG tohumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeterminizmBayragi {
    /// Seçili mod.
    pub modu: DeterminizmModu,
    /// Tekrarüretilebilir modda kullanılacak sabit RNG tohumu.
    pub tohum: u64,
}

impl Default for DeterminizmBayragi {
    fn default() -> Self {
        Self::hizli()
    }
}

impl DeterminizmBayragi {
    /// Hızlı keşif modu (deterministik değil).
    pub fn hizli() -> Self {
        Self {
            modu: DeterminizmModu::HizliKesif,
            tohum: 0,
        }
    }

    /// Tekrarüretilebilir mod, verilen sabit tohumla.
    pub fn tekrar_uretilebilir(tohum: u64) -> Self {
        Self {
            modu: DeterminizmModu::TekrarUretilebilir,
            tohum,
        }
    }

    /// Deterministik mod etkin mi?
    pub fn deterministik_mi(&self) -> bool {
        matches!(self.modu, DeterminizmModu::TekrarUretilebilir)
    }

    /// **Kanca:** paralel indirgemeler sıralı (deterministik) birleştirilmeli mi?
    /// Deterministik modda `true` — kayan-nokta toplama sırası sonuç değiştirebilir.
    pub fn sirali_indirgeme(&self) -> bool {
        self.deterministik_mi()
    }

    /// **Kanca:** hesap için sabit RNG tohumu (`None` = serbest/zaman-tabanlı).
    pub fn sabit_tohum(&self) -> Option<u64> {
        if self.deterministik_mi() {
            Some(self.tohum)
        } else {
            None
        }
    }

    /// **Kanca:** worker sayısını deterministik moda göre düzeltir.  Tekrarüretilebilir modda
    /// (MVP kancası) kayan-nokta birleştirme sırası worker sayısına bağlı olabileceğinden
    /// hesap **tek worker'a** sabitlenir; gerçek "sıralı paralel indirgeme" v1.x'te gelir.
    pub fn worker_kisidi(&self, onerilen: usize) -> usize {
        if self.deterministik_mi() {
            1
        } else {
            onerilen.max(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varsayilan_hizli_kesif_deterministik_degil() {
        let b = DeterminizmBayragi::default();
        assert_eq!(b.modu, DeterminizmModu::HizliKesif);
        assert!(!b.deterministik_mi());
        assert!(b.sabit_tohum().is_none());
        assert!(!b.sirali_indirgeme());
        assert_eq!(b.worker_kisidi(8), 8, "Hızlı keşifte worker kısılmaz");
    }

    #[test]
    fn tekrar_uretilebilir_kancalari_acar() {
        let b = DeterminizmBayragi::tekrar_uretilebilir(42);
        assert!(b.deterministik_mi());
        assert_eq!(b.sabit_tohum(), Some(42));
        assert!(b.sirali_indirgeme());
        assert_eq!(
            b.worker_kisidi(8),
            1,
            "Deterministik modda hesap tek worker'a sabitlenir"
        );
    }
}
