//! `biocraft.toml` manifest şeması — zengin, taşınabilir, **sürümlü** proje formatı (MK-31/34/59).
//!
//! Manifest, projenin **tek otoritatif tanımıdır**: kimlik, oluşturan (ORCID), veri
//! sınıflandırması, gizlilik/güvenlik profili, harici büyük veri referansları ve **uygulanan göç
//! geçmişi**.  Kök seviyede yalnızca alt-tablolar bulunur (TOML "değerler tablolardan önce"
//! kuralından kaçınmak için) → serileştirme her sürümde güvenli.
//!
//! **Güvenlik sınırı (madde 7):** `[guvenlik]` bölümü **hassastır**; dışa aktarımda (`.bcproj`)
//! varsayılan olarak çıkarılır ([`Manifest::disa_aktarim_icin_filtrele`]).  Aynı şekilde harici
//! veri referanslarının `gercek_yol_ipucu` alanı (kullanıcının disk düzenini açığa vurur) da
//! filtrelenir.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use biocraft_types::{DataClassification, Timestamp, Version};

use super::format;

// ─── Yardımcı enum'lar (manifet'in kalıcı, kanonik biçimi) ────────────────────

/// Proje verisinin nerede tutulduğu (manifest kalıcı biçimi).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VeriYerlesimi {
    /// Tüm veri proje klasörünün içinde.
    Yerel,
    /// Veri dış konumlarda; projede yalnızca referans tutulur.
    Baglantili,
}

/// Büyük dosyaların projeye dahil edilme stratejisi (MK-09).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuyukVeriStratejisi {
    /// Referansla tut (yol + boyut + BLAKE3; 50 GB BAM kopyalanmaz).
    Referans,
    /// Projeye göm (kopyala; taşınabilir ama şişer).
    Gomulu,
}

/// Determinizm bayrağı (kanca — MK-59/İP-08).  MVP'de görünür/seçilebilir; bit-bit garanti v1.x.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Determinizm {
    /// Hızlı keşif (varsayılan): hız önceliği.
    HizliKesif,
    /// Tekrarüretilebilir (bilimsel): tekrarlanabilirlik önceliği (gerçek garanti v1.x).
    TekrarUretilebilir,
}

// ─── Manifest bölümleri ───────────────────────────────────────────────────────

/// Kimlik bölümü: ad/açıklama/sürümler/tarihler/şablon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kimlik {
    /// Proje adı.
    pub ad: String,
    /// Açıklama (boş olabilir).
    pub aciklama: String,
    /// Projeyi oluşturan BioCraft sürümü.
    pub biocraft_surumu: Version,
    /// Proje format sürümü (MK-59; göç için).
    pub format_surumu: Version,
    /// Şablon anahtarı (kararlı dizge: `genomik`/`proteomik`/`crispr`/`bos`).
    pub sablon: String,
    /// Oluşturma tarihi (UTC).
    pub olusturma: Timestamp,
    /// Son değiştirme tarihi (UTC).
    pub degistirme: Timestamp,
}

/// Oluşturan bölümü: opsiyonel ORCID + kurum.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Olusturan {
    /// Opsiyonel ORCID (doğrulanmış biçim; yoksa `None`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orcid: Option<String>,
    /// Kurum/organizasyon (boş olabilir).
    #[serde(default)]
    pub kurum: String,
}

/// Sınıflandırma bölümü: zorunlu veri sınıfı + uyumluluk etiketleri + lisans + serbest etiketler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Siniflandirma {
    /// Proje geneli veri sınıflandırması (ZORUNLU — MK-42).
    pub sinif: DataClassification,
    /// Uyumluluk etiketleri (örn. "Akademik", "GDPR-OK").
    #[serde(default)]
    pub uyumluluk: Vec<String>,
    /// Proje lisansı (boş olabilir).
    #[serde(default)]
    pub lisans: String,
    /// Kullanıcı etiketleri (serbest).
    #[serde(default)]
    pub etiketler: Vec<String>,
}

/// Gizlilik profili (hassas DEĞİL; tercihler).  Her proje kendi profilini taşır (global'i ezer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gizlilik {
    /// Tamamen yerel çalışma.
    pub tamamen_yerel: bool,
    /// Anonimleştirilmiş sonuçları AI havuzuna katkı (varsayılan Hayır).
    pub ai_havuzu_katki: bool,
    /// Dağıtık ağ bu proje için etkin mi?
    pub dagitik_ag_etkin: bool,
    /// Determinizm bayrağı (kanca).
    pub determinizm: Determinizm,
}

