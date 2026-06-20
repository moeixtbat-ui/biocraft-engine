//! Bağımsız donanım izleme watchdog'u + sensör soyutlaması (Zero-Impact) — İP-08, MK-24.
//!
//! **Yüksek öncelikli, ayrı bir izleme thread'i** donanım sağlığını sürekli yoklar:
//! GPU/CPU/NVMe sıcaklığı, kullanım yüzdesi, fan/RPM (varsa).  [`crate::thermal`] tablosuna
//! göre iş yükü kademeli azaltılır, kritikte durdurulur ve **duraklamaya geçişte checkpoint
//! alınır** (veri kaybı yok); soğuyunca otomatik devam edilir.
//!
//! ## Bağımsızlık (MK-24)
//! Watchdog ayrı bir [`std::thread`]'de döner — ana arayüz/render döngüsüne **bağlı değildir**;
//! ana iş thread'i takılsa/panik etse bile izleme thread'i çalışmaya devam eder ve acil
//! durdurmayı tetikleyebilir.  *Tam süreç-üstü denetim (ana süreç tümüyle çökse de yaşayan
//! ayrı bir watchdog **süreci**) v1.x kapsamındadır* (// TODO(MK-24): ayrı süreç supervizör).
//!
//! ## Zarif bozulma (sensör yok)
//! Sıcaklık sensörü okunamayan donanımda (ör. yetki yok / desteklenmeyen platform) koruma
//! **kademeli devre dışı** bırakılır ve kullanıcı bilgilendirilir — uygulama **çökmez**
//! (spec: "Sensör okunamayan donanımda koruma kademeli devre dışı + bilgi").
//!
//! ## Soyutlama
//! Tüm okuma [`DonanimSensoru`] arkasındadır: gerçek ([`SistemSensoru`], sysinfo), sahte
//! ([`BetikSensor`]/[`SabitSensor`], testler) ve boş ([`SensorYok`], sensörsüz senaryo).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::thermal::{en_kotu_aksiyon, DonanimParca, TermalAksiyon, TermalEsikler};

/// Tek bir donanım okuması (örnek).  Her alan `Option`'dır: okunamayan değer `None`
/// (panik/çökme YOK).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DonanimOrnegi {
    /// GPU sıcaklığı (°C).
    pub gpu_c: Option<f32>,
    /// CPU sıcaklığı (°C).
    pub cpu_c: Option<f32>,
    /// NVMe sıcaklığı (°C).
    pub nvme_c: Option<f32>,
    /// CPU kullanım yüzdesi (0–100).
    pub cpu_yuzde: Option<f32>,
    /// Toplam RAM kullanım oranı (0.0–1.0).
    pub ram_orani: Option<f32>,
    /// En yüksek fan devri (RPM), varsa.
    pub fan_rpm: Option<u32>,
}

impl DonanimOrnegi {
    /// En az bir sıcaklık okunabildi mi?  `false` ise koruma kademeli devre dışı bırakılır.
    pub fn sicaklik_var(&self) -> bool {
        self.gpu_c.is_some() || self.cpu_c.is_some() || self.nvme_c.is_some()
    }
}

/// Donanım sensörü soyutlaması.  `oku` **asla panik etmez**; okunamayan değerler `None` döner.
pub trait DonanimSensoru: Send {
    /// Anlık bir donanım örneği okur.
    fn oku(&mut self) -> DonanimOrnegi;
}

/// Hiç sensör olmayan donanımı temsil eder — her zaman boş örnek döner (zarif bozulma testi).
#[derive(Debug, Default, Clone, Copy)]
pub struct SensorYok;

impl DonanimSensoru for SensorYok {
    fn oku(&mut self) -> DonanimOrnegi {
        DonanimOrnegi::default()
    }
}

/// Sabit bir örnek döndüren sensör (testler / sabit senaryo için).
#[derive(Debug, Clone, Copy)]
pub struct SabitSensor(pub DonanimOrnegi);

impl DonanimSensoru for SabitSensor {
    fn oku(&mut self) -> DonanimOrnegi {
        self.0
    }
}

/// Önceden yazılmış bir örnek dizisini sırayla oynatan sensör (ısınma→soğuma simülasyonu).
/// Dizi bittiğinde **son örnek** tekrar eder (kararlı son durum).
#[derive(Debug, Clone)]
pub struct BetikSensor {
    ornekler: Vec<DonanimOrnegi>,
    idx: usize,
}

impl BetikSensor {
    /// Verilen örnek dizisinden bir betik sensör kurar.
    pub fn yeni(ornekler: Vec<DonanimOrnegi>) -> Self {
        Self { ornekler, idx: 0 }
    }
}

