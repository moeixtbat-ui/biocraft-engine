//! Kalıcı uygulama durumu (UI state) — MK-38: oturumlar arası saklanan görünüm/düzen/sekme/tercih.
//!
//! egui *immediate-mode* olduğundan ekranı her karede sıfırdan çizer; kalıcı durum bu yüzden
//! **ayrı** bir yapıda (burada) tutulur ve her açılışta geri yüklenir.  Bu modül yalnızca **veri**
//! ve serileştirmedir; *ne zaman* kaydedileceği [`crate::autosave`], *nasıl* kaydedileceği
//! [`crate::store`], orkestrasyon [`crate::DurumYoneticisi`] sorumluluğundadır.
//!
//! Tema/dil için burada **nötr** enum'lar tanımlanır (UI katmanına bağımlılık yok — MK-40: L2,
//! L4'e bağlanamaz).  `biocraft-app` bunları `biocraft-ui` tipleriyle eşler.

use std::collections::BTreeMap;

use biocraft_types::ErrorReport;
use serde::{Deserialize, Serialize};

/// Durum şemasının sürümü.  İleride alan ekl/değişince artırılır; [`UygulamaDurumu::serde_oku`]
/// eski sürümleri yükseltir (MK-38: göç).
///
/// - Sürüm 2 (İP-03 Gün 11): 6-bölge kabuk durumu eklendi ([`KabukDurumu`]).
/// - Sürüm 3 (İP-03 Gün 12): kabuğa alt panel + inspector + editör bölme + yoğun mod alanları;
///   isimli **özel düzenler** ([`UygulamaDurumu::ozel_duzenler`]).  Eski kayıtlardaki yeni alanlar
///   `#[serde(default)]` ile güvenle dolar (eksik alana güvenli varsayılan — düzen bozulmaz).
pub const DURUM_SURUMU: u32 = 3;

/// Görünüm teması seçimi (nötr; UI'daki `Tema` ile eşlenir).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TemaSecimi {
    /// Koyu tema (varsayılan).
    #[default]
    Koyu,
    /// Açık tema.
    Acik,
    /// Yüksek kontrast (erişilebilirlik).
    YuksekKontrast,
}

/// Arayüz dili seçimi (nötr; UI'daki `Dil` ile eşlenir).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DilSecimi {
    /// Türkçe (varsayılan).
    #[default]
    Tr,
    /// İngilizce.
    En,
}

/// Pencere geometrisi (boyut + büyütülmüş durumu).  Konum platforma göre güvenilmez
/// olduğundan kapsam dışı; boyut + maksimize her açılışta geri yüklenir.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PencereDurumu {
    /// İç (içerik) genişliği — mantıksal piksel.
    pub genislik: u32,
    /// İç (içerik) yüksekliği — mantıksal piksel.
    pub yukseklik: u32,
    /// Pencere büyütülmüş (maksimize) mü?
    pub buyutulmus: bool,
}

impl Default for PencereDurumu {
    fn default() -> Self {
        // İP-04 host'unun açılış boyutuyla aynı.
        Self {
            genislik: 1280,
            yukseklik: 800,
            buyutulmus: false,
        }
    }
}

/// Panel düzeni: yan panelin açık/kapalı durumu + genişliği (oturumlar arası korunur).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PanelDurumu {
    /// Sağ (3B) panel açık mı?
    pub sag_panel_acik: bool,
    /// Sağ panelin genişliği — mantıksal piksel.
    pub sag_panel_genislik: f32,
}

impl Default for PanelDurumu {
    fn default() -> Self {
        Self {
            sag_panel_acik: true,
            sag_panel_genislik: 320.0,
        }
    }
}

