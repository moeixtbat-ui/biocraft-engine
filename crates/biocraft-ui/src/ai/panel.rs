//! YZ-01 — **AI paneli** (egui).  Sohbet + akış + dürüst çıktı şeması + "yapılandırılmadı".
//!
//! Üç ana durum:
//! 1. **AI kapalı** → sade "AI kapalı, uygulama tam çalışıyor" + "AI'ı aç" yönlendirmesi.
//! 2. **Sağlayıcı yok** → "AI yapılandırılmadı" + "AI sağlayıcı ekle" (sahte işlev YOK — MK-48).
//! 3. **Hazır** → sohbet yüzeyi; her çıktı kaynak + güven + "doğrulanmalı" + token/maliyet taşır
//!    (MK-47), eylem önerileri **onaya tabi** (kör otomasyon yok), klinik değil (MK-49).
// MK-47/48/49: dürüst, kör-güvene kapalı, klinik değil yüzey.

use biocraft_ai_surface::{AiCikti, GuvenSeviyesi};

use crate::components::EmptyState;
use crate::i18n::Dil;
use crate::tokens::Tokenlar;

use super::cost_badge::maliyet_rozeti_ciz;
use super::{AiPanelEylem, AiYuzey};

/// AI panelini çizer ve (varsa) app'in karşılaması gereken eylemi döndürür.
pub fn ai_panel_ciz(
    ui: &mut egui::Ui,
    yuzey: &mut AiYuzey,
    dil: Dil,
    tok: &Tokenlar,
) -> Option<AiPanelEylem> {
    let tr = matches!(dil, Dil::Tr);

    // ── Durum 1: AI kapalı ──────────────────────────────────────────────────
    if !yuzey.etkin {
        let tiklandi = EmptyState::yeni(
            "🌙",
            if tr { "AI kapalı" } else { "AI is off" },
            if tr {
                "AI bu oturumda kapalı. Uygulama tam çalışıyor; AI olmadan da her şeyi yapabilirsiniz. \
                 Açmak için Ayarlar → AI."
            } else {
                "AI is off this session. The app works fully; you can do everything without AI. \
                 Turn it on from Settings → AI."
            },
        )
        .with_eylem(if tr { "AI'ı aç" } else { "Turn on AI" })
        .show(ui, tok);
        return tiklandi.then_some(AiPanelEylem::AiAc);
    }

    // ── Durum 2: sağlayıcı yok → "yapılandırılmadı" (MK-48) ──────────────────
    if !yuzey.yapilandirildi_mi() {
        let tiklandi = EmptyState::yeni(
            "✨",
            if tr { "AI yapılandırılmadı" } else { "AI not configured" },
            if tr {
                "Bağlı bir AI sağlayıcı/motor yok. Bu sürümde AI yüzeyi hazırdır ama gerçek motor \
                 bir eklenti olarak gelir (İP-14 sonrası). Sahte yanıt göstermiyoruz."
            } else {
                "No AI provider/engine is connected. The AI surface is ready in this version, but a \
                 real engine ships as a plugin (after İP-14). We do not show fake answers."
            },
        )
        .with_eylem(if tr { "AI sağlayıcı ekle" } else { "Add AI provider" })
        .show(ui, tok);
        return tiklandi.then_some(AiPanelEylem::SaglayiciEkle);
    }

    // ── Durum 3: hazır → sohbet yüzeyi ───────────────────────────────────────
    sohbet_ciz(ui, yuzey, tr, tok)
}

