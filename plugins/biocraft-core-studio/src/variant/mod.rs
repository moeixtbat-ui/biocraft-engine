//! ÇE-04 — **Varyant (VCF) Görünümü ve Filtreleme**.
//!
//! Milyonlarca satırlık VCF/BCF için **out-of-core** tablo görünümü + güçlü filtreleme + çok-örnekli
//! genotip ızgarası + genom tarayıcıya bağlantı + filtreli dışa aktarma.  Tüm mantık **render-bağımsız
//! ve birim-testlenebilir** (MK-17: eklenti motorun render katmanına bağlanmaz; egui/wgpu çizimini
//! motor yapar, bu modül **veri + durum** üretir).
//!
//! ## Alt-modüller
//! * [`query`] — sorgu motoru + **`VaryantSorguMotoru` arayüzü** (saf-Rust bugün; **DuckDB** ileride
//!   aynı arayüzle takılır — MK-09) + out-of-core kaynak + istatistik.
//! * [`filter`] — yapılandırılmış filtre + ham sorgu ayrıştırıcı + **kayıtlı filtre setleri**.
//! * [`tablo`] — sütunlu tablo + sütun seç/gizle/sırala + **sanal kaydırma** + gruplama.
//! * [`genotype_grid`] — çok-örnekli genotip matrisi + zigosite + sanal liste.
//! * [`track`] — varyant izi (tür görsel ayrımı; filtreli alt-küme → genom tarayıcı varyant izi).
//! * [`detail`] — INFO/FORMAT detayı + **"varyanta git"** (genom tarayıcı) + rsID bağlantısı.
//! * [`disa_aktar`] — filtreli alt-küme → VCF/CSV + bcftools-eşdeğeri golden TSV.
//!
//! ## DuckDB kararı (Gün 38)
//! ARCHITECTURE.md DuckDB önerir; bu sürümde projenin "saf-Rust, ağır dış bağımlılık ekleme"
//! disiplini gereği DuckDB'nin C++ bağımlılığı **eklenmez** — büyük dosya `data_io` ile out-of-core
//! okunur, filtre/sıralama saf-Rust uygulanır.  Gerçek DuckDB motoru ileride [`query::VaryantSorguMotoru`]
//! arayüzünü uygulayarak **çağıranı değiştirmeden** takılabilir (dürüst sınır).

use biocraft_sdk::biocraft_types::{Capability, ErrorReport};
use biocraft_sdk::{Aktivasyon, YetkiKapisi};

pub mod detail;
pub mod disa_aktar;
pub mod filter;
pub mod genotype_grid;
pub mod query;
pub mod tablo;
pub mod track;

pub use detail::{detay, tarayici_hedefi, VaryantDetay, VARSAYILAN_BAGLAM_BP};
pub use filter::{ayristir, Filtre, FiltreSetleri, InfoKosul, Karsilastirma, KayitliFiltreSeti};
pub use genotype_grid::{zigosite_coz, GenotipHucre, GenotipIzgara, Zigosite};
pub use query::{
    BellekKaynak, DosyaKaynak, Istatistik, SafRustMotor, Siralama, SiralamaAnahtari, Sorgu,
    SorguSonuc, VaryantKaynak, VaryantSatiri, VaryantSorguMotoru, VARSAYILAN_AZAMI_TARA,
};
pub use tablo::{gorunur_pencere, GrupBolum, Gruplama, Sutun, SutunDurum, TabloDuzeni};
pub use track::{efsane, iz_parcalari, tur_rengi, VaryantIzRengi, VaryantParcasi, VaryantTuru};

use crate::genome_browser::canvas::GenomBolge;

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-04";

/// **Varyant inceleme görünümü** — durum makinesi (filtre + sıralama + sonuç + seçim + kayıtlı
/// setler).  Bir [`VaryantSorguMotoru`] ile beslenir; render-bağımsızdır (egui/wgpu motor tarafında).
pub struct VaryantGorunumu {
    /// Tablo sütun düzeni (seç/gizle/sırala).
    pub duzen: TabloDuzeni,
    /// Aktif filtre.
    pub filtre: Filtre,
    /// Aktif sıralama.
    pub siralama: Siralama,
    /// Kaydedilmiş filtre setleri.
    pub setler: FiltreSetleri,
    /// Tablo gruplama ölçütü.
    pub gruplama: Gruplama,
    /// Örnek (sample) adları.
    ornekler: Vec<String>,
    /// Son sorgu sonucu (maddileştirilmiş — sanal kaydırma bunu kullanır).
    sonuc: Option<SorguSonuc>,
    /// Son istatistik.
    istatistik: Option<Istatistik>,
    /// Seçili satır indeksi (sonuç içinde).
    secili: Option<usize>,
}

