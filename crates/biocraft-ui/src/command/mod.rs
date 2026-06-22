//! İP-13 — **Komut paleti + klavye kısayolları + tuş seti profili**.
//!
//! MK-51: klasik menü ile komut paleti **tek komut tanımına** ([`KabukAksiyon`]) bağlanır → "menü ile
//! palet çakışır" sorunu yok; davranış tek yerde.  Eklenti komutları da aynı listeye güvenle katılır
//! (İP-07 uzantı noktası).  MK-52: renkler token'dan, tüm metin i18n, **tüm aksiyonlar klavyeyle**.
//!
//! Modüller: [`palette`] (Ctrl+Shift+P, bulanık arama <50 ms), [`shortcuts`] (özelleştirme + çakışma),
//! [`keymap_profile`] (modern varsayılan + Vim/Emacs kancası), [`fuzzy`] (saf-Rust eşleştirici).

pub mod fuzzy;
pub mod keymap_profile;
pub mod palette;
pub mod shortcuts;

#[cfg(test)]
mod tests;

use crate::i18n::Dil;
use crate::shell::menu_bar::KabukAksiyon;

pub use fuzzy::{bulanik_skor, gevsek_benzerlik, BulanikSonuc};
pub use keymap_profile::{varsayilan_harita, TusSetiProfili};
pub use palette::{KomutPaleti, PaletEylem, PaletModu};
pub use shortcuts::{
    kisayol_penceresi, Cakisma, Degistiriciler, Kisayol, KisayolDuzenleyici, KisayolHaritasi,
};

/// Bir komutun KAYNAĞI — menü ile palet AYNI tanıma bağlanır (MK-51).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KomutKaynak {
    /// Çekirdek kabuk aksiyonu (menü öğesiyle aynı tanım).
    Kabuk(KabukAksiyon),
    /// Eklenti komutu (SDK `UiUzantiTuru::Komut` kaydı; kimlik = eklenti komut kimliği).
    Eklenti(String),
}

impl KomutKaynak {
    /// Kalıcılık için kararlı metin anahtarı ("kabuk:KomutPaleti", "eklenti:<kimlik>").
    pub fn anahtar(&self) -> String {
        match self {
            KomutKaynak::Kabuk(a) => format!("kabuk:{a:?}"),
            KomutKaynak::Eklenti(k) => format!("eklenti:{k}"),
        }
    }

    /// Metin anahtarından geri kurar.  Tanınmayan kabuk aksiyonu → `None` (ileri/geri uyumlu).
    pub fn anahtardan(s: &str) -> Option<KomutKaynak> {
        if let Some(rest) = s.strip_prefix("kabuk:") {
            KabukAksiyon::tumu()
                .iter()
                .find(|a| format!("{a:?}") == rest)
                .map(|&a| KomutKaynak::Kabuk(a))
        } else {
            s.strip_prefix("eklenti:")
                .map(|k| KomutKaynak::Eklenti(k.to_string()))
        }
    }
}

/// Komut kategorisi (palet sağında etiket; gruplama/filtre).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KomutKategori {
    Dosya,
    Duzen,
    Gorunum,
    Calistir,
    Eklenti,
    Yardim,
    Genel,
}

impl KomutKategori {
    /// Yerelleştirilmiş kısa etiket.
    pub fn etiket(self, dil: Dil) -> &'static str {
        use Dil::{En, Tr};
        use KomutKategori::*;
        match (self, dil) {
            (Dosya, Tr) => "Dosya",
            (Dosya, En) => "File",
            (Duzen, Tr) => "Düzen",
            (Duzen, En) => "Edit",
            (Gorunum, Tr) => "Görünüm",
            (Gorunum, En) => "View",
            (Calistir, Tr) => "Çalıştır",
            (Calistir, En) => "Run",
            (Eklenti, Tr) => "Eklenti",
            (Eklenti, En) => "Plugin",
            (Yardim, Tr) => "Yardım",
            (Yardim, En) => "Help",
            (Genel, Tr) => "Genel",
            (Genel, En) => "General",
        }
    }
}

