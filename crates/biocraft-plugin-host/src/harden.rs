//! **Sandbox sertleştirme** — kötü/bozuk dosya direnci + çekirdek-API sıkı doğrulama (MK-12/13/15, İP-09).
//!
//! Eklenti sandbox'ı (Wasmtime bellek/fuel limiti + VFS + capability) İP-07'de kurulmuştur; bu modül
//! onu **kötü niyetli girdiye karşı** sertleştirir:
//!
//! 1. **Kaynak istismarı (zip-bomb / şişme):** Bir arşiv/akış açılırken **bildirilen boyut**, **çıktı
//!    boyutu** ve **sıkıştırma oranı** önceden sınırlanır → küçük bir dosya açılınca GB'larca RAM/disk
//!    tüketen "bomba" **ayrıştırılmadan reddedilir** ([`AyristirmaLimitleri`]).
//! 2. **Checked aritmetik:** Boyut/ofset hesapları taşmaya karşı `checked_*` ile yapılır → bozuk
//!    başlık negatif/dev boyut bildirse bile panik/taşma yerine **net hata** ([`guvenli_topla`]).
//! 3. **Çekirdek-API sıkı doğrulama:** Eklentinin çekirdeğe geçtiği isim/yol/argüman, işlenmeden önce
//!    doğrulanır ([`cekirdek_arg_dogrula`]); şüpheli (boş/aşırı uzun/NUL/denetim karakteri) reddedilir.
//!
//! **OS-düzeyi sert limit (not):** Tier-3 (Python) alt-süreçlerine **gerçek** RAM/CPU tavanı uygulamak
//! için Windows **Job Object** / Linux **cgroup** kancası tasarımdadır ([`SurecSinirlari`]); bu MVP'de
//! işbirlikçi zaman-aşımı + bildirilen limitler uygulanır (İP-07 `subprocess.rs`), native zorlama
//! İP-09 sonrası sürücü/insan-eli entegrasyonudur (NVML deseni — `ARCHITECTURE` §donanım).

use biocraft_types::ErrorReport;

// ─── Kaynak istismarı limitleri (zip-bomb / şişme savunması) ───────────────────

/// Bir dosya/arşiv güvenle açılmadan önce uygulanan kaynak limitleri.
///
/// Varsayılanlar yerel-masaüstü için makul; çağıran bağlama göre sıkabilir (örn. eklenti girdisi).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AyristirmaLimitleri {
    /// Girdinin (sıkıştırılmış/ham) izin verilen en büyük boyutu (bayt).
    pub max_girdi_bayt: u64,
    /// Açılmış (decompressed) toplam çıktının izin verilen en büyük boyutu (bayt).
    pub max_cikti_bayt: u64,
    /// İzin verilen en yüksek şişme oranı (çıktı / girdi).  Tipik zip-bomb >1000x.
    pub max_oran: u64,
    /// Arşivdeki en çok girdi (dosya) sayısı (binlerce küçük dosyalı "milyon-dosya" bombası).
    pub max_girdi_sayisi: usize,
}

impl Default for AyristirmaLimitleri {
    fn default() -> Self {
        Self {
            max_girdi_bayt: 512 * 1024 * 1024,      // 512 MiB sıkıştırılmış
            max_cikti_bayt: 2 * 1024 * 1024 * 1024, // 2 GiB açılmış
            max_oran: 200,                          // 200x şişmeden fazlası şüpheli
            max_girdi_sayisi: 100_000,
        }
    }
}

impl AyristirmaLimitleri {
    /// Eklenti girdisi gibi güvenilmeyen küçük paketler için sıkı limitler.
    pub fn siki() -> Self {
        Self {
            max_girdi_bayt: 64 * 1024 * 1024,  // 64 MiB
            max_cikti_bayt: 256 * 1024 * 1024, // 256 MiB
            max_oran: 100,
            max_girdi_sayisi: 10_000,
        }
    }

    /// Girdi boyutunu denetler (açmadan önce — büyük dosya hiç okunmaz).
    pub fn girdi_denetle(&self, girdi_bayt: u64) -> Result<(), ErrorReport> {
        if girdi_bayt > self.max_girdi_bayt {
            return Err(limit_hatasi(
                "Dosya çok büyük",
                format!(
                    "Girdi {girdi_bayt} bayt; izin verilen en çok {} bayt.",
                    self.max_girdi_bayt
                ),
            ));
        }
        Ok(())
    }

