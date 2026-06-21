//! Basit, bağımlılıksız i18n (uluslararasılaştırma) katmanı — EN/TR.
//!
//! TDA madde 14 (tutarlılık) gereği **tüm** bileşen metinleri buradan gelir;
//! hiçbir bileşen ekrana sabit (hard-coded) metin yazmaz.  Yeni bir dil eklemek =
//! `ceviri` fonksiyonuna yeni bir eşleme eklemek.  Anahtarlar `enum` olduğundan
//! eksik çeviri **derleme zamanında** yakalanır (stringly-typed arama yoktur).

/// Desteklenen arayüz dilleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Dil {
    /// İngilizce.
    En,
    /// Türkçe (varsayılan).
    #[default]
    Tr,
}

impl Dil {
    /// Dilin kendi adındaki tam etiketi ("English" / "Türkçe").
    pub fn etiket(&self) -> &'static str {
        match self {
            Dil::En => "English",
            Dil::Tr => "Türkçe",
        }
    }

    /// İki harfli kısa kod ("EN" / "TR").
    pub fn kisa(&self) -> &'static str {
        match self {
            Dil::En => "EN",
            Dil::Tr => "TR",
        }
    }
}

/// Çevrilebilir metin anahtarları.  Her anahtar `ceviri` içinde her dil için karşılanır.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anahtar {
    // Ortak butonlar / eylemler
    Tamam,
    Iptal,
    Evet,
    Hayir,
    TekrarDene,
    Kapat,
    Detaylar,
    Indir,
    YenidenBagla,
    Devam,
    Kopyala,
    Geri,
    Ileri,
    Olustur,
    // Hata diyaloğu (standart şema)
    HataBasligi,
    NeOldu,
    Neden,
    NasilCozulur,
    TeknikDetay,
    KorelasyonKimligi,
    // Onay diyaloğu
    OnayBasligi,
    GeriAlinabilir,
    GeriAlinamaz,
    // Büyük işlem tahmini
    TahminBasligi,
    // Bellek bütçesi diyaloğu (İP-08)
    ButceBasligi,
    ButceAkisModu,
    ButceBulut,
    // İş / ilerleme
    IsCalisiyor,
    IsBekliyor,
    IsBitti,
    IsHata,
    IsIptal,
    IsIptalEdildi,
    TahminiSure,
    // Yükleme
    Yukleniyor,
    // Durum rozetleri
    DurumCevrimici,
    DurumCevrimdisi,
    DurumKaynakYetersiz,
    DurumSogutuluyor,
    DurumEklentiYok,
    DurumTasinmisKaynak,
    // Bildirim/toast başlıkları
    BildirimBasari,
    BildirimUyari,
    BildirimHata,
    BildirimBilgi,
}

impl Anahtar {
    /// Tüm anahtarlar — testler her anahtarın her dilde dolu olduğunu doğrular.
    pub const TUMU: &'static [Anahtar] = &[
        Anahtar::Tamam,
        Anahtar::Iptal,
        Anahtar::Evet,
        Anahtar::Hayir,
        Anahtar::TekrarDene,
        Anahtar::Kapat,
        Anahtar::Detaylar,
        Anahtar::Indir,
        Anahtar::YenidenBagla,
        Anahtar::Devam,
        Anahtar::Kopyala,
        Anahtar::Geri,
        Anahtar::Ileri,
        Anahtar::Olustur,
        Anahtar::HataBasligi,
        Anahtar::NeOldu,
        Anahtar::Neden,
        Anahtar::NasilCozulur,
        Anahtar::TeknikDetay,
        Anahtar::KorelasyonKimligi,
        Anahtar::OnayBasligi,
        Anahtar::GeriAlinabilir,
        Anahtar::GeriAlinamaz,
        Anahtar::TahminBasligi,
        Anahtar::ButceBasligi,
        Anahtar::ButceAkisModu,
        Anahtar::ButceBulut,
        Anahtar::IsCalisiyor,
        Anahtar::IsBekliyor,
        Anahtar::IsBitti,
        Anahtar::IsHata,
        Anahtar::IsIptal,
        Anahtar::IsIptalEdildi,
        Anahtar::TahminiSure,
        Anahtar::Yukleniyor,
        Anahtar::DurumCevrimici,
        Anahtar::DurumCevrimdisi,
        Anahtar::DurumKaynakYetersiz,
        Anahtar::DurumSogutuluyor,
        Anahtar::DurumEklentiYok,
        Anahtar::DurumTasinmisKaynak,
        Anahtar::BildirimBasari,
        Anahtar::BildirimUyari,
        Anahtar::BildirimHata,
        Anahtar::BildirimBilgi,
    ];
}

