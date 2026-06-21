//! Proje formatı uçtan-uca demo (İP-02, Gün 17).
//!
//! Çalıştır: `cargo run -p biocraft-data --example proje_demo`
//!
//! Gösterir: (1) proje kurulumu → açık klasör + `biocraft.toml`; (2) açılışta BLAKE3 bütünlük
//! denetimi; (3) bozuk dosyada net hata (sessiz açma yok); (4) tek dosya `.bcproj` dışa aktarımı
//! (hassas ayar hariç).

use std::fs;
use std::path::Path;

use biocraft_data::biocraft_types::{DataClassification, Version};
use biocraft_data::project::{self, DisaAktarSecenekleri, ProjeKurulumGirdisi};

fn main() {
    let taban = std::env::temp_dir().join(format!("biocraft_demo_{}", std::process::id()));
    let _ = fs::remove_dir_all(&taban);
    fs::create_dir_all(&taban).unwrap();
    println!("Demo çalışma klasörü: {}\n", taban.display());

    // ── 1) Kurulum ──────────────────────────────────────────────────────────
    let mut girdi = ProjeKurulumGirdisi::yeni(
        "İnsan Genomu Çalışması",
        &taban,
        "genomik",
        DataClassification::HasasPhi,
        Version::new(0, 1, 0),
    );
    girdi.orcid = Some("0000-0002-1825-0097".to_string());
    girdi.kurum = "Örnek Üniversitesi".to_string();
    girdi.etiketler = vec!["genom".into(), "phi".into()];
    girdi.uyumluluk = vec!["Akademik".into(), "GDPR-OK".into()];
    girdi.lisans = "CC-BY-4.0".into();

    let kurulan = project::olustur(&girdi).expect("kurulum başarısız");
    println!("[1] Proje oluşturuldu → {}", kurulan.kok.display());
    println!("    Klasör yapısı:");
    yazdir_agac(&kurulan.kok, 2);
    println!();

    // ── 2) Açılış + bütünlük denetimi ───────────────────────────────────────
    let acilan = project::ac(&kurulan.kok).expect("açılış başarısız");
    println!(
        "[2] Açıldı + bütünlük DOĞRULANDI ✓  (sınıf={:?}, format={}, göç_kaydı={}, uyarı={})",
        acilan.manifest.siniflandirma.sinif,
        acilan.manifest.kimlik.format_surumu,
        acilan.manifest.goc.len(),
        acilan.uyarilar.len(),
    );
    println!(
        "    Manifest: ORCID={:?}, şifreli={}",
        acilan.manifest.olusturan.orcid,
        acilan
            .manifest
            .guvenlik
            .map(|g| g.sifreleme)
            .unwrap_or(false),
    );
    println!();

    // ── 3) Bozuk dosya → net hata ───────────────────────────────────────────
    let manifest_yol = project::format::manifest_yolu(&kurulan.kok);
    let mut metin = fs::read_to_string(&manifest_yol).unwrap();
    metin.push_str("\n# elle kurcalandı\n");
    fs::write(&manifest_yol, metin).unwrap();
    match project::ac(&kurulan.kok) {
        Ok(_) => println!("[3] HATA: bozuk manifest sessizce açıldı (OLMAMALIYDI!)"),
        Err(h) => println!(
            "[3] Bozuk manifest yakalandı ✓  → \"{}\" (çözüm: {})",
            h.ne_oldu, h.nasil_cozulur
        ),
    }
    // Manifesti onar (özet yeniden tutsun diye projeyi yeniden kur).
    let _ = fs::remove_dir_all(&kurulan.kok);
    let kurulan = project::olustur(&girdi).unwrap();
    println!();

    // ── 4) .bcproj dışa aktarımı (hassas ayar hariç) ────────────────────────
    fs::write(
        kurulan.kok.join("data/inputs/notlar.txt"),
        b"kucuk ornek veri",
    )
    .unwrap();
    let hedef = taban.join("paket.bcproj");
    let rapor = project::disa_aktar(&kurulan.kok, &hedef, &DisaAktarSecenekleri::default())
        .expect("export başarısız");
    let zip = fs::read(&hedef).unwrap();
    let guvenlik_sizdi = zip.windows(10).any(|w| w == b"[guvenlik]");
    println!(
        "[4] Dışa aktarıldı → {} ({} dosya, {} bayt)",
        hedef.display(),
        rapor.dosya_sayisi,
        rapor.boyut_bayt
    );
    println!(
        "    Hassas [guvenlik] sızdı mı? {}  | hassas_haric={}",
        if guvenlik_sizdi {
            "EVET (HATA)"
        } else {
            "HAYIR ✓"
        },
        rapor.hassas_haric,
    );

    println!("\nDemo tamamlandı. (Geçici klasör: {})", taban.display());
}

/// Bir klasör ağacını basitçe yazdırır (girinti = derinlik).
fn yazdir_agac(dizin: &Path, girinti: usize) {
    let Ok(girdiler) = fs::read_dir(dizin) else {
        return;
    };
    let mut yollar: Vec<_> = girdiler.flatten().map(|g| g.path()).collect();
    yollar.sort();
    for yol in yollar {
        let ad = yol.file_name().unwrap_or_default().to_string_lossy();
        let isaret = if yol.is_dir() { "📁" } else { "📄" };
        println!("{:girinti$}{isaret} {ad}", "", girinti = girinti);
        if yol.is_dir() {
            yazdir_agac(&yol, girinti + 2);
        }
    }
}
