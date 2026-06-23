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
//! * [`reference`] — referans dizi izi (renkli bazlar) + kodon/aminoasit çevirisi (3 çerçeve).
//! * [`multisample`] — çoklu örnek (vaka/kontrol) senkron karşılaştırma (yan yana / üst üste).
//! * [`measure`] — ölçüm aracı (bp mesafe) + pozisyon kopyalama + yer imleri (bookmark).
//! * [`disa_aktar`] — görünümün anlık görüntüsünü SVG (yayın) / PNG (raster) olarak dışa aktarır.
//!
//! ## Kapsam (Gün 36 + Gün 37)
//! Tuval + cetvel + pan/zoom/"bölgeye git"/geri-ileri + hizalama/kapsama/anotasyon izleri +
//! tooltip/seçim + LOD (Gün 36); **referans dizi + çeviri, ileri LOD ("tam göster" + önemli
//! read koruma), çoklu örnek senkron, ölçüm/yer imi, varyant (mismatch/indel) vurgusu, PNG/SVG
//! dışa aktarma (Gün 37).**

use std::collections::{BTreeMap, HashMap};

use biocraft_sdk::biocraft_types::{Capability, ErrorReport};
use biocraft_sdk::{Aktivasyon, YetkiKapisi};

pub mod canvas;
pub mod cizim;
pub mod disa_aktar;
pub mod lod;
pub mod measure;
pub mod multisample;
pub mod navigate;
pub mod reference;
pub mod ruler;
pub mod tracks;
pub mod veri;

pub use canvas::{GenomBolge, Tuval};
pub use cizim::{CizimListesi, CizimRengi, IsabetBolgesi, MetinHiza, Primitif};
pub use disa_aktar::{png_olustur, svg_olustur, Palet};
pub use lod::{LodSeviyesi, VARSAYILAN_OGE_BUTCESI};
pub use measure::{bolge_metni, konum_metni, Olcum, Yerimi, YerimleriListesi};
pub use multisample::{
    katmanlar, ornek_izleri, ornek_rengi, KarsilastirmaModu, Ornek, OrnekKatman,
};
pub use navigate::{GenAdiCozucu, GezinmeGecmisi, TabloCozucu, ASGARI_PENCERE_BP};
pub use reference::{
    cevir, gorunur_referans_2bit, gorunur_referans_fasta, kodon_amino, referans_gerekli,
    ters_tumleyen, CeviriDurumu, Kodon, ReferansDizi,
};
pub use ruler::{Cetvel, CetvelIsareti, Olcek};
pub use tracks::{Iz, IzListesi, IzTuru, IzYer};
pub use veri::{OkumaParcasi, OzellikParcasi, Serit, VaryantParcasi, VaryantTuru};

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-02";

/// Bir izin bir karede çizilecek verisi (heterojen iz türleri için).
#[derive(Debug, Clone, PartialEq)]
pub enum IzVeri {
    /// Referans dizi (renkli bazlar + opsiyonel çeviri).
    Referans(ReferansDizi),
    /// Hizalama (read yığını).
    Hizalama(Vec<OkumaParcasi>),
    /// Kapsama (coverage) — okumalardan histogram binlenir.
    Kapsama(Vec<OkumaParcasi>),
    /// Çoklu örnek overlay kapsaması (vaka/kontrol; tek lane, farklı renkler).
    KapsamaCokOrnek(Vec<OrnekKatman>),
    /// Anotasyon (gen/ekson özellikleri).
    Anotasyon(Vec<OzellikParcasi>),
    /// Varyant (mismatch/insersiyon/delesyon işaretleri).
    Varyant(Vec<VaryantParcasi>),
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
    yerimleri: YerimleriListesi,
    olcum: Option<Olcum>,
    isaretli_bolge: Option<GenomBolge>,
    /// Cetvel şeridinin yüksekliği (piksel).
    pub cetvel_yuksekligi: f32,
    /// İzler arası dikey boşluk (piksel).
    pub izler_arasi: f32,
    /// Cetvelde hedeflenen yaklaşık büyük işaret sayısı.
    pub hedef_isaret: u32,
    /// Referans izinde kodon/aminoasit çevirisi görünüm durumu (varsayılan kapalı).
    pub ceviri: CeviriDurumu,
    /// "Tam göster": yoğun bölgede özet/seyreltmeyi atla (akıcılık pahasına hiçbir read gizlenmez).
    pub tam_goster: bool,
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
            yerimleri: YerimleriListesi::yeni(),
            olcum: None,
            isaretli_bolge: None,
            cetvel_yuksekligi: 24.0,
            izler_arasi: 4.0,
            hedef_isaret: 10,
            ceviri: CeviriDurumu::default(),
            tam_goster: false,
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

    // ── Ölçüm + pozisyon kopyalama ─────────────────────────────────────────────

    /// İki **ekran x'i** arasında ölçüm aracını kurar (kullanıcı iki noktaya tıklar).
    pub fn olcum_ekrandan(&mut self, x1: f32, x2: f32) {
        let a = self.tuval.ekran_to_genom(x1);
        let b = self.tuval.ekran_to_genom(x2);
        self.olcum = Some(Olcum::yeni(a, b));
    }

