//! ÇE-07 — **3B yapı görüntüleyici durum makinesi** ([`Yapi3BGorunumu`]).
//!
//! Yüklenmiş bir [`Yapi`]'yi (PDB/mmCIF, `data_io`) tutar; gösterim modu + renk şeması + yörünge
//! kamera + seçim + ölçüm + gizli zincir + GPU durumunu yönetir.  İki çıktı üretir:
//! 1. **3B sahne** ([`Sahne3B`]) → motorun wgpu render katmanı (GPU yolu).
//! 2. **2B ekran listesi** ([`Ekran2B`]) → CPU yedeği + PNG/SVG dışa aktarma.
//!
//! Tümü render-bağımsız ve birim-testlenebilir (MK-17).

use std::collections::BTreeSet;

use crate::data_io::{Atom, Yapi, YapiModeli};

use super::fallback::{karar, kure_bolunme, uyari, yalniz_omurga_mu, GpuDurumu, KaliteSeviyesi};
use super::interact::{sec_atom, Olcum3B, OlcumListesi, YorungeKamera};
use super::modes::{sahne_olustur, GosterimModu, RenkSemasi, SahneAyar};
use super::render::{
    projeksiyon, Ekran2B, Etiket2B, Kure, Palet3B, Parca2B, Renk3B, Sahne3B, Vec3,
};

/// 3B yapı görüntüleyici durumu.
#[derive(Debug, Clone)]
pub struct Yapi3BGorunumu {
    yapi: Yapi,
    model: usize,
    /// Aktif gösterim modu.
    pub mod_: GosterimModu,
    /// Aktif renklendirme şeması.
    pub sema: RenkSemasi,
    /// Yörünge kamerası (döndür/yakınlaş/kaydır).
    pub kamera: YorungeKamera,
    /// Su (HOH) molekülleri gösterilsin mi?
    pub su_goster: bool,
    secili: Option<usize>,
    olcumler: OlcumListesi,
    gizli_zincirler: BTreeSet<String>,
    gpu: GpuDurumu,
}

impl Yapi3BGorunumu {
    /// Bir yapıyı görüntüleyiciye yükler; kamerayı yapıyı çerçeveleyecek şekilde kurar.
    pub fn yeni(yapi: Yapi, gpu: GpuDurumu) -> Self {
        let (min, max) = model_sinir(yapi.modeller.first());
        Self {
            yapi,
            model: 0,
            mod_: GosterimModu::Kartonet,
            sema: RenkSemasi::Zincir,
            kamera: YorungeKamera::cercevele(min, max),
            su_goster: false,
            secili: None,
            olcumler: OlcumListesi::yeni(),
            gizli_zincirler: BTreeSet::new(),
            gpu,
        }
    }

    // ── Model ──────────────────────────────────────────────────────────────────

    /// Aktif model.
    pub fn aktif_model(&self) -> &YapiModeli {
        &self.yapi.modeller[self.model.min(self.yapi.modeller.len().saturating_sub(1))]
    }

    /// Model sayısı (NMR yapılarında >1).
    pub fn model_sayisi(&self) -> usize {
        self.yapi.modeller.len()
    }

    /// Aktif model indeksini değiştirir (sınıra kıstırılır).
    pub fn model_sec(&mut self, indeks: usize) {
        self.model = indeks.min(self.model_sayisi().saturating_sub(1));
        self.secili = None;
    }

    /// Aktif modeldeki atom sayısı.
    pub fn atom_sayisi(&self) -> usize {
        self.aktif_model().atomlar.len()
    }

    /// Aktif modeldeki zincir kimlikleri (görünme sırasına göre).
    pub fn zincirler(&self) -> Vec<String> {
        let mut gorulen = Vec::new();
        for a in &self.aktif_model().atomlar {
            if !gorulen.contains(&a.zincir) {
                gorulen.push(a.zincir.clone());
            }
        }
        gorulen
    }

    // ── Kalite / GPU yedeği ──────────────────────────────────────────────────────

    /// Mevcut kalite seviyesi (atom sayısı + GPU durumundan).
    pub fn kalite(&self) -> KaliteSeviyesi {
        karar(self.atom_sayisi(), self.gpu)
    }

    /// GPU durumunu günceller (host cihaz kaybında/kurtarmada çağırır — Gün 5 TDR).
    pub fn gpu_ayarla(&mut self, gpu: GpuDurumu) {
        self.gpu = gpu;
    }

    /// GPU durumu.
    pub fn gpu(&self) -> GpuDurumu {
        self.gpu
    }

    /// Kullanıcıya gösterilecek uyarı (GPU yok / büyük yapı sadeleştirme) — yoksa `None`.
    pub fn uyari(&self) -> Option<String> {
        uyari(self.atom_sayisi(), self.gpu, self.kalite())
    }

