//! Örnek komutlar: [`UygulamaDurumu`] üzerinde geri-alınabilir düzenlemeler (İP-11).
//!
//! Bunlar genel [`crate::command::Komut`] arayüzünün **gerçek bir model** üzerinde çalıştığını
//! gösterir: hepsi tek mantıksal depoya ([`DEPO_UYGULAMA_DURUMU`]) dokunur (MK-37) ve eksiksiz
//! ters-işlem taşır (doğru "yinele").  Sonraki paketler (node/kod/ayar) aynı kalıbı kendi
//! modelleriyle tekrarlar.
//!
//! İki ters-işlem kalıbı örneklenir:
//! - **Simetrik** (eski+yeni saklı): [`TemaDegistir`], [`DilDegistir`], [`PanelGenisligiDegistir`].
//! - **Yakalayan** (eski durum ilk uygulamada bir kez yakalanır): [`SekmeEkle`], [`SekmeKapat`].

use biocraft_types::ErrorReport;

use crate::command::{DepoKimligi, Komut};
use crate::state::{AcikSekme, DilSecimi, TemaSecimi, UygulamaDurumu};

/// Uygulama durumunun tek mantıksal depo kimliği (MK-37) — [`crate::ANAHTAR_DURUM`] ile eşleşir.
pub const DEPO_UYGULAMA_DURUMU: &str = "uygulama_durumu";

fn depo() -> DepoKimligi {
    DepoKimligi::yeni(DEPO_UYGULAMA_DURUMU)
}

/// Temayı değiştirir (simetrik: eski+yeni saklı → yinele her zaman doğru).
#[derive(Debug, Clone)]
pub struct TemaDegistir {
    /// Önceki tema (geri-al bunu geri yükler).
    pub eski: TemaSecimi,
    /// Yeni tema (uygula/yinele bunu uygular).
    pub yeni: TemaSecimi,
}

impl TemaDegistir {
    /// Hedefin **şu anki** temasını eski olarak alıp `yeni`'ye geçen bir komut kurar.
    pub fn yeni(hedef: &UygulamaDurumu, yeni: TemaSecimi) -> Self {
        Self {
            eski: hedef.tema,
            yeni,
        }
    }
}

impl Komut<UygulamaDurumu> for TemaDegistir {
    fn uygula(&mut self, hedef: &mut UygulamaDurumu) -> Result<(), ErrorReport> {
        hedef.tema = self.yeni;
        Ok(())
    }
    fn geri_al(&mut self, hedef: &mut UygulamaDurumu) -> Result<(), ErrorReport> {
        hedef.tema = self.eski;
        Ok(())
    }
    fn aciklama(&self) -> String {
        "Tema değiştir".to_string()
    }
    fn depo(&self) -> DepoKimligi {
        depo()
    }
}

/// Dili değiştirir (simetrik).
#[derive(Debug, Clone)]
pub struct DilDegistir {
    /// Önceki dil.
    pub eski: DilSecimi,
    /// Yeni dil.
    pub yeni: DilSecimi,
}

impl DilDegistir {
    /// Hedefin şu anki dilini eski alıp `yeni`'ye geçen komut.
    pub fn yeni(hedef: &UygulamaDurumu, yeni: DilSecimi) -> Self {
        Self {
            eski: hedef.dil,
            yeni,
        }
    }
}

impl Komut<UygulamaDurumu> for DilDegistir {
    fn uygula(&mut self, hedef: &mut UygulamaDurumu) -> Result<(), ErrorReport> {
        hedef.dil = self.yeni;
        Ok(())
    }
    fn geri_al(&mut self, hedef: &mut UygulamaDurumu) -> Result<(), ErrorReport> {
        hedef.dil = self.eski;
        Ok(())
    }
    fn aciklama(&self) -> String {
        "Dil değiştir".to_string()
    }
    fn depo(&self) -> DepoKimligi {
        depo()
    }
}

/// Sağ panel genişliğini değiştirir (simetrik).
#[derive(Debug, Clone)]
pub struct PanelGenisligiDegistir {
    /// Önceki genişlik (mantıksal piksel).
    pub eski: f32,
    /// Yeni genişlik (mantıksal piksel).
    pub yeni: f32,
}

