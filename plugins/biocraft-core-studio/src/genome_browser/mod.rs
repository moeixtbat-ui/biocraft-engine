//! ÇE-02 — **Genom Tarayıcı (Genome Browser)** — 1. kısım (Gün 36).
//!
//! IGV/JBrowse/UCSC seviyesinde, **akıcı** (60 FPS hedefi — MK-04) ve **out-of-core** (yalnız
//! görünen pencere — MK-09) çok-izli genom tarayıcı tuvalinin **saf-mantık çekirdeği**.
//!
//! Eklenti motorun render katmanına (wgpu/egui) doğrudan bağlanamadığından (MK-17), tarayıcı
//! çizilecekleri **render-bağımsız** bir çizim listesine ([`cizim::CizimListesi`]) derler; motor
//! (biocraft-ui/-render) bu listeyi GPU ile çizer.  Bu sayede tüm koordinat/cetvel/yerleşim/
//! gezinme/LOD mantığı dosyadan ve egui'den bağımsız **birim-testlenir**.
//!
//! ## Alt-modüller
//! * [`canvas`] — koordinat sistemi + görünüm dönüşümü (genom ↔ ekran; tek doğruluk kaynağı).
//! * [`ruler`] — koordinat cetveli (bp/kb/Mb ölçek + yuvarlak işaretler).
//! * [`tracks`] — iz modeli + dikey yerleşim (yeniden sırala / aç-kapa / yükseklik).
//! * [`navigate`] — bölge ayrıştırma (`chr:start-end`/gen adı) + pan/zoom + gezinme geçmişi.
//! * [`lod`] — LOD + culling + downsampling + kapsama binleme + read yığını (MK-04/MK-09).
//! * [`veri`] — `data_io` okuyucu kayıtlarını çizim-parçalarına çevirir (out-of-core yükleme).
//! * [`cizim`] — çizim listesi (display list) + kompozisyon + isabet testi (tooltip/seçim).
//!
//! ## Bugün (1. kısım)
//! Tuval + cetvel + pan/zoom/"bölgeye git"/geri-ileri + hizalama/kapsama/anotasyon izleri +
//! tooltip/seçim.  **Yarın (Gün 37):** referans dizi izi, ileri LOD, çoklu örnek senkron, ölçüm.

use std::collections::{BTreeMap, HashMap};

use biocraft_sdk::biocraft_types::{Capability, ErrorReport};
use biocraft_sdk::{Aktivasyon, YetkiKapisi};

pub mod canvas;
pub mod cizim;
pub mod lod;
pub mod navigate;
pub mod ruler;
pub mod tracks;
pub mod veri;

pub use canvas::{GenomBolge, Tuval};
pub use cizim::{CizimListesi, CizimRengi, IsabetBolgesi, MetinHiza, Primitif};
pub use lod::{LodSeviyesi, VARSAYILAN_OGE_BUTCESI};
pub use navigate::{GenAdiCozucu, GezinmeGecmisi, TabloCozucu, ASGARI_PENCERE_BP};
pub use ruler::{Cetvel, CetvelIsareti, Olcek};
pub use tracks::{Iz, IzListesi, IzTuru, IzYer};
pub use veri::{OkumaParcasi, OzellikParcasi, Serit};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-02";

/// Bir izin bir karede çizilecek verisi (heterojen iz türleri için).
#[derive(Debug, Clone, PartialEq)]
pub enum IzVeri {
    /// Hizalama (read yığını).
    Hizalama(Vec<OkumaParcasi>),
    /// Kapsama (coverage) — okumalardan histogram binlenir.
    Kapsama(Vec<OkumaParcasi>),
    /// Anotasyon (gen/ekson özellikleri).
    Anotasyon(Vec<OzellikParcasi>),
    /// Veri yüklenmemiş/boş iz.
    Bos,
}

/// Seçili öğe (içerik/inspector paneli için).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeciliOge {
    /// Hangi iz?
    pub iz_kimlik: String,
    /// Kısa ipucu (tooltip ile aynı; yeniden vurgulama anahtarı).
    pub ipucu: String,
    /// Çok-satırlı detay (inspector gövdesi).
    pub detay: String,
}

/// Genom tarayıcı durumu — görünüm penceresi + izler + gezinme geçmişi + seçim.
#[derive(Debug, Clone)]
pub struct GenomTarayici {
    tuval: Tuval,
    izler: IzListesi,
    gecmis: GezinmeGecmisi,
    gen_cozucu: TabloCozucu,
    kromozom_uzunluklari: HashMap<String, u64>,
    secili: Option<SeciliOge>,
    /// Cetvel şeridinin yüksekliği (piksel).
    pub cetvel_yuksekligi: f32,
    /// İzler arası dikey boşluk (piksel).
    pub izler_arasi: f32,
    /// Cetvelde hedeflenen yaklaşık büyük işaret sayısı.
    pub hedef_isaret: u32,
}

