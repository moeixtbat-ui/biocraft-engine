//! İP-12 — Kapsamlı, **aranabilir**, **kategorize** ayar sistemi (3. derece ince ayarlar).
//!
//! Bu modül; ayar **modelini** (katmanlı değerler + doğrulama + kalıcılık) ve **ekranını** (arama
//! kutusu + kategoriler + her ayarın açıklaması + "varsayılana dön" + profil dışa/içe aktarma)
//! birlikte sunar.  Tanımlar [`sections`], arama [`search`], profil [`profiles`] alt modüllerinde.
//!
//! ## Katmanlar (öncelik)
//! `Proje > Kullanıcı(global) > Varsayılan`.  Bir projede yapılan ayar **global'i geçersiz kılar**
//! (kabul kriteri).  Çalışma alanı katmanı ileride aynı kalıpla eklenir (kanca hazır).
//!
//! ## Güvenli varsayılan
//! Diskten/profilden gelen **geçersiz** bir değer asla uygulanmaz: okuma/çözme her zaman
//! [`AyarTuru::gecerli_kil`]'den geçer → aralığa sıkışır veya varsayılana iner (kabul kriteri).
//!
//! ## Kalıcılık
//! Bir katman, **sürüm alanlı** ([`AyarKatmaniKaydi`]) JSON olarak serileştirilir; `biocraft-app`
//! bunu `biocraft-state`'in atomik + BLAKE3 mühürlü deposuna (kullanıcı) yazar, proje katmanını ise
//! proje klasörüne (İP-02) bırakabilir.  Sürüm alanı yeni diller/ayarlar eklenince göçü kolaylaştırır.
// MK-52: renkler token'dan; MK-53: tek tanım, iki erişim (menü/komut paleti aynı modeli kullanır).

pub mod profiles;
pub mod search;
pub mod sections;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::components::EmptyState;
use crate::i18n::Dil;
use crate::tokens::{Onem, Tokenlar};

pub use profiles::{AyarProfili, IceAktarRapor, PROFIL_SURUMU};
pub use search::AyarIndeks;
pub use sections::{
    yerlesik_tanimlar, AyarDeger, AyarKategorisi, AyarTanimi, AyarTuru, SecimSecenegi,
};

/// Bir katmanın diske yazılan, **sürüm alanlı** anlık görüntüsü (göç toleransı).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AyarKatmaniKaydi {
    /// Şema sürümü (yeni ayar/şema değişiminde artar).
    pub surum: u32,
    /// Anahtar → değer (yalnızca varsayılandan farklı olan **geçersiz kılmalar** tutulur).
    #[serde(default)]
    pub degerler: BTreeMap<String, AyarDeger>,
}

/// Ayar katmanı şema sürümü.
pub const KATMAN_SURUMU: u32 = 1;

/// Hangi katmanın **düzenlendiği** (ekran set/sıfırla işlemleri bu katmana yazar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AyarKatmani {
    /// Kullanıcı (global) katmanı — tüm projelerde geçerli (varsayılan düzenleme katmanı).
    #[default]
    Kullanici,
    /// Proje katmanı — yalnız açık projede geçerli; global'i geçersiz kılar.
    Proje,
}

/// Ekranın döndürdüğü, **uygulamanın** ele alması gereken eylem (örn. onay gerektiren işlem).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AyarEylem {
    /// "Fabrika ayarlarına dön" istendi — uygulama onay diyaloğu gösterip [`Ayarlar::fabrika_sifirla`]
    /// çağırmalı (yıkıcı; kazara tetiklenmesin).
    FabrikaSifirlaIstendi,
}

/// Ayar **tanımları kayıt defteri** — yerleşik + eklenti tanımları + arama indeksi.
///
/// Eklentiler kendi ayar bölümlerini buraya [`AyarKayit::eklenti_ayari_kaydet`] ile ekler (SDK
/// sözleşmesi [`biocraft_sdk::ui::UiUzantiTuru::Ayar`]); her eklenti ayarı **Eklentiler**
/// kategorisinde gösterilir.
#[derive(Debug, Clone)]
pub struct AyarKayit {
    tanimlar: Vec<AyarTanimi>,
    indeks: AyarIndeks,
}

impl Default for AyarKayit {
    fn default() -> Self {
        Self::yerlesik()
    }
}

