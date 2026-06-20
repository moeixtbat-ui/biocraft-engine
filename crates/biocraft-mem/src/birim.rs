//! Bayt birimlerini insan-okunur biçime çevirme (KB/MB/GB/TB).
//!
//! Bütçe diyaloğu, hata mesajları ve durum çubuğu hep aynı biçimi kullansın diye
//! tek yerde toplanmıştır.  Saf fonksiyon — egui'siz, tam test edilebilir.

/// Bir bayt sayısını okunaklı bir dizgeye çevirir (örn. `1536` → `"1.5 KB"`).
///
/// 1024 tabanlı (ikilik) birimler kullanılır; tek ondalık basamak gösterilir.
pub fn insan_bayt(bayt: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let b = bayt as f64;
    if b >= TB {
        format!("{:.1} TB", b / TB)
    } else if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bayt} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bayt_altinda_kb_oldugu_gibi_gosterilir() {
        assert_eq!(insan_bayt(0), "0 B");
        assert_eq!(insan_bayt(512), "512 B");
        assert_eq!(insan_bayt(1023), "1023 B");
    }

    #[test]
    fn kb_mb_gb_tb_dogru_olcek() {
        assert_eq!(insan_bayt(1024), "1.0 KB");
        assert_eq!(insan_bayt(1536), "1.5 KB");
        assert_eq!(insan_bayt(1024 * 1024), "1.0 MB");
        assert_eq!(insan_bayt(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(insan_bayt(1024u64.pow(4)), "1.0 TB");
    }

    #[test]
    fn dort_tb_dosya_tb_ile_gosterilir() {
        // MK-09: 4 TB dosya senaryosu — birim taşması olmamalı.
        let dort_tb = 4 * 1024u64.pow(4);
        assert_eq!(insan_bayt(dort_tb), "4.0 TB");
    }
}
