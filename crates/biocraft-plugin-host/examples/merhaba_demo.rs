//! Örnek "merhaba" WASM eklentisinin canlı gösterimi (İP-07 — Gün 13).
//!
//! Çalıştır:  `cargo run -p biocraft-plugin-host --example merhaba_demo`
//!
//! Gösterilen: keşif → uyumluluk → yükleme → çağrı + **capability denetimi**
//! (fs yetkisi yokken dosya erişimi reddedilir; onaylanınca VFS üzerinden okunur).

use biocraft_mem::BellekOrkestratoru;
use biocraft_plugin_host::biocraft_ipc::EklentiYaniti;
use biocraft_plugin_host::biocraft_types::{Capability, Version};
use biocraft_plugin_host::EklentiHost;
use std::path::PathBuf;

fn yaniti_yaz(etiket: &str, yanit: EklentiYaniti) {
    match yanit {
        EklentiYaniti::Basari { donen, gunluk } => {
            println!("  {etiket}: ✅ Başarı (dönen={donen})");
            for satir in gunluk {
                println!("      eklenti günlüğü → {satir}");
            }
        }
        EklentiYaniti::YetkiReddi { yetki, rapor } => {
            println!(
                "  {etiket}: ⛔ Yetki reddedildi ({yetki:?}) — {}",
                rapor.neden
            );
        }
        EklentiYaniti::Hata(rapor) => {
            println!("  {etiket}: ❌ Hata — {}", rapor.ne_oldu);
        }
    }
}

fn main() {
    println!("BioCraft Eklenti Host — Gün 13 demosu\n");

    // 256 MiB bütçeli bellek orkestratörü + çekirdek sürümü 0.1.0.
    let ork = BellekOrkestratoru::yeni(256 * 1024 * 1024);
    let mut host = EklentiHost::yeni(Version::new(0, 1, 0), ork).expect("host kurulamadı");

    // 1) Keşif — bu crate'in ornek/ klasörünü tara.
    let dizin = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bulunan = host.kesfet(&dizin);
    println!("1) Keşif: {} eklenti adayı bulundu", bulunan.len());

    let kesf = bulunan
        .into_iter()
        .find_map(|r| r.ok())
        .expect("ornek eklenti bulunamadı");
    println!(
        "   • {} (\"{}\") sürüm {} — istenen yetkiler: {:?}\n",
        kesf.manifest.kimlik.metni(),
        kesf.manifest.ad,
        kesf.manifest.surum,
        kesf.manifest.istenen_yetkiler,
    );

    // 2) Yetki ONAYLANMADAN yükle → fs verilmez (en az yetki).
    println!("2) Yetki onaylanmadan yükle (en az yetki):");
    let eklenti = host.yukle(&kesf, &[]).expect("yükleme başarısız");
    yaniti_yaz("merhaba()", eklenti.cagir("merhaba"));
    yaniti_yaz("dosya_dene()", eklenti.cagir("dosya_dene"));
    println!();

    // 3) Kullanıcı fs yetkisini ONAYLAR → VFS üzerinden okur.
    println!("3) Kullanıcı 'fs' yetkisini onayladıktan sonra:");
    let eklenti = host
        .yukle(&kesf, &[Capability::Fs])
        .expect("yükleme başarısız");
    yaniti_yaz("dosya_dene()", eklenti.cagir("dosya_dene"));
    println!();

    // 4) AOT önbellek: ikinci yükleme derlemeyi atladı.
    let (isabet, iska) = host.aot_durumu();
    println!("4) AOT önbellek: {isabet} isabet / {iska} ıska (ikinci yüklemeler derlemedi)");
}