impl AyarKayit {
    /// Yalnız yerleşik (çekirdek) ayarlarla kayıt kurar.
    pub fn yerlesik() -> Self {
        let tanimlar = yerlesik_tanimlar();
        let indeks = AyarIndeks::olustur(&tanimlar);
        Self { tanimlar, indeks }
    }

    /// Bir eklenti ayar tanımını kaydeder (her zaman **Eklentiler** kategorisine zorlanır).
    ///
    /// Anahtar çakışırsa (zaten kayıtlı) **reddedilir** (`false`) — sessiz üzerine yazma yok.
    /// Başarılıysa arama indeksi yeniden kurulur.  Eklenti anahtarları `eklenti.<yayinci>.<ad>`
    /// gibi ad-alanlı olmalıdır (çakışmayı en aza indirir).
    pub fn eklenti_ayari_kaydet(&mut self, mut tanim: AyarTanimi) -> bool {
        if self.tanimlar.iter().any(|t| t.anahtar == tanim.anahtar) {
            return false;
        }
        tanim.kategori = AyarKategorisi::Eklentiler;
        self.tanimlar.push(tanim);
        self.indeks = AyarIndeks::olustur(&self.tanimlar);
        true
    }

    /// Bir anahtarın tanımı (yoksa `None`).
    pub fn tanim(&self, anahtar: &str) -> Option<&AyarTanimi> {
        self.tanimlar.iter().find(|t| t.anahtar == anahtar)
    }

    /// Tüm tanımlar.
    pub fn tanimlar(&self) -> &[AyarTanimi] {
        &self.tanimlar
    }

    /// Arama indeksi.
    pub fn indeks(&self) -> &AyarIndeks {
        &self.indeks
    }

    /// Bir kategorideki tanımlar (ekran sırasını korur).
    pub fn kategoride(&self, kategori: AyarKategorisi) -> impl Iterator<Item = &AyarTanimi> {
        self.tanimlar.iter().filter(move |t| t.kategori == kategori)
    }
}

/// Katmanlı ayar deposu + ekran durumu.
///
/// `coz`/`mantik`/`tam_sayi`/… ile **çözülmüş** (katman önceliği + doğrulama uygulanmış) değer
/// okunur; `ayarla`/`varsayilana_don` ile aktif düzenleme katmanı değiştirilir.
#[derive(Debug, Clone)]
pub struct Ayarlar {
    kayit: AyarKayit,
    /// Kullanıcı (global) katmanı geçersiz kılmaları.
    kullanici: BTreeMap<String, AyarDeger>,
    /// Proje katmanı geçersiz kılmaları (`None` = açık proje yok).
    proje: Option<BTreeMap<String, AyarDeger>>,
    /// Ekranın yazdığı aktif katman.
    duzenleme_katmani: AyarKatmani,
    /// Son değişiklikten beri kaydedilmemiş veri var mı (uygulama kalıcılık tetikler).
    kirli: bool,
    // ── Ekran (geçici) durumu ──
    secili_kategori: AyarKategorisi,
    arama: String,
    profil_panel_acik: bool,
    profil_ad: String,
    profil_metin: String,
    profil_durum: Option<String>,
}

impl Default for Ayarlar {
    fn default() -> Self {
        Self::yeni(AyarKayit::yerlesik())
    }
}

impl Ayarlar {
    /// Verilen kayıt defteriyle (yerleşik + varsa eklenti tanımları) boş katmanlı depo kurar.
    pub fn yeni(kayit: AyarKayit) -> Self {
        Self {
            kayit,
            kullanici: BTreeMap::new(),
            proje: None,
            duzenleme_katmani: AyarKatmani::Kullanici,
            kirli: false,
            secili_kategori: AyarKategorisi::Gorunum,
            arama: String::new(),
            profil_panel_acik: false,
            profil_ad: String::new(),
            profil_metin: String::new(),
            profil_durum: None,
        }
    }

    /// Kayıt defterine erişim (eklenti ayarı kaydetmek için).
    pub fn kayit_mut(&mut self) -> &mut AyarKayit {
        &mut self.kayit
    }

    /// Kayıt defteri (salt-okunur).
    pub fn kayit(&self) -> &AyarKayit {
        &self.kayit
    }

