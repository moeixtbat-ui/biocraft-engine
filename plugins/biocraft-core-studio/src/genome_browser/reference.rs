//! ÇE-02 — **Referans dizi izi** (baz dizisi) + **kodon/aminoasit çevirisi** (Gün 37).
//!
//! Yeterince yakınlaşınca ([`LodSeviyesi::Baz`](super::lod::LodSeviyesi)) referans genomun baz
//! dizisi (A/C/G/T/N) **renkli hücre + harf** olarak görünür; istenirse altında **3 okuma
//! çerçevesinin** (frame 0/1/2) aminoasit çevirisi gösterilir.  Çeviri **ileri veya geri şerit**
//! için doğru hesaplanır (geri şerit = ters-tümleyen → çevir; her kodonun **genom koordinatı**
//! korunur → "çerçeve/şerit yanlış" hatası yapısal olarak test edilir).
//!
//! Referans dizi **out-of-core** yüklenir (MK-09): yalnız görünen pencerenin bazları FASTA
//! (`.fai` indeksli) veya UCSC 2bit'ten çekilir; geniş bölgede (baz görünmeyecek kadar uzak)
//! hiç yüklenmez (akıcılık — MK-04).

use biocraft_sdk::biocraft_types::ErrorReport;

use super::canvas::{GenomBolge, Tuval};
use super::lod::{lod_sec, LodSeviyesi};
use super::veri::Serit;
use crate::data_io::{BellekButcesi, FastaOkuyucu, TwoBitOkuyucu};

/// Görünen pencerenin referans bazları (1-tabanlı; yalnız bu parça bellekte — MK-09).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferansDizi {
    /// Kromozom/kontig adı.
    pub kromozom: String,
    /// İlk bazın 1-tabanlı genom konumu.
    pub baslangic: u64,
    /// Bazlar (ham bayt: `A`/`C`/`G`/`T`/`N`, 2bit maske küçük harf olabilir).
    pub bazlar: Vec<u8>,
}

impl ReferansDizi {
    /// Bir bazın 1-tabanlı genom konumunu verir (`indeks` 0-tabanlı).
    pub fn konum(&self, indeks: usize) -> u64 {
        self.baslangic + indeks as u64
    }

    /// Bir genom konumundaki bazı döndürür (bölge dışıysa `None`).
    pub fn baz(&self, pos: u64) -> Option<u8> {
        if pos < self.baslangic {
            return None;
        }
        self.bazlar.get((pos - self.baslangic) as usize).copied()
    }
}

/// Tek bir çevrilmiş kodon → aminoasit (genom koordinatı korunur; her şerit/çerçeve için).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kodon {
    /// Kodonun kapladığı 1-tabanlı başlangıç (`bas <= bit`).
    pub bas: u64,
    /// Kodonun kapladığı 1-tabanlı bitiş (kapsayıcı; `bit = bas + 2`).
    pub bit: u64,
    /// Aminoasit tek-harf kodu (`*` = dur kodonu; `X` = belirsiz/N içeren).
    pub amino: char,
}

impl Kodon {
    /// Dur (stop) kodonu mu?
    pub fn dur_mu(self) -> bool {
        self.amino == '*'
    }
}

/// Bir bazın tümleyeni (A↔T, C↔G; diğer/N → `N`).  Büyük/küçük harf büyük harfe indirilir.
pub fn tumleyen(b: u8) -> u8 {
    match b.to_ascii_uppercase() {
        b'A' => b'T',
        b'T' => b'A',
        b'C' => b'G',
        b'G' => b'C',
        _ => b'N',
    }
}

/// Bir dizinin **ters-tümleyeni** (geri şerit okuması): tümleyen alınır ve dizi ters çevrilir.
pub fn ters_tumleyen(bazlar: &[u8]) -> Vec<u8> {
    bazlar.iter().rev().map(|&b| tumleyen(b)).collect()
}

/// Standart genetik kod (DNA; T kullanılır): bir kodonu (3 baz) tek-harf aminoasite çevirir.
/// Geçersiz uzunluk veya `N`/belirsiz baz → `X`.
pub fn kodon_amino(kodon: &[u8]) -> char {
    if kodon.len() != 3 {
        return 'X';
    }
    let k = [
        kodon[0].to_ascii_uppercase(),
        kodon[1].to_ascii_uppercase(),
        kodon[2].to_ascii_uppercase(),
    ];
    match &k {
        b"TTT" | b"TTC" => 'F',
        b"TTA" | b"TTG" | b"CTT" | b"CTC" | b"CTA" | b"CTG" => 'L',
        b"ATT" | b"ATC" | b"ATA" => 'I',
        b"ATG" => 'M',
        b"GTT" | b"GTC" | b"GTA" | b"GTG" => 'V',
        b"TCT" | b"TCC" | b"TCA" | b"TCG" | b"AGT" | b"AGC" => 'S',
        b"CCT" | b"CCC" | b"CCA" | b"CCG" => 'P',
        b"ACT" | b"ACC" | b"ACA" | b"ACG" => 'T',
        b"GCT" | b"GCC" | b"GCA" | b"GCG" => 'A',
        b"TAT" | b"TAC" => 'Y',
        b"TAA" | b"TAG" | b"TGA" => '*',
        b"CAT" | b"CAC" => 'H',
        b"CAA" | b"CAG" => 'Q',
        b"AAT" | b"AAC" => 'N',
        b"AAA" | b"AAG" => 'K',
        b"GAT" | b"GAC" => 'D',
        b"GAA" | b"GAG" => 'E',
        b"TGT" | b"TGC" => 'C',
        b"TGG" => 'W',
        b"CGT" | b"CGC" | b"CGA" | b"CGG" | b"AGA" | b"AGG" => 'R',
        b"GGT" | b"GGC" | b"GGA" | b"GGG" => 'G',
        _ => 'X',
    }
}

