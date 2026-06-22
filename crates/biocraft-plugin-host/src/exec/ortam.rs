//! **İzole proje ortamı** — her projeye kendi Python sanal ortamı + paket yönetimi + sürüm
//! kilidi (İP-06, 2. kısım — Gün 23, MK-02).
//!
//! Her proje **kendi** sanal ortamına (`<proje>/.venv`) sahiptir.  Paketler **yalnız bu
//! ortama** kurulur, **global Python'a ASLA dokunulmaz** → bir projeye kurulan paket başka
//! projeyi/işletim sistemini bozmaz (kabul kriteri: "Paket kurulumu projeleri bozuyor →
//! her projeye izole ortam; global ortama kurma").
//!
//! Kod editörü, kullanıcı kodunu bu ortamın yorumlayıcısıyla çalıştırır
//! ([`SanalOrtam::yorumlayici`] → [`crate::exec::calistir_baslat_ile`]); jedi tamamlaması da
//! bu yorumlayıcıyı kullanır (izole, MK-02).
//!
//! ## Sürüm kilidi
//! Kurulu paketler `biocraft-paketler.lock` dosyasında (`ad==sürüm`) sabitlenir
//! ([`SanalOrtam::kilit_yaz`]/[`SanalOrtam::kilit_oku`]) → proje başka makinede **birebir aynı
//! sürümlerle** kurulur (tekrar-üretilebilirlik).  Kilitte olup kurulu olmayan paket
//! [`SanalOrtam::eksik_paketler`] ile bulunur → editör **"[Kur]"** yönlendirir.
// MK-02: tüm Python işleri ayrı süreçte (venv/pip); in-process YASAK.  MK-09 ile uyumlu (izole, hafif).

use biocraft_types::ErrorReport;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Proje köküne göre sanal ortam dizininin adı.
pub const VENV_DIZIN: &str = ".venv";

/// Sürüm kilidi dosyasının adı (proje kökünde).
pub const KILIT_DOSYA: &str = "biocraft-paketler.lock";

// ─── Paket gereksinimi (sürüm kilidi modeli) ───────────────────────────────────

/// Bir paket gereksinimi: ad + (opsiyonel) sabit sürüm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaketGereksinimi {
    /// Paket adı (örn. "numpy").
    pub ad: String,
    /// Sabitlenmiş sürüm (örn. "1.26.0"); `None` = en güncel.
    pub surum: Option<String>,
}

impl PaketGereksinimi {
    /// Ad + opsiyonel sürümle.
    pub fn yeni(ad: impl Into<String>, surum: Option<String>) -> Self {
        Self {
            ad: ad.into(),
            surum,
        }
    }

    /// `"numpy==1.26.0"` / `"numpy"` biçimini ayrıştırır (geçersizse `None`).
    pub fn ayristir(satir: &str) -> Option<Self> {
        let s = satir.trim();
        if s.is_empty() || s.starts_with('#') {
            return None;
        }
        if let Some((ad, surum)) = s.split_once("==") {
            let ad = ad.trim();
            let surum = surum.trim();
            if ad.is_empty() || !paket_adi_gecerli(ad) {
                return None;
            }
            Some(Self::yeni(
                ad,
                (!surum.is_empty()).then(|| surum.to_string()),
            ))
        } else {
            paket_adi_gecerli(s).then(|| Self::yeni(s, None))
        }
    }

    /// `pip install` argümanı (`numpy==1.26.0` veya `numpy`).
    pub fn pip_argumani(&self) -> String {
        match &self.surum {
            Some(v) => format!("{}=={}", self.ad, v),
            None => self.ad.clone(),
        }
    }

    /// Kilit dosyası satırı (sürüm yoksa yalnız ad).
    pub fn kilit_satiri(&self) -> String {
        self.pip_argumani()
    }
}

/// Kurulu bir paket (pip list çıktısından).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KuruluPaket {
    /// Paket adı.
    pub ad: String,
    /// Kurulu sürüm.
    pub surum: String,
}

// ─── Sanal ortam ───────────────────────────────────────────────────────────────

/// Bir projenin izole Python sanal ortamı (`<proje>/.venv`).
#[derive(Debug, Clone)]
pub struct SanalOrtam {
    proje_kok: PathBuf,
}

impl SanalOrtam {
    /// Proje kökünden bir sanal ortam tanıtıcısı (henüz oluşturmaz).
    pub fn yeni(proje_kok: impl Into<PathBuf>) -> Self {
        Self {
            proje_kok: proje_kok.into(),
        }
    }

