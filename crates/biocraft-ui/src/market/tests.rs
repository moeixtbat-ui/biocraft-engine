//! İP-18 Bilim Pazarı — modül düzeyi testler (durum makinesi + kurulum + headless çizim).

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use biocraft_ai_surface::SaglayiciKayit;
use biocraft_net::{
    kuratorlu_veri, DogrulamaDurumu, Kategori, OgeTuru, PazarOgesi, YerelPazarKaynagi,
};
use chrono::Utc;

use super::*;
use crate::i18n::Dil;
use crate::tokens::{Tema, Tokenlar};

static SAYAC: AtomicU32 = AtomicU32::new(0);

fn gecici(etiket: &str) -> std::path::PathBuf {
    let n = SAYAC.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!(
        "biocraft_pazar_{etiket}_{}_{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// Geçerli, kurulabilir bir sentetik `.bcext` paketi üretir (manifest kimliği verilen kimlik).
fn sentetik_paket(kimlik: &str) -> Vec<u8> {
    let manifest = format!(
        r#"
[eklenti]
kimlik = "{kimlik}"
ad = "Pazar Test"
surum = "1.0.0"
katman = "wasm"
giris = "ana.wat"

[uyumluluk]
abi = "0.1"
cekirdek_min = "0.1.0"
"#
    );
    let dosyalar = vec![
        ("biocraft.toml".to_string(), manifest.into_bytes()),
        ("ana.wat".to_string(), b"(module)".to_vec()),
    ];
    biocraft_plugin_host::install::paketle(&dosyalar)
}

fn yoklayarak_bekle(p: &mut BioCraftPazar) {
    let baslangic = Instant::now();
    while p.yukleyici.yukleme_aktif() && baslangic.elapsed() < Duration::from_secs(5) {
        if p.yokla() {
            break;
        }
        std::thread::yield_now();
    }
}

#[test]
fn yeni_pazar_yukleniyor() {
    let p = BioCraftPazar::yeni();
    assert!(!p.baslatildi());
    assert!(p.yukleyici.durum().yukleniyor_mu());
}

#[test]
fn baslat_kuratorlu_veri_yukler() {
    let mut p = BioCraftPazar::yeni();
    p.baslat(YerelPazarKaynagi::yeni(Duration::ZERO, Utc::now()));
    assert!(p.baslatildi());
    yoklayarak_bekle(&mut p);
    let veri = p.veri().expect("veri yüklenmeli");
    assert!(!veri.ogeler.is_empty());
    assert!(!veri.haberler.is_empty());
}

#[test]
fn basarisiz_kaynak_onbellekle_cevrimdisi() {
    let mut p = BioCraftPazar::onbellek_ile(kuratorlu_veri(Utc::now()));
    p.baslat(YerelPazarKaynagi::basarisiz(Utc::now()));
    yoklayarak_bekle(&mut p);
    assert!(p.yukleyici.durum().cevrimdisi_mi());
    // Çevrimdışı bile olsa önbellekten içerik gösterilir.
    assert!(p.veri().is_some());
}

#[test]
fn kurulum_durumu_gecisleri() {
    let mut p = BioCraftPazar::yeni();
    let mut oge = PazarOgesi::yeni("biocraft.x.y", "X", "Y", OgeTuru::Eklenti, Kategori::Analiz);
    oge.surum = "1.0.0".into();
    // Başlangıçta kurulu değil.
    assert_eq!(p.kurulum_durumu(&oge), KurulumDurum::KuruluDegil);
    // Kur → Kurulu.
    p.kur(&oge).unwrap();
    assert_eq!(p.kurulum_durumu(&oge), KurulumDurum::Kurulu);
    // Sürüm artarsa → güncelleme var.
    oge.surum = "1.1.0".into();
    assert_eq!(p.kurulum_durumu(&oge), KurulumDurum::GuncellemeVar);
    // Güncelle → tekrar Kurulu (yeni sürüm).
    p.guncelle(&oge).unwrap();
    assert_eq!(p.kurulum_durumu(&oge), KurulumDurum::Kurulu);
    // Kaldır → kurulu değil.
    p.kaldir(&oge.kimlik).unwrap();
    assert_eq!(p.kurulum_durumu(&oge), KurulumDurum::KuruluDegil);
}

#[test]
fn gercek_host_kurulumu_diske_yazar() {
    // Host bağlamı yapılandırılınca .bcext gerçekten kurulur (imza/bütünlük denetimi — İP-07/MK-16).
    let ek = gecici("ek");
    let ayar = gecici("ayar");
    let mut p = BioCraftPazar::yeni();
    p.kurulum_baglami_ayarla(&ek, &ayar, GuvenDeposu::bos());

    let mut oge = PazarOgesi::yeni(
        "biocraft.test.pazar",
        "Pazar Test",
        "BioCraft",
        OgeTuru::Eklenti,
        Kategori::Arac,
    );
    oge.paket = Some(sentetik_paket("biocraft.test.pazar"));

    p.kur(&oge).unwrap();
    assert_eq!(p.kurulum_durumu(&oge), KurulumDurum::Kurulu);
    assert!(
        ek.join("biocraft.test.pazar/ana.wat").is_file(),
        "paket diske açılmalı"
    );

    // Kaldır → diskten de silinir.
    p.kaldir(&oge.kimlik).unwrap();
    assert!(!ek.join("biocraft.test.pazar").exists());

    let _ = std::fs::remove_dir_all(&ek);
    let _ = std::fs::remove_dir_all(&ayar);
}

#[test]
fn headless_cizim_tum_sekme_tema_dil() {
    // Yüklü veriyle: hiçbir sekme/tema/dil kombinasyonunda panik olmamalı.
    let kayit = SaglayiciKayit::yeni();
    for tema in [Tema::Koyu, Tema::Acik, Tema::YuksekKontrast] {
        for dil in [Dil::Tr, Dil::En] {
            for sekme in [PazarSekme::Magaza, PazarSekme::Haberler] {
                let mut p = BioCraftPazar::onbellek_ile(kuratorlu_veri(Utc::now()));
                // Önbellek 'çevrimdışı' değil 'yüklendi' göstermesi için sahte başarılı yükle.
                p.baslat(YerelPazarKaynagi::yeni(Duration::ZERO, Utc::now()));
                yoklayarak_bekle(&mut p);
                p.sekme = sekme;
                let ctx = egui::Context::default();
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let tok = Tokenlar::temalı(tema);
                        let _ = p.ciz(ui, &kayit, dil, &tok);
                    });
                });
            }
        }
    }
}

