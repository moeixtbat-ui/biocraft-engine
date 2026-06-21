//! Editör / Canvas alanı — sekmeli + yan-yana bölme + OS dosya sürükle-bırak (İP-03, E14).
//!
//! Merkez bölge: tuval/kod/node sekme olarak burada açılır (gerçek içerik İP-04/05/06).  Bu modül
//! iki katmandır:
//! - **Saf model** ([`Sekme`], [`SekmeGrubu`], [`EditorAlani`]): sekme ekle/kapat/sırala/sabitle +
//!   kaydedilmemiş işareti + bölme.  egui'den bağımsız → birim testlenir.  *Sürükle durumu modelde
//!   ayrı tutulur* (`surukle_kaynak`) — egui immediate-mode'da takılmayı önler (Gün-12 notu).
//! - **Çizim** ([`EditorAlani::ciz`]): sekme şeridi + içerik + bölme tutamağı; renkler token'dan
//!   (MK-52), metinler i18n'den (MK-53).
//!
//! **Sürükle-bırak (E14):** OS'tan bırakılan dosyanın uzantısına göre ne olacağı *önizlenir* ve
//! yanlış hedef *iptal* edilir ([`dosya_turu`]).  Bırakılan biyo-veri dosyasının gerçekten yüklenmesi
//! Gün-34 yükleyicisine bağlanır; burada doğru sekme türünde bir yer-tutucu sekme açılır.
// MK-52: renkler token'dan; bu modül sabit renk üretmez.

use crate::components::EmptyState;
use crate::i18n::Dil;
use crate::shell::split::{bol_boyut, orani_sikistir, BolmeYonu, TUTAMAK};
use crate::tokens::Tokenlar;

/// Bir editör sekmesinin içerik türü (gerçek içerik sonraki paketlerde).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SekmeTuru {
    /// 2B/3B/genom tuvali (İP-04 / ÇE-*).
    Tuval,
    /// Kod editörü (İP-06).
    Kod,
    /// Node/görsel akış editörü (İP-05).
    Node,
    /// Genel metin/belge.
    Genel,
}

impl SekmeTuru {
    /// Tür ikonu (tema-bağımsız sembol).
    pub fn ikon(self) -> &'static str {
        match self {
            SekmeTuru::Tuval => "🧬",
            SekmeTuru::Kod => "📜",
            SekmeTuru::Node => "🔗",
            SekmeTuru::Genel => "📄",
        }
    }

    /// Türün yerelleştirilmiş adı.
    pub fn ad(self, dil: Dil) -> &'static str {
        match (self, dil) {
            (SekmeTuru::Tuval, Dil::Tr) => "Tuval",
            (SekmeTuru::Tuval, Dil::En) => "Canvas",
            (SekmeTuru::Kod, Dil::Tr) => "Kod",
            (SekmeTuru::Kod, Dil::En) => "Code",
            (SekmeTuru::Node, Dil::Tr) => "Node",
            (SekmeTuru::Node, Dil::En) => "Node",
            (SekmeTuru::Genel, Dil::Tr) => "Belge",
            (SekmeTuru::Genel, Dil::En) => "Document",
        }
    }

    /// Bu türün gerçek içeriğini hangi paketin getireceği (yer-tutucu açıklaması).
    fn paket(self) -> &'static str {
        match self {
            SekmeTuru::Tuval => "İP-04 / ÇE-07",
            SekmeTuru::Kod => "İP-06",
            SekmeTuru::Node => "İP-05",
            SekmeTuru::Genel => "—",
        }
    }
}

/// Açık bir editör sekmesi (kalıcı [`biocraft_state::AcikSekme`]'nin UI tarafı zengin karşılığı).
#[derive(Debug, Clone)]
pub struct Sekme {
    /// Oturum içi benzersiz kimlik (yeniden-sıralama/kapatma sırasında kararlı referans).
    pub kimlik: u64,
    /// Kullanıcıya görünen başlık.
    pub baslik: String,
    /// İçerik türü.
    pub tur: SekmeTuru,
    /// Kaydedilmemiş değişiklik var mı? (• işareti + kapatma uyarısı).
    pub kaydedilmemis: bool,
    /// Sabitlenmiş (pin) mi? (sabit sekmeler şeritte solda kalır, kazara kapanmaz).
    pub sabit: bool,
}

/// Bir editör bölmesindeki sekme grubu (bir veya iki grup → bkz. [`EditorAlani`]).
#[derive(Debug, Clone, Default)]
pub struct SekmeGrubu {
    /// Sekmeler — gösterim sırası (sabitler her zaman başta; bkz. [`SekmeGrubu::duzenle`]).
    sekmeler: Vec<Sekme>,
    /// Etkin sekmenin dizini.
    aktif: Option<usize>,
    /// Sürükle-yeniden-sırala kaynağı (egui'den ayrı durum — takılmayı önler).
    pub surukle_kaynak: Option<usize>,
}

