//! Edge-case (sınır durum) entegrasyon testleri — İP-21, Bölüm 0.12.
//!
//! Gerçek dosya I/O yollarını (`olustur`/`ac` + BLAKE3 bütünlük) sınır durumlarda zorlar:
//! boş/eksik proje, **bozuk dosya**, **Unicode/özel ad**, kurcalanmış mühür.  Her hata
//! **standart şemaya** (ne oldu / neden / nasıl çözülür + correlation_id) uymalı ve **panik
//! olmadan** net bildirilmeli (TDA madde 4).  Eşik mantığı (disk %10/%2, ağ geri çekilme,
//! zaman aşımı) L0 `biocraft_types::esikler` birim testlerinde doğrulanır; burada o eşiklerin
//! sınır davranışını da entegrasyon düzeyinde teyit ederiz.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use biocraft_data::biocraft_types::esikler::{DiskDurumu, Gericekilme, GERICEKILME_MAKS_DENEME};
use biocraft_data::biocraft_types::{DataClassification, Version};
use biocraft_data::{ac, olustur, ProjeKurulumGirdisi};

// Format yerleşimi (project/format.rs ile aynı sabitler; test bunlara dışarıdan dokunur).
const MANIFEST_DOSYA: &str = "biocraft.toml";
const META_DIZIN: &str = ".biocraft_meta";
const BUTUNLUK_DOSYA: &str = "butunluk.bcp";

/// Çakışmasız geçici üst klasör (mevcut testlerle aynı kalıp: pid + ns).
fn gecici_konum(etiket: &str) -> PathBuf {
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = std::env::temp_dir().join(format!("bc_edge_{}_{}_{}", etiket, std::process::id(), ns));
    fs::create_dir_all(&p).unwrap();
    p
}

/// Standart varsayılanlarla bir proje girdisi.
fn girdi(ad: &str, konum: &Path) -> ProjeKurulumGirdisi {
    ProjeKurulumGirdisi::yeni(
        ad,
        konum.to_path_buf(),
        "bos",
        DataClassification::Normal,
        Version::new(0, 1, 0),
    )
}

/// Hata raporunun standart şemaya uyduğunu doğrular (İP-16): üç alan dolu + correlation_id.
fn sema_dogru(h: &biocraft_data::biocraft_types::ErrorReport) {
    assert!(!h.ne_oldu.trim().is_empty(), "ne_oldu boş olamaz");
    assert!(!h.neden.trim().is_empty(), "neden boş olamaz");
    assert!(
        !h.nasil_cozulur.trim().is_empty(),
        "nasil_cozulur boş olamaz"
    );
    // correlation_id (W3C trace_id öneki) 8 karakterlik kısa biçim üretebilmeli.
    assert_eq!(h.correlation_id.kisa().len(), 8);
}

#[test]
fn bos_klasor_proje_degil_net_hata() {
    // Tamamen boş bir klasörü açmaya çalışmak → panik değil, standart "proje değil" hatası.
    let konum = gecici_konum("bos");
    let bos = konum.join("bos_klasor");
    fs::create_dir_all(&bos).unwrap();

    let sonuc = ac(&bos);
    assert!(sonuc.is_err(), "boş klasör proje olarak açılmamalı");
    sema_dogru(&sonuc.unwrap_err());

    let _ = fs::remove_dir_all(&konum);
}

#[test]
fn gecerli_proje_olustur_ac_gidip_gelir() {
    let konum = gecici_konum("roundtrip");
    let g = girdi("Örnek Proje", &konum);
    let kurulan = olustur(&g).expect("kurulum başarılı olmalı");

    let acilan = ac(&kurulan.kok).expect("açılış başarılı olmalı");
    assert_eq!(acilan.manifest.kimlik.ad, "Örnek Proje");
    // Sağlam projede bozuk-referans uyarısı olmamalı.
    assert!(acilan.uyarilar.is_empty());

    let _ = fs::remove_dir_all(&konum);
}

