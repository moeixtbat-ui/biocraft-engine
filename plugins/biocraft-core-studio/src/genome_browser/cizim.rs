//! ÇE-02 — **Çizim listesi (display list)** ve **kompozisyon** + **isabet testi**.
//!
//! Eklenti motorun render katmanına (wgpu/egui) doğrudan bağlı **değildir** (MK-17); bu yüzden
//! tarayıcı, çizilecekleri **render-bağımsız** ilkellere (`Primitif`) derler.  Motor (biocraft-ui
//! /-render, L3/L4) bu listeyi alıp GPU ile çizer; renkler anlamsal anahtarlarla
//! ([`CizimRengi`]) verilir → motor tasarım jetonuna (Gün 31.2a) eşler (sabit renk eklentide yok).
//!
//! Kompozisyon culling + LOD + downsampling uygular (MK-04/MK-09): ekran-dışı çizilmez, yoğun
//! bölge özetlenir.  Her görünür öğe için bir [`IsabetBolgesi`] (ekran dikdörtgeni + ipucu) da
//! üretilir → tooltip ve seçim (inspector) bunu kullanır.

use std::collections::HashSet;

use super::canvas::{GenomBolge, Tuval};
use super::lod::{
    gorunur_indeksler, kapsama_binle, lod_sec, seyrelt, seyrelt_koruyarak, yigin_yerlesimi,
    LodSeviyesi,
};
use super::measure::Olcum;
use super::multisample::OrnekKatman;
use super::reference::{Kodon, ReferansDizi};
use super::ruler::Cetvel;
use super::tracks::IzYer;
use super::veri::{OkumaParcasi, OzellikParcasi, Serit, VaryantParcasi, VaryantTuru};

/// Anlamsal çizim rengi — motor tasarım jetonuna eşler (eklentide sabit RGB yok; yalnız
/// dışa aktarılan dosya için [`super::disa_aktar::Palet`] somut RGB verir).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CizimRengi {
    /// Cetvel zemini.
    CetvelZemin,
    /// Cetvel çizgisi (işaret).
    CetvelCizgi,
    /// Cetvel etiket metni.
    CetvelMetin,
    /// İz zemini (alternatif/ayraç).
    IzZemin,
    /// İz adı/etiketi.
    IzEtiket,
    /// İleri şerit okuma.
    ReadIleri,
    /// Geri şerit okuma.
    ReadGeri,
    /// Düşük kaliteli (MAPQ) okuma — soluk.
    ReadDusuk,
    /// Kapsama (coverage) histogram çubuğu.
    KapsamaCubuk,
    /// Gen/transkript gövdesi (ince).
    Gen,
    /// Ekson/CDS kutusu (dolu).
    Ekson,
    /// Anotasyon etiketi.
    AnotasyonMetin,
    /// Yoğunluk/özet (uzak LOD).
    OzetYogunluk,
    /// Seçili öğe vurgusu.
    Secim,
    // ── Gün 37: referans dizi + çeviri ──
    /// Referans baz A.
    BazA,
    /// Referans baz C.
    BazC,
    /// Referans baz G.
    BazG,
    /// Referans baz T (RNA'da U).
    BazT,
    /// Referans baz N/belirsiz.
    BazN,
    /// Baz/aminoasit harfi (hücre üzerine).
    BazMetin,
    /// Aminoasit kodonu (çeviri).
    AminoAsit,
    /// Dur (stop) kodonu — vurgulu.
    AminoAsitDur,
    // ── Gün 37: varyant (mismatch/indel) ──
    /// Tek-nükleotit varyantı (SNV).
    VaryantSnv,
    /// İnsersiyon (ekleme).
    VaryantIns,
    /// Delesyon (silme).
    VaryantDel,
    // ── Gün 37: ölçüm + işaretleme ──
    /// Ölçüm aracı çizgisi.
    OlcumCizgi,
    /// Ölçüm aracı etiketi.
    OlcumMetin,
    /// İşaretlenmiş bölge bandı (yarı saydam).
    IsaretliBolge,
    // ── Gün 37: çoklu örnek karşılaştırma paleti ──
    /// Örnek A (vaka).
    OrnekA,
    /// Örnek B (kontrol).
    OrnekB,
    /// Örnek C.
    OrnekC,
    /// Örnek D.
    OrnekD,
}

/// Bir baz baytı için anlamsal renk (büyük/küçük harf büyük harfe indirilir; bilinmeyen → N).
pub fn baz_rengi(b: u8) -> CizimRengi {
    match b.to_ascii_uppercase() {
        b'A' => CizimRengi::BazA,
        b'C' => CizimRengi::BazC,
        b'G' => CizimRengi::BazG,
        b'T' | b'U' => CizimRengi::BazT,
        _ => CizimRengi::BazN,
    }
}

/// Metin yatay hizalaması.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetinHiza {
    /// Sola.
    Sol,
    /// Ortaya.
    Orta,
}

/// Render-bağımsız çizim ilkeli (ekran koordinatları, piksel).
#[derive(Debug, Clone, PartialEq)]
pub enum Primitif {
    /// Dolu dikdörtgen.
    Dikdortgen {
        x: f32,
        y: f32,
        gen: f32,
        yuk: f32,
        renk: CizimRengi,
    },
    /// Çizgi.
    Cizgi {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        renk: CizimRengi,
        kalinlik: f32,
    },
    /// Metin (sol-üst köşe veya ortalanmış).
    Metin {
        x: f32,
        y: f32,
        icerik: String,
        renk: CizimRengi,
        boyut: f32,
        hiza: MetinHiza,
    },
}

