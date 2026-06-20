//! Bildirim / toast bileşeni (İP-16, TDA madde 15 — son işlem geri bildirimi).
//!
//! - Dört tür: başarı / uyarı / hata / bilgi.
//! - Otomatik kapanma **veya** kalıcı (manuel kapatma) seçeneği.
//! - Opsiyonel eylem butonu (ör. "Geri al", "Göster").
//! - **Tür bazında kısma:** aynı türden çok hızlı gelen bildirimler bastırılır
//!   (kullanıcıyı toast yağmuruna tutmamak için; İP-12 ayarıyla eşik değişebilir).
//!
//! Zaman bağımlı mantık (otomatik kapanma + kısma) `now: f64` parametresiyle saf tutulur;
//! bu sayede egui'siz birim testi yapılabilir.

use std::collections::HashMap;

use crate::i18n::{ceviri, Anahtar, Dil};
use crate::tokens::{Onem, Tokenlar};

/// Bildirim türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToastKind {
    /// Başarı (yeşil).
    Basari,
    /// Uyarı (amber).
    Uyari,
    /// Hata (kırmızı).
    Hata,
    /// Bilgi (mavi).
    Bilgi,
}

impl ToastKind {
    fn onem(self) -> Onem {
        match self {
            ToastKind::Basari => Onem::Basari,
            ToastKind::Uyari => Onem::Uyari,
            ToastKind::Hata => Onem::Hata,
            ToastKind::Bilgi => Onem::Bilgi,
        }
    }

    fn ikon(self) -> &'static str {
        match self {
            ToastKind::Basari => "✔",
            ToastKind::Uyari => "⚠",
            ToastKind::Hata => "✖",
            ToastKind::Bilgi => "ℹ",
        }
    }

    fn baslik(self, dil: Dil) -> &'static str {
        match self {
            ToastKind::Basari => ceviri(dil, Anahtar::BildirimBasari),
            ToastKind::Uyari => ceviri(dil, Anahtar::BildirimUyari),
            ToastKind::Hata => ceviri(dil, Anahtar::BildirimHata),
            ToastKind::Bilgi => ceviri(dil, Anahtar::BildirimBilgi),
        }
    }
}

/// Tek bir bildirim tanımı.
#[derive(Debug, Clone)]
pub struct Toast {
    /// Bildirim türü.
    pub kind: ToastKind,
    /// Kullanıcıya gösterilecek mesaj.
    pub mesaj: String,
    /// Opsiyonel eylem butonu etiketi.
    pub eylem_etiketi: Option<String>,
    /// `true` ise otomatik kapanmaz; yalnızca kullanıcı kapatır.
    pub kalici: bool,
    /// Otomatik kapanma süresi (saniye); `kalici` ise yok sayılır.
    pub sure_sn: f32,
}

impl Toast {
    /// Verilen tür ve mesajla varsayılan (4 sn, otomatik kapanan) bir toast kurar.
    pub fn yeni(kind: ToastKind, mesaj: impl Into<String>) -> Self {
        Self {
            kind,
            mesaj: mesaj.into(),
            eylem_etiketi: None,
            kalici: false,
            sure_sn: 4.0,
        }
    }

    /// Başarı bildirimi.
    pub fn basari(mesaj: impl Into<String>) -> Self {
        Self::yeni(ToastKind::Basari, mesaj)
    }
    /// Uyarı bildirimi.
    pub fn uyari(mesaj: impl Into<String>) -> Self {
        Self::yeni(ToastKind::Uyari, mesaj)
    }
    /// Hata bildirimi (varsayılan kalıcı — kaybolmasın).
    pub fn hata(mesaj: impl Into<String>) -> Self {
        Self::yeni(ToastKind::Hata, mesaj).kalici()
    }
    /// Bilgi bildirimi.
    pub fn bilgi(mesaj: impl Into<String>) -> Self {
        Self::yeni(ToastKind::Bilgi, mesaj)
    }

    /// Eylem butonu ekler.
    pub fn with_eylem(mut self, etiket: impl Into<String>) -> Self {
        self.eylem_etiketi = Some(etiket.into());
        self
    }
    /// Bildirimi kalıcı yapar (otomatik kapanmaz).
    pub fn kalici(mut self) -> Self {
        self.kalici = true;
        self
    }
    /// Otomatik kapanma süresini ayarlar.
    pub fn sure(mut self, sn: f32) -> Self {
        self.sure_sn = sn;
        self.kalici = false;
        self
    }
}