/// Bir referans dizinin verilen **çerçeve** (0/1/2) ve **şerit** için aminoasit çevirisini üretir.
///
/// * **İleri şerit:** kodonlar `cerceve` ofsetinden başlayarak soldan sağa okunur; her kodonun
///   genom koordinatı (`bas..=bit`) doğrudan dizideki yeridir.
/// * **Geri şerit:** dizi ters-tümlenir ve çevrilir; her kodon, **orijinal** genom
///   koordinatlarına geri eşlenir (sağdan sola okunsa da koordinat 5'→3' yönünde verilir).
pub fn cevir(referans: &ReferansDizi, cerceve: u8, serit: Serit) -> Vec<Kodon> {
    let n = referans.bazlar.len();
    let cerceve = (cerceve % 3) as usize;
    let mut kodonlar = Vec::new();

    if serit == Serit::Geri {
        let rc = ters_tumleyen(&referans.bazlar);
        let mut j = cerceve;
        while j + 3 <= n {
            let amino = kodon_amino(&rc[j..j + 3]);
            // rc indeksi j → orijinal indeks (n-1-j); kodon rc[j..j+3] orijinalde
            // [n-1-(j+2) .. n-1-j] aralığına denk gelir.
            let orij_son = n - 1 - j; // rc[j]'nin orijinal indeksi (büyük uç)
            let orij_bas = n - 1 - (j + 2); // rc[j+2]'nin orijinal indeksi (küçük uç)
            kodonlar.push(Kodon {
                bas: referans.konum(orij_bas),
                bit: referans.konum(orij_son),
                amino,
            });
            j += 3;
        }
    } else {
        let mut i = cerceve;
        while i + 3 <= n {
            let amino = kodon_amino(&referans.bazlar[i..i + 3]);
            kodonlar.push(Kodon {
                bas: referans.konum(i),
                bit: referans.konum(i + 2),
                amino,
            });
            i += 3;
        }
    }
    kodonlar
}

/// Çeviri görünüm durumu — referans izinin altında çevirinin gösterilip gösterilmeyeceği + şerit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CeviriDurumu {
    /// 3 çerçeveli aminoasit çevirisi gösterilsin mi?
    pub goster: bool,
    /// Hangi şerit çevrilsin (ileri = +, geri = −)?
    pub serit: Serit,
}

impl Default for CeviriDurumu {
    fn default() -> Self {
        Self {
            goster: false,
            serit: Serit::Ileri,
        }
    }
}

impl CeviriDurumu {
    /// Üç çerçeveyi de (0,1,2) ilgili şerit için üretir (çizim için hazır).
    pub fn cerceveler(&self, referans: &ReferansDizi) -> [Vec<Kodon>; 3] {
        [
            cevir(referans, 0, self.serit),
            cevir(referans, 1, self.serit),
            cevir(referans, 2, self.serit),
        ]
    }
}

// ─── Out-of-core yükleme eşiği + yükleyiciler ───────────────────────────────────

/// Bu yakınlaşmada baz dizisi anlamlı görünür mü? (Baz LOD = baz başına ≥ birkaç piksel.)
/// Üst katman, yalnız `true` ise referans yükler → geniş bölgede gereksiz okuma yapılmaz (MK-09).
pub fn referans_gerekli(tuval: &Tuval) -> bool {
    // Öğe sayısı LOD'u etkilemez (referansta öğe yok); yalnız bp/piksel'e bakılır → Baz mı?
    lod_sec(tuval.bp_per_piksel(), 0, usize::MAX) == LodSeviyesi::Baz
}

/// Görünen pencerenin referans bazlarını **indeksli FASTA**'dan yükler (yalnız bu parça — MK-09).
pub fn gorunur_referans_fasta(
    okuyucu: &FastaOkuyucu,
    bolge: &GenomBolge,
    butce: &BellekButcesi,
) -> Result<ReferansDizi, ErrorReport> {
    let parca = okuyucu.bolge(&bolge.etiket(), butce)?;
    Ok(ReferansDizi {
        kromozom: bolge.kromozom.clone(),
        baslangic: bolge.baslangic,
        bazlar: parca.diziler,
    })
}

