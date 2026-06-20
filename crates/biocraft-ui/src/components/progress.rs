//! İlerleme / iş (job) bileşeni (İP-16, TDA madde 3 — durum bildirimi; madde 12 — iptal).
//!
//! Bir arka plan işini gösterir: "ne yapılıyor" + yüzde + tahmini kalan süre + iptal +
//! durum.  Durum kaynağı [`biocraft_types::JobStatus`]'tur (motorun gerçek iş izleme tipi).
//! İleride Arka Plan İşleri paneliyle (İP-03) bu bileşen entegre olur.
//!
//! İptal ve süre-tahmini mantığı egui'den bağımsız metotlarla testlenebilir.

use biocraft_types::JobStatus;

use crate::i18n::{ceviri, kalan_sure_metni, Anahtar, Dil};
use crate::tokens::Tokenlar;

/// İlerleme bileşeninin durumu.
#[derive(Debug, Clone)]
pub struct IsIlerleme {
    /// "Ne yapılıyor" açıklaması (örn. "Varyantlar taranıyor").
    pub ad: String,
    /// İşin anlık durumu.
    pub durum: JobStatus,
    /// İş iptal edilebilir mi?
    pub iptal_edilebilir: bool,
    iptal_istendi: bool,
    baslangic: Option<f64>,
}

/// İlerleme bileşeninden çıkan kullanıcı eylemi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IlerlemeEylem {
    /// Kullanıcı iptal istedi.
    Iptal,
}

impl IsIlerleme {
    /// Bekliyor durumunda yeni bir iş kurar.
    pub fn yeni(ad: impl Into<String>) -> Self {
        Self {
            ad: ad.into(),
            durum: JobStatus::Bekliyor,
            iptal_edilebilir: true,
            iptal_istendi: false,
            baslangic: None,
        }
    }

    /// İşin durumunu günceller (motor tarafından çağrılır).
    pub fn durumu_ayarla(&mut self, durum: JobStatus) {
        self.durum = durum;
    }

    /// İptal istendi mi?
    pub fn iptal_istendi(&self) -> bool {
        self.iptal_istendi
    }

    /// İptal bayrağını kaldırır (cancellation token'a karşılık gelir; MK-11).
    pub fn iptal_et(&mut self) {
        self.iptal_istendi = true;
    }

    /// Geçen süre + yüzdeye göre tahmini kalan saniyeyi hesaplar.
    /// Yalnızca "Çalışıyor + bilinen yüzde" durumunda anlamlıdır.
    pub fn tahmini_kalan_saniye(&self, gecen_saniye: f64) -> Option<f64> {
        if let JobStatus::Calisiyor {
            ilerleme: Some(yuzde),
        } = self.durum
        {
            if yuzde == 0 {
                return None;
            }
            let oran = yuzde as f64 / 100.0;
            Some(gecen_saniye * (1.0 - oran) / oran)
        } else {
            None
        }
    }

    /// Bileşeni çizer.  Kullanıcı iptal isterse [`IlerlemeEylem::Iptal`] döner.
    pub fn show(&mut self, ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) -> Option<IlerlemeEylem> {
        let now = ui.input(|i| i.time);
        // İlk "çalışıyor" karesinde başlangıç zamanını yakala (ETA için).
        if matches!(self.durum, JobStatus::Calisiyor { .. }) && self.baslangic.is_none() {
            self.baslangic = Some(now);
        }

        let mut eylem = None;
        crate::components::kart_cercevesi(tok).show(ui, |ui| {
            ui.label(egui::RichText::new(&self.ad).strong().color(tok.renk.metin));
            ui.add_space(tok.bosluk.xs);

            match self.durum.clone() {
                JobStatus::Bekliyor => {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new().color(tok.renk.metin_soluk));
                        ui.label(
                            egui::RichText::new(ceviri(dil, Anahtar::IsBekliyor))
                                .color(tok.renk.metin_soluk),
                        );
                    });
                }
                JobStatus::Calisiyor { ilerleme } => {
                    match ilerleme {
                        Some(yuzde) => {
                            ui.add(
                                egui::ProgressBar::new(yuzde as f32 / 100.0)
                                    .show_percentage()
                                    .fill(tok.renk.vurgu),
                            );
                            if let Some(bas) = self.baslangic {
                                if let Some(kalan) = self.tahmini_kalan_saniye(now - bas) {
                                    ui.label(
                                        egui::RichText::new(kalan_sure_metni(dil, kalan))
                                            .small()
                                            .color(tok.renk.metin_soluk),
                                    );
                                }
                            }
                        }
                        None => {
                            // Belirsiz ilerleme: spinner + "çalışıyor".
                            ui.horizontal(|ui| {
                                ui.add(egui::Spinner::new().color(tok.renk.vurgu));
                                ui.label(
                                    egui::RichText::new(ceviri(dil, Anahtar::IsCalisiyor))
                                        .color(tok.renk.metin),
                                );
                            });
                        }
                    }

                    ui.add_space(tok.bosluk.xs);
                    if self.iptal_istendi {
                        ui.label(
                            egui::RichText::new(ceviri(dil, Anahtar::IsIptalEdildi))
                                .small()
                                .color(tok.renk.uyari),
                        );
                    } else if self.iptal_edilebilir
                        && ui.button(ceviri(dil, Anahtar::IsIptal)).clicked()
                    {
                        self.iptal_et();
                        eylem = Some(IlerlemeEylem::Iptal);
                    }
                    ui.ctx().request_repaint();
                }
                JobStatus::Bitti => {
                    ui.label(
                        egui::RichText::new(format!("✔  {}", ceviri(dil, Anahtar::IsBitti)))
                            .color(tok.renk.basari)
                            .strong(),
                    );
                }
                JobStatus::Hata { mesaj } => {
                    ui.label(
                        egui::RichText::new(format!(
                            "✖  {}: {mesaj}",
                            ceviri(dil, Anahtar::IsHata)
                        ))
                        .color(tok.renk.hata)
                        .strong(),
                    );
                }
            }
        });
        eylem
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iptal_edilebilir() {
        let mut is = IsIlerleme::yeni("Tarama");
        assert!(!is.iptal_istendi());
        is.iptal_et();
        assert!(is.iptal_istendi(), "İptal bayrağı kalkmalı");
    }

    #[test]
    fn tahmini_kalan_dogru_hesaplanir() {
        let mut is = IsIlerleme::yeni("İş");
        is.durumu_ayarla(JobStatus::Calisiyor { ilerleme: Some(50) });
        // %50'de 10 sn geçmişse, kalan ≈ 10 sn.
        let kalan = is.tahmini_kalan_saniye(10.0).unwrap();
        assert!((kalan - 10.0).abs() < 1e-6);
    }

    #[test]
    fn tahmini_kalan_belirsizken_yok() {
        let mut is = IsIlerleme::yeni("İş");
        is.durumu_ayarla(JobStatus::Calisiyor { ilerleme: None });
        assert!(is.tahmini_kalan_saniye(10.0).is_none());
        is.durumu_ayarla(JobStatus::Bekliyor);
        assert!(is.tahmini_kalan_saniye(10.0).is_none());
    }

    #[test]
    fn ilerleme_headless_cizilir() {
        let ctx = egui::Context::default();
        let mut is = IsIlerleme::yeni("Varyantlar taranıyor");
        is.durumu_ayarla(JobStatus::Calisiyor { ilerleme: Some(42) });
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = is.show(ui, Dil::Tr, &Tokenlar::acik());
            });
        });
    }
}
