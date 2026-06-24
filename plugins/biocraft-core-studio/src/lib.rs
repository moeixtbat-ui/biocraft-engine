//! # BioCraft Studio — çekirdek eklenti (ÇE-00 iskeleti)
//!
//! BioCraft Engine'in üzerine oturan **asıl bilim eklentisi**: genom tarayıcı, varyant
//! inceleme, 3B yapı, dizi hizalama, anotasyon, veritabanı arama ve node entegrasyonu.
//! Kullanıcıya **tek tutarlı eklenti**; içeride mantıksal alt-modüller (0-CE.4).
//!
//! Bu crate **motordan AYRIDIR** ve **yalnızca [`biocraft_sdk`]** üzerinden çekirdekle
//! konuşur (MK-17).  Motor crate'lerine (app/ui/host/…) doğrudan bağımlılığı YOKTUR;
//! çekirdek tiplere bile SDK'nın yeniden dışa aktarımıyla (`biocraft_sdk::biocraft_types`)
//! erişir → host↔eklenti sınırı tektir.
//!
//! ## Yaşam döngüsü (İP-07 host'u ile)
//! 1. **Keşif:** host [`MANIFEST`] (`biocraft.toml`) okur; kimlik/ABI/yetki doğrular.
//! 2. **Yetki:** host `istenen ∩ onaylanan` kümeyi bir [`YetkiKapisi`]'ye koyar.
//! 3. **Aktivasyon:** host [`aktiflestir`]'i çağırır; eklenti UI/komut/node kayıtlarını
//!    ([`Aktivasyon`]) döndürür (Activity Bar paneli + komutlar + ileride node'lar).
//! 4. **İzolasyon:** host eklentiyi kaldırıp (kayıtları temizler) yeniden yükleyebilir;
//!    aktivasyon **saf**tır → her yüklemede aynı kayıtlar (yan etkisiz).
//!
//! ## Bağımsız sürümleme (MK-19)
//! Eklenti kendi sürümünü ([`SURUM`]) taşır; çekirdek ABI uyumu ([`ABI`]) korunur.

// ErrorReport zengin/büyük bir tip; yetki denetimi yüzeylerinde doğrudan döndürülür
// (biocraft-sdk/host ile aynı desen) — bilinçli karar.
#![allow(clippy::result_large_err)]

use biocraft_sdk::biocraft_types::{Capability, ErrorReport};

// SDK'yı + aktivasyon kontrat tiplerini ([`Aktivasyon`], [`YetkiKapisi`]) yeniden dışa aktar →
// app/host, eklentiyi yükleyip kayıtlarını bağlarken tek bağımlılıkla (eklenti crate'i üzerinden)
// SDK tiplerine erişir (MK-17: tek sınır).  `pub use` hem yerel kapsamı hem re-export'u sağlar.
pub use biocraft_sdk;
pub use biocraft_sdk::{Aktivasyon, YetkiKapisi};

// ─── 0-CE.4 alt-modül haritası (her biri bir ÇE paketine karşılık gelir) ──────────
// Şimdilik boş/iskelet; sonraki günler dolduracak.  Her modül SDK kayıt uzantı noktasını
// (`kayitlar(&YetkiKapisi) -> Aktivasyon`) açar; aktivasyonda toplanır.
pub mod alignment; // ÇE-03 — hizalama (read) görünümü [TEMEL]
pub mod annotation; // ÇE-05 — anotasyon + iz yönetimi [TEMEL]
pub mod compute; // ÇE-08 — analiz/hesap + harici araç köprüsü [TEMEL]
pub mod data_io; // ÇE-01 — format ayrıştırma, indeksleme, BGZF-farkında, BLAKE3, uzak erişim
pub mod db_search; // ÇE-09 — birleşik veritabanı arama + konektörler
pub mod export; // ÇE-11 — dışa aktarma + oturum
pub mod genome_browser; // ÇE-02 — genom tarayıcı tuvali
pub mod nodes; // ÇE-10 — node kayıtları [TEMEL]
pub mod perf; // ÇE-12 — performans/erişilebilirlik/doğruluk yardımcıları
pub mod sequence; // ÇE-06 — dizi görüntüleme/düzenleme + MSA [TEMEL]
pub mod structure3d; // ÇE-07 — 3B PDB/mmCIF görüntüleyici
pub mod variant; // ÇE-04 — VCF görünüm + filtre

// ─── Eklenti kimliği/sürümü (manifest ile birebir tutarlı; test bunu doğrular) ────

