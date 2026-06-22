//! Onboarding — ilk kullanıcı deneyimi (İP-17).
//!
//! İlk açılışta **atlanabilir** bir "Rolün?" sorusu (K1) + **atlanabilir** hoş geldin turu;
//! zengin **proje şablonları** (ilgili panelleri/örnek akışı ön-kuran) + **gömülü demo veri**
//! ("Demo Projeyi Aç" → kullanıcı boş ekranla kalmaz) + bağlamsal **ipuçları** + entegre **yardım**.
//!
//! Tasarım: her parça **saf model + küçük egui adaptörü** (birim-testlenebilir).  Durum
//! [`OnboardingDurumu`]'nda toplanır; host ([`biocraft-app`]) bunu kalıcı `tercihler`'e JSON olarak
//! yazar (ayar/kısayol kalıbıyla aynı).  Tüm metin **TR/EN** (MK-53); renkler **token**'dan (MK-52).
//!
//! Bağımlılık yönü temiz: bu modül yalnızca alt katmanlara (egui + i18n/tokens/components/wizard)
//! bağlıdır; host'a (L5) bağlanmaz (MK-40).

pub mod help;
pub mod role;
pub mod templates;
pub mod tour;
pub mod tutorial;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use crate::i18n::Dil;
use crate::tokens::Tokenlar;

pub use help::YardimEylem;
pub use role::{Rol, RolEylem};
pub use templates::{DemoVeri, GaleriEylem, OnboardingSablon, PanelPlani};
pub use tour::{TurAdim, TurDurumu};
pub use tutorial::{bos_durum_rehberi, ipucu_baloncugu, IpucuDurumu, Kavram};

/// Dile göre iki sabit metinden birini seçen küçük yardımcı (onboarding'e özel metinler için;
/// paylaşılan butonlar [`crate::i18n::ceviri`]'den gelir — sihirbazla aynı desen).
pub(crate) fn metin(tr: bool, t: &'static str, e: &'static str) -> &'static str {
    if tr {
        t
    } else {
        e
    }
}

/// Onboarding kalıcı kaydının şema sürümü (göç için — MK-59 ruhu).
const KAYIT_SURUMU: u32 = 1;

/// `tercihler` içinde diskte saklanan minimal kayıt (egui durumu değil; yalnızca kalıcı seçimler).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OnboardingKayit {
    /// Şema sürümü (ileride göç için).
    surum: u32,
    /// Seçilen rol (kararlı kod; `None` = atlandı/seçilmedi).
    #[serde(default)]
    rol: Option<String>,
    /// Tur en az bir kez tamamlandı/atlandı mı.
    #[serde(default)]
    tur_tamamlandi: bool,
    /// Bağlamsal ipuçları kullanıcı tarafından kapatıldı mı.
    #[serde(default)]
    ipuclari_kapali: bool,
}

/// Onboarding'in tüm çalışma-zamanı durumu (host `Sahne`'de tutar).
#[derive(Debug, Clone, Default)]
pub struct OnboardingDurumu {
    /// Seçilen rol (yoksa atlandı/seçilmedi).
    pub rol: Option<Rol>,
    /// "Rolün?" diyaloğu şu an açık mı (ilk açılışta açılır).
    pub rol_dialog_acik: bool,
    /// Hoş geldin turunun durumu (aktif/adım/tamamlandı).
    pub tur: TurDurumu,
    /// Bağlamsal ipuçlarının açık/kapalı durumu.
    pub ipuclari: IpucuDurumu,
    /// Şablon galerisi penceresi açık mı.
    pub sablon_galerisi_acik: bool,
    /// Yardım penceresi açık mı.
    pub yardim_acik: bool,
    /// Yardım penceresi arama metni (oturum durumu).
    pub yardim_arama: String,
    /// Bu, hiç onboarding görülmemiş **ilk açılış** mı (kalıcı kayıt yoktu).
    pub ilk_acilis: bool,
    /// Diske yazılması gereken bir değişiklik var mı (host kalıcılaştırır).
    kirli: bool,
}

/// Onboarding örtülerinin host'a ilettiği eylem (host paneller/demo/dış-bağlantı uygular).
#[derive(Debug, Clone)]
pub enum OnboardingEylem {
    /// Bir rol seçildi (host öneriyi bilgilendirir; **dayatmaz** — K1).
    RolSecildi(Rol),
    /// Rol atlandı (dayatma yok).
    RolAtlandi,
    /// Bir şablonu uygula: ilgili panelleri aç + gömülü demo veriyi yükle.
    SablonUygula(OnboardingSablon),
    /// Dış (çevrimiçi) bağlantı açma isteği — host kullanıcı onayıyla açar.
    DisBaglanti(String),
}

impl OnboardingDurumu {
    /// İlk açılış: "Rolün?" diyaloğu açık gelir; tur rol seçimi/atlamasından sonra başlar.
    pub fn ilk_kez() -> Self {
        Self {
            rol_dialog_acik: true,
            ilk_acilis: true,
            ..Default::default()
        }
    }

