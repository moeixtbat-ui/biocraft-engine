//! Klasik menü çubuğu (Dosya/Düzen/Görünüm/Eklenti/Yardım) — İP-03.
//!
//! **Tek komut tanımı, iki erişim yolu.**  Her menü öğesi bir [`KabukAksiyon`] üretir; ileride
//! komut paleti (İP-13) de **aynı** enum'u üretecek.  Böylece "menü ile palet çakışır" sorunu
//! oluşmaz: iki yüzey de tek aksiyon tanımına bağlıdır, davranış tek yerde tanımlıdır (MK-53).
//!
//! Kısayollar burada yalnızca **görsel ipucu** olarak gösterilir; gerçek klavye bağlama İP-13'te
//! tuş-profilleriyle (Vim/Emacs kancası dahil) gelir.
// MK-52: renkler token'dan; bu modül metin/etiket üretir, sabit renk üretmez.

use crate::i18n::Dil;

/// Kabukta tetiklenebilen aksiyonlar (menü + ileride komut paleti ortak tanımı).
///
/// `biocraft-app` bu aksiyonu uygular (tema değiştir, çıkış, panel aç/kapa…).  Bazı aksiyonlar
/// (Geri Al/Yinele, Proje Aç…) ilgili paket gelene kadar yer tutucudur; menüde **devre dışı**
/// (gri) görünürler — "çalışıyormuş gibi" gösterilmez (MK-48 ruhu, TDA madde 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KabukAksiyon {
    // ── Dosya ──
    /// Yeni proje (İP-02 sihirbazı bağlanınca etkin).
    YeniProje,
    /// Proje aç (İP-01/02 bağlanınca etkin).
    ProjeAc,
    /// Kaydet (gerçek belge akışı İP-05/06 ile).
    Kaydet,
    /// Uygulamadan çık.
    Cikis,
    // ── Düzen ──
    /// Geri al (global belge geçmişi İP-05/06 ile etkin).
    GeriAl,
    /// Yinele.
    Yinele,
    // ── Görünüm ──
    /// Temayı döngüsel değiştir (Koyu→Açık→Yüksek Kontrast).
    TemaDegistir,
    /// Dili değiştir (TR↔EN).
    DilDegistir,
    /// Side Panel'i aç/kapa.
    YanPanelAcKapa,
    /// Komut paletini aç (İP-13 — şimdilik bilgilendirici yer tutucu).
    KomutPaleti,
    // ── Eklenti ──
    /// Eklentileri yönet (İP-07 host'u ile).
    EklentileriYonet,
    // ── Yardım ──
    /// Belgeler.
    Belgeler,
    /// Hakkında.
    Hakkinda,
}

