//! **App-tarafı auto-updater + gömülü çekirdek eklenti** (İP-20, MK-56, MK-19).
//!
//! Saf güncelleme *motoru* (imza/delta/atomik/geri-alma) bir alt katmandadır
//! ([`biocraft_data::update`], L2); bu modül onun üstüne **ince** bir orkestrasyon koyar:
//!
//! - **Kaynak soyutlaması** ([`GuncellemeKaynagi`]): arka planda (ayrı thread) "yeni sürüm var mı?"
//!   sorusunu sorar.  MVP'de **yapılandırılmamıştır** ([`KaynakYok`]) — gerçek HTTPS/TLS indirme
//!   insan-eli/altyapı işidir (bkz. `Hukuk-ve-Operasyon.md`); `--update-demo` ile sahte bir kaynak
//!   yüzeyi canlı gösterir.
//! - **Denetleyici** ([`GuncellemeDenetleyici`]): durum makinesi + arka plan yoklaması (kare
//!   bütçesini bloklamadan; MK-07) + onay akışı ("şimdi/sonra" + changelog).  Asıl iş **kullanıcı
//!   onaylayınca** yapılır → aktif iş bölünmez (atomik güvenli-an spec'i).
//! - **Gömülü çekirdek eklenti** (MK-19): BioCraft Studio motorla **aynı pakette** gelir; ilk
//!   açılışta kurulu → "eklenti yok" ekranı **asla** görünmez; **bağımsız sürümlenir**.
//!
//! Mimari karar: Spec (İP-20) saf delta/rollback'i `biocraft-app/src/update/`'e koyar; ancak imzalı
//! güncelleme **doğrulaması** İP-09'da `biocraft-data::security::integrity` (L2)'dedir ve proje
//! deseni saf+test-edilebilir mantığı kütüphane katmanına koyar (Gün 26 kararıyla aynı çizgi:
//! ARCHITECTURE > docs/specs).  Bu yüzden delta/atomik/rollback **L2**'de (`biocraft_data::update`),
//! app yalnız ince updater+UI taşır.

use std::sync::mpsc::{self, Receiver};
use std::thread;

use biocraft_data::update::{surum_yonu, GenisBildirim, SurumKanali, SurumYon};
use biocraft_ui::{Dil, Tokenlar};

// ─────────────────────────────────────────────────────────────────────────────
// Gömülü çekirdek eklenti (MK-19)
// ─────────────────────────────────────────────────────────────────────────────

/// Gömülü çekirdek eklentinin kimliği (pazar/host'ta "Kurulu" görünür; sihirbaz/onboarding ile aynı).
pub const CEKIRDEK_EKLENTI_KIMLIK: &str = "biocraft.studio.core";
/// Gömülü çekirdek eklentinin görünen adı.
pub const CEKIRDEK_EKLENTI_AD: &str = "BioCraft Studio";
/// Çekirdek eklenti sürümü — **motor sürümünden bağımsız** ilerler (MK-19: bağımsız sürümlenir).
pub const CEKIRDEK_EKLENTI_SURUM: &str = "0.1.0";

/// Motorla aynı pakette gelen, ilk açılışta kurulu olan çekirdek eklenti tanımı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GomuluEklenti {
    /// `biocraft.<yayinci>.<eklenti>` kimliği.
    pub kimlik: &'static str,
    /// Görünen ad.
    pub ad: &'static str,
    /// Eklenti sürümü (motordan bağımsız).
    pub surum: &'static str,
}

/// Pakete gömülü çekirdek eklentiyi döndürür (MK-19).
pub fn cekirdek_eklenti() -> GomuluEklenti {
    GomuluEklenti {
        kimlik: CEKIRDEK_EKLENTI_KIMLIK,
        ad: CEKIRDEK_EKLENTI_AD,
        surum: CEKIRDEK_EKLENTI_SURUM,
    }
}

