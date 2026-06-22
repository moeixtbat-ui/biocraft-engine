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
//! ## Temel LSP + izole ortam (Gün 23, İP-06 2. kısım)
//! - [`lsp`] — **temel** Python tamamlama (out-of-process jedi + saf-Rust yedek; tam zekâ v1.x).
//! - [`ortam`] — her projeye **izole** sanal ortam + paket yönetimi + sürüm kilidi.
// MK-02: Python her zaman ayrı süreçte; in-process (PyO3) YASAK — donmama bundan gelir.

pub mod lsp;
pub mod ortam;
pub mod python;

pub use lsp::{
    jedi_kur_rehberi, jedi_var_mi, lsp_durumu, onek_al, tamamla_async, temel_tamamla, LspDurumu,
    Tamamlama, TamamlamaIstegi, TamamlamaTuru, TamamlamaTutamac, TamamlamaYaniti,
};
pub use ortam::{KuruluPaket, PaketGereksinimi, SanalOrtam};
pub use python::{
    calistir_baslat, calistir_baslat_ile, CalismaModu, CalismaOlay, CalismaTutamac,
    KodCalismaLimitleri,
};