impl KabukAksiyon {
    /// Aksiyonun yerelleştirilmiş menü etiketi.
    pub fn etiket(self, dil: Dil) -> &'static str {
        use Dil::{En, Tr};
        use KabukAksiyon::*;
        match (self, dil) {
            (YeniProje, Tr) => "Yeni Proje",
            (YeniProje, En) => "New Project",
            (ProjeAc, Tr) => "Proje Aç…",
            (ProjeAc, En) => "Open Project…",
            (Kaydet, Tr) => "Kaydet",
            (Kaydet, En) => "Save",
            (Cikis, Tr) => "Çıkış",
            (Cikis, En) => "Exit",
            (GeriAl, Tr) => "Geri Al",
            (GeriAl, En) => "Undo",
            (Yinele, Tr) => "Yinele",
            (Yinele, En) => "Redo",
            (TemaDegistir, Tr) => "Temayı Değiştir",
            (TemaDegistir, En) => "Switch Theme",
            (DilDegistir, Tr) => "Dili Değiştir",
            (DilDegistir, En) => "Switch Language",
            (YanPanelAcKapa, Tr) => "Yan Paneli Aç/Kapa",
            (YanPanelAcKapa, En) => "Toggle Side Panel",
            (KomutPaleti, Tr) => "Komut Paleti…",
            (KomutPaleti, En) => "Command Palette…",
            (EklentileriYonet, Tr) => "Eklentileri Yönet…",
            (EklentileriYonet, En) => "Manage Plugins…",
            (Belgeler, Tr) => "Belgeler",
            (Belgeler, En) => "Documentation",
            (Hakkinda, Tr) => "Hakkında",
            (Hakkinda, En) => "About",
        }
    }

    /// Görsel kısayol ipucu (gerçek bağlama İP-13'te); yoksa `None`.
    pub fn kisayol(self) -> Option<&'static str> {
        use KabukAksiyon::*;
        match self {
            YeniProje => Some("Ctrl+N"),
            ProjeAc => Some("Ctrl+O"),
            Kaydet => Some("Ctrl+S"),
            Cikis => Some("Ctrl+Q"),
            GeriAl => Some("Ctrl+Z"),
            Yinele => Some("Ctrl+Y"),
            KomutPaleti => Some("Ctrl+Shift+P"),
            _ => None,
        }
    }

    /// Bu aksiyon bu sürümde işlevsel mi?  `false` ise menüde devre dışı (gri) gösterilir
    /// (ilgili paket henüz bağlı değil — sahte "çalışıyor" görüntüsü vermemek için).
    pub fn etkin_mi(self) -> bool {
        use KabukAksiyon::*;
        matches!(
            self,
            TemaDegistir | DilDegistir | YanPanelAcKapa | KomutPaleti | Hakkinda | Cikis
        )
    }
}

/// Bir menü başlığını yerelleştirir.
fn menu_basligi(menu: Menu, dil: Dil) -> &'static str {
    use Dil::{En, Tr};
    match (menu, dil) {
        (Menu::Dosya, Tr) => "Dosya",
        (Menu::Dosya, En) => "File",
        (Menu::Duzen, Tr) => "Düzen",
        (Menu::Duzen, En) => "Edit",
        (Menu::Gorunum, Tr) => "Görünüm",
        (Menu::Gorunum, En) => "View",
        (Menu::Eklenti, Tr) => "Eklenti",
        (Menu::Eklenti, En) => "Plugins",
        (Menu::Yardim, Tr) => "Yardım",
        (Menu::Yardim, En) => "Help",
    }
}

/// Klasik menü çubuğunun beş üst başlığı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Menu {
    Dosya,
    Duzen,
    Gorunum,
    Eklenti,
    Yardim,
}

/// Her menünün içerdiği aksiyonlar (sırayla; `None` = ayraç).
fn menu_ogeleri(menu: Menu) -> &'static [Option<KabukAksiyon>] {
    use KabukAksiyon::*;
    match menu {
        Menu::Dosya => &[
            Some(YeniProje),
            Some(ProjeAc),
            Some(Kaydet),
            None,
            Some(Cikis),
        ],
        Menu::Duzen => &[Some(GeriAl), Some(Yinele)],
        Menu::Gorunum => &[
            Some(TemaDegistir),
            Some(DilDegistir),
            Some(YanPanelAcKapa),
            None,
            Some(KomutPaleti),
        ],
        Menu::Eklenti => &[Some(EklentileriYonet)],
        Menu::Yardim => &[Some(Belgeler), Some(Hakkinda)],
    }
}

