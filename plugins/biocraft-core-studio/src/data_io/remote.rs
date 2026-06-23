//! ÇE-01 — **Uzak (HTTP/S3) bayt-aralığı erişim + önbellek + bütünlük + KARANTİNA** (MK-33).
//!
//! Hedef: indeksli uzak dosyalardan (BAM/VCF.gz…) **tüm dosyayı indirmeden** yalnızca gereken
//! **bayt aralığını** çekmek (HTTP `Range` / S3 byte-range), önbelleğe almak, **BLAKE3** ile
//! doğrulamak; bozuksa **karantinaya** alıp yeniden indirmek (**sessiz açma YOK** — MK-33).
//!
//! ## Mimari (İP-15 deseni)
//! Gerçek ağ istemcisi (HTTP/S3) **bu sürümde eklenmez** — proje hiçbir yere gerçek ağ yığını
//! koymadı (İP-15: Iroh crate'i bile eklenmedi, "yalnız arayüz").  Burada:
//! * **[`UzakOkuyucu`] trait'i** bayt-aralığı soyutlamasıdır (HTTP Range / S3 / yerel).
//! * **[`YerelBaytAralik`]** = yerel dosyayı "uzak" gibi okuyan backend → **tümünü okumadan**
//!   yalnızca istenen aralığı `seek`+`read` ile çeker (semantiğin offline, deterministik kanıtı).
//! * **[`HttpOkuyucu`]** gerçek istemci yerine **dürüst yer tutucudur** (MK-48): yapılandırma
//!   ([`HttpYapilandirma`]: bağlantı 10s / boşta 60s zaman aşımı + tekrar/geri-çekilme) taşır,
//!   ama bayt çekmeyi **"net istemcisi yapılandırılmadı"** ile reddeder; gerçek `ureq`/`reqwest`
//!   adaptörü ileride (eklenti/insan-eli) bu trait'i uygular.
//! * **[`Onbellek`]**, **bütünlük/karantina** ve **tekrar/geri-çekilme** gerçek ve testlidir.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use biocraft_sdk::biocraft_types::ErrorReport;

// ─── Bayt-aralığı soyutlaması ────────────────────────────────────────────────────

/// Bir uzak/yerel kaynaktan **yalnızca istenen bayt aralığını** çekebilen okuyucu (HTTP Range
/// / S3 byte-range semantiği).  Tüm dosyayı indirmeyi gerektirmez.
pub trait UzakOkuyucu {
    /// Kaynağın toplam boyutu (bayt).
    fn boyut(&self) -> Result<u64, ErrorReport>;
    /// `[ofset, ofset+uzunluk)` aralığını çeker (yalnızca bu baytlar).
    fn bayt_araligi(&self, ofset: u64, uzunluk: u64) -> Result<Vec<u8>, ErrorReport>;
}

/// Yerel dosyayı "uzak" gibi okuyan backend — yalnızca istenen aralığı `seek`+`read` eder
/// (tüm dosya belleğe/indirmeye ALINMAZ).  Uzak bayt-aralığı semantiğinin offline kanıtı.
pub struct YerelBaytAralik {
    yol: PathBuf,
}

impl YerelBaytAralik {
    /// Yeni yerel backend.
    pub fn yeni(yol: impl Into<PathBuf>) -> Self {
        Self { yol: yol.into() }
    }
}

impl UzakOkuyucu for YerelBaytAralik {
    fn boyut(&self) -> Result<u64, ErrorReport> {
        Ok(fs::metadata(&self.yol)
            .map_err(|e| io_hatasi(&self.yol, &e))?
            .len())
    }

    fn bayt_araligi(&self, ofset: u64, uzunluk: u64) -> Result<Vec<u8>, ErrorReport> {
        let mut f = File::open(&self.yol).map_err(|e| io_hatasi(&self.yol, &e))?;
        f.seek(SeekFrom::Start(ofset))
            .map_err(|e| io_hatasi(&self.yol, &e))?;
        // Yalnızca `uzunluk` bayt oku (dosya sonuna kadar kısalabilir).
        let mut tampon = vec![0u8; uzunluk as usize];
        let mut okunan = 0usize;
        while okunan < tampon.len() {
            let n = f
                .read(&mut tampon[okunan..])
                .map_err(|e| io_hatasi(&self.yol, &e))?;
            if n == 0 {
                break;
            }
            okunan += n;
        }
        tampon.truncate(okunan);
        Ok(tampon)
    }
}

