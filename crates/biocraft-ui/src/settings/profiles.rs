//! Ayar **profili** dışa/içe aktarma (İP-12) — paylaşım/yedek/yeni cihaz.
//!
//! Bir profil, ayar değerlerinin taşınabilir bir anlık görüntüsüdür (JSON).  İki güvence:
//! - **Hassas alanlar hariç (kabul kriteri):** API anahtarı gibi [`hassas`](super::AyarTanimi::hassas)
//!   ayarlar profile **yazılmaz** (yedek/paylaşım sırlarımızı sızdırmaz).
//! - **İçe aktarımda doğrulama:** her değer [`gecerli_kil`](super::AyarTuru::gecerli_kil)'den geçer;
//!   tanınmayan anahtar **atlanır** (ileri uyum), geçersiz değer güvenli varsayılana iner.
//!
//! Profilde **sürüm alanı** vardır (yeni ayarlar eklenince eski profil sorunsuz okunur — İP-19 göç).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::sections::{AyarDeger, AyarTanimi};

/// Profil şema sürümü.  Yeni ayar/şema değişiminde artar; okuyucu eski sürümü tolere eder.
pub const PROFIL_SURUMU: u32 = 1;

/// Taşınabilir bir ayar profili (dışa/içe aktarılır).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AyarProfili {
    /// Şema sürümü (göç toleransı).
    pub surum: u32,
    /// İnsan-okur profil adı (örn. "Sunum modu", "Laboratuvar PC").
    pub ad: String,
    /// Anahtar → değer (hassas alanlar **dahil değildir**).
    pub degerler: BTreeMap<String, AyarDeger>,
}

/// İçe aktarım sonucu — şeffaflık için uygulanan/atlanan anahtarlar.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IceAktarRapor {
    /// Doğrulanıp uygulanmaya hazır değerler.
    pub uygulanan: BTreeMap<String, AyarDeger>,
    /// Tanınmayan (bu sürümde olmayan) ve atlanan anahtarlar.
    pub bilinmeyen: Vec<String>,
    /// Profil bir hassas alan içeriyordu da güvenlik gereği atlandıysa o anahtarlar.
    pub atlanan_hassas: Vec<String>,
}

impl AyarProfili {
    /// Verilen değerlerden bir profil üretir; **hassas** ayarları dışarıda bırakır.
    ///
    /// `degerler`: bir katmanın (örn. kullanıcı) o anki değerleri.  `tanimlar`: hangi anahtarın
    /// hassas olduğunu bilmek için kayıt.  Tanımı olmayan anahtar dışa aktarılmaz (bilinmeyen).
    pub fn disa_aktar(
        ad: impl Into<String>,
        degerler: &BTreeMap<String, AyarDeger>,
        tanimlar: &[AyarTanimi],
    ) -> Self {
        let mut cikti = BTreeMap::new();
        for (anahtar, deger) in degerler {
            if let Some(t) = tanimlar.iter().find(|t| &t.anahtar == anahtar) {
                if t.hassas {
                    continue; // hassas → profile yazma.
                }
                cikti.insert(anahtar.clone(), deger.clone());
            }
            // Tanımı olmayan (eski/eklenti kaldırılmış) anahtar dışa aktarılmaz.
        }
        Self {
            surum: PROFIL_SURUMU,
            ad: ad.into(),
            degerler: cikti,
        }
    }

    /// Profili okunaklı JSON'a yazar.
    pub fn json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("Profil yazılamadı: {e}"))
    }

    /// JSON'dan bir profil okur (biçim hatasını anlaşılır metne çevirir).
    pub fn jsondan(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| format!("Profil okunamadı (geçersiz biçim): {e}"))
    }

    /// Profili doğrular: her değeri tipine göre güvenli kılar, tanınmayanı atlar, **hassas**
    /// anahtarları (profilde yanlışlıkla bulunsa bile) güvenlik gereği uygulamaz.
    ///
    /// Döndürdüğü [`IceAktarRapor`] çağırana hangi değerlerin uygulanacağını + neyin atlandığını verir.
    pub fn dogrula_ve_coz(&self, tanimlar: &[AyarTanimi]) -> IceAktarRapor {
        let mut rapor = IceAktarRapor::default();
        for (anahtar, deger) in &self.degerler {
            match tanimlar.iter().find(|t| &t.anahtar == anahtar) {
                Some(t) if t.hassas => rapor.atlanan_hassas.push(anahtar.clone()),
                Some(t) => {
                    rapor
                        .uygulanan
                        .insert(anahtar.clone(), t.gecerli_kil(deger));
                }
                None => rapor.bilinmeyen.push(anahtar.clone()),
            }
        }
        rapor
    }
}

