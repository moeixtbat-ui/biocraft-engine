//! Klavye **kısayol sistemi** — özelleştirilebilir bağlamalar + çakışma tespiti (İP-13).
//!
//! Bir kısayol = (değiştiriciler [Ctrl/Shift/Alt/Cmd]) + (tek tuş).  Tuş, hem egui olaylarından
//! hem de insan-yazımı metinden ("Ctrl+Shift+P") **kanonik bir belirteç**e indirgenir (örn. "p",
//! "comma", "backslash", "f5") → iki yol da aynı `Kisayol`'a çözülür (eşleşme tutarlı).
//!
//! Varsayılan set **tek kaynaktan** gelir: her [`KabukAksiyon`]'un kendi `kisayol()` ipucu
//! (`keymap_profile`).  Kullanıcı yeniden atar; **çakışma** (aynı kombinasyon iki aksiyonda)
//! tespit edilip uyarılır; tek tek veya tümden **varsayılana dönülür**; değişiklikler
//! kalıcı katmana (override) yazılır.  Renkler token'dan, tüm eylemler klavyeyle (MK-52).

use std::collections::BTreeMap;

use crate::i18n::Dil;
use crate::tokens::Tokenlar;

use super::keymap_profile::{varsayilan_harita, TusSetiProfili};
use super::KomutKaynak;

/// Bir kısayolun değiştirici (modifier) tuşları.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Degistiriciler {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    /// macOS ⌘ / "Command" (Windows/Linux'ta genelde kullanılmaz).
    pub cmd: bool,
}

impl Degistiriciler {
    /// Bir "hızlandırıcı" (accelerator) mı — yani Ctrl/Alt/Cmd içeriyor mu?  Sade harf/Shift
    /// kombinasyonları metin girişiyle çakışmasın diye global gönderimde bu ayırt edicidir.
    pub fn hizlandirici_mi(self) -> bool {
        self.ctrl || self.alt || self.cmd
    }
}

/// Tek bir klavye kısayolu (kombinasyon).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Kisayol {
    pub degistiriciler: Degistiriciler,
    /// Kanonik tuş belirteci ("p", "1", "f5", "comma", "backslash", "up", "enter"…).
    pub tus: String,
}

impl Kisayol {
    /// İnsan-yazımı bir dizgeden ("Ctrl+Shift+P", "Ctrl+,", "Ctrl+\\") ayrıştırır.
    pub fn ayristir(s: &str) -> Option<Kisayol> {
        let raw = s.trim();
        if raw.is_empty() {
            return None;
        }
        // "+" tuşunun kendisi son karakterse (örn. "Ctrl++") onu ayırıcıdan ayırt et.
        let artili_son = raw.ends_with('+') && raw.len() > 1;
        let govde = if artili_son {
            &raw[..raw.len() - 1]
        } else {
            raw
        };
        let mut parcalar: Vec<&str> = govde
            .split('+')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect();
        let tus_ham = if artili_son {
            "+".to_string()
        } else {
            parcalar.pop()?.to_string()
        };
        let mut d = Degistiriciler::default();
        for p in parcalar {
            match p.to_lowercase().as_str() {
                "ctrl" | "control" | "ctl" | "^" => d.ctrl = true,
                "shift" | "⇧" => d.shift = true,
                "alt" | "option" | "opt" | "⌥" => d.alt = true,
                "cmd" | "command" | "super" | "meta" | "win" | "⌘" => d.cmd = true,
                _ => return None, // tanınmayan değiştirici → geçersiz
            }
        }
        let tus = kanonik_insan(&tus_ham)?;
        Some(Kisayol {
            degistiriciler: d,
            tus,
        })
    }

