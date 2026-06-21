//! Tipli & renkli portlar — node bağlantı uyumluluğunun çekirdeği (İP-05).
//!
//! Bir port bir **veri türü** ([`VeriTuru`]) taşır.  İki port yalnızca **uyumlu tür** taşıyorsa
//! birbirine bağlanabilir; uyumsuzsa bağlantı *reddedilir* (TDA madde 8 / foolproof).  Tür
//! doğrudan uymuyor ama bir **dönüştürücü** node varsa, kullanıcıya o öneri sunulur
//! ([`DonusturucuKayit::oner`]).
//!
//! **Renk:** Port rengi türden türetilir ama **token'dan** gelir (MK-52): bu modül sabit RGB
//! üretmez; yalnızca türe göre kararlı bir token anahtarı seçer, somut renk tema token'ından okunur.
// MK-52: renkler token'dan; burada yalnızca türe göre token anahtarı seçilir.

use crate::i18n::Dil;

/// Port yönü — SDK kontratındaki tip yeniden kullanılır (eklenti tarafıyla aynı kavram).
pub use biocraft_sdk::node::PortYonu;

/// "Her tür" (joker) kimliği — bu türdeki bir port her türle uyumludur (genel/geçiş node'ları).
pub const HER: &str = "her";

/// Bir portun taşıdığı **veri türü** (tipli & renkli).
///
/// Tür, kanonik bir kimlik dizgesidir (örn. `"dizi"`, `"hizalama"`, `"varyant"`, `"tablo"`).
/// Eklentiler kendi türlerini ekleyebilir; çekirdek yalnızca *kimlik eşitliği* + joker kuralını
/// bilir (anlamı eklenti/akış belirler).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VeriTuru {
    /// Kanonik tür kimliği (küçük harf, boşluksuz).
    pub kimlik: String,
}

impl VeriTuru {
    /// Verilen kimlikten bir veri türü kurar.
    pub fn yeni(kimlik: impl Into<String>) -> Self {
        Self {
            kimlik: kimlik.into(),
        }
    }

    /// "Her tür" (joker) — genel/geçiş node'larının portu için.
    pub fn her() -> Self {
        Self::yeni(HER)
    }

    /// Bu tür joker mi ("her")?
    pub fn joker_mi(&self) -> bool {
        self.kimlik == HER
    }

    /// Bu tür `diger` ile **doğrudan** uyumlu mu? (Aynı kimlik **veya** biri joker.)
    pub fn uyumlu_mu(&self, diger: &VeriTuru) -> bool {
        self.kimlik == diger.kimlik || self.joker_mi() || diger.joker_mi()
    }

    /// Türün kullanıcıya görünen adı (bilinen türler yerelleştirilir; bilinmeyen kimlik aynen).
    pub fn ad(&self, dil: Dil) -> String {
        let tr = matches!(dil, Dil::Tr);
        let s = match (self.kimlik.as_str(), tr) {
            ("dizi", true) => "Dizi",
            ("dizi", false) => "Sequence",
            ("hizalama", true) => "Hizalama",
            ("hizalama", false) => "Alignment",
            ("varyant", true) => "Varyant",
            ("varyant", false) => "Variant",
            ("tablo", true) => "Tablo",
            ("tablo", false) => "Table",
            ("3b_yapi", true) => "3B Yapı",
            ("3b_yapi", false) => "3D Structure",
            ("metin", true) => "Metin",
            ("metin", false) => "Text",
            (HER, true) => "Her tür",
            (HER, false) => "Any type",
            _ => return self.kimlik.clone(),
        };
        s.to_string()
    }
}

/// Bir node portunun (giriş veya çıkış ucu) çalışma-zamanı tanımı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    /// Port adı (node içinde benzersiz).
    pub ad: String,
    /// Taşıdığı veri türü (tipli & renkli).
    pub veri_turu: VeriTuru,
}

impl Port {
    /// Ad + tür kimliğinden bir port kurar.
    pub fn yeni(ad: impl Into<String>, tur_kimligi: impl Into<String>) -> Self {
        Self {
            ad: ad.into(),
            veri_turu: VeriTuru::yeni(tur_kimligi),
        }
    }
}

/// Veri türünün **token renk anahtarını** (kararlı) döndürür — somut renk token'dan okunur (MK-52).
///
/// Türler anlamsal palet renklerine kararlı biçimde eşlenir (FNV-1a hash → palet indeksi).  Aynı
/// tür her zaman aynı rengi alır; farklı türler büyük olasılıkla farklı renk alır → kullanıcı portu
/// renginden tanır.  Joker tür nötr/soluk renk alır.
pub fn tur_renk_anahtari(tur: &VeriTuru) -> &'static str {
    if tur.joker_mi() {
        return "text.muted";
    }
    // Anlamsal token paleti — hepsi tema token'ından gelir; burada yalnız seçim yapılır.
    const PALET: &[&str] = &[
        "accent.primary",
        "success",
        "warning",
        "info",
        "error",
        "text.muted",
    ];
    let h = fnv1a(tur.kimlik.as_bytes());
    PALET[(h as usize) % PALET.len()]
}

