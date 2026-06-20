//! CUDA (cudarc) backend iskeleti — İP-04: "opsiyonel; `--features cuda`".
//!
//! Şu an yalnızca **iskelet**: gerçek cudarc bağlama MVP-sonrası (`MVP-sonrasi.md` §5.2).
//! Feature etkin olsa bile [`cuda_var`] `false` döndürür (henüz uygulanmadı) → host
//! otomatik olarak wgpu/CPU'ya düşer.  Böylece "tek aktif backend" kuralı korunur.

// TODO(İP-04 / MVP-sonrası §5.2): cudarc cihaz sorgulama + VRAM bütçe yöneticisi
// (wgpu ile aynı anda VRAM kullanılmaz; interop CPU üzerinden).

/// CUDA çalışma zamanında kullanılabilir mi?  İskelet aşamasında daima `false`.
pub fn cuda_var() -> bool {
    false
}
