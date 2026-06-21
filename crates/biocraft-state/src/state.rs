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
pub const DURUM_SURUMU: u32 = 1;

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
    /// Açık sekmeler/belgeler.
    pub sekmeler: Vec<AcikSekme>,
    /// Etkin sekmenin `sekmeler` içindeki dizini.
    pub aktif_sekme: Option<usize>,
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
            sekmeler: Vec::new(),
            aktif_sekme: None,
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

    /// Eski sürüm durumları güncel şemaya taşır (şimdilik sürüm damgasını günceller; yeni
    /// alanlar `serde` varsayılanıyla zaten dolar).  MK-38: ileri uyum kancası.
    fn gocet(&mut self) {
        if self.surum < DURUM_SURUMU {
            // Gelecekte alan-bazlı dönüşümler buraya; şu an yalnızca damga güncellenir.
            self.surum = DURUM_SURUMU;
        }
    }
}
