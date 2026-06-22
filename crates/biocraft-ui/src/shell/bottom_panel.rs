//! Alt Panel (Status Bar üstünde) — Konsol/çıktı, Arka Plan İşleri, AI sohbet, Günlük (İP-03).
//!
//! 0.9 tablosu: "Konsol/çıktı, Arka Plan İşleri (her Job ilerleme/iptal), AI sohbet, günlük
//! sekmeleri".  Saf durum ([`AltPanel`]) + çizim ([`alt_panel_ciz`]) ayrıdır; sekme seçimi nötr
//! [`AltSekmeSecimi`] olarak kalıcı duruma yazılır.  Arka Plan İşleri, İP-16 [`IsIlerleme`]
//! bileşenini (Gün-4/7) yeniden kullanır → ilerleme + tahmini süre + iptal tek yerden gelir.
// MK-52: renkler token'dan; metinler i18n'den (MK-53).

use biocraft_state::AltSekmeSecimi;
use biocraft_types::JobStatus;

use crate::ai::{ai_panel_ciz, AiPanelEylem, AiYuzey};
use crate::components::IsIlerleme;
use crate::i18n::Dil;
use crate::tokens::Tokenlar;

/// Alt Panel sekmesi (kalıcı [`AltSekmeSecimi`]'nin UI tarafı karşılığı).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AltSekme {
    /// Konsol / çıktı (varsayılan).
    #[default]
    Konsol,
    /// Arka plan işleri (ilerleme + iptal).
    Isler,
    /// AI sohbet (MVP'de yapılandırılmadı — MK-48).
    Ai,
    /// Günlük (log).
    Gunluk,
}

impl AltSekme {
    /// Şeritteki görünüm sırasıyla tüm sekmeler.
    pub const TUMU: &'static [AltSekme] = &[
        AltSekme::Konsol,
        AltSekme::Isler,
        AltSekme::Ai,
        AltSekme::Gunluk,
    ];

    /// Sekme ikonu.
    pub fn ikon(self) -> &'static str {
        match self {
            AltSekme::Konsol => "🖥",
            AltSekme::Isler => "⚙",
            AltSekme::Ai => "✨",
            AltSekme::Gunluk => "📋",
        }
    }

    /// Sekmenin yerelleştirilmiş adı.
    pub fn ad(self, dil: Dil) -> &'static str {
        match (self, dil) {
            (AltSekme::Konsol, Dil::Tr) => "Konsol",
            (AltSekme::Konsol, Dil::En) => "Console",
            (AltSekme::Isler, Dil::Tr) => "İşler",
            (AltSekme::Isler, Dil::En) => "Jobs",
            (AltSekme::Ai, Dil::Tr) => "AI",
            (AltSekme::Ai, Dil::En) => "AI",
            (AltSekme::Gunluk, Dil::Tr) => "Günlük",
            (AltSekme::Gunluk, Dil::En) => "Log",
        }
    }

    /// Kalıcı (nötr) seçimden UI sekmesine (L2 → L4).
    pub fn secimden(s: AltSekmeSecimi) -> Self {
        match s {
            AltSekmeSecimi::Konsol => AltSekme::Konsol,
            AltSekmeSecimi::Isler => AltSekme::Isler,
            AltSekmeSecimi::Ai => AltSekme::Ai,
            AltSekmeSecimi::Gunluk => AltSekme::Gunluk,
        }
    }

    /// UI sekmesinden kalıcı (nötr) seçime (L4 → L2).
    pub fn secime(self) -> AltSekmeSecimi {
        match self {
            AltSekme::Konsol => AltSekmeSecimi::Konsol,
            AltSekme::Isler => AltSekmeSecimi::Isler,
            AltSekme::Ai => AltSekmeSecimi::Ai,
            AltSekme::Gunluk => AltSekmeSecimi::Gunluk,
        }
    }
}

/// Alt Panel'in (durum + içerik) hali.  Konsol/günlük satırları + arka plan işleri burada tutulur.
pub struct AltPanel {
    /// Panel açık mı?
    pub acik: bool,
    /// Panel yüksekliği (mantıksal piksel; kalıcı).
    pub yukseklik: f32,
    /// Seçili sekme.
    pub aktif: AltSekme,
    /// Konsol/çıktı satırları.
    konsol: Vec<String>,
    /// Günlük (log) satırları.
    gunluk: Vec<String>,
    /// Arka plan işleri (İP-16 bileşeni — ilerleme/iptal).
    isler: Vec<IsIlerleme>,
}

impl Default for AltPanel {
    fn default() -> Self {
        Self::yeni()
    }
}

impl AltPanel {
    /// Örnek içerikle açılır (boş kabuk hissi vermesin; gerçek çıktı sonraki paketlerden gelir).
    pub fn yeni() -> Self {
        // Örnek arka plan işleri: biri belirli ilerlemeli, biri belirsiz, biri tamamlanmış.
        let mut tarama = IsIlerleme::yeni("Varyantlar taranıyor");
        tarama.durumu_ayarla(JobStatus::Calisiyor { ilerleme: Some(45) });
        let mut indeks = IsIlerleme::yeni("Genom indeksleniyor");
        indeks.durumu_ayarla(JobStatus::Calisiyor { ilerleme: None });
        let mut hazirlik = IsIlerleme::yeni("Önbellek hazırlandı");
        hazirlik.durumu_ayarla(JobStatus::Bitti);

        Self {
            acik: true,
            yukseklik: 180.0,
            aktif: AltSekme::Konsol,
            konsol: vec![
                "BioCraft Engine — kabuk başlatıldı.".to_string(),
                "Render host hazır.".to_string(),
            ],
            gunluk: vec!["[bilgi] Oturum başladı.".to_string()],
            isler: vec![tarama, indeks, hazirlik],
        }
    }

