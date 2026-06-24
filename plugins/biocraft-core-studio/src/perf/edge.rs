//! ÇE-12 — **Edge-case (sınır durum) dayanıklılığı** (Bölüm 0.12, İP-21, TDA).
//!
//! Çekirdek eklentinin tüm modülleri sınır durumlarda **güvenli + anlaşılır** davranmalı: boş /
//! tek-kayıt / çok-büyük / bozuk dosya, eksik indeks, Unicode ad, ağ kesintisi, GPU yok, düşük
//! bellek.  Eşik mantığı L0 [`biocraft_sdk::biocraft_types::esikler`]'tedir (disk/ağ/zaman aşımı);
//! bu modül onları **çekirdek eklenti yüzeyine** taşır ve her durum için **standart hata şeması**
//! (ne/neden/çözüm + correlation_id — Gün 4/32) üretir.
//!
//! Tüm hata üreticileri [`ErrorReport`] döndürür (panik yok — CLAUDE.md §3: `Result`, panik değil).

use biocraft_sdk::biocraft_types::esikler::Gericekilme;
use biocraft_sdk::biocraft_types::{CorrelationId, ErrorReport, TraceContext};

// ─── Veri boyutu durumu (boş / tek / büyük / çok büyük) ───────────────────────

/// Bir veri kümesinin (kayıt sayısına göre) boyut durumu — boş-durum rehberi (TDA 5) + LOD/bütçe
/// kararını sürer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VeriDurumu {
    /// Hiç kayıt yok — kullanıcıya **boş-durum rehberi** gösterilir (çökme/boş ekran değil).
    Bos,
    /// Tek kayıt — istatistik/özet anlamlı olmayabilir (uç durum; yine de güvenli).
    TekKayit,
    /// Normal aralık.
    Normal,
    /// Büyük — seyreltme/LOD önerilir.
    Buyuk,
    /// Çok büyük — yalnız özet + akışlı; "load all" yasak (MK-09).
    CokBuyuk,
}

/// Büyük eşiği (kayıt) — bunun üstü seyreltme önerir.
pub const ESIK_BUYUK: u64 = 100_000;
/// Çok büyük eşiği (kayıt) — yalnız özet/akışlı.
pub const ESIK_COK_BUYUK: u64 = 5_000_000;

impl VeriDurumu {
    /// Kayıt sayısından durumu sınıflandırır.
    pub fn siniflandir(kayit: u64) -> Self {
        match kayit {
            0 => VeriDurumu::Bos,
            1 => VeriDurumu::TekKayit,
            n if n > ESIK_COK_BUYUK => VeriDurumu::CokBuyuk,
            n if n > ESIK_BUYUK => VeriDurumu::Buyuk,
            _ => VeriDurumu::Normal,
        }
    }

    /// Boş durumda kullanıcıya gösterilecek **rehber metni** (TDA 5); doluysa `None`.
    pub fn bos_rehberi(&self, ne: &str) -> Option<String> {
        if *self == VeriDurumu::Bos {
            Some(format!(
                "{ne} bu bölgede kayıt içermiyor. Farklı bir bölgeye gidin veya filtreyi gevşetin."
            ))
        } else {
            None
        }
    }
}

// ─── Standart hata üreticileri (Gün 4 şeması + correlation_id) ────────────────

/// **Bozuk/ayrıştırılamayan dosya** hatası.  Sessiz/yanlış okuma YOK (MK-32) → net hata + (varsa)
/// satır/sütun + karantina önerisi (Bölüm 0.12).
pub fn bozuk_dosya(dosya: &str, satir: Option<u64>, detay: impl Into<String>) -> ErrorReport {
    let konum = match satir {
        Some(s) => format!(" (satır {s})"),
        None => String::new(),
    };
    ErrorReport::new(
        format!("Dosya okunamadı: {dosya}"),
        format!("Dosya bozuk veya beklenen biçimde değil{konum}."),
        "Dosyayı yeniden indirin/oluşturun; bozuk dosya karantinaya alınabilir.",
    )
    .with_eylem("Karantinaya al")
    .with_teknik_detay(detay)
}

