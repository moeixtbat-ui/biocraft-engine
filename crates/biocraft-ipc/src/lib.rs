//! biocraft-ipc — L1: IPC/gRPC/Arrow Flight köprüleri (MK-39, MK-30).
//!
//! Kontrol kanalı gRPC; büyük veri Arrow Flight + shared memory üzerinden taşınır.
// MK-40: L1 katmanı — yalnızca L0'a (biocraft-types) bağlı; üst katman yasak.

/// Temel tipler IPC mesajlarında kullanılacak — şimdilik yeniden dışa aktarım.
pub use biocraft_types;
