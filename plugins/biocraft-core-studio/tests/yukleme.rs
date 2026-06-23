//! ÇE-00 entegrasyon testi — çekirdek eklentinin İP-07 host'uyla uçtan uca yaşam döngüsü.
//!
//! Kapsananlar (bugünün test listesi):
//! * Çekirdek eklenti **keşfedilir/doğrulanır/yüklenir**; Activity Bar paneli + komut kaydı görünür.
//! * Eklenti **yalnızca `biocraft-sdk`'ya** bağlıdır (motora doğrudan değil — MK-17).
//! * **Onaylanmayan** capability isteği (kullanıcı net'i reddetmiş) çalışmada **reddedilir** (MK-13).
//! * Eklenti **kapatılıp yeniden yüklenir** (izolasyon): kayıtlar temizlenir, yeniden açılınca
//!   birebir aynı kayıtlar oluşur (aktivasyon saf).

use biocraft_core_studio as studio;
use biocraft_plugin_host::biocraft_sdk::biocraft_types::{Capability, Version};
use biocraft_plugin_host::biocraft_sdk::ui::UiUzantiTuru;
use biocraft_plugin_host::discover::manifest_oku;
use biocraft_plugin_host::{KayitDefteri, YetkiKumesi};

/// Eklentinin kök dizini (manifest + Cargo.toml buradadır).
fn kok() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Host'un yapacağı gibi: dizinden manifesti keşfeder, uyumluluğu denetler, yetki kapısı üretir.
fn kesfet_ve_yetki() -> (
    biocraft_plugin_host::Manifest,
    biocraft_plugin_host::biocraft_sdk::YetkiKapisi,
) {
    let kesfedilen = manifest_oku(&kok()).expect("çekirdek eklenti keşfedilmeli");
    let m = kesfedilen.manifest;

    // ABI + çekirdek sürüm uyumu (host kapısı) — çekirdek ABI 0.1, sürüm 0.1.0.
    let cekirdek_abi = biocraft_plugin_host::biocraft_sdk::ABI_SURUMU;
    let cekirdek_surum = Version::new(0, 1, 0);
    m.uyumluluk_denetle(&cekirdek_surum, &cekirdek_abi)
        .expect("çekirdek eklenti bu sürümle uyumlu olmalı");

    // Birinci-parti çekirdek eklenti: kullanıcı ilan edilen tüm yetkileri onaylar (varsayılan kurulu).
    // Verilen = istenen ∩ onaylanan → kapı.
    let kapi = YetkiKumesi::ver(&m.istenen_yetkiler, &m.istenen_yetkiler).kapi();
    (m, kapi)
}

#[test]
fn kesfedilir_dogrulanir_kimlik_ve_yetkiler_gorunur() {
    let (m, _kapi) = kesfet_ve_yetki();
    // Kimlik spec 0-CE.1 ile birebir; kod sabitiyle de tutarlı.
    assert_eq!(m.kimlik.metni(), "biocraft.core.studio");
    assert_eq!(m.kimlik.metni(), studio::KIMLIK);
    assert_eq!(m.ad, studio::AD);
    assert_eq!(m.surum, Version::new(0, 1, 0));
    // Capability'ler manifest'te ilan ve görünür (kurulumda kullanıcıya gösterilir).
    assert!(m.istenen_yetkiler.contains(&Capability::Fs));
    assert!(m.istenen_yetkiler.contains(&Capability::Db));
    assert!(m.istenen_yetkiler.contains(&Capability::Gpu));
    assert!(m.istenen_yetkiler.contains(&Capability::Ai));
    // net Gün 35'te (ÇE-01 uzak erişim) İLAN EDİLDİ.
    assert!(m.istenen_yetkiler.contains(&Capability::Net));
}

