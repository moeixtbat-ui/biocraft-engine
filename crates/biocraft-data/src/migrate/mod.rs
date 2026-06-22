//! Göç ve sürüm uyumu çerçevesi — eski proje/ayar/eklenti dosyalarını yeni sürümlere taşır (MK-59, MK-14, İP-19).
//!
//! Bu modül **çerçevedir**: sürümlü şema okuma + deterministik göç adımı zinciri + göç öncesi
//! yedek + atomik (yarıda kalırsa geri dön) uygulama + ileri-uyumluluk (daha yeni format →
//! salt-okunur) + eklenti veri/ABI uyumu.  **Gerçek göç kuralları** sürümler ilerledikçe
//! [`project::proje_format_kayit`](super::migrate::project::proje_format_kayit) içine eklenir;
//! ilk sürümde (format 1.0.0) kayıt boştur ama altyapı hazırdır (spec varsayımı, İP-19).
//!
//! ## Tasarım ilkeleri
//! - **Deterministik:** her göç adımı saf bir `fn(&mut toml::Value)` dönüşümüdür (yakalanan durum
//!   yok) → aynı girdi her zaman aynı çıktıyı verir; golden testle doğrulanabilir (MK-58).
//! - **Toleranslı sürüm okuma:** sürüm, manifesti **tam ayrıştırmadan** (`toml::Value`) okunur →
//!   daha yeni formattaki bilinmeyen alanlar açılışı bozmaz (ileri uyumluluk).
//! - **Güvenli:** göç **öncesi** çekirdek dosyaların yedeği alınır; uygulanan göç strict
//!   [`Manifest`](super::Manifest) doğrulamasından geçer; başarısızsa **yedekten geri yüklenir**
//!   (proje hiç bozulmaz) ve net hata döner.
//! - **Bozma yok:** daha yeni sürümle yapılmış proje **salt-okunur** açılır + "daha yeni BioCraft
//!   gerekiyor" uyarısı; üzerine yazılmaz.

pub mod project;

use serde::{Deserialize, Serialize};

use biocraft_types::{ErrorReport, Version};

pub use project::{
    ac_ve_goc, ac_ve_goc_ile, goc_plani, proje_format_kayit, GocAdimOzeti, GocPlani, GocSonucu,
    OnayPolitikasi, SaltOkunurProje, Yedek,
};

// ─── Sürüm durumu ──────────────────────────────────────────────────────────────

/// Bir dosyanın sürümünün, çalışan uygulamanın hedef sürümüne göre durumu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurumDurumu {
    /// Dosya zaten güncel hedef sürümde — göç gerekmez.
    Guncel,
    /// Dosya **daha eski** — göç gerekir (`mevcut` → `hedef`).
    GocGerekli {
        /// Dosyanın mevcut format sürümü.
        mevcut: Version,
        /// Uygulamanın hedeflediği güncel format sürümü.
        hedef: Version,
    },
    /// Dosya **daha yeni** bir sürümle yapılmış → salt-okunur + uyarı (bozma yok).
    DahaYeni {
        /// Dosyanın (daha yeni) format sürümü.
        dosya: Version,
        /// Bu uygulamanın desteklediği format sürümü.
        uygulama: Version,
    },
}

/// Bir dosya sürümünü hedef sürümle karşılaştırıp [`SurumDurumu`] üretir (saf/deterministik).
///
/// SemVer sıralaması ([`Version`] `Ord` türetir) kullanılır:
/// - eşit → [`SurumDurumu::Guncel`],
/// - dosya < hedef → [`SurumDurumu::GocGerekli`],
/// - dosya > hedef → [`SurumDurumu::DahaYeni`] (kesinlikle yeni olan **her** dosya salt-okunur
///   açılır; aksi halde kaydetme yeni alanları düşürüp **veri kaybına** yol açardı).
pub fn degerlendir(dosya: &Version, hedef: &Version) -> SurumDurumu {
    use std::cmp::Ordering::*;
    match dosya.cmp(hedef) {
        Equal => SurumDurumu::Guncel,
        Less => SurumDurumu::GocGerekli {
            mevcut: dosya.clone(),
            hedef: hedef.clone(),
        },
        Greater => SurumDurumu::DahaYeni {
            dosya: dosya.clone(),
            uygulama: hedef.clone(),
        },
    }
}

// ─── Göç adımı + kayıt ──────────────────────────────────────────────────────────