    /// Bir egui klavye olayından kısayol kurar (gönderim için).
    pub fn egui_olaydan(tus: egui::Key, m: egui::Modifiers) -> Option<Kisayol> {
        let belirtec = kanonik_egui(tus)?;
        Some(Kisayol {
            degistiriciler: Degistiriciler {
                ctrl: m.ctrl || m.command, // Windows/Linux: command=ctrl
                shift: m.shift,
                alt: m.alt,
                cmd: m.mac_cmd,
            },
            tus: belirtec,
        })
    }

    /// Kalıcılık için kanonik, **yeniden-ayrıştırılabilir** dizge ("ctrl+shift+p", "ctrl+comma").
    pub fn seri(&self) -> String {
        let mut s = String::new();
        let d = self.degistiriciler;
        if d.ctrl {
            s.push_str("ctrl+");
        }
        if d.alt {
            s.push_str("alt+");
        }
        if d.shift {
            s.push_str("shift+");
        }
        if d.cmd {
            s.push_str("cmd+");
        }
        s.push_str(&self.tus);
        s
    }

    /// Kalıcı dizgeden geri kurar (`seri`'nin tersi).
    pub fn seriden(s: &str) -> Option<Kisayol> {
        Kisayol::ayristir(s)
    }

    /// Kullanıcıya gösterilen okunabilir biçim ("Ctrl+Shift+P", "Ctrl+,", "Ctrl+\\").
    pub fn goster(&self) -> String {
        let mut s = String::new();
        let d = self.degistiriciler;
        if d.ctrl {
            s.push_str("Ctrl+");
        }
        if d.alt {
            s.push_str("Alt+");
        }
        if d.shift {
            s.push_str("Shift+");
        }
        if d.cmd {
            s.push_str("Cmd+");
        }
        s.push_str(&tus_goster(&self.tus));
        s
    }
}

/// egui `Key` → kanonik belirteç (varyant Debug adını normalize eder; egui minor sürümlerine dayanıklı).
fn kanonik_egui(key: egui::Key) -> Option<String> {
    kanonik_debug(&format!("{key:?}"))
}

/// Varyant Debug adını ("A", "Num1", "Comma", "ArrowUp", "F5") kanonik belirtece indirger.
fn kanonik_debug(ad: &str) -> Option<String> {
    if ad.len() == 1 {
        let c = ad.chars().next().unwrap();
        if c.is_ascii_alphanumeric() {
            return Some(c.to_ascii_lowercase().to_string());
        }
    }
    if let Some(n) = ad.strip_prefix("Num") {
        if n.len() == 1 && n.chars().all(|c| c.is_ascii_digit()) {
            return Some(n.to_string());
        }
    }
    if let Some(n) = ad.strip_prefix('F') {
        if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) {
            return Some(format!("f{n}"));
        }
    }
    let t = match ad {
        "Comma" => "comma",
        "Period" => "period",
        "Semicolon" => "semicolon",
        "Colon" => "colon",
        "Backslash" => "backslash",
        "Slash" => "slash",
        "Minus" => "minus",
        "Plus" | "PlusEquals" => "plus",
        "Equals" => "equals",
        "Backtick" => "backtick",
        "Questionmark" => "questionmark",
        "Pipe" => "pipe",
        "OpenBracket" => "openbracket",
        "CloseBracket" => "closebracket",
        "ArrowUp" => "up",
        "ArrowDown" => "down",
        "ArrowLeft" => "left",
        "ArrowRight" => "right",
        "Enter" => "enter",
        "Escape" => "escape",
        "Tab" => "tab",
        "Space" => "space",
        "Backspace" => "backspace",
        "Delete" => "delete",
        "Insert" => "insert",
        "Home" => "home",
        "End" => "end",
        "PageUp" => "pageup",
        "PageDown" => "pagedown",
        _ => return None,
    };
    Some(t.to_string())
}

