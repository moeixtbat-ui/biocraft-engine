//! Saf-Rust, **artımlı** söz dizimi vurgulama (İP-06, MK-55).
//!
//! Proje sahibinin kararıyla (Gün 22) vurgulama **saf Rust**'tır: Tree-sitter (C kütüphanesi)
//! bu sürümde eklenmez; böylece projenin "**C derleyici GEREKMEZ**" ilkesi korunur (her
//! makinede sorunsuz derlenir).  [`Vurgulayici`] trait'i sayesinde ileride Tree-sitter
//! tabanlı bir uygulama **takılabilir** (v1.x — `MVP-sonrasi.md` §3.1); arayüz değişmez.
//!
//! ## Neden "artımlı"?
//! Büyük dosyada akıcılığın anahtarı budur (MK-04 kare bütçesi): tüm dosya değil, yalnız
//! **değişen** satırlar yeniden jetonlanır ([`VurgulamaOnbellek`]).  Çok-satırlı yapılar
//! (Python üç-tırnak dizesi) için her satır, bir **giriş durumuyla** ([`SatirDurumu`])
//! jetonlanır; bu durum değişmediği sürece satır önbellekten gelir.
//!
//! ## Renk (MK-52)
//! Jeton türü somut RGB taşımaz; yalnızca **anlamsal token anahtarı**
//! ([`JetonTuru::token_anahtari`]) verir.  Arayüz bunu aktif temadan
//! ([`crate::tokens::Tokenlar::anahtar_renk`]) çözer → tema değişince renk de değişir.
// MK-52: sabit renk YOK — yalnız token anahtarı.  MK-55: native, artımlı.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Editörün tanıdığı kod dilleri.  Python önceliklidir; ötekiler temel düzeyde.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KodDili {
    /// Python (öncelikli dil).
    #[default]
    Python,
    /// R.
    R,
    /// Bash/Shell.
    Bash,
    /// JSON.
    Json,
    /// YAML.
    Yaml,
    /// RON (Rusty Object Notation).
    Ron,
    /// Düz metin / tanınmayan (vurgusuz).
    Duz,
}

impl KodDili {
    /// Dosya uzantısından dili tahmin eder (büyük/küçük harf duyarsız).
    pub fn uzantidan(uzanti: &str) -> Self {
        match uzanti.trim_start_matches('.').to_ascii_lowercase().as_str() {
            "py" | "pyw" | "pyi" => KodDili::Python,
            "r" => KodDili::R,
            "sh" | "bash" | "zsh" => KodDili::Bash,
            "json" => KodDili::Json,
            "yaml" | "yml" => KodDili::Yaml,
            "ron" => KodDili::Ron,
            _ => KodDili::Duz,
        }
    }

    /// Bir dosya yolundan dili tahmin eder.
    pub fn yoldan(yol: &std::path::Path) -> Self {
        yol.extension()
            .and_then(|e| e.to_str())
            .map(KodDili::uzantidan)
            .unwrap_or(KodDili::Duz)
    }

