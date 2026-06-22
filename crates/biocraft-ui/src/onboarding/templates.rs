//! Zengin proje şablonları + gömülü demo veri (İP-17).
//!
//! Sihirbazın 4 temel türünü ([`crate::wizard::ProjeSablonu`]) **kullanıcıya dönük zengin
//! şablonlara** çevirir: "Genom Görselleştirme", "Varyant İnceleme", "Protein Yapısı",
//! "Dizi Hizalama", "RNA-seq".  Her şablon (a) hangi **panellerin/örnek akışın** ön-kurulacağını
//! ([`PanelPlani`]) ve (b) hangi **gömülü demo verinin** açılacağını ([`DemoVeri`]) bilir → kullanıcı
//! **boş ekranla kalmaz**.
//!
//! Demo veri **derleme zamanında ikiliye gömülür** (`include_str!`); çalışmak için indirme/ağ
//! gerekmez (`Dikkat: hafif`).  Hepsi **sentetik / açık-lisanslı** ve `Sentetik` sınıfındadır —
//! gerçek hasta verisi değildir (MK-42); köken bilgisi her veriye iliştirilir (İP-10 ile tutarlı).

use crate::i18n::Dil;
use crate::tokens::Tokenlar;

use super::metin;

// ── Gömülü demo veri (assets/demo/*) ──────────────────────────────────────────
// Derleme zamanında gömülür; CWD'den bağımsızdır (indirme/ağ yok — MK-09 ruhu: küçük, hazır).
const MINI_FASTA: &str = include_str!("../../../../assets/demo/mini.fasta");
const MINI_VCF: &str = include_str!("../../../../assets/demo/mini.vcf");
const MINI_SAM: &str = include_str!("../../../../assets/demo/mini.sam");
const MINI_PDB: &str = include_str!("../../../../assets/demo/mini.pdb");

/// Tüm demo verilerin ortak kökeni (provenance) — gerçek proje kurulunca `koken.jsonl`'e yazılabilir.
pub const DEMO_KAYNAK: &str = "BioCraft sentetik örnek";
/// Demo verilerin lisansı (sentetik → kamu malı eşdeğeri).
pub const DEMO_LISANS: &str = "CC0-1.0 (sentetik)";

/// Gömülü tek bir demo veri dosyası (ad + biçim + içerik + köken).
#[derive(Debug, Clone, Copy)]
pub struct DemoVeri {
    /// Dosya adı ("mini.fasta").
    pub ad: &'static str,
    /// Biçim etiketi ("FASTA", "VCF", "SAM", "PDB").
    pub bicim: &'static str,
    /// Gömülü ham içerik (UTF-8).
    pub icerik: &'static str,
}

impl DemoVeri {
    /// İçerikteki satır sayısı (panel özetinde "X satır" göstermek için).
    pub fn satir_sayisi(&self) -> usize {
        self.icerik.lines().count()
    }

    /// Köken (provenance) — her demo veri sentetik + açık lisanslıdır (MK-42 / İP-10).
    pub fn koken(&self) -> (&'static str, &'static str) {
        (DEMO_KAYNAK, DEMO_LISANS)
    }
}

const FASTA: DemoVeri = DemoVeri {
    ad: "mini.fasta",
    bicim: "FASTA",
    icerik: MINI_FASTA,
};
const VCF: DemoVeri = DemoVeri {
    ad: "mini.vcf",
    bicim: "VCF",
    icerik: MINI_VCF,
};
const SAM: DemoVeri = DemoVeri {
    ad: "mini.sam",
    bicim: "SAM",
    icerik: MINI_SAM,
};
const PDB: DemoVeri = DemoVeri {
    ad: "mini.pdb",
    bicim: "PDB",
    icerik: MINI_PDB,
};

/// Bir şablonun ön-kuracağı paneller/görünümler.  Host (`biocraft-app`) bunu okuyup ilgili
/// panelleri açar → şablon seçilince "ilgili paneller/örnek akış ön-kurulur".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PanelPlani {
    /// Sol Yan Panel (Gezgin/Explorer) açık gelsin mi.
    pub yan_panel: bool,
    /// Alt Panel (Konsol/İşler) açık gelsin mi (demo veri özeti buraya yazılır).
    pub alt_panel: bool,
    /// Sağ Inspector (özellik + 3B önizleme) açık gelsin mi.
    pub inspector: bool,
    /// Node (görsel akış) editörü örnek akışla açık gelsin mi.
    pub node_tuvali: bool,
    /// Kod editörü açık gelsin mi.
    pub kod_editoru: bool,
}

