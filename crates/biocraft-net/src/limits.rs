//! **Kaynak paylaşım sınırı + Bio-kredi yer tutucusu** (İP-15) — özerklik ve gelecekteki ekonomi kancası.
//!
//! - [`KaynakSiniri`]: bu makinenin dağıtık ağa ne kadar CPU/GPU/bellek paylaşacağı.  **Varsayılan
//!   paylaşım YOK** (opt-in, MK-50): kullanıcı açıkça açmadıkça hiçbir kaynak paylaşılmaz.
//! - [`BioKrediKanca`]: gelecekteki "Bio-kredi" katkı ölçümünün **kavramsal yer tutucusu.**
//!   **Kripto/blockchain DEĞİL** (ARCHITECTURE §13).  Gerçek ekonomi/ödeme akışı dağıtık-ağ
//!   eklentisi + hukukçu onayından sonra gelir (`Hukuk-ve-Operasyon.md`, `MVP-sonrasi.md` §2.2).

use serde::{Deserialize, Serialize};

/// Bu makinenin dağıtık ağa **paylaşacağı kaynak sınırı** (özerklik — opt-in).
///
/// `Default` = **hiç paylaşım yok**: `etkin=false`, tüm yüzdeler 0.  Kullanıcı ayardan açmadıkça
/// makine kaynakları ağa verilmez (MK-50: "kaynak paylaşımı opt-in").
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KaynakSiniri {
    /// Kaynak paylaşımı açık mı?  Varsayılan **false** (kapalı).
    pub etkin: bool,
    /// Paylaşılacak azami CPU yüzdesi (0..=100).
    pub azami_cpu_yuzde: u8,
    /// Paylaşılacak azami GPU yüzdesi (0..=100).
    pub azami_gpu_yuzde: u8,
    /// Paylaşılacak azami bellek (MiB).  0 = paylaşım yok.
    pub azami_bellek_mib: u64,
    /// Yalnızca makine **boştayken** mi paylaşılsın (örn. ekran kilitliyken)?
    pub yalnizca_bostayken: bool,
}

impl Default for KaynakSiniri {
    /// Güvenli varsayılan: hiçbir kaynak paylaşılmaz (opt-in).
    fn default() -> Self {
        Self {
            etkin: false,
            azami_cpu_yuzde: 0,
            azami_gpu_yuzde: 0,
            azami_bellek_mib: 0,
            yalnizca_bostayken: true,
        }
    }
}

impl KaynakSiniri {
    /// Şu an gerçekten kaynak paylaşılıyor mu?  Etkin değilse veya tüm yüzdeler 0 ise hayır.
    pub fn paylasim_var_mi(&self) -> bool {
        self.etkin
            && (self.azami_cpu_yuzde > 0 || self.azami_gpu_yuzde > 0 || self.azami_bellek_mib > 0)
    }

    /// Yüzde alanlarını 0..=100 aralığına sıkıştırarak güvenli bir sınır döndürür.
    pub fn gecerli_kil(mut self) -> Self {
        self.azami_cpu_yuzde = self.azami_cpu_yuzde.min(100);
        self.azami_gpu_yuzde = self.azami_gpu_yuzde.min(100);
        self
    }
}

/// **Bio-kredi kancası** (İP-15 — kavramsal yer tutucu).
///
/// Bio-kredi, gelecekte kullanıcıların dağıtık ağa kaynak **katkısını** ölçen birimdir.  Bu MVP'de:
/// - **kripto/blockchain DEĞİL** (ARCHITECTURE §13 — bu sürüm kapsamı dışı),
/// - gerçek ölçüm/ödeme/ekonomi **bağlı değil** (`bagli=false`),
/// - yalnızca gelecekteki sözleşme noktasını sabitler: katkı (örn. CPU-saniye) bir kredi miktarına
///   **çevrilebilir** olmalıdır.  Gerçek katsayı/ekonomi eklenti + hukukçu sonrası gelir.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BioKrediKanca {
    /// Ekonomi bağlı mı?  MVP'de daima `false` (yer tutucu).
    pub bagli: bool,
    /// 1 birim katkı (örn. 1 CPU-saniye) kaç Bio-krediye denk?  `None` = bağlanmadı (MVP).
    pub katki_basi_kredi: Option<f64>,
}

impl BioKrediKanca {
    /// Bir katkı miktarını Bio-krediye çevirir; kanca bağlı değilse `None` (MVP durumu).
    pub fn krediye_cevir(&self, katki: f64) -> Option<f64> {
        if !self.bagli {
            return None;
        }
        self.katki_basi_kredi.map(|k| katki * k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varsayilan_paylasim_yok() {
        // MK-50: kaynak paylaşımı opt-in → varsayılan kapalı, hiçbir kaynak verilmez.
        let s = KaynakSiniri::default();
        assert!(!s.etkin);
        assert!(!s.paylasim_var_mi());
        assert_eq!(s.azami_cpu_yuzde, 0);
    }

    #[test]
    fn etkin_ama_sifir_yuzde_paylasim_sayilmaz() {
        let s = KaynakSiniri {
            etkin: true,
            ..Default::default()
        };
        assert!(!s.paylasim_var_mi(), "yüzdeler 0 → fiilen paylaşım yok");
    }

    #[test]
    fn gecerli_kil_yuzdeyi_sikistirir() {
        let s = KaynakSiniri {
            etkin: true,
            azami_cpu_yuzde: 250,
            azami_gpu_yuzde: 130,
            ..Default::default()
        }
        .gecerli_kil();
        assert_eq!(s.azami_cpu_yuzde, 100);
        assert_eq!(s.azami_gpu_yuzde, 100);
        assert!(s.paylasim_var_mi());
    }

    #[test]
    fn biokredi_mvp_de_bagli_degil() {
        let kanca = BioKrediKanca::default();
        assert!(!kanca.bagli);
        assert_eq!(
            kanca.krediye_cevir(100.0),
            None,
            "MVP'de Bio-kredi bağlı değil"
        );
    }
}
