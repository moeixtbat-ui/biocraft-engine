//! AI yüzeyi (L4) durum/mantık testleri — uçtan uca echo, çıkış kapısı, kapalı/yapılandırılmadı.

use std::sync::Arc;
use std::time::{Duration, Instant};

use biocraft_ai_surface::{BaglamOgesi, EchoSaglayici, SaglayiciTuru};
use biocraft_types::DataClassification;

use super::*;
use crate::i18n::Dil;
use crate::tokens::Tokenlar;

/// Akış bitene kadar (veya zaman aşımına dek) `yokla` çağırır.
fn akisi_bekle(yuzey: &mut AiYuzey) {
    let baslangic = Instant::now();
    while yuzey.mesgul() {
        yuzey.yokla();
        if baslangic.elapsed() > Duration::from_secs(5) {
            panic!("akış 5 sn içinde bitmedi");
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn kapali_yuzey_gondermez() {
    let mut y = AiYuzey::yeni();
    y.saglayici_ekle(Arc::new(EchoSaglayici::yeni()));
    y.etkin = false; // kapalı
    y.girdi = "merhaba".into();
    y.gonder();
    assert!(!y.mesgul(), "AI kapalıyken gönderim olmamalı");
    assert!(y.sohbet.is_empty());
}

#[test]
fn yapilandirilmamis_yuzey_gondermez() {
    let mut y = AiYuzey::yeni();
    y.etkin = true; // ama sağlayıcı yok
    y.girdi = "merhaba".into();
    y.gonder();
    assert!(!y.mesgul(), "sağlayıcı yokken gönderim olmamalı");
    assert!(!y.yapilandirildi_mi());
}

#[test]
fn echo_uctan_uca_cikti_semasi() {
    // Mock sağlayıcı ile uçtan uca: kaynak + güven + "doğrulanmalı" + token gelir.
    let mut y = AiYuzey::yeni();
    y.etkin = true;
    y.saglayici_ekle(Arc::new(EchoSaglayici::yeni()));
    y.girdi = "bu varyantı yorumla".into();
    y.gonder();
    assert!(y.mesgul(), "gönderince akış başlamalı");
    akisi_bekle(&mut y);

    let cikti = y.son_cikti.expect("çıktı gelmeli");
    assert!(cikti.metin.contains("varyantı yorumla"));
    assert!(!cikti.kaynaklar.is_empty(), "kaynak göstermeli");
    assert!(
        !cikti.dogrulama_uyarisi.is_empty(),
        "'doğrulanmalı' uyarısı olmalı"
    );
    assert!(cikti.klinik_degil, "klinik değil etiketli (MK-49)");
    assert!(cikti.kullanim.toplam() > 0, "token sayılmalı");
    // Geçmiş: kullanıcı + asistan mesajı.
    assert_eq!(y.sohbet.len(), 2);
    // Maliyet sayacı işledi (yerel → bedel 0).
    assert!(y.sayac.cagri_sayisi >= 1);
    assert!(y.sayac.oturum_jeton > 0);
    // Denetim kaydı tutuldu (PII'siz).
    assert!(y.denetim.say() >= 1);
}

#[test]
fn cikis_kapisi_phi_dis_saglayiciya_engeller() {
    // Bulut (dış) echo + PHI öğesi → gönderim ENGELLENİR; thread başlamaz.
    let mut y = AiYuzey::yeni();
    y.etkin = true;
    y.saglayici_ekle(Arc::new(EchoSaglayici::tur_ile(SaglayiciTuru::Bulut)));
    y.bekleyen_ogeler = vec![BaglamOgesi::yeni(
        "hasta kaydı",
        "gizli özet",
        DataClassification::HasasPhi,
    )];
    y.girdi = "bu hastayı özetle".into();
    y.gonder();

    assert!(
        !y.mesgul(),
        "PHI dış AI'a gitmemeli → akış başlamamalı (MK-42/43)"
    );
    assert!(y.son_hata.is_some(), "engel hatası gösterilmeli");
    assert!(y.sohbet.is_empty(), "engellenen sorgu geçmişe girmemeli");
    // Denetim: engellendi kaydı.
    assert!(y.denetim.say() >= 1);
}

#[test]
fn yerel_saglayici_phi_de_calisir() {
    // Yerel echo + PHI → izinli (veri cihazdan çıkmaz, 0-AI.5/1).
    let mut y = AiYuzey::yeni();
    y.etkin = true;
    y.saglayici_ekle(Arc::new(EchoSaglayici::yeni())); // Yerel
    y.bekleyen_ogeler = vec![BaglamOgesi::yeni(
        "kayıt",
        "x",
        DataClassification::HasasPhi,
    )];
    y.girdi = "özetle".into();
    y.gonder();
    assert!(
        y.mesgul() || y.son_cikti.is_some(),
        "yerel sağlayıcı PHI'de çalışabilmeli"
    );
    akisi_bekle(&mut y);
    assert!(y.son_hata.is_none(), "yerelde PHI engeli olmamalı");
}

#[test]
fn durum_token_gostergesi_ayara_bagli() {
    let mut y = AiYuzey::yeni();
    y.etkin = true;
    assert_eq!(y.durum_token(), None, "gösterge kapalı → None");
    y.token_goster = true;
    assert_eq!(y.durum_token(), Some(0));
    y.etkin = false;
    assert_eq!(y.durum_token(), None, "AI kapalı → None");
}

#[test]
fn panel_headless_tum_durumlar_panik_yok() {
    let ctx = egui::Context::default();
    // 1) kapalı, 2) yapılandırılmadı, 3) hazır+çıktılı — üçü de panik olmadan çizilmeli.
    let mut hazir = AiYuzey::yeni();
    hazir.etkin = true;
    hazir.saglayici_ekle(Arc::new(EchoSaglayici::yeni()));
    hazir.girdi = "merhaba".into();
    hazir.gonder();
    akisi_bekle(&mut hazir);

    let mut kapali = AiYuzey::yeni();
    let mut yapilandirilmadi = AiYuzey::yeni();
    yapilandirilmadi.etkin = true;

    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let tok = Tokenlar::acik();
            let _ = ai_panel_ciz(ui, &mut kapali, Dil::Tr, &tok);
            let _ = ai_panel_ciz(ui, &mut yapilandirilmadi, Dil::En, &tok);
            let _ = ai_panel_ciz(ui, &mut hazir, Dil::Tr, &tok);
        });
    });
}
