//! İP-09 güvenlik katmanı uçtan uca demo (MK-44/45/20).
//!
//! Çalıştır: `cargo run -p biocraft-data --example guvenlik_demo`
//!
//! Gösterilenler:
//! 1. **Dinlenmede şifreleme:** hassas veri AES-256-GCM ile şifrelenir; anahtar (bu demoda bellek-içi)
//!    güvenli depodadır; doğru anahtarla çözülür, yanlışla/kurcalanınca **reddedilir**.
//! 2. **Güvenli silme:** dosya üzerine yazılır + kaldırılır (gerçekten gider).
//! 3. **İmzalı güncelleme:** sahte/değiştirilmiş güncelleme **reddedilir**, gerçek imzalı kabul edilir.
//! 4. **Log temizleme:** PII (e-posta/yol/hasta no) loglarda maskelenir.
//!
//! (Sandbox sertleştirme = zip-bomb reddi, eklenti host katmanındadır:
//!  `cargo run -p biocraft-plugin-host --example eklenti_host_demo`.)
//!
//! NOT: Veri-güvenlik kodunun TAMAMI açıktır (Kerckhoffs — MK-20). Kapalı olan yalnız lisans/anti-tamper.

use std::fs;
use std::path::PathBuf;

use biocraft_data::security::{
    self,
    crypto::{self, VeriAnahtari},
    integrity::{guncelleme_dogrula, GuncellemeBildirimi},
    secure_delete::{guvenli_dosya_sil, UzerineYazSecenek},
    BellekKimlikDeposu,
};
use ed25519_dalek::{Signer, SigningKey};

fn ayrac(baslik: &str) {
    println!("\n──────── {baslik} ────────");
}

fn gecici_dosya(ad: &str, icerik: &[u8]) -> PathBuf {
    let p = std::env::temp_dir().join(format!("bc_guvenlik_demo_{}_{}", ad, std::process::id()));
    fs::write(&p, icerik).unwrap();
    p
}

fn main() {
    println!("BioCraft Engine — İP-09 Güvenlik Katmanı Demosu");

    // ── 1. Dinlenmede şifreleme + OS güvenli anahtar deposu ──
    ayrac("1) AES-256-GCM dinlenmede şifreleme (anahtar güvenli depoda)");
    let depo = BellekKimlikDeposu::yeni(); // çalışma-zamanında OsKimlikDeposu (Credential Manager)
    let hesap = "proje:demo:veri";
    let anahtar = security::anahtar_kur_veya_yukle(&depo, hesap).unwrap();
    let hassas = b"PHI: hasta dogum 1990-01-01, BRCA1 c.68_69delAG, tani C50.9";
    let zarf = crypto::sifrele(&anahtar, hassas).unwrap();
    println!("  Düz metin   : {} bayt (hassas)", hassas.len());
    println!(
        "  Şifreli     : {} bayt  nonce={:02x?}...",
        zarf.sifreli_metin.len(),
        &zarf.nonce[..4]
    );
    // Anahtarı depodan TEKRAR yükle (yeni oturum) → çöz.
    let anahtar2 = security::anahtar_kur_veya_yukle(&depo, hesap).unwrap();
    let cozulen = crypto::coz(&anahtar2, &zarf).unwrap();
    println!("  Çözülen     : '{}'", String::from_utf8_lossy(&cozulen));
    assert_eq!(cozulen, hassas);

    // Yanlış anahtar / kurcalama reddedilir.
    let yabanci = VeriAnahtari::rastgele().unwrap();
    println!(
        "  Yanlış anahtarla çözme   : {}",
        if crypto::coz(&yabanci, &zarf).is_err() {
            "REDDEDİLDİ ✔"
        } else {
            "açıldı ✗"
        }
    );
    let mut kurcalanmis = zarf.clone();
    kurcalanmis.sifreli_metin[0] ^= 0x01;
    println!(
        "  Kurcalanmış veriyi çözme : {}",
        if crypto::coz(&anahtar, &kurcalanmis).is_err() {
            "REDDEDİLDİ ✔ (GCM bütünlüğü)"
        } else {
            "açıldı ✗"
        }
    );

    // ── 2. Güvenli silme ──
    ayrac("2) Güvenli silme (üzerine yaz + kaldır)");
    let dosya = gecici_dosya("sil", b"hassas icerik diskte kaliyordu");
    println!("  Dosya var mı (silme öncesi) : {}", dosya.is_file());
    let yazilan = guvenli_dosya_sil(&dosya, &UzerineYazSecenek::default()).unwrap();
    println!("  Üzerine yazılan bayt        : {yazilan}");
    println!(
        "  Dosya var mı (silme sonrası): {} (silindi ✔)",
        dosya.is_file()
    );

    // ── 3. İmzalı güncelleme bütünlüğü ──
    ayrac("3) İmzalı güncelleme — sahte güncelleme reddedilir");
    let resmi = SigningKey::from_bytes(&[42u8; 32]); // demo: resmi yayın anahtarı
    let paket = b"BioCraft Engine 1.2.0 - gercek surum ikilisi";
    let bildirim = GuncellemeBildirimi {
        surum: "1.2.0".into(),
        blake3_hex: blake3::hash(paket).to_hex().to_string(),
        boyut: paket.len() as u64,
    };
    let imza = resmi.sign(&bildirim.kanonik()).to_bytes().to_vec();
    let vk = resmi.verifying_key().to_bytes().to_vec();
    println!(
        "  Gerçek imzalı güncelleme : {}",
        if guncelleme_dogrula(paket, &bildirim, &imza, &vk).is_ok() {
            "KABUL ✔"
        } else {
            "red ✗"
        }
    );
    let sahte_paket = b"KOTU AMACLI degistirilmis paket!!";
    println!(
        "  Değiştirilmiş paket      : {}",
        if guncelleme_dogrula(sahte_paket, &bildirim, &imza, &vk).is_err() {
            "REDDEDİLDİ ✔ (checksum tutmadı)"
        } else {
            "kabul ✗"
        }
    );
    let saldirgan = SigningKey::from_bytes(&[9u8; 32]);
    let sahte_imza = saldirgan.sign(&bildirim.kanonik()).to_bytes().to_vec();
    println!(
        "  Sahte anahtarla imza     : {}",
        if guncelleme_dogrula(paket, &bildirim, &sahte_imza, &vk).is_err() {
            "REDDEDİLDİ ✔ (resmi anahtar değil)"
        } else {
            "kabul ✗"
        }
    );

    // ── 4. Log / PII temizleme ──
    ayrac("4) Log temizleme — PII maskelenir");
    let ham_log = r"hata: hasta@klinik.com C:\Users\Ali\hasta_12345678.vcf okunamadi";
    let temiz = security::pii_temizle(ham_log);
    println!("  Ham log   : {ham_log}");
    println!("  Temiz log : {temiz}");

    println!("\n✅ İP-09 güvenlik katmanı uçtan uca çalıştı (şifreleme + güvenli silme + imzalı güncelleme + log).");
}
