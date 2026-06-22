//! **Temel** Python kod tamamlama — **out-of-process** (İP-06, 2. kısım — Gün 23, MK-02).
//!
//! Kod editörü yazarken tamamlama önerileri ister.  İki katman vardır:
//!
//! 1. **Saf-Rust yedek** ([`temel_tamamla`]) — **her zaman çalışır**, bağımlılıksız: Python
//!    anahtar kelimeleri + yerleşikler + **tampondaki tanımlayıcılar**, yazılan öneke göre
//!    süzülür.  Hiçbir araç kurulu olmasa bile editör temel tamamlama verir.
//! 2. **jedi (out-of-process)** ([`tamamla_async`]) — kuruluysa **bağlam-duyarlı** (içe
//!    aktarılan modüller, nesne öznitelikleri…) daha akıllı tamamlama.  **Ayrı süreçte**
//!    çalışır (jedi saf Python'dur → **C derleyici gerekmez**); yanıt **asenkron** toplanır,
//!    **arayüz beklemez/donmaz** (kabul kriteri: "LSP donduruyor" → ayrı süreç + asenkron).
//!
//! ## ⚠️ Tam dil zekâsı **v1.x**
//! Tanı (diagnostics), imleç-üstü bilgi (hover), tanıma-git (go-to-def), yeniden adlandırma,
//! tam LSP sunucusu (pyright) ve **Python dışı diller** bu sürümde **YOKTUR** —
//! `MVP-sonrasi.md` §3.1.  Buradaki yüzey "**temel**"dir ve UI'da öyle etiketlenir.
//!
//! ## Araç eksikse [Kur]
//! jedi kurulu değilse [`LspDurumu::JediYok`] döner; editör yedek tamamlamayı sürdürür ve
//! [`jedi_kur_rehberi`] ile **"[Kur]"** yönlendirmesi gösterir (proje sanal ortamına kurulur —
//! bkz. [`crate::exec::ortam`]).
// MK-02: tamamlama daima ayrı süreçte (jedi); in-process Python (PyO3) YASAK — donmama bundan gelir.

use biocraft_types::ErrorReport;
use serde::Deserialize;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// jedi tamamlamasının üst süre sınırı (aşılırsa süreç öldürülür → arayüz donmaz).
const TAMAMLAMA_ZAMAN_ASIMI: Duration = Duration::from_secs(5);

/// Bir çalıştırmada döndürülen azami öneri sayısı (UI + bellek koruması).
pub const AZAMI_ONERI: usize = 50;

/// Eşzamanlı tamamlama çağrıları için benzersiz geçici dizin sayacı.
static SAYAC: AtomicU64 = AtomicU64::new(0);

// ─── Tamamlama veri tipleri ────────────────────────────────────────────────────

/// Bir tamamlama önerisinin türü (UI ikonu/renk seçimi; token'dan renklenir — MK-52).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TamamlamaTuru {
    /// Dil anahtar kelimesi (`def`, `for`, `import`…).
    AnahtarKelime,
    /// Yerleşik fonksiyon/tip (`print`, `len`, `range`…).
    Yerlesik,
    /// Fonksiyon.
    Fonksiyon,
    /// Sınıf/tür.
    Sinif,
    /// Modül.
    Modul,
    /// Değişken/isim (tampondan veya jedi).
    Tanimlayici,
    /// Öznitelik/üye.
    Ozellik,
    /// Sınıflandırılamayan.
    Diger,
}

impl TamamlamaTuru {
    /// jedi'nin döndürdüğü tür dizgesinden eşler.
    fn jediden(s: &str) -> Self {
        match s {
            "keyword" => TamamlamaTuru::AnahtarKelime,
            "function" => TamamlamaTuru::Fonksiyon,
            "class" => TamamlamaTuru::Sinif,
            "module" => TamamlamaTuru::Modul,
            "instance" | "statement" | "param" => TamamlamaTuru::Tanimlayici,
            "property" => TamamlamaTuru::Ozellik,
            _ => TamamlamaTuru::Diger,
        }
    }

