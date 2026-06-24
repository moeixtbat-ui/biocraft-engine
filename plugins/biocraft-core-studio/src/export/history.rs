//! ÇE-11 — **Eklenti içi geçmiş**: son açılan **dosya** / gidilen **bölge** / yapılan **işlem** →
//! hızlı tekrar erişim ("kaldığın yerden devam").
//!
//! [`db_search::history`](crate::db_search) veritabanı aramalarına özgüdür; bu geçmiş **eklenti
//! genelidir** (tüm görünümler).  En yeni öne gelir, tekrarlar tekilleştirilir, kapasite sınırı vardır
//! (en eski düşer).  Geçmiş **boşken** kullanıcıya kısa bir rehber gösterilir (TDA 5).  JSON ile proje
//! içinde kalıcılaşır.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use biocraft_sdk::biocraft_types::Timestamp;

/// Geçmiş girdisinin türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GecmisTuru {
    /// Açılan dosya.
    Dosya,
    /// Gidilen genom bölgesi.
    Bolge,
    /// Yapılan işlem (analiz/filtre/dışa aktarma vb.).
    Islem,
}

impl GecmisTuru {
    /// Kısa insan-okur etiket.
    pub fn etiket(&self) -> &'static str {
        match self {
            GecmisTuru::Dosya => "Dosya",
            GecmisTuru::Bolge => "Bölge",
            GecmisTuru::Islem => "İşlem",
        }
    }
}

/// Tek bir geçmiş girdisi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GecmisGirdi {
    /// Tür.
    pub tur: GecmisTuru,
    /// İnsan-okur etiket (dosya adı / `chr:bas-bit` / işlem açıklaması).
    pub etiket: String,
    /// Kayıt zamanı (UTC).
    pub zaman: Timestamp,
}

/// Varsayılan geçmiş kapasitesi.
pub const VARSAYILAN_AZAMI: usize = 50;

/// Eklenti içi geçmiş (en yeni önde).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gecmis {
    /// Girdiler (önden arkaya = yeni → eski).
    girdiler: VecDeque<GecmisGirdi>,
    /// Azami girdi sayısı (aşılırsa en eski düşer).
    azami: usize,
}

impl Gecmis {
    /// Varsayılan kapasiteyle boş geçmiş.
    pub fn yeni() -> Self {
        Self::azami_ile(VARSAYILAN_AZAMI)
    }

    /// Belirli kapasiteyle boş geçmiş (en az 1).
    pub fn azami_ile(azami: usize) -> Self {
        Self {
            girdiler: VecDeque::new(),
            azami: azami.max(1),
        }
    }

    /// Bir girdi ekler: aynı (tür, etiket) varsa **öne taşınır** (tekilleştirme); kapasite aşılırsa
    /// en eski düşer.  Zaman damgası = şimdi (UTC).
    pub fn ekle(&mut self, tur: GecmisTuru, etiket: impl Into<String>) {
        let etiket = etiket.into();
        // Var olan eşi kaldır (tekilleştir).
        self.girdiler
            .retain(|g| !(g.tur == tur && g.etiket == etiket));
        self.girdiler.push_front(GecmisGirdi {
            tur,
            etiket,
            zaman: chrono::Utc::now(),
        });
        while self.girdiler.len() > self.azami {
            self.girdiler.pop_back();
        }
    }

    /// En yeni `n` girdi (yeni → eski).
    pub fn son(&self, n: usize) -> Vec<&GecmisGirdi> {
        self.girdiler.iter().take(n).collect()
    }

    /// Tüm girdiler (yeni → eski).
    pub fn tumu(&self) -> Vec<&GecmisGirdi> {
        self.girdiler.iter().collect()
    }

    /// Yalnızca belirli tür (yeni → eski).
    pub fn tur_filtrele(&self, tur: GecmisTuru) -> Vec<&GecmisGirdi> {
        self.girdiler.iter().filter(|g| g.tur == tur).collect()
    }

    /// Girdi sayısı.
    pub fn say(&self) -> usize {
        self.girdiler.len()
    }

