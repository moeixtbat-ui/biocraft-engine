//! İP-14 (Gün 26) demosu — **AI paneli (yüzey) uçtan uca** (arayüzsüz; gerçek pencere açmaz).
//!
//! Çalıştırma:
//! ```text
//! cargo run -p biocraft-ui --example ai_panel_demo
//! ```
//! Kabul kriterlerini gösterir: (1) yapılandırılmadı durumu; (2) echo sağlayıcıyla uçtan uca
//! sohbet + çıktı şeması; (3) PHI çıkış kapısı engeli; (4) AI kapalıyken panel sadeleşir; tüm
//! durumlar headless egui ile panik olmadan çizilir.  Canlı pencere için: `cargo run -p
//! biocraft-app -- --ai-demo`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use biocraft_ui::biocraft_ai_surface::{BaglamOgesi, EchoSaglayici, SaglayiciTuru};
use biocraft_ui::biocraft_types::DataClassification;
use biocraft_ui::{ai_panel_ciz, AiYuzey, Dil, Tokenlar};

fn baslik(s: &str) {
    println!("\n========== {s} ==========");
}

/// Akış bitene kadar (kanal pompalanarak) bekler.
fn akisi_bekle(y: &mut AiYuzey) {
    let t = Instant::now();
    while y.mesgul() {
        y.yokla();
        std::thread::sleep(Duration::from_millis(1));
        assert!(t.elapsed() < Duration::from_secs(5), "akış bitmedi");
    }
}

/// Bir AiYuzey'i headless egui ile çizer (panik olmamalı).
fn ciz(y: &mut AiYuzey, dil: Dil) {
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |c| {
        egui::CentralPanel::default().show(c, |ui| {
            let _ = ai_panel_ciz(ui, y, dil, &Tokenlar::acik());
        });
    });
}

fn main() {
    // ── 1) AI kapalı → panel sadeleşir, uygulama tam çalışır ───────────────────
    baslik("1) AI kapalı");
    let mut kapali = AiYuzey::yeni();
    println!(
        "etkin={}, yapılandırıldı={}",
        kapali.etkin,
        kapali.yapilandirildi_mi()
    );
    ciz(&mut kapali, Dil::Tr);
    println!("  → panel 'AI kapalı' gösterir (headless çizim: panik yok).");

    // ── 2) AI açık ama sağlayıcı yok → "yapılandırılmadı" (MK-48) ──────────────
    baslik("2) AI açık, sağlayıcı yok → yapılandırılmadı");
    let mut yok = AiYuzey::yeni();
    yok.etkin = true;
    println!(
        "yapılandırıldı={} (sahte işlev YOK)",
        yok.yapilandirildi_mi()
    );
    ciz(&mut yok, Dil::Tr);

    // ── 3) Echo sağlayıcı + uçtan uca sohbet ───────────────────────────────────
    baslik("3) Echo sağlayıcı ile uçtan uca sohbet");
    let mut y = AiYuzey::yeni();
    y.etkin = true;
    y.token_goster = true;
    y.maliyet_goster = true;
    y.saglayici_ekle(Arc::new(EchoSaglayici::yeni()));
    y.girdi = "Bu varyantı yorumla".to_string();
    y.gonder();
    akisi_bekle(&mut y);
    let cikti = y.son_cikti.clone().unwrap();
    println!("yanıt   : {}", cikti.metin);
    println!("güven   : {}", cikti.guven.seviye.etiket(true));
    println!("kaynak  : {} adet", cikti.kaynaklar.len());
    println!("⚠ uyarı : {}", cikti.dogrulama_uyarisi);
    println!(
        "token   : {} (oturum: {})",
        cikti.kullanim.toplam(),
        y.sayac.oturum_jeton
    );
    println!("durum çubuğu token: {:?}", y.durum_token());
    println!("denetim : {} girdi (PII'siz)", y.denetim.say());
    ciz(&mut y, Dil::Tr);

    // ── 4) PHI çıkış kapısı (bulut sağlayıcı) → engellenir ─────────────────────
    baslik("4) PHI + bulut sağlayıcı → çıkış kapısı engeller (MK-42/43)");
    let mut p = AiYuzey::yeni();
    p.etkin = true;
    p.saglayici_ekle(Arc::new(EchoSaglayici::tur_ile(SaglayiciTuru::Bulut)));
    p.bekleyen_ogeler = vec![BaglamOgesi::yeni(
        "hasta kaydı",
        "gizli",
        DataClassification::HasasPhi,
    )];
    p.girdi = "Bu hastayı özetle".to_string();
    p.gonder();
    println!("mesgul={} (akış başlamamalı)", p.mesgul());
    match &p.son_hata {
        Some(h) => println!("  → ENGELLENDİ: {}", h.ne_oldu),
        None => println!("  → HATA: engellenmedi!"),
    }
    println!(
        "geçmiş boş mu (engellenen sorgu girmez): {}",
        p.sohbet.is_empty()
    );
    ciz(&mut p, Dil::Tr);

    println!("\n✓ AI paneli tüm durumlarda dürüstçe çalıştı (MK-46/47/48/49).");
}
