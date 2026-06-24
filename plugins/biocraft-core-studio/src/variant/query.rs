//! ÇE-04 — **Sorgu motoru** (filtre/sıralama + out-of-core kaynak soyutlaması).
//!
//! ## DuckDB sınırı (önemli mimari karar — Gün 38)
//! ARCHITECTURE.md analitik için **DuckDB/Arrow** önerir (MK-09: predicate pushdown + out-of-core).
//! Bu sürümde, projenin **"saf-Rust, yeni ağır dış bağımlılık ekleme"** disiplinine (Gün 35/37) sadık
//! kalınarak DuckDB'nin **C++ bağımlılığı eklenmez**; bunun yerine:
//!
//! * Büyük VCF/BCF [`data_io::VaryantOkuyucu`](crate::data_io::VaryantOkuyucu) ile **out-of-core**
//!   (indeksli bölge sorgusu veya akışlı tarama; tüm dosya RAM'e alınmaz — MK-09) okunur,
//! * filtre/sıralama **saf-Rust** [`SafRustMotor`] ile uygulanır,
//! * ve tüm motorlar tek bir arayüz — [`VaryantSorguMotoru`] — arkasındadır.
//!
//! Böylece ileride gerçek bir **DuckDB motoru** (`db` yeteneği + C++ bağımlılığı, insan-eli iş)
//! aynı `VaryantSorguMotoru` trait'ini uygulayarak **çağıranı hiç değiştirmeden** takılabilir
//! (HTTP/Iroh/ödeme kancalarındaki "gerçek-sürücü-sonra" deseniyle aynı — dürüst sınır).
//!
//! `SafRustMotor` eşleşen satırları (bir **üst sınıra** kadar) bir kez maddileştirir → sanal
//! kaydırma O(1) sayfa erişimiyle akıcı kalır; sınır aşılırsa `kesildi` ile dürüstçe bildirilir
//! ("bölge daraltın / DuckDB motoru gerek").

use std::path::Path;

use biocraft_sdk::biocraft_types::{Capability, ErrorReport};
use biocraft_sdk::YetkiKapisi;

use crate::data_io::{BellekButcesi, VaryantBasligi, VaryantKaydi, VaryantOkuyucu, VeriFormati};
use crate::genome_browser::canvas::GenomBolge;
use crate::genome_browser::veri::VaryantTuru;

use super::filter::Filtre;

/// Saf-Rust motorun bir sorguda tarayacağı **azami ham kayıt** sayısı (out-of-core üst sınırı).
/// Eşleşen sonuçlar bunun altında maddileştirilir; aşılırsa [`SorguSonuc::kesildi`] = `true`.
pub const VARSAYILAN_AZAMI_TARA: usize = 200_000;

// ─── Satır modeli ────────────────────────────────────────────────────────────────

/// Tablo/ızgara/izde gösterilen tek bir varyant satırı: ham kayıt + türetilmiş tür.
#[derive(Debug, Clone, PartialEq)]
pub struct VaryantSatiri {
    /// Ham varyant kaydı (CHROM/POS/ID/REF/ALT/QUAL/FILTER/INFO/FORMAT/GT).
    pub kayit: VaryantKaydi,
    /// REF/ALT'tan türetilen varyant türü (SNV/INS/DEL/Diğer) — tür filtresi + görsel ayrım.
    pub tur: VaryantTuru,
}

impl VaryantSatiri {
    /// Bir [`VaryantKaydi`]'ndan satır üretir (türü hesaplar).
    pub fn yeni(kayit: VaryantKaydi) -> Self {
        let tur = VaryantTuru::belirle(&kayit.referans, &kayit.alternatifler);
        Self { kayit, tur }
    }

    /// Kromozom adı.
    pub fn kromozom(&self) -> &str {
        &self.kayit.kromozom
    }

    /// 1-tabanlı konum.
    pub fn konum(&self) -> usize {
        self.kayit.konum
    }

    /// Kalite skoru (varsa).
    pub fn kalite(&self) -> Option<f32> {
        self.kayit.kalite
    }