impl DonanimSensoru for BetikSensor {
    fn oku(&mut self) -> DonanimOrnegi {
        if self.ornekler.is_empty() {
            return DonanimOrnegi::default();
        }
        let o = self.ornekler[self.idx.min(self.ornekler.len() - 1)];
        if self.idx < self.ornekler.len() - 1 {
            self.idx += 1;
        }
        o
    }
}

/// Duraklamaya geçişte çağrılan **checkpoint kancası** — veri kaybını önler.  İş katmanı
/// burada açık dosyaları/ara sonuçları diske yazar.  MVP'de basit bir geri-çağrı.
pub type CheckpointKanca = Arc<dyn Fn() + Send + Sync>;

/// Hiçbir şey yapmayan checkpoint kancası (test/varsayılan).
pub fn bos_checkpoint() -> CheckpointKanca {
    Arc::new(|| {})
}

/// Watchdog'un paylaşılan durumu — arayüz (status bar) bunu okur.
#[derive(Debug, Clone, PartialEq)]
pub struct KoruyucuDurum {
    /// Son okunan örnek.
    pub son_ornek: DonanimOrnegi,
    /// Geçerli termal aksiyon.
    pub aksiyon: TermalAksiyon,
    /// Koruma etkin mi?  Sensör okunamıyorsa `false` (kademeli devre dışı).
    pub koruma_etkin: bool,
    /// "Soğutuluyor" rozeti gösterilmeli mi? (duraklatıldı, soğuyunca devam).
    pub sogutuluyor: bool,
    /// Acil durdurma etkin mi? (kritik sıcaklık).
    pub acil_durum: bool,
    /// Bugüne dek alınan checkpoint sayısı (duraklama geçişlerinde artar).
    pub checkpoint_sayisi: u32,
    /// Aksiyonu tetikleyen parça (varsa).
    pub tetikleyen: Option<DonanimParca>,
    /// Kullanıcıya gösterilecek bilgi (ör. sensör yok uyarısı); yoksa `None`.
    pub bilgi: Option<String>,
}

impl Default for KoruyucuDurum {
    fn default() -> Self {
        Self {
            son_ornek: DonanimOrnegi::default(),
            aksiyon: TermalAksiyon::TamKapasite,
            koruma_etkin: true,
            sogutuluyor: false,
            acil_durum: false,
            checkpoint_sayisi: 0,
            tetikleyen: None,
            bilgi: None,
        }
    }
}

/// **Bir izleme adımı (saf çekirdek).**  Örneği değerlendirip durumu günceller; duraklamaya
/// **yeni** geçişte checkpoint kancasını çağırır.  Watchdog thread'i ve testler bunu kullanır.
fn adim_uygula(
    durum: &mut KoruyucuDurum,
    ornek: DonanimOrnegi,
    esikler: &TermalEsikler,
    checkpoint: &CheckpointKanca,
) {
    durum.son_ornek = ornek;

    // Sensör yok / okunamadı → korumayı zarifçe devre dışı bırak, bilgilendir, ÇÖKME yok.
    if !ornek.sicaklik_var() {
        durum.koruma_etkin = false;
        // Ölçemediğimiz donanımı kısamayız → tam kapasite.
        durum.aksiyon = TermalAksiyon::TamKapasite;
        durum.sogutuluyor = false;
        durum.acil_durum = false;
        durum.tetikleyen = None;
        durum.bilgi = Some(
            "Sıcaklık sensörü okunamıyor — donanım koruma kademeli devre dışı. \
             Uygulama normal çalışır; cihazınız çok ısınırsa kendiniz ara verin."
                .to_string(),
        );
        return;
    }

    durum.koruma_etkin = true;
    durum.bilgi = None;

    // Histerezis: bir önceki adımda duraklı mıydık?
    let onceki_duraklatildi = durum.aksiyon.duraklatir();

    let aksiyonlar = [
        (DonanimParca::Gpu, ornek.gpu_c),
        (DonanimParca::Cpu, ornek.cpu_c),
        (DonanimParca::Nvme, ornek.nvme_c),
    ]
    .into_iter()
    .filter_map(|(parca, temp)| {
        temp.map(|t| {
            (
                parca,
                esikler.aksiyon_histerezisli(parca, t, onceki_duraklatildi),
            )
        })
    });

    let (en_kotu, tetik) = en_kotu_aksiyon(aksiyonlar);

    // Duraklamaya YENİ geçişte checkpoint al (veri kaybı yok).
    if en_kotu.duraklatir() && !onceki_duraklatildi {
        (checkpoint)();
        durum.checkpoint_sayisi += 1;
    }

    durum.sogutuluyor = en_kotu.sogutuluyor_mu();
    durum.acil_durum = en_kotu.acil_mi();
    durum.aksiyon = en_kotu;
    durum.tetikleyen = tetik;
}