/// Bir çizilen öğenin ekran dikdörtgeni + üzerine gelince/tıklayınca kullanılacak meta.
#[derive(Debug, Clone, PartialEq)]
pub struct IsabetBolgesi {
    pub x: f32,
    pub y: f32,
    pub gen: f32,
    pub yuk: f32,
    /// Hangi iz?
    pub iz_kimlik: String,
    /// İz verisindeki **görünür** öğe sırası (o kareye ait; teşhis için).
    pub oge_indeksi: usize,
    /// Üzerine gelince gösterilecek kısa ipucu (tooltip).
    pub ipucu: String,
    /// Seçilince içerik (inspector) panelinde gösterilecek çok-satırlı detay.
    pub detay: String,
}

impl IsabetBolgesi {
    fn icerir(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.gen && y >= self.y && y < self.y + self.yuk
    }
}

/// Bir karede çizilecek ilkeller + isabet bölgeleri.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CizimListesi {
    /// Çizim ilkelleri (sıra = çizim sırası; sonraki üste gelir).
    pub primitifler: Vec<Primitif>,
    /// İsabet bölgeleri (tooltip/seçim).
    pub isabetler: Vec<IsabetBolgesi>,
}

impl CizimListesi {
    /// Boş liste.
    pub fn yeni() -> Self {
        Self::default()
    }

    fn dikdortgen(&mut self, x: f32, y: f32, gen: f32, yuk: f32, renk: CizimRengi) {
        self.primitifler.push(Primitif::Dikdortgen {
            x,
            y,
            gen,
            yuk,
            renk,
        });
    }

    fn cizgi(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, renk: CizimRengi, kalinlik: f32) {
        self.primitifler.push(Primitif::Cizgi {
            x1,
            y1,
            x2,
            y2,
            renk,
            kalinlik,
        });
    }

    fn metin(
        &mut self,
        x: f32,
        y: f32,
        icerik: impl Into<String>,
        renk: CizimRengi,
        boyut: f32,
        hiza: MetinHiza,
    ) {
        self.primitifler.push(Primitif::Metin {
            x,
            y,
            icerik: icerik.into(),
            renk,
            boyut,
            hiza,
        });
    }

    /// Toplam ilkel sayısı.
    pub fn ilkel_sayisi(&self) -> usize {
        self.primitifler.len()
    }

    /// Bir ekran noktasının üzerindeki **en üst** isabet bölgesi (tooltip/seçim).  Sona eklenen
    /// (üstte çizilen) önce kazanır.
    pub fn isabet_bul(&self, x: f32, y: f32) -> Option<&IsabetBolgesi> {
        self.isabetler.iter().rev().find(|i| i.icerir(x, y))
    }
}

/// Seçili öğeyi (ipucusu eşleşen isabet bölgesini) bir kenarlıkla vurgular.  Kompozisyondan sonra
/// çağrılır; öğe görünür pencerede yoksa (`false`) hiçbir şey çizilmez.
pub fn secim_vurgula(liste: &mut CizimListesi, secili_ipucu: &str) -> bool {
    let dikdortgen = liste
        .isabetler
        .iter()
        .find(|i| i.ipucu == secili_ipucu)
        .map(|b| (b.x, b.y, b.gen, b.yuk));
    if let Some((x, y, gen, yuk)) = dikdortgen {
        let k = 1.5;
        liste.cizgi(x, y, x + gen, y, CizimRengi::Secim, k);
        liste.cizgi(x, y + yuk, x + gen, y + yuk, CizimRengi::Secim, k);
        liste.cizgi(x, y, x, y + yuk, CizimRengi::Secim, k);
        liste.cizgi(x + gen, y, x + gen, y + yuk, CizimRengi::Secim, k);
        true
    } else {
        false
    }
}

/// Bir metnin yaklaşık piksel genişliği (etiketin kutuya sığıp sığmayacağını kestirmek için).
fn metin_genisligi(metin: &str, boyut: f32) -> f32 {
    metin.chars().count() as f32 * boyut * 0.6
}

// ─── Cetvel ─────────────────────────────────────────────────────────────────────

/// Koordinat cetvelini çizer (üst şerit): zemin + işaret çizgileri + büyük işaret etiketleri.
pub fn cetvel_ciz(liste: &mut CizimListesi, cetvel: &Cetvel, genislik_px: f32, yukseklik: f32) {
    liste.dikdortgen(0.0, 0.0, genislik_px, yukseklik, CizimRengi::CetvelZemin);
    for m in &cetvel.isaretler {
        let (uzun, kalin) = if m.buyuk {
            (yukseklik * 0.5, 1.5)
        } else {
            (yukseklik * 0.3, 1.0)
        };
        liste.cizgi(
            m.x_px,
            yukseklik - uzun,
            m.x_px,
            yukseklik,
            CizimRengi::CetvelCizgi,
            kalin,
        );
        if m.buyuk && !m.etiket.is_empty() {
            liste.metin(
                m.x_px + 2.0,
                2.0,
                &m.etiket,
                CizimRengi::CetvelMetin,
                11.0,
                MetinHiza::Sol,
            );
        }
    }
}

// ─── Kapsama (coverage) izi ─────────────────────────────────────────────────────

