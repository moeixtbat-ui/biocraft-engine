//! Yerel geçmiş — zaman damgalı anlık görüntüler (İP-11 "yerel geçmiş", temel düzey).
//!
//! Geri-al/yinele yığını ([`crate::undo`]) oturum içi adım adım gezinme sağlar; **yerel geçmiş**
//! ise belirli anlardaki tam içeriğin (örn. her kayıtta) zaman damgalı bir listesini tutar ve
//! herhangi bir noktaya dönmeyi mümkün kılar.  Kullanıcı "şu sabahki hâle dön" diyebilsin diye.
//!
//! **Temel düzey (MVP):** Anlık görüntüler bellekte, derinlik sınırlı tutulur; her görüntü içeriğin
//! tam baytını + BLAKE3 özetini + zaman damgasını taşır.  Tam sürüm kontrolü (git) entegrasyonu
//! ve diske kalıcı geçmiş **sonraki** aşamadır (spec: "tam git entegrasyonu sonra").
//!
//! Saf ve test edilebilir: zaman damgası dışarıdan verilir (sahte saatle test) — tıpkı
//! [`crate::autosave`] gibi.

use std::collections::VecDeque;

use biocraft_types::{Blake3Hash, Timestamp};

use crate::conflict::damgala;

/// Varsayılan yerel geçmiş derinliği (kaç anlık görüntü saklanır).
pub const VARSAYILAN_GECMIS_DERINLIGI: usize = 50;

/// Belirli bir andaki tam içerik anlık görüntüsü.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnlikGoruntu {
    /// Görüntünün alındığı an (kullanıcıya listede gösterilir).
    pub zaman: Timestamp,
    /// Kısa etiket ("Otomatik kayıt", "Kapanış", "Elle"): görüntünün neden alındığı.
    pub etiket: String,
    /// İçeriğin BLAKE3 özeti (aynı içeriğin tekrar tekrar saklanmasını elemek için).
    pub hash: Blake3Hash,
    /// Görüntü anındaki tam içerik (geri yüklemede aynen döndürülür).
    pub icerik: Vec<u8>,
}

/// Zaman damgalı anlık görüntülerin derinlik sınırlı listesi (en yeni en sonda).
#[derive(Debug)]
pub struct YerelGecmis {
    goruntular: VecDeque<AnlikGoruntu>,
    derinlik: usize,
}

impl YerelGecmis {
    /// Varsayılan derinlikle boş bir geçmiş kurar.
    pub fn yeni() -> Self {
        Self::derinlikle(VARSAYILAN_GECMIS_DERINLIGI)
    }

    /// Belirtilen derinlikle boş bir geçmiş kurar (en az 1).
    pub fn derinlikle(derinlik: usize) -> Self {
        Self {
            goruntular: VecDeque::new(),
            derinlik: derinlik.max(1),
        }
    }

    /// Bir anlık görüntü ekler.  İçerik **son görüntüyle aynıysa** yeni görüntü eklenmez
    /// (gereksiz tekrar yok) ve `false` döner; eklendiyse `true`.
    ///
    /// Derinlik aşılırsa en eski görüntü atılır (temel düzey; kalıcı/git geçmişi sonra).
    pub fn anlik_al(&mut self, etiket: impl Into<String>, icerik: &[u8], zaman: Timestamp) -> bool {
        let hash = damgala(icerik);
        if let Some(son) = self.goruntular.back() {
            if son.hash == hash {
                return false; // değişmemiş; tekrar saklama.
            }
        }
        self.goruntular.push_back(AnlikGoruntu {
            zaman,
            etiket: etiket.into(),
            hash,
            icerik: icerik.to_vec(),
        });
        while self.goruntular.len() > self.derinlik {
            self.goruntular.pop_front();
        }
        true
    }

    /// Saklı anlık görüntüler (eskiden yeniye) — UI'da geçmiş listesi için.
    pub fn listele(&self) -> impl Iterator<Item = &AnlikGoruntu> {
        self.goruntular.iter()
    }

