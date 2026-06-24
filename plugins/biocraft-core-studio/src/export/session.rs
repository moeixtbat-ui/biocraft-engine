//! ÇE-11 / İP-11 — **Görünüm oturumu** (kaydet / yükle): "kaldığın görünüme dön".
//!
//! Mevcut görünümün durumu — açık dosyalar, görünür **izler** + ayarları, **bölge** (genom tarayıcı),
//! eklenti **ayarları** ve 3B yapı **kamerası** — proje içinde JSON olarak kalıcılaşır; açılışta tam
//! geri yüklenir (İP-11 oturum/kurtarma).
//!
//! **Şema sürümlenir** ([`SURUM_GUNCEL`]) ve **her alan `serde(default)`'tur** → eski/eksik bir oturum
//! yüklendiğinde alanlar **güvenli varsayılana** düşer ("oturum yüklenince görünüm bozuk" önlenir).
//! Daha eski sürümler [`OturumDurumu::yukle`] sırasında güncel sürüme **göç ettirilir**.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use biocraft_sdk::biocraft_types::ErrorReport;

/// Oturum şemasının **güncel** sürümü.  Kırıcı bir alan değişikliğinde artar; [`OturumDurumu::yukle`]
/// daha düşük sürümleri göç ettirir.
pub const SURUM_GUNCEL: u32 = 1;

/// Bir iz (track) görünüm durumu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IzDurumu {
    /// İz kimliği (veri kaynağı/ad).
    pub kimlik: String,
    /// Görünür mü?
    #[serde(default = "varsayilan_gorunur")]
    pub gorunur: bool,
    /// Piksel yüksekliği.
    #[serde(default)]
    pub yukseklik: f32,
}

fn varsayilan_gorunur() -> bool {
    true
}

/// 3B yapı görünümünün yörünge kamera durumu (ÇE-07).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KameraDurumu {
    /// Hedefe uzaklık (Å).
    pub mesafe: f32,
    /// Yatay açı (radyan).
    pub yaw: f32,
    /// Dikey açı (radyan).
    pub pitch: f32,
    /// Bakış hedefi (Å).
    pub hedef: [f32; 3],
}

/// Kaydedilebilir **görünüm oturumu** durumu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OturumDurumu {
    /// Şema sürümü (göç için).  Eski/sürümsüz dosyada 0 → güncel sürüme göç eder.
    #[serde(default)]
    pub surum: u32,
    /// Açık dosya yolları (mantıksal; gerçek açma yükleme sırasında).
    #[serde(default)]
    pub acik_dosyalar: Vec<String>,
    /// İz durumları (görünürlük/yükseklik/sıra = vektör sırası).
    #[serde(default)]
    pub izler: Vec<IzDurumu>,
    /// Genom tarayıcı görünür bölgesi (örn. `chr1:1000-2000`).
    #[serde(default)]
    pub bolge: Option<String>,
    /// Eklenti ayarları (anahtar → değer; deterministik sıra için `BTreeMap`).
    #[serde(default)]
    pub ayarlar: BTreeMap<String, String>,
    /// 3B yapı kamerası (yapı görünümü açıksa).
    #[serde(default)]
    pub yapi_kamera: Option<KameraDurumu>,
}

impl Default for OturumDurumu {
    fn default() -> Self {
        Self {
            surum: SURUM_GUNCEL,
            acik_dosyalar: Vec::new(),
            izler: Vec::new(),
            bolge: None,
            ayarlar: BTreeMap::new(),
            yapi_kamera: None,
        }
    }
}

impl OturumDurumu {
    /// Boş, güncel-sürümlü oturum.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Açık bir dosya kaydeder (akıcı).
    pub fn dosya_ekle(&mut self, yol: impl Into<String>) -> &mut Self {
        self.acik_dosyalar.push(yol.into());
        self
    }

    /// Bir iz durumu ekler (akıcı).
    pub fn iz_ekle(
        &mut self,
        kimlik: impl Into<String>,
        gorunur: bool,
        yukseklik: f32,
    ) -> &mut Self {
        self.izler.push(IzDurumu {
            kimlik: kimlik.into(),
            gorunur,
            yukseklik,
        });
        self
    }

    /// Bir ayar yazar (akıcı).
    pub fn ayar(&mut self, anahtar: impl Into<String>, deger: impl Into<String>) -> &mut Self {
        self.ayarlar.insert(anahtar.into(), deger.into());
        self
    }

