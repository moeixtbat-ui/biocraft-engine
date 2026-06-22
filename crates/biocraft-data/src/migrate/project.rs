//! Proje formatı göçü — açılışta sürüm denetimi + deterministik göç + yedek + salt-okunur (İP-19, MK-59).
//!
//! Açma akışı ([`ac_ve_goc`]):
//! 1. **Sürüm probu** (toleranslı): manifestten yalnızca format sürümü okunur.
//! 2. **Durum** ([`super::degerlendir`]): Güncel / GöçGerekli / DahaYeni.
//!    - **Güncel** → doğrudan [`crate::project::ac`] (bütünlük denetimiyle) açılır.
//!    - **DahaYeni** → **salt-okunur** açılır + "daha yeni BioCraft gerekiyor" uyarısı (bozma yok).
//!    - **GöçGerekli** → bütünlük doğrulanır → zincir çözülür → (onay politikasına göre) **yedek
//!      alınır** → göç **atomik** uygulanır → strict doğrulama → yeniden açılıp bütünlük denetlenir.
//!      Herhangi bir adım başarısızsa **yedekten geri yüklenir** (proje hiç bozulmaz) + net hata.
//! 3. **Desteklenmiyor** (göç yolu yok) → net hata + çözüm (sessiz bozulma yok).

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use biocraft_types::{ErrorReport, Version};

use crate::project::manifest::GocKaydi;
use crate::project::provenance::{olay_turu, ProvenansOlay};
use crate::project::{cekirdek_dosyalari_yaz, format, integrity, AcilanProje, Manifest, Meta};

use super::{degerlendir, Goc, GocKayit, SurumDurumu, ZincirHatasi};

// ─── Üretim göç kaydı ─────────────────────────────────────────────────────────

/// **Proje formatının** üretim göç kaydı (vN → vN+1 deterministik adımlar).
///
/// İlk sürümde (format 1.0.0'ın öncülü yok) **boştur** — altyapı hazır; gerçek göç kuralları
/// format sürümü ilerledikçe (1.0 → 1.1 → …) buraya eklenir (`MVP-sonrasi.md` §9.1).  Yeni bir adım
/// eklemek = `.ekle(Goc { kaynak, hedef, aciklama, kirici, donustur })` çağrısı; gerisini çerçeve
/// (sürüm/geçmiş/mühür/yedek) halleder.
pub fn proje_format_kayit() -> GocKayit {
    GocKayit::yeni()
}

// ─── Onay politikası ────────────────────────────────────────────────────────────

/// Göçün ne zaman otomatik uygulanacağını belirler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnayPolitikasi {
    /// Tüm göçleri otomatik uygula (kullanıcı önceden onayladı).
    Otomatik,
    /// Kırıcı-olmayan göçü otomatik uygula; **kırıcı** değişiklik varsa onay iste.
    KiriciIcinOnay,
    /// Hiçbir göç uygulama; yalnızca planı bildir (kuru çalışma / UI önizleme).
    YalnizcaBildir,
}

// ─── Plan + özet + salt-okunur ───────────────────────────────────────────────────

/// Bir göç adımının kullanıcıya gösterilebilir özeti (UI önizleme + onay için).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GocAdimOzeti {
    /// Kaynak format sürümü.
    pub kaynak: Version,
    /// Hedef format sürümü.
    pub hedef: Version,
    /// İnsan-okunur açıklama.
    pub aciklama: String,
    /// Kırıcı (geriye dönük uyumsuz) mı?
    pub kirici: bool,
}

impl GocAdimOzeti {
    fn goçten(g: &Goc) -> Self {
        Self {
            kaynak: g.kaynak.clone(),
            hedef: g.hedef.clone(),
            aciklama: g.aciklama.to_string(),
            kirici: g.kirici,
        }
    }
}

/// Bir göç planı — UI bunu kullanıcıya gösterip onay alır (sonra [`OnayPolitikasi::Otomatik`] ile çağırır).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GocPlani {
    /// Sürüm durumu (göç gerekli olduğu varsayılır; bu plan yalnızca göç gerektiğinde üretilir).
    pub durum: SurumDurumu,
    /// Uygulanacak adımlar (sırayla).
    pub adimlar: Vec<GocAdimOzeti>,
    /// Adımlardan herhangi biri kırıcı mı? (kırıcıysa kullanıcı uyarılır + yedek vurgulanır)
    pub kirici_var: bool,
}

