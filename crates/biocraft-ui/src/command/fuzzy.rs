//! Saf-Rust **bulanık (fuzzy) alt-dizi eşleştirici** — komut paleti için (İP-13).
//!
//! **Karar (Gün 25):** Spec `nucleo`'yu önerir; ancak proje "yeni dış bağımlılık YOK + saf Rust +
//! C derleyici gerekmez" ilkesini sürdürür (Gün 22'deki Tree-sitter→saf-Rust kararıyla aynı çizgi).
//! Komut kümesi küçük (~birkaç düzine) olduğundan basit ama iyi-puanlayan bir alt-dizi eşleştirici
//! **<50 ms p99**'u fazlasıyla karşılar (binlerce komutta bile mikro-saniyeler).  Eşleştirici **saf
//! fonksiyon** olarak yazıldı → ileride istenirse `nucleo` ile sorunsuz değiştirilebilir (API sabit).
//!
//! Eşleştirme: sorgu (`desen`) karakterleri adayda (`metin`) **sırayla** geçmeli (alt-dizi).  Puan;
//! ardışık eşleşme, sözcük başı / CamelCase sınırı ve baştan eşleşme bonuslarıyla, aradaki atlanan
//! karakterler için (sınırlı) ceza ile hesaplanır → "spl" → "Split Editor" gibi kısaltmaları bulur.

/// Tek bir bulanık eşleşmenin sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulanikSonuc {
    /// Eşleşme puanı (yüksek = daha iyi).
    pub skor: i32,
    /// Eşleşen karakterlerin adaydaki (orijinal) konumları — kalın vurgulama için.
    pub indeksler: Vec<usize>,
}

// Puanlama ağırlıkları (deneyimsel; komut paleti için dengeli).
const ESLESME: i32 = 4; // her eşleşen karakterin taban puanı
const BASTAN: i32 = 16; // adayın 0. konumunda eşleşme (en güçlü sinyal)
const SOZCUK_BASI: i32 = 12; // ayraçtan sonra veya CamelCase sınırında eşleşme
const ARDISIK: i32 = 8; // bir önceki eşleşmenin hemen ardından
const BOSLUK_CEZA: i32 = -1; // atlanan her karakter için (sınırlı)
const BOSLUK_TABAN: i32 = -9; // tek bir boşluğun verebileceği en büyük ceza

/// Bir karakterin "sözcük ayracı" sayılıp sayılmadığı (sonraki harf sözcük başı olur).
fn ayirici(c: char) -> bool {
    matches!(c, ' ' | '_' | '-' | '.' | '/' | ':' | '\\' | '(' | ')')
}