    /// İnsan-okur ad (durum çubuğu/teşhis).
    pub fn ad(self) -> &'static str {
        match self {
            KodDili::Python => "Python",
            KodDili::R => "R",
            KodDili::Bash => "Bash",
            KodDili::Json => "JSON",
            KodDili::Yaml => "YAML",
            KodDili::Ron => "RON",
            KodDili::Duz => "Düz metin",
        }
    }

    /// Bu dilin satır-içi yorum başlangıcı (varsa).  JSON'da yorum yoktur.
    fn yorum_oneki(self) -> Option<&'static str> {
        match self {
            KodDili::Python | KodDili::Bash | KodDili::Yaml | KodDili::R => Some("#"),
            KodDili::Ron => Some("//"),
            KodDili::Json | KodDili::Duz => None,
        }
    }

    /// Bu dilde Python tarzı üç-tırnak çok-satırlı dize var mı?
    fn uc_tirnak_var(self) -> bool {
        matches!(self, KodDili::Python)
    }

    /// Bu dilin anahtar kelimeleri (vurgulama için).
    fn anahtar_kelimeler(self) -> &'static [&'static str] {
        match self {
            KodDili::Python => &[
                "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
                "continue", "def", "del", "elif", "else", "except", "finally", "for", "from",
                "global", "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass",
                "raise", "return", "try", "while", "with", "yield", "match", "case",
            ],
            KodDili::R => &[
                "if", "else", "repeat", "while", "function", "for", "in", "next", "break", "TRUE",
                "FALSE", "NULL", "Inf", "NaN", "NA", "return", "library", "require",
            ],
            KodDili::Bash => &[
                "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac",
                "function", "in", "return", "export", "local", "echo", "exit",
            ],
            KodDili::Yaml => &["true", "false", "null", "yes", "no", "on", "off"],
            KodDili::Json => &["true", "false", "null"],
            KodDili::Ron => &["true", "false", "Some", "None"],
            KodDili::Duz => &[],
        }
    }
}

/// Bir jetonun anlamsal türü.  Somut renk taşımaz (MK-52); yalnız token anahtarı verir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JetonTuru {
    /// Dilin anahtar kelimesi (`def`, `if`, `return`…).
    AnahtarKelime,
    /// Dize sabiti (`"..."`, `'...'`, üç-tırnak).
    Dize,
    /// Yorum.
    Yorum,
    /// Sayısal sabit.
    Sayi,
    /// Operatör (`+ - = == < >`…).
    Operator,
    /// Tanımlayıcı (değişken/fonksiyon adı).
    Tanimlayici,
    /// Noktalama / ayraç (`( ) [ ] { } , :`…).
    Noktalama,
    /// Sıradan metin / boşluk (vurgusuz).
    Metin,
}

impl JetonTuru {
    /// Bu türün **anlamsal token anahtarı** (MK-52); arayüz [`crate::tokens::Tokenlar`] ile çözer.
    pub fn token_anahtari(self) -> &'static str {
        match self {
            JetonTuru::AnahtarKelime => "accent.primary",
            JetonTuru::Dize => "success",
            JetonTuru::Yorum => "text.muted",
            JetonTuru::Sayi => "info",
            JetonTuru::Operator => "warning",
            JetonTuru::Noktalama => "text.muted",
            JetonTuru::Tanimlayici | JetonTuru::Metin => "text.primary",
        }
    }
}

/// Bir satır içindeki tek bir jeton (bayt aralığı + tür).  Aralık char sınırlarında olur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Jeton {
    /// Jeton türü.
    pub tur: JetonTuru,
    /// Satır içindeki başlangıç bayt ofseti (dahil).
    pub baslangic: usize,
    /// Satır içindeki bitiş bayt ofseti (hariç).
    pub son: usize,
}

/// Bir satırın **giriş/çıkış** durumu — çok-satırlı yapılar (üç-tırnak dize) için.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SatirDurumu {
    /// Normal (çok-satırlı bir yapının ortasında değil).
    #[default]
    Normal,
    /// `'''` üç-tırnak dizesinin içinde.
    UcTirnakTek,
    /// `"""` üç-tırnak dizesinin içinde.
    UcTirnakCift,
}

/// Bir satırı jetonlara ayıran vurgulayıcı.  İleride Tree-sitter tabanlı uygulama da bu
/// trait'i sağlayabilir (v1.x) — arayüz değişmeden takılır.
pub trait Vurgulayici: Send + Sync {
    /// `satir`'ı `giris` durumuyla jetonlar; (jetonlar, **çıkış durumu**) döner.
    ///
    /// Jetonlar **bitişiktir** ve satırın tüm baytlarını (`[0, satir.len())`) kapsar —
    /// böylece arayüz satırı boşluksuz, renkli olarak yeniden kurabilir.
    fn satir(&self, satir: &str, dil: KodDili, giris: SatirDurumu) -> (Vec<Jeton>, SatirDurumu);
}