/// **Eksik indeks** hatası — indeks olmadan büyük dosyada bölge sorgusu yapılamaz; kullanıcıya
/// "indeks oluştur" eylemi sunulur (data_io ile aynı çözüm yolu).
pub fn indeks_yok(dosya: &str, indeks_uzanti: &str) -> ErrorReport {
    ErrorReport::new(
        format!("İndeks bulunamadı: {dosya}"),
        format!("Bölge sorgusu için {indeks_uzanti} indeksi gerekir; dosyanın yanında yok."),
        "Bu dosya için indeks oluşturun (bir kez); sonra bölge sorgusu hızlanır.",
    )
    .with_eylem("İndeks oluştur")
}

/// **GPU yok** — bilgi düzeyinde (hata değil): CPU yedeğine düşülür, görsel kaybolmaz (ÇE-07).
pub fn gpu_yok_bilgisi() -> ErrorReport {
    ErrorReport::new(
        "GPU bulunamadı",
        "Bu sistemde uygun bir GPU yok ya da sürücü kaybedildi (TDR/DeviceLost).",
        "Görüntüleme yazılım (CPU) yedeğiyle sürer; çok büyük 3B yapıda sadeleştirilir.",
    )
}

/// **Ağ kesintisi** — üstel geri çekilme (1s→60s, max 5; L0 [`Gericekilme`]).  Deneme bitince
/// kalıcı hata + çevrimdışı öneri.
pub fn ag_kesintisi(gericekilme: &Gericekilme, kaynak: &str) -> ErrorReport {
    if gericekilme.devam_eder_mi() {
        ErrorReport::new(
            format!("{kaynak} bağlantısı kesildi"),
            format!(
                "Ağ yanıt vermiyor; {} sn sonra yeniden denenecek (deneme {}/{}).",
                gericekilme.gecikme_saniye(),
                gericekilme.deneme + 1,
                biocraft_sdk::biocraft_types::esikler::GERICEKILME_MAKS_DENEME
            ),
            "İnternet bağlantınızı kontrol edin; otomatik yeniden denenecek.",
        )
        .with_eylem("Şimdi yeniden dene")
    } else {
        ErrorReport::new(
            format!("{kaynak} bağlantısı kurulamadı"),
            "Birden çok denemeye rağmen ağ yanıt vermedi.",
            "Çevrimdışı çalışmaya devam edin (önbellekteki veriler kullanılır) veya sonra deneyin.",
        )
        .with_eylem("Çevrimdışı sürdür")
    }
}

/// Bir [`ErrorReport`]'a **iz bağlamından** correlation_id basar (loglarla diyaloğu eşler — İP-16).
/// Hata bir uzun iş/dış çağrı içindeyse, o işin [`TraceContext`]'i geçilir → kullanıcının gördüğü
/// kısa kimlik loglardaki `trace_id` önekidir.
pub fn iz_ile(rapor: ErrorReport, iz: &TraceContext) -> ErrorReport {
    rapor.with_correlation_id(iz.correlation_id())
}

/// Bir [`ErrorReport`]'a belirli bir [`CorrelationId`] basar.
pub fn kimlik_ile(rapor: ErrorReport, kimlik: CorrelationId) -> ErrorReport {
    rapor.with_correlation_id(kimlik)
}

// ─── Unicode / güvenli ad ──────────────────────────────────────────────────────

