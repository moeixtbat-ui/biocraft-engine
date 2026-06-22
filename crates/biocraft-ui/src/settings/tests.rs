//! İP-12 ayar sistemi testleri — katmanlama, güvenli varsayılan, sıfırlama, kalıcılık, profil,
//! eklenti kaydı ve headless egui dumanı.

use std::collections::BTreeMap;

use super::sections::{AyarKategorisi, AyarTanimi, AyarTuru};
use super::*;
use crate::i18n::Dil;
use crate::tokens::Tokenlar;

fn ayarlar() -> Ayarlar {
    Ayarlar::default()
}

// ── Çözümleme + varsayılan ─────────────────────────────────────────────────────

#[test]
fn cozumleme_varsayilani_dondurur() {
    let a = ayarlar();
    // Görünüm teması varsayılanı "koyu".
    assert_eq!(a.secim("gorunum.tema"), "koyu");
    // FPS göstergesi varsayılan kapalı.
    assert!(!a.mantik("performans.fps_goster"));
    // Editör sekme genişliği varsayılan 4.
    assert_eq!(a.tam_sayi("editor.sekme_genisligi"), 4);
}

#[test]
fn kullanici_katmani_varsayilani_ezer() {
    let mut a = ayarlar();
    assert!(a.ayarla("gorunum.tema", AyarDeger::Secim("acik".into())));
    assert_eq!(a.secim("gorunum.tema"), "acik");
    // Aynı değeri tekrar yazmak "değişti" saymaz.
    assert!(!a.ayarla("gorunum.tema", AyarDeger::Secim("acik".into())));
}

// ── Katmanlama: proje global'i geçersiz kılar (kabul kriteri) ──────────────────

#[test]
fn proje_katmani_kullaniciyi_ezer() {
    let mut a = ayarlar();
    // Kullanıcı katmanı: tema = açık.
    a.ayarla("gorunum.tema", AyarDeger::Secim("acik".into()));
    // Proje katmanına geç ve tema = yüksek kontrast.
    a.duzenleme_katmani_ayarla(AyarKatmani::Proje);
    a.ayarla("gorunum.tema", AyarDeger::Secim("yuksek_kontrast".into()));
    // Çözülen değer proje katmanından gelmeli.
    assert_eq!(a.secim("gorunum.tema"), "yuksek_kontrast");
    // Proje katmanında geçersiz kılınmayan bir ayar kullanıcı katmanından gelir.
    a.duzenleme_katmani_ayarla(AyarKatmani::Kullanici);
    a.ayarla("editor.sekme_genisligi", AyarDeger::TamSayi(2));
    assert_eq!(a.tam_sayi("editor.sekme_genisligi"), 2);
    // Proje katmanı kapanınca kullanıcı değeri tekrar görünür.
    a.proje_kapat();
    assert_eq!(a.secim("gorunum.tema"), "acik");
}

// ── Güvenli varsayılan: geçersiz değer asla uygulanmaz (kabul kriteri) ─────────

#[test]
fn gecersiz_deger_guvenli_varsayilana_duser() {
    let mut a = ayarlar();
    // Aralık dışı tam sayı → sıkıştırılır.
    a.ayarla("editor.sekme_genisligi", AyarDeger::TamSayi(999));
    assert_eq!(a.tam_sayi("editor.sekme_genisligi"), 8); // max 8
                                                         // Tip uyumsuz değer → varsayılana iner (4).
    a.ayarla("editor.sekme_genisligi", AyarDeger::Mantik(true));
    assert_eq!(a.tam_sayi("editor.sekme_genisligi"), 4);
    // Tanınmayan seçim anahtarı → varsayılan ("koyu").
    a.ayarla("gorunum.tema", AyarDeger::Secim("mor".into()));
    assert_eq!(a.secim("gorunum.tema"), "koyu");
}

#[test]
fn bozuk_disk_degeri_yuklenince_guvenli() {
    let mut a = ayarlar();
    // Diskte (elle/eski) aralık dışı + tanınmayan anahtar içeren bir kayıt.
    let mut degerler = BTreeMap::new();
    degerler.insert(
        "editor.sekme_genisligi".to_string(),
        AyarDeger::TamSayi(-99),
    );
    degerler.insert("yok.olan".to_string(), AyarDeger::Mantik(true));
    a.kullanici_yukle(AyarKatmaniKaydi {
        surum: KATMAN_SURUMU,
        degerler,
    });
    // Aralığa sıkışmalı (min 2); tanınmayan atılmalı.
    assert_eq!(a.tam_sayi("editor.sekme_genisligi"), 2);
}

