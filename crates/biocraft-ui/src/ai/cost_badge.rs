//! YZ-06 — **Token / maliyet rozeti** (egui).  Maliyet şeffaflığı = güvenin parçası.
//!
//! Anlık (son çağrı) + oturum toplamı token/bedel; kota durumuna göre renk (token'dan — MK-52).
//! Yerel sağlayıcıda bedel "yerel (bedelsiz)" diye net gösterilir; sürpriz fatura yok.
// MK-52: renkler token'dan.  YZ-06: gizli maliyet yok.

use biocraft_ai_surface::{Kota, KotaDurumu, MaliyetSayaci};

use crate::i18n::Dil;
use crate::tokens::Tokenlar;

/// Maliyet/token rozetini satır içi çizer (panel başlığında / durum şeridinde kullanılır).
///
/// `token_goster` / `maliyet_goster` ayarları kapalıysa ilgili kısım gizlenir.
pub fn maliyet_rozeti_ciz(
    ui: &mut egui::Ui,
    sayac: &MaliyetSayaci,
    kota: &Kota,
    token_goster: bool,
    maliyet_goster: bool,
    dil: Dil,
    tok: &Tokenlar,
) {
    if !token_goster && !maliyet_goster {
        return;
    }
    let tr = matches!(dil, Dil::Tr);

    // Kota durumuna göre renk: normal → soluk, yaklaşıyor → uyarı, aşıldı → hata.
    let (renk, durum_metni) = match kota.durum(sayac) {
        KotaDurumu::Normal => (tok.renk.metin_soluk, None),
        KotaDurumu::Yaklasiyor(oran) => (
            tok.renk.uyari,
            Some(if tr {
                format!("kota %{:.0}", oran * 100.0)
            } else {
                format!("quota {:.0}%", oran * 100.0)
            }),
        ),
        KotaDurumu::Asildi => (
            tok.renk.hata,
            Some(if tr {
                "kota aşıldı".to_string()
            } else {
                "quota exceeded".to_string()
            }),
        ),
    };

    ui.horizontal(|ui| {
        if token_goster {
            let (cagri, jeton) = if tr {
                ("çağrı", "jeton")
            } else {
                ("calls", "tokens")
            };
            ui.label(
                egui::RichText::new(format!(
                    "🎟 {} {jeton} · {} {cagri}",
                    sayac.oturum_jeton, sayac.cagri_sayisi
                ))
                .small()
                .color(renk),
            );
        }
        if maliyet_goster {
            if token_goster {
                ui.separator();
            }
            let bedel = if sayac.oturum_bedel > 0.0 {
                format!("💲 {:.4} USD", sayac.oturum_bedel)
            } else if tr {
                "💲 yerel (bedelsiz)".to_string()
            } else {
                "💲 local (free)".to_string()
            };
            ui.label(egui::RichText::new(bedel).small().color(renk));
        }
        if let Some(d) = durum_metni {
            ui.separator();
            ui.label(egui::RichText::new(format!("⚠ {d}")).small().color(renk));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use biocraft_ai_surface::Maliyet;

    #[test]
    fn rozet_headless_panik_yok() {
        let ctx = egui::Context::default();
        let mut sayac = MaliyetSayaci::yeni();
        sayac.ekle(Maliyet::yerel(120));
        let kota = Kota {
            jeton_limiti: Some(100),
            bedel_limiti: None,
        };
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                // Kota aşıldı yolu (renk = hata) dâhil panik olmamalı.
                maliyet_rozeti_ciz(ui, &sayac, &kota, true, true, Dil::Tr, &Tokenlar::acik());
            });
        });
    }
}