    /// Sanal ortam dizini (`<proje>/.venv`).
    pub fn venv_dizini(&self) -> PathBuf {
        self.proje_kok.join(VENV_DIZIN)
    }

    /// Bu ortamın Python yorumlayıcısının yolu (platforma göre — henüz var olmayabilir).
    pub fn yorumlayici(&self) -> PathBuf {
        let venv = self.venv_dizini();
        if cfg!(windows) {
            venv.join("Scripts").join("python.exe")
        } else {
            venv.join("bin").join("python")
        }
    }

    /// Sanal ortam oluşturulmuş ve kullanılabilir mi? (yorumlayıcı dosyası var mı.)
    pub fn var_mi(&self) -> bool {
        self.yorumlayici().is_file()
    }

    /// Sürüm kilidi dosyasının yolu.
    pub fn kilit_yolu(&self) -> PathBuf {
        self.proje_kok.join(KILIT_DOSYA)
    }

    /// Sanal ortamı oluşturur (`<temel_python> -m venv .venv`).  **Bloklar** (saniyeler sürer).
    ///
    /// Zaten varsa bir şey yapmaz (idempotent).  `temel_python` sistem Python'u olmalıdır
    /// ([`crate::runtime::subprocess::python_bul`]).
    pub fn olustur(&self, temel_python: &Path) -> Result<(), ErrorReport> {
        if self.var_mi() {
            return Ok(());
        }
        std::fs::create_dir_all(&self.proje_kok)
            .map_err(|e| io_hata("Proje klasörü hazırlanamadı", &self.proje_kok, e))?;
        let cikti = Command::new(temel_python)
            .arg("-m")
            .arg("venv")
            .arg(self.venv_dizini())
            .output()
            .map_err(|e| {
                ErrorReport::new(
                    "Sanal ortam oluşturulamadı",
                    format!("'{}' başlatılamadı", temel_python.display()),
                    "Python kurulumunu doğrulayın (python -m venv çalışmalı)",
                )
                .with_teknik_detay(e.to_string())
            })?;
        if !cikti.status.success() || !self.var_mi() {
            return Err(ErrorReport::new(
                "Sanal ortam oluşturulamadı",
                "python -m venv komutu başarısız oldu",
                "Python'un 'venv' modülünün kurulu olduğundan emin olun",
            )
            .with_teknik_detay(String::from_utf8_lossy(&cikti.stderr).into_owned()));
        }
        Ok(())
    }

    /// Bir paketi **bu ortama** kurar (`<venv>/python -m pip install …`).  **Bloklar** (ağ + derleme).
    ///
    /// Global Python'a **dokunmaz** (venv yorumlayıcısı kullanılır → izolasyon garantisi).
    pub fn paket_kur(&self, gereksinim: &PaketGereksinimi) -> Result<(), ErrorReport> {
        self.venv_gerekli()?;
        let cikti = Command::new(self.yorumlayici())
            .arg("-m")
            .arg("pip")
            .arg("install")
            .arg("--disable-pip-version-check")
            .arg(gereksinim.pip_argumani())
            .output()
            .map_err(pip_baslatma_hatasi)?;
        if !cikti.status.success() {
            return Err(ErrorReport::new(
                "Paket kurulamadı",
                format!("'{}' kurulumu başarısız", gereksinim.pip_argumani()),
                "Paket adını/sürümünü ve internet bağlantısını kontrol edin",
            )
            .with_teknik_detay(String::from_utf8_lossy(&cikti.stderr).into_owned()));
        }
        Ok(())
    }

    /// Bir paketi ortamdan kaldırır (`pip uninstall -y`).  **Bloklar**.
    pub fn paket_kaldir(&self, ad: &str) -> Result<(), ErrorReport> {
        self.venv_gerekli()?;
        if !paket_adi_gecerli(ad) {
            return Err(gecersiz_paket_adi(ad));
        }
        let cikti = Command::new(self.yorumlayici())
            .args(["-m", "pip", "uninstall", "-y"])
            .arg(ad)
            .output()
            .map_err(pip_baslatma_hatasi)?;
        if !cikti.status.success() {
            return Err(ErrorReport::new(
                "Paket kaldırılamadı",
                format!("'{ad}' kaldırılamadı"),
                "Paketin kurulu olduğundan emin olun",
            )
            .with_teknik_detay(String::from_utf8_lossy(&cikti.stderr).into_owned()));
        }
        Ok(())
    }