/// **Hassas** güvenlik bölümü — dışa aktarımda varsayılan HARİÇ (madde 7).
///
/// Şimdilik yalnızca şifreleme bayrağını taşır; ileride (İP-09) anahtar referansları da burada
/// tutulacak.  Bu bölümün export'tan çıkarılması, "onaysız sızmaz" güvencesini sağlar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Guvenlik {
    /// Yerel şifreleme açık mı (varsayılan: açık — "şifreli-yerel").
    pub sifreleme: bool,
}

/// Veri ayarları bölümü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Veri {
    /// Verinin yerleşimi.
    pub yerlesim: VeriYerlesimi,
    /// Büyük veri stratejisi.
    pub buyuk_veri: BuyukVeriStratejisi,
    /// Akış (out-of-core) modu.
    pub akis_modu: bool,
}

/// Harici büyük veri referansı: dosya projede DEĞİL, kullanıcı diskinde — referansla izlenir (MK-09).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HariciVeri {
    /// Proje içindeki mantıksal yol (örn. `data/inputs/ornek.bam`).
    pub mantiksal_yol: String,
    /// Gerçek dosyanın disk üzerindeki yolu — **hassas ipucu** (export'ta filtrelenir).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gercek_yol_ipucu: Option<String>,
    /// Dosya boyutu (bayt).
    pub boyut: u64,
    /// Dosyanın BLAKE3 özeti (hex).
    pub blake3: String,
    /// Bu dosyanın sınıflandırması (dosya-başına).
    pub siniflandirma: DataClassification,
}

/// Uygulanan bir göç (migration) kaydı (MK-59/İP-19).  **Baştan konur** ki eski projeler göç edebilsin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GocKaydi {
    /// Bu göçün hedef format sürümü.
    pub surum: Version,
    /// Göçün uygulandığı tarih (UTC).
    pub tarih: Timestamp,
    /// İnsan-okunur açıklama (örn. "İlk oluşturma").
    pub aciklama: String,
}

// ─── Manifest ─────────────────────────────────────────────────────────────────

/// `biocraft.toml`'un tam yapısı.  Kök seviyede yalnızca alt-tablolar/array-of-tables bulunur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// Kimlik.
    pub kimlik: Kimlik,
    /// Oluşturan.
    #[serde(default)]
    pub olusturan: Olusturan,
    /// Sınıflandırma + uyumluluk + lisans.
    pub siniflandirma: Siniflandirma,
    /// Gizlilik profili (tercihler).
    pub gizlilik: Gizlilik,
    /// **Hassas** güvenlik bölümü; export'ta varsayılan çıkarılır.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guvenlik: Option<Guvenlik>,
    /// Veri ayarları.
    pub veri: Veri,
    /// Harici büyük veri referansları (`[[harici_veri]]`).
    #[serde(default)]
    pub harici_veri: Vec<HariciVeri>,
    /// Uygulanan göç geçmişi (`[[goc]]`).
    #[serde(default)]
    pub goc: Vec<GocKaydi>,
}

impl Manifest {
    /// Manifesti TOML metnine serileştirir.
    pub fn toml_metni(&self) -> Result<String, biocraft_types::ErrorReport> {
        toml::to_string_pretty(self).map_err(|e| {
            biocraft_types::ErrorReport::new(
                "Proje manifesti yazılamadı",
                "Manifest TOML biçimine dönüştürülürken bir sorun oluştu.",
                "Bu bir iç hatadır; lütfen hata kimliğiyle bildirin.",
            )
            .with_teknik_detay(format!("toml ser: {e}"))
        })
    }

    /// TOML metninden manifesti ayrıştırır.
    pub fn toml_coz(metin: &str) -> Result<Self, biocraft_types::ErrorReport> {
        toml::from_str(metin).map_err(|e| {
            biocraft_types::ErrorReport::new(
                "Proje manifesti okunamadı",
                "biocraft.toml dosyası beklenen biçimde değil veya bir alan eksik/hatalı.",
                "Dosyayı yedekten geri yükleyin veya elle düzeltin (TOML söz dizimi).",
            )
            .with_teknik_detay(format!("toml de: {e}"))
        })
    }

    /// **Dışa aktarım için filtreli** bir kopya üretir (madde 7).
    ///
    /// - `[guvenlik]` bölümü çıkarılır (şifreleme/anahtar ayarı sızmasın).
    /// - Harici referansların `gercek_yol_ipucu` alanı çıkarılır (disk düzeni sızmasın).
    ///
    /// `hassas_dahil = true` verilirse (kullanıcı açıkça onaylarsa) hiçbir şey çıkarılmaz.
    pub fn disa_aktarim_icin_filtrele(&self, hassas_dahil: bool) -> Manifest {
        if hassas_dahil {
            return self.clone();
        }
        let mut k = self.clone();
        k.guvenlik = None;
        for ref_ in &mut k.harici_veri {
            ref_.gercek_yol_ipucu = None;
        }
        k
    }

