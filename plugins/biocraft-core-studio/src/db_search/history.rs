//! ÇE-09 (Gün 41) — **Arama geçmişi + favoriler** (sorgu/kaynak/tarih; tekrar çalıştır; kaydet).
//!
//! ÇE-09 "Geçmiş: Son arama/yükleme kaydedilir, tekrar çalıştırılır, projeyle ilişkilendirilir."
//!
//! Geçmiş bellekte tutulur (saf; birim-testlenir) ve isteğe bağlı bir JSON dosyasına **kalıcı**
//! yazılır (proje alanı; `fs` gerektirir).  Sığa (`azami`) aşılınca **en eski favori-olmayan**
//! girdi düşer → favoriler korunur (kullanıcı "kaydet" dediği aramayı kaybetmez).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use biocraft_sdk::biocraft_types::{ErrorReport, Timestamp};

use super::framework::{KayitTuru, Sorgu};

/// Tek bir geçmiş girdisi (bir arama olayı).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GecmisGirdisi {
    /// Aranan serbest metin (gen adı / accession / dizi / anahtar kelime).
    pub sorgu: String,
    /// O aramada seçili olan kaynaklar (rozet adları).
    pub kaynaklar: Vec<String>,
    /// Aranan kayıt türü ipucu (varsa).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tur: Option<KayitTuru>,
    /// Arama tarihi (UTC).
    pub tarih: Timestamp,
    /// Dönen toplam sonuç sayısı (özet).
    pub sonuc_sayisi: usize,
    /// Kullanıcı bunu favori/kaydedilmiş işaretledi mi (sığa budamasından korunur).
    #[serde(default)]
    pub favori: bool,
}

impl GecmisGirdisi {
    /// Yeni girdi (şimdi, UTC; favori değil).
    pub fn yeni(sorgu: impl Into<String>, kaynaklar: Vec<String>, sonuc_sayisi: usize) -> Self {
        Self {
            sorgu: sorgu.into(),
            kaynaklar,
            tur: None,
            tarih: chrono::Utc::now(),
            sonuc_sayisi,
            favori: false,
        }
    }

    /// Bu girdiyi **tekrar çalıştırmak** için sorgu nesnesi üretir (panel sorgu kutusuna yüklenir).
    pub fn sorgu(&self) -> Sorgu {
        Sorgu {
            metin: self.sorgu.clone(),
            tur: self.tur,
        }
    }
}

/// Arama geçmişi (en yeni başta) + favoriler + sığa.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AramaGecmisi {
    /// En yeni başta sıralı girdiler.
    girdiler: Vec<GecmisGirdisi>,
    /// Favori-olmayan girdiler için sığa (favoriler sayılmaz).
    azami: usize,
}

impl AramaGecmisi {
    /// Boş geçmiş (verilen sığa ile).
    pub fn yeni(azami: usize) -> Self {
        Self {
            girdiler: Vec::new(),
            azami: azami.max(1),
        }
    }

    /// Bir arama olayını kaydeder (en başa ekler; sığa aşılırsa en eski favori-olmayanı düşürür).
    /// Aynı (sorgu+kaynaklar) en üstte tekrar varsa **taşınır** (yinelenen biriktirilmez).
    pub fn ekle(&mut self, girdi: GecmisGirdisi) {
        // Aynı sorgu+kaynak kombinasyonunu (favori durumunu koruyarak) kaldır.
        if let Some(idx) = self
            .girdiler
            .iter()
            .position(|g| g.sorgu == girdi.sorgu && g.kaynaklar == girdi.kaynaklar)
        {
            let eski = self.girdiler.remove(idx);
            // Daha önce favori yapılmışsa favoriliği koru.
            let mut yeni = girdi;
            yeni.favori = yeni.favori || eski.favori;
            self.girdiler.insert(0, yeni);
        } else {
            self.girdiler.insert(0, girdi);
        }
        self.budama();
    }

