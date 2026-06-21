//! İP-10 gizlilik uçtan uca demo (MK-41/42/43/34).
//!
//! Çalıştır: `cargo run -p biocraft-data --example gizlilik_demo`
//!
//! Gösterilenler:
//! 1. PHI verinin **her** dış kanaldan çekirdek tarafından engellenmesi (atlanamaz).
//! 2. Yerel-varsayılan profilin dış gönderimi durdurması; projenin global'i ezmesi.
//! 3. Normal verinin açık kanalda **onay** istemesi + onay defterine işlenmesi.
//! 4. Per-veri köken + köken gezgini (kaynak/sürüm/lisans/atıf).
//! 5. Haklar: envanter (erişim) + tam ihraç (taşınabilirlik) + güvenli silme (unutulma).

use std::fs;
use std::path::PathBuf;

use biocraft_data::privacy::{
    self, classify::DisKanal, consent::GonderimOzeti, profile::GizlilikProfili,
    provenance::LisansAtif,
};
use biocraft_data::{olustur, ProjeKurulumGirdisi};
use biocraft_types::{DataClassification, Version};

fn gecici() -> PathBuf {
    let p = std::env::temp_dir().join(format!("bc_gizlilik_demo_{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn ayrac(baslik: &str) {
    println!("\n──────── {baslik} ────────");
}

fn main() {
    let konum = gecici();

    // PHI sınıflı bir proje kur.
    let g = ProjeKurulumGirdisi::yeni(
        "Gizlilik Demo",
        &konum,
        "genomik",
        DataClassification::HasasPhi,
        Version::new(0, 1, 0),
    );
    let kurulan = olustur(&g).expect("proje kurulmalı");
    let kok = kurulan.kok.clone();
    println!("Proje kuruldu: {}", kok.display());

    // Birkaç kullanıcı dosyası + köken kaydı.
    fs::write(kok.join("data/inputs/hasta.vcf"), b"##fileformat=VCFv4.2\n").unwrap();
    fs::write(kok.join("data/inputs/dbsnp.vcf"), b"##dbSNP\n").unwrap();

    privacy::koken_ekle(
        &kok,
        &privacy::VeriKokeni::yeni(
            "data/inputs/hasta.vcf",
            "Kullanıcı yüklemesi (klinik)",
            "11".repeat(32),
            DataClassification::HasasPhi,
        ),
    )
    .unwrap();
    privacy::koken_ekle(
        &kok,
        &privacy::VeriKokeni::yeni(
            "data/inputs/dbsnp.vcf",
            "NCBI dbSNP",
            "22".repeat(32),
            DataClassification::Normal,
        )
        .surum_ile("build 156")
        .lisans_ile(LisansAtif {
            lisans: "Public Domain".into(),
            atif: "Sherry ST et al., dbSNP, Nucleic Acids Res. 2001".into(),
            url: Some("https://www.ncbi.nlm.nih.gov/snp/".into()),
        }),
    )
    .unwrap();

    // ── 1) PHI her dış kanaldan ENGELLENİR (MK-43) ──
    ayrac("1) PHI çıkış kapısı (atlanamaz)");
    for kanal in DisKanal::TUMU {
        let karar = privacy::cikis_denetle(DataClassification::HasasPhi, kanal);
        let durum = if karar.engellendi_mi() {
            "ENGELLENDİ ⛔"
        } else {
            "izinli"
        };
        println!("  PHI → {:<28} {durum}", kanal.ad());
    }

    // ── 2) Yerel-varsayılan profil + proje override ──
    ayrac("2) Profil: yerel-varsayılan vs proje override");
    let global = GizlilikProfili::varsayilan_global();
    println!(
        "  Global varsayılan: tamamen_yerel={}, hiçbir dış kanal açık değil",
        global.tamamen_yerel
    );

    let acilan = biocraft_data::ac(&kok).unwrap();
    let etkin = privacy::proje_ile_coz(&global, Some(&acilan.manifest.gizlilik));
    println!(
        "  Bu projenin etkin profili (manifest [gizlilik] global'i ezer): tamamen_yerel={}, P2P={}, AI havuzu={}",
        etkin.tamamen_yerel,
        etkin.dis_kanal_etkin_mi(DisKanal::P2p),
        etkin.dis_kanal_etkin_mi(DisKanal::DisAi),
    );

    // ── 3) Normal veri açık kanalda ONAY ister ──
    ayrac("3) Dış gönderim onay akışı (üç kapı)");
    // İllüstrasyon için kanalları açık bir profil kullan.
    let acik = GizlilikProfili {
        tamamen_yerel: false,
        ai_havuzu_katki: true,
        dagitik_ag_etkin: true,
        ..GizlilikProfili::varsayilan_global()
    };

    // 3a) PHI → AI: kanal açık olsa bile ENGELLENİR.
    let phi_talep = privacy::OnayTalebi::yeni(
        DisKanal::DisAi,
        "AI havuzu",
        "Model eğitimine katkı",
        GonderimOzeti {
            oge_sayisi: 1,
            siniflar: vec![DataClassification::HasasPhi],
            boyut_bayt: 2048,
            alanlar: vec!["hasta_no".into(), "genotip".into()],
        },
    );
    match privacy::gonderim_degerlendir(&acik, phi_talep) {
        privacy::GonderimDurumu::Engellendi(r) => {
            println!(
                "  PHI → AI havuzu: {} ({})",
                r.ne_oldu,
                r.eylem_etiketi.as_deref().unwrap_or("")
            )
        }
        _ => println!("  ! beklenmeyen: PHI gönderilebildi"),
    }

    // 3b) Normal → NCBI: onay bekler → kullanıcı onaylar → deftere işlenir.
    let normal_talep = privacy::OnayTalebi::yeni(
        DisKanal::DisApi,
        "NCBI BLAST",
        "Dizi benzerlik araması",
        GonderimOzeti {
            oge_sayisi: 1,
            siniflar: vec![DataClassification::Normal],
            boyut_bayt: 512,
            alanlar: vec!["dizi".into()],
        },
    );
    match privacy::gonderim_degerlendir(&acik, normal_talep) {
        privacy::GonderimDurumu::OnayBekliyor(talep) => {
            println!(
                "  Normal → NCBI: ONAY BEKLİYOR — gönderilecek: {}",
                talep.ozet.ozet_satiri()
            );
            // Kullanıcı "Evet" der → gönderim yapılır + denetim izine yazılır.
            let kayit = privacy::OnayKaydi::yeni(&talep, privacy::OnayKarari::Onaylandi);
            privacy::onay_ekle(&kok, &kayit).unwrap();
            println!("  Kullanıcı onayladı → onay defterine işlendi.");
        }
        privacy::GonderimDurumu::KanalKapali(r) => println!("  Normal → NCBI: {}", r.ne_oldu),
        privacy::GonderimDurumu::Engellendi(r) => println!("  Normal → NCBI: {}", r.ne_oldu),
    }

    // ── 4) Köken gezgini ──
    ayrac("4) Köken gezgini (kaynak / sürüm / lisans)");
    let gezgin = privacy::KokenGezgini::yeni(privacy::kokenleri_oku(&kok).unwrap());
    for s in gezgin.satirlar() {
        println!(
            "  {:<26} kaynak={:<28} sürüm={:<12} lisans={:<14} [{}]",
            s.veri_kimligi,
            s.kaynak,
            if s.surum.is_empty() { "—" } else { &s.surum },
            s.lisans_ozet,
            s.sinif_ad
        );
    }
    println!("  Yöntem bölümü atıfları:");
    for a in gezgin.atiflar() {
        println!("    • {a}");
    }

    // ── 5) Haklar: envanter + tam ihraç + güvenli silme ──
    ayrac("5) KVKK/GDPR hakları");
    let env = privacy::veri_envanteri(&kok).unwrap();
    println!(
        "  Erişim/envanter: {} dosya, toplam {} bayt",
        env.ogeler.len(),
        env.toplam_bayt
    );

    let hedef = konum.join("tam_ihrac.zip");
    let rapor =
        privacy::tam_veri_ihrac(&kok, &hedef, &privacy::export::IhracSecenekleri::default())
            .unwrap();
    println!(
        "  Taşınabilirlik (tam ihraç): {} → {} dosya, {} bayt (manifest filtresiz)",
        hedef.display(),
        rapor.dosya_sayisi,
        rapor.boyut_bayt
    );

    let silme = privacy::guvenli_sil(&kok, &privacy::export::SilmeSecenekleri::default()).unwrap();
    println!(
        "  Unutulma (güvenli silme): {} dosya silindi, {} bayt üzerine yazıldı; klasör kaldırıldı={}",
        silme.silinen_dosya, silme.uzerine_yazilan_bayt, !kok.exists()
    );

    let _ = fs::remove_dir_all(&konum);
    println!(
        "\nDemo tamam. Çekirdek sınırı PHI'yi her dış kanalda durdurdu; haklar uçtan uca çalıştı."
    );
}