impl SekmeGrubu {
    /// Boş grup.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Sekmelere salt-okunur erişim (çizim/test).
    pub fn sekmeler(&self) -> &[Sekme] {
        &self.sekmeler
    }

    /// Etkin sekme dizini.
    pub fn aktif(&self) -> Option<usize> {
        self.aktif
    }

    /// Etkin sekmeye salt-okunur erişim.
    pub fn aktif_sekme(&self) -> Option<&Sekme> {
        self.aktif.and_then(|i| self.sekmeler.get(i))
    }

    /// Sekme sayısı.
    pub fn len(&self) -> usize {
        self.sekmeler.len()
    }

    /// Grup boş mu?
    pub fn is_empty(&self) -> bool {
        self.sekmeler.is_empty()
    }

    /// Yeni sekme ekler (sona) ve onu etkin yapar.  Değişmezi korur (sabitler başta).
    pub fn ekle(&mut self, sekme: Sekme) {
        let kimlik = sekme.kimlik;
        self.sekmeler.push(sekme);
        self.duzenle();
        self.aktif = self.indeks_bul(kimlik);
    }

    /// `idx`'teki sekmeyi etkin yapar (geçerliyse).
    pub fn aktif_yap(&mut self, idx: usize) {
        if idx < self.sekmeler.len() {
            self.aktif = Some(idx);
        }
    }

    /// `idx`'teki sekmeyi kapatır; kaldırılan sekmeyi döner.  Etkin dizini güvenli komşuya taşır.
    pub fn kapat(&mut self, idx: usize) -> Option<Sekme> {
        if idx >= self.sekmeler.len() {
            return None;
        }
        let kaldirilan = self.sekmeler.remove(idx);
        self.aktif = if self.sekmeler.is_empty() {
            None
        } else if let Some(a) = self.aktif {
            // Etkin sekme kapandıysa solundaki komşuya geç; sağındakiyse dizini kaydır.
            Some(match a.cmp(&idx) {
                std::cmp::Ordering::Greater => a - 1,
                std::cmp::Ordering::Equal => idx.min(self.sekmeler.len() - 1),
                std::cmp::Ordering::Less => a,
            })
        } else {
            None
        };
        Some(kaldirilan)
    }

    /// Kimliğe göre sekmeyi kapatır (çizimden gelen kararlı referans için).
    pub fn kapat_kimlik(&mut self, kimlik: u64) -> Option<Sekme> {
        let idx = self.indeks_bul(kimlik)?;
        self.kapat(idx)
    }

    /// `idx`'teki sekmenin sabitleme (pin) durumunu değiştirir; değişmezi yeniden kurar.
    pub fn sabitle_degistir(&mut self, idx: usize) {
        let Some(aktif_kimlik) = self
            .aktif
            .and_then(|a| self.sekmeler.get(a))
            .map(|s| s.kimlik)
        else {
            if let Some(s) = self.sekmeler.get_mut(idx) {
                s.sabit = !s.sabit;
                self.duzenle();
            }
            return;
        };
        if let Some(s) = self.sekmeler.get_mut(idx) {
            s.sabit = !s.sabit;
        }
        self.duzenle();
        self.aktif = self.indeks_bul(aktif_kimlik);
    }

    /// Sekmeyi `kaynak`'tan `hedef`'e taşır (sürükle-yeniden-sırala); değişmezi korur.
    pub fn yeniden_sirala(&mut self, kaynak: usize, hedef: usize) {
        if kaynak >= self.sekmeler.len() || hedef >= self.sekmeler.len() || kaynak == hedef {
            return;
        }
        let aktif_kimlik = self
            .aktif
            .and_then(|a| self.sekmeler.get(a))
            .map(|s| s.kimlik);
        let s = self.sekmeler.remove(kaynak);
        self.sekmeler.insert(hedef, s);
        self.duzenle();
        if let Some(k) = aktif_kimlik {
            self.aktif = self.indeks_bul(k);
        }
    }

    /// Etkin sekmeyi "kaydedildi" işaretler (kaydedilmemiş • işaretini kaldırır).
    pub fn aktifi_kaydet(&mut self) -> bool {
        if let Some(s) = self.aktif.and_then(|a| self.sekmeler.get_mut(a)) {
            s.kaydedilmemis = false;
            true
        } else {
            false
        }
    }

    /// En az bir kaydedilmemiş sekme var mı?
    pub fn kaydedilmemis_var(&self) -> bool {
        self.sekmeler.iter().any(|s| s.kaydedilmemis)
    }

    /// Değişmezi kurar: **sabit sekmeler her zaman başta** (kararlı sıralama, göreli sıra korunur).
    fn duzenle(&mut self) {
        // sort_by_key kararlıdır → grup içi göreli sıra korunur; sabit (false<true) öne gelir.
        self.sekmeler.sort_by_key(|s| !s.sabit);
    }