/// Daha yeni sürümle yapılmış bir projenin **salt-okunur** açılımı (bozma yok).
#[derive(Debug, Clone)]
pub struct SaltOkunurProje {
    /// Proje kök klasörü.
    pub kok: PathBuf,
    /// Dosyanın (daha yeni) format sürümü.
    pub dosya_surumu: Version,
    /// Bu uygulamanın desteklediği format sürümü.
    pub uygulama_surumu: Version,
    /// En iyi çaba ile ayrıştırılan manifest (yeni alanlar serde tarafından göz ardı edilir);
    /// strict ayrıştırma başarısızsa `None` — proje yine salt-okunur açılır, sürüm bilinir.
    pub manifest: Option<Manifest>,
}

// ─── Göç sonucu ───────────────────────────────────────────────────────────────

/// [`ac_ve_goc`] sonucu.
#[derive(Debug)]
pub enum GocSonucu {
    /// Proje zaten güncel; doğrudan açıldı.
    Guncel(AcilanProje),
    /// Göç başarıyla uygulandı; proje yeni sürümde açıldı.
    GocEdildi {
        /// Göç sonrası açılan (bütünlüğü doğrulanmış) proje.
        proje: AcilanProje,
        /// Uygulanan göç kayıtları (manifest geçmişine de eklendi).
        uygulanan: Vec<GocKaydi>,
        /// Göç öncesi alınan yedeğin klasörü (gerekirse elle geri dönülebilir).
        yedek_dizini: PathBuf,
    },
    /// Daha yeni sürümle yapılmış → **salt-okunur** açıldı + uyarı.
    SaltOkunur {
        /// Salt-okunur proje.
        proje: SaltOkunurProje,
        /// "Daha yeni BioCraft gerekiyor" uyarısı.
        uyari: ErrorReport,
    },
    /// Göç gerekli ama onay bekleniyor (politika gereği) → UI planı gösterir, onaylayınca
    /// [`OnayPolitikasi::Otomatik`] ile yeniden çağrılır.
    OnayGerekli {
        /// Uygulanacak göç planı.
        plan: GocPlani,
    },
}

// ─── Açma + göç (üretim API'si) ──────────────────────────────────────────────────

/// Bir projeyi açar; gerekiyorsa **üretim kaydıyla** güncel format sürümüne göç eder.
pub fn ac_ve_goc(kok: &Path, politika: OnayPolitikasi) -> Result<GocSonucu, ErrorReport> {
    ac_ve_goc_ile(
        kok,
        &proje_format_kayit(),
        &format::format_surumu(),
        politika,
    )
}

/// Yalnızca **planı** üretir (uygulamaz) — UI önizleme/onay için.
///
/// Güncel proje → `Ok(None)` (göç gerekmez).  Göç gerekli → `Ok(Some(plan))`.  Daha yeni / bozuk /
/// desteklenmeyen → ilgili hata yine [`ac_ve_goc`] üzerinden raporlanır; bu fonksiyon yalnızca
/// "göç gerekli mi, hangi adımlarla" sorusunu yanıtlar.
pub fn goc_plani(kok: &Path) -> Result<Option<GocPlani>, ErrorReport> {
    goc_plani_ile(kok, &proje_format_kayit(), &format::format_surumu())
}

// ─── Açma + göç (çekirdek; test edilebilir) ───────────────────────────────────────

