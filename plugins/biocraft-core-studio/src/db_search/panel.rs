//! ÇE-09 — **Birleşik arama paneli** (durum makinesi; render-bağımsız).
//!
//! Tek arama kutusu → **seçili** konektörlerde ara → **kaynak rozetli birleşik sonuç**.  Her
//! sonuçtan eylem: **tarayıcıda aç** (dış web sayfası) / **yapıya bak** (ÇE-07 3B görüntüleyici) /
//! **projeye ekle** (indir + provenance).  Çizim motor (egui) tarafında; bu modül **veri + durum**
//! üretir (MK-17).
//!
//! **Federe dayanıklılık:** Bir kaynak hata verse (çevrimdışı, rate-limit, onay yok) bile diğer
//! kaynaklar sonuç döndürmeye devam eder; hatalar [`son_hatalar`](BirlesikPanel::son_hatalar)'da
//! kaynak adıyla toplanır (kullanıcıya kaynak-başına durum gösterilir).

use biocraft_sdk::biocraft_types::ErrorReport;

use super::framework::{
    AramaBaglami, AramaSonucu, GetirilenKayit, KayitTuru, Sayfalama, Sorgu, VeritabaniKonektoru,
};

/// Bir sonuç satırından yapılabilecek eylem (UI buton/menü).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Eylem {
    /// Kaynağın web sayfasını dış tarayıcıda aç (dış erişim — `net` + onay; host açar).
    TarayicidaAc(String),
    /// 3B yapı görüntüleyiciye yükle (ÇE-07) — yalnız [`KayitTuru::Yapi`].
    YapiyaBak(String),
    /// Kaydı indir + projeye ekle (provenance ile).
    ProjeyeEkle(String),
}

/// Birleşik panel — kayıtlı konektörler + seçim + son sorgu + birleşik sonuçlar.
pub struct BirlesikPanel {
    konektorler: Vec<Box<dyn VeritabaniKonektoru>>,
    secili: Vec<bool>,
    /// Arama kutusu metni.
    pub sorgu_metni: String,
    sonuclar: Vec<AramaSonucu>,
    son_hatalar: Vec<(String, String)>,
}

impl BirlesikPanel {
    /// Boş panel (konektör eklenmemiş).
    pub fn yeni() -> Self {
        Self {
            konektorler: Vec::new(),
            secili: Vec::new(),
            sorgu_metni: String::new(),
            sonuclar: Vec::new(),
            son_hatalar: Vec::new(),
        }
    }

    /// Bir konektör ekler (varsayılan: seçili).
    pub fn konektor_ekle(&mut self, konektor: Box<dyn VeritabaniKonektoru>) -> &mut Self {
        self.konektorler.push(konektor);
        self.secili.push(true);
        self
    }

    /// Kayıtlı konektörlerin (ad, seçili) listesi (UI kaynak seçimi için).
    pub fn kaynaklar(&self) -> Vec<(&str, bool)> {
        self.konektorler
            .iter()
            .zip(&self.secili)
            .map(|(k, s)| (k.kaynak_adi(), *s))
            .collect()
    }

    /// Bir kaynağın seçimini değiştirir (indeks sınır dışıysa yok sayar).
    pub fn secimi_degistir(&mut self, indeks: usize) {
        if let Some(s) = self.secili.get_mut(indeks) {
            *s = !*s;
        }
    }

    /// En az bir kaynak seçili mi?
    pub fn secili_kaynak_var_mi(&self) -> bool {
        self.secili.iter().any(|s| *s)
    }

    /// **Birleşik arama**: seçili her konektörü çalıştırır, sonuçları kaynak rozetiyle birleştirir.
    /// Bir kaynak hata verse de diğerleri çalışır; hatalar [`son_hatalar`](Self::son_hatalar)'da
    /// toplanır.  Hiç kaynak seçili değilse `Err`.
    pub fn ara(&mut self, sayfalama: Sayfalama, baglam: &AramaBaglami) -> Result<(), ErrorReport> {
        if !self.secili_kaynak_var_mi() {
            return Err(ErrorReport::new(
                "Kaynak seçilmedi",
                "birleşik aramada en az bir veritabanı kaynağı seçili olmalı",
                "Aranacak kaynaklardan en az birini işaretleyin",
            ));
        }

        self.sonuclar.clear();
        self.son_hatalar.clear();
        let sorgu = Sorgu::metin(self.sorgu_metni.clone());

        for (konektor, secili) in self.konektorler.iter().zip(&self.secili) {
            if !*secili {
                continue;
            }
            match konektor.ara(&sorgu, sayfalama, baglam) {
                Ok(liste) => self.sonuclar.extend(liste.sonuclar),
                Err(e) => self
                    .son_hatalar
                    .push((konektor.kaynak_adi().to_string(), e.ne_oldu.clone())),
            }
        }
        Ok(())
    }

