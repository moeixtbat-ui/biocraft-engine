//! **Köken gezgini** — bir verinin *nereden/ne zaman/hangi sürümle/hangi lisansla* geldiğini gösteren
//! saf model (İP-10).
//!
//! Bu, egui'siz, birim-testlenebilir bir görünüm modelidir (veri katmanı L2 → UI yok).  Üst katman
//! (`biocraft-ui`, L4) bunu ince bir egui paneline bağlar.  Model: arama, sınıfa göre süzme, tek
//! öğe detayı, ve **lisanslı bilimsel setlerin atıf listesi** (yöntem bölümü için).

use biocraft_types::DataClassification;

use super::classify::sinif_ad;
use super::provenance::VeriKokeni;

/// Köken gezgininde gösterilecek tek satır (özet görünüm).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KokenSatiri {
    /// Veri kimliği/yolu.
    pub veri_kimligi: String,
    /// Kaynak.
    pub kaynak: String,
    /// Sürüm (boş olabilir).
    pub surum: String,
    /// Sınıf adı (görünür — İP-10).
    pub sinif_ad: &'static str,
    /// Lisans özeti (varsa lisans tanımlayıcısı; yoksa "—").
    pub lisans_ozet: String,
}

impl KokenSatiri {
    fn from(k: &VeriKokeni) -> Self {
        Self {
            veri_kimligi: k.veri_kimligi.clone(),
            kaynak: k.kaynak.clone(),
            surum: k.surum.clone(),
            sinif_ad: sinif_ad(k.siniflandirma),
            lisans_ozet: k
                .lisans_atif
                .as_ref()
                .map(|la| la.lisans.clone())
                .unwrap_or_else(|| "—".to_string()),
        }
    }
}

/// Köken gezgini — bir köken kaydı kümesi üzerinde keşfedilebilir görünüm.
#[derive(Debug, Clone, Default)]
pub struct KokenGezgini {
    kokenler: Vec<VeriKokeni>,
}

impl KokenGezgini {
    /// Köken kayıtlarından bir gezgin kurar.
    pub fn yeni(kokenler: Vec<VeriKokeni>) -> Self {
        Self { kokenler }
    }

    /// Toplam kayıt sayısı.
    pub fn sayi(&self) -> usize {
        self.kokenler.len()
    }

    /// Tüm satırlar (özet görünüm).
    pub fn satirlar(&self) -> Vec<KokenSatiri> {
        self.kokenler.iter().map(KokenSatiri::from).collect()
    }

    /// Metinle arar (kimlik/kaynak/sürüm; harf-duyarsız).
    pub fn ara(&self, sorgu: &str) -> Vec<KokenSatiri> {
        let q = sorgu.trim().to_lowercase();
        if q.is_empty() {
            return self.satirlar();
        }
        self.kokenler
            .iter()
            .filter(|k| {
                k.veri_kimligi.to_lowercase().contains(&q)
                    || k.kaynak.to_lowercase().contains(&q)
                    || k.surum.to_lowercase().contains(&q)
            })
            .map(KokenSatiri::from)
            .collect()
    }

    /// Bir sınıfa göre süzer.
    pub fn sinifa_gore(&self, sinif: DataClassification) -> Vec<KokenSatiri> {
        self.kokenler
            .iter()
            .filter(|k| k.siniflandirma == sinif)
            .map(KokenSatiri::from)
            .collect()
    }

    /// Tek bir verinin tam köken detayını döner (nereden/ne zaman/hangi sürüm/hangi lisans).
    pub fn detay(&self, veri_kimligi: &str) -> Option<&VeriKokeni> {
        self.kokenler
            .iter()
            .find(|k| k.veri_kimligi == veri_kimligi)
    }

    /// Lisans/atıf yükümlülüğü olan (bilimsel) setler.
    pub fn lisansli_setler(&self) -> Vec<&VeriKokeni> {
        self.kokenler
            .iter()
            .filter(|k| k.lisans_atif.is_some())
            .collect()
    }