    /// Kimliği olan sekmenin güncel dizini.
    fn indeks_bul(&self, kimlik: u64) -> Option<usize> {
        self.sekmeler.iter().position(|s| s.kimlik == kimlik)
    }
}

/// İki veriyi karşılaştırmak için iki grup taşıyan editör alanı (sekmeli + bölmeli) — İP-03.
#[derive(Debug, Clone)]
pub struct EditorAlani {
    /// Birincil sekme grubu (sol/üst).
    pub birincil: SekmeGrubu,
    /// İkincil sekme grubu (sağ/alt) — yalnızca bölme etkinken kullanılır.
    pub ikincil: SekmeGrubu,
    /// Bölme yönü.
    pub yon: BolmeYonu,
    /// Bölme oranı (birincil grubun payı; `0.1..=0.9`).
    pub oran: f32,
    /// Odaktaki grup ikincil mi? (yeni sekme/inspector hedefi).
    pub odak_ikincil: bool,
    /// Yeni sekmelere benzersiz kimlik vermek için sayaç.
    sayac: u64,
}

impl Default for EditorAlani {
    fn default() -> Self {
        Self::yeni()
    }
}

impl EditorAlani {
    /// Bir örnek tuval sekmesiyle açılır (boş alanın "başlamak için" rehberi de ayrıca gösterilir).
    pub fn yeni() -> Self {
        let mut a = Self {
            birincil: SekmeGrubu::yeni(),
            ikincil: SekmeGrubu::yeni(),
            yon: BolmeYonu::Yok,
            oran: 0.5,
            odak_ikincil: false,
            sayac: 0,
        };
        // Açılışta örnek bir tuval sekmesi (boş kabuk hissi vermesin); kaydedilmiş kabul edilir.
        a.yeni_sekme_uret(SekmeTuru::Tuval, "Tuval 1", false);
        a
    }

    /// Odaktaki grubun değiştirilebilir referansı (yeni sekme buraya eklenir).
    pub fn odak_grup_mut(&mut self) -> &mut SekmeGrubu {
        if self.odak_ikincil && self.yon.bolundu_mu() {
            &mut self.ikincil
        } else {
            &mut self.birincil
        }
    }

    /// Odaktaki grubun salt-okunur etkin sekmesi (inspector için).
    pub fn odak_aktif_sekme(&self) -> Option<&Sekme> {
        let g = if self.odak_ikincil && self.yon.bolundu_mu() {
            &self.ikincil
        } else {
            &self.birincil
        };
        g.aktif_sekme()
    }

    /// Verilen tür + başlıkla yeni bir sekme üretir (benzersiz kimlik) ve odaktaki gruba ekler.
    pub fn yeni_sekme_uret(
        &mut self,
        tur: SekmeTuru,
        baslik: impl Into<String>,
        kaydedilmemis: bool,
    ) {
        self.sayac += 1;
        let sekme = Sekme {
            kimlik: self.sayac,
            baslik: baslik.into(),
            tur,
            kaydedilmemis,
            sabit: false,
        };
        self.odak_grup_mut().ekle(sekme);
    }

    /// Menü/komuttan "Yeni Sekme": numaralı boş tuval sekmesi (kaydedilmemiş — kapatma uyarısı demosu).
    pub fn yeni_sekme(&mut self, dil: Dil) {
        let n = self.sayac + 1;
        let baslik = match dil {
            Dil::Tr => format!("Yeni {n}"),
            Dil::En => format!("New {n}"),
        };
        self.yeni_sekme_uret(SekmeTuru::Tuval, baslik, true);
    }

    /// Bölme yönünü değiştirir (Yok→Yatay→Dikey→Yok); bölme açılırken ikincil grubu hazırlar.
    pub fn bolmeyi_degistir(&mut self, dil: Dil) {
        self.bolmeyi_ayarla(self.yon.sonraki(), self.oran, dil);
    }

    /// Bölme yön + oranını belirli bir değere ayarlar (kalıcı durumdan geri yükleme için).
    ///
    /// Bölme etkin ve ikincil grup boşsa örnek bir "Karşılaştır" sekmesi koyar; bölme kapanınca
    /// odak birincil gruba döner.  Oran güvenli aralığa sıkıştırılır.
    pub fn bolmeyi_ayarla(&mut self, yon: BolmeYonu, oran: f32, dil: Dil) {
        self.yon = yon;
        self.oran = orani_sikistir(oran);
        if yon.bolundu_mu() {
            if self.ikincil.is_empty() {
                self.odak_ikincil = true;
                let baslik = match dil {
                    Dil::Tr => "Karşılaştır",
                    Dil::En => "Compare",
                };
                self.yeni_sekme_uret(SekmeTuru::Tuval, baslik, false);
            }
        } else {
            self.odak_ikincil = false;
        }
    }