/// İnsan-yazımı bir tuş belirtecini ("P", ",", "\\", "F5", "Up", "Enter") kanonik biçime indirger.
fn kanonik_insan(s: &str) -> Option<String> {
    let s = s.trim();
    let mut ch = s.chars();
    if let (Some(c), None) = (ch.next(), ch.clone().next()) {
        // tek karakter
        if c.is_ascii_alphanumeric() {
            return Some(c.to_ascii_lowercase().to_string());
        }
        let t = match c {
            ',' => "comma",
            '.' => "period",
            ';' => "semicolon",
            ':' => "colon",
            '\\' => "backslash",
            '/' => "slash",
            '-' => "minus",
            '+' => "plus",
            '=' => "equals",
            '`' => "backtick",
            '?' => "questionmark",
            '|' => "pipe",
            '[' => "openbracket",
            ']' => "closebracket",
            _ => return None,
        };
        return Some(t.to_string());
    }
    let l = s.to_lowercase();
    // F-tuşları
    if let Some(n) = l.strip_prefix('f') {
        if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) {
            return Some(format!("f{n}"));
        }
    }
    let t = match l.as_str() {
        "comma" => "comma",
        "period" | "dot" => "period",
        "semicolon" => "semicolon",
        "colon" => "colon",
        "backslash" => "backslash",
        "slash" => "slash",
        "minus" | "dash" => "minus",
        "plus" => "plus",
        "equals" | "equal" => "equals",
        "up" | "arrowup" | "↑" => "up",
        "down" | "arrowdown" | "↓" => "down",
        "left" | "arrowleft" | "←" => "left",
        "right" | "arrowright" | "→" => "right",
        "enter" | "return" => "enter",
        "esc" | "escape" => "escape",
        "tab" => "tab",
        "space" | "spacebar" => "space",
        "backspace" => "backspace",
        "del" | "delete" => "delete",
        "ins" | "insert" => "insert",
        "home" => "home",
        "end" => "end",
        "pgup" | "pageup" => "pageup",
        "pgdn" | "pagedown" => "pagedown",
        _ => return None,
    };
    Some(t.to_string())
}

/// Kanonik belirteci kullanıcıya gösterilecek okunabilir biçime çevirir.
fn tus_goster(tus: &str) -> String {
    if tus.len() == 1 {
        let c = tus.chars().next().unwrap();
        if c.is_ascii_alphabetic() {
            return c.to_ascii_uppercase().to_string();
        }
        return tus.to_string();
    }
    if let Some(n) = tus.strip_prefix('f') {
        if n.chars().all(|c| c.is_ascii_digit()) {
            return format!("F{n}");
        }
    }
    let g = match tus {
        "comma" => ",",
        "period" => ".",
        "semicolon" => ";",
        "colon" => ":",
        "backslash" => "\\",
        "slash" => "/",
        "minus" => "-",
        "plus" => "+",
        "equals" => "=",
        "backtick" => "`",
        "questionmark" => "?",
        "pipe" => "|",
        "openbracket" => "[",
        "closebracket" => "]",
        "up" => "↑",
        "down" => "↓",
        "left" => "←",
        "right" => "→",
        "enter" => "Enter",
        "escape" => "Esc",
        "tab" => "Tab",
        "space" => "Space",
        "backspace" => "Backspace",
        "delete" => "Delete",
        "insert" => "Insert",
        "home" => "Home",
        "end" => "End",
        "pageup" => "PgUp",
        "pagedown" => "PgDn",
        diger => return diger.to_uppercase(),
    };
    g.to_string()
}

/// İki veya daha fazla aksiyonun **aynı** kısayola bağlanması (çakışma).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cakisma {
    pub kisayol: Kisayol,
    pub aksiyonlar: Vec<KomutKaynak>,
}

/// Aksiyon → kısayol bağlamalarının düzenlenebilir haritası.
#[derive(Debug, Clone)]
pub struct KisayolHaritasi {
    profil: TusSetiProfili,
    baglamalar: BTreeMap<KomutKaynak, Kisayol>,
    kirli: bool,
}

impl KisayolHaritasi {
    /// Bir profilin varsayılan setiyle başlar.
    pub fn varsayilan(profil: TusSetiProfili) -> Self {
        Self {
            profil,
            baglamalar: varsayilan_harita(profil),
            kirli: false,
        }
    }

