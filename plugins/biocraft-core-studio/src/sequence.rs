//! ÇE-06 — Dizi görüntüleme/düzenleme + MSA **[TEMEL · iskelet]**.
//!
//! Dizi görüntüleyici/düzenleyici, çoklu dizi hizalama (MSA) görünümü, çeviri/çerçeve.
//! Bugün yalnızca kayıt uzantı noktası açıktır; görünüm sonraki günlerde (ÇE-06) eklenecek.

use biocraft_sdk::{Aktivasyon, YetkiKapisi};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-06";

/// Alt-modülün UI/komut/node kayıtları (şimdilik boş — uzantı noktası).
pub fn kayitlar(_yetkiler: &YetkiKapisi) -> Aktivasyon {
    Aktivasyon::yeni()
}