    /// Herhangi bir grupta kaydedilmemiş sekme var mı?
    pub fn kaydedilmemis_var(&self) -> bool {
        self.birincil.kaydedilmemis_var() || self.ikincil.kaydedilmemis_var()
    }
}

/// Çizimden çağırana ulaşan olay: bir sekmenin kapatılması istendi (kaydedilmemişse onay gerekir).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KapatmaIstegi {
    /// Hangi grupta? (false=birincil, true=ikincil).
    pub ikincil: bool,
    /// Kapatılacak sekmenin kimliği.
    pub kimlik: u64,
}

// ─── Sürükle-bırak (E14): OS dosyası → editör ────────────────────────────────

/// Bir dosya uzantısının hangi sekme türünde açılacağı; tanınmayan uzantı `None` (geçersiz hedef).
///
/// Saf fonksiyon (egui'siz): bırakma önizlemesi + geçerlilik kararı buradan gelir → test edilebilir.
/// Gerçek yükleme Gün-34'te; burada yalnızca tür eşlemesi yapılır.
pub fn dosya_turu(uzanti: &str) -> Option<SekmeTuru> {
    match uzanti.to_ascii_lowercase().as_str() {
        // Biyo-veri → genom/3B tuvali.
        "fasta" | "fa" | "fastq" | "fq" | "bam" | "sam" | "cram" | "vcf" | "bcf" | "bed"
        | "gff" | "gff3" | "gtf" | "pdb" | "cif" | "mmcif" => Some(SekmeTuru::Tuval),
        // Betik → kod editörü.
        "py" | "r" | "sh" | "rs" => Some(SekmeTuru::Kod),
        // Akış/grafik → node editörü.
        "bcflow" | "ron" | "json" => Some(SekmeTuru::Node),
        // Düz metin/belge.
        "txt" | "md" | "log" | "csv" | "tsv" => Some(SekmeTuru::Genel),
        _ => None,
    }
}

/// Bir dosya yolundan uzantıyı (ham, büyük/küçük korunur) çıkarır ("/yol/x.bam" → "bam").
/// Eşleme [`dosya_turu`] içinde büyük/küçük harf duyarsızdır; bu yüzden burada çevrim yapılmaz.
pub fn uzanti_al(yol: &str) -> &str {
    yol.rsplit(['.', '/', '\\'])
        .next()
        .filter(|_| yol.contains('.'))
        .unwrap_or("")
}

/// Bırakılan dosya için kullanıcıya gösterilecek önizleme (vurgu + ne olacağı).
#[derive(Debug, Clone)]
pub struct BirakmaOnizleme {
    /// Hedef geçerli mi? (false → "buraya bırakılamaz").
    pub gecerli: bool,
    /// Açılacak sekme türü (geçerliyse).
    pub tur: Option<SekmeTuru>,
    /// Kullanıcıya gösterilecek metin.
    pub metin: String,
}

/// Bir dosya yolu için bırakma önizlemesini üretir (i18n metin dahil).
pub fn birakma_onizleme(yol: &str, dil: Dil) -> BirakmaOnizleme {
    let uzanti = uzanti_al(yol);
    match dosya_turu(uzanti) {
        Some(tur) => {
            let metin = match dil {
                Dil::Tr => format!(
                    "“.{uzanti}” → {} sekmesi olarak aç (yükleme Gün-34)",
                    tur.ad(dil)
                ),
                Dil::En => format!("“.{uzanti}” → open as {} tab (load on Day-34)", tur.ad(dil)),
            };
            BirakmaOnizleme {
                gecerli: true,
                tur: Some(tur),
                metin,
            }
        }
        None => {
            let metin = match dil {
                Dil::Tr => format!("“.{uzanti}” buraya bırakılamaz (desteklenmeyen tür)"),
                Dil::En => format!("“.{uzanti}” can't be dropped here (unsupported type)"),
            };
            BirakmaOnizleme {
                gecerli: false,
                tur: None,
                metin,
            }
        }
    }
}

// ─── Çizim ───────────────────────────────────────────────────────────────────