/// Activity Bar'da seçili ana mod (nötr; UI'daki `ActivityMod` ile eşlenir).
///
/// 6-bölge kabuğun (İP-03) Activity Bar'ı bu modu değiştirir; mod, Side Panel içeriğini
/// belirler.  L2 katmanı UI tiplerine bağlanamayacağından (MK-40) burada nötr tutulur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AktifModSecimi {
    /// Proje gezgini (varsayılan).
    #[default]
    Proje,
    /// Eklenti yönetimi.
    Eklentiler,
    /// Arama.
    Arama,
    /// AI yüzeyi.
    Ai,
    /// Veritabanı.
    Veritabani,
    /// Ayarlar.
    Ayar,
}

/// Alt Panel'de seçili sekme (nötr; UI'daki `AltSekme` ile eşlenir).
///
/// 0.9 tablosu / İP-03: Konsol/çıktı, "Arka Plan İşleri", AI sohbet, günlük sekmeleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AltSekmeSecimi {
    /// Konsol / çıktı (varsayılan).
    #[default]
    Konsol,
    /// Arka plan işleri (ilerleme + iptal).
    Isler,
    /// AI sohbet (MVP'de yapılandırılmadı — MK-48).
    Ai,
    /// Günlük (log).
    Gunluk,
}

/// Editör/Canvas alanının bölme (split) yönü (nötr; UI'daki `BolmeYonu` ile eşlenir).
///
/// İki veriyi (örn. iki BAM) yan-yana karşılaştırmak için (İP-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BolmeYonuSecimi {
    /// Bölme yok — tek editör grubu (varsayılan).
    #[default]
    Yok,
    /// Yatay bölme: iki grup yan-yana (sol|sağ).
    Yatay,
    /// Dikey bölme: iki grup alt-alta (üst/alt).
    Dikey,
}

/// 6-bölge kabuğun (İP-03) kalıcı düzeni: Activity mod + Side Panel + alt Panel + Inspector +
/// editör bölme + yoğun/sade mod.
///
/// Tüm panel ölçü/görünürlükleri oturumlar arası korunur (kabul kriteri: "kapatıp açınca kalıcı").
/// Bu yapı bir bütün olarak **özel düzen** ([`UygulamaDurumu::ozel_duzenler`]) olarak da kaydedilir;
/// `%100 sadakatle` geri yüklenir.  `Copy`'dir (yalnızca skaler alanlar) → düzen anlık-görüntüsü
/// ucuza klonlanır.  Sürüm 2 kayıtlarında olmayan yeni alanlar `#[serde(default)]` ile güvenli
/// varsayılana iner (düzen yüklenince bozulmaz).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KabukDurumu {
    /// Activity Bar'da seçili ana mod.
    pub aktif_mod: AktifModSecimi,
    /// Sol (Activity'ye komşu) Side Panel açık mı?
    pub yan_panel_acik: bool,
    /// Side Panel genişliği — mantıksal piksel.
    pub yan_panel_genislik: f32,
    /// Alt Panel (Konsol/İşler/AI/Günlük) açık mı?
    #[serde(default = "alt_panel_acik_varsayilan")]
    pub alt_panel_acik: bool,
    /// Alt Panel yüksekliği — mantıksal piksel.
    #[serde(default = "alt_panel_yukseklik_varsayilan")]
    pub alt_panel_yukseklik: f32,
    /// Alt Panel'de seçili sekme.
    #[serde(default)]
    pub alt_panel_sekme: AltSekmeSecimi,
    /// Inspector (sağ özellik paneli) açık mı?
    #[serde(default = "inspector_acik_varsayilan")]
    pub inspector_acik: bool,
    /// Inspector genişliği — mantıksal piksel.
    #[serde(default = "inspector_genislik_varsayilan")]
    pub inspector_genislik: f32,
    /// Editör/Canvas bölme yönü (yan-yana karşılaştırma).
    #[serde(default)]
    pub bolme_yonu: BolmeYonuSecimi,
    /// Bölme oranı (birincil grubun payı; `0.1..=0.9`).
    #[serde(default = "bolme_orani_varsayilan")]
    pub bolme_orani: f32,
    /// Yoğun mod açık mı? (kapalı = sade mod; daha geniş boşluk/az panel.)
    #[serde(default)]
    pub yogun_mod: bool,
}