/// Bir karakteri tek bir küçük-harf karaktere indirger (indeks hizası korunur).
fn kucuk(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// `desen`'in `metin` içinde (sırayla) geçip geçmediğini sınar; geçiyorsa puanlar.
///
/// Boş desen → `Some` (puan 0, indeks yok): "filtre yok" anlamına gelir.  Eşleşme yoksa `None`.
/// Karşılaştırma büyük/küçük harf duyarsızdır; eşleşen indeksler **orijinal** karakter konumlarıdır.
pub fn bulanik_skor(desen: &str, metin: &str) -> Option<BulanikSonuc> {
    let d: Vec<char> = desen
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(kucuk)
        .collect();
    if d.is_empty() {
        return Some(BulanikSonuc {
            skor: 0,
            indeksler: Vec::new(),
        });
    }
    let h: Vec<char> = metin.chars().collect();
    let hl: Vec<char> = h.iter().map(|&c| kucuk(c)).collect();

    let mut indeksler = Vec::with_capacity(d.len());
    let mut skor = 0i32;
    let mut konum = 0usize;
    let mut onceki: Option<usize> = None;

    for &dc in &d {
        // `dc`'yi `konum`'dan itibaren ara; aradaki karakterleri say (boşluk cezası).
        let mut atlanan = 0i32;
        let bulunan = loop {
            if konum >= hl.len() {
                return None; // desenin kalan karakteri bulunamadı → eşleşme yok.
            }
            if hl[konum] == dc {
                break konum;
            }
            konum += 1;
            atlanan += 1;
        };

        let mut s = ESLESME;
        if bulunan == 0 {
            s += BASTAN;
        } else {
            let onc = h[bulunan - 1];
            if ayirici(onc) {
                s += SOZCUK_BASI;
            } else if onc.is_lowercase() && h[bulunan].is_uppercase() {
                s += SOZCUK_BASI; // camelCase sınırı
            }
        }
        if let Some(p) = onceki {
            if bulunan == p + 1 {
                s += ARDISIK;
            }
        }
        s += (atlanan * BOSLUK_CEZA).max(BOSLUK_TABAN);

        skor += s;
        indeksler.push(bulunan);
        onceki = Some(bulunan);
        konum = bulunan + 1;
    }

    Some(BulanikSonuc { skor, indeksler })
}

/// "Şunu mu demek istediniz?" için **gevşek benzerlik**: desendeki (boşluksuz) ayrı karakterlerden
/// kaçı adayda (sıra önemsiz) geçiyor.  Alt-dizi eşleşmesi olmadığında en yakın komutu önermek için.
pub fn gevsek_benzerlik(desen: &str, metin: &str) -> usize {
    let h: Vec<char> = metin.chars().map(kucuk).collect();
    let mut sayim = 0;
    let mut gorulen = String::new();
    for c in desen.chars().filter(|c| !c.is_whitespace()).map(kucuk) {
        if gorulen.contains(c) {
            continue; // aynı karakteri bir kez say (küme yaklaşımı)
        }
        gorulen.push(c);
        if h.contains(&c) {
            sayim += 1;
        }
    }
    sayim
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bos_desen_filtre_yok() {
        let r = bulanik_skor("", "Herhangi").unwrap();
        assert_eq!(r.skor, 0);
        assert!(r.indeksler.is_empty());
    }

    #[test]
    fn eslesmeyen_none_doner() {
        assert!(bulanik_skor("xyz", "Split Editor").is_none());
        // Sıra önemli: "tes" "Set" içinde (t..e..s sırasıyla) yok.
        assert!(bulanik_skor("tes", "Set").is_none());
    }

    #[test]
    fn kisaltma_eslesir_ve_indeksler_dogru() {
        // "spl" → "Split" baştan ardışık eşleşir.
        let r = bulanik_skor("spl", "Split Editor").unwrap();
        assert_eq!(r.indeksler, vec![0, 1, 2]);
        assert!(r.skor > 0);
    }

    #[test]
    fn sozcuk_basi_camelcase_tercih_edilir() {
        // "oc" → "OpenChart": O (baş) + C (camelCase sınırı) yüksek puan almalı,
        // aynı harfleri ortada barındıran bir adaydan daha yüksek.
        let iyi = bulanik_skor("oc", "OpenChart").unwrap().skor;
        let zayif = bulanik_skor("oc", "monocode").unwrap().skor;
        assert!(
            iyi > zayif,
            "camelCase/başı tercih edilmeli: {iyi} > {zayif}"
        );
    }

    #[test]
    fn ardisik_eslesme_daha_yuksek() {
        // Bitişik "tema" (t-e-m-a ardışık) → boşluklu "tma" (e atlanır) eşleşmesinden yüksek.
        let bitisik = bulanik_skor("tema", "Tema Değiştir").unwrap().skor;
        let dagiik = bulanik_skor("tma", "Tema Değiştir").unwrap().skor;
        assert!(
            bitisik > dagiik,
            "ardışık daha yüksek olmalı: {bitisik} > {dagiik}"
        );
    }

    #[test]
    fn buyuk_kucuk_harf_duyarsiz() {
        assert!(bulanik_skor("SPLIT", "split editor").is_some());
        assert!(bulanik_skor("split", "SPLIT EDITOR").is_some());
    }

    #[test]
    fn gevsek_benzerlik_oneri_icin() {
        // Hiç alt-dizi eşleşmesi yokken en yakın aday önerilebilsin.
        assert!(gevsek_benzerlik("tena", "Tema") >= 3);
        assert_eq!(gevsek_benzerlik("xyz", "Tema"), 0);
    }
}
