//! biocraft-plugin-host — L3: Wasmtime + capability + subprocess/konteyner yönetimi stub.
//!
//! Tier1 Native / Tier2 WASM / Tier3 Python-subprocess / Tier4 Apptainer (MK-12, MK-13, MK-15).
// MK-40: L3 katmanı — yalnızca L0/L1/L2 katmanlarına bağlı; üst katman yasak.

pub use biocraft_ipc;
pub use biocraft_mem;
pub use biocraft_sdk;
pub use biocraft_types;