// İP-03 Gün 12: serde varsayılan üreticileri (eski kayıtta eksik alanlar için güvenli değerler).
fn alt_panel_acik_varsayilan() -> bool {
    true
}
fn alt_panel_yukseklik_varsayilan() -> f32 {
    180.0
}
fn inspector_acik_varsayilan() -> bool {
    true
}
fn inspector_genislik_varsayilan() -> f32 {
    300.0
}
fn bolme_orani_varsayilan() -> f32 {
    0.5
}

impl Default for KabukDurumu {
    fn default() -> Self {
        Self {
            aktif_mod: AktifModSecimi::default(),
            yan_panel_acik: true,
            // İP-03 / 0.9 tablosu: 200–600 px aralığı, makul bir başlangıç genişliği.
            yan_panel_genislik: 260.0,
            alt_panel_acik: alt_panel_acik_varsayilan(),
            alt_panel_yukseklik: alt_panel_yukseklik_varsayilan(),
            alt_panel_sekme: AltSekmeSecimi::default(),
            inspector_acik: inspector_acik_varsayilan(),
            inspector_genislik: inspector_genislik_varsayilan(),
            bolme_yonu: BolmeYonuSecimi::default(),
            bolme_orani: bolme_orani_varsayilan(),
            yogun_mod: false,
        }
    }
}

impl KabukDurumu {
    /// Bozuk/aralık-dışı (eski/elle düzenlenmiş/NaN) ölçüleri güvenli aralıklara çeker.
    ///
    /// Özel düzen yüklenince veya disk durumu okunurken çağrılır; böylece geçersiz bir kayıt bile
    /// kabuğu bozmaz (MK-28: güvenli varsayılana düş).  Genişlik aralıkları UI sabitleriyle
    /// (`YAN_PANEL_MIN/MAX` vb.) uyumlu tutulur; burada L2 katmanı olduğundan sayılar gömülüdür.
    pub fn gecerli_kil(&mut self) {
        self.yan_panel_genislik = sikistir(self.yan_panel_genislik, 200.0, 600.0, 260.0);
        self.alt_panel_yukseklik = sikistir(self.alt_panel_yukseklik, 80.0, 600.0, 180.0);
        self.inspector_genislik = sikistir(self.inspector_genislik, 180.0, 600.0, 300.0);
        self.bolme_orani = sikistir(self.bolme_orani, 0.1, 0.9, 0.5);
    }
}

/// Bir değeri `[min, max]`'a sıkıştırır; geçersiz (NaN/∞) ise `varsayilan`'a indirir.
fn sikistir(deger: f32, min: f32, max: f32, varsayilan: f32) -> f32 {
    if !deger.is_finite() {
        varsayilan
    } else {
        deger.clamp(min, max)
    }
}

/// Açık bir sekme/belge.  6-bölge kabuk (İP-03) ve düzenleyiciler (İP-05/06) gelince bu
/// listeyi doldurur; bugün model hazır, gerçek dosya/sekme akışı sonra bağlanır.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcikSekme {
    /// Disk yolu (geçici/isimsiz belgelerde `None`).
    pub yol: Option<String>,
    /// Sekme başlığı (kullanıcıya görünen ad).
    pub baslik: String,
    /// Kaydedilmemiş değişiklik var mı? (kapatma koruması + kurtarma için).
    pub kaydedilmemis: bool,
}