impl EditorAlani {
    /// Editör alanını verili `ui` içine çizer (sekme şeritleri + içerik + bölme tutamağı).
    ///
    /// Dönüş: kaydedilmemiş bir sekmenin kapatılması istendiyse [`KapatmaIstegi`] (çağıran onay
    /// diyaloğu gösterir).  Kaydedilmiş sekmeler, yeniden-sıralama/sabitleme ve "＋ yeni sekme"
    /// burada uygulanır; odak (yeni sekme/inspector hedefi) tıklanan gruba göre güncellenir.
    pub fn ciz(&mut self, ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) -> Option<KapatmaIstegi> {
        if !self.yon.bolundu_mu() {
            let sonuc = grup_ciz(&mut self.birincil, true, ui, dil, tok);
            if sonuc.odaklandi || sonuc.yeni_sekme {
                self.odak_ikincil = false;
            }
            if sonuc.yeni_sekme {
                self.yeni_sekme(dil);
            }
            return sonuc.kapatma_kimlik(false);
        }

        // Bölmeli: alanı orana göre iki çocuğa ayır + sürüklenebilir tutamak (ayrı borrow'lar).
        let alan = ui.available_size();
        let ((w1, h1), (w2, h2)) = bol_boyut(self.yon, self.oran, alan.x, alan.y);
        let (yon, oran) = (self.yon, self.oran);
        let birincil = &mut self.birincil;
        let ikincil = &mut self.ikincil;
        let birincil_odakli = !self.odak_ikincil;
        let mut r1 = GrupSonuc::default();
        let mut r2 = GrupSonuc::default();
        let mut yeni_oran = oran;
        let ciz_iki = |ui: &mut egui::Ui| {
            ui.allocate_ui(egui::vec2(w1, h1), |ui| {
                r1 = grup_ciz(birincil, birincil_odakli, ui, dil, tok);
            });
            yeni_oran = tutamak_ciz(
                ui,
                yon,
                oran,
                if yon == BolmeYonu::Yatay {
                    alan.x
                } else {
                    alan.y
                },
            );
            ui.allocate_ui(egui::vec2(w2, h2), |ui| {
                r2 = grup_ciz(ikincil, !birincil_odakli, ui, dil, tok);
            });
        };
        match yon {
            BolmeYonu::Yatay => {
                ui.horizontal_top(ciz_iki);
            }
            BolmeYonu::Dikey => {
                ui.vertical(ciz_iki);
            }
            BolmeYonu::Yok => unreachable!("bölünmemiş durum üstte ele alındı"),
        }
        self.oran = yeni_oran;

        // Odak + yeni sekme dönüş değerlerinden uygulanır (borrow çakışması olmadan).
        if r1.odaklandi || r1.yeni_sekme {
            self.odak_ikincil = false;
        }
        if r2.odaklandi || r2.yeni_sekme {
            self.odak_ikincil = true;
        }
        if r1.yeni_sekme {
            self.odak_ikincil = false;
            self.yeni_sekme(dil);
        }
        if r2.yeni_sekme {
            self.odak_ikincil = true;
            self.yeni_sekme(dil);
        }
        r1.kapatma_kimlik(false).or(r2.kapatma_kimlik(true))
    }
}

/// Bölme tutamağını (gutter) çizer; sürüklenirse oranı günceller ve döner.
fn tutamak_ciz(ui: &mut egui::Ui, yon: BolmeYonu, oran: f32, toplam: f32) -> f32 {
    let boyut = match yon {
        BolmeYonu::Yatay => egui::vec2(TUTAMAK, ui.available_height()),
        _ => egui::vec2(ui.available_width(), TUTAMAK),
    };
    let (rect, yanit) = ui.allocate_exact_size(boyut, egui::Sense::drag());
    let renk = if yanit.hovered() || yanit.dragged() {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke.color
    };
    ui.painter().rect_filled(rect, egui::Rounding::ZERO, renk);
    if yanit.hovered() || yanit.dragged() {
        let imlec = match yon {
            BolmeYonu::Yatay => egui::CursorIcon::ResizeHorizontal,
            _ => egui::CursorIcon::ResizeVertical,
        };
        ui.ctx().set_cursor_icon(imlec);
    }
    if yanit.dragged() && toplam > 1.0 {
        let delta = match yon {
            BolmeYonu::Yatay => yanit.drag_delta().x,
            _ => yanit.drag_delta().y,
        };
        return orani_sikistir(oran + delta / toplam);
    }
    oran
}

/// Tek bir grubun çizim sonucu (mutasyon çağıran [`EditorAlani`]'de uygulanır).
#[derive(Debug, Clone, Copy, Default)]
struct GrupSonuc {
    /// Kaydedilmemiş bir sekmenin kapatılması istendi (kimlik) — onay gerektirir.
    kapatma: Option<u64>,
    /// "＋" ile yeni sekme istendi.
    yeni_sekme: bool,
    /// Kullanıcı bu grupta bir sekmeye/＋'ya tıkladı (odak buraya geçer).
    odaklandi: bool,
}

impl GrupSonuc {
    /// Kapatma isteğini grup kimliğiyle (ikincil mi) [`KapatmaIstegi`]'ne çevirir.
    fn kapatma_kimlik(self, ikincil: bool) -> Option<KapatmaIstegi> {
        self.kapatma.map(|kimlik| KapatmaIstegi { ikincil, kimlik })
    }
}