    /// `FILTER` sütunu `PASS` mı? (boş/`.` ⇒ PASS sayılmaz; standart bcftools davranışı.)
    pub fn pass_mi(&self) -> bool {
        self.kayit.filtreler.iter().any(|f| f == "PASS")
    }

    /// ALT alel(ler)inin `,` ile birleşik metni.
    pub fn alt_metni(&self) -> String {
        if self.kayit.alternatifler.is_empty() {
            ".".into()
        } else {
            self.kayit.alternatifler.join(",")
        }
    }

    /// SNV ise (ref+alt tek baz) **geçiş** (transition: A↔G, C↔T) mı? Aksi halde `false`.
    pub fn gecis_mi(&self) -> bool {
        if self.tur != VaryantTuru::Snv {
            return false;
        }
        let r = self
            .kayit
            .referans
            .as_bytes()
            .first()
            .map(|b| b.to_ascii_uppercase());
        self.kayit.alternatifler.iter().any(|a| {
            let alt = a.as_bytes().first().map(|b| b.to_ascii_uppercase());
            matches!(
                (r, alt),
                (Some(b'A'), Some(b'G'))
                    | (Some(b'G'), Some(b'A'))
                    | (Some(b'C'), Some(b'T'))
                    | (Some(b'T'), Some(b'C'))
            )
        })
    }
}

// ─── Kaynak soyutlaması (out-of-core) ─────────────────────────────────────────────

/// Varyant satırlarının **out-of-core** kaynağı.  Bölge verilirse yalnız o bölge okunur
/// (predicate/region pushdown — MK-09); `None` ise tüm dosya akışlı taranır (üst sınıra kadar).
pub trait VaryantKaynak {
    /// Dosya başlığı (örnek adları, format, indeksli mi).
    fn basligi(&self) -> &VaryantBasligi;

    /// Bölge (varsa) + üst sınır dahilinde satırları okur.
    fn tara(
        &mut self,
        bolge: Option<&GenomBolge>,
        butce: &BellekButcesi,
        azami: usize,
    ) -> Result<Vec<VaryantSatiri>, ErrorReport>;
}

/// Bellek-içi kaynak (test/önizleme; gerçek dosya gerektirmez).
pub struct BellekKaynak {
    basligi: VaryantBasligi,
    satirlar: Vec<VaryantSatiri>,
}

impl BellekKaynak {
    /// Örnek adları + hazır satırlardan kaynak kurar.
    pub fn yeni(ornekler: Vec<String>, satirlar: Vec<VaryantSatiri>) -> Self {
        Self {
            basligi: VaryantBasligi {
                format: VeriFormati::Vcf,
                ornekler,
                indeksli: false,
            },
            satirlar,
        }
    }
}

impl VaryantKaynak for BellekKaynak {
    fn basligi(&self) -> &VaryantBasligi {
        &self.basligi
    }

    fn tara(
        &mut self,
        bolge: Option<&GenomBolge>,
        _butce: &BellekButcesi,
        azami: usize,
    ) -> Result<Vec<VaryantSatiri>, ErrorReport> {
        let mut sonuc = Vec::new();
        for s in &self.satirlar {
            if sonuc.len() >= azami {
                break;
            }
            if let Some(b) = bolge {
                let bas = s.konum() as u64;
                let bit = bas + (s.kayit.referans.len().max(1) as u64) - 1;
                if s.kromozom() != b.kromozom || !b.ortusur(bas, bit) {
                    continue;
                }
            }
            sonuc.push(s.clone());
        }
        Ok(sonuc)
    }
}

/// Gerçek dosya kaynağı: [`VaryantOkuyucu`]'yu (noodles, out-of-core) sarar.  `fs` yeteneği gerekir.
pub struct DosyaKaynak {
    okuyucu: VaryantOkuyucu,
    basligi: VaryantBasligi,
}

