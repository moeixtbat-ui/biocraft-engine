//! Global Memory Orchestrator — MK-21, MK-22, MK-09.
//!
//! **Tek bir bütçe, herkes rezervasyonla ister.**  UI, alt süreç (Python),
//! DuckDB ve eklentiler belleği doğrudan `malloc` ile değil, buradan **rezervasyonla**
//! alır.  Toplam bütçe aşılırsa talep **reddedilir** (panik/OOM çökmesi YOK — MK-22);
//! bunun yerine kullanıcıya gösterilebilir bir [`ErrorReport`] döner.
//!
//! Bellek baskısında (OS sinyali ya da yeni talep için yer açma) **boştaki önbellekler
//! LRU sırasıyla boşaltılır** (MK-21).  Önbellekler "yeniden üretilebilir" sayılır;
//! kullanılan (handle'lı) rezervasyonlar asla zorla boşaltılmaz.
//!
//! Tasarım: tüm durum `Arc<Mutex<…>>` arkasında; orkestratör ucuzca klonlanır
//! (her bileşen aynı bütçeyi paylaşır).  Rezervasyon ve önbellek tutamaçları
//! **RAII**'dir: düşürüldüklerinde (drop) baytları otomatik geri verirler.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use biocraft_types::ErrorReport;

use crate::birim::insan_bayt;

/// Belleği isteyen bileşenin türü — loglama, teşhis ve durum panelinde gösterim için.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BellekBileseni {
    /// Arayüz (egui/render ön yüz tarafı).
    Arayuz,
    /// Ayrı süreç olarak çalışan iş (MK-02: Python daima subprocess).
    Subprocess,
    /// Yerel analitik veritabanı (DuckDB/Arrow).
    VeriTabani,
    /// Bir eklenti (yayıncı/eklenti kimliği etiketiyle).
    Eklenti(String),
    /// Render/GPU tarafı CPU yan-tamponları.
    Render,
    /// Diğer / serbest etiketli bileşen.
    Diger(String),
}

impl BellekBileseni {
    /// İnsan-okunur kısa ad (durum paneli / log için).
    pub fn ad(&self) -> String {
        match self {
            BellekBileseni::Arayuz => "Arayüz".to_string(),
            BellekBileseni::Subprocess => "Alt süreç".to_string(),
            BellekBileseni::VeriTabani => "Veritabanı".to_string(),
            BellekBileseni::Eklenti(a) => format!("Eklenti ({a})"),
            BellekBileseni::Render => "Render".to_string(),
            BellekBileseni::Diger(a) => a.clone(),
        }
    }
}

// ─── İç durum (Mutex arkasında) ──────────────────────────────────────────────

struct RezervasyonKaydi {
    bilesen: BellekBileseni,
    bayt: u64,
}

struct OnbellekKaydi {
    bilesen: BellekBileseni,
    bayt: u64,
    /// LRU için monotonik "son erişim" damgası; küçük = daha eski.
    son_erisim: u64,
}

struct OrkestratorIc {
    toplam_butce: u64,
    rezerve: u64,
    handles: HashMap<u64, RezervasyonKaydi>,
    onbellekler: HashMap<u64, OnbellekKaydi>,
    /// id üretici (rezervasyon + önbellek ortak).
    sayac: u64,
    /// LRU monotonik saati.
    saat: u64,
}

impl OrkestratorIc {
    fn yeni_id(&mut self) -> u64 {
        self.sayac += 1;
        self.sayac
    }

    fn bos(&self) -> u64 {
        self.toplam_butce.saturating_sub(self.rezerve)
    }

    fn handle_birak(&mut self, id: u64) {
        if let Some(k) = self.handles.remove(&id) {
            self.rezerve = self.rezerve.saturating_sub(k.bayt);
        }
    }

    fn onbellek_birak(&mut self, id: u64) {
        if let Some(k) = self.onbellekler.remove(&id) {
            self.rezerve = self.rezerve.saturating_sub(k.bayt);
        }
    }

