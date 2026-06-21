//! Editör modülü testleri (İP-06) — model + headless egui çizim panik-yok.

use super::*;
use crate::i18n::Dil;
use crate::tokens::Tokenlar;

/// Headless bir egui bağlamında bir closure'ı bir kare çalıştırır (çizim panik kontrolü).
fn kare<F: FnMut(&mut egui::Ui)>(mut f: F) {
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| f(ui));
    });
}

// ─── Metin tamponu ──────────────────────────────────────────────────────────

#[test]
fn duzenlenebilir_tampon_satirlari() {
    let t = MetinTampon::metinden("a\nbb\nccc".into());
    assert!(!t.salt_okunur());
    assert_eq!(t.satir_sayisi(), 3);
    assert_eq!(t.satir(1), "bb");
    assert_eq!(t.satir(9), ""); // sınır dışı
}

#[test]
fn bos_belge_python_varsayilan() {
    let b = Belge::bos();
    assert_eq!(b.kod_dili, KodDili::Python);
    assert!(!b.kirli);
    assert!(b.metin().is_some());
}

#[test]
fn ornek_belge_hucre_isaretleri_icerir() {
    let b = Belge::ornek_python();
    assert!(b.metin().unwrap().contains("# %%"));
}

// ─── Akış görüntüleyici (out-of-core) ───────────────────────────────────────

/// Geçici bir dosyaya N satır yazar, yolunu döner.
fn cok_satirli_dosya(ad: &str, n: usize) -> std::path::PathBuf {
    let yol = std::env::temp_dir().join(format!("biocraft_akis_{}_{ad}", std::process::id()));
    let mut s = String::new();
    for i in 0..n {
        s.push_str(&format!("satir {i} icerigi\n"));
    }
    std::fs::write(&yol, s).unwrap();
    yol
}

#[test]
fn akis_dogru_satir_sayisi_ve_rastgele_erisim() {
    // KABA_ARALIK'tan (1000) büyük → kaba indeks gerçekten kullanılır.
    let yol = cok_satirli_dosya("erisim", 2500);
    let g = AkisGoruntuleyici::ac(&yol).unwrap();
    assert_eq!(g.satir_sayisi(), 2500);
    // Kaba blok sınırının ötesinde rastgele erişim doğru olmalı.
    assert_eq!(g.satir(0), "satir 0 icerigi");
    assert_eq!(g.satir(1234), "satir 1234 icerigi");
    assert_eq!(g.satir(2499), "satir 2499 icerigi");
    let _ = std::fs::remove_file(&yol);
}

#[test]
fn akis_belge_salt_okunur() {
    let yol = cok_satirli_dosya("salt", 1500);
    let b = Belge::akis_dosyadan(&yol).unwrap();
    assert!(b.tampon.salt_okunur());
    assert!(b.metin().is_none()); // akış belgesi düzenlenemez
    assert_eq!(b.tampon.satir(10), "satir 10 icerigi");
    let _ = std::fs::remove_file(&yol);
}

#[test]
fn bos_dosya_akis_panik_yok() {
    let yol = std::env::temp_dir().join(format!("biocraft_bos_{}", std::process::id()));
    std::fs::write(&yol, b"").unwrap();
    let g = AkisGoruntuleyici::ac(&yol).unwrap();
    assert_eq!(g.bayt(), 0);
    assert_eq!(g.satir(0), "");
    let _ = std::fs::remove_file(&yol);
}

#[test]
fn dosyadan_kucuk_duzenlenebilir_buyuk_akis() {
    // Küçük dosya → düzenlenebilir.
    let kucuk = cok_satirli_dosya("kucuk", 10);
    let b = Belge::dosyadan(&kucuk).unwrap();
    assert!(!b.tampon.salt_okunur());
    let _ = std::fs::remove_file(&kucuk);
}

// ─── Hücre bulma ─────────────────────────────────────────────────────────────

#[test]
fn hucre_bul_imlece_gore() {
    let metin = "a = 1\n# %% iki\nb = 2\n# %% uc\nc = 3\n";
    // İlk hücre (imleç 0).
    assert!(hucre_bul(metin, 0).contains("a = 1"));
    assert!(!hucre_bul(metin, 0).contains("b = 2"));
    // İkinci hücre (imleç 'b = 2' civarı).
    let idx = metin.find("b = 2").unwrap();
    let h = hucre_bul(metin, idx);
    assert!(h.contains("b = 2"));
    assert!(!h.contains("a = 1"));
    assert!(!h.contains("c = 3"));
}

#[test]
fn hucre_bul_isaretsiz_tum_metin() {
    let metin = "x = 1\ny = 2\n";
    assert_eq!(hucre_bul(metin, 3), metin);
}

// ─── KodEditoru API ──────────────────────────────────────────────────────────

#[test]
fn editor_sekme_ac_kapat() {
    let mut e = KodEditoru::yeni();
    assert_eq!(e.belgeler.len(), 1);
    e.yeni_sekme();
    assert_eq!(e.belgeler.len(), 2);
    assert_eq!(e.aktif, 1);
    e.sekme_kapat(1);
    assert_eq!(e.belgeler.len(), 1);
    // Son sekme kapanınca boş bir tane bırakılır (editör hiç boş kalmaz).
    e.sekme_kapat(0);
    assert_eq!(e.belgeler.len(), 1);
}

#[test]
fn editor_dosya_ac_ayni_dosyayi_tekrar_acmaz() {
    let yol = cok_satirli_dosya("tekrar", 5);
    let mut e = KodEditoru::yeni();
    e.dosya_ac(&yol).unwrap();
    let sayi = e.belgeler.len();
    e.dosya_ac(&yol).unwrap(); // ikinci kez → yeni sekme açmaz, var olana geçer
    assert_eq!(e.belgeler.len(), sayi);
    let _ = std::fs::remove_file(&yol);
}

// ─── Headless çizim (panik yok) ──────────────────────────────────────────────

#[test]
fn editor_cizimi_iki_dil_panik_yok() {
    for dil in [Dil::Tr, Dil::En] {
        let tok = Tokenlar::koyu();
        let mut e = KodEditoru::ornek();
        kare(|ui| e.ciz(ui, dil, &tok));
    }
}

#[test]
fn editor_akis_belge_cizimi_panik_yok() {
    let yol = cok_satirli_dosya("ciz", 1200);
    let mut e = KodEditoru::yeni();
    e.belgeler.push(Belge::akis_dosyadan(&yol).unwrap());
    e.aktif = e.belgeler.len() - 1;
    let tok = Tokenlar::acik();
    kare(|ui| e.ciz(ui, Dil::Tr, &tok));
    let _ = std::fs::remove_file(&yol);
}

#[test]
fn layout_job_tum_satiri_kapsar() {
    // Vurgulama önbelleğinden kurulan job, her karakteri kapsamalı (boşluk dahil).
    let metin = "def f(x):  # not";
    let mut onbellek = VurgulamaOnbellek::yeni();
    onbellek.guncelle(metin, KodDili::Python, &BasitVurgulayici);
    let job = layout_job_kur(metin, &onbellek, &Tokenlar::koyu(), f32::INFINITY);
    // job.text birleşik metni içerir; satır metniyle eşleşmeli.
    assert_eq!(job.text, metin);
}
