//! Eklenti çalıştırma katmanları (MK-12).
//!
//! Bugün (Gün 13): **Tier-2 WASM** (`wasm`) — Wasmtime sandbox.
//! Yarın+ (Gün 14+): Tier-3 Python subprocess (`subprocess`), Tier-4 konteyner (`container`).
//! Hepsi aynı [`EklentiCalistirici`] sözleşmesini uygular; çağıran taraf hangi katman
//! olduğunu bilmez (MK-17 — tek arayüz).

use biocraft_ipc::{EklentiCagrisi, EklentiYaniti};

pub mod wasm;

pub use wasm::{AotOnbellek, KaynakLimitleri, WasmCalistirici};

/// Bir eklentiye fonksiyon çağrısı yapabilen çalıştırıcı (katmandan bağımsız sözleşme).
pub trait EklentiCalistirici {
    /// Eklentide bir fonksiyonu çağırır; sonucu standart [`EklentiYaniti`] olarak döner.
    fn cagir(&self, cagri: EklentiCagrisi) -> EklentiYaniti;
}
