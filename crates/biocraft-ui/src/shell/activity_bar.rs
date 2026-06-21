//! Activity Bar (48 px sol şerit) — ana mod ikonları (İP-03, 0.9 tablosu).
//!
//! Her ikon bir [`ActivityMod`]'a karşılık gelir; aktif mod **Side Panel içeriğini** belirler
//! (örn. Proje → dosya ağacı, Eklentiler → eklenti listesi).  Mod, kalıcı durumda (İP-11)
//! nötr [`AktifModSecimi`] olarak saklanır; burada UI tarafındaki zengin enum'a eşlenir.
//!
//! Eklentiler ileride kendi modunu/ikonunu ekleyebilir (MK-17 üzerinden); bu sürümde sabit altı mod.
// MK-52: renkler token'dan; bu modül sabit renk üretmez.

use biocraft_state::AktifModSecimi;

use crate::i18n::Dil;
use crate::shell::layout::AKTIVITE_GENISLIK;
use crate::tokens::Tokenlar;

/// Activity Bar'da seçilebilen ana mod.  Aktif mod Side Panel içeriğini değiştirir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivityMod {
    /// Proje gezgini (dosya/şablon ağacı) — varsayılan.
    #[default]
    Proje,
    /// Eklenti yönetimi (kurulu/pazar).
    Eklentiler,
    /// Arama (proje/genom içi).
    Arama,
    /// AI yüzeyi (MVP'de "yapılandırılmadı" — MK-48).
    Ai,
    /// Veritabanı bağlantıları/sorgu.
    Veritabani,
    /// Ayarlar.
    Ayar,
}

impl ActivityMod {
    /// Activity Bar'da görünen sırayla tüm modlar.
    pub const TUMU: &'static [ActivityMod] = &[
        ActivityMod::Proje,
        ActivityMod::Eklentiler,
        ActivityMod::Arama,
        ActivityMod::Ai,
        ActivityMod::Veritabani,
        ActivityMod::Ayar,
    ];

    /// Mod ikonu (tema-bağımsız sembol).  Tek renk yok; ikon token rengiyle boyanır.
    pub fn ikon(self) -> &'static str {
        match self {
            ActivityMod::Proje => "📁",
            ActivityMod::Eklentiler => "🧩",
            ActivityMod::Arama => "🔍",
            ActivityMod::Ai => "✨",
            ActivityMod::Veritabani => "🗄",
            ActivityMod::Ayar => "⚙",
        }
    }

    /// Modun yerelleştirilmiş kısa adı (ipucu + Side Panel başlığı).
    pub fn baslik(self, dil: Dil) -> &'static str {
        match (self, dil) {
            (ActivityMod::Proje, Dil::Tr) => "Proje",
            (ActivityMod::Proje, Dil::En) => "Project",
            (ActivityMod::Eklentiler, Dil::Tr) => "Eklentiler",
            (ActivityMod::Eklentiler, Dil::En) => "Plugins",
            (ActivityMod::Arama, Dil::Tr) => "Arama",
            (ActivityMod::Arama, Dil::En) => "Search",
            (ActivityMod::Ai, Dil::Tr) => "AI",
            (ActivityMod::Ai, Dil::En) => "AI",
            (ActivityMod::Veritabani, Dil::Tr) => "Veritabanı",
            (ActivityMod::Veritabani, Dil::En) => "Database",
            (ActivityMod::Ayar, Dil::Tr) => "Ayarlar",
            (ActivityMod::Ayar, Dil::En) => "Settings",
        }
    }

    /// Kalıcı (nötr) mod seçiminden UI moduna eşler (L2 → L4).
    pub fn secimden(s: AktifModSecimi) -> Self {
        match s {
            AktifModSecimi::Proje => ActivityMod::Proje,
            AktifModSecimi::Eklentiler => ActivityMod::Eklentiler,
            AktifModSecimi::Arama => ActivityMod::Arama,
            AktifModSecimi::Ai => ActivityMod::Ai,
            AktifModSecimi::Veritabani => ActivityMod::Veritabani,
            AktifModSecimi::Ayar => ActivityMod::Ayar,
        }
    }

    /// UI modunu kalıcı (nötr) mod seçimine eşler (L4 → L2).
    pub fn secime(self) -> AktifModSecimi {
        match self {
            ActivityMod::Proje => AktifModSecimi::Proje,
            ActivityMod::Eklentiler => AktifModSecimi::Eklentiler,
            ActivityMod::Arama => AktifModSecimi::Arama,
            ActivityMod::Ai => AktifModSecimi::Ai,
            ActivityMod::Veritabani => AktifModSecimi::Veritabani,
            ActivityMod::Ayar => AktifModSecimi::Ayar,
        }
    }
}

/// Activity Bar'ı çizer (48 px genişlikte, yeniden boyutlanmaz sol şerit).
///
/// Her mod için dikey bir ikon-buton sunar; aktif mod token vurgu rengiyle belirginleşir.
/// Kullanıcı bir moda tıklarsa `aktif` güncellenir ve `true` döner (Side Panel içeriği yenilenir).
/// Renkler token'dan gelir (MK-52); ikon ipuçları i18n'dendir (MK-53).
pub fn aktivite_cubugu(
    ctx: &egui::Context,
    aktif: &mut ActivityMod,
    dil: Dil,
    tok: &Tokenlar,
) -> bool {
    let mut degisti = false;
    egui::SidePanel::left("biocraft_aktivite")
        .exact_width(AKTIVITE_GENISLIK)
        .resizable(false)
        .show(ctx, |ui| {
            // Şerit, ana zeminden bir ton koyu/açık yüzeyle ayrışır.
            ui.add_space(tok.bosluk.xs);
            ui.vertical_centered(|ui| {
                for &mod_ in ActivityMod::TUMU {
                    let secili = mod_ == *aktif;
                    // Aktif modu vurgu rengiyle, diğerlerini soluk metinle çiz (token).
                    let renk = if secili {
                        tok.renk.vurgu
                    } else {
                        tok.renk.metin_soluk
                    };
                    let dugme =
                        egui::Button::new(egui::RichText::new(mod_.ikon()).size(20.0).color(renk))
                            .frame(secili)
                            .min_size(egui::vec2(36.0, 36.0));
                    let yanit = ui.add(dugme).on_hover_text(mod_.baslik(dil));
                    if yanit.clicked() && !secili {
                        *aktif = mod_;
                        degisti = true;
                    }
                    ui.add_space(tok.bosluk.xs);
                }
            });
        });
    degisti
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tum_modlar_listede_ve_benzersiz() {
        assert_eq!(ActivityMod::TUMU.len(), 6);
        // Eşleme gidiş-dönüş tutarlı (L4↔L2).
        for &m in ActivityMod::TUMU {
            assert_eq!(ActivityMod::secimden(m.secime()), m);
        }
    }

    #[test]
    fn ikon_ve_baslik_dolu_ve_iki_dilli() {
        for &m in ActivityMod::TUMU {
            assert!(!m.ikon().is_empty(), "ikon boş: {m:?}");
            assert!(!m.baslik(Dil::Tr).is_empty(), "TR başlık boş: {m:?}");
            assert!(!m.baslik(Dil::En).is_empty(), "EN başlık boş: {m:?}");
        }
    }

    #[test]
    fn varsayilan_mod_proje() {
        assert_eq!(ActivityMod::default(), ActivityMod::Proje);
        assert_eq!(
            ActivityMod::secimden(AktifModSecimi::default()),
            ActivityMod::Proje
        );
    }
}