    /// Motorun instanced küre LOD'u için tesselasyon ipucu.
    pub fn kure_bolunme(&self) -> u32 {
        kure_bolunme(self.kalite())
    }

    // ── Sahne (GPU) + projeksiyon (CPU/dışa aktarma) ─────────────────────────────

    fn ayar(&self) -> SahneAyar {
        SahneAyar {
            mod_: self.mod_,
            sema: self.sema,
            su_goster: self.su_goster,
            yalniz_omurga: yalniz_omurga_mu(self.kalite()),
        }
    }

    /// Render-bağımsız 3B sahne (motorun GPU katmanına teslim edilir).  Seçili atom vurgulanır.
    pub fn sahne(&self) -> Sahne3B {
        let mut sahne = sahne_olustur(self.aktif_model(), self.ayar(), &self.gizli_zincirler);
        // Seçili atom vurgusu (en üstte; picking'e karışmasın diye atom_indeksi yok).
        if let Some(i) = self.secili {
            if let Some(a) = self.aktif_model().atomlar.get(i) {
                sahne.kureler.push(Kure {
                    merkez: Vec3::yeni(a.x, a.y, a.z),
                    yaricap: 0.9,
                    renk: Renk3B::Secim,
                    atom_indeksi: None,
                });
            }
        }
        sahne
    }

    /// Görünen atom var mı? (Hepsi gizliyse görüntüleyici rehberi gösterilir — TDA 5.)
    pub fn bos_mu(&self) -> bool {
        self.sahne().kureler.is_empty() && self.sahne().seritler.is_empty()
    }

    /// 2B ekran listesi (CPU yedeği + dışa aktarma): sahneyi projekte eder + ölçüm çizgi/etiketi ekler.
    pub fn projeksiyon(&self, gen: f32, yuk: f32) -> Ekran2B {
        let kamera = self.kamera.kamera();
        let mut ekran = projeksiyon(&self.sahne(), &kamera, gen, yuk);
        // Ölçüm çizgileri + etiketleri en üstte (sıralamadan sonra eklenir → üste çizilir).
        let g = kamera.hazirla(gen, yuk);
        for o in &self.olcumler.olcumler {
            let noktalar: Vec<Vec3> = o
                .atomlar
                .iter()
                .filter_map(|&i| self.aktif_model().atomlar.get(i))
                .map(|a| Vec3::yeni(a.x, a.y, a.z))
                .collect();
            for pencere in noktalar.windows(2) {
                if let (Some(p1), Some(p2)) = (g.projekte(pencere[0]), g.projekte(pencere[1])) {
                    ekran.parcalar.push(Parca2B::Cizgi {
                        x1: p1.x,
                        y1: p1.y,
                        x2: p2.x,
                        y2: p2.y,
                        renk: Renk3B::Olcum,
                        kalinlik: 1.5,
                        derinlik: 0.0,
                    });
                }
            }
            // Etiket: ölçüm atomlarının ortasına.
            if let Some(orta) = noktalar.get(noktalar.len() / 2) {
                if let Some(p) = g.projekte(*orta) {
                    ekran.etiketler.push(Etiket2B {
                        x: p.x + 4.0,
                        y: p.y - 4.0,
                        icerik: o.etiket(),
                    });
                }
            }
        }
        ekran
    }

    // ── Kamera etkileşimi ────────────────────────────────────────────────────────

    /// Döndür (fare sürükleme deltası → radyan).
    pub fn dondur(&mut self, dyaw: f32, dpitch: f32) {
        self.kamera.dondur(dyaw, dpitch);
    }

    /// Yakınlaş/uzaklaş (`faktor<1` yakın).
    pub fn yakinlastir(&mut self, faktor: f32) {
        self.kamera.yakinlastir(faktor);
    }

    /// Kaydır (pan; piksel deltası).
    pub fn kaydir(&mut self, dx: f32, dy: f32) {
        self.kamera.kaydir(dx, dy);
    }

    /// Kamerayı yapıyı yeniden çerçeveleyecek şekilde sıfırlar.
    pub fn odakla(&mut self) {
        let (min, max) = model_sinir(Some(self.aktif_model()));
        self.kamera = YorungeKamera::cercevele(min, max);
    }

    // ── Seçim ────────────────────────────────────────────────────────────────────

    /// Ekran tıklamasından en öndeki atomu seçer (varsa); seçilen atom indeksini döndürür.
    pub fn sec_ekrandan(&mut self, sx: f32, sy: f32, gen: f32, yuk: f32) -> Option<usize> {
        let kamera = self.kamera.kamera();
        let secilen = sec_atom(&self.sahne(), &kamera, gen, yuk, sx, sy);
        self.secili = secilen;
        secilen
    }

