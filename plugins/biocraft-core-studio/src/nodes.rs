//! ÇE-10 — Node entegrasyonu **[TEMEL · iskelet]**.
//!
//! Eklenti, node tabanlı iş akışına (İP-05) yeni düğüm türleri ekler (SDK [`NodeTanimi`] +
//! [`NodeCalistirici`](biocraft_sdk::node::NodeCalistirici)).  Bugün yalnızca kayıt uzantı
//! noktası açıktır; temel node seti sonraki günlerde (ÇE-10) eklenecek.

use biocraft_sdk::{Aktivasyon, YetkiKapisi};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-10";

/// Alt-modülün node/komut kayıtları (şimdilik boş — uzantı noktası).
pub fn kayitlar(_yetkiler: &YetkiKapisi) -> Aktivasyon {
    Aktivasyon::yeni()
}