impl GenomTarayici {
    /// Bir genişlik (piksel) ve başlangıç bölgesiyle tarayıcı kurar.
    pub fn yeni(genislik_px: f32, bolge: GenomBolge) -> Self {
        Self {
            tuval: Tuval::yeni(genislik_px, bolge.clone()),
            izler: IzListesi::yeni(),
            gecmis: GezinmeGecmisi::yeni(bolge),
            gen_cozucu: TabloCozucu::yeni(),
            kromozom_uzunluklari: HashMap::new(),
            secili: None,
            cetvel_yuksekligi: 24.0,
            izler_arasi: 4.0,
            hedef_isaret: 10,
        }
    }

    // ── Görünüm ───────────────────────────────────────────────────────────────

    /// Şu anki görünen bölge.
    pub fn bolge(&self) -> &GenomBolge {
        &self.tuval.bolge
    }

    /// Görünüm penceresi (koordinat dönüşümü).
    pub fn tuval(&self) -> &Tuval {
        &self.tuval
    }

    /// Tuval genişliğini günceller (pencere yeniden boyutlanınca; oran/zoom korunur).
    pub fn genislik_ayarla(&mut self, genislik_px: f32) {
        self.tuval.genislik_px = genislik_px.max(1.0);
    }

    /// Şu anki kromozomun bilinen uzunluğu (varsa).
    pub fn kromozom_uzunlugu(&self) -> Option<u64> {
        self.kromozom_uzunluklari
            .get(&self.tuval.bolge.kromozom)
            .copied()
    }

    /// Kromozom uzunluklarını ayarlar (hizalama başlığından: `(ad, uzunluk)`).  Sınırlama
    /// (`sinirla`) bunları kullanır.
    pub fn kromozom_uzunluklari_ayarla(
        &mut self,
        diziler: impl IntoIterator<Item = (String, u64)>,
    ) {
        self.kromozom_uzunluklari = diziler.into_iter().collect();
    }

    /// Gen adı çözücüsüne erişim (anotasyon yüklenince doldurmak için).
    pub fn gen_cozucu_mut(&mut self) -> &mut TabloCozucu {
        &mut self.gen_cozucu
    }

    // ── İzler ─────────────────────────────────────────────────────────────────

    /// İz listesi (salt-okur).
    pub fn izler(&self) -> &IzListesi {
        &self.izler
    }

    /// Sona iz ekler (kimlik çakışırsa eklemez).
    pub fn iz_ekle(&mut self, iz: Iz) -> bool {
        self.izler.ekle(iz)
    }

    /// İz görünürlüğünü değiştirir.
    pub fn iz_gorunurluk(&mut self, kimlik: &str) -> bool {
        self.izler.gorunurluk_degistir(kimlik)
    }

    /// İz yüksekliğini ayarlar.
    pub fn iz_yukseklik(&mut self, kimlik: &str, yukseklik_px: f32) -> bool {
        self.izler.yukseklik_ayarla(kimlik, yukseklik_px)
    }

    /// İzi yeniden sıralar.
    pub fn iz_tasi(&mut self, kaynak: usize, hedef: usize) -> bool {
        self.izler.tasi(kaynak, hedef)
    }

    // ── Gezinme ───────────────────────────────────────────────────────────────

    /// Bir bölgeyi uygular (kromozom uzunluğuyla sınırlar); `gecmise` true ise geçmişe işler.
    fn uygula(&mut self, b: GenomBolge, gecmise: bool) {
        let snr = b.sinirla(self.kromozom_uzunluklari.get(&b.kromozom).copied());
        if gecmise {
            self.gecmis.git(snr.clone());
        }
        self.tuval.bolge = snr;
    }