    /// Renk seçimi için token anahtarı (MK-52 — sabit RGB yok).
    pub fn token_anahtari(self) -> &'static str {
        match self {
            TamamlamaTuru::AnahtarKelime => "syntax.keyword",
            TamamlamaTuru::Yerlesik | TamamlamaTuru::Fonksiyon => "syntax.function",
            TamamlamaTuru::Sinif | TamamlamaTuru::Modul => "syntax.type",
            _ => "text.normal",
        }
    }
}

/// Tek bir tamamlama önerisi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tamamlama {
    /// Eklenecek/gösterilecek metin.
    pub etiket: String,
    /// Önerinin türü.
    pub tur: TamamlamaTuru,
    /// Kısa açıklama (jedi'den; yoksa boş).
    pub detay: String,
}

impl Tamamlama {
    /// Etiket + tür ile (detaysız) bir öneri.
    pub fn yeni(etiket: impl Into<String>, tur: TamamlamaTuru) -> Self {
        Self {
            etiket: etiket.into(),
            tur,
            detay: String::new(),
        }
    }
}

/// Bir tamamlama isteği — kod + imleç konumu (0-tabanlı satır/sütun).
#[derive(Debug, Clone)]
pub struct TamamlamaIstegi {
    /// Tüm tampon metni.
    pub kod: String,
    /// İmleç satırı (0-tabanlı).
    pub satir: usize,
    /// İmleç sütunu (0-tabanlı, bayt değil **karakter**).
    pub sutun: usize,
}

/// Asenkron jedi tamamlamasının sonucu.
#[derive(Debug, Clone)]
pub enum TamamlamaYaniti {
    /// Öneriler hazır.
    Hazir(Vec<Tamamlama>),
    /// jedi kurulu değil → editör yedek tamamlamayı kullanır + [Kur] gösterir.
    JediYok,
    /// Süreç/iletişim hatası (yedek tamamlama yine de kullanılabilir).
    Hata(String),
}

/// jedi LSP yüzeyinin o anki durumu ([Kur] kararı için).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspDurumu {
    /// jedi kurulu — bağlam-duyarlı tamamlama hazır.
    Hazir,
    /// Python var ama jedi yok → [Kur] (pip install jedi).
    JediYok,
    /// Python bulunamadı → önce Python kurulmalı.
    PythonYok,
}

// ─── Asenkron jedi tutamacı ────────────────────────────────────────────────────

/// Çalışan bir jedi tamamlamasına erişim — sonucu **bloklamadan** dener (UI her karede yoklar).
pub struct TamamlamaTutamac {
    al: Receiver<TamamlamaYaniti>,
}

impl TamamlamaTutamac {
    /// Sonuç hazırsa döner; değilse `None` (arayüz beklemez).
    pub fn dene(&self) -> Option<TamamlamaYaniti> {
        self.al.try_recv().ok()
    }
}

/// jedi tamamlamasını **ayrı süreçte** başlatır; hemen bir tutamaç döner (UI donmaz).
///
/// `yorumlayici` proje sanal ortamının Python'u olmalıdır (jedi oraya kurulur — izole ortam).
pub fn tamamla_async(yorumlayici: &Path, istek: TamamlamaIstegi) -> TamamlamaTutamac {
    let (gonder, al) = mpsc::channel();
    let yorumlayici = yorumlayici.to_path_buf();
    // Ayrı iş parçacığı: süreci başlatır, sonucu toplar, kanala yollar (UI bloklanmaz).
    std::thread::spawn(move || {
        let yanit = tamamla_senkron(&yorumlayici, &istek);
        let _ = gonder.send(yanit);
    });
    TamamlamaTutamac { al }
}

