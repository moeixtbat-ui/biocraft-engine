//! ÇE-02 — Genom Tarayıcı (Genome Browser) **[iskelet]**.
//!
//! IGV+JBrowse+UCSC seviyesinde, gerçek 60 FPS akıcılıkta çok-izli genom tarayıcı tuvali
//! (wgpu çizim + egui kontroller; Bevy ECS YOK).  `gpu` yeteneği gerektirir.  Bugün yalnızca
//! kayıt uzantı noktası açıktır; tuval sonraki günlerde (ÇE-02) eklenecek.

use biocraft_sdk::{Aktivasyon, YetkiKapisi};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-02";

/// Alt-modülün UI/komut/node kayıtları (şimdilik boş — uzantı noktası).
pub fn kayitlar(_yetkiler: &YetkiKapisi) -> Aktivasyon {
    Aktivasyon::yeni()
}
