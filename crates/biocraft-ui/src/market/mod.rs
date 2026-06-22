//! İP-18 — **Bilim Pazarı + doğrulanmış haber akışı + eklenti mağazası** (MK-16, MK-47).
//!
//! VS Code standardında bir mağaza (aranabilir/kategorili/derecelendirme/ekran görüntüsü + "resmi/
//! doğrulanmış" rozeti + tek-tık kur/güncelle/kaldır) **ve** küratörlü bilim haberi/makale akışı
//! (kaynak + tarih + dürüst rozet) tek bir yüzeyde toplanır.  İçerik **salt-okur** bir uzak akıştan
//! ([`biocraft_net::feed`]) gelir; MVP'de yayınlama/yorum yazma YOK.
//!
//! **Tasarım/mimari:**
//! - **Salt-okur akış L3'te** (`biocraft-net`): veri modeli + asenkron yükleyici (iskelet/önbellek/
//!   çevrimdışı).  Mağaza **UI'si L4'te** (burada) — token (MK-52) + i18n (MK-53) tek kaynağı UI'de.
//! - **Gerçek kurulum** İP-07 host'u ([`biocraft_plugin_host::Kurucu`]) ile yapılır: imza/bütünlük
//!   denetimi (MK-16) + geri-alınabilir güncelleme.  Yapılandırılmamışsa kurulum yalnızca yerel
//!   kayıt durumunu günceller (üst katman dürüstçe not düşer).
//! - **Dürüstlük:** "doğrulandı" yalnızca imzaya/resmi kaynağa bağlanır; topluluk içeriği
//!   "doğrulama: beklemede" kalır (sahte iddia yok — MK-48).  Çok-AI çapraz kontrol "kesin
//!   doğruluk" diye sunulmaz (MK-47); yalnızca güven sinyali.
//! - **Güvenlik:** içerik **düz metin** olarak render edilir (HTML çalıştırılmaz); dış bağlantı
//!   açmadan **onay** istenir (üst katman uygular); kaynak şeffaf gösterilir.

pub mod coklu_ai;
pub mod detail;
pub mod dogrulama;
pub mod reviews;
pub mod search;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use biocraft_net::{
    HaberKarti, PazarDurumu, PazarKaynagi, PazarOgesi, PazarVerisi, PazarYukleyici, RaporSebebi,
};
use biocraft_plugin_host::{BcextPaket, GuvenDeposu, Kurucu};
use biocraft_types::ErrorReport;

use crate::components::Skeleton;
use crate::i18n::Dil;
use crate::tokens::Tokenlar;

pub use coklu_ai::{coklu_ai_ciz, CokluAiYuzey};
pub use search::{suz_ve_sirala, PazarSuzgec, Siralama};

/// Pazarın iki üst sekmesi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PazarSekme {
    /// Eklenti/şablon/veri seti mağazası.
    #[default]
    Magaza,
    /// Bilim haberi/makale akışı.
    Haberler,
}

/// Bir öğenin kurulum durumu (detay düğmelerini belirler).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KurulumDurum {
    /// Kurulu değil → "Kur".
    KuruluDegil,
    /// Güncel sürüm kurulu → "Kaldır".
    Kurulu,
    /// Eski sürüm kurulu → "Güncelle".
    GuncellemeVar,
}

/// Mağaza/haber yüzeyinin **üst katmana ilettiği** eylemler (app uygular).
///
/// Kur/Güncelle/Kaldır **içeride** uygulanır (host ile); yalnızca onay/dış-etki gerektirenler dışarı
/// taşınır: dış bağlantı (onay), raporlama (moderasyon sinyali), çevrimdışı kurulum, yenileme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PazarEylem {
    /// Bir öğeyi kur (kimlik).
    Kur(String),
    /// Bir öğeyi güncelle (kimlik).
    Guncelle(String),
    /// Bir öğeyi kaldır (kimlik).
    Kaldir(String),
    /// Dış bağlantıyı aç — **onay gerektirir** (üst katman onaylar + tarayıcıda açar).
    DisBaglantiAc(String),
    /// Bir içeriği bildir (moderasyon temeli).
    Raporla {
        /// Bildirilen öğenin kimliği (haber için başlık).
        kimlik: String,
        /// Bildirim sebebi.
        sebep: RaporSebebi,
    },
    /// Çevrimdışı `.bcext` kurulum akışını aç (host ile; üst katman dosya seçer).
    CevrimdisiBcextKur,
    /// Akışı yeniden çek ("tekrar dene").
    Yenile,
}

