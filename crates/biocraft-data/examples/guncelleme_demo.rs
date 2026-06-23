//! İmzalı + delta + atomik + geri-alınabilir auto-update uçtan-uca demo (İP-20, Gün 31 — MK-56).
//!
//! Çalıştır: `cargo run -p biocraft-data --example guncelleme_demo`
//!
//! Gösterir (6 bölüm):
//!   1. İlk kurulum (1.0.0) → `current/` yazılır.
//!   2. **Tam paket** ile imzalı güncelleme (1.1.0) → atomik takas; `previous` = 1.0.0.
//!   3. **Delta** ile güncelleme (1.2.0) → yalnız değişen parça birleştirilir (kazanç gösterilir).
//!   4. **Sahte imza** → reddedilir; aktif kurulum (current) **dokunulmaz**.
//!   5. **Geri alma (rollback)** → bir önceki sürüme atomik dönüş (downgrade güvenlik ağı).
//!   6. **Yarıda kalan güncelleme** simülasyonu → eski sürüm korunur.

#![allow(clippy::result_large_err)]

use biocraft_data::update::{
    self, demo_imzala, guncellemeyi_uygula, AtomikKurulum, GuncellemePaketi, SurumKanali,
};

fn baslik(n: u32, s: &str) {
    println!("\n━━━ {n}. {s} ━━━");
}

fn main() {
    // Demo imzalama tohum anahtarı (üretim anahtarı DEĞİL — gerçek anahtar insan-eli/CI gizlisi).
    let tohum = [11u8; 32];

    // İzole bir geçici kurulum kökü.
    let kok = std::env::temp_dir().join(format!("biocraft-update-demo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&kok);
    let kurulum = AtomikKurulum::yeni(&kok);
    println!("Kurulum kökü: {}", kok.display());

    // ── 1) İlk kurulum ───────────────────────────────────────────────────────
    baslik(1, "İlk kurulum (1.0.0)");
    let paket_v1 = b"BioCraft Engine 1.0.0 paket govdesi ".repeat(64); // ~2.3 KB ortak gövde
    kurulum
        .uygula("1.0.0", &[("paket.bin", paket_v1.clone())])
        .unwrap();
    println!("  Aktif sürüm: {:?}", kurulum.gecerli_surum());
    println!("  Geri alınabilir mi? {}", kurulum.geri_al_mumkun_mu());

    // ── 2) Tam paketle güncelleme ─────────────────────────────────────────────
    baslik(2, "Tam paketle imzalı güncelleme (1.1.0)");
    let mut paket_v2 = paket_v1.clone();
    paket_v2.extend_from_slice(b"<< 1.1.0 ekleri >>");
    let (b2, imza2, vk) = demo_imzala(
        &paket_v2,
        "1.1.0",
        SurumKanali::Kararli,
        "- Yeni varyant paneli\n- Hız iyileştirmeleri",
        tohum,
    );
    let rapor = guncellemeyi_uygula(
        &kurulum,
        GuncellemePaketi::Tam(paket_v2.clone()),
        &b2,
        &imza2,
        &vk,
        SurumKanali::Kararli,
    )
    .unwrap();
    println!(
        "  {} → {} ({:?}); aktif={:?}, önceki={:?}",
        rapor.eski_surum,
        rapor.yeni_surum,
        rapor.yon,
        kurulum.gecerli_surum(),
        kurulum.onceki_surum()
    );

    // ── 3) Delta ile güncelleme ───────────────────────────────────────────────
    baslik(3, "Delta ile güncelleme (1.2.0 — yalnız değişen parça)");
    let mut paket_v3 = paket_v2.clone();
    paket_v3.extend_from_slice(b"<< 1.2.0 cok kucuk kuyruk >>");
    let yama = update::delta::uret(&paket_v2, &paket_v3);
    println!(
        "  Yeni paket: {} bayt; delta taşınan: {} bayt; yaklaşık indirme: {} bayt",
        paket_v3.len(),
        yama.tasinan_bayt(),
        yama.yaklasik_indirme_boyutu()
    );
    let (b3, imza3, _) = demo_imzala(
        &paket_v3,
        "1.2.0",
        SurumKanali::Kararli,
        "- Küçük düzeltmeler",
        tohum,
    );
    let rapor = guncellemeyi_uygula(
        &kurulum,
        GuncellemePaketi::Delta(yama),
        &b3,
        &imza3,
        &vk,
        SurumKanali::Kararli,
    )
    .unwrap();
    println!(
        "  {} → {} (delta={}); birebir yeniden üretildi: {}",
        rapor.eski_surum,
        rapor.yeni_surum,
        rapor.delta_mi,
        kurulum.gecerli_dosya("paket.bin").as_deref() == Some(&paket_v3[..])
    );

    // ── 4) Sahte imza reddi ───────────────────────────────────────────────────
    baslik(4, "Sahte imza → reddedilir (current korunur)");
    let kotu = b"kotu amacli paket".to_vec();
    let (b4, imza4, sahte_vk) = demo_imzala(
        &kotu,
        "9.9.9",
        SurumKanali::Kararli,
        "ele gecirildi",
        [99u8; 32],
    );
    let sonuc = guncellemeyi_uygula(
        &kurulum,
        GuncellemePaketi::Tam(kotu),
        &b4,
        &imza4,
        &vk, // GERÇEK resmi anahtar — saldırganın imzası tutmaz
        SurumKanali::Kararli,
    );
    println!(
        "  Saldırganın anahtarı resmi anahtarla aynı mı? {}",
        sahte_vk == vk
    );
    match sonuc {
        Err(e) => println!(
            "  Reddedildi: {} — aktif sürüm hâlâ {:?}",
            e.ne_oldu,
            kurulum.gecerli_surum()
        ),
        Ok(_) => println!("  HATA: sahte güncelleme kabul edildi (olmamalı!)"),
    }

    // ── 5) Geri alma (rollback / downgrade) ───────────────────────────────────
    baslik(5, "Geri alma → bir önceki sürüme atomik dönüş");
    let geri = kurulum.geri_al().unwrap();
    println!(
        "  {:?} → {:?} (aktif paket 1.1.0'a döndü mü? {})",
        geri.onceki_aktif,
        geri.yeni_aktif,
        kurulum.gecerli_dosya("paket.bin").as_deref() == Some(&paket_v2[..])
    );

    // ── 6) Yarıda kalan güncelleme: bozuk paket → eski sürüm korunur ──────────
    baslik(6, "Bozuk paket (yarım indirme) → eski sürüm korunur");
    let mut bozuk = paket_v2.clone();
    bozuk.truncate(bozuk.len() / 2); // yarım indirme
                                     // Bildirim tam paketin özetini taşır ama baytlar yarım → bütünlük tutmaz.
    let (b6, imza6, _) = demo_imzala(
        &paket_v3,
        "1.3.0",
        SurumKanali::Kararli,
        "tam paket bekleniyordu",
        tohum,
    );
    let sonuc = guncellemeyi_uygula(
        &kurulum,
        GuncellemePaketi::Tam(bozuk),
        &b6,
        &imza6,
        &vk,
        SurumKanali::Kararli,
    );
    match sonuc {
        Err(e) => println!(
            "  Reddedildi: {} — aktif sürüm hâlâ {:?}",
            e.ne_oldu,
            kurulum.gecerli_surum()
        ),
        Ok(_) => println!("  HATA: bozuk paket kabul edildi (olmamalı!)"),
    }

    // Temizlik.
    let _ = std::fs::remove_dir_all(&kok);
    println!("\n✓ Demo tamamlandı (geçici kurulum temizlendi).");
}