    /// JSON'a serileştirir (proje içine yazılır; `fs`-kapılı çağıran tarafta).
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// JSON'dan yükler + **göç** eder + eksik alanları güvenli varsayılana doldurur.
    ///
    /// * Bozuk/geçersiz JSON → net [`ErrorReport`] (sessiz kabul yok).
    /// * `surum < SURUM_GUNCEL` (veya 0/sürümsüz) → [`goc`](Self::goc) ile güncel sürüme taşınır.
    /// * `surum > SURUM_GUNCEL` (daha yeni uygulamadan) → bilinen alanlar **en iyi çabayla** yüklenir
    ///   (serde bilinmeyen alanları yok sayar); sürüm güncel olana **düşürülmez** ([`gelecek_surum_mu`]).
    pub fn yukle(json: &str) -> Result<Self, ErrorReport> {
        let mut durum: OturumDurumu = serde_json::from_str(json).map_err(|e| {
            ErrorReport::new(
                "Oturum dosyası okunamadı",
                "Kayıtlı görünüm oturumu (JSON) bozuk veya geçersiz olabilir",
                "Oturumu yeniden kaydedin; bu arada varsayılan görünümle devam edebilirsiniz",
            )
            .with_eylem("Varsayılan görünüm")
            .with_teknik_detay(e.to_string())
        })?;
        durum.goc();
        Ok(durum)
    }

    /// Eski/sürümsüz oturumu güncel şemaya taşır.  Bugün tek sürüm var; göç yalnızca sürüm damgasını
    /// günceller (alanlar zaten `serde(default)` ile dolu).  İleride alan-özel dönüşümler buraya eklenir.
    pub fn goc(&mut self) {
        if self.surum < SURUM_GUNCEL {
            // v0 → v1: yeni alan yok; yalnız sürümü damgala (eksik alanlar serde ile dolduruldu).
            self.surum = SURUM_GUNCEL;
        }
    }

    /// Bu oturum, uygulamanın bildiğinden **daha yeni** bir şemadan mı geldi? (ileri-uyum uyarısı için).
    pub fn gelecek_surum_mu(&self) -> bool {
        self.surum > SURUM_GUNCEL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ornek() -> OturumDurumu {
        let mut o = OturumDurumu::yeni();
        o.dosya_ekle("ornek.bam")
            .iz_ekle("hizalama", true, 80.0)
            .iz_ekle("kapsama", false, 40.0)
            .ayar("tema", "koyu");
        o.bolge = Some("chr1:1000-2000".into());
        o.yapi_kamera = Some(KameraDurumu {
            mesafe: 50.0,
            yaw: 0.5,
            pitch: 0.2,
            hedef: [1.0, 2.0, 3.0],
        });
        o
    }

    #[test]
    fn round_trip_kaydet_yukle() {
        let o = ornek();
        let json = o.to_json();
        let geri = OturumDurumu::yukle(&json).unwrap();
        assert_eq!(o, geri);
        assert_eq!(geri.surum, SURUM_GUNCEL);
        assert_eq!(geri.bolge.as_deref(), Some("chr1:1000-2000"));
        assert_eq!(geri.izler.len(), 2);
        assert!(geri.izler[0].gorunur);
        assert!(!geri.izler[1].gorunur);
    }

    #[test]
    fn eksik_alan_guvenli_varsayilan() {
        // Yalnız bölge içeren minimal/eski JSON — diğer her şey varsayılana düşmeli, panik yok.
        let json = r#"{ "bolge": "chr2:5-9" }"#;
        let o = OturumDurumu::yukle(json).unwrap();
        assert_eq!(o.bolge.as_deref(), Some("chr2:5-9"));
        assert!(o.acik_dosyalar.is_empty());
        assert!(o.izler.is_empty());
        assert!(o.ayarlar.is_empty());
        assert!(o.yapi_kamera.is_none());
        // Sürümsüz (0) → güncel sürüme göç etti.
        assert_eq!(o.surum, SURUM_GUNCEL);
    }

    #[test]
    fn iz_kismi_alan_varsayilana_duser() {
        // gorunur alanı olmayan iz → varsayılan true; yukseklik → 0.
        let json = r#"{ "izler": [ { "kimlik": "x" } ] }"#;
        let o = OturumDurumu::yukle(json).unwrap();
        assert_eq!(o.izler.len(), 1);
        assert!(o.izler[0].gorunur); // varsayilan_gorunur
        assert_eq!(o.izler[0].yukseklik, 0.0);
    }

    #[test]
    fn bozuk_json_net_hata() {
        let r = OturumDurumu::yukle("{ bu json değil ");
        let hata = r.err().unwrap();
        assert_eq!(hata.ne_oldu, "Oturum dosyası okunamadı");
    }

    #[test]
    fn gelecek_surum_korunur_ve_isaretlenir() {
        let json = r#"{ "surum": 999, "bolge": "chrX:1-2" }"#;
        let o = OturumDurumu::yukle(json).unwrap();
        // Daha yeni sürüm güncel olana düşürülmez; ama bilinen alanlar yüklenir.
        assert_eq!(o.surum, 999);
        assert!(o.gelecek_surum_mu());
        assert_eq!(o.bolge.as_deref(), Some("chrX:1-2"));
    }
}