impl PanelGenisligiDegistir {
    /// Hedefin şu anki panel genişliğini eski alıp `yeni`'ye geçen komut.
    pub fn yeni(hedef: &UygulamaDurumu, yeni: f32) -> Self {
        Self {
            eski: hedef.panel.sag_panel_genislik,
            yeni,
        }
    }
}

impl Komut<UygulamaDurumu> for PanelGenisligiDegistir {
    fn uygula(&mut self, hedef: &mut UygulamaDurumu) -> Result<(), ErrorReport> {
        hedef.panel.sag_panel_genislik = self.yeni;
        Ok(())
    }
    fn geri_al(&mut self, hedef: &mut UygulamaDurumu) -> Result<(), ErrorReport> {
        hedef.panel.sag_panel_genislik = self.eski;
        Ok(())
    }
    fn aciklama(&self) -> String {
        "Panel genişliği".to_string()
    }
    fn depo(&self) -> DepoKimligi {
        depo()
    }
}

/// Yeni bir sekme ekler (yakalayan: eklendiği dizini yakalar → geri-al o dizini siler).
#[derive(Debug, Clone)]
pub struct SekmeEkle {
    sekme: AcikSekme,
    eklendi_index: Option<usize>,
}

impl SekmeEkle {
    /// Verilen sekmeyi listenin sonuna ekleyecek bir komut kurar.
    pub fn yeni(sekme: AcikSekme) -> Self {
        Self {
            sekme,
            eklendi_index: None,
        }
    }
}

impl Komut<UygulamaDurumu> for SekmeEkle {
    fn uygula(&mut self, hedef: &mut UygulamaDurumu) -> Result<(), ErrorReport> {
        hedef.sekmeler.push(self.sekme.clone());
        self.eklendi_index = Some(hedef.sekmeler.len() - 1);
        Ok(())
    }
    fn geri_al(&mut self, hedef: &mut UygulamaDurumu) -> Result<(), ErrorReport> {
        if let Some(i) = self.eklendi_index {
            if i < hedef.sekmeler.len() {
                hedef.sekmeler.remove(i);
            }
        }
        Ok(())
    }
    fn aciklama(&self) -> String {
        format!("Sekme ekle: {}", self.sekme.baslik)
    }
    fn depo(&self) -> DepoKimligi {
        depo()
    }
}

/// Bir sekmeyi kapatır (yakalayan: kapatılan sekme ilk uygulamada bir kez yakalanır → geri-al
/// onu aynı dizine geri koyar).
#[derive(Debug, Clone)]
pub struct SekmeKapat {
    index: usize,
    kapatilan: Option<AcikSekme>,
}

impl SekmeKapat {
    /// `index`. sekmeyi kapatacak bir komut kurar.
    pub fn yeni(index: usize) -> Self {
        Self {
            index,
            kapatilan: None,
        }
    }
}

impl Komut<UygulamaDurumu> for SekmeKapat {
    fn uygula(&mut self, hedef: &mut UygulamaDurumu) -> Result<(), ErrorReport> {
        if self.index >= hedef.sekmeler.len() {
            return Err(ErrorReport::new(
                "Sekme kapatılamadı",
                "Kapatılmak istenen sekme artık listede yok (geçersiz dizin).",
                "Listeyi yenileyip tekrar deneyin.",
            ));
        }
        let s = hedef.sekmeler.remove(self.index);
        // İlk uygulamada yakala; yinelede aynı değer korunur (doğru yinele).
        if self.kapatilan.is_none() {
            self.kapatilan = Some(s);
        }
        Ok(())
    }
    fn geri_al(&mut self, hedef: &mut UygulamaDurumu) -> Result<(), ErrorReport> {
        if let Some(s) = self.kapatilan.clone() {
            let i = self.index.min(hedef.sekmeler.len());
            hedef.sekmeler.insert(i, s);
        }
        Ok(())
    }
    fn aciklama(&self) -> String {
        match &self.kapatilan {
            Some(s) => format!("Sekme kapat: {}", s.baslik),
            None => "Sekme kapat".to_string(),
        }
    }
    fn depo(&self) -> DepoKimligi {
        depo()
    }
}

