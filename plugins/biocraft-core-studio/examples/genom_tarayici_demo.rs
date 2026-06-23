//! ÇE-02 (Gün 36) — **Genom Tarayıcı tuvali** uçtan uca demo (saf mantık; dosya/ağ yok).
//!
//! Çalıştır: `cargo run -p biocraft-core-studio --example genom_tarayici_demo`
//!
//! Tuvalin koordinat cetvelini, çok-iz yerleşimini, pan/zoom/"bölgeye git"/geri-ileri gezinmeyi,
//! tooltip/seçimi ve yoğun bölgede LOD (özet) davranışını **terminalde ASCII** olarak gösterir.
//! Gerçek uygulamada bu çizim listesi GPU ile (wgpu/egui) çizilir; burada sade görmek için ASCII.

use std::collections::BTreeMap;

use biocraft_core_studio::genome_browser::{
    CizimListesi, CizimRengi, GenomBolge, GenomTarayici, Iz, IzTuru, IzVeri, IzYer, OkumaParcasi,
    OzellikParcasi, Primitif, Serit,
};

const SUTUN: usize = 78;

fn main() {
    println!("=== BioCraft Studio — Genom Tarayıcı (ÇE-02, 1. kısım) ===\n");

    // 1) Tarayıcı: chr1:1-1000, 1000 px genişlik.
    let mut t = GenomTarayici::yeni(1000.0, GenomBolge::yeni("chr1", 1, 1000).unwrap());
    t.kromozom_uzunluklari_ayarla([("chr1".to_string(), 1_000_000u64)]);
    t.iz_ekle(Iz::yeni("kapsama", "Kapsama", IzTuru::Kapsama));
    t.iz_ekle(Iz::yeni("reads", "Okumalar", IzTuru::Hizalama));
    t.iz_ekle(Iz::yeni("genler", "Genler", IzTuru::Anotasyon));

    // 2) Sentetik veri (gerçekte BAM/GFF'ten out-of-core yüklenir).
    let okumalar = ornek_okumalar();
    let ozellikler = ornek_ozellikler();
    t.gen_cozucu_mut()
        .ekle("MYC", GenomBolge::yeni("chr1", 700, 760).unwrap());

    let veri = veri_haritasi(&okumalar, &ozellikler);

    bolum("1) Tuval açıldı + koordinat cetveli (bp/kb/Mb otomatik)");
    println!("   Bölge: {}", t.bolge().etiket());
    println!(
        "   Ölçek: {} • {} bp/piksel",
        t.cetvel().olcek.birim(),
        t.tuval().bp_per_piksel()
    );
    tuvali_ciz(&t, &veri);

    bolum("2) 'Bölgeye git': gen adı (MYC) → görünüm atlar");
    t.bolgeye_git("MYC").unwrap();
    println!("   Yeni bölge: {}", t.bolge().etiket());
    t.bolgeye_git("chr1:1-1000").unwrap(); // geri dön (geçmişe işler)

    bolum("3) Yakınlaştır/uzaklaştır + pan (kaydır)");
    t.yakinlastir_merkez(0.5);
    println!(
        "   Yakınlaştı → {} ({} bp)",
        t.bolge().etiket(),
        t.bolge().uzunluk()
    );
    t.pan_bp(200);
    println!("   Sağa kaydı → {}", t.bolge().etiket());

    bolum("4) Geri / İleri (gezinme geçmişi)");
    println!("   geri? {}  ileri? {}", t.geri_var_mi(), t.ileri_var_mi());
    t.geri();
    println!("   ← Geri: {}", t.bolge().etiket());
    t.geri();
    println!("   ← Geri: {}", t.bolge().etiket());
    t.ileri();
    println!("   → İleri: {}", t.bolge().etiket());

    // Görünümü başa al ve tooltip/seçim göster.
    t.bolgeye_git("chr1:1-1000").unwrap();
    let liste = t.derle(&veri);

    bolum("5) Tooltip (üzerine gel) + Seçim (inspector detayı)");
    let (sol, _) = t.tuval().aralik_ekran(100, 180);
    let yer = t
        .yerlesim()
        .into_iter()
        .find(|y| y.kimlik == "reads")
        .unwrap();
    let (mx, my) = (sol + 1.0, yer.y_ust + 1.0);
    if let Some(ip) = t.tooltip(&liste, mx, my) {
        println!("   Tooltip @({mx:.0},{my:.0}): {ip}");
    }
    t.sec(&liste, mx, my);
    if let Some(s) = t.secili() {
        println!("   Seçim detayı (inspector):");
        for satir in s.detay.lines() {
            println!("      {satir}");
        }
    }

    bolum("6) Yoğun bölge → LOD özeti (akıcılık korunur, MK-04)");
    let yogun: Vec<OkumaParcasi> = (1..=2000)
        .map(|i| OkumaParcasi {
            ad: format!("r{i}"),
            bas: i,
            bit: i + 80,
            serit: Serit::Ileri,
            mapq: Some(60),
        })
        .collect();
    let mut yveri: BTreeMap<String, IzVeri> = BTreeMap::new();
    yveri.insert("reads".into(), IzVeri::Hizalama(yogun.clone()));
    let mut t2 = GenomTarayici::yeni(100.0, GenomBolge::yeni("chr1", 1, 2100).unwrap());
    t2.iz_ekle(Iz::yeni("reads", "Okumalar", IzTuru::Hizalama));
    let yliste = t2.derle(&yveri);
    let ozet = yliste
        .primitifler
        .iter()
        .filter(|p| {
            matches!(
                p,
                Primitif::Dikdortgen {
                    renk: CizimRengi::OzetYogunluk,
                    ..
                }
            )
        })
        .count();
    println!(
        "   {} okuma, bütçe {} → tek tek isabet: {} (0 = özet moduna geçti), özet çubuğu: {}",
        yogun.len(),
        t2.oge_butcesi(),
        yliste.isabetler.len(),
        ozet
    );

    println!("\n=== Demo bitti — tüm gezinme/çizim saf mantıkta, GPU'ya hazır çizim listesi. ===");
}

