//! Title Bar (32 px üst) — marka + klasik menü + komut paleti tetikleyici + hızlı eylemler (İP-03).
//!
//! 0.9 tablosu: "Başlık, pencere kontrolleri, klasik menü, komut paleti tetikleyici, hızlı
//! eylemler".  Klasik menü ([`menu_bar`](super::menu_bar)) ile hızlı eylemler **aynı**
//! [`KabukAksiyon`] tanımını üretir; iki erişim yolu, tek davranış (MK-53).  Pencere kontrolleri
//! (kapat/küçült/büyüt) işletim sistemi dekorasyonundadır; özel başlık çubuğu sürükleme v1.x.
// MK-52: tüm renkler aktif token temasından; sabit renk yok.

use crate::i18n::Dil;
use crate::shell::layout::BASLIK_YUKSEKLIK;
use crate::shell::menu_bar::{menu_cubugu, KabukAksiyon};
use crate::tokens::{Tema, Tokenlar};

/// Title Bar'ı çizer ve seçilen [`KabukAksiyon`]'u döner (menü **veya** hızlı eylemlerden).
///
/// - `tema`/`dil`: hızlı eylem butonlarının etiketini güncel duruma göre yazmak için.
/// - `geri_al`/`yinele`: Düzen menüsündeki ilgili öğelerin etkinliği (global geçmiş durumu).
///
/// Renkler token'dan (MK-52); etiketler i18n'den (MK-53).
pub fn baslik_cubugu(
    ctx: &egui::Context,
    dil: Dil,
    tema: Tema,
    tok: &Tokenlar,
    geri_al: bool,
    yinele: bool,
) -> Option<KabukAksiyon> {
    let mut aksiyon: Option<KabukAksiyon> = None;
    egui::TopBottomPanel::top("biocraft_baslik")
        .exact_height(BASLIK_YUKSEKLIK)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                // Marka: küçük amblem + ad (vurgu renginde amblem).
                ui.label(egui::RichText::new("🧬").size(16.0).color(tok.renk.vurgu));
                ui.label(egui::RichText::new("BioCraft Engine").strong());
                ui.separator();

                // Klasik menü çubuğu (Dosya/Düzen/Görünüm/Eklenti/Yardım).
                egui::menu::bar(ui, |ui| {
                    if let Some(a) = menu_cubugu(ui, dil, geri_al, yinele) {
                        aksiyon = Some(a);
                    }
                });

                // Hızlı eylemler sağa yaslı: komut paleti + tema + dil.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Dil hızlı geçişi (TR/EN) — menü "Dili Değiştir" ile aynı aksiyon.
                    if ui
                        .button(dil.kisa())
                        .on_hover_text(KabukAksiyon::DilDegistir.etiket(dil))
                        .clicked()
                    {
                        aksiyon = Some(KabukAksiyon::DilDegistir);
                    }
                    // Tema hızlı geçişi — etiket bir sonraki temaya geçişi anlatır.
                    if ui
                        .button(tema.dugme_etiketi(dil))
                        .on_hover_text(KabukAksiyon::TemaDegistir.etiket(dil))
                        .clicked()
                    {
                        aksiyon = Some(KabukAksiyon::TemaDegistir);
                    }
                    ui.separator();
                    // Komut paleti tetikleyici (güç kullanıcı yolu — İP-13 ile dolacak).
                    let palet = match dil {
                        Dil::Tr => "⌘ Komut Paleti",
                        Dil::En => "⌘ Command Palette",
                    };
                    if ui
                        .button(egui::RichText::new(palet).color(tok.renk.metin_soluk))
                        .on_hover_text(KabukAksiyon::KomutPaleti.kisayol().unwrap_or_default())
                        .clicked()
                    {
                        aksiyon = Some(KabukAksiyon::KomutPaleti);
                    }
                });
            });
        });
    aksiyon
}
