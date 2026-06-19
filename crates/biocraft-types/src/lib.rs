//! BioCraft Engine — L0 temel tipler.
//!
//! MK-40: Bu crate hiçbir başka `biocraft-*` crate'ine bağlı değildir;
//! yalnızca `serde`, `uuid` ve `chrono` harici bağımlılıklarını kullanır.
//! Tüm üst katmanlar bu crate'e bağlanır; tersi yasaktır.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Zaman ──────────────────────────────────────────────────────────────────

/// Tüm kayıtlarda UTC kullanılır; yerel saate dönüştürme üst katmanlarda yapılır.
pub type Timestamp = DateTime<Utc>;

// ─── Kimlik tipleri ─────────────────────────────────────────────────────────

/// Bir projeyi benzersiz biçimde tanımlayan UUID-v4 sarmalayıcısı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub Uuid);

impl ProjectId {
    /// Yeni rastgele bir proje kimliği üretir.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Sıfır-değerli (nil) kimlik; yer tutucu olarak kullanılır.
    pub fn nil() -> Self {
        Self(Uuid::nil())
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::nil()
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Bir eklentiyi benzersiz biçimde tanımlayan UUID-v4 sarmalayıcısı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(pub Uuid);

impl PluginId {
    /// Yeni rastgele bir eklenti kimliği üretir.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Sıfır-değerli (nil) kimlik; yer tutucu olarak kullanılır.
    pub fn nil() -> Self {
        Self(Uuid::nil())
    }
}

impl Default for PluginId {
    fn default() -> Self {
        Self::nil()
    }
}

impl std::fmt::Display for PluginId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ─── Sürüm ──────────────────────────────────────────────────────────────────

/// Anlamsal sürüm numarası (SemVer: ana.alt.yama).
/// Kırıcı değişiklik = `major` artar; yeni özellik = `minor`; hata düzeltmesi = `patch`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version {
    /// Ana sürüm — kırıcı (geriye dönük uyumsuz) değişiklikte artar.
    pub major: u32,
    /// Alt sürüm — geriye dönük uyumlu yeni özellikte artar.
    pub minor: u32,
    /// Yama sürümü — geriye dönük uyumlu hata düzeltmesinde artar.
    pub patch: u32,
}

impl Version {
    /// Verilen ana/alt/yama değerleriyle yeni bir sürüm oluşturur.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// ─── Veri sınıflandırması ────────────────────────────────────────────────────

/// Bir veri nesnesinin gizlilik/güvenlik sınıfı.
///
/// MK-42: PHI sınırı çekirdek tarafından korunur; hiçbir eklenti veya AI çağrısı
/// bu sınırı aşamaz.  Sınıflandırma proje yaratılırken zorunlu olarak seçilir (İP-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataClassification {
    /// Kısıtlama gerektirmeyen genel araştırma verisi.
    Normal,
    /// Hasta / kişisel sağlık bilgisi (Hassas-PHI) — en yüksek koruma seviyesi.
    HasasPhi,
    /// Test ve kıyaslama için üretilmiş yapay/sentetik veri.
    Sentetik,
}

// ─── Capability (yetki) ──────────────────────────────────────────────────────

/// Bir eklentinin manifest'inde ilan etmesi gereken sistem yetkisi.
///
/// MK-13: Eklentiler yalnızca talep ettikleri ve kullanıcının onayladığı
/// capability'leri kullanabilir; sandbox dışı erişim reddedilir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Ağ erişimi (giden/gelen TCP/UDP bağlantısı).
    Net,
    /// Dosya sistemi okuma ve/veya yazma erişimi.
    Fs,
    /// GPU hesaplama erişimi (wgpu compute veya cudarc).
    Gpu,
    /// Yerel veya bulut yapay zekâ modellerine erişim.
    Ai,
    /// Yerel veritabanı (SQLite/DuckDB/RocksDB) okuma/yazma erişimi.
    Db,
}

// ─── İş durumu ───────────────────────────────────────────────────────────────

/// Arka plan işinin anlık durumu.
/// Tüm uzun işlemler bu enum üzerinden izlenir; arayüz kare başına bu durumu okur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// İş kuyruğa alındı, henüz çalışmıyor.
    Bekliyor,
    /// İş şu an yürütülüyor. `ilerleme`: 0–100 arası tamamlanma yüzdesi; bilinmiyorsa `None`.
    Calisiyor {
        /// Tamamlanma yüzdesi (0–100); belirsizse `None`.
        ilerleme: Option<u8>,
    },
    /// İş başarıyla tamamlandı.
    Bitti,
    /// İş bir hatayla sonlandı. `mesaj` kullanıcıya gösterilebilir.
    Hata {
        /// Kullanıcıya gösterilebilir hata açıklaması.
        mesaj: String,
    },
}

// ─── BLAKE3 özet ─────────────────────────────────────────────────────────────

/// BLAKE3 kriptografik özet değeri (32 bayt).
///
/// MK-33/MK-34: Veri/proje/güncelleme bütünlük denetimi için kullanılır.
/// Gerçek hash hesaplaması `biocraft-data` crate'inde yapılır; bu tip yalnızca değeri taşır.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blake3Hash(pub [u8; 32]);

