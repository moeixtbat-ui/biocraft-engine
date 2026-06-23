//! `cargo run -p biocraft-core-studio --example merhaba_studio`
//!
//! Çekirdek eklenti BioCraft Studio'nun İP-07 host'uyla uçtan uca yaşam döngüsünü gösterir:
//! keşif → yetki kapısı → aktivasyon (kayıtlar) → capability denetimi → kapat/yeniden yükle.

use biocraft_core_studio as studio;
use biocraft_plugin_host::biocraft_sdk::ui::UiUzantiTuru;
use biocraft_plugin_host::discover::manifest_oku;
use biocraft_plugin_host::{KayitDefteri, YetkiKumesi};

fn main() {
    println!("=== BioCraft Studio — çekirdek eklenti (ÇE-00) ===\n");

    // 1) Keşif — host manifesti okur ve doğrular.
    let kok = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kesfedilen = manifest_oku(&kok).expect("manifest keşfedilmeli");
    let m = kesfedilen.manifest;
    println!("[1] Keşif:");
    println!("    kimlik = {}", m.kimlik.metni());
    println!("    ad     = {}", m.ad);
    println!("    sürüm  = {} (çekirdekten bağımsız — MK-19)", m.surum);
    println!(
        "    ilan edilen yetkiler = {:?}",
        m.istenen_yetkiler
            .iter()
            .map(|c| biocraft_plugin_host::biocraft_sdk::yetenek_metni(*c))
            .collect::<Vec<_>>()
    );

    // 2) Yetki kapısı — birinci-parti çekirdek eklenti: tüm ilan edilenler onaylı (varsayılan kurulu).
    let kapi = YetkiKumesi::ver(&m.istenen_yetkiler, &m.istenen_yetkiler).kapi();
    println!(
        "\n[2] Yetki kapısı (istenen ∩ onaylanan): {} yetki",
        kapi.sayi()
    );

    // 3) Aktivasyon — eklenti kayıtlarını döndürür; host UI kayıt defterine bağlar.
    let akt = studio::aktiflestir(&kapi);
    let mut defter = KayitDefteri::yeni();
    for k in &akt.ui {
        defter.kaydet(studio::KIMLIK, k.clone(), 100).unwrap();
    }
    println!("\n[3] Aktivasyon — kayıtlar:");
    println!(
        "    Activity Bar + Side Panel: {} panel",
        defter.alan(UiUzantiTuru::Panel).len()
    );
    for k in defter.alan(UiUzantiTuru::Panel) {
        println!("      · {} ({})", k.kayit.baslik, k.kayit.kimlik);
    }
    println!("    Komutlar (palette):");
    for k in defter.alan(UiUzantiTuru::Komut) {
        println!("      · {} ({})", k.kayit.baslik, k.kayit.kimlik);
    }
    println!(
        "    Ayar sayfası: {}",
        defter.alan(UiUzantiTuru::Ayar).len()
    );
    println!("\n    {}", studio::merhaba());

    // 4) Capability denetimi — ilan+onaylı db/net ✓; onaylanmamış kapıda net ✗ (MK-13).
    println!("\n[4] Capability denetimi:");
    match studio::db_erisimi_dene(&kapi) {
        Ok(()) => println!("    db  → izinli ✓ (ilan edildi + onaylandı)"),
        Err(e) => println!("    db  → reddedildi: {}", e.ne_oldu),
    }
    match studio::uzak_erisim_dene(&kapi) {
        Ok(()) => println!("    net → izinli ✓ (Gün 35'te ilan edildi + onaylandı)"),
        Err(e) => println!("    net → reddedildi: {}", e.ne_oldu),
    }
    // Kullanıcı net'i onaylamazsa: ilan edilse de çalışmada reddedilir.
    let onaylanan: Vec<_> = m
        .istenen_yetkiler
        .iter()
        .copied()
        .filter(|c| biocraft_plugin_host::biocraft_sdk::yetenek_metni(*c) != "net")
        .collect();
    let kisitli = YetkiKumesi::ver(&m.istenen_yetkiler, &onaylanan).kapi();
    match studio::uzak_erisim_dene(&kisitli) {
        Ok(()) => println!("    net (onaysız kapı) → izinli (beklenmiyordu!)"),
        Err(e) => println!(
            "    net (onaysız kapı) → reddedildi ✓ ({} — onaylanmadı)",
            e.ne_oldu
        ),
    }

    // 5) İzolasyon — kapat (kayıtları temizle) + yeniden yükle (birebir aynı).
    let panel_once = defter.alan(UiUzantiTuru::Panel).len();
    defter.eklenti_kaldir(studio::KIMLIK);
    let panel_kapali = defter.alan(UiUzantiTuru::Panel).len();
    for k in &studio::aktiflestir(&kapi).ui {
        defter.kaydet(studio::KIMLIK, k.clone(), 100).unwrap();
    }
    let panel_sonra = defter.alan(UiUzantiTuru::Panel).len();
    println!("\n[5] İzolasyon (kapat/yeniden yükle):");
    println!("    panel: {panel_once} → kapat → {panel_kapali} → yeniden yükle → {panel_sonra}");
    assert_eq!(
        panel_once, panel_sonra,
        "aktivasyon saf → izolasyon güvenli"
    );

    println!("\n=== Uçtan uca yükleme + kayıt + kapatma çalıştı. ===");
}
