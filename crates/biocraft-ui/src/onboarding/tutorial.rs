//! Bağlamsal ipuçları + kavram açıklamaları + boş-durum rehberleri (İP-17).
//!
//! "Öğretici" gerçek arayüzde, hafif ve **kapatılabilir** olmalıdır (oyuncak mod değil).  Bu modül:
//! - **İpucu baloncuğu** ([`ipucu_baloncugu`]): tek satırlık, kapatılabilir bağlamsal ipucu.
//! - **Kavram açıklaması** ([`Kavram`]): "BAM/VCF nedir?" gibi opsiyonel mikro-öğreticiler.
//! - **Boş-durum rehberi** ([`bos_durum_rehberi`]): boş ekran yerine "ne yapmalıyım?" yönlendirmesi
//!   (İP-16 [`crate::components::EmptyState`] üstüne onboarding metni).
//!
//! Tüm ipuçları **uyarlanabilir/kapatılabilir** ([`IpucuDurumu::kapali`]); tüm metin TR/EN (MK-53).

use crate::components::EmptyState;
use crate::i18n::Dil;
use crate::tokens::Tokenlar;

use super::metin;

/// İpuçlarının açık/kapalı durumu (kullanıcı tamamen kapatabilir; kalıcı).
///
/// Varsayılan: ipuçları **açık** (`kapali = false` — yeni kullanıcıya yardımcı olur; istenirse
/// "bir daha gösterme" ile kapatılır).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IpucuDurumu {
    /// İpuçları tamamen kapalı mı (kullanıcı "bir daha gösterme" dediyse).
    pub kapali: bool,
}

/// Tek bir bağlamsal ipucu baloncuğunu çizer.  İki buton döndürebilir:
/// `(anladim, bir_daha_gosterme)`.  `bir_daha_gosterme` → host `IpucuDurumu::kapali = true` yapar.
///
/// Hafif bir çerçeve (akıcılığı bozmaz); çağıran taraf metni yerelleştirilmiş verir.
pub fn ipucu_baloncugu(
    ui: &mut egui::Ui,
    dil: Dil,
    tok: &Tokenlar,
    metin_govde: &str,
) -> (bool, bool) {
    let tr = matches!(dil, Dil::Tr);
    let mut anladim = false;
    let mut bir_daha_gosterme = false;
    let cerceve = egui::Frame {
        fill: tok.renk.bilgi_zemin,
        stroke: egui::Stroke::new(1.0, tok.renk.bilgi),
        rounding: egui::Rounding::same(tok.yaricap),
        inner_margin: egui::Margin::same(tok.bosluk.s),
        ..Default::default()
    };
    cerceve.show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("💡").size(16.0));
            ui.label(egui::RichText::new(metin_govde).color(tok.renk.metin));
        });
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(metin(tr, "Bir daha gösterme", "Don't show again"))
                    .clicked()
                {
                    bir_daha_gosterme = true;
                }
                if ui.button(metin(tr, "Anladım", "Got it")).clicked() {
                    anladim = true;
                }
            });
        });
    });
    (anladim, bir_daha_gosterme)
}

/// Opsiyonel kavram mikro-öğreticileri ("BAM nedir?" vb.).  Yeni başlayan kullanıcı için;
/// deneyimliye dayatılmaz (yalnızca istenince/`Yardım`'da gösterilir).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kavram {
    /// FASTA — dizi dosyası.
    Fasta,
    /// BAM/SAM — hizalanmış okuma dosyası.
    Bam,
    /// VCF — varyant dosyası.
    Vcf,
    /// PDB — protein 3B yapı dosyası.
    Pdb,
    /// Node akışı — görsel iş akışı.
    NodeAkisi,
    /// Komut paleti — hızlı komut erişimi.
    KomutPaleti,
}