// ─── Testler ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::undo::GeriAlYigini;

    fn ornek_sekme(baslik: &str) -> AcikSekme {
        AcikSekme {
            yol: None,
            baslik: baslik.to_string(),
            kaydedilmemis: false,
        }
    }

    #[test]
    fn tum_komutlar_tek_depoya_dokunur() {
        // MK-37: örnek komutların hepsi AYNI mantıksal depoya dokunmalı.
        let beklenen = DepoKimligi::yeni(DEPO_UYGULAMA_DURUMU);
        let d = UygulamaDurumu::default();
        assert_eq!(TemaDegistir::yeni(&d, TemaSecimi::Acik).depo(), beklenen);
        assert_eq!(DilDegistir::yeni(&d, DilSecimi::En).depo(), beklenen);
        assert_eq!(PanelGenisligiDegistir::yeni(&d, 400.0).depo(), beklenen);
        assert_eq!(SekmeEkle::yeni(ornek_sekme("x")).depo(), beklenen);
        assert_eq!(SekmeKapat::yeni(0).depo(), beklenen);
    }

    #[test]
    fn tema_komutu_simetrik_geri_al_yinele() {
        let mut d = UygulamaDurumu::default(); // varsayılan Koyu.
        let mut k = TemaDegistir::yeni(&d, TemaSecimi::Acik);
        k.uygula(&mut d).unwrap();
        assert_eq!(d.tema, TemaSecimi::Acik);
        k.geri_al(&mut d).unwrap();
        assert_eq!(d.tema, TemaSecimi::Koyu, "geri-al eski temayı geri yükler");
        k.uygula(&mut d).unwrap();
        assert_eq!(d.tema, TemaSecimi::Acik, "yinele yeni temayı uygular");
    }

    #[test]
    fn sekme_ekle_kapat_yakalayan_geri_al() {
        let mut d = UygulamaDurumu::default();
        // Ekle.
        let mut ekle = SekmeEkle::yeni(ornek_sekme("genom.fasta"));
        ekle.uygula(&mut d).unwrap();
        assert_eq!(d.sekmeler.len(), 1);
        // Kapat (yakalayan: kapatılan sekmeyi saklar).
        let mut kapat = SekmeKapat::yeni(0);
        kapat.uygula(&mut d).unwrap();
        assert!(d.sekmeler.is_empty());
        // Geri-al → kapatılan sekme aynı yere döner.
        kapat.geri_al(&mut d).unwrap();
        assert_eq!(d.sekmeler.len(), 1);
        assert_eq!(d.sekmeler[0].baslik, "genom.fasta");
        // Yinele (kapatmayı tekrar uygula) → yine doğru.
        kapat.uygula(&mut d).unwrap();
        assert!(d.sekmeler.is_empty(), "yinele doğru çalışmalı");
    }

    #[test]
    fn gercek_durumda_cok_adimli_geri_al_yinele() {
        // Uçtan uca: örnek bir işlem dizisi yap, geri al, yinele — gerçek UygulamaDurumu üzerinde.
        let mut d = UygulamaDurumu::default();
        let mut y = GeriAlYigini::yeni();

        // Komutu önce kur (anlık durumu oku), sonra çalıştır → ödünç-alma çakışması olmaz.
        let k1 = Box::new(TemaDegistir::yeni(&d, TemaSecimi::Acik));
        y.calistir(&mut d, k1).unwrap();
        let k2 = Box::new(SekmeEkle::yeni(ornek_sekme("a.fasta")));
        y.calistir(&mut d, k2).unwrap();
        let k3 = Box::new(PanelGenisligiDegistir::yeni(&d, 500.0));
        y.calistir(&mut d, k3).unwrap();

        assert_eq!(d.tema, TemaSecimi::Acik);
        assert_eq!(d.sekmeler.len(), 1);
        assert_eq!(d.panel.sag_panel_genislik, 500.0);

        // Üç adımı geri al → başlangıç durumu.
        while y.geri_al(&mut d).unwrap() {}
        assert_eq!(d.tema, TemaSecimi::Koyu, "tema başa döndü");
        assert!(d.sekmeler.is_empty(), "sekme başa döndü");
        assert_eq!(
            d.panel.sag_panel_genislik,
            UygulamaDurumu::default().panel.sag_panel_genislik,
            "panel başa döndü"
        );

        // Üç adımı yinele → son durum geri gelir.
        while y.yinele(&mut d).unwrap() {}
        assert_eq!(d.tema, TemaSecimi::Acik);
        assert_eq!(d.sekmeler.len(), 1);
        assert_eq!(d.panel.sag_panel_genislik, 500.0);
    }
}
