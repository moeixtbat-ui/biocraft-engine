//! ÇE-09 (Gün 41) — **Akıllı arama önbelleği** (sonuç + indirilen veri).
//!
//! İki amaç (ÇE-09 "Önbellek" + "Çevrimdışı"):
//! 1. **Hız:** Aynı sorgu tekrar yapılınca ağ yerine yerelden döner (geçerlilik süresi içinde).
//! 2. **Çevrimdışı erişim:** Ağ yokken önbellekteki sonuç/veri sunulur (bayat olsa bile —
//!    [`sonuc_oku_zorla`](AramaOnbellegi::sonuc_oku_zorla)).
//!
//! Her girdi **tek dosyada** tutulur: bir satırlık JSON başlık ([`GirdiMeta`]: tarih + **BLAKE3** +
//! boyut + format + kalıcılık) `\n` ardından ham yük.  Okumada BLAKE3 **yeniden hesaplanıp**
//! başlıkla karşılaştırılır → bozuk/yarım yazılmış önbellek **sessizce sunulmaz** (MK-34).  Boyut
//! sınırı aşılınca **en eski** girdiler budanır (ÇE-09 "boyut limiti + temizleme").
//!
//! `data_io::Onbellek` ham bayt-aralığı önbelleğiydi; bu, DB araması için TTL + bütünlük + budama
//! + tipli (sonuç/veri) anahtarlar ekleyen üst katmandır.  `fs` yeteneği gerektirir (MK-13).

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use biocraft_sdk::biocraft_types::{ErrorReport, Timestamp};

use super::framework::{Sayfalama, SonucListesi};
use crate::data_io::Provenans;

/// Önbellek ayarı: azami boyut + sonuçların geçerlilik süresi (TTL).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnbellekAyari {
    /// Toplam önbellek boyutu üst sınırı (bayt); aşılınca en eski girdiler budanır.
    pub azami_bayt: u64,
    /// Arama **sonuçlarının** geçerlilik süresi (sonra ağdan tazelenir).  İndirilen **veri**
    /// (dizi/yapı) accession'a göre değişmez → kalıcı, yalnız boyut budamasına tabidir.
    pub sonuc_gecerlilik: Duration,
}

impl Default for OnbellekAyari {
    fn default() -> Self {
        Self {
            azami_bayt: 256 * 1024 * 1024,                 // 256 MB
            sonuc_gecerlilik: Duration::from_secs(86_400), // 24 saat
        }
    }
}

/// Bir önbellek girdisinin başlığı (yükten önce tek satır JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GirdiMeta {
    /// Yazılma tarihi (UTC) — TTL + budama (en eski) için.
    tarih: Timestamp,
    /// Yükün BLAKE3 özeti (okumada doğrulanır).
    blake3: String,
    /// Yük boyutu (bayt).
    boyut: u64,
    /// İçerik biçimi ("sonuc-json" / "fasta" / "pdb" / "json"…).
    format: String,
    /// Kalıcı mı (TTL uygulanmaz; yalnız boyut budaması) — indirilen veri için `true`.
    kalici: bool,
    /// İndirilen veri için köken/atıf (varsa) — önbellekten sunulsa bile provenance korunur (MK-34).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provenans: Option<Provenans>,
}

/// DB araması için akıllı önbellek (bir dizin = bir önbellek; izole proje alanında tutulur).
pub struct AramaOnbellegi {
    dizin: PathBuf,
    ayar: OnbellekAyari,
}

impl AramaOnbellegi {
    /// Bir önbellek dizini açar (yoksa oluşturur).
    pub fn ac(dizin: impl Into<PathBuf>, ayar: OnbellekAyari) -> Result<Self, ErrorReport> {
        let dizin = dizin.into();
        fs::create_dir_all(&dizin).map_err(|e| io_hatasi(&dizin, &e))?;
        Ok(Self { dizin, ayar })
    }

    /// Varsayılan ayarla açar.
    pub fn varsayilan(dizin: impl Into<PathBuf>) -> Result<Self, ErrorReport> {
        Self::ac(dizin, OnbellekAyari::default())
    }

    // ─── Anahtarlar ──────────────────────────────────────────────────────────────

    /// Bir arama **sonucu** için önbellek anahtarı (kaynak + sorgu + sayfa).
    pub fn sonuc_anahtari(kaynak: &str, sorgu: &str, sayfalama: Sayfalama) -> String {
        format!(
            "sonuc\x1f{kaynak}\x1f{sorgu}\x1f{}+{}",
            sayfalama.ofset, sayfalama.limit
        )
    }

    /// İndirilen bir **kayıt** (dizi/yapı) için önbellek anahtarı (kaynak + kimlik).
    pub fn kayit_anahtari(kaynak: &str, kimlik: &str) -> String {
        format!("veri\x1f{kaynak}\x1f{kimlik}")
    }