/// Kapsama histogramını çizer: görünen okumaları piksel-sütunu kovalara binler (out-of-core özet),
/// en yüksek derinliğe göre normalize ederek iz yüksekliğine sığdırır.
pub fn kapsama_ciz(
    liste: &mut CizimListesi,
    tyer: &IzYer,
    tuval: &Tuval,
    okumalar: &[OkumaParcasi],
) {
    let kova_sayisi = (tuval.genislik_px as usize).clamp(1, 4000);
    let kovalar = kapsama_binle(okumalar, &tuval.bolge, kova_sayisi);
    let maks = kovalar.iter().copied().max().unwrap_or(0);
    if maks == 0 {
        return;
    }
    let kova_gen = tuval.genislik_px / kova_sayisi as f32;
    for (k, &derinlik) in kovalar.iter().enumerate() {
        if derinlik == 0 {
            continue;
        }
        let h = (derinlik as f32 / maks as f32) * tyer.yukseklik;
        let x = k as f32 * kova_gen;
        liste.dikdortgen(
            x,
            tyer.y_alt() - h,
            kova_gen.max(1.0),
            h,
            CizimRengi::KapsamaCubuk,
        );
    }
}

// ─── Hizalama (read yığını) izi ─────────────────────────────────────────────────

const READ_MAKS_YUKSEKLIK: f32 = 11.0;
const READ_ASGARI_YUKSEKLIK: f32 = 2.0;

/// Hizalama çizim seçenekleri (LOD/downsampling) — argüman sayısını derli toplar.
#[derive(Debug, Clone, Copy)]
pub struct HizalamaSecenek<'a> {
    /// Bir karede çizilecek azami öğe sayısı (kare bütçesi; MK-04).
    pub oge_butcesi: usize,
    /// "Tam göster": özet/seyreltmeyi atla (hiçbir read gizlenmez; yoğun bölgede yavaş).
    pub tam_goster: bool,
    /// Seyreltmede her zaman korunacak (varyant üstündeki **önemli**) read indeksleri.
    pub korunan: &'a [usize],
}

impl<'a> HizalamaSecenek<'a> {
    /// Yalnız bütçeyle (tam göster kapalı, korunan yok) seçenek.
    pub fn butce(oge_butcesi: usize) -> Self {
        Self {
            oge_butcesi,
            tam_goster: false,
            korunan: &[],
        }
    }
}

/// Hizalama izini çizer: culling + LOD + downsampling (MK-04/MK-09).
///
/// * **Yoğun bölge** (Özet LOD): tek tek read yerine **yoğunluk özeti** çizilir (akıcılık) —
///   ancak `secenek.korunan` (varyant üstündeki **önemli** read'ler) **yine de tek tek** çizilir
///   (downsampling önemli read'i gizlemez; possible-error çözümü).
/// * `secenek.tam_goster` = `true` ise özet/seyreltme tümüyle **atlanır** (yoğun bölgede
///   yavaşlayabilir ama hiçbir read gizlenmez).
/// * Aksi halde (orta yoğunluk) yığınlanmış read kutuları; bütçe aşılırsa önemli read'ler
///   korunarak deterministik seyreltilir.
pub fn hizalama_ciz(
    liste: &mut CizimListesi,
    tyer: &IzYer,
    tuval: &Tuval,
    okumalar: &[OkumaParcasi],
    iz_kimlik: &str,
    secenek: HizalamaSecenek,
) {
    let HizalamaSecenek {
        oge_butcesi,
        tam_goster,
        korunan,
    } = secenek;
    let gorunur = gorunur_indeksler(okumalar, &tuval.bolge);
    let lod = lod_sec(tuval.bp_per_piksel(), gorunur.len(), oge_butcesi);

    if lod == LodSeviyesi::Ozet && !tam_goster {
        // Yoğunluk özeti (akıcılık korunur).
        let gorunur_okumalar: Vec<&OkumaParcasi> = gorunur.iter().map(|&i| &okumalar[i]).collect();
        ozet_yogunluk_ciz(liste, tyer, tuval, &gorunur_okumalar);
        // Önemli (korunan) read'leri özetin üstüne tek tek çiz → varyant kanıtı gizlenmez.
        let korunan_kume: HashSet<usize> = korunan.iter().copied().collect();
        let onemli: Vec<&OkumaParcasi> = gorunur
            .iter()
            .filter(|i| korunan_kume.contains(i))
            .map(|&i| &okumalar[i])
            .collect();
        if !onemli.is_empty() {
            read_kutulari_ciz(liste, tyer, tuval, &onemli, iz_kimlik);
        }
        return;
    }

    // "Tam göster"de seyreltme yok; aksi halde önemli read'ler korunarak seyreltilir.
    let secili = if tam_goster {
        gorunur.clone()
    } else {
        seyrelt_koruyarak(&gorunur, oge_butcesi, korunan)
    };
    let parcalar: Vec<&OkumaParcasi> = secili.iter().map(|&i| &okumalar[i]).collect();
    read_kutulari_ciz(liste, tyer, tuval, &parcalar, iz_kimlik);
}

/// Read'leri çakışmayan satırlara (pileup) yerleştirip kutu + isabet (tooltip/seçim) olarak çizer.
fn read_kutulari_ciz(
    liste: &mut CizimListesi,
    tyer: &IzYer,
    tuval: &Tuval,
    parcalar: &[&OkumaParcasi],
    iz_kimlik: &str,
) {
    let (yerler, satir_sayisi) = yigin_yerlesimi(parcalar, 1);
    if satir_sayisi == 0 {
        return;
    }
    let satir_h =
        (tyer.yukseklik / satir_sayisi as f32).clamp(READ_ASGARI_YUKSEKLIK, READ_MAKS_YUKSEKLIK);

    for yer in &yerler {
        let p = parcalar[yer.oge_indeksi];
        let y = tyer.y_ust + yer.satir as f32 * satir_h;
        if y + satir_h > tyer.y_alt() {
            continue; // iz yüksekliğine sığmayan alt satırlar çizilmez (dikey culling)
        }
        let (sol, sag) = tuval.aralik_ekran(p.bas, p.bit);
        let renk = if p.dusuk_kalite() {
            CizimRengi::ReadDusuk
        } else if p.serit == Serit::Geri {
            CizimRengi::ReadGeri
        } else {
            CizimRengi::ReadIleri
        };
        let gen = (sag - sol).max(1.0);
        let yuk = (satir_h - 1.0).max(1.0);
        liste.dikdortgen(sol, y, gen, yuk, renk);
        liste.isabetler.push(IsabetBolgesi {
            x: sol,
            y,
            gen,
            yuk,
            iz_kimlik: iz_kimlik.to_string(),
            oge_indeksi: yer.oge_indeksi,
            ipucu: p.ipucu(),
            detay: p.detay(),
        });
    }
}