#[test]
fn bozuk_manifest_butunluk_hatasi_verir_panik_yok() {
    // Bozuk dosya (0.12): manifest baytları kurcalanır → açılış BLAKE3 uyuşmazlığı yakalar.
    let konum = gecici_konum("bozuk_manifest");
    let g = girdi("Bozulacak", &konum);
    let kurulan = olustur(&g).expect("kurulum başarılı");

    let manifest_yol = kurulan.kok.join(MANIFEST_DOSYA);
    let mut icerik = fs::read(&manifest_yol).unwrap();
    icerik.extend_from_slice(b"\n# kurcalandi\n"); // baytları değiştir
    fs::write(&manifest_yol, &icerik).unwrap();

    let sonuc = ac(&kurulan.kok);
    assert!(sonuc.is_err(), "kurcalanmış manifest reddedilmeli");
    sema_dogru(&sonuc.unwrap_err());

    let _ = fs::remove_dir_all(&konum);
}

#[test]
fn kurcalanmis_butunluk_muhru_reddedilir() {
    // Bütünlük mührünün kendisi bozulursa → açılış net hata (sessiz açma yok).
    let konum = gecici_konum("bozuk_muhur");
    let g = girdi("MuhurBozulacak", &konum);
    let kurulan = olustur(&g).expect("kurulum başarılı");

    let muhur_yol = kurulan.kok.join(META_DIZIN).join(BUTUNLUK_DOSYA);
    fs::write(&muhur_yol, b"tamamen-bozuk-icerik").unwrap();

    let sonuc = ac(&kurulan.kok);
    assert!(sonuc.is_err(), "bozuk bütünlük mührü reddedilmeli");
    sema_dogru(&sonuc.unwrap_err());

    let _ = fs::remove_dir_all(&konum);
}

#[test]
fn unicode_ozel_ad_gidip_gelir() {
    // Unicode/özel ad (0.12): Türkçe + Yunan harfleri + boşluk/altçizgi içeren proje adı.
    let konum = gecici_konum("unicode");
    let ad = "Çığ Örneği_α-β Çalışma";
    let g = girdi(ad, &konum);
    let kurulan = olustur(&g).expect("Unicode adlı proje kurulabilmeli");

    let acilan = ac(&kurulan.kok).expect("Unicode adlı proje açılabilmeli");
    assert_eq!(acilan.manifest.kimlik.ad, ad);

    let _ = fs::remove_dir_all(&konum);
}

#[test]
fn ayni_konuma_iki_kez_kurulum_veri_ezmez() {
    // Var olan dolu klasörün üzerine yazılmamalı (kullanıcı verisi korunur) → standart hata.
    let konum = gecici_konum("cifte");
    let g = girdi("Tekil", &konum);
    olustur(&g).expect("ilk kurulum başarılı");

    let sonuc = olustur(&g);
    assert!(sonuc.is_err(), "dolu hedefin üzerine kurulum reddedilmeli");
    sema_dogru(&sonuc.unwrap_err());

    let _ = fs::remove_dir_all(&konum);
}

#[test]
fn bos_manifest_dosyasi_net_hata() {
    // Boş dosya (0.12): manifest 0 bayta indirgenirse → ayrıştırma/bütünlük net hata, panik yok.
    let konum = gecici_konum("bos_manifest");
    let g = girdi("BosManifest", &konum);
    let kurulan = olustur(&g).expect("kurulum başarılı");

    fs::write(kurulan.kok.join(MANIFEST_DOSYA), b"").unwrap(); // tamamen boşalt

    let sonuc = ac(&kurulan.kok);
    assert!(sonuc.is_err(), "boş manifest reddedilmeli");
    sema_dogru(&sonuc.unwrap_err());

    let _ = fs::remove_dir_all(&konum);
}

// ─── Eşik davranışı (0.12) — entegrasyon düzeyinde teyit ──────────────────────

#[test]
fn disk_esikleri_yazma_kararini_belirler() {
    // %10 altı uyarı (yazılır), %2 altı salt-okunur (yazılmaz).
    assert!(DiskDurumu::siniflandir(15.0).yazilabilir());
    assert!(DiskDurumu::siniflandir(5.0).yazilabilir()); // uyarı ama yazılır
    assert!(!DiskDurumu::siniflandir(1.0).yazilabilir()); // salt-okunur
}

#[test]
fn ag_geri_cekilme_sinirli_ve_sonlu() {
    // Ağ kesintisi: en fazla 5 deneme, gecikme 60s tavanını aşmaz.
    let mut g = Gericekilme::yeni();
    let mut deneme = 0;
    while g.devam_eder_mi() {
        assert!(g.gecikme_saniye() <= 60);
        g.ilerle();
        deneme += 1;
        assert!(deneme <= 100, "sonsuz döngü koruması");
    }
    assert_eq!(deneme, GERICEKILME_MAKS_DENEME as i32);
}
