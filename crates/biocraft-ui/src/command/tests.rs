//! İP-13 modül-üstü entegrasyon testleri — palet + kısayol + tuş seti birlikte (MK-51, MK-52).

use super::*;
use crate::i18n::Dil;
use crate::shell::menu_bar::KabukAksiyon;

/// Bir oturumun kuracağı tipik komut kümesini üretir (kabuk + bir eklenti komutu).
fn kayit(harita: &KisayolHaritasi) -> Vec<Komut> {
    let mut v: Vec<Komut> = KabukAksiyon::tumu()
        .iter()
        .map(|&a| {
            let ks = harita.kisayol(&KomutKaynak::Kabuk(a)).map(|k| k.goster());
            Komut::kabuktan(a, Dil::Tr, ks, a.etkin_mi())
        })
        .collect();
    let ek = EklentiKomut::yeni("biocraft.ornek.selam", "Örnek: Selam Ver");
    let ks = harita
        .kisayol(&KomutKaynak::Eklenti(ek.kimlik.clone()))
        .map(|k| k.goster());
    v.push(Komut::eklentiden(&ek, ks));
    v
}

#[test]
fn menu_ve_palet_ayni_komut_tanimina_baglanir() {
    // MK-51: paletteki "Kaydet" komutu, menünün ürettiği AYNI KabukAksiyon'a çözülmeli.
    let h = KisayolHaritasi::varsayilan(TusSetiProfili::Modern);
    let komutlar = kayit(&h);
    let kaydet = komutlar
        .iter()
        .find(|k| k.kaynak == KomutKaynak::Kabuk(KabukAksiyon::Kaydet))
        .expect("Kaydet komutu listede olmalı");
    assert_eq!(kaydet.ad, KabukAksiyon::Kaydet.etiket(Dil::Tr));
    // Kısayol ipucu da tek kaynaktan (keymap) gelir.
    assert_eq!(kaydet.kisayol.as_deref(), Some("Ctrl+S"));
}

#[test]
fn komut_paleti_kisayolu_klavyeden_cozulur() {
    // Ctrl+Shift+P → KomutPaleti aksiyonuna çözülür (paleti açan tek kaynak).
    let h = KisayolHaritasi::varsayilan(TusSetiProfili::Modern);
    let ks = Kisayol::ayristir("Ctrl+Shift+P").unwrap();
    assert_eq!(
        h.cozumle(&ks),
        Some(KomutKaynak::Kabuk(KabukAksiyon::KomutPaleti))
    );
}

#[test]
fn eklenti_komutu_palette_gorunur_ve_kisayol_atanabilir() {
    // Kabul kriteri: eklenti komutu palette görünür + kısayol atanabilir.
    let mut h = KisayolHaritasi::varsayilan(TusSetiProfili::Modern);
    let ek = KomutKaynak::Eklenti("biocraft.ornek.selam".into());
    h.ata(ek.clone(), Kisayol::ayristir("Ctrl+Alt+G").unwrap());

    let komutlar = kayit(&h);
    let ek_komut = komutlar
        .iter()
        .find(|k| k.kaynak == ek)
        .expect("eklenti komutu palette olmalı");
    assert_eq!(ek_komut.kisayol.as_deref(), Some("Ctrl+Alt+G"));
    assert!(ek_komut.etkin);

    // Palette aranabilir mi?
    let mut p = KomutPaleti::yeni();
    p.ac(komutlar);
    // (özel erişim test modülünde değil; davranış: ad araması "selam" eklentiyi bulur)
    // Bunu palette modülünün kendi testleri doğruluyor; burada kayıt bütünlüğü yeterli.
    assert!(p.acik);
}

#[test]
fn kabuk_kaynak_anahtar_gidis_donus() {
    for &a in KabukAksiyon::tumu() {
        let k = KomutKaynak::Kabuk(a);
        let geri = KomutKaynak::anahtardan(&k.anahtar());
        assert_eq!(geri, Some(k), "anahtar gidiş-dönüş bozuldu: {a:?}");
    }
    // Eklenti anahtarı da.
    let ek = KomutKaynak::Eklenti("biocraft.x.y".into());
    assert_eq!(KomutKaynak::anahtardan(&ek.anahtar()), Some(ek));
}

#[test]
fn her_etkin_kabuk_aksiyonu_kategorize() {
    // Her aksiyon bir kategoriye düşmeli (palet sağ etiketi boş kalmaz).
    for &a in KabukAksiyon::tumu() {
        let k = Komut::kabuktan(a, Dil::En, None, a.etkin_mi());
        assert!(!k.kategori.etiket(Dil::En).is_empty());
        assert!(!k.ad.is_empty());
    }
}
