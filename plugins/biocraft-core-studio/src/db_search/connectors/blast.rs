//! ÇE-09 — **NCBI BLAST URL API konektörü** (uzak dizi benzerlik araması).
//!
//! Uzun iş modeli (İP-21 [`IsKulpu`]; Gün 4/32):
//! * [`BlastKonektor::gonder`] — `CMD=Put` → bir **RID** (iş kimliği) + tahmini süre döndürür.
//! * [`BlastKonektor::durum_yokla`] — `CMD=Get&FORMAT_OBJECT=SearchInfo` → iş durumu (BEKLİYOR/HAZIR).
//! * [`BlastKonektor::sonuc_al`] — `CMD=Get&FORMAT_TYPE=Tabular` → hizalama tablosu.
//! * [`BlastKonektor::blast_calistir`] — üçünü bir [`IsKulpu`] + iptal jetonuyla sürer
//!   (ilerleme bildirir, iptal edilebilir, zaman aşımında RID ile devam önerir).
//!
//! Dizi BLAST'a **gönderilmeden önce** gizlilik kapısından geçer — PHI/hassas dizi dış sorguya
//! **çıkamaz** (MK-42/43); kullanıcı "dizi gönderiliyor" özetini görür (MK-41).

use std::time::Duration;

use biocraft_sdk::biocraft_types::ErrorReport;
use biocraft_sdk::biocraft_types::{Ilerleme, IsKulpu};

use crate::data_io::tekrar_ile;

use super::super::framework::{
    AramaBaglami, AramaSonucu, GetirilenKayit, KayitTuru, SayfaBilgisi, Sayfalama, SonucListesi,
    SonucSkoru, Sorgu, VeritabaniKonektoru,
};
use super::super::privacy::{DisVeri, HassasiyetEtiketi};
use super::super::transport::HttpIstek;

/// NCBI BLAST URL API uç noktası.
pub const BLAST_URL: &str = "https://blast.ncbi.nlm.nih.gov/Blast.cgi";
/// NCBI web tabanı (sonuç hizalamasını tarayıcıda açma).
const NCBI_ARAMA: &str = "https://www.ncbi.nlm.nih.gov/search/all/?term=";
const HEDEF: &str = "NCBI BLAST (blast.ncbi.nlm.nih.gov)";

/// BLAST programı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlastProgram {
    /// Nükleotid → nükleotid.
    Blastn,
    /// Protein → protein.
    Blastp,
    /// Çevrilmiş nükleotid → protein.
    Blastx,
    /// Protein → çevrilmiş nükleotid.
    Tblastn,
}

impl BlastProgram {
    /// API `PROGRAM` değeri.
    pub fn ad(&self) -> &'static str {
        match self {
            BlastProgram::Blastn => "blastn",
            BlastProgram::Blastp => "blastp",
            BlastProgram::Blastx => "blastx",
            BlastProgram::Tblastn => "tblastn",
        }
    }
    /// Sorgu dizisi nükleotid mi (girdi tipi)?
    pub fn nukleotid_sorgu_mu(&self) -> bool {
        matches!(self, BlastProgram::Blastn | BlastProgram::Blastx)
    }
    /// Programa uygun varsayılan veritabanı (nt/nr).
    pub fn varsayilan_db(&self) -> &'static str {
        match self {
            BlastProgram::Blastn => "nt",
            BlastProgram::Blastp | BlastProgram::Tblastn => "nr",
            BlastProgram::Blastx => "nr",
        }
    }
}

/// Gönderilen BLAST işi (uzak iş kimliği + tahmini süre).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlastIsi {
    /// Uzak iş kimliği (Request ID) — durum/sonuç sorgularında kullanılır.
    pub rid: String,
    /// Sunucunun tahmini bekleme süresi (saniye; RTOE).
    pub tahmini_sure_sn: u64,
}

/// BLAST işinin durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlastDurum {
    /// Henüz hazır değil (devam ediyor).
    Bekliyor,
    /// Sonuç hazır.
    Hazir,
    /// Bilinmiyor / RID süresi dolmuş / hata.
    Bilinmiyor,
}