/// jedi yardımcısını ayrı süreçte çalıştırıp yanıtı çözer (arka plan iş parçacığında çalışır).
fn tamamla_senkron(yorumlayici: &Path, istek: &TamamlamaIstegi) -> TamamlamaYaniti {
    let n = SAYAC.fetch_add(1, Ordering::Relaxed);
    let dizin = std::env::temp_dir().join(format!("biocraft_lsp_{}_{}", std::process::id(), n));
    if std::fs::create_dir_all(&dizin).is_err() {
        return TamamlamaYaniti::Hata("geçici dizin oluşturulamadı".into());
    }
    let kaynak = dizin.join("kaynak.py");
    let yardimci = dizin.join("_jedi_yardimci.py");
    let temizle = || {
        let _ = std::fs::remove_dir_all(&dizin);
    };
    if std::fs::write(&kaynak, &istek.kod).is_err()
        || std::fs::write(&yardimci, JEDI_YARDIMCI).is_err()
    {
        temizle();
        return TamamlamaYaniti::Hata("geçici dosya yazılamadı".into());
    }

    // jedi 1-tabanlı satır ister; sütun 0-tabanlı kalır.
    let mut komut = Command::new(yorumlayici);
    komut
        .arg(&yardimci)
        .arg(&kaynak)
        .arg((istek.satir + 1).to_string())
        .arg(istek.sutun.to_string())
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut cocuk = match komut.spawn() {
        Ok(c) => c,
        Err(e) => {
            temizle();
            return TamamlamaYaniti::Hata(format!("süreç başlatılamadı: {e}"));
        }
    };

    // Zaman aşımı: süreç askıda kalırsa öldür (arayüz donmasın — kabul kriteri).
    let bitis = Instant::now() + TAMAMLAMA_ZAMAN_ASIMI;
    loop {
        match cocuk.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() >= bitis {
                    let _ = cocuk.kill();
                    let _ = cocuk.wait();
                    temizle();
                    return TamamlamaYaniti::Hata("tamamlama zaman aşımına uğradı".into());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                temizle();
                return TamamlamaYaniti::Hata(e.to_string());
            }
        }
    }

    let cikti = match cocuk.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            temizle();
            return TamamlamaYaniti::Hata(e.to_string());
        }
    };
    temizle();
    let stdout = String::from_utf8_lossy(&cikti.stdout);
    cozumle(stdout.trim())
}

/// jedi yardımcısının JSON satırını [`TamamlamaYaniti`]'ne çözer (saf — testlenebilir).
fn cozumle(stdout: &str) -> TamamlamaYaniti {
    // Yardımcı son satırda JSON basar; en son JSON-benzeri satırı al.
    let satir = stdout
        .lines()
        .rev()
        .find(|s| s.trim_start().starts_with('{'));
    let Some(satir) = satir else {
        return TamamlamaYaniti::Hata("jedi yanıtı boş".into());
    };
    match serde_json::from_str::<JediYanit>(satir.trim()) {
        Ok(y) => {
            if y.hata.as_deref() == Some("jedi-yok") {
                return TamamlamaYaniti::JediYok;
            }
            if let Some(h) = y.hata {
                return TamamlamaYaniti::Hata(h);
            }
            let oneriler = y
                .tamamlamalar
                .into_iter()
                .take(AZAMI_ONERI)
                .map(|i| Tamamlama {
                    tur: TamamlamaTuru::jediden(&i.tur),
                    etiket: i.etiket,
                    detay: i.detay,
                })
                .collect();
            TamamlamaYaniti::Hazir(oneriler)
        }
        Err(e) => TamamlamaYaniti::Hata(format!("geçersiz jedi yanıtı: {e}")),
    }
}

#[derive(Deserialize)]
struct JediYanit {
    #[serde(default)]
    tamamlamalar: Vec<JediItem>,
    #[serde(default)]
    hata: Option<String>,
}

#[derive(Deserialize)]
struct JediItem {
    etiket: String,
    #[serde(default)]
    tur: String,
    #[serde(default)]
    detay: String,
}

