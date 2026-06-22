//! YZ-06 — **Maliyet, token, kota ve Bio-kredi kancası.**
//!
//! Maliyet şeffaflığı güvenin parçasıdır: hiçbir gizli bedel yok, her dış çağrının bedeli
//! önceden tahmin + sonradan gerçek (0-AI.5).  **Yerel AI bedeli=0** (yalnızca kaynak).  MVP'de
//! gösterge + oturum toplamı + kota uyarısı + Bio-kredi **kancası** vardır; gerçek hesap motorla
//! (YZ-03) ve gerçek ödeme `Hukuk-ve-Operasyon.md` ile gelir.
// MK-47/0-AI.5: maliyet şeffaf; sürpriz fatura yok.

use serde::{Deserialize, Serialize};

/// Tek bir çağrının maliyeti.  Yerel sağlayıcıda `bedel = 0.0`, yalnızca jeton sayılır.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Maliyet {
    /// Toplam jeton.
    pub jeton: u64,
    /// Tahmini/gerçek parasal bedel (yerelde 0).
    pub bedel: f64,
    /// Para birimi kodu (ör. "USD"); yerelde/bilinmeyende boş.
    pub para_birimi: String,
    /// Yerel sağlayıcı mı? (bedel=0 net gösterimi için.)
    pub yerel: bool,
}

impl Maliyet {
    /// Yerel sağlayıcı maliyeti — bedel her zaman 0, yalnızca kaynak (jeton).
    pub fn yerel(jeton: u64) -> Self {
        Self {
            jeton,
            bedel: 0.0,
            para_birimi: String::new(),
            yerel: true,
        }
    }

    /// Bulut/dış sağlayıcı maliyeti: jeton × birim-bedel.
    pub fn bulut(jeton: u64, jeton_basi_bedel: f64, para_birimi: impl Into<String>) -> Self {
        Self {
            jeton,
            bedel: jeton as f64 * jeton_basi_bedel,
            para_birimi: para_birimi.into(),
            yerel: false,
        }
    }

    /// Bedeli/jetonu bilinmeyen (sağlayıcı bildirmedi) durum — yalnızca jeton tutulur.
    pub fn yok(jeton: u64) -> Self {
        Self {
            jeton,
            bedel: 0.0,
            para_birimi: String::new(),
            yerel: false,
        }
    }

    /// İnsan-okunur kısa gösterim ("1.234 jeton · $0.0021" veya "1.234 jeton · yerel (0₺)").
    pub fn goster(&self, tr: bool) -> String {
        let jeton_etiket = if tr { "jeton" } else { "tokens" };
        if self.yerel {
            let yerel = if tr {
                "yerel (bedelsiz)"
            } else {
                "local (free)"
            };
            format!("{} {jeton_etiket} · {yerel}", self.jeton)
        } else if self.bedel > 0.0 {
            format!(
                "{} {jeton_etiket} · {:.4} {}",
                self.jeton, self.bedel, self.para_birimi
            )
        } else {
            format!("{} {jeton_etiket}", self.jeton)
        }
    }
}

/// **Oturum maliyet sayacı** — anlık + birikimli; UI durum çubuğunda gösterir (YZ-06).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MaliyetSayaci {
    /// Bu oturumdaki toplam jeton.
    pub oturum_jeton: u64,
    /// Bu oturumdaki toplam bedel.
    pub oturum_bedel: f64,
    /// Çağrı sayısı.
    pub cagri_sayisi: u64,
    /// Son çağrının maliyeti (anlık gösterge).
    pub son: Option<Maliyet>,
}

impl MaliyetSayaci {
    /// Yeni (sıfır) sayaç.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir çağrının maliyetini ekler.
    pub fn ekle(&mut self, m: Maliyet) {
        self.oturum_jeton = self.oturum_jeton.saturating_add(m.jeton);
        self.oturum_bedel += m.bedel;
        self.cagri_sayisi = self.cagri_sayisi.saturating_add(1);
        self.son = Some(m);
    }

    /// Oturumu sıfırlar.
    pub fn sifirla(&mut self) {
        *self = Self::default();
    }

    /// Oturum özeti ("3 çağrı · 1.234 jeton · $0.0042").
    pub fn ozet(&self, tr: bool) -> String {
        let (cagri, jeton) = if tr {
            ("çağrı", "jeton")
        } else {
            ("calls", "tokens")
        };
        if self.oturum_bedel > 0.0 {
            format!(
                "{} {cagri} · {} {jeton} · {:.4} USD",
                self.cagri_sayisi, self.oturum_jeton, self.oturum_bedel
            )
        } else {
            format!(
                "{} {cagri} · {} {jeton}",
                self.cagri_sayisi, self.oturum_jeton
            )
        }
    }
}

