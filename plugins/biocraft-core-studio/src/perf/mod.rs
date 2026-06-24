//! ÇE-12 — **Performans, erişilebilirlik ve doğruluk** yardımcıları (çapraz güvence).
//!
//! Bu modül çekirdek eklentiyi "cilalar": diğer tüm modüllerin (genom tarayıcı, varyant, 3B,
//! veritabanı, dışa aktarma) **ölçülebilir hız**, **erişilebilirlik** ve **bilimsel doğruluk**
//! güvencesine yaslandığı ortak katmandır.  Hepsi **render-bağımsız + saf + birim-testlenir**
//! (MK-17; motor/GPU/egui gerekmez):
//!
//! * [`accessibility`] — renk körü dostu palet (Okabe-Ito) + dichromat simülasyonu + WCAG kontrast
//!   + şekil/desen ipucu + yazı ölçeği + yüksek kontrast (MK-52).
//! * [`keyboard`] — tam klavye gezinme (tab halkası) + ekran okuyucu etiketleri/roller +
//!   erişilebilirlik denetimi.
//! * [`budget`] — 60 FPS kare bütçesi + uyarlamalı detay (LOD seyreltme hedefi) + performans
//!   şeffaflık göstergesi (MK-04).
//! * [`correctness`] — referans araç (IGV/samtools/bcftools) parametre/koordinat eşitleme + sayısal
//!   tolerans + golden rapor (MK-58).
//! * [`edge`] — sınır durum (boş/tek/büyük/bozuk/indeks-yok/GPU-yok/ağ/Unicode) sınıflama + standart
//!   hata şeması + correlation_id (Bölüm 0.12).

use biocraft_sdk::{Aktivasyon, YetkiKapisi};

pub mod accessibility;
pub mod budget;
pub mod correctness;
pub mod edge;
pub mod keyboard;

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-12";

/// Erişilebilirlik + performans ayar sayfasının kimliği.
pub const AYAR_KIMLIK: &str = "biocraft.core.studio.erisilebilirlik";

/// Alt-modülün UI/komut kayıtları.
///
/// Erişilebilirlik/performans **yetki gerektirmez** (saf görünüm tercihleri); bu yüzden `_yetkiler`
/// kullanılmaz ama imza ortak kayıt sözleşmesine uyar (her alt-modül `kayitlar(&YetkiKapisi)`).
/// Bir **ayar sayfası** (renk körü modu, yüksek kontrast, yazı ölçeği) + iki hızlı **komut**
/// (yüksek kontrast / performans göstergesi geçişi) sunar.
pub fn kayitlar(_yetkiler: &YetkiKapisi) -> Aktivasyon {
    let mut akt = Aktivasyon::yeni();
    akt.ayar(AYAR_KIMLIK, "Erişilebilirlik ve Performans")
        .komut(
            "biocraft.core.studio.yuksek_kontrast",
            "Erişilebilirlik: Yüksek Kontrastı Aç/Kapat",
        )
        .komut(
            "biocraft.core.studio.performans_gosterge",
            "Performans: FPS Göstergesini Aç/Kapat",
        );
    akt
}

#[cfg(test)]
mod tests {
    use super::*;
    use biocraft_sdk::ui::UiUzantiTuru;

    #[test]
    fn kayitlar_ayar_ve_komut_sunar() {
        let akt = kayitlar(&YetkiKapisi::yeni([]));
        // Erişilebilirlik ayar sayfası + iki hızlı komut (yetki gerektirmez).
        assert_eq!(akt.ui_say(UiUzantiTuru::Ayar), 1);
        assert!(akt.ui_say(UiUzantiTuru::Komut) >= 2);
        assert!(akt
            .ui_turden(UiUzantiTuru::Ayar)
            .any(|k| k.kimlik == AYAR_KIMLIK));
    }
}