/// Tek bir sekme grubunu (şerit + etkin içerik) çizer.  `odakli`: bu grup şu an odakta mı (vurgu).
fn grup_ciz(
    grup: &mut SekmeGrubu,
    odakli: bool,
    ui: &mut egui::Ui,
    dil: Dil,
    tok: &Tokenlar,
) -> GrupSonuc {
    let mut sonuc = GrupSonuc::default();
    let cerceve = egui::Frame::none()
        .fill(tok.renk.zemin_alt)
        .stroke(if odakli {
            egui::Stroke::new(1.0, tok.renk.vurgu)
        } else {
            egui::Stroke::new(1.0, tok.renk.kenarlik)
        });
    cerceve.show(ui, |ui| {
        // ── Sekme şeridi ──
        match sekme_seridi(grup, ui, dil, tok) {
            Some(SeritOlay::SekmeyeGec(idx)) => {
                grup.aktif_yap(idx);
                sonuc.odaklandi = true;
            }
            Some(SeritOlay::KapatmaIstendi(kimlik, kaydedilmemis)) => {
                if kaydedilmemis {
                    sonuc.kapatma = Some(kimlik); // onay için yukarı taşı
                } else {
                    grup.kapat_kimlik(kimlik); // kaydedilmiş → hemen kapat
                }
            }
            Some(SeritOlay::SabitDegistir(idx)) => grup.sabitle_degistir(idx),
            Some(SeritOlay::YeniSekme) => sonuc.yeni_sekme = true,
            None => {}
        }
        ui.separator();

        // ── İçerik ──
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if grup.is_empty() {
                    bos_alan_rehberi(ui, dil, tok);
                } else if let Some(s) = grup.aktif_sekme() {
                    sekme_icerigi(ui, s, dil, tok);
                }
            });
    });
    sonuc
}

/// Sekme şeridinden çıkan olay (mutasyon çağıran grupta uygulanır).
enum SeritOlay {
    /// `idx`'teki sekmeye geçildi.
    SekmeyeGec(usize),
    /// Kapatma istendi (kimlik, kaydedilmemis).
    KapatmaIstendi(u64, bool),
    /// `idx`'teki sekmenin sabitleme durumu değişti.
    SabitDegistir(usize),
    /// Yeni sekme istendi.
    YeniSekme,
}

/// Yatay sekme şeridini çizer: sürükle-yeniden-sırala + sabit + kaydedilmemiş • + kapat ✕ + "＋".
fn sekme_seridi(
    grup: &mut SekmeGrubu,
    ui: &mut egui::Ui,
    dil: Dil,
    tok: &Tokenlar,
) -> Option<SeritOlay> {
    let mut olay = None;
    let tr = matches!(dil, Dil::Tr);
    let pointer = ui.input(|i| i.pointer.interact_pos());
    // Anlık-kopya: grubu çizim döngüsü boyunca ödünç tutmadan sürükle/kapat mutasyonu yapabilmek için.
    let aktif = grup.aktif();
    let anlik: Vec<(usize, u64, String, &'static str, bool, bool)> = grup
        .sekmeler()
        .iter()
        .enumerate()
        .map(|(i, s)| {
            (
                i,
                s.kimlik,
                s.baslik.clone(),
                s.tur.ikon(),
                s.sabit,
                s.kaydedilmemis,
            )
        })
        .collect();

    egui::ScrollArea::horizontal()
        .auto_shrink([false, false])
        .max_height(28.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let mut surukle_hedef: Option<usize> = None;
                for (i, kimlik, baslik, ikon, sabit, kaydedilmemis) in &anlik {
                    let (i, kimlik, sabit, kaydedilmemis) = (*i, *kimlik, *sabit, *kaydedilmemis);
                    let secili = aktif == Some(i);
                    // Etiket: [📌] ikon  •başlık
                    let mut etiket = String::new();
                    if sabit {
                        etiket.push_str("📌 ");
                    }
                    etiket.push_str(ikon);
                    etiket.push(' ');
                    if kaydedilmemis {
                        etiket.push_str("• ");
                    }
                    etiket.push_str(baslik);

                    let renk = if secili {
                        tok.renk.metin
                    } else {
                        tok.renk.metin_soluk
                    };
                    let dugme = egui::Button::new(egui::RichText::new(etiket).color(renk))
                        .frame(secili)
                        .sense(egui::Sense::click_and_drag());
                    let yanit = ui.add(dugme);

                    // Sürükle-yeniden-sırala: kaynak yakala, bırakınca hedefe taşı (ayrı state).
                    if yanit.drag_started() {
                        grup.surukle_kaynak = Some(i);
                    }
                    if grup.surukle_kaynak.is_some() {
                        if let Some(p) = pointer {
                            if yanit.rect.contains(p) {
                                surukle_hedef = Some(i);
                            }
                        }
                    }
                    if yanit.clicked() {
                        olay = Some(SeritOlay::SekmeyeGec(i));
                    }
                    // Orta tık = kapat (kararlı kimlikle).
                    if yanit.middle_clicked() {
                        olay = Some(SeritOlay::KapatmaIstendi(kimlik, kaydedilmemis));
                    }
                    // Sağ tık menüsü: sabitle/çöz + kapat.
                    yanit.context_menu(|ui| {
                        let pin_etiket = match (sabit, tr) {
                            (true, true) => "📌 Sabiti kaldır",
                            (true, false) => "📌 Unpin",
                            (false, true) => "📌 Sabitle",
                            (false, false) => "📌 Pin",
                        };
                        if ui.button(pin_etiket).clicked() {
                            olay = Some(SeritOlay::SabitDegistir(i));
                            ui.close_menu();
                        }
                        if ui.button(if tr { "✕ Kapat" } else { "✕ Close" }).clicked() {
                            olay = Some(SeritOlay::KapatmaIstendi(kimlik, kaydedilmemis));
                            ui.close_menu();
                        }
                    });

                    // Kapat ✕ (küçük) — sabit sekmede gösterilmez (kazara kapanmasın).
                    if !sabit {
                        let x = ui.add(
                            egui::Button::new(
                                egui::RichText::new("✕").small().color(tok.renk.metin_soluk),
                            )
                            .frame(false),
                        );
                        if x.clicked() {
                            olay = Some(SeritOlay::KapatmaIstendi(kimlik, kaydedilmemis));
                        }
                    }
                    ui.add_space(tok.bosluk.xs);
                }

                // Sürükle bitti → yeniden sırala.
                if ui.input(|i| i.pointer.any_released()) {
                    if let (Some(k), Some(h)) = (grup.surukle_kaynak, surukle_hedef) {
                        if k != h {
                            grup.yeniden_sirala(k, h);
                        }
                    }
                    grup.surukle_kaynak = None;
                }

                // "＋" yeni sekme.
                if ui
                    .button("＋")
                    .on_hover_text(if tr { "Yeni sekme" } else { "New tab" })
                    .clicked()
                {
                    olay = Some(SeritOlay::YeniSekme);
                }
            });
        });
    olay
}

