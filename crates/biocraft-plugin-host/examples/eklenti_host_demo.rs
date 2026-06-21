//! Eklenti Host'u **2. kısım** canlı gösterimi (İP-07 — Gün 14).
//!
//! Çalıştır:  `cargo run -p biocraft-plugin-host --example eklenti_host_demo`
//!
//! Gösterilenler:
//! 1. UI uzantı kaydı + **çakışma yönetimi** (aynı alanı iki eklenti ister → sessiz bozmaz)
//! 2. **İmza/bütünlük** + "resmi/doğrulanmış" rozeti (MK-16)
//! 3. **Çökme izolasyonu** + yeniden başlatma + çökme döngüsü (MK-15)
//! 4. **`.bcext` çevrimdışı kurulum** + güncelleme (geri alınabilir) + kaldırma
//! 5. **Güvenli mod** — 3. parti eklentileri kapatır
//! 6. **Python out-of-process** köprüsü (MK-02; Python kuruluysa)

use biocraft_mem::BellekOrkestratoru;
use biocraft_plugin_host::biocraft_ipc::EklentiYaniti;
use biocraft_plugin_host::biocraft_sdk::ui::{UiKayit, UiUzantiTuru};
use biocraft_plugin_host::biocraft_types::Version;
use biocraft_plugin_host::install::{self, BcextPaket};
use biocraft_plugin_host::{
    safe_mode, EklentiHost, GuvenDeposu, GuvenliMod, GuvenliModSebep, ImzaDurumu,
    IzolasyonYoneticisi, KayitDefteri, Kurucu, SigningKey,
};
use std::path::PathBuf;

fn baslik(no: u32, ad: &str) {
    println!("\n=== {no}) {ad} ===");
}

