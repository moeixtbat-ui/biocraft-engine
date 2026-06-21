//! Çökme kurtarma (MK-28 self-healing) — temiz-kapanış bayrağıyla çökme tespiti.
//!
//! Fikir basit ve sağlam: depoda bir **"oturum temiz mi"** bayrağı tutulur.
//! - Açılışta önce **önceki** bayrak okunur.  `temiz` ise (ya da ilk açılışsa) → çökme yok.
//!   `çalışıyor` kalmışsa → önceki oturum düzgün kapanmadı (çöktü/öldürüldü) → **kurtarma sunulur**.
//! - Açılış hemen bayrağı `çalışıyor` yapar; bu oturum çökerse sonraki açılış bunu görür.
//! - Düzgün kapanışta bayrak `temiz` yazılır → bir sonraki açılışta kurtarma sunulmaz.
//!
//! Böylece "çökme yokken kurtarma sunma" (CLAUDE.md not) garanti edilir: kurtarma **yalnızca**
//! bayrak `çalışıyor` kalmışsa sunulur.  Kalıcı durum her hâlükârda yüklenir; bayrak yalnızca
//! kullanıcıya "önceki oturum düzgün kapanmadı" bilgisini (MK-28 kural 3) verip verilmeyeceğini belirler.

use biocraft_types::ErrorReport;

use crate::store::KaliciDepo;

/// Oturum temizlik bayrağının depo anahtarı.
pub const ANAHTAR_TEMIZ: &str = "oturum_temiz";

/// Bayrak değerleri (tek bayt; bütünlük zarfı zaten depoda).
const TEMIZ: &[u8] = b"1";
const CALISIYOR: &[u8] = b"0";

/// Açılışta verilen kurtarma kararı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KurtarmaKarari {
    /// Önceki oturum düzgün kapanmış (ya da ilk açılış) — kurtarma sunulmaz.
    TemizAcilis,
    /// Önceki oturum düzgün kapanmamış — kullanıcıya "kurtarılan oturum" sunulur.
    KurtarmaSunulur,
}

impl KurtarmaKarari {
    /// Kurtarma sunulmalı mı?
    pub fn kurtarma_mi(&self) -> bool {
        matches!(self, KurtarmaKarari::KurtarmaSunulur)
    }
}

/// Açılış kontrolü: önceki bayrağı okur, kararı üretir, sonra bayrağı `çalışıyor` yapar.
///
/// Bayrak yazılamazsa (disk sorunu) yine de güvenli tarafta kalınır: karar `TemizAcilis` döner
/// (kullanıcıyı yanlışlıkla kurtarma diyaloğuyla yormamak için) ama hata çağırana iletilir.
pub fn acilis_kontrol(depo: &dyn KaliciDepo) -> Result<KurtarmaKarari, ErrorReport> {
    let onceki_temiz = match depo.oku(ANAHTAR_TEMIZ)? {
        Some(b) => b.as_slice() == TEMIZ,
        None => true, // ilk açılış: çökme değil.
    };
    // Bu oturumu "çalışıyor" işaretle (çökme tespiti için).
    depo.yaz(ANAHTAR_TEMIZ, CALISIYOR)?;

    Ok(if onceki_temiz {
        KurtarmaKarari::TemizAcilis
    } else {
        KurtarmaKarari::KurtarmaSunulur
    })
}

/// Düzgün kapanış: bayrağı `temiz` yazar → sonraki açılışta kurtarma sunulmaz.
pub fn temiz_kapat(depo: &dyn KaliciDepo) -> Result<(), ErrorReport> {
    depo.yaz(ANAHTAR_TEMIZ, TEMIZ)
}