#[derive(Debug, Clone)]
struct AktifToast {
    toast: Toast,
    id: u64,
    dogdu: f64,
}

/// Bir kullanıcı etkileşimi sonucu çıkan toast olayı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToastEylem {
    /// Eylem butonuna tıklandı (toast id'si ile).
    EylemTiklandi(u64),
    /// Bildirim kapatıldı (otomatik veya manuel; toast id'si ile).
    Kapatildi(u64),
}

/// Aktif bildirimleri yöneten ve çizen yönetici.
pub struct ToastManager {
    aktifler: Vec<AktifToast>,
    sonraki_id: u64,
    son_gosterim: HashMap<ToastKind, f64>,
    /// Aynı türden iki bildirim arasındaki en kısa süre (saniye); altındakiler bastırılır.
    pub kisma_araligi_sn: f64,
    /// Aynı anda gösterilecek en fazla bildirim sayısı (eskiler düşer).
    pub maks_gorunur: usize,
}

impl Default for ToastManager {
    fn default() -> Self {
        Self {
            aktifler: Vec::new(),
            sonraki_id: 0,
            son_gosterim: HashMap::new(),
            kisma_araligi_sn: 1.0,
            maks_gorunur: 5,
        }
    }
}

impl ToastManager {
    /// Yeni, boş bir yönetici.
    pub fn new() -> Self {
        Self::default()
    }

    /// Belirtilen zamanda bildirim ekler.  Aynı türden çok hızlı geldiyse (kısma)
    /// `None`, eklendiyse `Some(id)` döner.  Testler için `now` açıkça verilir.
    pub fn ekle_zamanli(&mut self, toast: Toast, now: f64) -> Option<u64> {
        if let Some(&son) = self.son_gosterim.get(&toast.kind) {
            if now - son < self.kisma_araligi_sn {
                return None; // kısma: aynı türden çok sık → bastır
            }
        }
        self.son_gosterim.insert(toast.kind, now);
        let id = self.sonraki_id;
        self.sonraki_id += 1;
        self.aktifler.push(AktifToast {
            toast,
            id,
            dogdu: now,
        });
        // Üst sınırı aşarsak en eskiyi düşür.
        while self.aktifler.len() > self.maks_gorunur {
            self.aktifler.remove(0);
        }
        Some(id)
    }

    /// Zaman ilerledikçe süresi dolan (kalıcı olmayan) bildirimleri kaldırır;
    /// kaldırılanların id'lerini döndürür.
    pub fn guncelle(&mut self, now: f64) -> Vec<u64> {
        let mut dusenler = Vec::new();
        self.aktifler.retain(|a| {
            if a.toast.kalici {
                return true;
            }
            if now - a.dogdu >= a.toast.sure_sn as f64 {
                dusenler.push(a.id);
                false
            } else {
                true
            }
        });
        dusenler
    }

    /// Verilen bildirimi kapatır (id ile).
    pub fn kapat(&mut self, id: u64) {
        self.aktifler.retain(|a| a.id != id);
    }

    /// Tüm bildirimleri temizler.
    pub fn temizle(&mut self) {
        self.aktifler.clear();
    }

    /// Şu an gösterilen bildirim sayısı.
    pub fn aktif_sayisi(&self) -> usize {
        self.aktifler.len()
    }

    /// egui bağlamının saatini kullanarak bildirim ekler (uygulama içi pratik yol).
    pub fn ekle(&mut self, ctx: &egui::Context, toast: Toast) -> Option<u64> {
        let now = ctx.input(|i| i.time);
        self.ekle_zamanli(toast, now)
    }