    /// Aktif tuş seti profili.
    pub fn profil(&self) -> TusSetiProfili {
        self.profil
    }

    /// Profili değiştir → o profilin varsayılan setini yükler (kullanıcı override'ları sıfırlanır).
    pub fn profil_degistir(&mut self, profil: TusSetiProfili) {
        self.profil = profil;
        self.baglamalar = varsayilan_harita(profil);
        self.kirli = true;
    }

    /// Bir aksiyonun atanmış kısayolu (yoksa `None`).
    pub fn kisayol(&self, kaynak: &KomutKaynak) -> Option<&Kisayol> {
        self.baglamalar.get(kaynak)
    }

    /// Bir aksiyona kısayol atar; **aynı** kombinasyonu zaten kullanan DİĞER aksiyonları döndürür
    /// (çakışma uyarısı için).  Atama yine de yapılır (kullanıcı bilinçli ezebilir — VS Code gibi).
    pub fn ata(&mut self, kaynak: KomutKaynak, ks: Kisayol) -> Vec<KomutKaynak> {
        let cakisanlar: Vec<KomutKaynak> = self
            .baglamalar
            .iter()
            .filter(|(k, v)| **v == ks && **k != kaynak)
            .map(|(k, _)| k.clone())
            .collect();
        self.baglamalar.insert(kaynak, ks);
        self.kirli = true;
        cakisanlar
    }

    /// Bir aksiyonun kısayolunu kaldırır (bağlanmamış yapar).
    pub fn kaldir(&mut self, kaynak: &KomutKaynak) {
        if self.baglamalar.remove(kaynak).is_some() {
            self.kirli = true;
        }
    }

    /// Bir aksiyonu profil varsayılanına döndürür (varsayılanı yoksa bağlamayı kaldırır).
    pub fn varsayilana_don(&mut self, kaynak: &KomutKaynak) {
        let varsayilan = varsayilan_harita(self.profil);
        match varsayilan.get(kaynak) {
            Some(ks) => {
                self.baglamalar.insert(kaynak.clone(), ks.clone());
            }
            None => {
                self.baglamalar.remove(kaynak);
            }
        }
        self.kirli = true;
    }

    /// Tüm bağlamaları profil varsayılanına döndürür.
    pub fn tumunu_varsayilana_don(&mut self) {
        self.baglamalar = varsayilan_harita(self.profil);
        self.kirli = true;
    }

    /// Tüm çakışmalar (aynı kombinasyona bağlı ≥2 aksiyon) — referans/uyarı için.
    pub fn cakismalar(&self) -> Vec<Cakisma> {
        let mut grup: BTreeMap<String, (Kisayol, Vec<KomutKaynak>)> = BTreeMap::new();
        for (k, ks) in &self.baglamalar {
            let e = grup
                .entry(ks.seri())
                .or_insert_with(|| (ks.clone(), Vec::new()));
            e.1.push(k.clone());
        }
        grup.into_values()
            .filter(|(_, v)| v.len() >= 2)
            .map(|(kisayol, aksiyonlar)| Cakisma {
                kisayol,
                aksiyonlar,
            })
            .collect()
    }

    /// Bir aksiyonun kısayolu başka bir aksiyonla çakışıyor mu?
    pub fn cakisiyor_mu(&self, kaynak: &KomutKaynak) -> bool {
        let Some(ks) = self.baglamalar.get(kaynak) else {
            return false;
        };
        self.baglamalar.iter().any(|(k, v)| v == ks && k != kaynak)
    }

    /// Basılan bir kombinasyonu aksiyona **çözer** (klavye gönderimi için).  Çakışma varsa
    /// kararlı sırada ilki seçilir (uyarı kullanıcıyı düzeltmeye yöneltir).
    pub fn cozumle(&self, ks: &Kisayol) -> Option<KomutKaynak> {
        self.baglamalar
            .iter()
            .find(|(_, v)| *v == ks)
            .map(|(k, _)| k.clone())
    }

