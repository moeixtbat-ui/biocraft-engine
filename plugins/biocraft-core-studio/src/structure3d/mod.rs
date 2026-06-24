//! ÇE-07 — **3B PDB/mmCIF Yapı Görüntüleyici**.
//!
//! Protein/nükleik asit/molekül 3B yapılarının (PDB/mmCIF, `data_io::structure`) GPU-hızlandırmalı
//! görselleştirilmesi.  Eklenti motorun wgpu render katmanına doğrudan bağlanamaz (MK-17); bu yüzden
//! görüntüleyici **render-bağımsız bir 3B sahne** ([`render::Sahne3B`]: küre/silindir/şerit + anlamsal
//! renk) + **kamera parametreleri** üretir; motorun render katmanı (biocraft-render, wgpu — Gün 6)
//! bunu instanced shader'larla çizer.  GPU yoksa/çökerse (Gün 5 TDR) aynı sahne **CPU yedeği** ile
//! 2B'ye projekte edilir ([`render::Ekran2B`]) → PNG/SVG dışa aktarma da buradan beslenir.
//!
//! ## Alt-modüller (`src/structure3d/`)
//! * [`render`] — 3B sahne ilkelleri + yörünge kamera/perspektif projeksiyon + PNG/SVG dışa aktarma.
//! * [`modes`] — gösterim modları (kartonet/top-çubuk/çubuk/dolgu) + bağ çıkarımı + renklendirme.
//! * [`interact`] — yörünge kamera kontrolü + atom seçimi (picking) + ölçüm (mesafe/açı).
//! * [`fallback`] — GPU tespiti + CPU yedeği + büyük yapı sadeleştirme + uyarı.
//! * [`viewer`] — [`Yapi3BGorunumu`] durum makinesi (hepsini bağlar).
//!
//! ## Bevy/THREE.js YOK (MK-01)
//! Tüm 3B doğrudan wgpu + özel geometri/shader (motor tarafı); harici 3B kütüphane kullanılmaz.

use biocraft_sdk::biocraft_types::Capability;
use biocraft_sdk::{Aktivasyon, YetkiKapisi};

pub mod fallback;
pub mod interact;
pub mod modes;
pub mod render;
pub mod viewer;

pub use fallback::{GpuDurumu, KaliteSeviyesi};
pub use interact::{Olcum3B, OlcumListesi, OlcumTur, YorungeKamera};
pub use modes::{GosterimModu, IkincilYapi, RenkSemasi, SahneAyar};
pub use render::{Ekran2B, Kamera, Palet3B, Parca2B, Renk3B, Sahne3B, Vec3};
pub use viewer::Yapi3BGorunumu;

/// Bu alt-modülün karşılık geldiği ÇE paketi.
pub const CE: &str = "ÇE-07";

/// Alt-modülün UI/komut kayıtları — 3B görüntüleyici **GPU** ile çizildiğinden yalnızca `gpu`
/// yetkisi verildiyse komutlar sunulur (en az yetki + dürüstlük; genom tarayıcı deseni).
/// Anlık görüntü dışa aktarma diske yazar → ayrıca `fs` ister (MK-13).
pub fn kayitlar(yetkiler: &YetkiKapisi) -> Aktivasyon {
    let mut akt = Aktivasyon::yeni();
    if yetkiler.var_mi(Capability::Gpu) {
        akt.komut(
            "biocraft.core.studio.structure.ac",
            "BioCraft Studio: 3B Yapı Görüntüleyici",
        )
        .komut(
            "biocraft.core.studio.structure.mod",
            "BioCraft Studio: 3B Gösterim Modu (kartonet/top-çubuk/çubuk/dolgu)",
        )
        .komut(
            "biocraft.core.studio.structure.renk",
            "BioCraft Studio: 3B Renklendirme (zincir/ikincil yapı/eleman/B-faktör)",
        )
        .komut(
            "biocraft.core.studio.structure.olcum",
            "BioCraft Studio: 3B Ölçüm (mesafe/açı)",
        );
        // Dışa aktarma diske yazar → fs gerekir.
        if yetkiler.var_mi(Capability::Fs) {
            akt.komut(
                "biocraft.core.studio.structure.disa_aktar",
                "BioCraft Studio: 3B Anlık Görüntü Dışa Aktar (PNG/SVG)",
            );
        }
    }
    akt
}

#[cfg(test)]
mod tests {
    use super::*;
    use biocraft_sdk::ui::UiUzantiTuru;

    #[test]
    fn gpu_yoksa_komut_yok() {
        assert_eq!(kayitlar(&YetkiKapisi::bos()).ui_say(UiUzantiTuru::Komut), 0);
        // Yalnız gpu → 4 görünüm komutu (dışa aktarma yok, fs gerekir).
        let gpu = kayitlar(&YetkiKapisi::yeni([Capability::Gpu]));
        assert_eq!(gpu.ui_say(UiUzantiTuru::Komut), 4);
        // gpu + fs → 5 (PNG/SVG dışa aktarma da sunulur).
        let gpu_fs = kayitlar(&YetkiKapisi::yeni([Capability::Gpu, Capability::Fs]));
        assert_eq!(gpu_fs.ui_say(UiUzantiTuru::Komut), 5);
        // Yalnız fs → 0 (3B görüntüleyici gpu ister).
        assert_eq!(
            kayitlar(&YetkiKapisi::yeni([Capability::Fs])).ui_say(UiUzantiTuru::Komut),
            0
        );
    }
}