/// Kullanıcıya dönük zengin başlangıç şablonu.  Sihirbazın temel türüne eşlenir; ek olarak
/// panel planı + demo veri taşır.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingSablon {
    /// Genom Görselleştirme: tarayıcı + örnek akış + dizi/varyant demo.
    GenomGorsel,
    /// Varyant İnceleme: varyant tablosu + hizalama + örnek akış.
    VaryantInceleme,
    /// Protein Yapısı: 3B yapı görüntüleyici (inspector) + PDB demo.
    ProteinYapi,
    /// Dizi Hizalama: hizalama akışı + FASTA/SAM demo.
    DiziHizalama,
    /// RNA-seq İnceleme: ifade akışı + örnek akış.
    RnaSeq,
    /// Boş: hiçbir panel ön-kurulmaz (deneyimli kullanıcı kendi kurar).
    Bos,
}

impl OnboardingSablon {
    /// Galeride gösterilecek tüm şablonlar (sıra sabit).
    pub const TUMU: &'static [OnboardingSablon] = &[
        OnboardingSablon::GenomGorsel,
        OnboardingSablon::VaryantInceleme,
        OnboardingSablon::ProteinYapi,
        OnboardingSablon::DiziHizalama,
        OnboardingSablon::RnaSeq,
        OnboardingSablon::Bos,
    ];

    /// Şablon ikonu (renk view'da token'dan).
    pub fn ikon(&self) -> &'static str {
        match self {
            OnboardingSablon::GenomGorsel => "🧬",
            OnboardingSablon::VaryantInceleme => "🧪",
            OnboardingSablon::ProteinYapi => "🔬",
            OnboardingSablon::DiziHizalama => "🧩",
            OnboardingSablon::RnaSeq => "📊",
            OnboardingSablon::Bos => "📄",
        }
    }

    /// Dile göre kısa ad.
    pub fn ad(&self, tr: bool) -> &'static str {
        match self {
            OnboardingSablon::GenomGorsel => {
                metin(tr, "Genom Görselleştirme", "Genome Visualization")
            }
            OnboardingSablon::VaryantInceleme => metin(tr, "Varyant İnceleme", "Variant Review"),
            OnboardingSablon::ProteinYapi => metin(tr, "Protein Yapısı", "Protein Structure"),
            OnboardingSablon::DiziHizalama => metin(tr, "Dizi Hizalama", "Sequence Alignment"),
            OnboardingSablon::RnaSeq => metin(tr, "RNA-seq İnceleme", "RNA-seq Review"),
            OnboardingSablon::Bos => metin(tr, "Boş Proje", "Empty Project"),
        }
    }

    /// Dile göre tek cümlelik açıklama.
    pub fn aciklama(&self, tr: bool) -> &'static str {
        match self {
            OnboardingSablon::GenomGorsel => metin(
                tr,
                "Genom tarayıcı + örnek akış; dizi ve varyant demo verisiyle gelir.",
                "Genome browser + sample flow; comes with sequence and variant demo data.",
            ),
            OnboardingSablon::VaryantInceleme => metin(
                tr,
                "Varyant tablosu + hizalama; örnek VCF/SAM demo verisiyle gelir.",
                "Variant table + alignment; comes with sample VCF/SAM demo data.",
            ),
            OnboardingSablon::ProteinYapi => metin(
                tr,
                "3B yapı görüntüleyici; örnek PDB demo verisiyle gelir.",
                "3D structure viewer; comes with a sample PDB demo file.",
            ),
            OnboardingSablon::DiziHizalama => metin(
                tr,
                "Hizalama akışı; örnek FASTA/SAM demo verisiyle gelir.",
                "Alignment flow; comes with sample FASTA/SAM demo data.",
            ),
            OnboardingSablon::RnaSeq => metin(
                tr,
                "İfade analizi akışı; örnek dizi demo verisiyle gelir.",
                "Expression analysis flow; comes with sample sequence demo data.",
            ),
            OnboardingSablon::Bos => metin(
                tr,
                "Hiçbir panel ön-kurulmaz; her şeyi kendiniz eklersiniz.",
                "No panels pre-installed; you add everything yourself.",
            ),
        }
    }

    /// Bu şablonun ön-kuracağı paneller/akış.
    pub fn panel_plani(&self) -> PanelPlani {
        match self {
            OnboardingSablon::GenomGorsel => PanelPlani {
                yan_panel: true,
                alt_panel: true,
                inspector: true,
                node_tuvali: true,
                kod_editoru: false,
            },
            OnboardingSablon::VaryantInceleme => PanelPlani {
                yan_panel: true,
                alt_panel: true,
                inspector: true,
                node_tuvali: true,
                kod_editoru: false,
            },
            OnboardingSablon::ProteinYapi => PanelPlani {
                yan_panel: true,
                alt_panel: true,
                inspector: true,
                node_tuvali: false,
                kod_editoru: false,
            },
            OnboardingSablon::DiziHizalama => PanelPlani {
                yan_panel: true,
                alt_panel: true,
                inspector: false,
                node_tuvali: true,
                kod_editoru: false,
            },
            OnboardingSablon::RnaSeq => PanelPlani {
                yan_panel: true,
                alt_panel: true,
                inspector: false,
                node_tuvali: true,
                kod_editoru: false,
            },
            OnboardingSablon::Bos => PanelPlani::default(),
        }
    }

    /// Bu şablonla gelen gömülü demo veriler (boş şablonda yok).
    pub fn demo_veriler(&self) -> &'static [DemoVeri] {
        match self {
            OnboardingSablon::GenomGorsel => &[FASTA, VCF],
            OnboardingSablon::VaryantInceleme => &[VCF, SAM],
            OnboardingSablon::ProteinYapi => &[PDB],
            OnboardingSablon::DiziHizalama => &[FASTA, SAM],
            OnboardingSablon::RnaSeq => &[FASTA],
            OnboardingSablon::Bos => &[],
        }
    }

    /// Sihirbazın temel türüne ([`crate::wizard::ProjeSablonu`]) eşleme (entegrasyon).
    pub fn proje_sablonu(&self) -> crate::wizard::ProjeSablonu {
        use crate::wizard::ProjeSablonu as P;
        match self {
            OnboardingSablon::GenomGorsel
            | OnboardingSablon::VaryantInceleme
            | OnboardingSablon::DiziHizalama
            | OnboardingSablon::RnaSeq => P::Genomik,
            OnboardingSablon::ProteinYapi => P::Proteomik,
            OnboardingSablon::Bos => P::Bos,
        }
    }
}