    /// Bildirimleri sağ-üstte çizer, otomatik kapanmayı işler ve olayları döndürür.
    pub fn show(&mut self, ctx: &egui::Context, dil: Dil, tok: &Tokenlar) -> Vec<ToastEylem> {
        let now = ctx.input(|i| i.time);
        let mut olaylar: Vec<ToastEylem> = self
            .guncelle(now)
            .into_iter()
            .map(ToastEylem::Kapatildi)
            .collect();

        // Süre dolmasını canlandırmak için, kalıcı olmayan bildirim varken sürekli yeniden çiz.
        if self.aktifler.iter().any(|a| !a.toast.kalici) {
            ctx.request_repaint();
        }

        if self.aktifler.is_empty() {
            return olaylar;
        }

        let anlik: Vec<AktifToast> = self.aktifler.clone();
        egui::Area::new(egui::Id::new("biocraft_toasts"))
            .anchor(
                egui::Align2::RIGHT_TOP,
                egui::vec2(-tok.bosluk.l, tok.bosluk.l),
            )
            .interactable(true)
            .show(ctx, |ui| {
                ui.set_max_width(360.0);
                for a in &anlik {
                    let onem = a.toast.kind.onem();
                    let kenar = tok.onem_rengi(onem);
                    let zemin = tok.onem_zemini(onem);
                    // Son 0.5 sn'de yumuşak kaybolma.
                    let alpha = if a.toast.kalici {
                        1.0
                    } else {
                        let kalan = a.toast.sure_sn as f64 - (now - a.dogdu);
                        (kalan / 0.5).clamp(0.0, 1.0) as f32
                    };
                    ui.scope(|ui| {
                        ui.set_opacity(alpha);
                        crate::components::vurgu_cercevesi(zemin, kenar, tok).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(a.toast.kind.ikon())
                                        .color(kenar)
                                        .strong(),
                                );
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(a.toast.kind.baslik(dil))
                                            .strong()
                                            .color(tok.renk.metin),
                                    );
                                    ui.label(
                                        egui::RichText::new(&a.toast.mesaj).color(tok.renk.metin),
                                    );
                                    ui.horizontal(|ui| {
                                        if let Some(e) = &a.toast.eylem_etiketi {
                                            if ui.button(e).clicked() {
                                                olaylar.push(ToastEylem::EylemTiklandi(a.id));
                                            }
                                        }
                                        if ui.button(ceviri(dil, Anahtar::Kapat)).clicked() {
                                            olaylar.push(ToastEylem::Kapatildi(a.id));
                                        }
                                    });
                                });
                            });
                        });
                    });
                    ui.add_space(tok.bosluk.s);
                }
            });

        // Tıklanarak kapatılanları/eylem verilenleri listeden düş.
        for olay in &olaylar {
            match olay {
                ToastEylem::Kapatildi(id) | ToastEylem::EylemTiklandi(id) => self.kapat(*id),
            }
        }
        olaylar
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ekleme_ve_otomatik_kapanma() {
        let mut m = ToastManager::new();
        let id = m.ekle_zamanli(Toast::basari("ok").sure(4.0), 0.0);
        assert!(id.is_some());
        assert_eq!(m.aktif_sayisi(), 1);

        // 2 sn sonra hâlâ duruyor.
        assert!(m.guncelle(2.0).is_empty());
        assert_eq!(m.aktif_sayisi(), 1);

        // 4 sn sonra otomatik kapanır.
        let dusen = m.guncelle(4.0);
        assert_eq!(dusen.len(), 1);
        assert_eq!(m.aktif_sayisi(), 0);
    }

    #[test]
    fn kalici_toast_otomatik_kapanmaz() {
        let mut m = ToastManager::new();
        m.ekle_zamanli(Toast::hata("kritik"), 0.0); // hata varsayılan kalıcı
                                                    // Çok zaman geçse de kalıcı olduğu için kalır.
        assert!(m.guncelle(1000.0).is_empty());
        assert_eq!(m.aktif_sayisi(), 1);
    }

    #[test]
    fn tur_bazinda_kisma_bastirir() {
        let mut m = ToastManager::new();
        m.kisma_araligi_sn = 1.0;
        // Aynı türden art arda iki bildirim: ikincisi (0.5 sn) bastırılır.
        assert!(m.ekle_zamanli(Toast::uyari("1"), 0.0).is_some());
        assert!(m.ekle_zamanli(Toast::uyari("2"), 0.5).is_none());
        // Aralık dolunca tekrar kabul edilir.
        assert!(m.ekle_zamanli(Toast::uyari("3"), 1.2).is_some());
        // Farklı tür kısmadan etkilenmez.
        assert!(m.ekle_zamanli(Toast::bilgi("x"), 0.6).is_some());
    }

    #[test]
    fn maks_gorunur_eskiyi_dusurur() {
        let mut m = ToastManager::new();
        m.maks_gorunur = 2;
        m.kisma_araligi_sn = 0.0; // kısmayı kapat
        m.ekle_zamanli(Toast::bilgi("1"), 0.0);
        m.ekle_zamanli(Toast::bilgi("2"), 0.1);
        m.ekle_zamanli(Toast::bilgi("3"), 0.2);
        assert_eq!(m.aktif_sayisi(), 2);
    }
}