/// Gerçek kurulum için (opsiyonel) host bağlamı — yapılandırılmamışsa kurulum yalnızca yerel kayıt.
struct KurulumBaglami {
    kurucu: Kurucu,
    depo: GuvenDeposu,
}

/// **Bilim Pazarı** durumu (app `Sahne`'ye gömülür; merkez bölgede çizilir).
pub struct BioCraftPazar {
    /// Aktif sekme (Mağaza / Haberler).
    pub sekme: PazarSekme,
    /// Çok-AI çapraz kontrol yüzeyi (opsiyonel kanca).
    pub coklu_ai: CokluAiYuzey,
    yukleyici: PazarYukleyici,
    baslatildi: bool,
    suzgec: PazarSuzgec,
    siralama: Siralama,
    /// Detayı açık öğenin kimliği (None = liste görünümü).
    secili: Option<String>,
    /// Haber akışı araması.
    haber_sorgu: String,
    /// Kurulu eklentiler: kimlik → kurulu sürüm.
    kurulu: BTreeMap<String, String>,
    /// Gerçek kurulum bağlamı (host); None = yalnızca yerel kayıt.
    kurulum_baglami: Option<KurulumBaglami>,
    /// Son işlem bildirimi (panelde kısa gösterilir).
    son_bildirim: Option<String>,
}

impl Default for BioCraftPazar {
    fn default() -> Self {
        Self::yeni()
    }
}

impl BioCraftPazar {
    /// Yeni (boş) bir pazar — yükleme henüz başlamaz (ilk açılışta [`baslat`](Self::baslat)).
    pub fn yeni() -> Self {
        Self {
            sekme: PazarSekme::Magaza,
            coklu_ai: CokluAiYuzey::yeni(),
            yukleyici: PazarYukleyici::yeni(None),
            baslatildi: false,
            suzgec: PazarSuzgec::default(),
            siralama: Siralama::default(),
            secili: None,
            haber_sorgu: String::new(),
            kurulu: BTreeMap::new(),
            kurulum_baglami: None,
            son_bildirim: None,
        }
    }

    /// Önbellekle kurar (çevrimdışı ilk görünüm için).
    pub fn onbellek_ile(onbellek: PazarVerisi) -> Self {
        let mut s = Self::yeni();
        s.yukleyici = PazarYukleyici::yeni(Some(onbellek));
        s
    }

    /// Gerçek host kurulumunu etkinleştirir (imza denetimi + geri-alınabilir güncelleme — İP-07).
    pub fn kurulum_baglami_ayarla(
        &mut self,
        eklenti_dizini: impl Into<std::path::PathBuf>,
        ayar_dizini: impl Into<std::path::PathBuf>,
        depo: GuvenDeposu,
    ) {
        self.kurulum_baglami = Some(KurulumBaglami {
            kurucu: Kurucu::yeni(eklenti_dizini, ayar_dizini),
            depo,
        });
    }

    /// Önceden kurulu kabul edilen eklentileri tohumlar (örn. çekirdek eklenti varsayılan kurulu).
    pub fn kurulu_tohumla(&mut self, kimlik: impl Into<String>, surum: impl Into<String>) {
        self.kurulu.insert(kimlik.into(), surum.into());
    }

    /// Yükleme başlatıldı mı?
    pub fn baslatildi(&self) -> bool {
        self.baslatildi
    }

