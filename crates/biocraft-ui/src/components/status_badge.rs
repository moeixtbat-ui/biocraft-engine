//! Durum rozetleri (İP-16, TDA madde 11 — durum farkındalığı; madde 1/19 — eksik/taşınmış kaynak).
//!
//! Sistem ve kaynak durumunu standart, tutarlı renkli "hap" (pill) biçiminde gösterir:
//! çevrimiçi/çevrimdışı, kaynak yetersiz, soğutuluyor (İP-08 termal), eklenti-yok ([İndir]),
//! taşınmış kaynak ([Yeniden bağla]).  Eyleme bağlı rozetlerde buton da yerleşiktir.

use crate::i18n::{ceviri, Anahtar, Dil};
use crate::tokens::{Onem, Tokenlar};

/// Tek bir durum rozeti.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusBadge {
    /// Çevrimiçi.
    Cevrimici,
    /// Çevrimdışı (yalnızca yerel; ağ yok).
    Cevrimdisi,
    /// Donanım/bellek kaynağı yetersiz.
    KaynakYetersiz,
    /// GPU/CPU termal sınırda; iş yavaşlatılıyor (İP-08).
    Sogutuluyor,
    /// Gerekli eklenti kurulu değil → [İndir] butonu.
    EklentiYok {
        /// Eksik eklentinin adı.
        ad: String,
    },
    /// Bağlı kaynak/dosya bulunamadı (taşınmış) → [Yeniden bağla] butonu.
    TasinmisKaynak {
        /// Taşınmış kaynağın adı.
        ad: String,
    },
}

/// Rozet üzerindeki butona basılınca dönen eylem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RozetEylem {
    /// Eksik eklentiyi indir.
    Indir,
    /// Taşınmış kaynağı yeniden bağla.
    YenidenBagla,
}

impl StatusBadge {
    fn onem(&self) -> Onem {
        match self {
            StatusBadge::Cevrimici => Onem::Basari,
            StatusBadge::Cevrimdisi => Onem::Notr,
            StatusBadge::KaynakYetersiz => Onem::Hata,
            StatusBadge::Sogutuluyor => Onem::Uyari,
            StatusBadge::EklentiYok { .. } => Onem::Bilgi,
            StatusBadge::TasinmisKaynak { .. } => Onem::Uyari,
        }
    }

    fn ikon(&self) -> &'static str {
        match self {
            StatusBadge::Cevrimici => "●",
            StatusBadge::Cevrimdisi => "○",
            StatusBadge::KaynakYetersiz => "▼",
            StatusBadge::Sogutuluyor => "❄",
            StatusBadge::EklentiYok { .. } => "⬇",
            StatusBadge::TasinmisKaynak { .. } => "⚲",
        }
    }

    /// Rozetin yerelleştirilmiş etiketi (gerekirse kaynak adıyla).
    pub fn etiket(&self, dil: Dil) -> String {
        match self {
            StatusBadge::Cevrimici => ceviri(dil, Anahtar::DurumCevrimici).to_string(),
            StatusBadge::Cevrimdisi => ceviri(dil, Anahtar::DurumCevrimdisi).to_string(),
            StatusBadge::KaynakYetersiz => ceviri(dil, Anahtar::DurumKaynakYetersiz).to_string(),
            StatusBadge::Sogutuluyor => ceviri(dil, Anahtar::DurumSogutuluyor).to_string(),
            StatusBadge::EklentiYok { ad } => {
                format!("{}: {ad}", ceviri(dil, Anahtar::DurumEklentiYok))
            }
            StatusBadge::TasinmisKaynak { ad } => {
                format!("{}: {ad}", ceviri(dil, Anahtar::DurumTasinmisKaynak))
            }
        }
    }

    /// Rozeti çizer.  Yerleşik butona basılırsa ilgili eylemi döndürür.
    pub fn show(&self, ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) -> Option<RozetEylem> {
        let onem = self.onem();
        let renk = tok.onem_rengi(onem);
        let zemin = tok.onem_zemini(onem);
        let mut eylem = None;

        let cerceve = egui::Frame {
            fill: zemin,
            stroke: egui::Stroke::new(1.0, renk),
            rounding: egui::Rounding::same(999.0),
            inner_margin: egui::Margin::symmetric(tok.bosluk.m, tok.bosluk.xs),
            ..Default::default()
        };

        cerceve.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(self.ikon()).color(renk).small());
                ui.label(
                    egui::RichText::new(self.etiket(dil))
                        .small()
                        .color(tok.renk.metin),
                );
                if matches!(self, StatusBadge::EklentiYok { .. })
                    && ui.small_button(ceviri(dil, Anahtar::Indir)).clicked()
                {
                    eylem = Some(RozetEylem::Indir);
                }
                if matches!(self, StatusBadge::TasinmisKaynak { .. })
                    && ui
                        .small_button(ceviri(dil, Anahtar::YenidenBagla))
                        .clicked()
                {
                    eylem = Some(RozetEylem::YenidenBagla);
                }
            });
        });

        eylem
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etiketler_kaynak_adini_icerir() {
        let r = StatusBadge::EklentiYok {
            ad: "Dağıtık Ağ".into(),
        };
        assert!(r.etiket(Dil::Tr).contains("Dağıtık Ağ"));
        let t = StatusBadge::TasinmisKaynak {
            ad: "genome.bam".into(),
        };
        assert!(t.etiket(Dil::En).contains("genome.bam"));
    }

    #[test]
    fn onem_renkleri_ayrisir() {
        let tok = Tokenlar::acik();
        assert_eq!(StatusBadge::Cevrimici.onem(), Onem::Basari);
        assert_eq!(StatusBadge::KaynakYetersiz.onem(), Onem::Hata);
        // Çevrimiçi yeşil, kaynak yetersiz kırmızı: token'dan ayrışır.
        assert_ne!(
            tok.onem_rengi(StatusBadge::Cevrimici.onem()),
            tok.onem_rengi(StatusBadge::KaynakYetersiz.onem())
        );
    }

    #[test]
    fn rozetler_headless_cizilir() {
        let ctx = egui::Context::default();
        let tok = Tokenlar::acik();
        let rozetler = [
            StatusBadge::Cevrimici,
            StatusBadge::Cevrimdisi,
            StatusBadge::KaynakYetersiz,
            StatusBadge::Sogutuluyor,
            StatusBadge::EklentiYok { ad: "X".into() },
            StatusBadge::TasinmisKaynak { ad: "y.bam".into() },
        ];
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                for r in &rozetler {
                    let _ = r.show(ui, Dil::Tr, &tok);
                }
            });
        });
    }
}
