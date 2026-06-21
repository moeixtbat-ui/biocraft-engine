//! Taşınabilir, zengin **proje formatı** — kurulum, açma (bütünlük denetimi), dışa aktarım (MK-31/33/34/59).
//!
//! İP-02 (2. kısım): Proje sihirbazının topladığı taslak burada **gerçek diske** dönüşür:
//! klasör yapısı + `biocraft.toml` manifest + `.biocraft_meta` (meta + BLAKE3 mührü) + provenance.
//! Açılışta bütünlük doğrulanır (bozuk/eksik dosya **net bildirilir**); proje tek dosya `.bcproj`
//! olarak dışa aktarılabilir (hassas ayar varsayılan hariç).
//!
//! **Katman köprüsü (MK-40):** `biocraft-data` (L1/L2) UI'ye (L4) bağlanamaz; sihirbaz taslağını
//! buraya **app (L5)** [`ProjeKurulumGirdisi`] olarak köprüler.

pub mod export;
pub mod format;
pub mod integrity;
pub mod manifest;
pub mod provenance;

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use biocraft_types::{DataClassification, ErrorReport, Version};

pub use export::{DisaAktarRaporu, DisaAktarSecenekleri, BCPROJ_UZANTI};
pub use manifest::{BuyukVeriStratejisi, Determinizm, Manifest, VeriYerlesimi};
pub use provenance::{Meta, ProvenansOlay};

// ─── Girdi: app (L5) sihirbaz taslağından doldurur ────────────────────────────

/// Proje kurulumu için toplanan tüm alanlar.  App (L5), sihirbazın `ProjeTaslagi`'ndan doldurur.
///
/// `yeni` ile zorunlu alanlar + güvenli varsayılanlar kurulur; geri kalan alanlar doğrudan
/// atanabilir (hepsi `pub`).
#[derive(Debug, Clone)]
pub struct ProjeKurulumGirdisi {
    /// Proje adı (doğrulanmış dosya adı).
    pub ad: String,
    /// **Üst** klasör; proje kökü `konum/ad` olur.
    pub konum: PathBuf,
    /// Açıklama.
    pub aciklama: String,
    /// Kurum.
    pub kurum: String,
    /// Kullanıcı etiketleri.
    pub etiketler: Vec<String>,
    /// Doğrulanmış ORCID (yoksa `None`).
    pub orcid: Option<String>,
    /// Şablon anahtarı (kararlı dizge: `genomik`/`proteomik`/`crispr`/`bos`).
    pub sablon_anahtari: String,
    /// Projeyi oluşturan BioCraft sürümü.
    pub biocraft_surumu: Version,
    /// Zorunlu veri sınıflandırması (MK-42).
    pub siniflandirma: DataClassification,
    /// Veri yerleşimi.
    pub veri_yerlesim: VeriYerlesimi,
    /// Büyük veri stratejisi.
    pub buyuk_veri: BuyukVeriStratejisi,
    /// Akış modu.
    pub akis_modu: bool,
    /// Tamamen yerel.
    pub tamamen_yerel: bool,
    /// AI havuzuna katkı.
    pub ai_havuzu_katki: bool,
    /// Yerel şifreleme.
    pub sifreleme: bool,
    /// Dağıtık ağ etkin.
    pub dagitik_ag_etkin: bool,
    /// Determinizm bayrağı.
    pub determinizm: Determinizm,
    /// Lisans.
    pub lisans: String,
    /// Uyumluluk etiketleri.
    pub uyumluluk: Vec<String>,
}