    /// Girdi sayısını denetler.
    pub fn girdi_sayisi_denetle(&self, sayi: usize) -> Result<(), ErrorReport> {
        if sayi > self.max_girdi_sayisi {
            return Err(limit_hatasi(
                "Arşivde çok fazla girdi",
                format!(
                    "{sayi} girdi var; izin verilen en çok {}.",
                    self.max_girdi_sayisi
                ),
            ));
        }
        Ok(())
    }

    /// Açma sırasında: şimdiye dek üretilen çıktı + bu adımın çıktısı limiti aşıyor mu?
    ///
    /// `checked_add` ile taşma güvenli; toplam çıktı `max_cikti_bayt`'ı aşarsa **derhal** reddedilir
    /// (kalan baytlar üretilmeden) — zip-bomb açılmaya başlamadan durdurulur.
    pub fn cikti_denetle(&self, su_ana_dek: u64, eklenecek: u64) -> Result<u64, ErrorReport> {
        let toplam = su_ana_dek.checked_add(eklenecek).ok_or_else(|| {
            limit_hatasi(
                "Açılmış boyut taştı",
                "Bozuk başlık aşırı büyük boyut bildirdi.".to_string(),
            )
        })?;
        if toplam > self.max_cikti_bayt {
            return Err(limit_hatasi(
                "Açılmış veri çok büyük (olası zip-bomb)",
                format!(
                    "Açılmış boyut {toplam} bayt, sınır {} bayt — açma durduruldu.",
                    self.max_cikti_bayt
                ),
            ));
        }
        Ok(toplam)
    }

    /// Sıkıştırma oranını denetler (çıktı / girdi).  `girdi_bayt == 0` ise atlanır.
    pub fn oran_denetle(&self, girdi_bayt: u64, cikti_bayt: u64) -> Result<(), ErrorReport> {
        if girdi_bayt == 0 {
            return Ok(());
        }
        let oran = cikti_bayt / girdi_bayt;
        if oran > self.max_oran {
            return Err(limit_hatasi(
                "Aşırı sıkıştırma oranı (olası zip-bomb)",
                format!(
                    "Şişme oranı {oran}x, sınır {}x — dosya reddedildi.",
                    self.max_oran
                ),
            ));
        }
        Ok(())
    }
}

/// Taşmaya karşı güvenli toplama (bozuk başlıkların bildirdiği boyutları toplarken).
///
/// `checked_add` taşarsa panik yerine net hata → bozuk dosya **çökertmez** (kabul kriteri).
pub fn guvenli_topla(a: u64, b: u64) -> Result<u64, ErrorReport> {
    a.checked_add(b).ok_or_else(|| {
        limit_hatasi(
            "Boyut hesabı taştı",
            "Dosya başlığı geçersiz (aşırı büyük) boyut bildirdi.".to_string(),
        )
    })
}

// ─── Çekirdek-API argüman doğrulama ────────────────────────────────────────────

/// Eklentiden çekirdeğe geçen bir metin argümanını (fonksiyon adı, anahtar, yol parçası) doğrular.
///
/// Reddedilenler: boş, aşırı uzun (>`max_uzunluk`), NUL baytı, denetim (control) karakteri.
/// Bu, eklentinin çekirdek API'sini bozuk/aşırı girdiyle sömürmesini engeller (savunmada derinlik —
/// VFS yol kaçışı ve capability denetimi ayrı kapılarıdır).
pub fn cekirdek_arg_dogrula(arg: &str, max_uzunluk: usize) -> Result<(), ErrorReport> {
    if arg.is_empty() {
        return Err(arg_hatasi("Argüman boş olamaz"));
    }
    if arg.len() > max_uzunluk {
        return Err(arg_hatasi("Argüman çok uzun (olası taşma denemesi)"));
    }
    if arg.contains('\0') {
        return Err(arg_hatasi("Argüman NUL baytı içeremez"));
    }
    if arg.chars().any(|c| c.is_control()) {
        return Err(arg_hatasi("Argüman denetim karakteri içeremez"));
    }
    Ok(())
}

// ─── OS-düzeyi süreç sınırı kancası (Job Object / cgroup) ──────────────────────

/// Tier-3 alt-süreçlerine uygulanacak OS-düzeyi kaynak tavanı (gerçek zorlama kancası).
///
/// MVP'de bu değerler **bildirilir** ve işbirlikçi olarak (zaman-aşımı + alt-süreç limitleri,
/// `subprocess.rs`) uygulanır.  Native sert zorlama — Windows **Job Object**
/// (`CreateJobObject`/`SetInformationJobObject`) ve Linux **cgroup v2** — sürücü/platform
/// entegrasyonudur ve İP-09 sonrası eklenir (bkz. modül notu).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurecSinirlari {
    /// İşletim sistemince zorlanacak en yüksek RAM (bayt).
    pub max_ram_bayt: u64,
    /// İşletim sistemince zorlanacak en yüksek CPU yüzdesi (0–100).
    pub max_cpu_yuzde: u8,
}

