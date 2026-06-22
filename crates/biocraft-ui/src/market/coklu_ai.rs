//! **Opsiyonel çok-AI çapraz kontrol yüzeyi** (İP-18 kancası; MK-47).
//!
//! Bir iddiayı/çıktıyı birden çok AI sağlayıcısına sorup uyumu **güven sinyali** olarak gösterir.
//! Gerçek koordinasyon [`biocraft_ai_surface::CokluAiKontrol`]'dedir (L3); burada yalnızca egui
//! yüzeyi + **arka plan thread'i** (arayüz donmaz — MK-07/MK-48) vardır.
//!
//! **EN ÖNEMLİ — dürüstlük:** "uyum = kesin doğruluk" ASLA sunulmaz (MK-47).  Her sonuçta "garanti
//! değil" uyarısı **kalıcı** olarak gösterilir; uyum yüksek olsa bile bu yalnızca bir sinyaldir.
//! Bu yüzey yalnızca **herkese açık** pazar/haber içeriği içindir (PHI girilmemelidir; gerçek motor
//! eklenti, dış sağlayıcılarda çıkış kapısını uygular).

use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread;

use biocraft_ai_surface::{
    AiBaglam, CokluAiKontrol, CokluAiSonuc, Provider, SaglayiciKayit, UyumSeviyesi,
};

use crate::i18n::Dil;
use crate::tokens::Tokenlar;

/// Çapraz kontrolün arka plan durumu.
enum CokluDurum {
    /// Henüz çalıştırılmadı.
    Bosta,
    /// Arka planda sağlayıcılara soruluyor.
    Calisiyor,
    /// Tamamlandı; özet hazır.
    Bitti(CokluAiSonuc),
}

/// Çok-AI çapraz kontrol yüzey durumu (mağaza paneline gömülür).
pub struct CokluAiYuzey {
    /// Panel açık mı (kullanıcı genişletti mi)?
    pub acik: bool,
    /// Çapraz kontrol edilecek metin/iddia.
    pub sorgu: String,
    durum: CokluDurum,
    /// Arka plan thread'inden sonucu taşıyan kanal.
    alici: Option<Receiver<CokluAiSonuc>>,
}

impl Default for CokluAiYuzey {
    fn default() -> Self {
        Self {
            acik: false,
            sorgu: String::new(),
            durum: CokluDurum::Bosta,
            alici: None,
        }
    }
}

impl CokluAiYuzey {
    /// Yeni (boş) bir yüzey.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Çalışıyor mu (arka planda sorgu sürüyor)?
    pub fn calisiyor(&self) -> bool {
        matches!(self.durum, CokluDurum::Calisiyor)
    }

    /// Verilen sağlayıcılara `sorgu`'yu sorar (arka plan thread; arayüz bloklanmaz).
    pub fn baslat(&mut self, saglayicilar: Vec<Arc<dyn Provider>>) {
        let baglam = AiBaglam::sorgudan(self.sorgu.clone());
        let (gonderen, alici) = std::sync::mpsc::channel();
        thread::Builder::new()
            .name("biocraft-coklu-ai".into())
            .spawn(move || {
                let sonuc = CokluAiKontrol::calistir(&saglayicilar, &baglam);
                let _ = gonderen.send(sonuc);
            })
            .map(|_| ())
            .unwrap_or_else(|e| log::warn!("Çok-AI thread'i başlatılamadı: {e}"));
        self.alici = Some(alici);
        self.durum = CokluDurum::Calisiyor;
    }

    /// Kanalı bloklamadan yoklar (her karede çağrılır).  Durum değiştiyse `true` döner.
    pub fn yokla(&mut self) -> bool {
        let Some(alici) = self.alici.as_ref() else {
            return false;
        };
        match alici.try_recv() {
            Ok(sonuc) => {
                self.durum = CokluDurum::Bitti(sonuc);
                self.alici = None;
                true
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => false,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.durum = CokluDurum::Bosta;
                self.alici = None;
                true
            }
        }
    }
}