/// Görünür öğeleri yoğunluk çubuğu olarak çizer (uzak/yoğun LOD özeti).
fn ozet_yogunluk_ciz(
    liste: &mut CizimListesi,
    tyer: &IzYer,
    tuval: &Tuval,
    ogeler: &[&OkumaParcasi],
) {
    let kova_sayisi = (tuval.genislik_px as usize).clamp(1, 4000);
    let kovalar = kapsama_binle(ogeler, &tuval.bolge, kova_sayisi);
    let maks = kovalar.iter().copied().max().unwrap_or(0);
    if maks == 0 {
        return;
    }
    let kova_gen = tuval.genislik_px / kova_sayisi as f32;
    for (k, &d) in kovalar.iter().enumerate() {
        if d == 0 {
            continue;
        }
        let h = (d as f32 / maks as f32) * tyer.yukseklik;
        liste.dikdortgen(
            k as f32 * kova_gen,
            tyer.y_alt() - h,
            kova_gen.max(1.0),
            h,
            CizimRengi::OzetYogunluk,
        );
    }
}

// ─── Anotasyon izi ──────────────────────────────────────────────────────────────

const OZELLIK_YUKSEKLIK: f32 = 12.0;

/// Anotasyon izini çizer: culling → yığınlanmış gen/ekson kutuları; yer varsa ad etiketi.
pub fn anotasyon_ciz(
    liste: &mut CizimListesi,
    tyer: &IzYer,
    tuval: &Tuval,
    ozellikler: &[OzellikParcasi],
    iz_kimlik: &str,
    oge_butcesi: usize,
) {
    let gorunur = gorunur_indeksler(ozellikler, &tuval.bolge);
    let lod = lod_sec(tuval.bp_per_piksel(), gorunur.len(), oge_butcesi);

    if lod == LodSeviyesi::Ozet {
        let gorunur_ozellikler: Vec<&OzellikParcasi> =
            gorunur.iter().map(|&i| &ozellikler[i]).collect();
        ozet_ozellik_ciz(liste, tyer, tuval, &gorunur_ozellikler);
        return;
    }

    let secili = seyrelt(&gorunur, oge_butcesi);
    let parcalar: Vec<&OzellikParcasi> = secili.iter().map(|&i| &ozellikler[i]).collect();
    let (yerler, satir_sayisi) = yigin_yerlesimi(&parcalar, 2);
    if satir_sayisi == 0 {
        return;
    }
    let satir_h = (OZELLIK_YUKSEKLIK + 4.0)
        .min(tyer.yukseklik / satir_sayisi as f32)
        .max(OZELLIK_YUKSEKLIK);

    for yer in &yerler {
        let p = parcalar[yer.oge_indeksi];
        let y = tyer.y_ust + yer.satir as f32 * satir_h;
        if y + OZELLIK_YUKSEKLIK > tyer.y_alt() + 0.01 {
            continue;
        }
        let (sol, sag) = tuval.aralik_ekran(p.bas, p.bit);
        let gen = (sag - sol).max(1.0);
        if p.ekson_mu() {
            liste.dikdortgen(sol, y, gen, OZELLIK_YUKSEKLIK, CizimRengi::Ekson);
        } else {
            // Gen/transkript: ince orta çizgi (intron benzeri) + ad.
            let orta = y + OZELLIK_YUKSEKLIK / 2.0;
            liste.cizgi(sol, orta, sag, orta, CizimRengi::Gen, 2.0);
        }
        // Etiket yalnız sığarsa (görsel kalabalık önlenir).
        let ad = p.gorunen_ad();
        if gen >= metin_genisligi(ad, 10.0) + 4.0 {
            liste.metin(
                sol + 2.0,
                y,
                ad,
                CizimRengi::AnotasyonMetin,
                10.0,
                MetinHiza::Sol,
            );
        }
        liste.isabetler.push(IsabetBolgesi {
            x: sol,
            y,
            gen,
            yuk: OZELLIK_YUKSEKLIK,
            iz_kimlik: iz_kimlik.to_string(),
            oge_indeksi: yer.oge_indeksi,
            ipucu: p.ipucu(),
            detay: p.detay(),
        });
    }
}

fn ozet_ozellik_ciz(
    liste: &mut CizimListesi,
    tyer: &IzYer,
    tuval: &Tuval,
    ogeler: &[&OzellikParcasi],
) {
    let kova_sayisi = (tuval.genislik_px as usize).clamp(1, 4000);
    let kovalar = kapsama_binle(ogeler, &tuval.bolge, kova_sayisi);
    let maks = kovalar.iter().copied().max().unwrap_or(0);
    if maks == 0 {
        return;
    }
    let kova_gen = tuval.genislik_px / kova_sayisi as f32;
    let orta = tyer.y_ust + tyer.yukseklik / 2.0;
    for (k, &d) in kovalar.iter().enumerate() {
        if d == 0 {
            continue;
        }
        let h = (d as f32 / maks as f32) * tyer.yukseklik;
        liste.dikdortgen(
            k as f32 * kova_gen,
            orta - h / 2.0,
            kova_gen.max(1.0),
            h,
            CizimRengi::OzetYogunluk,
        );
    }
}

