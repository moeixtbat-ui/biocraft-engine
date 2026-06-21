//! **Anonimleştirme temeli** (MVP) — AI havuzuna/paylaşıma katkı için sınıf düşürme (İP-10).
//!
//! Bu, üretim seviyesi bir DP sistemi **değildir**; *temeldir* (basis): doğrudan tanımlayıcıların
//! silinmesi, yarı-tanımlayıcıların genelleştirilmesi, **k-anonimlik** ölçümü ve sayımlar için basit
//! **Laplace diferansiyel gizlilik** gürültüsü.  Geri-tanımlama testine açıktır; gerçek sertleştirme
//! v1.x (`MVP-sonrasi.md`).
//!
//! **Önemli (MK-42):** Anonimleştirme PHI verinin *kendisini* dışarı çıkarmaz; ondan **türetilmiş,
//! geri-tanımlanamaz** ayrı bir çıktı üretir.  Ham PHI yine [`super::classify`] tarafından bloklanır.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Bir alanın tanımlayıcılık rolü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlanRolu {
    /// Doğrudan tanımlayıcı (ad, TC kimlik, hasta no, e-posta) — **silinir**.
    Dogrudan,
    /// Yarı-tanımlayıcı (yaş, posta kodu, tarih) — **genelleştirilir**.
    Yari,
    /// Tanımlayıcı olmayan (analiz değeri) — korunur.
    Normal,
}

/// Anonimleştirme yapılandırması: hangi alan hangi rolde.
#[derive(Debug, Clone, Default)]
pub struct AnonimAyar {
    /// Alan adı → rol.
    pub roller: BTreeMap<String, AlanRolu>,
    /// Yarı-tanımlayıcıların genelleştirme "kovası" (örn. yaş için 10 → 0-9,10-19…).  0 = genelleştirme yok.
    pub genelleme_kovasi: i64,
}

impl AnonimAyar {
    /// Boş ayar (kova 10).
    pub fn yeni() -> Self {
        Self {
            roller: BTreeMap::new(),
            genelleme_kovasi: 10,
        }
    }

    /// Bir alana rol atar (akıcı).
    pub fn rol(mut self, alan: impl Into<String>, rol: AlanRolu) -> Self {
        self.roller.insert(alan.into(), rol);
        self
    }

    fn rol_of(&self, alan: &str) -> AlanRolu {
        self.roller.get(alan).copied().unwrap_or(AlanRolu::Normal)
    }
}

/// Tek bir kaydı (alan→değer) anonimleştirir: doğrudan tanımlayıcılar **çıkarılır**, yarı-tanımlayıcılar
/// **genelleştirilir**, normal alanlar korunur.
pub fn kayit_anonimlestir(
    kayit: &BTreeMap<String, String>,
    ayar: &AnonimAyar,
) -> BTreeMap<String, String> {
    let mut cikti = BTreeMap::new();
    for (alan, deger) in kayit {
        match ayar.rol_of(alan) {
            AlanRolu::Dogrudan => { /* sil: çıktıya hiç koyma */ }
            AlanRolu::Yari => {
                cikti.insert(alan.clone(), genellestir(deger, ayar.genelleme_kovasi));
            }
            AlanRolu::Normal => {
                cikti.insert(alan.clone(), deger.clone());
            }
        }
    }
    cikti
}

/// Bir yarı-tanımlayıcı değeri genelleştirir.  Sayıysa kovaya yuvarlar (örn. 37, kova 10 → "30-39");
/// sayı değilse ilk karakter dışında maskeler.
fn genellestir(deger: &str, kova: i64) -> String {
    if kova <= 0 {
        return deger.to_string();
    }
    if let Ok(n) = deger.trim().parse::<i64>() {
        let alt = (n / kova) * kova;
        let ust = alt + kova - 1;
        return format!("{alt}-{ust}");
    }
    // Metin yarı-tanımlayıcı: ilk karakteri bırak, kalanını maskele.
    match deger.chars().next() {
        Some(c) => format!("{c}***"),
        None => String::new(),
    }
}

/// Bir kayıt kümesinin **k-anonimlik** değeri: yarı-tanımlayıcı birleşimine göre en küçük grup boyutu.
///
/// `k = 1` → en az bir kayıt tekil (geri-tanımlanabilir).  Yüksek `k` daha güvenli.  Boş küme → 0.
pub fn k_anonimlik(kayitlar: &[BTreeMap<String, String>], yari_alanlar: &[&str]) -> usize {
    if kayitlar.is_empty() {
        return 0;
    }
    let mut gruplar: BTreeMap<Vec<String>, usize> = BTreeMap::new();
    for kayit in kayitlar {
        let anahtar: Vec<String> = yari_alanlar
            .iter()
            .map(|a| kayit.get(*a).cloned().unwrap_or_default())
            .collect();
        *gruplar.entry(anahtar).or_insert(0) += 1;
    }
    gruplar.values().copied().min().unwrap_or(0)
}

