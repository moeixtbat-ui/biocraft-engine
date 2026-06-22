//! İP-14 / YZ-01 — **AI yüzeyi (kullanıcı arayüzü tarafı).**
//!
//! Sağlayıcı-bağımsız sözleşme [`biocraft_ai_surface`]'tedir (L3, saf mantık).  Burası (L4) onun
//! **görünen yüzü**dür: durum ([`AiYuzey`]) + panel çizimi ([`panel`]) + maliyet rozeti
//! ([`cost_badge`]).  Token (MK-52) ve i18n (MK-53) tek kaynağı UI'de olduğundan AI'ın egui'si
//! burada yaşar.
//!
//! **Dürüstlük (MK-48):** Gerçek uygulamada hiçbir motor kaydolmaz → panel "yapılandırılmadı"
//! gösterir.  Demo/örnek akışlarında [`biocraft_ai_surface::EchoSaglayici`] kaydolur → pipeline
//! uçtan uca çalışır (sahte zekâ yok; echo açıkça "gerçek AI değil" etiketli).
//!
//! **Akıcılık (MK-48/MK-07):** Üretim arka plan thread'inde çalışır; arayüz her kare kanalı
//! `yokla()` ile bloklamadan okur → 60 FPS korunur, "Durdur" anında etki eder.
// MK-46/47/48/49: sözleşme + dürüst çıktı + asenkron + klinik değil.

pub mod cost_badge;
pub mod panel;

#[cfg(test)]
mod tests;

use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Arc;
use std::thread;

use biocraft_ai_surface::{
    baglam_denetle, AiBaglam, AiCikti, AkisOlay, BaglamOgesi, CagriSonucu, DenetimGirdisi,
    DenetimKaydi, GuardKarari, IptalBayragi, Kota, Maliyet, MaliyetSayaci, Provider,
    SaglayiciKayit, SohbetMesaji,
};
use biocraft_types::ErrorReport;

pub use cost_badge::maliyet_rozeti_ciz;
pub use panel::ai_panel_ciz;

/// Panelin ürettiği, app'in karşılaması gereken eylem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiPanelEylem {
    /// "AI sağlayıcı ekle" — app ayarların AI kategorisini açar (gerçek motor İP-14 sonrası).
    SaglayiciEkle,
    /// "AI'ı aç" — kapalıyken; app `ai.etkin` ayarını açar.
    AiAc,
    /// Bir eylem önerisi kullanıcı onayıyla uygulanmak isteniyor (indeks = son çıktıdaki sıra).
    EylemUygula(usize),
}

/// Akış hâlindeki yanıtın geçici durumu (arka plan thread'i + kanal + iptal).
struct AkanYanit {
    rx: Receiver<AkisOlay>,
    iptal: IptalBayragi,
    saglayici: Arc<dyn Provider>,
    baglam: AiBaglam,
    /// Şimdiye dek biriken kısmi metin (akışla dolar).
    kismi: String,
}

/// **AI yüzeyinin durumu.**  App bunu `Sahne`'de tutar; ayarlardan `etkin`/gösterge bayraklarını
/// senkronlar ve her kare [`AiYuzey::yokla`] çağırır.
pub struct AiYuzey {
    /// AI etkin mi? (Ayar `ai.etkin` ile senkron.)  Kapalıyken panel sadeleşir, app tam çalışır.
    pub etkin: bool,
    /// Token sayacı gösterilsin mi? (Ayar `ai.token_sayaci_goster`.)
    pub token_goster: bool,
    /// Maliyet göstergesi gösterilsin mi? (Ayar `ai.maliyet_goster`.)
    pub maliyet_goster: bool,
    /// Sağlayıcı kayıt defteri (MVP'de gerçek uygulamada boş → "yapılandırılmadı").
    pub kayit: SaglayiciKayit,
    /// Oturum maliyet sayacı.
    pub sayac: MaliyetSayaci,
    /// Kota/limit (MVP'de varsayılan sınırsız).
    pub kota: Kota,
    /// Denetim kaydı (şeffaflık; PII'siz).
    pub denetim: DenetimKaydi,
    /// Konuşma geçmişi (proje bağlamlı; gerçek kalıcılık motorla).
    pub sohbet: Vec<SohbetMesaji>,
    /// Girdi kutusu tamponu.
    pub girdi: String,
    /// Bir sonraki sorguya iliştirilecek bağlam öğeleri (seçili veri — PHI denetimi buna bakar).
    pub bekleyen_ogeler: Vec<BaglamOgesi>,
    /// Son tamamlanan zengin çıktı (kaynak/güven/doğrulama/maliyet gösterimi için).
    pub son_cikti: Option<AiCikti>,
    /// Son hata (çıkış kapısı engeli ya da sağlayıcı hatası).
    pub son_hata: Option<ErrorReport>,
    /// App'in karşılaması gereken bekleyen panel eylemi.
    pub bekleyen_eylem: Option<AiPanelEylem>,
    /// Akış hâlindeki yanıt (varsa).
    akan: Option<AkanYanit>,
}