/// Eklenti kimliği — spec 0-CE.1 (`biocraft.<yayinci>.<eklenti>`).
pub const KIMLIK: &str = "biocraft.core.studio";
/// Kullanıcıya görünen ad.
pub const AD: &str = "BioCraft Studio";
/// Eklenti sürümü (çekirdekten bağımsız — MK-19).
pub const SURUM: &str = "0.1.0";
/// Hedeflenen ABI sürümü (MK-14; çekirdek ABI ile aynı major olmalı).
pub const ABI: &str = "0.1";

/// Activity Bar girişi + Side Panel sekmesini temsil eden panel kaydının kimliği.
pub const PANEL_KIMLIK: &str = "biocraft.core.studio.panel";

/// Gömülü manifest metni — host ayrı bir dosya okumadan da kimliği/yetkileri görebilir.
/// (Gerçek keşifte host yine de dizindeki `biocraft.toml`'u okur; bu, test/teşhis kolaylığı.)
pub const MANIFEST: &str = include_str!("../biocraft.toml");

/// Manifest'te **ilan edilen** yetkiler (`biocraft.toml [yetkiler].istenen` ile birebir).
///
/// `net` Gün 35'te (ÇE-01 uzak erişim) eklendi.  İlan ≠ otomatik kullanım: kullanıcı kurulumda
/// onaylamazsa (istenen ∩ onaylanan) net çalışmada yine reddedilir (MK-13).
pub const ISTENEN_YETKILER: &[Capability] = &[
    Capability::Fs,
    Capability::Gpu,
    Capability::Db,
    Capability::Ai,
    Capability::Net,
];

/// Birinci-parti çekirdek eklenti için **varsayılan** yetki kapısı: ilan edilen tüm yetkiler
/// verilmiş kabul edilir (kullanıcı kurulumda onaylar; çekirdek eklenti varsayılan kurulu — MK-19).
///
/// Gerçek host yolunda kapı `YetkiKumesi::ver(istenen, onaylanan).kapi()` ile üretilir;
/// bu yardımcı yan-yükleme (geliştirme) ve örnek/test içindir.
pub fn yetki_kapisi_varsayilan() -> YetkiKapisi {
    YetkiKapisi::yeni(ISTENEN_YETKILER.iter().copied())
}

/// **Aktivasyon giriş noktası** — host eklentiyi yükleyince çağırır (İP-07).
///
/// Eklentinin tüm UI/komut/node kayıtlarını toplar:
/// * Activity Bar girişi + Side Panel'i temsil eden **"BioCraft Studio" paneli**,
/// * birkaç **komut** (palette) — "Hoş Geldin" / "Hakkında" uçtan uca yükleme+kayıt+kapatmayı gösterir,
/// * eklenti **ayar** sayfası,
/// * tüm alt-modüllerin (0-CE.4) kayıtları (şimdilik çoğu boş; uzantı noktaları açık).
///
/// **Saf**tır: aynı yetki kapısı → aynı kayıtlar (yan etkisiz) → host güvenle kaldırıp
/// yeniden yükleyebilir (izolasyon).
pub fn aktiflestir(yetkiler: &YetkiKapisi) -> Aktivasyon {
    let mut akt = Aktivasyon::yeni();

    // Activity Bar ikonu + Side Panel sekmesi (tek tutarlı eklenti yüzeyi → tek panel kaydı).
    akt.panel(PANEL_KIMLIK, AD);

    // Komutlar (komut paleti) — uçtan uca "Merhaba BioCraft Studio" gösterimi.
    akt.komut(
        "biocraft.core.studio.hosgeldin",
        "BioCraft Studio: Hoş Geldin",
    )
    .komut("biocraft.core.studio.hakkinda", "BioCraft Studio: Hakkında");

    // Eklenti ayar sayfası.
    akt.ayar("biocraft.core.studio.ayarlar", AD);

    // Alt-modüllerin kayıtlarını topla (her biri yalnızca verilen yetkilere göre kayıt açar).
    akt.birlestir(data_io::kayitlar(yetkiler))
        .birlestir(genome_browser::kayitlar(yetkiler))
        .birlestir(alignment::kayitlar(yetkiler))
        .birlestir(variant::kayitlar(yetkiler))
        .birlestir(annotation::kayitlar(yetkiler))
        .birlestir(sequence::kayitlar(yetkiler))
        .birlestir(structure3d::kayitlar(yetkiler))
        .birlestir(compute::kayitlar(yetkiler))
        .birlestir(db_search::kayitlar(yetkiler))
        .birlestir(nodes::kayitlar(yetkiler))
        .birlestir(export::kayitlar(yetkiler))
        .birlestir(perf::kayitlar(yetkiler));

    akt
}

