#![no_main]
//! Fuzz hedefi: **`.bcext` eklenti paketi ayrıştırıcısı** (İP-07/İP-09).
//!
//! Amaç: güvenilmeyen/kötü niyetli paket baytları `BcextPaket::ac`'ı **çökertmemeli**; bozuk
//! uzunluk/sayı alanları (zip-bomb/taşma denemesi) net `Err` ile reddedilmeli (kaynak limiti +
//! checked aritmetik).
//!
//! Çalıştırma: `cargo +nightly fuzz run bcext_paket`

use libfuzzer_sys::fuzz_target;

fuzz_target!(|veri: &[u8]| {
    let _ = biocraft_plugin_host::BcextPaket::ac(veri);
});
