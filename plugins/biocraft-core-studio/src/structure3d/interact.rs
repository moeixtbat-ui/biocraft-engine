//! ÇE-07 — **Etkileşim**: yörünge kamera (döndür/yakınlaş/kaydır) + atom seçimi (picking) +
//! ölçüm (mesafe/açı, geri alınabilir).
//!
//! Yörünge kamerası hedef etrafında küresel koordinatla (yaw/pitch/mesafe) döner; seçim, 3B
//! sahnenin ekrana izdüşümünden **en öndeki** (en küçük derinlik) atomu bulur (ray döküm yerine
//! izdüşüm-mesafesi — saf-Rust, GPU'suz, birim-testlenir).

use super::render::{Kamera, Sahne3B, Vec3};

/// Hedef etrafında dönen perspektif **yörünge kamerası**.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct YorungeKamera {
    /// Bakılan merkez (yapının ağırlık merkezi).
    pub hedef: Vec3,
    /// Hedefe uzaklık.
    pub mesafe: f32,
    /// Yatay açı (azimut, radyan).
    pub yaw: f32,
    /// Dikey açı (yükseliş, radyan; ±~86°'ye kıstırılır).
    pub pitch: f32,
    /// Dikey görüş açısı (derece).
    pub fov_derece: f32,
}

/// Pitch sınırı (gimbal kilidini önler).
const PITCH_SINIR: f32 = 1.5; // ~86°
/// Yakınlaşma mesafe sınırları (Ångström).
const MESAFE_MIN: f32 = 1.5;
const MESAFE_MAX: f32 = 100_000.0;

impl YorungeKamera {
    /// Varsayılan açılarla bir kamera (hafif eğik bakış).
    pub fn yeni(hedef: Vec3, mesafe: f32) -> Self {
        Self {
            hedef,
            mesafe: mesafe.clamp(MESAFE_MIN, MESAFE_MAX),
            yaw: 0.6,
            pitch: 0.35,
            fov_derece: 45.0,
        }
    }

    /// Bir sınır kutusunu (`min`, `max`) tam çerçeveleyen kamera — yapı ekrana sığar.
    pub fn cercevele(min: Vec3, max: Vec3) -> Self {
        let merkez = min.topla(max).olcekle(0.5);
        let yaricap = (max.cikar(min).uzunluk() * 0.5).max(1.0);
        let fov = 45.0_f32;
        // Küreyi görüş konisine sığdır + %30 pay.
        let mesafe = yaricap / (fov.to_radians() * 0.5).sin() * 1.3;
        let mut k = Self::yeni(merkez, mesafe);
        k.fov_derece = fov;
        k
    }