/// Tek bir deterministik göç adımının dönüşüm fonksiyonu.
///
/// Manifesti **toleranslı** `toml::Value` biçiminde alır ve **yalnızca şema/veri** değişikliğini
/// uygular (alan ekle/yeniden adlandır/dönüştür).  Sürüm alanı + göç geçmişi + bütünlük mührü
/// **çerçeve** tarafından yönetilir; dönüşüm bunlara dokunmaz.  Saf `fn` işaretçisidir (yakalanan
/// durum yok) → determinizm garanti.
pub type GocDonusum = fn(&mut toml::Value) -> Result<(), ErrorReport>;

/// Bir format sürümünden bir sonrakine **deterministik** göç adımı.
#[derive(Clone)]
pub struct Goc {
    /// Bu adımın uygulanabileceği kaynak format sürümü.
    pub kaynak: Version,
    /// Bu adımın ürettiği hedef format sürümü (`kaynak`'tan kesinlikle büyük olmalı).
    pub hedef: Version,
    /// İnsan-okunur açıklama (göç geçmişine yazılır).
    pub aciklama: &'static str,
    /// Kırıcı (geriye dönük uyumsuz) bir değişiklik mi? Kullanıcı onayı + uyarı tetikler.
    pub kirici: bool,
    /// Deterministik dönüşüm.
    pub donustur: GocDonusum,
}

impl std::fmt::Debug for Goc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Goc")
            .field("kaynak", &self.kaynak)
            .field("hedef", &self.hedef)
            .field("aciklama", &self.aciklama)
            .field("kirici", &self.kirici)
            .finish()
    }
}

/// Bir format ailesi için sıralı göç adımları kaydı (vN → vN+1 → …).
#[derive(Debug, Clone, Default)]
pub struct GocKayit {
    adimlar: Vec<Goc>,
}

impl GocKayit {
    /// Boş bir kayıt.
    pub fn yeni() -> Self {
        Self {
            adimlar: Vec::new(),
        }
    }

    /// Bir göç adımı ekler (zincir sırası `coz` tarafından çözülür).
    pub fn ekle(mut self, goc: Goc) -> Self {
        self.adimlar.push(goc);
        self
    }

    /// Kayıtlı adım sayısı.
    pub fn len(&self) -> usize {
        self.adimlar.len()
    }

    /// Kayıt boş mu?
    pub fn is_empty(&self) -> bool {
        self.adimlar.is_empty()
    }

    /// `mevcut` sürümden `hedef` sürüme götüren **deterministik** adım zincirini çözer.
    ///
    /// - `mevcut == hedef` → boş zincir (göç gerekmez).
    /// - Her sürümde tam olarak bir ileri adım izlenir; adım `hedef`'i aşarsa veya hiç adım
    ///   bulunamazsa → [`ZincirHatasi`] (göç yolu yok → çağıran "desteklenmiyor" raporlar).
    /// - Sonsuz döngü/geri-adım koruması: her adım sürümü kesinlikle ilerletmelidir.
    pub fn coz(&self, mevcut: &Version, hedef: &Version) -> Result<Vec<&Goc>, ZincirHatasi> {
        if mevcut == hedef {
            return Ok(Vec::new());
        }
        if mevcut > hedef {
            // Daha yeni dosya buraya gelmemeli (çağıran `degerlendir` ile ayırır); savunma.
            return Err(ZincirHatasi::HedefGeride);
        }
        let mut zincir: Vec<&Goc> = Vec::new();
        let mut su_an = mevcut.clone();
        // Kayıt sonlu; adım sayısı kadar yineleme yeterli (her adım benzersiz kaynak ilerletir).
        for _ in 0..=self.adimlar.len() {
            if &su_an == hedef {
                return Ok(zincir);
            }
            let Some(adim) = self.adimlar.iter().find(|g| g.kaynak == su_an) else {
                return Err(ZincirHatasi::YolYok {
                    takili: su_an,
                    hedef: hedef.clone(),
                });
            };
            if adim.hedef <= su_an {
                return Err(ZincirHatasi::GeriyeAdim);
            }
            if &adim.hedef > hedef {
                return Err(ZincirHatasi::HedefiAsti);
            }
            su_an = adim.hedef.clone();
            zincir.push(adim);
        }
        Err(ZincirHatasi::DonguSuphesi)
    }
}