    /// Eklenti ayar tanımını kaydetmenin kısayolu (SDK akışı).
    pub fn eklenti_ayari_kaydet(&mut self, tanim: AyarTanimi) -> bool {
        self.kayit.eklenti_ayari_kaydet(tanim)
    }

    // ── Çözümleme (okuma) ──────────────────────────────────────────────────────

    /// Bir ayarı katman önceliği + doğrulama ile **çözer** (Proje > Kullanıcı > Varsayılan).
    ///
    /// Tanımsız anahtar (programlama hatası) güvenli bir `Mantik(false)` döndürür + debug-assert.
    pub fn coz(&self, anahtar: &str) -> AyarDeger {
        let Some(tanim) = self.kayit.tanim(anahtar) else {
            debug_assert!(false, "tanımsız ayar anahtarı: {anahtar}");
            return AyarDeger::Mantik(false);
        };
        if let Some(proje) = &self.proje {
            if let Some(v) = proje.get(anahtar) {
                return tanim.gecerli_kil(v);
            }
        }
        if let Some(v) = self.kullanici.get(anahtar) {
            return tanim.gecerli_kil(v);
        }
        tanim.varsayilan.clone()
    }

    /// Çözülmüş `bool` (tip uymazsa `false`).
    pub fn mantik(&self, anahtar: &str) -> bool {
        self.coz(anahtar).mantik().unwrap_or(false)
    }

    /// Çözülmüş `i64` (tip uymazsa 0).
    pub fn tam_sayi(&self, anahtar: &str) -> i64 {
        self.coz(anahtar).tam_sayi().unwrap_or(0)
    }

    /// Çözülmüş `f64` (tip uymazsa 0.0).
    pub fn ondalik(&self, anahtar: &str) -> f64 {
        self.coz(anahtar).ondalik().unwrap_or(0.0)
    }

    /// Çözülmüş seçim/metin anahtarı (tip uymazsa boş).
    pub fn secim(&self, anahtar: &str) -> String {
        self.coz(anahtar).metin().unwrap_or_default().to_string()
    }

    // ── Yazma (aktif katman) ────────────────────────────────────────────────────

    /// Aktif düzenleme katmanının değer haritasına değiştirilebilir erişim (gerekirse oluşturur).
    fn aktif_map_mut(&mut self) -> &mut BTreeMap<String, AyarDeger> {
        match self.duzenleme_katmani {
            AyarKatmani::Kullanici => &mut self.kullanici,
            AyarKatmani::Proje => self.proje.get_or_insert_with(BTreeMap::new),
        }
    }

    /// Aktif katman bu anahtar için bir geçersiz kılma içeriyor mu? ("varsayılana dön" etkin mi).
    pub fn aktif_katman_icerir(&self, anahtar: &str) -> bool {
        match self.duzenleme_katmani {
            AyarKatmani::Kullanici => self.kullanici.contains_key(anahtar),
            AyarKatmani::Proje => self.proje.as_ref().is_some_and(|m| m.contains_key(anahtar)),
        }
    }

    /// Bir ayarı aktif katmana yazar (doğrulayarak).  Gerçekten değiştiyse `true` döner + kirletir.
    pub fn ayarla(&mut self, anahtar: &str, deger: AyarDeger) -> bool {
        let Some(tanim) = self.kayit.tanim(anahtar) else {
            return false;
        };
        let gecerli = tanim.gecerli_kil(&deger); // owned → tanim borrow burada biter.
        let map = self.aktif_map_mut();
        if map.get(anahtar) == Some(&gecerli) {
            return false;
        }
        map.insert(anahtar.to_string(), gecerli);
        self.kirli = true;
        true
    }

    /// Bir ayarı aktif katmanda **varsayılana döndürür** (geçersiz kılmayı kaldırır).
    pub fn varsayilana_don(&mut self, anahtar: &str) -> bool {
        let degisti = self.aktif_map_mut().remove(anahtar).is_some();
        if degisti {
            self.kirli = true;
        }
        degisti
    }

    /// Bir kategorinin tüm ayarlarını aktif katmanda varsayılana döndürür.
    pub fn kategori_varsayilana_don(&mut self, kategori: AyarKategorisi) -> usize {
        let anahtarlar: Vec<String> = self
            .kayit
            .kategoride(kategori)
            .map(|t| t.anahtar.clone())
            .collect();
        let mut n = 0;
        for a in anahtarlar {
            if self.varsayilana_don(&a) {
                n += 1;
            }
        }
        n
    }

