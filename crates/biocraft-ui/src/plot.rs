//! 2B plot widget'i — **egui adaptörü** (İP-04).
//!
//! Render katmanındaki saf [`Plot2B`](biocraft_render::plot::Plot2B) modelinin ürettiği
//! çizim komutlarını egui `Painter` ile çizer.  Renkler doğrudan değil, **token anahtarıyla**
//! gelir ([`Tokenlar::anahtar_renk`](crate::tokens::Tokenlar::anahtar_renk)) → sabit renk yok
//! (MK-52).  Culling + LOD render modelinde uygulanır → büyük seride bile akıcı (MK-04).

use biocraft_render::lod::Dortgen;
use biocraft_render::plot::{Plot2B, Sekil};

use crate::tokens::Tokenlar;

/// Bir [`Plot2B`]'yi token-renkli olarak çizen widget (coverage/çizgi/scatter).
pub struct PlotWidget<'a> {
    plot: &'a Plot2B,
    yukseklik: f32,
}

impl<'a> PlotWidget<'a> {
    /// Verilen çizim için widget (varsayılan yükseklik 160 px).
    pub fn yeni(plot: &'a Plot2B) -> Self {
        Self {
            plot,
            yukseklik: 160.0,
        }
    }

    /// Çizim yüksekliğini ayarlar (px).
    pub fn yukseklik(mut self, yukseklik: f32) -> Self {
        self.yukseklik = yukseklik;
        self
    }

    /// Widget'ı çizer; kullanılan ekran dikdörtgenini içeren yanıtı döndürür.
    pub fn goster(self, ui: &mut egui::Ui, tok: &Tokenlar) -> egui::Response {
        let en = ui.available_width().max(32.0);
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(en, self.yukseklik), egui::Sense::hover());
        let painter = ui.painter_at(rect);

        // Zemin + kenarlık (token).
        painter.rect_filled(rect, tok.yaricap, tok.renk.yuzey);
        painter.rect_stroke(rect, tok.yaricap, egui::Stroke::new(1.0, tok.renk.kenarlik));

        // İç çizim alanı (kenar boşluğu) — render modeline mutlak ekran koordinatı verilir.
        let ic = rect.shrink(tok.bosluk.m);
        if ic.width() < 1.0 || ic.height() < 1.0 {
            return resp;
        }
        let alan = Dortgen::yeni(ic.min.x, ic.min.y, ic.width(), ic.height());

        // Taban çizgisi (alt eksen) — soluk.
        painter.line_segment(
            [
                egui::pos2(ic.min.x, ic.max.y),
                egui::pos2(ic.max.x, ic.max.y),
            ],
            egui::Stroke::new(1.0, tok.renk.metin_soluk),
        );

        for komut in self.plot.ciz_komutlari(alan) {
            let renk = tok.anahtar_renk(&komut.renk_anahtari);
            match komut.sekil {
                Sekil::Cizgi { a, b } => {
                    painter.line_segment(
                        [egui::pos2(a[0], a[1]), egui::pos2(b[0], b[1])],
                        egui::Stroke::new(komut.kalinlik, renk),
                    );
                }
                Sekil::Nokta { merkez, yaricap } => {
                    painter.circle_filled(egui::pos2(merkez[0], merkez[1]), yaricap, renk);
                }
            }
        }
        resp
    }
}

/// Galeri/örnek için temsilî bir coverage (çizgi) + varyant (scatter) çizimi üretir.
pub fn ornek_plot() -> Plot2B {
    use biocraft_render::plot::Seri;
    // Coverage benzeri dalgalı çizgi (çok noktalı → LOD seyreltme devreye girer).
    let coverage: Vec<[f64; 2]> = (0..2000)
        .map(|i| {
            let x = i as f64;
            let y =
                40.0 + 18.0 * (x * 0.03).sin() + 8.0 * (x * 0.11).cos() + 5.0 * (x * 0.005).sin();
            [x, y]
        })
        .collect();
    // Birkaç "varyant" noktası (scatter).
    let varyantlar: Vec<[f64; 2]> = (0..14)
        .map(|i| {
            let x = (i as f64) * 140.0 + 30.0;
            [x, 70.0 + 12.0 * ((i as f64) * 1.7).sin()]
        })
        .collect();
    Plot2B::yeni()
        .seri_ekle(Seri::cizgi("Coverage", coverage, "accent.primary"))
        .seri_ekle(Seri::sacilim("Varyantlar", varyantlar, "warning"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ornek_plot_iki_seri_icerir() {
        let p = ornek_plot();
        assert_eq!(p.seriler.len(), 2);
        // Renkler token anahtarı taşır (sabit renk değil).
        assert_eq!(p.seriler[0].renk_anahtari, "accent.primary");
    }

    #[test]
    fn plot_widget_headless_panik_atmaz() {
        let ctx = egui::Context::default();
        let plot = ornek_plot();
        let tok = Tokenlar::koyu();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                PlotWidget::yeni(&plot).yukseklik(160.0).goster(ui, &tok);
            });
        });
    }
}