// ─── Referans dizi izi (bazlar + çeviri) — Gün 37 ───────────────────────────────

/// Bir baz/aminoasit hücresinin üstüne harf yazmak için en küçük hücre genişliği (piksel).
const HARF_ASGARI_GEN: f32 = 7.0;

/// Referans dizi izini çizer: her baz **renkli hücre + harf** (yeterince yakınsa); `cerceveler`
/// verildiyse altına 3 okuma çerçevesinin aminoasit çevirisi (her kodon hücre + harf).
/// Çizim **yalnız görünen penceredeki** bazlara yapılır (out-of-core dizi zaten pencereye ait).
pub fn referans_ciz(
    liste: &mut CizimListesi,
    tyer: &IzYer,
    tuval: &Tuval,
    referans: &ReferansDizi,
    cerceveler: Option<&[Vec<Kodon>]>,
) {
    let cerc: &[Vec<Kodon>] = cerceveler.unwrap_or(&[]);
    let satir_say = 1 + cerc.len();
    let satir_h = (tyer.yukseklik / satir_say as f32).clamp(8.0, 16.0);

    // Baz satırı.
    for (i, &b) in referans.bazlar.iter().enumerate() {
        let pos = referans.konum(i);
        if !tuval.bolge.kapsar(pos) {
            continue;
        }
        let (sol, sag) = tuval.aralik_ekran(pos, pos);
        let gen = (sag - sol).max(1.0);
        liste.dikdortgen(sol, tyer.y_ust, gen, (satir_h - 1.0).max(1.0), baz_rengi(b));
        if gen >= HARF_ASGARI_GEN {
            let harf = (b as char).to_ascii_uppercase().to_string();
            liste.metin(
                sol + gen / 2.0,
                tyer.y_ust + 1.0,
                harf,
                CizimRengi::BazMetin,
                (satir_h - 3.0).clamp(8.0, 12.0),
                MetinHiza::Orta,
            );
        }
    }

    // Çeviri çerçeveleri (varsa).
    for (f, kodonlar) in cerc.iter().enumerate() {
        let y = tyer.y_ust + (f as f32 + 1.0) * satir_h;
        if y + satir_h > tyer.y_alt() + 0.5 {
            break; // ize sığmayan çerçeve çizilmez (dikey culling)
        }
        for k in kodonlar {
            if !tuval.bolge.ortusur(k.bas, k.bit) {
                continue;
            }
            let (sol, sag) = tuval.aralik_ekran(k.bas, k.bit);
            let gen = (sag - sol).max(1.0);
            let renk = if k.dur_mu() {
                CizimRengi::AminoAsitDur
            } else {
                CizimRengi::AminoAsit
            };
            liste.dikdortgen(sol, y, gen, (satir_h - 1.0).max(1.0), renk);
            if gen >= HARF_ASGARI_GEN {
                liste.metin(
                    sol + gen / 2.0,
                    y + 1.0,
                    k.amino.to_string(),
                    CizimRengi::BazMetin,
                    (satir_h - 3.0).clamp(8.0, 12.0),
                    MetinHiza::Orta,
                );
            }
        }
    }
}

// ─── Varyant izi (mismatch / insersiyon / delesyon vurgusu) — Gün 37 ─────────────

/// Bir varyant türünün anlamsal rengi.
fn varyant_rengi(tur: VaryantTuru) -> CizimRengi {
    match tur {
        VaryantTuru::Snv => CizimRengi::VaryantSnv,
        VaryantTuru::Insersiyon => CizimRengi::VaryantIns,
        VaryantTuru::Delesyon => CizimRengi::VaryantDel,
        VaryantTuru::Diger => CizimRengi::VaryantSnv,
    }
}

/// Varyant izini çizer: her varyant türüne göre renkli işaret + isabet (tooltip/seçim).
/// Çok yoğunsa (bütçe üstü) deterministik seyreltilir (akıcılık — MK-04).
pub fn varyant_ciz(
    liste: &mut CizimListesi,
    tyer: &IzYer,
    tuval: &Tuval,
    varyantlar: &[VaryantParcasi],
    iz_kimlik: &str,
    oge_butcesi: usize,
) {
    let gorunur = gorunur_indeksler(varyantlar, &tuval.bolge);
    let secili = seyrelt(&gorunur, oge_butcesi);
    for &i in &secili {
        let v = &varyantlar[i];
        let (sol, sag) = tuval.aralik_ekran(v.bas, v.bit);
        let gen = (sag - sol).max(2.0);
        liste.dikdortgen(sol, tyer.y_ust, gen, tyer.yukseklik, varyant_rengi(v.tur));
        liste.isabetler.push(IsabetBolgesi {
            x: sol,
            y: tyer.y_ust,
            gen,
            yuk: tyer.yukseklik,
            iz_kimlik: iz_kimlik.to_string(),
            oge_indeksi: i,
            ipucu: v.ipucu(),
            detay: v.detay(),
        });
    }
}

// ─── Çoklu örnek overlay kapsama (karşılaştırma) — Gün 37 ────────────────────────