/// Uzak HTTP(S)/S3 istemci **yapılandırması** (zaman aşımı + tekrar; MK Gün 4 hata + tekrar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpYapilandirma {
    /// Bağlantı zaman aşımı (varsayılan 10 sn).
    pub baglanti_zaman_asimi: Duration,
    /// Boşta (idle) zaman aşımı (varsayılan 60 sn).
    pub bosta_zaman_asimi: Duration,
    /// En fazla tekrar sayısı (geri-çekilme ile).
    pub azami_tekrar: u32,
    /// Üstel geri-çekilme tabanı (0 → bekleme yok; test için).
    pub geri_cekilme_taban: Duration,
}

impl Default for HttpYapilandirma {
    fn default() -> Self {
        Self {
            baglanti_zaman_asimi: Duration::from_secs(10),
            bosta_zaman_asimi: Duration::from_secs(60),
            azami_tekrar: 5,
            geri_cekilme_taban: Duration::from_secs(1),
        }
    }
}

/// Gerçek HTTP/S3 istemcisi yerine **dürüst yer tutucu** (MK-48).  Yapılandırma taşır; bayt
/// çekmeyi "net istemcisi yapılandırılmadı" ile reddeder.  Gerçek adaptör (ureq/reqwest, S3)
/// ileride bu trait'i uygular (eklenti/insan-eli; `net` yetkisi gerektirir).
pub struct HttpOkuyucu {
    url: String,
    #[allow(dead_code)]
    yapi: HttpYapilandirma,
}

impl HttpOkuyucu {
    /// Yeni (yapılandırılmamış) HTTP okuyucu.
    pub fn yeni(url: impl Into<String>, yapi: HttpYapilandirma) -> Self {
        Self {
            url: url.into(),
            yapi,
        }
    }
}

impl UzakOkuyucu for HttpOkuyucu {
    fn boyut(&self) -> Result<u64, ErrorReport> {
        Err(net_yapilandirilmadi(&self.url))
    }
    fn bayt_araligi(&self, _ofset: u64, _uzunluk: u64) -> Result<Vec<u8>, ErrorReport> {
        Err(net_yapilandirilmadi(&self.url))
    }
}

// ─── Önbellek ────────────────────────────────────────────────────────────────────

/// Çekilen aralıkları/dosyaları yerelde tutan basit dosya önbelleği (URL+aralık → dosya).
pub struct Onbellek {
    dizin: PathBuf,
}

impl Onbellek {
    /// Bir önbellek dizini açar (yoksa oluşturur).
    pub fn ac(dizin: impl Into<PathBuf>) -> Result<Self, ErrorReport> {
        let dizin = dizin.into();
        fs::create_dir_all(&dizin).map_err(|e| io_hatasi(&dizin, &e))?;
        Ok(Self { dizin })
    }

    /// Bir anahtar (URL+aralık) için önbellek dosya yolu (BLAKE3 ile adlandırılır).
    pub fn yol(&self, anahtar: &str) -> PathBuf {
        let ad = blake3::hash(anahtar.as_bytes()).to_hex().to_string();
        self.dizin.join(ad)
    }

    /// Anahtar önbellekte var mı?
    pub fn var_mi(&self, anahtar: &str) -> bool {
        self.yol(anahtar).exists()
    }

    /// Önbellekten oku (yoksa `None`).
    pub fn oku(&self, anahtar: &str) -> Option<Vec<u8>> {
        fs::read(self.yol(anahtar)).ok()
    }

    /// Önbelleğe yaz.
    pub fn yaz(&self, anahtar: &str, veri: &[u8]) -> Result<(), ErrorReport> {
        let yol = self.yol(anahtar);
        let mut f = File::create(&yol).map_err(|e| io_hatasi(&yol, &e))?;
        f.write_all(veri).map_err(|e| io_hatasi(&yol, &e))?;
        Ok(())
    }
}

