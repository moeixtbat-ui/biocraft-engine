//! **Komut paleti** (İP-13) — Ctrl+Shift+P; bulanık arama (<50 ms); son/sık kullanılanlar üstte.
//!
//! Palet, kabuk komutlarını (menüyle **aynı** [`KabukAksiyon`] tanımı — MK-51) ve eklenti
//! komutlarını tek listede aranabilir kılar.  Önek modları: `>` komut, `@` sembol (temel); öneksiz
//! "hızlı geçiş" (komutlar + son kullanılanlar).  Eşleşme yoksa "şunu mu demek istediniz" önerisi.
//!
//! Tüm gezinme klavyeyle (↑/↓/Enter/Esc) + fareyle yapılabilir (MK-52); renkler token'dan.

use std::collections::HashMap;

use crate::i18n::Dil;
use crate::tokens::Tokenlar;

use super::fuzzy::{bulanik_skor, gevsek_benzerlik};
use super::{Komut, KomutKaynak};

/// Önek moduna göre paletin ne aradığı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletModu {
    /// Öneksiz: komutlar + son kullanılanlar (hızlı geçiş).
    HizliGecis,
    /// `>` — yalnızca komutlar.
    Komut,
    /// `@` — sembol (temel; aktif görünümden enjekte edilen liste).
    Sembol,
}

/// Paletten dönen kullanıcı eylemi (çiziciyi çağıran uygular).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletEylem {
    /// Bir komutu çalıştır (kabuk veya eklenti — tek tanım).
    Calistir(KomutKaynak),
    /// Bir sembole git (temel `@` modu).
    SemboleGit(String),
}

/// Komut paleti durumu (oturum boyunca yaşar; son/sık kullanım belleği taşır).
#[derive(Debug, Default)]
pub struct KomutPaleti {
    /// Palet açık mı?
    pub acik: bool,
    sorgu: String,
    secili: usize,
    /// O an gösterilen komut kümesi (palet açılırken tazelenir).
    komutlar: Vec<Komut>,
    /// `@` modu için enjekte edilen semboller (aktif görünümden; boşsa dürüst boş-durum).
    semboller: Vec<String>,
    /// Son kullanılan komutlar (en yeni başta; sınırlı).
    son: Vec<KomutKaynak>,
    /// Sık kullanım sayacı (üst sıralama bonusu).
    sayac: HashMap<KomutKaynak, u32>,
    /// Bir sonraki karede arama kutusuna odak iste.
    odakla: bool,
}

const SON_AZAMI: usize = 8;
const AZAMI_GORUNUR: usize = 50;

impl KomutPaleti {
    /// Boş bir palet kurar.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Paleti taze bir komut kümesiyle açar (arama sıfırlanır, odak istenir).
    pub fn ac(&mut self, komutlar: Vec<Komut>) {
        self.ac_ile(komutlar, Vec::new());
    }

    /// Paleti komut kümesi + `@` sembol listesiyle açar.
    pub fn ac_ile(&mut self, komutlar: Vec<Komut>, semboller: Vec<String>) {
        self.komutlar = komutlar;
        self.semboller = semboller;
        self.sorgu.clear();
        self.secili = 0;
        self.acik = true;
        self.odakla = true;
    }

    /// Paleti kapatır.
    pub fn kapat(&mut self) {
        self.acik = false;
    }

    /// Açık↔kapalı çevirir (kapalıyken verilen komut kümesiyle açar).
    pub fn ac_kapa(&mut self, komutlar: impl FnOnce() -> Vec<Komut>) {
        if self.acik {
            self.kapat();
        } else {
            self.ac(komutlar());
        }
    }

    /// Bir komutun kullanıldığını kaydeder (son + sık) — üst sıralama için.
    pub fn kullanildi(&mut self, kaynak: &KomutKaynak) {
        *self.sayac.entry(kaynak.clone()).or_insert(0) += 1;
        self.son.retain(|k| k != kaynak);
        self.son.insert(0, kaynak.clone());
        self.son.truncate(SON_AZAMI);
    }

    /// Sorgu önekinden modu ve kalan metni çözer.
    fn modu(sorgu: &str) -> (PaletModu, &str) {
        let t = sorgu.trim_start();
        if let Some(r) = t.strip_prefix('>') {
            (PaletModu::Komut, r.trim_start())
        } else if let Some(r) = t.strip_prefix('@') {
            (PaletModu::Sembol, r.trim_start())
        } else {
            (PaletModu::HizliGecis, t)
        }
    }

