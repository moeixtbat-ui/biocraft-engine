//! Disk doluluk + yanlış-sürücü koruması (Zero-Impact) — İP-08, MK-25.
//!
//! **Yazma öncesi** aktif projenin bulunduğu sürücü kontrol edilir:
//! - Boş alan **%10 altına** inerse: uyarı (kullanıcı yer açsın).
//! - Boş alan **%2 altına** iner **veya** yazımdan sonra **100 MB güvenlik marjı**nın
//!   altına düşecekse: **salt-okunur** korumaya geçilir (disk dolması → veri kaybı/çökme önlenir).
//! - **Yanlış sürücüye yazma** koruması: hedef yol, açık projenin sürücüsünde mi?
//!
//! İzleme **sürücü başına**dır: her zaman ölçülen, projenin gerçekten yazıldığı sürücüdür
//! (spec dikkat notu: "Disk eşiği yanlış sürücüyü ölçüyor → aktif projenin sürücüsünü hedefle").
//! Karar mantığı **saf**tır; gerçek boş-alan sorgusu [`disk_durumu_oku`]'dadır (sysinfo).

use std::path::Path;

use biocraft_types::ErrorReport;

use crate::birim::insan_bayt;

const MB: u64 = 1024 * 1024;

/// Disk koruma eşikleri.  Varsayılan İP-08 spec değerleridir.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiskKoruma {
    /// Bu boş-oranın altında uyarı verilir (varsayılan 0.10 = %10).
    pub uyari_oran: f32,
    /// Bu boş-oranın altında salt-okunur korumaya geçilir (varsayılan 0.02 = %2).
    pub salt_okunur_oran: f32,
    /// Yazımdan sonra korunması gereken güvenlik marjı (bayt; varsayılan 100 MB).
    pub guvenlik_marji_bayt: u64,
}

impl Default for DiskKoruma {
    fn default() -> Self {
        Self {
            uyari_oran: 0.10,
            salt_okunur_oran: 0.02,
            guvenlik_marji_bayt: 100 * MB,
        }
    }
}

/// Bir sürücünün anlık doluluk durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskDurumu {
    /// Sürücünün toplam kapasitesi (bayt).
    pub toplam_bayt: u64,
    /// Boştaki alan (bayt).
    pub bos_bayt: u64,
}

impl DiskDurumu {
    /// Boş alan oranı (0.0–1.0).  Toplam 0 ise 0.0.
    pub fn bos_oran(&self) -> f32 {
        if self.toplam_bayt == 0 {
            0.0
        } else {
            (self.bos_bayt as f64 / self.toplam_bayt as f64) as f32
        }
    }
}

/// Yazma-öncesi disk kontrolünün sonucu.
#[derive(Debug, Clone, PartialEq)]
pub enum DiskKarari {
    /// Yeterli yer var → normal yaz.
    Normal,
    /// Yer azalıyor (%10 altı) → uyar ama yazmaya izin ver.
    Uyari {
        /// Boş alan oranı (0.0–1.0).
        bos_oran: f32,
        /// Kullanıcıya gösterilecek sade uyarı.
        ozet: String,
    },
    /// Yer kritik (%2 altı / marj altına düşecek) → **salt-okunur** koru (yazma reddedilir).
    SaltOkunur(ErrorReport),
}

impl DiskKarari {
    /// Yazmaya izin var mı?  Salt-okunur ise `false`.
    pub fn yazilabilir_mi(&self) -> bool {
        !matches!(self, DiskKarari::SaltOkunur(_))
    }
}

