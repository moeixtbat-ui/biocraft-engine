//! Hafif ayar **arama indeksi** (İP-12) — anında filtre.
//!
//! Ayar listesi bir kez **önceden indekslenir** (her ayar için küçük-harfli bir "saman" dizgesi:
//! anahtar + iki dilde başlık + iki dilde açıklama + ek anahtar kelimeler).  Arama, her tuşta bu
//! hafif dizgelerde `contains` taraması yapar — ekran her karede tüm tanımları yeniden taramaz
//! ("arama yavaş" sorununa karşı: indeksle + filtrelemeyi hafif tut).
//!
//! Eşleştirme **alt-dizge** temellidir (fuzzy değil); birden çok sözcük girilirse **hepsi** (AND)
//! eşleşmelidir.  Bu, ayar gibi küçük/sabit bir küme için hem hızlı hem öngörülebilirdir.

use super::sections::{AyarKategorisi, AyarTanimi};

/// Tek bir ayarın arama girdisi (önceden hesaplanmış saman dizgesi).
#[derive(Debug, Clone)]
struct AramaGiris {
    anahtar: String,
    kategori: AyarKategorisi,
    /// Küçük-harfe indirgenmiş, aranabilir tüm metin (başlık+açıklama+kelimeler).
    saman: String,
}

/// Ayar tanımlarından bir kez kurulan arama indeksi.
#[derive(Debug, Clone, Default)]
pub struct AyarIndeks {
    girisler: Vec<AramaGiris>,
}

impl AyarIndeks {
    /// Verilen tanımlardan indeksi kurar (kayıt değişince — örn. eklenti eklenince — yeniden kurulur).
    pub fn olustur(tanimlar: &[AyarTanimi]) -> Self {
        let girisler = tanimlar
            .iter()
            .map(|d| {
                let saman = format!(
                    "{} {} {} {} {} {}",
                    d.anahtar,
                    d.baslik_tr,
                    d.baslik_en,
                    d.aciklama_tr,
                    d.aciklama_en,
                    d.anahtar_kelimeler
                )
                .to_lowercase();
                AramaGiris {
                    anahtar: d.anahtar.clone(),
                    kategori: d.kategori,
                    saman,
                }
            })
            .collect();
        Self { girisler }
    }

    /// İndeksteki ayar sayısı.
    pub fn len(&self) -> usize {
        self.girisler.len()
    }

    /// İndeks boş mu?
    pub fn is_empty(&self) -> bool {
        self.girisler.is_empty()
    }

    /// Bir sorgunun girişle eşleşip eşleşmediği: sorgudaki **tüm** sözcükler samanda geçmeli.
    fn eslesir(saman: &str, sozcukler: &[String]) -> bool {
        sozcukler.iter().all(|s| saman.contains(s.as_str()))
    }

    /// `sorgu`'ya uyan ayar anahtarlarını döndürür (indeks sırasını korur).
    ///
    /// Boş/yalnız-boşluk sorgu → tüm anahtarlar (filtre yok).
    pub fn ara(&self, sorgu: &str) -> Vec<&str> {
        let sozcukler: Vec<String> = sorgu
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        self.girisler
            .iter()
            .filter(|g| sozcukler.is_empty() || Self::eslesir(&g.saman, &sozcukler))
            .map(|g| g.anahtar.as_str())
            .collect()
    }

    /// `sorgu`'ya uyan ve **belirli bir kategoriye** ait anahtarları döndürür.
    ///
    /// Ekranın olağan akışı: bir kategori seçiliyken o kategorinin ayarlarını sorguya göre süz.
    pub fn ara_kategori(&self, sorgu: &str, kategori: AyarKategorisi) -> Vec<&str> {
        let sozcukler: Vec<String> = sorgu
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        self.girisler
            .iter()
            .filter(|g| g.kategori == kategori)
            .filter(|g| sozcukler.is_empty() || Self::eslesir(&g.saman, &sozcukler))
            .map(|g| g.anahtar.as_str())
            .collect()
    }

    /// Bir sorgunun **en az bir** sonuç verdiği kategoriler (sıralı, tekrarsız) — kategori
    /// listesini ararken "boş kategorileri gizle/soldur" için.
    pub fn eslesen_kategoriler(&self, sorgu: &str) -> Vec<AyarKategorisi> {
        let sozcukler: Vec<String> = sorgu
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let mut sonuc: Vec<AyarKategorisi> = Vec::new();
        for g in &self.girisler {
            if (sozcukler.is_empty() || Self::eslesir(&g.saman, &sozcukler))
                && !sonuc.contains(&g.kategori)
            {
                sonuc.push(g.kategori);
            }
        }
        sonuc
    }
}

#[cfg(test)]
mod tests {
    use super::super::sections::yerlesik_tanimlar;
    use super::*;

    fn indeks() -> AyarIndeks {
        AyarIndeks::olustur(&yerlesik_tanimlar())
    }

    #[test]
    fn bos_sorgu_hepsini_dondurur() {
        let i = indeks();
        assert_eq!(i.ara("").len(), i.len());
        assert_eq!(i.ara("   ").len(), i.len());
    }

    #[test]
    fn arama_bir_ayari_bulur() {
        // Kabul kriteri: arama bir ayarı anında bulur.
        let i = indeks();
        // Başlığa göre.
        let r = i.ara("tema");
        assert!(r.contains(&"gorunum.tema"), "tema bulunmalı: {r:?}");
        // Anahtar kelimeye göre (başlıkta "FPS" geçer, kelimelerde "framerate").
        let r2 = i.ara("framerate");
        assert!(r2.contains(&"performans.fps_goster"));
        // İngilizce açıklamadaki sözcükle.
        let r3 = i.ara("temperature");
        assert!(r3.contains(&"performans.sicaklik_goster"));
    }

    #[test]
    fn coklu_sozcuk_and_eslesir() {
        let i = indeks();
        // "token" + "göster" yalnızca token sayacı ayarında birlikte geçer.
        let r = i.ara("token göster");
        assert!(r.contains(&"ai.token_sayaci_goster"));
        // Birbiriyle ilgisiz iki sözcük → sonuç yok.
        assert!(i.ara("tema framerate").is_empty());
    }

    #[test]
    fn olmayan_terim_bos_doner() {
        let i = indeks();
        assert!(i.ara("zzzbulunmazterim").is_empty());
    }

    #[test]
    fn kategori_filtresi_calisir() {
        let i = indeks();
        let r = i.ara_kategori("", AyarKategorisi::Ai);
        assert!(r.contains(&"ai.etkin"));
        // Editör ayarı AI kategorisinde çıkmamalı.
        assert!(!r.contains(&"editor.satir_numaralari"));
    }

    #[test]
    fn eslesen_kategoriler_dogru() {
        let i = indeks();
        // "bildirim" yalnız Görünüm kategorisinde.
        let k = i.eslesen_kategoriler("bildirim");
        assert_eq!(k, vec![AyarKategorisi::Gorunum]);
    }
}