    /// Sorguya göre sıralanmış aday indeksleri (komut listesindeki) + eşleşen karakter indeksleri.
    fn adaylar(&self, mod_: PaletModu, sorgu: &str) -> Vec<(usize, Vec<usize>)> {
        let mut puanli: Vec<(i32, usize, Vec<usize>)> = Vec::new();
        for (i, k) in self.komutlar.iter().enumerate() {
            if !k.etkin {
                continue; // yalnızca yapılabilir komutlar (devre dışı olanlar menüde keşfedilir)
            }
            let (skor, indeksler) = match self.komut_skoru(k, sorgu) {
                Some(v) => v,
                None => continue,
            };
            // Son/sık kullanım bonusu (özellikle boş sorguda üstte tutar).
            let bonus = self.kullanim_bonusu(&k.kaynak);
            puanli.push((skor + bonus, i, indeksler));
        }
        // Skora göre azalan; eşitlikte kısa ada öncelik (daha alakalı görünür).
        puanli.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| {
                    self.komutlar[a.1]
                        .ad
                        .len()
                        .cmp(&self.komutlar[b.1].ad.len())
                })
                .then_with(|| self.komutlar[a.1].ad.cmp(&self.komutlar[b.1].ad))
        });
        let _ = mod_; // mod ayrımı (Komut/HizliGecis) skorlama açısından aynı; ileride genişler.
        puanli
            .into_iter()
            .take(AZAMI_GORUNUR)
            .map(|(_, i, idx)| (i, idx))
            .collect()
    }

    /// Tek bir komutun sorguya göre puanı + vurgulanacak karakter indeksleri.
    fn komut_skoru(&self, k: &Komut, sorgu: &str) -> Option<(i32, Vec<usize>)> {
        if sorgu.is_empty() {
            return Some((0, Vec::new()));
        }
        // Önce görünen ad (vurgu indeksleri buradan); olmazsa arama samanı (vurgu yok, düşük puan).
        if let Some(r) = bulanik_skor(sorgu, &k.ad) {
            if !r.indeksler.is_empty() {
                return Some((r.skor, r.indeksler));
            }
        }
        bulanik_skor(sorgu, &k.saman).map(|r| (r.skor - 6, Vec::new()))
    }

    /// Son/sık kullanım sıralama bonusu.
    fn kullanim_bonusu(&self, kaynak: &KomutKaynak) -> i32 {
        let mut b = 0;
        if let Some(p) = self.son.iter().position(|k| k == kaynak) {
            b += 30 - (p as i32) * 3; // en yeni en yüksek
        }
        b += (*self.sayac.get(kaynak).unwrap_or(&0)).min(10) as i32 * 2;
        b
    }

    /// Paleti çizer; seçilen eylemi döner (varsa).  Açık değilse hiçbir şey çizmez.
    pub fn ciz(&mut self, ctx: &egui::Context, dil: Dil, tok: &Tokenlar) -> Option<PaletEylem> {
        if !self.acik {
            return None;
        }
        let tr = matches!(dil, Dil::Tr);

        // Klavye gezinmesini metin kutusundan ÖNCE tüket (yön/Enter kutuya gitmesin).
        let (yukari, asagi, gir, esc, baş, son_) = ctx.input_mut(|i| {
            use egui::{Key, Modifiers};
            (
                i.consume_key(Modifiers::NONE, Key::ArrowUp),
                i.consume_key(Modifiers::NONE, Key::ArrowDown),
                i.consume_key(Modifiers::NONE, Key::Enter),
                i.consume_key(Modifiers::NONE, Key::Escape),
                i.consume_key(Modifiers::NONE, Key::Home),
                i.consume_key(Modifiers::NONE, Key::End),
            )
        });

        // `kalan` sahipli (owned) tutulur → metin kutusunun `&mut self.sorgu`'su ile çakışmaz.
        let (mod_, kalan_ref) = Self::modu(&self.sorgu);
        let kalan = kalan_ref.to_string();
        let mut secilen_eylem: Option<PaletEylem> = None;

        // Sembol modu mu, komut modu mu? Aday listesini hazırla.
        let sembol_modu = mod_ == PaletModu::Sembol;
        let adaylar: Vec<(usize, Vec<usize>)> = if sembol_modu {
            Vec::new()
        } else {
            self.adaylar(mod_, &kalan)
        };
        let sembol_adaylar: Vec<(usize, Vec<usize>)> = if sembol_modu {
            let mut puanli: Vec<(i32, usize, Vec<usize>)> = self
                .semboller
                .iter()
                .enumerate()
                .filter_map(|(i, s)| bulanik_skor(&kalan, s).map(|r| (r.skor, i, r.indeksler)))
                .collect();
            puanli.sort_by_key(|x| std::cmp::Reverse(x.0));
            puanli.into_iter().map(|(_, i, idx)| (i, idx)).collect()
        } else {
            Vec::new()
        };

        let satir_sayisi = if sembol_modu {
            sembol_adaylar.len()
        } else {
            adaylar.len()
        };

        // Gezinme uygula.
        if satir_sayisi > 0 {
            if asagi {
                self.secili = (self.secili + 1).min(satir_sayisi - 1);
            }
            if yukari {
                self.secili = self.secili.saturating_sub(1);
            }
            if baş {
                self.secili = 0;
            }
            if son_ {
                self.secili = satir_sayisi - 1;
            }
            self.secili = self.secili.min(satir_sayisi - 1);
        } else {
            self.secili = 0;
        }

        if esc {
            self.kapat();
            return None;
        }

        // Enter → seçili satırı çalıştır.
        if gir && satir_sayisi > 0 {
            if sembol_modu {
                let (i, _) = sembol_adaylar[self.secili];
                secilen_eylem = Some(PaletEylem::SemboleGit(self.semboller[i].clone()));
            } else {
                let (i, _) = adaylar[self.secili];
                secilen_eylem = Some(PaletEylem::Calistir(self.komutlar[i].kaynak.clone()));
            }
        }

        // ── Çizim: ekranı hafifçe karart + ortada panel ──
        let ekran = ctx.screen_rect();
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("ip13_palet_golge"),
        ))
        .rect_filled(
            ekran,
            egui::Rounding::ZERO,
            egui::Color32::from_black_alpha(96),
        );

        let genislik = (ekran.width() * 0.6).clamp(360.0, 640.0);
        egui::Area::new(egui::Id::new("ip13_komut_paleti"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 84.0))
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(tok.renk.yuzey)
                    .stroke(egui::Stroke::new(1.0, tok.renk.kenarlik))
                    .rounding(egui::Rounding::same(tok.yaricap))
                    .inner_margin(egui::Margin::same(8.0))
                    .shadow(egui::epaint::Shadow {
                        offset: egui::vec2(0.0, 6.0),
                        blur: 24.0,
                        spread: 0.0,
                        color: egui::Color32::from_black_alpha(120),
                    })
                    .show(ui, |ui| {
                        ui.set_width(genislik);

                        // Arama kutusu (otomatik odak) + mod ipucu.
                        let ipucu = match mod_ {
                            PaletModu::Sembol => {
                                if tr {
                                    "@ sembol ara…"
                                } else {
                                    "@ search symbol…"
                                }
                            }
                            PaletModu::Komut => {
                                if tr {
                                    "> komut ara…"
                                } else {
                                    "> search command…"
                                }
                            }
                            PaletModu::HizliGecis => {
                                if tr {
                                    "Komut ara…  ( > komut · @ sembol )"
                                } else {
                                    "Search command…  ( > command · @ symbol )"
                                }
                            }
                        };
                        let yanit = ui.add(
                            egui::TextEdit::singleline(&mut self.sorgu)
                                .hint_text(ipucu)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Heading),
                        );
                        if self.odakla {
                            yanit.request_focus();
                            self.odakla = false;
                        }
                        // Sorgu değişince seçimi başa al (kullanıcı yazdıkça en iyi sonuç seçili kalsın).
                        if yanit.changed() {
                            self.secili = 0;
                        }
                        ui.separator();

                        // Not: Liste çizimleri SERBEST fonksiyonlardır (self metodu değil) → metin
                        // kutusunun `&mut self.sorgu`'su ile alanların paylaşımlı ödünçleri (komutlar/
                        // son/semboller) Rust 2021 ayrık-yakalamayla çakışmaz.
                        let komutlar = &self.komutlar;
                        let son = &self.son;
                        let semboller = &self.semboller;
                        let secili = self.secili;
                        let bos_sorgu = kalan.is_empty();
                        egui::ScrollArea::vertical()
                            .max_height((ekran.height() * 0.5).clamp(160.0, 480.0))
                            .show(ui, |ui| {
                                if secilen_eylem.is_some() {
                                    return;
                                }
                                if sembol_modu {
                                    secilen_eylem = Self::sembol_listesi(
                                        ui,
                                        semboller,
                                        &sembol_adaylar,
                                        secili,
                                        tr,
                                        tok,
                                    );
                                } else if adaylar.is_empty() {
                                    secilen_eylem = Self::bos_durum(ui, komutlar, &kalan, tr, tok);
                                } else {
                                    secilen_eylem = Self::komut_listesi(
                                        ui, komutlar, &adaylar, son, secili, bos_sorgu, dil, tok,
                                    );
                                }
                            });

                        // Altbilgi: gezinme ipuçları (keşfedilebilirlik).
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.small(if tr {
                                "↑↓ gez · Enter çalıştır · Esc kapat"
                            } else {
                                "↑↓ navigate · Enter run · Esc close"
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.small(
                                        egui::RichText::new(format!("{satir_sayisi}"))
                                            .color(tok.renk.metin_soluk),
                                    );
                                },
                            );
                        });
                    });
            });

        // Seçim yapıldıysa kullanımı kaydet + kapat.
        if let Some(PaletEylem::Calistir(k)) = &secilen_eylem {
            self.kullanildi(&k.clone());
            self.kapat();
        } else if matches!(secilen_eylem, Some(PaletEylem::SemboleGit(_))) {
            self.kapat();
        }
        secilen_eylem
    }

    /// Komut satırlarını çizer; tıklanırsa eylemi döner.  (Serbest/ilişkili fonksiyon — `self`
    /// almaz → çizim sırasında alanların paylaşımlı ödünçleriyle çakışmaz.)
    #[allow(clippy::too_many_arguments)]
    fn komut_listesi(
        ui: &mut egui::Ui,
        komutlar: &[Komut],
        adaylar: &[(usize, Vec<usize>)],
        son: &[KomutKaynak],
        secili_idx: usize,
        bos_sorgu: bool,
        dil: Dil,
        tok: &Tokenlar,
    ) -> Option<PaletEylem> {
        let tr = matches!(dil, Dil::Tr);
        let mut eylem = None;
        // Boş sorguda son/diğer ayrımı için bölüm başlıkları (her biri bir kez).
        let mut son_baslik = false;
        let mut diger_baslik = false;
        for (sira, (i, indeksler)) in adaylar.iter().enumerate() {
            let k = &komutlar[*i];
            if bos_sorgu {
                let sonda = son.contains(&k.kaynak);
                if sonda && !son_baslik {
                    ui.small(
                        egui::RichText::new(if tr { "Son kullanılanlar" } else { "Recent" })
                            .color(tok.renk.metin_soluk),
                    );
                    son_baslik = true;
                } else if !sonda && son_baslik && !diger_baslik {
                    ui.small(
                        egui::RichText::new(if tr { "Tüm komutlar" } else { "All commands" })
                            .color(tok.renk.metin_soluk),
                    );
                    diger_baslik = true;
                }
            }
            let secili = sira == secili_idx;
            if Self::komut_satiri(ui, k, indeksler, secili, dil, tok).clicked() {
                eylem = Some(PaletEylem::Calistir(k.kaynak.clone()));
            }
        }
        eylem
    }

    /// Tek bir komut satırı (ikon + vurgulu ad + kategori + kısayol).
    fn komut_satiri(
        ui: &mut egui::Ui,
        k: &Komut,
        indeksler: &[usize],
        secili: bool,
        dil: Dil,
        tok: &Tokenlar,
    ) -> egui::Response {
        let job = vurgu_job(&k.ad, indeksler, tok, secili);
        let yukseklik = ui.spacing().interact_size.y.max(22.0);
        let (rect, resp) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), yukseklik),
            egui::Sense::click(),
        );
        let vurgulu = secili || resp.hovered();
        if vurgulu {
            ui.painter()
                .rect_filled(rect, egui::Rounding::same(tok.yaricap), tok.renk.yuzey_alt);
        }
        let p = ui.painter();
        let mut x = rect.left() + 8.0;
        // İkon.
        if let Some(ikon) = k.ikon {
            p.text(
                egui::pos2(x, rect.center().y),
                egui::Align2::LEFT_CENTER,
                ikon,
                egui::FontId::proportional(14.0),
                tok.renk.metin_soluk,
            );
        }
        x += 22.0;
        // Vurgulu ad (galley).
        let galley = ui.fonts(|f| {
            f.layout_job({
                let mut j = job;
                j.wrap.max_width = rect.width() * 0.62;
                j
            })
        });
        p.galley(
            egui::pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            tok.renk.metin,
        );
        // Sağ: kısayol + kategori (soluk).
        let mut sag = rect.right() - 8.0;
        if let Some(ks) = &k.kisayol {
            let gal = p.layout_no_wrap(
                ks.clone(),
                egui::FontId::monospace(12.0),
                tok.renk.metin_soluk,
            );
            p.galley(
                egui::pos2(sag - gal.size().x, rect.center().y - gal.size().y / 2.0),
                gal,
                tok.renk.metin_soluk,
            );
            sag -= 130.0;
        }
        let kat = k.kategori.etiket(dil);
        let gal = p.layout_no_wrap(
            kat.to_string(),
            egui::FontId::proportional(11.0),
            tok.renk.metin_soluk,
        );
        p.galley(
            egui::pos2(sag - gal.size().x, rect.center().y - gal.size().y / 2.0),
            gal,
            tok.renk.metin_soluk,
        );

        if secili {
            resp.scroll_to_me(Some(egui::Align::Center));
        }
        resp
    }

    /// `@` sembol satırları.
    fn sembol_listesi(
        ui: &mut egui::Ui,
        semboller: &[String],
        adaylar: &[(usize, Vec<usize>)],
        secili_idx: usize,
        tr: bool,
        tok: &Tokenlar,
    ) -> Option<PaletEylem> {
        if semboller.is_empty() {
            ui.add_space(6.0);
            ui.colored_label(
                tok.renk.metin_soluk,
                if tr {
                    "Aktif görünümde sembol yok (temel `@` modu)."
                } else {
                    "No symbols in the active view (basic `@` mode)."
                },
            );
            return None;
        }
        let mut eylem = None;
        for (sira, (i, indeksler)) in adaylar.iter().enumerate() {
            let metin = &semboller[*i];
            let job = vurgu_job(metin, indeksler, tok, sira == secili_idx);
            if ui.selectable_label(sira == secili_idx, job).clicked() {
                eylem = Some(PaletEylem::SemboleGit(metin.clone()));
            }
        }
        eylem
    }

    /// Hiç sonuç yokken "şunu mu demek istediniz" + boş-durum.
    fn bos_durum(
        ui: &mut egui::Ui,
        komutlar: &[Komut],
        sorgu: &str,
        tr: bool,
        tok: &Tokenlar,
    ) -> Option<PaletEylem> {
        ui.add_space(6.0);
        if sorgu.is_empty() {
            ui.colored_label(
                tok.renk.metin_soluk,
                if tr { "Komut yok." } else { "No commands." },
            );
            return None;
        }
        // En yakın komutu öner (gevşek benzerlik).
        let oneri = komutlar
            .iter()
            .filter(|k| k.etkin)
            .map(|k| (gevsek_benzerlik(sorgu, &k.ad), k))
            .filter(|(s, _)| *s > 0)
            .max_by_key(|(s, _)| *s)
            .map(|(_, k)| k);
        ui.colored_label(
            tok.renk.metin_soluk,
            if tr {
                format!("\"{sorgu}\" için sonuç yok.")
            } else {
                format!("No results for \"{sorgu}\".")
            },
        );
        if let Some(k) = oneri {
            ui.add_space(2.0);
            let metin = if tr {
                format!("Şunu mu demek istediniz: {}", k.ad)
            } else {
                format!("Did you mean: {}", k.ad)
            };
            if ui
                .add(
                    egui::Button::new(egui::RichText::new(metin).color(tok.renk.vurgu))
                        .frame(false),
                )
                .clicked()
            {
                return Some(PaletEylem::Calistir(k.kaynak.clone()));
            }
        }
        None
    }
}