    /// **Fabrika ayarları:** tüm katmanlardaki geçersiz kılmaları siler (her şey varsayılana döner).
    pub fn fabrika_sifirla(&mut self) {
        self.kullanici.clear();
        if let Some(p) = &mut self.proje {
            p.clear();
        }
        self.kirli = true;
    }

    // ── Katman yönetimi ──────────────────────────────────────────────────────────

    /// Aktif düzenleme katmanı.
    pub fn duzenleme_katmani(&self) -> AyarKatmani {
        self.duzenleme_katmani
    }

    /// Aktif düzenleme katmanını ayarlar (Proje seçilirse ve proje yoksa proje katmanı başlatılır).
    pub fn duzenleme_katmani_ayarla(&mut self, katman: AyarKatmani) {
        if matches!(katman, AyarKatmani::Proje) && self.proje.is_none() {
            self.proje = Some(BTreeMap::new());
        }
        self.duzenleme_katmani = katman;
    }

    /// Açık bir proje katmanı var mı?
    pub fn proje_var(&self) -> bool {
        self.proje.is_some()
    }

    /// Proje katmanını başlatır (boş geçersiz kılma kümesi).
    pub fn proje_baslat(&mut self) {
        self.proje.get_or_insert_with(BTreeMap::new);
    }

    /// Proje katmanını kapatır (geçersiz kılmaları unutur; düzenleme katmanı Kullanıcı'ya döner).
    pub fn proje_kapat(&mut self) {
        self.proje = None;
        self.duzenleme_katmani = AyarKatmani::Kullanici;
    }

    // ── Kirlilik (kalıcılık tetikleyici) ────────────────────────────────────────

    /// Kaydedilmemiş değişiklik var mı?
    pub fn kirli_mi(&self) -> bool {
        self.kirli
    }

    /// Kirlilik bayrağını temizler (uygulama kalıcılık yazdıktan sonra çağırır).
    pub fn kirli_temizle(&mut self) {
        self.kirli = false;
    }

    // ── Kalıcılık (sürüm alanlı serde) ──────────────────────────────────────────

    /// Kullanıcı katmanının kalıcı kaydı.
    pub fn kullanici_kaydi(&self) -> AyarKatmaniKaydi {
        AyarKatmaniKaydi {
            surum: KATMAN_SURUMU,
            degerler: self.kullanici.clone(),
        }
    }

    /// Kullanıcı katmanını bir kayıttan yükler — her değer doğrulanır, tanınmayan **atlanır**.
    pub fn kullanici_yukle(&mut self, kayit: AyarKatmaniKaydi) {
        self.kullanici = self.dogrula_katman(kayit.degerler);
        self.kirli = false;
    }

    /// Kullanıcı katmanını JSON metnine yazar.
    pub fn kullanici_json(&self) -> String {
        serde_json::to_string(&self.kullanici_kaydi()).unwrap_or_else(|_| "{}".to_string())
    }

    /// Kullanıcı katmanını JSON'dan yükler; **biçim bozuksa mevcut değerleri korur** (güvenli).
    pub fn kullanici_yukle_json(&mut self, s: &str) -> bool {
        match serde_json::from_str::<AyarKatmaniKaydi>(s) {
            Ok(k) => {
                self.kullanici_yukle(k);
                true
            }
            Err(_) => false,
        }
    }

    /// Proje katmanının kalıcı kaydı (proje yoksa `None`).
    pub fn proje_kaydi(&self) -> Option<AyarKatmaniKaydi> {
        self.proje.as_ref().map(|m| AyarKatmaniKaydi {
            surum: KATMAN_SURUMU,
            degerler: m.clone(),
        })
    }

    /// Proje katmanını bir kayıttan yükler (doğrulayarak); proje katmanını da başlatır.
    pub fn proje_yukle(&mut self, kayit: AyarKatmaniKaydi) {
        self.proje = Some(self.dogrula_katman(kayit.degerler));
    }

