//! ÇE-09 — **Veritabanı Erişimi: Birleşik Arama + Konektörler** (1. kısım — Gün 40).
//!
//! Kullanıcının yerel indirme yerine, arayüzden bilimsel veritabanlarını (NCBI/BLAST…) arayıp
//! sonucu **tek tıkla** görselleştirmeye/projeye getirmesini sağlayan çerçeve.  MK-41 (dış iletişim
//! onayı) + MK-42/43 (PHI sınırı) çekirdek kuraldır.
//!
//! ## Alt-modüller
//! * [`framework`] — soyut [`VeritabaniKonektoru`] arayüzü + birleşik sorgu/sonuç/sayfalama şeması +
//!   [`AramaBaglami`] (taşıma/gizlilik/hız) + [`HizSinirlayici`].  Yeni veritabanı = yeni konektör.
//! * [`transport`] — HTTP taşıma soyutlaması (dürüst yer-tutucu + test ikizi; gerçek `net` adaptörü
//!   ileride aynı trait'i uygular).
//! * [`privacy`] — dış sorgu onayı (şeffaflık) + **PHI/hassas engeli** (İP-10 savunma derinliği).
//! * [`provenance`] — indirilen kayıt için kaynak/erişim-tarihi/BLAKE3 + lisans/atıf (İP-10/ÇE-09).
//! * [`connectors`] — [`connectors::ncbi`] (E-utilities) + [`connectors::blast`] (BLAST URL API).
//! * [`panel`] — [`BirlesikPanel`]: tek arama kutusu → kaynak rozetli birleşik sonuç + eylemler.
//!
//! ## Mimari kararlar (Gün 40)
//! * **MK-17:** Eklenti yalnızca `biocraft-sdk`'ya bağlıdır → görevde geçen `biocraft-net`'e
//!   **doğrudan bağlanılmaz** (motor crate'i); ağ soyutlaması eklenti-yereldir ([`transport`]).
//! * **Tokio yok:** Uzun iş (BLAST) senkron-pull [`IsKulpu`](biocraft_sdk::biocraft_types::IsKulpu)
//!   ile modellenir (İP-21); "yeni ağır bağımlılık ekleme" disiplini korunur.
//! * **Dürüst sınır:** Gerçek HTTP istemcisi bu sürümde bağlı değildir; tüm mantık çevrimdışı
//!   ([`transport::SahteUlastirici`]) birim-testlenir (`data_io::remote` deseni).
//!
//! Gün 41: PDB/UniProt/Ensembl/UCSC konektörleri + önbellek/geçmiş/rate-limit derinleştirme.

use biocraft_sdk::biocraft_types::Capability;
use biocraft_sdk::{Aktivasyon, YetkiKapisi};

pub mod connectors;
pub mod framework;
pub mod panel;
pub mod privacy;
pub mod provenance;
pub mod transport;

pub use connectors::{
    BlastDurum, BlastIsi, BlastKonektor, BlastProgram, Hizalama, NcbiKonektor, NcbiVeritabani,
    YoklamaAyari,
};
pub use framework::{
    AramaBaglami, AramaSonucu, GetirilenKayit, HizSinirlayici, KayitTuru, SayfaBilgisi, Sayfalama,
    SonucListesi, SonucSkoru, Sorgu, VeritabaniKonektoru,
};
pub use panel::{BirlesikPanel, Eylem};
pub use privacy::{DisGonderimOzeti, DisVeri, GizlilikKapisi, HassasiyetEtiketi};
pub use provenance::{db_provenansi, ncbi_lisans_atif};
pub use transport::{
    HttpIstek, HttpUlastirici, HttpYanit, SahteUlastirici, YapilandirilmamisUlastirici,
};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-09";

/// Alt-modülün UI/komut kayıtları (capability-kapılı; en az yetki + dürüstlük):
/// * `db` → birleşik **Veritabanı Ara** paneli (yerel + uzak kaynak çerçevesi).
/// * `db` **+** `net` → uzak konektör komutları: **NCBI Ara**, **BLAST Çalıştır** (dış erişim
///   gerektirir → `net` yoksa sahte/erişilemez özellik ifşa edilmez).
pub fn kayitlar(yetkiler: &YetkiKapisi) -> Aktivasyon {
    let mut akt = Aktivasyon::yeni();
    if yetkiler.var_mi(Capability::Db) {
        akt.komut(
            "biocraft.core.studio.db.ara",
            "BioCraft Studio: Veritabanı Ara (birleşik)",
        );
        // Uzak (dış ağ) konektörleri yalnızca `net` onaylıysa sunulur (MK-13/MK-41).
        if yetkiler.var_mi(Capability::Net) {
            akt.komut(
                "biocraft.core.studio.db.ncbi",
                "BioCraft Studio: NCBI'de Ara (nucleotide/protein/gene)",
            )
            .komut(
                "biocraft.core.studio.db.blast",
                "BioCraft Studio: BLAST (dizi benzerlik araması)",
            );
        }
    }
    akt
}

#[cfg(test)]
mod tests {
    use super::*;
    use biocraft_sdk::ui::UiUzantiTuru;

    #[test]
    fn db_yoksa_arama_komutu_kaydedilmez() {
        let bos = kayitlar(&YetkiKapisi::bos());
        assert_eq!(bos.ui_say(UiUzantiTuru::Komut), 0);
    }

    #[test]
    fn db_varsa_birlesik_panel_komutu_acilir() {
        // Yalnız `db` (net yok) → yalnız birleşik panel komutu (uzak konektörler gizli).
        let dbli = kayitlar(&YetkiKapisi::yeni([Capability::Db]));
        assert_eq!(dbli.ui_say(UiUzantiTuru::Komut), 1);
    }

    #[test]
    fn db_ve_net_varsa_uzak_konektorler_eklenir() {
        let tam = kayitlar(&YetkiKapisi::yeni([Capability::Db, Capability::Net]));
        // birleşik panel + NCBI + BLAST = 3 komut.
        assert_eq!(tam.ui_say(UiUzantiTuru::Komut), 3);
    }

    #[test]
    fn net_tek_basina_yetmez() {
        // db olmadan net tek başına veritabanı komutu açmaz.
        let yalniz_net = kayitlar(&YetkiKapisi::yeni([Capability::Net]));
        assert_eq!(yalniz_net.ui_say(UiUzantiTuru::Komut), 0);
    }
}