/// [`ac_ve_goc`]'in kayıt + hedef sürümü dışarıdan alan biçimi (test/ileri kullanım).
pub fn ac_ve_goc_ile(
    kok: &Path,
    kayit: &GocKayit,
    hedef: &Version,
    politika: OnayPolitikasi,
) -> Result<GocSonucu, ErrorReport> {
    let manifest_metin = manifest_metni_oku(kok)?;
    let dosya_surumu = super::manifest_surumu_oku(&manifest_metin)?;

    match degerlendir(&dosya_surumu, hedef) {
        SurumDurumu::Guncel => Ok(GocSonucu::Guncel(crate::project::ac(kok)?)),

        SurumDurumu::DahaYeni { dosya, uygulama } => {
            // En iyi çaba ayrıştırma (yeni major strict parse'ı bozabilir → None).
            let manifest = Manifest::toml_coz(&manifest_metin).ok();
            Ok(GocSonucu::SaltOkunur {
                uyari: daha_yeni_uyarisi(&dosya, &uygulama),
                proje: SaltOkunurProje {
                    kok: kok.to_path_buf(),
                    dosya_surumu: dosya,
                    uygulama_surumu: uygulama,
                    manifest,
                },
            })
        }

        SurumDurumu::GocGerekli { mevcut, hedef: h } => {
            // 1) Bütünlük + strict ayrıştırma — **bozuk projeyi göç etmeyiz** (net hata önce gelir).
            let acilan = crate::project::ac(kok)?;

            // 2) Deterministik göç zincirini çöz.
            let zincir = kayit
                .coz(&mevcut, &h)
                .map_err(|e| desteklenmiyor_hatasi(&mevcut, &h, &e))?;
            let plan = GocPlani {
                durum: SurumDurumu::GocGerekli {
                    mevcut: mevcut.clone(),
                    hedef: h.clone(),
                },
                adimlar: zincir.iter().map(|g| GocAdimOzeti::goçten(g)).collect(),
                kirici_var: zincir.iter().any(|g| g.kirici),
            };

            // 3) Onay politikası.
            match politika {
                OnayPolitikasi::YalnizcaBildir => return Ok(GocSonucu::OnayGerekli { plan }),
                OnayPolitikasi::KiriciIcinOnay if plan.kirici_var => {
                    return Ok(GocSonucu::OnayGerekli { plan })
                }
                _ => {}
            }

            // 4) Göç ÖNCESİ otomatik yedek (çekirdek format dosyaları).
            let etiket = format!("goc-{mevcut}-{h}");
            let yedek = Yedek::al(kok, &etiket)?;

            // 5) Göçü atomik uygula; herhangi bir başarısızlıkta yedekten geri yükle.
            let uygula = goc_uygula_ic(kok, &acilan, &zincir, &h, &manifest_metin);
            match uygula {
                Ok(uygulanan) => match crate::project::ac(kok) {
                    // 6) Doğrula: göç sonrası proje bütünlük + strict denetiminden geçmeli.
                    Ok(proje) => Ok(GocSonucu::GocEdildi {
                        proje,
                        uygulanan,
                        yedek_dizini: yedek.dizin.clone(),
                    }),
                    Err(e) => {
                        yedek.geri_yukle()?;
                        Err(geri_yuklendi_hatasi(e))
                    }
                },
                Err(e) => {
                    yedek.geri_yukle()?;
                    Err(geri_yuklendi_hatasi(e))
                }
            }
        }
    }
}

/// [`goc_plani`]'nın kayıt + hedef sürümü dışarıdan alan biçimi.
pub fn goc_plani_ile(
    kok: &Path,
    kayit: &GocKayit,
    hedef: &Version,
) -> Result<Option<GocPlani>, ErrorReport> {
    let manifest_metin = manifest_metni_oku(kok)?;
    let dosya_surumu = super::manifest_surumu_oku(&manifest_metin)?;
    match degerlendir(&dosya_surumu, hedef) {
        SurumDurumu::Guncel | SurumDurumu::DahaYeni { .. } => Ok(None),
        SurumDurumu::GocGerekli { mevcut, hedef: h } => {
            let zincir = kayit
                .coz(&mevcut, &h)
                .map_err(|e| desteklenmiyor_hatasi(&mevcut, &h, &e))?;
            Ok(Some(GocPlani {
                durum: SurumDurumu::GocGerekli { mevcut, hedef: h },
                adimlar: zincir.iter().map(|g| GocAdimOzeti::goçten(g)).collect(),
                kirici_var: zincir.iter().any(|g| g.kirici),
            }))
        }
    }
}

// ─── Göç uygulama (iç) ────────────────────────────────────────────────────────

