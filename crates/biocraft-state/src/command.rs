//! Komut deseni (Command Pattern) + ters-işlem (inverse) — İP-11 Geri Al/Yinele çekirdeği.
//!
//! Düzenlenebilir **her** işlem bir [`Komut`] olarak ifade edilir.  Her komut iki yönü bilir:
//! - **uygula** (ileri): değişikliği hedefe işler.
//! - **geri_al** (ters-işlem): değişikliği tam tersine çevirir → hedefi öncesine döndürür.
//!
//! Bu sayede [`crate::undo::GeriAlYigini`] yalnızca komutları bir yığında tutarak çok-adımlı
//! geri-al/yinele sağlar; *ne* değiştiğini bilmesine gerek yoktur (komut kendi tersini taşır).
//!
//! ## Doğru "yinele" (redo) için altın kural
//! Bir komutun **eski + yeni** durumu eksiksiz saklanmalıdır (simetrik komut), ya da eski durum
//! ilk uygulamada **bir kez** yakalanıp saklanmalıdır (yakalayan komut).  Böylece "yinele"
//! (= `uygula`'yı tekrar çalıştırmak) her zaman aynı sonucu verir; ara durumdan etkilenmez.
//! (Sık hata: yinele yanlış sonuç verirse, ters-işlem eksik veya durum yakalama hatalıdır.)
//!
//! ## MK-37 — tek mantıksal depo
//! Her komut [`Komut::depo`] ile **tek** bir mantıksal depoya dokunduğunu ilan eder.  Birden çok
//! depoya dokunan nadir işlemler tek komutta **birleştirilmez**: [`BirlesikKomut`] yalnızca *aynı*
//! depoya dokunan komutları gruplar, farklı depoları reddeder.  Böylece her geri-alınabilir birim
//! tek depo sınırında atomik kalır (saga/iki-aşamalı taahhüt gerektirmez).
//!
//! ## Genel arayüz (sonraki paketler için)
//! `Komut<H>` hedef model `H` üzerinde geneldir.  Sonraki paketler kendi modelini koyup
//! (`H` = node grafı / kod tamponu / ayar ağacı) işlemlerini `Komut<H>` olarak yazar; aynı
//! [`crate::undo::GeriAlYigini`] motorunu hiç değiştirmeden kullanır.

use biocraft_types::ErrorReport;

/// Bir komutun dokunduğu **tek mantıksal depo**nun kimliği (MK-37).
///
/// Örn. `"uygulama_durumu"`, `"proje:genom.fasta"`, `"node-graf:ana"`, `"ayarlar"`.  Aynı kimlik
/// = aynı atomik tutarlılık sınırı.  [`BirlesikKomut`] yalnızca aynı kimlikli komutları gruplar.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DepoKimligi(pub String);

impl DepoKimligi {
    /// Verilen addan bir depo kimliği oluşturur.
    pub fn yeni(ad: impl Into<String>) -> Self {
        Self(ad.into())
    }
}

impl std::fmt::Display for DepoKimligi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Geri-alınabilir **tek** bir düzenleme işlemi (Command Pattern + ters-işlem).
///
/// `H`: işlemin üzerinde çalıştığı hedef model (tek mantıksal depo).  Aynı yığında farklı komut
/// türleri `Box<dyn Komut<H>>` olarak bir arada tutulabilir.
///
/// **Sözleşme:** `geri_al`, hemen önce gelen `uygula`'nın etkisini tam olarak geri almalıdır
/// (idempotent değil — her `uygula` bir `geri_al` ile eşleşir).  `uygula`+`geri_al`+`uygula`
/// dizisi (yinele) ilk `uygula` ile **aynı** sonucu vermelidir.
pub trait Komut<H> {
    /// Değişikliği hedefe **ileri** yönde işler.  Gerekirse ters-işlem için eski durumu yakalar.
    fn uygula(&mut self, hedef: &mut H) -> Result<(), ErrorReport>;