/// Tek bir BLAST hizalaması (tabular `-outfmt 6` satırı).
#[derive(Debug, Clone, PartialEq)]
pub struct Hizalama {
    /// Konu (subject) dizisi kimliği/accession.
    pub konu_kimligi: String,
    /// Özdeşlik yüzdesi (0–100).
    pub ozdeslik_yuzde: f64,
    /// Hizalama uzunluğu.
    pub hizalama_uzunlugu: u64,
    /// Uyuşmazlık sayısı.
    pub uyusmazlik: u64,
    /// Boşluk açılışı sayısı.
    pub bosluk_acilis: u64,
    /// Sorgu başlangıcı.
    pub sorgu_bas: u64,
    /// Sorgu bitişi.
    pub sorgu_bit: u64,
    /// Konu başlangıcı.
    pub konu_bas: u64,
    /// Konu bitişi.
    pub konu_bit: u64,
    /// E-değeri.
    pub e_deger: f64,
    /// Bit skoru.
    pub bit_skoru: f64,
}

/// Uzun BLAST işini yoklama ayarı (kaç kez, hangi aralıkla).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YoklamaAyari {
    /// En fazla yoklama sayısı.
    pub azami_yoklama: u32,
    /// Yoklamalar arası bekleme (test: `Duration::ZERO`).
    pub aralik: Duration,
}

impl Default for YoklamaAyari {
    fn default() -> Self {
        Self {
            azami_yoklama: 60,
            aralik: Duration::from_secs(3),
        }
    }
}

/// NCBI BLAST konektörü (program + veritabanı + yoklama ayarı).
pub struct BlastKonektor {
    program: BlastProgram,
    veritabani: String,
    kaynak_adi: String,
    turler: [KayitTuru; 1],
    yoklama: YoklamaAyari,
}

impl BlastKonektor {
    /// Program + veritabanı ile konektör.
    pub fn yeni(program: BlastProgram, veritabani: impl Into<String>) -> Self {
        let veritabani = veritabani.into();
        Self {
            kaynak_adi: format!("BLAST {}", program.ad()),
            program,
            veritabani,
            turler: [KayitTuru::Hizalama],
            yoklama: YoklamaAyari::default(),
        }
    }

    /// Programın varsayılan veritabanıyla konektör (blastn→nt, blastp→nr…).
    pub fn varsayilan(program: BlastProgram) -> Self {
        let db = program.varsayilan_db().to_string();
        Self::yeni(program, db)
    }

    /// Yoklama ayarını değiştirir (akıcı; test/UI).
    pub fn with_yoklama(mut self, yoklama: YoklamaAyari) -> Self {
        self.yoklama = yoklama;
        self
    }

    /// İsteği gönderir (tekrar/geri-çekilme + HTTP durum denetimi) → gövde.
    fn gonder_istek(
        &self,
        istek: &HttpIstek,
        baglam: &AramaBaglami,
    ) -> Result<String, ErrorReport> {
        baglam.hiz_bekle();
        let yanit = tekrar_ile(&baglam.yapi, |_| {
            let y = baglam.ulastirici.gonder(istek)?;
            y.metin()?;
            Ok(y)
        })?;
        Ok(yanit.govde)
    }

    /// Diziyi BLAST'a **gönderir** (CMD=Put) → RID.  Gizlilik kapısı: dizi PHI/hassasiyetine göre
    /// engellenebilir (MK-42/43); `etiket` çağıran tarafından verilir (verinin sınıfı).
    pub fn gonder(
        &self,
        dizi: &str,
        etiket: HassasiyetEtiketi,
        baglam: &AramaBaglami,
    ) -> Result<BlastIsi, ErrorReport> {
        baglam
            .gizlilik
            .dis_gonderim(&self.kaynak_adi, HEDEF, DisVeri::Dizi(dizi), etiket)?;

        let istek = HttpIstek::put(BLAST_URL)
            .param("CMD", "Put")
            .param("PROGRAM", self.program.ad())
            .param("DATABASE", &self.veritabani)
            .param("QUERY", dizi);
        let govde = self.gonder_istek(&istek, baglam)?;
        qblast_is_ayristir(&govde)
    }

    /// İşin durumunu **yoklar** (CMD=Get&FORMAT_OBJECT=SearchInfo).
    pub fn durum_yokla(&self, rid: &str, baglam: &AramaBaglami) -> Result<BlastDurum, ErrorReport> {
        let istek = HttpIstek::get(BLAST_URL)
            .param("CMD", "Get")
            .param("FORMAT_OBJECT", "SearchInfo")
            .param("RID", rid);
        let govde = self.gonder_istek(&istek, baglam)?;
        Ok(qblast_durum_ayristir(&govde))
    }