/// jedi yardımcı betiği — **ayrı süreçte** çalışır (MK-02).  jedi yoksa `{"hata":"jedi-yok"}`.
const JEDI_YARDIMCI: &str = r#"# BioCraft Engine — jedi out-of-process tamamlama yardımcısı (İP-06).
import sys, json
def main():
    try:
        import jedi
    except Exception:
        print(json.dumps({"hata": "jedi-yok"}))
        return
    try:
        kaynak_yolu = sys.argv[1]
        satir = int(sys.argv[2])   # 1-tabanlı
        sutun = int(sys.argv[3])   # 0-tabanlı
        with open(kaynak_yolu, "r", encoding="utf-8") as f:
            kaynak = f.read()
        script = jedi.Script(code=kaynak, path=kaynak_yolu)
        comps = script.complete(satir, sutun)
        out = []
        for c in comps[:50]:
            try:
                detay = (c.description or "")[:80]
            except Exception:
                detay = ""
            out.append({"etiket": c.name, "tur": c.type or "", "detay": detay})
        print(json.dumps({"tamamlamalar": out}))
    except Exception as e:
        print(json.dumps({"hata": str(e)}))
if __name__ == "__main__":
    main()
"#;

// ─── jedi keşfi + [Kur] rehberi ────────────────────────────────────────────────

/// Verilen yorumlayıcıda jedi kurulu mu? (`python -c "import jedi"`; hızlı, bloklar.)
pub fn jedi_var_mi(yorumlayici: &Path) -> bool {
    Command::new(yorumlayici)
        .arg("-c")
        .arg("import jedi")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// LSP durumunu belirler ([Kur] kararı için): jedi var mı / Python var mı.
pub fn lsp_durumu(yorumlayici: Option<&Path>) -> LspDurumu {
    match yorumlayici {
        None => LspDurumu::PythonYok,
        Some(y) if jedi_var_mi(y) => LspDurumu::Hazir,
        Some(_) => LspDurumu::JediYok,
    }
}

/// jedi kurulu değilken gösterilecek **[Kur]** rehberi (proje sanal ortamına kurulur).
pub fn jedi_kur_rehberi() -> ErrorReport {
    ErrorReport::new(
        "Akıllı tamamlama için jedi gerekli",
        "Bağlam-duyarlı Python tamamlaması için 'jedi' paketi proje ortamında kurulu değil",
        "Paketler panelinden 'jedi' paketini [Kur]; bu sırada temel tamamlama çalışmayı sürdürür",
    )
    .with_eylem("jedi'yi kur")
}

// ─── Saf-Rust yedek tamamlama (her zaman çalışır) ──────────────────────────────

/// Python anahtar kelimeleri (yedek tamamlama).
const PYTHON_ANAHTAR: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
    "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda",
    "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with", "yield", "True",
    "False", "None",
];

/// Python yerleşikleri (yedek tamamlama).
const PYTHON_YERLESIK: &[&str] = &[
    "print",
    "len",
    "range",
    "int",
    "float",
    "str",
    "bool",
    "list",
    "dict",
    "set",
    "tuple",
    "sum",
    "min",
    "max",
    "abs",
    "round",
    "sorted",
    "enumerate",
    "zip",
    "map",
    "filter",
    "open",
    "input",
    "type",
    "isinstance",
    "format",
    "repr",
    "any",
    "all",
    "reversed",
    "id",
    "hash",
];

/// **Saf-Rust temel tamamlama** — bağımlılıksız, her zaman çalışır.
///
/// Anahtar kelimeler + yerleşikler + **tampondaki tanımlayıcılar**, `onek` ile süzülür
/// (önek boşsa anahtar kelime + yerleşikler döner).  Önek **büyük/küçük harf duyarlı**
/// (Python böyle); sonuç tekilleştirilir, türe + alfabetik sıraya göre dizilir, [`AZAMI_ONERI`]
/// ile kırpılır.
pub fn temel_tamamla(kod: &str, onek: &str) -> Vec<Tamamlama> {
    let eslesir = |aday: &str| -> bool {
        if onek.is_empty() {
            true
        } else {
            aday.starts_with(onek) && aday != onek
        }
    };

    let mut sonuc: Vec<Tamamlama> = Vec::new();
    let mut gorulen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let ekle = |sonuc: &mut Vec<Tamamlama>,
                gorulen: &mut std::collections::HashSet<String>,
                ad: &str,
                tur: TamamlamaTuru| {
        if eslesir(ad) && gorulen.insert(ad.to_string()) {
            sonuc.push(Tamamlama::yeni(ad, tur));
        }
    };

    for a in PYTHON_ANAHTAR {
        ekle(&mut sonuc, &mut gorulen, a, TamamlamaTuru::AnahtarKelime);
    }
    for y in PYTHON_YERLESIK {
        ekle(&mut sonuc, &mut gorulen, y, TamamlamaTuru::Yerlesik);
    }
    // Tampondaki tanımlayıcılar (kullanıcının kendi adları) — önek boşsa eklenmez (gürültü).
    if !onek.is_empty() {
        for ad in tanimlayicilar(kod) {
            ekle(&mut sonuc, &mut gorulen, &ad, TamamlamaTuru::Tanimlayici);
        }
    }

    // Tür önceliği (anahtar/yerleşik üstte) + alfabetik; sonra kırp.
    sonuc.sort_by(|a, b| {
        tur_oncelik(a.tur)
            .cmp(&tur_oncelik(b.tur))
            .then_with(|| a.etiket.cmp(&b.etiket))
    });
    sonuc.truncate(AZAMI_ONERI);
    sonuc
}