impl DosyaKaynak {
    /// Bir VCF/BCF dosyasını açar (MK-13: `fs` yetkisi doğrulanır).
    pub fn ac(yol: &Path, yetkiler: &YetkiKapisi) -> Result<Self, ErrorReport> {
        yetkiler.iste(Capability::Fs)?;
        let (okuyucu, basligi) = VaryantOkuyucu::ac(yol)?;
        Ok(Self { okuyucu, basligi })
    }
}

impl VaryantKaynak for DosyaKaynak {
    fn basligi(&self) -> &VaryantBasligi {
        &self.basligi
    }

    fn tara(
        &mut self,
        bolge: Option<&GenomBolge>,
        butce: &BellekButcesi,
        azami: usize,
    ) -> Result<Vec<VaryantSatiri>, ErrorReport> {
        let kayitlar = match bolge {
            Some(b) => {
                let ifade = format!("{}:{}-{}", b.kromozom, b.baslangic, b.bitis);
                self.okuyucu.bolge_sorgu(&ifade, butce, azami)?
            }
            None => self.okuyucu.tum_tara(butce, azami)?,
        };
        Ok(kayitlar.into_iter().map(VaryantSatiri::yeni).collect())
    }
}

// ─── Sorgu / sıralama / sonuç ──────────────────────────────────────────────────

/// Sıralama anahtarı (sütun).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiralamaAnahtari {
    /// Genomik konum (kromozom, sonra POS) — varsayılan.
    Konum,
    /// Kalite (QUAL).
    Kalite,
    /// Kimlik (ID/rsID).
    Kimlik,
    /// Varyant türü.
    Tur,
}

/// Sıralama yönergesi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Siralama {
    /// Hangi anahtara göre.
    pub anahtar: SiralamaAnahtari,
    /// Artan mı (`true`) azalan mı (`false`).
    pub artan: bool,
}

impl Default for Siralama {
    fn default() -> Self {
        Self {
            anahtar: SiralamaAnahtari::Konum,
            artan: true,
        }
    }
}

impl Siralama {
    /// Verilen satırları yerinde **kararlı** (stable) sıralar.
    pub fn uygula(&self, satirlar: &mut [VaryantSatiri]) {
        match self.anahtar {
            SiralamaAnahtari::Konum => satirlar.sort_by(|a, b| {
                a.kromozom()
                    .cmp(b.kromozom())
                    .then(a.konum().cmp(&b.konum()))
            }),
            SiralamaAnahtari::Kalite => satirlar.sort_by(|a, b| {
                // None kaliteler en sona (artan); NaN-güvenli total_cmp.
                let ka = a.kalite().unwrap_or(f32::NEG_INFINITY);
                let kb = b.kalite().unwrap_or(f32::NEG_INFINITY);
                ka.total_cmp(&kb)
            }),
            SiralamaAnahtari::Kimlik => {
                satirlar.sort_by(|a, b| a.kayit.kimlik.cmp(&b.kayit.kimlik))
            }
            SiralamaAnahtari::Tur => satirlar.sort_by_key(|a| tur_sira(a.tur)),
        }
        if !self.artan {
            satirlar.reverse();
        }
    }
}

/// Tür için kararlı sıralama indeksi.
fn tur_sira(t: VaryantTuru) -> u8 {
    match t {
        VaryantTuru::Snv => 0,
        VaryantTuru::Insersiyon => 1,
        VaryantTuru::Delesyon => 2,
        VaryantTuru::Diger => 3,
    }
}

/// Bir sorgu: filtre + sıralama.  (Sayfalama maddileştirilmiş sonuç üzerinde tablo katmanında
/// yapılır → sanal kaydırma yeniden-sorgu yapmaz.)
#[derive(Debug, Clone, PartialEq)]
pub struct Sorgu {
    /// Uygulanacak filtre.
    pub filtre: Filtre,
    /// Sıralama yönergesi.
    pub siralama: Siralama,
}

impl Sorgu {
    /// Filtreden varsayılan sıralamalı sorgu.
    pub fn yeni(filtre: Filtre) -> Self {
        Self {
            filtre,
            siralama: Siralama::default(),
        }
    }
}