    /// Seçili atom indeksi.
    pub fn secili(&self) -> Option<usize> {
        self.secili
    }

    /// Seçimi temizler (geri alınabilir etkileşim — TDA 2).
    pub fn secimi_temizle(&mut self) {
        self.secili = None;
    }

    /// Bir atoma erişim.
    pub fn atom(&self, indeks: usize) -> Option<&Atom> {
        self.aktif_model().atomlar.get(indeks)
    }

    /// Seçili atomun çok-satırlı bilgisi (inspector için).
    pub fn secili_bilgi(&self) -> Option<String> {
        let a = self.atom(self.secili?)?;
        Some(format!(
            "Atom: {} ({})\nKalıntı: {} {}{}\nB-faktör: {:.1}\nKonum: ({:.2}, {:.2}, {:.2}) Å",
            a.ad, a.element, a.kalinti, a.zincir, a.kalinti_no, a.b_faktor, a.x, a.y, a.z
        ))
    }

    // ── Zincir görünürlüğü ───────────────────────────────────────────────────────

    /// Bir zinciri gizler.
    pub fn zincir_gizle(&mut self, zincir: impl Into<String>) {
        self.gizli_zincirler.insert(zincir.into());
        self.secili = None;
    }

    /// Bir zinciri tekrar gösterir.
    pub fn zincir_goster(&mut self, zincir: &str) {
        self.gizli_zincirler.remove(zincir);
    }

    /// Bir zincir gizli mi?
    pub fn gizli_mi(&self, zincir: &str) -> bool {
        self.gizli_zincirler.contains(zincir)
    }

    // ── Ölçüm ────────────────────────────────────────────────────────────────────

    /// İki atom arası mesafe ölçümü ekler (Å).
    pub fn mesafe_ekle(&mut self, a: usize, b: usize) -> Option<f32> {
        let pa = self.atom(a)?;
        let pa = Vec3::yeni(pa.x, pa.y, pa.z);
        let pb = self.atom(b)?;
        let pb = Vec3::yeni(pb.x, pb.y, pb.z);
        let o = Olcum3B::mesafe(a, b, pa, pb);
        let d = o.deger;
        self.olcumler.ekle(o);
        Some(d)
    }

    /// Üç atom arası açı ölçümü ekler (derece; tepe `b`).
    pub fn aci_ekle(&mut self, a: usize, b: usize, c: usize) -> Option<f32> {
        let v = |i: usize| self.atom(i).map(|a| Vec3::yeni(a.x, a.y, a.z));
        let o = Olcum3B::aci(a, b, c, v(a)?, v(b)?, v(c)?);
        let d = o.deger;
        self.olcumler.ekle(o);
        Some(d)
    }

    /// Son ölçümü geri alır.
    pub fn olcum_geri_al(&mut self) -> Option<Olcum3B> {
        self.olcumler.geri_al()
    }

    /// Ölçümler.
    pub fn olcumler(&self) -> &[Olcum3B] {
        &self.olcumler.olcumler
    }

    // ── Dışa aktarma ─────────────────────────────────────────────────────────────

    /// Yüksek çözünürlüklü **PNG** anlık görüntü (saf-Rust raster; ÇE-11/Gün 42).
    pub fn png_disa_aktar(&self, gen: u32, yuk: u32, palet: &Palet3B) -> Vec<u8> {
        let ekran = self.projeksiyon(gen as f32, yuk as f32);
        super::render::png_olustur(&ekran, gen, yuk, palet)
    }

    /// **SVG** anlık görüntü (yayın kalitesi vektör; etiketler dâhil).
    pub fn svg_disa_aktar(&self, gen: f32, yuk: f32, palet: &Palet3B) -> String {
        let ekran = self.projeksiyon(gen, yuk);
        super::render::svg_olustur(&ekran, gen, yuk, palet)
    }
}