/// Görünen pencerenin referans bazlarını **UCSC 2bit**'ten yükler (ofset hesabıyla out-of-core).
pub fn gorunur_referans_2bit(
    okuyucu: &TwoBitOkuyucu,
    bolge: &GenomBolge,
    butce: &BellekButcesi,
) -> Result<ReferansDizi, ErrorReport> {
    let parca = okuyucu.bolge(&bolge.etiket(), butce)?;
    Ok(ReferansDizi {
        kromozom: bolge.kromozom.clone(),
        baslangic: bolge.baslangic,
        bazlar: parca.diziler,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dizi(bas: u64, s: &str) -> ReferansDizi {
        ReferansDizi {
            kromozom: "chr1".into(),
            baslangic: bas,
            bazlar: s.bytes().collect(),
        }
    }

    #[test]
    fn tumleyen_ve_ters_tumleyen() {
        assert_eq!(tumleyen(b'A'), b'T');
        assert_eq!(tumleyen(b'g'), b'C');
        assert_eq!(tumleyen(b'X'), b'N');
        // ATGC → (tümleyen TACG) → ters GCAT.
        assert_eq!(ters_tumleyen(b"ATGC"), b"GCAT");
    }

    #[test]
    fn kodon_tablosu_standart() {
        assert_eq!(kodon_amino(b"ATG"), 'M'); // başlangıç
        assert_eq!(kodon_amino(b"TAA"), '*'); // dur
        assert_eq!(kodon_amino(b"TGA"), '*');
        assert_eq!(kodon_amino(b"TTT"), 'F');
        assert_eq!(kodon_amino(b"GGG"), 'G');
        assert_eq!(kodon_amino(b"NNN"), 'X');
        assert_eq!(kodon_amino(b"AT"), 'X'); // eksik
    }

    #[test]
    fn ileri_cevirisi_koordinatli() {
        // chr1:10'dan başlayan "ATGTTTTAA" → M F * (çerçeve 0).
        let r = dizi(10, "ATGTTTTAA");
        let k = cevir(&r, 0, Serit::Ileri);
        assert_eq!(k.len(), 3);
        assert_eq!((k[0].bas, k[0].bit, k[0].amino), (10, 12, 'M'));
        assert_eq!((k[1].bas, k[1].bit, k[1].amino), (13, 15, 'F'));
        assert_eq!((k[2].bas, k[2].bit, k[2].amino), (16, 18, '*'));
        assert!(k[2].dur_mu());

        // Çerçeve 1 → bir baz kaydırır.
        let k1 = cevir(&r, 1, Serit::Ileri);
        assert_eq!(k1[0].bas, 11);
    }

    #[test]
    fn geri_cevirisi_dogru_serit_ve_koordinat() {
        // İleri "ATGTTTTAA"; geri şerit = ters-tümleyen "TTAAAACAT" → çerçeve 0: TTA AAA CAT = L K H.
        let r = dizi(10, "ATGTTTTAA");
        let k = cevir(&r, 0, Serit::Geri);
        assert_eq!(k.len(), 3);
        let aminolar: String = k.iter().map(|c| c.amino).collect();
        assert_eq!(aminolar, "LKH");
        // İlk geri kodon dizinin SAĞ ucundadır (genom koordinatı büyük uç korunur).
        assert_eq!((k[0].bas, k[0].bit), (16, 18));
        assert_eq!((k[2].bas, k[2].bit), (10, 12));
    }

    #[test]
    fn referans_baz_ve_konum() {
        let r = dizi(100, "ACGT");
        assert_eq!(r.konum(0), 100);
        assert_eq!(r.baz(100), Some(b'A'));
        assert_eq!(r.baz(103), Some(b'T'));
        assert_eq!(r.baz(104), None); // dışı
        assert_eq!(r.baz(99), None);
    }

    #[test]
    fn referans_gerekli_yakinlasinca() {
        // 100 bp / 1000 px = 0.1 bp/px ≤ 0.125 → Baz → gerekli.
        let yakin = Tuval::yeni(1000.0, GenomBolge::yeni("chr1", 1, 100).unwrap());
        assert!(referans_gerekli(&yakin));
        // 100 kb / 1000 px = 100 bp/px → çok uzak → gereksiz.
        let uzak = Tuval::yeni(1000.0, GenomBolge::yeni("chr1", 1, 100_000).unwrap());
        assert!(!referans_gerekli(&uzak));
    }

    #[test]
    fn cerceveler_uctan_uca() {
        let r = dizi(1, "ATGAAATTTGGG");
        let c = CeviriDurumu {
            goster: true,
            serit: Serit::Ileri,
        };
        let f = c.cerceveler(&r);
        assert_eq!(f[0].len(), 4); // ATG AAA TTT GGG
        assert_eq!(f[0][0].amino, 'M');
    }
}