impl ProjeKurulumGirdisi {
    /// Zorunlu alanlar + spec varsayılanlarıyla yeni bir girdi kurar.
    pub fn yeni(
        ad: impl Into<String>,
        konum: impl Into<PathBuf>,
        sablon_anahtari: impl Into<String>,
        siniflandirma: DataClassification,
        biocraft_surumu: Version,
    ) -> Self {
        Self {
            ad: ad.into(),
            konum: konum.into(),
            aciklama: String::new(),
            kurum: String::new(),
            etiketler: Vec::new(),
            orcid: None,
            sablon_anahtari: sablon_anahtari.into(),
            biocraft_surumu,
            siniflandirma,
            veri_yerlesim: VeriYerlesimi::Yerel,
            buyuk_veri: BuyukVeriStratejisi::Referans,
            akis_modu: false,
            tamamen_yerel: true,
            ai_havuzu_katki: false,
            sifreleme: true,
            dagitik_ag_etkin: false,
            determinizm: Determinizm::HizliKesif,
            lisans: String::new(),
            uyumluluk: Vec::new(),
        }
    }

    /// Bu girdinin oluşturacağı proje kök klasörü (`konum/ad`).
    pub fn proje_kok(&self) -> PathBuf {
        self.konum.join(&self.ad)
    }
}

// ─── Sonuç tipleri ────────────────────────────────────────────────────────────

/// Başarılı bir kurulumun sonucu.
#[derive(Debug, Clone)]
pub struct KurulanProje {
    /// Proje kök klasörü.
    pub kok: PathBuf,
    /// Yazılan manifest.
    pub manifest: Manifest,
}

/// Açılan bir projenin (bütünlüğü doğrulanmış) durumu.
#[derive(Debug, Clone)]
pub struct AcilanProje {
    /// Proje kök klasörü.
    pub kok: PathBuf,
    /// Manifest (bütünlüğü doğrulanmış).
    pub manifest: Manifest,
    /// Meta.
    pub meta: Meta,
    /// Harici veri referanslarında saptanan **eksik/bozuk** sorunlar (açılışı engellemez ama
    /// net bildirilir — sessiz açma yok).
    pub uyarilar: Vec<ErrorReport>,
}

// ─── Bütünlük mührü kaydı ─────────────────────────────────────────────────────

/// `.biocraft_meta/butunluk.bcp` içeriği: manifest + meta dosyalarının beklenen BLAKE3 özetleri.
/// Kendisi BCP1 zarfıyla korunur (kurcalanması açılışta yakalanır).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ButunlukKaydi {
    /// `biocraft.toml`'un beklenen BLAKE3 özeti (hex).
    manifest_blake3: String,
    /// `.biocraft_meta/meta.toml`'un beklenen BLAKE3 özeti (hex).
    meta_blake3: String,
}

impl ButunlukKaydi {
    fn hesapla(manifest_bayt: &[u8], meta_bayt: &[u8]) -> Self {
        Self {
            manifest_blake3: integrity::icerik_ozeti(manifest_bayt).to_hex(),
            meta_blake3: integrity::icerik_ozeti(meta_bayt).to_hex(),
        }
    }
}

// ─── Kurulum ──────────────────────────────────────────────────────────────────

/// Bir projeyi diske kurar: klasör + manifest + meta + bütünlük mührü + ilk provenance olayı.
///
/// **İptal/hata güvenliği (madde 12):** Herhangi bir adım başarısız olursa, **bu çağrının
/// oluşturduğu** proje klasörü tümüyle silinir (atomik temizlik) — yarım kalıntı bırakılmaz.
/// Hedef klasör zaten dolu bir projeyse veya boş değilse, üzerine yazılmaz (net hata).
pub fn olustur(girdi: &ProjeKurulumGirdisi) -> Result<KurulanProje, ErrorReport> {
    let kok = girdi.proje_kok();

    // Üzerine yazmama: hedef varsa ve boş değilse reddet (kullanıcı verisini ezmeyiz).
    let onceden_vardi = kok.exists();
    if onceden_vardi && !klasor_bos(&kok) {
        return Err(zaten_var_hatasi(&kok));
    }

    let sonuc = olustur_ic(girdi, &kok);

    if sonuc.is_err() {
        // Atomik temizlik: klasör ya bizim oluşturduğumuzdu ya da boştu → güvenle silinir.
        let _ = fs::remove_dir_all(&kok);
    }
    sonuc
}