/// Birden çok örneğin kapsamasını **tek lane**'de farklı renklerle (overlay/üst üste) çizer.
/// Normalizasyon **tüm örnekler arasında ortak** en yüksek derinliğe göredir → bar yükseklikleri
/// örnekler arası **karşılaştırılabilir** (vaka/kontrol).  Okluzyonu önlemek için ince çizgi.
pub fn kapsama_overlay_ciz(
    liste: &mut CizimListesi,
    tyer: &IzYer,
    tuval: &Tuval,
    katmanlar: &[OrnekKatman],
) {
    let kova_sayisi = (tuval.genislik_px as usize).clamp(1, 4000);
    let binlenmis: Vec<(CizimRengi, Vec<u32>)> = katmanlar
        .iter()
        .map(|k| {
            (
                k.renk,
                kapsama_binle(&k.okumalar, &tuval.bolge, kova_sayisi),
            )
        })
        .collect();
    let maks = binlenmis
        .iter()
        .flat_map(|(_, k)| k.iter().copied())
        .max()
        .unwrap_or(0);
    if maks == 0 {
        return;
    }
    let kova_gen = tuval.genislik_px / kova_sayisi as f32;
    for (renk, kovalar) in &binlenmis {
        for (k, &d) in kovalar.iter().enumerate() {
            if d == 0 {
                continue;
            }
            let h = (d as f32 / maks as f32) * tyer.yukseklik;
            liste.dikdortgen(
                k as f32 * kova_gen,
                tyer.y_alt() - h,
                kova_gen.max(1.0),
                1.5,
                *renk,
            );
        }
    }
}

// ─── Ölçüm + bölge işaretleme — Gün 37 ──────────────────────────────────────────

/// İki nokta arası ölçüm aracını çizer: yatay çizgi + uç çubukları + ortada bp etiketi (`y` ekran
/// dikeyi).
pub fn olcum_ciz(liste: &mut CizimListesi, tuval: &Tuval, olcum: &Olcum, y: f32) {
    let (sol, _) = tuval.aralik_ekran(olcum.sol(), olcum.sol());
    let (_, sag) = tuval.aralik_ekran(olcum.sag(), olcum.sag());
    liste.cizgi(sol, y, sag, y, CizimRengi::OlcumCizgi, 1.5);
    liste.cizgi(sol, y - 4.0, sol, y + 4.0, CizimRengi::OlcumCizgi, 1.5);
    liste.cizgi(sag, y - 4.0, sag, y + 4.0, CizimRengi::OlcumCizgi, 1.5);
    liste.metin(
        (sol + sag) / 2.0,
        y - 14.0,
        olcum.etiket(),
        CizimRengi::OlcumMetin,
        11.0,
        MetinHiza::Orta,
    );
}

/// İşaretlenmiş bir bölgeyi tüm tuval boyunca yarı saydam bir bantla vurgular (bölge seçme/işaret).
pub fn bolge_isaretle_ciz(
    liste: &mut CizimListesi,
    tuval: &Tuval,
    bolge: &GenomBolge,
    toplam_yukseklik: f32,
) {
    if bolge.kromozom != tuval.bolge.kromozom || !tuval.bolge.ortusur(bolge.baslangic, bolge.bitis)
    {
        return;
    }
    let bas = bolge.baslangic.max(tuval.bolge.baslangic);
    let bit = bolge.bitis.min(tuval.bolge.bitis);
    let (sol, sag) = tuval.aralik_ekran(bas, bit);
    liste.dikdortgen(
        sol,
        0.0,
        (sag - sol).max(1.0),
        toplam_yukseklik,
        CizimRengi::IsaretliBolge,
    );
}

#[cfg(test)]
mod tests {
    use super::super::reference::CeviriDurumu;
    use super::super::ruler::cetvel;
    use super::super::tracks::IzTuru;
    use super::*;

    fn tuval(bas: u64, bit: u64) -> Tuval {
        Tuval::yeni(1000.0, GenomBolge::yeni("chr1", bas, bit).unwrap())
    }

    fn tyer(tur: IzTuru, y: f32, h: f32) -> IzYer {
        IzYer {
            kimlik: "iz1".into(),
            tur,
            y_ust: y,
            yukseklik: h,
        }
    }

    fn okuma(bas: u64, bit: u64, serit: Serit, mapq: u8) -> OkumaParcasi {
        OkumaParcasi {
            ad: format!("r{bas}"),
            bas,
            bit,
            serit,
            mapq: Some(mapq),
        }
    }

    #[test]
    fn cetvel_zemin_ve_etiket_cizer() {
        let t = tuval(1, 1000);
        let c = cetvel(&t, 10);
        let mut l = CizimListesi::yeni();
        cetvel_ciz(&mut l, &c, t.genislik_px, 24.0);
        // En az bir zemin dikdörtgeni + bir etiket metni.
        assert!(l.primitifler.iter().any(|p| matches!(
            p,
            Primitif::Dikdortgen {
                renk: CizimRengi::CetvelZemin,
                ..
            }
        )));
        assert!(l
            .primitifler
            .iter()
            .any(|p| matches!(p, Primitif::Metin { .. })));
    }

    #[test]
    fn hizalama_yigin_ve_isabet() {
        let t = tuval(1, 1000);
        let yer = tyer(IzTuru::Hizalama, 24.0, 160.0);
        let okumalar = vec![
            okuma(100, 200, Serit::Ileri, 60),
            okuma(150, 250, Serit::Geri, 60), // ilkle çakışır → satır 1
            okuma(800, 900, Serit::Ileri, 5), // düşük kalite
        ];
        let mut l = CizimListesi::yeni();
        hizalama_ciz(
            &mut l,
            &yer,
            &t,
            &okumalar,
            "reads",
            HizalamaSecenek::butce(4000),
        );
        // 3 okuma → 3 dikdörtgen + 3 isabet.
        assert_eq!(l.isabetler.len(), 3);
        assert!(l.primitifler.iter().any(|p| matches!(
            p,
            Primitif::Dikdortgen {
                renk: CizimRengi::ReadDusuk,
                ..
            }
        )));
        assert!(l.primitifler.iter().any(|p| matches!(
            p,
            Primitif::Dikdortgen {
                renk: CizimRengi::ReadGeri,
                ..
            }
        )));

        // İsabet testi: ilk okumanın ortasına denk gelen ekran noktası.
        let (sol, _) = t.aralik_ekran(100, 200);
        let bulundu = l.isabet_bul(sol + 1.0, 25.0);
        assert!(bulundu.is_some());
        assert!(bulundu.unwrap().ipucu.contains("r100"));
    }

