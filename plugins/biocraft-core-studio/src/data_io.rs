//! ÇE-01 — Veri G/Ç ve format ayrıştırıcıları **[iskelet]**.
//!
//! FASTA/FASTQ · BAM/SAM/CRAM · VCF/BCF · BED · GFF/GTF · BigWig/BigBed · Wig · 2bit ·
//! PDB/mmCIF · GenBank; indeksli + out-of-core + BGZF-farkında + BLAKE3 bütünlük + uzak erişim.
//! `fs` yeteneği gerektirir (uzak erişim ileride `net` ile).  Bugün yalnızca kayıt uzantı
//! noktası açıktır; ayrıştırıcılar sonraki günlerde (ÇE-01) eklenecek.

use biocraft_sdk::{Aktivasyon, YetkiKapisi};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-01";

/// Alt-modülün UI/komut/node kayıtları (şimdilik boş — uzantı noktası).
pub fn kayitlar(_yetkiler: &YetkiKapisi) -> Aktivasyon {
    Aktivasyon::yeni()
}