    /// Değişikliği **tersine** çevirir → hedefi `uygula` öncesine döndürür.
    fn geri_al(&mut self, hedef: &mut H) -> Result<(), ErrorReport>;

    /// Kullanıcıya görünen kısa açıklama (geçmiş listesi/menü için): "Tema değiştir", "Sekme kapat".
    fn aciklama(&self) -> String;

    /// MK-37: bu komutun dokunduğu **tek** mantıksal depo.
    fn depo(&self) -> DepoKimligi;
}

/// Aynı depoya dokunan birden çok komutu **tek atomik geri-alınabilir birim** olarak gruplar.
///
/// MK-37 koruması: yapıcı, komutların **hepsinin aynı depoya** dokunduğunu doğrular; farklı
/// depolar verilirse [`ErrorReport`] döner (çok-depo tek komutta birleştirilemez).
///
/// Atomiklik: `uygula` sırasında bir alt komut başarısız olursa, o ana dek uygulanmış alt komutlar
/// ters sırada geri alınır ve hata döndürülür → hedef yarım kalmaz (hep-ya-hiç, tek depo içinde).
pub struct BirlesikKomut<H> {
    aciklama: String,
    depo: DepoKimligi,
    komutlar: Vec<Box<dyn Komut<H>>>,
}

impl<H> BirlesikKomut<H> {
    /// Aynı depoya dokunan komutlardan birleşik bir komut kurar.
    ///
    /// `Err`: liste boşsa veya komutlar **farklı** depolara dokunuyorsa (MK-37 ihlali).
    pub fn yeni(
        aciklama: impl Into<String>,
        komutlar: Vec<Box<dyn Komut<H>>>,
    ) -> Result<Self, ErrorReport> {
        let ilk = komutlar.first().ok_or_else(|| {
            ErrorReport::new(
                "Birleşik komut oluşturulamadı",
                "Birleşik bir komut en az bir alt komut içermelidir (liste boş).",
                "Bu bir program hatasıdır; işlemi tekrar deneyin.",
            )
        })?;
        let depo = ilk.depo();
        // MK-37: tüm alt komutlar AYNI mantıksal depoya dokunmalı.
        if let Some(farkli) = komutlar.iter().find(|k| k.depo() != depo) {
            return Err(ErrorReport::new(
                "Birden çok depoya dokunan işlem tek komutta birleştirilemez",
                format!(
                    "Bir geri-alınabilir komut yalnızca tek mantıksal depoya dokunabilir (MK-37); \
                     bu grup '{}' ile '{}' depolarını karıştırıyor.",
                    depo,
                    farkli.depo()
                ),
                "Bu işlemi her depo için ayrı komutlara bölün; \
                 çok-depolu tek atomik işlem desteklenmez.",
            ));
        }
        Ok(Self {
            aciklama: aciklama.into(),
            depo,
            komutlar,
        })
    }
}

impl<H> Komut<H> for BirlesikKomut<H> {
    fn uygula(&mut self, hedef: &mut H) -> Result<(), ErrorReport> {
        for i in 0..self.komutlar.len() {
            if let Err(e) = self.komutlar[i].uygula(hedef) {
                // Atomik geri sarma: uygulanmış alt komutları ters sırada geri al (i-1 .. 0).
                for j in (0..i).rev() {
                    let _ = self.komutlar[j].geri_al(hedef);
                }
                return Err(e);
            }
        }
        Ok(())
    }

    fn geri_al(&mut self, hedef: &mut H) -> Result<(), ErrorReport> {
        // Ters-işlem ters sırada uygulanır (LIFO): en son uygulanan ilk geri alınır.
        for k in self.komutlar.iter_mut().rev() {
            k.geri_al(hedef)?;
        }
        Ok(())
    }

    fn aciklama(&self) -> String {
        self.aciklama.clone()
    }

    fn depo(&self) -> DepoKimligi {
        self.depo.clone()
    }
}