/// İmleç sütununda **yazılmakta olan öneki** çıkarır (tanımlayıcı parçası).
///
/// `satir_metni` = imlecin bulunduğu satır; `sutun` = 0-tabanlı **karakter** sütunu.
/// Geriye doğru tanımlayıcı karakterleri (`A-Za-z0-9_`) toplar.  Nokta öncesi durur
/// (`obj.me|` → önek `me`).
pub fn onek_al(satir_metni: &str, sutun: usize) -> String {
    let karakterler: Vec<char> = satir_metni.chars().collect();
    let son = sutun.min(karakterler.len());
    let mut bas = son;
    while bas > 0 {
        let c = karakterler[bas - 1];
        if c.is_alphanumeric() || c == '_' {
            bas -= 1;
        } else {
            break;
        }
    }
    karakterler[bas..son].iter().collect()
}

/// Metindeki benzersiz tanımlayıcıları (Python adlarını) çıkarır (saf — regex'siz).
fn tanimlayicilar(kod: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut gorulen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut gecerli = String::new();
    let bitir = |gecerli: &mut String,
                 out: &mut Vec<String>,
                 gorulen: &mut std::collections::HashSet<String>| {
        if !gecerli.is_empty() {
            // Yalnız adlar (rakamla başlamayan) ve makul uzunluk.
            let ilk = gecerli.chars().next().unwrap();
            if (ilk.is_alphabetic() || ilk == '_')
                && gecerli.len() >= 2
                && gorulen.insert(gecerli.clone())
            {
                out.push(gecerli.clone());
            }
            gecerli.clear();
        }
    };
    for c in kod.chars() {
        if c.is_alphanumeric() || c == '_' {
            gecerli.push(c);
        } else {
            bitir(&mut gecerli, &mut out, &mut gorulen);
        }
    }
    bitir(&mut gecerli, &mut out, &mut gorulen);
    out
}

