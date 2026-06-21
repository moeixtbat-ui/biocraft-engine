//! Tier-2 WASM çalıştırıcı — Wasmtime sandbox (MK-12, MK-18).
//!
//! Sağladığı güvenceler:
//! * **Bellek limiti** — `StoreLimits` ile eklentinin doğrusal belleği sınırlanır
//!   (MK-18: WASM 4 GB duvarı; büyük iş host'a delege edilir, WASM'da tutulmaz).
//! * **CPU fuel** — `consume_fuel` ile sınırsız döngü/asılı kalma engellenir; fuel
//!   biterse çağrı güvenle durdurulur (trap).
//! * **Capability denetimi** — host fonksiyonları (dosya/ağ/…) çağrı başında
//!   [`YetkiKumesi::denetle`]'den geçer; yetki yoksa çağrı reddedilir (MK-13).
//! * **VFS** — dosya erişimi yalnızca kök-kısıtlı [`SanalDosyaSistemi`] handle'ı üzerinden;
//!   eklentiye gerçek yol verilmez.
//! * **AOT önbellek** — modül bir kez derlenir, serileştirilmiş artefakt önbelleğe alınır;
//!   tekrar yüklemede `deserialize` ile derleme atlanır.

use crate::capability::YetkiKumesi;
use crate::vfs::SanalDosyaSistemi;
use biocraft_ipc::{EklentiCagrisi, EklentiYaniti};
use biocraft_sdk::abi;
use biocraft_types::{Capability, ErrorReport};
use std::collections::HashMap;
use wasmtime::{Caller, Engine, Extern, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

use super::EklentiCalistirici;

/// Eklentiye uygulanacak kaynak limitleri.
#[derive(Debug, Clone, Copy)]
pub struct KaynakLimitleri {
    /// Doğrusal belleğin üst sınırı (bayt).  Varsayılan 16 MiB.
    pub bellek_bayt: usize,
    /// CPU fuel bütçesi (kabaca "yürütülebilir komut sayısı").  Varsayılan 100M.
    pub fuel: u64,
}

impl Default for KaynakLimitleri {
    fn default() -> Self {
        Self {
            // MK-18: WASM içi bellek sınırlı tutulur; ağır iş host'a delege edilir.
            bellek_bayt: 16 * 1024 * 1024,
            fuel: 100_000_000,
        }
    }
}

/// Bir WASM eklenti örneğinin çalışma-zamanı durumu (her çağrıda taze kurulur).
struct EklentiDurumu {
    /// Bellek limiti uygulayıcı (Store `limiter`'ına bağlanır).
    limits: StoreLimits,
    /// Bu örneğe verilmiş yetkiler (capability denetiminin kaynağı).
    yetkiler: YetkiKumesi,
    /// Kök-kısıtlı sanal dosya sistemi handle'ı.
    vfs: SanalDosyaSistemi,
    /// Eklentinin host günlüğüne yazdığı satırlar.
    gunluk: Vec<String>,
    /// Bir yetki reddi olduysa hangi yetki (çağrı sonucunu sınıflandırmak için).
    reddedilen_yetki: Option<Capability>,
}

/// Tier-2 WASM çalıştırıcı — tek bir yüklenmiş eklentiyi temsil eder.
pub struct WasmCalistirici {
    engine: Engine,
    module: Module,
    linker: Linker<EklentiDurumu>,
    yetkiler: YetkiKumesi,
    vfs: SanalDosyaSistemi,
    limitler: KaynakLimitleri,
}

impl WasmCalistirici {
    /// Derlenmiş bir modül + verilmiş yetkiler + VFS + limitlerle çalıştırıcı kurar.
    pub fn yeni(
        engine: Engine,
        module: Module,
        yetkiler: YetkiKumesi,
        vfs: SanalDosyaSistemi,
        limitler: KaynakLimitleri,
    ) -> Result<Self, ErrorReport> {
        let mut linker: Linker<EklentiDurumu> = Linker::new(&engine);
        host_fonksiyonlari_baglan(&mut linker)?;
        Ok(Self {
            engine,
            module,
            linker,
            yetkiler,
            vfs,
            limitler,
        })
    }

    /// Bu örnek için taze bir Store kurar (eklenti durumu çağrılar arası sızmaz).
    fn store_kur(&self) -> Result<Store<EklentiDurumu>, ErrorReport> {
        let durum = EklentiDurumu {
            limits: StoreLimitsBuilder::new()
                .memory_size(self.limitler.bellek_bayt)
                .build(),
            yetkiler: self.yetkiler.clone(),
            vfs: self.vfs.clone(),
            gunluk: Vec::new(),
            reddedilen_yetki: None,
        };
        let mut store = Store::new(&self.engine, durum);
        store.limiter(|d| &mut d.limits);
        store.set_fuel(self.limitler.fuel).map_err(|e| {
            ErrorReport::new(
                "Eklenti başlatılamadı",
                "CPU fuel ayarlanamadı",
                "Bu bir iç hatadır; lütfen tekrar deneyin",
            )
            .with_teknik_detay(e.to_string())
        })?;
        Ok(store)
    }
}

impl EklentiCalistirici for WasmCalistirici {
    fn cagir(&self, cagri: EklentiCagrisi) -> EklentiYaniti {
        let kid = cagri.correlation_id;
        let hata = |ne: &str, neden: String, e: Option<wasmtime::Error>| {
            let mut r = ErrorReport::new(
                ne,
                neden,
                "Eklenti günlüğünü inceleyin veya eklentiyi yeniden başlatın",
            )
            .with_correlation_id(kid);
            if let Some(e) = e {
                r = r.with_teknik_detay(format!("{e:?}"));
            }
            EklentiYaniti::Hata(r)
        };

        let mut store = match self.store_kur() {
            Ok(s) => s,
            Err(r) => return EklentiYaniti::Hata(r.with_correlation_id(kid)),
        };

        // Sandbox'a örnekle (bellek limiti burada da uygulanır: başlangıç belleği
        // limitten büyükse örnekleme başarısız olur).
        let instance = match self.linker.instantiate(&mut store, &self.module) {
            Ok(i) => i,
            Err(e) => {
                return hata(
                    "Eklenti yüklenemedi",
                    "WASM modülü sandbox'a yerleştirilemedi (bellek limiti veya bozuk modül)"
                        .into(),
                    Some(e),
                )
            }
        };

        let func = match instance.get_typed_func::<(), i32>(&mut store, &cagri.fonksiyon) {
            Ok(f) => f,
            Err(e) => {
                return hata(
                    "Eklenti fonksiyonu bulunamadı",
                    format!(
                        "eklentide '{}' adlı uygun bir fonksiyon yok",
                        cagri.fonksiyon
                    ),
                    Some(e),
                )
            }
        };

        match func.call(&mut store, ()) {
            Ok(donen) => {
                let gunluk = std::mem::take(&mut store.data_mut().gunluk);
                EklentiYaniti::Basari {
                    donen: donen as i64,
                    gunluk,
                }
            }
            Err(e) => {
                // Yetki reddi mi, yoksa başka bir trap mı (fuel/bellek/…)?
                if let Some(cap) = store.data().reddedilen_yetki {
                    let rapor = match self.yetkiler.denetle(cap) {
                        Err(r) => r.with_correlation_id(kid),
                        // denetle() Ok dönerse bu dala girilmez; yine de güvenli bir varsayılan.
                        Ok(()) => ErrorReport::new(
                            "Eklenti erişimi reddedildi",
                            "yetki denetimi başarısız",
                            "Eklenti yetkilerini gözden geçirin",
                        )
                        .with_correlation_id(kid),
                    };
                    EklentiYaniti::YetkiReddi { yetki: cap, rapor }
                } else {
                    hata(
                        "Eklenti çalışırken durduruldu",
                        "WASM çağrısı bir trap ile sonlandı (CPU fuel bitmiş, bellek sınırı aşılmış veya iç hata olabilir)".into(),
                        Some(e),
                    )
                }
            }
        }
    }
}

/// Host fonksiyonlarını ("biocraft" ad alanı) linker'a bağlar.
fn host_fonksiyonlari_baglan(linker: &mut Linker<EklentiDurumu>) -> Result<(), ErrorReport> {
    let baglama_hatasi = |ad: &str, e: wasmtime::Error| {
        ErrorReport::new(
            "Eklenti ortamı kurulamadı",
            format!("host fonksiyonu '{ad}' bağlanamadı"),
            "Bu bir iç hatadır; lütfen geliştiriciye bildirin",
        )
        .with_teknik_detay(e.to_string())
    };

    // gunluk_yaz(ptr, len) — yetki GEREKTİRMEZ; eklentinin günlüğe yazmasını sağlar.
    linker
        .func_wrap(
            abi::AD_ALANI,
            abi::IMPORT_GUNLUK_YAZ,
            |mut caller: Caller<'_, EklentiDurumu>,
             ptr: i32,
             len: i32|
             -> Result<(), wasmtime::Error> {
                let metin = bellekten_dizge(&mut caller, ptr, len)?;
                caller.data_mut().gunluk.push(metin);
                Ok(())
            },
        )
        .map_err(|e| baglama_hatasi(abi::IMPORT_GUNLUK_YAZ, e))?;

    // dosya_oku(ptr, len) -> i32 — fs YETKİSİ gerektirir; VFS üzerinden okur.
    linker
        .func_wrap(
            abi::AD_ALANI,
            abi::IMPORT_DOSYA_OKU,
            |mut caller: Caller<'_, EklentiDurumu>,
             ptr: i32,
             len: i32|
             -> Result<i32, wasmtime::Error> {
                // 1) Capability denetimi (MK-13) — yetki yoksa reddet + işaretle.
                if !caller.data().yetkiler.var_mi(Capability::Fs) {
                    caller.data_mut().reddedilen_yetki = Some(Capability::Fs);
                    return Err(wasmtime::Error::msg(
                        "yetki reddedildi: fs (eklenti dosya erişimine sahip değil)",
                    ));
                }
                // 2) Yolu bellekten oku.
                let yol = bellekten_dizge(&mut caller, ptr, len)?;
                // 3) VFS üzerinden (kök-kısıtlı) oku; gerçek yol eklentiye verilmez.
                match caller.data().vfs.oku(&yol) {
                    Ok(icerik) => Ok(icerik.len() as i32),
                    Err(rapor) => Err(wasmtime::Error::msg(format!(
                        "dosya okunamadı: {}",
                        rapor.neden
                    ))),
                }
            },
        )
        .map_err(|e| baglama_hatasi(abi::IMPORT_DOSYA_OKU, e))?;

    Ok(())
}

/// Eklentinin doğrusal belleğindeki `[ptr, ptr+len)` aralığını UTF-8 dizgeye çevirir.
/// Sınır dışı erişim güvenle hata döner (sandbox ihlali engellenir).
fn bellekten_dizge(
    caller: &mut Caller<'_, EklentiDurumu>,
    ptr: i32,
    len: i32,
) -> Result<String, wasmtime::Error> {
    let mem = match caller.get_export(abi::EXPORT_BELLEK) {
        Some(Extern::Memory(m)) => m,
        _ => {
            return Err(wasmtime::Error::msg(
                "eklenti zorunlu 'memory' dışa aktarımına sahip değil",
            ))
        }
    };
    let baslangic = ptr.max(0) as usize;
    let bitis = baslangic.saturating_add(len.max(0) as usize);
    let veri = mem.data(&*caller);
    let dilim = veri
        .get(baslangic..bitis)
        .ok_or_else(|| wasmtime::Error::msg("bellek sınırı dışı erişim"))?;
    Ok(String::from_utf8_lossy(dilim).into_owned())
}

// ─── AOT önbellek ─────────────────────────────────────────────────────────────

/// Derlenmiş WASM modüllerinin **AOT önbelleği**.
///
/// Anahtar = kaynak baytların BLAKE3 özeti (MK-33).  Bir modül ilk görüldüğünde
/// derlenir ve **serileştirilmiş** (cwasm) artefaktı saklanır; sonraki yüklemelerde
/// `deserialize` ile derleme atlanır (soğuk başlatmayı hızlandırır — MK-08).
#[derive(Default)]
pub struct AotOnbellek {
    harita: HashMap<[u8; 32], Vec<u8>>,
    isabet: usize,
    iska: usize,
}

impl AotOnbellek {
    /// Boş bir önbellek.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Kaynak baytları (WAT metni veya `.wasm`) derler; önbellekte varsa derlemez.
    pub fn derle_veya_onbellekten(
        &mut self,
        engine: &Engine,
        kaynak: &[u8],
    ) -> Result<Module, ErrorReport> {
        let anahtar = *blake3::hash(kaynak).as_bytes();

        if let Some(aot) = self.harita.get(&anahtar) {
            self.isabet += 1;
            // SAFETY: bu baytlar aynı süreçte, aynı `engine` ile üretildi (güvenilir kaynak).
            return unsafe { Module::deserialize(engine, aot) }.map_err(derleme_hatasi);
        }

        self.iska += 1;
        let module = Module::new(engine, kaynak).map_err(derleme_hatasi)?;
        // Serileştirme başarısız olsa bile derlenmiş modül kullanılabilir (önbelleksiz).
        if let Ok(aot) = module.serialize() {
            self.harita.insert(anahtar, aot);
        }
        Ok(module)
    }

    /// Önbellek isabet sayısı (teşhis/test).
    pub fn isabet_sayisi(&self) -> usize {
        self.isabet
    }

    /// Önbellek ıska (miss) sayısı (teşhis/test).
    pub fn iska_sayisi(&self) -> usize {
        self.iska
    }
}

fn derleme_hatasi(e: wasmtime::Error) -> ErrorReport {
    ErrorReport::new(
        "Eklenti derlenemedi",
        "WASM modülü geçersiz veya bozuk",
        "Eklentinin sağlam bir sürümünü yeniden kurun",
    )
    .with_teknik_detay(format!("{e:?}"))
}
