//! Golden (altın referans) test çerçevesi — bilimsel/çıktı doğruluğunun güvencesi
//! (İP-21, MK-58).
//!
//! Bir hesaplama/serileştirme çıktısını **diske kayıtlı referans** ile karşılaştırır.
//! Referans, samtools/bcftools/IGV gibi altın-standart bir araçtan ya da bilinen-doğru bir
//! örnekten alınır (çekirdek eklenti — ÇE — bunu kullanır).  Bu çerçeve `insta`'ya benzer ama
//! **harici bağımlılık eklemez** (saf Rust): referanslar düz metin dosyalarıdır.
//!
//! ## Kullanım
//! ```ignore
//! golden::dogrula("vcf_ozet", &cikti);
//! ```
//! - Referans yoksa: `BIOCRAFT_GOLDEN_UPDATE=1` ile **oluşturulur** (ilk yazım); aksi hâlde
//!   test **başarısız** olur (yanlışlıkla referans üretmeyi önler).
//! - Referans varsa ve farklıysa: net bir **fark (diff)** ile panik → testin amacı budur.
//! - `BIOCRAFT_GOLDEN_UPDATE=1` ayarlıysa: referans **güncellenir** (bilinçli yenileme).
//!
//! ## Kırılganlığa karşı normalizasyon
//! Zaman damgası, korelasyon/iz kimliği, mutlak yol gibi **gürültülü** alanlar
//! [`Normalize`] ile sabit belirteçlere indirgenmeli → golden yalnızca **anlamlı** farkta kırılır
//! (Muhtemel hata: "Golden test kırılgan" → çözümü: gürültülü alanları normalize et).

use std::path::PathBuf;

/// Golden referanslarının kök dizini (her crate'in `tests/golden/` klasörü).
const GOLDEN_DIZIN: &str = "tests/golden";

