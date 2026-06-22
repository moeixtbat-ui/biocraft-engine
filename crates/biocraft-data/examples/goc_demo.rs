//! Göç ve sürüm uyumu uçtan-uca demo (İP-19, Gün 30 — MK-59/MK-14).
//!
//! Çalıştır: `cargo run -p biocraft-data --example goc_demo`
//!
//! Gösterir (5 bölüm):
//!   1. Güncel proje → doğrudan açılır (göç gerekmez).
//!   2. Eski (1.0.0) proje → 1.1.0'a **otomatik göç** + göç öncesi **yedek** + geçmiş kaydı.
//!   3. **Kırıcı** göç → `KiriciIcinOnay` ile **onay bekler** (uygulanmaz).
//!   4. Daha **yeni** formatlı proje → eski uygulamada **salt-okunur + uyarı** (bozma yok).
//!   5. Başarısız göç → **yedekten otomatik geri yükleme** (proje bozulmaz).

// Göç dönüşümleri `Result<(), ErrorReport>` döndürür; `ErrorReport` (İP-16) bilinçli zengindir →
// `Err` varyantı büyük.  Lib'teki crate-seviye `allow` örneklere yansımaz, burada tekrar edilir.
#![allow(clippy::result_large_err)]

use std::fs;
use std::path::{Path, PathBuf};

use biocraft_data::biocraft_types::{DataClassification, ErrorReport, Version};
use biocraft_data::migrate::{ac_ve_goc_ile, Goc, GocKayit, GocSonucu, OnayPolitikasi};
use biocraft_data::project::{self, ProjeKurulumGirdisi};

fn v(a: u32, b: u32, c: u32) -> Version {
    Version::new(a, b, c)
}

/// 1.0.0 → 1.1.0: sınıflandırma etiketlerine "format-1.1" ekler (kırıcı değil).
fn goc_1_0_to_1_1(deger: &mut toml::Value) -> Result<(), ErrorReport> {
    if let Some(s) = deger
        .get_mut("siniflandirma")
        .and_then(|s| s.as_table_mut())
    {
        let etiketler = s
            .entry("etiketler".to_string())
            .or_insert_with(|| toml::Value::Array(Vec::new()));
        if let Some(arr) = etiketler.as_array_mut() {
            arr.push(toml::Value::String("format-1.1".to_string()));
        }
    }
    Ok(())
}

/// 1.0.0 → 2.0.0: KIRICI şema değişikliği (uyumluluk etiketi).
fn goc_kirici(deger: &mut toml::Value) -> Result<(), ErrorReport> {
    if let Some(s) = deger
        .get_mut("siniflandirma")
        .and_then(|s| s.as_table_mut())
    {
        let u = s
            .entry("uyumluluk".to_string())
            .or_insert_with(|| toml::Value::Array(Vec::new()));
        if let Some(arr) = u.as_array_mut() {
            arr.push(toml::Value::String("v2-semasi".to_string()));
        }
    }
    Ok(())
}

/// BOZUK göç: zorunlu alanı siler → strict doğrulama başarısız olur (rollback tetikler).
fn goc_bozuk(deger: &mut toml::Value) -> Result<(), ErrorReport> {
    if let Some(s) = deger
        .get_mut("siniflandirma")
        .and_then(|s| s.as_table_mut())
    {
        s.remove("sinif");
    }
    Ok(())
}

fn ornek_proje(taban: &Path, ad: &str) -> PathBuf {
    let mut girdi = ProjeKurulumGirdisi::yeni(
        ad,
        taban,
        "genomik",
        DataClassification::Sentetik,
        v(0, 1, 0),
    );
    girdi.etiketler = vec!["başlangıç".into()];
    project::olustur(&girdi).expect("kurulum").kok
}

fn surum_yazdir(kok: &Path) {
    let metin = fs::read_to_string(kok.join("biocraft.toml")).unwrap();
    let s = biocraft_data::migrate::manifest_surumu_oku(&metin).unwrap();
    println!("    disk format sürümü = {s}");
}

