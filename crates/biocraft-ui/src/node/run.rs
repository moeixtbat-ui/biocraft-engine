//! Çalıştırma motoru — DAG'i paralel, bütçeli, önbellekli ve iptal edilebilir çalıştırır (İP-05).
//!
//! ## Garantiler (İP-05 "Çalıştırma" + "Durum")
//! - **Paralel:** Bağımsız dallar aynı anda çalışır.  Topolojik *dalgalar* hâlinde ilerlenir;
//!   bir dalgadaki birbirinden bağımsız node'lar [`std::thread::scope`] ile paralelleştirilir.
//!   (Rayon/Tokio yerine std seçildi — proje "az bağımlılık" ilkesi, bkz. [`super::dag`].)
//! - **Bellek bütçesi (İP-08, MK-21/22):** Her node çalışmadan **önce** [`BellekOrkestratoru`]'ndan
//!   rezervasyon ister.  Bir dalgada eş zamanlı çalışan node'lar **bütçeye sığacak şekilde**
//!   gruplanır → büyük akışta bile **OOM yok** (rezervasyon reddedilirse node hata verir, çökmez).
//! - **Önbellek:** İmzası değişmeyen node yeniden hesaplanmaz ([`SonucOnbellek`]); yalnızca
//!   **değişen alt-graf** yeniden çalışır.
//! - **Hata izolasyonu:** Bir node hata verirse o **dal durur** (alt düğümleri atlanır), ama
//!   **bağımsız dallar devam eder**.
//! - **İptal/ilerleme:** [`IptalJetonu`] her dalga öncesi denetlenir; [`IlerlemeOlay`] her node
//!   bittikçe bildirilir (arayüz arka planda çalıştırıp donmadan ilerlemeyi gösterebilir).
// MK-04: ağır iş arka planda; MK-21/22: rezervasyonla bellek; MK-54: motor temelde, node'lar eklentiden.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use biocraft_mem::{BellekBileseni, BellekOrkestratoru};
use biocraft_sdk::node::{AkisDeger, NodeCalistirici, NodeKaydi, Parametreler};

use super::cache::SonucOnbellek;
use super::dag::dongu_var_mi;
use super::graph::{BaglantiKimlik, NodeDurumu, NodeGraf, NodeKimlik, PortRef};
use super::port::PortYonu;

// ─── Çalıştırıcı kaydı (eklenti SDK node'ları burada toplanır) ─────────────────

/// Tür kimliğinden çalıştırma davranışına eşleme.  Eklentiler [`NodeKaydi`] ile katkı verir;
/// çekirdek demo türleri [`YurutucuKayit::ornek`] ile gelir (MK-54: node'lar çoğunlukla eklentiden).
#[derive(Clone, Default)]
pub struct YurutucuKayit {
    kayitlar: HashMap<String, NodeKaydi>,
}

impl YurutucuKayit {
    /// Boş kayıt.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir node kaydı ekler/günceller (tür kimliğine göre).  Eklenti SDK entegrasyon noktası.
    pub fn kaydet(&mut self, kayit: NodeKaydi) {
        self.kayitlar.insert(kayit.tanim.kimlik.clone(), kayit);
    }

    /// Bir tür kimliğinin kaydı (varsa).
    pub fn al(&self, tur_kimligi: &str) -> Option<&NodeKaydi> {
        self.kayitlar.get(tur_kimligi)
    }

    /// Bu tür için çalıştırıcı kayıtlı mı?
    pub fn icerir(&self, tur_kimligi: &str) -> bool {
        self.kayitlar.contains_key(tur_kimligi)
    }

    /// Kayıtlı tür sayısı.
    pub fn adet(&self) -> usize {
        self.kayitlar.len()
    }

    /// Çekirdek demo akışı için çalıştırıcılar (örnek katalogla birebir uyumlu — Gün 20).
    ///
    /// Gerçek bilim node'ları eklentilerden gelecek; bunlar motoru uçtan uca çalıştırılabilir
    /// kılan **deterministik** taklit davranışlardır (önbellek + paralel + bütçe gözlemlenebilir).
    pub fn ornek() -> Self {
        let mut k = Self::yeni();
        // Kaynak: dosyadan dizi okur (girdisiz → kök node).
        k.kaydet(NodeKaydi::yeni(
            tanim("girdi.dizi_oku", "Dizi Oku (FASTA)"),
            Arc::new(DemoYurutucu {
                cikti_turu: Some("dizi".into()),
                eleman_orani: 1.0,
                bayt_per_eleman: 120,
                taban_eleman: 1000,
                agir: false,
                azalt_param: None,
            }),
        ));
        k.kaydet(NodeKaydi::yeni(
            tanim("isle.hizala", "Hizala"),
            Arc::new(DemoYurutucu {
                cikti_turu: Some("hizalama".into()),
                eleman_orani: 1.0,
                bayt_per_eleman: 240,
                taban_eleman: 0,
                agir: true, // hizalama pahalıdır → canlı modda uyarı
                azalt_param: None,
            }),
        ));
        k.kaydet(NodeKaydi::yeni(
            tanim("isle.varyant_cagir", "Varyant Çağır"),
            Arc::new(DemoYurutucu {
                cikti_turu: Some("varyant".into()),
                eleman_orani: 0.1,
                bayt_per_eleman: 64,
                taban_eleman: 0,
                agir: true,
                azalt_param: None,
            }),
        ));
        k.kaydet(NodeKaydi::yeni(
            tanim("donustur.varyant_tablo", "Varyant → Tablo"),
            Arc::new(DemoYurutucu {
                cikti_turu: Some("tablo".into()),
                eleman_orani: 1.0,
                bayt_per_eleman: 32,
                taban_eleman: 0,
                agir: false,
                azalt_param: None,
            }),
        ));
        k.kaydet(NodeKaydi::yeni(
            tanim("isle.tablo_filtrele", "Tablo Filtrele"),
            Arc::new(DemoYurutucu {
                cikti_turu: Some("tablo".into()),
                eleman_orani: 1.0,
                bayt_per_eleman: 32,
                taban_eleman: 0,
                agir: false,
                azalt_param: Some("esik".into()), // parametre değişince çıktı değişir → önbellek geçersiz
            }),
        ));
        k.kaydet(NodeKaydi::yeni(
            tanim("cikti.ozet", "Özet İstatistik"),
            Arc::new(DemoYurutucu {
                cikti_turu: Some("metin".into()),
                eleman_orani: 0.0, // özet = tek metin
                bayt_per_eleman: 0,
                taban_eleman: 1,
                agir: false,
                azalt_param: None,
            }),
        ));
        k.kaydet(NodeKaydi::yeni(
            tanim("cikti.uc_boyut", "3B Görüntüle"),
            Arc::new(DemoYurutucu {
                cikti_turu: None, // sink
                eleman_orani: 0.0,
                bayt_per_eleman: 0,
                taban_eleman: 0,
                agir: false,
                azalt_param: None,
            }),
        ));
        k
    }
}