/// Saf-Rust temel vurgulayıcı (durumsuz; tüm durum argümanlarda taşınır).
#[derive(Debug, Clone, Copy, Default)]
pub struct BasitVurgulayici;

impl Vurgulayici for BasitVurgulayici {
    fn satir(&self, satir: &str, dil: KodDili, giris: SatirDurumu) -> (Vec<Jeton>, SatirDurumu) {
        let mut jetonlar: Vec<Jeton> = Vec::new();
        let n = satir.len();

        // 1) Bir üç-tırnak dizesinin ORTASINDA başlıyorsak: kapanışı bu satırda ara.
        if let Some(kapanis) = devam_eden_uc_tirnak(giris) {
            if let Some(bitis) = satir.find(kapanis) {
                let son = bitis + kapanis.len();
                jetonlar.push(Jeton {
                    tur: JetonTuru::Dize,
                    baslangic: 0,
                    son,
                });
                // Kapanıştan sonrası normal olarak taranır.
                jetonla_normal(&satir[son..], son, dil, &mut jetonlar);
                return (jetonlar, SatirDurumu::Normal);
            }
            // Kapanış yok → tüm satır dize, durum devam eder.
            if n > 0 {
                jetonlar.push(Jeton {
                    tur: JetonTuru::Dize,
                    baslangic: 0,
                    son: n,
                });
            }
            return (jetonlar, giris);
        }

        // 2) Normal tarama (boş satır → boş jeton listesi; arayüz boş satırı zaten kapsar).
        let cikis = jetonla_normal_durumlu(satir, dil, &mut jetonlar);
        (jetonlar, cikis)
    }
}

/// Giriş durumu bir üç-tırnak dizesi ise aranacak kapanış dizgesini döner.
fn devam_eden_uc_tirnak(durum: SatirDurumu) -> Option<&'static str> {
    match durum {
        SatirDurumu::UcTirnakTek => Some("'''"),
        SatirDurumu::UcTirnakCift => Some("\"\"\""),
        SatirDurumu::Normal => None,
    }
}

/// Normal (durum taşımayan) bir parçayı `ofset` kaydırarak jetonlar — kapanış sonrası kuyruk için.
fn jetonla_normal(parca: &str, ofset: usize, dil: KodDili, cikti: &mut Vec<Jeton>) {
    let mut alt = Vec::new();
    jetonla_normal_durumlu(parca, dil, &mut alt);
    for j in alt {
        cikti.push(Jeton {
            tur: j.tur,
            baslangic: j.baslangic + ofset,
            son: j.son + ofset,
        });
    }
}