// ─── Bütünlük + KARANTİNA (MK-33) ────────────────────────────────────────────────

/// Bir karantinaya alma işleminin sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KarantinaSonucu {
    /// Bozuk dosyanın taşındığı karantina yolu.
    pub karantina_yolu: PathBuf,
}

/// İndirilen bir baytlar dizisini **beklenen BLAKE3** ile doğrular.  Uyuşmazsa
/// `Err` (sessiz açma YOK) — çağıran yeniden indirir.
pub fn baytlari_dogrula(veri: &[u8], beklenen_hex: &str) -> Result<(), ErrorReport> {
    let gercek = blake3::hash(veri).to_hex().to_string();
    if gercek.eq_ignore_ascii_case(beklenen_hex) {
        Ok(())
    } else {
        Err(butunluk_hatasi(beklenen_hex, &gercek))
    }
}

/// Bir **dosyayı** beklenen BLAKE3'e karşı doğrular; **bozuksa KARANTİNAYA alır** (dosyayı
/// `karantina_dizin`'e taşır) ve yeniden indirme öneren bir hata döner — **sessiz açma YOK**
/// (MK-33).  Sağlamsa `Ok(None)`; bozuksa `Ok` döndürmez (taşıma sonrası `Err`).
pub fn dogrula_veya_karantina(
    yol: &Path,
    beklenen_hex: &str,
    karantina_dizin: &Path,
) -> Result<(), ErrorReport> {
    let gercek = super::integrity::blake3_dosya(yol)?;
    if gercek.eq_ignore_ascii_case(beklenen_hex) {
        return Ok(());
    }
    // Bozuk → karantinaya taşı.
    let sonuc = karantinaya_al(yol, karantina_dizin)?;
    Err(butunluk_hatasi(beklenen_hex, &gercek)
        .with_teknik_detay(format!("karantina: {}", sonuc.karantina_yolu.display())))
}

/// Bir dosyayı karantina dizinine taşır (rename; aygıtlar arası ise kopyala+sil).
pub fn karantinaya_al(yol: &Path, karantina_dizin: &Path) -> Result<KarantinaSonucu, ErrorReport> {
    fs::create_dir_all(karantina_dizin).map_err(|e| io_hatasi(karantina_dizin, &e))?;
    let ad = yol
        .file_name()
        .map(|s| s.to_os_string())
        .unwrap_or_else(|| std::ffi::OsString::from("bilinmeyen"));
    let mut hedef = karantina_dizin.join(&ad);
    // Çakışmayı önlemek için zaman damgası ekle.
    if hedef.exists() {
        let zd = chrono::Utc::now().timestamp_millis();
        hedef = karantina_dizin.join(format!("{}.{zd}", ad.to_string_lossy()));
    }
    if fs::rename(yol, &hedef).is_err() {
        // Aygıtlar arası taşıma: kopyala + sil.
        fs::copy(yol, &hedef).map_err(|e| io_hatasi(&hedef, &e))?;
        let _ = fs::remove_file(yol);
    }
    Ok(KarantinaSonucu {
        karantina_yolu: hedef,
    })
}

// ─── Tekrar / geri-çekilme (Gün 4 hata + tekrar) ─────────────────────────────────

/// Bir işlemi **üstel geri-çekilme** ile en fazla `azami_tekrar+1` kez dener (ağ kesintisi →
/// tekrar).  Her deneme `f(deneme_no)` çağrılır; başarısızsa bekleyip tekrar dener.
pub fn tekrar_ile<T, F>(yapi: &HttpYapilandirma, mut f: F) -> Result<T, ErrorReport>
where
    F: FnMut(u32) -> Result<T, ErrorReport>,
{
    let mut son_hata: Option<ErrorReport> = None;
    for deneme in 0..=yapi.azami_tekrar {
        match f(deneme) {
            Ok(t) => return Ok(t),
            Err(e) => {
                son_hata = Some(e);
                if deneme < yapi.azami_tekrar && !yapi.geri_cekilme_taban.is_zero() {
                    // Üstel: taban * 2^deneme (tavan 60 sn).
                    let carpan = 1u32 << deneme.min(6);
                    let bekle = (yapi.geri_cekilme_taban * carpan).min(Duration::from_secs(60));
                    std::thread::sleep(bekle);
                }
            }
        }
    }
    Err(son_hata.unwrap_or_else(|| {
        ErrorReport::new(
            "Uzak işlem başarısız",
            "tüm denemeler tükendi",
            "Ağ bağlantısını kontrol edip yeniden deneyin",
        )
        .with_eylem("Yeniden dene")
    }))
}