    /// LRU sırasıyla (en eski önce) önbellekleri boşaltarak en az `gereken` bayt
    /// açmaya çalışır.  Boşaltılan toplam baytı ve boşaltılan önbellek id'lerini döndürür.
    /// `gereken` çok büyükse (örn. `u64::MAX`) tüm önbellekler boşaltılır (agresif temizleme).
    fn lru_bosalt(&mut self, gereken: u64) -> (u64, Vec<u64>) {
        let mut bosalan: u64 = 0;
        let mut bosaltilan: Vec<u64> = Vec::new();
        while bosalan < gereken {
            // En eski (en küçük son_erisim) önbelleği seç.
            let en_eski = self
                .onbellekler
                .iter()
                .min_by_key(|(_, k)| k.son_erisim)
                .map(|(id, _)| *id);
            match en_eski {
                Some(id) => {
                    if let Some(k) = self.onbellekler.remove(&id) {
                        self.rezerve = self.rezerve.saturating_sub(k.bayt);
                        bosalan = bosalan.saturating_add(k.bayt);
                        bosaltilan.push(id);
                    }
                }
                None => break, // Boşaltacak önbellek kalmadı.
            }
        }
        (bosalan, bosaltilan)
    }
}

// ─── Genel API ───────────────────────────────────────────────────────────────

/// Global bellek orkestratörü.  Ucuzca klonlanır; tüm klonlar aynı bütçeyi paylaşır.
#[derive(Clone)]
pub struct BellekOrkestratoru {
    ic: Arc<Mutex<OrkestratorIc>>,
}

/// Durum panelinde / status bar'da göstermek için anlık bellek görüntüsü.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BellekDurumu {
    /// Toplam bellek bütçesi (bayt).
    pub toplam_butce: u64,
    /// Şu an rezerve edilmiş toplam (bayt).
    pub rezerve: u64,
    /// Boş (bayt) = toplam − rezerve.
    pub bos: u64,
    /// Aktif (handle'lı) rezervasyon adedi.
    pub handle_adet: usize,
    /// Boşaltılabilir önbellek adedi.
    pub onbellek_adet: usize,
}

impl BellekDurumu {
    /// Doluluk oranı 0.0–1.0.
    pub fn doluluk(&self) -> f32 {
        if self.toplam_butce == 0 {
            0.0
        } else {
            (self.rezerve as f64 / self.toplam_butce as f64) as f32
        }
    }
}

/// Bir boşaltma (eviction) turunun özeti.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BosaltmaOzeti {
    /// Boşaltılan toplam bayt.
    pub bosaltilan_bayt: u64,
    /// Boşaltılan önbellek adedi.
    pub bosaltilan_adet: usize,
}

impl BellekOrkestratoru {
    /// Verilen toplam bütçeyle (bayt) yeni bir orkestratör kurar.
    pub fn yeni(toplam_butce: u64) -> Self {
        Self {
            ic: Arc::new(Mutex::new(OrkestratorIc {
                toplam_butce,
                rezerve: 0,
                handles: HashMap::new(),
                onbellekler: HashMap::new(),
                sayac: 0,
                saat: 0,
            })),
        }
    }

    /// Toplam bütçe (bayt).
    pub fn toplam_butce(&self) -> u64 {
        self.ic.lock().unwrap().toplam_butce
    }

    /// Şu an rezerve edilmiş toplam (bayt).
    pub fn rezerve_edilen(&self) -> u64 {
        self.ic.lock().unwrap().rezerve
    }

    /// Boş bayt = toplam − rezerve.
    pub fn bos(&self) -> u64 {
        self.ic.lock().unwrap().bos()
    }

    /// Anlık durum görüntüsü (UI/status bar için).
    pub fn durum(&self) -> BellekDurumu {
        let ic = self.ic.lock().unwrap();
        BellekDurumu {
            toplam_butce: ic.toplam_butce,
            rezerve: ic.rezerve,
            bos: ic.bos(),
            handle_adet: ic.handles.len(),
            onbellek_adet: ic.onbellekler.len(),
        }
    }