/// İlk açılışta "eklenti yok" ekranı gösterilmeli mi?  Çekirdek eklenti **gömülü** geldiği için
/// **asla** (MK-19) — bu fonksiyon her zaman `false` döner ve kabul kriterini açıkça belgeler/test eder.
pub fn eklenti_yok_ekrani_gerekli() -> bool {
    false
}

/// Açılışta loglanan tek satırlık updater hazırlık özeti (kod okuyamayan sahip için şeffaflık):
/// çekirdek eklenti gömülü mü, "eklenti yok" ekranı devre dışı mı, resmi anahtar bağlı mı.
pub fn updater_ozeti() -> String {
    let e = cekirdek_eklenti();
    format!(
        "Auto-update hazır (İP-20): kanal=Kararlı · çekirdek eklenti gömülü={} v{} · 'eklenti yok' \
         ekranı={} · resmi yayın anahtarı={} bayt",
        e.ad,
        e.surum,
        if eklenti_yok_ekrani_gerekli() {
            "GÖSTERİLİR"
        } else {
            "yok (MK-19)"
        },
        RESMI_YAYIN_ANAHTARI.len(),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Resmi yayın anahtarı + mevcut sürüm
// ─────────────────────────────────────────────────────────────────────────────

/// Çekirdeğe **gömülü** resmi yayın doğrulama anahtarı (Ed25519, 32 bayt).
///
/// **İNSAN-ELİ:** Buradaki değer bir **yer tutucudur**; gerçek dağıtımda BioCraft'ın tüzel
/// kişiliğine ait kod-imzalama/yayın anahtarının **açık** kısmıyla değiştirilir (özel anahtar asla
/// repoda/kodda tutulmaz — `Hukuk-ve-Operasyon.md`).  İmza zinciri kurulana dek auto-update üretimde
/// **kapalıdır** ([`KaynakYok`]); yer tutucu yalnız tip/derleme içindir.
pub const RESMI_YAYIN_ANAHTARI: [u8; 32] = [0u8; 32];

/// Çalışan motorun sürümü (Cargo paket sürümünden — SemVer, tek kaynak).
pub fn mevcut_surum() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ─────────────────────────────────────────────────────────────────────────────
// Kaynak soyutlaması (arka plan "yeni sürüm var mı?")
// ─────────────────────────────────────────────────────────────────────────────

/// Bir güncelleme adayı: imzalı geniş bildirim (+ imza).  Gerçek indirme/uygulama üst akışta.
#[derive(Debug, Clone)]
pub struct GuncellemeAdayi {
    /// İmzalı geniş bildirim (sürüm + changelog + kanal + nihai paket özeti).
    pub bildirim: GenisBildirim,
    /// `bildirim.kanonik()` üzerine Ed25519 imzası (64 bayt).
    pub imza: Vec<u8>,
}

/// Arka planda güncelleme kontrol eden kaynak (gerçek HTTPS/TLS sağlayıcı eklenti/altyapı işidir).
pub trait GuncellemeKaynagi: Send {
    /// `mevcut` sürüme göre, `kanal`'da görülebilir **daha yeni** bir sürüm varsa onu döndürür.
    fn kontrol(&self, mevcut: &str, kanal: SurumKanali) -> Result<Option<GuncellemeAdayi>, String>;
}

/// MVP varsayılanı: **kaynak yapılandırılmadı** → güncelleme sunulmaz (çevrimdışı/güvenli).
pub struct KaynakYok;

impl GuncellemeKaynagi for KaynakYok {
    fn kontrol(
        &self,
        _mevcut: &str,
        _kanal: SurumKanali,
    ) -> Result<Option<GuncellemeAdayi>, String> {
        Ok(None)
    }
}

/// `--update-demo`: yüzeyi canlı göstermek için **sahte** bir "yeni sürüm var" üretir.
///
/// Gerçek indirme/uygulama yapmaz; yalnız mevcut sürümün üstüne bir minor bump + örnek changelog'u
/// imzalı bir adaya çevirir.  İmza, demo tohum anahtarıyla atılır (üretim anahtarı değil).
pub struct DemoKaynak;

impl GuncellemeKaynagi for DemoKaynak {
    fn kontrol(&self, mevcut: &str, kanal: SurumKanali) -> Result<Option<GuncellemeAdayi>, String> {
        // Mevcut sürümün minor'ını artır (ör. 0.1.0 → 0.2.0) — her zaman daha yeni.
        let aday_surum = bir_minor_artir(mevcut);
        let changelog = "- Delta auto-update (yalnız değişen parça)\n\
                         - Atomik + geri alınabilir güncelleme\n\
                         - Çekirdek eklenti pakete gömülü";
        let (bildirim, imza, _vk) = biocraft_data::update::demo_imzala(
            b"BioCraft Engine demo paketi",
            &aday_surum,
            SurumKanali::Kararli,
            changelog,
            [42u8; 32],
        );
        // Kanal görünürlüğüne saygı (kararlı yayını herkes görür).
        if !kanal.gorur_mu(bildirim.kanal) {
            return Ok(None);
        }
        Ok(Some(GuncellemeAdayi { bildirim, imza }))
    }
}

/// "a.b.c" → "a.(b+1).0" (ayrıştırılamazsa güvenli bir aday döndürür).
fn bir_minor_artir(surum: &str) -> String {
    let p: Vec<&str> = surum.split('.').collect();
    let major = p.first().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    let minor = p.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    format!("{}.{}.0", major, minor + 1)
}

// ─────────────────────────────────────────────────────────────────────────────
// Denetleyici durum makinesi
// ─────────────────────────────────────────────────────────────────────────────

/// Kullanıcıya gösterilecek güncelleme bilgisi (onay ekranı).
#[derive(Debug, Clone)]
pub struct GuncellemeBilgi {
    /// Yeni sürüm.
    pub surum: String,
    /// "Yenilikler" (changelog) metni — imzalı (kurcalanamaz).
    pub changelog: String,
    /// Yön (yükseltme/aynı/downgrade) — bilgi amaçlı.
    pub yon: SurumYon,
    /// Yayın kanalı.
    pub kanal: SurumKanali,
}

/// Updater durumu (UI bunu okur).
#[derive(Debug, Clone)]
pub enum GuncellemeDurum {
    /// Henüz kontrol edilmedi ya da kaynak yok.
    Bos,
    /// Arka planda kontrol ediliyor.
    Kontrol,
    /// Güncel — yeni sürüm yok.
    Guncel,
    /// Yeni sürüm mevcut — kullanıcı onayı bekliyor.
    Mevcut(GuncellemeBilgi),
    /// Kullanıcı "sonra" dedi — bu oturumda tekrar sorulmaz.
    Ertelendi,
    /// Kontrol/uygulama hatası (sessiz; UI/loga küçük not).
    Hata(String),
}

impl GuncellemeDurum {
    /// Onay ekranı şu an gösterilmeli mi? (Yalnız `Mevcut` durumunda.)
    pub fn onay_gosterilsin_mi(&self) -> bool {
        matches!(self, GuncellemeDurum::Mevcut(_))
    }

    /// Hata durumundaki mesaj (varsa) — app sessizce loglar (auto-update kullanıcıyı rahatsız etmez).
    pub fn hata(&self) -> Option<&str> {
        match self {
            GuncellemeDurum::Hata(m) => Some(m.as_str()),
            _ => None,
        }
    }
}

/// Overlay'in ürettiği kullanıcı eylemi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuncellemeEylem {
    /// "Şimdi yeniden başlat" — güncellemeyi uygula + yeniden başlat (üst akış işler).
    SimdiUygula,
    /// "Sonra" — bu oturumda erteler.
    Sonra,
    /// Changelog için dış bağlantı (tüm sürüm notları) — açmadan önce onay (üst akış).
    TumNotlar,
}

/// Auto-update orkestrasyonu: arka plan kontrolü + durum makinesi + onay akışı.
pub struct GuncellemeDenetleyici {
    durum: GuncellemeDurum,
    kanal: SurumKanali,
    alici: Option<Receiver<Result<Option<GuncellemeAdayi>, String>>>,
    /// Son seçilen aday (uygulama akışına taşınır).
    aday: Option<GuncellemeAdayi>,
}

impl GuncellemeDenetleyici {
    /// Belirli bir kanal için boş denetleyici.
    pub fn yeni(kanal: SurumKanali) -> Self {
        Self {
            durum: GuncellemeDurum::Bos,
            kanal,
            alici: None,
            aday: None,
        }
    }

    /// Güncel durum (UI okur).
    pub fn durum(&self) -> &GuncellemeDurum {
        &self.durum
    }

    /// Arka planda kontrolü başlatır (ayrı thread; UI bloklanmaz).  Kaynak `Box<dyn ...>` olarak
    /// enjekte edilir → test/demo/gerçek aynı yolla takılır.
    pub fn kontrol_baslat(&mut self, kaynak: Box<dyn GuncellemeKaynagi>) {
        let (gonder, al) = mpsc::channel();
        let mevcut = mevcut_surum().to_string();
        let kanal = self.kanal;
        thread::spawn(move || {
            let sonuc = kaynak.kontrol(&mevcut, kanal);
            // Alıcı kapanmış olabilir (uygulama kapandı) — hatayı yut.
            let _ = gonder.send(sonuc);
        });
        self.alici = Some(al);
        self.durum = GuncellemeDurum::Kontrol;
    }

    /// Arka plan sonucunu **bloklamadan** yoklar (her kare çağrılır).  Durum değiştiyse `true`.
    pub fn yokla(&mut self) -> bool {
        let Some(al) = &self.alici else {
            return false;
        };
        match al.try_recv() {
            Ok(sonuc) => {
                self.alici = None;
                match sonuc {
                    Ok(Some(aday)) => {
                        let bilgi = self.adaydan_bilgi(&aday);
                        self.aday = Some(aday);
                        self.durum = match bilgi {
                            Ok(b) => GuncellemeDurum::Mevcut(b),
                            Err(e) => GuncellemeDurum::Hata(e),
                        };
                    }
                    Ok(None) => self.durum = GuncellemeDurum::Guncel,
                    Err(e) => self.durum = GuncellemeDurum::Hata(e),
                }
                true
            }
            Err(mpsc::TryRecvError::Empty) => false,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.alici = None;
                self.durum = GuncellemeDurum::Hata("kontrol iş parçacığı yanıt vermedi".into());
                true
            }
        }
    }

    fn adaydan_bilgi(&self, aday: &GuncellemeAdayi) -> Result<GuncellemeBilgi, String> {
        let yon =
            surum_yonu(mevcut_surum(), aday.bildirim.surum()).map_err(|e| e.ne_oldu.clone())?;
        Ok(GuncellemeBilgi {
            surum: aday.bildirim.surum().to_string(),
            changelog: aday.bildirim.changelog.clone(),
            yon,
            kanal: aday.bildirim.kanal,
        })
    }

    /// Kullanıcı "sonra" dedi: onay ekranını kapat, bu oturumda tekrar gösterme.
    pub fn ertele(&mut self) {
        if matches!(self.durum, GuncellemeDurum::Mevcut(_)) {
            self.durum = GuncellemeDurum::Ertelendi;
        }
    }

    /// Uygulama akışına taşınacak seçili aday (imza + bildirim).
    pub fn secili_aday(&self) -> Option<&GuncellemeAdayi> {
        self.aday.as_ref()
    }

    /// Onay overlay'ini çizer (yalnız `Mevcut` durumunda).  Aktif işi **bölmez** — küçük modal;
    /// asıl uygulama yalnız buton tıklanınca üst akışta yapılır.
    pub fn overlay_ciz(
        &mut self,
        ctx: &egui::Context,
        dil: Dil,
        tok: &Tokenlar,
    ) -> Option<GuncellemeEylem> {
        if !self.durum.onay_gosterilsin_mi() {
            return None;
        }
        let GuncellemeDurum::Mevcut(bilgi) = &self.durum else {
            return None;
        };
        let bilgi = bilgi.clone();
        let tr = matches!(dil, Dil::Tr);
        let mut eylem: Option<GuncellemeEylem> = None;
        // Downgrade ve kanal rozetleri (yön/kanal alanları burada okunur).
        let downgrade = matches!(bilgi.yon, SurumYon::Indirme);
        let kanal_rozeti = if bilgi.kanal != SurumKanali::Kararli {
            format!("[{}] ", bilgi.kanal.etiket(tr))
        } else {
            String::new()
        };

        egui::Window::new("guncelleme_onay")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
            .frame(egui::Frame {
                fill: tok.renk.yuzey,
                stroke: egui::Stroke::new(1.0, tok.renk.kenarlik),
                rounding: egui::Rounding::same(tok.yaricap),
                inner_margin: egui::Margin::same(tok.bosluk.l),
                shadow: tok.golge_modal(),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.set_max_width(380.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(if tr {
                            "✨ Güncelleme hazır"
                        } else {
                            "✨ Update ready"
                        })
                        .strong()
                        .color(tok.renk.vurgu),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("v{}", bilgi.surum))
                                .small()
                                .color(tok.renk.metin_soluk),
                        );
                    });
                });
                ui.add_space(tok.bosluk.s);

                ui.label(
                    egui::RichText::new(if tr {
                        format!("{}Sürüm {} yüklenmeye hazır.", kanal_rozeti, bilgi.surum)
                    } else {
                        format!(
                            "{}Version {} is ready to install.",
                            kanal_rozeti, bilgi.surum
                        )
                    })
                    .color(tok.renk.metin),
                );
                // Downgrade (sürüm düşürme) güvenlik ağı — açıkça işaretlenir.
                if downgrade {
                    ui.label(
                        egui::RichText::new(if tr {
                            "⚠ Bu bir sürüm düşürmedir (downgrade)."
                        } else {
                            "⚠ This is a downgrade."
                        })
                        .small()
                        .color(tok.renk.metin_soluk),
                    );
                }

                // "Yenilikler" (changelog) — imzalı metin (kurcalanamaz).
                if !bilgi.changelog.trim().is_empty() {
                    ui.add_space(tok.bosluk.s);
                    egui::Frame {
                        fill: tok.renk.yuzey_alt,
                        rounding: egui::Rounding::same(tok.yaricap),
                        inner_margin: egui::Margin::same(tok.bosluk.m),
                        ..Default::default()
                    }
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(if tr { "Yenilikler" } else { "What's new" })
                                .small()
                                .strong()
                                .color(tok.renk.metin_soluk),
                        );
                        ui.label(egui::RichText::new(bilgi.changelog.trim()).color(tok.renk.metin));
                    });
                }

                ui.add_space(tok.bosluk.m);
                ui.horizontal(|ui| {
                    if ui
                        .button(if tr {
                            "Şimdi yeniden başlat"
                        } else {
                            "Restart now"
                        })
                        .clicked()
                    {
                        eylem = Some(GuncellemeEylem::SimdiUygula);
                    }
                    if ui.button(if tr { "Sonra" } else { "Later" }).clicked() {
                        eylem = Some(GuncellemeEylem::Sonra);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .link(if tr {
                                "Tüm notlar →"
                            } else {
                                "All notes →"
                            })
                            .clicked()
                        {
                            eylem = Some(GuncellemeEylem::TumNotlar);
                        }
                    });
                });
            });

        // Eylemi durum makinesine uygula (Sonra → ertele).
        if matches!(eylem, Some(GuncellemeEylem::Sonra)) {
            self.ertele();
        }
        eylem
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test kaynağı: sabit bir adayı döndürür (kanal görünürlüğüne saygı duyar).
    struct SabitKaynak {
        aday: Option<GuncellemeAdayi>,
    }
    impl GuncellemeKaynagi for SabitKaynak {
        fn kontrol(
            &self,
            _mevcut: &str,
            kanal: SurumKanali,
        ) -> Result<Option<GuncellemeAdayi>, String> {
            Ok(self
                .aday
                .clone()
                .filter(|a| kanal.gorur_mu(a.bildirim.kanal)))
        }
    }

    fn imzali_aday(surum: &str, kanal: SurumKanali, changelog: &str) -> GuncellemeAdayi {
        let (bildirim, imza, _vk) =
            biocraft_data::update::demo_imzala(b"paket", surum, kanal, changelog, [3u8; 32]);
        GuncellemeAdayi { bildirim, imza }
    }

    /// `yokla`'yı sonuç gelene dek döndürür (test yardımcı).
    fn yokla_bitene_dek(d: &mut GuncellemeDenetleyici) {
        for _ in 0..1000 {
            if d.yokla() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        panic!("kontrol zamanında tamamlanmadı");
    }

    #[test]
    fn gomulu_cekirdek_eklenti_hazir() {
        let e = cekirdek_eklenti();
        assert_eq!(e.kimlik, "biocraft.studio.core");
        // MK-19: gömülü geldiği için "eklenti yok" ekranı asla görünmez.
        assert!(!eklenti_yok_ekrani_gerekli());
        // Bağımsız sürümlenir — geçerli SemVer.
        assert!(e
            .surum
            .parse::<biocraft_data::biocraft_types::Version>()
            .is_ok());
    }

    #[test]
    fn kaynak_yok_guncel_doner() {
        let mut d = GuncellemeDenetleyici::yeni(SurumKanali::Kararli);
        d.kontrol_baslat(Box::new(KaynakYok));
        yokla_bitene_dek(&mut d);
        assert!(matches!(d.durum(), GuncellemeDurum::Guncel));
        assert!(!d.durum().onay_gosterilsin_mi());
    }

    #[test]
    fn yeni_surum_onay_acar() {
        let mut d = GuncellemeDenetleyici::yeni(SurumKanali::Kararli);
        d.kontrol_baslat(Box::new(SabitKaynak {
            aday: Some(imzali_aday(
                "9.9.9",
                SurumKanali::Kararli,
                "- Harika yenilik",
            )),
        }));
        yokla_bitene_dek(&mut d);
        match d.durum() {
            GuncellemeDurum::Mevcut(b) => {
                assert_eq!(b.surum, "9.9.9");
                assert_eq!(b.yon, SurumYon::Yukseltme);
                assert!(b.changelog.contains("Harika"));
            }
            other => panic!("beklenen Mevcut, gelen {other:?}"),
        }
        assert!(d.secili_aday().is_some());
    }

    #[test]
    fn kararli_kullanici_beta_gormez() {
        let mut d = GuncellemeDenetleyici::yeni(SurumKanali::Kararli);
        d.kontrol_baslat(Box::new(SabitKaynak {
            aday: Some(imzali_aday("9.9.9", SurumKanali::Beta, "beta")),
        }));
        yokla_bitene_dek(&mut d);
        // Kanal görünmüyor → kaynak None döndürür → Guncel.
        assert!(matches!(d.durum(), GuncellemeDurum::Guncel));
    }

    #[test]
    fn ertele_onayi_kapatir() {
        let mut d = GuncellemeDenetleyici::yeni(SurumKanali::Kararli);
        d.kontrol_baslat(Box::new(SabitKaynak {
            aday: Some(imzali_aday("9.9.9", SurumKanali::Kararli, "x")),
        }));
        yokla_bitene_dek(&mut d);
        assert!(d.durum().onay_gosterilsin_mi());
        d.ertele();
        assert!(!d.durum().onay_gosterilsin_mi());
        assert!(matches!(d.durum(), GuncellemeDurum::Ertelendi));
    }

    #[test]
    fn mevcut_surum_gecerli_semver() {
        assert!(mevcut_surum()
            .parse::<biocraft_data::biocraft_types::Version>()
            .is_ok());
    }
}