/// Hazır durumdaki sohbet yüzeyini çizer.
fn sohbet_ciz(
    ui: &mut egui::Ui,
    yuzey: &mut AiYuzey,
    tr: bool,
    tok: &Tokenlar,
) -> Option<AiPanelEylem> {
    let mut eylem: Option<AiPanelEylem> = None;

    // Başlık şeridi: sağlayıcı seçici + maliyet rozeti + temizle.
    ui.horizontal(|ui| {
        saglayici_secici(ui, yuzey, tr);
        ui.separator();
        maliyet_rozeti_ciz(
            ui,
            &yuzey.sayac,
            &yuzey.kota,
            yuzey.token_goster,
            yuzey.maliyet_goster,
            if tr { Dil::Tr } else { Dil::En },
            tok,
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(if tr { "🗑 Temizle" } else { "🗑 Clear" })
                .on_hover_text(if tr {
                    "Konuşmayı sıfırla"
                } else {
                    "Reset conversation"
                })
                .clicked()
            {
                yuzey.sohbeti_temizle();
            }
        });
    });

    // Kalıcı uyarı: araştırma amaçlı, klinik değil (MK-49).
    ui.label(
        egui::RichText::new(if tr {
            "Yalnızca araştırma/Ar-Ge amaçlıdır; klinik/tanısal karar üretmez. Çıktılar doğrulanmalıdır."
        } else {
            "Research/R&D only; not a clinical/diagnostic decision. Outputs must be verified."
        })
        .small()
        .italics()
        .color(tok.renk.metin_soluk),
    );
    ui.separator();

    // Konuşma + çıktı + hata (kaydırılabilir).
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .max_height(ui.available_height() - 38.0)
        .show(ui, |ui| {
            if yuzey.sohbet.is_empty() && yuzey.son_cikti.is_none() && !yuzey.mesgul() {
                ui.weak(if tr {
                    "Bir soru sorun. Örn: \"Bu varyantı yorumla\" (demo sağlayıcıyla yüzey gösterilir)."
                } else {
                    "Ask a question. e.g. \"Interpret this variant\" (surface shown with demo provider)."
                });
            }

            // Geçmiş mesajlar.
            for msj in &yuzey.sohbet {
                let renk = match msj.rol {
                    biocraft_ai_surface::MesajRol::Kullanici => tok.renk.vurgu,
                    _ => tok.renk.bilgi,
                };
                ui.label(
                    egui::RichText::new(format!("{}:", msj.rol.etiket(tr)))
                        .small()
                        .strong()
                        .color(renk),
                );
                ui.label(egui::RichText::new(&msj.metin).color(tok.renk.metin));
                ui.add_space(tok.bosluk.xs);
            }

            // Akan kısmi metin ("yazıyor…").
            if yuzey.mesgul() {
                let kismi = yuzey.kismi_metin();
                ui.label(
                    egui::RichText::new(if tr { "Asistan (yazıyor…):" } else { "Assistant (typing…):" })
                        .small()
                        .strong()
                        .color(tok.renk.bilgi),
                );
                ui.label(egui::RichText::new(kismi).color(tok.renk.metin));
            }

            // Son tamamlanan zengin çıktı (kaynak/güven/doğrulama/maliyet + eylemler).
            if !yuzey.mesgul() {
                if let Some(cikti) = &yuzey.son_cikti {
                    if let Some(idx) = cikti_ciz(ui, cikti, tr, tok) {
                        eylem = Some(AiPanelEylem::EylemUygula(idx));
                    }
                }
            }

            // Hata (çıkış kapısı engeli ya da sağlayıcı hatası).
            if let Some(h) = &yuzey.son_hata {
                hata_ciz(ui, h, tr, tok);
            }
        });

    // Girdi şeridi.
    ui.separator();
    ui.horizontal(|ui| {
        if yuzey.mesgul() {
            if ui
                .button(
                    egui::RichText::new(if tr { "■ Durdur" } else { "■ Stop" })
                        .color(tok.renk.hata),
                )
                .clicked()
            {
                yuzey.durdur();
            }
        } else {
            let gonder_aktif = !yuzey.girdi.trim().is_empty();
            let yanit = ui.add(
                egui::TextEdit::singleline(&mut yuzey.girdi)
                    .desired_width(ui.available_width() - 90.0)
                    .hint_text(if tr { "AI'a sor…" } else { "Ask AI…" }),
            );
            let enter = yanit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let tikla = ui
                .add_enabled(
                    gonder_aktif,
                    egui::Button::new(if tr { "Gönder" } else { "Send" }),
                )
                .clicked();
            if (tikla || enter) && gonder_aktif {
                yuzey.gonder();
                yanit.request_focus();
            }
        }
    });

    eylem
}

