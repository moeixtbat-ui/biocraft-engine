//! biocraft-plugin-host — L3: Eklenti host'u (İP-07, 1. kısım — Gün 13).
//!
//! Eklenti mimarisinin **kalbi**: manifest keşfi/doğrulama, **Wasmtime WASM sandbox**
//! (bellek limiti + CPU fuel), **capability** (izin) denetimi ve **VFS** (sanal dosya
//! sistemi handle'ı).  Eklentiler diske/ağa doğrudan erişemez (MK-12, MK-13, MK-18);
//! birbirine değil yalnızca `biocraft-sdk` üzerinden bağlanır (MK-17).
//!
//! Akış: [`EklentiHost::kesfet`] → [`EklentiHost::yukle`] → [`YuklenmisEklenti::cagir`].
//!
//! Bu sürümde **Tier-2 WASM** (çekirdek modül) çalışır.  UI uzantı/imza/izolasyon/kurulum
//! ve Tier-3 (Python subprocess) / Component Model köprüsü **Gün 14+**'dır.

// ErrorReport zengin (ne/neden/çözüm) ve büyük bir tip; kullanıcı-görünür hatalarda
// onu doğrudan döndürüyoruz (biocraft-mem ile aynı desen).  Bu clippy uyarısı gerekçeli.
#![allow(clippy::result_large_err)]

pub mod capability;
pub mod discover;
pub mod manifest;
pub mod runtime;
pub mod vfs;

// Kontrat katmanlarını downstream'e yeniden dışa aktar (tek bağımlılıkla erişim).
pub use biocraft_ipc;
pub use biocraft_sdk;
pub use biocraft_types;

pub use capability::YetkiKumesi;
pub use discover::{kesfet, KesfedilenEklenti};
pub use manifest::{EklentiKimligi, Manifest};
pub use runtime::{AotOnbellek, EklentiCalistirici, KaynakLimitleri, WasmCalistirici};
pub use vfs::SanalDosyaSistemi;

use biocraft_ipc::{EklentiCagrisi, EklentiYaniti};
use biocraft_mem::{BellekBileseni, BellekOrkestratoru, Rezervasyon};
use biocraft_sdk::{AbiKontrati, EklentiKatmani};
use biocraft_types::{Capability, ErrorReport, Version};
use std::path::Path;
use wasmtime::{Config, Engine};

/// Eklenti host'u — keşif, uyumluluk denetimi, derleme/önbellek ve yükleme orkestratörü.
pub struct EklentiHost {
    engine: Engine,
    cekirdek_surum: Version,
    abi: AbiKontrati,
    orkestrator: BellekOrkestratoru,
    aot: AotOnbellek,
}

impl EklentiHost {
    /// Yeni bir host kurar.
    ///
    /// * `cekirdek_surum` — çalışan çekirdeğin sürümü (manifest uyumu için).
    /// * `orkestrator`    — **global** bellek orkestratörü (MK-21); eklenti belleği
    ///   buradan rezerve edilir, böylece toplam bütçe aşılırsa eklenti yüklenmez (MK-22).
    pub fn yeni(
        cekirdek_surum: Version,
        orkestrator: BellekOrkestratoru,
    ) -> Result<Self, ErrorReport> {
        let mut config = Config::new();
        // MK-18: CPU fuel ile sınırsız döngü/asılı kalma engellenir.
        config.consume_fuel(true);
        let engine = Engine::new(&config).map_err(|e| {
            ErrorReport::new(
                "Eklenti motoru başlatılamadı",
                "WASM çalışma-zamanı (Wasmtime) kurulamadı",
                "BioCraft'ı yeniden başlatın; sorun sürerse geliştiriciye bildirin",
            )
            .with_teknik_detay(e.to_string())
        })?;
        Ok(Self {
            engine,
            cekirdek_surum,
            abi: AbiKontrati::cekirdek(),
            orkestrator,
            aot: AotOnbellek::yeni(),
        })
    }

    /// Çekirdeğin ilan ettiği ABI kontratı.
    pub fn abi(&self) -> &AbiKontrati {
        &self.abi
    }

    /// AOT önbellek durumu `(isabet, iska)` — teşhis/test.
    pub fn aot_durumu(&self) -> (usize, usize) {
        (self.aot.isabet_sayisi(), self.aot.iska_sayisi())
    }

    /// Verilen dizindeki eklentileri keşfeder (alt klasör tarama + manifest doğrulama).
    pub fn kesfet(&self, dizin: &Path) -> Vec<Result<KesfedilenEklenti, ErrorReport>> {
        discover::kesfet(dizin)
    }