    /// Bir değer haritasını kayıt defterine göre doğrular: bilinmeyen anahtar atlanır, geçersiz
    /// değer güvenli kılınır.
    fn dogrula_katman(&self, ham: BTreeMap<String, AyarDeger>) -> BTreeMap<String, AyarDeger> {
        let mut temiz = BTreeMap::new();
        for (anahtar, deger) in ham {
            if let Some(tanim) = self.kayit.tanim(&anahtar) {
                temiz.insert(anahtar, tanim.gecerli_kil(&deger));
            }
            // tanımsız (eski/eklenti yok) → atla.
        }
        temiz
    }

    // ── Profil (dışa/içe aktarma) ───────────────────────────────────────────────

    /// Aktif katmanın değerlerinden bir profil üretir (**hassas alanlar hariç**).
    pub fn profil_disa_aktar(&self, ad: impl Into<String>) -> AyarProfili {
        let kaynak = match self.duzenleme_katmani {
            AyarKatmani::Kullanici => &self.kullanici,
            AyarKatmani::Proje => self.proje.as_ref().unwrap_or(&self.kullanici),
        };
        AyarProfili::disa_aktar(ad, kaynak, self.kayit.tanimlar())
    }

    /// Bir profili aktif katmana uygular (doğrulayarak; bilinmeyen/hassas atlanır).  Uygulanan
    /// ayar sayısını döner.
    pub fn profil_ice_aktar(&mut self, profil: &AyarProfili) -> usize {
        let rapor = profil.dogrula_ve_coz(self.kayit.tanimlar());
        let n = rapor.uygulanan.len();
        for (anahtar, deger) in rapor.uygulanan {
            self.aktif_map_mut().insert(anahtar, deger);
        }
        if n > 0 {
            self.kirli = true;
        }
        n
    }

    // ── Ekran (egui) ────────────────────────────────────────────────────────────