    /// Arka planda içerik çekmeyi başlatır (arayüzü bloklamaz).
    pub fn baslat<K: PazarKaynagi + 'static>(&mut self, kaynak: K) {
        self.yukleyici.baslat(kaynak);
        self.baslatildi = true;
    }

    /// Asenkron kanalları yoklar (her karede).  Durum değişirse `true`.
    pub fn yokla(&mut self) -> bool {
        let a = self.yukleyici.yokla();
        let b = self.coklu_ai.yokla();
        a || b
    }

    /// Şu an gösterilen içerik (yüklendi veya çevrimdışı önbellek).
    fn veri(&self) -> Option<&PazarVerisi> {
        self.yukleyici.durum().veri()
    }

    /// Bir öğenin kurulum durumu (kurulu sürüm vs. mağaza sürümü).
    pub fn kurulum_durumu(&self, oge: &PazarOgesi) -> KurulumDurum {
        match self.kurulu.get(&oge.kimlik) {
            None => KurulumDurum::KuruluDegil,
            Some(kurulu) if *kurulu != oge.surum => KurulumDurum::GuncellemeVar,
            Some(_) => KurulumDurum::Kurulu,
        }
    }

    /// Bir öğeyi **kurar** — host varsa gerçek `.bcext` kurulumu (imza denetimi), yoksa yerel kayıt.
    pub fn kur(&mut self, oge: &PazarOgesi) -> Result<(), ErrorReport> {
        if let Some(b) = &self.kurulum_baglami {
            if let Some(bytes) = &oge.paket {
                let paket = BcextPaket::ac(bytes)?;
                b.kurucu.kur(&paket, &b.depo)?;
            }
        }
        self.kurulu.insert(oge.kimlik.clone(), oge.surum.clone());
        Ok(())
    }

    /// Bir öğeyi **günceller** (geri-alınabilir; host varsa gerçek).
    pub fn guncelle(&mut self, oge: &PazarOgesi) -> Result<(), ErrorReport> {
        if let Some(b) = &self.kurulum_baglami {
            if let Some(bytes) = &oge.paket {
                let paket = BcextPaket::ac(bytes)?;
                b.kurucu.guncelle(&paket, false, &b.depo)?;
            }
        }
        self.kurulu.insert(oge.kimlik.clone(), oge.surum.clone());
        Ok(())
    }

    /// Bir öğeyi **kaldırır** (ayarları korur).
    pub fn kaldir(&mut self, kimlik: &str) -> Result<(), ErrorReport> {
        if let Some(b) = &self.kurulum_baglami {
            // Host'ta kurulu değilse (yalnız yerel kayıt) hatayı yut.
            let _ = b.kurucu.kaldir(kimlik, true);
        }
        self.kurulu.remove(kimlik);
        Ok(())
    }

    // ─── çizim ────────────────────────────────────────────────────────────────

    /// Pazarı verili `ui` içine çizer.  Üst katmanın işlemesi gereken eylemi (dış bağlantı/rapor/
    /// çevrimdışı kurulum/yenileme) döner; kur/güncelle/kaldır **içeride** uygulanır.
    pub fn ciz(
        &mut self,
        ui: &mut egui::Ui,
        kayit: &biocraft_ai_surface::SaglayiciKayit,
        dil: Dil,
        tok: &Tokenlar,
    ) -> Option<PazarEylem> {
        self.yokla();
        let tr = matches!(dil, Dil::Tr);
        let mut dis_eylem: Option<PazarEylem> = None;

        // ── Üst bar: başlık + sekmeler + çevrimdışı/yenile ─────────────────────
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new(if tr {
                    "🛒 Bilim Pazarı"
                } else {
                    "🛒 Science Market"
                })
                .color(tok.renk.vurgu),
            );
            ui.separator();
            if ui
                .selectable_label(
                    self.sekme == PazarSekme::Magaza,
                    if tr { "Mağaza" } else { "Store" },
                )
                .clicked()
            {
                self.sekme = PazarSekme::Magaza;
                self.secili = None;
            }
            if ui
                .selectable_label(
                    self.sekme == PazarSekme::Haberler,
                    if tr { "Haberler" } else { "News" },
                )
                .clicked()
            {
                self.sekme = PazarSekme::Haberler;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(if tr { "⟳ Yenile" } else { "⟳ Refresh" })
                    .clicked()
                {
                    dis_eylem = Some(PazarEylem::Yenile);
                }
                if self.yukleyici.durum().cevrimdisi_mi() {
                    ui.label(
                        egui::RichText::new(if tr {
                            "⚠ Çevrimdışı (önbellek)"
                        } else {
                            "⚠ Offline (cache)"
                        })
                        .color(tok.renk.uyari),
                    );
                }
            });
        });
        ui.separator();

        // Son işlem bildirimi (kur/kaldır geri bildirimi).
        if let Some(b) = &self.son_bildirim {
            ui.label(egui::RichText::new(b).italics().color(tok.renk.basari));
            ui.add_space(tok.bosluk.xs);
        }

        // ── Durum: yükleniyor / hata / içerik ──────────────────────────────────
        match self.yukleyici.durum() {
            PazarDurumu::Yukleniyor => {
                yukleniyor_iskelet(ui, tok);
                return dis_eylem;
            }
            PazarDurumu::Hata(rapor) => {
                hata_goster(
                    ui,
                    rapor.ne_oldu.clone(),
                    rapor.nasil_cozulur.clone(),
                    tr,
                    tok,
                    &mut dis_eylem,
                );
                return dis_eylem;
            }
            _ => {}
        }

        // İçerik var → sekmeye göre çiz.  Detay/liste/haber içinden gelen ham eylem
        // (kur/güncelle/kaldır içeride uygulanır; diğerleri üst katmana taşınır).
        let ham_eylem = match self.sekme {
            PazarSekme::Magaza => self.magaza_ciz(ui, kayit, dil, tok),
            PazarSekme::Haberler => self.haber_ciz(ui, dil, tok),
        };

        // ── Ham eylemi uygula: kur/güncelle/kaldır içeride; diğerleri dışarı ────
        if let Some(e) = ham_eylem {
            match e {
                PazarEylem::Kur(kimlik) => self.kurulum_eylemi(&kimlik, KurEylem::Kur, tr),
                PazarEylem::Guncelle(kimlik) => {
                    self.kurulum_eylemi(&kimlik, KurEylem::Guncelle, tr)
                }
                PazarEylem::Kaldir(kimlik) => self.kurulum_eylemi(&kimlik, KurEylem::Kaldir, tr),
                diger => dis_eylem = Some(diger),
            }
        }

        dis_eylem
    }

    /// Kur/güncelle/kaldır ham eylemini uygular + bildirim üretir (borç çakışmasını önlemek için
    /// öğe klonlanır).
    fn kurulum_eylemi(&mut self, kimlik: &str, eylem: KurEylem, tr: bool) {
        let oge = self.veri().and_then(|v| v.oge(kimlik)).cloned();
        let sonuc = match (eylem, &oge) {
            (KurEylem::Kur, Some(o)) => self.kur(o).map(|_| {
                if tr {
                    format!("Kuruldu: {}", o.ad)
                } else {
                    format!("Installed: {}", o.ad)
                }
            }),
            (KurEylem::Guncelle, Some(o)) => self.guncelle(o).map(|_| {
                if tr {
                    format!("Güncellendi: {} → v{}", o.ad, o.surum)
                } else {
                    format!("Updated: {} → v{}", o.ad, o.surum)
                }
            }),
            (KurEylem::Kaldir, _) => self.kaldir(kimlik).map(|_| {
                if tr {
                    format!("Kaldırıldı: {kimlik}")
                } else {
                    format!("Removed: {kimlik}")
                }
            }),
            (_, None) => Ok(if tr {
                "Öğe bulunamadı.".to_string()
            } else {
                "Item not found.".to_string()
            }),
        };
        self.son_bildirim = Some(match sonuc {
            Ok(m) => m,
            Err(rapor) => format!("⚠ {}", rapor.ne_oldu),
        });
    }

    /// Mağaza görünümü (liste + detay).  Ham kurulum/dış eylemini döner.
    fn magaza_ciz(
        &mut self,
        ui: &mut egui::Ui,
        kayit: &biocraft_ai_surface::SaglayiciKayit,
        dil: Dil,
        tok: &Tokenlar,
    ) -> Option<PazarEylem> {
        let tr = matches!(dil, Dil::Tr);
        // Detay açıksa onu çiz.
        if let Some(secili) = self.secili.clone() {
            let oge = self.veri().and_then(|v| v.oge(&secili)).cloned();
            let Some(oge) = oge else {
                self.secili = None;
                return None;
            };
            let mut eylem = None;
            if ui
                .button(if tr {
                    "‹ Mağazaya dön"
                } else {
                    "‹ Back to store"
                })
                .clicked()
            {
                self.secili = None;
            }
            ui.add_space(tok.bosluk.s);
            let durum = self.kurulum_durumu(&oge);
            egui::ScrollArea::vertical().show(ui, |ui| {
                eylem = detail::detay_ciz(ui, &oge, durum, dil, tok);
                ui.add_space(tok.bosluk.l);
                ui.separator();
                // Çok-AI çapraz kontrol kancası (opsiyonel) — detay altında.
                coklu_ai::coklu_ai_ciz(ui, &mut self.coklu_ai, kayit, dil, tok);
            });
            return eylem;
        }

        // Liste görünümü: sol filtreler + sağ kart listesi.
        let mut eylem = None;
        // Arama çubuğu (üstte; anlık filtre).
        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.add(
                egui::TextEdit::singleline(&mut self.suzgec.sorgu)
                    .hint_text(if tr {
                        "Mağazada ara…"
                    } else {
                        "Search store…"
                    })
                    .desired_width(260.0),
            );
            ui.separator();
            // Sıralama seçici.
            egui::ComboBox::from_id_salt("pazar_siralama")
                .selected_text(self.siralama.etiket(tr))
                .show_ui(ui, |ui| {
                    for &s in Siralama::TUMU {
                        ui.selectable_value(&mut self.siralama, s, s.etiket(tr));
                    }
                });
            ui.checkbox(
                &mut self.suzgec.yalniz_ucretsiz,
                if tr { "Ücretsiz" } else { "Free" },
            );
            ui.checkbox(
                &mut self.suzgec.yalniz_dogrulanmis,
                if tr { "Doğrulanmış" } else { "Verified" },
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(if tr {
                        "📦 Çevrimdışı .bcext kur…"
                    } else {
                        "📦 Install offline .bcext…"
                    })
                    .on_hover_text(if tr {
                        "Bir .bcext dosyasından kur (host imza/bütünlük denetiminden geçirir)."
                    } else {
                        "Install from a .bcext file (host checks signature/integrity)."
                    })
                    .clicked()
                {
                    eylem = Some(PazarEylem::CevrimdisiBcextKur);
                }
            });
        });
        ui.add_space(tok.bosluk.xs);

        ui.columns(2, |s| {
            // Sol: kategori + tür filtreleri.
            kategori_panel(&mut s[0], &mut self.suzgec, dil, tok);
            // Sağ: filtrelenmiş kart listesi.
            if let Some(e) = self.kart_listesi(&mut s[1], dil, tok) {
                eylem = Some(e);
            }
        });
        eylem
    }

    /// Sağ sütun: filtrelenip sıralanmış öğe kartları.  Bir kart tıklanırsa detay açılır.
    fn kart_listesi(&mut self, ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) -> Option<PazarEylem> {
        let tr = matches!(dil, Dil::Tr);
        let veri = self.veri()?;
        let sirali = suz_ve_sirala(&veri.ogeler, &self.suzgec, self.siralama, tr);

        if sirali.is_empty() {
            ui.label(
                egui::RichText::new(if tr {
                    "Eşleşen öğe yok. Filtreleri temizleyin."
                } else {
                    "No matching items. Clear the filters."
                })
                .italics()
                .color(tok.renk.metin_soluk),
            );
            return None;
        }

        let mut tiklanan: Option<String> = None;
        let mut hizli_kur: Option<String> = None;
        // İndeks → (kimlik, kurulum durumu) önceden topla (borç çakışmasını önlemek için).
        let kartlar: Vec<(String, KurulumDurum)> = sirali
            .iter()
            .map(|&i| {
                let o = &veri.ogeler[i];
                (o.kimlik.clone(), self.kurulum_durumu(o))
            })
            .collect();
        let veri = self.veri().unwrap(); // yeniden ödünç (immut)
        egui::ScrollArea::vertical()
            .id_salt("pazar_kartlar")
            .show(ui, |ui| {
                for (kimlik, durum) in &kartlar {
                    let Some(oge) = veri.oge(kimlik) else {
                        continue;
                    };
                    let (t, k) = kart_ciz(ui, oge, *durum, dil, tok);
                    if t {
                        tiklanan = Some(kimlik.clone());
                    }
                    if k {
                        hizli_kur = Some(kimlik.clone());
                    }
                    ui.add_space(tok.bosluk.xs);
                }
            });
        if let Some(k) = tiklanan {
            self.secili = Some(k);
        }
        hizli_kur.map(PazarEylem::Kur)
    }

    /// Haber/makale akışı (salt-okur kartlar; güvenli render + dış bağlantı onayı).
    fn haber_ciz(&mut self, ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) -> Option<PazarEylem> {
        let tr = matches!(dil, Dil::Tr);
        let mut eylem = None;

        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.add(
                egui::TextEdit::singleline(&mut self.haber_sorgu)
                    .hint_text(if tr {
                        "Haberlerde ara…"
                    } else {
                        "Search news…"
                    })
                    .desired_width(260.0),
            );
        });
        ui.add_space(tok.bosluk.xs);

        let veri = self.veri()?;
        let sorgu = self.haber_sorgu.to_lowercase();
        let kartlar: Vec<&HaberKarti> = veri
            .haberler
            .iter()
            .filter(|h| sorgu.split_whitespace().all(|w| h.saman().contains(w)))
            .collect();

        if kartlar.is_empty() {
            ui.label(
                egui::RichText::new(if tr {
                    "Eşleşen haber yok."
                } else {
                    "No matching news."
                })
                .italics()
                .color(tok.renk.metin_soluk),
            );
            return None;
        }

        egui::ScrollArea::vertical()
            .id_salt("pazar_haberler")
            .show(ui, |ui| {
                for h in &kartlar {
                    if let Some(e) = haber_kart_ciz(ui, h, dil, tok) {
                        eylem = Some(e);
                    }
                    ui.add_space(tok.bosluk.s);
                }
            });
        eylem
    }
}