#[cfg(test)]
mod tests {
    use super::super::sections::{yerlesik_tanimlar, AyarDeger};
    use super::*;

    fn ornek_degerler() -> BTreeMap<String, AyarDeger> {
        let mut m = BTreeMap::new();
        m.insert("gorunum.tema".to_string(), AyarDeger::Secim("acik".into()));
        m.insert("editor.sekme_genisligi".to_string(), AyarDeger::TamSayi(2));
        // Hassas: profile sızmamalı.
        m.insert(
            "ai.api_anahtari".to_string(),
            AyarDeger::Metin("gizli-sır".into()),
        );
        m
    }

    #[test]
    fn disa_aktarim_hassasi_dislar() {
        // Kabul kriteri: profil dışa aktarma anahtar (hassas) hariç çalışır.
        let t = yerlesik_tanimlar();
        let p = AyarProfili::disa_aktar("Yedek", &ornek_degerler(), &t);
        assert!(p.degerler.contains_key("gorunum.tema"));
        assert!(p.degerler.contains_key("editor.sekme_genisligi"));
        assert!(
            !p.degerler.contains_key("ai.api_anahtari"),
            "hassas API anahtarı profile yazılmamalı"
        );
        // JSON'da da sır geçmemeli.
        let j = p.json().unwrap();
        assert!(!j.contains("gizli-sır"), "JSON sır içermemeli");
    }

    #[test]
    fn json_gidis_donus() {
        let t = yerlesik_tanimlar();
        let p = AyarProfili::disa_aktar("Sunum", &ornek_degerler(), &t);
        let j = p.json().unwrap();
        let geri = AyarProfili::jsondan(&j).unwrap();
        assert_eq!(p, geri);
    }

    #[test]
    fn ice_aktarim_dogrular_ve_bilinmeyeni_atlar() {
        let t = yerlesik_tanimlar();
        let mut p = AyarProfili {
            surum: PROFIL_SURUMU,
            ad: "Test".into(),
            degerler: BTreeMap::new(),
        };
        // Geçerli.
        p.degerler
            .insert("gorunum.tema".into(), AyarDeger::Secim("acik".into()));
        // Aralık dışı → sıkıştırılır.
        p.degerler
            .insert("editor.sekme_genisligi".into(), AyarDeger::TamSayi(999));
        // Tanınmayan → atlanır.
        p.degerler
            .insert("yok.olan.ayar".into(), AyarDeger::Mantik(true));

        let r = p.dogrula_ve_coz(&t);
        assert_eq!(
            r.uygulanan.get("gorunum.tema"),
            Some(&AyarDeger::Secim("acik".into()))
        );
        // 8'e sıkıştırılmış olmalı (editör sekme genişliği max 8).
        assert_eq!(
            r.uygulanan.get("editor.sekme_genisligi"),
            Some(&AyarDeger::TamSayi(8))
        );
        assert!(r.bilinmeyen.contains(&"yok.olan.ayar".to_string()));
    }

    #[test]
    fn ice_aktarim_profildeki_hassasi_uygulamaz() {
        let t = yerlesik_tanimlar();
        // Elle/eski bir profilde hassas alan bulunsa bile uygulanmamalı.
        let p = AyarProfili {
            surum: PROFIL_SURUMU,
            ad: "Eski".into(),
            degerler: {
                let mut m = BTreeMap::new();
                m.insert("ai.api_anahtari".into(), AyarDeger::Metin("sızdı".into()));
                m
            },
        };
        let r = p.dogrula_ve_coz(&t);
        assert!(r.uygulanan.is_empty());
        assert!(r.atlanan_hassas.contains(&"ai.api_anahtari".to_string()));
    }
}