/// **Yazma öncesi disk kontrolü (MK-25).**  `yazilacak_bayt`: bu işlemde yazılacak tahmini boyut.
pub fn yazma_oncesi_kontrol(
    disk: DiskDurumu,
    yazilacak_bayt: u64,
    koruma: &DiskKoruma,
) -> DiskKarari {
    let oran = disk.bos_oran();
    let bos_sonra = disk.bos_bayt.saturating_sub(yazilacak_bayt);

    // Salt-okunur: %2 altı VEYA yazımdan sonra güvenlik marjının altına düşecek.
    if oran <= koruma.salt_okunur_oran || bos_sonra < koruma.guvenlik_marji_bayt {
        let hata = ErrorReport::new(
            "Disk neredeyse dolu — yazma korumalı",
            format!(
                "Hedef sürücüde yalnızca {} boş alan kaldı (%{:.1}). Bu işlem ~{} yazacaktı; \
                 disk dolarsa veri kaybı/çökme olabileceğinden geçici olarak salt-okunur moda geçildi.",
                insan_bayt(disk.bos_bayt),
                oran * 100.0,
                insan_bayt(yazilacak_bayt),
            ),
            "Sürücüde yer açın (gereksiz dosyaları silin) ya da projeyi başka bir sürücüye taşıyın; \
             en az 100 MB güvenlik marjı korunur.",
        )
        .with_eylem("Yer aç")
        .with_teknik_detay(format!(
            "disk koruması: toplam={}B bos={}B yazilacak={}B marj={}B oran={:.4}",
            disk.toplam_bayt, disk.bos_bayt, yazilacak_bayt, koruma.guvenlik_marji_bayt, oran
        ));
        return DiskKarari::SaltOkunur(hata);
    }

    // Uyarı: %10 altı ama henüz kritik değil.
    if oran <= koruma.uyari_oran {
        let ozet = format!(
            "Hedef sürücüde boş alan azalıyor: {} kaldı (%{:.0}). Yakında yer açmanız önerilir.",
            insan_bayt(disk.bos_bayt),
            oran * 100.0,
        );
        return DiskKarari::Uyari {
            bos_oran: oran,
            ozet,
        };
    }

    DiskKarari::Normal
}

/// **Yanlış sürücüye yazma koruması.**  `hedef`, açık projenin (`proje_koku`) sürücüsünde mi?
/// Windows'ta sürücü harfi (C:/D:), diğer platformlarda kök/ilk bileşen karşılaştırılır.
pub fn dogru_surucu_mu(hedef: &Path, proje_koku: &Path) -> bool {
    surucu_kimligi(hedef) == surucu_kimligi(proje_koku)
}

/// Bir yolun "sürücü kimliği"ni döndürür: Windows'ta sürücü harfi (büyük harfe normalize),
/// diğer platformlarda kök öncesi ilk bileşen ("/" tabanlı sistemde kök).  Karşılaştırma içindir.
fn surucu_kimligi(p: &Path) -> String {
    use std::path::{Component, Prefix};
    for bilesen in p.components() {
        match bilesen {
            // Windows: C:\... → "C"
            Component::Prefix(on) => {
                return match on.kind() {
                    Prefix::Disk(harf) | Prefix::VerbatimDisk(harf) => {
                        (harf as char).to_ascii_uppercase().to_string()
                    }
                    // UNC vb. → ön ekin tamamını kimlik say.
                    _ => on.as_os_str().to_string_lossy().to_ascii_uppercase(),
                };
            }
            // Unix kökü: "/" → tek kök; ilk normal bileşeni kimlik say (mount sezgisi).
            Component::RootDir => return "/".to_string(),
            _ => {}
        }
    }
    // Göreli yol → kimlik yok; boş (eşleşmeyi kullanıcıya bırak).
    String::new()
}

