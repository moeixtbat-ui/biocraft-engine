//! ÇE-09 — **Veritabanı Erişimi: Birleşik Arama + Konektörler + Önbellek/Geçmiş** (Gün 40-41).
//!
//! Kullanıcının yerel indirme yerine, arayüzden bilimsel veritabanlarını arayıp sonucu **tek tıkla**
//! görselleştirmeye/projeye getirmesini sağlayan çerçeve.  MK-41 (dış iletişim onayı) + MK-42/43
//! (PHI sınırı) + MK-34 (provenance/atıf) çekirdek kuraldır.
//!
//! ## Alt-modüller
//! * [`framework`] — soyut [`VeritabaniKonektoru`] arayüzü + birleşik sorgu/sonuç/sayfalama şeması +
//!   [`AramaBaglami`] (taşıma/gizlilik/hız) + [`HizSinirlayici`] + [`Konum`] (çapraz bağlantı).
//! * [`transport`] — HTTP taşıma soyutlaması (GET/POST/PUT; dürüst yer-tutucu + test ikizi).
//! * [`privacy`] — dış sorgu onayı (şeffaflık) + **PHI/hassas engeli** (İP-10 savunma derinliği).
//! * [`provenance`] — indirilen kayıt için kaynak/erişim-tarihi/BLAKE3 + lisans/atıf (her kaynak
//!   için: NCBI/PDB/UniProt/Ensembl/UCSC).
//! * [`ratelimit`] — **kaynak-başına** hız sınırlama (her DB kendi kovası; Gün 41).
//! * [`cache`] — **akıllı önbellek**: sonuç+veri, BLAKE3 bütünlük, TTL, boyut/temizleme (Gün 41).
//! * [`history`] — **arama geçmişi** + favori + tekrar çalıştır + JSON kalıcılık (Gün 41).
//! * [`connectors`] — NCBI/BLAST (Gün 40) + **PDB/UniProt/Ensembl/UCSC** (Gün 41).
//! * [`panel`] — [`BirlesikPanel`]: tek arama kutusu → kaynak rozetli birleşik sonuç + eylemler
//!   (tarayıcıda aç / yapıya bak / genom tarayıcıda göster / projeye ekle) + önbellek + geçmiş.
//!
//! ## Mimari kararlar
//! * **MK-17:** Eklenti yalnızca `biocraft-sdk`'ya bağlıdır → görevde geçen `biocraft-net`'e
//!   **doğrudan bağlanılmaz** (motor crate'i); ağ soyutlaması eklenti-yereldir ([`transport`]).
//! * **Tokio yok:** Uzun iş (BLAST) senkron-pull [`IsKulpu`](biocraft_sdk::biocraft_types::IsKulpu)
//!   ile modellenir (İP-21); "yeni ağır bağımlılık ekleme" disiplini korunur.
//! * **Dürüst sınır:** Gerçek HTTP istemcisi bu sürümde bağlı değildir; tüm mantık çevrimdışı
//!   ([`transport::SahteUlastirici`]) birim-testlenir (`data_io::remote` deseni).

use biocraft_sdk::biocraft_types::Capability;
use biocraft_sdk::{Aktivasyon, YetkiKapisi};

pub mod cache;
pub mod connectors;
pub mod framework;
pub mod history;
pub mod panel;
pub mod privacy;
pub mod provenance;
pub mod ratelimit;
pub mod transport;

pub use cache::{AramaOnbellegi, OnbellekAyari};
pub use connectors::{
    BlastDurum, BlastIsi, BlastKonektor, BlastProgram, EnsemblKonektor, Hizalama, NcbiKonektor,
    NcbiVeritabani, PdbKonektor, UcscKonektor, UniprotKonektor, YoklamaAyari,
};
pub use framework::{
    AramaBaglami, AramaSonucu, GetirilenKayit, HizSinirlayici, KayitTuru, Konum, SayfaBilgisi,
    Sayfalama, SonucListesi, SonucSkoru, Sorgu, VeritabaniKonektoru,
};
pub use history::{AramaGecmisi, GecmisGirdisi};
pub use panel::{BirlesikPanel, Eylem};
pub use privacy::{DisGonderimOzeti, DisVeri, GizlilikKapisi, HassasiyetEtiketi};
pub use provenance::{
    db_provenansi, ensembl_lisans_atif, ncbi_lisans_atif, pdb_lisans_atif, ucsc_lisans_atif,
    uniprot_lisans_atif,
};
pub use ratelimit::{varsayilan_aralik, KaynakHizYoneticisi};
pub use transport::{
    HttpIstek, HttpUlastirici, HttpYanit, SahteUlastirici, YapilandirilmamisUlastirici,
};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-09";

/// Alt-modülün UI/komut kayıtları (capability-kapılı; en az yetki + dürüstlük):
/// * `db` → birleşik **Veritabanı Ara** paneli (yerel + uzak kaynak çerçevesi).
/// * `db` **+** `net` → uzak konektör komutları (NCBI/BLAST/PDB/UniProt/Ensembl/UCSC); dış erişim
///   gerektirir → `net` yoksa sahte/erişilemez özellik ifşa edilmez (MK-13/MK-41).
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
            )
            .komut(
                "biocraft.core.studio.db.pdb",
                "BioCraft Studio: PDB'de Ara (3B yapı → 3B görüntüleyici)",
            )
            .komut(
                "biocraft.core.studio.db.uniprot",
                "BioCraft Studio: UniProt'ta Ara (protein bilgisi/dizi)",
            )
            .komut(
                "biocraft.core.studio.db.ensembl",
                "BioCraft Studio: Ensembl'de Ara (gen/transkript/anotasyon)",
            )
            .komut(
                "biocraft.core.studio.db.ucsc",
                "BioCraft Studio: UCSC'de Ara (genom derlemesi/iz)",
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
        // birleşik panel + NCBI + BLAST + PDB + UniProt + Ensembl + UCSC = 7 komut.
        assert_eq!(tam.ui_say(UiUzantiTuru::Komut), 7);
    }

    #[test]
    fn net_tek_basina_yetmez() {
        // db olmadan net tek başına veritabanı komutu açmaz.
        let yalniz_net = kayitlar(&YetkiKapisi::yeni([Capability::Net]));
        assert_eq!(yalniz_net.ui_say(UiUzantiTuru::Komut), 0);
    }
}
