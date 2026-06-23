//! ÇE-09 — Birleşik veritabanı arama + konektörler **[iskelet]**.
//!
//! Yerel SQLite/DuckDB sorgu + (ileride) BLAST/PDB/NCBI konektörleri.  `db` yeteneği gerektirir
//! (uzak konektörler ileride `net` ile).  Bu modül **capability-kapılı kayıt** desenini gösterir:
//! `db` verilmemişse veritabanı arama komutu **kaydedilmez** (sahte/erişilemez özellik ifşa edilmez).

use biocraft_sdk::biocraft_types::Capability;
use biocraft_sdk::{Aktivasyon, YetkiKapisi};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-09";

/// Alt-modülün UI/komut kayıtları — yalnızca `db` yetkisi verildiyse arama komutu açılır.
pub fn kayitlar(yetkiler: &YetkiKapisi) -> Aktivasyon {
    let mut akt = Aktivasyon::yeni();
    // En az yetki + dürüstlük: arka planı `db` olmadan çalışmayan komutu hiç sunmayız.
    if yetkiler.var_mi(Capability::Db) {
        akt.komut(
            "biocraft.core.studio.db.ara",
            "BioCraft Studio: Veritabanı Ara",
        );
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

        let dbli = kayitlar(&YetkiKapisi::yeni([Capability::Db]));
        assert_eq!(dbli.ui_say(UiUzantiTuru::Komut), 1);
    }
}
