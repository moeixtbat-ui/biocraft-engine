//! biocraft-plugin-host — L3: Eklenti host'u (İP-07).
//!
//! Eklenti mimarisinin **kalbi**: manifest keşfi/doğrulama, **Wasmtime WASM sandbox**
//! (bellek limiti + CPU fuel), **Python/R out-of-process köprüsü** (MK-02), **capability**
//! denetimi, **VFS**, **UI uzantı kayıt defteri** (çakışma/sıra), **kriptografik imza**
//! (MK-16), **çökme izolasyonu** (MK-15), **kurulum/güncelleme/kaldırma** (`.bcext`) ve
//! **güvenli mod**.  Eklentiler diske/ağa doğrudan erişemez (MK-12/13/18); birbirine değil
//! yalnızca `biocraft-sdk` üzerinden bağlanır (MK-17).
//!
//! Akış: [`EklentiHost::kesfet`] → [`EklentiHost::yukle`] → [`YuklenmisEklenti::cagir`].
//! Yükleme; uyumluluk → **imza/bütünlük denetimi (politika)** → katman dağıtımı
//! (WASM sandbox **veya** ayrı süreç) → **en az yetki** → **bellek rezervasyonu** sırasını izler.

// ErrorReport zengin (ne/neden/çözüm) ve büyük bir tip; kullanıcı-görünür hatalarda
// onu doğrudan döndürüyoruz (biocraft-mem ile aynı desen).  Bu clippy uyarısı gerekçeli.
#![allow(clippy::result_large_err)]

pub mod capability;
pub mod discover;
pub mod exec;
pub mod harden;
pub mod install;
pub mod isolate;
pub mod manifest;
pub mod runtime;
pub mod safe_mode;
pub mod signature;
pub mod ui_ext;
pub mod vfs;

// Kontrat katmanlarını downstream'e yeniden dışa aktar (tek bağımlılıkla erişim).
pub use biocraft_ipc;
pub use biocraft_sdk;
pub use biocraft_types;

pub use capability::YetkiKumesi;
pub use discover::{kesfet, KesfedilenEklenti};
// İP-06: kod editöründen gelen kullanıcı kodunu ayrı süreçte çalıştırma (MK-02).
pub use exec::{calistir_baslat, CalismaModu, CalismaOlay, CalismaTutamac, KodCalismaLimitleri};
pub use harden::{cekirdek_arg_dogrula, AyristirmaLimitleri, SurecSinirlari};
pub use install::{BcextPaket, GuncellemeSonucu, Kurucu, KurulumSonucu};
pub use isolate::{CokmeKarari, EklentiSagligi, IzolasyonYoneticisi, KaynakKullanim};
pub use manifest::{EklentiKimligi, Manifest};
pub use runtime::{
    python_bul, AltSurecCalistirici, AltSurecLimitleri, AotOnbellek, EklentiCalistirici,
    KaynakLimitleri, WasmCalistirici,
};
pub use safe_mode::{GuvenliMod, GuvenliModSebep, YuklemeKarari};
pub use signature::{GuvenDeposu, Imza, ImzaDurumu, ImzaPolitikasi, Rozet, SigningKey};
pub use ui_ext::{Cakisma, KayitDefteri, KayitliUzanti, KullaniciTercihi};
pub use vfs::SanalDosyaSistemi;

use biocraft_ipc::{EklentiCagrisi, EklentiYaniti};
use biocraft_mem::{BellekBileseni, BellekOrkestratoru, Rezervasyon};
use biocraft_sdk::{AbiKontrati, EklentiKatmani};
use biocraft_types::{Capability, ErrorReport, Version};
use std::path::Path;
use wasmtime::{Config, Engine};

/// Eklenti host'u — keşif, uyumluluk/imza denetimi, derleme/önbellek ve yükleme orkestratörü.
pub struct EklentiHost {
    engine: Engine,
    cekirdek_surum: Version,
    abi: AbiKontrati,
    orkestrator: BellekOrkestratoru,
    aot: AotOnbellek,
    guven_deposu: GuvenDeposu,
    imza_politikasi: ImzaPolitikasi,
}

