//! **Delta güncelleme** — yalnızca *değişen parçayı* taşıyan, saf-Rust ikili yama (İP-20, MK-56).
//!
//! Auto-update bant genişliğini düşürür: kullanıcı tüm paketi değil, eski paketle yeni paket
//! arasındaki **farkı** indirir.  Yama, eski paketin baytlarına uygulanarak yeni paket **birebir**
//! yeniden üretilir.  Format kasıtlı olarak basit + **deterministik**tir (golden-test edilebilir):
//!
//! - Yeni paket, sabit boyutlu bloklar (`BLOK`) hâlinde taranır.
//! - Bir blok eski pakette de varsa → **`Kopya`** (yalnız ofset+uzunluk; bayt taşınmaz) ve eşleşme
//!   bayt bayt uzatılır.
//! - Eşleşmeyen baytlar → **`Ekle`** (yeni içerik; tek taşınan kısım budur).
//!
//! **Güvenlik:** yama uygulanırken hem **kaynak** (eski paket) hem **sonuç** (yeni paket) BLAKE3
//! özetiyle doğrulanır.  Yanlış tabana uygulanan ya da kurcalanmış bir yama **net hata** verir ve
//! sonuç **asla** teslim edilmez (atomik güncellemenin bütünlük ayağı — `guncelleme_dogrula` imza
//! ayağını tamamlar).  C derleyici/dış bağımlılık gerekmez (proje ilkesi: yeni dış bağımlılık yok).

use serde::{Deserialize, Serialize};

use biocraft_types::ErrorReport;

/// Blok eşleştirme boyutu (bayt).  Küçük = daha iyi sıkıştırma + daha çok op; büyük = tersi.
/// 1 KiB, ikili paketler için makul bir denge; format bu sabitten bağımsız çalışır (yama kendini
/// taşır), yalnız `uret` tarafının seçimidir.
const BLOK: usize = 1024;

/// Bir delta yamasının tek adımı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeltaOp {
    /// Eski paketten `[ofset, ofset+uzunluk)` aralığını **olduğu gibi** kopyala (bayt taşınmaz).
    Kopya { ofset: u64, uzunluk: u64 },
    /// Yeni içerik — taşınan tek bayt grubu budur.
    Ekle(Vec<u8>),
}

/// Eski paketten yeni paketi üreten, **kendini doğrulayan** ikili yama.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeltaYama {
    /// Uygulanacağı **eski** paketin beklenen BLAKE3 özeti (yanlış tabana uygulama engellenir).
    pub eski_blake3: String,
    /// Üretilecek **yeni** paketin beklenen BLAKE3 özeti (sonuç doğrulanır).
    pub yeni_blake3: String,
    /// Yeni paketin boyutu (bayt) — ön denetim + sonuç tutarlılığı.
    pub yeni_boyut: u64,
    /// Yama adımları (sırayla uygulanır).
    pub ops: Vec<DeltaOp>,
}

impl DeltaYama {
    /// Bu yamada **gerçekten taşınan** bayt sayısı (yalnız `Ekle` içerikleri) — delta kazancı ölçüsü.
    pub fn tasinan_bayt(&self) -> usize {
        self.ops
            .iter()
            .map(|op| match op {
                DeltaOp::Ekle(b) => b.len(),
                DeltaOp::Kopya { .. } => 0,
            })
            .sum()
    }

    /// Yamanın serileştirilmiş (JSON) boyutu yaklaşık olarak indirilecek veriyi temsil eder.
    pub fn yaklasik_indirme_boyutu(&self) -> usize {
        serde_json::to_vec(self).map(|v| v.len()).unwrap_or(0)
    }

    /// Yamayı JSON baytlarına serileştirir (release hattı/dağıtım dosyası için).
    pub fn json(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).unwrap_or_default()
    }
}

