//! ÇE-12 (Gün 43) demo — **çekirdek eklenti cilası**: performans + erişilebilirlik + doğruluk +
//! edge-case.  Render-bağımsız (GPU/egui gerekmez); konsola özet basar.
//!
//! Çalıştır:  `cargo run -p biocraft-core-studio --example cilala_demo`

use biocraft_core_studio::genome_browser::CizimRengi;
use biocraft_core_studio::perf::accessibility::{
    self, ErisilebilirlikAyari, KontrastSeviyesi, RenkGormeTuru, GENOM_KATEGORIK,
};
use biocraft_core_studio::perf::budget::{
    detay_sec, detay_uyarisi, KareButcesi, PerformansGostergesi,
};
use biocraft_core_studio::perf::correctness::{DogrulukRaporu, KoordinatTabani, ReferansArac};
use biocraft_core_studio::perf::edge::{self, VeriDurumu};
use biocraft_core_studio::perf::keyboard::{erisilebilirlik_denetimi, OdakHalkasi, OdakOgesi, Rol};

fn main() {
    println!("=== ÇE-12 — Çekirdek Eklenti Cilası (Gün 43) ===\n");

    // ── 1) Performans: kare bütçesi + uyarlamalı detay ───────────────────────
    println!("1) PERFORMANS (60 FPS kare bütçesi)");
    let b = KareButcesi::fps60();
    println!("   Kare bütçesi: {} µs (~16.7 ms)", b.us());
    let oge_us = 16.666; // ~1000 öğe sığar
    for oge in [500usize, 8_000, 200_000] {
        let d = detay_sec(oge, b, oge_us);
        let uyari = detay_uyarisi(oge, d).unwrap_or_else(|| "(uyarı yok)".into());
        println!("   {oge:>8} öğe → detay: {:<11} {uyari}", d.ad());
    }
    let mut g = PerformansGostergesi::yeni(8, b);
    for _ in 0..8 {
        g.kare_ekle(16_000);
    }
    println!("   Performans göstergesi: {}\n", g.ozet());

    // ── 2) Erişilebilirlik: renk körü palet + kontrast + klavye ──────────────
    println!("2) ERİŞİLEBİLİRLİK (MK-52)");
    println!("   Renk körü güvenli kategorik palet (Okabe-Ito alt kümesi):");
    for tur in [
        RenkGormeTuru::Normal,
        RenkGormeTuru::Protanopi,
        RenkGormeTuru::Deuteranopi,
    ] {
        let en_az = accessibility::en_yakin_cift(&GENOM_KATEGORIK, tur);
        println!("     {:<26} en yakın çift uzaklığı: {en_az:.0}", tur.ad());
    }
    let s_b = accessibility::kontrast_orani([0, 0, 0], [255, 255, 255]);
    println!(
        "   Kontrast (siyah/beyaz): {s_b:.1}:1 — AAA geçer: {}",
        accessibility::kontrast_gecer([0, 0, 0], [255, 255, 255], KontrastSeviyesi::Aaa)
    );
    let yk = accessibility::genom_paleti(&ErisilebilirlikAyari {
        yuksek_kontrast: true,
        ..Default::default()
    });
    println!(
        "   Yüksek kontrast modu: metin/zemin AAA geçer: {}",
        accessibility::kontrast_gecer(
            yk.rgb(CizimRengi::CetvelMetin),
            yk.zemin_rgb(),
            KontrastSeviyesi::Aaa
        )
    );
    let halka = OdakHalkasi::ogelerden(vec![
        OdakOgesi::yeni("ara", "Bölgeye git", Rol::GirisAlani).kisayol("Ctrl+G"),
        OdakOgesi::yeni("geri", "Geri", Rol::Buton).kisayol("Alt+Sol"),
        OdakOgesi::yeni("tuval", "Genom tuvali", Rol::Tuval).kisayol("Ok tuşları"),
    ]);
    println!(
        "   Klavye/ekran okuyucu denetimi temiz: {}",
        erisilebilirlik_denetimi(halka.ogeler()).temiz()
    );
    if let Some(o) = halka.odakli() {
        println!("   Odak anlatımı: \"{}\"\n", o.ekran_okuyucu_metni());
    }

    // ── 3) Doğruluk: referans araç parametre eşitleme + golden rapor ──────────
    println!("3) DOĞRULUK (golden — referans araçla parametre eşitleme)");
    let rapor = DogrulukRaporu::yeni(ReferansArac::Bcftools, "QUAL>=50 && FILTER=PASS")
        .parametre("koordinat", "1-tabanlı kapalı")
        .sonuc("167", "167");
    println!("   bcftools eşleşti: {}", rapor.eslesti());
    let s_uz = ReferansArac::Samtools.koordinat_tabani().uzunluk(100, 199);
    let (bb, be) = KoordinatTabani::SifirTabanliYariAcik.bir_tabanli_kapaliden(100, 199);
    let b_uz = ReferansArac::Bedtools.koordinat_tabani().uzunluk(bb, be);
    println!("   chr1:100-199 uzunluğu — samtools(1-tab)={s_uz}, bedtools(0-tab)={b_uz} (eşit)\n");

    // ── 4) Edge-case: boş/büyük + standart hata + Unicode ────────────────────
    println!("4) EDGE-CASE (Bölüm 0.12)");
    for n in [0u64, 1, 250_000, 9_000_000] {
        println!("   {n:>9} kayıt → durum: {:?}", VeriDurumu::siniflandir(n));
    }
    if let Some(r) = VeriDurumu::Bos.bos_rehberi("Varyant") {
        println!("   Boş rehberi: {r}");
    }
    let h = edge::bozuk_dosya("ornek.bam", Some(42), "beklenmedik EOF");
    println!(
        "   Bozuk dosya hatası → \"{}\" | çözüm: \"{}\" | id: {}",
        h.ne_oldu,
        h.eylem_etiketi.as_deref().unwrap_or("-"),
        h.correlation_id.kisa()
    );
    println!(
        "   Unicode ad korunur: {}",
        edge::guvenli_ad("hasta_çalışması_基因🧬", 64)
    );

    println!("\n=== Demo bitti — performans/erişilebilirlik/doğruluk/edge-case güvencesi ===");
}
