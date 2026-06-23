//! ÇE-11 — Dışa aktarma + oturum **[iskelet]**.
//!
//! Görsel (PNG/SVG) + veri (tablo/altküme) dışa aktarma, temel rapor, oturum/görünüm kaydı.
//! Bugün yalnızca kayıt uzantı noktası açıktır; dışa aktarma sonraki günlerde (ÇE-11) eklenecek.

use biocraft_sdk::{Aktivasyon, YetkiKapisi};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-11";

/// Alt-modülün UI/komut kayıtları (şimdilik boş — uzantı noktası).
pub fn kayitlar(_yetkiler: &YetkiKapisi) -> Aktivasyon {
    Aktivasyon::yeni()
}