    /// "Bölgeye git": `chr:start-end` / `chr:pos` koordinatı, **gen adı** veya çıplak **kromozom
    /// adı** (`chr2` → tüm kromozom).  Başarıda görünümü değiştirir ve **geçmişe** işler.
    pub fn bolgeye_git(&mut self, metin: &str) -> Result<(), ErrorReport> {
        let pencere = self.tuval.bolge.uzunluk();
        let b = match navigate::bolgeye_git(metin, Some(&self.gen_cozucu), pencere) {
            Ok(b) => b,
            Err(e) => {
                // Çıplak kromozom adı mı? (kromozom uzunluklarını yalnız bu katman bilir.)
                let ad = metin.trim();
                if let Some((krom, &uzun)) = self
                    .kromozom_uzunluklari
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(ad))
                {
                    GenomBolge::yeni(krom.clone(), 1, uzun).unwrap_or_else(|_| GenomBolge {
                        kromozom: krom.clone(),
                        baslangic: 1,
                        bitis: 1,
                    })
                } else {
                    return Err(e);
                }
            }
        };
        self.uygula(b, true);
        Ok(())
    }

    /// Görünümü `delta_bp` kadar kaydırır (pan).  Sürekli (sürükleme/ok) — geçmişe işlemez;
    /// ayrık jest bitince [`gecmise_kaydet`](Self::gecmise_kaydet) çağrılır.
    pub fn pan_bp(&mut self, delta_bp: i64) {
        let b = navigate::kaydir(&self.tuval.bolge, delta_bp, self.kromozom_uzunlugu());
        self.uygula(b, false);
    }

    /// Ekranda `dx` piksel sürüklemeye karşılık gelen pan (sağa sürükleme → sola kayma).
    pub fn pan_piksel(&mut self, dx: f32) {
        let bp = (dx as f64 * self.tuval.bp_per_piksel()).round() as i64;
        self.pan_bp(-bp);
    }

    /// `odak` (1-tabanlı) pozisyon sabit kalacak şekilde yakınlaştırır/uzaklaştırır.
    /// `faktor < 1` yakınlaş, `> 1` uzaklaş.  Geçmişe işlemez (ayrık jestte `gecmise_kaydet`).
    pub fn yakinlastir_odak(&mut self, faktor: f64, odak: u64) {
        let b = navigate::yakinlastir(&self.tuval.bolge, faktor, odak, self.kromozom_uzunlugu());
        self.uygula(b, false);
    }

    /// Görünüm merkezinde yakınlaştırır/uzaklaştırır (`+`/`-` düğmeleri); **geçmişe işler**
    /// (ayrık jest).
    pub fn yakinlastir_merkez(&mut self, faktor: f64) {
        let odak = self.tuval.bolge.merkez();
        let b = navigate::yakinlastir(&self.tuval.bolge, faktor, odak, self.kromozom_uzunlugu());
        self.uygula(b, true);
    }

    /// Şu anki görünümü geçmişe işler (sürükleme/tekerlek jesti bitince çağrılır → ayrık geçmiş).
    pub fn gecmise_kaydet(&mut self) {
        self.gecmis.git(self.tuval.bolge.clone());
    }

    /// Geçmişte bir adım geri (varsa görünümü geri yükler).
    pub fn geri(&mut self) -> bool {
        if let Some(b) = self.gecmis.geri() {
            self.tuval.bolge = b.clone();
            true
        } else {
            false
        }
    }

    /// Geçmişte bir adım ileri.
    pub fn ileri(&mut self) -> bool {
        if let Some(b) = self.gecmis.ileri() {
            self.tuval.bolge = b.clone();
            true
        } else {
            false
        }
    }

    /// Geri gidilebilir mi? (UI düğmesi aktifliği)
    pub fn geri_var_mi(&self) -> bool {
        self.gecmis.geri_var_mi()
    }

    /// İleri gidilebilir mi?
    pub fn ileri_var_mi(&self) -> bool {
        self.gecmis.ileri_var_mi()
    }

    // ── Seçim / tooltip ───────────────────────────────────────────────────────

    /// Bir ekran noktasındaki öğenin ipucusu (tooltip); yoksa `None`.  Çizim listesi o kareye ait.
    pub fn tooltip<'a>(&self, liste: &'a CizimListesi, x: f32, y: f32) -> Option<&'a str> {
        liste.isabet_bul(x, y).map(|i| i.ipucu.as_str())
    }

    /// Bir ekran noktasındaki öğeyi seçer (inspector için).  Bir şey seçildiyse `true`.
    pub fn sec(&mut self, liste: &CizimListesi, x: f32, y: f32) -> bool {
        if let Some(i) = liste.isabet_bul(x, y) {
            self.secili = Some(SeciliOge {
                iz_kimlik: i.iz_kimlik.clone(),
                ipucu: i.ipucu.clone(),
                detay: i.detay.clone(),
            });
            true
        } else {
            self.secili = None;
            false
        }
    }

    /// Seçimi temizler.
    pub fn secimi_temizle(&mut self) {
        self.secili = None;
    }

    /// Şu anki seçim (inspector gövdesi).
    pub fn secili(&self) -> Option<&SeciliOge> {
        self.secili.as_ref()
    }

    // ── Türetilmiş çıktı ──────────────────────────────────────────────────────

    /// Şu anki görünümün cetveli.
    pub fn cetvel(&self) -> Cetvel {
        ruler::cetvel(&self.tuval, self.hedef_isaret)
    }

    /// Görünür izlerin dikey yerleşimi.
    pub fn yerlesim(&self) -> Vec<IzYer> {
        tracks::dikey_yerlesim(&self.izler, self.cetvel_yuksekligi, self.izler_arasi)
    }

    /// Tüm tuval içeriğinin toplam yüksekliği (dikey kaydırma için).
    pub fn toplam_yukseklik(&self) -> f32 {
        tracks::toplam_yukseklik(&self.izler, self.cetvel_yuksekligi, self.izler_arasi)
    }

    /// Bu karenin öğe bütçesi (kare bütçesi MK-04 pratik karşılığı; tuval genişliğiyle ölçekli).
    pub fn oge_butcesi(&self) -> usize {
        lod::oge_butcesi(&self.tuval, VARSAYILAN_OGE_BUTCESI)
    }

    /// Bir kareyi **derler**: cetvel + görünür izler (culling/LOD/downsampling) + seçim vurgusu →
    /// render-bağımsız çizim listesi.  `veri` iz-kimliğinden o izin verisine eşler (görünen
    /// pencereye ait; out-of-core yükleme çağıran tarafça yapılır).
    pub fn derle(&self, veri: &BTreeMap<String, IzVeri>) -> CizimListesi {
        let mut l = CizimListesi::yeni();
        let c = self.cetvel();
        cizim::cetvel_ciz(&mut l, &c, self.tuval.genislik_px, self.cetvel_yuksekligi);

        let yerler = self.yerlesim();
        let butce = self.oge_butcesi();
        for yer in &yerler {
            match veri.get(&yer.kimlik) {
                Some(IzVeri::Hizalama(okumalar)) => {
                    cizim::hizalama_ciz(&mut l, yer, &self.tuval, okumalar, &yer.kimlik, butce)
                }
                Some(IzVeri::Kapsama(okumalar)) => {
                    cizim::kapsama_ciz(&mut l, yer, &self.tuval, okumalar)
                }
                Some(IzVeri::Anotasyon(ozellikler)) => {
                    cizim::anotasyon_ciz(&mut l, yer, &self.tuval, ozellikler, &yer.kimlik, butce)
                }
                Some(IzVeri::Bos) | None => { /* veri yok: boş iz rehberi UI tarafında */ }
            }
        }

        if let Some(s) = &self.secili {
            cizim::secim_vurgula(&mut l, &s.ipucu);
        }
        l
    }
}