/// Bir anahtarı verilen dile çevirir.  Eksik dil/anahtar kombinasyonu derlenemez.
pub fn ceviri(dil: Dil, anahtar: Anahtar) -> &'static str {
    use Anahtar::*;
    use Dil::*;
    match (dil, anahtar) {
        (Tr, Tamam) => "Tamam",
        (En, Tamam) => "OK",
        (Tr, Iptal) => "İptal",
        (En, Iptal) => "Cancel",
        (Tr, Evet) => "Evet",
        (En, Evet) => "Yes",
        (Tr, Hayir) => "Hayır",
        (En, Hayir) => "No",
        (Tr, TekrarDene) => "Tekrar dene",
        (En, TekrarDene) => "Retry",
        (Tr, Kapat) => "Kapat",
        (En, Kapat) => "Close",
        (Tr, Detaylar) => "Detaylar",
        (En, Detaylar) => "Details",
        (Tr, Indir) => "İndir",
        (En, Indir) => "Download",
        (Tr, YenidenBagla) => "Yeniden bağla",
        (En, YenidenBagla) => "Reconnect",
        (Tr, Devam) => "Devam et",
        (En, Devam) => "Continue",
        (Tr, Kopyala) => "Kopyala",
        (En, Kopyala) => "Copy",
        (Tr, Geri) => "‹ Geri",
        (En, Geri) => "‹ Back",
        (Tr, Ileri) => "İleri ›",
        (En, Ileri) => "Next ›",
        (Tr, Olustur) => "Oluştur",
        (En, Olustur) => "Create",
        (Tr, HataBasligi) => "Bir hata oluştu",
        (En, HataBasligi) => "An error occurred",
        (Tr, NeOldu) => "Ne oldu?",
        (En, NeOldu) => "What happened?",
        (Tr, Neden) => "Neden oldu?",
        (En, Neden) => "Why did it happen?",
        (Tr, NasilCozulur) => "Nasıl çözülür?",
        (En, NasilCozulur) => "How to fix it",
        (Tr, TeknikDetay) => "Teknik detay",
        (En, TeknikDetay) => "Technical details",
        (Tr, KorelasyonKimligi) => "Hata kimliği",
        (En, KorelasyonKimligi) => "Error ID",
        (Tr, OnayBasligi) => "Emin misiniz?",
        (En, OnayBasligi) => "Are you sure?",
        (Tr, GeriAlinabilir) => "Bu işlem daha sonra geri alınabilir.",
        (En, GeriAlinabilir) => "This action can be undone later.",
        (Tr, GeriAlinamaz) => "Bu işlem geri alınamaz.",
        (En, GeriAlinamaz) => "This action cannot be undone.",
        (Tr, TahminBasligi) => "Büyük işlem",
        (En, TahminBasligi) => "Large operation",
        (Tr, ButceBasligi) => "Bellek bütçesi",
        (En, ButceBasligi) => "Memory budget",
        (Tr, ButceAkisModu) => "Akış modunda aç",
        (En, ButceAkisModu) => "Open in stream mode",
        (Tr, ButceBulut) => "Bulutta işle (yakında)",
        (En, ButceBulut) => "Process in cloud (soon)",
        (Tr, IsCalisiyor) => "Çalışıyor…",
        (En, IsCalisiyor) => "Running…",
        (Tr, IsBekliyor) => "Sırada bekliyor…",
        (En, IsBekliyor) => "Queued…",
        (Tr, IsBitti) => "Tamamlandı",
        (En, IsBitti) => "Completed",
        (Tr, IsHata) => "Hata",
        (En, IsHata) => "Error",
        (Tr, IsIptal) => "İptal et",
        (En, IsIptal) => "Cancel",
        (Tr, IsIptalEdildi) => "İptal ediliyor…",
        (En, IsIptalEdildi) => "Cancelling…",
        (Tr, TahminiSure) => "Tahmini süre",
        (En, TahminiSure) => "Estimated time",
        (Tr, Yukleniyor) => "Yükleniyor…",
        (En, Yukleniyor) => "Loading…",
        (Tr, DurumCevrimici) => "Çevrimiçi",
        (En, DurumCevrimici) => "Online",
        (Tr, DurumCevrimdisi) => "Çevrimdışı",
        (En, DurumCevrimdisi) => "Offline",
        (Tr, DurumKaynakYetersiz) => "Kaynak yetersiz",
        (En, DurumKaynakYetersiz) => "Low resources",
        (Tr, DurumSogutuluyor) => "Soğutuluyor",
        (En, DurumSogutuluyor) => "Cooling down",
        (Tr, DurumEklentiYok) => "Eklenti kurulu değil",
        (En, DurumEklentiYok) => "Plugin not installed",
        (Tr, DurumTasinmisKaynak) => "Kaynak taşınmış",
        (En, DurumTasinmisKaynak) => "Resource moved",
        (Tr, BildirimBasari) => "Başarılı",
        (En, BildirimBasari) => "Success",
        (Tr, BildirimUyari) => "Uyarı",
        (En, BildirimUyari) => "Warning",
        (Tr, BildirimHata) => "Hata",
        (En, BildirimHata) => "Error",
        (Tr, BildirimBilgi) => "Bilgi",
        (En, BildirimBilgi) => "Info",
    }
}