/// Eski → yeni paket için bir delta yaması **üretir** (deterministik).
///
/// Strateji: yeni paketi blok blok tara; her blok eski pakette varsa kopyala (ve eşleşmeyi bayt
/// bayt uzat), yoksa literal biriktir.  Üretim *tek yönlüdür* — yama her zaman `uygula` ile birebir
/// yeni paketi geri verir (round-trip testleriyle garanti).
pub fn uret(eski: &[u8], yeni: &[u8]) -> DeltaYama {
    // Eski paketin blok başlangıçlarını özetlerine göre indeksle (ilk görülen ofset tutulur →
    // deterministik).
    use std::collections::HashMap;
    let mut indeks: HashMap<[u8; 32], usize> = HashMap::new();
    if eski.len() >= BLOK {
        let mut i = 0;
        while i + BLOK <= eski.len() {
            let h = *blake3::hash(&eski[i..i + BLOK]).as_bytes();
            indeks.entry(h).or_insert(i);
            i += BLOK;
        }
    }

    let mut ops: Vec<DeltaOp> = Vec::new();
    let mut bekleyen: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < yeni.len() {
        let mut eslesti = false;
        if i + BLOK <= yeni.len() {
            let h = *blake3::hash(&yeni[i..i + BLOK]).as_bytes();
            if let Some(&off) = indeks.get(&h) {
                // Özet çakışmasına karşı baytları gerçekten doğrula.
                if eski[off..off + BLOK] == yeni[i..i + BLOK] {
                    // Bekleyen literal varsa önce onu yaz.
                    if !bekleyen.is_empty() {
                        ops.push(DeltaOp::Ekle(std::mem::take(&mut bekleyen)));
                    }
                    // Eşleşmeyi olabildiğince uzat.
                    let mut uzunluk = BLOK;
                    while off + uzunluk < eski.len()
                        && i + uzunluk < yeni.len()
                        && eski[off + uzunluk] == yeni[i + uzunluk]
                    {
                        uzunluk += 1;
                    }
                    ops.push(DeltaOp::Kopya {
                        ofset: off as u64,
                        uzunluk: uzunluk as u64,
                    });
                    i += uzunluk;
                    eslesti = true;
                }
            }
        }
        if !eslesti {
            bekleyen.push(yeni[i]);
            i += 1;
        }
    }
    if !bekleyen.is_empty() {
        ops.push(DeltaOp::Ekle(bekleyen));
    }

    DeltaYama {
        eski_blake3: blake3::hash(eski).to_hex().to_string(),
        yeni_blake3: blake3::hash(yeni).to_hex().to_string(),
        yeni_boyut: yeni.len() as u64,
        ops,
    }
}

/// Eski paket baytlarına yamayı uygular ve **doğrulanmış** yeni paketi döndürür.
///
/// Sıra (en ucuz/en kesin reddi önce): **taban özeti** → **sınır denetimli uygula** → **sonuç
/// özeti + boyut**.  Herhangi biri tutmazsa net hata; sonuç **asla** kısmen teslim edilmez.
pub fn uygula(eski: &[u8], yama: &DeltaYama) -> Result<Vec<u8>, ErrorReport> {
    // 1) Taban: bu yama bu eski pakete mi ait?
    let eski_ozet = blake3::hash(eski);
    if !eski_ozet
        .to_hex()
        .as_str()
        .eq_ignore_ascii_case(yama.eski_blake3.trim())
    {
        return Err(taban_hatasi());
    }

    // 2) Op'ları sınır denetimiyle uygula.
    let mut cikti = Vec::with_capacity(yama.yeni_boyut as usize);
    for op in &yama.ops {
        match op {
            DeltaOp::Kopya { ofset, uzunluk } => {
                let bas = *ofset as usize;
                let son = bas
                    .checked_add(*uzunluk as usize)
                    .ok_or_else(|| bozuk_yama_hatasi("Kopya aralığı taşması"))?;
                if son > eski.len() {
                    return Err(bozuk_yama_hatasi(
                        "Kopya aralığı eski paketin dışına taşıyor",
                    ));
                }
                cikti.extend_from_slice(&eski[bas..son]);
            }
            DeltaOp::Ekle(b) => cikti.extend_from_slice(b),
        }
    }

    // 3) Sonuç: yeniden üretilen paket beklenen boyut + özetle birebir mi?
    if cikti.len() as u64 != yama.yeni_boyut {
        return Err(bozuk_yama_hatasi("Yama sonucu beklenen boyutta değil"));
    }
    let yeni_ozet = blake3::hash(&cikti);
    if !yeni_ozet
        .to_hex()
        .as_str()
        .eq_ignore_ascii_case(yama.yeni_blake3.trim())
    {
        return Err(bozuk_yama_hatasi(
            "Yama sonucu beklenen özetle uyuşmuyor (kurcalanmış yama)",
        ));
    }

    Ok(cikti)
}

fn taban_hatasi() -> ErrorReport {
    ErrorReport::new(
        "Delta güncelleme uygulanamadı (taban uyuşmuyor)",
        "Bu delta yaması, kurulu sürümünüzün paketine ait değil; aradaki bir sürümü atlamış \
         olabilirsiniz.",
        "Tam (delta olmayan) güncelleme paketini indirin; uygulama bunu otomatik dener.",
    )
}

