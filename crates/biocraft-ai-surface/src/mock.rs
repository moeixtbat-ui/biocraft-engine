//! **Demo/echo sağlayıcı** — sözleşmenin uçtan uca çalıştığını göstermek için (gerçek AI DEĞİL).
//!
//! [`EchoSaglayici`] [`Provider`] sözleşmesini uygular: sorguyu geri yansıtır ve **tam zengin
//! çıktıyı** (kaynak + güven + "doğrulanmalı" uyarısı + token/maliyet) kurar; akışta yanıtı
//! sözcük sözcük yayınlayıp her sözcükte iptali denetler.  Bu, MVP'de bağlı motor olmadan
//! pipeline'ı kanıtlar.  **Dürüstlük (MK-48):** kimliği açıkça "gerçek AI değil" der; gerçek
//! uygulamada bu sağlayıcı kaydolmaz → yüzey "yapılandırılmadı" gösterir.  Demo/test/örnek
//! akışlarında kaydolur.
// MK-48: sahte zekâ yok — echo açıkça demo etiketli.

use biocraft_types::ErrorReport;

use crate::context::AiBaglam;
use crate::contract::{AiCikti, EylemOnerisi, EylemTuru, Guven, GuvenSeviyesi, Kaynak, Kullanim};
use crate::provider::{
    AkisOlay, IptalBayragi, Provider, SaglayiciKimlik, SaglayiciTuru, SaglayiciYetenekleri,
};

/// Sorguyu geri yansıtan demo sağlayıcı (gerçek AI değil).
pub struct EchoSaglayici {
    kimlik: SaglayiciKimlik,
}

impl Default for EchoSaglayici {
    fn default() -> Self {
        Self::yeni()
    }
}

impl EchoSaglayici {
    /// Yerel türünde bir demo echo sağlayıcı kurar (bedel=0).
    pub fn yeni() -> Self {
        Self {
            kimlik: SaglayiciKimlik {
                kimlik: "biocraft.demo.echo".to_string(),
                ad: "Demo Echo (gerçek AI değil)".to_string(),
                tur: SaglayiciTuru::Yerel,
                model: Some("echo-1".to_string()),
                aciklama:
                    "Sözleşmeyi uçtan uca gösteren demo sağlayıcı; gerçek bir AI motoru değildir."
                        .to_string(),
            },
        }
    }

    /// Belirtilen türde demo sağlayıcı (ör. çıkış kapısını denemek için `Bulut`).
    pub fn tur_ile(tur: SaglayiciTuru) -> Self {
        let mut s = Self::yeni();
        s.kimlik.tur = tur;
        s
    }

    /// Echo'nun ürettiği yanıt metni (saf — testlerde de kullanılır).
    fn yanit_metni(baglam: &AiBaglam) -> String {
        if baglam.sorgu.trim().is_empty() {
            "[demo/echo] Bir soru yazın; bu sağlayıcı yalnızca yüzeyi gösterir (gerçek AI değil)."
                .to_string()
        } else {
            format!(
                "[demo/echo] Sorunuzu aldım: \"{}\". Bu bir demo yanıttır; gerçek bir AI motoru \
                 bağlandığında burada anlamlı bir öneri görürsünüz.",
                baglam.sorgu.trim()
            )
        }
    }
}

impl Provider for EchoSaglayici {
    fn kimlik(&self) -> &SaglayiciKimlik {
        &self.kimlik
    }

    fn yetenekler(&self) -> SaglayiciYetenekleri {
        SaglayiciYetenekleri {
            akis: true,
            gomme: false,
            goru: false,
            maks_baglam_jeton: Some(4096),
        }
    }