impl Default for AiYuzey {
    fn default() -> Self {
        Self::yeni()
    }
}

impl AiYuzey {
    /// Yeni, boş AI yüzeyi (kapalı, sağlayıcısız → "yapılandırılmadı").
    pub fn yeni() -> Self {
        Self {
            etkin: false,
            token_goster: false,
            maliyet_goster: false,
            kayit: SaglayiciKayit::yeni(),
            sayac: MaliyetSayaci::yeni(),
            kota: Kota::default(),
            denetim: DenetimKaydi::yeni(),
            sohbet: Vec::new(),
            girdi: String::new(),
            bekleyen_ogeler: Vec::new(),
            son_cikti: None,
            son_hata: None,
            bekleyen_eylem: None,
            akan: None,
        }
    }

    /// Demo/örnek akışı için bir sağlayıcı kaydeder (gerçek uygulamada çağrılmaz — MK-48).
    pub fn saglayici_ekle(&mut self, saglayici: Arc<dyn Provider>) {
        self.kayit.kaydet(saglayici);
    }

    /// Şu an bir yanıt akıyor mu? (UI "Durdur" gösterir, girdi pasifleşir.)
    pub fn mesgul(&self) -> bool {
        self.akan.is_some()
    }

    /// Yapılandırıldı mı (en az bir sağlayıcı var)?
    pub fn yapilandirildi_mi(&self) -> bool {
        !self.kayit.bos_mu()
    }

    /// Girdi kutusundaki sorguyu gönderir.  Çıkış kapısı (PHI) denetiminden geçer; geçerse arka
    /// plan thread'inde akış başlatır.  AI kapalıysa / sağlayıcı yoksa / meşgulse / boşsa hiçbir
    /// şey yapmaz.
    pub fn gonder(&mut self) {
        if !self.etkin || self.mesgul() {
            return;
        }
        let sorgu = self.girdi.trim().to_string();
        if sorgu.is_empty() {
            return;
        }
        let Some(saglayici) = self.kayit.secili() else {
            return;
        };

        // Bağlamı kur (sorgu + geçmiş + seçili veri öğeleri).
        let mut baglam = AiBaglam::sorgudan(&sorgu);
        baglam.gecmis = self.sohbet.clone();
        baglam.ogeler = self.bekleyen_ogeler.clone();

        // ÇIKIŞ KAPISI (YZ-08/MK-42/43): dış sağlayıcıya PHI gidemez — gönderimden ÖNCE denetle.
        if let GuardKarari::Engellendi { hata, .. } =
            baglam_denetle(&baglam, saglayici.kimlik().tur)
        {
            self.denetim.kaydet(DenetimGirdisi::baglamdan(
                &baglam,
                saglayici.kimlik(),
                0,
                CagriSonucu::Engellendi,
            ));
            self.son_hata = Some(*hata);
            return;
        }

        // Kullanıcı mesajını geçmişe ekle, girdiyi temizle.
        self.sohbet.push(SohbetMesaji::kullanici(&sorgu));
        self.girdi.clear();
        self.son_hata = None;
        self.son_cikti = None;

        // Arka plan thread'inde akış başlat (UI donmaz — MK-48/MK-07).
        let iptal = IptalBayragi::yeni();
        let (tx, rx) = std::sync::mpsc::channel();
        let saglayici_th = Arc::clone(&saglayici);
        let baglam_th = baglam.clone();
        let iptal_th = iptal.clone();
        thread::spawn(move || {
            saglayici_th.akis(&baglam_th, &iptal_th, &mut |olay| {
                let _ = tx.send(olay);
            });
        });

        self.akan = Some(AkanYanit {
            rx,
            iptal,
            saglayici,
            baglam,
            kismi: String::new(),
        });
    }

