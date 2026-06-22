//! Onboarding modülü entegrasyon testleri (İP-17) — kabul kriterlerini doğrular.

use super::*;
use crate::i18n::Dil;
use crate::tokens::Tokenlar;

#[test]
fn ilk_acilis_rol_diyalogu_acar() {
    // Kalıcı kayıt yoksa → ilk açılış; "Rolün?" diyaloğu açık gelmeli (K1).
    let d = OnboardingDurumu::yukle_veya_ilk(None);
    assert!(d.ilk_acilis, "kayıt yokken ilk açılış olmalı");
    assert!(d.rol_dialog_acik, "ilk açılışta rol diyaloğu açık olmalı");
    assert!(!d.tur.aktif, "tur rol seçiminden sonra başlar");
}

#[test]
fn ikinci_acilis_onboarding_acmaz() {
    // Bir kez tamamlanmış kayıt → ikinci açılışta hiçbir örtü otomatik açılmaz.
    let mut d = OnboardingDurumu::ilk_kez();
    d.rol = Some(Rol::Arastirmaci);
    d.tur.tamamlandi = true;
    let json = d.json();

    let d2 = OnboardingDurumu::yukle_veya_ilk(Some(&json));
    assert!(!d2.ilk_acilis);
    assert!(!d2.rol_dialog_acik);
    assert!(!d2.tur.aktif);
    assert_eq!(d2.rol, Some(Rol::Arastirmaci), "rol kalıcı olmalı");
}

#[test]
fn kayit_gidis_donus_korur() {
    let mut d = OnboardingDurumu::ilk_kez();
    d.rol = Some(Rol::Gelistirici);
    d.tur.tamamlandi = true;
    d.ipuclari.kapali = true;
    let json = d.json();
    let geri = OnboardingDurumu::yukle_veya_ilk(Some(&json));
    assert_eq!(geri.rol, Some(Rol::Gelistirici));
    assert!(geri.tur.tamamlandi);
    assert!(geri.ipuclari.kapali);
}

#[test]
fn bozuk_kayit_onboarding_zorlamaz() {
    // Bozuk JSON → kullanıcıyı tekrar onboarding'e zorlama (sessiz/zararsız).
    let d = OnboardingDurumu::yukle_veya_ilk(Some("{bozuk json"));
    assert!(!d.ilk_acilis);
    assert!(!d.rol_dialog_acik);
}

#[test]
fn rol_secimi_turu_baslatir_ve_kirletir() {
    // Rol diyaloğunda seçim/atlama → tur başlar + kalıcılık kirlenir.
    let mut d = OnboardingDurumu::ilk_kez();
    assert!(d.rol_dialog_acik);
    // Diyalog kararını taklit et (overlay_ciz iç akışıyla aynı sonuç).
    d.rol_dialog_acik = false;
    d.kirli = true;
    d.tur.baslat();
    d.rol = Some(Rol::Ogrenci);
    assert!(d.tur.aktif, "rol sonrası tur başlamalı");
    assert!(d.kirli_mi());
}

#[test]
fn varsayilan_demo_her_zaman_dolu_sablon() {
    // "Demo Projeyi Aç" hiçbir zaman boş şablon vermemeli (kullanıcı boş ekranla kalmaz).
    let mut d = OnboardingDurumu::default();
    assert_eq!(d.varsayilan_demo_sablonu(), OnboardingSablon::GenomGorsel);
    d.rol = Some(Rol::Gelistirici); // önerisi Boş → görsel demoya düşmeli
    assert_ne!(d.varsayilan_demo_sablonu(), OnboardingSablon::Bos);
    d.rol = Some(Rol::Arastirmaci);
    assert_eq!(
        d.varsayilan_demo_sablonu(),
        OnboardingSablon::VaryantInceleme
    );
}

#[test]
fn ipuclari_kapatma_kirletir() {
    let mut d = OnboardingDurumu::default();
    assert!(!d.ipuclari.kapali);
    d.ipuclari_kapat();
    assert!(d.ipuclari.kapali);
    assert!(d.kirli_mi());
}

#[test]
fn overlay_headless_tum_durumlarda_cizilir() {
    // Her örtü durumu panik atmadan çizilmeli (headless egui).
    let tok = Tokenlar::koyu();

    // 1) Rol diyaloğu açık.
    let mut d = OnboardingDurumu::ilk_kez();
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |c| {
        let _ = d.overlay_ciz(c, Dil::Tr, &tok);
    });

    // 2) Tur aktif.
    let mut d2 = OnboardingDurumu::default();
    d2.turu_baslat();
    let _ = ctx.run(egui::RawInput::default(), |c| {
        let _ = d2.overlay_ciz(c, Dil::En, &tok);
    });

    // 3) Galeri açık.
    let mut d3 = OnboardingDurumu::default();
    d3.galeriyi_ac();
    let _ = ctx.run(egui::RawInput::default(), |c| {
        let _ = d3.overlay_ciz(c, Dil::Tr, &tok);
    });

    // 4) Yardım açık.
    let mut d4 = OnboardingDurumu::default();
    d4.yardimi_ac();
    let _ = ctx.run(egui::RawInput::default(), |c| {
        let _ = d4.overlay_ciz(c, Dil::En, &tok);
    });
}