/// Göç zinciri çözülürken oluşabilecek tutarsızlıklar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZincirHatasi {
    /// `mevcut`'tan ileri giden adım yok → desteklenmeyen/çok eski format.
    YolYok {
        /// Zincirin takıldığı sürüm.
        takili: Version,
        /// Ulaşılmak istenen hedef.
        hedef: Version,
    },
    /// Bir adım sürümü ilerletmiyor (kaynak ≥ hedef) — bozuk kayıt.
    GeriyeAdim,
    /// Zincir hedefi aştı (kayıtta atlama/çakışma var).
    HedefiAsti,
    /// Hedef, mevcuttan geride (bu fonksiyona yanlış yönde girildi).
    HedefGeride,
    /// Beklenenden çok yineleme — döngü şüphesi (savunma).
    DonguSuphesi,
}

// ─── Manifest sürüm probu (toleranslı) ───────────────────────────────────────────

/// Manifest TOML metninden **yalnızca** format sürümünü (`kimlik.format_surumu`) okur.
///
/// Manifesti **tam ayrıştırmaz** (`toml::Value`) → daha yeni formatın eklediği bilinmeyen alanlar
/// bu okumayı bozmaz (ileri uyumluluk).  Sürüm alanı yoksa/biçimsizse net hata.
pub fn manifest_surumu_oku(toml_metin: &str) -> Result<Version, ErrorReport> {
    let deger: toml::Value = toml::from_str(toml_metin).map_err(|e| {
        ErrorReport::new(
            "Proje sürümü okunamadı",
            "biocraft.toml geçerli bir TOML dosyası değil.",
            "Dosyayı yedekten geri yükleyin (sessiz açma yapılmaz).",
        )
        .with_teknik_detay(format!("toml value de: {e}"))
    })?;
    deger
        .get("kimlik")
        .and_then(|k| k.get("format_surumu"))
        .and_then(surum_degeri_oku)
        .ok_or_else(|| {
            ErrorReport::new(
                "Proje sürümü bulunamadı",
                "biocraft.toml içinde format sürümü alanı (kimlik.format_surumu) eksik veya biçimsiz.",
                "Bu dosya bir BioCraft projesi olmayabilir ya da çok eski/bozuk olabilir.",
            )
        })
}

/// Bir `toml::Value` alt-tablosundan `Version` (major/minor/patch) okur (toleranslı).
fn surum_degeri_oku(v: &toml::Value) -> Option<Version> {
    let t = v.as_table()?;
    let alan = |ad: &str| -> Option<u32> { u32::try_from(t.get(ad)?.as_integer()?).ok() };
    Some(Version::new(alan("major")?, alan("minor")?, alan("patch")?))
}

// ─── Eklenti veri/ABI uyumu (MK-14) ──────────────────────────────────────────────

/// Bir eklentinin sakladığı veri sürümünün, uygulamanın beklediği ABI sürümüne göre uyumu (MK-14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EklentiVeriUyumu {
    /// Birebir aynı sürüm — uyum tam.
    Uyumlu,
    /// Aynı ana (major) sürüm, farklı alt/yama → veri göçü gerekebilir ama ABI uyumlu (MK-14).
    GocGerekli {
        /// Eklenti verisinin mevcut sürümü.
        mevcut: Version,
        /// Uygulamanın beklediği ABI sürümü.
        beklenen: Version,
    },
    /// Farklı ana (major) sürüm → ABI **kırıcı**; eklenti bu sürümle yüklenemez (net bilgilendirme).
    Uyumsuz {
        /// Eklenti verisinin mevcut sürümü.
        mevcut: Version,
        /// Uygulamanın beklediği ABI sürümü.
        beklenen: Version,
    },
}

/// Eklenti veri/ABI uyumunu belirler (MK-14: kırıcı = ana sürüm farkı).
///
/// [`Version::uyumlu_mu`] (aynı `major`) temel kuraldır: ana sürüm farkı ABI kırıcıdır →
/// [`EklentiVeriUyumu::Uyumsuz`].  Aynı ana sürümde alt/yama farkı varsa, veri göçü gerekebilir
/// ama eklenti **çalışır** ([`EklentiVeriUyumu::GocGerekli`]).
pub fn eklenti_veri_uyumu(mevcut_abi: &Version, beklenen_abi: &Version) -> EklentiVeriUyumu {
    if mevcut_abi == beklenen_abi {
        EklentiVeriUyumu::Uyumlu
    } else if !mevcut_abi.uyumlu_mu(beklenen_abi) {
        EklentiVeriUyumu::Uyumsuz {
            mevcut: mevcut_abi.clone(),
            beklenen: beklenen_abi.clone(),
        }
    } else {
        EklentiVeriUyumu::GocGerekli {
            mevcut: mevcut_abi.clone(),
            beklenen: beklenen_abi.clone(),
        }
    }
}

