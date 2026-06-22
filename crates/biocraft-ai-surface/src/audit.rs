//! YZ-08 — **AI denetim kaydı.**  Şeffaflık: kullanıcı ne gönderildiğini görebilir.
//!
//! Her AI çağrısı (ne, hangi sağlayıcı, hangi sınıflar, kaç jeton, sonuç) yerel bir denetim
//! kaydına yazılır.  Kayıt **PII'sizdir**: ham sorgu/öğe içeriği saklanmaz; yalnızca özet
//! meta (uzunluk, öğe sayısı, sınıflar) tutulur → şeffaflık gizliliği bozmadan sağlanır.
// MK-43/45: denetim kaydı PII içermez; şeffaflık + gizlilik birlikte.

use biocraft_types::DataClassification;
use serde::{Deserialize, Serialize};

use crate::context::AiBaglam;
use crate::provider::SaglayiciKimlik;

/// Bir AI çağrısının sonucu (denetim için).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CagriSonucu {
    /// Başarıyla tamamlandı.
    Tamam,
    /// Çıkış kapısı engelledi (PHI).
    Engellendi,
    /// Kullanıcı durdurdu.
    Durduruldu,
    /// Hata.
    Hata,
}

/// Tek bir denetim girdisi — **PII'siz özet** (ham içerik yok).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenetimGirdisi {
    /// Sağlayıcı kimliği (ör. `biocraft.demo.echo`).
    pub saglayici: String,
    /// Sorgu karakter uzunluğu (içerik DEĞİL).
    pub sorgu_uzunlugu: usize,
    /// Bağlama eklenen öğe sayısı.
    pub oge_sayisi: usize,
    /// Gönderilen/denetlenen öğelerin sınıfları (içerik değil, yalnız etiket).
    pub siniflar: Vec<DataClassification>,
    /// Harcanan toplam jeton (biliniyorsa).
    pub jeton: u64,
    /// Çağrının sonucu.
    pub sonuc: CagriSonucu,
}

impl DenetimGirdisi {
    /// Bir bağlam + sağlayıcıdan PII'siz denetim girdisi üretir.
    pub fn baglamdan(
        baglam: &AiBaglam,
        saglayici: &SaglayiciKimlik,
        jeton: u64,
        sonuc: CagriSonucu,
    ) -> Self {
        Self {
            saglayici: saglayici.kimlik.clone(),
            sorgu_uzunlugu: baglam.sorgu.len(),
            oge_sayisi: baglam.ogeler.len(),
            siniflar: baglam.siniflar().collect(),
            jeton,
            sonuc,
        }
    }
}

/// **Denetim kaydı** — append-only (bellekte; kalıcılık kancası ileride).  Son N girdi tutulur.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DenetimKaydi {
    girdiler: Vec<DenetimGirdisi>,
}

impl DenetimKaydi {
    /// Maksimum bellekte tutulan girdi (taşınca en eski düşer).
    pub const MAKS: usize = 500;

    /// Boş kayıt.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir girdi ekler (append-only; MAKS taşınca en eski silinir).
    pub fn kaydet(&mut self, girdi: DenetimGirdisi) {
        self.girdiler.push(girdi);
        if self.girdiler.len() > Self::MAKS {
            self.girdiler.remove(0);
        }
    }

    /// Tüm girdiler (en eskiden yeniye).
    pub fn girdiler(&self) -> &[DenetimGirdisi] {
        &self.girdiler
    }

    /// Toplam girdi sayısı.
    pub fn say(&self) -> usize {
        self.girdiler.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::BaglamOgesi;
    use crate::provider::SaglayiciTuru;

    fn kimlik() -> SaglayiciKimlik {
        SaglayiciKimlik {
            kimlik: "biocraft.demo.echo".into(),
            ad: "Demo".into(),
            tur: SaglayiciTuru::Yerel,
            model: None,
            aciklama: String::new(),
        }
    }

    #[test]
    fn girdi_pii_icermez() {
        // Sorgu metni "hasta Ahmet 555..." olsa bile denetimde yalnız uzunluk/sınıf tutulur.
        let b = AiBaglam::sorgudan("hasta Ahmet 5551234567").oge_ile(BaglamOgesi::yeni(
            "kayıt",
            "gizli",
            DataClassification::HasasPhi,
        ));
        let g = DenetimGirdisi::baglamdan(&b, &kimlik(), 42, CagriSonucu::Tamam);
        assert_eq!(g.oge_sayisi, 1);
        assert_eq!(g.siniflar, vec![DataClassification::HasasPhi]);
        // Serileştirmede ham sorgu/özet GEÇMEZ (yalnız uzunluk).
        let j = serde_json::to_string(&g).unwrap();
        assert!(!j.contains("Ahmet"), "denetim kaydı PII içermemeli");
        assert!(!j.contains("gizli"), "öğe içeriği kaydedilmemeli");
    }

    #[test]
    fn maks_tasinca_en_eski_duser() {
        let mut k = DenetimKaydi::yeni();
        let b = AiBaglam::sorgudan("x");
        for _ in 0..(DenetimKaydi::MAKS + 10) {
            k.kaydet(DenetimGirdisi::baglamdan(
                &b,
                &kimlik(),
                1,
                CagriSonucu::Tamam,
            ));
        }
        assert_eq!(k.say(), DenetimKaydi::MAKS);
    }
}
