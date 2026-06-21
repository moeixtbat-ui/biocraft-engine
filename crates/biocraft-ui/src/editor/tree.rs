//! Proje dosya ağacı (İP-06) — **tembel** dizin tarama + dosyalar arası gezinme.
//!
//! Ağaç, bir düğüm **genişletilene kadar** alt dizini okumaz ([`AgacDugumu::genislet`]);
//! böylece çok sayıda/derin dosya içeren proje köklerinde açılış donmaz (MK-07 ruhu).
//! Dosyaya tıklanınca [`ProjeAgaci::ciz`] o dosyanın yolunu döner → editör onu açar.
// MK-52: renkler token'dan; bu modül metin/etiket üretir, sabit renk üretmez.

use crate::i18n::Dil;
use crate::tokens::Tokenlar;
use std::path::{Path, PathBuf};

/// Tek bir ağaç düğümü (dosya veya dizin).
#[derive(Debug, Clone)]
pub struct AgacDugumu {
    /// Tam yol.
    pub yol: PathBuf,
    /// Görünen ad (yolun son bileşeni).
    pub ad: String,
    /// Dizin mi?
    pub dizin_mi: bool,
    /// Dizin açık (genişletilmiş) mi?
    pub acik: bool,
    /// Çocuklar — `None` = henüz okunmadı (tembel).
    pub cocuklar: Option<Vec<AgacDugumu>>,
}

impl AgacDugumu {
    /// Bir yoldan düğüm kurar (çocukları okumadan).
    pub fn yeni(yol: impl Into<PathBuf>) -> Self {
        let yol = yol.into();
        let dizin_mi = yol.is_dir();
        let ad = yol
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| yol.to_string_lossy().into_owned());
        Self {
            yol,
            ad,
            dizin_mi,
            acik: false,
            cocuklar: None,
        }
    }

    /// Alt dizini **tembel** okur (dizinler önce, sonra alfabetik).  Zaten okunduysa atlar.
    pub fn genislet(&mut self) {
        self.acik = true;
        if self.cocuklar.is_some() || !self.dizin_mi {
            return;
        }
        let mut cocuklar: Vec<AgacDugumu> = Vec::new();
        if let Ok(girisler) = std::fs::read_dir(&self.yol) {
            for giris in girisler.flatten() {
                cocuklar.push(AgacDugumu::yeni(giris.path()));
            }
        }
        cocuklar.sort_by(|a, b| {
            b.dizin_mi
                .cmp(&a.dizin_mi) // dizinler önce
                .then_with(|| a.ad.to_lowercase().cmp(&b.ad.to_lowercase()))
        });
        self.cocuklar = Some(cocuklar);
    }

    /// Düğümü kapatır (çocuklar bellekte kalır; yeniden açışta diskten okunmaz).
    pub fn daralt(&mut self) {
        self.acik = false;
    }
}

/// Proje dosya ağacı.  Kök yoksa boş (editör yine de boş sekmeyle çalışır).
#[derive(Debug, Clone, Default)]
pub struct ProjeAgaci {
    /// Kök düğüm (proje klasörü).
    pub kok: Option<AgacDugumu>,
}

impl ProjeAgaci {
    /// Boş ağaç (proje açık değil).
    pub fn bos() -> Self {
        Self::default()
    }

    /// Bir klasörü kök olarak açar (genişletilmiş başlar).
    pub fn ac(kok: impl Into<PathBuf>) -> Self {
        let mut dugum = AgacDugumu::yeni(kok);
        dugum.genislet();
        Self { kok: Some(dugum) }
    }

    /// Ağacı çizer; bir **dosyaya** tıklanırsa o dosyanın yolunu döner (editör açar).
    pub fn ciz(&mut self, ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) -> Option<PathBuf> {
        let baslik = if matches!(dil, Dil::Tr) {
            "Proje"
        } else {
            "Project"
        };
        ui.label(
            egui::RichText::new(baslik)
                .color(tok.renk.metin_soluk)
                .small(),
        );
        ui.separator();

        let mut acilacak = None;
        match &mut self.kok {
            Some(kok) => {
                dugum_ciz(ui, kok, tok, &mut acilacak);
            }
            None => {
                ui.label(
                    egui::RichText::new(if matches!(dil, Dil::Tr) {
                        "Proje açık değil"
                    } else {
                        "No project open"
                    })
                    .color(tok.renk.metin_soluk)
                    .italics(),
                );
            }
        }
        acilacak
    }
}