    /// Kurulu paketleri listeler (`pip list --format=json`).  **Bloklar**.
    pub fn paketler(&self) -> Result<Vec<KuruluPaket>, ErrorReport> {
        self.venv_gerekli()?;
        let cikti = Command::new(self.yorumlayici())
            .args([
                "-m",
                "pip",
                "list",
                "--format=json",
                "--disable-pip-version-check",
            ])
            .output()
            .map_err(pip_baslatma_hatasi)?;
        if !cikti.status.success() {
            return Err(ErrorReport::new(
                "Paket listesi alınamadı",
                "pip list başarısız oldu",
                "Sanal ortamı yeniden oluşturmayı deneyin",
            )
            .with_teknik_detay(String::from_utf8_lossy(&cikti.stderr).into_owned()));
        }
        let metin = String::from_utf8_lossy(&cikti.stdout);
        Ok(pip_list_coz(&metin))
    }

    /// Bir paket kurulu mu? (ad büyük/küçük harf duyarsız.)
    pub fn paket_var_mi(&self, ad: &str) -> Result<bool, ErrorReport> {
        let hedef = ad.to_lowercase();
        Ok(self
            .paketler()?
            .iter()
            .any(|p| p.ad.to_lowercase() == hedef))
    }

    // ── Sürüm kilidi ──

    /// Verilen gereksinimleri kilit dosyasına yazar (`biocraft-paketler.lock`).
    pub fn kilit_yaz(&self, gereksinimler: &[PaketGereksinimi]) -> Result<(), ErrorReport> {
        let metin = kilit_metni(gereksinimler);
        std::fs::write(self.kilit_yolu(), metin)
            .map_err(|e| io_hata("Sürüm kilidi yazılamadı", &self.kilit_yolu(), e))
    }

    /// Kurulu paketlerin tümünü **sabit sürümle** kilit dosyasına dökerek dondurur (`pip freeze` ruhu).
    pub fn kilidi_kurulu_paketlerden_yaz(&self) -> Result<Vec<PaketGereksinimi>, ErrorReport> {
        let gereksinimler: Vec<PaketGereksinimi> = self
            .paketler()?
            .into_iter()
            .map(|p| PaketGereksinimi::yeni(p.ad, Some(p.surum)))
            .collect();
        self.kilit_yaz(&gereksinimler)?;
        Ok(gereksinimler)
    }

    /// Kilit dosyasını okur (yoksa boş liste — hata değil).
    pub fn kilit_oku(&self) -> Result<Vec<PaketGereksinimi>, ErrorReport> {
        let yol = self.kilit_yolu();
        if !yol.exists() {
            return Ok(Vec::new());
        }
        let metin = std::fs::read_to_string(&yol)
            .map_err(|e| io_hata("Sürüm kilidi okunamadı", &yol, e))?;
        Ok(kilit_coz(&metin))
    }

    /// Kilitte olup **kurulu olmayan** paketleri döner (editör "[Kur]" için).  **Bloklar** (pip list).
    pub fn eksik_paketler(&self) -> Result<Vec<PaketGereksinimi>, ErrorReport> {
        let kilit = self.kilit_oku()?;
        if kilit.is_empty() {
            return Ok(Vec::new());
        }
        let kurulu = self.paketler()?;
        Ok(eksikleri_bul(&kilit, &kurulu))
    }

    // ── iç yardımcılar ──

    fn venv_gerekli(&self) -> Result<(), ErrorReport> {
        if self.var_mi() {
            Ok(())
        } else {
            Err(ErrorReport::new(
                "Sanal ortam bulunamadı",
                "Bu proje için izole Python ortamı henüz oluşturulmadı",
                "Önce proje ortamını oluşturun ([Ortamı kur])",
            )
            .with_eylem("Ortamı kur"))
        }
    }
}

// ─── Saf yardımcılar (birim-testlenebilir) ──────────────────────────────────────

/// Kilit dosyası metnini üretir (sıralı, yorum başlıklı).
pub fn kilit_metni(gereksinimler: &[PaketGereksinimi]) -> String {
    let mut sirali: Vec<&PaketGereksinimi> = gereksinimler.iter().collect();
    sirali.sort_by_key(|g| g.ad.to_lowercase());
    let mut s = String::new();
    s.push_str("# BioCraft Engine — proje paket sürüm kilidi (İP-06).\n");
    s.push_str(
        "# Bu dosya projenin izole ortamındaki paketleri sabitler (tekrar-üretilebilirlik).\n",
    );
    for g in sirali {
        s.push_str(&g.kilit_satiri());
        s.push('\n');
    }
    s
}