    /// Tüm ayar ekranını verili `ui` içine çizer (merkez bölge).  Döndürdüğü eylem uygulamaca
    /// işlenir (örn. fabrika sıfırlama onayı).
    pub fn ciz(&mut self, ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) -> Option<AyarEylem> {
        let tr = matches!(dil, Dil::Tr);
        let mut eylem = None;

        // ── Başlık satırı ──
        ui.horizontal(|ui| {
            ui.heading(if tr { "⚙ Ayarlar" } else { "⚙ Settings" });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(if tr {
                        "↺ Fabrika ayarları"
                    } else {
                        "↺ Factory reset"
                    })
                    .on_hover_text(if tr {
                        "Tüm ayarları varsayılana döndürür (onay istenir)."
                    } else {
                        "Resets all settings to defaults (asks for confirmation)."
                    })
                    .clicked()
                {
                    eylem = Some(AyarEylem::FabrikaSifirlaIstendi);
                }
                // Katman seçici (yalnız proje açıkken anlamlı).
                if self.proje_var() {
                    ui.separator();
                    let mut katman = self.duzenleme_katmani;
                    ui.label(if tr { "Katman:" } else { "Layer:" });
                    if ui
                        .selectable_label(
                            matches!(katman, AyarKatmani::Proje),
                            if tr { "Proje" } else { "Project" },
                        )
                        .clicked()
                    {
                        katman = AyarKatmani::Proje;
                    }
                    if ui
                        .selectable_label(
                            matches!(katman, AyarKatmani::Kullanici),
                            if tr { "Genel" } else { "Global" },
                        )
                        .clicked()
                    {
                        katman = AyarKatmani::Kullanici;
                    }
                    if katman != self.duzenleme_katmani {
                        self.duzenleme_katmani_ayarla(katman);
                    }
                }
            });
        });

        // ── Arama kutusu ──
        ui.horizontal(|ui| {
            ui.label("🔎");
            let yanit = ui.add(
                egui::TextEdit::singleline(&mut self.arama)
                    .desired_width(f32::INFINITY)
                    .hint_text(if tr {
                        "Ayarlarda ara… (örn. tema, token, sıcaklık)"
                    } else {
                        "Search settings… (e.g. theme, token, temperature)"
                    }),
            );
            if !self.arama.is_empty() && ui.small_button("✕").clicked() {
                self.arama.clear();
            }
            let _ = yanit;
        });
        ui.separator();

        // Arama varsa ve seçili kategoride sonuç yoksa, eşleşen ilk kategoriye geç (anında bulma).
        if !self.arama.trim().is_empty() {
            let secili_bos = self
                .kayit
                .indeks()
                .ara_kategori(&self.arama, self.secili_kategori)
                .is_empty();
            if secili_bos {
                if let Some(ilk) = self
                    .kayit
                    .indeks()
                    .eslesen_kategoriler(&self.arama)
                    .first()
                    .copied()
                {
                    self.secili_kategori = ilk;
                }
            }
        }

        // Eşleşen kategoriler (sol listede vurgulama/soldurma için).
        let eslesen: Vec<AyarKategorisi> = self.kayit.indeks().eslesen_kategoriler(&self.arama);

        // ── İki sütun: sol kategoriler, sağ ayarlar ──
        let mut degisiklikler: Vec<(String, AyarDeger)> = Vec::new();
        let mut sifirla: Vec<String> = Vec::new();
        let mut kategori_sifirla: Option<AyarKategorisi> = None;

        ui.horizontal_top(|ui| {
            // Sol: kategori listesi (sabit genişlik).
            ui.allocate_ui_with_layout(
                egui::vec2(190.0, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("ayar_kategori")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for &kat in AyarKategorisi::TUMU {
                                let secili = kat == self.secili_kategori;
                                let var = eslesen.contains(&kat);
                                let etiket = format!("{}  {}", kat.ikon(), kat.baslik(dil));
                                let rich = if var {
                                    egui::RichText::new(etiket)
                                } else {
                                    egui::RichText::new(etiket).color(tok.renk.metin_soluk)
                                };
                                if ui.selectable_label(secili, rich).clicked() {
                                    self.secili_kategori = kat;
                                }
                            }
                        });
                },
            );
            ui.separator();
            // Sağ: seçili kategorinin (arama ile süzülmüş) ayarları.
            ui.vertical(|ui| {
                let kat = self.secili_kategori;
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{}  {}", kat.ikon(), kat.baslik(dil)))
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button(if tr {
                                "↺ Kategoriyi sıfırla"
                            } else {
                                "↺ Reset category"
                            })
                            .clicked()
                        {
                            kategori_sifirla = Some(kat);
                        }
                        // Profil panelini aç/kapat.
                        if ui
                            .small_button(if tr { "⇄ Profil" } else { "⇄ Profile" })
                            .clicked()
                        {
                            self.profil_panel_acik = !self.profil_panel_acik;
                        }
                    });
                });
                if kat == AyarKategorisi::Gelismis {
                    ui.colored_label(
                        tok.onem_rengi(Onem::Uyari),
                        if tr {
                            "⚠ Gelişmiş ayarlar deneyseldir; dikkatli değiştirin."
                        } else {
                            "⚠ Advanced settings are experimental; change with care."
                        },
                    );
                }
                ui.separator();

                let anahtarlar: Vec<String> = self
                    .kayit
                    .indeks()
                    .ara_kategori(&self.arama, kat)
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect();

                egui::ScrollArea::vertical()
                    .id_salt("ayar_liste")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if anahtarlar.is_empty() {
                            EmptyState::yeni(
                                "🔎",
                                if tr { "Eşleşme yok" } else { "No matches" },
                                if tr {
                                    "Bu kategoride aramanızla eşleşen ayar yok."
                                } else {
                                    "No setting in this category matches your search."
                                },
                            )
                            .show(ui, tok);
                        }
                        for anahtar in &anahtarlar {
                            let Some(tanim) = self.kayit.tanim(anahtar) else {
                                continue;
                            };
                            let mevcut = self.coz(anahtar);
                            let gecersiz = self.aktif_katman_icerir(anahtar);
                            if let Some(s) = ayar_satiri(ui, tanim, &mevcut, gecersiz, dil, tok) {
                                match s {
                                    SatirSonuc::Degisti(v) => {
                                        degisiklikler.push((anahtar.clone(), v))
                                    }
                                    SatirSonuc::Sifirla => sifirla.push(anahtar.clone()),
                                }
                            }
                            ui.separator();
                        }

                        // Profil paneli (dışa/içe aktarma).
                        if self.profil_panel_acik {
                            self.profil_paneli_ciz(ui, dil, tok);
                        }
                    });
            });
        });

        // Toplanan değişiklikleri uygula (çizim borç çakışması olmadan).
        for (anahtar, deger) in degisiklikler {
            self.ayarla(&anahtar, deger);
        }
        for anahtar in sifirla {
            self.varsayilana_don(&anahtar);
        }
        if let Some(kat) = kategori_sifirla {
            self.kategori_varsayilana_don(kat);
        }

        eylem
    }

    /// Profil dışa/içe aktarma panelini çizer (JSON metin alanı + butonlar + pano).
    fn profil_paneli_ciz(&mut self, ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) {
        let tr = matches!(dil, Dil::Tr);
        ui.add_space(tok.bosluk.s);
        ui.group(|ui| {
            ui.label(
                egui::RichText::new(if tr {
                    "⇄ Ayar profili (dışa/içe aktar)"
                } else {
                    "⇄ Settings profile (export/import)"
                })
                .strong(),
            );
            ui.label(
                egui::RichText::new(if tr {
                    "Profil paylaşım/yedek içindir. Hassas alanlar (API anahtarı) DAHİL EDİLMEZ."
                } else {
                    "A profile is for sharing/backup. Sensitive fields (API keys) are EXCLUDED."
                })
                .small()
                .color(tok.renk.metin_soluk),
            );
            ui.horizontal(|ui| {
                ui.label(if tr { "Ad:" } else { "Name:" });
                ui.add(
                    egui::TextEdit::singleline(&mut self.profil_ad)
                        .desired_width(180.0)
                        .hint_text(if tr {
                            "örn. Sunum modu"
                        } else {
                            "e.g. Presentation"
                        }),
                );
            });
            ui.horizontal(|ui| {
                if ui
                    .button(if tr { "Dışa aktar →" } else { "Export →" })
                    .clicked()
                {
                    let ad = if self.profil_ad.trim().is_empty() {
                        if tr {
                            "Profil"
                        } else {
                            "Profile"
                        }
                    } else {
                        self.profil_ad.trim()
                    };
                    let profil = self.profil_disa_aktar(ad.to_string());
                    match profil.json() {
                        Ok(j) => {
                            self.profil_metin = j;
                            self.profil_durum = Some(if tr {
                                format!("{} ayar dışa aktarıldı.", profil.degerler.len())
                            } else {
                                format!("{} settings exported.", profil.degerler.len())
                            });
                        }
                        Err(e) => self.profil_durum = Some(e),
                    }
                }
                if ui
                    .button(if tr { "← İçe aktar" } else { "← Import" })
                    .clicked()
                {
                    match AyarProfili::jsondan(&self.profil_metin) {
                        Ok(p) => {
                            let n = self.profil_ice_aktar(&p);
                            self.profil_durum = Some(if tr {
                                format!("{n} ayar içe aktarıldı (hassas/bilinmeyen atlandı).")
                            } else {
                                format!("{n} settings imported (sensitive/unknown skipped).")
                            });
                        }
                        Err(e) => self.profil_durum = Some(e),
                    }
                }
                if !self.profil_metin.is_empty()
                    && ui
                        .button(if tr {
                            "📋 Panoya kopyala"
                        } else {
                            "📋 Copy"
                        })
                        .clicked()
                {
                    let s = self.profil_metin.clone();
                    ui.output_mut(|o| o.copied_text = s);
                    self.profil_durum = Some(if tr {
                        "Panoya kopyalandı.".to_string()
                    } else {
                        "Copied to clipboard.".to_string()
                    });
                }
            });
            ui.add(
                egui::TextEdit::multiline(&mut self.profil_metin)
                    .desired_rows(6)
                    .desired_width(f32::INFINITY)
                    .code_editor()
                    .hint_text(if tr {
                        "Profil JSON'u burada görünür; içe aktarmak için JSON yapıştırın."
                    } else {
                        "Profile JSON appears here; paste JSON to import."
                    }),
            );
            if let Some(durum) = &self.profil_durum {
                ui.colored_label(tok.onem_rengi(Onem::Bilgi), durum);
            }
        });
    }
}

