//! İP-15 (Gün 27) demosu — **dağıtık ağ pasif kancaları uçtan uca** (saf, ağsız).
//!
//! Çalıştırma:
//! ```text
//! cargo run -p biocraft-net --example dagitik_kanca_demo
//! ```
//! Kabul kriterlerini uçtan uca gösterir:
//! 1. Eklenti yokken **sıfır maliyet** + [İndir] yönlendirmesi (hiç ağ etkinliği yok).
//! 2. **Veri sınırı:** PHI'den P2P yükü inşa edilemez (çekirdek çıkış kapısı engeller).
//! 3. **Varsayılan KAPALI:** eklenti kayıtlı olsa bile ağ açılmadıkça iş gitmez.
//! 4. Etkinleştirilince Normal/Sentetik iş eklentiye iletilir.
//! 5. **Iroh** yalnızca arayüz (gerçek bağlantı yok); **kaynak paylaşımı** opt-in; **Bio-kredi** bağlı değil.

use std::sync::Arc;

use biocraft_net::{
    AgDurumu, BioKrediKanca, DagitikAg, DagitikAgSaglayici, Is, IsDurumu, IsKimlik, IsSonucu,
    KaynakSiniri, P2pYuku, SaglayiciKimlik,
};
use biocraft_types::{DataClassification, ErrorReport};

fn baslik(s: &str) {
    println!("\n========== {s} ==========");
}

/// Gerçek ağ yapmayan sahte eklenti sağlayıcı (yalnızca demo).
struct DemoEklenti;

impl DagitikAgSaglayici for DemoEklenti {
    fn kimlik(&self) -> SaglayiciKimlik {
        SaglayiciKimlik {
            kimlik: "biocraft.demo.dagitik-ag".into(),
            ad: "Demo Dağıtık Ağ (gerçek ağ değil)".into(),
            surum: "0.0.0".into(),
        }
    }
    fn is_gonder(&self, is: Is) -> Result<IsKimlik, Box<ErrorReport>> {
        println!(
            "    [eklenti] iş alındı: '{}' ({} bayt)",
            is.tur,
            is.toplam_bayt()
        );
        Ok(IsKimlik::yeni("demo-is-1"))
    }
    fn is_durumu(&self, _is: &IsKimlik) -> Result<IsDurumu, Box<ErrorReport>> {
        Ok(IsDurumu::Tamamlandi)
    }
    fn sonuclari_topla(&self, _is: &IsKimlik) -> Result<Vec<IsSonucu>, Box<ErrorReport>> {
        Ok(Vec::new())
    }
    fn kaynak_siniri_ayarla(&self, sinir: KaynakSiniri) -> Result<(), Box<ErrorReport>> {
        println!("    [eklenti] kaynak sınırı uygulandı: {sinir:?}");
        Ok(())
    }
}

fn main() {
    baslik("1) Eklenti YOK → sıfır maliyet + [İndir]");
    let mut ag = DagitikAg::yeni();
    println!("  eklenti var mı? {}", ag.eklenti_var_mi());
    match ag.durum() {
        AgDurumu::EklentiYok { indir_url } => println!("  durum: EklentiYok → İndir: {indir_url}"),
        d => println!("  durum: {d:?}"),
    }
    match ag.is_gonder(Is::yeni("test", vec![])) {
        Err(h) => println!(
            "  iş gönderimi reddedildi (beklenen): {} [{}]",
            h.ne_oldu,
            h.eylem_etiketi.as_deref().unwrap_or("-")
        ),
        Ok(_) => println!("  HATA: eklenti yokken iş gitmemeliydi!"),
    }

    baslik("2) Veri sınırı — PHI'den P2P yükü inşa EDİLEMEZ");
    match P2pYuku::sonuc(DataClassification::HasasPhi, "hasta sonucu", vec![1, 2, 3]) {
        Err(h) => println!("  PHI yükü engellendi (beklenen): {}", h.ne_oldu),
        Ok(_) => println!("  HATA: PHI yükü oluşturulabildi — sınır delindi!"),
    }
    let normal = P2pYuku::metadata(
        DataClassification::Normal,
        "hizalama parametreleri",
        vec![0u8; 16],
    )
    .expect("normal yük oluşmalı");
    println!(
        "  Normal yük oluştu: {} ({} bayt, sınıf={:?})",
        normal.aciklama(),
        normal.bayt_sayisi(),
        normal.sinif()
    );

    baslik("3) Eklenti kayıtlı ama ağ VARSAYILAN KAPALI");
    ag.saglayici_kaydet(Arc::new(DemoEklenti));
    println!(
        "  eklenti var mı? {} | etkin mi? {}",
        ag.eklenti_var_mi(),
        ag.etkin_mi()
    );
    println!("  durum: {:?}", ag.durum());
    let is = Is::yeni("hizalama", vec![normal.clone()]);
    match ag.is_gonder(is) {
        Err(h) => println!(
            "  iş reddedildi (beklenen): {} [{}]",
            h.ne_oldu,
            h.eylem_etiketi.as_deref().unwrap_or("-")
        ),
        Ok(_) => println!("  HATA: kapalıyken iş gitmemeliydi!"),
    }

    baslik("4) Kullanıcı ağı AÇINCA → iş eklentiye gider");
    ag.etkinlestir();
    println!("  durum: {:?}", ag.durum());
    let is = Is::yeni("hizalama", vec![normal]);
    match ag.is_gonder(is) {
        Ok(k) => println!("  iş gönderildi → kimlik: {k:?}"),
        Err(h) => println!("  HATA: {}", h.ne_oldu),
    }

    baslik("5) Kaynak paylaşımı opt-in + Bio-kredi yer tutucu");
    println!("  varsayılan kaynak sınırı: {:?}", KaynakSiniri::default());
    println!(
        "  paylaşım var mı? {}",
        KaynakSiniri::default().paylasim_var_mi()
    );
    ag.kaynak_siniri_ayarla(KaynakSiniri {
        etkin: true,
        azami_cpu_yuzde: 25,
        yalnizca_bostayken: true,
        ..Default::default()
    })
    .unwrap();
    let bk = BioKrediKanca::default();
    println!(
        "  Bio-kredi bağlı mı? {} | 100 birim → {:?}",
        bk.bagli,
        bk.krediye_cevir(100.0)
    );

    println!("\n✅ Tüm kancalar pasif & güvenli: eklenti yokken sıfır maliyet, PHI ağa çıkamaz, varsayılan kapalı, Iroh yalnız arayüz.");
}
