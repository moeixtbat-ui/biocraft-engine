//! ÇE-09 — **Veritabanı konektörleri** (kaynak başına bir modül).
//!
//! Her konektör [`super::framework::VeritabaniKonektoru`] trait'ini uygular → çerçeve değişmeden
//! yeni kaynak eklenir (MK-41 genişletilebilirlik).
//!
//! * [`ncbi`] — NCBI E-utilities (nucleotide / protein / gene).  **[Gün 40]**
//! * [`blast`] — NCBI BLAST URL API (uzak dizi benzerlik araması).  **[Gün 40]**
//! * [`pdb`] — RCSB PDB (3B yapı → ÇE-07 görüntüleyici).  **[Gün 41]**
//! * [`uniprot`] — UniProtKB (protein bilgisi/dizi).  **[Gün 41]**
//! * [`ensembl`] — Ensembl REST (gen/transkript/anotasyon + koordinat çapraz bağlantı).  **[Gün 41]**
//! * [`ucsc`] — UCSC Genom Tarayıcı API (derleme/iz/bölge).  **[Gün 41]**

pub mod blast;
pub mod ensembl;
pub mod ncbi;
pub mod pdb;
pub mod ucsc;
pub mod uniprot;

pub use blast::{BlastDurum, BlastIsi, BlastKonektor, BlastProgram, Hizalama, YoklamaAyari};
pub use ensembl::EnsemblKonektor;
pub use ncbi::{NcbiKonektor, NcbiVeritabani};
pub use pdb::PdbKonektor;
pub use ucsc::UcscKonektor;
pub use uniprot::UniprotKonektor;
