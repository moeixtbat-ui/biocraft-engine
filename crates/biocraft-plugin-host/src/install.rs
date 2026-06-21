//! Eklenti **kurulum / güncelleme / kaldırma** + `.bcext` çevrimdışı paket (İP-07).
//!
//! Eklentiler mağazadan **veya** `.bcext` dosyasından (çevrimdışı/kurumsal) kurulur.
//! Bu modül:
//! * **`.bcext` biçimi** (`BCEXT1`) — bağımlılıksız, saf-Rust, uzunluk-önekli arşiv +
//!   opsiyonel **Ed25519 imza fragmanı** (bkz. [`crate::signature`]).  ZIP/sıkıştırma
//!   yerine bilinçli olarak basit/denetlenebilir bir biçim (MK-60 tekrarüretilebilirlik).
//! * **Kurulum** — imza politikası denetimi, yol-kaçışı koruması (VFS ile aynı), dosyaları
//!   eklenti dizinine açma.
//! * **Güncelleme** — **geri alınabilir** (eski sürüm yedeklenir; hata olursa geri sarılır);
//!   **aktif** eklenti güncellemesi **güvenli ana ertelenir** (`.beklemede` evresi).
//! * **Kaldırma** — "ayarları koru/sil" seçeneği (varsayılan **koru**).

use crate::manifest::Manifest;
use crate::signature::{GuvenDeposu, Imza, ImzaDurumu, ImzaPolitikasi, SigningKey};
use crate::vfs::SanalDosyaSistemi;
use biocraft_types::ErrorReport;
use std::path::{Path, PathBuf};

/// `.bcext` biçim sihirli başlığı (sürümlü).
pub const BCEXT_MAGIC: &[u8] = b"BCEXT1\n";

// ─── .bcext biçimi: yazma ─────────────────────────────────────────────────────

fn ic_hata(ne: &str, neden: impl Into<String>) -> ErrorReport {
    ErrorReport::new(
        ne,
        neden,
        "Paketi yeniden indirin; sorun sürerse yayıncıya bildirin",
    )
}

/// İmzalanan **yük** bölgesini (magic + sayım + girdiler) üretir.
fn yuk_uret(dosyalar: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(BCEXT_MAGIC);
    b.extend_from_slice(&(dosyalar.len() as u32).to_le_bytes());
    for (ad, icerik) in dosyalar {
        let ad_b = ad.as_bytes();
        b.extend_from_slice(&(ad_b.len() as u16).to_le_bytes());
        b.extend_from_slice(ad_b);
        b.extend_from_slice(&(icerik.len() as u64).to_le_bytes());
        b.extend_from_slice(icerik);
    }
    b
}

/// İmzasız bir `.bcext` paketi üretir.
pub fn paketle(dosyalar: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut b = yuk_uret(dosyalar);
    b.push(0u8); // imza yok
    b
}

/// Ed25519 ile **imzalı** bir `.bcext` paketi üretir.
pub fn paketle_imzali(dosyalar: &[(String, Vec<u8>)], imzalayici: &SigningKey) -> Vec<u8> {
    let yuk = yuk_uret(dosyalar);
    let imza = Imza::olustur(&yuk, imzalayici);
    let mut b = yuk;
    b.push(1u8); // imza var
    b.extend_from_slice(&imza.acik_anahtar);
    b.extend_from_slice(&imza.imza);
    b
}

/// Bir dizini özyinelemeli tarayıp `(göreli-yol, içerik)` listesi üretir (paketleme için).
///
/// Yollar `/` ile normalize edilir (platformdan bağımsız paket).
pub fn dizinden_topla(kok: &Path) -> Result<Vec<(String, Vec<u8>)>, ErrorReport> {
    let mut out = Vec::new();
    topla_ic(kok, kok, &mut out)?;
    out.sort_by(|a, b| a.0.cmp(&b.0)); // belirleyici sıra
    Ok(out)
}

fn topla_ic(kok: &Path, dizin: &Path, out: &mut Vec<(String, Vec<u8>)>) -> Result<(), ErrorReport> {
    let okuma = std::fs::read_dir(dizin)
        .map_err(|e| ic_hata("Paketleme için dizin okunamadı", e.to_string()))?;
    for giris in okuma.flatten() {
        let yol = giris.path();
        if yol.is_dir() {
            topla_ic(kok, &yol, out)?;
        } else if yol.is_file() {
            let goreli = yol
                .strip_prefix(kok)
                .map_err(|_| ic_hata("Paketleme yolu çözülemedi", "kök dışı yol"))?;
            let ad = goreli.to_string_lossy().replace('\\', "/");
            let icerik = std::fs::read(&yol)
                .map_err(|e| ic_hata("Paketleme için dosya okunamadı", e.to_string()))?;
            out.push((ad, icerik));
        }
    }
    Ok(())
}

