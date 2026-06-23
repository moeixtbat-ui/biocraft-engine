//! ÇE-04 — Varyant (VCF) görünüm + filtre **[iskelet]**.
//!
//! Milyonlarca satırlık VCF/BCF: DuckDB/Arrow ile out-of-core tablo, predicate pushdown,
//! filtre/sıralama, varyant izi.  Bugün yalnızca kayıt uzantı noktası açıktır; görünüm
//! sonraki günlerde (ÇE-04) eklenecek.

use biocraft_sdk::{Aktivasyon, YetkiKapisi};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-04";

/// Alt-modülün UI/komut/node kayıtları (şimdilik boş — uzantı noktası).
pub fn kayitlar(_yetkiler: &YetkiKapisi) -> Aktivasyon {
    Aktivasyon::yeni()
}