/// Kilit dosyası metnini ayrıştırır (yorum/boş satır atlanır; geçersiz satır yok sayılır).
pub fn kilit_coz(metin: &str) -> Vec<PaketGereksinimi> {
    metin
        .lines()
        .filter_map(PaketGereksinimi::ayristir)
        .collect()
}

/// `pip list --format=json` çıktısını ayrıştırır (saf — testlenebilir).
fn pip_list_coz(json: &str) -> Vec<KuruluPaket> {
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let Some(dizi) = v.as_array() else {
        return Vec::new();
    };
    dizi.iter()
        .filter_map(|e| {
            let ad = e.get("name")?.as_str()?.to_string();
            let surum = e.get("version")?.as_str()?.to_string();
            Some(KuruluPaket { ad, surum })
        })
        .collect()
}

/// Kilitte olup kurulu olmayan (veya **sürümü uyuşmayan**) gereksinimleri döner.
pub fn eksikleri_bul(kilit: &[PaketGereksinimi], kurulu: &[KuruluPaket]) -> Vec<PaketGereksinimi> {
    kilit
        .iter()
        .filter(|g| {
            let ad = g.ad.to_lowercase();
            match kurulu.iter().find(|p| p.ad.to_lowercase() == ad) {
                None => true, // hiç kurulu değil
                Some(p) => match &g.surum {
                    // Sürüm sabitlenmişse ve kurulu sürüm farklıysa → eksik (yanlış sürüm).
                    Some(istenen) => istenen != &p.surum,
                    None => false,
                },
            }
        })
        .cloned()
        .collect()
}

/// Bir paket adının kabul edilebilir olup olmadığını denetler (kabuk enjeksiyonu/saçma giriş savunması).
fn paket_adi_gecerli(ad: &str) -> bool {
    !ad.is_empty()
        && ad.len() <= 100
        && ad
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && ad
            .chars()
            .next()
            .map(|c| c.is_ascii_alphanumeric())
            .unwrap_or(false)
}

fn gecersiz_paket_adi(ad: &str) -> ErrorReport {
    ErrorReport::new(
        "Geçersiz paket adı",
        format!("'{ad}' geçerli bir paket adı değil"),
        "Yalnız harf/rakam/-/_/. içeren bir paket adı girin",
    )
}

fn pip_baslatma_hatasi(e: std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "pip çalıştırılamadı",
        "sanal ortamın pip'i başlatılamadı",
        "Sanal ortamı yeniden oluşturmayı deneyin",
    )
    .with_teknik_detay(e.to_string())
}