    /// Tüm girdiler (en yeni başta).
    pub fn girdiler(&self) -> &[GecmisGirdisi] {
        &self.girdiler
    }

    /// Yalnız favori (kaydedilmiş) girdiler.
    pub fn favoriler(&self) -> Vec<&GecmisGirdisi> {
        self.girdiler.iter().filter(|g| g.favori).collect()
    }

    /// Bir girdinin favori durumunu değiştirir (indeks sınır dışıysa yok sayar).
    pub fn favori_degistir(&mut self, indeks: usize) {
        if let Some(g) = self.girdiler.get_mut(indeks) {
            g.favori = !g.favori;
        }
    }

    /// Bir geçmiş girdisini **tekrar çalıştırmak** için sorgusunu döndürür (yoksa `None`).
    pub fn tekrar_calistir(&self, indeks: usize) -> Option<Sorgu> {
        self.girdiler.get(indeks).map(|g| g.sorgu())
    }

    /// Favori-olmayan tüm girdileri temizler (favoriler kalır — "geçmişi temizle").
    pub fn temizle(&mut self) {
        self.girdiler.retain(|g| g.favori);
    }

    /// Girdi sayısı.
    pub fn len(&self) -> usize {
        self.girdiler.len()
    }

    /// Geçmiş boş mu?
    pub fn is_empty(&self) -> bool {
        self.girdiler.is_empty()
    }

    /// Sığa aşıldıysa en eski **favori-olmayan** girdileri düşürür.
    fn budama(&mut self) {
        let favori_olmayan = self.girdiler.iter().filter(|g| !g.favori).count();
        if favori_olmayan <= self.azami {
            return;
        }
        let mut atilacak = favori_olmayan - self.azami;
        // Sondan (en eski) başlayarak favori-olmayanları at.
        let mut i = self.girdiler.len();
        while i > 0 && atilacak > 0 {
            i -= 1;
            if !self.girdiler[i].favori {
                self.girdiler.remove(i);
                atilacak -= 1;
            }
        }
    }

    // ─── Kalıcılık (opsiyonel; fs) ─────────────────────────────────────────────────

    /// Geçmişi bir JSON dosyasına yazar (proje alanı; `fs`).
    pub fn kaydet(&self, yol: &Path) -> Result<(), ErrorReport> {
        let js = serde_json::to_string_pretty(self).map_err(|e| {
            ErrorReport::new(
                "Geçmiş kaydedilemedi",
                "arama geçmişi JSON'a çevrilemedi",
                "Sorun sürerse geçmişi temizleyip yeniden deneyin",
            )
            .with_teknik_detay(e.to_string())
        })?;
        std::fs::write(yol, js).map_err(|e| {
            ErrorReport::new(
                "Geçmiş dosyası yazılamadı",
                format!("'{}' yazılamadı", yol.display()),
                "Yolu, izinleri ve disk alanını kontrol edin",
            )
            .with_teknik_detay(e.to_string())
        })
    }

    /// Bir JSON dosyasından geçmişi yükler; dosya yoksa **boş** geçmiş döndürür (ilk açılış).
    pub fn yukle(yol: &Path, azami: usize) -> Result<Self, ErrorReport> {
        let yol_buf: PathBuf = yol.to_path_buf();
        match std::fs::read_to_string(&yol_buf) {
            Ok(js) => {
                let mut g: AramaGecmisi = serde_json::from_str(&js).map_err(|e| {
                    ErrorReport::new(
                        "Geçmiş dosyası çözümlenemedi",
                        "geçmiş JSON'u beklenen biçimde değil (bozuk olabilir)",
                        "Dosyayı silip yeniden başlayın",
                    )
                    .with_teknik_detay(e.to_string())
                })?;
                g.azami = azami.max(1);
                g.budama();
                Ok(g)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::yeni(azami)),
            Err(e) => Err(ErrorReport::new(
                "Geçmiş dosyası okunamadı",
                format!("'{}' okunamadı", yol_buf.display()),
                "Yolu ve izinleri kontrol edin",
            )
            .with_teknik_detay(e.to_string())),
        }
    }
}