/// Bir sorgunun sonucu: eşleşen (filtreli + sıralı) satırlar + sayım + kesilme bayrağı.
#[derive(Debug, Clone, PartialEq)]
pub struct SorguSonuc {
    /// Eşleşen satırlar (filtreli + sıralı; üst sınıra kadar maddileştirilmiş).
    pub satirlar: Vec<VaryantSatiri>,
    /// Eşleşen toplam satır (= `satirlar.len()`; sanal kaydırma bunu kullanır).
    pub toplam_eslesme: usize,
    /// Taranan ham kayıt sayısı (out-of-core; teşhis).
    pub taranan: usize,
    /// Tarama üst sınırına ulaşıldı mı? (`true` ⇒ sonuç eksik olabilir, bölge daraltın.)
    pub kesildi: bool,
}

/// Filtre üzerinden hesaplanan özet istatistik (varyant sayısı + tür dağılımı + Ts/Tv).
#[derive(Debug, Clone, PartialEq)]
pub struct Istatistik {
    /// Toplam (filtreli) varyant.
    pub toplam: usize,
    /// SNV sayısı.
    pub snv: usize,
    /// İnsersiyon sayısı.
    pub insersiyon: usize,
    /// Delesyon sayısı.
    pub delesyon: usize,
    /// Diğer/karmaşık sayısı.
    pub diger: usize,
    /// `FILTER=PASS` olan varyant.
    pub pass: usize,
    /// SNV geçiş (transition) sayısı.
    pub ts: usize,
    /// SNV geçişsizlik (transversion) sayısı.
    pub tv: usize,
    /// Ortalama kalite (QUAL olanlar üzerinden; yoksa `None`).
    pub ortalama_kalite: Option<f32>,
    /// İstatistik tarama üst sınırında kesildi mi?
    pub kesildi: bool,
}

impl Istatistik {
    /// Ts/Tv oranı (transversion 0 ise `None`).
    pub fn ts_tv(&self) -> Option<f32> {
        if self.tv == 0 {
            None
        } else {
            Some(self.ts as f32 / self.tv as f32)
        }
    }
}

// ─── Motor arayüzü (DuckDB sınırı) ────────────────────────────────────────────

/// Varyant sorgu motoru — **filtre/sıralama/istatistik** sözleşmesi.  Saf-Rust ([`SafRustMotor`])
/// bugün; ileride **DuckDB motoru** aynı trait'i uygulayarak çağıranı değiştirmeden takılabilir.
pub trait VaryantSorguMotoru {
    /// Sorguyu çalıştırır (filtreli + sıralı sonuç).
    fn calistir(&mut self, sorgu: &Sorgu) -> Result<SorguSonuc, ErrorReport>;

    /// Filtre üzerinden özet istatistik üretir.
    fn istatistik(&mut self, filtre: &Filtre) -> Result<Istatistik, ErrorReport>;

    /// Motorun adı (teşhis/UI: "saf-rust" / "duckdb").
    fn motor_adi(&self) -> &'static str;
}

/// Saf-Rust sorgu motoru: kaynaktan (out-of-core) okur, filtreyi/sıralamayı bellekte uygular.
pub struct SafRustMotor<K: VaryantKaynak> {
    kaynak: K,
    azami_tara: usize,
    butce: BellekButcesi,
}

impl<K: VaryantKaynak> SafRustMotor<K> {
    /// Varsayılan üst sınır + sınırsız bütçe ile motor kurar.
    pub fn yeni(kaynak: K) -> Self {
        Self {
            kaynak,
            azami_tara: VARSAYILAN_AZAMI_TARA,
            butce: BellekButcesi::sinirsiz(),
        }
    }

    /// Tarama üst sınırını ayarlar (sanal kaydırma için maddileştirme tavanı).
    pub fn azami_tara_ile(mut self, azami: usize) -> Self {
        self.azami_tara = azami.max(1);
        self
    }

    /// Bellek bütçesini ayarlar (İP-08).
    pub fn butce_ile(mut self, butce: BellekButcesi) -> Self {
        self.butce = butce;
        self
    }