    /// İki **genom konumu** arasında ölçüm aracını kurar.
    pub fn olcum_ayarla(&mut self, a: u64, b: u64) {
        self.olcum = Some(Olcum::yeni(a, b));
    }

    /// Şu anki ölçüm (varsa).
    pub fn olcum(&self) -> Option<Olcum> {
        self.olcum
    }

    /// Ölçümü temizler.
    pub fn olcum_temizle(&mut self) {
        self.olcum = None;
    }

    /// Bir ekran x'indeki genom konumunu kopyalanabilir metne çevirir (ör. `chr1:12,345`).
    pub fn konum_kopyala(&self, x: f32) -> String {
        measure::konum_metni(&self.tuval)(x)
    }

    /// Şu anki görünen bölgeyi kopyalanabilir metne çevirir (ör. `chr1:1,000-2,000`).
    pub fn bolge_kopyala(&self) -> String {
        measure::bolge_metni(&self.tuval.bolge)
    }

    // ── Bölge işaretleme + yer imleri (bookmark; geri-alınabilir) ───────────────

    /// Bir bölgeyi görünümde işaretler (vurgulanan bant).
    pub fn bolge_isaretle(&mut self, bolge: GenomBolge) {
        self.isaretli_bolge = Some(bolge);
    }

    /// Şu anki tüm görünen bölgeyi işaretler.
    pub fn gorunumu_isaretle(&mut self) {
        self.isaretli_bolge = Some(self.tuval.bolge.clone());
    }

    /// İşaretli bölge (varsa).
    pub fn isaretli_bolge(&self) -> Option<&GenomBolge> {
        self.isaretli_bolge.as_ref()
    }

    /// Bölge işaretini temizler.
    pub fn isareti_temizle(&mut self) {
        self.isaretli_bolge = None;
    }

    /// Şu anki görünen bölgeyi bir **yer imi** olarak kaydeder; eklenen indeksi döner.
    pub fn yerimi_ekle(&mut self, ad: impl Into<String>) -> usize {
        self.yerimleri
            .ekle(Yerimi::yeni(ad, self.tuval.bolge.clone()))
    }

    /// Bir yer imini siler ve **geri döndürür** (geri-alma için UI tekrar ekleyebilir).
    pub fn yerimi_sil(&mut self, indeks: usize) -> Option<Yerimi> {
        self.yerimleri.sil(indeks)
    }

    /// Kayıtlı yer imleri (salt-okur).
    pub fn yerimleri(&self) -> &YerimleriListesi {
        &self.yerimleri
    }

    /// Bir yer imine gider (geçmişe işler); geçersiz indekste `false`.
    pub fn yerimine_git(&mut self, indeks: usize) -> bool {
        if let Some(y) = self.yerimleri.al(indeks) {
            let b = y.bolge.clone();
            self.uygula(b, true);
            true
        } else {
            false
        }
    }

    // ── Çoklu örnek (vaka/kontrol senkron karşılaştırma) ────────────────────────

