//! ÇE-09 / İP-10 — **Dış sorgu gizlilik sınırı** (MK-41 dış iletişim onayı; MK-42/MK-43 PHI).
//!
//! İki görev:
//! 1. **Şeffaflık (MK-41):** Her dış çağrıdan önce *ne gönderiliyor* (sorgu metni / dizi özeti) ve
//!    *nereye* gittiği özetlenir → kullanıcı onayı bu özet üzerinden alınır.  Onay yoksa istek
//!    **sessizce gitmez** (konektör reddeder).
//! 2. **PHI/hassas engeli (MK-42/43):** [`HassasiyetEtiketi::Phi`] etiketli veri dış sorguya
//!    **çıkamaz** — onaylanmış olsa bile.
//!
//! > **DİKKAT (CLAUDE.md §7):** Gerçek PHI sınırı **çekirdektedir** (İP-10 veri sınıflandırma);
//! > eklenti bunu *aşamaz*.  Bu modül **savunma derinliği + kullanıcı şeffaflığıdır**: dış gönderim
//! > özetini üretir ve eklenti-yerel olarak PHI'yi reddeder, ama nihai güvence çekirdektedir.

use biocraft_sdk::biocraft_types::ErrorReport;

/// Bir verinin hassasiyet sınıfı (İP-10 çekirdek sınıflandırmasına **uyumlu** eklenti-yerel kopya).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HassasiyetEtiketi {
    /// Kamuya açık / yayınlanmış (referans dizi, accession, gen adı) — dış sorgu serbest (onayla).
    Genel,
    /// Hassas (kullanıcının kendi/sınıflandırılmamış verisi) — dış gönderim açık uyarı ister.
    Hassas,
    /// PHI / korunan sağlık verisi — dış sorguya **çıkamaz** (çekirdek de engeller).
    Phi,
}

impl HassasiyetEtiketi {
    /// Bu etiketli veri dış sorguya gönderilebilir mi (PHI hariç hepsi onayla gönderilebilir)?
    pub fn dis_gonderilebilir_mi(&self) -> bool {
        !matches!(self, HassasiyetEtiketi::Phi)
    }
}

/// Dış sorguda gönderilecek veri (onay özeti üretmek + PHI denetimi için).
#[derive(Debug, Clone, Copy)]
pub enum DisVeri<'a> {
    /// Serbest metin sorgu (gen adı / accession / anahtar kelime).
    Metin(&'a str),
    /// Biyolojik dizi (BLAST) — yalnız **uzunluk + baş kısmı** özetlenir (tamamı log'a yazılmaz).
    Dizi(&'a str),
}

impl DisVeri<'_> {
    /// Kullanıcıya gösterilecek "ne gönderiliyor" özeti (dizi tamamı ifşa edilmez).
    pub fn ozet(&self) -> String {
        match self {
            DisVeri::Metin(t) => {
                let kisa = kisalt(t, 120);
                format!("Sorgu metni: {kisa}")
            }
            DisVeri::Dizi(s) => {
                let temiz: String = s.chars().filter(|c| !c.is_whitespace()).collect();
                let bas = kisalt(&temiz, 24);
                format!("Dizi: {} kalıntı (baş: {bas}…)", temiz.len())
            }
        }
    }
}

/// Dış gönderim **onay özeti** — kullanıcıya "ne, nereye" gösterilir (MK-41 şeffaflık).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisGonderimOzeti {
    /// Kaynak/konektör adı (örn. "NCBI nucleotide").
    pub kaynak: String,
    /// Hedef tanımı (örn. "NCBI E-utilities (eutils.ncbi.nlm.nih.gov)").
    pub hedef_aciklama: String,
    /// Gönderilenin insan-okur özeti (dizi tamamı değil).
    pub gonderilen_ozet: String,
    /// Bu gönderim için açık kullanıcı onayı gerekli mi (henüz onaylanmadıysa true)?
    pub onay_gerekli: bool,
}

/// Gizlilik kapısı — dış çağrı izni + şeffaflık özeti üretir (MK-41/42/43).
///
/// `dis_sorgu_onaylandi` oturum/kullanıcı onayını taşır: panel, kullanıcı onay diyaloğunda
/// [`onizleme`](Self::onizleme) özetini gösterip onay alınca [`onayli`](Self::onayli) bir kapı
/// kullanır → konektörler gerçek isteği o zaman gönderir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GizlilikKapisi {
    /// Kullanıcı dış sorguları (bu oturumda) onayladı mı?
    pub dis_sorgu_onaylandi: bool,
}

impl GizlilikKapisi {
    /// Henüz onaylanmamış kapı (her dış çağrı önce onay ister — güvenli varsayılan).
    pub fn yeni() -> Self {
        Self {
            dis_sorgu_onaylandi: false,
        }
    }

    /// Kullanıcının dış sorguları onayladığı kapı.
    pub fn onayli() -> Self {
        Self {
            dis_sorgu_onaylandi: true,
        }
    }

    /// Bir dış gönderimin **önizleme** özeti (saf; onay diyaloğunda gösterilir).
    /// PHI yine `onay_gerekli`'den bağımsız olarak [`dis_gonderim`](Self::dis_gonderim)'de engellenir.
    pub fn onizleme(&self, kaynak: &str, hedef: &str, veri: DisVeri<'_>) -> DisGonderimOzeti {
        DisGonderimOzeti {
            kaynak: kaynak.to_string(),
            hedef_aciklama: hedef.to_string(),
            gonderilen_ozet: veri.ozet(),
            onay_gerekli: !self.dis_sorgu_onaylandi,
        }
    }

