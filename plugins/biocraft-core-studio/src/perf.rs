//! ÇE-12 — Performans/erişilebilirlik/doğruluk yardımcıları **[iskelet]**.
//!
//! Çapraz güvence: 60 FPS bütçesi, erişilebilirlik (klavye/kontrast), bilinen araçlarla
//! golden doğruluk.  Bugün yalnızca kayıt uzantı noktası açıktır; yardımcılar sonraki günlerde
//! (ÇE-12) eklenecek.

use biocraft_sdk::{Aktivasyon, YetkiKapisi};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-12";

/// Alt-modülün UI/komut kayıtları (şimdilik boş — uzantı noktası).
pub fn kayitlar(_yetkiler: &YetkiKapisi) -> Aktivasyon {
    Aktivasyon::yeni()
}
