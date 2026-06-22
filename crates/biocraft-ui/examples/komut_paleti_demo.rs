//! İP-13 (Gün 25) demosu — **komut paleti (bulanık arama) + klavye kısayolları + tuş seti**.
//!
//! Çalıştırma:
//! ```text
//! cargo run -p biocraft-ui --example komut_paleti_demo
//! ```
//! Arayüzsüz (headless); gerçek pencere açmaz.  Kabul kriterlerini uçtan uca gösterir:
//! bulanık arama hızı (<50 ms), menü↔palet tek tanım (MK-51), kısayol yeniden atama + çakışma +
//! varsayılana dön, eklenti komutu palette + kısayol atanabilir, override kalıcılığı.

use std::time::Instant;

use biocraft_ui::command::{bulanik_skor, EklentiKomut};
use biocraft_ui::{
    Dil, KabukAksiyon, Kisayol, KisayolHaritasi, Komut, KomutKaynak, KomutPaleti, TusSetiProfili,
};

fn baslik(s: &str) {
    println!("\n========== {s} ==========");
}

/// Bir oturumun komut kümesini kurar (kabuk aksiyonları + örnek eklenti komutu).
fn komut_kumesi(harita: &KisayolHaritasi) -> Vec<Komut> {
    let mut v: Vec<Komut> = KabukAksiyon::tumu()
        .iter()
        .map(|&a| {
            let ks = harita.kisayol(&KomutKaynak::Kabuk(a)).map(|k| k.goster());
            Komut::kabuktan(a, Dil::Tr, ks, a.etkin_mi())
        })
        .collect();
    let ek = EklentiKomut::yeni("biocraft.ornek.selam", "Örnek: Selam Ver");
    let ks = harita
        .kisayol(&KomutKaynak::Eklenti(ek.kimlik.clone()))
        .map(|k| k.goster());
    v.push(Komut::eklentiden(&ek, ks));
    v
}

/// Bir sorgu için en iyi N komutu (bulanık skor) sıralı döndürür.
fn ara<'a>(komutlar: &'a [Komut], sorgu: &str, n: usize) -> Vec<&'a Komut> {
    let mut puanli: Vec<(i32, &Komut)> = komutlar
        .iter()
        .filter(|k| k.etkin)
        .filter_map(|k| bulanik_skor(sorgu, &k.ad).map(|r| (r.skor, k)))
        .collect();
    puanli.sort_by_key(|(s, _)| std::cmp::Reverse(*s));
    puanli.into_iter().take(n).map(|(_, k)| k).collect()
}