/// Tek bir düğümü (ve açıksa çocuklarını) yinelemeli çizer.
fn dugum_ciz(
    ui: &mut egui::Ui,
    dugum: &mut AgacDugumu,
    tok: &Tokenlar,
    acilacak: &mut Option<PathBuf>,
) {
    if dugum.dizin_mi {
        let ok = if dugum.acik { "▼" } else { "▶" };
        let etiket = format!("{ok} 📁 {}", dugum.ad);
        if ui
            .add(
                egui::Label::new(egui::RichText::new(etiket).color(tok.renk.metin))
                    .sense(egui::Sense::click()),
            )
            .clicked()
        {
            if dugum.acik {
                dugum.daralt();
            } else {
                dugum.genislet();
            }
        }
        if dugum.acik {
            if let Some(cocuklar) = &mut dugum.cocuklar {
                ui.indent(dugum.yol.clone(), |ui| {
                    for c in cocuklar.iter_mut() {
                        dugum_ciz(ui, c, tok, acilacak);
                    }
                });
            }
        }
    } else {
        let simge = uzanti_simgesi(&dugum.yol);
        let etiket = format!("{simge} {}", dugum.ad);
        if ui
            .add(
                egui::Label::new(egui::RichText::new(etiket).color(tok.renk.metin))
                    .sense(egui::Sense::click()),
            )
            .clicked()
        {
            *acilacak = Some(dugum.yol.clone());
        }
    }
}

/// Dosya uzantısına göre küçük bir simge (yalnız görsel ipucu).
fn uzanti_simgesi(yol: &Path) -> &'static str {
    match yol
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "py" | "pyw" | "pyi" => "🐍",
        "r" => "📊",
        "sh" | "bash" => "💻",
        "json" | "yaml" | "yml" | "ron" | "toml" => "⚙",
        "md" | "txt" => "📄",
        _ => "•",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test için benzersiz geçici dizin kurar (birkaç dosya + bir alt klasör).
    fn ornek_proje() -> PathBuf {
        let kok = std::env::temp_dir().join(format!("biocraft_agac_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&kok);
        std::fs::create_dir_all(kok.join("alt")).unwrap();
        std::fs::write(kok.join("main.py"), "print(1)").unwrap();
        std::fs::write(kok.join("veri.json"), "{}").unwrap();
        std::fs::write(kok.join("alt").join("yardim.py"), "pass").unwrap();
        kok
    }

    #[test]
    fn bos_agac_kok_yok() {
        let a = ProjeAgaci::bos();
        assert!(a.kok.is_none());
    }

    #[test]
    fn ac_kokun_cocuklarini_okur_dizin_once() {
        let kok = ornek_proje();
        let a = ProjeAgaci::ac(&kok);
        let cocuklar = a.kok.unwrap().cocuklar.unwrap();
        assert_eq!(cocuklar.len(), 3); // alt/, main.py, veri.json
                                       // Dizinler önce sıralanır.
        assert!(cocuklar[0].dizin_mi);
        assert_eq!(cocuklar[0].ad, "alt");
        let _ = std::fs::remove_dir_all(&kok);
    }

    #[test]
    fn genislet_tembel_alt_dizini_okur() {
        let kok = ornek_proje();
        let mut dugum = AgacDugumu::yeni(&kok);
        // Genişletmeden önce çocuklar okunmamış (tembel).
        assert!(dugum.cocuklar.is_none());
        dugum.genislet();
        assert!(dugum.cocuklar.is_some());
        assert!(dugum.acik);
        // İki kez genişletmek diski yeniden okumaz (idempotent).
        let sayi = dugum.cocuklar.as_ref().unwrap().len();
        dugum.genislet();
        assert_eq!(dugum.cocuklar.as_ref().unwrap().len(), sayi);
        let _ = std::fs::remove_dir_all(&kok);
    }

    #[test]
    fn uzanti_simgesi_python() {
        assert_eq!(uzanti_simgesi(Path::new("a/b/x.py")), "🐍");
        assert_eq!(uzanti_simgesi(Path::new("y.json")), "⚙");
        assert_eq!(uzanti_simgesi(Path::new("z.bilinmeyen")), "•");
    }
}