/// Bağımsız donanım izleme watchdog'u.  [`DonanimMuhafiz::baslat`] ile ayrı bir thread başlatır;
/// [`DonanimMuhafiz::durum`] anlık durumu verir; düşürülünce (drop) thread temiz durdurulur.
pub struct DonanimMuhafiz {
    durum: Arc<Mutex<KoruyucuDurum>>,
    calisiyor: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl DonanimMuhafiz {
    /// **Watchdog'u başlat.**  `sensor` ayrı thread'e taşınır; `aralik` her yoklama arası süredir.
    /// Duraklamaya geçişte `checkpoint` çağrılır.
    pub fn baslat(
        mut sensor: Box<dyn DonanimSensoru>,
        esikler: TermalEsikler,
        aralik: Duration,
        checkpoint: CheckpointKanca,
    ) -> Self {
        let durum = Arc::new(Mutex::new(KoruyucuDurum::default()));
        let calisiyor = Arc::new(AtomicBool::new(true));

        let durum_t = Arc::clone(&durum);
        let calisiyor_t = Arc::clone(&calisiyor);

        // MK-24: ayrı, ana döngüden bağımsız bir thread.  (İsim ile teşhis kolaylaşır;
        // gerçek OS thread-önceliği yükseltmesi platforma özgüdür — v1.x.)
        let join = std::thread::Builder::new()
            .name("biocraft-watchdog".to_string())
            .spawn(move || {
                while calisiyor_t.load(Ordering::Acquire) {
                    let ornek = sensor.oku();
                    if let Ok(mut d) = durum_t.lock() {
                        adim_uygula(&mut d, ornek, &esikler, &checkpoint);
                    }
                    // Durdurma sinyaline hızlı yanıt için aralığı küçük dilimlere böl.
                    uyu_bolerek(aralik, &calisiyor_t);
                }
            })
            .expect("watchdog thread'i başlatılamadı");

        Self {
            durum,
            calisiyor,
            join: Some(join),
        }
    }

    /// Anlık koruyucu durumun bir kopyası (UI/status bar için).
    pub fn durum(&self) -> KoruyucuDurum {
        self.durum.lock().unwrap().clone()
    }

    /// Geçerli yük oranı (0.0–1.0) — iş katmanı ağır işi buna göre kısar.
    pub fn yuk_orani(&self) -> f32 {
        self.durum.lock().unwrap().aksiyon.yuk_orani()
    }

    /// Watchdog'u temiz durdurur (thread'i bekler).  Tekrar çağrı güvenlidir.
    pub fn durdur(&mut self) {
        self.calisiyor.store(false, Ordering::Release);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for DonanimMuhafiz {
    fn drop(&mut self) {
        self.durdur();
    }
}

/// `aralik` kadar uyur ama her 50 ms'de bir durdurma sinyalini yoklar (hızlı kapanış).
fn uyu_bolerek(aralik: Duration, calisiyor: &AtomicBool) {
    let dilim = Duration::from_millis(50);
    let mut kalan = aralik;
    while kalan > Duration::ZERO && calisiyor.load(Ordering::Acquire) {
        let u = kalan.min(dilim);
        std::thread::sleep(u);
        kalan = kalan.saturating_sub(u);
    }
}

// ─── Gerçek sensör (sysinfo) ─────────────────────────────────────────────────

/// **Gerçek** donanım sensörü (sysinfo tabanlı).  CPU kullanımı, RAM oranı ve (platform
/// destekliyorsa) bileşen sıcaklıklarını okur.  Çoğu Windows kurulumunda bileşen sıcaklığı
/// yetki/sürücü olmadan **boş** gelir → bu durumda koruma zarifçe devre dışı kalır.
///
/// `simule_gpu`: demo/simülasyon kancası — `Some(t)` iken GPU sıcaklığı bu değere **zorlanır**
/// (gerçek sensör yoksa bile termal korumayı canlı göstermek için; ör. uygulamada 'I' tuşu).
pub struct SistemSensoru {
    sys: sysinfo::System,
    bilesenler: sysinfo::Components,
    /// Demo amaçlı GPU sıcaklığı ezme değeri (paylaşımlı).
    pub simule_gpu: Arc<Mutex<Option<f32>>>,
}

impl SistemSensoru {
    /// Yeni bir sistem sensörü kurar (ilk CPU örneklemesini ısıtır).
    pub fn yeni() -> Self {
        let mut sys = sysinfo::System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        let bilesenler = sysinfo::Components::new_with_refreshed_list();
        Self {
            sys,
            bilesenler,
            simule_gpu: Arc::new(Mutex::new(None)),
        }
    }

    /// Demo ezme kancasının paylaşımlı tutamacı (uygulama buna sıcaklık yazabilir).
    pub fn simulasyon_kancasi(&self) -> Arc<Mutex<Option<f32>>> {
        Arc::clone(&self.simule_gpu)
    }
}

impl Default for SistemSensoru {
    fn default() -> Self {
        Self::yeni()
    }
}

impl DonanimSensoru for SistemSensoru {
    fn oku(&mut self) -> DonanimOrnegi {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        // true: listeyi tazele (kaybolan bileşenleri çıkar, yenilerini ekle).
        self.bilesenler.refresh(true);

        let cpu_yuzde = Some(self.sys.global_cpu_usage());
        let toplam = self.sys.total_memory();
        let ram_orani = if toplam > 0 {
            Some((self.sys.used_memory() as f32 / toplam as f32).clamp(0.0, 1.0))
        } else {
            None
        };

        // Bileşen etiketlerinden sıcaklıkları kategorize et (en sıcak değeri al).
        let mut gpu_c = None;
        let mut cpu_c = None;
        let mut nvme_c = None;
        for c in self.bilesenler.iter() {
            let etiket = c.label().to_ascii_lowercase();
            let Some(t) = bilesen_sicakligi(c) else {
                continue;
            };
            if etiket.contains("gpu")
                || etiket.contains("nvidia")
                || etiket.contains("amdgpu")
                || etiket.contains("radeon")
            {
                gpu_c = Some(gpu_c.map_or(t, |e: f32| e.max(t)));
            } else if etiket.contains("nvme") || etiket.contains("composite") {
                nvme_c = Some(nvme_c.map_or(t, |e: f32| e.max(t)));
            } else if etiket.contains("cpu")
                || etiket.contains("core")
                || etiket.contains("package")
                || etiket.contains("tctl")
                || etiket.contains("tdie")
                || etiket.contains("k10temp")
            {
                cpu_c = Some(cpu_c.map_or(t, |e: f32| e.max(t)));
            }
        }

        // Demo/simülasyon: GPU sıcaklığını ez (gerçek sensör yoksa bile korumayı göster).
        if let Ok(s) = self.simule_gpu.lock() {
            if let Some(t) = *s {
                gpu_c = Some(t);
            }
        }

        DonanimOrnegi {
            gpu_c,
            cpu_c,
            nvme_c,
            cpu_yuzde,
            ram_orani,
            fan_rpm: None,
        }
    }
}

/// sysinfo bileşen sıcaklığını alır (NaN/eksik/sıfır → None; sensör güvenilmez).
fn bilesen_sicakligi(c: &sysinfo::Component) -> Option<f32> {
    c.temperature().filter(|t| t.is_finite() && *t > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gpu_ornek(t: f32) -> DonanimOrnegi {
        DonanimOrnegi {
            gpu_c: Some(t),
            ..Default::default()
        }
    }

    #[test]
    fn sensor_yok_korumayi_zarifce_devre_disi_birakir() {
        // Senaryo: sensör okunamıyor → çökme yok, koruma kapalı + bilgi var.
        let mut durum = KoruyucuDurum::default();
        adim_uygula(
            &mut durum,
            DonanimOrnegi::default(),
            &TermalEsikler::default(),
            &bos_checkpoint(),
        );
        assert!(
            !durum.koruma_etkin,
            "Sensör yokken koruma devre dışı olmalı"
        );
        assert!(durum.bilgi.is_some(), "Kullanıcı bilgilendirilmeli");
        assert_eq!(durum.aksiyon, TermalAksiyon::TamKapasite);
        assert!(!durum.acil_durum);
    }

    #[test]
    fn yuksek_sicaklik_kademeli_azaltir_sonra_durdurur() {
        // Senaryo: simüle ısınma → yük kademeli düşer, kritikte durur + checkpoint alınır.
        let esikler = TermalEsikler::default();
        let cp = bos_checkpoint();
        let mut durum = KoruyucuDurum::default();

        adim_uygula(&mut durum, gpu_ornek(60.0), &esikler, &cp);
        assert_eq!(durum.aksiyon, TermalAksiyon::TamKapasite);

        adim_uygula(&mut durum, gpu_ornek(72.0), &esikler, &cp);
        assert_eq!(durum.aksiyon, TermalAksiyon::YukAzalt(75));

        adim_uygula(&mut durum, gpu_ornek(77.0), &esikler, &cp);
        assert_eq!(durum.aksiyon, TermalAksiyon::YukAzalt(50));

        // 82°C → duraklat + İLK checkpoint.
        adim_uygula(&mut durum, gpu_ornek(82.0), &esikler, &cp);
        assert_eq!(durum.aksiyon, TermalAksiyon::Duraklat);
        assert!(durum.sogutuluyor);
        assert_eq!(
            durum.checkpoint_sayisi, 1,
            "Duraklamaya geçişte checkpoint alınmalı"
        );

        // 90°C → acil durdur.
        adim_uygula(&mut durum, gpu_ornek(90.0), &esikler, &cp);
        assert!(durum.acil_durum);
        assert_eq!(durum.aksiyon.yuk_orani(), 0.0);
    }

    #[test]
    fn checkpoint_yalniz_yeni_duraklamada_alinir() {
        let esikler = TermalEsikler::default();
        // Checkpoint kaç kez çağrıldı? Bir Mutex<u32> ile sayalım.
        let cagri = Arc::new(Mutex::new(0u32));
        let c2 = Arc::clone(&cagri);
        let cp: CheckpointKanca = Arc::new(move || {
            *c2.lock().unwrap() += 1;
        });

        let mut durum = KoruyucuDurum::default();
        adim_uygula(&mut durum, gpu_ornek(82.0), &esikler, &cp); // duraklat → checkpoint 1
        adim_uygula(&mut durum, gpu_ornek(83.0), &esikler, &cp); // hâlâ duraklı → checkpoint YOK
        adim_uygula(&mut durum, gpu_ornek(84.0), &esikler, &cp); // hâlâ duraklı → checkpoint YOK
        assert_eq!(
            *cagri.lock().unwrap(),
            1,
            "Checkpoint yalnız geçişte alınmalı"
        );
    }

    #[test]
    fn soguyunca_otomatik_devam_eder() {
        let esikler = TermalEsikler::default();
        let cp = bos_checkpoint();
        let mut durum = KoruyucuDurum::default();
        adim_uygula(&mut durum, gpu_ornek(82.0), &esikler, &cp); // duraklat
        assert!(durum.aksiyon.duraklatir());
        adim_uygula(&mut durum, gpu_ornek(73.0), &esikler, &cp); // histerezis altına indi
        assert!(
            !durum.aksiyon.duraklatir(),
            "Soğuyunca otomatik devam etmeli"
        );
        assert!(!durum.sogutuluyor);
    }

    #[test]
    fn betik_sensor_dizi_bitince_son_ornegi_tekrar_eder() {
        let mut s = BetikSensor::yeni(vec![gpu_ornek(50.0), gpu_ornek(85.0)]);
        assert_eq!(s.oku().gpu_c, Some(50.0));
        assert_eq!(s.oku().gpu_c, Some(85.0));
        assert_eq!(s.oku().gpu_c, Some(85.0)); // son örnek tekrar
    }

    #[test]
    fn watchdog_thread_canli_calisir_ve_temiz_durur() {
        // MK-24: bağımsız thread; ısınma betiğiyle acil duruma ulaşıp checkpoint almalı.
        let betik = BetikSensor::yeni(vec![
            gpu_ornek(60.0),
            gpu_ornek(78.0),
            gpu_ornek(82.0),
            gpu_ornek(90.0),
        ]);
        let cagri = Arc::new(Mutex::new(0u32));
        let c2 = Arc::clone(&cagri);
        let cp: CheckpointKanca = Arc::new(move || *c2.lock().unwrap() += 1);

        let mut muhafiz = DonanimMuhafiz::baslat(
            Box::new(betik),
            TermalEsikler::default(),
            Duration::from_millis(5),
            cp,
        );

        // Birkaç adım çalışsın.
        std::thread::sleep(Duration::from_millis(120));
        let d = muhafiz.durum();
        assert!(
            d.acil_durum,
            "Watchdog ısınma betiğinde acil duruma ulaşmalı"
        );
        assert!(d.checkpoint_sayisi >= 1, "Duraklamada checkpoint alınmalı");
        assert_eq!(muhafiz.yuk_orani(), 0.0);

        muhafiz.durdur(); // temiz kapanış
    }
}