fn io_hata(ne: &str, yol: &Path, e: std::io::Error) -> ErrorReport {
    ErrorReport::new(
        ne,
        format!("'{}' işlenemedi", yol.display()),
        "Disk alanını ve klasör iznini kontrol edin",
    )
    .with_teknik_detay(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gereksinim_ayristir_surumlu_surumsuz() {
        let g = PaketGereksinimi::ayristir("numpy==1.26.0").unwrap();
        assert_eq!(g.ad, "numpy");
        assert_eq!(g.surum.as_deref(), Some("1.26.0"));
        assert_eq!(g.pip_argumani(), "numpy==1.26.0");

        let g2 = PaketGereksinimi::ayristir("  jedi  ").unwrap();
        assert_eq!(g2.ad, "jedi");
        assert_eq!(g2.surum, None);
        assert_eq!(g2.pip_argumani(), "jedi");
    }

    #[test]
    fn gereksinim_yorum_ve_bos_atlanir() {
        assert!(PaketGereksinimi::ayristir("# yorum").is_none());
        assert!(PaketGereksinimi::ayristir("   ").is_none());
        // Kötü niyetli/geçersiz ad reddedilir (kabuk enjeksiyonu savunması).
        assert!(PaketGereksinimi::ayristir("rm -rf /; numpy").is_none());
    }

    #[test]
    fn kilit_gidis_donus() {
        let g = vec![
            PaketGereksinimi::yeni("numpy", Some("1.26.0".into())),
            PaketGereksinimi::yeni("Biopython", Some("1.83".into())),
            PaketGereksinimi::yeni("jedi", None),
        ];
        let metin = kilit_metni(&g);
        // Sıralı (Biopython < jedi < numpy).
        let geri = kilit_coz(&metin);
        assert_eq!(geri.len(), 3);
        assert_eq!(geri[0].ad, "Biopython");
        assert_eq!(geri[1].ad, "jedi");
        assert_eq!(geri[2].ad, "numpy");
        assert_eq!(geri[2].surum.as_deref(), Some("1.26.0"));
    }

    #[test]
    fn pip_list_coz_calisir() {
        let json =
            r#"[{"name": "numpy", "version": "1.26.0"}, {"name": "jedi", "version": "0.19.1"}]"#;
        let p = pip_list_coz(json);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].ad, "numpy");
        assert_eq!(p[0].surum, "1.26.0");
    }

    #[test]
    fn eksik_paketler_bulunur() {
        let kilit = vec![
            PaketGereksinimi::yeni("numpy", Some("1.26.0".into())),
            PaketGereksinimi::yeni("jedi", None),
            PaketGereksinimi::yeni("pandas", Some("2.0.0".into())),
        ];
        let kurulu = vec![
            KuruluPaket {
                ad: "numpy".into(),
                surum: "1.26.0".into(), // doğru sürüm → eksik değil
            },
            KuruluPaket {
                ad: "pandas".into(),
                surum: "1.5.0".into(), // yanlış sürüm → eksik
            },
            // jedi hiç yok → eksik
        ];
        let eksik = eksikleri_bul(&kilit, &kurulu);
        let adlar: Vec<&str> = eksik.iter().map(|g| g.ad.as_str()).collect();
        assert!(adlar.contains(&"jedi"));
        assert!(adlar.contains(&"pandas"));
        assert!(!adlar.contains(&"numpy"));
    }

    #[test]
    fn yorumlayici_yolu_platforma_gore() {
        let o = SanalOrtam::yeni("C:/projeler/deney");
        let y = o.yorumlayici();
        if cfg!(windows) {
            assert!(y.ends_with("Scripts/python.exe") || y.ends_with("Scripts\\python.exe"));
        } else {
            assert!(y.ends_with("bin/python"));
        }
        assert!(y.starts_with("C:/projeler/deney"));
    }

    #[test]
    fn olmayan_venv_var_mi_false() {
        let o = SanalOrtam::yeni(std::env::temp_dir().join("biocraft_olmayan_venv_xyz"));
        assert!(!o.var_mi());
        // venv yokken paket işlemi net hata verir (panik değil).
        assert!(o.paketler().is_err());
    }

    #[test]
    fn kilit_dosyasi_gidis_donus_diske() {
        let dizin =
            std::env::temp_dir().join(format!("biocraft_kilit_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dizin);
        let o = SanalOrtam::yeni(&dizin);
        // Boşta kilit okuma → boş (hata değil).
        assert!(o.kilit_oku().unwrap().is_empty());
        let g = vec![PaketGereksinimi::yeni("jedi", Some("0.19.1".into()))];
        o.kilit_yaz(&g).unwrap();
        let geri = o.kilit_oku().unwrap();
        assert_eq!(geri, g);
        let _ = std::fs::remove_dir_all(&dizin);
    }

    /// İsteğe bağlı uçtan uca: gerçek venv oluştur + paket kur + kilitle.
    /// Yavaş + ağ gerektirir → yalnız `BIOCRAFT_VENV_TEST=1` ile çalışır (CI'ı yavaşlatmaz).
    #[test]
    fn venv_uctan_uca_istege_bagli() {
        if std::env::var("BIOCRAFT_VENV_TEST").is_err() {
            eprintln!("BIOCRAFT_VENV_TEST ayarlı değil → atlandı");
            return;
        }
        let Some(py) = crate::runtime::subprocess::python_bul() else {
            eprintln!("Python yok → atlandı");
            return;
        };
        let dizin = std::env::temp_dir().join(format!("biocraft_venv_e2e_{}", std::process::id()));
        let o = SanalOrtam::yeni(&dizin);
        o.olustur(&py).unwrap();
        assert!(o.var_mi(), "venv oluşmalı");
        o.paket_kur(&PaketGereksinimi::yeni("jedi", None)).unwrap();
        assert!(o.paket_var_mi("jedi").unwrap(), "jedi kurulu olmalı");
        let kilit = o.kilidi_kurulu_paketlerden_yaz().unwrap();
        assert!(kilit.iter().any(|g| g.ad.to_lowercase() == "jedi"));
        let _ = std::fs::remove_dir_all(&dizin);
    }
}