/// Etkin sekmenin (yer-tutucu) içeriğini çizer.
fn sekme_icerigi(ui: &mut egui::Ui, s: &Sekme, dil: Dil, tok: &Tokenlar) {
    let tr = matches!(dil, Dil::Tr);
    ui.add_space(tok.bosluk.m);
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new(s.tur.ikon())
                .size(40.0)
                .color(tok.renk.vurgu),
        );
        ui.heading(&s.baslik);
        ui.add_space(tok.bosluk.xs);
        let aciklama = if tr {
            format!(
                "{} içeriği bu sürümde yer-tutucudur; gerçek görünüm {} ile gelir.",
                s.tur.ad(dil),
                s.tur.paket()
            )
        } else {
            format!(
                "{} content is a placeholder in this version; the real view arrives with {}.",
                s.tur.ad(dil),
                s.tur.paket()
            )
        };
        ui.label(egui::RichText::new(aciklama).color(tok.renk.metin_soluk));
        if s.kaydedilmemis {
            ui.add_space(tok.bosluk.xs);
            ui.colored_label(
                tok.renk.uyari,
                if tr {
                    "• Kaydedilmemiş değişiklik (kapatınca onay istenir)"
                } else {
                    "• Unsaved changes (closing asks for confirmation)"
                },
            );
        }
    });
}

