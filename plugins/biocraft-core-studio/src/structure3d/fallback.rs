//! ÇE-07 — **GPU yedeği + ölçeklenir kalite**: GPU yokluğunda/çökmesinde (TDR/DeviceLost, Gün 5)
//! CPU yedeği; büyük yapıda sadeleştirme + uyarı (TDA 1, 11).
//!
//! Eklenti GPU'ya doğrudan dokunamaz (MK-17); **karar** mantığı burada saf/birim-testlenebilir
//! durur, motor (host) GPU durumunu besler.  GPU kaybında host [`GpuDurumu::Yok`] verir → CPU
//! yedeği projeksiyonu ([`super::render::projeksiyon`]) devreye girer; yapı asla kaybolmaz.

/// GPU/render yolu durumu — host (motor) tarafından beslenir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuDurumu {
    /// Çalışır GPU var (wgpu cihazı sağlıklı).
    Var,
    /// GPU yok ya da cihaz kaybedildi (TDR/DeviceLost) → CPU yedeği.
    Yok,
}

impl GpuDurumu {
    /// Cihaz kaybı (TDR/DeviceLost) sonrası: GPU'yu kayıp say → bir sonraki kare CPU yedeğiyle
    /// çizilir (Gün 5 kurtarma kancası).  Host yeniden cihaz kurabilirse tekrar [`GpuDurumu::Var`]
    /// verir.
    pub fn cihaz_kaybi() -> Self {
        GpuDurumu::Yok
    }
}

/// Render kalite seviyesi — atom sayısı + GPU durumuna göre seçilir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KaliteSeviyesi {
    /// Tam detay (yüksek tesselasyonlu küreler; tüm gösterim modları).
    Yuksek,
    /// Orta detay (azaltılmış tesselasyon).
    Orta,
    /// Düşük detay — yalnız omurga izi (büyük yapı / CPU yedeği): akıcılık korunur.
    Dusuk,
}

/// GPU ile sadeleştirmeye geçiş eşiği (atom).
const ESIK_ORTA: usize = 30_000;
/// Düşük detaya (omurga) zorlama eşiği (atom).
const ESIK_DUSUK: usize = 120_000;
/// CPU yedeğinde omurgaya zorlama eşiği (atom) — yazılım rasteri daha yavaş.
const ESIK_CPU_OMURGA: usize = 8_000;

/// Atom sayısı + GPU durumundan kalite seviyesini seçer (ölçeklenir performans — MK-04).
pub fn karar(atom_sayisi: usize, gpu: GpuDurumu) -> KaliteSeviyesi {
    match gpu {
        GpuDurumu::Yok => {
            if atom_sayisi > ESIK_CPU_OMURGA {
                KaliteSeviyesi::Dusuk
            } else {
                KaliteSeviyesi::Orta
            }
        }
        GpuDurumu::Var => {
            if atom_sayisi > ESIK_DUSUK {
                KaliteSeviyesi::Dusuk
            } else if atom_sayisi > ESIK_ORTA {
                KaliteSeviyesi::Orta
            } else {
                KaliteSeviyesi::Yuksek
            }
        }
    }
}

/// Bu kalitede tüm gösterim yalnız omurga izine sadeleşmeli mi? (Düşük → evet.)
pub fn yalniz_omurga_mu(kalite: KaliteSeviyesi) -> bool {
    kalite == KaliteSeviyesi::Dusuk
}

/// GPU küre tesselasyon bölünmesi (motorun instanced küre LOD'u için ipucu).
pub fn kure_bolunme(kalite: KaliteSeviyesi) -> u32 {
    match kalite {
        KaliteSeviyesi::Yuksek => 24,
        KaliteSeviyesi::Orta => 12,
        KaliteSeviyesi::Dusuk => 6,
    }
}

/// Kullanıcıya gösterilecek uyarı (varsa) — GPU yok / büyük yapı sadeleştirme (TDA 1, 11).
pub fn uyari(atom_sayisi: usize, gpu: GpuDurumu, kalite: KaliteSeviyesi) -> Option<String> {
    let mut mesajlar = Vec::new();
    if gpu == GpuDurumu::Yok {
        mesajlar.push(
            "GPU bulunamadı — yazılım (CPU) yedeğiyle çiziliyor (yavaş olabilir).".to_string(),
        );
    }
    if kalite == KaliteSeviyesi::Dusuk {
        mesajlar.push(format!(
            "Büyük yapı ({atom_sayisi} atom) — akıcılık için sadeleştirilmiş omurga gösterimi."
        ));
    } else if kalite == KaliteSeviyesi::Orta && atom_sayisi > ESIK_ORTA {
        // Yalnız gerçekten büyük yapıda "detay azaltıldı" uyar (küçük yapı CPU'da tam çizilir).
        mesajlar.push(format!(
            "Orta-büyük yapı ({atom_sayisi} atom) — detay bir miktar azaltıldı."
        ));
    }
    if mesajlar.is_empty() {
        None
    } else {
        Some(mesajlar.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_var_kucuk_yapi_yuksek() {
        assert_eq!(karar(2_000, GpuDurumu::Var), KaliteSeviyesi::Yuksek);
        assert!(uyari(2_000, GpuDurumu::Var, KaliteSeviyesi::Yuksek).is_none());
    }

    #[test]
    fn gpu_var_buyuk_yapi_sadeleserek_omurga() {
        let k = karar(200_000, GpuDurumu::Var);
        assert_eq!(k, KaliteSeviyesi::Dusuk);
        assert!(yalniz_omurga_mu(k));
        assert!(uyari(200_000, GpuDurumu::Var, k)
            .unwrap()
            .contains("omurga"));
    }

    #[test]
    fn gpu_yok_cpu_yedegi_uyarir() {
        let k = karar(1_000, GpuDurumu::Yok);
        assert_eq!(k, KaliteSeviyesi::Orta); // küçük yapı CPU'da tam çizilir
        let u = uyari(1_000, GpuDurumu::Yok, k).unwrap();
        assert!(u.contains("CPU") || u.contains("yedek"));
        // CPU'da büyükçe yapı → omurga.
        assert_eq!(karar(50_000, GpuDurumu::Yok), KaliteSeviyesi::Dusuk);
    }

    #[test]
    fn cihaz_kaybi_gpu_yok_yapar() {
        assert_eq!(GpuDurumu::cihaz_kaybi(), GpuDurumu::Yok);
    }

    #[test]
    fn kure_bolunme_kaliteyle_azalir() {
        assert!(kure_bolunme(KaliteSeviyesi::Yuksek) > kure_bolunme(KaliteSeviyesi::Dusuk));
    }
}