/// Bir satırı normal kurallarla jetonlar; üç-tırnak açılırsa **çıkış durumunu** döner.
fn jetonla_normal_durumlu(satir: &str, dil: KodDili, cikti: &mut Vec<Jeton>) -> SatirDurumu {
    let yorum = dil.yorum_oneki();
    let karakterler: Vec<(usize, char)> = satir.char_indices().collect();
    let mut i = 0usize;
    let son_bayt = satir.len();

    while i < karakterler.len() {
        let (bas, c) = karakterler[i];

        // Boşluk → Metin jetonu (bitişikliği korur).
        if c.is_whitespace() {
            let mut j = i + 1;
            while j < karakterler.len() && karakterler[j].1.is_whitespace() {
                j += 1;
            }
            let son = bayt_sonu(&karakterler, j, son_bayt);
            cikti.push(Jeton {
                tur: JetonTuru::Metin,
                baslangic: bas,
                son,
            });
            i = j;
            continue;
        }

        // Yorum → satır sonuna kadar.
        if let Some(onek) = yorum {
            if satir[bas..].starts_with(onek) {
                cikti.push(Jeton {
                    tur: JetonTuru::Yorum,
                    baslangic: bas,
                    son: son_bayt,
                });
                return SatirDurumu::Normal;
            }
        }

        // Üç-tırnak dize açılışı (Python).
        if dil.uc_tirnak_var() {
            if satir[bas..].starts_with("\"\"\"") {
                return uc_tirnak_ac(satir, bas, "\"\"\"", SatirDurumu::UcTirnakCift, dil, cikti);
            }
            if satir[bas..].starts_with("'''") {
                return uc_tirnak_ac(satir, bas, "'''", SatirDurumu::UcTirnakTek, dil, cikti);
            }
        }

        // Tek-satır dize ("..." veya '...').
        if c == '"' || c == '\'' {
            let son = tek_satir_dize_sonu(&karakterler, i, c, son_bayt);
            cikti.push(Jeton {
                tur: JetonTuru::Dize,
                baslangic: bas,
                son,
            });
            i = char_indeksi_ileri(&karakterler, son);
            continue;
        }

        // Sayı (basamakla başlar; nokta/e/x içerebilir).
        if c.is_ascii_digit() {
            let mut j = i + 1;
            while j < karakterler.len() {
                let ch = karakterler[j].1;
                if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' {
                    j += 1;
                } else {
                    break;
                }
            }
            let son = bayt_sonu(&karakterler, j, son_bayt);
            cikti.push(Jeton {
                tur: JetonTuru::Sayi,
                baslangic: bas,
                son,
            });
            i = j;
            continue;
        }

        // Tanımlayıcı / anahtar kelime (harf veya _ ile başlar).
        if c.is_alphabetic() || c == '_' {
            let mut j = i + 1;
            while j < karakterler.len() {
                let ch = karakterler[j].1;
                if ch.is_alphanumeric() || ch == '_' {
                    j += 1;
                } else {
                    break;
                }
            }
            let son = bayt_sonu(&karakterler, j, son_bayt);
            let kelime = &satir[bas..son];
            let tur = if dil.anahtar_kelimeler().contains(&kelime) {
                JetonTuru::AnahtarKelime
            } else {
                JetonTuru::Tanimlayici
            };
            cikti.push(Jeton {
                tur,
                baslangic: bas,
                son,
            });
            i = j;
            continue;
        }

        // Operatör mü, noktalama mı?
        let son = bayt_sonu(&karakterler, i + 1, son_bayt);
        let tur = if "+-*/%=<>!&|^~".contains(c) {
            JetonTuru::Operator
        } else {
            JetonTuru::Noktalama
        };
        cikti.push(Jeton {
            tur,
            baslangic: bas,
            son,
        });
        i += 1;
    }

    SatirDurumu::Normal
}

/// Üç-tırnak dize açılışı: bu satırda kapanırsa Normal, kapanmazsa açık durumu döner.
fn uc_tirnak_ac(
    satir: &str,
    bas: usize,
    dizge: &str,
    acik_durum: SatirDurumu,
    dil: KodDili,
    cikti: &mut Vec<Jeton>,
) -> SatirDurumu {
    let icerik_bas = bas + dizge.len();
    if let Some(rel) = satir[icerik_bas..].find(dizge) {
        // Aynı satırda kapandı → tek dize jetonu, kuyruğu normal tara (örn. `x = """a""" + 1`).
        let son = icerik_bas + rel + dizge.len();
        cikti.push(Jeton {
            tur: JetonTuru::Dize,
            baslangic: bas,
            son,
        });
        jetonla_normal(&satir[son..], son, dil, cikti);
        SatirDurumu::Normal
    } else {
        // Satır sonuna kadar dize; sonraki satıra taşar.
        cikti.push(Jeton {
            tur: JetonTuru::Dize,
            baslangic: bas,
            son: satir.len(),
        });
        acik_durum
    }
}