impl Kavram {
    /// Tüm kavramlar (Yardım'da listelenir; testler her dilde doluluğunu doğrular).
    pub const TUMU: &'static [Kavram] = &[
        Kavram::Fasta,
        Kavram::Bam,
        Kavram::Vcf,
        Kavram::Pdb,
        Kavram::NodeAkisi,
        Kavram::KomutPaleti,
    ];

    /// Kavramın kısa başlığı/terimi.
    pub fn terim(self) -> &'static str {
        match self {
            Kavram::Fasta => "FASTA",
            Kavram::Bam => "BAM / SAM",
            Kavram::Vcf => "VCF",
            Kavram::Pdb => "PDB",
            Kavram::NodeAkisi => metin(true, "Node akışı", "Node flow"), // terim aynı; tek dil yeter
            Kavram::KomutPaleti => metin(true, "Komut paleti", "Command palette"),
        }
    }

    /// Kavramın bir cümlelik sade açıklaması.
    pub fn aciklama(self, tr: bool) -> &'static str {
        match self {
            Kavram::Fasta => metin(
                tr,
                "DNA/RNA/protein dizilerini saklayan basit metin dosyası biçimi.",
                "A simple text format that stores DNA/RNA/protein sequences.",
            ),
            Kavram::Bam => metin(
                tr,
                "Okumaların bir referansa hizalanmış halini tutar; BAM ikili, SAM metin biçimidir.",
                "Holds reads aligned to a reference; BAM is binary, SAM is its text form.",
            ),
            Kavram::Vcf => metin(
                tr,
                "Bir örnekteki genetik varyantları (farklılıkları) listeleyen dosya biçimi.",
                "A format that lists the genetic variants (differences) in a sample.",
            ),
            Kavram::Pdb => metin(
                tr,
                "Bir proteinin/molekülün 3 boyutlu atom koordinatlarını tutan dosya biçimi.",
                "A format that holds the 3D atomic coordinates of a protein/molecule.",
            ),
            Kavram::NodeAkisi => metin(
                tr,
                "Kutuları (node) kablolarla bağlayarak kod yazmadan bir iş akışı kurmanızı sağlar.",
                "Lets you build a workflow without writing code by wiring boxes (nodes) together.",
            ),
            Kavram::KomutPaleti => metin(
                tr,
                "Ctrl+Shift+P ile açılır; her komuta adını yazarak hızla ulaşırsınız.",
                "Opens with Ctrl+Shift+P; reach any command quickly by typing its name.",
            ),
        }
    }
}

/// Boş bir merkez/editör alanı için onboarding rehberi (boş ekran yerine yönlendirme — TDA 5).
/// Birincil eylem etiketi "Demo Projeyi Aç" (host tıklamayı yakalar).
pub fn bos_durum_rehberi(dil: Dil) -> EmptyState {
    let tr = matches!(dil, Dil::Tr);
    EmptyState::yeni(
        "🧬",
        metin(tr, "Buradan başlayın", "Start here"),
        metin(
            tr,
            "Henüz açık bir şey yok. Örnek veriyle dolu bir projeyle hemen deneyebilirsiniz.",
            "Nothing is open yet. You can try right away with a project full of sample data.",
        ),
    )
    .with_eylem(metin(tr, "▶ Demo Projeyi Aç", "▶ Open Demo Project"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tum_kavramlar_iki_dilde_dolu() {
        for &k in Kavram::TUMU {
            assert!(!k.terim().is_empty());
            assert!(!k.aciklama(true).is_empty() && !k.aciklama(false).is_empty());
            // Açıklamalar gerçekten çevrilmiş olmalı (TR ≠ EN).
            assert_ne!(
                k.aciklama(true),
                k.aciklama(false),
                "açıklama çevrilmemiş: {k:?}"
            );
        }
    }

    #[test]
    fn ipucu_varsayilan_acik() {
        assert!(!IpucuDurumu::default().kapali);
    }

    #[test]
    fn bos_durum_rehberi_eylemli() {
        let bd = bos_durum_rehberi(Dil::Tr);
        assert!(bd.eylem_etiketi.is_some());
        let bd_en = bos_durum_rehberi(Dil::En);
        assert_ne!(bd.baslik, bd_en.baslik, "boş-durum başlığı çevrilmemiş");
    }

    #[test]
    fn ipucu_headless_cizilir() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |c| {
            egui::CentralPanel::default().show(c, |ui| {
                let _ = ipucu_baloncugu(ui, Dil::Tr, &Tokenlar::koyu(), "Deneme ipucu");
            });
        });
    }
}
