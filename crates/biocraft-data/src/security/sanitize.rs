//! **Log / PII temizleme** — loglar hassas veri (PII/PHI) içermez (MK-45, İP-09).
//!
//! Hata raporları ve günlükler destek/teşhis için paylaşılabilir; bu yüzden içlerine **hasta
//! tanımlayıcısı, dosya yolu, e-posta, olası sağlık kimliği** sızmamalıdır.  Bu modül, loglanacak
//! bir metni en iyi-çaba **maskeler** (tam çözüm değil; "güvenli varsayılan" — şüpheli kalıplar
//! `[gizlendi]` ile değiştirilir).
//!
//! Tasarım: regex bağımlılığı eklemeden, std ile basit/öngörülebilir kalıp tarama.  Amaç kusursuz
//! PII tespiti değil (imkânsız); **kaza eseri sızıntıyı azaltmak** + hata raporunu opt-in/anonim tutmak.

/// Maskeleme yer tutucusu.
const GIZLI: &str = "[gizlendi]";

/// Bir log/teknik-detay metnini en iyi-çaba temizler: e-posta, mutlak yol ve uzun rakam dizilerini
/// (olası hasta no/kimlik/telefon) maskeler.
pub fn pii_temizle(metin: &str) -> String {
    let mut cikti = String::with_capacity(metin.len());
    for kelime in metin.split_inclusive(char::is_whitespace) {
        // Sondaki boşluğu ayır (geri ekleyeceğiz).
        let (govde, bosluk) = ayir_bosluk(kelime);
        if govde_pii_mi(govde) {
            cikti.push_str(GIZLI);
        } else {
            cikti.push_str(govde);
        }
        cikti.push_str(bosluk);
    }
    cikti
}

/// Bir "kelime"nin (boşlukla ayrılmış parça) PII benzeri olup olmadığına karar verir.
fn govde_pii_mi(govde: &str) -> bool {
    if govde.is_empty() {
        return false;
    }
    eposta_mi(govde) || mutlak_yol_mu(govde) || uzun_rakam_dizisi_mi(govde)
}

fn eposta_mi(s: &str) -> bool {
    // En basit kural: bir '@' ve ondan sonra bir '.' (kullanıcı@alan.tld).
    if let Some(at) = s.find('@') {
        let alan = &s[at + 1..];
        return !s[..at].is_empty() && alan.contains('.') && !alan.starts_with('.');
    }
    false
}

fn mutlak_yol_mu(s: &str) -> bool {
    // Unix mutlak yol (/home/...), UNC (\\sunucu\...) veya Windows sürücü yolu (C:\...).
    s.starts_with('/') && s.len() > 1
        || s.starts_with("\\\\")
        || (s.len() >= 3
            && s.as_bytes()[1] == b':'
            && (s.as_bytes()[2] == b'\\' || s.as_bytes()[2] == b'/'))
}

fn uzun_rakam_dizisi_mi(s: &str) -> bool {
    // 7+ haneli bitişik rakam grubu (telefon/hasta no/TC kimlik benzeri) → maskele.
    let mut ardisik = 0usize;
    for b in s.bytes() {
        if b.is_ascii_digit() {
            ardisik += 1;
            if ardisik >= 7 {
                return true;
            }
        } else {
            ardisik = 0;
        }
    }
    false
}

/// Bir parçayı gövde + sonundaki boşluk olarak ayırır.
fn ayir_bosluk(parca: &str) -> (&str, &str) {
    match parca.char_indices().rev().find(|(_, c)| !c.is_whitespace()) {
        Some((i, c)) => {
            let bol = i + c.len_utf8();
            (&parca[..bol], &parca[bol..])
        }
        None => ("", parca),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eposta_maskelenir() {
        let s = pii_temizle("kullanici hasta@ornek.com ile islem yapti");
        assert!(!s.contains("hasta@ornek.com"));
        assert!(s.contains("[gizlendi]"));
        assert!(s.contains("kullanici") && s.contains("islem"));
    }

    #[test]
    fn mutlak_yol_maskelenir() {
        let s = pii_temizle(r"dosya C:\Users\Furkan\hasta_kayit.vcf acildi");
        assert!(!s.contains("Furkan"));
        assert!(s.contains("[gizlendi]"));

        let u = pii_temizle("dosya /home/ali/dna.bam okundu");
        assert!(!u.contains("/home/ali"));
        assert!(u.contains("[gizlendi]"));
    }

    #[test]
    fn uzun_rakam_maskelenir() {
        let s = pii_temizle("hasta no 1234567 kaydedildi");
        assert!(!s.contains("1234567"));
        assert!(s.contains("[gizlendi]"));
    }

    #[test]
    fn normal_metin_korunur() {
        let s = "varyant BRCA1 c.68_69delAG islendi 42 kez";
        // Kısa sayı (42) ve normal kelimeler korunmalı.
        assert_eq!(pii_temizle(s), s);
    }

    #[test]
    fn bosluk_yapisi_korunur() {
        // Maskeleme kelime sayısını/boşlukları bozmamalı (log okunabilirliği).
        let s = pii_temizle("a b@c.d e");
        assert_eq!(s, "a [gizlendi] e");
    }

    #[test]
    fn bos_metin() {
        assert_eq!(pii_temizle(""), "");
    }
}
