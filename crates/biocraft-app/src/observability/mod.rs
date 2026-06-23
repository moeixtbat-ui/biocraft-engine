//! Gözlemlenebilirlik (İP-21, MK-57) — uygulama düzeyinde **log sink'i** + **çökme raporu**.
//!
//! Veri modeli (yapılandırılmış kayıt, iz bağlamı, PII süzgeci) L0
//! [`biocraft_types::obslog`] / [`biocraft_types::trace`]'tedir; bu modül onları gerçek
//! **çıktıya** bağlar: konsol (insan-okur) + dosya (NDJSON, OTel-uyumlu).  `log` cephesi
//! (façade) tüm crate'lerde zaten kullanıldığından, mevcut `log::info!/warn!/error!`
//! çağrıları **olduğu gibi** yapılandırılmış + iz-damgalı hâle gelir.
//!
//! ## Neden `tracing-subscriber` değil?
//! `tracing` cephesi ağaçta (wgpu üzerinden) zaten var; ancak `tracing-subscriber` yeni bir
//! bağımlılık ağacı (sharded-slab, thread_local, matchers, regex-automata…) getirirdi.  Bu
//! projenin "yeni dış bağımlılık yok" disiplini gereği, `log` cephesi üzerine **saf-Rust**
//! ince bir sink yazdık; `tracing` köprüsü (`tracing-log`) gelecekteki bir kancadır.

mod crash;
mod tracing;

pub use crash::kur_panik_kancasi;
pub use tracing::{init, with_iz, GozlemAyari};