/// Kurulumun iç adımları (hata olursa çağıran temizler).
fn olustur_ic(girdi: &ProjeKurulumGirdisi, kok: &Path) -> Result<KurulanProje, ErrorReport> {
    format::iskele_olustur(kok)?;

    let simdi = Utc::now();
    let manifest = girdi_manifest(girdi, simdi);

    // 1) Manifest.
    let manifest_metin = manifest.toml_metni()?;
    integrity::atomik_yaz(&format::manifest_yolu(kok), manifest_metin.as_bytes())?;

    // 2) Meta.
    let meta = Meta::yeni(simdi, manifest.goc.len());
    let meta_metin = meta.toml_metni()?;
    integrity::atomik_yaz(&format::meta_yolu(kok), meta_metin.as_bytes())?;

    // 3) Bütünlük mührü (manifest + meta özetleri, BCP1 zarflı).
    butunluk_muhru_yaz(kok, manifest_metin.as_bytes(), meta_metin.as_bytes())?;

    // 4) İlk provenance olayı.
    provenance::olay_ekle(
        kok,
        &ProvenansOlay::yeni(
            provenance::olay_turu::PROJE_OLUSTURULDU,
            "Proje oluşturuldu",
            girdi.biocraft_surumu.clone(),
        ),
    )?;

    Ok(KurulanProje {
        kok: kok.to_path_buf(),
        manifest,
    })
}

/// Bütünlük mührünü hesaplayıp diske (BCP1 zarflı) yazar.
fn butunluk_muhru_yaz(
    kok: &Path,
    manifest_bayt: &[u8],
    meta_bayt: &[u8],
) -> Result<(), ErrorReport> {
    let kayit = ButunlukKaydi::hesapla(manifest_bayt, meta_bayt);
    let toml_metin = toml::to_string_pretty(&kayit).map_err(|e| {
        ErrorReport::new(
            "Bütünlük mührü yazılamadı",
            "Mühür TOML'a dönüştürülürken sorun oluştu.",
            "Bu bir iç hatadır; lütfen bildirin.",
        )
        .with_teknik_detay(format!("toml ser: {e}"))
    })?;
    let sarili = integrity::zarf_sar(toml_metin.as_bytes());
    integrity::atomik_yaz(&format::butunluk_yolu(kok), &sarili)
}

/// Girdiyi bir [`Manifest`]'e dönüştürür (göç geçmişini baştan koyar — MK-59).
fn girdi_manifest(g: &ProjeKurulumGirdisi, simdi: biocraft_types::Timestamp) -> Manifest {
    Manifest {
        kimlik: manifest::Kimlik {
            ad: g.ad.clone(),
            aciklama: g.aciklama.clone(),
            biocraft_surumu: g.biocraft_surumu.clone(),
            format_surumu: format::format_surumu(),
            sablon: g.sablon_anahtari.clone(),
            olusturma: simdi,
            degistirme: simdi,
        },
        olusturan: manifest::Olusturan {
            orcid: g.orcid.clone(),
            kurum: g.kurum.clone(),
        },
        siniflandirma: manifest::Siniflandirma {
            sinif: g.siniflandirma,
            uyumluluk: g.uyumluluk.clone(),
            lisans: g.lisans.clone(),
            etiketler: g.etiketler.clone(),
        },
        gizlilik: manifest::Gizlilik {
            tamamen_yerel: g.tamamen_yerel,
            ai_havuzu_katki: g.ai_havuzu_katki,
            dagitik_ag_etkin: g.dagitik_ag_etkin,
            determinizm: g.determinizm,
        },
        guvenlik: Some(manifest::Guvenlik {
            sifreleme: g.sifreleme,
        }),
        veri: manifest::Veri {
            yerlesim: g.veri_yerlesim,
            buyuk_veri: g.buyuk_veri,
            akis_modu: g.akis_modu,
        },
        harici_veri: Vec::new(),
        goc: Manifest::ilk_goc_gecmisi(simdi),
    }
}

