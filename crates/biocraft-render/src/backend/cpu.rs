//! CPU yazılım fallback notu (İP-04 TDA: "GPU yoksa CPU fallback + uyarı").
//!
//! Gerçek CPU yolu, wgpu'nun *fallback adapter*'ı (WARP / lavapipe gibi yazılım
//! rasterleştirici) istenerek elde edilir — bkz. [`crate::gpu`].  Ayrı bir yazılım çizici
//! yoktur; **tek** wgpu yolu hem donanımda hem yazılımda çalışır (bakım kolaylığı).

/// Kullanıcıya gösterilecek CPU modu uyarısının varsayılan (TR) metni.
/// Gerçek i18n UI katmanındadır; render katmanı çekirdek metni sağlar.
pub fn cpu_notu() -> &'static str {
    "GPU bulunamadı veya devre dışı — yazılım (CPU) modunda çalışılıyor. \
     Görüntü akıcı kalır ancak ağır 3B sahnelerde performans düşebilir."
}