/// Eşleşen karakterleri vurgulayan bir `LayoutJob` üretir (eşleşen = vurgu rengi + kalın).
fn vurgu_job(
    metin: &str,
    indeksler: &[usize],
    tok: &Tokenlar,
    _secili: bool,
) -> egui::text::LayoutJob {
    use egui::text::{LayoutJob, TextFormat};
    let mut job = LayoutJob::default();
    let font = egui::FontId::proportional(14.0);
    for (i, c) in metin.chars().enumerate() {
        let eslesti = indeksler.contains(&i);
        let fmt = TextFormat {
            font_id: font.clone(),
            color: if eslesti {
                tok.renk.vurgu
            } else {
                tok.renk.metin
            },
            ..Default::default()
        };
        job.append(&c.to_string(), 0.0, fmt);
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{Komut, KomutKategori};
    use crate::shell::menu_bar::KabukAksiyon;

    fn komut(ad: &str, a: KabukAksiyon) -> Komut {
        Komut {
            kaynak: KomutKaynak::Kabuk(a),
            ad: ad.to_string(),
            kategori: KomutKategori::Gorunum,
            kisayol: None,
            ikon: None,
            etkin: true,
            saman: ad.to_lowercase(),
        }
    }

    fn palet() -> KomutPaleti {
        let mut p = KomutPaleti::yeni();
        p.komutlar = vec![
            komut("Tema Değiştir", KabukAksiyon::TemaDegistir),
            komut("Dili Değiştir", KabukAksiyon::DilDegistir),
            komut("Yeni Sekme", KabukAksiyon::YeniSekme),
            komut("Editörü Böl", KabukAksiyon::EditoruBol),
            komut("Ayarlar", KabukAksiyon::Ayarlar),
        ];
        p
    }

    #[test]
    fn mod_onek_ayristirma() {
        assert_eq!(KomutPaleti::modu(">tema").0, PaletModu::Komut);
        assert_eq!(KomutPaleti::modu("@sym").0, PaletModu::Sembol);
        assert_eq!(KomutPaleti::modu("tema").0, PaletModu::HizliGecis);
        assert_eq!(KomutPaleti::modu(">  tema").1, "tema");
    }

    #[test]
    fn bulanik_arama_bulur_ve_siralar() {
        let p = palet();
        // "tema" → Tema Değiştir en üstte.
        let a = p.adaylar(PaletModu::HizliGecis, "tema");
        assert!(!a.is_empty());
        assert_eq!(p.komutlar[a[0].0].ad, "Tema Değiştir");
        // Kısaltma "es" → Yeni Sekme veya Ayarlar… en azından bir şey döner.
        let a2 = p.adaylar(PaletModu::HizliGecis, "ed böl");
        assert!(a2.iter().any(|(i, _)| p.komutlar[*i].ad == "Editörü Böl"));
    }

    #[test]
    fn bos_sorgu_hepsini_dondurur_son_ustte() {
        let mut p = palet();
        p.kullanildi(&KomutKaynak::Kabuk(KabukAksiyon::Ayarlar));
        let a = p.adaylar(PaletModu::HizliGecis, "");
        assert_eq!(a.len(), p.komutlar.len());
        // Son kullanılan (Ayarlar) bonusla en üstte olmalı.
        assert_eq!(p.komutlar[a[0].0].ad, "Ayarlar");
    }

    #[test]
    fn devre_disi_komut_listede_yok() {
        let mut p = palet();
        p.komutlar.push(Komut {
            kaynak: KomutKaynak::Kabuk(KabukAksiyon::YeniProje),
            ad: "Yeni Proje".into(),
            kategori: KomutKategori::Dosya,
            kisayol: None,
            ikon: None,
            etkin: false,
            saman: "yeni proje".into(),
        });
        let a = p.adaylar(PaletModu::HizliGecis, "proje");
        assert!(a.is_empty(), "devre dışı komut palette görünmemeli");
    }

    #[test]
    fn kullanildi_son_ve_sayac_gunceller() {
        let mut p = palet();
        let k = KomutKaynak::Kabuk(KabukAksiyon::TemaDegistir);
        p.kullanildi(&k);
        p.kullanildi(&k);
        assert_eq!(p.son.first(), Some(&k));
        assert_eq!(*p.sayac.get(&k).unwrap(), 2);
    }
}