// ─── Açma + bütünlük denetimi ─────────────────────────────────────────────────

/// Bir projeyi açar ve **bütünlüğünü doğrular** (MK-33).
///
/// Sıra: mühür zarfı → manifest özeti → meta özeti → ayrıştırma → harici referans denetimi.
/// Çekirdek dosya (mühür/manifest/meta) bozuksa **`Err`** (net hata).  Harici veri eksik/bozuksa
/// proje yine açılır ama sorunlar `uyarilar` içinde **net** döner (sessiz açma yok).
pub fn ac(kok: &Path) -> Result<AcilanProje, ErrorReport> {
    // 1) Mühür dosyası + zarf bütünlüğü.
    let muhur_yol = format::butunluk_yolu(kok);
    let muhur_ham = match fs::read(&muhur_yol) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(proje_degil_hatasi(kok));
        }
        Err(e) => {
            return Err(integrity::io_hatasi(
                "Bütünlük mührü okunamadı",
                &muhur_yol,
                &e,
            ))
        }
    };
    let muhur_yuk = integrity::zarf_coz(&muhur_ham, &muhur_yol)?;
    let kayit: ButunlukKaydi = toml::from_str(&metne_cevir(&muhur_yuk, &muhur_yol)?)
        .map_err(|e| bozuk_toml_hatasi("Bütünlük mührü", &muhur_yol, &e.to_string()))?;

    // 2) Manifest özeti.
    let manifest_yol = format::manifest_yolu(kok);
    let manifest_bayt = fs::read(&manifest_yol)
        .map_err(|e| integrity::io_hatasi("Manifest okunamadı", &manifest_yol, &e))?;
    if integrity::icerik_ozeti(&manifest_bayt).to_hex() != kayit.manifest_blake3 {
        return Err(integrity::uyusmazlik_hatasi(
            "Manifest (biocraft.toml)",
            &manifest_yol,
        ));
    }

    // 3) Meta özeti.
    let meta_yol = format::meta_yolu(kok);
    let meta_bayt =
        fs::read(&meta_yol).map_err(|e| integrity::io_hatasi("Meta okunamadı", &meta_yol, &e))?;
    if integrity::icerik_ozeti(&meta_bayt).to_hex() != kayit.meta_blake3 {
        return Err(integrity::uyusmazlik_hatasi(
            "Meta (.biocraft_meta/meta.toml)",
            &meta_yol,
        ));
    }

    // 4) Ayrıştırma.
    let manifest = Manifest::toml_coz(&metne_cevir(&manifest_bayt, &manifest_yol)?)?;
    let meta = Meta::toml_coz(&metne_cevir(&meta_bayt, &meta_yol)?)?;

    // 5) Harici büyük veri referanslarını denetle.
    let mut uyarilar = Vec::new();
    for ref_ in &manifest.harici_veri {
        let Some(ipucu) = &ref_.gercek_yol_ipucu else {
            continue; // ipucu yok (ör. export'tan gelmiş) — yerelde doğrulanamaz.
        };
        let p = Path::new(ipucu);
        if !p.exists() {
            uyarilar.push(harici_eksik_hatasi(&ref_.mantiksal_yol, ipucu));
        } else if let Ok(ozet) = integrity::dosya_ozeti(p) {
            if ozet.to_hex() != ref_.blake3 {
                uyarilar.push(harici_bozuk_hatasi(&ref_.mantiksal_yol, ipucu));
            }
        }
    }

    Ok(AcilanProje {
        kok: kok.to_path_buf(),
        manifest,
        meta,
        uyarilar,
    })
}