impl VaryantGorunumu {
    /// Örnek adlarıyla yeni görünüm kurar (varsayılan sütun düzeni + boş filtre).
    pub fn yeni(ornekler: Vec<String>) -> Self {
        let duzen = TabloDuzeni::varsayilan(ornekler.len());
        Self {
            duzen,
            filtre: Filtre::default(),
            siralama: Siralama::default(),
            setler: FiltreSetleri::yeni(),
            gruplama: Gruplama::Yok,
            ornekler,
            sonuc: None,
            istatistik: None,
            secili: None,
        }
    }

    /// Aktif filtre + sıralamayı motorla çalıştırıp sonucu ve istatistiği günceller.
    pub fn yenile<M: VaryantSorguMotoru>(&mut self, motor: &mut M) -> Result<(), ErrorReport> {
        let sorgu = Sorgu {
            filtre: self.filtre.clone(),
            siralama: self.siralama,
        };
        let sonuc = motor.calistir(&sorgu)?;
        let ist = motor.istatistik(&self.filtre)?;
        // Seçimi geçerli aralıkta tut.
        if let Some(i) = self.secili {
            if i >= sonuc.satirlar.len() {
                self.secili = None;
            }
        }
        self.sonuc = Some(sonuc);
        self.istatistik = Some(ist);
        Ok(())
    }

    /// Filtreyi değiştirip yeniler.
    pub fn filtre_uygula<M: VaryantSorguMotoru>(
        &mut self,
        filtre: Filtre,
        motor: &mut M,
    ) -> Result<(), ErrorReport> {
        self.filtre = filtre;
        self.secili = None;
        self.yenile(motor)
    }

    /// Ham sorgu ifadesini ayrıştırıp filtre olarak uygular + yeniler.
    pub fn ham_sorgu_uygula<M: VaryantSorguMotoru>(
        &mut self,
        ifade: &str,
        motor: &mut M,
    ) -> Result<(), ErrorReport> {
        let filtre = ayristir(ifade)?;
        self.filtre_uygula(filtre, motor)
    }

    /// Sıralamayı değiştirip yeniler.
    pub fn sirala<M: VaryantSorguMotoru>(
        &mut self,
        anahtar: SiralamaAnahtari,
        artan: bool,
        motor: &mut M,
    ) -> Result<(), ErrorReport> {
        self.siralama = Siralama { anahtar, artan };
        self.yenile(motor)
    }

    /// Son sorgu sonucu.
    pub fn sonuc(&self) -> Option<&SorguSonuc> {
        self.sonuc.as_ref()
    }

    /// Son istatistik.
    pub fn istatistik(&self) -> Option<&Istatistik> {
        self.istatistik.as_ref()
    }

    /// Örnek adları.
    pub fn ornekler(&self) -> &[String] {
        &self.ornekler
    }

    /// Eşleşen toplam satır (sanal kaydırma için).
    pub fn toplam(&self) -> usize {
        self.sonuc.as_ref().map_or(0, |s| s.toplam_eslesme)
    }

    /// **Sanal kaydırma** penceresi: `[ilk, ilk+adet)` görünür satırları döndürür.
    pub fn gorunur_satirlar(&self, ilk: usize, adet: usize) -> &[VaryantSatiri] {
        match &self.sonuc {
            Some(s) => {
                let aralik = gorunur_pencere(s.satirlar.len(), ilk, adet);
                &s.satirlar[aralik]
            }
            None => &[],
        }
    }

    /// `idx` satırını döndürür (sonuç içinde).
    pub fn satir(&self, idx: usize) -> Option<&VaryantSatiri> {
        self.sonuc.as_ref()?.satirlar.get(idx)
    }

    /// Bir satırı seçer (sınır dışıysa seçim kalkar).
    pub fn sec(&mut self, idx: usize) {
        self.secili = match &self.sonuc {
            Some(s) if idx < s.satirlar.len() => Some(idx),
            _ => None,
        };
    }

    /// Seçili satır indeksi.
    pub fn secili_indeks(&self) -> Option<usize> {
        self.secili
    }

    /// Seçili satır.
    pub fn secili_satir(&self) -> Option<&VaryantSatiri> {
        self.satir(self.secili?)
    }