fn main() {
    println!("BioCraft Eklenti Host — Gün 14 demosu (İP-07 2. kısım)");

    // ── 1) UI uzantı kaydı + çakışma yönetimi ───────────────────────────────
    baslik(1, "UI uzantı kaydı + çakışma yönetimi");
    let mut defter = KayitDefteri::yeni();
    defter
        .kaydet(
            "biocraft.acme.analiz",
            UiKayit::yeni("ana", "Acme Sonuç Paneli", UiUzantiTuru::Panel),
            5,
        )
        .unwrap();
    defter
        .kaydet(
            "biocraft.foo.gorsel",
            UiKayit::yeni("ana", "Foo Görselleştirme", UiUzantiTuru::Panel),
            10, // daha yüksek öncelik
        )
        .unwrap();
    defter
        .kaydet(
            "biocraft.acme.analiz",
            UiKayit::yeni("calistir", "Analizi Çalıştır", UiUzantiTuru::Komut),
            0,
        )
        .unwrap();
    println!("  Panel alanı (öncelik sırasıyla):");
    for u in defter.alan(UiUzantiTuru::Panel) {
        println!("    • {} ({})", u.kayit.baslik, u.sahip);
    }
    println!("  Çakışmalar (kullanıcıya bildirilir, hiçbiri kaybolmaz):");
    for c in defter.cakismalar() {
        println!(
            "    ⚠ '{}' yuvasını {} eklenti paylaşıyor: {:?}",
            c.kimlik,
            c.sahipler.len(),
            c.sahipler
        );
    }

    // ── 2) İmza/bütünlük + rozet ────────────────────────────────────────────
    baslik(2, "İmza / bütünlük + rozet (MK-16)");
    let resmi_anahtar = SigningKey::from_bytes(&[42u8; 32]);
    let mut depo = GuvenDeposu::bos();
    depo.resmi_ekle("BioCraft", resmi_anahtar.verifying_key());

    let dosyalar = install::dizinden_topla(&ornek_python_dizin()).unwrap();
    // İmzasız paket → uyarı rozeti.
    let imzasiz = BcextPaket::ac(&install::paketle(&dosyalar)).unwrap();
    rozet_yaz("İmzasız paket", &imzasiz.imza_durumu(&depo));
    // Resmi anahtarla imzalı → "Resmi" rozeti.
    let imzali = BcextPaket::ac(&install::paketle_imzali(&dosyalar, &resmi_anahtar)).unwrap();
    rozet_yaz("Resmi imzalı paket", &imzali.imza_durumu(&depo));
    // Bilinmeyen anahtarla imzalı → "İmzalı (bilinmeyen)".
    let yabanci = SigningKey::from_bytes(&[7u8; 32]);
    let yabanci_paket = BcextPaket::ac(&install::paketle_imzali(&dosyalar, &yabanci)).unwrap();
    rozet_yaz(
        "Bilinmeyen yayıncı imzalı",
        &yabanci_paket.imza_durumu(&depo),
    );

    // ── 3) Çökme izolasyonu ─────────────────────────────────────────────────
    baslik(3, "Çökme izolasyonu + yeniden başlatma (MK-15)");
    let mut izol = IzolasyonYoneticisi::yeni();
    izol.kaydet("biocraft.kotu.eklenti");
    let karar = izol.cokme_bildir("biocraft.kotu.eklenti", "trap: bellek sınırı");
    println!(
        "  İlk çökme → yalıtıldı={}, yeniden başlat sun={}",
        karar.yalitildi, karar.yeniden_baslat_sun
    );
    println!("    Kullanıcıya: {}", karar.bildirim.ne_oldu);
    izol.yeniden_baslat("biocraft.kotu.eklenti").unwrap();
    println!("  Yeniden başlatıldı; çekirdek hiç düşmedi.");
    // Çökme döngüsü → kalıcı hata.
    let son = std::iter::repeat_with(|| izol.cokme_bildir("biocraft.kotu.eklenti", "trap"))
        .take(3)
        .last()
        .unwrap();
    println!(
        "  Tekrarlı çökme → yeniden başlat sun={} (artık önerilmiyor); kalıcı hatalı eklenti sayısı={}",
        son.yeniden_baslat_sun,
        izol.kalici_hatali_sayisi()
    );

    // ── 4) .bcext çevrimdışı kurulum + güncelleme + kaldırma ─────────────────
    baslik(4, ".bcext çevrimdışı kurulum / güncelleme / kaldırma");
    let kok = gecici_dizin();
    let kurucu = Kurucu::yeni(kok.join("eklentiler"), kok.join("ayarlar"));
    let sonuc = kurucu.kur(&imzali, &depo).unwrap();
    println!(
        "  Kuruldu: {} → {} (rozet: {})",
        sonuc.kimlik,
        sonuc.hedef.display(),
        sonuc.imza_durumu.rozet().etiket()
    );
    // Güncelle (geri alınabilir).
    match kurucu.guncelle(&imzali, false, &depo).unwrap() {
        biocraft_plugin_host::GuncellemeSonucu::Guncellendi {
            geri_alinabilir, ..
        } => {
            println!("  Güncellendi (geri alınabilir={geri_alinabilir})");
        }
        diger => println!("  {diger:?}"),
    }
    kurucu.kaldir(&sonuc.kimlik, true).unwrap();
    println!("  Kaldırıldı (ayarlar korundu — varsayılan).");
    let _ = std::fs::remove_dir_all(&kok);

    // ── 5) Güvenli mod ──────────────────────────────────────────────────────
    baslik(5, "Güvenli mod — 3. parti eklentiler kapalı");
    let gm = GuvenliMod::acik(GuvenliModSebep::KullaniciSecti);
    let resmi = ImzaDurumu::Resmi {
        yayinci: "BioCraft".into(),
    };
    let imzasiz_durum = ImzaDurumu::Imzasiz;
    let adaylar = vec![
        ("biocraft.studio.ana", &resmi),
        ("biocraft.acme.analiz", &imzasiz_durum),
    ];
    for (kimlik, karar) in safe_mode::filtrele(gm, adaylar) {
        let durum = if karar.yuklenebilir() {
            "YÜKLENDİ"
        } else {
            "atlandı (3. parti)"
        };
        println!("  {kimlik}: {durum}");
    }

    // ── 6) Python out-of-process köprüsü ────────────────────────────────────
    baslik(6, "Python out-of-process köprüsü (MK-02)");
    let ork = BellekOrkestratoru::yeni(256 * 1024 * 1024);
    let mut host = EklentiHost::yeni(Version::new(0, 1, 0), ork).unwrap();
    let bulunan = host.kesfet(&PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let py = bulunan
        .into_iter()
        .filter_map(|r| r.ok())
        .find(|k| k.manifest.katman == biocraft_plugin_host::biocraft_sdk::EklentiKatmani::Python);
    match py {
        Some(kesf) => match host.yukle(&kesf, &[]) {
            Ok(eklenti) => {
                println!("  Yüklendi: {}", eklenti.manifest.kimlik.metni());
                match eklenti.cagir("merhaba") {
                    EklentiYaniti::Basari { donen, gunluk } => {
                        println!("    merhaba() → {donen}");
                        for s in gunluk {
                            println!("      eklenti günlüğü → {s}");
                        }
                    }
                    diger => println!("    {diger:?}"),
                }
            }
            Err(e) => println!("  Yüklenemedi: {} — {}", e.ne_oldu, e.neden),
        },
        None => println!("  Python örnek eklentisi bulunamadı."),
    }

    println!("\nDemo tamamlandı (çekirdek hiç panik yapmadı).");
}

fn rozet_yaz(etiket: &str, durum: &ImzaDurumu) {
    let r = durum.rozet();
    let isaret = if r.uyari_mi() { "⚠" } else { "✓" };
    println!("  {isaret} {etiket}: {}", r.etiket());
}

fn ornek_python_dizin() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ornek-python")
}

fn gecici_dizin() -> PathBuf {
    let p = std::env::temp_dir().join(format!("biocraft_demo_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&p);
    p
}
