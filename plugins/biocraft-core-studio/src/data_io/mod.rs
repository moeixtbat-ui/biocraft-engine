//! ÇE-01 — **Veri G/Ç ve format ayrıştırıcıları**.
//!
//! Çekirdek eklentinin veri okuma temeli: dizilim (FASTA/FASTQ) ve hizalama (BAM/SAM/CRAM)
//! formatları; **BGZF blok-farkında** okuma (MK-32), **indeksli rastgele erişim** (.fai/.bai/
//! .csi/.crai), **out-of-core** bölge sorgusu (yalnız görünen pencere; MK-09), **BLAKE3 bütünlük**
//! + **provenance** kaydı (MK-34, İP-10) ve **bellek bütçesi** (İP-08).
//!
//! Tüm format ayrıştırma `noodles` (saf-Rust, MIT) ile yapılır — doğruluk-kritik BGZF/BAM/CRAM
//! elle yazılmaz.  `fs` yeteneği gerektirir (MK-13); uzak erişim ileride `net` ile gelir.
//!
//! ## Kapsam
//! * **Bugün (Gün 34):** FASTA/FASTQ + BAM/SAM/CRAM.
//! * **Yarın (Gün 35):** VCF/BCF + BED + GFF/GTF + BigWig/BigBed + 2bit + PDB/mmCIF + **uzak
//!   erişim** (HTTP/S3 bayt-aralığı → manifest'e `net` eklenir).

mod alignment;
mod bgzf;
mod budget;
mod detect;
mod fasta;
mod index;
mod integrity;
mod provenance;

pub use alignment::{HizalamaBasligi, HizalamaKaydi, HizalamaOkuyucu};
pub use bgzf::{coz_ve_olc as bgzf_coz_ve_olc, BgzfOzet};
pub use budget::BellekButcesi;
pub use detect::{formati_belirle, VeriFormati};
pub use fasta::{
    fai_olustur, fai_yolu, fasta_akis, fastq_akis, DiziParcasi, FastaOkuyucu, KaliteOzeti,
};
pub use index::{indeks_durumu, indeks_olustur, IndeksDurumu};
pub use integrity::{blake3_dosya, dogrula_blake3};
pub use provenance::{LisansAtif, Provenans};

use biocraft_sdk::biocraft_types::{Capability, ErrorReport};
use biocraft_sdk::{Aktivasyon, YetkiKapisi};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-01";

/// Bir veri dosyasını **yüklemeye hazırlar**: `fs` yetkisini doğrula (MK-13) → formatı otomatik
/// tanı → **BLAKE3 bütünlük** + **provenance** kaydı üret.  Asıl içerik okuma (bölge sorgusu vb.)
/// format-özel okuyucularla (out-of-core) yapılır; bu fonksiyon güvenli "açılış kapısıdır".
pub fn dosya_hazirla(
    yol: &std::path::Path,
    kaynak: impl Into<String>,
    yetkiler: &YetkiKapisi,
) -> Result<(VeriFormati, Provenans), ErrorReport> {
    // MK-13: eklenti dosya sistemine yalnızca ilan edilmiş+onaylanmış `fs` yetkisiyle erişir.
    yetkiler.iste(Capability::Fs)?;
    let format = formati_belirle(yol)?;
    let provenans = Provenans::olustur(yol, kaynak, format)?;
    Ok((format, provenans))
}

/// Alt-modülün UI/komut kayıtları — veri yükleme komutları yalnızca `fs` yetkisi verildiyse açılır
/// (en az yetki + dürüstlük: dosya erişemeyecek komutu hiç sunma; ÇE-09 `db_search` deseniyle aynı).
pub fn kayitlar(yetkiler: &YetkiKapisi) -> Aktivasyon {
    let mut akt = Aktivasyon::yeni();
    if yetkiler.var_mi(Capability::Fs) {
        akt.komut(
            "biocraft.core.studio.data.ac",
            "BioCraft Studio: Veri Dosyası Aç (FASTA/FASTQ/BAM/SAM/CRAM)",
        )
        .komut(
            "biocraft.core.studio.data.indeksle",
            "BioCraft Studio: Veri İndeksi Oluştur (.fai/.bai)",
        );
    }
    akt
}

#[cfg(test)]
mod tests {
    use super::*;
    use biocraft_sdk::ui::UiUzantiTuru;
    use std::io::Write;

    fn yaz(ad: &str, icerik: &[u8]) -> std::path::PathBuf {
        let mut yol = std::env::temp_dir();
        yol.push(format!("biocraft_dataio_{}_{ad}", std::process::id()));
        std::fs::File::create(&yol)
            .unwrap()
            .write_all(icerik)
            .unwrap();
        yol
    }

    #[test]
    fn fs_yoksa_veri_komutu_kaydedilmez() {
        let bos = kayitlar(&YetkiKapisi::bos());
        assert_eq!(bos.ui_say(UiUzantiTuru::Komut), 0);

        let fsli = kayitlar(&YetkiKapisi::yeni([Capability::Fs]));
        assert_eq!(fsli.ui_say(UiUzantiTuru::Komut), 2);
    }

    #[test]
    fn dosya_hazirla_fs_yoksa_reddeder() {
        let fa = yaz("h.fasta", b">sq0\nACGT\n");
        let hata = dosya_hazirla(&fa, "Test", &YetkiKapisi::bos()).unwrap_err();
        assert_eq!(hata.ne_oldu, "Eklenti erişimi reddedildi");
        let _ = std::fs::remove_file(&fa);
    }

    #[test]
    fn dosya_hazirla_format_ve_provenans_uretir() {
        let fa = yaz("h2.fasta", b">sq0\nACGTACGT\n");
        let (format, prov) = dosya_hazirla(
            &fa,
            "Kullanıcı yüklemesi",
            &YetkiKapisi::yeni([Capability::Fs]),
        )
        .unwrap();
        assert_eq!(format, VeriFormati::Fasta);
        assert_eq!(prov.format, "FASTA");
        assert_eq!(prov.blake3.len(), 64);
        assert!(prov.boyut_bayt > 0);
        let _ = std::fs::remove_file(&fa);
    }
}
