//! biocraft-sdk — L1: Eklenti SDK'sı ve host↔eklenti **kontratı** (MK-17, MK-14).
//!
//! Bu crate, eklentilerin çekirdekle konuştuğu **tek resmi sınırdır.** Eklentiler
//! birbirine doğrudan bağlanmaz; yalnızca burada tanımlı kontrat üzerinden konuşur
//! (MK-17).  Kontratın parçaları:
//!
//! * **Capability adlandırması** — manifest'teki `"fs"`/`"net"` dizgeleri ↔ [`Capability`].
//! * **Eklenti katmanı** ([`EklentiKatmani`]) — MK-12'nin 4 tier'i.
//! * **ABI yüzeyi** — WASM eklentisinin host'tan göreceği import ad alanı + fonksiyon
//!   adları ([`abi`] modülü) ve host'un ilan ettiği [`AbiKontrati`] sürümü (MK-14).
//!
//! ABI **SemVer'lidir** (MK-14): kırıcı değişiklik major artırır.  WASI sürüm
//! seviyesi güncellense bile eksik bir yetenek host SDK üzerinden köprülenir
//! (ekosistem kırılmaz).

// MK-40: L1 katmanı — yalnızca L0'a (biocraft-types) bağlı; üst katman yasak.

/// BioCraft temel tiplerini yeniden dışa aktar; SDK kullananlar tek bağımlılıkla erişir.
pub use biocraft_types;

use biocraft_types::{Capability, Version};

pub mod data;
pub mod node;
pub mod ui;

// ─── ABI yüzeyi (WASM import/export sözleşmesi) ───────────────────────────────

/// WASM eklentisinin host ile konuştuğu sabit isimler.
///
/// Hem host (`biocraft-plugin-host`) hem de gelecekteki rehber-SDK (guest tarafı)
/// **bu sabitlere** bakar; böylece iki taraf isim üzerinde anlaşır (tek doğruluk kaynağı).
pub mod abi {
    /// WASM `import` modül adı (ad alanı).  Tüm host fonksiyonları bu ad altında.
    pub const AD_ALANI: &str = "biocraft";

    /// Host fonksiyonu: `gunluk_yaz(ptr: i32, len: i32)` — eklenti günlüğe yazar.
    /// **Yetenek gerektirmez** (zararsız); host konsolunda görünür.
    pub const IMPORT_GUNLUK_YAZ: &str = "gunluk_yaz";

    /// Host fonksiyonu: `dosya_oku(ptr: i32, len: i32) -> i32` — VFS üzerinden dosya
    /// okur, okunan bayt sayısını döndürür.  **`fs` yeteneği gerektirir** (MK-13);
    /// yetki yoksa host çağrıyı reddeder (trap).
    pub const IMPORT_DOSYA_OKU: &str = "dosya_oku";

    /// Eklentinin **dışa aktarması zorunlu** doğrusal bellek (host buradan okur/yazar).
    pub const EXPORT_BELLEK: &str = "memory";
}

/// Host'un ilan ettiği ABI kontratının **mevcut sürümü** (MK-14, SemVer).
///
/// Eklenti manifest'i bununla **aynı major** ABI'yi hedeflemelidir; farklı major =
/// kırıcı = yükleme reddedilir.  ABI'yi baştan `0.1` sabitliyoruz.
pub const ABI_SURUMU: Version = Version {
    major: 0,
    minor: 1,
    patch: 0,
};

/// Host'un eklentilere sunduğu ABI kontratının özeti (teşhis/uyumluluk için).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbiKontrati {
    /// Kontrat sürümü (SemVer; bkz. [`ABI_SURUMU`]).
    pub surum: Version,
}

impl AbiKontrati {
    /// Çekirdeğin mevcut ABI kontratını döndürür.
    pub fn cekirdek() -> Self {
        Self { surum: ABI_SURUMU }
    }