// ─── Testler ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Basit test modeli + komutları (Vec<i32> = tek mantıksal depo benzetimi).

    /// Listenin sonuna bir değer ekleyen komut (geri-al: son değeri çıkarır).
    struct Ekle {
        depo: String,
        deger: i32,
    }
    impl Komut<Vec<i32>> for Ekle {
        fn uygula(&mut self, hedef: &mut Vec<i32>) -> Result<(), ErrorReport> {
            hedef.push(self.deger);
            Ok(())
        }
        fn geri_al(&mut self, hedef: &mut Vec<i32>) -> Result<(), ErrorReport> {
            hedef.pop();
            Ok(())
        }
        fn aciklama(&self) -> String {
            format!("ekle {}", self.deger)
        }
        fn depo(&self) -> DepoKimligi {
            DepoKimligi::yeni(&self.depo)
        }
    }

    /// Daima `uygula`'da başarısız olan komut (atomik geri sarma testi için).
    struct Patla {
        depo: String,
    }
    impl Komut<Vec<i32>> for Patla {
        fn uygula(&mut self, _hedef: &mut Vec<i32>) -> Result<(), ErrorReport> {
            Err(ErrorReport::new("patladı", "kasıtlı hata", "yok"))
        }
        fn geri_al(&mut self, _hedef: &mut Vec<i32>) -> Result<(), ErrorReport> {
            Ok(())
        }
        fn aciklama(&self) -> String {
            "patla".to_string()
        }
        fn depo(&self) -> DepoKimligi {
            DepoKimligi::yeni(&self.depo)
        }
    }

    #[test]
    fn birlesik_bos_liste_hata() {
        let komutlar: Vec<Box<dyn Komut<Vec<i32>>>> = Vec::new();
        assert!(BirlesikKomut::yeni("boş", komutlar).is_err());
    }

    #[test]
    fn birlesik_ayni_depo_kabul() {
        // MK-37: aynı depoya dokunan komutlar tek birime gruplanabilir.
        let komutlar: Vec<Box<dyn Komut<Vec<i32>>>> = vec![
            Box::new(Ekle {
                depo: "A".into(),
                deger: 1,
            }),
            Box::new(Ekle {
                depo: "A".into(),
                deger: 2,
            }),
        ];
        let mut b = BirlesikKomut::yeni("iki ekle", komutlar).expect("aynı depo kabul edilmeli");
        let mut hedef = Vec::new();
        b.uygula(&mut hedef).unwrap();
        assert_eq!(hedef, vec![1, 2], "alt komutlar sırayla uygulanmalı");
        b.geri_al(&mut hedef).unwrap();
        assert!(hedef.is_empty(), "geri-al ters sırada hepsini geri almalı");
    }

    #[test]
    fn birlesik_farkli_depo_reddedilir() {
        // MK-37 koruması: farklı depolara dokunan komutlar tek komutta BİRLEŞTİRİLEMEZ.
        let komutlar: Vec<Box<dyn Komut<Vec<i32>>>> = vec![
            Box::new(Ekle {
                depo: "A".into(),
                deger: 1,
            }),
            Box::new(Ekle {
                depo: "B".into(),
                deger: 2,
            }),
        ];
        let sonuc = BirlesikKomut::yeni("karışık", komutlar);
        assert!(
            sonuc.is_err(),
            "çok-depolu birleştirme reddedilmeli (MK-37)"
        );
    }

    #[test]
    fn birlesik_atomik_geri_sarma() {
        // Alt komut ortada başarısız olursa, uygulanmışlar geri alınır → hedef yarım kalmaz.
        let komutlar: Vec<Box<dyn Komut<Vec<i32>>>> = vec![
            Box::new(Ekle {
                depo: "A".into(),
                deger: 1,
            }),
            Box::new(Patla { depo: "A".into() }),
        ];
        let mut b = BirlesikKomut::yeni("ekle+patla", komutlar).unwrap();
        let mut hedef = vec![9];
        let sonuc = b.uygula(&mut hedef);
        assert!(sonuc.is_err(), "alt komut hatası dışa yansımalı");
        assert_eq!(hedef, vec![9], "kısmen uygulanan değişiklik geri sarılmalı");
    }
}