/// Saniyeyi insana okunaklı süreye çevirir ("30 sn", "5 dk", "1 sa 5 dk").
pub fn insan_sure(dil: Dil, saniye: f64) -> String {
    let toplam = saniye.max(0.0).round() as u64;
    let sa = toplam / 3600;
    let dk = (toplam % 3600) / 60;
    let sn = toplam % 60;
    match dil {
        Dil::Tr => {
            if sa > 0 && dk > 0 {
                format!("{sa} sa {dk} dk")
            } else if sa > 0 {
                format!("{sa} sa")
            } else if dk > 0 && sn > 0 && dk < 5 {
                format!("{dk} dk {sn} sn")
            } else if dk > 0 {
                format!("{dk} dk")
            } else {
                format!("{sn} sn")
            }
        }
        Dil::En => {
            if sa > 0 && dk > 0 {
                format!("{sa} h {dk} min")
            } else if sa > 0 {
                format!("{sa} h")
            } else if dk > 0 && sn > 0 && dk < 5 {
                format!("{dk} min {sn} s")
            } else if dk > 0 {
                format!("{dk} min")
            } else {
                format!("{sn} s")
            }
        }
    }
}

/// "Bu işlem ~X sürebilir. Devam edilsin mi?" metnini üretir (büyük işlem tahmini).
pub fn tahmin_metni(dil: Dil, sure: &str) -> String {
    match dil {
        Dil::Tr => format!("Bu işlem yaklaşık {sure} sürebilir. Devam edilsin mi?"),
        Dil::En => format!("This may take about {sure}. Do you want to continue?"),
    }
}

/// Bellek bütçesi diyaloğunun açıklama metnini üretir (İP-08).
/// `dosya`/`tahmini`/`bos`: önceden insan-okunur biçime çevrilmiş bayt dizgeleri.
pub fn butce_metni(dil: Dil, dosya: &str, tahmini: &str, bos: &str) -> String {
    match dil {
        Dil::Tr => format!(
            "Bu dosya diskte {dosya} yer kaplıyor; tamamını açmak tahminen {tahmini} bellek \
             ister, ama şu an boşta yalnızca {bos} var. Tümünü belleğe almak yerine akış \
             (stream) modunu öneriyoruz — dosya parça parça işlenir, çökme olmaz."
        ),
        Dil::En => format!(
            "This file is {dosya} on disk; opening it fully would need about {tahmini} of \
             memory, but only {bos} is free right now. Instead of loading it all, we recommend \
             stream mode — the file is processed piece by piece, with no crash."
        ),
    }
}

/// "Tahmini kalan: X" metnini üretir (ilerleme bileşeni).
pub fn kalan_sure_metni(dil: Dil, saniye: f64) -> String {
    let s = insan_sure(dil, saniye);
    match dil {
        Dil::Tr => format!("Tahmini kalan: {s}"),
        Dil::En => format!("Estimated remaining: {s}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tum_anahtarlar_her_dilde_dolu_olmali() {
        for &a in Anahtar::TUMU {
            assert!(!ceviri(Dil::Tr, a).is_empty(), "TR çeviri boş: {a:?}");
            assert!(!ceviri(Dil::En, a).is_empty(), "EN çeviri boş: {a:?}");
        }
    }

    #[test]
    fn tr_ve_en_farkli_metin_vermeli() {
        // Birkaç temsilî anahtar için iki dil gerçekten farklı olmalı (çeviri yapılmış).
        for &a in &[Anahtar::Tamam, Anahtar::NeOldu, Anahtar::DurumCevrimdisi] {
            assert_ne!(ceviri(Dil::Tr, a), ceviri(Dil::En, a));
        }
    }

    #[test]
    fn insan_sure_dogru_bicimlenir() {
        assert_eq!(insan_sure(Dil::Tr, 30.0), "30 sn");
        assert_eq!(insan_sure(Dil::Tr, 90.0), "1 dk 30 sn");
        assert_eq!(insan_sure(Dil::Tr, 600.0), "10 dk");
        assert_eq!(insan_sure(Dil::Tr, 3600.0), "1 sa");
        assert_eq!(insan_sure(Dil::En, 30.0), "30 s");
    }

    #[test]
    fn tahmin_metni_sureyi_icerir() {
        let m = tahmin_metni(Dil::Tr, "5 dk");
        assert!(m.contains("5 dk"));
    }
}