    /// Geçmiş boş mu?
    pub fn bos_mu(&self) -> bool {
        self.girdiler.is_empty()
    }

    /// Geçmiş **boşsa** gösterilecek rehber metni (TDA 5: boş-durum yönlendirmesi); doluysa `None`.
    pub fn rehber(&self) -> Option<&'static str> {
        if self.bos_mu() {
            Some(
                "Henüz geçmiş yok. Bir dosya açın veya bir bölgeye gidin; \
son işlemleriniz burada listelenir (tek tıkla geri dönün).",
            )
        } else {
            None
        }
    }

    /// Geçmişi tümüyle temizler.
    pub fn temizle(&mut self) {
        self.girdiler.clear();
    }

    /// JSON'a serileştirir.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// JSON'dan yükler (bozuksa boş geçmiş — geçmiş kritik değil, sessiz güvenli varsayılan uygun).
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or_else(|_| Self::yeni())
    }
}

impl Default for Gecmis {
    fn default() -> Self {
        Self::yeni()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ekle_ve_son_yeni_onde() {
        let mut g = Gecmis::yeni();
        g.ekle(GecmisTuru::Dosya, "a.bam");
        g.ekle(GecmisTuru::Bolge, "chr1:1-100");
        let son = g.son(10);
        assert_eq!(son.len(), 2);
        assert_eq!(son[0].etiket, "chr1:1-100"); // en yeni önde
        assert_eq!(son[1].etiket, "a.bam");
    }

    #[test]
    fn tekillestirme_one_tasir() {
        let mut g = Gecmis::yeni();
        g.ekle(GecmisTuru::Dosya, "a.bam");
        g.ekle(GecmisTuru::Dosya, "b.bam");
        g.ekle(GecmisTuru::Dosya, "a.bam"); // tekrar → öne
        assert_eq!(g.say(), 2);
        assert_eq!(g.son(1)[0].etiket, "a.bam");
    }

    #[test]
    fn ayni_etiket_farkli_tur_ayri_kayit() {
        let mut g = Gecmis::yeni();
        g.ekle(GecmisTuru::Dosya, "chr1");
        g.ekle(GecmisTuru::Bolge, "chr1");
        assert_eq!(g.say(), 2); // tür farklı → tekilleşmez
    }

    #[test]
    fn kapasite_en_eskiyi_dusurur() {
        let mut g = Gecmis::azami_ile(2);
        g.ekle(GecmisTuru::Islem, "1");
        g.ekle(GecmisTuru::Islem, "2");
        g.ekle(GecmisTuru::Islem, "3");
        assert_eq!(g.say(), 2);
        let etiketler: Vec<&str> = g.son(10).iter().map(|x| x.etiket.as_str()).collect();
        assert_eq!(etiketler, vec!["3", "2"]); // "1" düştü
    }

    #[test]
    fn tur_filtrele() {
        let mut g = Gecmis::yeni();
        g.ekle(GecmisTuru::Dosya, "a.bam");
        g.ekle(GecmisTuru::Bolge, "chr1:1-9");
        g.ekle(GecmisTuru::Dosya, "b.vcf");
        assert_eq!(g.tur_filtrele(GecmisTuru::Dosya).len(), 2);
        assert_eq!(g.tur_filtrele(GecmisTuru::Bolge).len(), 1);
    }

    #[test]
    fn bos_rehber_var_dolu_yok() {
        let mut g = Gecmis::yeni();
        assert!(g.bos_mu());
        assert!(g.rehber().is_some());
        g.ekle(GecmisTuru::Dosya, "a.bam");
        assert!(g.rehber().is_none());
    }

    #[test]
    fn json_round_trip() {
        let mut g = Gecmis::azami_ile(5);
        g.ekle(GecmisTuru::Dosya, "a.bam");
        g.ekle(GecmisTuru::Islem, "QUAL>=30 filtresi");
        let json = g.to_json();
        let geri = Gecmis::from_json(&json);
        assert_eq!(g, geri);
    }

    #[test]
    fn bozuk_json_bos_gecmis() {
        let g = Gecmis::from_json("değil");
        assert!(g.bos_mu());
    }
}