/// Tür sıralama önceliği (küçük = üstte).
fn tur_oncelik(t: TamamlamaTuru) -> u8 {
    match t {
        TamamlamaTuru::AnahtarKelime => 0,
        TamamlamaTuru::Yerlesik | TamamlamaTuru::Fonksiyon => 1,
        TamamlamaTuru::Sinif | TamamlamaTuru::Modul => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onek_dogru_cikarilir() {
        assert_eq!(onek_al("    pri", 7), "pri");
        assert_eq!(onek_al("obj.me", 6), "me"); // nokta öncesi durur
        assert_eq!(onek_al("x = ran", 7), "ran");
        assert_eq!(onek_al("", 0), "");
        // Sütun kelimenin ortasındaysa yalnız soldaki parça.
        assert_eq!(onek_al("hello", 3), "hel");
    }

    #[test]
    fn temel_tamamla_anahtar_kelime_onek() {
        let oneriler = temel_tamamla("", "de");
        assert!(oneriler.iter().any(|o| o.etiket == "def"));
        assert!(oneriler.iter().any(|o| o.etiket == "del"));
        // "def" anahtar kelime türünde.
        let def = oneriler.iter().find(|o| o.etiket == "def").unwrap();
        assert_eq!(def.tur, TamamlamaTuru::AnahtarKelime);
    }

    #[test]
    fn temel_tamamla_yerlesik() {
        let oneriler = temel_tamamla("", "pri");
        assert!(oneriler
            .iter()
            .any(|o| o.etiket == "print" && o.tur == TamamlamaTuru::Yerlesik));
    }

    #[test]
    fn temel_tamamla_tampon_tanimlayici() {
        let kod = "def hesapla_toplam(x):\n    return x\nhesapla_to";
        let oneriler = temel_tamamla(kod, "hesapla_to");
        assert!(
            oneriler.iter().any(|o| o.etiket == "hesapla_toplam"),
            "tampondaki ad önerilmeli: {oneriler:?}"
        );
    }

    #[test]
    fn temel_tamamla_kendini_onermez() {
        // Önek tam bir anahtar kelimeye eşitse onu tekrar önermez (zaten yazılı).
        let oneriler = temel_tamamla("", "def");
        assert!(!oneriler.iter().any(|o| o.etiket == "def"));
    }

    #[test]
    fn tanimlayicilar_cikarilir() {
        let kod = "x = 1\ndef foo():\n    bar = x + 2";
        let adlar = tanimlayicilar(kod);
        assert!(adlar.contains(&"foo".to_string()));
        assert!(adlar.contains(&"bar".to_string()));
        // Tek harfli 'x' atlanır (>=2 kuralı, gürültü azaltma).
        assert!(!adlar.contains(&"x".to_string()));
    }

    #[test]
    fn cozumle_jedi_yok() {
        let y = cozumle(r#"{"hata": "jedi-yok"}"#);
        assert!(matches!(y, TamamlamaYaniti::JediYok));
    }

    #[test]
    fn cozumle_tamamlamalar() {
        let json = r#"{"tamamlamalar": [{"etiket": "append", "tur": "function", "detay": "list.append"}]}"#;
        match cozumle(json) {
            TamamlamaYaniti::Hazir(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0].etiket, "append");
                assert_eq!(v[0].tur, TamamlamaTuru::Fonksiyon);
            }
            d => panic!("Hazir bekleniyordu: {d:?}"),
        }
    }

    #[test]
    fn cozumle_bos_hata() {
        assert!(matches!(cozumle(""), TamamlamaYaniti::Hata(_)));
    }

    #[test]
    fn jedi_tur_eslemesi() {
        assert_eq!(
            TamamlamaTuru::jediden("keyword"),
            TamamlamaTuru::AnahtarKelime
        );
        assert_eq!(TamamlamaTuru::jediden("class"), TamamlamaTuru::Sinif);
        assert_eq!(TamamlamaTuru::jediden("bilinmeyen"), TamamlamaTuru::Diger);
    }

    /// Python + jedi varsa: gerçek out-of-process tamamlama çalışır.  Yoksa test atlanır.
    #[test]
    fn jedi_varsa_uctan_uca() {
        let Some(py) = crate::runtime::subprocess::python_bul() else {
            eprintln!("Python yok → test atlandı");
            return;
        };
        if !jedi_var_mi(&py) {
            eprintln!("jedi kurulu değil → test atlandı");
            return;
        }
        let istek = TamamlamaIstegi {
            kod: "import os\nos.".into(),
            satir: 1,
            sutun: 3,
        };
        let tutamac = tamamla_async(&py, istek);
        let basla = Instant::now();
        loop {
            if let Some(y) = tutamac.dene() {
                match y {
                    TamamlamaYaniti::Hazir(v) => {
                        assert!(!v.is_empty(), "os. için öneri beklenir");
                        // os modülünün bilinen bir üyesi gelmeli.
                        assert!(v.iter().any(|o| o.etiket == "path" || o.etiket == "getcwd"));
                    }
                    TamamlamaYaniti::JediYok => panic!("jedi var sanılıyordu"),
                    TamamlamaYaniti::Hata(h) => panic!("tamamlama hatası: {h}"),
                }
                break;
            }
            if basla.elapsed() > Duration::from_secs(15) {
                panic!("jedi tamamlaması zamanında dönmedi");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