fn bolum(baslik: &str) {
    println!("\n── {baslik} ──");
}

fn ornek_okumalar() -> Vec<OkumaParcasi> {
    vec![
        oku("read1", 100, 180, Serit::Ileri, 60),
        oku("read2", 130, 210, Serit::Geri, 60),
        oku("read3", 300, 380, Serit::Ileri, 5), // düşük kalite
        oku("read4", 500, 580, Serit::Geri, 60),
        oku("read5", 760, 840, Serit::Ileri, 60),
    ]
}

fn oku(ad: &str, bas: u64, bit: u64, serit: Serit, mapq: u8) -> OkumaParcasi {
    OkumaParcasi {
        ad: ad.into(),
        bas,
        bit,
        serit,
        mapq: Some(mapq),
    }
}

fn ornek_ozellikler() -> Vec<OzellikParcasi> {
    vec![
        OzellikParcasi {
            ad: Some("MYC".into()),
            bas: 100,
            bit: 900,
            serit: Serit::Ileri,
            tur: "gene".into(),
        },
        OzellikParcasi {
            ad: Some("ekson1".into()),
            bas: 100,
            bit: 250,
            serit: Serit::Ileri,
            tur: "exon".into(),
        },
        OzellikParcasi {
            ad: Some("ekson2".into()),
            bas: 700,
            bit: 900,
            serit: Serit::Ileri,
            tur: "exon".into(),
        },
    ]
}

fn veri_haritasi(
    okumalar: &[OkumaParcasi],
    ozellikler: &[OzellikParcasi],
) -> BTreeMap<String, IzVeri> {
    let mut v: BTreeMap<String, IzVeri> = BTreeMap::new();
    v.insert("kapsama".into(), IzVeri::Kapsama(okumalar.to_vec()));
    v.insert("reads".into(), IzVeri::Hizalama(okumalar.to_vec()));
    v.insert("genler".into(), IzVeri::Anotasyon(ozellikler.to_vec()));
    v
}

/// Çizim listesini terminale ASCII tuval olarak basar (cetvel + her iz bir/iki satır).
fn tuvali_ciz(t: &GenomTarayici, veri: &BTreeMap<String, IzVeri>) {
    let liste = t.derle(veri);
    let gen = t.tuval().genislik_px;
    let yerler = t.yerlesim();

    // Cetvel etiket satırı (büyük işaretler).
    let cetvel = t.cetvel();
    let mut etiket_satiri = vec![' '; SUTUN];
    for m in cetvel.isaretler.iter().filter(|m| m.buyuk) {
        let c = sutun(m.x_px, gen);
        for (i, ch) in m.etiket.chars().enumerate() {
            if c + i < SUTUN {
                etiket_satiri[c + i] = ch;
            }
        }
    }
    println!("   cetvel │{}│", etiket_satiri.iter().collect::<String>());

    // Her iz için bir satır: o izin dikey aralığına düşen dikdörtgenleri sütuna boya.
    for yer in &yerler {
        let satir = iz_satiri(&liste, yer, gen);
        let etiket = format!("{:>7}", kisalt(&yer.kimlik));
        println!("   {etiket} │{satir}│");
    }
}

fn iz_satiri(liste: &CizimListesi, yer: &IzYer, gen: f32) -> String {
    let mut sat = vec![' '; SUTUN];
    for p in &liste.primitifler {
        if let Primitif::Dikdortgen {
            x, y, gen: w, renk, ..
        } = p
        {
            // Bu dikdörtgen bu izin dikey bandına düşüyor mu?
            if *y < yer.y_ust - 0.5 || *y >= yer.y_alt() + 0.5 {
                continue;
            }
            let ch = match renk {
                CizimRengi::ReadIleri => '>',
                CizimRengi::ReadGeri => '<',
                CizimRengi::ReadDusuk => ':',
                CizimRengi::Ekson => '#',
                CizimRengi::KapsamaCubuk => '|',
                CizimRengi::OzetYogunluk => '+',
                _ => continue,
            };
            let bas = sutun(*x, gen);
            let son = sutun(x + w, gen).max(bas + 1);
            for s in sat.iter_mut().take(son.min(SUTUN)).skip(bas) {
                *s = ch;
            }
        }
        // Gen gövde çizgisi.
        if let Primitif::Cizgi {
            x1,
            x2,
            y1,
            renk: CizimRengi::Gen,
            ..
        } = p
        {
            if *y1 >= yer.y_ust - 0.5 && *y1 < yer.y_alt() + 0.5 {
                let bas = sutun(*x1, gen);
                let son = sutun(*x2, gen).max(bas + 1);
                for s in sat.iter_mut().take(son.min(SUTUN)).skip(bas) {
                    if *s == ' ' {
                        *s = '-';
                    }
                }
            }
        }
    }
    sat.iter().collect()
}

fn sutun(x: f32, gen: f32) -> usize {
    ((x / gen) * SUTUN as f32)
        .round()
        .clamp(0.0, (SUTUN - 1) as f32) as usize
}

fn kisalt(s: &str) -> &str {
    if s.len() > 7 {
        &s[..7]
    } else {
        s
    }
}
