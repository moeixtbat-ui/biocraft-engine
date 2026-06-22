//! Doğrulama **rozeti + dürüst etiketler** + "doğrulama hattı" açıklaması (İP-18, MK-16/MK-47/MK-48).
//!
//! İki ayrı kavram, **karıştırılmadan** gösterilir:
//! 1. **Güven rozeti (MK-16):** Kriptografik imzaya dayanan "Resmi" / "Doğrulanmış yayıncı" durumu.
//!    Bu **gerçektir** — kurulumda host imzayı teyit eder (İP-07).
//! 2. **Doğrulama hattı (vizyon):** "3 AI + insan onayı" akışı bu sürümde **çalışmaz**; yalnızca bir
//!    **durum etiketi** olarak gösterilir.  Bu yüzden topluluk içeriği "doğrulama: beklemede" kalır
//!    ve **asla** sahte "doğrulandı" iddiası üretilmez (MK-48).

use biocraft_net::DogrulamaDurumu;
use egui::Color32;

use crate::i18n::Dil;
use crate::tokens::Tokenlar;

/// Bir doğrulama durumunun rozet rengi (token'dan; MK-52) ve ikonu.
fn rozet_stili(durum: DogrulamaDurumu, tok: &Tokenlar) -> (Color32, Color32, &'static str) {
    match durum {
        // İmzaya dayalı güven → vurgu/başarı rengi (gerçek güven).
        DogrulamaDurumu::Resmi => (tok.renk.vurgu_ustu, tok.renk.vurgu, "✓"),
        DogrulamaDurumu::DogrulanmisYayinci => (tok.renk.basari, tok.renk.basari_zemin, "✓"),
        // Küratörlü → bilgi (seçki; doğrulama değil).
        DogrulamaDurumu::Kuratorlu => (tok.renk.bilgi, tok.renk.bilgi_zemin, "★"),
        // Beklemede → nötr/soluk (henüz doğrulanmadı — uyarı değil, sadece dürüst durum).
        DogrulamaDurumu::IncelemeBekliyor => (tok.renk.metin_soluk, tok.renk.yuzey_alt, "…"),
    }
}

/// Bir doğrulama durumu rozetini çizer (ikon + dürüst etiket + açıklayıcı ipucu).
pub fn dogrulama_rozeti(ui: &mut egui::Ui, durum: DogrulamaDurumu, dil: Dil, tok: &Tokenlar) {
    let tr = matches!(dil, Dil::Tr);
    let (metin_renk, zemin, ikon) = rozet_stili(durum, tok);
    let etiket = format!("{ikon} {}", durum.etiket(tr));

    let yanit = egui::Frame::none()
        .fill(zemin)
        .rounding(tok.yaricap * 0.5)
        .inner_margin(egui::Margin::symmetric(tok.bosluk.s, 2.0))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(etiket)
                    .color(metin_renk)
                    .size(12.0)
                    .strong(),
            );
        })
        .response;

    // İpucu: rozetin ne anlama geldiğini DÜRÜST açıklar (sahte güven yok).
    yanit.on_hover_text(rozet_ipucu(durum, tr));
}

/// Bir rozetin dürüst açıklaması (ipucu metni).
pub fn rozet_ipucu(durum: DogrulamaDurumu, tr: bool) -> &'static str {
    match (durum, tr) {
        (DogrulamaDurumu::Resmi, true) => {
            "BioCraft'ın resmi imzasıyla doğrulanır (MK-16). Kurulumda imza/bütünlük teyit edilir."
        }
        (DogrulamaDurumu::Resmi, false) => {
            "Verified by BioCraft's official signature (MK-16). Signature/integrity checked at install."
        }
        (DogrulamaDurumu::DogrulanmisYayinci, true) => {
            "Güven deposundaki bir yayıncının imzasıyla doğrulanır (MK-16). İçeriği yine de değerlendirin."
        }
        (DogrulamaDurumu::DogrulanmisYayinci, false) => {
            "Verified by a trusted publisher's signature (MK-16). Still evaluate the content yourself."
        }
        (DogrulamaDurumu::Kuratorlu, true) => {
            "Seçilmiş (küratörlü) içerik — biçimsel bir doğrulama DEĞİL; yalnızca bir seçki."
        }
        (DogrulamaDurumu::Kuratorlu, false) => {
            "Curated content — NOT a formal verification; just a selection."
        }
        (DogrulamaDurumu::IncelemeBekliyor, true) => {
            "Henüz doğrulanmadı. Gelecekteki inceleme hattına aday; kendi sorumluluğunuzla kullanın."
        }
        (DogrulamaDurumu::IncelemeBekliyor, false) => {
            "Not yet verified. Candidate for the future review pipeline; use at your own discretion."
        }
    }
}