    /// Konsola bir satır yazar (en fazla son 200 satır tutulur).
    pub fn konsol_yaz(&mut self, satir: impl Into<String>) {
        self.konsol.push(satir.into());
        if self.konsol.len() > 200 {
            self.konsol.remove(0);
        }
    }

    /// Günlüğe bir satır yazar (en fazla son 200 satır tutulur).
    pub fn gunluk_yaz(&mut self, satir: impl Into<String>) {
        self.gunluk.push(satir.into());
        if self.gunluk.len() > 200 {
            self.gunluk.remove(0);
        }
    }

    /// Çalışan (iptal edilmemiş, bitmemiş) iş sayısı — Status Bar özetinde kullanılır.
    pub fn calisan_sayisi(&self) -> usize {
        self.isler
            .iter()
            .filter(|i| matches!(i.durum, JobStatus::Calisiyor { .. }) && !i.iptal_istendi())
            .count()
    }
}

/// Alt Panel'i Status Bar'ın hemen üstüne çizer; `(güncel yükseklik, AI panel eylemi)` döner.
///
/// AI sekmesi gerçek AI yüzeyini ([`ai_panel_ciz`]) çizer; ürettiği eylem (sağlayıcı ekle / AI'ı
/// aç / eylem uygula) çağırana iletilir.
///
/// **Çizim sırası önemli:** Status Bar bu panelden ÖNCE eklenmelidir (egui alt panelleri ekleme
/// sırasına göre dipten yukarı yığar) → bu panel Status Bar'ın üstünde oturur.
pub fn alt_panel_ciz(
    ctx: &egui::Context,
    panel: &mut AltPanel,
    ai: &mut AiYuzey,
    dil: Dil,
    tok: &Tokenlar,
) -> (f32, Option<AiPanelEylem>) {
    let mut olculen = panel.yukseklik;
    let mut ai_eylem = None;
    egui::TopBottomPanel::bottom("biocraft_alt_panel")
        .resizable(true)
        .default_height(panel.yukseklik)
        .height_range(80.0..=600.0)
        .show(ctx, |ui| {
            // Sekme şeridi.
            ui.add_space(tok.bosluk.xs);
            ui.horizontal(|ui| {
                for &s in AltSekme::TUMU {
                    let secili = s == panel.aktif;
                    let renk = if secili {
                        tok.renk.vurgu
                    } else {
                        tok.renk.metin_soluk
                    };
                    let etiket = format!("{} {}", s.ikon(), s.ad(dil));
                    // İşler sekmesinde çalışan iş sayısını rozet gibi göster.
                    let etiket = if s == AltSekme::Isler && panel.calisan_sayisi() > 0 {
                        format!("{etiket} ({})", panel.calisan_sayisi())
                    } else {
                        etiket
                    };
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new(etiket).color(renk))
                                .frame(secili),
                        )
                        .clicked()
                    {
                        panel.aktif = s;
                    }
                }
            });
            ui.separator();

            // Seçili sekme içeriği.  AI sekmesi kendi kaydırmasını yönetir (sohbet + girdi şeridi);
            // diğer sekmeler ortak ScrollArea kullanır.
            match panel.aktif {
                AltSekme::Ai => {
                    ai_eylem = ai_panel_ciz(ui, ai, dil, tok);
                }
                aktif => {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| match aktif {
                            AltSekme::Konsol => metin_listesi(ui, &panel.konsol, tok),
                            AltSekme::Gunluk => metin_listesi(ui, &panel.gunluk, tok),
                            AltSekme::Isler => isler_ciz(ui, &mut panel.isler, dil, tok),
                            AltSekme::Ai => unreachable!(),
                        });
                }
            }

            olculen = ui.min_rect().height();
        });
    (olculen.clamp(80.0, 600.0), ai_eylem)
}

/// Konsol/günlük satırlarını monospace ile çizer.
fn metin_listesi(ui: &mut egui::Ui, satirlar: &[String], tok: &Tokenlar) {
    if satirlar.is_empty() {
        ui.weak("—");
        return;
    }
    for s in satirlar {
        ui.label(egui::RichText::new(s).monospace().color(tok.renk.metin));
    }
}

/// Arka plan işlerini çizer (her biri ilerleme + iptal — İP-16 bileşeni).
fn isler_ciz(ui: &mut egui::Ui, isler: &mut [IsIlerleme], dil: Dil, tok: &Tokenlar) {
    if isler.is_empty() {
        ui.weak(if matches!(dil, Dil::Tr) {
            "Aktif iş yok."
        } else {
            "No active jobs."
        });
        return;
    }
    for is_ in isler.iter_mut() {
        // İptal istenirse bileşen kendi durumunu işaretler (cancellation token — MK-11).
        let _ = is_.show(ui, dil, tok);
        ui.add_space(tok.bosluk.xs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sekme_secim_gidis_donus() {
        for &s in AltSekme::TUMU {
            assert_eq!(AltSekme::secimden(s.secime()), s);
            assert!(!s.ad(Dil::Tr).is_empty());
            assert!(!s.ad(Dil::En).is_empty());
        }
        assert_eq!(AltSekme::TUMU.len(), 4);
    }

    #[test]
    fn konsol_gunluk_son_200_satir() {
        let mut p = AltPanel::yeni();
        for i in 0..250 {
            p.konsol_yaz(format!("satır {i}"));
        }
        assert!(p.konsol.len() <= 200, "konsol son 200 satıra kırpılır");
    }

    #[test]
    fn calisan_sayisi_dogru() {
        let p = AltPanel::yeni();
        // Örnek: biri belirli (45%), biri belirsiz → 2 çalışan; biri Bitti.
        assert_eq!(p.calisan_sayisi(), 2);
    }
}