    /// Başlık (örnek adları vb.).
    pub fn basligi(&self) -> &VaryantBasligi {
        self.kaynak.basligi()
    }

    /// Filtreyi uygulayıp eşleşenleri toplar (sıralamasız) — ortak iç yardımcı.
    fn topla(&mut self, filtre: &Filtre) -> Result<(Vec<VaryantSatiri>, usize, bool), ErrorReport> {
        let ham = self
            .kaynak
            .tara(filtre.bolge.as_ref(), &self.butce, self.azami_tara)?;
        let taranan = ham.len();
        let kesildi = taranan >= self.azami_tara;
        let eslesen: Vec<VaryantSatiri> = ham.into_iter().filter(|s| filtre.gecer(s)).collect();
        Ok((eslesen, taranan, kesildi))
    }
}

impl<K: VaryantKaynak> VaryantSorguMotoru for SafRustMotor<K> {
    fn calistir(&mut self, sorgu: &Sorgu) -> Result<SorguSonuc, ErrorReport> {
        let (mut eslesen, taranan, kesildi) = self.topla(&sorgu.filtre)?;
        sorgu.siralama.uygula(&mut eslesen);
        Ok(SorguSonuc {
            toplam_eslesme: eslesen.len(),
            satirlar: eslesen,
            taranan,
            kesildi,
        })
    }

    fn istatistik(&mut self, filtre: &Filtre) -> Result<Istatistik, ErrorReport> {
        let (eslesen, _taranan, kesildi) = self.topla(filtre)?;
        let mut ist = Istatistik {
            toplam: eslesen.len(),
            snv: 0,
            insersiyon: 0,
            delesyon: 0,
            diger: 0,
            pass: 0,
            ts: 0,
            tv: 0,
            ortalama_kalite: None,
            kesildi,
        };
        let mut kalite_top = 0.0f64;
        let mut kalite_say = 0usize;
        for s in &eslesen {
            match s.tur {
                VaryantTuru::Snv => {
                    ist.snv += 1;
                    if s.gecis_mi() {
                        ist.ts += 1;
                    } else {
                        ist.tv += 1;
                    }
                }
                VaryantTuru::Insersiyon => ist.insersiyon += 1,
                VaryantTuru::Delesyon => ist.delesyon += 1,
                VaryantTuru::Diger => ist.diger += 1,
            }
            if s.pass_mi() {
                ist.pass += 1;
            }
            if let Some(q) = s.kalite() {
                kalite_top += q as f64;
                kalite_say += 1;
            }
        }
        if kalite_say > 0 {
            ist.ortalama_kalite = Some((kalite_top / kalite_say as f64) as f32);
        }
        Ok(ist)
    }

    fn motor_adi(&self) -> &'static str {
        "saf-rust"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kayit(kr: &str, pos: usize, id: &str, r: &str, a: &str, q: f32, filt: &str) -> VaryantKaydi {
        VaryantKaydi {
            kromozom: kr.into(),
            konum: pos,
            kimlik: id.into(),
            referans: r.into(),
            alternatifler: a.split(',').map(|s| s.to_string()).collect(),
            kalite: Some(q),
            filtreler: vec![filt.into()],
            info: vec![],
            ornek_sayisi: 0,
            format_anahtarlari: vec![],
            genotipler: vec![],
        }
    }

    fn ornek_kaynak() -> BellekKaynak {
        let satirlar = vec![
            VaryantSatiri::yeni(kayit("chr1", 100, "rs1", "A", "G", 50.0, "PASS")), // SNV ts (A>G)
            VaryantSatiri::yeni(kayit("chr1", 200, "rs2", "A", "T", 20.0, "q10")),  // SNV tv (A>T)
            VaryantSatiri::yeni(kayit("chr1", 300, ".", "A", "ACGT", 60.0, "PASS")), // INS
            VaryantSatiri::yeni(kayit("chr2", 400, "rs3", "ACGT", "A", 40.0, "PASS")), // DEL
        ];
        BellekKaynak::yeni(vec!["S1".into()], satirlar)
    }