// ── Varsayılana dön (bazda + kategori + fabrika) ───────────────────────────────

#[test]
fn varsayilana_don_bazda() {
    let mut a = ayarlar();
    a.ayarla("performans.fps_goster", AyarDeger::Mantik(true));
    assert!(a.mantik("performans.fps_goster"));
    assert!(a.aktif_katman_icerir("performans.fps_goster"));
    assert!(a.varsayilana_don("performans.fps_goster"));
    assert!(!a.mantik("performans.fps_goster"));
    assert!(!a.aktif_katman_icerir("performans.fps_goster"));
}

#[test]
fn kategori_ve_fabrika_sifirlama() {
    let mut a = ayarlar();
    a.ayarla("gorunum.tema", AyarDeger::Secim("acik".into()));
    a.ayarla("gorunum.font_boyutu", AyarDeger::TamSayi(20));
    a.ayarla("editor.satir_numaralari", AyarDeger::Mantik(false));
    // Görünüm kategorisini sıfırla → görünüm varsayılana, editör korunur.
    let n = a.kategori_varsayilana_don(AyarKategorisi::Gorunum);
    assert_eq!(n, 2);
    assert_eq!(a.secim("gorunum.tema"), "koyu");
    assert!(!a.mantik("editor.satir_numaralari"));
    // Fabrika → her şey varsayılana.
    a.ayarla("gorunum.tema", AyarDeger::Secim("acik".into()));
    a.fabrika_sifirla();
    assert_eq!(a.secim("gorunum.tema"), "koyu");
    assert!(a.mantik("editor.satir_numaralari"));
}

// ── 3. derece göstergeler ayardan aç/kapat (kabul kriteri) ─────────────────────

#[test]
fn ucuncu_derece_gostergeler_acilip_kapanir() {
    let mut a = ayarlar();
    for anahtar in [
        "performans.fps_goster",
        "performans.bellek_goster",
        "performans.sicaklik_goster",
        "ai.token_sayaci_goster",
    ] {
        assert!(!a.mantik(anahtar), "varsayılan kapalı: {anahtar}");
        a.ayarla(anahtar, AyarDeger::Mantik(true));
        assert!(a.mantik(anahtar), "açılmalı: {anahtar}");
    }
}

// ── Kalıcılık (sürüm alanlı serde gidiş-dönüş) ─────────────────────────────────

#[test]
fn kullanici_katmani_serde_gidis_donus() {
    let mut a = ayarlar();
    a.ayarla("gorunum.tema", AyarDeger::Secim("acik".into()));
    a.ayarla("performans.fps_goster", AyarDeger::Mantik(true));
    a.ayarla("performans.bellek_limiti_mb", AyarDeger::TamSayi(8192));
    let json = a.kullanici_json();

    let mut geri = ayarlar();
    assert!(geri.kullanici_yukle_json(&json));
    assert_eq!(geri.secim("gorunum.tema"), "acik");
    assert!(geri.mantik("performans.fps_goster"));
    assert_eq!(geri.tam_sayi("performans.bellek_limiti_mb"), 8192);
}

#[test]
fn bozuk_json_mevcut_korur() {
    let mut a = ayarlar();
    a.ayarla("gorunum.tema", AyarDeger::Secim("acik".into()));
    // Bozuk JSON yüklemesi başarısız olmalı ve mevcut değeri korumalı (güvenli).
    assert!(!a.kullanici_yukle_json("{ bu json degil"));
    assert_eq!(a.secim("gorunum.tema"), "acik");
}

#[test]
fn kayit_surum_alani_var() {
    let a = ayarlar();
    let k = a.kullanici_kaydi();
    assert_eq!(k.surum, KATMAN_SURUMU, "kalıcı kayıt sürüm alanlı olmalı");
}

// ── Kirlilik bayrağı ───────────────────────────────────────────────────────────

#[test]
fn kirlilik_degisiklikte_isaretlenir() {
    let mut a = ayarlar();
    assert!(!a.kirli_mi());
    a.ayarla("gorunum.tema", AyarDeger::Secim("acik".into()));
    assert!(a.kirli_mi());
    a.kirli_temizle();
    assert!(!a.kirli_mi());
    // Yükleme kirliliği temizler.
    a.ayarla("gorunum.tema", AyarDeger::Secim("yuksek_kontrast".into()));
    assert!(a.kirli_mi());
    a.kullanici_yukle(a.kullanici_kaydi());
    assert!(!a.kirli_mi());
}

