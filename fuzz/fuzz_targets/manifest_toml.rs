#![no_main]
//! Fuzz hedefi: **proje manifesti (`biocraft.toml`) ayrıştırıcısı** (İP-02/İP-09).
//!
//! Amaç: rastgele/kötü baytlarla `Manifest::toml_coz` çağrılınca **panik/çökme** olmamalı —
//! ya `Ok` ya net `Err` dönmeli (Rust bellek güvenliği + checked ayrıştırma).
//!
//! Çalıştırma: `cargo +nightly fuzz run manifest_toml`

use libfuzzer_sys::fuzz_target;

fuzz_target!(|veri: &[u8]| {
    if let Ok(metin) = std::str::from_utf8(veri) {
        // Panik etmemeli; sonuç (Ok/Err) önemsiz — yalnızca çökme aranır.
        let _ = biocraft_data::Manifest::toml_coz(metin);
    }
});