    /// Tüm bağlamalar (referans listesi; kararlı sırada).
    pub fn tumu(&self) -> impl Iterator<Item = (&KomutKaynak, &Kisayol)> {
        self.baglamalar.iter()
    }

    /// Değişiklik var mı (kalıcı katmana yazılmalı mı)?
    pub fn kirli_mi(&self) -> bool {
        self.kirli
    }

    /// Kirlilik bayrağını temizler (kalıcı katmana yazıldıktan sonra).
    pub fn kirli_temizle(&mut self) {
        self.kirli = false;
    }

    /// Profil varsayılanından **farklı** bağlamaları (override'lar) kalıcı haritaya çıkarır.
    /// Kaldırılan varsayılanlar boş dizge (`""`) ile işaretlenir → yükleyince "bağlanmamış" olur.
    pub fn override_haritasi(&self) -> BTreeMap<String, String> {
        let varsayilan = varsayilan_harita(self.profil);
        let mut farklar = BTreeMap::new();
        // Değişen/eklenenler.
        for (k, ks) in &self.baglamalar {
            if varsayilan.get(k) != Some(ks) {
                farklar.insert(k.anahtar(), ks.seri());
            }
        }
        // Kullanıcının kaldırdığı varsayılanlar.
        for k in varsayilan.keys() {
            if !self.baglamalar.contains_key(k) {
                farklar.insert(k.anahtar(), String::new());
            }
        }
        farklar
    }

    /// Override'ları (varsayılandan farklar) kalıcı **JSON** dizgesi olarak verir.
    pub fn override_json(&self) -> String {
        serde_json::to_string(&self.override_haritasi()).unwrap_or_else(|_| "{}".to_string())
    }

    /// Kalıcı **JSON** override dizgesini profil varsayılanı üzerine uygular (bozuk JSON → yok sayılır).
    pub fn override_json_uygula(&mut self, json: &str) {
        if let Ok(farklar) = serde_json::from_str::<BTreeMap<String, String>>(json) {
            self.override_uygula(&farklar);
        }
    }

    /// Override haritasını profil varsayılanı üzerine uygular (kalıcı katmandan yükleme).
    pub fn override_uygula(&mut self, farklar: &BTreeMap<String, String>) {
        self.baglamalar = varsayilan_harita(self.profil);
        for (anahtar, seri) in farklar {
            let Some(kaynak) = KomutKaynak::anahtardan(anahtar) else {
                continue; // tanınmayan aksiyon → atla (ileri/geri uyumlu)
            };
            if seri.is_empty() {
                self.baglamalar.remove(&kaynak);
            } else if let Some(ks) = Kisayol::seriden(seri) {
                self.baglamalar.insert(kaynak, ks);
            }
        }
        self.kirli = false;
    }
}

/// Kısayol düzenleme penceresinin oturum durumu (yakalama modu + arama).
#[derive(Debug, Clone, Default)]
pub struct KisayolDuzenleyici {
    /// Şu an yeni tuşu beklenen aksiyon (tuşa basılınca atanır); `None` = yakalama yok.
    pub yakalama: Option<KomutKaynak>,
    /// Referans listesini süzen arama metni.
    pub arama: String,
}

