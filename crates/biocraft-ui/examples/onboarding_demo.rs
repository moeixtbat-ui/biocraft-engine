//! İP-17 (Gün 28) demosu — **Onboarding uçtan uca** (arayüzsüz; gerçek pencere açmaz).
//!
//! Çalıştırma:
//! ```text
//! cargo run -p biocraft-ui --example onboarding_demo
//! ```
//! Kabul kriterlerini gösterir: (1) ilk açılışta "Rolün?" (K1) + tur çıkar, atlanabilir, tekrar
//! açılır; (2) rol role-göre şablon önerir (dayatmaz); (3) şablon ilgili panelleri + örnek akışı
//! ön-kurar; (4) "Demo Projeyi Aç" gömülü örnek veriyle dolu gelir; (5) tüm metin TR/EN çevrilir;
//! tüm örtüler headless egui ile panik olmadan çizilir.  Canlı pencere için: `cargo run -p
//! biocraft-app` (ilk açılışta otomatik) ya da launcher'da "🎓 Tur" / "▶ Demo Projesi".

use biocraft_ui::onboarding::templates::OnboardingSablon;
use biocraft_ui::onboarding::tutorial::Kavram;
use biocraft_ui::onboarding::{OnboardingDurumu, OnboardingEylem, TurAdim};
use biocraft_ui::{Dil, Rol, Tokenlar};

fn baslik(s: &str) {
    println!("\n========== {s} ==========");
}

/// Bir onboarding örtüsünü headless egui ile çizer (panik olmamalı).
fn ciz(d: &mut OnboardingDurumu, dil: Dil) -> Option<OnboardingEylem> {
    let ctx = egui::Context::default();
    let mut eylem = None;
    let _ = ctx.run(egui::RawInput::default(), |c| {
        eylem = d.overlay_ciz(c, dil, &Tokenlar::koyu());
    });
    eylem
}

fn main() {
    baslik("1) İlk açılış: kayıt yok → Rolün? + tur otomatik");
    let mut d = OnboardingDurumu::yukle_veya_ilk(None);
    println!(
        "ilk_acilis={} · rol_dialog_acik={} · tur.aktif={}",
        d.ilk_acilis, d.rol_dialog_acik, d.tur.aktif
    );
    assert!(d.ilk_acilis && d.rol_dialog_acik);
    let _ = ciz(&mut d, Dil::Tr); // rol diyaloğu headless çizilir

    baslik("2) Rol seçimi role-göre şablon ÖNERİR (dayatmaz)");
    for &r in Rol::TUMU {
        println!(
            "  {} {} → önerilen: {}",
            r.ikon(),
            r.ad(true),
            r.onerilen_sablon().ad(true)
        );
    }

    baslik("3) Tur: atlanabilir + tekrar açılabilir");
    let mut d2 = OnboardingDurumu::default();
    d2.turu_baslat();
    println!(
        "tur başladı: adım {}/{}",
        d2.tur.adim + 1,
        TurAdim::toplam()
    );
    d2.tur.bitir(); // Atla
    println!(
        "atlandı: aktif={} tamamlandi={}",
        d2.tur.aktif, d2.tur.tamamlandi
    );
    d2.turu_baslat(); // Yardım > Tur
    println!(
        "yeniden açıldı: aktif={} adım={}",
        d2.tur.aktif, d2.tur.adim
    );
    assert!(d2.tur.aktif && d2.tur.adim == 0);

    baslik("4) Şablon → ilgili paneller + gömülü demo veri");
    for &s in OnboardingSablon::TUMU {
        let p = s.panel_plani();
        let v = s.demo_veriler();
        let dosyalar: Vec<&str> = v.iter().map(|d| d.ad).collect();
        println!(
            "  {} {:<22} paneller[yan={} alt={} insp={} node={} kod={}] demo={:?}",
            s.ikon(),
            s.ad(true),
            p.yan_panel as u8,
            p.alt_panel as u8,
            p.inspector as u8,
            p.node_tuvali as u8,
            p.kod_editoru as u8,
            dosyalar,
        );
    }

    baslik("5) Demo veri gömülü + sentetik/açık-lisans (köken)");
    for v in OnboardingSablon::GenomGorsel.demo_veriler() {
        let (kaynak, lisans) = v.koken();
        println!(
            "  📎 {} [{}] · {} satır · köken: {} / {}",
            v.ad,
            v.bicim,
            v.satir_sayisi(),
            kaynak,
            lisans
        );
    }

    baslik("6) i18n: tüm onboarding metni TR↔EN çevrilir");
    let mut sayac = 0;
    for &r in Rol::TUMU {
        assert_ne!(r.ad(true), r.ad(false));
        sayac += 1;
    }
    for &a in TurAdim::TUMU {
        assert_ne!(a.baslik(true), a.baslik(false));
        sayac += 1;
    }
    for &s in OnboardingSablon::TUMU {
        assert_ne!(s.ad(true), s.ad(false));
        sayac += 1;
    }
    for &k in Kavram::TUMU {
        assert_ne!(k.aciklama(true), k.aciklama(false));
        sayac += 1;
    }
    println!("  {sayac} öğenin TR ve EN metni farklı (çeviri yapılmış).");

    baslik("7) Kalıcılık: seçimler oturumlar arası korunur");
    let mut d3 = OnboardingDurumu::ilk_kez();
    d3.rol = Some(Rol::Arastirmaci);
    d3.tur.tamamlandi = true;
    let json = d3.json();
    println!("  kayıt JSON: {json}");
    let geri = OnboardingDurumu::yukle_veya_ilk(Some(&json));
    println!(
        "  geri yüklendi: ilk_acilis={} rol={:?} tur_tamam={}",
        geri.ilk_acilis, geri.rol, geri.tur.tamamlandi
    );
    assert!(!geri.ilk_acilis && geri.rol == Some(Rol::Arastirmaci));

    baslik("8) Tüm örtüler headless çizilir (panik yok)");
    let mut r = OnboardingDurumu::ilk_kez();
    let _ = ciz(&mut r, Dil::En);
    println!("  ✓ rol diyaloğu çizildi");
    let mut g = OnboardingDurumu::default();
    g.galeriyi_ac();
    let _ = ciz(&mut g, Dil::Tr);
    println!("  ✓ şablon galerisi çizildi");
    let mut y = OnboardingDurumu::default();
    y.yardimi_ac();
    let _ = ciz(&mut y, Dil::En);
    println!("  ✓ yardım penceresi çizildi");

    println!("\n✅ İP-17 onboarding: tüm kabul kriterleri uçtan uca doğrulandı.");
}
