//! ÇE-01 — **Veri G/Ç ve format ayrıştırıcıları**.
//!
//! Çekirdek eklentinin veri okuma temeli; **BGZF blok-farkında** okuma (MK-32), **indeksli
//! rastgele erişim** (.fai/.bai/.csi/.crai/.tbi), **out-of-core** bölge sorgusu (yalnız görünen
//! pencere; MK-09), **BLAKE3 bütünlük** + **provenance** (MK-34, İP-10), **bellek bütçesi**
//! (İP-08) ve **uzak bayt-aralığı + KARANTİNA** (MK-33).
//!
//! Biyoinformatik ana formatlar (FASTA/FASTQ/SAM/BAM/CRAM/VCF/BCF/BED/GFF/GTF) `noodles`
//! (saf-Rust, MIT) ile okunur — doğruluk-kritik ikili (BGZF/BAM/CRAM) elle yazılmaz.  noodles'ta
//! olmayanlar (**2bit / GenBank / PDB / mmCIF**) **saf-Rust** elle yazılır (yeni dış bağımlılık
//! eklenmez); **BigWig/BigBed** karmaşık ikili indeksli olduğundan **ÇE-02'ye ertelenir** (sessiz
//! okuma yok).  `fs` yeteneği gerektirir (MK-13); uzak erişim `net` ister.
//!
//! ## Kapsam
//! * **Gün 34:** FASTA/FASTQ + BAM/SAM/CRAM.
//! * **Gün 35:** VCF/BCF + BED/GFF/GTF + GenBank + 2bit + PDB/mmCIF + uzak erişim/karantina;
//!   BigWig/BigBed → ÇE-02.

mod alignment;
mod annotation;
mod bgzf;
mod bigwig;
mod budget;
mod detect;
mod fasta;
mod genbank;
mod index;
mod integrity;
mod provenance;
mod remote;
mod structure;
mod twobit;
mod variant;

pub use alignment::{HizalamaBasligi, HizalamaKaydi, HizalamaOkuyucu};
pub use annotation::{AnotasyonBasligi, AnotasyonKaydi, AnotasyonOkuyucu};
pub use bgzf::{coz_ve_olc as bgzf_coz_ve_olc, BgzfOzet};
pub use bigwig::{bolge_sorgu_dene as bigwig_bolge_dene, durum as bigwig_durum, BigWigDurumu};
pub use budget::BellekButcesi;
pub use detect::{formati_belirle, VeriFormati};
pub use fasta::{
    fai_olustur, fai_yolu, fasta_akis, fastq_akis, DiziParcasi, FastaOkuyucu, KaliteOzeti,
};
pub use genbank::{genbank_akis, ilk_kayit as genbank_ilk_kayit, GenBankKaydi, GenBankOzellik};
pub use index::{indeks_durumu, indeks_olustur, IndeksDurumu};
pub use integrity::{blake3_dosya, dogrula_blake3};
pub use provenance::{LisansAtif, Provenans};
pub use remote::{
    bolge_baytlari, dogrula_veya_karantina, karantinaya_al, tekrar_ile, HttpOkuyucu,
    HttpYapilandirma, KarantinaSonucu, Onbellek, UzakOkuyucu, YerelBaytAralik,
};
pub use structure::{Atom, Yapi, YapiModeli};
pub use twobit::{TwoBitOkuyucu, TwoBitParca};
pub use variant::{VaryantBasligi, VaryantKaydi, VaryantOkuyucu};

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

/// Alt-modülün UI/komut kayıtları — yerel veri komutları yalnızca `fs`, uzak veri komutu yalnızca
/// `net` yetkisi verildiyse açılır (en az yetki + dürüstlük: dosya/ağ erişemeyecek komutu hiç
/// sunma; ÇE-09 `db_search` deseniyle aynı).
pub fn kayitlar(yetkiler: &YetkiKapisi) -> Aktivasyon {
    let mut akt = Aktivasyon::yeni();
    if yetkiler.var_mi(Capability::Fs) {
        akt.komut(
            "biocraft.core.studio.data.ac",
            "BioCraft Studio: Veri Dosyası Aç (FASTA/FASTQ/BAM/SAM/CRAM/VCF/BCF/BED/GFF/GTF/2bit/PDB/mmCIF/GenBank)",
        )
        .komut(
            "biocraft.core.studio.data.indeksle",
            "BioCraft Studio: Veri İndeksi Oluştur (.fai/.bai)",
        );
    }
    // Uzak (HTTP/S3) dosya açma yalnızca `net` ilan edilip onaylandıysa sunulur (MK-13).
    if yetkiler.var_mi(Capability::Net) {
        akt.komut(
            "biocraft.core.studio.data.uzak_ac",
            "BioCraft Studio: Uzak Veri Aç (HTTP/S3 bayt-aralığı)",
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

        // Yalnız fs → 2 yerel komut (uzak komut yok).
        let fsli = kayitlar(&YetkiKapisi::yeni([Capability::Fs]));
        assert_eq!(fsli.ui_say(UiUzantiTuru::Komut), 2);

        // fs + net → 2 yerel + 1 uzak = 3 komut.
        let fs_net = kayitlar(&YetkiKapisi::yeni([Capability::Fs, Capability::Net]));
        assert_eq!(fs_net.ui_say(UiUzantiTuru::Komut), 3);

        // Yalnız net → yerel yok, uzak var = 1 komut.
        let netli = kayitlar(&YetkiKapisi::yeni([Capability::Net]));
        assert_eq!(netli.ui_say(UiUzantiTuru::Komut), 1);
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