// ─── .bcext biçimi: okuma ─────────────────────────────────────────────────────

/// Ayrıştırılmış bir `.bcext` paketi.
#[derive(Debug, Clone)]
pub struct BcextPaket {
    /// `(göreli-yol, içerik)` çiftleri.
    pub dosyalar: Vec<(String, Vec<u8>)>,
    /// Varsa ayrık imza.
    pub imza: Option<Imza>,
    /// İmzanın kapsadığı yük bölgesi (imza doğrulamasında kullanılır).
    yuk: Vec<u8>,
}

/// Bayt diliminden küçük yardımcı LE okuyucu.
struct Okuyucu<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Okuyucu<'a> {
    fn yeni(b: &'a [u8]) -> Self {
        Self { b, i: 0 }
    }
    fn al(&mut self, n: usize) -> Result<&'a [u8], ErrorReport> {
        let son = self
            .i
            .checked_add(n)
            .filter(|s| *s <= self.b.len())
            .ok_or_else(|| {
                ic_hata(
                    "Eklenti paketi bozuk",
                    "paket beklenenden kısa (eksik bayt)",
                )
            })?;
        let dilim = &self.b[self.i..son];
        self.i = son;
        Ok(dilim)
    }
    fn u16(&mut self) -> Result<u16, ErrorReport> {
        Ok(u16::from_le_bytes(self.al(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, ErrorReport> {
        Ok(u32::from_le_bytes(self.al(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, ErrorReport> {
        Ok(u64::from_le_bytes(self.al(8)?.try_into().unwrap()))
    }
}

impl BcextPaket {
    /// `.bcext` baytlarını ayrıştırır (yapısal bütünlük denetimiyle).
    pub fn ac(bytes: &[u8]) -> Result<Self, ErrorReport> {
        // İP-09 sertleştirme: güvenilmeyen paket → kaynak-istismarı (devasa/çok-girdi) limitleri.
        // (Okuyucu zaten checked_add + sınır denetimiyle bellek-güvenlidir; bu, ek kaynak tavanıdır.)
        let limit = crate::harden::AyristirmaLimitleri::siki();
        limit.girdi_denetle(bytes.len() as u64)?;

        let mut o = Okuyucu::yeni(bytes);
        if o.al(BCEXT_MAGIC.len())? != BCEXT_MAGIC {
            return Err(ErrorReport::new(
                "Geçersiz eklenti paketi",
                "dosya bir .bcext paketi değil (sihirli başlık eşleşmiyor)",
                "Doğru .bcext dosyasını seçtiğinizden emin olun",
            ));
        }
        let sayi = o.u32()? as usize;
        limit.girdi_sayisi_denetle(sayi)?;
        let mut dosyalar = Vec::with_capacity(sayi.min(1024));
        let mut toplam_icerik = 0u64;
        for _ in 0..sayi {
            let ad_len = o.u16()? as usize;
            let ad = String::from_utf8(o.al(ad_len)?.to_vec())
                .map_err(|_| ic_hata("Eklenti paketi bozuk", "dosya adı geçerli UTF-8 değil"))?;
            let icerik_len = o.u64()? as usize;
            // Birikmiş içerik tavanı (çok büyük paket açma sırasında durdurulur — checked).
            toplam_icerik = limit.cikti_denetle(toplam_icerik, icerik_len as u64)?;
            let icerik = o.al(icerik_len)?.to_vec();
            dosyalar.push((ad, icerik));
        }
        // Yük bölgesi = magic'ten girdi sonuna; imza bunun üzerine atılmıştır.
        let yuk = bytes[..o.i].to_vec();

        let imza = match o.al(1)?[0] {
            0 => None,
            1 => {
                let acik_anahtar = o.al(32)?.to_vec();
                let imza_b = o.al(64)?.to_vec();
                Some(Imza {
                    acik_anahtar,
                    imza: imza_b,
                })
            }
            d => {
                return Err(ic_hata(
                    "Eklenti paketi bozuk",
                    format!("bilinmeyen imza bayrağı: {d}"),
                ))
            }
        };

        Ok(Self {
            dosyalar,
            imza,
            yuk,
        })
    }

    /// Paketteki bir dosyanın içeriği.
    pub fn dosya(&self, ad: &str) -> Option<&[u8]> {
        self.dosyalar
            .iter()
            .find(|(a, _)| a == ad)
            .map(|(_, c)| c.as_slice())
    }

    /// Paketteki `biocraft.toml`'u bulup ayrıştırır.
    pub fn manifest(&self) -> Result<Manifest, ErrorReport> {
        let metin = self.dosya(crate::discover::MANIFEST_DOSYA).ok_or_else(|| {
            ErrorReport::new(
                "Eklenti paketinde manifest yok",
                "pakette biocraft.toml bulunamadı",
                "Geçerli bir eklenti paketi kullanın",
            )
        })?;
        let metin = std::str::from_utf8(metin)
            .map_err(|_| ic_hata("Manifest okunamadı", "biocraft.toml geçerli UTF-8 değil"))?;
        Manifest::ayristir(metin)
    }

    /// Paketin imza/bütünlük durumunu güven deposuna karşı değerlendirir.
    pub fn imza_durumu(&self, depo: &GuvenDeposu) -> ImzaDurumu {
        ImzaDurumu::degerlendir(&self.yuk, self.imza.as_ref(), depo)
    }
}

// ─── Kurulum yöneticisi ───────────────────────────────────────────────────────

/// Başarılı kurulum sonucu.
#[derive(Debug, Clone)]
pub struct KurulumSonucu {
    /// Kurulan eklentinin kimliği.
    pub kimlik: String,
    /// Kurulduğu dizin.
    pub hedef: PathBuf,
    /// Kurulumdaki imza durumu (rozet için).
    pub imza_durumu: ImzaDurumu,
}

/// Güncelleme sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuncellemeSonucu {
    /// Güncellendi; `geri_alinabilir` = eski sürüm yedeği mevcut.
    Guncellendi {
        kimlik: String,
        geri_alinabilir: bool,
    },
    /// Eklenti **aktif** olduğu için güncelleme **güvenli ana ertelendi** (`.beklemede`).
    Ertelendi { kimlik: String },
}

/// Eklenti kurulum/güncelleme/kaldırma yöneticisi.
#[derive(Debug, Clone)]
pub struct Kurucu {
    eklenti_dizini: PathBuf,
    ayar_dizini: PathBuf,
    politika: ImzaPolitikasi,
}

impl Kurucu {
    /// Eklenti ve ayar dizinleriyle yeni bir kurucu (varsayılan imza politikası).
    pub fn yeni(eklenti_dizini: impl Into<PathBuf>, ayar_dizini: impl Into<PathBuf>) -> Self {
        Self {
            eklenti_dizini: eklenti_dizini.into(),
            ayar_dizini: ayar_dizini.into(),
            politika: ImzaPolitikasi::default(),
        }
    }

    /// İmza politikasını ayarlar (akıcı).
    pub fn politika_ile(mut self, politika: ImzaPolitikasi) -> Self {
        self.politika = politika;
        self
    }

    fn hedef(&self, kimlik: &str) -> PathBuf {
        self.eklenti_dizini.join(kimlik)
    }
    fn yedek(&self, kimlik: &str) -> PathBuf {
        self.eklenti_dizini.join(format!("{kimlik}.yedek"))
    }
    fn beklemede(&self, kimlik: &str) -> PathBuf {
        self.eklenti_dizini.join(format!("{kimlik}.beklemede"))
    }

    /// Bir paketi **kurar** (ilk kez).  Zaten kuruluysa hata → güncelleme kullanılır.
    pub fn kur(
        &self,
        paket: &BcextPaket,
        depo: &GuvenDeposu,
    ) -> Result<KurulumSonucu, ErrorReport> {
        let manifest = paket.manifest()?;
        let durum = paket.imza_durumu(depo);
        self.politika.denetle(&durum)?;

        let kimlik = manifest.kimlik.metni().to_string();
        let hedef = self.hedef(&kimlik);
        if hedef.exists() {
            return Err(ErrorReport::new(
                "Eklenti zaten kurulu",
                format!("'{kimlik}' zaten kurulu görünüyor"),
                "Yeni sürümü kurmak için güncelleme yapın",
            )
            .with_eylem("Güncelle"));
        }

        self.dosyalari_yaz(&hedef, paket)?;
        Ok(KurulumSonucu {
            kimlik,
            hedef,
            imza_durumu: durum,
        })
    }

    /// Bir paketi **günceller** (geri alınabilir; aktifse ertelenir).
    ///
    /// * `aktif` — eklenti şu an çalışıyor mu?  Çalışıyorsa güncelleme **`.beklemede`**
    ///   evresine alınır ve güvenli anda [`bekleyeni_uygula`](Self::bekleyeni_uygula) ile
    ///   tamamlanır (kullanım sırasında dosya değiştirmeyiz).
    pub fn guncelle(
        &self,
        paket: &BcextPaket,
        aktif: bool,
        depo: &GuvenDeposu,
    ) -> Result<GuncellemeSonucu, ErrorReport> {
        let manifest = paket.manifest()?;
        let durum = paket.imza_durumu(depo);
        self.politika.denetle(&durum)?;

        let kimlik = manifest.kimlik.metni().to_string();
        let hedef = self.hedef(&kimlik);
        if !hedef.exists() {
            return Err(ErrorReport::new(
                "Eklenti kurulu değil",
                format!("'{kimlik}' güncellenemiyor çünkü kurulu değil"),
                "Önce eklentiyi kurun",
            ));
        }

        if aktif {
            // Aktif eklenti: dosyaları şimdi değiştirme; .beklemede'ye hazırla.
            let beklemede = self.beklemede(&kimlik);
            self.sil_varsa(&beklemede)?;
            self.dosyalari_yaz(&beklemede, paket)?;
            return Ok(GuncellemeSonucu::Ertelendi { kimlik });
        }

        // Pasif eklenti: yedekle → yaz → hata olursa geri sar.
        self.takasla_yedekli(&kimlik, |h| self.dosyalari_yaz(h, paket))?;
        Ok(GuncellemeSonucu::Guncellendi {
            kimlik,
            geri_alinabilir: true,
        })
    }

    /// Ertelenmiş (`.beklemede`) bir güncellemeyi güvenli anda uygular.
    pub fn bekleyeni_uygula(&self, kimlik: &str) -> Result<GuncellemeSonucu, ErrorReport> {
        let beklemede = self.beklemede(kimlik);
        if !beklemede.exists() {
            return Err(ErrorReport::new(
                "Bekleyen güncelleme yok",
                format!("'{kimlik}' için ertelenmiş bir güncelleme bulunamadı"),
                "Önce güncellemeyi başlatın",
            ));
        }
        self.takasla_yedekli(kimlik, |hedef| {
            // .beklemede → hedef taşı.
            std::fs::rename(&beklemede, hedef)
                .map_err(|e| ic_hata("Bekleyen güncelleme uygulanamadı", e.to_string()))
        })?;
        Ok(GuncellemeSonucu::Guncellendi {
            kimlik: kimlik.to_string(),
            geri_alinabilir: true,
        })
    }

    /// Son güncellemeyi **geri alır** (yedekten eski sürümü döndürür).
    pub fn geri_al(&self, kimlik: &str) -> Result<(), ErrorReport> {
        let yedek = self.yedek(kimlik);
        if !yedek.exists() {
            return Err(ErrorReport::new(
                "Geri alınacak sürüm yok",
                format!("'{kimlik}' için bir yedek bulunamadı"),
                "Geri alma yalnızca son güncellemeden hemen sonra mümkündür",
            ));
        }
        let hedef = self.hedef(kimlik);
        self.sil_varsa(&hedef)?;
        std::fs::rename(&yedek, &hedef)
            .map_err(|e| ic_hata("Geri alma başarısız", e.to_string()))?;
        Ok(())
    }

    /// Bir eklentiyi **kaldırır**.  `ayarlari_koru=true` (varsayılan) ayarları bırakır.
    pub fn kaldir(&self, kimlik: &str, ayarlari_koru: bool) -> Result<(), ErrorReport> {
        let hedef = self.hedef(kimlik);
        if !hedef.exists() {
            return Err(ErrorReport::new(
                "Eklenti kurulu değil",
                format!("'{kimlik}' kaldırılamıyor çünkü kurulu değil"),
                "Kaldırılacak eklentinin kurulu olduğundan emin olun",
            ));
        }
        self.sil_varsa(&hedef)?;
        // Artık (varsa) yedek/beklemede kalıntılarını da temizle.
        self.sil_varsa(&self.yedek(kimlik))?;
        self.sil_varsa(&self.beklemede(kimlik))?;
        if !ayarlari_koru {
            self.sil_varsa(&self.ayar_dizini.join(kimlik))?;
        }
        Ok(())
    }

    // ─── iç yardımcılar ───────────────────────────────────────────────────────

    /// Paketi belirli bir hedefe açar; yol-kaçışı (VFS ile aynı kural) reddedilir.
    fn dosyalari_yaz(&self, hedef: &Path, paket: &BcextPaket) -> Result<(), ErrorReport> {
        std::fs::create_dir_all(hedef)
            .map_err(|e| ic_hata("Eklenti dizini oluşturulamadı", e.to_string()))?;
        let vfs = SanalDosyaSistemi::yeni(hedef);
        for (ad, icerik) in &paket.dosyalar {
            // Kök-kaçışı koruması (../, mutlak, sürücü öneki reddedilir).
            let yol = vfs.cozumle(ad)?;
            if let Some(ust) = yol.parent() {
                std::fs::create_dir_all(ust)
                    .map_err(|e| ic_hata("Alt dizin oluşturulamadı", e.to_string()))?;
            }
            std::fs::write(&yol, icerik)
                .map_err(|e| ic_hata("Eklenti dosyası yazılamadı", e.to_string()))?;
        }
        Ok(())
    }

    /// hedef'i yedekle, `yaz` ile yenisini koy; hata olursa yedekten geri sar (atomik etki).
    fn takasla_yedekli(
        &self,
        kimlik: &str,
        yaz: impl FnOnce(&Path) -> Result<(), ErrorReport>,
    ) -> Result<(), ErrorReport> {
        let hedef = self.hedef(kimlik);
        let yedek = self.yedek(kimlik);
        self.sil_varsa(&yedek)?;
        // Mevcut sürümü yedeğe taşı.
        std::fs::rename(&hedef, &yedek)
            .map_err(|e| ic_hata("Güncelleme için yedek alınamadı", e.to_string()))?;
        // Yeni sürümü yaz; hata olursa yedekten geri sar.
        match yaz(&hedef) {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = self.sil_varsa(&hedef);
                let _ = std::fs::rename(&yedek, &hedef);
                Err(e)
            }
        }
    }

    fn sil_varsa(&self, yol: &Path) -> Result<(), ErrorReport> {
        if yol.is_dir() {
            std::fs::remove_dir_all(yol).map_err(|e| ic_hata("Dizin silinemedi", e.to_string()))?;
        } else if yol.is_file() {
            std::fs::remove_file(yol).map_err(|e| ic_hata("Dosya silinemedi", e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static SAYAC: AtomicU32 = AtomicU32::new(0);

    /// Benzersiz geçici test dizini.
    fn gecici(etiket: &str) -> PathBuf {
        let n = SAYAC.fetch_add(1, Ordering::Relaxed);
        let p =
            std::env::temp_dir().join(format!("biocraft_test_{etiket}_{}_{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn ornek_dosyalar() -> Vec<(String, Vec<u8>)> {
        vec![
            (
                "biocraft.toml".to_string(),
                br#"
[eklenti]
kimlik = "biocraft.test.paket"
ad = "Test Paketi"
surum = "1.0.0"
katman = "wasm"
giris = "ana.wat"

[uyumluluk]
abi = "0.1"
cekirdek_min = "0.1.0"
"#
                .to_vec(),
            ),
            ("ana.wat".to_string(), b"(module)".to_vec()),
            ("veri/x.txt".to_string(), b"merhaba".to_vec()),
        ]
    }

    fn anahtar(t: u8) -> SigningKey {
        SigningKey::from_bytes(&[t; 32])
    }

    #[test]
    fn paketle_ac_gidis_donus() {
        let dosyalar = ornek_dosyalar();
        let bytes = paketle(&dosyalar);
        let paket = BcextPaket::ac(&bytes).unwrap();
        assert_eq!(paket.dosyalar.len(), 3);
        assert_eq!(paket.dosya("ana.wat"), Some(&b"(module)"[..]));
        assert!(paket.imza.is_none());
        assert_eq!(
            paket.manifest().unwrap().kimlik.metni(),
            "biocraft.test.paket"
        );
    }

    #[test]
    fn imzali_paket_resmi_dogrulanir() {
        let sk = anahtar(7);
        let mut depo = GuvenDeposu::bos();
        depo.resmi_ekle("BioCraft", sk.verifying_key());
        let bytes = paketle_imzali(&ornek_dosyalar(), &sk);
        let paket = BcextPaket::ac(&bytes).unwrap();
        assert!(paket.imza.is_some());
        assert!(paket.imza_durumu(&depo).resmi_mi());
    }

    #[test]
    fn bozulan_paket_imzasi_gecersiz() {
        let sk = anahtar(8);
        let mut bytes = paketle_imzali(&ornek_dosyalar(), &sk);
        // İçerik bölgesini boz (magic'ten sonra bir bayt).
        bytes[20] ^= 0xFF;
        // Yapı hâlâ ayrıştırılabiliyorsa imza geçersiz olmalı; ayrıştırılamıyorsa da hata.
        if let Ok(paket) = BcextPaket::ac(&bytes) {
            assert!(paket.imza_durumu(&GuvenDeposu::bos()).gecersiz_mi());
        }
    }

    #[test]
    fn gecersiz_magic_reddedilir() {
        assert!(BcextPaket::ac(b"NOTBCEXT....").is_err());
    }

    #[test]
    fn kisa_paket_reddedilir() {
        assert!(BcextPaket::ac(b"BC").is_err());
    }

    #[test]
    fn kur_dosyalari_yazar() {
        let ek = gecici("ek");
        let ayar = gecici("ayar");
        let kurucu = Kurucu::yeni(&ek, &ayar);
        let paket = BcextPaket::ac(&paketle(&ornek_dosyalar())).unwrap();
        let sonuc = kurucu.kur(&paket, &GuvenDeposu::bos()).unwrap();
        assert_eq!(sonuc.kimlik, "biocraft.test.paket");
        assert!(ek.join("biocraft.test.paket/ana.wat").is_file());
        assert!(ek.join("biocraft.test.paket/veri/x.txt").is_file());
        // İmzasız → durum Imzasiz (varsayılan politika izin verir).
        assert_eq!(sonuc.imza_durumu, ImzaDurumu::Imzasiz);
        let _ = std::fs::remove_dir_all(&ek);
        let _ = std::fs::remove_dir_all(&ayar);
    }

    #[test]
    fn cift_kurulum_reddedilir() {
        let ek = gecici("ek");
        let ayar = gecici("ayar");
        let kurucu = Kurucu::yeni(&ek, &ayar);
        let paket = BcextPaket::ac(&paketle(&ornek_dosyalar())).unwrap();
        kurucu.kur(&paket, &GuvenDeposu::bos()).unwrap();
        assert!(kurucu.kur(&paket, &GuvenDeposu::bos()).is_err());
        let _ = std::fs::remove_dir_all(&ek);
        let _ = std::fs::remove_dir_all(&ayar);
    }

    #[test]
    fn imza_politikasi_imzasizi_reddeder() {
        let ek = gecici("ek");
        let ayar = gecici("ayar");
        let kurucu = Kurucu::yeni(&ek, &ayar).politika_ile(ImzaPolitikasi::GuvenilirGerekli);
        let paket = BcextPaket::ac(&paketle(&ornek_dosyalar())).unwrap();
        assert!(kurucu.kur(&paket, &GuvenDeposu::bos()).is_err());
        let _ = std::fs::remove_dir_all(&ek);
        let _ = std::fs::remove_dir_all(&ayar);
    }

    #[test]
    fn guncelleme_geri_alinabilir() {
        let ek = gecici("ek");
        let ayar = gecici("ayar");
        let kurucu = Kurucu::yeni(&ek, &ayar);
        let v1 = BcextPaket::ac(&paketle(&ornek_dosyalar())).unwrap();
        kurucu.kur(&v1, &GuvenDeposu::bos()).unwrap();

        // v2: ana.wat içeriği değişsin.
        let mut d2 = ornek_dosyalar();
        d2[1].1 = b"(module (func))".to_vec();
        let v2 = BcextPaket::ac(&paketle(&d2)).unwrap();
        let sonuc = kurucu.guncelle(&v2, false, &GuvenDeposu::bos()).unwrap();
        assert!(matches!(sonuc, GuncellemeSonucu::Guncellendi { .. }));
        let icerik = std::fs::read(ek.join("biocraft.test.paket/ana.wat")).unwrap();
        assert_eq!(icerik, b"(module (func))");

        // Geri al → v1 içeriği döner.
        kurucu.geri_al("biocraft.test.paket").unwrap();
        let icerik = std::fs::read(ek.join("biocraft.test.paket/ana.wat")).unwrap();
        assert_eq!(icerik, b"(module)");
        let _ = std::fs::remove_dir_all(&ek);
        let _ = std::fs::remove_dir_all(&ayar);
    }

    #[test]
    fn aktif_eklenti_guncellemesi_ertelenir() {
        let ek = gecici("ek");
        let ayar = gecici("ayar");
        let kurucu = Kurucu::yeni(&ek, &ayar);
        let v1 = BcextPaket::ac(&paketle(&ornek_dosyalar())).unwrap();
        kurucu.kur(&v1, &GuvenDeposu::bos()).unwrap();

        let mut d2 = ornek_dosyalar();
        d2[1].1 = b"(module (func))".to_vec();
        let v2 = BcextPaket::ac(&paketle(&d2)).unwrap();

        // aktif=true → ertelenir; aktif dosya değişmez.
        let sonuc = kurucu.guncelle(&v2, true, &GuvenDeposu::bos()).unwrap();
        assert_eq!(
            sonuc,
            GuncellemeSonucu::Ertelendi {
                kimlik: "biocraft.test.paket".into()
            }
        );
        let aktif = std::fs::read(ek.join("biocraft.test.paket/ana.wat")).unwrap();
        assert_eq!(aktif, b"(module)", "aktif sürüm henüz değişmemeli");
        assert!(ek.join("biocraft.test.paket.beklemede").is_dir());

        // Güvenli anda uygula → artık değişir.
        kurucu.bekleyeni_uygula("biocraft.test.paket").unwrap();
        let yeni = std::fs::read(ek.join("biocraft.test.paket/ana.wat")).unwrap();
        assert_eq!(yeni, b"(module (func))");
        let _ = std::fs::remove_dir_all(&ek);
        let _ = std::fs::remove_dir_all(&ayar);
    }

    #[test]
    fn kaldir_ayarlari_korur_veya_siler() {
        let ek = gecici("ek");
        let ayar = gecici("ayar");
        let kurucu = Kurucu::yeni(&ek, &ayar);
        let paket = BcextPaket::ac(&paketle(&ornek_dosyalar())).unwrap();
        kurucu.kur(&paket, &GuvenDeposu::bos()).unwrap();
        // Eklentiye ait bir ayar dosyası simüle et.
        let ayar_klasor = ayar.join("biocraft.test.paket");
        std::fs::create_dir_all(&ayar_klasor).unwrap();
        std::fs::write(ayar_klasor.join("ayar.json"), b"{}").unwrap();

        // Varsayılan: ayarları KORU.
        kurucu.kaldir("biocraft.test.paket", true).unwrap();
        assert!(!ek.join("biocraft.test.paket").exists());
        assert!(ayar_klasor.exists(), "ayar korunmalıydı");

        // Yeniden kur + ayarları SİL.
        kurucu.kur(&paket, &GuvenDeposu::bos()).unwrap();
        kurucu.kaldir("biocraft.test.paket", false).unwrap();
        assert!(!ayar_klasor.exists(), "ayar silinmeliydi");
        let _ = std::fs::remove_dir_all(&ek);
        let _ = std::fs::remove_dir_all(&ayar);
    }

    #[test]
    fn kurulu_olmayani_kaldirma_hata() {
        let ek = gecici("ek");
        let ayar = gecici("ayar");
        let kurucu = Kurucu::yeni(&ek, &ayar);
        assert!(kurucu.kaldir("biocraft.yok.eklenti", true).is_err());
        let _ = std::fs::remove_dir_all(&ek);
        let _ = std::fs::remove_dir_all(&ayar);
    }

    #[test]
    fn dizinden_topla_calisir() {
        let kok = gecici("kaynak");
        std::fs::write(kok.join("a.txt"), b"A").unwrap();
        std::fs::create_dir_all(kok.join("alt")).unwrap();
        std::fs::write(kok.join("alt/b.txt"), b"B").unwrap();
        let dosyalar = dizinden_topla(&kok).unwrap();
        assert_eq!(dosyalar.len(), 2);
        // Belirleyici sıra + ileri eğik çizgi.
        assert_eq!(dosyalar[0].0, "a.txt");
        assert_eq!(dosyalar[1].0, "alt/b.txt");
        let _ = std::fs::remove_dir_all(&kok);
    }
}