/// **Gerçek boş-alan sorgusu (sysinfo).**  `yol`u içeren sürücüyü bulup [`DiskDurumu`] döner;
/// bulunamazsa `None` (çökme yok).  İzleme **sürücü başına**dır: yolun ait olduğu en uzun
/// eşleşen bağlama noktası seçilir.
pub fn disk_durumu_oku(yol: &Path) -> Option<DiskDurumu> {
    let diskler = sysinfo::Disks::new_with_refreshed_list();
    let mut en_iyi: Option<(usize, DiskDurumu)> = None;
    for d in diskler.iter() {
        let bnokta = d.mount_point();
        if yol.starts_with(bnokta) {
            let uzunluk = bnokta.components().count();
            let durum = DiskDurumu {
                toplam_bayt: d.total_space(),
                bos_bayt: d.available_space(),
            };
            if en_iyi.as_ref().map(|(u, _)| uzunluk > *u).unwrap_or(true) {
                en_iyi = Some((uzunluk, durum));
            }
        }
    }
    en_iyi.map(|(_, d)| d)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1024 * MB;

    #[test]
    fn bol_disk_normal() {
        let disk = DiskDurumu {
            toplam_bayt: 500 * GB,
            bos_bayt: 200 * GB,
        };
        let karar = yazma_oncesi_kontrol(disk, GB, &DiskKoruma::default());
        assert_eq!(karar, DiskKarari::Normal);
        assert!(karar.yazilabilir_mi());
    }

    #[test]
    fn yuzde_on_alti_uyari_verir_ama_yazilir() {
        // %8 boş → uyarı, ama hâlâ yazılabilir.
        let disk = DiskDurumu {
            toplam_bayt: 100 * GB,
            bos_bayt: 8 * GB,
        };
        let karar = yazma_oncesi_kontrol(disk, GB, &DiskKoruma::default());
        assert!(matches!(karar, DiskKarari::Uyari { .. }));
        assert!(karar.yazilabilir_mi());
    }

    #[test]
    fn yuzde_iki_alti_salt_okunur() {
        // MK-25: %2 boş → salt-okunur koruma.
        let disk = DiskDurumu {
            toplam_bayt: 100 * GB,
            bos_bayt: GB, // %1
        };
        let karar = yazma_oncesi_kontrol(disk, 10 * MB, &DiskKoruma::default());
        assert!(!karar.yazilabilir_mi(), "%2 altında yazma reddedilmeli");
        match karar {
            DiskKarari::SaltOkunur(h) => {
                assert!(!h.ne_oldu.is_empty());
                assert!(!h.neden.is_empty());
                assert!(!h.nasil_cozulur.is_empty());
            }
            _ => panic!("SaltOkunur bekleniyordu"),
        }
    }

    #[test]
    fn guvenlik_marji_altina_dusurecek_yazma_reddedilir() {
        // %5 boş (uyarı bölgesi) ama yazım sonrası 100 MB marjın altına düşecek → salt-okunur.
        let disk = DiskDurumu {
            toplam_bayt: 100 * GB,
            bos_bayt: 5 * GB,
        };
        // 5 GB - 4.95 GB = 50 MB < 100 MB marj.
        let yazilacak = 5 * GB - 50 * MB;
        let karar = yazma_oncesi_kontrol(disk, yazilacak, &DiskKoruma::default());
        assert!(
            !karar.yazilabilir_mi(),
            "Marj altına düşürecek yazma reddedilmeli"
        );
    }

    #[test]
    fn bos_oran_dogru_hesaplanir() {
        let disk = DiskDurumu {
            toplam_bayt: 200 * GB,
            bos_bayt: 50 * GB,
        };
        assert!((disk.bos_oran() - 0.25).abs() < 1e-6);
        let bos_disk = DiskDurumu {
            toplam_bayt: 0,
            bos_bayt: 0,
        };
        assert_eq!(bos_disk.bos_oran(), 0.0);
    }

    #[cfg(windows)]
    #[test]
    fn yanlis_surucu_windows_harf_karsilastirir() {
        let proje = Path::new(r"C:\Users\x\proje");
        assert!(dogru_surucu_mu(
            Path::new(r"C:\Users\x\proje\cikti.bcproj"),
            proje
        ));
        assert!(!dogru_surucu_mu(Path::new(r"D:\baska\yer.bcproj"), proje));
        // Büyük/küçük harf normalize edilir.
        assert!(dogru_surucu_mu(Path::new(r"c:\Users\x\a"), proje));
    }

    #[cfg(unix)]
    #[test]
    fn yanlis_surucu_unix_kok_karsilastirir() {
        let proje = Path::new("/home/x/proje");
        assert!(dogru_surucu_mu(
            Path::new("/home/x/proje/cikti.bcproj"),
            proje
        ));
        // Unix'te tek kök; mutlak yollar aynı kökü paylaşır.
        assert!(dogru_surucu_mu(Path::new("/mnt/disk2/a"), proje));
    }
}