    /// **Bileşen bazında rezerve dökümü** (handle + önbellek toplamı) — teşhis ve durum
    /// paneli için.  "Hangi bileşen ne kadar bellek tutuyor?" sorusunu yanıtlar; büyükten
    /// küçüğe sıralı döner.
    pub fn bilesen_dokumu(&self) -> Vec<(BellekBileseni, u64)> {
        let ic = self.ic.lock().unwrap();
        let mut harita: HashMap<String, (BellekBileseni, u64)> = HashMap::new();
        for k in ic.handles.values() {
            let giris = harita
                .entry(k.bilesen.ad())
                .or_insert((k.bilesen.clone(), 0));
            giris.1 += k.bayt;
        }
        for k in ic.onbellekler.values() {
            let giris = harita
                .entry(k.bilesen.ad())
                .or_insert((k.bilesen.clone(), 0));
            giris.1 += k.bayt;
        }
        let mut dokum: Vec<(BellekBileseni, u64)> = harita.into_values().collect();
        dokum.sort_by_key(|(_, bayt)| std::cmp::Reverse(*bayt)); // büyükten küçüğe
        dokum
    }

    /// **Rezervasyon talep et (MK-21).**  Yer varsa bir [`Rezervasyon`] döner; yer
    /// açmak için önce boştaki önbellekler LRU ile boşaltılır.  Yine de sığmıyorsa
    /// **panik değil**, kullanıcıya gösterilebilir bir [`ErrorReport`] döner (MK-22).
    pub fn rezerve_et(
        &self,
        bilesen: BellekBileseni,
        bayt: u64,
    ) -> Result<Rezervasyon, ErrorReport> {
        let mut ic = self.ic.lock().unwrap();

        // Tek bir talep bütçenin tamamından büyükse hiçbir boşaltma kurtaramaz.
        if bayt > ic.toplam_butce {
            return Err(yetersiz_hata(bayt, ic.bos(), ic.toplam_butce, true));
        }
        // Yer yoksa, önce boştaki önbellekleri LRU ile boşaltmayı dene.
        if bayt > ic.bos() {
            let gereken = bayt - ic.bos();
            let _ = ic.lru_bosalt(gereken);
        }
        // Boşaltmadan sonra hâlâ sığmıyorsa: reddet (OOM çökmesi yok).
        if bayt > ic.bos() {
            return Err(yetersiz_hata(bayt, ic.bos(), ic.toplam_butce, false));
        }

        let id = ic.yeni_id();
        ic.handles.insert(id, RezervasyonKaydi { bilesen, bayt });
        ic.rezerve += bayt;
        Ok(Rezervasyon {
            id,
            bayt,
            ic: Arc::downgrade(&self.ic),
        })
    }

    /// **Boşaltılabilir önbellek kaydı (MK-21).**  Rezervasyon gibi yer tutar ama
    /// bellek baskısında / yeni talep için LRU ile **zorla boşaltılabilir.**  Tutamaç
    /// düşürülünce de yer geri verilir.  Yine sığmazsa [`ErrorReport`] döner.
    pub fn onbellek_ekle(
        &self,
        bilesen: BellekBileseni,
        bayt: u64,
    ) -> Result<OnbellekTutamac, ErrorReport> {
        let mut ic = self.ic.lock().unwrap();

        if bayt > ic.toplam_butce {
            return Err(yetersiz_hata(bayt, ic.bos(), ic.toplam_butce, true));
        }
        if bayt > ic.bos() {
            let gereken = bayt - ic.bos();
            let _ = ic.lru_bosalt(gereken);
        }
        if bayt > ic.bos() {
            return Err(yetersiz_hata(bayt, ic.bos(), ic.toplam_butce, false));
        }

        let id = ic.yeni_id();
        let saat = {
            ic.saat += 1;
            ic.saat
        };
        ic.onbellekler.insert(
            id,
            OnbellekKaydi {
                bilesen,
                bayt,
                son_erisim: saat,
            },
        );
        ic.rezerve += bayt;
        Ok(OnbellekTutamac {
            id,
            bayt,
            ic: Arc::downgrade(&self.ic),
        })
    }