/// Bir modeldeki atomları kapsayan sınır kutusu (atomsuz/None → ±1 birim).
fn model_sinir(model: Option<&YapiModeli>) -> (Vec3, Vec3) {
    let varsayilan = (Vec3::yeni(-1.0, -1.0, -1.0), Vec3::yeni(1.0, 1.0, 1.0));
    let Some(model) = model else {
        return varsayilan;
    };
    let mut atomlar = model.atomlar.iter();
    let Some(ilk) = atomlar.next() else {
        return varsayilan;
    };
    let mut min = Vec3::yeni(ilk.x, ilk.y, ilk.z);
    let mut max = min;
    for a in std::iter::once(ilk).chain(atomlar) {
        min.x = min.x.min(a.x);
        min.y = min.y.min(a.y);
        min.z = min.z.min(a.z);
        max.x = max.x.max(a.x);
        max.y = max.y.max(a.y);
        max.z = max.z.max(a.z);
    }
    (min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(ad: &str, el: &str, zincir: &str, no: i64, x: f32, y: f32, z: f32) -> Atom {
        Atom {
            seri: 0,
            ad: ad.into(),
            kalinti: "ALA".into(),
            zincir: zincir.into(),
            kalinti_no: no,
            x,
            y,
            z,
            element: el.into(),
            b_faktor: 10.0,
            hetatm: false,
        }
    }

    fn ornek_yapi() -> Yapi {
        // İki zincirli küçük yapı (her zincir 3 Cα).
        let mut atomlar = Vec::new();
        for (z, ofset) in [("A", 0.0), ("B", 10.0)] {
            for i in 0..3 {
                atomlar.push(atom("CA", "C", z, i + 1, ofset + i as f32 * 2.0, 0.0, 0.0));
            }
        }
        Yapi {
            format: "PDB",
            modeller: vec![YapiModeli {
                model_no: 1,
                atomlar,
            }],
        }
    }

    #[test]
    fn yeni_kamerayi_cerceveler() {
        let g = Yapi3BGorunumu::yeni(ornek_yapi(), GpuDurumu::Var);
        // Kamera yapının merkezine bakmalı (x ekseninde yayılmış → merkez ~ orta).
        assert!(g.kamera.mesafe > 0.0);
        assert_eq!(g.atom_sayisi(), 6);
        assert_eq!(g.zincirler(), vec!["A", "B"]);
    }

    #[test]
    fn zincir_gizleme_sahneden_dusurur() {
        let mut g = Yapi3BGorunumu::yeni(ornek_yapi(), GpuDurumu::Var);
        g.mod_ = GosterimModu::Dolgu;
        let once = g.sahne().kureler.len();
        assert_eq!(once, 6);
        g.zincir_gizle("B");
        assert!(g.gizli_mi("B"));
        assert_eq!(g.sahne().kureler.len(), 3); // yalnız A
        g.zincir_goster("B");
        assert_eq!(g.sahne().kureler.len(), 6);
    }

    #[test]
    fn secim_ve_bilgi() {
        let mut g = Yapi3BGorunumu::yeni(ornek_yapi(), GpuDurumu::Var);
        g.mod_ = GosterimModu::Dolgu;
        // İlk atomun ekran izdüşümünü hesapla, tam oraya tıkla → o atom (ya da örtüşen biri) seçilmeli.
        let kamera = g.kamera.kamera();
        let p = kamera.hazirla(400.0, 400.0).projekte(Vec3::SIFIR).unwrap();
        let secilen = g.sec_ekrandan(p.x, p.y, 400.0, 400.0);
        assert!(secilen.is_some());
        assert!(g.secili_bilgi().unwrap().contains("Kalıntı"));
        g.secimi_temizle();
        assert!(g.secili().is_none());
    }

    #[test]
    fn olcum_mesafe_ve_geri_al() {
        let mut g = Yapi3BGorunumu::yeni(ornek_yapi(), GpuDurumu::Var);
        // A zincirinin ilk iki Cα'sı: 0 ve 2 → 2 Å.
        let d = g.mesafe_ekle(0, 1).unwrap();
        assert!((d - 2.0).abs() < 1e-4);
        assert_eq!(g.olcumler().len(), 1);
        g.olcum_geri_al();
        assert_eq!(g.olcumler().len(), 0);
    }

    #[test]
    fn gpu_kaybi_cpu_yedegi_uyarir_ve_projekte_eder() {
        let mut g = Yapi3BGorunumu::yeni(ornek_yapi(), GpuDurumu::Var);
        assert!(g.uyari().is_none());
        g.gpu_ayarla(GpuDurumu::cihaz_kaybi());
        assert_eq!(g.gpu(), GpuDurumu::Yok);
        assert!(g.uyari().unwrap().contains("CPU") || g.uyari().unwrap().contains("yedek"));
        // CPU yedeği projeksiyonu yine de geometri üretir (yapı kaybolmaz).
        let ekran = g.projeksiyon(300.0, 300.0);
        assert!(!ekran.parcalar.is_empty());
    }

    #[test]
    fn png_ve_svg_disa_aktarma() {
        let mut g = Yapi3BGorunumu::yeni(ornek_yapi(), GpuDurumu::Var);
        g.mod_ = GosterimModu::TopCubuk;
        let png = g.png_disa_aktar(120, 120, &Palet3B::yayin());
        assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        let svg = g.svg_disa_aktar(120.0, 120.0, &Palet3B::yayin());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }
}