    /// Saklı görüntü sayısı.
    pub fn uzunluk(&self) -> usize {
        self.goruntular.len()
    }

    /// Geçmiş boş mu?
    pub fn bos_mu(&self) -> bool {
        self.goruntular.is_empty()
    }

    /// `index`. görüntüyü (eskiden yeniye, 0 tabanlı) döndürür — geri yüklemek için içeriği taşır.
    pub fn goruntu(&self, index: usize) -> Option<&AnlikGoruntu> {
        self.goruntular.get(index)
    }

    /// En son anlık görüntü (varsa).
    pub fn son(&self) -> Option<&AnlikGoruntu> {
        self.goruntular.back()
    }

    /// `index`. görüntünün **içeriğini** geri yükleme için klonlayarak döndürür (yoksa `None`).
    ///
    /// Geri yükleme tek başına bir düzenlemedir; çağıran bunu bir [`crate::command::Komut`] olarak
    /// uygulayıp geri-al yığınına koyabilir (böylece "geçmişe dönme" de geri alınabilir olur).
    pub fn geri_yukle(&self, index: usize) -> Option<Vec<u8>> {
        self.goruntular.get(index).map(|g| g.icerik.clone())
    }
}

impl Default for YerelGecmis {
    fn default() -> Self {
        Self::yeni()
    }
}

// ─── Testler ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use biocraft_types::Timestamp;

    fn t(saniye: i64) -> Timestamp {
        chrono::DateTime::from_timestamp(saniye, 0).expect("geçerli zaman")
    }

    #[test]
    fn anlik_al_ve_listele() {
        // Kabul kriteri: geçmiş (snapshot) listesi zaman damgalı görünmeli.
        let mut g = YerelGecmis::yeni();
        assert!(g.bos_mu());
        assert!(g.anlik_al("Elle", b"v1", t(100)));
        assert!(g.anlik_al("Otomatik kayıt", b"v2", t(160)));
        assert_eq!(g.uzunluk(), 2);

        let liste: Vec<&AnlikGoruntu> = g.listele().collect();
        assert_eq!(liste[0].etiket, "Elle");
        assert_eq!(liste[0].zaman, t(100));
        assert_eq!(liste[1].etiket, "Otomatik kayıt");
        assert_eq!(liste[1].zaman, t(160), "eskiden yeniye sıralı");
    }

    #[test]
    fn ayni_icerik_tekrar_saklanmaz() {
        let mut g = YerelGecmis::yeni();
        assert!(g.anlik_al("a", b"ayni", t(1)));
        assert!(
            !g.anlik_al("b", b"ayni", t(2)),
            "değişmemiş içerik tekrar saklanmaz"
        );
        assert_eq!(g.uzunluk(), 1);
        // İçerik değişince tekrar saklanır.
        assert!(g.anlik_al("c", b"farkli", t(3)));
        assert_eq!(g.uzunluk(), 2);
    }

    #[test]
    fn derinlik_siniri_en_eskiyi_atar() {
        let mut g = YerelGecmis::derinlikle(2);
        g.anlik_al("1", b"a", t(1));
        g.anlik_al("2", b"b", t(2));
        g.anlik_al("3", b"c", t(3));
        assert_eq!(g.uzunluk(), 2, "derinlik 2'yi aşamaz");
        let liste: Vec<&AnlikGoruntu> = g.listele().collect();
        assert_eq!(liste[0].etiket, "2", "en eski (1) düştü");
        assert_eq!(liste[1].etiket, "3");
    }

    #[test]
    fn geri_yukle_icerigi_dondurur() {
        let mut g = YerelGecmis::yeni();
        g.anlik_al("ilk", b"sabahki hal", t(100));
        g.anlik_al("son", b"aksamki hal", t(200));
        assert_eq!(g.geri_yukle(0).as_deref(), Some(&b"sabahki hal"[..]));
        assert_eq!(g.geri_yukle(1).as_deref(), Some(&b"aksamki hal"[..]));
        assert!(g.geri_yukle(99).is_none(), "olmayan dizin None");
        assert_eq!(g.son().unwrap().etiket, "son");
    }
}