/// Göç zincirini deterministik uygular: manifest `toml::Value` üzerinde dönüştür → strict doğrula →
/// sürüm + geçmiş güncelle → çekirdek dosyaları atomik yaz + mühürle → provenance kaydı.
fn goc_uygula_ic(
    kok: &Path,
    acilan: &AcilanProje,
    zincir: &[&Goc],
    hedef: &Version,
    manifest_metin: &str,
) -> Result<Vec<GocKaydi>, ErrorReport> {
    // Manifesti toleranslı biçimde aç (dönüşümler şema alanlarıyla çalışır).
    let mut deger: toml::Value = toml::from_str(manifest_metin).map_err(|e| {
        ErrorReport::new(
            "Göç için manifest okunamadı",
            "biocraft.toml geçerli bir TOML değil.",
            "Dosyayı yedekten geri yükleyin.",
        )
        .with_teknik_detay(format!("toml value de: {e}"))
    })?;

    let simdi = Utc::now();
    let mut uygulanan: Vec<GocKaydi> = Vec::new();
    for adim in zincir {
        (adim.donustur)(&mut deger).map_err(|e| {
            ErrorReport::new(
                "Göç adımı başarısız",
                format!(
                    "'{}' → '{}' göç adımı uygulanırken sorun oluştu: {}",
                    adim.kaynak, adim.hedef, e.ne_oldu
                ),
                "Bu bir göç hatasıdır; proje yedekten geri yüklendi (değişiklik uygulanmadı).",
            )
            .with_teknik_detay(format!("{}: {}", adim.aciklama, e.neden))
        })?;
        uygulanan.push(GocKaydi {
            surum: adim.hedef.clone(),
            tarih: simdi,
            aciklama: adim.aciklama.to_string(),
        });
    }

    // Strict doğrulama: dönüşmüş manifest hâlâ geçerli bir BioCraft manifesti mi? (deterministik kapı)
    let yeni_metin = toml::to_string_pretty(&deger).map_err(|e| {
        ErrorReport::new(
            "Göç sonucu yazılamadı",
            "Dönüştürülmüş manifest TOML'a çevrilirken sorun oluştu.",
            "Bu bir iç hatadır; proje yedekten geri yüklendi.",
        )
        .with_teknik_detay(format!("toml ser: {e}"))
    })?;
    let mut yeni_manifest = Manifest::toml_coz(&yeni_metin)?;

    // Sürüm + göç geçmişi + değiştirme zamanı (çerçeve yönetir).
    yeni_manifest.goc.extend(uygulanan.iter().cloned());
    yeni_manifest.kimlik.format_surumu = hedef.clone();
    yeni_manifest.dokun();

    // Meta'yı tutarlı kıl.
    let mut yeni_meta: Meta = acilan.meta.clone();
    yeni_meta.format_surumu = hedef.clone();
    yeni_meta.uygulanan_goc_sayisi = yeni_manifest.goc.len();

    // Çekirdek dosyaları atomik yaz + yeniden mühürle (tek kaynak helper).
    cekirdek_dosyalari_yaz(kok, &yeni_manifest, &yeni_meta)?;

    // Provenance: her uygulanan göç bir olay bırakır (köken/iz).
    for k in &uygulanan {
        let olay = ProvenansOlay::yeni(
            olay_turu::GOC_UYGULANDI,
            format!("Göç → {} ({})", k.surum, k.aciklama),
            acilan.manifest.kimlik.biocraft_surumu.clone(),
        );
        crate::project::provenance::olay_ekle(kok, &olay)?;
    }

    Ok(uygulanan)
}

// ─── Yedek (göç öncesi; atomik geri dönüş) ──────────────────────────────────────

/// Göç öncesi alınan **çekirdek format dosyaları** yedeği (manifest + meta + bütünlük mührü).
///
/// `.biocraft_meta/yedekler/<etiket>-<zaman>/` altına kopyalanır.  Göç başarısızsa
/// [`geri_yukle`](Yedek::geri_yukle) ile atomik olarak eski hâle dönülür → proje **hiç bozulmaz**.
/// (Gelecekte gömülü veri göçleri eklendiğinde yedek kapsamı genişletilir; kanca hazır.)
#[derive(Debug, Clone)]
pub struct Yedek {
    /// Yedek klasörü.
    pub dizin: PathBuf,
    /// (orijinal yol, yedek kopya yolu) çiftleri.
    kopyalar: Vec<(PathBuf, PathBuf)>,
}

