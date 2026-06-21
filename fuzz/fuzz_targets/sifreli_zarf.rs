#![no_main]
//! Fuzz hedefi: **şifreli veri zarfı çözücüsü** (`SifreliVeri::duz_bayttan`) (İP-09).
//!
//! Amaç: diskten okunan rastgele/bozuk şifreli baytlar ayrıştırılırken **panik** olmamalı; kısa/bozuk
//! girdi net `Err`.  (AES-GCM çözme zaten kimlik-doğrulamalı; bu hedef zarf *ayrıştırma* sağlamlığıdır.)
//!
//! Çalıştırma: `cargo +nightly fuzz run sifreli_zarf`

use libfuzzer_sys::fuzz_target;

fuzz_target!(|veri: &[u8]| {
    let _ = biocraft_data::security::crypto::SifreliVeri::duz_bayttan(veri);
});