/// Kısa NodeTanimi yardımcısı (yalnız kimlik+başlık — port/parametre kataloğda).
fn tanim(kimlik: &str, baslik: &str) -> biocraft_sdk::node::NodeTanimi {
    biocraft_sdk::node::NodeTanimi {
        kimlik: kimlik.into(),
        baslik: baslik.into(),
        kategori: String::new(),
        aciklama: String::new(),
        portlar: vec![],
        parametreler: vec![],
    }
}

/// Deterministik demo çalıştırıcı (gerçek bilim davranışı eklentilerden gelir — MK-54).
struct DemoYurutucu {
    /// Çıktı türü (`None` = sink/çıktı yok).
    cikti_turu: Option<String>,
    /// Çıktı eleman sayısı = (girdi en büyüğü veya taban) × bu oran.
    eleman_orani: f64,
    /// Eleman başına tahmini bayt.
    bayt_per_eleman: u64,
    /// Girdisiz (kaynak) node için taban eleman sayısı.
    taban_eleman: u64,
    /// Ağır (pahalı) node mu? → canlı modda uyarı.
    agir: bool,
    /// Bu tam-sayı parametre varsa, çıktı elemanını o kadar azaltır (filtre davranışı).
    azalt_param: Option<String>,
}

impl NodeCalistirici for DemoYurutucu {
    fn calistir(
        &self,
        girdiler: &[AkisDeger],
        parametreler: &Parametreler,
    ) -> Result<Vec<AkisDeger>, String> {
        let taban = if girdiler.is_empty() {
            self.taban_eleman
        } else {
            girdiler.iter().map(|g| g.eleman).max().unwrap_or(0)
        };
        let mut eleman = (taban as f64 * self.eleman_orani).round() as u64;
        if self.eleman_orani == 0.0 {
            eleman = self.taban_eleman; // özet/sabit çıktı
        }
        if let Some(p) = &self.azalt_param {
            if let Some(esik) = parametreler.tam_sayi(p) {
                eleman = eleman.saturating_sub(esik.max(0) as u64);
            }
        }
        match &self.cikti_turu {
            Some(t) => {
                let bayt = (eleman * self.bayt_per_eleman).max(1);
                Ok(vec![AkisDeger::yeni(
                    t.clone(),
                    format!("{eleman} öğe"),
                    eleman,
                    bayt,
                )])
            }
            None => Ok(vec![]),
        }
    }

    fn agir_mi(&self) -> bool {
        self.agir
    }

    fn tahmini_bayt(&self, girdiler: &[AkisDeger]) -> u64 {
        let girdi: u64 = girdiler.iter().map(|g| g.bayt).sum();
        // Girdi + tahmini çıktı için yer (kabaca girdinin 2 katı, en az 1).
        girdi
            .saturating_mul(2)
            .max(self.taban_eleman * self.bayt_per_eleman)
            .max(1)
    }
}

// ─── İptal + ilerleme ──────────────────────────────────────────────────────────

/// Çalıştırmayı dışarıdan iptal etmek için paylaşılan bayrak (thread-güvenli, ucuz klon).
#[derive(Clone, Default)]
pub struct IptalJetonu(Arc<AtomicBool>);