/// Kur/güncelle/kaldır ham eylem ayrımı.
#[derive(Clone, Copy)]
enum KurEylem {
    Kur,
    Guncelle,
    Kaldir,
}

/// Tek bir mağaza kartı.  `(tıklandı, hızlı_kur_tıklandı)` döner.
fn kart_ciz(
    ui: &mut egui::Ui,
    oge: &PazarOgesi,
    durum: KurulumDurum,
    dil: Dil,
    tok: &Tokenlar,
) -> (bool, bool) {
    let tr = matches!(dil, Dil::Tr);
    let mut tiklandi = false;
    let mut hizli_kur = false;
    egui::Frame::none()
        .fill(tok.renk.yuzey)
        .stroke(egui::Stroke::new(1.0, tok.renk.kenarlik))
        .rounding(tok.yaricap)
        .inner_margin(egui::Margin::same(tok.bosluk.m))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() - 110.0);
                    ui.horizontal(|ui| {
                        let baslik = ui.add(
                            egui::Label::new(
                                egui::RichText::new(&oge.ad)
                                    .strong()
                                    .size(15.0)
                                    .color(tok.renk.metin),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if baslik.clicked() {
                            tiklandi = true;
                        }
                        dogrulama::dogrulama_rozeti(ui, oge.dogrulama, dil, tok);
                    });
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · {} · {}",
                            oge.yayinci,
                            oge.tur.etiket(tr),
                            oge.fiyat.etiket(tr),
                        ))
                        .small()
                        .color(tok.renk.metin_soluk),
                    );
                    ui.label(egui::RichText::new(&oge.ozet).color(tok.renk.metin));
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(reviews::yildizlar(oge.puan))
                                .color(tok.renk.uyari)
                                .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.1}", oge.puan))
                                .small()
                                .color(tok.renk.metin_soluk),
                        );
                    });
                });
                // Sağ: kurulum durumu düğmesi + detay.
                ui.vertical(|ui| {
                    match durum {
                        KurulumDurum::KuruluDegil => {
                            if ui.button(if tr { "⬇ Kur" } else { "⬇ Install" }).clicked() {
                                hizli_kur = true;
                            }
                        }
                        KurulumDurum::Kurulu => {
                            ui.label(
                                egui::RichText::new(if tr {
                                    "✓ Kurulu"
                                } else {
                                    "✓ Installed"
                                })
                                .color(tok.renk.basari),
                            );
                        }
                        KurulumDurum::GuncellemeVar => {
                            ui.label(
                                egui::RichText::new(if tr {
                                    "⬆ Güncelleme"
                                } else {
                                    "⬆ Update"
                                })
                                .color(tok.renk.bilgi),
                            );
                        }
                    }
                    if ui
                        .button(if tr { "Detay ›" } else { "Details ›" })
                        .clicked()
                    {
                        tiklandi = true;
                    }
                });
            });
        });
    (tiklandi, hizli_kur)
}