/// Şablon galerisinin bir karedeki sonucu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaleriEylem {
    /// Bu şablonla demo başlat (panelleri aç + demo veri yükle).
    Baslat(OnboardingSablon),
    /// Galeriyi kapat.
    Kapat,
}

/// Şablon galerisini (yüzen pencere) çizer; kullanıcı bir şablon seçer veya kapatır.
pub fn galeri_ciz(
    ctx: &egui::Context,
    acik: &mut bool,
    dil: Dil,
    tok: &Tokenlar,
) -> Option<GaleriEylem> {
    let tr = matches!(dil, Dil::Tr);
    let mut eylem: Option<GaleriEylem> = None;
    let mut acik_yerel = *acik;

    egui::Window::new(metin(tr, "Şablon Galerisi", "Template Gallery"))
        .open(&mut acik_yerel)
        .collapsible(false)
        .resizable(true)
        .default_width(560.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(metin(
                    tr,
                    "Bir şablon seçin; ilgili paneller ve örnek demo veri hazır gelir.",
                    "Pick a template; the relevant panels and sample demo data come ready.",
                ))
                .color(tok.renk.metin_soluk),
            );
            ui.add_space(tok.bosluk.s);
            egui::ScrollArea::vertical()
                .max_height(420.0)
                .show(ui, |ui| {
                    for &s in OnboardingSablon::TUMU {
                        sablon_karti_ciz(ui, s, tr, tok, &mut eylem);
                        ui.add_space(tok.bosluk.s);
                    }
                });
        });

    // ✕ ile kapatıldıysa.
    if !acik_yerel {
        eylem = eylem.or(Some(GaleriEylem::Kapat));
    }
    *acik = acik_yerel;
    eylem
}