    /// Hazır işin **sonuç hizalamalarını** getirir (CMD=Get&FORMAT_TYPE=Tabular).
    pub fn sonuc_al(&self, rid: &str, baglam: &AramaBaglami) -> Result<Vec<Hizalama>, ErrorReport> {
        let istek = HttpIstek::get(BLAST_URL)
            .param("CMD", "Get")
            .param("FORMAT_TYPE", "Tabular")
            .param("RID", rid);
        let govde = self.gonder_istek(&istek, baglam)?;
        Ok(tabular_ayristir(&govde))
    }

    /// **Tam akış**: gönder → (iptal/ilerleme ile) yokla → hazır olunca sonuç al.  `is` ile ilerleme
    /// bildirilir ve iptal denetlenir (İP-21); zaman aşımında RID döndürerek devam önerir.
    pub fn blast_calistir(
        &self,
        dizi: &str,
        etiket: HassasiyetEtiketi,
        baglam: &AramaBaglami,
        is: &IsKulpu,
    ) -> Result<Vec<Hizalama>, ErrorReport> {
        is.ilerleme_bildir(Ilerleme::Belirsiz);
        if self.iptal_kontrol(is) {
            return Err(iptal_hatasi());
        }

        let isi = self.gonder(dizi, etiket, baglam)?;

        let mut hazir = false;
        for deneme in 0..self.yoklama.azami_yoklama {
            if self.iptal_kontrol(is) {
                return Err(iptal_hatasi());
            }
            match self.durum_yokla(&isi.rid, baglam)? {
                BlastDurum::Hazir => {
                    hazir = true;
                    break;
                }
                BlastDurum::Bekliyor => {
                    is.ilerleme_bildir(Ilerleme::Adim {
                        tamam: (deneme + 1) as u64,
                        toplam: self.yoklama.azami_yoklama as u64,
                    });
                    if !self.yoklama.aralik.is_zero() {
                        std::thread::sleep(self.yoklama.aralik);
                    }
                }
                BlastDurum::Bilinmiyor => {
                    let hata = rid_bilinmiyor(&isi.rid);
                    is.basarisiz(hata.ne_oldu.clone());
                    return Err(hata);
                }
            }
        }

        if !hazir {
            let hata = zaman_asimi(&isi.rid);
            is.basarisiz(hata.ne_oldu.clone());
            return Err(hata);
        }

        let hizalamalar = self.sonuc_al(&isi.rid, baglam)?;
        is.tamamla();
        Ok(hizalamalar)
    }

    fn iptal_kontrol(&self, is: &IsKulpu) -> bool {
        if is.iptal_mi() {
            is.iptal_tamam();
            true
        } else {
            false
        }
    }
}

/// Bir BLAST hizalamasını birleşik sonuç satırına çevirir (rozet + skor).
pub fn hizalama_sonucu(kaynak: &str, h: &Hizalama) -> AramaSonucu {
    AramaSonucu::yeni(
        kaynak,
        &h.konu_kimligi,
        &h.konu_kimligi,
        KayitTuru::Hizalama,
    )
    .with_uzunluk(h.hizalama_uzunlugu)
    .with_aciklama(format!(
        "Özdeşlik %{:.1} · uzunluk {} · E={:.1e}",
        h.ozdeslik_yuzde, h.hizalama_uzunlugu, h.e_deger
    ))
    .with_skor(SonucSkoru {
        bit_skoru: h.bit_skoru,
        e_deger: h.e_deger,
        ozdeslik_yuzde: h.ozdeslik_yuzde,
    })
}

impl VeritabaniKonektoru for BlastKonektor {
    fn kaynak_adi(&self) -> &str {
        &self.kaynak_adi
    }

    fn turler(&self) -> &[KayitTuru] {
        &self.turler
    }

    fn tarayici_url(&self, kimlik: &str) -> Option<String> {
        let k = kimlik.trim();
        if k.is_empty() {
            None
        } else {
            Some(format!(
                "{NCBI_ARAMA}{}",
                super::super::transport::yuzde_kodla(k)
            ))
        }
    }

