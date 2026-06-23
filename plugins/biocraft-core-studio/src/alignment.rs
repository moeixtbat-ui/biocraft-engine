//! ÇE-03 — Hizalama (read) görünümü **[TEMEL · iskelet]**.
//!
//! Hizalama izleri (BAM/CRAM): read yığını, kapsama, mismatch/indel gösterimi.  Bugün yalnızca
//! kayıt uzantı noktası açıktır; görünüm sonraki günlerde (ÇE-03) eklenecek.

use biocraft_sdk::{Aktivasyon, YetkiKapisi};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-03";

/// Alt-modülün UI/komut/node kayıtları (şimdilik boş — uzantı noktası).
pub fn kayitlar(_yetkiler: &YetkiKapisi) -> Aktivasyon {
    Aktivasyon::yeni()
}