    /// Akan yanıtı durdurur ("Durdur" butonu).  İptal bayrağını işaretler; sağlayıcı bir sonraki
    /// parçada durur ([`AkisOlay::Durduruldu`] gelir → [`AiYuzey::yokla`] sonlandırır).
    pub fn durdur(&mut self) {
        if let Some(a) = &self.akan {
            a.iptal.iptal_et();
        }
    }

    /// **Her kare çağrılır** — kanaldan biriken akış olaylarını bloklamadan işler (MK-07).
    /// Yeni içerik geldiyse `true` döner (UI yeniden çizim ister).
    pub fn yokla(&mut self) -> bool {
        if self.akan.is_none() {
            return false;
        }
        let mut degisti = false;
        // Bu kareye sığacak kadar olayı boşalt (kare bütçesi — MK-07); döngü kanal boşalınca biter.
        loop {
            let olay = match self.akan.as_ref().unwrap().rx.try_recv() {
                Ok(o) => o,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // Thread kapandı ama Tamamlandı/Durduruldu gelmediyse akışı temizle.
                    self.akan = None;
                    degisti = true;
                    break;
                }
            };
            degisti = true;
            match olay {
                AkisOlay::Parca(p) => {
                    if let Some(a) = self.akan.as_mut() {
                        a.kismi.push_str(&p);
                    }
                }
                AkisOlay::Tamamlandi(cikti) => {
                    self.akisi_sonlandir(*cikti);
                    break;
                }
                AkisOlay::Durduruldu => {
                    self.akisi_durdur();
                    break;
                }
                AkisOlay::Hata(e) => {
                    self.akisi_hata(*e);
                    break;
                }
            }
        }
        degisti
    }

    /// Akış başarıyla tamamlandı: çıktıyı sakla, geçmişe ekle, maliyet/denetim işle.
    fn akisi_sonlandir(&mut self, cikti: AiCikti) {
        let Some(akan) = self.akan.take() else { return };
        let maliyet = akan.saglayici.maliyet(&cikti.kullanim);
        self.sayac.ekle(maliyet.clone());
        self.denetim.kaydet(DenetimGirdisi::baglamdan(
            &akan.baglam,
            akan.saglayici.kimlik(),
            cikti.kullanim.toplam(),
            CagriSonucu::Tamam,
        ));
        self.sohbet.push(SohbetMesaji::asistan(&cikti.metin));
        self.son_cikti = Some(cikti);
        let _ = maliyet;
    }

    /// Kullanıcı durdurdu: kısmi metni geçmişe (varsa) ekle, denetime "durduruldu" yaz.
    fn akisi_durdur(&mut self) {
        let Some(akan) = self.akan.take() else { return };
        if !akan.kismi.trim().is_empty() {
            self.sohbet.push(SohbetMesaji::asistan(format!(
                "{} (durduruldu)",
                akan.kismi.trim()
            )));
        }
        self.denetim.kaydet(DenetimGirdisi::baglamdan(
            &akan.baglam,
            akan.saglayici.kimlik(),
            0,
            CagriSonucu::Durduruldu,
        ));
    }

    /// Sağlayıcı hatası: hatayı sakla, denetime "hata" yaz.
    fn akisi_hata(&mut self, hata: ErrorReport) {
        let Some(akan) = self.akan.take() else { return };
        self.denetim.kaydet(DenetimGirdisi::baglamdan(
            &akan.baglam,
            akan.saglayici.kimlik(),
            0,
            CagriSonucu::Hata,
        ));
        self.son_hata = Some(hata);
    }

    /// Şu ana dek akan kısmi metin (UI canlı gösterir).  Akış yoksa boş.
    pub fn kismi_metin(&self) -> &str {
        self.akan.as_ref().map(|a| a.kismi.as_str()).unwrap_or("")
    }

    /// Oturumu/konuşmayı sıfırlar (yeni sohbet).
    pub fn sohbeti_temizle(&mut self) {
        self.sohbet.clear();
        self.son_cikti = None;
        self.son_hata = None;
    }

    /// Durum çubuğu token göstergesi için oturum jeton sayısı (gösterge kapalıysa `None`).
    pub fn durum_token(&self) -> Option<u64> {
        if self.etkin && self.token_goster {
            Some(self.sayac.oturum_jeton)
        } else {
            None
        }
    }
}

/// Son maliyetin kısa gösterimi (rozet için yardımcı — boşsa `None`).
pub fn son_maliyet_etiketi(m: &Option<Maliyet>, tr: bool) -> Option<String> {
    m.as_ref().map(|m| m.goster(tr))
}