/// Bir ayar satırının çizim sonucu.
enum SatirSonuc {
    /// Widget değeri değişti — yeni değer.
    Degisti(AyarDeger),
    /// "Varsayılana dön" tıklandı.
    Sifirla,
}

/// Tek bir ayar satırını çizer: başlık + widget + açıklama + "varsayılana dön".
fn ayar_satiri(
    ui: &mut egui::Ui,
    tanim: &AyarTanimi,
    mevcut: &AyarDeger,
    gecersiz_kildi: bool,
    dil: Dil,
    tok: &Tokenlar,
) -> Option<SatirSonuc> {
    let tr = matches!(dil, Dil::Tr);
    let mut sonuc = None;

    // Başlık + (yeniden başlat rozeti) + varsayılana dön.
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tanim.baslik(dil)).strong());
        if tanim.yeniden_baslat {
            ui.label(
                egui::RichText::new(if tr {
                    "⟳ yeniden başlat"
                } else {
                    "⟳ restart"
                })
                .small()
                .color(tok.onem_rengi(Onem::Uyari)),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Yalnız aktif katmanda bir geçersiz kılma varsa "varsayılana dön" etkin.
            if ui
                .add_enabled(gecersiz_kildi, egui::Button::new("↺").small())
                .on_hover_text(if tr {
                    "Varsayılana dön"
                } else {
                    "Reset to default"
                })
                .clicked()
            {
                sonuc = Some(SatirSonuc::Sifirla);
            }
        });
    });

    // Değere göre widget.
    if let Some(v) = ayar_widget(ui, tanim, mevcut, dil) {
        sonuc = Some(SatirSonuc::Degisti(v));
    }

    // Açıklama (her ayarın açıklaması var — kabul kriteri).
    ui.label(
        egui::RichText::new(tanim.aciklama(dil))
            .small()
            .color(tok.renk.metin_soluk),
    );

    sonuc
}