/// Çok-AI çapraz kontrol yüzeyini çizer (kayıtlı sağlayıcıları kullanır).
///
/// `kayit`: AI yüzeyinin sağlayıcı kaydı (İP-14).  En az 2 sağlayıcı yoksa anlamlı çapraz kontrol
/// yapılamaz → dürüst "yapılandırılmadı/yetersiz" notu (sahte işlev yok, MK-48).
pub fn coklu_ai_ciz(
    ui: &mut egui::Ui,
    yuzey: &mut CokluAiYuzey,
    kayit: &SaglayiciKayit,
    dil: Dil,
    tok: &Tokenlar,
) {
    let tr = matches!(dil, Dil::Tr);
    yuzey.yokla();

    egui::CollapsingHeader::new(if tr {
        "🔀 Çok-AI çapraz kontrol (opsiyonel)"
    } else {
        "🔀 Multi-AI cross-check (optional)"
    })
    .default_open(yuzey.acik)
    .show(ui, |ui| {
        // KALICI dürüstlük uyarısı — uyum = kesin doğruluk DEĞİL (MK-47).  Her zaman görünür.
        egui::Frame::none()
            .fill(tok.renk.uyari_zemin)
            .rounding(tok.yaricap)
            .inner_margin(egui::Margin::same(tok.bosluk.s))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(if tr {
                        "⚠ Uyum bir DOĞRULUK GARANTİSİ DEĞİLDİR. AI'lar ortak önyargıyla birlikte \
                         yanılabilir; sonuç yalnızca bir güven sinyalidir. Sonucu kendiniz doğrulayın."
                    } else {
                        "⚠ Agreement is NOT a CORRECTNESS GUARANTEE. AIs can be wrong together due to \
                         shared bias; this is only a confidence signal. Verify the result yourself."
                    })
                    .color(tok.renk.uyari),
                );
            });
        ui.add_space(tok.bosluk.s);

        let saglayici_sayisi = kayit.say();
        ui.label(
            egui::RichText::new(format!(
                "{}: {saglayici_sayisi}",
                if tr { "Kayıtlı AI sağlayıcısı" } else { "Registered AI providers" }
            ))
            .small()
            .color(tok.renk.metin_soluk),
        );

        // Sorgu girişi.
        ui.add(
            egui::TextEdit::multiline(&mut yuzey.sorgu)
                .desired_rows(2)
                .hint_text(if tr {
                    "Çapraz kontrol edilecek iddia/metin (yalnız herkese açık içerik; PHI girmeyin)"
                } else {
                    "Claim/text to cross-check (public content only; do not enter PHI)"
                })
                .desired_width(f32::INFINITY),
        );
        ui.add_space(tok.bosluk.xs);

        ui.horizontal(|ui| {
            let calistirilabilir =
                saglayici_sayisi >= 2 && !yuzey.calisiyor() && !yuzey.sorgu.trim().is_empty();
            if ui
                .add_enabled(
                    calistirilabilir,
                    egui::Button::new(if tr { "Çapraz kontrol et" } else { "Cross-check" }),
                )
                .clicked()
            {
                let saglayicilar: Vec<Arc<dyn Provider>> =
                    (0..saglayici_sayisi).filter_map(|i| kayit.al(i)).collect();
                yuzey.baslat(saglayicilar);
            }
            if yuzey.calisiyor() {
                ui.spinner();
                ui.label(
                    egui::RichText::new(if tr { "Sağlayıcılara soruluyor…" } else { "Asking providers…" })
                        .color(tok.renk.metin_soluk),
                );
            }
        });

        // 2'den az sağlayıcı → dürüst "yetersiz/yapılandırılmadı" notu.
        if saglayici_sayisi < 2 {
            ui.add_space(tok.bosluk.xs);
            ui.label(
                egui::RichText::new(if tr {
                    "Çapraz kontrol için en az 2 AI sağlayıcı gerekir. Sağlayıcı eklemek için Ayarlar > AI."
                } else {
                    "Cross-check needs at least 2 AI providers. Add providers under Settings > AI."
                })
                .italics()
                .color(tok.renk.metin_soluk),
            );
        }

        // Sonuç.
        if let CokluDurum::Bitti(sonuc) = &yuzey.durum {
            ui.add_space(tok.bosluk.s);
            ui.separator();
            sonuc_ciz(ui, sonuc, tr, tok);
        }
    });
}