    /// Verilen ABI sürümünü hedefleyen eklenti bu kontratla **uyumlu mu?**
    /// (Aynı ana sürüm = uyumlu — MK-14.)
    pub fn uyumlu_mu(&self, eklenti_abi: &Version) -> bool {
        self.surum.uyumlu_mu(eklenti_abi)
    }
}

// ─── Eklenti katmanı (MK-12) ──────────────────────────────────────────────────

/// Eklentinin çalıştırma katmanı (MK-12 — hepsi sandbox/süreç-dışı).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EklentiKatmani {
    /// Tier 1: Native Rust (WIT Component) — en hızlı.
    Native,
    /// Tier 2: WASM (Wasmtime sandbox) — herhangi bir dil → WASM. **MVP birincil yol.**
    Wasm,
    /// Tier 3: Python/R — ayrı subprocess + IPC (Gün 14+).
    Python,
    /// Tier 4: Harici ikili — konteyner (Apptainer/Docker, opsiyonel; Gün 14+).
    External,
}

impl EklentiKatmani {
    /// Manifest dizgesinden katman ("native"/"wasm"/"python"/"external") çözer.
    pub fn metinden(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "native" => Some(Self::Native),
            "wasm" => Some(Self::Wasm),
            "python" => Some(Self::Python),
            "external" => Some(Self::External),
            _ => None,
        }
    }

    /// Manifest dizgesi karşılığı.
    pub fn metni(&self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Wasm => "wasm",
            Self::Python => "python",
            Self::External => "external",
        }
    }
}

// ─── Capability adlandırması (manifest ↔ tip) ─────────────────────────────────

/// Manifest dizgesinden ([`Capability`]) yeteneğini çözer (`"net"/"fs"/"gpu"/"ai"/"db"`).
///
/// Tanınmayan dizge `None` döner (manifest doğrulamasında hata olarak raporlanır).
pub fn yetenek_ayristir(s: &str) -> Option<Capability> {
    match s.trim().to_ascii_lowercase().as_str() {
        "net" => Some(Capability::Net),
        "fs" => Some(Capability::Fs),
        "gpu" => Some(Capability::Gpu),
        "ai" => Some(Capability::Ai),
        "db" => Some(Capability::Db),
        _ => None,
    }
}

/// Bir yeteneğin manifest/UI'da görünen kısa adı.
pub fn yetenek_metni(cap: Capability) -> &'static str {
    match cap {
        Capability::Net => "net",
        Capability::Fs => "fs",
        Capability::Gpu => "gpu",
        Capability::Ai => "ai",
        Capability::Db => "db",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_ayni_major_uyumlu() {
        let k = AbiKontrati::cekirdek();
        assert!(k.uyumlu_mu(&Version::new(0, 1, 0)));
        assert!(k.uyumlu_mu(&Version::new(0, 5, 9))); // aynı major (0) → uyumlu
        assert!(!k.uyumlu_mu(&Version::new(1, 0, 0))); // farklı major → kırıcı
    }

    #[test]
    fn katman_gidis_donus() {
        for k in [
            EklentiKatmani::Native,
            EklentiKatmani::Wasm,
            EklentiKatmani::Python,
            EklentiKatmani::External,
        ] {
            assert_eq!(EklentiKatmani::metinden(k.metni()), Some(k));
        }
        assert_eq!(EklentiKatmani::metinden("WASM"), Some(EklentiKatmani::Wasm));
        assert_eq!(EklentiKatmani::metinden("bilinmeyen"), None);
    }

    #[test]
    fn yetenek_gidis_donus() {
        for c in [
            Capability::Net,
            Capability::Fs,
            Capability::Gpu,
            Capability::Ai,
            Capability::Db,
        ] {
            assert_eq!(yetenek_ayristir(yetenek_metni(c)), Some(c));
        }
        assert_eq!(yetenek_ayristir("FS"), Some(Capability::Fs));
        assert_eq!(yetenek_ayristir("dosya"), None);
    }
}