/// Ayarın tipine uygun widget'ı çizer; değer değiştiyse yeni değeri döner.
fn ayar_widget(
    ui: &mut egui::Ui,
    tanim: &AyarTanimi,
    mevcut: &AyarDeger,
    dil: Dil,
) -> Option<AyarDeger> {
    let tr = matches!(dil, Dil::Tr);
    match &tanim.tur {
        AyarTuru::Mantik => {
            let mut b = mevcut.mantik().unwrap_or(false);
            let etiket = if b {
                if tr {
                    "Açık"
                } else {
                    "On"
                }
            } else if tr {
                "Kapalı"
            } else {
                "Off"
            };
            if ui.checkbox(&mut b, etiket).changed() {
                return Some(AyarDeger::Mantik(b));
            }
        }
        AyarTuru::TamSayi { min, max, .. } => {
            let mut n = mevcut.tam_sayi().unwrap_or(*min);
            if ui
                .add(egui::Slider::new(&mut n, *min..=*max).clamping(egui::SliderClamping::Always))
                .changed()
            {
                return Some(AyarDeger::TamSayi(n));
            }
        }
        AyarTuru::Ondalik { min, max, .. } => {
            let mut f = mevcut.ondalik().unwrap_or(*min);
            if ui.add(egui::Slider::new(&mut f, *min..=*max)).changed() {
                return Some(AyarDeger::Ondalik(f));
            }
        }
        AyarTuru::Metin { azami_uzunluk } => {
            let mut s = mevcut.metin().unwrap_or_default().to_string();
            let alan = egui::TextEdit::singleline(&mut s)
                .desired_width(260.0)
                .char_limit(*azami_uzunluk)
                // Hassas alan (API anahtarı) gizli gösterilir.
                .password(tanim.hassas);
            if ui.add(alan).changed() {
                return Some(AyarDeger::Metin(s));
            }
        }
        AyarTuru::Secim { secenekler } => {
            let mut secili = mevcut.metin().unwrap_or_default().to_string();
            let gosterilen = secenekler
                .iter()
                .find(|o| o.anahtar == secili)
                .map(|o| o.etiket(dil).to_string())
                .unwrap_or_else(|| secili.clone());
            let mut degisti = false;
            egui::ComboBox::from_id_salt(&tanim.anahtar)
                .selected_text(gosterilen)
                .show_ui(ui, |ui| {
                    for o in secenekler {
                        if ui
                            .selectable_label(o.anahtar == secili, o.etiket(dil))
                            .clicked()
                        {
                            secili = o.anahtar.clone();
                            degisti = true;
                        }
                    }
                });
            if degisti {
                return Some(AyarDeger::Secim(secili));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests;
