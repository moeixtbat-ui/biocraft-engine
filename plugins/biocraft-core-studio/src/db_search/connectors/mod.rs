//! ÇE-09 — **Veritabanı konektörleri** (kaynak başına bir modül).
//!
//! Her konektör [`super::framework::VeritabaniKonektoru`] trait'ini uygular → çerçeve değişmeden
//! yeni kaynak eklenir (MK-41 genişletilebilirlik).
//!
//! * [`ncbi`] — NCBI E-utilities (nucleotide / protein / gene).  **[Gün 40]**
//! * [`blast`] — NCBI BLAST URL API (uzak dizi benzerlik araması).  **[Gün 40]**
//!
//! Gün 41: `pdb` (RCSB → ÇE-07), `uniprot`, `ensembl`, `ucsc`.

pub mod blast;
pub mod ncbi;

pub use blast::{BlastDurum, BlastIsi, BlastKonektor, BlastProgram, Hizalama, YoklamaAyari};
pub use ncbi::{NcbiKonektor, NcbiVeritabani};