impl Default for AramaGecmisi {
    /// 200 favori-olmayan girdi sığalı varsayılan geçmiş.
    fn default() -> Self {
        Self::yeni(200)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(sorgu: &str, sonuc: usize) -> GecmisGirdisi {
        GecmisGirdisi::yeni(sorgu, vec!["NCBI nucleotide".to_string()], sonuc)
    }

    #[test]
    fn ekleme_en_yeni_basta() {
        let mut h = AramaGecmisi::yeni(10);
        h.ekle(g("p53", 2));
        h.ekle(g("BRCA1", 3));
        assert_eq!(h.girdiler()[0].sorgu, "BRCA1");
        assert_eq!(h.girdiler()[1].sorgu, "p53");
    }

    #[test]
    fn tekrar_calistir_sorgu_doner() {
        let mut h = AramaGecmisi::yeni(10);
        h.ekle(g("TP53", 5));
        let sorgu = h.tekrar_calistir(0).unwrap();
        assert_eq!(sorgu.metin, "TP53");
    }

    #[test]
    fn yinelenen_tasinir_birikmez() {
        let mut h = AramaGecmisi::yeni(10);
        h.ekle(g("p53", 2));
        h.ekle(g("BRCA1", 3));
        h.ekle(g("p53", 9)); // aynı sorgu+kaynak → taşınır
        assert_eq!(h.len(), 2);
        assert_eq!(h.girdiler()[0].sorgu, "p53");
        assert_eq!(h.girdiler()[0].sonuc_sayisi, 9);
    }

    #[test]
    fn siga_asilinca_en_eski_favori_olmayan_dusur() {
        let mut h = AramaGecmisi::yeni(2);
        h.ekle(g("a", 1));
        h.ekle(g("b", 1));
        h.ekle(g("c", 1)); // "a" düşmeli
        assert_eq!(h.len(), 2);
        assert!(h.girdiler().iter().all(|x| x.sorgu != "a"));
    }

    #[test]
    fn favori_budamadan_korunur() {
        let mut h = AramaGecmisi::yeni(2);
        h.ekle(g("a", 1));
        h.favori_degistir(0); // "a" favori
        h.ekle(g("b", 1));
        h.ekle(g("c", 1));
        h.ekle(g("d", 1));
        // Favori "a" hâlâ var; favori-olmayan sığası 2.
        assert!(h.girdiler().iter().any(|x| x.sorgu == "a"));
        assert_eq!(h.favoriler().len(), 1);
        assert_eq!(h.girdiler().iter().filter(|x| !x.favori).count(), 2);
    }

    #[test]
    fn temizle_favorileri_birakir() {
        let mut h = AramaGecmisi::yeni(10);
        h.ekle(g("a", 1));
        h.ekle(g("b", 1));
        h.favori_degistir(0); // "b" favori
        h.temizle();
        assert_eq!(h.len(), 1);
        assert_eq!(h.girdiler()[0].sorgu, "b");
    }

    #[test]
    fn kaydet_yukle_round_trip() {
        let yol = std::env::temp_dir().join(format!("biocraft_gecmis_{}.json", std::process::id()));
        let mut h = AramaGecmisi::yeni(10);
        h.ekle(g("p53", 2));
        h.favori_degistir(0);
        h.kaydet(&yol).unwrap();

        let geri = AramaGecmisi::yukle(&yol, 10).unwrap();
        assert_eq!(geri.len(), 1);
        assert!(geri.girdiler()[0].favori);
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn yukle_olmayan_dosya_bos_dondurur() {
        let yol = std::env::temp_dir().join("biocraft_yok_dosya_xyz.json");
        let _ = std::fs::remove_file(&yol);
        let h = AramaGecmisi::yukle(&yol, 10).unwrap();
        assert!(h.is_empty());
    }
}