impl Yedek {
    /// Çekirdek format dosyalarının yedeğini alır.
    pub fn al(kok: &Path, etiket: &str) -> Result<Yedek, ErrorReport> {
        let damga = Utc::now().format("%Y%m%dT%H%M%S%3f").to_string();
        let dizin = format::alt_yol(
            kok,
            &[format::META_DIZIN, "yedekler", &format!("{etiket}-{damga}")],
        );
        fs::create_dir_all(&dizin)
            .map_err(|e| integrity::io_hatasi("Yedek klasörü oluşturulamadı", &dizin, &e))?;

        let mut kopyalar = Vec::new();
        for orijinal in [
            format::manifest_yolu(kok),
            format::meta_yolu(kok),
            format::butunluk_yolu(kok),
        ] {
            if !orijinal.is_file() {
                continue;
            }
            let ad = orijinal
                .file_name()
                .map(|s| s.to_os_string())
                .unwrap_or_default();
            let yedek_yol = dizin.join(&ad);
            let veri = fs::read(&orijinal)
                .map_err(|e| integrity::io_hatasi("Yedek için dosya okunamadı", &orijinal, &e))?;
            integrity::atomik_yaz(&yedek_yol, &veri)?;
            kopyalar.push((orijinal, yedek_yol));
        }
        Ok(Yedek { dizin, kopyalar })
    }

    /// Yedekteki dosyaları orijinal konumlarına **atomik** geri yazar (göç başarısızsa).
    pub fn geri_yukle(&self) -> Result<(), ErrorReport> {
        for (orijinal, yedek_yol) in &self.kopyalar {
            let veri = fs::read(yedek_yol)
                .map_err(|e| integrity::io_hatasi("Yedek dosyası okunamadı", yedek_yol, &e))?;
            integrity::atomik_yaz(orijinal, &veri)?;
        }
        Ok(())
    }
}

// ─── Yardımcılar + hatalar ───────────────────────────────────────────────────────

/// Manifest metnini okur; dosya yoksa "proje değil" net hatası döner.
fn manifest_metni_oku(kok: &Path) -> Result<String, ErrorReport> {
    let yol = format::manifest_yolu(kok);
    match fs::read_to_string(&yol) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(ErrorReport::new(
            "Bu klasör bir BioCraft projesi değil",
            format!("'{}' içinde biocraft.toml bulunamadı.", kok.display()),
            "Geçerli bir proje klasörü (biocraft.toml içeren) seçin.",
        )),
        Err(e) => Err(integrity::io_hatasi("Manifest okunamadı", &yol, &e)),
    }
}

/// Daha yeni sürümle yapılmış proje için salt-okunur uyarısı (bozma yok — yalnızca bilgi).
fn daha_yeni_uyarisi(dosya: &Version, uygulama: &Version) -> ErrorReport {
    ErrorReport::new(
        "Bu proje daha yeni bir BioCraft sürümüyle yapılmış",
        format!(
            "Proje format sürümü {dosya}; bu uygulama en fazla {uygulama} destekliyor. Proje \
             güvenle açıldı ancak **salt-okunur** — kaydetmek yeni özellikleri bozabileceğinden engellendi."
        ),
        "Projeyi düzenlemek için BioCraft'ı en son sürüme güncelleyin.",
    )
    .with_eylem("BioCraft'ı güncelle")
}

/// Göç yolu bulunamadığında (çok eski/desteklenmeyen format) net hata + çözüm.
fn desteklenmiyor_hatasi(mevcut: &Version, hedef: &Version, e: &ZincirHatasi) -> ErrorReport {
    ErrorReport::new(
        "Bu proje sürümü doğrudan yükseltilemiyor",
        format!(
            "Proje format sürümü {mevcut}; hedef {hedef}. Bu iki sürüm arasında tanımlı bir göç \
             yolu yok (format çok eski veya desteklenmiyor)."
        ),
        "Projeyi açabilen bir ara BioCraft sürümüyle önce yükseltin ya da projeyi yeniden dışa aktarın.",
    )
    .with_teknik_detay(format!("zincir çözüm: {e:?}"))
}

