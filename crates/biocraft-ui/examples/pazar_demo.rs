//! İP-18 (Gün 29) demosu — **Bilim Pazarı + doğrulanmış haber akışı + çok-AI çapraz kontrol**
//! (arayüzsüz; gerçek pencere açmaz).
//!
//! Çalıştırma:
//! ```text
//! cargo run -p biocraft-ui --example pazar_demo
//! ```
//! Kabul kriterlerini gösterir: (1) küratörlü içerik + dürüst doğrulama rozetleri; (2) arama/
//! kategori/sıralama; (3) **gerçek** host kurulumu (imza/bütünlük denetimi — İP-07/MK-16);
//! (4) çok-AI çapraz kontrol "garanti değil" (MK-47); (5) tüm sekme/tema/dilde headless çizim.

use std::sync::Arc;
use std::time::{Duration, Instant};

use biocraft_ui::biocraft_ai_surface::{
    AiBaglam, CokluAiKontrol, EchoSaglayici, Provider, SaglayiciKayit,
};
use biocraft_ui::biocraft_net::{kuratorlu_veri, Kategori, OgeTuru, PazarOgesi, YerelPazarKaynagi};
use biocraft_ui::market::suz_ve_sirala;
use biocraft_ui::{BioCraftPazar, Dil, KurulumDurum, PazarSekme, PazarSuzgec, Siralama, Tokenlar};
use chrono::Utc;

fn baslik(s: &str) {
    println!("\n========== {s} ==========");
}

/// Geçerli, kurulabilir bir sentetik `.bcext` paketi (manifest kimliği = verilen kimlik).
fn sentetik_paket(kimlik: &str) -> Vec<u8> {
    let manifest = format!(
        "[eklenti]\nkimlik = \"{kimlik}\"\nad = \"Demo\"\nsurum = \"1.0.0\"\n\
         katman = \"wasm\"\ngiris = \"ana.wat\"\n\n[uyumluluk]\nabi = \"0.1\"\ncekirdek_min = \"0.1.0\"\n"
    );
    let dosyalar = vec![
        ("biocraft.toml".to_string(), manifest.into_bytes()),
        ("ana.wat".to_string(), b"(module)".to_vec()),
    ];
    biocraft_plugin_host::install::paketle(&dosyalar)
}

fn pazari_yukle() -> BioCraftPazar {
    let mut p = BioCraftPazar::yeni();
    p.baslat(YerelPazarKaynagi::yeni(Duration::ZERO, Utc::now()));
    let t = Instant::now();
    while !p.yokla() {
        std::thread::sleep(Duration::from_millis(1));
        assert!(t.elapsed() < Duration::from_secs(5), "yükleme bitmedi");
    }
    p
}

fn main() {
    baslik("1) Küratörlü içerik + dürüst doğrulama rozetleri");
    let veri = kuratorlu_veri(Utc::now());
    for o in &veri.ogeler {
        println!(
            "  • {:<26} [{}] {:<12} ⭐{:.1}  → {}",
            o.ad,
            o.tur.etiket(true),
            o.fiyat.etiket(true),
            o.puan,
            o.dogrulama.etiket(true),
        );
    }
    println!("  Haber akışı:");
    for h in &veri.haberler {
        println!(
            "  • [{}] {}  ({} · {}){}",
            h.tur.etiket(true),
            h.baslik,
            h.kaynak,
            h.tarih,
            if h.dogrulanmis {
                "  ✓küratörlü kaynak"
            } else {
                ""
            },
        );
    }

    baslik("2) Arama / kategori / sıralama");
    let suzgec = PazarSuzgec {
        sorgu: "acme".into(),
        ..Default::default()
    };
    let sirali = suz_ve_sirala(&veri.ogeler, &suzgec, Siralama::Puan, true);
    println!("  'acme' araması (puana göre): {} sonuç", sirali.len());
    for &i in &sirali {
        println!(
            "    - {} (puan {:.1})",
            veri.ogeler[i].ad, veri.ogeler[i].puan
        );
    }
    let populer = suz_ve_sirala(
        &veri.ogeler,
        &PazarSuzgec::default(),
        Siralama::Populer,
        true,
    );
    println!("  En popüler: {}", veri.ogeler[populer[0]].ad);

    baslik("3) Gerçek host kurulumu (imza/bütünlük — İP-07/MK-16)");
    let ek = std::env::temp_dir().join(format!("biocraft_pazar_demo_ek_{}", std::process::id()));
    let ayar =
        std::env::temp_dir().join(format!("biocraft_pazar_demo_ayar_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&ek);
    let _ = std::fs::remove_dir_all(&ayar);
    let mut p = pazari_yukle();
    p.kurulum_baglami_ayarla(&ek, &ayar, biocraft_plugin_host::GuvenDeposu::bos());

    let mut oge = PazarOgesi::yeni(
        "biocraft.demo.pazar",
        "Demo Eklenti",
        "BioCraft",
        OgeTuru::Eklenti,
        Kategori::Arac,
    );
    oge.paket = Some(sentetik_paket("biocraft.demo.pazar"));
    println!("  Kurulum öncesi: {:?}", p.kurulum_durumu(&oge));
    p.kur(&oge).expect("kurulum başarılı");
    println!("  Kurulum sonrası: {:?}", p.kurulum_durumu(&oge));
    println!(
        "  Diske açıldı mı: {}",
        ek.join("biocraft.demo.pazar/ana.wat").is_file()
    );
    p.kaldir(&oge.kimlik).expect("kaldırma");
    println!(
        "  Kaldırma sonrası diskte: {}",
        ek.join("biocraft.demo.pazar").exists()
    );
    let _ = std::fs::remove_dir_all(&ek);
    let _ = std::fs::remove_dir_all(&ayar);

    baslik("4) Çok-AI çapraz kontrol — 'uyum = garanti değil' (MK-47)");
    let saglayicilar: Vec<Arc<dyn Provider>> = vec![
        Arc::new(EchoSaglayici::yeni()),
        Arc::new(EchoSaglayici::yeni()),
    ];
    let baglam = AiBaglam::sorgudan("Bu varyant patojenik mi?");
    let sonuc = CokluAiKontrol::calistir(&saglayicilar, &baglam);
    println!(
        "  Seviye: {} | {} sağlayıcıdan {} hemfikir",
        sonuc.seviye.etiket(true),
        sonuc.uyum.saglayici_sayisi,
        sonuc.uyum.hemfikir
    );
    println!("  garanti_degil() = {}", sonuc.garanti_degil());
    println!("  Uyarı: {}", sonuc.uyari(true));

    baslik("5) Headless çizim — tüm sekme/tema/dil (panik yok)");
    let kayit = SaglayiciKayit::yeni();
    let mut sayac = 0;
    for tema in [
        Tokenlar::koyu(),
        Tokenlar::acik(),
        Tokenlar::yuksek_kontrast(),
    ] {
        for dil in [Dil::Tr, Dil::En] {
            for sekme in [PazarSekme::Magaza, PazarSekme::Haberler] {
                let mut pz = pazari_yukle();
                pz.sekme = sekme;
                let ctx = egui::Context::default();
                let _ = ctx.run(egui::RawInput::default(), |c| {
                    egui::CentralPanel::default().show(c, |ui| {
                        let _ = pz.ciz(ui, &kayit, dil, &tema);
                    });
                });
                sayac += 1;
            }
        }
    }
    println!("  {sayac} kombinasyon panik olmadan çizildi.");

    // Kurulum durumu enum'u kapsama göstergesi.
    let _ = KurulumDurum::Kurulu;
    println!("\nTüm İP-18 kabul kriterleri gösterildi. ✅");
}