    /// Kalıcı `tercihler` değerinden (varsa) yükler; yoksa **ilk açılış** kurar.
    ///
    /// Anahtar mevcut ama bozuksa: kullanıcıyı tekrar tekrar rahatsız etmemek için "görülmüş"
    /// kabul edilir (onboarding tekrar açılmaz; "Yardım"dan erişilebilir).
    pub fn yukle_veya_ilk(deger: Option<&str>) -> Self {
        let Some(json) = deger else {
            return Self::ilk_kez();
        };
        match serde_json::from_str::<OnboardingKayit>(json) {
            Ok(kayit) => Self {
                rol: kayit.rol.as_deref().and_then(Rol::koddan),
                rol_dialog_acik: false,
                tur: TurDurumu {
                    aktif: false,
                    adim: 0,
                    tamamlandi: kayit.tur_tamamlandi,
                },
                ipuclari: IpucuDurumu {
                    kapali: kayit.ipuclari_kapali,
                },
                ilk_acilis: false,
                ..Default::default()
            },
            // Bozuk kayıt → tekrar onboarding'e zorlamayı önle (sessiz/zararsız).
            Err(_) => Self {
                ilk_acilis: false,
                ..Default::default()
            },
        }
    }

    /// Kalıcı kayda dönüştürür (host JSON olarak `tercihler`'e yazar).
    pub fn json(&self) -> String {
        let kayit = OnboardingKayit {
            surum: KAYIT_SURUMU,
            rol: self.rol.map(|r| r.kod().to_string()),
            tur_tamamlandi: self.tur.tamamlandi,
            ipuclari_kapali: self.ipuclari.kapali,
        };
        serde_json::to_string(&kayit).unwrap_or_else(|_| "{}".to_string())
    }

    /// Diske yazılması gereken bir değişiklik var mı.
    pub fn kirli_mi(&self) -> bool {
        self.kirli
    }

    /// Kirlilik bayrağını temizler (host diske yazdıktan sonra çağırır).
    pub fn kirli_temizle(&mut self) {
        self.kirli = false;
    }

    /// Turu baştan başlatır (ilk açılış sonrası veya "Yardım > Tur").
    pub fn turu_baslat(&mut self) {
        self.tur.baslat();
    }

    /// Şablon galerisini açar.
    pub fn galeriyi_ac(&mut self) {
        self.sablon_galerisi_acik = true;
    }

    /// Yardım penceresini açar.
    pub fn yardimi_ac(&mut self) {
        self.yardim_acik = true;
    }

    /// İpuçlarını tamamen kapatır (kullanıcı "bir daha gösterme" dediğinde).
    pub fn ipuclari_kapat(&mut self) {
        if !self.ipuclari.kapali {
            self.ipuclari.kapali = true;
            self.kirli = true;
        }
    }

    /// "Demo Projeyi Aç" için kullanılacak şablon: rolün önerisi (boş ise görsel demoya düşer →
    /// kullanıcı her zaman dolu bir proje görür).
    pub fn varsayilan_demo_sablonu(&self) -> OnboardingSablon {
        match self.rol.map(|r| r.onerilen_sablon()) {
            Some(OnboardingSablon::Bos) | None => OnboardingSablon::GenomGorsel,
            Some(s) => s,
        }
    }

    /// Aktif onboarding örtüsünü çizer (öncelik: rol diyaloğu → tur → galeri → yardım).
    /// Host'a iletilecek bir eylem ürettiyse döndürür.
    pub fn overlay_ciz(
        &mut self,
        ctx: &egui::Context,
        dil: Dil,
        tok: &Tokenlar,
    ) -> Option<OnboardingEylem> {
        // 1) "Rolün?" diyaloğu — modal; diğer her şeyin önünde.
        if self.rol_dialog_acik {
            if let Some(e) = role::rol_dialog_ciz(ctx, dil, tok) {
                self.rol_dialog_acik = false;
                self.kirli = true;
                // Seçim/atlama sonrası hoş geldin turunu başlat (ilk açılış akışı).
                self.tur.baslat();
                return match e {
                    RolEylem::Sec(r) => {
                        self.rol = Some(r);
                        Some(OnboardingEylem::RolSecildi(r))
                    }
                    RolEylem::Atla => Some(OnboardingEylem::RolAtlandi),
                };
            }
            return None;
        }

        // 2) Hoş geldin turu.
        if self.tur.aktif {
            if tour::tur_ciz(ctx, &mut self.tur, dil, tok) {
                self.kirli = true;
            }
            return None;
        }

        // 3) Şablon galerisi.
        if self.sablon_galerisi_acik {
            if let Some(e) = templates::galeri_ciz(ctx, &mut self.sablon_galerisi_acik, dil, tok) {
                return match e {
                    GaleriEylem::Baslat(s) => {
                        self.sablon_galerisi_acik = false;
                        Some(OnboardingEylem::SablonUygula(s))
                    }
                    GaleriEylem::Kapat => None,
                };
            }
            return None;
        }

        // 4) Yardım penceresi.
        if self.yardim_acik {
            if let Some(YardimEylem::DisBaglanti(u)) =
                help::yardim_penceresi(ctx, &mut self.yardim_acik, &mut self.yardim_arama, dil, tok)
            {
                return Some(OnboardingEylem::DisBaglanti(u));
            }
            return None;
        }

        None
    }
}