    /// Bir önbelleğin "kullanıldığını" işaretler (LRU tazeleme) — en son dokunulan
    /// önbellek en son boşaltılır.
    pub fn onbellek_dokun(&self, tutamac: &OnbellekTutamac) {
        let mut ic = self.ic.lock().unwrap();
        let saat = {
            ic.saat += 1;
            ic.saat
        };
        if let Some(k) = ic.onbellekler.get_mut(&tutamac.id) {
            k.son_erisim = saat;
        }
    }

    /// **Bellek baskısı (OS sinyali) — agresif temizleme (MK-21).**  Tüm boştaki
    /// önbellekleri LRU sırasıyla boşaltır ve ne kadar yer açıldığını döndürür.
    /// Handle'lı (kullanılan) rezervasyonlara dokunulmaz.
    pub fn bellek_baskisi(&self) -> BosaltmaOzeti {
        let mut ic = self.ic.lock().unwrap();
        let (bayt, ids) = ic.lru_bosalt(u64::MAX);
        BosaltmaOzeti {
            bosaltilan_bayt: bayt,
            bosaltilan_adet: ids.len(),
        }
    }
}

// ─── RAII tutamaçlar ─────────────────────────────────────────────────────────

/// Aktif bir bellek rezervasyonu.  Düşürülünce (drop) ayrılan bayt otomatik geri verilir.
pub struct Rezervasyon {
    id: u64,
    bayt: u64,
    ic: Weak<Mutex<OrkestratorIc>>,
}

impl Rezervasyon {
    /// Bu rezervasyonun bayt boyutu.
    pub fn bayt(&self) -> u64 {
        self.bayt
    }
}

impl std::fmt::Debug for Rezervasyon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rezervasyon({} bayt)", self.bayt)
    }
}

impl Drop for Rezervasyon {
    fn drop(&mut self) {
        if let Some(arc) = self.ic.upgrade() {
            if let Ok(mut ic) = arc.lock() {
                ic.handle_birak(self.id);
            }
        }
    }
}

/// Boşaltılabilir bir önbellek rezervasyonu.  Düşürülünce yer geri verilir; ayrıca
/// orkestratör bellek baskısında bunu **zorla** boşaltabilir (bkz. [`OnbellekTutamac::canli`]).
pub struct OnbellekTutamac {
    id: u64,
    bayt: u64,
    ic: Weak<Mutex<OrkestratorIc>>,
}

impl OnbellekTutamac {
    /// Bu önbelleğin bayt boyutu.
    pub fn bayt(&self) -> u64 {
        self.bayt
    }

    /// Önbellek hâlâ canlı mı?  `false` ise orkestratör onu (bellek baskısı/LRU ile)
    /// boşaltmıştır; bileşen verisini yeniden üretmelidir.
    pub fn canli(&self) -> bool {
        match self.ic.upgrade() {
            Some(arc) => arc
                .lock()
                .map(|ic| ic.onbellekler.contains_key(&self.id))
                .unwrap_or(false),
            None => false,
        }
    }
}

impl std::fmt::Debug for OnbellekTutamac {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OnbellekTutamac({} bayt)", self.bayt)
    }
}

impl Drop for OnbellekTutamac {
    fn drop(&mut self) {
        if let Some(arc) = self.ic.upgrade() {
            if let Ok(mut ic) = arc.lock() {
                ic.onbellek_birak(self.id);
            }
        }
    }
}

// ─── Hata üretimi (MK-22, standart şema İP-16) ───────────────────────────────

/// Bütçe aşımında kullanıcıya gösterilecek standart hata raporu.
fn yetersiz_hata(istenen: u64, bos: u64, toplam: u64, tek_basina_buyuk: bool) -> ErrorReport {
    let neden = if tek_basina_buyuk {
        format!(
            "İstenen {} tek başına toplam bellek bütçesinden ({}) büyük.",
            insan_bayt(istenen),
            insan_bayt(toplam)
        )
    } else {
        format!(
            "Bellek bütçesi neredeyse dolu: istenen {}, boşta yalnızca {} var (toplam {}).",
            insan_bayt(istenen),
            insan_bayt(bos),
            insan_bayt(toplam)
        )
    };
    ErrorReport::new(
        "Bellek ayrılamadı",
        neden,
        "Açık işlemleri/dosyaları kapatın, dosyayı akış (stream) modunda açın ya da bellek bütçesini artırın.",
    )
    .with_eylem("Akış modunda aç")
    .with_teknik_detay(format!(
        "rezervasyon reddi: istenen={istenen}B bos={bos}B toplam={toplam}B"
    ))
}