/// Bağımlılıksız, kararlı 32-bit FNV-1a hash (port renk seçimi için; kriptografik değildir).
fn fnv1a(baytlar: &[u8]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for &b in baytlar {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

// ─── Dönüştürücü öneri kaydı ──────────────────────────────────────────────────

/// Bir veri türünü başka bir türe çeviren node'un kaydı (otomatik dönüştürücü önerisi için).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Donusturucu {
    /// Girdi türü.
    pub kaynak: VeriTuru,
    /// Çıktı türü.
    pub hedef: VeriTuru,
    /// Bu dönüşümü yapan node türünün kataloğdaki kimliği.
    pub node_tur_kimligi: String,
    /// Kullanıcıya görünen başlık ("Varyant → Tablo").
    pub baslik: String,
}

/// Bilinen dönüştürücü node'ların kaydı.
///
/// İki port doğrudan uyumlu değilse ([`VeriTuru::uyumlu_mu`] false), tuval bu kayıttan bir köprü
/// node önerebilir ("şu dönüştürücüyü araya ekle").  Önerinin *uygulanması* Gün 21 (çalıştırma/SDK).
#[derive(Debug, Clone, Default)]
pub struct DonusturucuKayit {
    kayitlar: Vec<Donusturucu>,
}

impl DonusturucuKayit {
    /// Boş kayıt.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir dönüştürücü ekler.
    pub fn ekle(&mut self, d: Donusturucu) {
        self.kayitlar.push(d);
    }

    /// `kaynak` türünden `hedef` türüne doğrudan bir dönüştürücü varsa onu döndürür.
    pub fn oner(&self, kaynak: &VeriTuru, hedef: &VeriTuru) -> Option<&Donusturucu> {
        self.kayitlar
            .iter()
            .find(|d| d.kaynak.uyumlu_mu(kaynak) && d.hedef.uyumlu_mu(hedef))
    }

    /// Kayıtlı tüm dönüştürücüler (salt-okunur).
    pub fn tumu(&self) -> &[Donusturucu] {
        &self.kayitlar
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ayni_tur_uyumlu_farkli_tur_degil() {
        let dizi = VeriTuru::yeni("dizi");
        let dizi2 = VeriTuru::yeni("dizi");
        let tablo = VeriTuru::yeni("tablo");
        assert!(dizi.uyumlu_mu(&dizi2));
        assert!(!dizi.uyumlu_mu(&tablo));
    }

    #[test]
    fn joker_her_turle_uyumlu() {
        let her = VeriTuru::her();
        assert!(her.uyumlu_mu(&VeriTuru::yeni("dizi")));
        assert!(VeriTuru::yeni("tablo").uyumlu_mu(&her));
        assert!(her.joker_mi());
    }

    #[test]
    fn renk_anahtari_kararli_ve_joker_notr() {
        let t = VeriTuru::yeni("hizalama");
        // Aynı tür her çağrıda aynı anahtarı vermeli (kararlı).
        assert_eq!(tur_renk_anahtari(&t), tur_renk_anahtari(&t));
        // Joker nötr/soluk.
        assert_eq!(tur_renk_anahtari(&VeriTuru::her()), "text.muted");
    }

    #[test]
    fn donusturucu_oneri_calisir() {
        let mut kayit = DonusturucuKayit::yeni();
        kayit.ekle(Donusturucu {
            kaynak: VeriTuru::yeni("varyant"),
            hedef: VeriTuru::yeni("tablo"),
            node_tur_kimligi: "donustur.varyant_tablo".into(),
            baslik: "Varyant → Tablo".into(),
        });
        let oneri = kayit.oner(&VeriTuru::yeni("varyant"), &VeriTuru::yeni("tablo"));
        assert!(oneri.is_some());
        assert_eq!(oneri.unwrap().node_tur_kimligi, "donustur.varyant_tablo");
        // Uygun dönüştürücü yoksa None.
        assert!(kayit
            .oner(&VeriTuru::yeni("dizi"), &VeriTuru::yeni("3b_yapi"))
            .is_none());
    }

    #[test]
    fn tur_adi_yerellesir_bilinmeyen_aynen() {
        assert_eq!(VeriTuru::yeni("dizi").ad(Dil::Tr), "Dizi");
        assert_eq!(VeriTuru::yeni("dizi").ad(Dil::En), "Sequence");
        // Bilinmeyen kimlik aynen döner.
        assert_eq!(VeriTuru::yeni("ozeltür").ad(Dil::Tr), "ozeltür");
    }
}