// ── Profil dışa/içe aktarma (anahtar hariç — kabul kriteri) ────────────────────

#[test]
fn profil_disa_ice_anahtar_haric() {
    let mut a = ayarlar();
    a.ayarla("gorunum.tema", AyarDeger::Secim("acik".into()));
    a.ayarla("ai.api_anahtari", AyarDeger::Metin("ÇOK-GİZLİ".into()));
    let profil = a.profil_disa_aktar("Yedek");
    // Hassas alan profile girmemeli.
    assert!(!profil.degerler.contains_key("ai.api_anahtari"));
    assert!(profil.degerler.contains_key("gorunum.tema"));
    let j = profil.json().unwrap();
    assert!(!j.contains("ÇOK-GİZLİ"));

    // Taze depoya içe aktar → tema gelir, API anahtarı gelmez.
    let mut b = ayarlar();
    let n = b.profil_ice_aktar(&AyarProfili::jsondan(&j).unwrap());
    assert!(n >= 1);
    assert_eq!(b.secim("gorunum.tema"), "acik");
    assert_eq!(b.secim("ai.api_anahtari"), ""); // boş varsayılan
}

// ── Eklenti ayar kaydı (SDK akışı) ─────────────────────────────────────────────

#[test]
fn eklenti_ayari_eklentiler_kategorisine_kaydolur() {
    let mut a = ayarlar();
    let tanim = AyarTanimi::yeni_dis(
        "eklenti.ornek.parlaklik",
        AyarKategorisi::Gorunum, // yanlış kategori verilse bile Eklentiler'e zorlanır.
        AyarTuru::TamSayi {
            min: 0,
            max: 100,
            adim: 1,
        },
        AyarDeger::TamSayi(50),
        "Parlaklık",
        "Brightness",
        "Örnek eklenti ayarı.",
        "Sample plugin setting.",
    );
    assert!(a.eklenti_ayari_kaydet(tanim));
    // Çözülür + Eklentiler kategorisinde görünür.
    assert_eq!(a.tam_sayi("eklenti.ornek.parlaklik"), 50);
    let kategoride: Vec<&str> = a
        .kayit()
        .kategoride(AyarKategorisi::Eklentiler)
        .map(|t| t.anahtar.as_str())
        .collect();
    assert!(kategoride.contains(&"eklenti.ornek.parlaklik"));
    // Aynı anahtar tekrar kaydedilemez.
    let kopya = AyarTanimi::yeni_dis(
        "eklenti.ornek.parlaklik",
        AyarKategorisi::Eklentiler,
        AyarTuru::Mantik,
        AyarDeger::Mantik(false),
        "x",
        "x",
        "x",
        "x",
    );
    assert!(!a.eklenti_ayari_kaydet(kopya));
    // Arama indeksi de eklentiyi bulmalı.
    assert!(a
        .kayit()
        .indeks()
        .ara("parlaklık")
        .contains(&"eklenti.ornek.parlaklik"));
}

// ── Headless egui dumanı (panik = başarısız) ───────────────────────────────────

fn bir_kare(a: &mut Ayarlar, dil: Dil, tok: &Tokenlar) {
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            a.ciz(ui, dil, tok);
        });
    });
}

#[test]
fn ekran_her_kategoride_panik_olmadan_cizilir() {
    for &kat in AyarKategorisi::TUMU {
        let mut a = ayarlar();
        a.secili_kategori = kat;
        bir_kare(&mut a, Dil::Tr, &Tokenlar::koyu());
        bir_kare(&mut a, Dil::En, &Tokenlar::acik());
    }
}

#[test]
fn ekran_arama_ve_profil_acikken_cizilir() {
    let mut a = ayarlar();
    a.arama = "token".to_string();
    a.profil_panel_acik = true;
    a.profil_metin = a.profil_disa_aktar("t").json().unwrap();
    bir_kare(&mut a, Dil::Tr, &Tokenlar::yuksek_kontrast());
    bir_kare(&mut a, Dil::En, &Tokenlar::koyu());
}

#[test]
fn ekran_proje_katmaniyla_cizilir() {
    let mut a = ayarlar();
    a.proje_baslat();
    a.duzenleme_katmani_ayarla(AyarKatmani::Proje);
    bir_kare(&mut a, Dil::Tr, &Tokenlar::koyu());
}