fn bozuk_yama_hatasi(detay: &str) -> ErrorReport {
    ErrorReport::new(
        "Delta güncelleme reddedildi (bozuk yama)",
        "Delta yaması eksik/bozuk ya da değiştirilmiş; sonuç paket güvenle yeniden üretilemedi.",
        "Güncellemeyi yeniden indirin; sorun sürerse tam paketi tercih edin.",
    )
    .with_teknik_detay(detay.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        blake3::hash(b).to_hex().to_string()
    }

    #[test]
    fn ozdes_round_trip() {
        let eski = vec![1u8; 4096];
        let yama = uret(&eski, &eski);
        let sonuc = uygula(&eski, &yama).unwrap();
        assert_eq!(sonuc, eski);
        // Özdeş içerik → hiç literal taşınmaz (tamamı kopya).
        assert_eq!(yama.tasinan_bayt(), 0);
    }

    #[test]
    fn ortadan_degisiklik_round_trip() {
        let mut eski = vec![0u8; 8192];
        for (i, b) in eski.iter_mut().enumerate() {
            *b = (i % 251) as u8;
        }
        let mut yeni = eski.clone();
        // Ortada küçük bir blok değiştir.
        for b in yeni.iter_mut().take(4200).skip(4100) {
            *b = 0xAB;
        }
        let yama = uret(&eski, &yeni);
        let sonuc = uygula(&eski, &yama).unwrap();
        assert_eq!(sonuc, yeni);
        // Büyük ortak gövde → taşınan bayt yeni paketten **çok** küçük (delta kazancı).
        assert!(yama.tasinan_bayt() < yeni.len() / 2);
    }

    #[test]
    fn ekleme_ve_silme_round_trip() {
        let eski: Vec<u8> = (0..6000u32).map(|i| (i % 256) as u8).collect();
        // Başa ekle, ortadan çıkar, sona ekle.
        let mut yeni = vec![9u8; 300];
        yeni.extend_from_slice(&eski[0..2000]);
        yeni.extend_from_slice(&eski[3000..]);
        yeni.extend_from_slice(&[7u8; 200]);
        let yama = uret(&eski, &yeni);
        assert_eq!(uygula(&eski, &yama).unwrap(), yeni);
    }

    #[test]
    fn yanlis_tabana_uygulama_reddedilir() {
        let eski = vec![1u8; 2048];
        let yeni = vec![2u8; 2048];
        let yama = uret(&eski, &yeni);
        let baska = vec![3u8; 2048];
        let hata = uygula(&baska, &yama).unwrap_err();
        assert!(hata.ne_oldu.contains("taban"));
    }

    #[test]
    fn kurcalanmis_sonuc_ozeti_reddedilir() {
        let eski = vec![1u8; 2048];
        let yeni = vec![2u8; 2048];
        let mut yama = uret(&eski, &yeni);
        // Saldırgan beklenen sonuç özetini değiştirdi → sonuç doğrulaması tutmaz.
        yama.yeni_blake3 = hex(b"baska");
        assert!(uygula(&eski, &yama).is_err());
    }

    #[test]
    fn sinir_disi_kopya_reddedilir() {
        let eski = vec![1u8; 1024];
        let yama = DeltaYama {
            eski_blake3: hex(&eski),
            yeni_blake3: hex(b"x"),
            yeni_boyut: 1,
            ops: vec![DeltaOp::Kopya {
                ofset: 1000,
                uzunluk: 9999,
            }],
        };
        let hata = uygula(&eski, &yama).unwrap_err();
        assert!(hata.ne_oldu.contains("bozuk"));
    }

    #[test]
    fn rastgele_round_trip_property() {
        // Basit deterministik PRNG (xorshift) — harici bağımlılık yok.
        let mut s: u64 = 0x9E3779B97F4A7C15;
        let mut rnd = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        };
        for _ in 0..20 {
            let n = (rnd() % 5000) as usize;
            let eski: Vec<u8> = (0..n).map(|_| (rnd() & 0xFF) as u8).collect();
            // Yeni = eski'nin mutasyonu.
            let mut yeni = eski.clone();
            if !yeni.is_empty() {
                for _ in 0..(rnd() % 50) {
                    let idx = (rnd() as usize) % yeni.len();
                    yeni[idx] = (rnd() & 0xFF) as u8;
                }
            }
            yeni.extend_from_slice(&[(rnd() & 0xFF) as u8; 13]);
            let yama = uret(&eski, &yeni);
            assert_eq!(uygula(&eski, &yama).unwrap(), yeni, "round-trip n={n}");
        }
    }
}
