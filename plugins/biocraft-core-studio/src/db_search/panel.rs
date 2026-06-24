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

use super::cache::AramaOnbellegi;
use super::framework::{
    AramaBaglami, AramaSonucu, GetirilenKayit, KayitTuru, Konum, Sayfalama, Sorgu,
    VeritabaniKonektoru,
};
use super::history::{AramaGecmisi, GecmisGirdisi};

/// Bir sonuç satırından yapılabilecek eylem (UI buton/menü).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Eylem {
    /// Kaynağın web sayfasını dış tarayıcıda aç (dış erişim — `net` + onay; host açar).
    TarayicidaAc(String),
    /// 3B yapı görüntüleyiciye yükle (ÇE-07) — yalnız [`KayitTuru::Yapi`].
    YapiyaBak(String),
    /// Genom tarayıcıda (ÇE-02) bu konuma git — konumlu sonuçlar (Ensembl/UCSC) için çapraz bağlantı.
    GenomdaGoster(Konum),
    /// Kaydı indir + projeye ekle (provenance ile).
    ProjeyeEkle(String),
}

/// Birleşik panel — kayıtlı konektörler + seçim + son sorgu + birleşik sonuçlar + önbellek/geçmiş.
pub struct BirlesikPanel {
    konektorler: Vec<Box<dyn VeritabaniKonektoru>>,
    secili: Vec<bool>,
    /// Arama kutusu metni.
    pub sorgu_metni: String,
    sonuclar: Vec<AramaSonucu>,
    son_hatalar: Vec<(String, String)>,
    /// Opsiyonel akıllı önbellek (hız + çevrimdışı; Gün 41).  Yoksa her arama ağa gider.
    onbellek: Option<AramaOnbellegi>,
    /// Arama geçmişi (sorgu/kaynak/tarih; tekrar çalıştır; favori) — Gün 41.
    gecmis: AramaGecmisi,
    /// Son aramada (en az bir kaynak) önbellekten/çevrimdışı mı sunuldu (UI rozeti)?
    onbellekten: bool,
}

impl BirlesikPanel {
    /// Boş panel (konektör eklenmemiş; önbelleksiz — her arama ağa gider).
    pub fn yeni() -> Self {
        Self {
            konektorler: Vec::new(),
            secili: Vec::new(),
            sorgu_metni: String::new(),
            sonuclar: Vec::new(),
            son_hatalar: Vec::new(),
            onbellek: None,
            gecmis: AramaGecmisi::default(),
            onbellekten: false,
        }
    }

    /// Akıllı önbellek bağlar (akıcı): aynı sorgu hızlı döner + çevrimdışı sunulur (Gün 41).
    pub fn with_onbellek(mut self, onbellek: AramaOnbellegi) -> Self {
        self.onbellek = Some(onbellek);
        self
    }

    /// Arama geçmişine erişim (UI listesi).
    pub fn gecmis(&self) -> &AramaGecmisi {
        &self.gecmis
    }

    /// Arama geçmişini değiştirmek için (favori_degistir / temizle / kaydet).
    pub fn gecmis_mut(&mut self) -> &mut AramaGecmisi {
        &mut self.gecmis
    }

    /// Son arama önbellekten/çevrimdışı mı sunuldu (UI "önbellekten" rozeti)?
    pub fn onbellekten(&self) -> bool {
        self.onbellekten
    }