/// **Bir bölgenin baytlarını** uzak kaynaktan çeker: önce önbellek, yoksa bayt-aralığı (tekrar
/// ile) + (opsiyonel) BLAKE3 doğrulama → önbelleğe yaz.  Tüm dosya indirilmez.
pub fn bolge_baytlari(
    okuyucu: &dyn UzakOkuyucu,
    url: &str,
    ofset: u64,
    uzunluk: u64,
    beklenen_blake3: Option<&str>,
    onbellek: Option<&Onbellek>,
    yapi: &HttpYapilandirma,
) -> Result<Vec<u8>, ErrorReport> {
    let anahtar = format!("{url}#{ofset}+{uzunluk}");

    // 1) Önbellek.
    if let Some(ob) = onbellek {
        if let Some(veri) = ob.oku(&anahtar) {
            if let Some(hex) = beklenen_blake3 {
                baytlari_dogrula(&veri, hex)?;
            }
            return Ok(veri);
        }
    }

    // 2) Çek (tekrar ile) — yalnızca istenen aralık.
    let veri = tekrar_ile(yapi, |_deneme| okuyucu.bayt_araligi(ofset, uzunluk))?;

    // 3) Bütünlük (beklenen biliniyorsa).
    if let Some(hex) = beklenen_blake3 {
        baytlari_dogrula(&veri, hex)?;
    }

    // 4) Önbelleğe yaz.
    if let Some(ob) = onbellek {
        ob.yaz(&anahtar, &veri)?;
    }

    Ok(veri)
}

// ─── Hatalar ────────────────────────────────────────────────────────────────────

fn net_yapilandirilmadi(url: &str) -> ErrorReport {
    ErrorReport::new(
        "Uzak ağ istemcisi yapılandırılmadı",
        format!(
            "'{url}' için gerçek HTTP/S3 istemcisi bu sürümde bağlı değil (yalnız arayüz; İP-15 deseni)",
        ),
        "Uzak erişim, 'net' yetkili bir ağ adaptörü (ureq/reqwest/S3) bağlanınca etkin olur",
    )
    .with_eylem("Daha sonra")
}

fn butunluk_hatasi(beklenen: &str, gercek: &str) -> ErrorReport {
    ErrorReport::new(
        "Dosya bütünlüğü doğrulanamadı",
        "indirilen verinin BLAKE3 özeti beklenenle eşleşmiyor (bozuk/eksik/değiştirilmiş olabilir)",
        "Dosya karantinaya alındı; yeniden indirin (sessiz açma yapılmaz)",
    )
    .with_eylem("Yeniden indir")
    .with_teknik_detay(format!("beklenen={beklenen} gerçek={gercek}"))
}