/// Bir dosya/örnek adının **görüntüleme için güvenli** kısaltması.  Unicode (Türkçe/emoji/CJK)
/// **korunur** (bozulmaz); yalnız kontrol karakterleri ayıklanır ve çok uzun ad kısaltılır.
/// (Bayt değil **karakter** sınırı → çok-baytlı UTF-8 ortadan kesilmez = panik/� yok.)
pub fn guvenli_ad(ad: &str, azami_karakter: usize) -> String {
    let temiz: String = ad
        .chars()
        .filter(|c| !c.is_control()) // satır sonu/sekme vb. gösterimde ayıkla
        .collect();
    let sayim = temiz.chars().count();
    if sayim <= azami_karakter {
        temiz
    } else {
        let bas: String = temiz
            .chars()
            .take(azami_karakter.saturating_sub(1))
            .collect();
        format!("{bas}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn veri_durumu_siniflandirma() {
        assert_eq!(VeriDurumu::siniflandir(0), VeriDurumu::Bos);
        assert_eq!(VeriDurumu::siniflandir(1), VeriDurumu::TekKayit);
        assert_eq!(VeriDurumu::siniflandir(500), VeriDurumu::Normal);
        assert_eq!(VeriDurumu::siniflandir(ESIK_BUYUK + 1), VeriDurumu::Buyuk);
        assert_eq!(
            VeriDurumu::siniflandir(ESIK_COK_BUYUK + 1),
            VeriDurumu::CokBuyuk
        );
    }

    #[test]
    fn bos_durumda_rehber_dolu_durumda_yok() {
        assert!(VeriDurumu::Bos
            .bos_rehberi("Varyant")
            .unwrap()
            .contains("kayıt içermiyor"));
        assert!(VeriDurumu::Normal.bos_rehberi("Varyant").is_none());
    }

    #[test]
    fn bozuk_dosya_standart_sema_correlation_id() {
        let h = bozuk_dosya("ornek.bam", Some(42), "EOF beklenmedik blok");
        assert!(h.ne_oldu.contains("ornek.bam"));
        assert!(h.neden.contains("satır 42"));
        assert_eq!(h.eylem_etiketi.as_deref(), Some("Karantinaya al"));
        assert!(h.teknik_detay.is_some());
        // Her hata otomatik correlation_id taşır (Gün 4/32).
        assert_eq!(h.correlation_id.kisa().len(), 8);
    }

    #[test]
    fn indeks_yok_indeksle_eylemi() {
        let h = indeks_yok("buyuk.vcf.gz", ".tbi");
        assert!(h.neden.contains(".tbi"));
        assert_eq!(h.eylem_etiketi.as_deref(), Some("İndeks oluştur"));
    }

    #[test]
    fn gpu_yok_bilgi_cpu_yedegi() {
        let h = gpu_yok_bilgisi();
        assert!(h.nasil_cozulur.contains("CPU"));
        assert!(h.ne_oldu.contains("GPU"));
    }

    #[test]
    fn ag_kesintisi_devam_ve_tukenme() {
        // Devam ederken: yeniden deneme mesajı.
        let mut g = Gericekilme::yeni();
        let devam = ag_kesintisi(&g, "NCBI");
        assert!(devam.neden.contains("yeniden denenecek"));
        assert_eq!(devam.eylem_etiketi.as_deref(), Some("Şimdi yeniden dene"));
        // Maks denemeye kadar ilerlet → tükenince çevrimdışı öneri.
        for _ in 0..biocraft_sdk::biocraft_types::esikler::GERICEKILME_MAKS_DENEME {
            g.ilerle();
        }
        assert!(!g.devam_eder_mi());
        let bitti = ag_kesintisi(&g, "NCBI");
        assert!(bitti.nasil_cozulur.contains("Çevrimdışı"));
        assert_eq!(bitti.eylem_etiketi.as_deref(), Some("Çevrimdışı sürdür"));
    }

    #[test]
    fn iz_ile_correlation_id_eslesir() {
        let iz = TraceContext::kok();
        let h = iz_ile(bozuk_dosya("x.vcf", None, "test"), &iz);
        // Hata kimliği iz kimliğiyle aynı → log↔diyalog eşlenir.
        assert_eq!(h.correlation_id, iz.correlation_id());
    }

    #[test]
    fn guvenli_ad_unicode_korur_kontrol_ayiklar() {
        // Türkçe/emoji/CJK korunur.
        assert_eq!(guvenli_ad("örnek_çalışma_基因", 50), "örnek_çalışma_基因");
        assert_eq!(guvenli_ad("veri🧬", 50), "veri🧬");
        // Kontrol karakteri (yeni satır/sekme) ayıklanır.
        assert_eq!(guvenli_ad("a\nb\tc", 50), "abc");
        // Çok uzun ad çok-baytlı karakteri ortadan KESMEDEN kısaltılır (panik/� yok).
        let uzun = "ä".repeat(100);
        let k = guvenli_ad(&uzun, 10);
        assert_eq!(k.chars().count(), 10); // 9 karakter + "…"
        assert!(k.ends_with('…'));
    }
}