/// `BIOCRAFT_GOLDEN_UPDATE` ayarlı mı? (referansları yaz/güncelle)
pub fn guncelleme_modu() -> bool {
    std::env::var("BIOCRAFT_GOLDEN_UPDATE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Gürültülü/zamana-bağlı alanları sabit belirteçlere indirgeyen normalizasyon zinciri.
///
/// Yerleşik kurallar idempotenttir; istenildiği kadar zincirlenebilir.
#[derive(Debug, Clone, Default)]
pub struct Normalize {
    girdi: String,
}

impl Normalize {
    /// Ham çıktıdan başlar.
    pub fn yeni(girdi: impl Into<String>) -> Self {
        Self {
            girdi: girdi.into(),
        }
    }

    /// RFC3339 / ISO-8601 zaman damgalarını `<ZAMAN>` yapar.
    pub fn zaman_damgalari(mut self) -> Self {
        self.girdi = zaman_maskele(&self.girdi);
        self
    }

    /// UUID (8-4-4-4-12) ve 32-hex iz kimliklerini `<KIMLIK>` yapar.
    pub fn kimlikler(mut self) -> Self {
        self.girdi = uuid_maskele(&self.girdi);
        self.girdi = uzun_hex_maskele(&self.girdi);
        self
    }

    /// Verilen alt dizgeyi sabit bir belirteçle değiştirir (ör. mutlak proje yolu).
    pub fn degistir(mut self, ara: &str, yerine: &str) -> Self {
        self.girdi = self.girdi.replace(ara, yerine);
        self
    }

    /// Normalize edilmiş metni döndürür.
    pub fn bitir(self) -> String {
        self.girdi
    }
}

/// `actual`'ı `ad` referansıyla karşılaştırır.  Çağıran crate'in dizininden göreli çözer.
///
/// Eşleşmezse panik (testi başarısız kılar).  Güncelleme modunda referansı yazar/günceller.
pub fn dogrula(ad: &str, actual: &str) {
    let yol = referans_yolu(ad);
    let mevcut = std::fs::read_to_string(&yol).ok();

    match mevcut {
        Some(beklenen) if beklenen == actual => { /* eşleşti — başarı */ }
        Some(beklenen) => {
            if guncelleme_modu() {
                yaz(&yol, actual);
            } else {
                panic!(
                    "GOLDEN UYUŞMAZLIK '{ad}'\n\
                     Referans: {}\n\
                     Güncellemek için: BIOCRAFT_GOLDEN_UPDATE=1\n\
                     {}",
                    yol.display(),
                    fark(&beklenen, actual)
                );
            }
        }
        None => {
            if guncelleme_modu() {
                yaz(&yol, actual);
            } else {
                panic!(
                    "GOLDEN YOK '{ad}' ({})\n\
                     İlk kez oluşturmak için: BIOCRAFT_GOLDEN_UPDATE=1\n\
                     Üretilecek çıktı:\n{actual}",
                    yol.display()
                );
            }
        }
    }
}

/// Referans dosya yolu: `<CARGO_MANIFEST_DIR>/tests/golden/<ad>.txt`.
fn referans_yolu(ad: &str) -> PathBuf {
    let kok = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(kok)
        .join(GOLDEN_DIZIN)
        .join(format!("{ad}.txt"))
}

fn yaz(yol: &PathBuf, icerik: &str) {
    if let Some(ana) = yol.parent() {
        let _ = std::fs::create_dir_all(ana);
    }
    std::fs::write(yol, icerik)
        .unwrap_or_else(|e| panic!("golden yazılamadı {}: {e}", yol.display()));
}

/// Basit satır-bazlı fark gösterimi (ilk farklı satırı işaretler).
fn fark(beklenen: &str, gercek: &str) -> String {
    let b: Vec<&str> = beklenen.lines().collect();
    let g: Vec<&str> = gercek.lines().collect();
    let n = b.len().max(g.len());
    let mut cikti = String::from("--- fark (- beklenen / + gerçek) ---\n");
    let mut farkli = 0;
    for i in 0..n {
        let bs = b.get(i).copied().unwrap_or("<yok>");
        let gs = g.get(i).copied().unwrap_or("<yok>");
        if bs != gs {
            cikti.push_str(&format!("satır {}:\n  - {bs}\n  + {gs}\n", i + 1));
            farkli += 1;
            if farkli >= 10 {
                cikti.push_str("… (daha fazla fark gizlendi)\n");
                break;
            }
        }
    }
    cikti
}

// ─── Normalizasyon yardımcıları ───────────────────────────────────────────────

/// RFC3339 benzeri zaman damgalarını maskele (ör. `2026-06-23T12:00:00+00:00`).
fn zaman_maskele(girdi: &str) -> String {
    let mut cikti = String::with_capacity(girdi.len());
    let ch: Vec<char> = girdi.chars().collect();
    let mut i = 0;
    while i < ch.len() {
        if zaman_damgasi_baslar(&ch, i) {
            cikti.push_str("<ZAMAN>");
            // Zaman damgası karakterlerini atla (rakam, '-', ':', 'T', 'Z', '+', '.').
            while i < ch.len() && zaman_karakteri(ch[i]) {
                i += 1;
            }
        } else {
            cikti.push(ch[i]);
            i += 1;
        }
    }
    cikti
}

/// `YYYY-MM-DDT` deseniyle başlıyor mu?
fn zaman_damgasi_baslar(ch: &[char], i: usize) -> bool {
    if i + 10 >= ch.len() {
        return false;
    }
    ch[i..i + 4].iter().all(|c| c.is_ascii_digit())
        && ch[i + 4] == '-'
        && ch[i + 5].is_ascii_digit()
        && ch[i + 6].is_ascii_digit()
        && ch[i + 7] == '-'
        && ch[i + 8].is_ascii_digit()
        && ch[i + 9].is_ascii_digit()
        && ch[i + 10] == 'T'
}

fn zaman_karakteri(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '-' | ':' | 'T' | 'Z' | '+' | '.')
}

/// UUID (8-4-4-4-12 hex) desenlerini `<KIMLIK>` yapar.
fn uuid_maskele(girdi: &str) -> String {
    let parcalar: Vec<&str> = girdi
        .split(|c: char| !(c.is_ascii_hexdigit() || c == '-'))
        .collect();
    let mut sonuc = girdi.to_string();
    for p in parcalar {
        if uuid_mi(p) {
            sonuc = sonuc.replace(p, "<KIMLIK>");
        }
    }
    sonuc
}

fn uuid_mi(s: &str) -> bool {
    let p: Vec<&str> = s.split('-').collect();
    p.len() == 5
        && [8, 4, 4, 4, 12] == [p[0].len(), p[1].len(), p[2].len(), p[3].len(), p[4].len()]
        && p.iter().all(|x| x.chars().all(|c| c.is_ascii_hexdigit()))
}

/// ≥32 ardışık hex karakteri (iz kimliği) `<KIMLIK>` yapar.
fn uzun_hex_maskele(girdi: &str) -> String {
    let mut cikti = String::with_capacity(girdi.len());
    let mut tampon = String::new();
    let bosalt = |cikti: &mut String, tampon: &mut String| {
        if tampon.len() >= 32 && tampon.chars().all(|c| c.is_ascii_hexdigit()) {
            cikti.push_str("<KIMLIK>");
        } else {
            cikti.push_str(tampon);
        }
        tampon.clear();
    };
    for c in girdi.chars() {
        if c.is_ascii_hexdigit() {
            tampon.push(c);
        } else {
            bosalt(&mut cikti, &mut tampon);
            cikti.push(c);
        }
    }
    bosalt(&mut cikti, &mut tampon);
    cikti
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zaman_damgasi_normalize() {
        let n = Normalize::yeni("olay 2026-06-23T12:34:56+00:00 oldu")
            .zaman_damgalari()
            .bitir();
        assert_eq!(n, "olay <ZAMAN> oldu");
    }

    #[test]
    fn uuid_normalize() {
        let n = Normalize::yeni("id=550e8400-e29b-41d4-a716-446655440000 bitti")
            .kimlikler()
            .bitir();
        assert_eq!(n, "id=<KIMLIK> bitti");
    }

    #[test]
    fn uzun_hex_iz_normalize() {
        let hex = "a".repeat(32);
        let n = Normalize::yeni(format!("trace_id={hex}"))
            .kimlikler()
            .bitir();
        assert_eq!(n, "trace_id=<KIMLIK>");
    }

    #[test]
    fn degistir_kurali() {
        let n = Normalize::yeni(r"yol=C:\Users\Furkan\proje")
            .degistir(r"C:\Users\Furkan", "<KOK>")
            .bitir();
        assert_eq!(n, r"yol=<KOK>\proje");
    }

    #[test]
    fn fark_ilk_farkli_satiri_gosterir() {
        let f = fark("a\nb\nc", "a\nX\nc");
        assert!(f.contains("satır 2"));
        assert!(f.contains("- b"));
        assert!(f.contains("+ X"));
    }

    #[test]
    fn guncelleme_modu_varsayilan_kapali() {
        // Test ortamında değişken ayarlı değilse kapalı olmalı.
        if std::env::var("BIOCRAFT_GOLDEN_UPDATE").is_err() {
            assert!(!guncelleme_modu());
        }
    }
}