    /// BLAST araması: `sorgu.metin` = dizi; iş gönderilir, yoklanır (kendi iş kulpu), sonuç hizalamaları
    /// birleşik sonuç satırlarına çevrilir.  (PHI sınırı için diziyi `Genel` kabul eder; PHI'li dizi
    /// üst katmanda işaretlenip [`gonder`](Self::gonder)'e doğru etiketle gönderilmelidir.)
    fn ara(
        &self,
        sorgu: &Sorgu,
        _sayfalama: Sayfalama,
        baglam: &AramaBaglami,
    ) -> Result<SonucListesi, ErrorReport> {
        let is = IsKulpu::yeni(format!("BLAST {}", self.program.ad()), None);
        let hizalamalar =
            self.blast_calistir(&sorgu.metin, HassasiyetEtiketi::Genel, baglam, &is)?;
        let sonuclar: Vec<AramaSonucu> = hizalamalar
            .iter()
            .map(|h| hizalama_sonucu(&self.kaynak_adi, h))
            .collect();
        let n = sonuclar.len() as u64;
        Ok(SonucListesi {
            sonuclar,
            sayfa: SayfaBilgisi {
                toplam: n,
                ofset: 0,
                limit: n.max(1),
            },
        })
    }

    fn detay(&self, kimlik: &str, _baglam: &AramaBaglami) -> Result<AramaSonucu, ErrorReport> {
        // BLAST konusu için ayrı bir özet çağrısı yapılmaz; satır zaten sonuç listesindedir.
        Ok(AramaSonucu::yeni(
            &self.kaynak_adi,
            kimlik,
            kimlik,
            KayitTuru::Hizalama,
        ))
    }

    fn getir(&self, _kimlik: &str, _baglam: &AramaBaglami) -> Result<GetirilenKayit, ErrorReport> {
        Err(ErrorReport::new(
            "BLAST sonucu doğrudan indirilemez",
            "bir BLAST hizalaması ham bir dosya değildir (konu accession'ı bir dizidir)",
            "Konuyu projeye eklemek için ilgili accession'ı NCBI konektörüyle getirin ya da 'tarayıcıda aç' kullanın",
        )
        .with_eylem("Tarayıcıda aç"))
    }
}

// ─── QBlastInfo + tabular ayrıştırma ─────────────────────────────────────────────

/// CMD=Put yanıtından RID + RTOE çıkarır.
fn qblast_is_ayristir(govde: &str) -> Result<BlastIsi, ErrorReport> {
    let mut rid: Option<String> = None;
    let mut rtoe: u64 = 0;
    for satir in govde.lines() {
        let t = satir.trim();
        if let Some(deger) = anahtar_deger(t, "RID") {
            if !deger.is_empty() {
                rid = Some(deger.to_string());
            }
        } else if let Some(deger) = anahtar_deger(t, "RTOE") {
            rtoe = deger.trim().parse().unwrap_or(0);
        }
    }
    match rid {
        Some(rid) => Ok(BlastIsi {
            rid,
            tahmini_sure_sn: rtoe,
        }),
        None => Err(ErrorReport::new(
            "BLAST işi başlatılamadı",
            "sunucu yanıtında bir RID (iş kimliği) bulunamadı",
            "Diziyi ve programı kontrol edip yeniden gönderin",
        )
        .with_teknik_detay(kisa(govde))),
    }
}

/// CMD=Get&SearchInfo yanıtından durumu çıkarır.
fn qblast_durum_ayristir(govde: &str) -> BlastDurum {
    for satir in govde.lines() {
        let t = satir.trim();
        if let Some(deger) = anahtar_deger(t, "Status") {
            return match deger.trim().to_ascii_uppercase().as_str() {
                "READY" => BlastDurum::Hazir,
                "WAITING" => BlastDurum::Bekliyor,
                _ => BlastDurum::Bilinmiyor,
            };
        }
    }
    BlastDurum::Bilinmiyor
}

/// `ANAHTAR = değer` veya `ANAHTAR=değer` satırından değeri ayıklar (boşluk toleranslı).
fn anahtar_deger<'a>(satir: &'a str, anahtar: &str) -> Option<&'a str> {
    let kalan = satir.strip_prefix(anahtar)?;
    let kalan = kalan.trim_start();
    let kalan = kalan.strip_prefix('=')?;
    Some(kalan.trim_start())
}