    #[test]
    fn yogun_bolge_ozete_duser() {
        // Çok sayıda okuma + küçük bütçe → Özet LOD → tek tek isabet üretilmez (yoğunluk çubuğu).
        let t = tuval(1, 1000);
        let yer = tyer(IzTuru::Hizalama, 24.0, 160.0);
        let okumalar: Vec<OkumaParcasi> = (1..=500)
            .map(|i| okuma(i, i + 50, Serit::Ileri, 60))
            .collect();
        let mut l = CizimListesi::yeni();
        hizalama_ciz(
            &mut l,
            &yer,
            &t,
            &okumalar,
            "reads",
            HizalamaSecenek::butce(100),
        ); // 100 < 500
        assert!(l.isabetler.is_empty(), "özet modda tek tek isabet yok");
        assert!(l.primitifler.iter().any(|p| matches!(
            p,
            Primitif::Dikdortgen {
                renk: CizimRengi::OzetYogunluk,
                ..
            }
        )));

        // "Tam göster": özet atlanır → tek tek read çizilir (isabet üretilir).
        let mut l2 = CizimListesi::yeni();
        hizalama_ciz(
            &mut l2,
            &yer,
            &t,
            &okumalar,
            "reads",
            HizalamaSecenek {
                oge_butcesi: 100,
                tam_goster: true,
                korunan: &[],
            },
        );
        assert!(!l2.isabetler.is_empty(), "tam göster: tek tek read çizilir");

        // Önemli read koruması: yoğun (özet) bölgede bile korunan read tek tek çizilir.
        // okumalar[41] = okuma(42, …) → ad "r42".
        let mut l3 = CizimListesi::yeni();
        hizalama_ciz(
            &mut l3,
            &yer,
            &t,
            &okumalar,
            "reads",
            HizalamaSecenek {
                oge_butcesi: 100,
                tam_goster: false,
                korunan: &[41],
            },
        );
        // Özet çubuğu + korunan read'in isabeti.
        assert!(l3.primitifler.iter().any(|p| matches!(
            p,
            Primitif::Dikdortgen {
                renk: CizimRengi::OzetYogunluk,
                ..
            }
        )));
        assert_eq!(l3.isabetler.len(), 1, "yalnız korunan read tek tek görünür");
        assert!(l3.isabetler[0].ipucu.contains("r42"));
    }

    #[test]
    fn kapsama_normalize_histogram() {
        let t = tuval(1, 100);
        let yer = tyer(IzTuru::Kapsama, 24.0, 60.0);
        let okumalar = vec![
            okuma(1, 50, Serit::Ileri, 60),
            okuma(30, 100, Serit::Ileri, 60),
        ];
        let mut l = CizimListesi::yeni();
        kapsama_ciz(&mut l, &yer, &t, &okumalar);
        assert!(l.primitifler.iter().any(|p| matches!(
            p,
            Primitif::Dikdortgen {
                renk: CizimRengi::KapsamaCubuk,
                ..
            }
        )));
        // Çubuklar iz alt kenarından yukarı; hepsi iz dikey aralığında.
        for p in &l.primitifler {
            if let Primitif::Dikdortgen { y, yuk, .. } = p {
                assert!(*y >= yer.y_ust - 0.01 && y + yuk <= yer.y_alt() + 0.01);
            }
        }
    }

    #[test]
    fn anotasyon_ekson_ve_etiket() {
        let t = tuval(1, 2000);
        let yer = tyer(IzTuru::Anotasyon, 24.0, 40.0);
        let ozellikler = vec![
            OzellikParcasi {
                ad: Some("BRCA".into()),
                bas: 100,
                bit: 1500,
                serit: Serit::Ileri,
                tur: "gene".into(),
            },
            OzellikParcasi {
                ad: Some("e1".into()),
                bas: 100,
                bit: 300,
                serit: Serit::Ileri,
                tur: "exon".into(),
            },
        ];
        let mut l = CizimListesi::yeni();
        anotasyon_ciz(&mut l, &yer, &t, &ozellikler, "genler", 4000);
        assert_eq!(l.isabetler.len(), 2);
        // Ekson kutusu + gen çizgisi + en az bir etiket.
        assert!(l.primitifler.iter().any(|p| matches!(
            p,
            Primitif::Dikdortgen {
                renk: CizimRengi::Ekson,
                ..
            }
        )));
        assert!(l.primitifler.iter().any(|p| matches!(
            p,
            Primitif::Cizgi {
                renk: CizimRengi::Gen,
                ..
            }
        )));
        assert!(l
            .primitifler
            .iter()
            .any(|p| matches!(p, Primitif::Metin { .. })));
    }

    fn varyant(bas: u64, refa: &str, alt: &str, tur: VaryantTuru) -> VaryantParcasi {
        VaryantParcasi {
            kimlik: format!("v{bas}"),
            bas,
            bit: bas + (refa.len().max(1) as u64) - 1,
            referans: refa.into(),
            alternatifler: vec![alt.into()],
            tur,
        }
    }