/// Kısayol düzenleme/referans penceresini çizer (yeniden ata + çakışma uyarısı + varsayılana dön).
///
/// `etiket`: bir aksiyonun yerelleştirilmiş adını verir (palet ile aynı kaynaktan).
/// `acik` kapatılırsa pencere kapanır.  Renkler token'dan; tüm satırlar klavye/fareyle erişilebilir.
#[allow(clippy::too_many_arguments)]
pub fn kisayol_penceresi(
    ctx: &egui::Context,
    acik: &mut bool,
    harita: &mut KisayolHaritasi,
    duzenleyici: &mut KisayolDuzenleyici,
    siralanmis: &[(KomutKaynak, String)],
    dil: Dil,
    tok: &Tokenlar,
) {
    let tr = matches!(dil, Dil::Tr);
    let baslik = if tr {
        "Klavye Kısayolları"
    } else {
        "Keyboard Shortcuts"
    };

    // Yakalama modu: bir sonraki tuşu al → ata.
    if let Some(kaynak) = duzenleyici.yakalama.clone() {
        // Esc → iptal.
        let iptal = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if iptal {
            duzenleyici.yakalama = None;
        } else if let Some(ks) = sonraki_kisayol_yakala(ctx) {
            harita.ata(kaynak, ks);
            duzenleyici.yakalama = None;
        }
    }

    let mut pencere_acik = *acik;
    egui::Window::new(baslik)
        .id(egui::Id::new("ip13_kisayol_penceresi"))
        .open(&mut pencere_acik)
        .default_width(460.0)
        .default_height(440.0)
        .resizable(true)
        .show(ctx, |ui| {
            // Üst: arama + tümünü varsayılana döndür.
            ui.horizontal(|ui| {
                ui.label(if tr { "Ara:" } else { "Search:" });
                ui.add(
                    egui::TextEdit::singleline(&mut duzenleyici.arama)
                        .hint_text(if tr {
                            "komut adı…"
                        } else {
                            "command name…"
                        })
                        .desired_width(180.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(if tr {
                            "Tümünü Varsayılana Döndür"
                        } else {
                            "Reset All to Default"
                        })
                        .clicked()
                    {
                        harita.tumunu_varsayilana_don();
                    }
                });
            });

            // Profil seçici (Modern; Vim/Emacs İP-13 kancası — v1.x'te etkinleşir).
            ui.horizontal(|ui| {
                ui.label(if tr { "Tuş seti:" } else { "Keymap:" });
                let mevcut = harita.profil();
                egui::ComboBox::from_id_salt("ip13_profil")
                    .selected_text(mevcut.ad(dil))
                    .show_ui(ui, |ui| {
                        for &p in TusSetiProfili::TUMU {
                            let secili = p == mevcut;
                            if ui.selectable_label(secili, p.ad(dil)).clicked() && !secili {
                                harita.profil_degistir(p);
                            }
                        }
                        // Gelecek profiller (Vim/Emacs) — dürüst, devre dışı (MK-48).
                        ui.add_enabled(
                            false,
                            egui::Button::new(if tr { "Vim (yakında)" } else { "Vim (soon)" }),
                        );
                        ui.add_enabled(
                            false,
                            egui::Button::new(if tr {
                                "Emacs (yakında)"
                            } else {
                                "Emacs (soon)"
                            }),
                        );
                    });
                if duzenleyici.yakalama.is_some() {
                    ui.colored_label(
                        tok.renk.vurgu,
                        if tr {
                            "Yeni tuşa basın…  (Esc: iptal)"
                        } else {
                            "Press the new key…  (Esc: cancel)"
                        },
                    );
                }
            });
            ui.separator();

            let sorgu = duzenleyici.arama.to_lowercase();
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("ip13_kisayol_grid")
                    .num_columns(2)
                    .striped(true)
                    .spacing(egui::vec2(12.0, 6.0))
                    .show(ui, |ui| {
                        for (kaynak, ad) in siralanmis {
                            if !sorgu.is_empty() && !ad.to_lowercase().contains(&sorgu) {
                                continue;
                            }
                            // Sol: komut adı (+ çakışma uyarısı).
                            ui.horizontal(|ui| {
                                ui.label(ad);
                                if harita.cakisiyor_mu(kaynak) {
                                    ui.colored_label(tok.renk.uyari, "⚠").on_hover_text(if tr {
                                        "Bu kısayol başka bir komutla çakışıyor"
                                    } else {
                                        "This shortcut conflicts with another command"
                                    });
                                }
                            });
                            // Sağ: mevcut kısayol + değiştir/kaldır/varsayılan.
                            ui.horizontal(|ui| {
                                let metin = harita
                                    .kisayol(kaynak)
                                    .map(|k| k.goster())
                                    .unwrap_or_else(|| "—".to_string());
                                let yakalaniyor = duzenleyici.yakalama.as_ref() == Some(kaynak);
                                let etiket = if yakalaniyor {
                                    if tr {
                                        "… bekleniyor".to_string()
                                    } else {
                                        "… waiting".to_string()
                                    }
                                } else {
                                    metin
                                };
                                if ui
                                    .add(egui::Button::new(etiket).min_size(egui::vec2(120.0, 0.0)))
                                    .on_hover_text(if tr {
                                        "Yeniden atamak için tıklayın"
                                    } else {
                                        "Click to reassign"
                                    })
                                    .clicked()
                                {
                                    duzenleyici.yakalama = Some(kaynak.clone());
                                }
                                if ui
                                    .small_button("↺")
                                    .on_hover_text(if tr {
                                        "Varsayılana dön"
                                    } else {
                                        "Reset to default"
                                    })
                                    .clicked()
                                {
                                    harita.varsayilana_don(kaynak);
                                }
                                if ui
                                    .small_button("✕")
                                    .on_hover_text(if tr { "Kaldır" } else { "Remove" })
                                    .clicked()
                                {
                                    harita.kaldir(kaynak);
                                }
                            });
                            ui.end_row();
                        }
                    });
            });
        });
    *acik = pencere_acik;
    if !*acik {
        duzenleyici.yakalama = None; // pencere kapanınca yakalamayı bırak
    }
}