/// Tek bir haber/makale kartı (salt-okur; düz metin; dış bağlantı onayı).
fn haber_kart_ciz(
    ui: &mut egui::Ui,
    h: &HaberKarti,
    dil: Dil,
    tok: &Tokenlar,
) -> Option<PazarEylem> {
    let tr = matches!(dil, Dil::Tr);
    let mut eylem = None;
    egui::Frame::none()
        .fill(tok.renk.yuzey)
        .stroke(egui::Stroke::new(1.0, tok.renk.kenarlik))
        .rounding(tok.yaricap)
        .inner_margin(egui::Margin::same(tok.bosluk.m))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("[{}]", h.tur.etiket(tr)))
                        .small()
                        .color(tok.renk.bilgi),
                );
                ui.label(
                    egui::RichText::new(&h.baslik)
                        .strong()
                        .size(15.0)
                        .color(tok.renk.metin),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    dogrulama::haber_kaynak_rozeti(ui, h.dogrulanmis, dil, tok);
                });
            });
            // Kaynak + tarih (şeffaf).
            ui.label(
                egui::RichText::new(format!("{} · {}", h.kaynak, h.tarih))
                    .small()
                    .color(tok.renk.metin_soluk),
            );
            // Özet — düz metin (HTML YOK).
            ui.label(egui::RichText::new(&h.ozet).color(tok.renk.metin));
            // Dış bağlantı → onay gerektirir (üst katman onaylar).
            if let Some(url) = &h.baglanti {
                ui.horizontal(|ui| {
                    if ui
                        .button(if tr {
                            "🔗 Kaynağı aç"
                        } else {
                            "🔗 Open source"
                        })
                        .on_hover_text(url)
                        .clicked()
                    {
                        eylem = Some(PazarEylem::DisBaglantiAc(url.clone()));
                    }
                    ui.label(
                        egui::RichText::new(if tr {
                            "(dış bağlantı — onay istenir)"
                        } else {
                            "(external link — needs approval)"
                        })
                        .small()
                        .italics()
                        .color(tok.renk.metin_soluk),
                    );
                });
            }
        });
    eylem
}