fn main() {
    let taban = std::env::temp_dir().join(format!("biocraft_goc_demo_{}", std::process::id()));
    let _ = fs::remove_dir_all(&taban);
    fs::create_dir_all(&taban).unwrap();
    println!("Demo çalışma klasörü: {}\n", taban.display());

    let kayit = GocKayit::yeni().ekle(Goc {
        kaynak: v(1, 0, 0),
        hedef: v(1, 1, 0),
        aciklama: "Etiket şeması güncellendi",
        kirici: false,
        donustur: goc_1_0_to_1_1,
    });

    // ── 1) Güncel proje ─────────────────────────────────────────────────────
    println!("[1] Güncel proje (format 1.0.0, hedef 1.0.0)");
    let k1 = ornek_proje(&taban, "Guncel");
    let s1 = ac_ve_goc_ile(&k1, &kayit, &v(1, 0, 0), OnayPolitikasi::Otomatik).unwrap();
    println!("    sonuç: {}\n", ozet(&s1));

    // ── 2) Eski proje otomatik göç ──────────────────────────────────────────
    println!("[2] Eski proje (1.0.0) → otomatik göç (hedef 1.1.0)");
    let k2 = ornek_proje(&taban, "EskiProje");
    surum_yazdir(&k2);
    let s2 = ac_ve_goc_ile(&k2, &kayit, &v(1, 1, 0), OnayPolitikasi::Otomatik).unwrap();
    if let GocSonucu::GocEdildi {
        proje,
        uygulanan,
        yedek_dizini,
    } = &s2
    {
        println!(
            "    GÖÇ EDİLDİ → yeni sürüm = {}",
            proje.manifest.kimlik.format_surumu
        );
        println!("    uygulanan göç sayısı = {}", uygulanan.len());
        println!(
            "    göç geçmişi (manifest) = {} kayıt",
            proje.manifest.goc.len()
        );
        println!(
            "    eklenen etiket = {:?}",
            proje.manifest.siniflandirma.etiketler
        );
        println!("    göç öncesi yedek = {}", yedek_dizini.display());
    }
    surum_yazdir(&k2);
    println!();

    // ── 3) Kırıcı göç onay bekler ───────────────────────────────────────────
    println!("[3] Kırıcı göç (1.0.0 → 2.0.0) — KiriciIcinOnay");
    let k3 = ornek_proje(&taban, "Kirici");
    let kayit_kirici = GocKayit::yeni().ekle(Goc {
        kaynak: v(1, 0, 0),
        hedef: v(2, 0, 0),
        aciklama: "v2 şeması (kırıcı)",
        kirici: true,
        donustur: goc_kirici,
    });
    let s3 = ac_ve_goc_ile(
        &k3,
        &kayit_kirici,
        &v(2, 0, 0),
        OnayPolitikasi::KiriciIcinOnay,
    )
    .unwrap();
    if let GocSonucu::OnayGerekli { plan } = &s3 {
        println!("    ONAY GEREKLİ (kırıcı = {})", plan.kirici_var);
        for a in &plan.adimlar {
            println!(
                "      • {} → {} : {} [kırıcı={}]",
                a.kaynak, a.hedef, a.aciklama, a.kirici
            );
        }
        println!("    (kullanıcı onaylarsa OnayPolitikasi::Otomatik ile yeniden çağrılır)");
    }
    surum_yazdir(&k3);
    println!();

    // ── 4) Daha yeni format → salt-okunur ───────────────────────────────────
    println!("[4] Daha yeni formatlı proje, eski uygulamada açılıyor");
    let k4 = ornek_proje(&taban, "Gelecek");
    // Önce 2.0.0'a ileri göç et (sanki yeni sürümle yapılmış gibi).
    ac_ve_goc_ile(&k4, &kayit_kirici, &v(2, 0, 0), OnayPolitikasi::Otomatik).unwrap();
    surum_yazdir(&k4);
    // Şimdi ESKİ uygulama (hedef 1.0.0) ile aç.
    let s4 = ac_ve_goc_ile(
        &k4,
        &GocKayit::yeni(),
        &v(1, 0, 0),
        OnayPolitikasi::Otomatik,
    )
    .unwrap();
    if let GocSonucu::SaltOkunur { proje, uyari } = &s4 {
        println!(
            "    SALT-OKUNUR açıldı (dosya {} > uygulama {})",
            proje.dosya_surumu, proje.uygulama_surumu
        );
        println!("    uyarı: {}", uyari.ne_oldu);
    }
    surum_yazdir(&k4); // değişmemiş olmalı
    println!();

    // ── 5) Başarısız göç → yedekten geri yükleme ────────────────────────────
    println!("[5] Başarısız göç → yedekten otomatik geri yükleme");
    let k5 = ornek_proje(&taban, "Rollback");
    let kayit_bozuk = GocKayit::yeni().ekle(Goc {
        kaynak: v(1, 0, 0),
        hedef: v(1, 1, 0),
        aciklama: "bozuk (zorunlu alan siler)",
        kirici: false,
        donustur: goc_bozuk,
    });
    match ac_ve_goc_ile(&k5, &kayit_bozuk, &v(1, 1, 0), OnayPolitikasi::Otomatik) {
        Ok(_) => println!("    BEKLENMEDİK: göç başarılı oldu"),
        Err(e) => println!("    HATA (beklenen): {}", e.ne_oldu),
    }
    print!("    geri yükleme sonrası: ");
    surum_yazdir(&k5); // hâlâ 1.0.0 — proje bozulmadı
    let acilan = project::ac(&k5).unwrap();
    println!(
        "    proje hâlâ açılabilir, göç geçmişi = {} kayıt",
        acilan.manifest.goc.len()
    );
    println!();

    println!("Bitti. (Tüm değişiklikler geçici klasörde — kalıcı etki yok.)");
    let _ = fs::remove_dir_all(&taban);
}

fn ozet(s: &GocSonucu) -> &'static str {
    match s {
        GocSonucu::Guncel(_) => "Güncel (göç gerekmedi)",
        GocSonucu::GocEdildi { .. } => "Göç edildi",
        GocSonucu::SaltOkunur { .. } => "Salt-okunur (daha yeni format)",
        GocSonucu::OnayGerekli { .. } => "Onay gerekli",
    }
}