/// Tek-satır dizenin bitiş bayt ofsetini bulur (kapanış tırnağı dahil; kaçış `\` atlanır).
fn tek_satir_dize_sonu(
    karakterler: &[(usize, char)],
    baslangic_idx: usize,
    tirnak: char,
    son_bayt: usize,
) -> usize {
    let mut j = baslangic_idx + 1;
    while j < karakterler.len() {
        let ch = karakterler[j].1;
        if ch == '\\' {
            j += 2; // kaçış → bir sonraki karakteri atla
            continue;
        }
        if ch == tirnak {
            return bayt_sonu(karakterler, j + 1, son_bayt);
        }
        j += 1;
    }
    son_bayt // kapanmadıysa satır sonuna kadar
}

/// `karakterler` dizisinde `idx`'inci karakterin başlangıç baytını (yoksa `son_bayt`) verir.
fn bayt_sonu(karakterler: &[(usize, char)], idx: usize, son_bayt: usize) -> usize {
    karakterler.get(idx).map(|(b, _)| *b).unwrap_or(son_bayt)
}

/// Verilen bayt ofsetine karşılık gelen karakter indeksini ileriye doğru bulur.
fn char_indeksi_ileri(karakterler: &[(usize, char)], bayt: usize) -> usize {
    karakterler
        .iter()
        .position(|(b, _)| *b >= bayt)
        .unwrap_or(karakterler.len())
}

// ─── Artımlı önbellek ─────────────────────────────────────────────────────────

/// Tek bir satırın önbelleğe alınmış vurgusu.
#[derive(Debug, Clone)]
struct SatirVurgu {
    icerik_hash: u64,
    giris: SatirDurumu,
    cikis: SatirDurumu,
    jetonlar: Vec<Jeton>,
}

/// **Artımlı** vurgulama önbelleği.  Yalnız içeriği veya giriş durumu değişen satırlar
/// yeniden jetonlanır → büyük dosyada bir satır düzenlemek tüm dosyayı yeniden taramaz.
#[derive(Debug, Default)]
pub struct VurgulamaOnbellek {
    satirlar: Vec<SatirVurgu>,
    son_yeniden: usize,
}

impl VurgulamaOnbellek {
    /// Boş önbellek.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Metni satırlara böler ve yalnız **değişen** satırları yeniden jetonlar.
    ///
    /// Çok-satırlı bir yapı (üç-tırnak) bir satırın çıkış durumunu değiştirirse, aşağıdaki
    /// satırlar giriş durumları değiştiği için kendiliğinden yeniden hesaplanır — ta ki
    /// durum yeniden örtüşene kadar (doğru + verimli).
    pub fn guncelle(&mut self, metin: &str, dil: KodDili, v: &dyn Vurgulayici) {
        let satirlar: Vec<&str> = metin.split('\n').collect();
        let mut yeniden = 0usize;
        let mut durum = SatirDurumu::Normal;

        for (i, satir) in satirlar.iter().enumerate() {
            let h = hash_satir(satir);
            let yeniden_gerek = match self.satirlar.get(i) {
                Some(s) => s.icerik_hash != h || s.giris != durum,
                None => true,
            };

            if yeniden_gerek {
                let (jetonlar, cikis) = v.satir(satir, dil, durum);
                let yeni = SatirVurgu {
                    icerik_hash: h,
                    giris: durum,
                    cikis,
                    jetonlar,
                };
                if i < self.satirlar.len() {
                    self.satirlar[i] = yeni;
                } else {
                    self.satirlar.push(yeni);
                }
                yeniden += 1;
            }
            durum = self.satirlar[i].cikis;
        }

        // Fazla (silinmiş) satırları at.
        self.satirlar.truncate(satirlar.len());
        self.son_yeniden = yeniden;
    }

    /// `i`. satırın jetonları (yoksa boş dilim).
    pub fn satir_jetonlari(&self, i: usize) -> &[Jeton] {
        self.satirlar
            .get(i)
            .map(|s| s.jetonlar.as_slice())
            .unwrap_or(&[])
    }

    /// Önbellekteki satır sayısı.
    pub fn satir_sayisi(&self) -> usize {
        self.satirlar.len()
    }