    /// Anahtarın disk yolu (BLAKE3 ile adlandırılır → güvenli dosya adı).
    fn yol(&self, anahtar: &str) -> PathBuf {
        let ad = blake3::hash(anahtar.as_bytes()).to_hex().to_string();
        self.dizin.join(ad)
    }

    // ─── Sonuç önbelleği (TTL'li) ──────────────────────────────────────────────────

    /// Bir arama sonucunu önbelleğe yazar (boyut sınırını korumak için ardından budar).
    pub fn sonuc_yaz(&self, anahtar: &str, liste: &SonucListesi) -> Result<(), ErrorReport> {
        let yuk = serde_json::to_vec(liste).map_err(serde_hatasi)?;
        self.ham_yaz(anahtar, &yuk, "sonuc-json", false, None)?;
        self.buda();
        Ok(())
    }

    /// Bir arama sonucunu önbellekten okur — **yalnız geçerlilik süresi içindeyse** (hız yolu).
    /// Süresi geçmiş/bozuk/yok → `None` (ağa gidilir).
    pub fn sonuc_oku(&self, anahtar: &str) -> Option<SonucListesi> {
        let (meta, yuk) = self.ham_oku(anahtar)?;
        if self.suresi_gecti(&meta) {
            return None;
        }
        serde_json::from_slice(&yuk).ok()
    }

    /// Bir arama sonucunu **süre gözetmeksizin** okur (çevrimdışı fallback: bayat olsa da göster).
    pub fn sonuc_oku_zorla(&self, anahtar: &str) -> Option<SonucListesi> {
        let (_meta, yuk) = self.ham_oku(anahtar)?;
        serde_json::from_slice(&yuk).ok()
    }

    // ─── Veri önbelleği (kalıcı; yalnız boyut budaması) ────────────────────────────

    /// İndirilen ham veriyi (dizi/yapı) + (varsa) köken/atıf önbelleğe yazar (kalıcı).
    pub fn veri_yaz(
        &self,
        anahtar: &str,
        format: &str,
        veri: &[u8],
        provenans: Option<&Provenans>,
    ) -> Result<(), ErrorReport> {
        self.ham_yaz(anahtar, veri, format, true, provenans)?;
        self.buda();
        Ok(())
    }

    /// İndirilen ham veriyi önbellekten okur (format + bayt + köken/atıf); yoksa/bozuksa `None`.
    pub fn veri_oku(&self, anahtar: &str) -> Option<(String, Vec<u8>, Option<Provenans>)> {
        let (meta, yuk) = self.ham_oku(anahtar)?;
        Some((meta.format, yuk, meta.provenans))
    }

    // ─── Bakım ─────────────────────────────────────────────────────────────────────

    /// Önbellekteki toplam yük boyutu (bayt).
    pub fn toplam_bayt(&self) -> u64 {
        self.girdileri_topla()
            .iter()
            .map(|(_, _, boyut)| *boyut)
            .sum()
    }

    /// Tüm önbelleği temizler (kullanıcı "önbelleği boşalt").
    pub fn temizle(&self) -> Result<(), ErrorReport> {
        for (yol, _, _) in self.girdileri_topla() {
            let _ = fs::remove_file(&yol);
        }
        Ok(())
    }

    /// Boyut sınırı aşıldıysa **en eski** girdileri sınıra inene dek siler (sessiz; en iyi çaba).
    pub fn buda(&self) {
        let mut girdiler = self.girdileri_topla();
        let mut toplam: u64 = girdiler.iter().map(|(_, _, b)| *b).sum();
        if toplam <= self.ayar.azami_bayt {
            return;
        }
        // En eski → en yeni sırala (tarih artan); en eskiden silmeye başla.
        girdiler.sort_by_key(|a| a.1);
        for (yol, _tarih, boyut) in girdiler {
            if toplam <= self.ayar.azami_bayt {
                break;
            }
            if fs::remove_file(&yol).is_ok() {
                toplam = toplam.saturating_sub(boyut);
            }
        }
    }

    // ─── İç: ham dosya biçimi (meta satırı + yük) ──────────────────────────────────

    fn ham_yaz(
        &self,
        anahtar: &str,
        yuk: &[u8],
        format: &str,
        kalici: bool,
        provenans: Option<&Provenans>,
    ) -> Result<(), ErrorReport> {
        let meta = GirdiMeta {
            tarih: chrono::Utc::now(),
            blake3: blake3::hash(yuk).to_hex().to_string(),
            boyut: yuk.len() as u64,
            format: format.to_string(),
            kalici,
            provenans: provenans.cloned(),
        };
        let meta_satiri = serde_json::to_string(&meta).map_err(serde_hatasi)?;
        let yol = self.yol(anahtar);
        let mut f = File::create(&yol).map_err(|e| io_hatasi(&yol, &e))?;
        f.write_all(meta_satiri.as_bytes())
            .and_then(|_| f.write_all(b"\n"))
            .and_then(|_| f.write_all(yuk))
            .map_err(|e| io_hatasi(&yol, &e))?;
        Ok(())
    }