    /// Göz (kamera) konumu — küresel koordinattan.
    pub fn goz(&self) -> Vec3 {
        let yon = Vec3::yeni(
            self.pitch.cos() * self.yaw.sin(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.cos(),
        );
        self.hedef.topla(yon.olcekle(self.mesafe))
    }

    /// Render kamerası (projeksiyon için).
    pub fn kamera(&self) -> Kamera {
        Kamera {
            goz: self.goz(),
            hedef: self.hedef,
            yukari: Vec3::yeni(0.0, 1.0, 0.0),
            fov_derece: self.fov_derece,
            yakin: (self.mesafe * 0.001).max(0.05),
        }
    }

    /// Döndür: fare sürüklemesinden (radyan deltası).
    pub fn dondur(&mut self, dyaw: f32, dpitch: f32) {
        self.yaw += dyaw;
        self.pitch = (self.pitch + dpitch).clamp(-PITCH_SINIR, PITCH_SINIR);
    }

    /// Yakınlaş/uzaklaş: `faktor < 1` yakınlaşır, `> 1` uzaklaşır (tekerlek).
    pub fn yakinlastir(&mut self, faktor: f32) {
        self.mesafe = (self.mesafe * faktor.max(0.01)).clamp(MESAFE_MIN, MESAFE_MAX);
    }

    /// Kaydır (pan): hedefi kamera düzleminde (sağ/yukarı) taşır (piksel deltası).
    pub fn kaydir(&mut self, dx: f32, dy: f32) {
        let goz = self.goz();
        let ileri = self.hedef.cikar(goz).normalize();
        let sag = ileri.capraz(Vec3::yeni(0.0, 1.0, 0.0)).normalize();
        let yukari = sag.capraz(ileri);
        // Piksel → dünya ölçeği mesafeyle orantılı.
        let k = self.mesafe * 0.0015;
        self.hedef = self
            .hedef
            .topla(sag.olcekle(-dx * k))
            .topla(yukari.olcekle(dy * k));
    }
}

/// Ekran tıklamasından **en öndeki** atomu seçer; izdüşmüş daire içine düşen, en küçük derinlikli
/// kürenin `atom_indeksi`'ni döndürür.  Hiçbiri içine düşmezse `None`.
pub fn sec_atom(
    sahne: &Sahne3B,
    kamera: &Kamera,
    gen: f32,
    yuk: f32,
    sx: f32,
    sy: f32,
) -> Option<usize> {
    let g = kamera.hazirla(gen, yuk);
    let mut en_iyi: Option<(f32, usize)> = None; // (derinlik, atom_indeksi)
    for k in &sahne.kureler {
        let Some(atom_idx) = k.atom_indeksi else {
            continue;
        };
        let Some(n) = g.projekte(k.merkez) else {
            continue;
        };
        let r = g.yaricap_px(k.yaricap, n.derinlik).max(3.0); // tıklama kolaylığı
        let dx = sx - n.x;
        let dy = sy - n.y;
        if dx * dx + dy * dy <= r * r {
            match en_iyi {
                Some((d, _)) if d <= n.derinlik => {}
                _ => en_iyi = Some((n.derinlik, atom_idx)),
            }
        }
    }
    en_iyi.map(|(_, i)| i)
}

// ─── Ölçüm (mesafe/açı) ───────────────────────────────────────────────────────────

/// Ölçüm türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OlcumTur {
    /// İki atom arası mesafe (Å).
    Mesafe,
    /// Üç atom arası açı (derece; tepe = ortadaki atom).
    Aci,
}

/// Bir ölçüm: ilgili atom indeksleri + hesaplanmış değer.
#[derive(Debug, Clone, PartialEq)]
pub struct Olcum3B {
    pub tur: OlcumTur,
    /// İlgili atom indeksleri (mesafe: 2, açı: 3).
    pub atomlar: Vec<usize>,
    /// Değer (Å ya da derece).
    pub deger: f32,
}

impl Olcum3B {
    /// İki atom arası mesafe ölçümü.
    pub fn mesafe(a: usize, b: usize, pa: Vec3, pb: Vec3) -> Self {
        Self {
            tur: OlcumTur::Mesafe,
            atomlar: vec![a, b],
            deger: pa.uzaklik(pb),
        }
    }

    /// Üç atom arası açı ölçümü (tepe `b`).
    pub fn aci(a: usize, b: usize, c: usize, pa: Vec3, pb: Vec3, pc: Vec3) -> Self {
        let u = pa.cikar(pb);
        let v = pc.cikar(pb);
        let nu = u.uzunluk();
        let nv = v.uzunluk();
        let deger = if nu < 1e-6 || nv < 1e-6 {
            0.0
        } else {
            (u.nokta(v) / (nu * nv))
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees()
        };
        Self {
            tur: OlcumTur::Aci,
            atomlar: vec![a, b, c],
            deger,
        }
    }

    /// Kullanıcıya görünen kısa etiket.
    pub fn etiket(&self) -> String {
        match self.tur {
            OlcumTur::Mesafe => format!("{:.2} Å", self.deger),
            OlcumTur::Aci => format!("{:.1}°", self.deger),
        }
    }
}

/// Geri alınabilir ölçüm listesi (TDA 0.2: seçim/ölçüm geri alınabilir).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OlcumListesi {
    pub olcumler: Vec<Olcum3B>,
}

impl OlcumListesi {
    /// Boş liste.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Ölçüm ekler.
    pub fn ekle(&mut self, o: Olcum3B) {
        self.olcumler.push(o);
    }