/// Tabular (`-outfmt 6`) gövdesini hizalamalara çevirir; `#` yorum + boş satırları atlar.
fn tabular_ayristir(govde: &str) -> Vec<Hizalama> {
    let mut cikti = Vec::new();
    for satir in govde.lines() {
        let t = satir.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let alanlar: Vec<&str> = if t.contains('\t') {
            t.split('\t').collect()
        } else {
            t.split_whitespace().collect()
        };
        if alanlar.len() < 12 {
            continue;
        }
        cikti.push(Hizalama {
            konu_kimligi: alanlar[1].to_string(),
            ozdeslik_yuzde: alanlar[2].parse().unwrap_or(0.0),
            hizalama_uzunlugu: alanlar[3].parse().unwrap_or(0),
            uyusmazlik: alanlar[4].parse().unwrap_or(0),
            bosluk_acilis: alanlar[5].parse().unwrap_or(0),
            sorgu_bas: alanlar[6].parse().unwrap_or(0),
            sorgu_bit: alanlar[7].parse().unwrap_or(0),
            konu_bas: alanlar[8].parse().unwrap_or(0),
            konu_bit: alanlar[9].parse().unwrap_or(0),
            e_deger: alanlar[10].parse().unwrap_or(0.0),
            bit_skoru: alanlar[11].parse().unwrap_or(0.0),
        });
    }
    cikti
}

fn kisa(s: &str) -> String {
    s.chars().take(200).collect()
}

// ─── Hatalar ─────────────────────────────────────────────────────────────────────

fn iptal_hatasi() -> ErrorReport {
    ErrorReport::new(
        "BLAST işi iptal edildi",
        "iş kullanıcı tarafından durduruldu",
        "Gerekirse yeniden başlatın",
    )
}

fn zaman_asimi(rid: &str) -> ErrorReport {
    ErrorReport::new(
        "BLAST işi zaman aşımına uğradı",
        format!("iş ('{rid}') ayrılan yoklama sayısında tamamlanmadı"),
        "Daha sonra aynı RID ile sonucu sorgulayabilirsiniz (iş sunucuda devam ediyor olabilir)",
    )
    .with_eylem("Daha sonra")
    .with_teknik_detay(format!("RID={rid}"))
}

fn rid_bilinmiyor(rid: &str) -> ErrorReport {
    ErrorReport::new(
        "BLAST işi durumu bilinmiyor",
        format!("sunucu '{rid}' için geçerli bir durum döndürmedi (RID süresi dolmuş olabilir)"),
        "İşi yeniden gönderin",
    )
    .with_teknik_detay(format!("RID={rid}"))
}

#[cfg(test)]
mod tests {
    use super::super::super::privacy::GizlilikKapisi;
    use super::super::super::transport::SahteUlastirici;
    use super::*;
    use biocraft_sdk::biocraft_types::JobStatus;