#[test]
fn yuklenir_activity_bar_paneli_ve_komut_kaydi_gorunur() {
    let (_m, kapi) = kesfet_ve_yetki();
    let akt = studio::aktiflestir(&kapi);

    // Host UI kayıt defterine bağla (çekirdeğin yaptığı gibi).
    let mut defter = KayitDefteri::yeni();
    for k in &akt.ui {
        defter
            .kaydet(studio::KIMLIK, k.clone(), 100) // çekirdek eklenti yüksek öncelik
            .expect("kayıt çakışmamalı");
    }
    for n in &akt.nodelar {
        defter.node_kaydet(studio::KIMLIK, n.clone());
    }

    // Activity Bar + Side Panel paneli görünür.
    let paneller = defter.alan(UiUzantiTuru::Panel);
    assert_eq!(paneller.len(), 1);
    assert_eq!(paneller[0].kayit.kimlik, studio::PANEL_KIMLIK);
    assert_eq!(paneller[0].sahip, studio::KIMLIK);
    // Komut kaydı görünür (en az "Hoş Geldin"/"Hakkında" + db arama).
    assert!(defter.alan(UiUzantiTuru::Komut).len() >= 3);
    // Çakışma yok (tek eklenti).
    assert!(defter.cakismalar().is_empty());
}

#[test]
fn kapatilip_yeniden_yuklenir_izolasyon() {
    let (_m, kapi) = kesfet_ve_yetki();

    let yukle = |defter: &mut KayitDefteri| {
        for k in &studio::aktiflestir(&kapi).ui {
            defter.kaydet(studio::KIMLIK, k.clone(), 100).unwrap();
        }
    };

    let mut defter = KayitDefteri::yeni();
    yukle(&mut defter);
    let ilk_panel = defter.alan(UiUzantiTuru::Panel).len();
    let ilk_komut = defter.alan(UiUzantiTuru::Komut).len();
    assert!(ilk_panel >= 1 && ilk_komut >= 1);

    // Kapat (kaldır): eklentinin TÜM kayıtları temizlenir.
    defter.eklenti_kaldir(studio::KIMLIK);
    assert_eq!(defter.alan(UiUzantiTuru::Panel).len(), 0);
    assert_eq!(defter.alan(UiUzantiTuru::Komut).len(), 0);

    // Yeniden yükle: birebir aynı kayıtlar (aktivasyon saf → izolasyon güvenli).
    yukle(&mut defter);
    assert_eq!(defter.alan(UiUzantiTuru::Panel).len(), ilk_panel);
    assert_eq!(defter.alan(UiUzantiTuru::Komut).len(), ilk_komut);
}

#[test]
fn onaylanmayan_net_calismada_reddedilir() {
    let (m, kapi) = kesfet_ve_yetki();
    // Tümü onaylı kapıda (varsayılan) net artık KABUL (Gün 35'te ilan edildi).
    assert!(studio::db_erisimi_dene(&kapi).is_ok());
    assert!(studio::uzak_erisim_dene(&kapi).is_ok());

    // Kullanıcı net'i ONAYLAMAZSA (istenen ∩ onaylanan): ilan edilse de çalışmada reddedilir (MK-13).
    let onaylanan: Vec<Capability> = m
        .istenen_yetkiler
        .iter()
        .copied()
        .filter(|c| *c != Capability::Net)
        .collect();
    let kisitli_kapi = YetkiKumesi::ver(&m.istenen_yetkiler, &onaylanan).kapi();
    let hata = studio::uzak_erisim_dene(&kisitli_kapi).unwrap_err();
    assert_eq!(hata.ne_oldu, "Eklenti erişimi reddedildi");
    assert!(hata.neden.contains("net"));
}

#[test]
fn yalnizca_biocraft_sdk_baginti() {
    // MK-17: eklentinin SEVKİYAT (regular) bağımlılıkları arasında biocraft-sdk dışında
    // hiçbir motor crate'i (biocraft-app/ui/host/data/…) olmamalı.  Kendi Cargo.toml'unu ayrıştır.
    let cargo = std::fs::read_to_string(kok().join("Cargo.toml")).unwrap();
    let v: toml::Value = toml::from_str(&cargo).unwrap();
    let deps = v
        .get("dependencies")
        .and_then(|d| d.as_table())
        .expect("[dependencies] olmalı");

    let biocraft_depler: Vec<&String> =
        deps.keys().filter(|k| k.starts_with("biocraft-")).collect();
    assert_eq!(
        biocraft_depler,
        vec![&"biocraft-sdk".to_string()],
        "çekirdek eklenti yalnızca biocraft-sdk'ya bağlı olmalı (MK-17); bulunan: {biocraft_depler:?}"
    );
}
