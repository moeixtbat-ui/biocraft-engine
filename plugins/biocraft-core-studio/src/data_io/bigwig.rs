//! ÇE-01 — **BigWig / BigBed** — **bilinçli olarak ERTELENDİ** (ÇE-02 genom tarayıcı sinyal izi).
//!
//! BigWig/BigBed, UCSC'nin **karmaşık ikili indeksli** formatlarıdır: kromozom **B+ ağacı**,
//! veri için **R-ağacı** uzaysal indeksi, çok seviyeli **zoom** özetleri ve blok-içi **zlib**
//! sıkıştırma içerir.  Bu, BAM/CRAM ile aynı sınıfta **doğruluk-kritik** bir ikilidir → elle
//! yazmak (hatalı R-ağacı dolaşımı → yanlış/eksik sinyal) **sorumsuzluk** olur.
//!
//! Proje **"yeni dış bağımlılık yok"** ilkesini bu sürümde koruduğundan (kullanıcı kararı),
//! `bigtools` gibi bir crate **eklenmez**.  BigWig'i gerçekten **kullanacak** olan ÇE-02
//! (genom tarayıcı sinyal izi) ile birlikte ele alınır; o gün ya odaklı bir okuyucu yazılır ya
//! da o bağlamda bir bağımlılık kararı verilir.
//!
//! Bu modül **sessizce/yanlış okumaz**: format doğru tanınır, ama her okuma denemesi **net bir
//! "yapılandırılmadı / ileride" hatası** döndürür (MK-48 dürüstlük: çalışmayan özelliği çalışıyor
//! gibi gösterme).

use std::path::Path;

use biocraft_sdk::biocraft_types::ErrorReport;

use super::detect::{formati_belirle, VeriFormati};

/// BigWig/BigBed desteğinin bu sürümdeki durumu (UI "yapılandırılmadı" rozeti için).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BigWigDurumu {
    /// Bu sürümde okuma yapılandırılmadı; ÇE-02 ile gelecek.
    Ertelendi,
}

/// BigWig/BigBed desteğinin durumu.
pub fn durum() -> BigWigDurumu {
    BigWigDurumu::Ertelendi
}

/// Bir BigWig/BigBed bölge sorgusu **denemesi**.  Bu sürümde her zaman **net hata** döner
/// (sessiz/yanlış okuma yok); dosya gerçekten BigWig/BigBed ise format doğrulanır, sonra ertelenmiş
/// olduğu açıkça bildirilir.
pub fn bolge_sorgu_dene(yol: &Path, _bolge: &str) -> Result<(), ErrorReport> {
    let format = formati_belirle(yol)?;
    match format {
        VeriFormati::BigWig | VeriFormati::BigBed => Err(ertelendi_hatasi(format.etiket())),
        _ => Err(ErrorReport::new(
            "BigWig/BigBed dosyası değil",
            format!("'{}' bir BigWig/BigBed dosyası değil", yol.display()),
            "BigWig (.bw) veya BigBed (.bb) bir dosya seçin",
        )),
    }
}

fn ertelendi_hatasi(etiket: &str) -> ErrorReport {
    ErrorReport::new(
        format!("{etiket} okuma bu sürümde yapılandırılmadı"),
        "BigWig/BigBed karmaşık ikili indeksli (B+/R-ağacı + zoom + sıkıştırma) bir formattır; \
         doğru okuma ÇE-02 (genom tarayıcı sinyal izi) ile gelecek",
        "Sinyal izi için ÇE-02 hazır olduğunda bu dosya açılabilecek; şimdilik başka bir format kullanın",
    )
    .with_eylem("Daha sonra (ÇE-02)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    fn yaz_bigwig(ad: &str) -> std::path::PathBuf {
        // BigWig sihirli imzası 0x888FFC26 (little-endian: 26 FC 8F 88).
        let mut yol = std::env::temp_dir();
        yol.push(format!("biocraft_bw_{}_{ad}", std::process::id()));
        File::create(&yol)
            .unwrap()
            .write_all(&[0x26, 0xFC, 0x8F, 0x88, 0, 0, 0, 0])
            .unwrap();
        yol
    }

    #[test]
    fn bigwig_taninir_ama_ertelenmis_net_hata() {
        let p = yaz_bigwig("a.bw");
        let hata = bolge_sorgu_dene(&p, "chr1:1-100").err().unwrap();
        assert!(hata.ne_oldu.contains("yapılandırılmadı"));
        assert_eq!(hata.eylem_etiketi.as_deref(), Some("Daha sonra (ÇE-02)"));
        assert_eq!(durum(), BigWigDurumu::Ertelendi);
        let _ = std::fs::remove_file(&p);
    }
}
