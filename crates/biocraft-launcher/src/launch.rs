//! Başlatma protokolü + launcher eylemleri — İP-01.
//!
//! Launcher motoru **argümanlarla** başlatır (proje yolu, sürüm, mod) — Epic-benzeri ayrı istemci
//! kalıcı bir üründür, kaldırılmaz.  MVP'de launcher ile motor **aynı binary** olduğundan geçiş
//! süreç-içi yapılır; yine de başlatma argüman listesi ([`BaslatmaArgumanlari::argv`]) burada
//! **gerçek ve test-edilebilir** kurulur → ileride ayrı-süreç başlatma (v1.x yan-yana çoklu sürüm,
//! `MVP-sonrasi.md` §8.3) yeniden yazım gerektirmez.

use std::path::PathBuf;

/// Motorun hangi modda açılacağı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BaslatmaModu {
    /// Normal mod (tam özellik).
    #[default]
    Normal,
    /// Güvenli mod (yalnızca resmi eklentiler — İP-07 `safe_mode` ile tutarlı).
    GuvenliMod,
}

impl BaslatmaModu {
    /// CLI argüman karşılığı (yoksa `None` = normal, bayrak eklenmez).
    fn bayrak(&self) -> Option<&'static str> {
        match self {
            BaslatmaModu::Normal => None,
            BaslatmaModu::GuvenliMod => Some("--safe-mode"),
        }
    }
}

/// Motoru başlatmak için toplanan argümanlar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BaslatmaArgumanlari {
    /// Açılacak proje yolu (yeni proje akışında `None`; motor boş/giriş ekranı açar).
    pub proje_yolu: Option<PathBuf>,
    /// Hedeflenen motor sürümü (proje sürüm kaydıyla eşleşir; MVP tek sürüm).
    pub surum: Option<String>,
    /// Başlatma modu.
    pub mod_: BaslatmaModu,
    /// Yazılım (CPU) backend'ini zorla (GPU sorunu/teşhis) — motorun `--cpu` bayrağı.
    pub cpu_zorla: bool,
}

impl BaslatmaArgumanlari {
    /// Belirli bir projeyi açan argümanlar.
    pub fn proje(yol: impl Into<PathBuf>) -> Self {
        Self {
            proje_yolu: Some(yol.into()),
            ..Default::default()
        }
    }

    /// Motora geçirilecek argüman listesini (`argv`, program adı hariç) kurar.
    ///
    /// Sıra kararlı (test-edilebilir): `[--project <yol>] [--engine-version <v>] [--safe-mode] [--cpu]`.
    pub fn argv(&self) -> Vec<String> {
        let mut a = Vec::new();
        if let Some(yol) = &self.proje_yolu {
            a.push("--project".to_string());
            a.push(yol.to_string_lossy().to_string());
        }
        if let Some(s) = &self.surum {
            a.push("--engine-version".to_string());
            a.push(s.clone());
        }
        if let Some(b) = self.mod_.bayrak() {
            a.push(b.to_string());
        }
        if self.cpu_zorla {
            a.push("--cpu".to_string());
        }
        a
    }
}

/// Launcher arayüzünde tetiklenen üst-düzey eylem (host bunu uygular).
///
/// View saf bir eylem döndürür; gerçek tarafı (motora geçiş, dosya seçici, çıkış) host yapar —
/// böylece view test edilebilir ve motor/launcher gevşek bağlı kalır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherEylem {
    /// [Yeni Proje] → İP-02 sihirbazı (henüz yoksa "yakında" bilgisi).
    YeniProje,
    /// [Proje Aç] → OS dosya seçici.
    ProjeAc,
    /// Listeden bir projeyi motorda aç (başlatma argümanlarıyla).
    ProjeyiBaslat(BaslatmaArgumanlari),
    /// Taşınmış projeyi yeniden bağla (eski yol) → dosya seçici (madde 19).
    YenidenBagla(PathBuf),
    /// [Ayarlar] → İP-12.
    Ayarlar,
    /// [Yardım/Dokümanlar].
    Yardim,
    /// Eğitim/onboarding modunu yeniden başlat (İP-17).
    EgitimiBaslat,
    /// Bir dış bağlantıyı aç — **kullanıcı zaten onayladı** (view onay diyaloğunu yönetir).
    DisBaglantiAc(String),
    /// Launcher'ı (ve uygulamayı) kapat.
    Cikis,
}

// ─── Testler ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bos_argumanlar_bos_argv() {
        assert!(BaslatmaArgumanlari::default().argv().is_empty());
    }

    #[test]
    fn proje_argv_dogru() {
        let a = BaslatmaArgumanlari::proje("/p/genom.bcproj");
        assert_eq!(a.argv(), vec!["--project", "/p/genom.bcproj"]);
    }

    #[test]
    fn tam_argv_sirali() {
        let a = BaslatmaArgumanlari {
            proje_yolu: Some("/p/x".into()),
            surum: Some("0.1.0".into()),
            mod_: BaslatmaModu::GuvenliMod,
            cpu_zorla: true,
        };
        assert_eq!(
            a.argv(),
            vec![
                "--project",
                "/p/x",
                "--engine-version",
                "0.1.0",
                "--safe-mode",
                "--cpu"
            ]
        );
    }

    #[test]
    fn normal_mod_bayrak_eklemez() {
        let a = BaslatmaArgumanlari {
            mod_: BaslatmaModu::Normal,
            ..Default::default()
        };
        assert!(!a.argv().iter().any(|x| x == "--safe-mode"));
    }
}