/// Alt-modülün UI/komut kayıtları — genom tarayıcı **GPU** ile çizildiğinden yalnızca `gpu`
/// yetkisi verildiyse komutlar sunulur (en az yetki + dürüstlük: `db_search`/`data_io` deseni).
pub fn kayitlar(yetkiler: &YetkiKapisi) -> Aktivasyon {
    let mut akt = Aktivasyon::yeni();
    if yetkiler.var_mi(Capability::Gpu) {
        akt.komut(
            "biocraft.core.studio.browser.ac",
            "BioCraft Studio: Genom Tarayıcı",
        )
        .komut(
            "biocraft.core.studio.browser.git",
            "BioCraft Studio: Bölgeye Git… (chr:konum / gen adı)",
        );
    }
    akt
}

#[cfg(test)]
mod tests {
    use super::*;
    use biocraft_sdk::ui::UiUzantiTuru;

    fn tarayici() -> GenomTarayici {
        let mut t = GenomTarayici::yeni(1000.0, GenomBolge::yeni("chr1", 1, 1000).unwrap());
        t.kromozom_uzunluklari_ayarla([("chr1".to_string(), 5000u64)]);
        t.iz_ekle(Iz::yeni("kapsama", "Kapsama", IzTuru::Kapsama));
        t.iz_ekle(Iz::yeni("reads", "Okumalar", IzTuru::Hizalama));
        t.iz_ekle(Iz::yeni("genler", "Genler", IzTuru::Anotasyon));
        t
    }

    #[test]
    fn gpu_yoksa_komut_yok() {
        assert_eq!(kayitlar(&YetkiKapisi::bos()).ui_say(UiUzantiTuru::Komut), 0);
        let gpu = kayitlar(&YetkiKapisi::yeni([Capability::Gpu]));
        assert_eq!(gpu.ui_say(UiUzantiTuru::Komut), 2);
    }