    /// Bir geçmiş girdisini panele yükler (tekrar çalıştır): sorgu kutusunu doldurur, sorgu döner.
    pub fn gecmisten_yukle(&mut self, indeks: usize) -> Option<Sorgu> {
        let sorgu = self.gecmis.tekrar_calistir(indeks)?;
        self.sorgu_metni = sorgu.metin.clone();
        Some(sorgu)
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
    ///
    /// Önbellek bağlıysa (Gün 41) her kaynak için:
    /// 1. **Önbellek-öncelik:** Geçerlilik süresi içinde sonuç varsa ağ yerine ondan döner (hız).
    /// 2. **Ağ:** Yoksa konektör çalışır; başarı önbelleğe yazılır.
    /// 3. **Çevrimdışı fallback:** Ağ başarısızsa (kesinti/rate-limit) önbellekteki **bayat** sonuç
    ///    sunulur (yarı yolda bırakmaz); yoksa hata [`son_hatalar`](Self::son_hatalar)'a yazılır.
    ///
    /// Federe dayanıklılık: bir kaynak düşse de diğerleri çalışır.  Arama [`gecmis`](Self::gecmis)'e
    /// kaydedilir.  Hiç kaynak seçili değilse `Err`.
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
        self.onbellekten = false;
        let sorgu = Sorgu::metin(self.sorgu_metni.clone());
        let mut aranan_kaynaklar: Vec<String> = Vec::new();

        for (konektor, secili) in self.konektorler.iter().zip(&self.secili) {
            if !*secili {
                continue;
            }
            let kaynak = konektor.kaynak_adi().to_string();
            aranan_kaynaklar.push(kaynak.clone());
            let anahtar = AramaOnbellegi::sonuc_anahtari(&kaynak, &sorgu.metin, sayfalama);

            // 1) Önbellek-öncelik (geçerli/taze).
            if let Some(ob) = &self.onbellek {
                if let Some(liste) = ob.sonuc_oku(&anahtar) {
                    self.sonuclar.extend(liste.sonuclar);
                    self.onbellekten = true;
                    continue;
                }
            }

            // 2) Ağ.
            match konektor.ara(&sorgu, sayfalama, baglam) {
                Ok(liste) => {
                    if let Some(ob) = &self.onbellek {
                        let _ = ob.sonuc_yaz(&anahtar, &liste);
                    }
                    self.sonuclar.extend(liste.sonuclar);
                }
                Err(e) => {
                    // 3) Çevrimdışı fallback: bayat önbellek varsa onu sun.
                    if let Some(liste) = self
                        .onbellek
                        .as_ref()
                        .and_then(|ob| ob.sonuc_oku_zorla(&anahtar))
                    {
                        self.sonuclar.extend(liste.sonuclar);
                        self.onbellekten = true;
                        self.son_hatalar
                            .push((kaynak, "çevrimdışı — önbellekten sunuldu".to_string()));
                    } else {
                        self.son_hatalar.push((kaynak, e.ne_oldu.clone()));
                    }
                }
            }
        }

        // Aramayı geçmişe kaydet (boş sorgu da bir olaydır; UI listeler).
        self.gecmis.ekle(GecmisGirdisi::yeni(
            self.sorgu_metni.clone(),
            aranan_kaynaklar,
            self.sonuclar.len(),
        ));
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

    /// Bir sonuç satırı için olası eylemler (çapraz bağlantı dâhil):
    /// * **Tarayıcıda aç** — konektör bir web URL'i verirse.
    /// * **Yapıya bak** (ÇE-07) — [`KayitTuru::Yapi`] (PDB).
    /// * **Genom tarayıcıda göster** (ÇE-02) — sonucun bir [`Konum`]'u varsa (Ensembl gen, vb.).
    /// * **Projeye ekle** — doğrudan indirilebilir tür (iz hariç; iz bölge gerektirir → tarayıcı).
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
        if let Some(konum) = &sonuc.konum {
            eylemler.push(Eylem::GenomdaGoster(konum.clone()));
        }
        // İz (track) tek başına dosya değildir (bölge gerekir) → "projeye ekle" sunulmaz.
        if sonuc.tur != KayitTuru::Iz {
            eylemler.push(Eylem::ProjeyeEkle(sonuc.kimlik.clone()));
        }
        eylemler
    }

    /// Bir sonucu projeye **getirir** (kaydı indir + provenance) — ilgili konektöre yönlendirir.
    ///
    /// Önbellek bağlıysa indirilen ham veri yerelde tutulur (kalıcı): aynı kayıt tekrar istenince
    /// ağa gidilmez ve çevrimdışıyken de sunulur.  İçerik formatı/kimliği korunarak provenance
    /// önbellekten okunamadığında yeniden üretilir (köken her zaman kaynağı belgeler — MK-34).
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
        let anahtar = AramaOnbellegi::kayit_anahtari(&sonuc.kaynak, &sonuc.kimlik);

        // 1) Önbellekteki ham veri (hız + çevrimdışı) — köken/atıf önbellekte saklandığından korunur.
        if let Some((format, icerik, prov)) =
            self.onbellek.as_ref().and_then(|ob| ob.veri_oku(&anahtar))
        {
            let provenans = prov.unwrap_or_else(|| {
                super::provenance::db_provenansi(
                    format!("{}.{}", sonuc.kimlik, format),
                    format!("{} (önbellek)", sonuc.kaynak),
                    format.to_uppercase(),
                    &icerik,
                    None,
                )
            });
            return Ok(GetirilenKayit {
                kimlik: sonuc.kimlik.clone(),
                format_ipucu: format,
                icerik,
                provenans,
            });
        }