/// Bir sayıma **Laplace diferansiyel gizlilik** gürültüsü ekler (temel mekanizma).
///
/// `epsilon` küçükse daha çok gürültü (daha güçlü gizlilik).  Tohum verildiğinden **deterministiktir**
/// (test edilebilir); gerçek dağıtımda tohum kriptografik RNG'den gelmelidir (v1.x).  Duyarlılık = 1
/// (tek kayıt ekle/çıkar bir sayımı en çok 1 değiştirir) varsayılır.
pub fn diferansiyel_say(gercek: i64, epsilon: f64, tohum: u64) -> i64 {
    if epsilon <= 0.0 {
        return gercek; // gürültü yok (anlamsız epsilon) — çağıran düzeltmeli.
    }
    let b = 1.0 / epsilon; // ölçek = duyarlılık/epsilon.
    let u = lcg_uniform(tohum) - 0.5; // (-0.5, 0.5)
                                      // Laplace ters-CDF: -b * sign(u) * ln(1 - 2|u|).
    let gurultu = -b * u.signum() * (1.0 - 2.0 * u.abs()).max(1e-12).ln();
    (gercek as f64 + gurultu).round() as i64
}

/// Tohumdan [0,1) düzgün dağılımlı bir değer (basit LCG — yalnızca DP temeli için, kripto değil).
fn lcg_uniform(tohum: u64) -> f64 {
    // Numerical Recipes LCG sabitleri.
    let x = tohum
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    // Üst 53 bit → [0,1).
    ((x >> 11) as f64) / ((1u64 << 53) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kayit(ciftler: &[(&str, &str)]) -> BTreeMap<String, String> {
        ciftler
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn dogrudan_tanimlayici_silinir() {
        let ayar = AnonimAyar::yeni()
            .rol("ad", AlanRolu::Dogrudan)
            .rol("hasta_no", AlanRolu::Dogrudan)
            .rol("yas", AlanRolu::Yari)
            .rol("varyant", AlanRolu::Normal);
        let k = kayit(&[
            ("ad", "Ayşe Yılmaz"),
            ("hasta_no", "123456"),
            ("yas", "37"),
            ("varyant", "BRCA1"),
        ]);
        let a = kayit_anonimlestir(&k, &ayar);
        // Doğrudan tanımlayıcılar çıkmış.
        assert!(!a.contains_key("ad"));
        assert!(!a.contains_key("hasta_no"));
        // Yaş genelleştirilmiş, varyant korunmuş.
        assert_eq!(a.get("yas").unwrap(), "30-39");
        assert_eq!(a.get("varyant").unwrap(), "BRCA1");
    }

    #[test]
    fn metin_yari_tanimlayici_maskelenir() {
        assert_eq!(genellestir("34020", 10), "34020-34029");
        assert_eq!(genellestir("İstanbul", 10), "İ***");
    }

    #[test]
    fn k_anonimlik_tekil_kaydi_yakalar() {
        let kayitlar = vec![
            kayit(&[("yas", "30-39"), ("il", "06")]),
            kayit(&[("yas", "30-39"), ("il", "06")]),
            kayit(&[("yas", "40-49"), ("il", "34")]), // tekil grup → k=1
        ];
        assert_eq!(k_anonimlik(&kayitlar, &["yas", "il"]), 1);
    }

    #[test]
    fn k_anonimlik_homojen_kume() {
        let kayitlar = vec![
            kayit(&[("yas", "30-39")]),
            kayit(&[("yas", "30-39")]),
            kayit(&[("yas", "30-39")]),
        ];
        assert_eq!(k_anonimlik(&kayitlar, &["yas"]), 3);
    }

    #[test]
    fn diferansiyel_say_deterministik_ve_gercege_yakin() {
        // Aynı tohum → aynı sonuç (test edilebilir).
        let a = diferansiyel_say(100, 1.0, 42);
        let b = diferansiyel_say(100, 1.0, 42);
        assert_eq!(a, b);
        // Yüksek epsilon (1.0) → gürültü makul; gerçeğe yakın kalmalı.
        assert!((a - 100).abs() < 50, "gürültü aşırı: {a}");
    }

    #[test]
    fn diferansiyel_say_epsilon_sifir_degismez() {
        assert_eq!(diferansiyel_say(100, 0.0, 1), 100);
    }
}