/// Kota / limit (YZ-06).  `None` = sınırsız.  Limit aşımında UI uyarır (gizli bedel yok).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Kota {
    /// Jeton limiti (oturum başına).
    pub jeton_limiti: Option<u64>,
    /// Bedel limiti (oturum başına).
    pub bedel_limiti: Option<f64>,
}

/// Kota durumu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KotaDurumu {
    /// Limit yok veya bol.
    Normal,
    /// Limite yaklaşıldı (>= %80) — `oran` doluluk.
    Yaklasiyor(f32),
    /// Limit aşıldı.
    Asildi,
}

impl Kota {
    /// Sayaca göre kota durumunu döndürür (en kısıtlayıcı limit kazanır).
    pub fn durum(&self, sayac: &MaliyetSayaci) -> KotaDurumu {
        let mut en_kotu = KotaDurumu::Normal;
        let mut en_yuksek_oran = 0.0_f32;

        let mut degerlendir = |kullanilan: f64, limit: Option<f64>| {
            if let Some(l) = limit {
                if l <= 0.0 {
                    return;
                }
                let oran = (kullanilan / l) as f32;
                if oran >= 1.0 {
                    en_kotu = KotaDurumu::Asildi;
                } else if oran >= 0.8 && !matches!(en_kotu, KotaDurumu::Asildi) {
                    en_yuksek_oran = en_yuksek_oran.max(oran);
                    en_kotu = KotaDurumu::Yaklasiyor(en_yuksek_oran);
                }
            }
        };

        degerlendir(
            sayac.oturum_jeton as f64,
            self.jeton_limiti.map(|v| v as f64),
        );
        degerlendir(sayac.oturum_bedel, self.bedel_limiti);
        en_kotu
    }
}

/// **Bio-kredi kancası** (YZ-06) — MVP'de yalnızca tasarımsal yer tutucu.
///
/// Gerçek Bio-kredi ekonomisi/ödeme akışı `Hukuk-ve-Operasyon.md` + `MVP-sonrasi.md` §2.2 ile
/// gelir.  Bu tip, gelecekteki bağlanışın sözleşme noktasını sabitler: bir maliyet Bio-krediye
/// **çevrilebilir** olmalıdır.  MVP'de çevrim yapılmaz (`None`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BioKrediKanca {
    /// 1 birim bedel kaç Bio-krediye denk? `None` = bağlanmadı (MVP).
    pub bedel_basi_kredi: Option<f64>,
}

impl BioKrediKanca {
    /// Bir bedeli Bio-krediye çevirir; kanca bağlı değilse `None` (MVP durumu).
    pub fn krediye_cevir(&self, bedel: f64) -> Option<f64> {
        self.bedel_basi_kredi.map(|k| bedel * k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yerel_maliyet_bedelsiz() {
        let m = Maliyet::yerel(500);
        assert!(m.yerel);
        assert_eq!(m.bedel, 0.0, "yerel AI bedeli=0 (YZ-06)");
    }

    #[test]
    fn sayac_birikir() {
        let mut s = MaliyetSayaci::yeni();
        s.ekle(Maliyet::bulut(100, 0.0001, "USD"));
        s.ekle(Maliyet::bulut(200, 0.0001, "USD"));
        assert_eq!(s.oturum_jeton, 300);
        assert_eq!(s.cagri_sayisi, 2);
        assert!((s.oturum_bedel - 0.03).abs() < 1e-9);
    }

    #[test]
    fn kota_asimi_ve_yaklasma() {
        let k = Kota {
            jeton_limiti: Some(100),
            bedel_limiti: None,
        };
        let mut s = MaliyetSayaci::yeni();
        s.ekle(Maliyet::yok(85));
        assert!(
            matches!(k.durum(&s), KotaDurumu::Yaklasiyor(_)),
            "%85 → yaklaşıyor"
        );
        s.ekle(Maliyet::yok(50));
        assert!(
            matches!(k.durum(&s), KotaDurumu::Asildi),
            "135/100 → aşıldı"
        );
    }

    #[test]
    fn biokredi_kancasi_mvp_de_bagli_degil() {
        let kanca = BioKrediKanca::default();
        assert_eq!(
            kanca.krediye_cevir(1.0),
            None,
            "MVP'de Bio-kredi bağlı değil"
        );
    }
}