    #[test]
    fn bolgeye_git_ve_gecmis() {
        let mut t = tarayici();
        assert!(t.bolgeye_git("chr1:2000-2999").is_ok());
        assert_eq!(t.bolge().baslangic, 2000);
        assert!(t.geri_var_mi());
        // Geri → başlangıç bölgesi.
        assert!(t.geri());
        assert_eq!(t.bolge().baslangic, 1);
        assert!(t.ileri());
        assert_eq!(t.bolge().baslangic, 2000);

        // Geçersiz giriş hata döndürür, görünüm değişmez.
        let onceki = t.bolge().clone();
        assert!(t.bolgeye_git("saçma giriş !!").is_err());
        assert_eq!(t.bolge(), &onceki);
    }

    #[test]
    fn gen_adiyla_git() {
        let mut t = tarayici();
        t.gen_cozucu_mut()
            .ekle("MYC", GenomBolge::yeni("chr1", 3000, 3500).unwrap());
        assert!(t.bolgeye_git("myc").is_ok());
        assert_eq!(t.bolge().baslangic, 3000);
    }

    #[test]
    fn ciplak_kromozom_adina_git() {
        // "chr1" (gen değil) → tüm kromozom [1, 5000].
        let mut t = tarayici();
        assert!(t.bolgeye_git("chr1").is_ok());
        assert_eq!((t.bolge().baslangic, t.bolge().bitis), (1, 5000));
        // Bilinmeyen ad → hata (sessizce kromozom sanılmaz).
        assert!(t.bolgeye_git("YOKGEN").is_err());
    }

    #[test]
    fn pan_ve_zoom_sinirli() {
        let mut t = tarayici(); // chr1:1-1000, krom uzunluğu 5000
        t.pan_bp(500);
        assert_eq!(t.bolge().baslangic, 501);
        // Sağ sınıra dayanır (5000).
        t.pan_bp(100_000);
        assert_eq!(t.bolge().bitis, 5000);
        assert_eq!(t.bolge().uzunluk(), 1000, "pan uzunluğu korur");

        // Yakınlaş (0.5) → 500 bp.
        let mut t2 = tarayici();
        t2.yakinlastir_merkez(0.5);
        assert_eq!(t2.bolge().uzunluk(), 500);
        // Uzaklaş kromozom uzunluğuyla sınırlı.
        t2.yakinlastir_merkez(100.0);
        assert_eq!(t2.bolge().uzunluk(), 5000);
    }

    #[test]
    fn derle_ve_secim() {
        let t = {
            let mut t = tarayici();
            t.bolgeye_git("chr1:1-1000").ok();
            t
        };
        let mut veri: BTreeMap<String, IzVeri> = BTreeMap::new();
        veri.insert(
            "reads".into(),
            IzVeri::Hizalama(vec![OkumaParcasi {
                ad: "read1".into(),
                bas: 100,
                bit: 200,
                serit: Serit::Ileri,
                mapq: Some(60),
            }]),
        );
        veri.insert(
            "kapsama".into(),
            IzVeri::Kapsama(vec![OkumaParcasi {
                ad: "read1".into(),
                bas: 100,
                bit: 200,
                serit: Serit::Ileri,
                mapq: Some(60),
            }]),
        );
        let liste = t.derle(&veri);
        assert!(liste.ilkel_sayisi() > 0);
        assert_eq!(liste.isabetler.len(), 1, "tek read → tek isabet");

        // Seçim: read kutusunun üstüne tıkla.
        let mut t = t;
        let yer = t
            .yerlesim()
            .into_iter()
            .find(|y| y.kimlik == "reads")
            .unwrap();
        let (sol, _) = t.tuval().aralik_ekran(100, 200);
        let secti = t.sec(&liste, sol + 1.0, yer.y_ust + 1.0);
        assert!(secti);
        assert!(t.secili().unwrap().detay.contains("read1"));

        // Tooltip aynı noktada.
        assert!(t.tooltip(&liste, sol + 1.0, yer.y_ust + 1.0).is_some());

        t.secimi_temizle();
        assert!(t.secili().is_none());
    }

    #[test]
    fn bos_veride_panik_yok() {
        let t = tarayici();
        let veri: BTreeMap<String, IzVeri> = BTreeMap::new();
        let liste = t.derle(&veri); // veri yok → yalnız cetvel
        assert!(liste.isabetler.is_empty());
        assert!(liste.ilkel_sayisi() > 0); // cetvel zemini/işaretleri
    }
}