    fn uret(&self, baglam: &AiBaglam) -> Result<AiCikti, ErrorReport> {
        let metin = Self::yanit_metni(baglam);
        let girdi = baglam.tahmini_girdi_jeton();
        let cikti_jeton = (metin.len() / 4).max(1) as u64;

        let mut c = AiCikti::oneri(metin);
        c.oneriler = vec![
            "Gerçek bir AI sağlayıcı/eklenti bağlayın (İP-14 sonrası).".to_string(),
            "Çıktıyı her zaman doğrulayın; bu yalnızca bir yüzey demosudur.".to_string(),
        ];
        c.eylem_onerileri = vec![EylemOnerisi::yeni(
            "Bu metni koda not olarak ekle (onaylı)",
            EylemTuru::KodEkle,
        )];
        c.kaynaklar = vec![Kaynak {
            baslik: "BioCraft Engine — AI yüzey demosu".to_string(),
            url: None,
            atif: Some("Demo/echo sağlayıcı; bilimsel kaynak değildir.".to_string()),
        }];
        c.guven = Guven {
            seviye: GuvenSeviyesi::Bilinmiyor,
            aciklama: Some("Demo sağlayıcı — güven hesaplanmadı (gerçek motor yok).".to_string()),
            coklu_ai_uyumu: None,
        };
        c.kullanim = Kullanim::yeni(girdi, cikti_jeton);
        Ok(c)
    }

    fn akis(&self, baglam: &AiBaglam, iptal: &IptalBayragi, gonder: &mut dyn FnMut(AkisOlay)) {
        // Tam çıktıyı önceden kur (kaynak/güven/maliyet için), metni sözcük sözcük yayınla.
        let cikti = match self.uret(baglam) {
            Ok(c) => c,
            Err(e) => {
                gonder(AkisOlay::Hata(Box::new(e)));
                return;
            }
        };

        let mut yayinlanan = String::new();
        for (i, sozcuk) in cikti.metin.split_inclusive(' ').enumerate() {
            // Her parçada iptal denetlenir (MK-11) — "Durdur" anında etki eder.
            if iptal.iptal_mi() {
                gonder(AkisOlay::Durduruldu);
                return;
            }
            // Demo'da gerçek bir gecikme YOK (UI thread'i bloklanmaz); gerçek akış motorla gelir.
            yayinlanan.push_str(sozcuk);
            gonder(AkisOlay::Parca(sozcuk.to_string()));
            let _ = i;
        }

        // Akan metni nihai çıktının metniyle aynı tut (tutarlılık).
        let mut son = cikti;
        son.metin = yayinlanan.trim_end().to_string();
        gonder(AkisOlay::Tamamlandi(Box::new(son)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_tam_sema_uretir() {
        // MK-47: çıktı kaynak + güven + doğrulama + maliyet taşır.
        let s = EchoSaglayici::yeni();
        let c = s.uret(&AiBaglam::sorgudan("varyantı yorumla")).unwrap();
        assert!(c.metin.contains("varyantı yorumla"));
        assert!(!c.kaynaklar.is_empty(), "kaynak olmalı");
        assert!(!c.dogrulama_uyarisi.is_empty(), "doğrulama uyarısı olmalı");
        assert!(c.klinik_degil, "klinik değil etiketi olmalı (MK-49)");
        assert!(c.kullanim.toplam() > 0, "token sayılmalı");
    }

    #[test]
    fn akis_iptal_edilince_durdurulur() {
        let s = EchoSaglayici::yeni();
        let iptal = IptalBayragi::yeni();
        iptal.iptal_et(); // baştan iptal
        let mut olaylar = Vec::new();
        s.akis(
            &AiBaglam::sorgudan("uzun bir soru cümlesi"),
            &iptal,
            &mut |o| olaylar.push(o),
        );
        assert!(
            matches!(olaylar.first(), Some(AkisOlay::Durduruldu)),
            "iptal edilince akış durmalı"
        );
    }

    #[test]
    fn akis_tamamlanir_ve_son_cikti_gelir() {
        let s = EchoSaglayici::yeni();
        let iptal = IptalBayragi::yeni();
        let mut tamam = false;
        let mut parca_sayisi = 0;
        s.akis(
            &AiBaglam::sorgudan("merhaba dünya"),
            &iptal,
            &mut |o| match o {
                AkisOlay::Parca(_) => parca_sayisi += 1,
                AkisOlay::Tamamlandi(_) => tamam = true,
                _ => {}
            },
        );
        assert!(parca_sayisi >= 2, "metin parça parça yayınlanmalı");
        assert!(tamam, "akış tam çıktıyla bitmeli");
    }

    #[test]
    fn yerel_echo_bedelsiz() {
        let s = EchoSaglayici::yeni();
        let m = s.maliyet(&Kullanim::yeni(10, 20));
        assert!(m.yerel && m.bedel == 0.0, "yerel echo bedeli=0");
    }
}