/// Grup boşken "başlamak için…" rehberi (TDA madde 5).
fn bos_alan_rehberi(ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) {
    let tr = matches!(dil, Dil::Tr);
    EmptyState::yeni(
        "🧬",
        if tr {
            "Başlamak için bir sekme açın"
        } else {
            "Open a tab to get started"
        },
        if tr {
            "Üstteki ＋ ile yeni bir tuval açın ya da bir dosyayı buraya sürükleyip bırakın."
        } else {
            "Use ＋ above to open a new canvas, or drag and drop a file here."
        },
    )
    .show(ui, tok);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sekme(kimlik: u64, baslik: &str, sabit: bool, kaydedilmemis: bool) -> Sekme {
        Sekme {
            kimlik,
            baslik: baslik.to_string(),
            tur: SekmeTuru::Tuval,
            kaydedilmemis,
            sabit,
        }
    }

    #[test]
    fn ekle_yeni_sekmeyi_aktif_yapar() {
        let mut g = SekmeGrubu::yeni();
        g.ekle(sekme(1, "a", false, false));
        g.ekle(sekme(2, "b", false, false));
        assert_eq!(g.len(), 2);
        assert_eq!(g.aktif_sekme().unwrap().kimlik, 2);
    }

    #[test]
    fn kapat_aktif_komsuya_kayar() {
        let mut g = SekmeGrubu::yeni();
        for i in 1..=3 {
            g.ekle(sekme(i, "t", false, false));
        }
        g.aktif_yap(1); // ortadaki
        let kaldirilan = g.kapat(1).unwrap();
        assert_eq!(kaldirilan.kimlik, 2);
        assert_eq!(g.len(), 2);
        // Etkin, kapatılanın yerindeki (eski 3 → şimdi index 1) kalır.
        assert_eq!(g.aktif_sekme().unwrap().kimlik, 3);
    }

    #[test]
    fn sabit_sekmeler_basta_kalir() {
        let mut g = SekmeGrubu::yeni();
        g.ekle(sekme(1, "a", false, false));
        g.ekle(sekme(2, "b", false, false));
        g.ekle(sekme(3, "c", false, false));
        // 3 numaralıyı sabitle → başa gelmeli.
        let idx3 = g.sekmeler().iter().position(|s| s.kimlik == 3).unwrap();
        g.sabitle_degistir(idx3);
        assert_eq!(g.sekmeler()[0].kimlik, 3, "sabit sekme başa gelmeli");
        assert!(g.sekmeler()[0].sabit);
        // Etkin sekme kimliği korunur (sıralama değişse de).
        assert_eq!(g.aktif_sekme().unwrap().kimlik, 3);
    }

    #[test]
    fn yeniden_sirala_dizini_tasir_ve_aktifi_korur() {
        let mut g = SekmeGrubu::yeni();
        for i in 1..=3 {
            g.ekle(sekme(i, "t", false, false));
        }
        g.aktif_yap(0); // kimlik 1 etkin
        g.yeniden_sirala(0, 2); // 1'i sona taşı → [2,3,1]
        assert_eq!(g.sekmeler()[2].kimlik, 1);
        // Etkin, taşınan sekme (kimlik 1) olarak kalmalı.
        assert_eq!(g.aktif_sekme().unwrap().kimlik, 1);
    }

    #[test]
    fn yeniden_sirala_sabit_degismezini_bozmaz() {
        let mut g = SekmeGrubu::yeni();
        g.ekle(sekme(1, "pin", true, false)); // sabit
        g.ekle(sekme(2, "a", false, false));
        g.ekle(sekme(3, "b", false, false));
        // Sabit başta: [1, 2, 3]. 3'ü en başa taşımaya çalış → sabit yine başta kalmalı.
        g.yeniden_sirala(2, 0);
        assert!(g.sekmeler()[0].sabit, "sabit sekme her zaman başta");
        assert_eq!(g.sekmeler()[0].kimlik, 1);
    }

    #[test]
    fn kaydedilmemis_izlenir() {
        let mut g = SekmeGrubu::yeni();
        g.ekle(sekme(1, "a", false, true));
        assert!(g.kaydedilmemis_var());
        g.aktif_yap(0);
        assert!(g.aktifi_kaydet());
        assert!(!g.kaydedilmemis_var());
    }

    #[test]
    fn dosya_turu_bilinen_uzantilari_eslerken_bilinmeyeni_reddeder() {
        assert_eq!(dosya_turu("FASTA"), Some(SekmeTuru::Tuval));
        assert_eq!(dosya_turu("bam"), Some(SekmeTuru::Tuval));
        assert_eq!(dosya_turu("py"), Some(SekmeTuru::Kod));
        assert_eq!(dosya_turu("bcflow"), Some(SekmeTuru::Node));
        assert_eq!(dosya_turu("txt"), Some(SekmeTuru::Genel));
        assert_eq!(dosya_turu("exe"), None);
        assert_eq!(dosya_turu(""), None);
    }

    #[test]
    fn uzanti_al_yoldan_uzanti_cikarir() {
        // Ham uzantı (büyük/küçük korunur); küçük-harf eşleme dosya_turu içinde yapılır.
        assert_eq!(uzanti_al("/yol/x.FASTA"), "FASTA");
        assert_eq!(uzanti_al("c:\\veri\\ornek.bam"), "bam");
        assert_eq!(uzanti_al("uzantisiz"), "");
        assert_eq!(
            dosya_turu(uzanti_al("/yol/x.FASTA")),
            Some(SekmeTuru::Tuval)
        );
    }

    #[test]
    fn birakma_onizleme_gecerli_ve_gecersiz() {
        let ge = birakma_onizleme("ornek.vcf", Dil::Tr);
        assert!(ge.gecerli);
        assert_eq!(ge.tur, Some(SekmeTuru::Tuval));
        let gz = birakma_onizleme("kurulum.exe", Dil::Tr);
        assert!(!gz.gecerli);
        assert!(gz.tur.is_none());
    }

    #[test]
    fn bolme_ikincil_grubu_doldurur() {
        let mut e = EditorAlani::yeni();
        assert_eq!(e.yon, BolmeYonu::Yok);
        assert!(e.ikincil.is_empty());
        e.bolmeyi_degistir(Dil::Tr); // Yok → Yatay
        assert_eq!(e.yon, BolmeYonu::Yatay);
        assert!(
            !e.ikincil.is_empty(),
            "bölme açılınca ikincil grup örnek sekme alır"
        );
    }
}
