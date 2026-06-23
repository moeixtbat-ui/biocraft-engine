//! ÇE-07 — 3B PDB/mmCIF yapı görüntüleyici **[iskelet]**.
//!
//! Protein/nükleik asit 3B yapısı (PDB/mmCIF): kartonet/çubuk/yüzey, modern GPU render.
//! `gpu` yeteneği gerektirir.  Bugün yalnızca kayıt uzantı noktası açıktır; görüntüleyici
//! sonraki günlerde (ÇE-07) eklenecek.

use biocraft_sdk::{Aktivasyon, YetkiKapisi};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-07";

/// Alt-modülün UI/komut/node kayıtları (şimdilik boş — uzantı noktası).
pub fn kayitlar(_yetkiler: &YetkiKapisi) -> Aktivasyon {
    Aktivasyon::yeni()
}