/// Sağlayıcı/model seçici (ComboBox).  MVP'de yalnız demo/echo görünür; gerçek motor İP-14 sonrası.
fn saglayici_secici(ui: &mut egui::Ui, yuzey: &mut AiYuzey, tr: bool) {
    let kimlikler = yuzey.kayit.kimlikler();
    let secili_idx = yuzey.kayit.secili_indeks().unwrap_or(0);
    let secili_ad = kimlikler
        .get(secili_idx)
        .map(|k| format!("{} · {}", k.ad, k.tur.etiket(tr)))
        .unwrap_or_else(|| "—".to_string());
    // Kimlik listesini önce topla (borrow'u erken bitir), sonra seç.
    let secenekler: Vec<(usize, String)> = kimlikler
        .iter()
        .enumerate()
        .map(|(i, k)| (i, format!("{} · {}", k.ad, k.tur.etiket(tr))))
        .collect();
    let mut yeni_secim: Option<usize> = None;
    egui::ComboBox::from_id_salt("ai_saglayici_secici")
        .selected_text(secili_ad)
        .show_ui(ui, |ui| {
            for (i, ad) in &secenekler {
                if ui.selectable_label(*i == secili_idx, ad).clicked() {
                    yeni_secim = Some(*i);
                }
            }
        });
    if let Some(i) = yeni_secim {
        yuzey.kayit.sec(i);
    }
}