    /// Seçili varyantın **genom tarayıcı hedefi** ("varyanta git").
    pub fn tarayiciya_git(&self) -> Option<GenomBolge> {
        Some(tarayici_hedefi(self.secili_satir()?, VARSAYILAN_BAGLAM_BP))
    }

    /// Seçili varyantın detay paneli (INFO/FORMAT).
    pub fn secili_detay(&self) -> Option<VaryantDetay> {
        Some(detay(self.secili_satir()?, &self.ornekler))
    }

    /// Filtreli alt-kümeyi genom tarayıcı varyant izine besler.
    pub fn iz_parcalari(&self) -> Vec<VaryantParcasi> {
        match &self.sonuc {
            Some(s) => iz_parcalari(&s.satirlar),
            None => Vec::new(),
        }
    }

    /// Çok-örnekli genotip ızgarası (sonuç + örnek adları üzerinde; sanal liste).
    pub fn genotip_izgara(&self) -> Option<GenotipIzgara<'_>> {
        let s = self.sonuc.as_ref()?;
        Some(GenotipIzgara::yeni(&self.ornekler, &s.satirlar))
    }

    /// Sonucu gruplara böler (aktif gruplama ölçütüne göre).
    pub fn gruplar(&self) -> Vec<GrupBolum> {
        match &self.sonuc {
            Some(s) => tablo::gruplandir(&s.satirlar, self.gruplama),
            None => Vec::new(),
        }
    }

    /// Aktif filtreyi adla kaydeder.
    pub fn filtre_kaydet(&mut self, ad: impl Into<String>) {
        self.setler.kaydet(ad, &self.filtre);
    }

    /// Kaydedilmiş bir filtreyi yükleyip uygular.
    pub fn filtre_yukle<M: VaryantSorguMotoru>(
        &mut self,
        ad: &str,
        motor: &mut M,
    ) -> Result<(), ErrorReport> {
        let filtre = self.setler.getir(ad).ok_or_else(|| {
            ErrorReport::new(
                "Filtre seti yok",
                format!("'{ad}' adlı kayıtlı filtre bulunamadı"),
                "Kayıtlı setler listesinden geçerli bir ad seçin",
            )
        })??;
        self.filtre_uygula(filtre, motor)
    }

    /// Filtreli sonucu CSV olarak dışa aktarır (görünür sütunlarla).
    pub fn csv_disa_aktar(&self) -> Option<String> {
        let s = self.sonuc.as_ref()?;
        Some(disa_aktar::csv_olustur(
            &s.satirlar,
            &self.duzen,
            &self.ornekler,
        ))
    }

    /// Filtreli sonucu VCF olarak dışa aktarır.
    pub fn vcf_disa_aktar(&self) -> Option<String> {
        let s = self.sonuc.as_ref()?;
        Some(disa_aktar::vcf_olustur(&s.satirlar, &self.ornekler))
    }
}

