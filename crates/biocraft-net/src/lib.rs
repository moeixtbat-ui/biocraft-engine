//! biocraft-net — L3: **dağıtık ağ PASİF kancaları** (İP-15; MK-50, MK-42, MK-43, MK-40).
//!
//! Bu crate, gelecekteki dağıtık (P2P) hesaplama ağının yalnızca **kancalarını** sağlar.  Gerçek
//! ağ/Iroh/iş dağıtımı/ekonomi bu sürümün kapsamı **değildir**; hepsi sonradan bir **eklenti** ile
//! gelir ve buradaki arayüzleri uygular.  Üç ilke crate'in tamamına hâkimdir:
//!
//! 1. **Eklenti yokken sıfır maliyet (MK-50):** Kanca kayıt defteri ([`DagitikAg`]) yalnızca bir
//!    `Option`'dur; eklenti yokken `None` → arka plan görevi/soket/tahsisat yoktur, tüm ağ yolları
//!    kısa devre yapar.  Ağ ayrıca **varsayılan KAPALI**'dır (kullanıcı açıkça açmadıkça çalışmaz).
//! 2. **Veri sınırı çekirdekte (MK-42/43):** P2P'ye giden tek tip [`P2pYuku`]'dür ve **tek** kurucusu
//!    her yükü çekirdek çıkış kapısından (`biocraft_data::privacy::classify::cikis_denetle(_,
//!    DisKanal::P2p)`) geçirir.  PHI/hassas veri yük olarak *inşa edilemez* + içerik türü enum'unda
//!    "ham veri" varyantı *yoktur* → P2P yalnızca metadata/sonuç/eklenti taşır.  Sınır eklentiye
//!    emanet değildir: kapı L2'de, bu kanal (L3) ona bağımlı; MK-40 ile kapının altına inilemez.
//! 3. **Yalnızca arayüz, gerçek bağlantı YOK:** Iroh ([`iroh`]) yalnızca bir arayüz iskeletidir;
//!    `iroh` crate'i bağımlılık olarak eklenmez, hiçbir QUIC bağlantısı kurulmaz.
//!
//! **Bio-kredi** ([`limits::BioKrediKanca`]) yalnızca kavramsal bir yer tutucudur (kripto DEĞİL —
//! ARCHITECTURE §13); gerçek ekonomi eklenti + hukukçu onayından sonra gelir.
//!
//! **İP-18 — Bilim Pazarı salt-okur akışı ([`feed`] + [`katalog`]):** Bu crate ayrıca platform içi
//! mağaza/haber içeriğinin **gelen (salt-okur)** akışını taşır.  P2P kancalarının aksine bu yön
//! **dışarı veri göndermez**; küratörlü uzak bir JSON/RSS akışının (MVP'de yerel/sentetik karşılığı)
//! asenkron + önbellekli + çevrimdışı-dayanıklı yükleyicisidir.  Doğrulama etiketleri **abartısızdır**
//! (sahte "doğrulandı" yok — MK-47/MK-48); güvenli render + dış-bağlantı onayı üst katmanda uygulanır.
// MK-40: L3 katmanı — yalnızca L0/L1/L2 katmanlarına bağlı; üst katman yasak.

// İP-16: `ErrorReport` projenin standart, zengin (çok-alanlı) kullanıcı-görünür hata tipidir.
// Sağlayıcı/kanca yollarında bu tip `Box`'lanarak döner (büyük-err ergonomisi korunur); yine de
// ai-surface ile tutarlı olsun diye lint bilinçli kapatılır.
#![allow(clippy::result_large_err)]

pub mod contract;
pub mod feed;
pub mod hooks;
pub mod identity;
pub mod iroh;
pub mod job;
pub mod katalog;
pub mod limits;

pub use biocraft_ipc;
pub use biocraft_sdk;
pub use biocraft_types;

// Pratik kök-seviye yeniden dışa aktarımlar (üst katmanlar için tek içe-aktarım noktası).
pub use contract::{P2pIcerikTuru, P2pYuku};
// İP-18: Bilim Pazarı salt-okur içerik akışı (mağaza + haber) — veri modeli + asenkron yükleyici.
pub use feed::{kuratorlu_veri, PazarDurumu, PazarKaynagi, PazarYukleyici, YerelPazarKaynagi};
pub use hooks::{
    ag_kapali_hatasi, eklenti_yok_hatasi, sinir_ihlali_hatasi, AgDurumu, DagitikAg,
    DagitikAgSaglayici, SaglayiciKimlik, DAGITIK_AG_EKLENTI_URL,
};
pub use identity::{DugumKimlik, GuvenSeviyesi, ItibarKaydi, KimlikSaglayici};
pub use iroh::{BaglantiTutamac, DugumAdresi, IrohUcKancasi};
pub use job::{DayaniklilikPolitikasi, Is, IsDagitici, IsDurumu, IsKimlik, IsSonucu};
pub use katalog::{
    DogrulamaDurumu, Fiyat, HaberKarti, HaberTuru, Kategori, OgeTuru, PazarOgesi, PazarVerisi,
    RaporSebebi, Yorum,
};
pub use limits::{BioKrediKanca, KaynakSiniri};