/// Oturumlar arası saklanan tüm kalıcı UI durumu.
///
/// `serde` ile JSON'a yazılır; [`crate::store`] bunu BLAKE3 mühürlü + atomik yazar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UygulamaDurumu {
    /// Şema sürümü (göç için).
    pub surum: u32,
    /// Pencere geometrisi.
    pub pencere: PencereDurumu,
    /// Aktif tema (görünüm).
    pub tema: TemaSecimi,
    /// Aktif dil (tercih).
    pub dil: DilSecimi,
    /// Panel düzeni (boyut/görünürlük).
    pub panel: PanelDurumu,
    /// 6-bölge kabuk durumu (Activity mod + Side Panel düzeni — İP-03).
    ///
    /// `#[serde(default)]`: sürüm 1 (kabuksuz) kayıtlar bu alan olmadan da okunur; eksikse
    /// varsayılan kabuk durumu kullanılır (MK-38: ileri/geri uyum).
    #[serde(default)]
    pub kabuk: KabukDurumu,
    /// Açık sekmeler/belgeler.
    pub sekmeler: Vec<AcikSekme>,
    /// Etkin sekmenin `sekmeler` içindeki dizini.
    pub aktif_sekme: Option<usize>,
    /// İsimli **özel düzenler** (İP-03 Gün 12): kullanıcı bir kabuk düzenini adlandırıp kaydeder;
    /// daha sonra `%100 sadakatle` geri yükler.  Anahtar = düzen adı, değer = o anki [`KabukDurumu`].
    ///
    /// `#[serde(default)]`: sürüm 2 (bu alan olmadan) kayıtlar boş map ile okunur (MK-38: ileri uyum).
    #[serde(default)]
    pub ozel_duzenler: BTreeMap<String, KabukDurumu>,
    /// Serbest biçim ek tercihler (ileride ayar sistemi — İP-12 — buraya yazar).
    pub tercihler: BTreeMap<String, String>,
}

impl Default for UygulamaDurumu {
    fn default() -> Self {
        Self {
            surum: DURUM_SURUMU,
            pencere: PencereDurumu::default(),
            tema: TemaSecimi::default(),
            dil: DilSecimi::default(),
            panel: PanelDurumu::default(),
            kabuk: KabukDurumu::default(),
            sekmeler: Vec::new(),
            aktif_sekme: None,
            ozel_duzenler: BTreeMap::new(),
            tercihler: BTreeMap::new(),
        }
    }
}

impl UygulamaDurumu {
    /// Kaydedilmemiş değişiklik içeren en az bir sekme var mı? (kapatma uyarısı + kurtarma).
    pub fn kaydedilmemis_var(&self) -> bool {
        self.sekmeler.iter().any(|s| s.kaydedilmemis)
    }

    /// Durumu serileştirir (okunaklı JSON — gerektiğinde elle incelenebilir).
    pub fn serde_yaz(&self) -> Result<Vec<u8>, ErrorReport> {
        serde_json::to_vec_pretty(self).map_err(|e| {
            ErrorReport::new(
                "Durum kaydedilemedi",
                "Uygulama durumu metne çevrilemedi (serileştirme hatası).",
                "Bu beklenmedik bir durum; lütfen tekrar deneyin.",
            )
            .with_teknik_detay(format!("serde_json: {e}"))
        })
    }

    /// Serileştirilmiş durumu okur ve gerekiyorsa güncel şemaya yükseltir (göç).
    pub fn serde_oku(baytlar: &[u8]) -> Result<Self, ErrorReport> {
        let mut durum: UygulamaDurumu = serde_json::from_slice(baytlar).map_err(|e| {
            ErrorReport::new(
                "Kayıtlı durum okunamadı",
                "Durum dosyasının biçimi tanınamadı (eski/eksik/bozuk olabilir).",
                "Uygulama güvenli varsayılan durumla açılır; ayarlarınızı yeniden yapabilirsiniz.",
            )
            .with_teknik_detay(format!("serde_json: {e}"))
        })?;
        durum.gocet();
        Ok(durum)
    }