/// Sol filtre paneli (kategori + tür radyo benzeri seçim).
fn kategori_panel(ui: &mut egui::Ui, suzgec: &mut PazarSuzgec, dil: Dil, tok: &Tokenlar) {
    let tr = matches!(dil, Dil::Tr);
    ui.label(
        egui::RichText::new(if tr { "Kategori" } else { "Category" })
            .strong()
            .color(tok.renk.metin),
    );
    if ui
        .selectable_label(suzgec.kategori.is_none(), if tr { "Tümü" } else { "All" })
        .clicked()
    {
        suzgec.kategori = None;
    }
    for &k in biocraft_net::Kategori::TUMU {
        if ui
            .selectable_label(suzgec.kategori == Some(k), k.etiket(tr))
            .clicked()
        {
            suzgec.kategori = if suzgec.kategori == Some(k) {
                None
            } else {
                Some(k)
            };
        }
    }
    ui.add_space(tok.bosluk.s);
    ui.label(
        egui::RichText::new(if tr { "Tür" } else { "Type" })
            .strong()
            .color(tok.renk.metin),
    );
    if ui
        .selectable_label(suzgec.tur.is_none(), if tr { "Tümü" } else { "All" })
        .clicked()
    {
        suzgec.tur = None;
    }
    for &t in biocraft_net::OgeTuru::TUMU {
        if ui
            .selectable_label(suzgec.tur == Some(t), t.etiket(tr))
            .clicked()
        {
            suzgec.tur = if suzgec.tur == Some(t) { None } else { Some(t) };
        }
    }
}