/// O karede basılan ilk "gerçek" tuşu kısayola çevirir (değiştiriciler hariç bir ana tuş).
fn sonraki_kisayol_yakala(ctx: &egui::Context) -> Option<Kisayol> {
    ctx.input(|i| {
        for olay in &i.events {
            if let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = olay
            {
                if let Some(ks) = Kisayol::egui_olaydan(*key, *modifiers) {
                    return Some(ks);
                }
            }
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::menu_bar::KabukAksiyon;

    fn kabuk(a: KabukAksiyon) -> KomutKaynak {
        KomutKaynak::Kabuk(a)
    }

    #[test]
    fn ayristir_ve_goster_gidis_donus() {
        let k = Kisayol::ayristir("Ctrl+Shift+P").unwrap();
        assert!(k.degistiriciler.ctrl && k.degistiriciler.shift);
        assert_eq!(k.tus, "p");
        assert_eq!(k.goster(), "Ctrl+Shift+P");
        // Sembol tuşlar.
        assert_eq!(Kisayol::ayristir("Ctrl+,").unwrap().goster(), "Ctrl+,");
        assert_eq!(Kisayol::ayristir("Ctrl+\\").unwrap().goster(), "Ctrl+\\");
    }

    #[test]
    fn seri_gidis_donus() {
        let k = Kisayol::ayristir("Ctrl+Shift+P").unwrap();
        let geri = Kisayol::seriden(&k.seri()).unwrap();
        assert_eq!(k, geri);
    }

    #[test]
    fn varsayilan_harita_dolu() {
        let h = KisayolHaritasi::varsayilan(TusSetiProfili::Modern);
        // Komut paleti varsayılanı Ctrl+Shift+P olmalı (tek kaynaktan: KabukAksiyon::kisayol).
        let ks = h.kisayol(&kabuk(KabukAksiyon::KomutPaleti)).unwrap();
        assert_eq!(ks.goster(), "Ctrl+Shift+P");
        // Kaydet → Ctrl+S.
        assert_eq!(
            h.kisayol(&kabuk(KabukAksiyon::Kaydet)).unwrap().goster(),
            "Ctrl+S"
        );
    }

    #[test]
    fn cozumle_kombinasyonu_aksiyona_dondurur() {
        let h = KisayolHaritasi::varsayilan(TusSetiProfili::Modern);
        let ks = Kisayol::ayristir("Ctrl+S").unwrap();
        assert_eq!(h.cozumle(&ks), Some(kabuk(KabukAksiyon::Kaydet)));
    }

    #[test]
    fn cakisma_tespiti_calisir() {
        let mut h = KisayolHaritasi::varsayilan(TusSetiProfili::Modern);
        // Yeni Sekme'yi Ctrl+S'e ata → Kaydet ile çakışır.
        let ks = Kisayol::ayristir("Ctrl+S").unwrap();
        let cakisanlar = h.ata(kabuk(KabukAksiyon::YeniSekme), ks);
        assert!(cakisanlar.contains(&kabuk(KabukAksiyon::Kaydet)));
        assert!(h.cakisiyor_mu(&kabuk(KabukAksiyon::Kaydet)));
        assert!(!h.cakismalar().is_empty());
    }

    #[test]
    fn varsayilana_don_tek_ve_tum() {
        let mut h = KisayolHaritasi::varsayilan(TusSetiProfili::Modern);
        let orijinal = h.kisayol(&kabuk(KabukAksiyon::Kaydet)).cloned();
        h.ata(
            kabuk(KabukAksiyon::Kaydet),
            Kisayol::ayristir("Ctrl+Alt+K").unwrap(),
        );
        assert_ne!(h.kisayol(&kabuk(KabukAksiyon::Kaydet)).cloned(), orijinal);
        h.varsayilana_don(&kabuk(KabukAksiyon::Kaydet));
        assert_eq!(h.kisayol(&kabuk(KabukAksiyon::Kaydet)).cloned(), orijinal);
        // Tümden sıfırla.
        h.ata(
            kabuk(KabukAksiyon::Kaydet),
            Kisayol::ayristir("Ctrl+Alt+K").unwrap(),
        );
        h.tumunu_varsayilana_don();
        assert_eq!(h.kisayol(&kabuk(KabukAksiyon::Kaydet)).cloned(), orijinal);
    }

    #[test]
    fn override_gidis_donus_kalici() {
        let mut h = KisayolHaritasi::varsayilan(TusSetiProfili::Modern);
        h.ata(
            kabuk(KabukAksiyon::Kaydet),
            Kisayol::ayristir("Ctrl+Alt+K").unwrap(),
        );
        h.kaldir(&kabuk(KabukAksiyon::YeniSekme));
        let farklar = h.override_haritasi();
        // Yeni bir haritaya uygula → aynı duruma gelmeli.
        let mut h2 = KisayolHaritasi::varsayilan(TusSetiProfili::Modern);
        h2.override_uygula(&farklar);
        assert_eq!(
            h2.kisayol(&kabuk(KabukAksiyon::Kaydet)).map(|k| k.seri()),
            Some("ctrl+alt+k".to_string())
        );
        assert!(h2.kisayol(&kabuk(KabukAksiyon::YeniSekme)).is_none());
    }

    #[test]
    fn eklenti_kisayolu_atanabilir() {
        // Eklenti komutuna kısayol atanabilir + kalıcılaşır (İP-07 uzantı noktası).
        let mut h = KisayolHaritasi::varsayilan(TusSetiProfili::Modern);
        let ek = KomutKaynak::Eklenti("biocraft.ornek.selam".into());
        h.ata(ek.clone(), Kisayol::ayristir("Ctrl+Alt+G").unwrap());
        assert_eq!(
            h.cozumle(&Kisayol::ayristir("Ctrl+Alt+G").unwrap()),
            Some(ek.clone())
        );
        let farklar = h.override_haritasi();
        let mut h2 = KisayolHaritasi::varsayilan(TusSetiProfili::Modern);
        h2.override_uygula(&farklar);
        assert_eq!(h2.kisayol(&ek).map(|k| k.seri()), Some("ctrl+alt+g".into()));
    }
}