/// Göç başarısız olup **yedekten geri yüklendiğinde** kullanıcıya verilen net hata.
fn geri_yuklendi_hatasi(asil: ErrorReport) -> ErrorReport {
    ErrorReport::new(
        "Göç tamamlanamadı — proje eski hâline geri yüklendi",
        format!(
            "Göç sırasında bir sorun oluştu ({}). Proje **bozulmadı**; göç öncesi yedekten otomatik \
             geri yüklendi.",
            asil.ne_oldu
        ),
        "Hata kimliğiyle bildirin; proje güvende ve önceki sürümde açılabilir.",
    )
    .with_teknik_detay(format!("asıl: {} — {}", asil.ne_oldu, asil.neden))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{olustur, ProjeKurulumGirdisi};
    use biocraft_types::DataClassification;

    fn v(a: u32, b: u32, c: u32) -> Version {
        Version::new(a, b, c)
    }

    fn gecici_kok(etiket: &str) -> PathBuf {
        let ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let p =
            std::env::temp_dir().join(format!("bc_goc_{}_{}_{}", etiket, std::process::id(), ns));
        let _ = fs::remove_dir_all(&p);
        p
    }

    /// Diske gerçek bir (format 1.0.0) proje kurar, kök yolunu döndürür.
    fn ornek_proje(etiket: &str) -> (PathBuf, PathBuf) {
        let konum = gecici_kok(etiket);
        fs::create_dir_all(&konum).unwrap();
        let mut g = ProjeKurulumGirdisi::yeni(
            "Eski Proje",
            &konum,
            "genomik",
            DataClassification::Sentetik,
            v(0, 1, 0),
        );
        g.etiketler = vec!["başlangıç".to_string()];
        let kurulan = olustur(&g).unwrap();
        (konum, kurulan.kok)
    }

    // ── Test göç fonksiyonları (üretim değil; çerçeveyi çalıştırır) ──────────────

    /// 1.0.0 → 1.1.0: sınıflandırma etiketlerine "format-1.1" ekler (kırıcı değil, golden).
    fn goc_1_0_to_1_1(deger: &mut toml::Value) -> Result<(), ErrorReport> {
        let Some(siniflandirma) = deger
            .get_mut("siniflandirma")
            .and_then(|s| s.as_table_mut())
        else {
            return Err(ErrorReport::new("siniflandirma yok", "beklenmedik", "—"));
        };
        let etiketler = siniflandirma
            .entry("etiketler".to_string())
            .or_insert_with(|| toml::Value::Array(Vec::new()));
        if let Some(arr) = etiketler.as_array_mut() {
            arr.push(toml::Value::String("format-1.1".to_string()));
        }
        Ok(())
    }

    /// 1.1.0 → 2.0.0: KIRICI — uyumluluk etiketine "v2-semasi" ekler (onay gerektirir).
    fn goc_1_1_to_2_0(deger: &mut toml::Value) -> Result<(), ErrorReport> {
        let siniflandirma = deger
            .get_mut("siniflandirma")
            .and_then(|s| s.as_table_mut())
            .unwrap();
        let uyumluluk = siniflandirma
            .entry("uyumluluk".to_string())
            .or_insert_with(|| toml::Value::Array(Vec::new()));
        if let Some(arr) = uyumluluk.as_array_mut() {
            arr.push(toml::Value::String("v2-semasi".to_string()));
        }
        Ok(())
    }

    /// BOZUK göç: zorunlu alanı (siniflandirma.sinif) siler → strict doğrulama başarısız olmalı.
    fn goc_bozuk(deger: &mut toml::Value) -> Result<(), ErrorReport> {
        if let Some(s) = deger
            .get_mut("siniflandirma")
            .and_then(|s| s.as_table_mut())
        {
            s.remove("sinif");
        }
        Ok(())
    }

    fn kayit_1_1() -> GocKayit {
        GocKayit::yeni().ekle(Goc {
            kaynak: v(1, 0, 0),
            hedef: v(1, 1, 0),
            aciklama: "Etiket şeması güncellendi",
            kirici: false,
            donustur: goc_1_0_to_1_1,
        })
    }

    // ── Golden / kabul testleri ──────────────────────────────────────────────────

    #[test]
    fn guncel_proje_dogrudan_acilir() {
        let (konum, kok) = ornek_proje("guncel");
        let sonuc =
            ac_ve_goc_ile(&kok, &kayit_1_1(), &v(1, 0, 0), OnayPolitikasi::Otomatik).unwrap();
        assert!(matches!(sonuc, GocSonucu::Guncel(_)));
        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn eski_proje_otomatik_goc_ile_acilir() {
        // GOLDEN: format 1.0.0 proje → 1.1.0'a göç → etiket eklenmiş + sürüm yükselmiş + yeniden açılabilir.
        let (konum, kok) = ornek_proje("goc_ileri");
        let sonuc =
            ac_ve_goc_ile(&kok, &kayit_1_1(), &v(1, 1, 0), OnayPolitikasi::Otomatik).unwrap();

        let GocSonucu::GocEdildi {
            proje,
            uygulanan,
            yedek_dizini,
        } = sonuc
        else {
            panic!("göç bekleniyordu, gelen: {sonuc:?}");
        };

        // Sürüm yükseldi.
        assert_eq!(proje.manifest.kimlik.format_surumu, v(1, 1, 0));
        assert_eq!(proje.meta.format_surumu, v(1, 1, 0));
        // Göç deterministik dönüşümü uyguladı (etiket eklendi).
        assert!(proje
            .manifest
            .siniflandirma
            .etiketler
            .contains(&"format-1.1".to_string()));
        // Göç geçmişi büyüdü (ilk oluşturma + 1 göç).
        assert_eq!(proje.manifest.goc.len(), 2);
        assert_eq!(uygulanan.len(), 1);
        assert_eq!(uygulanan[0].surum, v(1, 1, 0));
        // Yedek alındı.
        assert!(yedek_dizini.is_dir());
        assert!(yedek_dizini.join(format::MANIFEST_DOSYA).is_file());

        // Yeniden açmak göç gerektirmemeli (artık güncel).
        let tekrar =
            ac_ve_goc_ile(&kok, &kayit_1_1(), &v(1, 1, 0), OnayPolitikasi::Otomatik).unwrap();
        assert!(matches!(tekrar, GocSonucu::Guncel(_)));

        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn cok_adimli_zincir_uygulanir() {
        let (konum, kok) = ornek_proje("zincir");
        let kayit = GocKayit::yeni()
            .ekle(Goc {
                kaynak: v(1, 0, 0),
                hedef: v(1, 1, 0),
                aciklama: "1.1 şeması",
                kirici: false,
                donustur: goc_1_0_to_1_1,
            })
            .ekle(Goc {
                kaynak: v(1, 1, 0),
                hedef: v(2, 0, 0),
                aciklama: "2.0 şeması",
                kirici: true,
                donustur: goc_1_1_to_2_0,
            });
        let sonuc = ac_ve_goc_ile(&kok, &kayit, &v(2, 0, 0), OnayPolitikasi::Otomatik).unwrap();
        let GocSonucu::GocEdildi {
            proje, uygulanan, ..
        } = sonuc
        else {
            panic!("göç bekleniyordu");
        };
        assert_eq!(proje.manifest.kimlik.format_surumu, v(2, 0, 0));
        assert_eq!(uygulanan.len(), 2);
        assert!(proje
            .manifest
            .siniflandirma
            .etiketler
            .contains(&"format-1.1".to_string()));
        assert!(proje
            .manifest
            .siniflandirma
            .uyumluluk
            .contains(&"v2-semasi".to_string()));
        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn kirici_goc_onay_bekler() {
        let (konum, kok) = ornek_proje("kirici_onay");
        let kayit = GocKayit::yeni().ekle(Goc {
            kaynak: v(1, 0, 0),
            hedef: v(2, 0, 0),
            aciklama: "kırıcı",
            kirici: true,
            donustur: goc_1_1_to_2_0,
        });
        // KiriciIcinOnay politikası → uygulanmaz, plan döner.
        let sonuc =
            ac_ve_goc_ile(&kok, &kayit, &v(2, 0, 0), OnayPolitikasi::KiriciIcinOnay).unwrap();
        let GocSonucu::OnayGerekli { plan } = sonuc else {
            panic!("onay bekleniyordu");
        };
        assert!(plan.kirici_var);
        assert_eq!(plan.adimlar.len(), 1);
        // Disk değişmemeli (hâlâ 1.0.0).
        let metin = manifest_metni_oku(&kok).unwrap();
        assert_eq!(
            super::super::manifest_surumu_oku(&metin).unwrap(),
            v(1, 0, 0)
        );
        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn basarisiz_goc_yedekten_geri_doner() {
        // BOZUK göç → strict doğrulama başarısız → yedekten geri yükleme → proje bozulmaz.
        let (konum, kok) = ornek_proje("rollback");
        let kayit = GocKayit::yeni().ekle(Goc {
            kaynak: v(1, 0, 0),
            hedef: v(1, 1, 0),
            aciklama: "bozuk",
            kirici: false,
            donustur: goc_bozuk,
        });
        let hata = ac_ve_goc_ile(&kok, &kayit, &v(1, 1, 0), OnayPolitikasi::Otomatik).unwrap_err();
        assert!(hata.ne_oldu.contains("geri yüklendi"));

        // Proje hâlâ açılabilir VE hâlâ 1.0.0 (göç uygulanmadı).
        let acilan = crate::project::ac(&kok).unwrap();
        assert_eq!(acilan.manifest.kimlik.format_surumu, v(1, 0, 0));
        assert_eq!(acilan.manifest.goc.len(), 1); // yalnızca ilk oluşturma
        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn daha_yeni_proje_salt_okunur_acilir() {
        // Projeyi daha yeni bir format sürümüne "ileri" göç ettir, sonra eski uygulamayla aç.
        let (konum, kok) = ornek_proje("ileri_uyum");
        let kayit = GocKayit::yeni().ekle(Goc {
            kaynak: v(1, 0, 0),
            hedef: v(2, 0, 0),
            aciklama: "2.0",
            kirici: false,
            donustur: goc_1_1_to_2_0,
        });
        ac_ve_goc_ile(&kok, &kayit, &v(2, 0, 0), OnayPolitikasi::Otomatik).unwrap();

        // Şimdi ESKİ uygulama (hedef 1.0.0) ile aç → salt-okunur + uyarı, bozma yok.
        let sonuc = ac_ve_goc_ile(
            &kok,
            &GocKayit::yeni(),
            &v(1, 0, 0),
            OnayPolitikasi::Otomatik,
        )
        .unwrap();
        let GocSonucu::SaltOkunur { proje, uyari } = sonuc else {
            panic!("salt-okunur bekleniyordu");
        };
        assert_eq!(proje.dosya_surumu, v(2, 0, 0));
        assert_eq!(proje.uygulama_surumu, v(1, 0, 0));
        assert!(uyari.ne_oldu.contains("daha yeni"));
        // Disk dokunulmadı (hâlâ 2.0.0).
        let metin = manifest_metni_oku(&kok).unwrap();
        assert_eq!(
            super::super::manifest_surumu_oku(&metin).unwrap(),
            v(2, 0, 0)
        );
        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn desteklenmeyen_surum_net_hata() {
        let (konum, kok) = ornek_proje("desteklenmiyor");
        // Kayıt yalnızca 1.0→1.1; hedef 3.0.0 → yol yok → net hata.
        let hata =
            ac_ve_goc_ile(&kok, &kayit_1_1(), &v(3, 0, 0), OnayPolitikasi::Otomatik).unwrap_err();
        assert!(hata.ne_oldu.contains("yükseltilemiyor"));
        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn goc_plani_adimlari_dogru_bildirir() {
        let (konum, kok) = ornek_proje("plan");
        let plan = goc_plani_ile(&kok, &kayit_1_1(), &v(1, 1, 0))
            .unwrap()
            .unwrap();
        assert_eq!(plan.adimlar.len(), 1);
        assert_eq!(plan.adimlar[0].kaynak, v(1, 0, 0));
        assert_eq!(plan.adimlar[0].hedef, v(1, 1, 0));
        assert!(!plan.kirici_var);
        // Plan uygulamaz → disk hâlâ 1.0.0.
        let metin = manifest_metni_oku(&kok).unwrap();
        assert_eq!(
            super::super::manifest_surumu_oku(&metin).unwrap(),
            v(1, 0, 0)
        );
        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn guncel_proje_plani_yok() {
        let (konum, kok) = ornek_proje("plan_yok");
        assert!(goc_plani_ile(&kok, &kayit_1_1(), &v(1, 0, 0))
            .unwrap()
            .is_none());
        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn uretim_kaydi_bos_ve_proje_guncel() {
        // Üretim: format 1.0.0, kayıt boş → her gerçek proje "Güncel".
        let (konum, kok) = ornek_proje("uretim");
        assert!(proje_format_kayit().is_empty());
        let sonuc = ac_ve_goc(&kok, OnayPolitikasi::Otomatik).unwrap();
        assert!(matches!(sonuc, GocSonucu::Guncel(_)));
        let _ = fs::remove_dir_all(&konum);
    }
}
