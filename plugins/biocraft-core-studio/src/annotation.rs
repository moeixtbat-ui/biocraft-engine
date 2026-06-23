//! ÇE-05 — Anotasyon + iz yönetimi **[TEMEL · iskelet]**.
//!
//! Gen/öznitelik anotasyonu (GFF/GTF/BED), iz ekleme/sıralama/gizleme, kullanıcı izleri.
//! Bugün yalnızca kayıt uzantı noktası açıktır; yönetim sonraki günlerde (ÇE-05) eklenecek.

use biocraft_sdk::{Aktivasyon, YetkiKapisi};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-05";

/// Alt-modülün UI/komut/node kayıtları (şimdilik boş — uzantı noktası).
pub fn kayitlar(_yetkiler: &YetkiKapisi) -> Aktivasyon {
    Aktivasyon::yeni()
}