/// "Merhaba BioCraft Studio" — uçtan uca aktivasyonun çalıştığını gösteren kısa selam.
/// (App, çekirdek eklenti komutu çalıştırıldığında bunu konsola yazar.)
pub fn merhaba() -> &'static str {
    "Merhaba BioCraft Studio — çekirdek eklenti aktif (genom tarayıcı + varyant + 3B + veritabanı)."
}

// ─── Capability denetimi (MK-13) — eklenti yalnızca ilan ettiği yetkiyi kullanır ──

/// Yetki-kapılı bir **veritabanı** işlemi denemesi (ÇE-09 `db` yeteneği ister).
/// `db` verilmemişse host standart reddi döner.
pub fn db_erisimi_dene(yetkiler: &YetkiKapisi) -> Result<(), ErrorReport> {
    yetkiler.iste(Capability::Db)
}

/// Yetki-kapılı bir **uzak ağ** işlemi denemesi (`net`).  Manifest `net` İLAN EDER (Gün 35,
/// ÇE-01 uzak erişim); bu yüzden net **onaylanmış** kapıda kabul, **onaylanmamış** kapıda (kullanıcı
/// kurulumda net'i reddetmişse) standart hatayla reddedilir (MK-13).  PHI/hassas sınırı yine
/// çekirdekte korunur (İP-10/MK-42/43); net bu sınırı aşmaz.
pub fn uzak_erisim_dene(yetkiler: &YetkiKapisi) -> Result<(), ErrorReport> {
    yetkiler.iste(Capability::Net)
}

#[cfg(test)]
mod tests {
    use super::*;
    use biocraft_sdk::ui::UiUzantiTuru;

    #[test]
    fn manifest_kimligi_ve_sabitler_tutarli() {
        // Gömülü manifest, koddaki sabitlerle birebir uyumlu olmalı (tek doğruluk kaynağı).
        assert!(MANIFEST.contains(r#"kimlik = "biocraft.core.studio""#));
        assert!(MANIFEST.contains(r#"surum  = "0.1.0""#));
        assert!(MANIFEST.contains(r#"abi          = "0.1""#));
        assert_eq!(KIMLIK, "biocraft.core.studio");
        assert_eq!(SURUM, "0.1.0");
    }

    #[test]
    fn aktivasyon_panel_komut_ve_ayar_kaydeder() {
        let akt = aktiflestir(&yetki_kapisi_varsayilan());
        // Activity Bar + Side Panel paneli (1 adet, doğru kimlikle).
        assert_eq!(akt.ui_say(UiUzantiTuru::Panel), 1);
        assert!(akt
            .ui_turden(UiUzantiTuru::Panel)
            .any(|k| k.kimlik == PANEL_KIMLIK && k.baslik == AD));
        // En az iki komut (palette) + ayar sayfası/sayfaları.  Eklenti genel ayarı +
        // ÇE-12 erişilebilirlik/performans ayarı (perf modülü, Gün 43) → en az 2 ayar.
        assert!(akt.ui_say(UiUzantiTuru::Komut) >= 2);
        assert!(akt.ui_say(UiUzantiTuru::Ayar) >= 2);
        assert!(akt
            .ui_turden(UiUzantiTuru::Ayar)
            .any(|k| k.kimlik == "biocraft.core.studio.ayarlar"));
    }

    #[test]
    fn aktivasyon_saf_tekrar_edilebilir() {
        // İzolasyon temeli: aynı kapı → aynı kayıtlar (kaldırıp yeniden yükleme güvenli).
        let kapi = yetki_kapisi_varsayilan();
        assert_eq!(aktiflestir(&kapi), aktiflestir(&kapi));
    }

    #[test]
    fn net_onaylanmissa_kabul_onaylanmamissa_reddedilir() {
        // Varsayılan (tümü ilan+onaylı) kapıda net artık KABUL (Gün 35).
        let kapi = yetki_kapisi_varsayilan();
        assert!(uzak_erisim_dene(&kapi).is_ok());
        assert!(db_erisimi_dene(&kapi).is_ok());

        // Kullanıcı net'i onaylamamışsa (yalnız fs onaylı) → çalışmada reddedilir (MK-13).
        let kisitli = YetkiKapisi::yeni([Capability::Fs]);
        assert!(uzak_erisim_dene(&kisitli).is_err());
        assert!(db_erisimi_dene(&kisitli).is_err());
    }

    #[test]
    fn istenen_yetkiler_net_icerir() {
        assert!(ISTENEN_YETKILER.contains(&Capability::Net));
        assert!(ISTENEN_YETKILER.contains(&Capability::Fs));
        assert!(ISTENEN_YETKILER.contains(&Capability::Db));
        assert!(ISTENEN_YETKILER.contains(&Capability::Gpu));
        assert!(ISTENEN_YETKILER.contains(&Capability::Ai));
    }
}