    #[test]
    fn filtresiz_hepsini_konuma_gore_dondurur() {
        let mut motor = SafRustMotor::yeni(ornek_kaynak());
        let sonuc = motor.calistir(&Sorgu::yeni(Filtre::default())).unwrap();
        assert_eq!(sonuc.toplam_eslesme, 4);
        assert!(!sonuc.kesildi);
        // Konum sırası: chr1:100, chr1:200, chr1:300, chr2:400.
        assert_eq!(sonuc.satirlar[0].konum(), 100);
        assert_eq!(sonuc.satirlar[3].kromozom(), "chr2");
    }

    #[test]
    fn kalite_filtresi_ve_azalan_siralama() {
        let mut motor = SafRustMotor::yeni(ornek_kaynak());
        let filtre = Filtre {
            kalite_min: Some(40.0),
            ..Filtre::default()
        };
        let sorgu = Sorgu {
            filtre,
            siralama: Siralama {
                anahtar: SiralamaAnahtari::Kalite,
                artan: false,
            },
        };
        let sonuc = motor.calistir(&sorgu).unwrap();
        // QUAL>=40 → 50,60,40 (üç kayıt); azalan: 60,50,40.
        assert_eq!(sonuc.toplam_eslesme, 3);
        assert_eq!(sonuc.satirlar[0].kalite(), Some(60.0));
        assert_eq!(sonuc.satirlar[2].kalite(), Some(40.0));
    }

    #[test]
    fn sadece_pass_filtresi() {
        let mut motor = SafRustMotor::yeni(ornek_kaynak());
        let filtre = Filtre {
            sadece_pass: true,
            ..Filtre::default()
        };
        let sonuc = motor.calistir(&Sorgu::yeni(filtre)).unwrap();
        assert_eq!(sonuc.toplam_eslesme, 3); // q10 hariç
        assert!(sonuc.satirlar.iter().all(|s| s.pass_mi()));
    }

    #[test]
    fn istatistik_tur_dagilimi_ve_ts_tv() {
        let mut motor = SafRustMotor::yeni(ornek_kaynak());
        let ist = motor.istatistik(&Filtre::default()).unwrap();
        assert_eq!(ist.toplam, 4);
        assert_eq!(ist.snv, 2);
        assert_eq!(ist.insersiyon, 1);
        assert_eq!(ist.delesyon, 1);
        assert_eq!(ist.ts, 1); // A>G
        assert_eq!(ist.tv, 1); // A>T
        assert_eq!(ist.pass, 3);
        assert_eq!(ist.ts_tv(), Some(1.0));
    }

    #[test]
    fn kesildi_bayragi_ust_sinirda() {
        let mut motor = SafRustMotor::yeni(ornek_kaynak()).azami_tara_ile(2);
        let sonuc = motor.calistir(&Sorgu::yeni(Filtre::default())).unwrap();
        // Yalnız 2 ham kayıt tarandı → kesildi.
        assert!(sonuc.kesildi);
        assert!(sonuc.toplam_eslesme <= 2);
    }

    #[test]
    fn bolge_pushdown_yalniz_o_bolge() {
        let mut motor = SafRustMotor::yeni(ornek_kaynak());
        let filtre = Filtre {
            bolge: Some(GenomBolge::yeni("chr1", 150, 350).unwrap()),
            ..Filtre::default()
        };
        let sonuc = motor.calistir(&Sorgu::yeni(filtre)).unwrap();
        // chr1:150-350 → 200, 300 (100 ve chr2:400 hariç).
        assert_eq!(sonuc.toplam_eslesme, 2);
        assert_eq!(sonuc.satirlar[0].konum(), 200);
        assert_eq!(sonuc.satirlar[1].konum(), 300);
    }

    #[test]
    fn motor_adi_saf_rust() {
        let motor = SafRustMotor::yeni(ornek_kaynak());
        assert_eq!(motor.motor_adi(), "saf-rust");
    }
}
