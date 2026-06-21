//! Kullanıcı kodunu **ayrı süreçte** çalıştırma (İP-06, MK-02).
//!
//! Kod editörünün "Çalıştır" / "Hücreyi Çalıştır" düğmeleri buraya iner.  Kullanıcının
//! yazdığı kod **asla çekirdek süreçte** değil, **ayrı bir Python sürecinde** çalışır.
//! Bu sayede:
//! * sonsuz döngü / kötü kod **arayüzü dondurmaz** (ayrı süreç + kare-başı yoklama),
//! * **"Durdur"** her an süreci öldürür ([`CalismaTutamac::durdur`]),
//! * çıktı (stdout/stderr) **satır satır akışla** geri gelir (gerçek zamanlı).
//!
//! Eklenti RPC köprüsünden ([`crate::runtime::subprocess`]) farkı: o, eklentinin **tanımlı**
//! fonksiyonlarını JSON protokolüyle çağırır; bu modül ise kullanıcının yazdığı **serbest**
//! bir betiği çalıştırıp **ham** çıktısını yayınlar.  İkisi de Python keşfini
//! ([`crate::runtime::subprocess::python_bul`]) paylaşır (MK-02 — Python yoksa
//! in-process'e DÖNÜLMEZ, "kur" rehberi gösterilir).
//!
//! ## Tier-3 → LSP (Gün 23)
//! Temel LSP (pyright/jedi, out-of-process) ve `lsp.rs` yarın eklenir; bugünkü kapsam
//! yalnızca **çalıştırma**dır (İP-06 1. kısım).
// MK-02: Python her zaman ayrı süreçte; in-process (PyO3) YASAK — donmama bundan gelir.

pub mod python;

pub use python::{calistir_baslat, CalismaModu, CalismaOlay, CalismaTutamac, KodCalismaLimitleri};