    /// Son ölçümü geri alır (silineni döndürür).
    pub fn geri_al(&mut self) -> Option<Olcum3B> {
        self.olcumler.pop()
    }

    /// Tüm ölçümleri temizler.
    pub fn temizle(&mut self) {
        self.olcumler.clear();
    }

    /// Ölçüm sayısı.
    pub fn sayi(&self) -> usize {
        self.olcumler.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure3d::render::{Kure, Renk3B};

    #[test]
    fn cercevele_yapiyi_sigdirir() {
        let kam = YorungeKamera::cercevele(Vec3::yeni(-5.0, -5.0, -5.0), Vec3::yeni(5.0, 5.0, 5.0));
        assert_eq!(kam.hedef, Vec3::SIFIR);
        // Göz, hedeften mesafe kadar uzakta.
        assert!((kam.goz().uzaklik(kam.hedef) - kam.mesafe).abs() < 1e-3);
    }

    #[test]
    fn dondurme_pitch_kistirir() {
        let mut kam = YorungeKamera::yeni(Vec3::SIFIR, 20.0);
        kam.dondur(0.0, 10.0); // aşırı yukarı
        assert!(kam.pitch <= PITCH_SINIR + 1e-6);
        kam.dondur(0.0, -100.0);
        assert!(kam.pitch >= -PITCH_SINIR - 1e-6);
    }

    #[test]
    fn yakinlastirma_sinirli() {
        let mut kam = YorungeKamera::yeni(Vec3::SIFIR, 20.0);
        for _ in 0..100 {
            kam.yakinlastir(0.5);
        }
        assert!(kam.mesafe >= MESAFE_MIN - 1e-6);
    }

    #[test]
    fn picking_en_on_atomu_secer() {
        let kam = YorungeKamera::yeni(Vec3::SIFIR, 20.0).kamera();
        let mut sahne = Sahne3B::yeni();
        // İki üst üste atom: yakın (z büyük, kameraya yakın) seçilmeli.
        sahne.kureler.push(Kure {
            merkez: Vec3::yeni(0.0, 0.0, -2.0),
            yaricap: 1.5,
            renk: Renk3B::ElemC,
            atom_indeksi: Some(10),
        });
        sahne.kureler.push(Kure {
            merkez: Vec3::yeni(0.0, 0.0, 2.0),
            yaricap: 1.5,
            renk: Renk3B::ElemN,
            atom_indeksi: Some(20),
        });
        // Ekran merkezine tıkla.
        let secilen = sec_atom(&sahne, &kam, 200.0, 200.0, 100.0, 100.0);
        assert_eq!(secilen, Some(20), "kameraya yakın atom seçilmeli");
        // Boş bölgeye tıkla → seçim yok.
        assert_eq!(sec_atom(&sahne, &kam, 200.0, 200.0, 0.0, 0.0), None);
    }

    #[test]
    fn olcum_mesafe_ve_aci() {
        let m = Olcum3B::mesafe(0, 1, Vec3::SIFIR, Vec3::yeni(3.0, 4.0, 0.0));
        assert!((m.deger - 5.0).abs() < 1e-4);
        assert!(m.etiket().contains("Å"));
        // Dik açı (90°).
        let a = Olcum3B::aci(
            0,
            1,
            2,
            Vec3::yeni(1.0, 0.0, 0.0),
            Vec3::SIFIR,
            Vec3::yeni(0.0, 1.0, 0.0),
        );
        assert!((a.deger - 90.0).abs() < 1e-3);
    }

    #[test]
    fn olcum_geri_alinabilir() {
        let mut l = OlcumListesi::yeni();
        l.ekle(Olcum3B::mesafe(
            0,
            1,
            Vec3::SIFIR,
            Vec3::yeni(1.0, 0.0, 0.0),
        ));
        l.ekle(Olcum3B::mesafe(
            1,
            2,
            Vec3::SIFIR,
            Vec3::yeni(2.0, 0.0, 0.0),
        ));
        assert_eq!(l.sayi(), 2);
        let silinen = l.geri_al().unwrap();
        assert!((silinen.deger - 2.0).abs() < 1e-4);
        assert_eq!(l.sayi(), 1);
    }
}