impl Blake3Hash {
    /// Sıfır-değerli yer tutucu (henüz hesaplanmamış).
    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    /// Hash'i onaltılı (hex) dizge olarak döndürür — 64 karakter, küçük harf.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

// Güvenlik açısından Debug çıktısında hash değerini maskeliyoruz (loglarda sızmasın diye).
impl std::fmt::Debug for Blake3Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Blake3Hash({})", self.to_hex())
    }
}

// ─── Testler ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- ProjectId ---

    #[test]
    fn project_id_new_benzersiz_olmali() {
        let a = ProjectId::new();
        let b = ProjectId::new();
        assert_ne!(a, b, "Farklı çağrılar aynı ProjectId üretmemeli");
    }

    #[test]
    fn project_id_nil_sifir_olmali() {
        let id = ProjectId::nil();
        assert_eq!(id.0, Uuid::nil());
    }

    #[test]
    fn project_id_serde_gidis_donus() {
        let id = ProjectId::new();
        let json = serde_json::to_string(&id).unwrap();
        let geri: ProjectId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, geri);
    }

    // --- PluginId ---

    #[test]
    fn plugin_id_new_benzersiz_olmali() {
        let a = PluginId::new();
        let b = PluginId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn plugin_id_serde_gidis_donus() {
        let id = PluginId::new();
        let json = serde_json::to_string(&id).unwrap();
        let geri: PluginId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, geri);
    }

    // --- Version ---

    #[test]
    fn version_display_dogru_bicim() {
        let v = Version::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");
    }

    #[test]
    fn version_siralama_dogru() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 1, 0);
        let v3 = Version::new(2, 0, 0);
        assert!(v1 < v2 && v2 < v3);
    }

    #[test]
    fn version_serde_gidis_donus() {
        let v = Version::new(0, 1, 0);
        let json = serde_json::to_string(&v).unwrap();
        let geri: Version = serde_json::from_str(&json).unwrap();
        assert_eq!(v, geri);
    }

    // --- DataClassification ---

    #[test]
    fn data_classification_phi_farkli_olmali() {
        assert_ne!(DataClassification::Normal, DataClassification::HasasPhi);
        assert_ne!(DataClassification::HasasPhi, DataClassification::Sentetik);
    }

    #[test]
    fn data_classification_serde_gidis_donus() {
        let sinif = DataClassification::HasasPhi;
        let json = serde_json::to_string(&sinif).unwrap();
        let geri: DataClassification = serde_json::from_str(&json).unwrap();
        assert_eq!(sinif, geri);
    }

    // --- Capability ---

    #[test]
    fn capability_hashset_icinde_aranabilmeli() {
        use std::collections::HashSet;
        let mut yetkiler = HashSet::new();
        yetkiler.insert(Capability::Net);
        yetkiler.insert(Capability::Fs);
        assert!(yetkiler.contains(&Capability::Net));
        assert!(!yetkiler.contains(&Capability::Gpu));
    }

    #[test]
    fn capability_serde_gidis_donus() {
        let cap = Capability::Ai;
        let json = serde_json::to_string(&cap).unwrap();
        let geri: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, geri);
    }

    // --- JobStatus ---

    #[test]
    fn job_status_calisiyor_ilerleme_okunabilmeli() {
        let durum = JobStatus::Calisiyor { ilerleme: Some(42) };
        match durum {
            JobStatus::Calisiyor { ilerleme: Some(p) } => assert_eq!(p, 42),
            _ => panic!("Beklenen: Calisiyor(42)"),
        }
    }

    #[test]
    fn job_status_hata_mesaj_tasimali() {
        let durum = JobStatus::Hata {
            mesaj: "dosya bulunamadı".to_string(),
        };
        match &durum {
            JobStatus::Hata { mesaj } => assert!(!mesaj.is_empty()),
            _ => panic!("Beklenen: Hata"),
        }
    }

    #[test]
    fn job_status_serde_gidis_donus() {
        let durum = JobStatus::Calisiyor { ilerleme: None };
        let json = serde_json::to_string(&durum).unwrap();
        let geri: JobStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(durum, geri);
    }

    // --- Blake3Hash ---

    #[test]
    fn blake3_zero_hex_uzunluk_64_olmali() {
        let h = Blake3Hash::zero();
        let hex = h.to_hex();
        assert_eq!(hex.len(), 64);
    }

    #[test]
    fn blake3_zero_hex_sadece_sifir_olmali() {
        let h = Blake3Hash::zero();
        assert_eq!(h.to_hex(), "0".repeat(64));
    }

    #[test]
    fn blake3_serde_gidis_donus() {
        let mut baytlar = [0u8; 32];
        baytlar[0] = 0xAB;
        baytlar[31] = 0xCD;
        let h = Blake3Hash(baytlar);
        let json = serde_json::to_string(&h).unwrap();
        let geri: Blake3Hash = serde_json::from_str(&json).unwrap();
        assert_eq!(h, geri);
    }
}