#[test]
fn headless_detay_gorunumu_cizilir() {
    let kayit = SaglayiciKayit::yeni();
    let mut p = BioCraftPazar::onbellek_ile(kuratorlu_veri(Utc::now()));
    p.baslat(YerelPazarKaynagi::yeni(Duration::ZERO, Utc::now()));
    yoklayarak_bekle(&mut p);
    // İlk öğeyi seç (detay görünümü).
    let ilk = p.veri().unwrap().ogeler[0].kimlik.clone();
    p.secili = Some(ilk);
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let tok = Tokenlar::koyu();
            let _ = p.ciz(ui, &kayit, Dil::Tr, &tok);
        });
    });
}

#[test]
fn kurulum_eylemi_bildirim_uretir() {
    // Ham "Kur" eylemi içeride uygulanır + kullanıcıya bildirim üretir (geri bildirim — TDA m.15).
    let mut p = BioCraftPazar::onbellek_ile(kuratorlu_veri(Utc::now()));
    p.baslat(YerelPazarKaynagi::yeni(Duration::ZERO, Utc::now()));
    yoklayarak_bekle(&mut p);
    let kimlik = p.veri().unwrap().ogeler[0].kimlik.clone();
    p.kurulum_eylemi(&kimlik, KurEylem::Kur, true);
    assert!(p.son_bildirim.as_ref().unwrap().contains("Kuruldu"));
    let oge = p.veri().unwrap().oge(&kimlik).unwrap().clone();
    assert_eq!(p.kurulum_durumu(&oge), KurulumDurum::Kurulu);
    // Doğrulama modeli erişilebilir (dürüst rozet ayrımı).
    assert!(DogrulamaDurumu::Resmi.guven_rozeti_mi());
}