/// Alt-modülün UI/komut kayıtları — varyant inceleme + dışa aktarma dosya gerektirir → yalnız `fs`
/// yetkisi verildiyse sunulur (en az yetki + dürüstlük; `data_io`/`db_search` deseni).
pub fn kayitlar(yetkiler: &YetkiKapisi) -> Aktivasyon {
    let mut akt = Aktivasyon::yeni();
    if yetkiler.var_mi(Capability::Fs) {
        akt.komut(
            "biocraft.core.studio.variant.incele",
            "BioCraft Studio: Varyant İncele (VCF Tablo + Filtre)",
        )
        .komut(
            "biocraft.core.studio.variant.disa_aktar",
            "BioCraft Studio: Filtreli Varyantları Dışa Aktar (VCF/CSV)",
        );
    }
    akt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_io::VaryantKaydi;
    use biocraft_sdk::ui::UiUzantiTuru;

    fn kayit(pos: usize, id: &str, r: &str, a: &str, q: f32, filt: &str, gt: &str) -> VaryantKaydi {
        VaryantKaydi {
            kromozom: "chr1".into(),
            konum: pos,
            kimlik: id.into(),
            referans: r.into(),
            alternatifler: a.split(',').map(|s| s.to_string()).collect(),
            kalite: Some(q),
            filtreler: vec![filt.into()],
            info: vec![("DP".into(), "Integer(30)".into())],
            ornek_sayisi: 1,
            format_anahtarlari: vec!["GT".into()],
            genotipler: vec![gt.into()],
        }
    }

    fn motor() -> SafRustMotor<BellekKaynak> {
        let satirlar = vec![
            VaryantSatiri::yeni(kayit(100, "rs1", "A", "G", 50.0, "PASS", "0/1")),
            VaryantSatiri::yeni(kayit(200, "rs2", "A", "T", 20.0, "q10", "1/1")),
            VaryantSatiri::yeni(kayit(300, ".", "A", "ACGT", 60.0, "PASS", "0/0")),
        ];
        SafRustMotor::yeni(BellekKaynak::yeni(vec!["S1".into()], satirlar))
    }

    #[test]
    fn ucta_uca_filtre_secim_git_detay() {
        let mut gorunum = VaryantGorunumu::yeni(vec!["S1".into()]);
        let mut m = motor();

        // Filtresiz: 3 satır.
        gorunum.yenile(&mut m).unwrap();
        assert_eq!(gorunum.toplam(), 3);

        // PASS filtresi → 2 satır.
        gorunum.ham_sorgu_uygula("FILTER = PASS", &mut m).unwrap();
        assert_eq!(gorunum.toplam(), 2);

        // İlk satırı seç → "varyanta git" hedefi + detay.
        gorunum.sec(0);
        let hedef = gorunum.tarayiciya_git().unwrap();
        assert_eq!(hedef.kromozom, "chr1");
        assert!(hedef.kapsar(100)); // ilk PASS varyantı konum=100
        let d = gorunum.secili_detay().unwrap();
        assert!(d.satirlar.iter().any(|(k, v)| k == "GT[S1]" && v == "0/1"));
    }

    #[test]
    fn istatistik_ve_genotip_izgara() {
        let mut gorunum = VaryantGorunumu::yeni(vec!["S1".into()]);
        let mut m = motor();
        gorunum.yenile(&mut m).unwrap();
        let ist = gorunum.istatistik().unwrap();
        assert_eq!(ist.toplam, 3);
        assert_eq!(ist.snv, 2);
        assert_eq!(ist.insersiyon, 1);

        let izgara = gorunum.genotip_izgara().unwrap();
        assert_eq!(izgara.satir_sayisi(), 3);
        assert_eq!(izgara.hucre(0, 0).unwrap().zigosite, Zigosite::Het);
    }

    #[test]
    fn sanal_kaydirma_penceresi() {
        let mut gorunum = VaryantGorunumu::yeni(vec!["S1".into()]);
        let mut m = motor();
        gorunum.yenile(&mut m).unwrap();
        // Konum sıralı: 100,200,300. Pencere [1,3) → 200,300.
        let pencere = gorunum.gorunur_satirlar(1, 10);
        assert_eq!(pencere.len(), 2);
        assert_eq!(pencere[0].konum(), 200);
    }

    #[test]
    fn filtre_kaydet_yukle() {
        let mut gorunum = VaryantGorunumu::yeni(vec!["S1".into()]);
        let mut m = motor();
        gorunum.ham_sorgu_uygula("QUAL >= 40", &mut m).unwrap();
        assert_eq!(gorunum.toplam(), 2); // 50, 60
        gorunum.filtre_kaydet("yüksek");

        // Filtreyi sıfırla → 3.
        gorunum.filtre_uygula(Filtre::default(), &mut m).unwrap();
        assert_eq!(gorunum.toplam(), 3);

        // Kayıtlı seti yükle → tekrar 2.
        gorunum.filtre_yukle("yüksek", &mut m).unwrap();
        assert_eq!(gorunum.toplam(), 2);
    }

    #[test]
    fn disa_aktarma_csv_vcf() {
        let mut gorunum = VaryantGorunumu::yeni(vec!["S1".into()]);
        let mut m = motor();
        gorunum.ham_sorgu_uygula("FILTER = PASS", &mut m).unwrap();
        let csv = gorunum.csv_disa_aktar().unwrap();
        assert_eq!(csv.lines().count(), 3); // başlık + 2 satır
        let vcf = gorunum.vcf_disa_aktar().unwrap();
        assert!(vcf.contains("##fileformat=VCFv4.3"));
        assert_eq!(vcf.lines().filter(|l| !l.starts_with('#')).count(), 2);
    }

    #[test]
    fn fs_yoksa_komut_yok() {
        assert_eq!(kayitlar(&YetkiKapisi::bos()).ui_say(UiUzantiTuru::Komut), 0);
        let fsli = kayitlar(&YetkiKapisi::yeni([Capability::Fs]));
        assert_eq!(fsli.ui_say(UiUzantiTuru::Komut), 2);
    }
}
