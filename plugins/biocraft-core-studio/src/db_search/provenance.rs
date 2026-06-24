//! ÇE-09 / İP-10 — **Veritabanı kaynaklı köken (provenance)** yardımcısı.
//!
//! İndirilen her kayıt için kaynak / accession / **erişim tarihi** / BLAKE3 + (bilimsel set ise)
//! lisans/atıf yükümlülüğü kaydedilir (ÇE-09 "Provenance/atıf/lisans"; İP-10 köken günlüğü).
//! `data_io::provenance::Provenans` ile **aynı tip** kullanılır; tek fark, içerik dosyada değil
//! **bellekte** (DB indirmesi) olduğundan BLAKE3 bayttan hesaplanır.

use chrono::Utc;

use crate::data_io::{LisansAtif, Provenans};

/// Bir DB kaydının içeriğinden köken kaydı üretir (erişim tarihi = şimdi, UTC).
///
/// * `veri_kimligi` — mantıksal ad (örn. `NM_007294.fasta`).
/// * `kaynak` — insan-okur kaynak (örn. `NCBI nucleotide (efetch)`).
/// * `format` — `data_io::detect` etiketiyle uyumlu (örn. `FASTA`, `PDB`).
/// * `lisans_atif` — kamuya açık bilimsel set yükümlülüğü (NCBI = Public Domain) veya `None`.
pub fn db_provenansi(
    veri_kimligi: impl Into<String>,
    kaynak: impl Into<String>,
    format: impl Into<String>,
    icerik: &[u8],
    lisans_atif: Option<LisansAtif>,
) -> Provenans {
    Provenans {
        veri_kimligi: veri_kimligi.into(),
        kaynak: kaynak.into(),
        format: format.into(),
        surum: String::new(),
        tarih: Utc::now(),
        blake3: blake3::hash(icerik).to_hex().to_string(),
        boyut_bayt: icerik.len() as u64,
        lisans_atif,
    }
}

/// NCBI verisi için standart lisans/atıf (E-utilities içeriği kamuya açıktır).
pub fn ncbi_lisans_atif() -> LisansAtif {
    LisansAtif {
        lisans: "Public Domain (US Government work)".to_string(),
        atif: "NCBI, National Library of Medicine (NLM), E-utilities".to_string(),
        url: Some("https://www.ncbi.nlm.nih.gov/home/about/policies/".to_string()),
    }
}

/// RCSB PDB yapı verisi için lisans/atıf.  PDB ana arşiv verisi kamuya açıktır (CC0 1.0).
pub fn pdb_lisans_atif() -> LisansAtif {
    LisansAtif {
        lisans: "CC0 1.0 (Public Domain Dedication)".to_string(),
        atif: "Berman HM, et al. The Protein Data Bank. Nucleic Acids Res. 2000;28:235-242. RCSB PDB (rcsb.org)".to_string(),
        url: Some("https://www.rcsb.org/pages/policies".to_string()),
    }
}

/// UniProt protein verisi için lisans/atıf (CC BY 4.0).
pub fn uniprot_lisans_atif() -> LisansAtif {
    LisansAtif {
        lisans: "CC BY 4.0".to_string(),
        atif: "The UniProt Consortium. UniProt: the Universal Protein Knowledgebase. Nucleic Acids Res. 2023;51:D523-D531.".to_string(),
        url: Some("https://www.uniprot.org/help/license".to_string()),
    }
}

/// Ensembl gen/dizi/anotasyon verisi için lisans/atıf (Apache 2.0; veri kısıtlamasız).
pub fn ensembl_lisans_atif() -> LisansAtif {
    LisansAtif {
        lisans: "Apache 2.0 (kod) / kısıtlamasız (veri)".to_string(),
        atif: "Martin FJ, et al. Ensembl 2023. Nucleic Acids Res. 2023;51:D933-D941. EMBL-EBI."
            .to_string(),
        url: Some("https://www.ensembl.org/info/about/legal/disclaimer.html".to_string()),
    }
}

/// UCSC Genom Tarayıcı verisi için lisans/atıf (akademik/kâr-amaçsız kullanım serbest; bazı
/// derlemelerde kısıt olabilir → kullanıcı atıfta görür).
pub fn ucsc_lisans_atif() -> LisansAtif {
    LisansAtif {
        lisans:
            "UCSC Genome Browser — akademik/kâr-amaçsız kullanım serbest (bazı derlemeler kısıtlı)"
                .to_string(),
        atif: "Kent WJ, et al. The Human Genome Browser at UCSC. Genome Res. 2002;12:996-1006."
            .to_string(),
        url: Some("https://genome.ucsc.edu/conditions.html".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenans_blake3_boyut_ve_lisans_doldurur() {
        let icerik = b">NM_007294\nACGTACGT\n";
        let p = db_provenansi(
            "NM_007294.fasta",
            "NCBI nucleotide (efetch)",
            "FASTA",
            icerik,
            Some(ncbi_lisans_atif()),
        );
        assert_eq!(p.veri_kimligi, "NM_007294.fasta");
        assert_eq!(p.format, "FASTA");
        assert_eq!(p.boyut_bayt, icerik.len() as u64);
        assert_eq!(p.blake3.len(), 64);
        assert_eq!(p.blake3, blake3::hash(icerik).to_hex().to_string());
        let la = p.lisans_atif.unwrap();
        assert!(la.lisans.contains("Public Domain"));
        assert!(la.atif.contains("NCBI"));
    }

    #[test]
    fn json_serilestirilebilir() {
        let p = db_provenansi("a.fasta", "NCBI", "FASTA", b"ACGT", None);
        let js = p.to_json();
        assert!(js.contains("\"kaynak\":\"NCBI\""));
        assert!(js.contains("\"blake3\""));
    }
}
