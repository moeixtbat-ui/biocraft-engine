//! ÇE-08 — Analiz/hesap + harici araç köprüsü **[TEMEL · iskelet]**.
//!
//! Hesap işleri (istatistik, dönüşüm) ve harici bio-araç köprüsü (subprocess/konteyner — İP-07).
//! Bugün yalnızca kayıt uzantı noktası açıktır; hesaplar sonraki günlerde (ÇE-08) eklenecek.

use biocraft_sdk::{Aktivasyon, YetkiKapisi};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-08";

/// Alt-modülün UI/komut/node kayıtları (şimdilik boş — uzantı noktası).
pub fn kayitlar(_yetkiler: &YetkiKapisi) -> Aktivasyon {
    Aktivasyon::yeni()
}