    /// Birleşik sonuçlar (kaynak rozetli).
    pub fn sonuclar(&self) -> &[AramaSonucu] {
        &self.sonuclar
    }

    /// Son aramada kaynak-başına oluşan hatalar (ad, "ne oldu").
    pub fn son_hatalar(&self) -> &[(String, String)] {
        &self.son_hatalar
    }

    /// Bir sonuç satırı için olası eylemler (tarayıcıda aç / yapıya bak / projeye ekle).
    pub fn eylemler(&self, sonuc: &AramaSonucu) -> Vec<Eylem> {
        let mut eylemler = Vec::new();
        if let Some(konektor) = self.konektor_bul(&sonuc.kaynak) {
            if let Some(url) = konektor.tarayici_url(&sonuc.kimlik) {
                eylemler.push(Eylem::TarayicidaAc(url));
            }
        }
        if sonuc.tur.yapiya_bakilabilir_mi() {
            eylemler.push(Eylem::YapiyaBak(sonuc.kimlik.clone()));
        }
        eylemler.push(Eylem::ProjeyeEkle(sonuc.kimlik.clone()));
        eylemler
    }

    /// Bir sonucu projeye **getirir** (kaydı indir + provenance) — ilgili konektöre yönlendirir.
    pub fn projeye_ekle(
        &self,
        sonuc: &AramaSonucu,
        baglam: &AramaBaglami,
    ) -> Result<GetirilenKayit, ErrorReport> {
        let konektor = self.konektor_bul(&sonuc.kaynak).ok_or_else(|| {
            ErrorReport::new(
                "Kaynak konektörü bulunamadı",
                format!("'{}' sonucu için kayıtlı bir konektör yok", sonuc.kaynak),
                "Sonucu yeniden arayın",
            )
        })?;
        konektor.getir(&sonuc.kimlik, baglam)
    }

    fn konektor_bul(&self, kaynak: &str) -> Option<&dyn VeritabaniKonektoru> {
        self.konektorler
            .iter()
            .find(|k| k.kaynak_adi() == kaynak)
            .map(|b| b.as_ref())
    }
}

impl Default for BirlesikPanel {
    fn default() -> Self {
        Self::yeni()
    }
}

/// Bir sonuç türünün tek-tık hedefini özetleyen yardımcı (UI ipucu).
pub fn tek_tik_ipucu(tur: KayitTuru) -> &'static str {
    match tur {
        KayitTuru::Yapi => "3B görüntüleyicide aç (ÇE-07)",
        KayitTuru::Nukleotid | KayitTuru::Protein => "diziyi projeye ekle",
        KayitTuru::Gen => "tarayıcıda gen kaydını aç",
        KayitTuru::Hizalama => "konuyu tarayıcıda aç",
        KayitTuru::Diger => "projeye ekle",
    }
}

#[cfg(test)]
mod tests {
    use super::super::connectors::{BlastKonektor, BlastProgram, NcbiKonektor, YoklamaAyari};
    use super::super::privacy::GizlilikKapisi;
    use super::super::transport::SahteUlastirici;
    use super::*;
    use std::time::Duration;

