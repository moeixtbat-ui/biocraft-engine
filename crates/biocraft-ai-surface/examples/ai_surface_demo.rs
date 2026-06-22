//! İP-14 (Gün 26) demosu — **AI yüzey sözleşmesi uçtan uca** (saf pipeline, arayüzsüz).
//!
//! Çalıştırma:
//! ```text
//! cargo run -p biocraft-ai-surface --example ai_surface_demo
//! ```
//! Kabul kriterlerini uçtan uca gösterir: sağlayıcı kayıt/keşif + "yapılandırılmadı" durumu;
//! mock/echo sağlayıcı ile generate/stream; **çıktı şeması** (kaynak + güven + "doğrulanmalı" +
//! token/maliyet); **çıkış kapısı** (PHI dış AI'a engellenir, yerelde izinli); denetim kaydı.

use std::sync::Arc;

use biocraft_ai_surface::{
    baglam_denetle, AiBaglam, AkisOlay, BaglamOgesi, CagriSonucu, DenetimGirdisi, DenetimKaydi,
    EchoSaglayici, GuardKarari, IptalBayragi, MaliyetSayaci, SaglayiciKayit, SaglayiciTuru,
};
use biocraft_types::DataClassification;

fn baslik(s: &str) {
    println!("\n========== {s} ==========");
}

fn main() {
    // ── 1) Sağlayıcı yok → "yapılandırılmadı" (MK-48) ──────────────────────────
    baslik("1) Kayıt boş → AI yapılandırılmadı");
    let mut kayit = SaglayiciKayit::yeni();
    println!("durum: {:?} (bos_mu={})", kayit.durum(), kayit.bos_mu());

    // ── 2) Demo/echo sağlayıcı kaydı (gerçek AI değil) ─────────────────────────
    baslik("2) Echo (demo) sağlayıcı kaydı");
    kayit.kaydet(Arc::new(EchoSaglayici::yeni()));
    println!("durum: {:?}", kayit.durum());
    for k in kayit.kimlikler() {
        println!("  • {} [{}] — {}", k.ad, k.tur.etiket(true), k.aciklama);
    }

    let saglayici = kayit.secili().unwrap();
    let mut sayac = MaliyetSayaci::yeni();
    let mut denetim = DenetimKaydi::yeni();

    // ── 3) generate → zengin çıktı şeması (MK-47) ──────────────────────────────
    baslik("3) generate → çıktı şeması (kaynak + güven + doğrulanmalı + token)");
    let baglam = AiBaglam::sorgudan("Bu varyantın olası etkisini yorumla");
    let cikti = saglayici.uret(&baglam).unwrap();
    println!("metin     : {}", cikti.metin);
    println!("öneriler  : {:?}", cikti.oneriler);
    println!(
        "güven     : {} (oran {:.2})",
        cikti.guven.seviye.etiket(true),
        cikti.guven.seviye.oran()
    );
    for k in &cikti.kaynaklar {
        println!("kaynak    : {} — {:?}", k.baslik, k.atif);
    }
    for e in &cikti.eylem_onerileri {
        println!(
            "eylem     : {} (onay_gerekli={}, geri_alinabilir={})",
            e.aciklama,
            e.onay_gerekli(),
            e.geri_alinabilir
        );
    }
    println!("⚠ uyarı   : {}", cikti.dogrulama_uyarisi);
    println!(
        "klinik?   : {}",
        if cikti.klinik_degil {
            "KLİNİK DEĞİL"
        } else {
            "?"
        }
    );
    let maliyet = saglayici.maliyet(&cikti.kullanim);
    sayac.ekle(maliyet.clone());
    println!("maliyet   : {}", maliyet.goster(true));
    denetim.kaydet(DenetimGirdisi::baglamdan(
        &baglam,
        saglayici.kimlik(),
        cikti.kullanim.toplam(),
        CagriSonucu::Tamam,
    ));

    // ── 4) stream → parça parça + durdurulabilir ───────────────────────────────
    baslik("4) stream → akış (parça parça)");
    let iptal = IptalBayragi::yeni();
    let mut parca = 0;
    print!("akış: ");
    saglayici.akis(
        &AiBaglam::sorgudan("Bölgeyi kısaca özetle"),
        &iptal,
        &mut |olay| match olay {
            AkisOlay::Parca(p) => {
                parca += 1;
                print!("{p}");
            }
            AkisOlay::Tamamlandi(_) => println!("\n  → tamamlandı ({parca} parça)"),
            AkisOlay::Durduruldu => println!("\n  → durduruldu"),
            AkisOlay::Hata(e) => println!("\n  → hata: {}", e.ne_oldu),
        },
    );

    // ── 5) Çıkış kapısı (YZ-08/MK-42/43): PHI dış AI'a engellenir ──────────────
    baslik("5) Çıkış kapısı — PHI dış AI'a ENGELLENİR, yerelde İZİNLİ");
    let phi_baglam = AiBaglam::sorgudan("Bu hastayı özetle")
        .oge_ile(BaglamOgesi::yeni(
            "normal tablo",
            "x",
            DataClassification::Normal,
        ))
        .oge_ile(BaglamOgesi::yeni(
            "hasta kaydı",
            "y",
            DataClassification::HasasPhi,
        ));

    for tur in [
        SaglayiciTuru::Yerel,
        SaglayiciTuru::Bulut,
        SaglayiciTuru::Ozel,
    ] {
        match baglam_denetle(&phi_baglam, tur) {
            GuardKarari::Izinli => println!("  {:<6} → İZİNLİ", tur.etiket(true)),
            GuardKarari::Engellendi {
                engellenen_ogeler,
                hata,
            } => {
                println!(
                    "  {:<6} → ENGELLENDİ (öğeler: {:?})\n          → {}",
                    tur.etiket(true),
                    engellenen_ogeler,
                    hata.ne_oldu
                );
                denetim.kaydet(DenetimGirdisi::baglamdan(
                    &phi_baglam,
                    saglayici.kimlik(),
                    0,
                    CagriSonucu::Engellendi,
                ));
            }
        }
    }

    // ── 6) Oturum özeti + denetim kaydı (PII'siz) ──────────────────────────────
    baslik("6) Oturum özeti + denetim kaydı (PII'siz, şeffaf)");
    println!("maliyet özeti: {}", sayac.ozet(true));
    println!("denetim girdileri ({}):", denetim.say());
    for g in denetim.girdiler() {
        println!(
            "  • sağlayıcı={} sorgu_uzunluğu={} öğe={} sınıflar={:?} jeton={} sonuç={:?}",
            g.saglayici, g.sorgu_uzunlugu, g.oge_sayisi, g.siniflar, g.jeton, g.sonuc
        );
    }

    println!("\n✓ AI yüzey sözleşmesi uçtan uca çalıştı (gerçek motor olmadan; echo = demo).");
}