fn io_hatasi(yol: &Path, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "Dosya/dizin erişilemedi",
        format!("'{}' erişiminde G/Ç hatası", yol.display()),
        "Yolu, izinleri ve disk alanını kontrol edin",
    )
    .with_teknik_detay(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn gecici_dosya(ad: &str, icerik: &[u8]) -> PathBuf {
        let mut yol = std::env::temp_dir();
        yol.push(format!("biocraft_remote_{}_{ad}", std::process::id()));
        File::create(&yol).unwrap().write_all(icerik).unwrap();
        yol
    }

    #[test]
    fn yerel_bayt_araligi_yalniz_bolge_okur() {
        // 1000 baytlık "dosya"; yalnızca [100,110) okunur — tümü indirilmez.
        let veri: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
        let yol = gecici_dosya("buyuk.bin", &veri);
        let backend = YerelBaytAralik::yeni(&yol);

        assert_eq!(backend.boyut().unwrap(), 1000);
        let parca = backend.bayt_araligi(100, 10).unwrap();
        assert_eq!(parca.len(), 10);
        assert_eq!(parca, &veri[100..110]);
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn onbellek_yazar_ve_okur() {
        let dizin = std::env::temp_dir().join(format!("biocraft_ob_{}", std::process::id()));
        let ob = Onbellek::ac(&dizin).unwrap();
        assert!(!ob.var_mi("a#0+5"));
        ob.yaz("a#0+5", b"hello").unwrap();
        assert!(ob.var_mi("a#0+5"));
        assert_eq!(ob.oku("a#0+5").as_deref(), Some(&b"hello"[..]));
        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn bolge_baytlari_onbellekler_ve_dogrular() {
        let veri: Vec<u8> = (0..500u32).map(|i| i as u8).collect();
        let yol = gecici_dosya("c.bin", &veri);
        let backend = YerelBaytAralik::yeni(&yol);
        let dizin = std::env::temp_dir().join(format!("biocraft_ob2_{}", std::process::id()));
        let ob = Onbellek::ac(&dizin).unwrap();
        let yapi = HttpYapilandirma::default();

        let beklenen = blake3::hash(&veri[10..20]).to_hex().to_string();
        let p1 =
            bolge_baytlari(&backend, "u://x", 10, 10, Some(&beklenen), Some(&ob), &yapi).unwrap();
        assert_eq!(p1, &veri[10..20]);
        // İkinci çağrı önbellekten gelir (içerik aynı).
        assert!(ob.var_mi("u://x#10+10"));
        let p2 =
            bolge_baytlari(&backend, "u://x", 10, 10, Some(&beklenen), Some(&ob), &yapi).unwrap();
        assert_eq!(p2, p1);

        let _ = std::fs::remove_file(&yol);
        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn bozuk_dosya_karantinaya_alinir() {
        let yol = gecici_dosya("bozuk.bin", b"bozuk veri");
        let karantina = std::env::temp_dir().join(format!("biocraft_kar_{}", std::process::id()));
        // Yanlış beklenen özet → karantina + hata (sessiz açma yok).
        let hata = dogrula_veya_karantina(&yol, &"0".repeat(64), &karantina)
            .err()
            .unwrap();
        assert_eq!(hata.ne_oldu, "Dosya bütünlüğü doğrulanamadı");
        // Orijinal taşındı (artık yerinde yok), karantinada bir dosya var.
        assert!(!yol.exists());
        let say = std::fs::read_dir(&karantina).unwrap().count();
        assert_eq!(say, 1);
        let _ = std::fs::remove_dir_all(&karantina);
    }

    #[test]
    fn saglam_dosya_karantinaya_alinmaz() {
        let icerik = b"saglam veri";
        let yol = gecici_dosya("saglam.bin", icerik);
        let blake3 = blake3::hash(icerik).to_hex().to_string();
        let karantina = std::env::temp_dir().join(format!("biocraft_kar2_{}", std::process::id()));
        assert!(dogrula_veya_karantina(&yol, &blake3, &karantina).is_ok());
        assert!(yol.exists()); // taşınmadı
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn tekrar_geri_cekilme_n_kez_dener() {
        let yapi = HttpYapilandirma {
            azami_tekrar: 3,
            geri_cekilme_taban: Duration::ZERO, // test: bekleme yok
            ..Default::default()
        };
        let sayac = AtomicU32::new(0);
        let sonuc: Result<(), _> = tekrar_ile(&yapi, |_| {
            sayac.fetch_add(1, Ordering::SeqCst);
            Err(ErrorReport::new("hata", "deneme", "tekrar"))
        });
        assert!(sonuc.is_err());
        // 1 ilk + 3 tekrar = 4 deneme.
        assert_eq!(sayac.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn http_okuyucu_durust_reddeder() {
        let r = HttpOkuyucu::yeni("https://example.org/a.bam", HttpYapilandirma::default());
        let hata = r.bayt_araligi(0, 10).err().unwrap();
        assert_eq!(hata.ne_oldu, "Uzak ağ istemcisi yapılandırılmadı");
    }
}
