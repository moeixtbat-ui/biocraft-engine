//! ÇE-11 — **Dışa Aktarma ve Oturum**: çalışmayı dışarı çıkarma + paylaşma yetenekleri (MK-34).
//!
//! Bu paket, BioCraft Studio'daki bir çalışmayı yayın/paylaşım için dışarı taşır ve görünümü kalıcı
//! kılar.  Tümü **render-bağımsız ve birim-testlenebilir** (MK-17; gerçek dosya yazımı `fs`-kapılı
//! çağıran tarafta — MK-13) ve **gizlilik sınırına** uyar (Gün 18 / MK-42/43: hassas alan onaysız sızmaz).
//!
//! ## Alt-modüller
//! * [`figure`] — **görsel dışa aktarma**: genom tarayıcı / grafik görünümleri **yüksek-DPI PNG** +
//!   **vektör SVG** + **vektör PDF**; boyut/DPI/arka plan seçimi (yayın kalitesi).  3B yapı görünümü
//!   ([`crate::structure3d`]) kendi PNG/SVG'sini kullanır; [`figure::GorselAyari`] boyut/DPI hesabı
//!   oraya da hizmet eder.
//! * [`data`] — **veri/tablo dışa aktarma**: tablolar/seçimler **CSV/TSV**, dizi **FASTA**, varyant
//!   alt-kümesi **VCF** (ÇE-04 köprüsü); seçim korunur; [`data::GizlilikSuzgeci`] PHI/hassas sütunu düşürür.
//! * [`session`] — **görünüm oturumu**: açık dosyalar/izler/bölge/ayar/3B kamera **kaydet-yükle**
//!   (sürümlü şema + eksik alana güvenli varsayılan; İP-11 "kaldığın görünüme dön").
//! * [`history`] — **eklenti içi geçmiş**: son dosya/bölge/işlem → hızlı tekrar erişim (boş rehberi).
//! * [`report`] — **temel rapor**: görseller + parametreler + **veri kaynakları (provenance/atıf)** →
//!   Markdown/HTML/PDF + **"Yöntem ve Materyaller"** taslağı (Gün 18 köken + Gün 41 atıf).

use biocraft_sdk::biocraft_types::Capability;
use biocraft_sdk::{Aktivasyon, YetkiKapisi};

pub mod data;
pub mod figure;
pub mod history;
pub mod report;
pub mod session;

pub use data::{
    fasta_olustur, tablo_disa_aktar, varyant_vcf, Ayrac, FastaKaydi, GizlilikSuzgeci, Tablo,
    FASTA_SATIR,
};
pub use figure::{
    cizimi_disa_aktar, metin_pdf, pdf_olustur, ArkaPlan, Boyut, GorselAyari, GorselCikti,
    GorselFormat, TEMEL_DPI, YAYIN_DPI,
};
pub use history::{Gecmis, GecmisGirdi, GecmisTuru, VARSAYILAN_AZAMI};
pub use report::{GorselReferans, Parametre, Rapor, RaporBolumu};
pub use session::{IzDurumu, KameraDurumu, OturumDurumu, SURUM_GUNCEL};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-11";

/// Alt-modülün UI/komut kayıtları.  Dışa aktarma + oturum dosya yazımı gerektirir → komutlar yalnız
/// **`fs`** yetkisi verildiyse sunulur (en az yetki + dürüstlük; `data_io`/`variant` deseni).
pub fn kayitlar(yetkiler: &YetkiKapisi) -> Aktivasyon {
    let mut akt = Aktivasyon::yeni();
    if yetkiler.var_mi(Capability::Fs) {
        akt.komut(
            "biocraft.core.studio.export.gorsel",
            "BioCraft Studio: Görünümü Dışa Aktar (PNG/SVG/PDF)",
        )
        .komut(
            "biocraft.core.studio.export.veri",
            "BioCraft Studio: Veriyi Dışa Aktar (CSV/TSV/FASTA/VCF)",
        )
        .komut(
            "biocraft.core.studio.export.oturum_kaydet",
            "BioCraft Studio: Görünüm Oturumunu Kaydet",
        )
        .komut(
            "biocraft.core.studio.export.oturum_yukle",
            "BioCraft Studio: Görünüm Oturumunu Yükle",
        )
        .komut(
            "biocraft.core.studio.export.rapor",
            "BioCraft Studio: Rapor Oluştur (Köken + Atıf)",
        );
    }
    akt
}

#[cfg(test)]
mod tests {
    use super::*;
    use biocraft_sdk::ui::UiUzantiTuru;

    #[test]
    fn fs_yoksa_komut_yok() {
        assert_eq!(kayitlar(&YetkiKapisi::bos()).ui_say(UiUzantiTuru::Komut), 0);
    }

    #[test]
    fn fs_varsa_bes_komut() {
        let akt = kayitlar(&YetkiKapisi::yeni([Capability::Fs]));
        assert_eq!(akt.ui_say(UiUzantiTuru::Komut), 5);
    }

    #[test]
    fn ce_etiketi() {
        assert_eq!(CE, "ÇE-11");
    }
}