    fn put_yanit() -> &'static str {
        "<!--QBlastInfoBegin\n    RID = ABCD123XYZ\n    RTOE = 27\nQBlastInfoEnd-->"
    }
    fn hazir_yanit() -> &'static str {
        "<!--QBlastInfoBegin\n    Status=READY\nQBlastInfoEnd-->"
    }
    fn bekleyen_yanit() -> &'static str {
        "<!--QBlastInfoBegin\n    Status=WAITING\nQBlastInfoEnd-->"
    }
    fn tabular_yanit() -> &'static str {
        "# blastn\n# Query: q\n# 2 hits found\n\
         q\tNM_000546.6\t99.50\t200\t1\t0\t1\t200\t1\t200\t1e-100\t370\n\
         q\tXM_011544981.1\t98.00\t200\t4\t0\t1\t200\t5\t204\t1e-90\t350"
    }

    fn baglam_kur<'a>(u: &'a SahteUlastirici, g: &'a GizlilikKapisi) -> AramaBaglami<'a> {
        let mut b = AramaBaglami::yeni(u, g);
        b.yapi.geri_cekilme_taban = Duration::ZERO;
        b
    }

    #[test]
    fn put_yaniti_rid_rtoe_ayristirir() {
        let isi = qblast_is_ayristir(put_yanit()).unwrap();
        assert_eq!(isi.rid, "ABCD123XYZ");
        assert_eq!(isi.tahmini_sure_sn, 27);
    }

    #[test]
    fn durum_ayristirma() {
        assert_eq!(qblast_durum_ayristir(hazir_yanit()), BlastDurum::Hazir);
        assert_eq!(
            qblast_durum_ayristir(bekleyen_yanit()),
            BlastDurum::Bekliyor
        );
        assert_eq!(qblast_durum_ayristir("boş"), BlastDurum::Bilinmiyor);
    }

    #[test]
    fn tabular_iki_hizalama() {
        let h = tabular_ayristir(tabular_yanit());
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].konu_kimligi, "NM_000546.6");
        assert_eq!(h[0].ozdeslik_yuzde, 99.5);
        assert_eq!(h[0].hizalama_uzunlugu, 200);
        assert_eq!(h[0].bit_skoru, 370.0);
    }

    #[test]
    fn gonder_phi_engeller() {
        let u = SahteUlastirici::yeni().ekle("CMD=Put", 200, put_yanit());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = BlastKonektor::varsayilan(BlastProgram::Blastn);
        // PHI dizi → engel (onaylı olsa bile).
        let hata = kon
            .gonder("ACGTACGT", HassasiyetEtiketi::Phi, &baglam)
            .err()
            .unwrap();
        assert_eq!(hata.ne_oldu, "Hassas/PHI veri dış sorguya gönderilemez");
    }

    #[test]
    fn blast_calistir_tam_akis_is_tamamlanir() {
        let u = SahteUlastirici::yeni()
            .ekle("CMD=Put", 200, put_yanit())
            .ekle("FORMAT_OBJECT=SearchInfo", 200, hazir_yanit())
            .ekle("FORMAT_TYPE=Tabular", 200, tabular_yanit());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = BlastKonektor::varsayilan(BlastProgram::Blastn).with_yoklama(YoklamaAyari {
            azami_yoklama: 5,
            aralik: Duration::ZERO,
        });
        let is = IsKulpu::yeni("blast", None);

        let h = kon
            .blast_calistir("ACGTACGTACGT", HassasiyetEtiketi::Genel, &baglam, &is)
            .unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(is.durum(), JobStatus::Bitti);
    }

    #[test]
    fn blast_iptal_edilince_durur() {
        let u = SahteUlastirici::yeni()
            .ekle("CMD=Put", 200, put_yanit())
            .ekle("FORMAT_OBJECT=SearchInfo", 200, bekleyen_yanit());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = BlastKonektor::varsayilan(BlastProgram::Blastn).with_yoklama(YoklamaAyari {
            azami_yoklama: 5,
            aralik: Duration::ZERO,
        });
        let is = IsKulpu::yeni("blast", None);
        is.iptal_et(); // önceden iptal

        let hata = kon
            .blast_calistir("ACGT", HassasiyetEtiketi::Genel, &baglam, &is)
            .err()
            .unwrap();
        assert_eq!(hata.ne_oldu, "BLAST işi iptal edildi");
    }

    #[test]
    fn blast_zaman_asimi_rid_doner() {
        let u = SahteUlastirici::yeni()
            .ekle("CMD=Put", 200, put_yanit())
            .ekle("FORMAT_OBJECT=SearchInfo", 200, bekleyen_yanit());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = BlastKonektor::varsayilan(BlastProgram::Blastn).with_yoklama(YoklamaAyari {
            azami_yoklama: 2,
            aralik: Duration::ZERO,
        });
        let is = IsKulpu::yeni("blast", None);
        let hata = kon
            .blast_calistir("ACGT", HassasiyetEtiketi::Genel, &baglam, &is)
            .err()
            .unwrap();
        assert!(hata.ne_oldu.contains("zaman aşımına"));
        assert!(matches!(is.durum(), JobStatus::Hata { .. }));
    }

    #[test]
    fn ara_birlesik_sonuc_skorlu() {
        let u = SahteUlastirici::yeni()
            .ekle("CMD=Put", 200, put_yanit())
            .ekle("FORMAT_OBJECT=SearchInfo", 200, hazir_yanit())
            .ekle("FORMAT_TYPE=Tabular", 200, tabular_yanit());
        let g = GizlilikKapisi::onayli();
        let baglam = baglam_kur(&u, &g);
        let kon = BlastKonektor::varsayilan(BlastProgram::Blastn).with_yoklama(YoklamaAyari {
            azami_yoklama: 3,
            aralik: Duration::ZERO,
        });
        let liste = kon
            .ara(&Sorgu::metin("ACGT"), Sayfalama::ilk(20), &baglam)
            .unwrap();
        assert_eq!(liste.sonuclar.len(), 2);
        let s = &liste.sonuclar[0];
        assert_eq!(s.tur, KayitTuru::Hizalama);
        assert!(s.skor.is_some());
        assert_eq!(s.kaynak, "BLAST blastn");
    }
}