    /// Çoklu örnek izlerini ekler (yan yana ya da üst üste).  Tüm izler **aynı tuval** üzerinden
    /// çizildiğinden gezinme otomatik senkrondur (tek koordinat modeli).
    pub fn ornekleri_ekle(&mut self, ornekler: &[Ornek], mod_: KarsilastirmaModu) {
        for iz in multisample::ornek_izleri(ornekler, mod_) {
            self.izler.ekle(iz);
        }
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

    /// Bir kareyi **derler**: işaretli bölge + cetvel + görünür izler (culling/LOD/downsampling) +
    /// seçim vurgusu + ölçüm → render-bağımsız çizim listesi.  `veri` iz-kimliğinden o izin
    /// verisine eşler (görünen pencereye ait; out-of-core yükleme çağıran tarafça yapılır).
    pub fn derle(&self, veri: &BTreeMap<String, IzVeri>) -> CizimListesi {
        let mut l = CizimListesi::yeni();

        // İşaretli bölge bandı en altta (izler üstüne çizilir).
        if let Some(b) = &self.isaretli_bolge {
            cizim::bolge_isaretle_ciz(&mut l, &self.tuval, b, self.toplam_yukseklik());
        }

        let c = self.cetvel();
        cizim::cetvel_ciz(&mut l, &c, self.tuval.genislik_px, self.cetvel_yuksekligi);

        // Önemli read koruması için varyant konumları (haritadaki tüm varyant izlerinden).
        let varyant_konumlari = self.varyant_konumlari(veri);

        let yerler = self.yerlesim();
        let butce = self.oge_butcesi();
        for yer in &yerler {
            match veri.get(&yer.kimlik) {
                Some(IzVeri::Referans(referans)) => {
                    let cerc = if self.ceviri.goster {
                        Some(self.ceviri.cerceveler(referans))
                    } else {
                        None
                    };
                    cizim::referans_ciz(
                        &mut l,
                        yer,
                        &self.tuval,
                        referans,
                        cerc.as_ref().map(|c| c.as_slice()),
                    );
                }
                Some(IzVeri::Hizalama(okumalar)) => {
                    let korunan = onemli_okumalar(okumalar, &varyant_konumlari);
                    cizim::hizalama_ciz(
                        &mut l,
                        yer,
                        &self.tuval,
                        okumalar,
                        &yer.kimlik,
                        cizim::HizalamaSecenek {
                            oge_butcesi: butce,
                            tam_goster: self.tam_goster,
                            korunan: &korunan,
                        },
                    );
                }
                Some(IzVeri::Kapsama(okumalar)) => {
                    cizim::kapsama_ciz(&mut l, yer, &self.tuval, okumalar)
                }
                Some(IzVeri::KapsamaCokOrnek(katmanlar)) => {
                    cizim::kapsama_overlay_ciz(&mut l, yer, &self.tuval, katmanlar)
                }
                Some(IzVeri::Anotasyon(ozellikler)) => {
                    cizim::anotasyon_ciz(&mut l, yer, &self.tuval, ozellikler, &yer.kimlik, butce)
                }
                Some(IzVeri::Varyant(varyantlar)) => {
                    cizim::varyant_ciz(&mut l, yer, &self.tuval, varyantlar, &yer.kimlik, butce)
                }
                Some(IzVeri::Bos) | None => { /* veri yok: boş iz rehberi UI tarafında */ }
            }
        }

        if let Some(s) = &self.secili {
            cizim::secim_vurgula(&mut l, &s.ipucu);
        }

        // Ölçüm aracı (varsa) cetvelin hemen altında.
        if let Some(o) = &self.olcum {
            cizim::olcum_ciz(&mut l, &self.tuval, o, self.cetvel_yuksekligi + 12.0);
        }
        l
    }

    /// Veri haritasındaki tüm varyant izlerinin görünen varyant konumlarını toplar (önemli read
    /// korumasında kullanılır).
    fn varyant_konumlari(&self, veri: &BTreeMap<String, IzVeri>) -> Vec<(u64, u64)> {
        let mut konumlar = Vec::new();
        for v in veri.values() {
            if let IzVeri::Varyant(varyantlar) = v {
                for vp in varyantlar {
                    if self.tuval.bolge.ortusur(vp.bas, vp.bit) {
                        konumlar.push((vp.bas, vp.bit));
                    }
                }
            }
        }
        konumlar
    }
}

/// Bir varyantın (`bas..=bit`) üstünden geçen okuma indekslerini döndürür (downsampling'de
/// korunacak "önemli" read'ler — varyant kanıtı gizlenmez).
fn onemli_okumalar(okumalar: &[OkumaParcasi], varyant_konumlari: &[(u64, u64)]) -> Vec<usize> {
    if varyant_konumlari.is_empty() {
        return Vec::new();
    }
    okumalar
        .iter()
        .enumerate()
        .filter(|(_, o)| {
            varyant_konumlari
                .iter()
                .any(|&(vb, ve)| o.bas <= ve && o.bit >= vb)
        })
        .map(|(i, _)| i)
        .collect()
}

/// Alt-modülün UI/komut kayıtları — genom tarayıcı **GPU** ile çizildiğinden yalnızca `gpu`
/// yetkisi verildiyse komutlar sunulur (en az yetki + dürüstlük: `db_search`/`data_io` deseni).
/// Dışa aktarma (PNG/SVG dosyaya yazar) ayrıca `fs` ister.
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
        )
        .komut(
            "biocraft.core.studio.browser.referans",
            "BioCraft Studio: Referans Dizi/Çeviri Göster",
        )
        .komut(
            "biocraft.core.studio.browser.karsilastir",
            "BioCraft Studio: Çoklu Örnek Karşılaştır (vaka/kontrol)",
        )
        .komut(
            "biocraft.core.studio.browser.olcum",
            "BioCraft Studio: Ölçüm Aracı (bp mesafe)",
        );
        // Dışa aktarma diske yazar → fs gerekir (MK-13).
        if yetkiler.var_mi(Capability::Fs) {
            akt.komut(
                "biocraft.core.studio.browser.disa_aktar",
                "BioCraft Studio: Görünümü Dışa Aktar (PNG/SVG)",
            );
        }
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
        // Yalnız gpu → 5 görünüm komutu (dışa aktarma yok, fs gerekir).
        let gpu = kayitlar(&YetkiKapisi::yeni([Capability::Gpu]));
        assert_eq!(gpu.ui_say(UiUzantiTuru::Komut), 5);
        // gpu + fs → 6 (PNG/SVG dışa aktarma da sunulur).
        let gpu_fs = kayitlar(&YetkiKapisi::yeni([Capability::Gpu, Capability::Fs]));
        assert_eq!(gpu_fs.ui_say(UiUzantiTuru::Komut), 6);
        // Yalnız fs → 0 (genom tarayıcı gpu ister).
        assert_eq!(
            kayitlar(&YetkiKapisi::yeni([Capability::Fs])).ui_say(UiUzantiTuru::Komut),
            0
        );
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