    /// Bir girdiyi okur + **BLAKE3 doğrular**.  Yok/bozuk/yarım → `None` (bozuk girdiyi siler).
    fn ham_oku(&self, anahtar: &str) -> Option<(GirdiMeta, Vec<u8>)> {
        let yol = self.yol(anahtar);
        let mut ham = Vec::new();
        File::open(&yol).ok()?.read_to_end(&mut ham).ok()?;
        // İlk satır = meta JSON, kalan = yük.
        let nl = ham.iter().position(|&b| b == b'\n')?;
        let meta: GirdiMeta = serde_json::from_slice(&ham[..nl]).ok()?;
        let yuk = ham[nl + 1..].to_vec();
        // Bütünlük: BLAKE3 eşleşmezse bozuk → sil + None (sessiz sunma yok).
        if blake3::hash(&yuk).to_hex().to_string() != meta.blake3 {
            let _ = fs::remove_file(&yol);
            return None;
        }
        Some((meta, yuk))
    }

    fn suresi_gecti(&self, meta: &GirdiMeta) -> bool {
        if meta.kalici {
            return false;
        }
        let yas = chrono::Utc::now().signed_duration_since(meta.tarih);
        match yas.to_std() {
            Ok(d) => d > self.ayar.sonuc_gecerlilik,
            Err(_) => false, // negatif (saat ileri kaymış) → bayat sayma
        }
    }

    /// (yol, tarih, boyut) üçlülerini toplar (budama/boyut için).  Bozuk meta'lı dosyalar atlanır.
    fn girdileri_topla(&self) -> Vec<(PathBuf, Timestamp, u64)> {
        let mut cikti = Vec::new();
        let Ok(okuyucu) = fs::read_dir(&self.dizin) else {
            return cikti;
        };
        for giris in okuyucu.flatten() {
            let yol = giris.path();
            if !yol.is_file() {
                continue;
            }
            // Yalnız meta satırını oku (yükü belleğe almadan).
            if let Ok(f) = File::open(&yol) {
                let mut tampon = Vec::new();
                // Meta satırı (provenance dâhil) küçüktür; ilk parçayı okuyup `\n`'e dek ayrıştır.
                let mut sinirli = std::io::Read::take(f, 16_384);
                if sinirli.read_to_end(&mut tampon).is_ok() {
                    if let Some(nl) = tampon.iter().position(|&b| b == b'\n') {
                        if let Ok(meta) = serde_json::from_slice::<GirdiMeta>(&tampon[..nl]) {
                            cikti.push((yol, meta.tarih, meta.boyut));
                        }
                    }
                }
            }
        }
        cikti
    }
}

fn serde_hatasi(e: serde_json::Error) -> ErrorReport {
    ErrorReport::new(
        "Önbellek serileştirme hatası",
        "arama sonucu/meta JSON'a çevrilemedi",
        "Sorun sürerse önbelleği temizleyin",
    )
    .with_teknik_detay(e.to_string())
}