    /// Son değiştirme tarihini şimdiye (UTC) çeker.
    pub fn dokun(&mut self) {
        self.kimlik.degistirme = Utc::now();
    }

    /// İlk göç kaydını (format 1.0.0, "İlk oluşturma") içeren bir başlangıç göç geçmişi üretir.
    pub fn ilk_goc_gecmisi(zaman: Timestamp) -> Vec<GocKaydi> {
        vec![GocKaydi {
            surum: format::format_surumu(),
            tarih: zaman,
            aciklama: "İlk oluşturma".to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ornek() -> Manifest {
        let simdi = Utc::now();
        Manifest {
            kimlik: Kimlik {
                ad: "Deneme".to_string(),
                aciklama: "açıklama".to_string(),
                biocraft_surumu: Version::new(0, 1, 0),
                format_surumu: format::format_surumu(),
                sablon: "genomik".to_string(),
                olusturma: simdi,
                degistirme: simdi,
            },
            olusturan: Olusturan {
                orcid: Some("0000-0002-1825-0097".to_string()),
                kurum: "Kurum".to_string(),
            },
            siniflandirma: Siniflandirma {
                sinif: DataClassification::HasasPhi,
                uyumluluk: vec!["Akademik".to_string()],
                lisans: "CC-BY-4.0".to_string(),
                etiketler: vec!["genom".to_string()],
            },
            gizlilik: Gizlilik {
                tamamen_yerel: true,
                ai_havuzu_katki: false,
                dagitik_ag_etkin: false,
                determinizm: Determinizm::HizliKesif,
            },
            guvenlik: Some(Guvenlik { sifreleme: true }),
            veri: Veri {
                yerlesim: VeriYerlesimi::Yerel,
                buyuk_veri: BuyukVeriStratejisi::Referans,
                akis_modu: true,
            },
            harici_veri: vec![HariciVeri {
                mantiksal_yol: "data/inputs/ornek.bam".to_string(),
                gercek_yol_ipucu: Some("D:/genom/ornek.bam".to_string()),
                boyut: 53_687_091_200,
                blake3: "ab".repeat(32),
                siniflandirma: DataClassification::HasasPhi,
            }],
            goc: Manifest::ilk_goc_gecmisi(simdi),
        }
    }

    #[test]
    fn toml_gidis_donus() {
        let m = ornek();
        let metin = m.toml_metni().unwrap();
        let geri = Manifest::toml_coz(&metin).unwrap();
        assert_eq!(m, geri);
    }

    #[test]
    fn manifest_zorunlu_alanlari_tasir() {
        let m = ornek();
        let metin = m.toml_metni().unwrap();
        // ORCID + sınıflandırma + sürüm + göç geçmişi alanları manifeste yazılmış olmalı.
        assert!(metin.contains("orcid"));
        assert!(metin.contains("0000-0002-1825-0097"));
        assert!(metin.contains("HasasPhi"));
        assert!(metin.contains("format_surumu"));
        assert!(metin.contains("[[goc]]"), "göç geçmişi alanı bulunmalı");
    }

    #[test]
    fn export_filtresi_hassas_ayari_cikarir() {
        let m = ornek();
        let filtreli = m.disa_aktarim_icin_filtrele(false);
        assert!(
            filtreli.guvenlik.is_none(),
            "guvenlik bölümü export'tan çıkmalı"
        );
        assert!(
            filtreli.harici_veri[0].gercek_yol_ipucu.is_none(),
            "gerçek yol ipucu export'tan çıkmalı"
        );
        // Checksum + mantıksal yol KORUNUR (taşınabilirlik için gerekli).
        assert_eq!(filtreli.harici_veri[0].blake3, m.harici_veri[0].blake3);
        assert_eq!(
            filtreli.harici_veri[0].mantiksal_yol,
            m.harici_veri[0].mantiksal_yol
        );
        let metin = filtreli.toml_metni().unwrap();
        assert!(!metin.contains("[guvenlik]"));
        assert!(!metin.contains("D:/genom"));
    }

    #[test]
    fn export_hassas_dahil_birakirsa_korur() {
        let m = ornek();
        let tam = m.disa_aktarim_icin_filtrele(true);
        assert_eq!(tam, m);
    }

    #[test]
    fn ilk_goc_format_bir_sifir_sifir() {
        let g = Manifest::ilk_goc_gecmisi(Utc::now());
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].surum, Version::new(1, 0, 0));
    }
}