impl EklentiHost {
    /// Yeni bir host kurar (boş güven deposu + varsayılan imza politikası = imzasıza izin).
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
            guven_deposu: GuvenDeposu::bos(),
            imza_politikasi: ImzaPolitikasi::default(),
        })
    }

    /// Güven deposunu ayarlar (resmi/doğrulanmış yayıncı anahtarları — MK-16).
    pub fn guven_deposu_ile(mut self, depo: GuvenDeposu) -> Self {
        self.guven_deposu = depo;
        self
    }

    /// İmza politikasını ayarlar (kurumsal/kiosk için katı politika — MK-16).
    pub fn imza_politikasi_ile(mut self, politika: ImzaPolitikasi) -> Self {
        self.imza_politikasi = politika;
        self
    }

    /// Güven deposu (teşhis/UI).
    pub fn guven_deposu(&self) -> &GuvenDeposu {
        &self.guven_deposu
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

    /// Keşfedilmiş bir eklentiyi katmanına göre uygun çalıştırıcıyla yükler.
    ///
    /// WASM → sandbox (varsayılan limitler); Python → ayrı süreç (varsayılan alt-süreç
    /// limitleri).  Native/External bu sürümde desteklenmez.
    pub fn yukle(
        &mut self,
        kesf: &KesfedilenEklenti,
        onaylanan_yetkiler: &[Capability],
    ) -> Result<YuklenmisEklenti, ErrorReport> {
        match kesf.manifest.katman {
            EklentiKatmani::Wasm => {
                self.yukle_wasm(kesf, onaylanan_yetkiler, KaynakLimitleri::default())
            }
            EklentiKatmani::Python => {
                self.yukle_python(kesf, onaylanan_yetkiler, AltSurecLimitleri::default())
            }
            diger => Err(katman_desteklenmiyor(diger)),
        }
    }

    /// Bir WASM eklentisini belirtilen kaynak limitleriyle yükler.
    pub fn yukle_limitli(
        &mut self,
        kesf: &KesfedilenEklenti,
        onaylanan_yetkiler: &[Capability],
        limitler: KaynakLimitleri,
    ) -> Result<YuklenmisEklenti, ErrorReport> {
        self.yukle_wasm(kesf, onaylanan_yetkiler, limitler)
    }

    /// Bir Python eklentisini belirtilen alt-süreç limitleriyle yükler.
    pub fn yukle_python_limitli(
        &mut self,
        kesf: &KesfedilenEklenti,
        onaylanan_yetkiler: &[Capability],
        limitler: AltSurecLimitleri,
    ) -> Result<YuklenmisEklenti, ErrorReport> {
        self.yukle_python(kesf, onaylanan_yetkiler, limitler)
    }

    // ─── katmana özel yükleme yolları ───────────────────────────────────────

    fn yukle_wasm(
        &mut self,
        kesf: &KesfedilenEklenti,
        onaylanan_yetkiler: &[Capability],
        limitler: KaynakLimitleri,
    ) -> Result<YuklenmisEklenti, ErrorReport> {
        let manifest = &kesf.manifest;

        // 1) Uyumluluk: ABI aynı major + çekirdek min/max (MK-14).
        manifest.uyumluluk_denetle(&self.cekirdek_surum, &self.abi.surum)?;
        if manifest.katman != EklentiKatmani::Wasm {
            return Err(katman_desteklenmiyor(manifest.katman));
        }

        // 2) Giriş dosyasını VFS köküyle (kök-kısıtlı) oku — kaçışa kapalı.
        let vfs = SanalDosyaSistemi::yeni(&kesf.kok_dizin);
        let kaynak = vfs.oku(&manifest.giris)?;

        // 3) İmza/bütünlük denetimi (MK-16) + politika kapısı.
        let imza_durumu = self.imza_degerlendir(&vfs, &kaynak);
        self.imza_politikasi.denetle(&imza_durumu)?;

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
            imza_durumu,
            calistirici: Box::new(calistirici),
            _rezervasyon: rezervasyon,
        })
    }

    fn yukle_python(
        &mut self,
        kesf: &KesfedilenEklenti,
        onaylanan_yetkiler: &[Capability],
        limitler: AltSurecLimitleri,
    ) -> Result<YuklenmisEklenti, ErrorReport> {
        let manifest = &kesf.manifest;

        // 1) Uyumluluk.
        manifest.uyumluluk_denetle(&self.cekirdek_surum, &self.abi.surum)?;
        if manifest.katman != EklentiKatmani::Python {
            return Err(katman_desteklenmiyor(manifest.katman));
        }

        // 2) Betik baytlarını oku (imza için) + kök-kısıtlı gerçek yol (çalıştırma için).
        let vfs = SanalDosyaSistemi::yeni(&kesf.kok_dizin);
        let betik_baytlari = vfs.oku(&manifest.giris)?;
        let betik_yolu = vfs.cozumle(&manifest.giris)?;

        // 3) İmza/bütünlük + politika.
        let imza_durumu = self.imza_degerlendir(&vfs, &betik_baytlari);
        self.imza_politikasi.denetle(&imza_durumu)?;

        // 4) Python keşfi — yoksa **in-process'e DÖNME** (MK-02), net hata + kur rehberi.
        let python =
            runtime::subprocess::python_bul().ok_or_else(runtime::subprocess::python_yok_hatasi)?;
        runtime::subprocess::betik_dogrula(&betik_yolu)?;

        // 5) En az yetki.
        let yetkiler = YetkiKumesi::ver(&manifest.istenen_yetkiler, onaylanan_yetkiler);

        // 6) Host-tarafı bellek rezervasyonu (çocuğun RAM'i ayrı süreçtedir — MK-22).
        let rezervasyon = self.orkestrator.rezerve_et(
            BellekBileseni::Eklenti(manifest.kimlik.metni().to_string()),
            limitler.host_rezervasyon,
        )?;

        // 7) Ayrı-süreç köprüsü (MK-02).
        let calistirici = AltSurecCalistirici::yeni(python, betik_yolu, limitler);

        Ok(YuklenmisEklenti {
            manifest: manifest.clone(),
            verilen_yetkiler: yetkiler,
            imza_durumu,
            calistirici: Box::new(calistirici),
            _rezervasyon: rezervasyon,
        })
    }

    /// Eklenti klasöründe opsiyonel `imza.json`'u (giriş dosyasının imzası) değerlendirir.
    fn imza_degerlendir(&self, vfs: &SanalDosyaSistemi, giris_baytlari: &[u8]) -> ImzaDurumu {
        let imza = vfs
            .oku("imza.json")
            .ok()
            .and_then(|b| serde_json::from_slice::<Imza>(&b).ok());
        ImzaDurumu::degerlendir(giris_baytlari, imza.as_ref(), &self.guven_deposu)
    }
}

/// Native/External gibi henüz desteklenmeyen katmanlar için net hata.
fn katman_desteklenmiyor(katman: EklentiKatmani) -> ErrorReport {
    ErrorReport::new(
        "Eklenti katmanı henüz desteklenmiyor",
        format!(
            "'{}' katmanı bu sürümde çalıştırılamıyor (wasm ve python desteklenir)",
            katman.metni()
        ),
        "Eklentinin WASM veya Python sürümünü kullanın; diğer katmanlar yakında",
    )
}

/// Yüklenmiş, çağrılabilir bir eklenti.  Düşürüldüğünde belleği orkestratöre iade eder (RAII).
pub struct YuklenmisEklenti {
    /// Eklentinin manifest'i.
    pub manifest: Manifest,
    /// Bu örneğe **fiilen verilmiş** yetkiler.
    pub verilen_yetkiler: YetkiKumesi,
    /// Yükleme anındaki imza/bütünlük durumu (UI rozeti için — MK-16).
    pub imza_durumu: ImzaDurumu,
    // Katmandan bağımsız çalıştırıcı (WASM sandbox veya ayrı süreç köprüsü — MK-17).
    calistirici: Box<dyn EklentiCalistirici>,
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