// ─── Testler ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const MB: u64 = 1024 * 1024;

    #[test]
    fn basit_rezervasyon_ve_serbest_birakma() {
        let ork = BellekOrkestratoru::yeni(100 * MB);
        assert_eq!(ork.bos(), 100 * MB);

        let r = ork.rezerve_et(BellekBileseni::Arayuz, 40 * MB).unwrap();
        assert_eq!(ork.rezerve_edilen(), 40 * MB);
        assert_eq!(ork.bos(), 60 * MB);
        assert_eq!(r.bayt(), 40 * MB);

        drop(r); // RAII: düşünce geri verilmeli
        assert_eq!(ork.rezerve_edilen(), 0);
        assert_eq!(ork.bos(), 100 * MB);
    }

    #[test]
    fn butce_asiminda_panik_degil_reddetme_uretir() {
        // MK-22: OOM çökmesi YOK — talep reddedilir + ErrorReport döner.
        let ork = BellekOrkestratoru::yeni(100 * MB);
        let _r = ork.rezerve_et(BellekBileseni::Subprocess, 80 * MB).unwrap();

        let sonuc = ork.rezerve_et(BellekBileseni::Subprocess, 30 * MB);
        assert!(sonuc.is_err(), "Bütçe aşımı reddedilmeliydi");

        let hata = sonuc.unwrap_err();
        // Standart hata şeması (İP-16): üç alan dolu.
        assert!(!hata.ne_oldu.is_empty());
        assert!(!hata.neden.is_empty());
        assert!(!hata.nasil_cozulur.is_empty());

        // Reddedilen talep bütçeyi DEĞİŞTİRMEMELİ (tutarlılık).
        assert_eq!(ork.rezerve_edilen(), 80 * MB);
    }

    #[test]
    fn tek_basina_butceden_buyuk_talep_reddedilir() {
        let ork = BellekOrkestratoru::yeni(50 * MB);
        let sonuc = ork.rezerve_et(BellekBileseni::VeriTabani, 60 * MB);
        assert!(sonuc.is_err());
        assert!(sonuc.unwrap_err().neden.contains("tek başına"));
    }

    #[test]
    fn yeni_talep_icin_lru_onbellek_bosaltilir() {
        // MK-21: yer açmak için boştaki önbellekler LRU ile boşaltılır.
        let ork = BellekOrkestratoru::yeni(100 * MB);

        let eski = ork.onbellek_ekle(BellekBileseni::Render, 40 * MB).unwrap();
        let yeni = ork.onbellek_ekle(BellekBileseni::Render, 40 * MB).unwrap();
        // "eski" en eski; ama "yeni"den sonra "eski"ye dokunarak onu tazeleyelim →
        // artık "yeni" en eski (LRU kurbanı) olur.
        ork.onbellek_dokun(&eski);

        // 80 MB önbellek dolu, 20 MB boş. 50 MB handle isteyince LRU boşaltma şart.
        let _h = ork
            .rezerve_et(BellekBileseni::Subprocess, 50 * MB)
            .expect("LRU boşaltma ile sığmalıydı");

        // En son dokunulan "eski" canlı kalmalı; "yeni" boşaltılmış olmalı.
        assert!(eski.canli(), "En son kullanılan önbellek korunmalıydı");
        assert!(!yeni.canli(), "En eski önbellek (LRU) boşaltılmalıydı");
    }

    #[test]
    fn bellek_baskisi_tum_onbellekleri_bosaltir_handle_kalir() {
        let ork = BellekOrkestratoru::yeni(100 * MB);
        let _handle = ork.rezerve_et(BellekBileseni::Arayuz, 20 * MB).unwrap();
        let c1 = ork.onbellek_ekle(BellekBileseni::Render, 30 * MB).unwrap();
        let c2 = ork
            .onbellek_ekle(BellekBileseni::VeriTabani, 25 * MB)
            .unwrap();

        let ozet = ork.bellek_baskisi();
        assert_eq!(ozet.bosaltilan_bayt, 55 * MB);
        assert_eq!(ozet.bosaltilan_adet, 2);
        assert!(!c1.canli() && !c2.canli());

        // Handle (kullanılan) rezervasyon korunur.
        assert_eq!(ork.rezerve_edilen(), 20 * MB);
    }

    #[test]
    fn bosaltilan_onbellek_drop_olunca_cift_dusum_yok() {
        // Önce orkestratör boşalttı; sonra tutamaç drop olursa bayt iki kez düşmemeli.
        let ork = BellekOrkestratoru::yeni(100 * MB);
        let c = ork.onbellek_ekle(BellekBileseni::Render, 30 * MB).unwrap();
        ork.bellek_baskisi();
        assert_eq!(ork.rezerve_edilen(), 0);
        drop(c); // no-op olmalı
        assert_eq!(ork.rezerve_edilen(), 0);
    }

    #[test]
    fn durum_anlik_goruntu_dogru() {
        let ork = BellekOrkestratoru::yeni(200 * MB);
        let _h = ork.rezerve_et(BellekBileseni::Arayuz, 50 * MB).unwrap();
        let _c = ork.onbellek_ekle(BellekBileseni::Render, 50 * MB).unwrap();
        let d = ork.durum();
        assert_eq!(d.toplam_butce, 200 * MB);
        assert_eq!(d.rezerve, 100 * MB);
        assert_eq!(d.bos, 100 * MB);
        assert_eq!(d.handle_adet, 1);
        assert_eq!(d.onbellek_adet, 1);
        assert!((d.doluluk() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn bilesen_dokumu_bileşen_basina_toplar() {
        let ork = BellekOrkestratoru::yeni(200 * MB);
        let _a = ork.rezerve_et(BellekBileseni::VeriTabani, 60 * MB).unwrap();
        let _b = ork.rezerve_et(BellekBileseni::Arayuz, 20 * MB).unwrap();
        let _c = ork
            .onbellek_ekle(BellekBileseni::VeriTabani, 10 * MB)
            .unwrap();

        let dokum = ork.bilesen_dokumu();
        // En büyük tüketici Veritabanı (60+10=70 MB), sonra Arayüz (20 MB).
        assert_eq!(dokum[0].0, BellekBileseni::VeriTabani);
        assert_eq!(dokum[0].1, 70 * MB);
        assert_eq!(dokum[1].0, BellekBileseni::Arayuz);
        assert_eq!(dokum[1].1, 20 * MB);
    }

    #[test]
    fn cok_threadli_rezervasyon_butceyi_asmaz() {
        // MK-11: eşzamanlı talepler altında bile toplam rezerve bütçeyi aşmamalı,
        // panik olmamalı.  (Loom ile derinlemesine model-kontrol ileride — TODO.)
        use std::thread;

        let ork = BellekOrkestratoru::yeni(10 * MB);
        let mut tutamaclar = Vec::new();
        let mut isciler = Vec::new();

        // 100 iş parçacığı 1'er MB istemeye çalışsın; en çok 10'u başarmalı.
        let (gonder, al) = std::sync::mpsc::channel();
        for _ in 0..100 {
            let o = ork.clone();
            let g = gonder.clone();
            isciler.push(thread::spawn(move || {
                if let Ok(r) = o.rezerve_et(BellekBileseni::Diger("t".into()), MB) {
                    g.send(r).unwrap();
                }
            }));
        }
        drop(gonder);
        for i in isciler {
            i.join().unwrap();
        }
        while let Ok(r) = al.recv() {
            tutamaclar.push(r);
        }

        assert!(ork.rezerve_edilen() <= 10 * MB, "Bütçe aşılmamalı");
        assert_eq!(tutamaclar.len(), 10, "Tam olarak 10 talep başarmalı");
    }
}
