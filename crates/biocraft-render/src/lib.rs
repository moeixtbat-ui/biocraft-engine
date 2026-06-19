//! biocraft-render — L3: wgpu/egui render altyapısı + tasarım token'ları stub (MK-01, MK-04).
//!
//! GPU batch ≤100 ms (TDR-güvenli); DeviceLost kurtarma <5 s; tasarım token'ları tokens.json'dan.
// MK-40: L3 katmanı — yalnızca L0/L1/L2 katmanlarına bağlı; üst katman yasak.

pub use biocraft_mem;
pub use biocraft_sdk;
pub use biocraft_types;