    /// Eski sürüm durumları güncel şemaya taşır.  Yeni alanlar `serde` varsayılanıyla zaten
    /// dolar; burada ek olarak tüm düzen ölçüleri **güvenli aralıklara** çekilir (eski/elle
    /// düzenlenmiş/bozuk kayıt kabuğu bozmasın — MK-28, MK-38).
    fn gocet(&mut self) {
        if self.surum < DURUM_SURUMU {
            self.surum = DURUM_SURUMU;
        }
        // Aktif kabuk + tüm kayıtlı özel düzenler aralık-zorlanır (sürümden bağımsız savunma).
        self.kabuk.gecerli_kil();
        for duzen in self.ozel_duzenler.values_mut() {
            duzen.gecerli_kil();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surum2_kayit_yeni_alanlari_varsayilanla_doldurur() {
        // Sürüm 2 (Gün 11) formatı: kabuk yalnızca 3 alan içerir, ozel_duzenler hiç yok.
        let surum2 = r#"{
            "surum": 2,
            "pencere": { "genislik": 1280, "yukseklik": 800, "buyutulmus": false },
            "tema": "Koyu",
            "dil": "Tr",
            "panel": { "sag_panel_acik": true, "sag_panel_genislik": 320.0 },
            "kabuk": { "aktif_mod": "Veritabani", "yan_panel_acik": true, "yan_panel_genislik": 300.0 },
            "sekmeler": [],
            "aktif_sekme": null,
            "tercihler": {}
        }"#;
        let d = UygulamaDurumu::serde_oku(surum2.as_bytes()).expect("sürüm 2 okunmalı");
        // Göç sürümü yükseltir.
        assert_eq!(d.surum, DURUM_SURUMU);
        // Eski alanlar korunur.
        assert_eq!(d.kabuk.aktif_mod, AktifModSecimi::Veritabani);
        assert_eq!(d.kabuk.yan_panel_genislik, 300.0);
        // Yeni alanlar güvenli varsayılana iner (düzen bozulmaz).
        assert_eq!(d.kabuk.alt_panel_acik, alt_panel_acik_varsayilan());
        assert_eq!(
            d.kabuk.alt_panel_yukseklik,
            alt_panel_yukseklik_varsayilan()
        );
        assert_eq!(d.kabuk.alt_panel_sekme, AltSekmeSecimi::Konsol);
        assert_eq!(d.kabuk.inspector_acik, inspector_acik_varsayilan());
        assert_eq!(d.kabuk.bolme_yonu, BolmeYonuSecimi::Yok);
        assert!(!d.kabuk.yogun_mod);
        // Özel düzenler boş map ile gelir.
        assert!(d.ozel_duzenler.is_empty());
    }

    #[test]
    fn gecerli_kil_bozuk_olculeri_guvenli_aralaga_ceker() {
        let mut k = KabukDurumu {
            yan_panel_genislik: f32::NAN, // geçersiz → varsayılan
            alt_panel_yukseklik: 5000.0,  // üst sınır aşımı → max
            inspector_genislik: 10.0,     // alt sınır altı → min
            bolme_orani: 2.0,             // aralık dışı → 0.9
            ..KabukDurumu::default()
        };
        k.gecerli_kil();
        assert_eq!(k.yan_panel_genislik, 260.0);
        assert_eq!(k.alt_panel_yukseklik, 600.0);
        assert_eq!(k.inspector_genislik, 180.0);
        assert_eq!(k.bolme_orani, 0.9);
    }

    #[test]
    fn ozel_duzen_tam_sadakatle_kaydedilip_yuklenir() {
        // Özel düzen = KabukDurumu anlık-görüntüsü; serde gidiş-dönüşü birebir korur (%100 sadakat).
        let mut d = UygulamaDurumu::default();
        let ozel = KabukDurumu {
            aktif_mod: AktifModSecimi::Ai,
            yan_panel_acik: false,
            yan_panel_genislik: 420.0,
            alt_panel_acik: false,
            alt_panel_yukseklik: 240.0,
            alt_panel_sekme: AltSekmeSecimi::Gunluk,
            inspector_acik: true,
            inspector_genislik: 360.0,
            bolme_yonu: BolmeYonuSecimi::Dikey,
            bolme_orani: 0.35,
            yogun_mod: true,
        };
        d.ozel_duzenler.insert("Karşılaştırma".to_string(), ozel);
        let baytlar = d.serde_yaz().unwrap();
        let geri = UygulamaDurumu::serde_oku(&baytlar).unwrap();
        assert_eq!(geri.ozel_duzenler.get("Karşılaştırma"), Some(&ozel));
    }
}