    /// **Atıf listesi** — yöntem bölümüne yapıştırılabilir, tekilleştirilmiş atıf metinleri.
    pub fn atiflar(&self) -> Vec<String> {
        let mut sonuc: Vec<String> = Vec::new();
        for k in &self.kokenler {
            if let Some(la) = &k.lisans_atif {
                let satir = match &la.url {
                    Some(u) => format!("{} ({}). {}", la.atif, la.lisans, u),
                    None => format!("{} ({}).", la.atif, la.lisans),
                };
                if !sonuc.contains(&satir) {
                    sonuc.push(satir);
                }
            }
        }
        sonuc
    }
}

#[cfg(test)]
mod tests {
    use super::super::provenance::LisansAtif;
    use super::*;

    fn ornek_kokenler() -> Vec<VeriKokeni> {
        vec![
            VeriKokeni::yeni(
                "data/inputs/hasta.vcf",
                "Kullanıcı yüklemesi",
                "11".repeat(32),
                DataClassification::HasasPhi,
            ),
            VeriKokeni::yeni(
                "data/inputs/dbsnp.vcf",
                "NCBI dbSNP",
                "22".repeat(32),
                DataClassification::Normal,
            )
            .surum_ile("build 156")
            .lisans_ile(LisansAtif {
                lisans: "Public Domain".into(),
                atif: "Sherry ST et al., dbSNP, NAR 2001".into(),
                url: None,
            }),
            VeriKokeni::yeni(
                "data/inputs/grch38.fa",
                "Ensembl",
                "33".repeat(32),
                DataClassification::Normal,
            )
            .surum_ile("GRCh38.p14")
            .lisans_ile(LisansAtif {
                lisans: "CC-BY-4.0".into(),
                atif: "Ensembl, GRCh38".into(),
                url: Some("https://www.ensembl.org".into()),
            }),
        ]
    }

    #[test]
    fn satirlar_lisans_ve_sinif_gosterir() {
        let g = KokenGezgini::yeni(ornek_kokenler());
        let satirlar = g.satirlar();
        assert_eq!(satirlar.len(), 3);
        // PHI satırı: lisans yok → "—".
        let phi = satirlar
            .iter()
            .find(|s| s.kaynak == "Kullanıcı yüklemesi")
            .unwrap();
        assert_eq!(phi.lisans_ozet, "—");
        assert_eq!(phi.sinif_ad, "Hassas / PHI");
        // dbSNP satırı: lisans görünür.
        let db = satirlar.iter().find(|s| s.kaynak == "NCBI dbSNP").unwrap();
        assert_eq!(db.lisans_ozet, "Public Domain");
        assert_eq!(db.surum, "build 156");
    }

    #[test]
    fn ara_kaynaga_gore_bulur() {
        let g = KokenGezgini::yeni(ornek_kokenler());
        assert_eq!(g.ara("ensembl").len(), 1);
        assert_eq!(g.ara("vcf").len(), 2);
        assert_eq!(g.ara("").len(), 3);
    }

    #[test]
    fn detay_tek_veri_doner() {
        let g = KokenGezgini::yeni(ornek_kokenler());
        let d = g.detay("data/inputs/dbsnp.vcf").unwrap();
        assert_eq!(d.kaynak, "NCBI dbSNP");
        assert!(g.detay("yok").is_none());
    }

    #[test]
    fn atiflar_yontem_bolumu_icin() {
        let g = KokenGezgini::yeni(ornek_kokenler());
        let atiflar = g.atiflar();
        // Yalnızca lisanslı 2 set atıf üretir (PHI yüklemesi üretmez).
        assert_eq!(atiflar.len(), 2);
        assert!(atiflar.iter().any(|a| a.contains("dbSNP")));
        assert!(atiflar.iter().any(|a| a.contains("ensembl.org")));
    }

    #[test]
    fn lisansli_setler_sadece_bilimsel() {
        let g = KokenGezgini::yeni(ornek_kokenler());
        assert_eq!(g.lisansli_setler().len(), 2);
    }

    #[test]
    fn sinifa_gore_suzer() {
        let g = KokenGezgini::yeni(ornek_kokenler());
        assert_eq!(g.sinifa_gore(DataClassification::HasasPhi).len(), 1);
        assert_eq!(g.sinifa_gore(DataClassification::Normal).len(), 2);
    }
}
