//! ÇE-09 (Gün 41) — **Kaynak-başına hız sınırlama** (rate-limit derinleştirme).
//!
//! Gün 40 [`HizSinirlayici`](super::framework::HizSinirlayici) tek bir kovaydı (tüm konektörler
//! aynı sınırı paylaşırdı).  Burada her dış kaynağın **kendi kovası** olur: NCBI E-utilities'in
//! sınırı (3/sn, anahtarla 10/sn) UniProt/PDB/Ensembl/UCSC'den **bağımsızdır** → bir kaynak yavaş
//! diye diğerleri beklemez, ve bir kaynak kendi politikasını aşmaz (ÇE-09 "Otomatik hız sınırlama
//! + kuyruk; aşımda bekle/uyar; kullanıcı limiti görür").
//!
//! Üstel geri-çekilme + yeniden deneme zaten taşıma tarafında
//! ([`tekrar_ile`](crate::data_io::tekrar_ile)); bu modül **istekler-arası asgari aralığı**
//! (sürekli hız) yönetir.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use super::framework::HizSinirlayici;

/// Bir kaynağın varsayılan **istek/sn** politikası → asgari istekler-arası aralık.
///
/// Değerler kaynakların kamuya açık "kibar kullanım" tavsiyelerine göre korumacı seçilir
/// (kesin değil; aşılırsa sunucu 429 döndürür, taşıma geri-çekilmeyle yeniden dener).
pub fn varsayilan_aralik(kaynak: &str) -> Duration {
    // Kaynak adının rozet önekiyle eşle (örn. "NCBI nucleotide", "BLAST blastn").
    let k = kaynak.to_ascii_lowercase();
    let ms: u64 = if k.starts_with("ncbi") {
        334 // ~3/sn (anahtarsız NCBI); anahtarla yönetici ayrıca güncellenebilir
    } else if k.starts_with("blast") {
        3_000 // BLAST işi başlatma seyrek olmalı (sunucu yükü büyük)
    } else if k.starts_with("ensembl") {
        67 // ~15/sn (Ensembl REST kibar sınırı)
    } else if k.starts_with("uniprot")
        || k.starts_with("pdb")
        || k.starts_with("rcsb")
        || k.starts_with("ucsc")
    {
        100 // ~10/sn (UniProt / RCSB-PDB / UCSC kibar sınırı)
    } else {
        200 // bilinmeyen kaynak: korumacı 5/sn
    };
    Duration::from_millis(ms)
}

/// **Kaynak-başına** hız yöneticisi: ad → o kaynağın [`HizSinirlayici`] kovası.
///
/// İlk görülen kaynak için [`varsayilan_aralik`] ile bir kova oluşturulur; özel aralık
/// [`kaynak_ayarla`](Self::kaynak_ayarla) ile geçersiz kılınabilir (örn. NCBI API anahtarı → 10/sn).
/// `Mutex` ile iç-değişebilir → `&self` üzerinden eşzamanlı konektörler güvenle kullanır.
pub struct KaynakHizYoneticisi {
    kovalar: Mutex<HashMap<String, HizSinirlayici>>,
}

impl KaynakHizYoneticisi {
    /// Boş yönetici (kovalar talep edildikçe oluşturulur).
    pub fn yeni() -> Self {
        Self {
            kovalar: Mutex::new(HashMap::new()),
        }
    }

    /// Bir kaynağa **özel** asgari aralık atar (varsayılanı geçersiz kılar).
    /// Örn. NCBI API anahtarı varsa `kaynak_ayarla("NCBI nucleotide", 100ms)`.
    pub fn kaynak_ayarla(&self, kaynak: impl Into<String>, asgari_aralik: Duration) {
        let mut kilit = self.kovalar.lock().unwrap();
        kilit.insert(kaynak.into(), HizSinirlayici::yeni(asgari_aralik));
    }

    /// Bir kaynağın bir sonraki isteğe izin vermeden önce **beklenmesi gereken süre**si; iç zamanı
    /// ilerletir (saf-test edilebilir; UI "şu kadar bekleniyor" göstergesi için de kullanılır).
    pub fn talep_et(&self, kaynak: &str) -> Duration {
        let mut kilit = self.kovalar.lock().unwrap();
        let kova = kilit
            .entry(kaynak.to_string())
            .or_insert_with(|| HizSinirlayici::yeni(varsayilan_aralik(kaynak)));
        kova.talep_et()
    }

    /// Kaynağın hız sınırı gerektiriyorsa **bekler** (gerçek sınırlama; istek öncesi çağrılır).
    pub fn bekle(&self, kaynak: &str) {
        let d = self.talep_et(kaynak);
        if !d.is_zero() {
            std::thread::sleep(d);
        }
    }

    /// Bu yönetici şu ana dek kaç kaynak kovası oluşturdu (UI/tanı).
    pub fn kaynak_sayisi(&self) -> usize {
        self.kovalar.lock().unwrap().len()
    }
}

impl Default for KaynakHizYoneticisi {
    fn default() -> Self {
        Self::yeni()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kaynaklar_bagimsiz_kovalara_sahip() {
        let y = KaynakHizYoneticisi::yeni();
        // İki farklı kaynağın ilk isteği birbirinden bağımsız → ikisi de beklemesiz geçer.
        assert_eq!(y.talep_et("NCBI nucleotide"), Duration::ZERO);
        assert_eq!(y.talep_et("UniProt"), Duration::ZERO);
        assert_eq!(y.kaynak_sayisi(), 2);
    }

    #[test]
    fn ayni_kaynak_ardisik_istegi_bekletir() {
        let y = KaynakHizYoneticisi::yeni();
        assert_eq!(y.talep_et("UniProt"), Duration::ZERO);
        // Aynı kaynağın hemen ardından gelen isteği beklemeli.
        let bekle = y.talep_et("UniProt");
        assert!(bekle > Duration::ZERO);
        assert!(bekle <= varsayilan_aralik("UniProt"));
    }

    #[test]
    fn varsayilan_araliklar_kaynaga_gore() {
        // NCBI (anahtarsız) UniProt'tan daha yavaş kibar sınıra sahip.
        assert!(varsayilan_aralik("NCBI nucleotide") > varsayilan_aralik("UniProt"));
        // BLAST işi en seyrek.
        assert!(varsayilan_aralik("BLAST blastn") >= varsayilan_aralik("NCBI nucleotide"));
        // Ensembl en hızlı kibar sınır (~15/sn).
        assert!(varsayilan_aralik("Ensembl") < varsayilan_aralik("UCSC hg38"));
    }

    #[test]
    fn ozel_aralik_varsayilani_ezer() {
        let y = KaynakHizYoneticisi::yeni();
        // NCBI'ye API anahtarı hızı (10/sn = 100ms) ata.
        y.kaynak_ayarla("NCBI nucleotide", Duration::from_millis(100));
        assert_eq!(y.talep_et("NCBI nucleotide"), Duration::ZERO);
        let bekle = y.talep_et("NCBI nucleotide");
        assert!(bekle <= Duration::from_millis(100));
    }
}