/// Klasik menü çubuğunu verili `ui` (genelde Title Bar satırı) içine çizer.
///
/// Tıklanan aksiyonu döner.  `geri_al`/`yinele`: global belge geçmişinin o anki kullanılabilirliği
/// (yoksa ilgili öğeler devre dışı).  Renkler aktif token temasından gelir (MK-52).
pub fn menu_cubugu(
    ui: &mut egui::Ui,
    dil: Dil,
    geri_al: bool,
    yinele: bool,
) -> Option<KabukAksiyon> {
    let mut secilen = None;
    for menu in [
        Menu::Dosya,
        Menu::Duzen,
        Menu::Gorunum,
        Menu::Eklenti,
        Menu::Yardim,
    ] {
        ui.menu_button(menu_basligi(menu, dil), |ui| {
            for oge in menu_ogeleri(menu) {
                match oge {
                    None => {
                        ui.separator();
                    }
                    Some(aksiyon) => {
                        // Geri Al/Yinele'nin etkinliği o anki geçmişe bağlı; diğerleri statik.
                        let etkin = match aksiyon {
                            KabukAksiyon::GeriAl => geri_al,
                            KabukAksiyon::Yinele => yinele,
                            a => a.etkin_mi(),
                        };
                        // "Etiket … Kısayol" tek satırda; kısayol sağa yaslı ipucu.
                        let metin = match aksiyon.kisayol() {
                            Some(k) => format!("{}\t{}", aksiyon.etiket(dil), k),
                            None => aksiyon.etiket(dil).to_string(),
                        };
                        if ui.add_enabled(etkin, egui::Button::new(metin)).clicked() {
                            secilen = Some(*aksiyon);
                            ui.close_menu();
                        }
                    }
                }
            }
        });
    }
    secilen
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test kapsamı için tüm aksiyonlar (etiket/kısayol bütünlüğü).
    const TUM_AKSIYONLAR: &[KabukAksiyon] = &[
        KabukAksiyon::YeniProje,
        KabukAksiyon::ProjeAc,
        KabukAksiyon::Kaydet,
        KabukAksiyon::Cikis,
        KabukAksiyon::GeriAl,
        KabukAksiyon::Yinele,
        KabukAksiyon::TemaDegistir,
        KabukAksiyon::DilDegistir,
        KabukAksiyon::YanPanelAcKapa,
        KabukAksiyon::KomutPaleti,
        KabukAksiyon::EklentileriYonet,
        KabukAksiyon::Belgeler,
        KabukAksiyon::Hakkinda,
    ];

    #[test]
    fn tum_aksiyon_etiketleri_iki_dilde_dolu_ve_farkli() {
        for &a in TUM_AKSIYONLAR {
            assert!(!a.etiket(Dil::Tr).is_empty(), "TR etiket boş: {a:?}");
            assert!(!a.etiket(Dil::En).is_empty(), "EN etiket boş: {a:?}");
        }
        // Birkaç temsilî aksiyon için iki dil gerçekten farklı (çeviri yapılmış).
        for &a in &[
            KabukAksiyon::Kaydet,
            KabukAksiyon::Cikis,
            KabukAksiyon::Belgeler,
        ] {
            assert_ne!(a.etiket(Dil::Tr), a.etiket(Dil::En));
        }
    }

    #[test]
    fn etkin_aksiyonlar_bu_surumde_calisir() {
        // Bu sürümde fiilen işlevsel olan aksiyonlar etkin görünmeli.
        assert!(KabukAksiyon::TemaDegistir.etkin_mi());
        assert!(KabukAksiyon::YanPanelAcKapa.etkin_mi());
        assert!(KabukAksiyon::Cikis.etkin_mi());
        // İlgili paketi henüz olmayanlar devre dışı (sahte "çalışıyor" yok).
        assert!(!KabukAksiyon::YeniProje.etkin_mi());
        assert!(!KabukAksiyon::EklentileriYonet.etkin_mi());
    }

    #[test]
    fn menu_ogeleri_yalnizca_tanimli_aksiyonlari_icerir() {
        // Her menüdeki her öge ya ayraç ya da etiketi dolu bir aksiyon olmalı.
        for menu in [
            Menu::Dosya,
            Menu::Duzen,
            Menu::Gorunum,
            Menu::Eklenti,
            Menu::Yardim,
        ] {
            assert!(!menu_basligi(menu, Dil::Tr).is_empty());
            let var = menu_ogeleri(menu).iter().any(|o| o.is_some());
            assert!(var, "menü boş olmamalı: {menu:?}");
        }
    }
}