impl Default for SurecSinirlari {
    fn default() -> Self {
        Self {
            max_ram_bayt: 2 * 1024 * 1024 * 1024, // 2 GiB (subprocess.rs ile aynı varsayılan)
            max_cpu_yuzde: 50,
        }
    }
}

impl SurecSinirlari {
    /// Bu sürümde native OS zorlaması bağlı mı?  (Şeffaflık/teşhis için — şu an `false`.)
    pub fn native_zorlama_aktif() -> bool {
        // TODO(MK-12): Windows Job Object + Linux cgroup v2 native zorlaması (İP-09 sonrası).
        false
    }
}

// ─── Hata yardımcıları ─────────────────────────────────────────────────────────

fn limit_hatasi(ne: &str, neden: String) -> ErrorReport {
    ErrorReport::new(
        ne.to_string(),
        neden,
        "Dosyayı güvenilir bir kaynaktan tekrar alın; kasıtlı 'bomba' dosyaları reddedilir.",
    )
}

fn arg_hatasi(ne: &str) -> ErrorReport {
    ErrorReport::new(
        ne.to_string(),
        "Eklentiden çekirdeğe geçen bir değer güvenlik doğrulamasını geçmedi.",
        "Bu bir eklenti hatasıdır; eklenti geliştiricisine bildirin.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buyuk_girdi_reddedilir() {
        let l = AyristirmaLimitleri::siki();
        assert!(l.girdi_denetle(l.max_girdi_bayt + 1).is_err());
        assert!(l.girdi_denetle(1024).is_ok());
    }

    #[test]
    fn zip_bomb_orani_reddedilir() {
        let l = AyristirmaLimitleri::default();
        // 1 KiB girdi → 1 GiB çıktı = ~1.000.000x → sınır 200x → reddedilmeli.
        assert!(l.oran_denetle(1024, 1024 * 1024 * 1024).is_err());
        // Makul oran (10x) geçer.
        assert!(l.oran_denetle(1024, 10 * 1024).is_ok());
    }

    #[test]
    fn cikti_limiti_acma_sirasinda_durdurur() {
        let l = AyristirmaLimitleri::siki();
        // Birikmiş çıktı sınıra yakın; bir sonraki blok sınırı aşar → derhal reddet.
        let mevcut = l.max_cikti_bayt - 10;
        assert!(l.cikti_denetle(mevcut, 100).is_err());
        // Sınır içindeyse toplam döner.
        assert_eq!(l.cikti_denetle(0, 100).unwrap(), 100);
    }

    #[test]
    fn cikti_tasmasi_panik_degil_hata() {
        let l = AyristirmaLimitleri::default();
        // u64 taşması checked_add ile yakalanmalı (panik YOK).
        assert!(l.cikti_denetle(u64::MAX, 1).is_err());
    }

    #[test]
    fn guvenli_topla_tasma() {
        assert_eq!(guvenli_topla(2, 3).unwrap(), 5);
        assert!(guvenli_topla(u64::MAX, 1).is_err());
    }

    #[test]
    fn cok_girdi_reddedilir() {
        let l = AyristirmaLimitleri::siki();
        assert!(l.girdi_sayisi_denetle(l.max_girdi_sayisi + 1).is_err());
        assert!(l.girdi_sayisi_denetle(5).is_ok());
    }

    #[test]
    fn cekirdek_arg_dogrulama() {
        assert!(cekirdek_arg_dogrula("merhaba", 256).is_ok());
        assert!(cekirdek_arg_dogrula("", 256).is_err()); // boş
        assert!(cekirdek_arg_dogrula(&"x".repeat(300), 256).is_err()); // çok uzun
        assert!(cekirdek_arg_dogrula("ad\0gizli", 256).is_err()); // NUL
        assert!(cekirdek_arg_dogrula("satir\nsonu", 256).is_err()); // denetim karakteri
    }

    #[test]
    fn native_zorlama_seffaf() {
        // Şeffaflık: bu sürümde native OS zorlaması bağlı değil (işbirlikçi limit + zaman aşımı).
        assert!(!SurecSinirlari::native_zorlama_aktif());
        assert_eq!(SurecSinirlari::default().max_cpu_yuzde, 50);
    }
}