    /// Keşfedilmiş bir eklentiyi **varsayılan** kaynak limitleriyle yükler.
    pub fn yukle(
        &mut self,
        kesf: &KesfedilenEklenti,
        onaylanan_yetkiler: &[Capability],
    ) -> Result<YuklenmisEklenti, ErrorReport> {
        self.yukle_limitli(kesf, onaylanan_yetkiler, KaynakLimitleri::default())
    }

    /// Keşfedilmiş bir eklentiyi belirtilen kaynak limitleriyle yükler.
    ///
    /// Sırasıyla: (1) uyumluluk (ABI + çekirdek min/max), (2) katman desteği,
    /// (3) giriş dosyası okuma, (4) AOT derleme/önbellek, (5) **en az yetki** kümesi,
    /// (6) **bellek rezervasyonu** (bütçe aşılırsa reddedilir), (7) VFS + sandbox kurulumu.
    pub fn yukle_limitli(
        &mut self,
        kesf: &KesfedilenEklenti,
        onaylanan_yetkiler: &[Capability],
        limitler: KaynakLimitleri,
    ) -> Result<YuklenmisEklenti, ErrorReport> {
        let manifest = &kesf.manifest;

        // 1) Uyumluluk: ABI aynı major + çekirdek min/max (MK-14).
        manifest.uyumluluk_denetle(&self.cekirdek_surum, &self.abi.surum)?;

        // 2) Bu sürümde yalnızca WASM (Tier-2) çalışır; diğer katmanlar Gün 14+.
        if manifest.katman != EklentiKatmani::Wasm {
            return Err(ErrorReport::new(
                "Eklenti katmanı henüz desteklenmiyor",
                format!(
                    "'{}' katmanı bu sürümde çalıştırılamıyor (yalnızca wasm)",
                    manifest.katman.metni()
                ),
                "Eklentinin WASM (Tier-2) sürümünü kullanın; diğer katmanlar yakında",
            ));
        }

        // 3) Giriş dosyasını VFS köküyle (kök-kısıtlı) oku — kaçışa da kapalı.
        let vfs = SanalDosyaSistemi::yeni(&kesf.kok_dizin);
        let kaynak = vfs.oku(&manifest.giris)?;

        // 4) AOT derleme/önbellek.
        let module = self.aot.derle_veya_onbellekten(&self.engine, &kaynak)?;

        // 5) En az yetki: verilen = istenen ∩ onaylanan (MK-13).
        let yetkiler = YetkiKumesi::ver(&manifest.istenen_yetkiler, onaylanan_yetkiler);

        // 6) Bellek rezervasyonu (MK-21/MK-22): bütçe yetmezse eklenti yüklenmez.
        let rezervasyon = self.orkestrator.rezerve_et(
            BellekBileseni::Eklenti(manifest.kimlik.metni().to_string()),
            limitler.bellek_bayt as u64,
        )?;

        // 7) VFS + sandbox kurulumu.
        let calistirici =
            WasmCalistirici::yeni(self.engine.clone(), module, yetkiler.clone(), vfs, limitler)?;

        Ok(YuklenmisEklenti {
            manifest: manifest.clone(),
            verilen_yetkiler: yetkiler,
            calistirici,
            _rezervasyon: rezervasyon,
        })
    }
}

/// Yüklenmiş, çağrılabilir bir eklenti.  Düşürüldüğünde belleği orkestratöre iade eder (RAII).
pub struct YuklenmisEklenti {
    /// Eklentinin manifest'i.
    pub manifest: Manifest,
    /// Bu örneğe **fiilen verilmiş** yetkiler.
    pub verilen_yetkiler: YetkiKumesi,
    calistirici: WasmCalistirici,
    // RAII: drop = rezerve edilen bellek orkestratöre geri verilir.
    _rezervasyon: Rezervasyon,
}

impl YuklenmisEklenti {
    /// Eklentide adıyla bir fonksiyon çağırır.
    pub fn cagir(&self, fonksiyon: &str) -> EklentiYaniti {
        self.calistirici.cagir(EklentiCagrisi::yeni(fonksiyon))
    }

