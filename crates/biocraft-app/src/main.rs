//! biocraft-app — L5: Ana binary; tüm katmanları birleştirir (MK-40).
//!
//! Gerçek giriş noktası İP-03 (6-bölge kabuk) ve İP-04 (wgpu/egui pencere) ile oluşturulacak.
// MK-08: Aşamalı başlatma — UI <500 ms görünür (stub şimdilik yalnızca yazdırır).

fn main() {
    println!(
        "BioCraft Engine v{} — stub (İP-00 Gün 2; gerçek pencere İP-04'te)",
        env!("CARGO_PKG_VERSION"),
    );
}