// ─── Dışa aktarım (.bcproj) ───────────────────────────────────────────────────

/// Bir projeyi tek dosya `.bcproj` (ZIP stored) olarak dışa aktarır (MK-31, madde 7).
///
/// Önce kaynak proje **doğrulanır** (bozuksa export edilmez).  Manifest export filtresinden geçer
/// (hassas `[guvenlik]` + gerçek yol ipuçları varsayılan çıkarılır); küçük dosyalar gömülür, büyük
/// veri referansla kalır.  Pakete taze bir bütünlük mührü yazılır → açıldığında doğrulanabilir.
pub fn disa_aktar(
    kok: &Path,
    hedef_bcproj: &Path,
    secenek: &DisaAktarSecenekleri,
) -> Result<DisaAktarRaporu, ErrorReport> {
    let acilan = ac(kok)?; // bozuk projeyi paketleme.

    let filtreli = acilan
        .manifest
        .disa_aktarim_icin_filtrele(secenek.hassas_dahil);
    let manifest_metin = filtreli.toml_metni()?;
    let meta_metin = acilan.meta.toml_metni()?;

    // Filtreli manifest + meta için taze mühür.
    let kayit = ButunlukKaydi::hesapla(manifest_metin.as_bytes(), meta_metin.as_bytes());
    let muhur_toml = toml::to_string_pretty(&kayit).map_err(|e| {
        ErrorReport::new(
            "Paket mührü yazılamadı",
            "Mühür TOML'a dönüştürülürken sorun oluştu.",
            "Bu bir iç hatadır; lütfen bildirin.",
        )
        .with_teknik_detay(format!("toml ser: {e}"))
    })?;
    let muhur = integrity::zarf_sar(muhur_toml.as_bytes());

    // Küçük kullanıcı dosyaları (büyük olanlar referansla kalır — atlanır).
    let mut girisler = export::kucuk_dosyalari_topla(kok, secenek.gomme_esigi_bayt)?;

    // Çekirdek format dosyaları (filtreli).
    girisler.push(export::Giris {
        yol: format::MANIFEST_DOSYA.to_string(),
        veri: manifest_metin.into_bytes(),
    });
    girisler.push(export::Giris {
        yol: format!("{}/{}", format::META_DIZIN, format::META_DOSYA),
        veri: meta_metin.into_bytes(),
    });
    girisler.push(export::Giris {
        yol: format!("{}/{}", format::META_DIZIN, format::BUTUNLUK_DOSYA),
        veri: muhur,
    });

    let zip = export::zip_stored(&girisler);
    export::dosyaya_yaz(hedef_bcproj, &zip)?;

    Ok(DisaAktarRaporu {
        dosya_sayisi: girisler.len(),
        hassas_haric: !secenek.hassas_dahil,
        boyut_bayt: zip.len() as u64,
    })
}

// ─── Yardımcılar ──────────────────────────────────────────────────────────────

/// Bir klasörün boş olup olmadığını söyler (okunamıyorsa "boş değil" varsayar — güvenli taraf).
fn klasor_bos(kok: &Path) -> bool {
    match fs::read_dir(kok) {
        Ok(mut it) => it.next().is_none(),
        Err(_) => false,
    }
}

/// Bayt dilimini UTF-8 metne çevirir; başarısızsa bozulma hatası döner.
fn metne_cevir(bayt: &[u8], yol: &Path) -> Result<String, ErrorReport> {
    String::from_utf8(bayt.to_vec())
        .map_err(|_| bozuk_toml_hatasi("Proje dosyası", yol, "geçersiz UTF-8"))
}

fn zaten_var_hatasi(kok: &Path) -> ErrorReport {
    ErrorReport::new(
        "Hedef klasör boş değil",
        format!(
            "'{}' zaten var ve içinde dosyalar bulunuyor.",
            kok.display()
        ),
        "Farklı bir proje adı/konumu seçin ya da hedef klasörü boşaltın.",
    )
    .with_eylem("Başka konum seç")
}

