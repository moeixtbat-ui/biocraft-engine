//! Tuş seti **profilleri** (İP-13) — modern varsayılan + Vim/Emacs için kanca.
//!
//! Bir profil, "varsayılan kısayol seti"ni tanımlar.  MVP'de yalnızca **Modern** vardır ve
//! varsayılanlar **tek kaynaktan** gelir: her [`KabukAksiyon`]'un kendi `kisayol()` ipucu.  Böylece
//! menüde gösterilen ipucu, paletteki ipucu ve gerçek bağlama **aynı** tablodan türer (MK-51).
//!
//! Vim/Emacs emülasyonu **v1.x**'tir (`MVP-sonrasi.md` §8.2).  `TusSetiProfili` ileri-uyumlu bir
//! `enum`'dur; ayar sistemindeki `kisayol.tus_seti` değeri [`TusSetiProfili::ayardan`] ile çözülür —
//! tanınmayan/gelecek profiller güvenle **Modern**'e düşer (sahte "çalışıyor" görüntüsü yok, MK-48).

use std::collections::BTreeMap;

use crate::i18n::Dil;
use crate::shell::menu_bar::KabukAksiyon;

use super::shortcuts::Kisayol;
use super::KomutKaynak;

/// Klavye tuş seti profili.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum TusSetiProfili {
    /// Modern editör varsayılanları (VS Code benzeri) — MVP'de tek etkin profil.
    #[default]
    Modern,
    // Vim / Emacs → v1.x (MVP-sonrasi.md §8.2).  Kanca: yeni varyant + `modern`'in yanına bir
    // `vim()`/`emacs()` tablosu eklemek yeterli; gerisi (UI/çözümleme) değişmeden çalışır.
}

impl TusSetiProfili {
    /// Seçicide gösterilen tüm **etkin** profiller.
    pub const TUMU: &'static [TusSetiProfili] = &[TusSetiProfili::Modern];

    /// Profilin yerelleştirilmiş adı.
    pub fn ad(self, dil: Dil) -> &'static str {
        match (self, dil) {
            (TusSetiProfili::Modern, Dil::Tr) => "Modern",
            (TusSetiProfili::Modern, Dil::En) => "Modern",
        }
    }

    /// Ayar değeri ("modern", "vim"…) → profil.  Tanınmayan/gelecek değerler **Modern**'e düşer.
    pub fn ayardan(deger: &str) -> TusSetiProfili {
        match deger.trim().to_lowercase().as_str() {
            "modern" => TusSetiProfili::Modern,
            _ => TusSetiProfili::Modern, // vim/emacs → v1.x; şimdilik Modern
        }
    }

    /// Profilin ayar değeri karşılığı.
    pub fn ayar_degeri(self) -> &'static str {
        match self {
            TusSetiProfili::Modern => "modern",
        }
    }
}

/// Bir profilin varsayılan bağlamaları (aksiyon → kısayol).
pub fn varsayilan_harita(profil: TusSetiProfili) -> BTreeMap<KomutKaynak, Kisayol> {
    match profil {
        TusSetiProfili::Modern => modern_harita(),
    }
}

/// Modern varsayılan set — her `KabukAksiyon`'un kendi kısayol ipucundan türetilir (tek kaynak).
fn modern_harita() -> BTreeMap<KomutKaynak, Kisayol> {
    KabukAksiyon::tumu()
        .iter()
        .filter_map(|&a| {
            let metin = a.kisayol()?;
            let ks = Kisayol::ayristir(metin)?;
            Some((KomutKaynak::Kabuk(a), ks))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_set_kabuk_aksiyon_ipuclariyla_tutarli() {
        // Her varsayılan bağlama, ilgili KabukAksiyon::kisayol() ipucuyla AYNI olmalı (tek kaynak).
        let h = varsayilan_harita(TusSetiProfili::Modern);
        for &a in KabukAksiyon::tumu() {
            if let Some(metin) = a.kisayol() {
                let beklenen = Kisayol::ayristir(metin).unwrap();
                assert_eq!(
                    h.get(&KomutKaynak::Kabuk(a)),
                    Some(&beklenen),
                    "ipucu ile harita farklı: {a:?}"
                );
            }
        }
    }

    #[test]
    fn ayardan_bilinmeyen_moderne_duser() {
        assert_eq!(TusSetiProfili::ayardan("modern"), TusSetiProfili::Modern);
        assert_eq!(TusSetiProfili::ayardan("vim"), TusSetiProfili::Modern);
        assert_eq!(TusSetiProfili::ayardan("zzz"), TusSetiProfili::Modern);
    }
}
