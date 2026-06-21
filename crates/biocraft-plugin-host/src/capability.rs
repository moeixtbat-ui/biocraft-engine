//! Capability (yetki) modeli — ilan + onay + **çalışma-zamanı denetimi** (MK-13).
//!
//! Üç aşama:
//! 1. **İlan** — manifest'te `istenen_yetkiler` (bkz. `manifest.rs`).
//! 2. **Onay** — kurulumda kullanıcı onaylar (UI Gün 14; bu modül onaylanan listeyi alır).
//! 3. **Denetim** — her host API çağrısı [`YetkiKumesi::denetle`]'den geçer; eklenti
//!    diske/ağa **doğrudan** erişemez, yalnızca yetki verilmiş host fonksiyonu üzerinden.
//!
//! **En az yetki (least privilege):** verilen küme = `istenen ∩ onaylanan`.  Manifest bir
//! yetki istemese kullanıcı onaylasa bile verilmez; kullanıcı onaylamazsa manifest istese
//! bile verilmez.

use biocraft_types::{Capability, ErrorReport};
use std::collections::BTreeSet;

/// Bir eklentiye **fiilen verilmiş** yetkilerin kümesi (çalışma-zamanı denetiminin kaynağı).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct YetkiKumesi {
    verilen: BTreeSet<Capability>,
}

impl YetkiKumesi {
    /// Hiç yetki içermeyen küme (varsayılan = en az yetki).
    pub fn bos() -> Self {
        Self::default()
    }

    /// **En az yetki** kuralıyla verilen kümeyi hesaplar: `istenen ∩ onaylanan`.
    ///
    /// * `istenen`   — manifest'te ilan edilen yetkiler.
    /// * `onaylanan` — kullanıcının kurulumda onayladığı yetkiler.
    pub fn ver(istenen: &[Capability], onaylanan: &[Capability]) -> Self {
        let onay: BTreeSet<Capability> = onaylanan.iter().copied().collect();
        let verilen = istenen
            .iter()
            .copied()
            .filter(|c| onay.contains(c))
            .collect();
        Self { verilen }
    }

    /// Bu küme verilen yetkiyi içeriyor mu? (sessiz sorgu)
    pub fn var_mi(&self, cap: Capability) -> bool {
        self.verilen.contains(&cap)
    }

    /// Çalışma-zamanı denetimi: yetki yoksa açıklayıcı [`ErrorReport`] döner.
    ///
    /// Host'un yetki-kapılı her fonksiyonu (dosya/ağ/…) çağrı başında bunu çağırır.
    pub fn denetle(&self, cap: Capability) -> Result<(), ErrorReport> {
        if self.var_mi(cap) {
            Ok(())
        } else {
            let ad = biocraft_sdk::yetenek_metni(cap);
            Err(ErrorReport::new(
                "Eklenti erişimi reddedildi",
                format!("eklenti '{ad}' yetkisini kullanmaya çalıştı ama bu yetki verilmemiş"),
                format!("Eklentiye '{ad}' iznini vermek için eklenti ayarlarından yetkilerini onaylayın"),
            )
            .with_eylem("İzinleri yönet"))
        }
    }

    /// Verilen yetkilerin sıralı listesi (UI/teşhis için).
    pub fn liste(&self) -> Vec<Capability> {
        self.verilen.iter().copied().collect()
    }

    /// Verilen yetki sayısı.
    pub fn sayi(&self) -> usize {
        self.verilen.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn en_az_yetki_kesisim() {
        // İstenen {fs, net}, onaylanan {net, db} → verilen yalnızca {net}.
        let k = YetkiKumesi::ver(
            &[Capability::Fs, Capability::Net],
            &[Capability::Net, Capability::Db],
        );
        assert!(k.var_mi(Capability::Net));
        assert!(!k.var_mi(Capability::Fs)); // onaylanmadı
        assert!(!k.var_mi(Capability::Db)); // istenmedi
        assert_eq!(k.sayi(), 1);
    }

    #[test]
    fn istenmeyen_onaylansa_bile_verilmez() {
        // Manifest hiçbir şey istemiyor; kullanıcı fs onaylasa bile verilmez.
        let k = YetkiKumesi::ver(&[], &[Capability::Fs]);
        assert!(!k.var_mi(Capability::Fs));
        assert_eq!(k.sayi(), 0);
    }

    #[test]
    fn onaylanmayan_istense_bile_verilmez() {
        let k = YetkiKumesi::ver(&[Capability::Fs], &[]);
        assert!(!k.var_mi(Capability::Fs));
    }

    #[test]
    fn denetle_yetki_yoksa_hata() {
        let k = YetkiKumesi::bos();
        let hata = k.denetle(Capability::Fs).unwrap_err();
        assert_eq!(hata.ne_oldu, "Eklenti erişimi reddedildi");
        assert!(hata.neden.contains("fs"));
    }

    #[test]
    fn denetle_yetki_varsa_gecer() {
        let k = YetkiKumesi::ver(&[Capability::Fs], &[Capability::Fs]);
        assert!(k.denetle(Capability::Fs).is_ok());
    }
}