/// Tek bir şablon kartını çizer ("Bu şablonla başla" butonu dahil).
fn sablon_karti_ciz(
    ui: &mut egui::Ui,
    s: OnboardingSablon,
    tr: bool,
    tok: &Tokenlar,
    eylem: &mut Option<GaleriEylem>,
) {
    let cerceve = egui::Frame {
        fill: tok.renk.yuzey_alt,
        stroke: egui::Stroke::new(1.0, tok.renk.kenarlik),
        rounding: egui::Rounding::same(tok.yaricap),
        inner_margin: egui::Margin::same(tok.bosluk.m),
        ..Default::default()
    };
    cerceve.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(s.ikon()).size(26.0));
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(s.ad(tr)).strong().color(tok.renk.metin));
                ui.label(
                    egui::RichText::new(s.aciklama(tr))
                        .small()
                        .color(tok.renk.metin_soluk),
                );
                // Demo veri rozetleri (varsa).
                let veriler = s.demo_veriler();
                if !veriler.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new(metin(tr, "Demo veri:", "Demo data:"))
                                .small()
                                .color(tok.renk.metin_soluk),
                        );
                        for v in veriler {
                            ui.label(
                                egui::RichText::new(format!("📎 {}", v.ad))
                                    .small()
                                    .color(tok.renk.bilgi),
                            );
                        }
                    });
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let buton = egui::Button::new(
                    egui::RichText::new(metin(tr, "Başlat", "Start"))
                        .color(tok.renk.vurgu_ustu)
                        .strong(),
                )
                .fill(tok.renk.vurgu);
                if ui.add(buton).clicked() {
                    *eylem = Some(GaleriEylem::Baslat(s));
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gomulu_demo_veri_dolu() {
        // include_str! ile gömülü dosyalar boş olmamalı (yol doğru + dosya mevcut).
        for v in [FASTA, VCF, SAM, PDB] {
            assert!(!v.icerik.trim().is_empty(), "boş demo veri: {}", v.ad);
            assert!(v.satir_sayisi() > 0);
            let (kaynak, lisans) = v.koken();
            assert!(!kaynak.is_empty() && !lisans.is_empty());
        }
    }

    #[test]
    fn tum_sablonlar_iki_dilde_dolu_ve_farkli() {
        for &s in OnboardingSablon::TUMU {
            assert!(!s.ad(true).is_empty() && !s.ad(false).is_empty());
            assert!(!s.aciklama(true).is_empty() && !s.aciklama(false).is_empty());
            assert_ne!(s.ad(true), s.ad(false), "ad çevrilmemiş: {s:?}");
            assert_ne!(s.aciklama(true), s.aciklama(false));
            assert!(!s.ikon().is_empty());
        }
    }

    #[test]
    fn bos_sablon_panel_acmaz_demo_yok() {
        let p = OnboardingSablon::Bos.panel_plani();
        assert_eq!(p, PanelPlani::default());
        assert!(OnboardingSablon::Bos.demo_veriler().is_empty());
    }

    #[test]
    fn dolu_sablonlar_panel_acar_ve_demo_getirir() {
        // Boş dışındaki her şablon en az bir panel açmalı ve en az bir demo veri getirmeli
        // (kullanıcı boş ekranla kalmaz — kabul kriteri).
        for &s in OnboardingSablon::TUMU {
            if s == OnboardingSablon::Bos {
                continue;
            }
            let p = s.panel_plani();
            let panel_var =
                p.yan_panel || p.alt_panel || p.inspector || p.node_tuvali || p.kod_editoru;
            assert!(panel_var, "şablon hiç panel açmıyor: {s:?}");
            assert!(
                !s.demo_veriler().is_empty(),
                "şablon demo veri getirmiyor: {s:?}"
            );
        }
    }

    #[test]
    fn galeri_headless_cizilir() {
        let ctx = egui::Context::default();
        let mut acik = true;
        let _ = ctx.run(egui::RawInput::default(), |c| {
            let _ = galeri_ciz(c, &mut acik, Dil::Tr, &Tokenlar::koyu());
        });
    }
}