/// Bir `KabukAksiyon`'un komut kategorisi (palet sunumu — tek kaynak menü tanımıyla tutarlı).
fn kabuk_kategori(a: KabukAksiyon) -> KomutKategori {
    use KabukAksiyon::*;
    match a {
        YeniProje | ProjeAc | YeniSekme | Kaydet | Cikis => KomutKategori::Dosya,
        GeriAl | Yinele => KomutKategori::Duzen,
        TemaDegistir | DilDegistir | YanPanelAcKapa | AltPanelAcKapa | InspectorAcKapa
        | EditoruBol | YogunMod | DuzenYonetici | KomutPaleti | Ayarlar | KisayolAyarlari => {
            KomutKategori::Gorunum
        }
        NodeEditoru | KodEditoru | AkisiKodAc => KomutKategori::Calistir,
        EklentileriYonet => KomutKategori::Eklenti,
        DemoGalerisi | Belgeler | Hakkinda => KomutKategori::Yardim,
    }
}

/// Bir `KabukAksiyon`'un palet ikonu (yoksa kategori ikonu kullanılmaz; sade kalır).
fn kabuk_ikon(a: KabukAksiyon) -> Option<&'static str> {
    use KabukAksiyon::*;
    Some(match a {
        YeniProje | YeniSekme => "＋",
        ProjeAc => "📂",
        Kaydet => "💾",
        Cikis => "⎋",
        GeriAl => "↶",
        Yinele => "↷",
        TemaDegistir => "🎨",
        DilDegistir => "🌐",
        Ayarlar => "⚙",
        KisayolAyarlari => "⌨",
        KomutPaleti => "⌘",
        NodeEditoru => "🔗",
        KodEditoru => "📝",
        AkisiKodAc => "🐍",
        EklentileriYonet => "🧩",
        DemoGalerisi => "🖼",
        Belgeler => "📖",
        Hakkinda => "ℹ",
        _ => return None,
    })
}

/// Palet/kısayol için **çözülmüş** tek komut (görünür ad + kategori + kısayol ipucu + etkinlik).
#[derive(Debug, Clone)]
pub struct Komut {
    pub kaynak: KomutKaynak,
    pub ad: String,
    pub kategori: KomutKategori,
    /// Görsel kısayol ipucu (atanmış kısayolun gösterimi).
    pub kisayol: Option<String>,
    /// Palet ikonu.
    pub ikon: Option<&'static str>,
    /// Bu bağlamda yapılabilir mi (koşul) — `false` ise palette listelenmez.
    pub etkin: bool,
    /// Önceden hesaplanmış küçük-harf arama samanı (iki dilde ad + kategori).
    pub(crate) saman: String,
}

impl Komut {
    /// Bir kabuk aksiyonundan komut kurar (ad + kategori + ikon menü tanımıyla aynı kaynaktan).
    pub fn kabuktan(a: KabukAksiyon, dil: Dil, kisayol: Option<String>, etkin: bool) -> Komut {
        let kategori = kabuk_kategori(a);
        let saman = format!(
            "{} {} {} {}",
            a.etiket(Dil::Tr),
            a.etiket(Dil::En),
            kategori.etiket(Dil::Tr),
            kategori.etiket(Dil::En),
        )
        .to_lowercase();
        Komut {
            kaynak: KomutKaynak::Kabuk(a),
            ad: a.etiket(dil).to_string(),
            kategori,
            kisayol,
            ikon: kabuk_ikon(a),
            etkin,
            saman,
        }
    }

    /// Bir eklenti komutundan kurar (İP-07 `UiUzantiTuru::Komut`).
    pub fn eklentiden(ek: &EklentiKomut, kisayol: Option<String>) -> Komut {
        let saman = format!("{} {}", ek.baslik, ek.kimlik).to_lowercase();
        Komut {
            kaynak: KomutKaynak::Eklenti(ek.kimlik.clone()),
            ad: ek.baslik.clone(),
            kategori: KomutKategori::Eklenti,
            kisayol,
            ikon: Some("🧩"),
            etkin: true,
            saman,
        }
    }
}

/// Bir eklentinin ilan ettiği komut (SDK `UiKayit { tur: Komut }` karşılığı; çekirdek tarafı).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EklentiKomut {
    pub kimlik: String,
    pub baslik: String,
}

impl EklentiKomut {
    pub fn yeni(kimlik: impl Into<String>, baslik: impl Into<String>) -> Self {
        Self {
            kimlik: kimlik.into(),
            baslik: baslik.into(),
        }
    }
}