impl IptalJetonu {
    /// Yeni (iptal edilmemiş) jeton.
    pub fn yeni() -> Self {
        Self::default()
    }
    /// Çalıştırmayı iptal et (kalan dalgalar başlatılmaz).
    pub fn iptal_et(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    /// İptal istendi mi?
    pub fn iptal_mi(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Bir node tamamlandığında bildirilen ilerleme olayı.
#[derive(Debug, Clone, Copy)]
pub struct IlerlemeOlay {
    /// Şimdiye kadar işlenen node sayısı.
    pub tamamlanan: usize,
    /// Toplam node sayısı.
    pub toplam: usize,
    /// Bu olayı tetikleyen node.
    pub node: NodeKimlik,
    /// Node'un yeni durumu.
    pub durum: NodeDurumu,
}

// ─── Sonuç tipleri ─────────────────────────────────────────────────────────────

/// Tek bir node'un çalıştırma sonucu.
#[derive(Debug, Clone)]
pub struct NodeSonuc {
    /// Durum halkası rengi (Bekliyor=atlandı/çalışmadı, Bitti, Hata).
    pub durum: NodeDurumu,
    /// Çıktı değerleri (çıkış portu sırasında).
    pub ciktilar: Vec<AkisDeger>,
    /// Hata mesajı (varsa).
    pub hata: Option<String>,
    /// Sonuç önbellekten mi geldi (yeniden hesaplanmadı)?
    pub onbellekten: bool,
    /// Yukarı-akış hatası nedeniyle atlandı mı? (Dal durdu.)
    pub atlandi: bool,
}

impl NodeSonuc {
    fn bos(durum: NodeDurumu) -> Self {
        Self {
            durum,
            ciktilar: vec![],
            hata: None,
            onbellekten: false,
            atlandi: false,
        }
    }
}

/// Tüm çalıştırmanın sonucu.
#[derive(Debug, Clone, Default)]
pub struct CalismaSonucu {
    /// Node başına sonuç.
    pub node_sonuclari: BTreeMap<NodeKimlik, NodeSonuc>,
    /// Bağlantı başına ara veri (kabloya tıkla → önizleme).
    pub baglanti_onizleme: BTreeMap<BaglantiKimlik, AkisDeger>,
    /// Önbellekten gelen (yeniden hesaplanmayan) node sayısı.
    pub onbellekten_atlanan: usize,
    /// Gerçekten hesaplanan node sayısı.
    pub hesaplanan: usize,
    /// Hata veren node sayısı.
    pub hata_sayisi: usize,
    /// Yukarı-akış hatası nedeniyle atlanan node sayısı.
    pub atlanan: usize,
    /// Çalıştırma iptal edildi mi?
    pub iptal_edildi: bool,
    /// Gözlemlenen en yüksek eş zamanlılık (paralelliğin kanıtı / teşhis).
    pub azami_es_zamanli: usize,
}

impl CalismaSonucu {
    /// Sonuç durumlarını grafiğe uygular (durum halkalarını günceller).
    pub fn durumu_uygula(&self, graf: &mut NodeGraf) {
        for (k, s) in &self.node_sonuclari {
            graf.durum_ayarla(*k, s.durum);
        }
    }
}

/// Çalıştırma ayarları.
#[derive(Debug, Clone)]
pub struct CalismaAyari {
    /// Aynı anda çalışacak en fazla node (paralellik tavanı).
    pub azami_worker: usize,
    /// Canlı mod (her değişiklikte otomatik) — yalnız işaret; ağır node uyarısı çağırana ait.
    pub canli_mod: bool,
}

impl Default for CalismaAyari {
    fn default() -> Self {
        Self {
            azami_worker: varsayilan_worker(),
            canli_mod: false,
        }
    }
}

/// Mevcut donanıma göre makul varsayılan worker sayısı (en az 1).
pub fn varsayilan_worker() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(1, 16)
}

// ─── Eş zamanlılık sayacı (paralelliği gözlemler) ──────────────────────────────

#[derive(Default)]
struct EsZamanliSayac {
    simdi: AtomicUsize,
    azami: AtomicUsize,
}
impl EsZamanliSayac {
    fn gir(&self) {
        let c = self.simdi.fetch_add(1, Ordering::SeqCst) + 1;
        self.azami.fetch_max(c, Ordering::SeqCst);
    }
    fn cik(&self) {
        self.simdi.fetch_sub(1, Ordering::SeqCst);
    }
    fn tepe(&self) -> usize {
        self.azami.load(Ordering::SeqCst)
    }
}

// ─── İmza (önbellek anahtarı) ──────────────────────────────────────────────────

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv_kat(mut h: u64, baytlar: &[u8]) -> u64 {
    for &b in baytlar {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// Bir node'un önbellek imzası: tür + parametreler + tüm girdi (yukarı-akış çıktı) imzaları.
fn node_imzasi(tur_kimligi: &str, parametreler: &Parametreler, girdiler: &[AkisDeger]) -> u64 {
    let mut h = FNV_OFFSET;
    h = fnv_kat(h, tur_kimligi.as_bytes());
    h = fnv_kat(h, b"|p|");
    h = fnv_kat(h, parametreler.imza().as_bytes());
    for g in girdiler {
        h = fnv_kat(h, b"|i|");
        h = fnv_kat(h, g.imza().as_bytes());
    }
    h
}

// ─── Motor ─────────────────────────────────────────────────────────────────────

/// Bir node'un yukarı-akış (bağımlı olduğu) node kimlikleri.
fn oncekiler(graf: &NodeGraf, node: NodeKimlik) -> BTreeSet<NodeKimlik> {
    graf.baglantilar()
        .iter()
        .filter(|b| b.hedef.node == node)
        .map(|b| b.kaynak.node)
        .collect()
}

/// Bir node girişlerini toplar: her giriş portu için bağlantının kaynağındaki çıktı değeri.
///
/// Dönüş: `Ok(girdiler)` veya `Err(())` = en az bir bağlı kaynak **atlandı/hata** → bu node atlanır.
/// `onizleme` her bağlantının taşıdığı değeri (kablo önizlemesi) biriktirir.
#[allow(clippy::type_complexity)]
fn girdileri_topla(
    graf: &NodeGraf,
    node: NodeKimlik,
    sonuclar: &BTreeMap<NodeKimlik, NodeSonuc>,
    onizleme: &mut BTreeMap<BaglantiKimlik, AkisDeger>,
) -> Result<Vec<AkisDeger>, ()> {
    let giris_say = graf.node(node).map(|n| n.girisler.len()).unwrap_or(0);
    let mut girdiler = Vec::new();
    for gi in 0..giris_say {
        let hedef = PortRef::yeni(node, PortYonu::Giris, gi);
        let Some(b) = graf.baglantilar().iter().find(|b| b.hedef == hedef) else {
            continue; // bağlanmamış giriş → atla (opsiyonel girdi)
        };
        let kaynak_sonuc = sonuclar.get(&b.kaynak.node);
        match kaynak_sonuc {
            Some(s) if s.atlandi || s.durum == NodeDurumu::Hata => return Err(()),
            Some(s) => match s.ciktilar.get(b.kaynak.indeks) {
                Some(deger) => {
                    onizleme.insert(b.kimlik, deger.clone());
                    girdiler.push(deger.clone());
                }
                None => return Err(()), // kaynak beklenen çıktıyı üretmedi → dal durur
            },
            None => return Err(()), // kaynak henüz işlenmedi (olmamalı: dalga sırası garanti eder)
        }
    }
    Ok(girdiler)
}

/// Çalıştırılacak bir node'un hazırlığı (bir dalgada paralel grup için).
struct Is {
    node: NodeKimlik,
    imza: u64,
    yurutucu: Arc<dyn NodeCalistirici>,
    girdiler: Vec<AkisDeger>,
    parametreler: Parametreler,
}

/// Bir paralel grup için thread'e taşınan iş verisi (node + imza + çalıştırıcı + girdi + param).
type GrupIs = (
    NodeKimlik,
    u64,
    Arc<dyn NodeCalistirici>,
    Vec<AkisDeger>,
    Parametreler,
);

/// **Akışı çalıştırır.**  Bkz. modül başlığı: paralel + bütçeli + önbellekli + iptal edilebilir.
#[allow(clippy::too_many_arguments)]
pub fn calistir(
    graf: &NodeGraf,
    kayit: &YurutucuKayit,
    parametreler: &HashMap<NodeKimlik, Parametreler>,
    orkestrator: &BellekOrkestratoru,
    onbellek: &mut SonucOnbellek,
    ayar: &CalismaAyari,
    iptal: &IptalJetonu,
    ilerleme: &mut dyn FnMut(IlerlemeOlay),
) -> CalismaSonucu {
    let mut sonuc = CalismaSonucu::default();
    let toplam = graf.nodelar().len();

    // Döngü savunması (ham bağlantı testleri döngü kurabilir; baglanti_kontrol normalde engeller).
    if dongu_var_mi(graf) {
        for n in graf.nodelar() {
            let mut s = NodeSonuc::bos(NodeDurumu::Hata);
            s.hata = Some("Akışta döngü var: çalıştırılamaz (DAG olmalı).".into());
            sonuc.node_sonuclari.insert(n.kimlik, s);
            sonuc.hata_sayisi += 1;
        }
        return sonuc;
    }

    let sayac = EsZamanliSayac::default();
    let mut islenen: HashSet<NodeKimlik> = HashSet::new();
    let oncekiler_map: HashMap<NodeKimlik, BTreeSet<NodeKimlik>> = graf
        .nodelar()
        .iter()
        .map(|n| (n.kimlik, oncekiler(graf, n.kimlik)))
        .collect();
    let mut tamamlanan = 0usize;

    'dalgalar: while islenen.len() < toplam {
        if iptal.iptal_mi() {
            sonuc.iptal_edildi = true;
            break;
        }
        // Hazır: tüm öncülleri işlenmiş, kendisi işlenmemiş node'lar (kararlı sıra için BTreeSet).
        let mut hazir: Vec<NodeKimlik> = graf
            .nodelar()
            .iter()
            .map(|n| n.kimlik)
            .filter(|k| !islenen.contains(k))
            .filter(|k| oncekiler_map[k].iter().all(|p| islenen.contains(p)))
            .collect();
        hazir.sort();
        if hazir.is_empty() {
            break; // ilerleme yok (olmamalı — döngü yukarıda yakalandı)
        }

        // Bu dalgadaki node'ları: önbellek/atlama/hesaplama olarak sınıfla.
        let mut isler: Vec<Is> = Vec::new();
        for k in hazir {
            let tur = match graf.node(k) {
                Some(n) => n.tur_kimligi.clone(),
                None => continue,
            };
            // Girdileri topla (önizleme dahil); bağlı kaynak hatalı/atlanmışsa node atlanır.
            let girdiler =
                match girdileri_topla(graf, k, &sonuc.node_sonuclari, &mut sonuc.baglanti_onizleme)
                {
                    Ok(g) => g,
                    Err(()) => {
                        let mut s = NodeSonuc::bos(NodeDurumu::Bekliyor);
                        s.atlandi = true;
                        sonuc.node_sonuclari.insert(k, s);
                        sonuc.atlanan += 1;
                        islenen.insert(k);
                        tamamlanan += 1;
                        ilerleme(IlerlemeOlay {
                            tamamlanan,
                            toplam,
                            node: k,
                            durum: NodeDurumu::Bekliyor,
                        });
                        continue;
                    }
                };
            let pars = parametreler.get(&k).cloned().unwrap_or_default();
            let imza = node_imzasi(&tur, &pars, &girdiler);

            // Önbellek: imza taze ise yeniden hesaplama.
            if let Some(onb) = onbellek.al(k, imza) {
                let mut s = NodeSonuc::bos(NodeDurumu::Bitti);
                s.ciktilar = onb.to_vec();
                s.onbellekten = true;
                sonuc.node_sonuclari.insert(k, s);
                sonuc.onbellekten_atlanan += 1;
                islenen.insert(k);
                tamamlanan += 1;
                ilerleme(IlerlemeOlay {
                    tamamlanan,
                    toplam,
                    node: k,
                    durum: NodeDurumu::Bitti,
                });
                continue;
            }

            // Çalıştırıcı yoksa: "yapılandırılmadı" hatası (MK-48 ruhu — eklenti kurulu değil).
            let Some(kayit_g) = kayit.al(&tur) else {
                let mut s = NodeSonuc::bos(NodeDurumu::Hata);
                s.hata = Some(format!(
                    "Bu node türü için çalıştırıcı yok (eklenti kurulu değil): {tur}"
                ));
                sonuc.node_sonuclari.insert(k, s);
                sonuc.hata_sayisi += 1;
                islenen.insert(k);
                tamamlanan += 1;
                ilerleme(IlerlemeOlay {
                    tamamlanan,
                    toplam,
                    node: k,
                    durum: NodeDurumu::Hata,
                });
                continue;
            };

            isler.push(Is {
                node: k,
                imza,
                yurutucu: kayit_g.yurutucu.clone(),
                girdiler,
                parametreler: pars,
            });
        }

        // Hesaplanacak işleri **bütçeye sığacak gruplar** hâlinde paralel çalıştır.
        let mut sira = isler.into_iter();
        let mut bekleyen: Option<Is> = sira.next();
        while bekleyen.is_some() {
            if iptal.iptal_mi() {
                sonuc.iptal_edildi = true;
                break 'dalgalar;
            }
            // Grup oluştur: bütçeye + worker tavanına sığdığı kadar rezervasyon al.
            let mut grup: Vec<(Is, biocraft_mem::Rezervasyon)> = Vec::new();
            while let Some(is) = bekleyen.take() {
                let tahmin = is.yurutucu.tahmini_bayt(&is.girdiler);
                match orkestrator
                    .rezerve_et(BellekBileseni::Diger(format!("node:{}", is.node.0)), tahmin)
                {
                    Ok(rez) => {
                        grup.push((is, rez));
                        if grup.len() >= ayar.azami_worker.max(1) {
                            bekleyen = sira.next();
                            break;
                        }
                        bekleyen = sira.next();
                    }
                    Err(hata) => {
                        if grup.is_empty() {
                            // Tek node bile sığmıyor (tahmin > bütçe): bu node hata, dal durur (OOM yok).
                            let k = is.node;
                            let mut s = NodeSonuc::bos(NodeDurumu::Hata);
                            s.hata = Some(format!("Bellek bütçesi yetersiz: {}", hata.ne_oldu));
                            sonuc.node_sonuclari.insert(k, s);
                            sonuc.hata_sayisi += 1;
                            islenen.insert(k);
                            tamamlanan += 1;
                            ilerleme(IlerlemeOlay {
                                tamamlanan,
                                toplam,
                                node: k,
                                durum: NodeDurumu::Hata,
                            });
                            bekleyen = sira.next();
                        } else {
                            // Grup dolu (bütçe doldu); önce bu grubu çalıştır, sonra devam.
                            bekleyen = Some(is);
                            break;
                        }
                    }
                }
            }

            if grup.is_empty() {
                continue;
            }

            // Grup içi node'ları paralel çalıştır (bağımsız → eş zamanlı).
            let mut grup_sonuc: Vec<(NodeKimlik, u64, Result<Vec<AkisDeger>, String>)> = Vec::new();
            std::thread::scope(|s| {
                let sayac = &sayac;
                let isler_ref: Vec<GrupIs> = grup
                    .iter()
                    .map(|(is, _)| {
                        (
                            is.node,
                            is.imza,
                            is.yurutucu.clone(),
                            is.girdiler.clone(),
                            is.parametreler.clone(),
                        )
                    })
                    .collect();
                let mut handles = Vec::new();
                for (node, imza, yur, gir, par) in isler_ref {
                    let h = s.spawn(move || {
                        sayac.gir();
                        let r = yur.calistir(&gir, &par);
                        sayac.cik();
                        r
                    });
                    handles.push((node, imza, h));
                }
                for (node, imza, h) in handles {
                    let r = h.join().unwrap_or_else(|_| {
                        Err("Çalıştırıcı beklenmedik şekilde durdu (panik).".into())
                    });
                    grup_sonuc.push((node, imza, r));
                }
            });
            // Rezervasyonlar burada düşer → bellek serbest kalır (sonraki grup için yer açılır).
            drop(grup);

            // Sonuçları işle (sıralı → önbellek + ilerleme tek thread'de).
            grup_sonuc.sort_by_key(|(k, _, _)| k.0);
            for (k, imza, r) in grup_sonuc {
                match r {
                    Ok(ciktilar) => {
                        onbellek.yaz(k, imza, ciktilar.clone());
                        let mut sn = NodeSonuc::bos(NodeDurumu::Bitti);
                        sn.ciktilar = ciktilar;
                        sonuc.node_sonuclari.insert(k, sn);
                        sonuc.hesaplanan += 1;
                        islenen.insert(k);
                        tamamlanan += 1;
                        ilerleme(IlerlemeOlay {
                            tamamlanan,
                            toplam,
                            node: k,
                            durum: NodeDurumu::Bitti,
                        });
                    }
                    Err(mesaj) => {
                        let mut sn = NodeSonuc::bos(NodeDurumu::Hata);
                        sn.hata = Some(mesaj);
                        sonuc.node_sonuclari.insert(k, sn);
                        sonuc.hata_sayisi += 1;
                        islenen.insert(k);
                        tamamlanan += 1;
                        ilerleme(IlerlemeOlay {
                            tamamlanan,
                            toplam,
                            node: k,
                            durum: NodeDurumu::Hata,
                        });
                    }
                }
            }
        }
    }

    // İptal → kalan node'lar "bekliyor" kalır (durum değişmez); sonuçta işaretlenir.
    sonuc.azami_es_zamanli = sayac.tepe();
    sonuc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::graph::{Baglanti, Node};
    use crate::node::katalog::NodeKatalogu;
    use crate::node::port::{Port, PortYonu as UiPortYonu};

    /// 64 MiB bütçeli orkestratör.
    fn orkestrator() -> BellekOrkestratoru {
        BellekOrkestratoru::yeni(64 * 1024 * 1024)
    }

    fn bos_ilerleme() -> impl FnMut(IlerlemeOlay) {
        |_o| {}
    }

    /// Demo katalogdan örnek (dizi→hizala→varyant) doğrusal akış kurar.
    fn dogrusal_akis() -> NodeGraf {
        let katalog = NodeKatalogu::ornek();
        let mut g = NodeGraf::yeni("ana");
        let ekle = |g: &mut NodeGraf, tur: &str, konum: (f32, f32)| -> NodeKimlik {
            let k = g.yeni_node_kimlik();
            g.node_ekle_ham(katalog.bul(tur).unwrap().ornekle(k, konum));
            k
        };
        let oku = ekle(&mut g, "girdi.dizi_oku", (0.0, 0.0));
        let hiz = ekle(&mut g, "isle.hizala", (200.0, 0.0));
        let var = ekle(&mut g, "isle.varyant_cagir", (400.0, 0.0));
        let baglan = |g: &mut NodeGraf, a: NodeKimlik, b: NodeKimlik| {
            let bk = g.yeni_baglanti_kimlik();
            g.baglanti_ekle_ham(Baglanti {
                kimlik: bk,
                kaynak: PortRef::yeni(a, UiPortYonu::Cikis, 0),
                hedef: PortRef::yeni(b, UiPortYonu::Giris, 0),
            });
        };
        baglan(&mut g, oku, hiz);
        baglan(&mut g, hiz, var);
        g
    }

    #[test]
    fn dogrusal_akis_calisir_ve_topolojik_dogru() {
        let g = dogrusal_akis();
        let kayit = YurutucuKayit::ornek();
        let pars = HashMap::new();
        let mut onb = SonucOnbellek::yeni();
        let mut ilr = bos_ilerleme();
        let s = calistir(
            &g,
            &kayit,
            &pars,
            &orkestrator(),
            &mut onb,
            &CalismaAyari::default(),
            &IptalJetonu::yeni(),
            &mut ilr,
        );
        assert_eq!(s.hesaplanan, 3);
        assert_eq!(s.hata_sayisi, 0);
        // Her node Bitti.
        assert!(s
            .node_sonuclari
            .values()
            .all(|n| n.durum == NodeDurumu::Bitti));
        // Kablo önizlemesi 2 bağlantı için dolmalı.
        assert_eq!(s.baglanti_onizleme.len(), 2);
    }

    #[test]
    fn onbellek_degismeyeni_atlar_degiseni_yeniden_hesaplar() {
        let g = dogrusal_akis();
        let kayit = YurutucuKayit::ornek();
        let mut pars: HashMap<NodeKimlik, Parametreler> = HashMap::new();
        let mut onb = SonucOnbellek::yeni();
        let ork = orkestrator();
        // 1. çalıştırma: hepsi hesaplanır.
        let mut ilr = bos_ilerleme();
        let s1 = calistir(
            &g,
            &kayit,
            &pars,
            &ork,
            &mut onb,
            &CalismaAyari::default(),
            &IptalJetonu::yeni(),
            &mut ilr,
        );
        assert_eq!(s1.hesaplanan, 3);
        assert_eq!(s1.onbellekten_atlanan, 0);
        // 2. çalıştırma: değişiklik yok → hepsi önbellekten.
        let mut ilr = bos_ilerleme();
        let s2 = calistir(
            &g,
            &kayit,
            &pars,
            &ork,
            &mut onb,
            &CalismaAyari::default(),
            &IptalJetonu::yeni(),
            &mut ilr,
        );
        assert_eq!(s2.onbellekten_atlanan, 3, "değişmeyen tüm node önbellekten");
        assert_eq!(s2.hesaplanan, 0);

        // İlk node'a parametre ekle (çıktısı değişir) → o + tüm alt-graf yeniden hesaplanmalı.
        let oku = *pars.keys().next().unwrap_or(&NodeKimlik(1));
        let _ = oku;
        let ilk = g.nodelar()[0].kimlik;
        let mut p = Parametreler::yeni();
        p.ayarla("tohum", biocraft_sdk::node::ParametreDeger::TamSayi(7));
        // Not: girdi.dizi_oku çıktısı parametreye bağlı değil, bu yüzden imza yalnız ilk node için değişir,
        // alt-graf girdi imzası aynı kalırsa atlanır.  Bunu görmek için filtreyi test edelim (aşağıda).
        pars.insert(ilk, p);
        let mut ilr = bos_ilerleme();
        let s3 = calistir(
            &g,
            &kayit,
            &pars,
            &ork,
            &mut onb,
            &CalismaAyari::default(),
            &IptalJetonu::yeni(),
            &mut ilr,
        );
        // İlk node imzası değişti → en az 1 yeniden hesaplama.
        assert!(s3.hesaplanan >= 1);
    }

    #[test]
    fn filtre_parametresi_alt_grafi_gecersiz_kilar() {
        // dizi_oku → hizala → varyant → tablo → filtrele zinciri; filtre parametresi değişince
        // filtre + (varsa) altı yeniden hesaplanır, üstü önbellekte kalır.
        let katalog = NodeKatalogu::ornek();
        let mut g = NodeGraf::yeni("ana");
        let ekle = |g: &mut NodeGraf, tur: &str| -> NodeKimlik {
            let k = g.yeni_node_kimlik();
            g.node_ekle_ham(katalog.bul(tur).unwrap().ornekle(k, (0.0, 0.0)));
            k
        };
        let oku = ekle(&mut g, "girdi.dizi_oku");
        let hiz = ekle(&mut g, "isle.hizala");
        let var = ekle(&mut g, "isle.varyant_cagir");
        let tab = ekle(&mut g, "donustur.varyant_tablo");
        let fil = ekle(&mut g, "isle.tablo_filtrele");
        let baglan = |g: &mut NodeGraf, a: NodeKimlik, b: NodeKimlik| {
            let bk = g.yeni_baglanti_kimlik();
            g.baglanti_ekle_ham(Baglanti {
                kimlik: bk,
                kaynak: PortRef::yeni(a, UiPortYonu::Cikis, 0),
                hedef: PortRef::yeni(b, UiPortYonu::Giris, 0),
            });
        };
        baglan(&mut g, oku, hiz);
        baglan(&mut g, hiz, var);
        baglan(&mut g, var, tab);
        baglan(&mut g, tab, fil);

        let kayit = YurutucuKayit::ornek();
        let mut pars: HashMap<NodeKimlik, Parametreler> = HashMap::new();
        let mut onb = SonucOnbellek::yeni();
        let ork = orkestrator();
        let mut ilr = bos_ilerleme();
        let s1 = calistir(
            &g,
            &kayit,
            &pars,
            &ork,
            &mut onb,
            &CalismaAyari::default(),
            &IptalJetonu::yeni(),
            &mut ilr,
        );
        assert_eq!(s1.hesaplanan, 5);

        // Filtre eşiğini değiştir → çıktısı değişir; üstteki 4 node önbellekte kalmalı.
        let mut p = Parametreler::yeni();
        p.ayarla("esik", biocraft_sdk::node::ParametreDeger::TamSayi(5));
        pars.insert(fil, p);
        let mut ilr = bos_ilerleme();
        let s2 = calistir(
            &g,
            &kayit,
            &pars,
            &ork,
            &mut onb,
            &CalismaAyari::default(),
            &IptalJetonu::yeni(),
            &mut ilr,
        );
        assert_eq!(s2.onbellekten_atlanan, 4, "üst zincir önbellekten");
        assert_eq!(s2.hesaplanan, 1, "yalnız filtre yeniden hesaplandı");
    }

    #[test]
    fn bagimsiz_dallar_paralel_calisir() {
        // İki tamamen bağımsız doğrusal dal → aynı dalgada paralel çalışmalı (eş zamanlılık ≥ 2).
        // Dallar yapay olarak yavaşlatılır (barrier yerine kısa uyku) ki örtüşme gözlemlensin.
        struct Yavas;
        impl NodeCalistirici for Yavas {
            fn calistir(
                &self,
                _g: &[AkisDeger],
                _p: &Parametreler,
            ) -> Result<Vec<AkisDeger>, String> {
                std::thread::sleep(std::time::Duration::from_millis(40));
                Ok(vec![AkisDeger::yeni("dizi", "1", 1, 1024)])
            }
        }
        let mut kayit = YurutucuKayit::yeni();
        kayit.kaydet(NodeKaydi::yeni(tanim("yavas", "Yavaş"), Arc::new(Yavas)));

        let mut g = NodeGraf::yeni("ana");
        for _ in 0..4 {
            let k = g.yeni_node_kimlik();
            g.node_ekle_ham(Node {
                kimlik: k,
                tur_kimligi: "yavas".into(),
                baslik: "Y".into(),
                konum: (0.0, 0.0),
                girisler: vec![],
                cikislar: vec![Port::yeni("c", "dizi")],
                durum: NodeDurumu::Bekliyor,
            });
        }
        let kayit_p = kayit;
        let mut onb = SonucOnbellek::yeni();
        let mut ilr = bos_ilerleme();
        let s = calistir(
            &g,
            &kayit_p,
            &HashMap::new(),
            &orkestrator(),
            &mut onb,
            &CalismaAyari {
                azami_worker: 4,
                canli_mod: false,
            },
            &IptalJetonu::yeni(),
            &mut ilr,
        );
        assert_eq!(s.hesaplanan, 4);
        assert!(
            s.azami_es_zamanli >= 2,
            "bağımsız dallar paralel çalışmalı (gözlemlenen={})",
            s.azami_es_zamanli
        );
    }

    #[test]
    fn hata_o_dali_durdurur_bagimsiz_dal_devam() {
        // Dal A: kaynak → patlayan. Dal B: bağımsız kaynak (başarılı).
        struct Patlar;
        impl NodeCalistirici for Patlar {
            fn calistir(
                &self,
                _g: &[AkisDeger],
                _p: &Parametreler,
            ) -> Result<Vec<AkisDeger>, String> {
                Err("kasıtlı hata".into())
            }
            fn tahmini_bayt(&self, _g: &[AkisDeger]) -> u64 {
                1024
            }
        }
        let mut kayit = YurutucuKayit::ornek();
        kayit.kaydet(NodeKaydi::yeni(tanim("patlar", "Patlar"), Arc::new(Patlar)));

        let mut g = NodeGraf::yeni("ana");
        // Dal A: dizi_oku(1) → patlar(2) → hizala(3) [patlar yüzünden atlanmalı]
        let a1 = g.yeni_node_kimlik();
        g.node_ekle_ham(
            NodeKatalogu::ornek()
                .bul("girdi.dizi_oku")
                .unwrap()
                .ornekle(a1, (0.0, 0.0)),
        );
        let a2 = g.yeni_node_kimlik();
        g.node_ekle_ham(Node {
            kimlik: a2,
            tur_kimligi: "patlar".into(),
            baslik: "P".into(),
            konum: (0.0, 0.0),
            girisler: vec![Port::yeni("g", "dizi")],
            cikislar: vec![Port::yeni("c", "dizi")],
            durum: NodeDurumu::Bekliyor,
        });
        let a3 = g.yeni_node_kimlik();
        g.node_ekle_ham(
            NodeKatalogu::ornek()
                .bul("isle.hizala")
                .unwrap()
                .ornekle(a3, (0.0, 0.0)),
        );
        // Dal B: bağımsız dizi_oku(4)
        let b1 = g.yeni_node_kimlik();
        g.node_ekle_ham(
            NodeKatalogu::ornek()
                .bul("girdi.dizi_oku")
                .unwrap()
                .ornekle(b1, (0.0, 0.0)),
        );

        let baglan = |g: &mut NodeGraf, a: NodeKimlik, b: NodeKimlik| {
            let bk = g.yeni_baglanti_kimlik();
            g.baglanti_ekle_ham(Baglanti {
                kimlik: bk,
                kaynak: PortRef::yeni(a, UiPortYonu::Cikis, 0),
                hedef: PortRef::yeni(b, UiPortYonu::Giris, 0),
            });
        };
        baglan(&mut g, a1, a2);
        baglan(&mut g, a2, a3);

        let mut onb = SonucOnbellek::yeni();
        let mut ilr = bos_ilerleme();
        let s = calistir(
            &g,
            &kayit,
            &HashMap::new(),
            &orkestrator(),
            &mut onb,
            &CalismaAyari::default(),
            &IptalJetonu::yeni(),
            &mut ilr,
        );
        // a1 ve b1 başarılı; a2 hata; a3 atlanmış.
        assert_eq!(s.node_sonuclari[&a1].durum, NodeDurumu::Bitti);
        assert_eq!(
            s.node_sonuclari[&b1].durum,
            NodeDurumu::Bitti,
            "bağımsız dal devam etti"
        );
        assert_eq!(s.node_sonuclari[&a2].durum, NodeDurumu::Hata);
        assert!(s.node_sonuclari[&a3].atlandi, "hatalı dalın altı atlandı");
        assert_eq!(s.hata_sayisi, 1);
        assert_eq!(s.atlanan, 1);
    }

    #[test]
    fn bellek_butcesi_asilmaz_oom_yok() {
        // Çok küçük bütçe (4 KiB); her node 1500 bayt ister → aynı anda en çok ~2 sığar.
        // Hepsi yine de tamamlanmalı (gruplara bölünür), hiç OOM olmamalı.
        struct Kucuk;
        impl NodeCalistirici for Kucuk {
            fn calistir(
                &self,
                _g: &[AkisDeger],
                _p: &Parametreler,
            ) -> Result<Vec<AkisDeger>, String> {
                Ok(vec![AkisDeger::yeni("dizi", "x", 1, 1500)])
            }
            fn tahmini_bayt(&self, _g: &[AkisDeger]) -> u64 {
                1500
            }
        }
        let mut kayit = YurutucuKayit::yeni();
        kayit.kaydet(NodeKaydi::yeni(tanim("kucuk", "Küçük"), Arc::new(Kucuk)));
        let mut g = NodeGraf::yeni("ana");
        for _ in 0..6 {
            let k = g.yeni_node_kimlik();
            g.node_ekle_ham(Node {
                kimlik: k,
                tur_kimligi: "kucuk".into(),
                baslik: "K".into(),
                konum: (0.0, 0.0),
                girisler: vec![],
                cikislar: vec![Port::yeni("c", "dizi")],
                durum: NodeDurumu::Bekliyor,
            });
        }
        let ork = BellekOrkestratoru::yeni(4096); // 4 KiB
        let mut onb = SonucOnbellek::yeni();
        let mut ilr = bos_ilerleme();
        let s = calistir(
            &g,
            &kayit,
            &HashMap::new(),
            &ork,
            &mut onb,
            &CalismaAyari {
                azami_worker: 8,
                canli_mod: false,
            },
            &IptalJetonu::yeni(),
            &mut ilr,
        );
        assert_eq!(
            s.hesaplanan, 6,
            "küçük bütçeyle bile hepsi tamamlanır (gruplandı)"
        );
        assert_eq!(s.hata_sayisi, 0);
        // Çalıştırma sonunda tüm bellek geri verilmiş olmalı.
        assert_eq!(ork.durum().rezerve, 0);
    }

    #[test]
    fn cok_buyuk_node_zarif_hata_verir() {
        // Tahmini bütçeden büyük → o node hata (OOM çökmesi değil), çökme yok.
        struct Devasa;
        impl NodeCalistirici for Devasa {
            fn calistir(
                &self,
                _g: &[AkisDeger],
                _p: &Parametreler,
            ) -> Result<Vec<AkisDeger>, String> {
                Ok(vec![])
            }
            fn tahmini_bayt(&self, _g: &[AkisDeger]) -> u64 {
                1024 * 1024 * 1024 // 1 GiB
            }
        }
        let mut kayit = YurutucuKayit::yeni();
        kayit.kaydet(NodeKaydi::yeni(tanim("devasa", "Devasa"), Arc::new(Devasa)));
        let mut g = NodeGraf::yeni("ana");
        let k = g.yeni_node_kimlik();
        g.node_ekle_ham(Node {
            kimlik: k,
            tur_kimligi: "devasa".into(),
            baslik: "D".into(),
            konum: (0.0, 0.0),
            girisler: vec![],
            cikislar: vec![],
            durum: NodeDurumu::Bekliyor,
        });
        let ork = BellekOrkestratoru::yeni(1024); // 1 KiB << 1 GiB
        let mut onb = SonucOnbellek::yeni();
        let mut ilr = bos_ilerleme();
        let s = calistir(
            &g,
            &kayit,
            &HashMap::new(),
            &ork,
            &mut onb,
            &CalismaAyari::default(),
            &IptalJetonu::yeni(),
            &mut ilr,
        );
        assert_eq!(s.hata_sayisi, 1);
        assert_eq!(s.node_sonuclari[&k].durum, NodeDurumu::Hata);
    }

    #[test]
    fn iptal_kalan_isi_durdurur() {
        let g = dogrusal_akis();
        let kayit = YurutucuKayit::ornek();
        let iptal = IptalJetonu::yeni();
        iptal.iptal_et(); // başlamadan iptal
        let mut onb = SonucOnbellek::yeni();
        let mut ilr = bos_ilerleme();
        let s = calistir(
            &g,
            &kayit,
            &HashMap::new(),
            &orkestrator(),
            &mut onb,
            &CalismaAyari::default(),
            &iptal,
            &mut ilr,
        );
        assert!(s.iptal_edildi);
        assert_eq!(s.hesaplanan, 0);
    }
}
