#![no_main]
//! Fuzz hedefi: **BCP1 bütünlük zarfı çözücüsü** (İP-02/İP-09).
//!
//! Amaç: bozuk/kısa/uydurma zarf baytları `zarf_coz`'u **çökertmemeli**; bütünlük tutmazsa net `Err`.
//!
//! Çalıştırma: `cargo +nightly fuzz run bcp1_zarf`

use libfuzzer_sys::fuzz_target;
use std::path::Path;

fuzz_target!(|veri: &[u8]| {
    let _ = biocraft_data::project::integrity::zarf_coz(veri, Path::new("fuzz"));
});
