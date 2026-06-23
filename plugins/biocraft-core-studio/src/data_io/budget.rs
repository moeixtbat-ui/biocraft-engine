//! ÇE-01 — **Bellek bütçesi** guard'ı (İP-08, MK-21/MK-22).
//!
//! Out-of-core ilkesi: yalnızca **görünen/sorgulanan bölge** belleğe alınır, tüm dosya değil
//! (MK-09).  Bir bölgeyi belleğe **materyalize etmeden ÖNCE** tahmini bayt, bütçeyle kıyaslanır;
//! aşımda işlem reddedilir ve kullanıcıya **akışla oku / iptal et** önerilir (OOM yerine net karar).
//!
//! Bu, motorun Global Memory Orchestrator'ına (`biocraft-mem`, L2) hafif bir **vekildir**:
//! eklenti L2'ye doğrudan bağlanamaz (MK-17), bu yüzden bütçe değeri host tarafından verilir;
//! gerçek rezervasyon ileride SDK kontratıyla orkestratöre bağlanır.

use biocraft_sdk::biocraft_types::ErrorReport;

/// Bir veri yükleme işlemi için izin verilen tepe bellek (bayt).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BellekButcesi {
    limit_bayt: u64,
}

impl BellekButcesi {
    /// Verili limitle bütçe (host/İP-08 orkestratöründen gelir).
    pub fn yeni(limit_bayt: u64) -> Self {
        Self { limit_bayt }
    }

    /// Pratikte sınırsız (yalnızca test/araç için; üretimde host gerçek bütçe verir).
    pub fn sinirsiz() -> Self {
        Self {
            limit_bayt: u64::MAX,
        }
    }

    /// Makul varsayılan tek-bölge bütçesi (256 MiB) — host belirtmezse.
    pub fn varsayilan() -> Self {
        Self::yeni(256 * (1 << 20))
    }

    /// Bütçe limiti (bayt).
    pub fn limit(&self) -> u64 {
        self.limit_bayt
    }

    /// Tahmini `tahmini_bayt` bellekte tutulacak; bütçeye sığıyor mu?  Aşımda akış/iptal önerir.
    pub fn kontrol(&self, tahmini_bayt: u64) -> Result<(), ErrorReport> {
        if tahmini_bayt <= self.limit_bayt {
            Ok(())
        } else {
            Err(ErrorReport::new(
                "Bellek bütçesi aşıldı",
                format!(
                    "bu işlem ~{} bellek isterken ayrılan bütçe {}",
                    bayt_insan(tahmini_bayt),
                    bayt_insan(self.limit_bayt)
                ),
                "Daha küçük bir bölge seçin veya akışlı (streaming) modda okuyun; isterseniz işlemi iptal edin",
            )
            .with_eylem("Akışlı oku"))
        }
    }
}

impl Default for BellekButcesi {
    fn default() -> Self {
        Self::varsayilan()
    }
}

/// Bayt sayısını insana okunur (KiB/MiB/GiB) gösterir (yalnızca hata mesajı için).
fn bayt_insan(b: u64) -> String {
    const BIRIM: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut deger = b as f64;
    let mut i = 0;
    while deger >= 1024.0 && i < BIRIM.len() - 1 {
        deger /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{b} B")
    } else {
        format!("{deger:.1} {}", BIRIM[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigan_kabul_asan_red() {
        let b = BellekButcesi::yeni(1000);
        assert!(b.kontrol(1000).is_ok());
        assert!(b.kontrol(999).is_ok());
        let hata = b.kontrol(1001).unwrap_err();
        assert_eq!(hata.ne_oldu, "Bellek bütçesi aşıldı");
        assert_eq!(hata.eylem_etiketi.as_deref(), Some("Akışlı oku"));
    }

    #[test]
    fn sinirsiz_her_seyi_kabul() {
        assert!(BellekButcesi::sinirsiz().kontrol(u64::MAX).is_ok());
    }

    #[test]
    fn insan_okunur_birim() {
        assert_eq!(bayt_insan(512), "512 B");
        assert_eq!(bayt_insan(2 * 1024), "2.0 KiB");
        assert_eq!(bayt_insan(3 * 1024 * 1024), "3.0 MiB");
    }
}