fn main() {
    println!("BioCraft Engine — İP-13 demosu (komut paleti + klavye kısayolları)");

    let mut harita = KisayolHaritasi::varsayilan(TusSetiProfili::Modern);
    let komutlar = komut_kumesi(&harita);
    println!(
        "Komut kümesi: {} komut (kabuk + 1 örnek eklenti).",
        komutlar.len()
    );

    // ─── 1) Bulanık arama + HIZ (<50 ms p99) ───────────────────────────────────
    baslik("1) Bulanık arama (<50 ms)");
    for sorgu in ["tema", "böl", "ayar", "selam", ">kod"] {
        let temiz = sorgu.trim_start_matches('>');
        let sonuc = ara(&komutlar, temiz, 3);
        let liste: Vec<&str> = sonuc.iter().map(|k| k.ad.as_str()).collect();
        println!("  '{sorgu}' → {liste:?}");
    }
    // Hız: tüm komutlarda 5000 kez arama → ortalama mikro-saniye.
    let baslangic = Instant::now();
    let tekrar = 5000;
    let mut toplam = 0usize;
    for _ in 0..tekrar {
        toplam += ara(&komutlar, "ed", 5).len();
    }
    let gecen = baslangic.elapsed();
    println!(
        "  Hız: {tekrar} arama × {} komut = {:.1} µs/arama (toplam {toplam} eşleşme).",
        komutlar.len(),
        gecen.as_secs_f64() * 1e6 / tekrar as f64
    );

    // ─── 2) MK-51: menü ile palet AYNI komut tanımı ────────────────────────────
    baslik("2) Tek komut tanımı (menü ↔ palet, MK-51)");
    let kaydet = komutlar
        .iter()
        .find(|k| k.kaynak == KomutKaynak::Kabuk(KabukAksiyon::Kaydet))
        .unwrap();
    println!(
        "  Paletteki 'Kaydet' → {:?}; menü etiketi: '{}'; kısayol ipucu: {:?}",
        kaydet.kaynak,
        KabukAksiyon::Kaydet.etiket(Dil::Tr),
        kaydet.kisayol
    );

    // ─── 3) Klavye kısayolu çözümü ─────────────────────────────────────────────
    baslik("3) Kısayol → komut çözümü");
    for s in ["Ctrl+S", "Ctrl+Shift+P", "Ctrl+J", "Ctrl+\\"] {
        let ks = Kisayol::ayristir(s).unwrap();
        println!("  {s:<14} → {:?}", harita.cozumle(&ks));
    }

    // ─── 4) Yeniden atama + çakışma uyarısı + varsayılana dön ───────────────────
    baslik("4) Yeniden atama + çakışma + varsayılana dön");
    let cakisanlar = harita.ata(
        KomutKaynak::Kabuk(KabukAksiyon::YeniSekme),
        Kisayol::ayristir("Ctrl+S").unwrap(),
    );
    println!(
        "  'Yeni Sekme' → Ctrl+S atandı.  Çakışma: {:?}",
        cakisanlar.iter().map(|k| k.anahtar()).collect::<Vec<_>>()
    );
    println!("  Toplam çakışma sayısı: {}", harita.cakismalar().len());
    harita.varsayilana_don(&KomutKaynak::Kabuk(KabukAksiyon::YeniSekme));
    println!(
        "  Varsayılana dönüldü → 'Yeni Sekme' kısayolu: {:?}",
        harita
            .kisayol(&KomutKaynak::Kabuk(KabukAksiyon::YeniSekme))
            .map(|k| k.goster())
    );

    // ─── 5) Eklenti komutu palette + kısayol atanabilir ────────────────────────
    baslik("5) Eklenti komutu (palette + kısayol)");
    let ek = KomutKaynak::Eklenti("biocraft.ornek.selam".into());
    harita.ata(ek.clone(), Kisayol::ayristir("Ctrl+Alt+G").unwrap());
    let yeniden = komut_kumesi(&harita);
    let ek_komut = yeniden.iter().find(|k| k.kaynak == ek).unwrap();
    println!(
        "  '{}' palette görünür; atanan kısayol: {:?}; Ctrl+Alt+G → {:?}",
        ek_komut.ad,
        ek_komut.kisayol,
        harita.cozumle(&Kisayol::ayristir("Ctrl+Alt+G").unwrap())
    );

    // ─── 6) Override kalıcılığı (oturumlar arası) ──────────────────────────────
    baslik("6) Override kalıcılığı (JSON gidiş-dönüş)");
    let json = harita.override_json();
    println!("  Kaydedilen override JSON: {json}");
    let mut taze = KisayolHaritasi::varsayilan(TusSetiProfili::Modern);
    taze.override_json_uygula(&json);
    println!(
        "  Yeniden yüklendi → Ctrl+Alt+G → {:?} (eklenti kısayolu korundu).",
        taze.cozumle(&Kisayol::ayristir("Ctrl+Alt+G").unwrap())
    );

    // ─── 7) Son/sık kullanım belleği ───────────────────────────────────────────
    baslik("7) Son/sık kullanılanlar (palet sıralaması)");
    let mut palet = KomutPaleti::yeni();
    palet.ac(komut_kumesi(&harita));
    palet.kullanildi(&KomutKaynak::Kabuk(KabukAksiyon::Ayarlar));
    palet.kullanildi(&KomutKaynak::Kabuk(KabukAksiyon::TemaDegistir));
    println!(
        "  Palet açık: {}; iki komut 'kullanıldı' olarak işaretlendi (üst sırada görünür).",
        palet.acik
    );

    println!("\nTüm kabul kriterleri uçtan uca gösterildi (İP-13).");
}