        // 2) Ağdan getir → ham veriyi + provenance'ı önbelleğe yaz.
        let kayit = konektor.getir(&sonuc.kimlik, baglam)?;
        if let Some(ob) = &self.onbellek {
            let _ = ob.veri_yaz(
                &anahtar,
                &kayit.format_ipucu,
                &kayit.icerik,
                Some(&kayit.provenans),
            );
        }
        Ok(kayit)
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
        KayitTuru::Iz => "genom tarayıcıda izi göster",
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

    #[test]
    fn konumlu_sonuc_genomda_goster_eylemi() {
        // Ensembl gibi konumlu bir sonuç → genom tarayıcı çapraz bağlantısı.
        let panel = BirlesikPanel::yeni();
        let sonuc = AramaSonucu::yeni("Ensembl", "ENSG00000141510", "TP53", KayitTuru::Gen)
            .with_konum(super::super::framework::Konum::yeni(
                "17", 7_661_779, 7_687_550,
            ));
        let eylemler = panel.eylemler(&sonuc);
        assert!(eylemler
            .iter()
            .any(|e| matches!(e, Eylem::GenomdaGoster(k) if k.kromozom == "17")));
        // Gen indirilebilir → projeye ekle de var.
        assert!(eylemler.iter().any(|e| matches!(e, Eylem::ProjeyeEkle(_))));
    }

    #[test]
    fn iz_sonucu_projeye_ekle_sunmaz() {
        // UCSC izi tek başına dosya değildir (bölge gerekir) → projeye ekle sunulmaz.
        let panel = BirlesikPanel::yeni();
        let sonuc = AramaSonucu::yeni("UCSC", "refGene", "RefSeq Genes", KayitTuru::Iz);
        let eylemler = panel.eylemler(&sonuc);
        assert!(!eylemler.iter().any(|e| matches!(e, Eylem::ProjeyeEkle(_))));
    }

    #[test]
    fn arama_gecmise_kaydedilir() {
        let u = SahteUlastirici::yeni()
            .ekle("esearch.fcgi", 200, esearch_json())
            .ekle("esummary.fcgi", 200, esummary_json());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let mut panel = BirlesikPanel::yeni();
        panel.konektor_ekle(Box::new(NcbiKonektor::nukleotid()));
        panel.sorgu_metni = "TP53".into();
        panel.ara(Sayfalama::ilk(20), &baglam).unwrap();
        assert_eq!(panel.gecmis().len(), 1);
        assert_eq!(panel.gecmis().girdiler()[0].sorgu, "TP53");
        // Tekrar çalıştır: sorgu kutusu dolar.
        panel.sorgu_metni.clear();
        let sorgu = panel.gecmisten_yukle(0).unwrap();
        assert_eq!(sorgu.metin, "TP53");
        assert_eq!(panel.sorgu_metni, "TP53");
    }

    #[test]
    fn onbellek_ikinci_aramayi_hizlandirir_ve_cevrimdisi_sunar() {
        let dizin = std::env::temp_dir().join(format!("biocraft_panel_ob_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dizin);

        // 1) İlk arama: ağ var → sonuç önbelleğe yazılır.
        {
            let u = SahteUlastirici::yeni()
                .ekle("esearch.fcgi", 200, esearch_json())
                .ekle("esummary.fcgi", 200, esummary_json());
            let g = GizlilikKapisi::onayli();
            let baglam = baglam_kur(&u, &g);
            let ob = super::super::cache::AramaOnbellegi::varsayilan(&dizin).unwrap();
            let mut panel = BirlesikPanel::yeni().with_onbellek(ob);
            panel.konektor_ekle(Box::new(NcbiKonektor::nukleotid()));
            panel.sorgu_metni = "TP53".into();
            panel.ara(Sayfalama::ilk(20), &baglam).unwrap();
            assert_eq!(panel.sonuclar().len(), 1);
            assert!(!panel.onbellekten()); // ağdan geldi
        }

        // 2) İkinci arama: ÇEVRİMDIŞI ulaştırıcı → yalnız önbellekten sunulur.
        {
            let u = SahteUlastirici::cevrimdisi();
            let g = GizlilikKapisi::onayli();
            let baglam = baglam_kur(&u, &g);
            let ob = super::super::cache::AramaOnbellegi::varsayilan(&dizin).unwrap();
            let mut panel = BirlesikPanel::yeni().with_onbellek(ob);
            panel.konektor_ekle(Box::new(NcbiKonektor::nukleotid()));
            panel.sorgu_metni = "TP53".into();
            panel.ara(Sayfalama::ilk(20), &baglam).unwrap();
            // Önbellek-öncelik (taze) → ağ hiç denenmez; sonuç gelir.
            assert_eq!(panel.sonuclar().len(), 1);
            assert!(panel.onbellekten());
        }
        let _ = std::fs::remove_dir_all(&dizin);
    }
}