/// Yükleniyor iskeleti (TDA madde 6).
fn yukleniyor_iskelet(ui: &mut egui::Ui, tok: &Tokenlar) {
    for _ in 0..3 {
        egui::Frame::none()
            .fill(tok.renk.yuzey)
            .stroke(egui::Stroke::new(1.0, tok.renk.kenarlik))
            .rounding(tok.yaricap)
            .inner_margin(egui::Margin::same(tok.bosluk.m))
            .show(ui, |ui| {
                Skeleton::paragraf(ui, tok, 2);
            });
        ui.add_space(tok.bosluk.s);
    }
}

/// Hata + "tekrar dene" (TDA madde 4).
fn hata_goster(
    ui: &mut egui::Ui,
    ne_oldu: String,
    nasil: String,
    tr: bool,
    tok: &Tokenlar,
    dis_eylem: &mut Option<PazarEylem>,
) {
    egui::Frame::none()
        .fill(tok.renk.hata_zemin)
        .rounding(tok.yaricap)
        .inner_margin(egui::Margin::same(tok.bosluk.m))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(ne_oldu).strong().color(tok.renk.hata));
            ui.label(egui::RichText::new(nasil).color(tok.renk.metin));
            if ui
                .button(if tr { "Tekrar Dene" } else { "Retry" })
                .clicked()
            {
                *dis_eylem = Some(PazarEylem::Yenile);
            }
        });
}