    #[test]
    fn referans_bazlar_renkli_ve_harfli() {
        // 10 bp pencere, 1000 px → baz ~100 px → harfler çizilir.
        let t = tuval(1, 10);
        let yer = tyer(IzTuru::Referans, 24.0, 16.0);
        let referans = ReferansDizi {
            kromozom: "chr1".into(),
            baslangic: 1,
            bazlar: b"ACGTACGTAC".to_vec(),
        };
        let mut l = CizimListesi::yeni();
        referans_ciz(&mut l, &yer, &t, &referans, None);
        // Her baz türü için renkli hücre + harf.
        assert!(l.primitifler.iter().any(|p| matches!(
            p,
            Primitif::Dikdortgen {
                renk: CizimRengi::BazA,
                ..
            }
        )));
        assert!(l.primitifler.iter().any(|p| matches!(
            p,
            Primitif::Metin {
                renk: CizimRengi::BazMetin,
                ..
            }
        )));
    }

    #[test]
    fn referans_ceviri_cerceveleri_cizilir() {
        let t = tuval(1, 12);
        let yer = tyer(IzTuru::Referans, 24.0, 64.0); // baz + 3 çerçeve sığar
        let referans = ReferansDizi {
            kromozom: "chr1".into(),
            baslangic: 1,
            bazlar: b"ATGAAATTTTAA".to_vec(),
        };
        let c = CeviriDurumu {
            goster: true,
            serit: Serit::Ileri,
        };
        let cerc = c.cerceveler(&referans);
        let mut l = CizimListesi::yeni();
        referans_ciz(&mut l, &yer, &t, &referans, Some(&cerc));
        // Dur kodonu (TAA) çerçeve 0'da → AminoAsitDur rengi var.
        assert!(l.primitifler.iter().any(|p| matches!(
            p,
            Primitif::Dikdortgen {
                renk: CizimRengi::AminoAsitDur,
                ..
            }
        )));
        assert!(l.primitifler.iter().any(|p| matches!(
            p,
            Primitif::Dikdortgen {
                renk: CizimRengi::AminoAsit,
                ..
            }
        )));
    }

    #[test]
    fn varyant_turleri_renkli_ve_isabet() {
        let t = tuval(1, 1000);
        let yer = tyer(IzTuru::Varyant, 24.0, 20.0);
        let varyantlar = vec![
            varyant(100, "A", "G", VaryantTuru::Snv),
            varyant(300, "A", "ACGT", VaryantTuru::Insersiyon),
            varyant(500, "ACGT", "A", VaryantTuru::Delesyon),
        ];
        let mut l = CizimListesi::yeni();
        varyant_ciz(&mut l, &yer, &t, &varyantlar, "varyantlar", 4000);
        assert_eq!(l.isabetler.len(), 3);
        for renk in [
            CizimRengi::VaryantSnv,
            CizimRengi::VaryantIns,
            CizimRengi::VaryantDel,
        ] {
            assert!(
                l.primitifler
                    .iter()
                    .any(|p| matches!(p, Primitif::Dikdortgen { renk: r, .. } if *r == renk)),
                "varyant rengi yok: {renk:?}"
            );
        }
        // İsabet → tooltip/seçim.
        let (sol, _) = t.aralik_ekran(100, 100);
        let bulundu = l.isabet_bul(sol + 0.5, 30.0).unwrap();
        assert!(bulundu.ipucu.contains("SNV"));
    }

    #[test]
    fn overlay_kapsama_ortak_normalizasyon() {
        let t = tuval(1, 100);
        let yer = tyer(IzTuru::Kapsama, 24.0, 60.0);
        let katmanlar = vec![
            OrnekKatman {
                ad: "Vaka".into(),
                renk: CizimRengi::OrnekA,
                okumalar: vec![okuma(1, 50, Serit::Ileri, 60)],
            },
            OrnekKatman {
                ad: "Kontrol".into(),
                renk: CizimRengi::OrnekB,
                okumalar: vec![okuma(30, 100, Serit::Ileri, 60)],
            },
        ];
        let mut l = CizimListesi::yeni();
        kapsama_overlay_ciz(&mut l, &yer, &t, &katmanlar);
        // Her iki örneğin rengi de görünür.
        for renk in [CizimRengi::OrnekA, CizimRengi::OrnekB] {
            assert!(l
                .primitifler
                .iter()
                .any(|p| matches!(p, Primitif::Dikdortgen { renk: r, .. } if *r == renk)));
        }
    }

    #[test]
    fn olcum_ve_isaretli_bolge() {
        let t = tuval(1, 1000);
        let mut l = CizimListesi::yeni();
        olcum_ciz(&mut l, &t, &Olcum::yeni(100, 600), 50.0);
        // Etiket (mesafe) + çizgiler.
        assert!(l.primitifler.iter().any(|p| matches!(
            p,
            Primitif::Metin {
                renk: CizimRengi::OlcumMetin,
                ..
            }
        )));
        assert!(l.primitifler.iter().any(|p| matches!(
            p,
            Primitif::Cizgi {
                renk: CizimRengi::OlcumCizgi,
                ..
            }
        )));

        // İşaretli bölge bandı.
        let mut l2 = CizimListesi::yeni();
        bolge_isaretle_ciz(
            &mut l2,
            &t,
            &GenomBolge::yeni("chr1", 200, 400).unwrap(),
            300.0,
        );
        assert!(l2.primitifler.iter().any(|p| matches!(
            p,
            Primitif::Dikdortgen {
                renk: CizimRengi::IsaretliBolge,
                ..
            }
        )));
        // Farklı kromozom → çizilmez.
        let mut l3 = CizimListesi::yeni();
        bolge_isaretle_ciz(
            &mut l3,
            &t,
            &GenomBolge::yeni("chr2", 200, 400).unwrap(),
            300.0,
        );
        assert!(l3.primitifler.is_empty());
    }
}