/// **Zengin çıktıyı dürüstçe çizer (MK-47).**  Tıklanan eylem önerisinin indeksini döndürür.
fn cikti_ciz(ui: &mut egui::Ui, cikti: &AiCikti, tr: bool, tok: &Tokenlar) -> Option<usize> {
    let mut tiklanan: Option<usize> = None;
    egui::Frame::group(ui.style())
        .fill(tok.renk.yuzey_alt)
        .show(ui, |ui| {
            // "Öneri" etiketi.
            ui.label(
                egui::RichText::new(if tr {
                    "💡 AI önerisi"
                } else {
                    "💡 AI suggestion"
                })
                .small()
                .strong()
                .color(tok.renk.bilgi),
            );

            // Güven göstergesi (kesin doğruluk değil).
            let g = &cikti.guven;
            let renk = match g.seviye {
                GuvenSeviyesi::Yuksek => tok.renk.basari,
                GuvenSeviyesi::Orta => tok.renk.uyari,
                GuvenSeviyesi::Dusuk => tok.renk.hata,
                GuvenSeviyesi::Bilinmiyor => tok.renk.metin_soluk,
            };
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{}: {}",
                        if tr { "Güven" } else { "Confidence" },
                        g.seviye.etiket(tr)
                    ))
                    .small()
                    .color(renk),
                );
                ui.add(
                    egui::ProgressBar::new(g.seviye.oran())
                        .desired_width(80.0)
                        .fill(renk),
                );
            });
            if let Some(uyum) = &g.coklu_ai_uyumu {
                ui.label(
                    egui::RichText::new(if tr {
                        format!(
                            "Çok-AI: {}/{} hemfikir (garanti değil — sinyal)",
                            uyum.hemfikir, uyum.saglayici_sayisi
                        )
                    } else {
                        format!(
                            "Multi-AI: {}/{} agree (not a guarantee — signal)",
                            uyum.hemfikir, uyum.saglayici_sayisi
                        )
                    })
                    .small()
                    .color(tok.renk.metin_soluk),
                );
            }

            // "Doğrulanmalı" uyarısı — her çıktıda (kör güven yok).
            ui.label(
                egui::RichText::new(format!("⚠ {}", cikti.dogrulama_uyarisi))
                    .small()
                    .color(tok.renk.uyari),
            );

            // Öneriler.
            if !cikti.oneriler.is_empty() {
                ui.add_space(tok.bosluk.xs);
                ui.label(
                    egui::RichText::new(if tr { "Öneriler:" } else { "Suggestions:" })
                        .small()
                        .strong()
                        .color(tok.renk.metin),
                );
                for o in &cikti.oneriler {
                    ui.label(egui::RichText::new(format!("• {o}")).color(tok.renk.metin_soluk));
                }
            }

            // Eylem önerileri — her biri ONAYA tabi (kör otomasyon yok).
            if !cikti.eylem_onerileri.is_empty() {
                ui.add_space(tok.bosluk.xs);
                ui.label(
                    egui::RichText::new(if tr {
                        "Önerilen eylemler (onaylı):"
                    } else {
                        "Suggested actions (approval):"
                    })
                    .small()
                    .strong()
                    .color(tok.renk.metin),
                );
                for (i, e) in cikti.eylem_onerileri.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("→ {}", e.aciklama)).color(tok.renk.metin),
                        );
                        if !e.geri_alinabilir {
                            ui.label(
                                egui::RichText::new(if tr {
                                    "(geri alınamaz!)"
                                } else {
                                    "(irreversible!)"
                                })
                                .small()
                                .color(tok.renk.hata),
                            );
                        }
                        if ui
                            .button(if tr {
                                "Uygula (onayla)"
                            } else {
                                "Apply (confirm)"
                            })
                            .clicked()
                        {
                            tiklanan = Some(i);
                        }
                    });
                }
            }

            // Kaynak / atıf.
            if !cikti.kaynaklar.is_empty() {
                ui.add_space(tok.bosluk.xs);
                ui.label(
                    egui::RichText::new(if tr { "Kaynaklar:" } else { "Sources:" })
                        .small()
                        .strong()
                        .color(tok.renk.metin),
                );
                for k in &cikti.kaynaklar {
                    let mut s = format!("• {}", k.baslik);
                    if let Some(u) = &k.url {
                        s.push_str(&format!(" — {u}"));
                    }
                    ui.label(egui::RichText::new(s).small().color(tok.renk.metin_soluk));
                    if let Some(a) = &k.atif {
                        ui.label(
                            egui::RichText::new(format!("   {a}"))
                                .small()
                                .italics()
                                .color(tok.renk.metin_soluk),
                        );
                    }
                }
            }

            // Token / maliyet.
            ui.add_space(tok.bosluk.xs);
            ui.label(
                egui::RichText::new(format!(
                    "{}: {} {} (girdi {} / çıktı {})",
                    if tr { "Kullanım" } else { "Usage" },
                    cikti.kullanim.toplam(),
                    if tr { "jeton" } else { "tokens" },
                    cikti.kullanim.girdi_jeton,
                    cikti.kullanim.cikti_jeton,
                ))
                .small()
                .color(tok.renk.metin_soluk),
            );
        });
    tiklanan
}

/// Çıkış kapısı engeli / sağlayıcı hatasını standart şemayla çizer.
fn hata_ciz(ui: &mut egui::Ui, h: &biocraft_types::ErrorReport, tr: bool, tok: &Tokenlar) {
    egui::Frame::group(ui.style())
        .fill(tok.renk.hata_zemin)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(format!("⛔ {}", h.ne_oldu))
                    .strong()
                    .color(tok.renk.hata),
            );
            ui.label(egui::RichText::new(&h.neden).small().color(tok.renk.metin));
            ui.label(
                egui::RichText::new(format!(
                    "{}: {}",
                    if tr { "Çözüm" } else { "Fix" },
                    h.nasil_cozulur
                ))
                .small()
                .color(tok.renk.metin_soluk),
            );
        });
}
