//! Eklenti çalıştırma katmanları (MK-12).
//!
//! * **Tier-2 WASM** (`wasm`) — Wasmtime sandbox (Gün 13).
//! * **Tier-3 Python/R** (`subprocess`) — ayrı süreç + JSON satır protokolü (Gün 14, MK-02).
//! * Tier-4 konteyner (`container`) — Gün 14+.
//!
//! Hepsi aynı [`EklentiCalistirici`] sözleşmesini uygular; çağıran taraf hangi katman
//! olduğunu bilmez (MK-17 — tek arayüz).

use biocraft_ipc::{EklentiCagrisi, EklentiYaniti};

pub mod subprocess;
pub mod wasm;

pub use subprocess::{python_bul, AltSurecCalistirici, AltSurecLimitleri};
pub use wasm::{AotOnbellek, KaynakLimitleri, WasmCalistirici};

/// Bir eklentiye fonksiyon çağrısı yapabilen çalıştırıcı (katmandan bağımsız sözleşme).
pub trait EklentiCalistirici {
    /// Eklentide bir fonksiyonu çağırır; sonucu standart [`EklentiYaniti`] olarak döner.
    fn cagir(&self, cagri: EklentiCagrisi) -> EklentiYaniti;
}