/// Uyumsuz bir eklenti için kullanıcıya gösterilecek net hata (MK-14 — sessiz başarısızlık yok).
pub fn uyumsuz_eklenti_hatasi(
    eklenti_kimlik: &str,
    mevcut: &Version,
    beklenen: &Version,
) -> ErrorReport {
    ErrorReport::new(
        "Eklenti bu BioCraft sürümüyle uyumlu değil",
        format!(
            "'{eklenti_kimlik}' eklentisi ABI sürüm {mevcut} ile yazılmış; bu uygulama \
             {beklenen} (ana sürüm farkı = kırıcı değişiklik) bekliyor."
        ),
        "Eklentinin bu BioCraft sürümüne uygun bir güncellemesini kurun ya da uyumlu bir BioCraft \
         sürümü kullanın.",
    )
    .with_eylem("Eklentiyi güncelle")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: u32, b: u32, c: u32) -> Version {
        Version::new(a, b, c)
    }

    #[test]
    fn degerlendir_uc_durum() {
        assert_eq!(degerlendir(&v(1, 0, 0), &v(1, 0, 0)), SurumDurumu::Guncel);
        assert!(matches!(
            degerlendir(&v(1, 0, 0), &v(1, 1, 0)),
            SurumDurumu::GocGerekli { .. }
        ));
        assert!(matches!(
            degerlendir(&v(2, 0, 0), &v(1, 0, 0)),
            SurumDurumu::DahaYeni { .. }
        ));
    }

    fn boş_donustur(_d: &mut toml::Value) -> Result<(), ErrorReport> {
        Ok(())
    }

    fn ornek_goc(kaynak: Version, hedef: Version, kirici: bool) -> Goc {
        Goc {
            kaynak,
            hedef,
            aciklama: "test",
            kirici,
            donustur: boş_donustur,
        }
    }

    #[test]
    fn coz_eşit_boş_zincir() {
        let kayit = GocKayit::yeni();
        assert_eq!(kayit.coz(&v(1, 0, 0), &v(1, 0, 0)).unwrap().len(), 0);
    }

    #[test]
    fn coz_zinciri_takip_eder() {
        let kayit = GocKayit::yeni()
            .ekle(ornek_goc(v(1, 0, 0), v(1, 1, 0), false))
            .ekle(ornek_goc(v(1, 1, 0), v(2, 0, 0), true));
        let zincir = kayit.coz(&v(1, 0, 0), &v(2, 0, 0)).unwrap();
        assert_eq!(zincir.len(), 2);
        assert_eq!(zincir[0].hedef, v(1, 1, 0));
        assert_eq!(zincir[1].hedef, v(2, 0, 0));
        assert!(zincir[1].kirici);
    }

    #[test]
    fn coz_yol_yoksa_hata() {
        let kayit = GocKayit::yeni().ekle(ornek_goc(v(1, 0, 0), v(1, 1, 0), false));
        // 1.1.0'dan 3.0.0'a adım yok → desteklenmiyor.
        let hata = kayit.coz(&v(1, 0, 0), &v(3, 0, 0)).unwrap_err();
        assert!(matches!(hata, ZincirHatasi::YolYok { .. }));
    }

    #[test]
    fn surum_probu_toleranslidir() {
        // Bilinmeyen yeni alanlar olmasına rağmen sürüm okunmalı (ileri uyumluluk).
        let metin = r#"
[kimlik]
ad = "X"
format_surumu = { major = 2, minor = 3, patch = 1 }
gelecekteki_yeni_alan = "tanınmıyor ama bozmamalı"

[gelecek_bolum]
deger = 42
"#;
        assert_eq!(manifest_surumu_oku(metin).unwrap(), v(2, 3, 1));
    }

    #[test]
    fn surum_probu_alan_yoksa_hata() {
        let metin = "[kimlik]\nad = \"X\"\n";
        assert!(manifest_surumu_oku(metin).is_err());
    }

    #[test]
    fn eklenti_uyumu_kurallari() {
        assert_eq!(
            eklenti_veri_uyumu(&v(1, 0, 0), &v(1, 0, 0)),
            EklentiVeriUyumu::Uyumlu
        );
        assert!(matches!(
            eklenti_veri_uyumu(&v(1, 0, 0), &v(1, 2, 0)),
            EklentiVeriUyumu::GocGerekli { .. }
        ));
        assert!(matches!(
            eklenti_veri_uyumu(&v(1, 5, 0), &v(2, 0, 0)),
            EklentiVeriUyumu::Uyumsuz { .. }
        ));
    }
}