fn io_hatasi(yol: &Path, e: &std::io::Error) -> ErrorReport {
    ErrorReport::new(
        "Önbellek dosyası erişilemedi",
        format!("'{}' erişiminde G/Ç hatası", yol.display()),
        "Yolu, izinleri ve disk alanını kontrol edin",
    )
    .with_teknik_detay(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::super::framework::{AramaSonucu, KayitTuru, SayfaBilgisi};
    use super::*;

    fn gecici_dizin(ad: &str) -> PathBuf {
        std::env::temp_dir().join(format!("biocraft_dbob_{}_{ad}", std::process::id()))
    }

    fn ornek_liste() -> SonucListesi {
        SonucListesi {
            sonuclar: vec![
                AramaSonucu::yeni("PDB", "1TUP", "p53 core domain", KayitTuru::Yapi),
                AramaSonucu::yeni(
                    "UniProt",
                    "P04637",
                    "Cellular tumor antigen p53",
                    KayitTuru::Protein,
                ),
            ],
            sayfa: SayfaBilgisi {
                toplam: 2,
                ofset: 0,
                limit: 20,
            },
        }
    }

    #[test]
    fn sonuc_yaz_oku_round_trip() {
        let dizin = gecici_dizin("a");
        let ob = AramaOnbellegi::varsayilan(&dizin).unwrap();
        let anahtar = AramaOnbellegi::sonuc_anahtari("PDB", "p53", Sayfalama::ilk(20));
        assert!(ob.sonuc_oku(&anahtar).is_none());
        ob.sonuc_yaz(&anahtar, &ornek_liste()).unwrap();
        let geri = ob.sonuc_oku(&anahtar).unwrap();
        assert_eq!(geri, ornek_liste());
        let _ = fs::remove_dir_all(&dizin);
    }

    #[test]
    fn ttl_gecince_sonuc_dondurmez_ama_zorla_doner() {
        let dizin = gecici_dizin("b");
        // TTL sıfır → her sonuç anında bayat sayılır.
        let ob = AramaOnbellegi::ac(
            &dizin,
            OnbellekAyari {
                sonuc_gecerlilik: Duration::ZERO,
                ..Default::default()
            },
        )
        .unwrap();
        let anahtar = AramaOnbellegi::sonuc_anahtari("PDB", "p53", Sayfalama::ilk(20));
        ob.sonuc_yaz(&anahtar, &ornek_liste()).unwrap();
        // TTL geçti → hız yolu None.
        assert!(ob.sonuc_oku(&anahtar).is_none());
        // Ama çevrimdışı fallback yine de döner (bayat).
        assert_eq!(ob.sonuc_oku_zorla(&anahtar).unwrap(), ornek_liste());
        let _ = fs::remove_dir_all(&dizin);
    }

    #[test]
    fn veri_kalici_ttl_uygulanmaz() {
        let dizin = gecici_dizin("c");
        let ob = AramaOnbellegi::ac(
            &dizin,
            OnbellekAyari {
                sonuc_gecerlilik: Duration::ZERO,
                ..Default::default()
            },
        )
        .unwrap();
        let anahtar = AramaOnbellegi::kayit_anahtari("NCBI nucleotide", "7157");
        ob.veri_yaz(&anahtar, "fasta", b">NM_000546\nACGT\n", None)
            .unwrap();
        // Kalıcı → TTL sıfır olsa bile döner.
        let (format, veri, _prov) = ob.veri_oku(&anahtar).unwrap();
        assert_eq!(format, "fasta");
        assert_eq!(veri, b">NM_000546\nACGT\n");
        let _ = fs::remove_dir_all(&dizin);
    }

    #[test]
    fn bozuk_yuk_sessizce_sunulmaz() {
        let dizin = gecici_dizin("d");
        let ob = AramaOnbellegi::varsayilan(&dizin).unwrap();
        let anahtar = AramaOnbellegi::kayit_anahtari("PDB", "1TUP");
        ob.veri_yaz(&anahtar, "pdb", b"ATOM ...", None).unwrap();
        // Dosyanın yükünü boz (son baytı değiştir).
        let yol = ob.yol(&anahtar);
        let mut ham = fs::read(&yol).unwrap();
        *ham.last_mut().unwrap() ^= 0xFF;
        fs::write(&yol, &ham).unwrap();
        // BLAKE3 uyuşmaz → None (ve bozuk dosya silinir).
        assert!(ob.veri_oku(&anahtar).is_none());
        assert!(!yol.exists());
        let _ = fs::remove_dir_all(&dizin);
    }

    #[test]
    fn boyut_sinirini_asinca_budanir() {
        let dizin = gecici_dizin("e");
        // Çok küçük sınır → her yazımdan sonra budama eski girdileri atar.
        let ob = AramaOnbellegi::ac(
            &dizin,
            OnbellekAyari {
                azami_bayt: 300,
                sonuc_gecerlilik: Duration::from_secs(3600),
            },
        )
        .unwrap();
        for i in 0..10 {
            let anahtar = AramaOnbellegi::kayit_anahtari("PDB", &format!("ID{i}"));
            ob.veri_yaz(&anahtar, "pdb", &[b'A'; 100], None).unwrap();
        }
        // Toplam boyut sınırın üstüne çıkmamalı (budama çalıştı).
        assert!(ob.toplam_bayt() <= 300 + 100); // tolerans: son yazım + budama
        let _ = fs::remove_dir_all(&dizin);
    }

    #[test]
    fn temizle_hepsini_siler() {
        let dizin = gecici_dizin("f");
        let ob = AramaOnbellegi::varsayilan(&dizin).unwrap();
        ob.veri_yaz(
            &AramaOnbellegi::kayit_anahtari("PDB", "1TUP"),
            "pdb",
            b"x",
            None,
        )
        .unwrap();
        assert!(ob.toplam_bayt() > 0);
        ob.temizle().unwrap();
        assert_eq!(ob.toplam_bayt(), 0);
        let _ = fs::remove_dir_all(&dizin);
    }
}