    /// Son [`guncelle`](Self::guncelle) çağrısında kaç satırın yeniden hesaplandığı (artımlılık testi).
    pub fn son_yeniden_sayisi(&self) -> usize {
        self.son_yeniden
    }
}

/// Bir satır içeriğinin hızlı hash'i (önbellek anahtarı).
fn hash_satir(satir: &str) -> u64 {
    let mut h = DefaultHasher::new();
    satir.hash(&mut h);
    h.finish()
}

/// Tek seferlik kolaylık: bir satırı temel vurgulayıcıyla jetonlar (durumsuz çağrı).
pub fn vurgula(satir: &str, dil: KodDili) -> Vec<Jeton> {
    BasitVurgulayici.satir(satir, dil, SatirDurumu::Normal).0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turler(jetonlar: &[Jeton]) -> Vec<JetonTuru> {
        jetonlar.iter().map(|j| j.tur).collect()
    }

    /// Jetonlar bitişik olmalı ve tüm satırı kapsamalı (arayüz boşluksuz kurabilsin).
    fn bitisik_ve_tam(satir: &str, jetonlar: &[Jeton]) {
        let mut beklenen = 0usize;
        for j in jetonlar {
            assert_eq!(j.baslangic, beklenen, "jeton boşluğu: {jetonlar:?}");
            assert!(j.son <= satir.len());
            beklenen = j.son;
        }
        if !satir.is_empty() {
            assert_eq!(beklenen, satir.len(), "satır sonuna kadar kapsanmalı");
        }
    }

    #[test]
    fn uzantidan_dil() {
        assert_eq!(KodDili::uzantidan("py"), KodDili::Python);
        assert_eq!(KodDili::uzantidan(".PY"), KodDili::Python);
        assert_eq!(KodDili::uzantidan("json"), KodDili::Json);
        assert_eq!(KodDili::uzantidan("yml"), KodDili::Yaml);
        assert_eq!(KodDili::uzantidan("ron"), KodDili::Ron);
        assert_eq!(KodDili::uzantidan("bin"), KodDili::Duz);
    }

    #[test]
    fn python_anahtar_dize_yorum_sayi() {
        let satir = "def f(x):  # not\n";
        let satir = satir.trim_end();
        let (j, durum) = BasitVurgulayici.satir(satir, KodDili::Python, SatirDurumu::Normal);
        bitisik_ve_tam(satir, &j);
        assert_eq!(durum, SatirDurumu::Normal);
        // 'def' anahtar, 'f'/'x' tanımlayıcı, yorum sonda.
        let t = turler(&j);
        assert!(t.contains(&JetonTuru::AnahtarKelime));
        assert!(t.contains(&JetonTuru::Yorum));
        // İlk jeton 'def' anahtar kelime.
        assert_eq!(j[0].tur, JetonTuru::AnahtarKelime);
        assert_eq!(&satir[j[0].baslangic..j[0].son], "def");
    }

    #[test]
    fn dize_ve_sayi() {
        let satir = "x = \"abc\" + 42";
        let (j, _) = BasitVurgulayici.satir(satir, KodDili::Python, SatirDurumu::Normal);
        bitisik_ve_tam(satir, &j);
        let t = turler(&j);
        assert!(t.contains(&JetonTuru::Dize));
        assert!(t.contains(&JetonTuru::Sayi));
        assert!(t.contains(&JetonTuru::Operator)); // '=' ve '+'
    }

    #[test]
    fn uc_tirnak_cok_satir_durumu_tasir() {
        // Açılış satırı → UcTirnakCift durumu taşır.
        let (_j1, d1) =
            BasitVurgulayici.satir("s = \"\"\"başla", KodDili::Python, SatirDurumu::Normal);
        assert_eq!(d1, SatirDurumu::UcTirnakCift);
        // Orta satır → hâlâ açık.
        let (j2, d2) = BasitVurgulayici.satir("orta satır", KodDili::Python, d1);
        assert_eq!(d2, SatirDurumu::UcTirnakCift);
        assert_eq!(j2[0].tur, JetonTuru::Dize);
        // Kapanış satırı → Normal'e döner.
        let (_j3, d3) = BasitVurgulayici.satir("bitti\"\"\" + x", KodDili::Python, d2);
        assert_eq!(d3, SatirDurumu::Normal);
    }

    #[test]
    fn json_yorumsuz() {
        // JSON'da '#' yorum DEĞİLDİR (noktalama/operatör olarak ele alınır, asla Yorum değil).
        let satir = "{\"a\": 1}  # x";
        let (j, _) = BasitVurgulayici.satir(satir, KodDili::Json, SatirDurumu::Normal);
        assert!(!turler(&j).contains(&JetonTuru::Yorum));
    }

    #[test]
    fn onbellek_artimli_yalniz_degisen_satiri_yeniden_hesaplar() {
        let mut o = VurgulamaOnbellek::yeni();
        let metin = "a = 1\nb = 2\nc = 3\nd = 4\n";
        o.guncelle(metin, KodDili::Python, &BasitVurgulayici);
        // İlk geçişte tüm satırlar (5: son '\n' sonrası boş satır dahil) hesaplanır.
        assert_eq!(o.son_yeniden_sayisi(), 5);

        // Tek satırı değiştir → yalnız o satır yeniden hesaplanmalı (artımlı).
        let metin2 = "a = 1\nb = 22\nc = 3\nd = 4\n";
        o.guncelle(metin2, KodDili::Python, &BasitVurgulayici);
        assert_eq!(
            o.son_yeniden_sayisi(),
            1,
            "yalnız değişen satır yeniden hesaplanmalı"
        );

        // Hiç değişmezse 0 yeniden hesap.
        o.guncelle(metin2, KodDili::Python, &BasitVurgulayici);
        assert_eq!(o.son_yeniden_sayisi(), 0);
    }

    #[test]
    fn onbellek_uc_tirnak_alt_satirlari_yeniden_tetikler() {
        let mut o = VurgulamaOnbellek::yeni();
        let metin = "x = 1\ny = 2\nz = 3\n";
        o.guncelle(metin, KodDili::Python, &BasitVurgulayici);
        // İlk satıra açık üç-tırnak ekle → alt satırların giriş durumu değişir → yeniden hesap.
        let metin2 = "x = \"\"\"açık\ny = 2\nz = 3\n";
        o.guncelle(metin2, KodDili::Python, &BasitVurgulayici);
        // 1. satır (değişti) + altındakiler (giriş durumu değişti) yeniden hesaplanır.
        assert!(o.son_yeniden_sayisi() >= 2);
        // 2. satır artık dize içinde → ilk jetonu Dize.
        assert_eq!(o.satir_jetonlari(1)[0].tur, JetonTuru::Dize);
    }

    #[test]
    fn token_anahtarlari_anlamsal() {
        // MK-52: her tür anlamsal bir anahtar verir (sabit RGB yok).
        assert_eq!(JetonTuru::AnahtarKelime.token_anahtari(), "accent.primary");
        assert_eq!(JetonTuru::Dize.token_anahtari(), "success");
        assert_eq!(JetonTuru::Yorum.token_anahtari(), "text.muted");
    }

    #[test]
    fn turkce_tanimlayici() {
        // Unicode harfli tanımlayıcı (Türkçe) bölünmeden tek jeton olmalı.
        let satir = "değişken = 5";
        let (j, _) = BasitVurgulayici.satir(satir, KodDili::Python, SatirDurumu::Normal);
        bitisik_ve_tam(satir, &j);
        assert_eq!(&satir[j[0].baslangic..j[0].son], "değişken");
        assert_eq!(j[0].tur, JetonTuru::Tanimlayici);
    }

    #[test]
    fn bos_satir_panik_yok() {
        let (j, d) = BasitVurgulayici.satir("", KodDili::Python, SatirDurumu::Normal);
        assert!(j.is_empty());
        assert_eq!(d, SatirDurumu::Normal);
    }
}