/// Çapraz kontrol sonucunu çizer (uyum seviyesi + sağlayıcı yanıtları + kalıcı "garanti değil").
fn sonuc_ciz(ui: &mut egui::Ui, sonuc: &CokluAiSonuc, tr: bool, tok: &Tokenlar) {
    // Uyum seviyesi rengi: hemfikir → bilgi (sinyal); ayrışıyor → uyarı; yetersiz → soluk.
    let seviye_renk = match sonuc.seviye {
        UyumSeviyesi::Hemfikir | UyumSeviyesi::KismenHemfikir => tok.renk.bilgi,
        UyumSeviyesi::Ayrisiyor => tok.renk.uyari,
        UyumSeviyesi::Yetersiz => tok.renk.metin_soluk,
    };
    ui.label(
        egui::RichText::new(format!(
            "{}: {}",
            if tr { "Sonuç" } else { "Result" },
            sonuc.seviye.etiket(tr)
        ))
        .strong()
        .color(seviye_renk),
    );
    ui.label(
        egui::RichText::new(format!(
            "{} {} {} {}",
            sonuc.uyum.saglayici_sayisi,
            if tr { "sağlayıcıdan" } else { "providers," },
            sonuc.uyum.hemfikir,
            if tr {
                "tanesi hemfikir"
            } else {
                "in agreement"
            },
        ))
        .small()
        .color(tok.renk.metin_soluk),
    );
    ui.add_space(tok.bosluk.xs);

    // Sağlayıcı yanıtları (şeffaflık — kullanıcı tek tek görebilir).
    for y in &sonuc.yanitlar {
        egui::Frame::none()
            .fill(tok.renk.yuzey_alt)
            .rounding(tok.yaricap)
            .inner_margin(egui::Margin::same(tok.bosluk.s))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&y.kimlik.ad)
                            .strong()
                            .color(tok.renk.metin),
                    );
                    if !y.basarili {
                        ui.label(
                            egui::RichText::new(if tr { "(yanıt yok)" } else { "(no answer)" })
                                .small()
                                .color(tok.renk.hata),
                        );
                    }
                });
                let metin = if y.basarili {
                    y.metin.clone()
                } else {
                    y.hata.clone().unwrap_or_default()
                };
                ui.label(egui::RichText::new(metin).color(tok.renk.metin_soluk));
            });
        ui.add_space(tok.bosluk.xs);
    }

    // KALICI uyarı — sonuç ne olursa olsun "garanti değil" (MK-47).
    debug_assert!(sonuc.garanti_degil());
    egui::Frame::none()
        .fill(tok.renk.uyari_zemin)
        .rounding(tok.yaricap)
        .inner_margin(egui::Margin::same(tok.bosluk.s))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(sonuc.uyari(tr)).color(tok.renk.uyari));
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use biocraft_ai_surface::EchoSaglayici;
    use std::time::{Duration, Instant};

    fn yoklayarak_bekle(y: &mut CokluAiYuzey) {
        let baslangic = Instant::now();
        while y.alici.is_some() && baslangic.elapsed() < Duration::from_secs(5) {
            if y.yokla() {
                break;
            }
            thread::yield_now();
        }
    }

    #[test]
    fn iki_echo_saglayici_hemfikir() {
        // İki echo sağlayıcı aynı sorguya aynı yanıtı verir → hemfikir (yine de garanti değil).
        let mut y = CokluAiYuzey::yeni();
        y.sorgu = "Varyant nedir?".into();
        let saglayicilar: Vec<Arc<dyn Provider>> = vec![
            Arc::new(EchoSaglayici::yeni()),
            Arc::new(EchoSaglayici::yeni()),
        ];
        y.baslat(saglayicilar);
        yoklayarak_bekle(&mut y);
        match &y.durum {
            CokluDurum::Bitti(s) => {
                assert!(s.garanti_degil(), "her zaman garanti değil");
                assert_eq!(s.yanit_veren(), 2);
            }
            _ => panic!("sonuç bekleniyordu"),
        }
    }

    #[test]
    fn yuzey_headless_cizilir() {
        let mut kayit = SaglayiciKayit::yeni();
        kayit.kaydet(Arc::new(EchoSaglayici::yeni()));
        let mut y = CokluAiYuzey::yeni();
        y.acik = true;
        for dil in [Dil::Tr, Dil::En] {
            let ctx = egui::Context::default();
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let tok = Tokenlar::koyu();
                    coklu_ai_ciz(ui, &mut y, &kayit, dil, &tok);
                });
            });
        }
    }
}