    /// Dış gönderimi **denetler** (konektör isteği yollamadan ÖNCE çağırır):
    /// * PHI/hassasiyet sınırını aşan veri → `Err` (gönderilemez; MK-42/43).
    /// * Onay alınmamışsa → `Err` (sessiz gönderim yok; MK-41) — UI önce [`onizleme`](Self::onizleme)
    ///   ile onay almalı.
    /// * Aksi hâlde `Ok(özet)` (kayda/loga yazılacak şeffaf özet).
    pub fn dis_gonderim(
        &self,
        kaynak: &str,
        hedef: &str,
        veri: DisVeri<'_>,
        etiket: HassasiyetEtiketi,
    ) -> Result<DisGonderimOzeti, ErrorReport> {
        // 1) PHI/hassas engeli — onaydan bağımsız (çekirdek de engeller).
        if !etiket.dis_gonderilebilir_mi() {
            return Err(phi_engeli(kaynak));
        }
        // 2) Açık onay yoksa sessizce gönderme.
        let ozet = self.onizleme(kaynak, hedef, veri);
        if ozet.onay_gerekli {
            return Err(onay_gerekli_hatasi(&ozet));
        }
        Ok(ozet)
    }
}

impl Default for GizlilikKapisi {
    fn default() -> Self {
        Self::yeni()
    }
}

fn kisalt(s: &str, azami: usize) -> String {
    if s.chars().count() <= azami {
        s.to_string()
    } else {
        s.chars().take(azami).collect()
    }
}

// ─── Hatalar ─────────────────────────────────────────────────────────────────────

fn phi_engeli(kaynak: &str) -> ErrorReport {
    ErrorReport::new(
        "Hassas/PHI veri dış sorguya gönderilemez",
        format!("'{kaynak}' dış sorgusuna PHI/hassas etiketli veri gönderilmek istendi"),
        "Yalnızca kamuya açık (genel) veriyle dış arama yapın; hasta/hassas dizi cihazdan çıkamaz",
    )
    .with_eylem("İptal")
    .with_teknik_detay("MK-42/MK-43: çekirdek veri sınıflandırma sınırı (İP-10)")
}

fn onay_gerekli_hatasi(ozet: &DisGonderimOzeti) -> ErrorReport {
    ErrorReport::new(
        "Dış sorgu onayı gerekli",
        format!(
            "'{}' kaynağına bir dış istek gönderilecek — {}",
            ozet.kaynak, ozet.gonderilen_ozet
        ),
        "Ne gönderildiğini onayladıktan sonra arama yapın (dış çağrı sessizce gönderilmez)",
    )
    .with_eylem("Onayla ve gönder")
    .with_teknik_detay(format!("hedef={}", ozet.hedef_aciklama))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onaysiz_kapi_genel_metni_reddeder() {
        let kapi = GizlilikKapisi::yeni();
        let r = kapi.dis_gonderim(
            "NCBI nucleotide",
            "eutils",
            DisVeri::Metin("BRCA1"),
            HassasiyetEtiketi::Genel,
        );
        let hata = r.err().unwrap();
        assert_eq!(hata.ne_oldu, "Dış sorgu onayı gerekli");
    }

    #[test]
    fn onayli_kapi_genel_metni_gecer() {
        let kapi = GizlilikKapisi::onayli();
        let ozet = kapi
            .dis_gonderim(
                "NCBI nucleotide",
                "eutils",
                DisVeri::Metin("BRCA1"),
                HassasiyetEtiketi::Genel,
            )
            .unwrap();
        assert!(!ozet.onay_gerekli);
        assert!(ozet.gonderilen_ozet.contains("BRCA1"));
    }

    #[test]
    fn phi_onayli_olsa_bile_engellenir() {
        // Onaylı kapı bile PHI'yi geçirmez (MK-42/43).
        let kapi = GizlilikKapisi::onayli();
        let r = kapi.dis_gonderim(
            "NCBI BLAST",
            "blast",
            DisVeri::Dizi("ACGTACGTACGT"),
            HassasiyetEtiketi::Phi,
        );
        let hata = r.err().unwrap();
        assert_eq!(hata.ne_oldu, "Hassas/PHI veri dış sorguya gönderilemez");
    }

    #[test]
    fn dizi_ozeti_tamami_ifsa_etmez() {
        let uzun = "ACGT".repeat(100); // 400 kalıntı
        let ozet = DisVeri::Dizi(&uzun).ozet();
        assert!(ozet.contains("400 kalıntı"));
        // Tüm dizi değil, yalnız baş kısmı.
        assert!(ozet.len() < uzun.len());
    }

    #[test]
    fn onizleme_onay_durumunu_yansitir() {
        let onaysiz = GizlilikKapisi::yeni().onizleme("k", "h", DisVeri::Metin("x"));
        assert!(onaysiz.onay_gerekli);
        let onayli = GizlilikKapisi::onayli().onizleme("k", "h", DisVeri::Metin("x"));
        assert!(!onayli.onay_gerekli);
    }
}