    /// Hazır bir çağrı zarfıyla (korelasyon kimliği taşıyan) fonksiyon çağırır.
    pub fn cagir_zarf(&self, cagri: EklentiCagrisi) -> EklentiYaniti {
        self.calistirici.cagir(cagri)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn ornek() -> KesfedilenEklenti {
        let dizin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ornek");
        discover::manifest_oku(&dizin).unwrap()
    }

    fn host() -> EklentiHost {
        // 256 MiB bütçeli orkestratör — varsayılan 16 MiB eklenti rezervasyonuna yeter.
        let ork = BellekOrkestratoru::yeni(256 * 1024 * 1024);
        EklentiHost::yeni(Version::new(0, 1, 0), ork).unwrap()
    }

    #[test]
    fn kesfet_yukle_merhaba_calisir() {
        let mut h = host();
        let e = h.yukle(&ornek(), &[]).unwrap();
        match e.cagir("merhaba") {
            EklentiYaniti::Basari { donen, gunluk } => {
                assert_eq!(donen, 16);
                assert!(gunluk.iter().any(|s| s.contains("Merhaba BioCraft")));
            }
            diger => panic!("Başarı bekleniyordu, gelen: {diger:?}"),
        }
    }

    #[test]
    fn yetki_yoksa_dosya_reddedilir() {
        // Manifest fs İSTER ama kullanıcı onaylamadı → fs verilmez → dosya_dene reddedilir.
        let mut h = host();
        let e = h.yukle(&ornek(), &[]).unwrap();
        assert!(!e.verilen_yetkiler.var_mi(Capability::Fs));
        match e.cagir("dosya_dene") {
            EklentiYaniti::YetkiReddi { yetki, .. } => assert_eq!(yetki, Capability::Fs),
            diger => panic!("YetkiReddi bekleniyordu, gelen: {diger:?}"),
        }
    }

    #[test]
    fn yetki_varsa_dosya_okunur() {
        // Kullanıcı fs onayladı → fs verilir → dosya_dene VFS üzerinden okur.
        let mut h = host();
        let e = h.yukle(&ornek(), &[Capability::Fs]).unwrap();
        assert!(e.verilen_yetkiler.var_mi(Capability::Fs));
        match e.cagir("dosya_dene") {
            EklentiYaniti::Basari { donen, .. } => assert!(donen > 0, "okunan bayt > 0 olmalı"),
            diger => panic!("Başarı bekleniyordu, gelen: {diger:?}"),
        }
    }

    #[test]
    fn abi_uyumsuz_eklenti_yuklenmez() {
        let mut h = host();
        let mut k = ornek();
        k.manifest.abi = Version::new(1, 0, 0); // çekirdek ABI 0.1 → farklı major (kırıcı)
                                                // YuklenmisEklenti (Ok tarafı) Debug değil → unwrap_err yerine eşleştir.
        let hata = match h.yukle(&k, &[]) {
            Err(e) => e,
            Ok(_) => panic!("uyumsuz eklenti yüklenmemeliydi"),
        };
        assert_eq!(hata.ne_oldu, "Eklenti bu sürümle uyumsuz");
    }

    #[test]
    fn aot_onbellek_ikinci_yuklemede_isabet() {
        let mut h = host();
        let _a = h.yukle(&ornek(), &[]).unwrap();
        assert_eq!(h.aot_durumu(), (0, 1), "ilk yükleme ıska olmalı");
        let _b = h.yukle(&ornek(), &[]).unwrap();
        assert_eq!(
            h.aot_durumu(),
            (1, 1),
            "ikinci yükleme önbellek isabeti olmalı"
        );
    }

    #[test]
    fn bellek_limiti_kucukse_calismaz() {
        // Modülün başlangıç belleği (1 sayfa = 64 KiB) limitten büyük → sandbox'a yerleşemez.
        let mut h = host();
        let limit = KaynakLimitleri {
            bellek_bayt: 1000,
            ..Default::default()
        };
        let e = h.yukle_limitli(&ornek(), &[], limit).unwrap();
        match e.cagir("merhaba") {
            EklentiYaniti::Hata(_) => {}
            diger => panic!("Bellek limiti hatası bekleniyordu, gelen: {diger:?}"),
        }
    }

    #[test]
    fn fuel_bitince_durur() {
        // Çok düşük fuel → ilk komutlarda trap (sınırsız döngü koruması).
        let mut h = host();
        let limit = KaynakLimitleri {
            fuel: 1,
            ..Default::default()
        };
        let e = h.yukle_limitli(&ornek(), &[], limit).unwrap();
        match e.cagir("merhaba") {
            EklentiYaniti::Hata(_) => {}
            diger => panic!("Fuel tükenme hatası bekleniyordu, gelen: {diger:?}"),
        }
    }

    #[test]
    fn olmayan_fonksiyon_net_hata() {
        let mut h = host();
        let e = h.yukle(&ornek(), &[]).unwrap();
        match e.cagir("olmayan_fonksiyon") {
            EklentiYaniti::Hata(r) => assert_eq!(r.ne_oldu, "Eklenti fonksiyonu bulunamadı"),
            diger => panic!("Hata bekleniyordu, gelen: {diger:?}"),
        }
    }

    #[test]
    fn bellek_butcesi_yetmezse_yuklenmez() {
        // 1 MiB bütçeli orkestratör; varsayılan 16 MiB eklenti rezervasyonu sığmaz (MK-22).
        let ork = BellekOrkestratoru::yeni(1024 * 1024);
        let mut h = EklentiHost::yeni(Version::new(0, 1, 0), ork).unwrap();
        assert!(h.yukle(&ornek(), &[]).is_err());
    }
}