    fn esearch_json() -> &'static str {
        r#"{"esearchresult":{"count":"1","idlist":["7157"]}}"#
    }
    fn esummary_json() -> &'static str {
        r#"{"result":{"uids":["7157"],
            "7157":{"uid":"7157","caption":"NM_000546","title":"Homo sapiens TP53 mRNA","slen":2591,"organism":"Homo sapiens"}}}"#
    }
    fn put_yanit() -> &'static str {
        "QBlastInfoBegin\nRID = R1\nRTOE = 5\nQBlastInfoEnd"
    }
    fn hazir_yanit() -> &'static str {
        "QBlastInfoBegin\nStatus=READY\nQBlastInfoEnd"
    }
    fn tabular_yanit() -> &'static str {
        "q\tNM_000546.6\t99.5\t200\t1\t0\t1\t200\t1\t200\t1e-100\t370"
    }

    fn baglam_kur<'a>(u: &'a SahteUlastirici, g: &'a GizlilikKapisi) -> AramaBaglami<'a> {
        let mut b = AramaBaglami::yeni(u, g);
        b.yapi.geri_cekilme_taban = Duration::ZERO;
        b
    }

    #[test]
    fn birlesik_panel_kaynak_rozetli_sonuc() {
        let u = SahteUlastirici::yeni()
            .ekle("esearch.fcgi", 200, esearch_json())
            .ekle("esummary.fcgi", 200, esummary_json())
            .ekle("CMD=Put", 200, put_yanit())
            .ekle("FORMAT_OBJECT=SearchInfo", 200, hazir_yanit())
            .ekle("FORMAT_TYPE=Tabular", 200, tabular_yanit());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);

        let mut panel = BirlesikPanel::yeni();
        panel.konektor_ekle(Box::new(NcbiKonektor::nukleotid()));
        panel.konektor_ekle(Box::new(
            BlastKonektor::varsayilan(BlastProgram::Blastn).with_yoklama(YoklamaAyari {
                azami_yoklama: 3,
                aralik: Duration::ZERO,
            }),
        ));
        panel.sorgu_metni = "TP53".to_string();

        panel.ara(Sayfalama::ilk(20), &baglam).unwrap();
        // İki kaynaktan birleşik sonuç (1 NCBI + 1 BLAST).
        assert_eq!(panel.sonuclar().len(), 2);
        let rozetler: Vec<&str> = panel.sonuclar().iter().map(|s| s.kaynak.as_str()).collect();
        assert!(rozetler.contains(&"NCBI nucleotide"));
        assert!(rozetler.contains(&"BLAST blastn"));
        assert!(panel.son_hatalar().is_empty());
    }

    #[test]
    fn tarayicida_ac_eylemi_url_uretir() {
        let mut panel = BirlesikPanel::yeni();
        panel.konektor_ekle(Box::new(NcbiKonektor::nukleotid()));
        let sonuc = AramaSonucu::yeni("NCBI nucleotide", "7157", "TP53", KayitTuru::Nukleotid);
        let eylemler = panel.eylemler(&sonuc);
        assert!(eylemler
            .iter()
            .any(|e| matches!(e, Eylem::TarayicidaAc(u) if u.contains("nuccore/7157"))));
        assert!(eylemler.iter().any(|e| matches!(e, Eylem::ProjeyeEkle(_))));
        // Nükleotid → "yapıya bak" YOK.
        assert!(!eylemler.iter().any(|e| matches!(e, Eylem::YapiyaBak(_))));
    }

    #[test]
    fn yapi_turu_yapiya_bak_eylemi_verir() {
        let panel = BirlesikPanel::yeni();
        let sonuc = AramaSonucu::yeni("PDB", "1TUP", "p53 core domain", KayitTuru::Yapi);
        let eylemler = panel.eylemler(&sonuc);
        assert!(eylemler
            .iter()
            .any(|e| matches!(e, Eylem::YapiyaBak(k) if k == "1TUP")));
    }

    #[test]
    fn bir_kaynak_dusse_digeri_calisir() {
        // NCBI yanıtı kayıtlı; BLAST için yanıt YOK → BLAST hata, NCBI sonucu gelir.
        let u = SahteUlastirici::yeni()
            .ekle("esearch.fcgi", 200, esearch_json())
            .ekle("esummary.fcgi", 200, esummary_json());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);

        let mut panel = BirlesikPanel::yeni();
        panel.konektor_ekle(Box::new(NcbiKonektor::nukleotid()));
        panel.konektor_ekle(Box::new(
            BlastKonektor::varsayilan(BlastProgram::Blastn).with_yoklama(YoklamaAyari {
                azami_yoklama: 2,
                aralik: Duration::ZERO,
            }),
        ));
        panel.sorgu_metni = "TP53".into();
        panel.ara(Sayfalama::ilk(20), &baglam).unwrap();

        assert_eq!(panel.sonuclar().len(), 1); // yalnız NCBI
        assert_eq!(panel.son_hatalar().len(), 1); // BLAST düştü
        assert_eq!(panel.son_hatalar()[0].0, "BLAST blastn");
    }

    #[test]
    fn kaynak_secimi_kapatilinca_atlanir() {
        let u = SahteUlastirici::yeni()
            .ekle("esearch.fcgi", 200, esearch_json())
            .ekle("esummary.fcgi", 200, esummary_json());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);

        let mut panel = BirlesikPanel::yeni();
        panel.konektor_ekle(Box::new(NcbiKonektor::nukleotid()));
        panel.secimi_degistir(0); // kapat
        assert!(!panel.secili_kaynak_var_mi());
        assert!(panel.ara(Sayfalama::ilk(20), &baglam).is_err());
    }

    #[test]
    fn projeye_ekle_konektore_yonlendirir() {
        let fasta = ">NM_000546\nACGT\n";
        let u = SahteUlastirici::yeni().ekle("efetch.fcgi", 200, fasta);
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);

        let mut panel = BirlesikPanel::yeni();
        panel.konektor_ekle(Box::new(NcbiKonektor::nukleotid()));
        let sonuc = AramaSonucu::yeni("NCBI nucleotide", "7157", "TP53", KayitTuru::Nukleotid);
        let kayit = panel.projeye_ekle(&sonuc, &baglam).unwrap();
        assert_eq!(kayit.icerik, fasta.as_bytes());
    }
}