fn proje_degil_hatasi(kok: &Path) -> ErrorReport {
    ErrorReport::new(
        "Bu klasör bir BioCraft projesi değil",
        format!(
            "'{}' içinde beklenen bütünlük/manifest dosyaları bulunamadı.",
            kok.display()
        ),
        "Geçerli bir proje klasörü (biocraft.toml içeren) seçin.",
    )
}

fn bozuk_toml_hatasi(ad: &str, yol: &Path, detay: &str) -> ErrorReport {
    ErrorReport::new(
        format!("{ad} okunamadı"),
        format!("{ad} beklenen biçimde değil ({detay})."),
        "Dosyayı yedekten geri yükleyin (sessiz açma yapılmaz).",
    )
    .with_teknik_detay(format!("{detay}: {}", yol.display()))
}

fn harici_eksik_hatasi(mantiksal: &str, ipucu: &str) -> ErrorReport {
    ErrorReport::new(
        "Bağlantılı veri dosyası bulunamadı",
        format!("'{mantiksal}' için beklenen dosya '{ipucu}' konumunda yok."),
        "Dosyayı eski konumuna geri koyun ya da proje ayarlarından referansı güncelleyin.",
    )
    .with_eylem("Konumu güncelle")
}

fn harici_bozuk_hatasi(mantiksal: &str, ipucu: &str) -> ErrorReport {
    ErrorReport::new(
        "Bağlantılı veri dosyası değişmiş",
        format!("'{mantiksal}' ('{ipucu}') dosyasının BLAKE3 özeti kayıtlı değerle uyuşmuyor."),
        "Dosya değiştirilmiş/bozulmuş olabilir; doğru sürümle değiştirin ya da referansı yenileyin.",
    )
    .with_eylem("Referansı yenile")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gecici_kok(etiket: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "bc_proje_{}_{}_{}",
            etiket,
            std::process::id(),
            simdi_ns()
        ));
        let _ = fs::remove_dir_all(&p);
        p
    }

    fn simdi_ns() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    }

    fn ornek_girdi(konum: &Path) -> ProjeKurulumGirdisi {
        let mut g = ProjeKurulumGirdisi::yeni(
            "Deneme Projesi",
            konum,
            "genomik",
            DataClassification::HasasPhi,
            Version::new(0, 1, 0),
        );
        g.orcid = Some("0000-0002-1825-0097".to_string());
        g.kurum = "Test Lab".to_string();
        g.etiketler = vec!["genom".to_string(), "deneme".to_string()];
        g
    }

    #[test]
    fn olustur_acik_klasor_ve_gecerli_manifest_uretir() {
        let konum = gecici_kok("create");
        fs::create_dir_all(&konum).unwrap();
        let g = ornek_girdi(&konum);
        let kurulan = olustur(&g).unwrap();

        // Açık klasör + manifest var.
        assert!(format::proje_mi(&kurulan.kok));
        assert!(format::meta_yolu(&kurulan.kok).is_file());
        assert!(format::butunluk_yolu(&kurulan.kok).is_file());
        // ORCID + sınıflandırma + sürüm + göç alanları manifestte.
        assert_eq!(
            kurulan.manifest.olusturan.orcid.as_deref(),
            Some("0000-0002-1825-0097")
        );
        assert_eq!(
            kurulan.manifest.siniflandirma.sinif,
            DataClassification::HasasPhi
        );
        assert_eq!(
            kurulan.manifest.kimlik.format_surumu,
            format::format_surumu()
        );
        assert_eq!(kurulan.manifest.goc.len(), 1);

        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn acilista_butunluk_denetimi_gecer() {
        let konum = gecici_kok("open_ok");
        fs::create_dir_all(&konum).unwrap();
        let kurulan = olustur(&ornek_girdi(&konum)).unwrap();

        let acilan = ac(&kurulan.kok).unwrap();
        assert_eq!(acilan.manifest, kurulan.manifest);
        assert!(acilan.uyarilar.is_empty());

        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn bozuk_manifest_acilista_net_hata() {
        let konum = gecici_kok("open_corrupt");
        fs::create_dir_all(&konum).unwrap();
        let kurulan = olustur(&ornek_girdi(&konum)).unwrap();

        // Manifesti boz (özet artık tutmaz).
        let mut metin = fs::read_to_string(format::manifest_yolu(&kurulan.kok)).unwrap();
        metin.push_str("\n# kurcalandı\n");
        fs::write(format::manifest_yolu(&kurulan.kok), metin).unwrap();

        let hata = ac(&kurulan.kok).unwrap_err();
        assert!(hata.ne_oldu.contains("Manifest"));

        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn proje_olmayan_klasor_net_hata() {
        let bos = gecici_kok("not_project");
        fs::create_dir_all(&bos).unwrap();
        let hata = ac(&bos).unwrap_err();
        assert!(hata.ne_oldu.contains("BioCraft projesi değil"));
        let _ = fs::remove_dir_all(&bos);
    }

    #[test]
    fn dolu_klasore_kurulum_reddedilir() {
        let konum = gecici_kok("occupied");
        let kok = konum.join("Deneme Projesi");
        fs::create_dir_all(&kok).unwrap();
        fs::write(kok.join("var.txt"), b"dolu").unwrap();

        let hata = olustur(&ornek_girdi(&konum)).unwrap_err();
        assert!(hata.ne_oldu.contains("boş değil"));
        // Var olan kullanıcı dosyası KORUNMUŞ olmalı (atomik temizlik onu silmez).
        assert!(kok.join("var.txt").is_file());

        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn export_bcproj_uretir_hassas_ayar_haric() {
        let konum = gecici_kok("export");
        fs::create_dir_all(&konum).unwrap();
        let kurulan = olustur(&ornek_girdi(&konum)).unwrap();

        // Küçük bir veri dosyası ekle (gömülmeli).
        fs::write(kurulan.kok.join("data/inputs/ornek.txt"), b"kucuk veri").unwrap();

        let hedef = konum.join("paket.bcproj");
        let rapor = disa_aktar(&kurulan.kok, &hedef, &DisaAktarSecenekleri::default()).unwrap();
        assert!(hedef.is_file());
        assert!(rapor.hassas_haric);

        // ZIP içinde hassas [guvenlik] bölümü görünmemeli.
        let icerik = fs::read(&hedef).unwrap();
        let pencere_yok = !icerik.windows(10).any(|w| w == b"[guvenlik]");
        assert!(pencere_yok, "hassas [guvenlik] bölümü .bcproj'a sızmış");
        // Küçük veri gömülmüş olmalı.
        assert!(icerik.windows(10).any(|w| w == b"kucuk veri"));

        let _ = fs::remove_dir_all(&konum);
    }

    #[test]
    fn kurulum_basarisizsa_yari_klasor_birakmaz() {
        // 'konum' bir DOSYA olsun → altına klasör açılamaz → kurulum başarısız olur.
        let konum_dosya = gecici_kok("cleanup");
        fs::write(&konum_dosya, b"ben bir dosyayim").unwrap();
        let g = ProjeKurulumGirdisi::yeni(
            "Proje",
            &konum_dosya,
            "bos",
            DataClassification::Normal,
            Version::new(0, 1, 0),
        );
        let sonuc = olustur(&g);
        assert!(sonuc.is_err(), "dosya altına kurulum başarısız olmalı");
        // Yarım proje klasörü oluşmamalı; kullanıcının dosyası yerinde durmalı.
        assert!(konum_dosya.is_file());
        assert!(!g.proje_kok().exists());
        let _ = fs::remove_file(&konum_dosya);
    }
}