/// Bir haber kartı kaynağının "küratörlü/güvenilir kaynak" rozeti.
///
/// **Dürüstlük:** bu, *kaynağın* küratörlü olduğunu söyler; içeriğin bağımsız doğruluğunu
/// **garanti etmez** (ipucu bunu açıkça yazar).
pub fn haber_kaynak_rozeti(ui: &mut egui::Ui, dogrulanmis: bool, dil: Dil, tok: &Tokenlar) {
    if !dogrulanmis {
        return;
    }
    let tr = matches!(dil, Dil::Tr);
    let yanit = egui::Frame::none()
        .fill(tok.renk.basari_zemin)
        .rounding(tok.yaricap * 0.5)
        .inner_margin(egui::Margin::symmetric(tok.bosluk.s, 2.0))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(if tr {
                    "✓ Küratörlü kaynak"
                } else {
                    "✓ Curated source"
                })
                .color(tok.renk.basari)
                .size(11.0),
            );
        })
        .response;
    yanit.on_hover_text(if tr {
        "Kaynak küratörlü/güvenilir seçilmiştir; içeriğin doğruluğu bağımsız olarak doğrulanmalıdır."
    } else {
        "The source is curated/trusted; the content's accuracy must be independently verified."
    });
}

/// "Doğrulama hattı" (3 AI + insan onayı) **vizyon** açıklaması.
///
/// Bu hattın bu sürümde **çalışmadığını** açıkça yazar; aşamaları yalnızca **planlanan** durum
/// olarak gösterir.  Hiçbir öğe için "doğrulandı" iddiası üretmez (MK-48).
pub fn dogrulama_hatti_acikla(ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) {
    let tr = matches!(dil, Dil::Tr);
    egui::Frame::none()
        .fill(tok.renk.yuzey_alt)
        .stroke(egui::Stroke::new(1.0, tok.renk.kenarlik))
        .rounding(tok.yaricap)
        .inner_margin(egui::Margin::same(tok.bosluk.m))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(if tr {
                    "🛡 Doğrulama hattı (vizyon — bu sürümde çalışmıyor)"
                } else {
                    "🛡 Verification pipeline (vision — not active in this version)"
                })
                .strong()
                .color(tok.renk.metin),
            );
            ui.add_space(tok.bosluk.xs);
            ui.label(
                egui::RichText::new(if tr {
                    "İleride içerik şu aşamalardan geçecek. Şu an bu aşamalar PLANLANMIŞ durumdadır; \
                     herhangi bir öğe için \"doğrulandı\" iddiası üretilmez."
                } else {
                    "In the future, content will pass these stages. For now these stages are PLANNED; \
                     no \"verified\" claim is produced for any item."
                })
                .color(tok.renk.metin_soluk),
            );
            ui.add_space(tok.bosluk.s);
            // Aşamalar — hepsi "planlanan" (gri); hiçbiri "tamamlandı" göstermez.
            for (tr_metin, en_metin) in [
                ("1. AI ön-tarama (3 bağımsız sağlayıcı)", "1. AI pre-screen (3 independent providers)"),
                ("2. Çapraz kontrol uyumu (güven sinyali — garanti değil)", "2. Cross-check agreement (confidence signal — not a guarantee)"),
                ("3. İnsan uzman onayı", "3. Human expert approval"),
            ] {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("○").color(tok.renk.metin_soluk));
                    ui.label(
                        egui::RichText::new(if tr { tr_metin } else { en_metin })
                            .color(tok.renk.metin_soluk),
                    );
                    ui.label(
                        egui::RichText::new(if tr { "[planlanan]" } else { "[planned]" })
                            .italics()
                            .color(tok.renk.uyari),
                    );
                });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rozet_ipuclari_durust() {
        // Küratörlü/beklemede ipuçları sahte "doğrulandı" iddiası içermez.
        assert!(rozet_ipucu(DogrulamaDurumu::Kuratorlu, true).contains("DEĞİL"));
        assert!(rozet_ipucu(DogrulamaDurumu::IncelemeBekliyor, true).contains("doğrulanmadı"));
        // İmza temelli olanlar MK-16'ya atıfta bulunur.
        assert!(rozet_ipucu(DogrulamaDurumu::Resmi, true).contains("MK-16"));
    }

    #[test]
    fn rozet_headless_cizilir() {
        for durum in [
            DogrulamaDurumu::Resmi,
            DogrulamaDurumu::DogrulanmisYayinci,
            DogrulamaDurumu::Kuratorlu,
            DogrulamaDurumu::IncelemeBekliyor,
        ] {
            for dil in [Dil::Tr, Dil::En] {
                let ctx = egui::Context::default();
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let tok = Tokenlar::koyu();
                        dogrulama_rozeti(ui, durum, dil, &tok);
                        haber_kaynak_rozeti(ui, true, dil, &tok);
                        dogrulama_hatti_acikla(ui, dil, &tok);
                    });
                });
            }
        }
    }
}
