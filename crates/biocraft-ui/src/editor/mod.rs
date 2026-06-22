//! Native kod editörü (İP-06, MK-55, MK-02) — **1. kısım (Gün 22)**.
//!
//! Monaco/web YOK; tamamen **egui** ile native çizilir.  Bu kısımda kurulanlar:
//! - **Çoklu dosya/sekme** + **proje ağacı** ([`tree`]) + dosyalar arası gezinme.
//! - **Saf-Rust, artımlı söz dizimi vurgulama** ([`syntax`]; Python öncelikli; R/Bash/JSON/
//!   YAML/RON) — Tree-sitter v1.x kancası.
//! - **Kodu AYRI SÜREÇTE çalıştırma** ([`run`] → [`biocraft_plugin_host::exec`], MK-02):
//!   tam script + hücre; **arayüz donmaz**, **"Durdur"** her an; çıktı alt panelde akar.
//! - **Büyük dosya akışı** ([`AkisGoruntuleyici`], MK-09): 1 GB log RAM'e alınmadan,
//!   bellek-eşlemeli (mmap) ve yalnız **görünür pencere** çizilerek açılır.
//!
//! ## Yarın (Gün 23)
//! Node↔kod köprüsü (`bridge.rs`), temel LSP (pyright/jedi, out-of-process), izole ortam
//! (proje sanal ortamı + paket kurma) ve kaydet'te ruff/black biçimlendirme eklenecek.
//!
//! ## Düzenlenebilir tampon notu
//! Düzenlenebilir dosyalar egui'nin yerel metin motoruyla (`TextEdit` + söz dizimi
//! `layouter`'ı) düzenlenir; bu, imleç/seçim/geri-al'ı bedavaya getirir.  `ropey` rope
//! arka-uç + tamamen özel render, çok büyük **düzenlenebilir** dosyalar için bir v1.x
//! iyileştirmesidir (büyük **salt-okunur** dosyalar zaten akışla açılır).
// MK-40: L4 modülü; yalnız L0/L1/L2/L3'e bağlı.  MK-52 renk token'dan, MK-53 metin i18n'den.
// MK-02: kullanıcı kodu daima ayrı süreçte (biocraft-plugin-host/exec); in-process YASAK.

pub mod bridge;
pub mod history;
pub mod run;
pub mod syntax;
pub mod tree;

#[cfg(test)]
mod tests;

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biocraft_types::ErrorReport;
use egui::text::LayoutJob;
use egui::{FontId, TextFormat};

use crate::i18n::Dil;
use crate::node::{NodeGraf, NodeKimlik};
use crate::tokens::Tokenlar;
use biocraft_plugin_host::exec::{
    jedi_kur_rehberi, lsp_durumu, onek_al, tamamla_async, temel_tamamla, LspDurumu, SanalOrtam,
    Tamamlama, TamamlamaIstegi, TamamlamaTutamac, TamamlamaYaniti,
};
use biocraft_sdk::node::Parametreler;

pub use bridge::{CalismaAlani, DegiskenDeger, KodDugumTanimi};
pub use history::{GecmisKaydi, YerelGecmis};
pub use run::{Calisma, CalistirmaDurumu, CiktiSatiri};
pub use syntax::{
    vurgula, BasitVurgulayici, Jeton, JetonTuru, KodDili, SatirDurumu, VurgulamaOnbellek,
    Vurgulayici,
};
pub use tree::{AgacDugumu, ProjeAgaci};

// Çalıştırma kipi kontratını üst katmanlara aç (app menüsü/kısayolları için).
pub use biocraft_plugin_host::exec::CalismaModu;

/// Dosyanın bu boyuttan büyüğü **akışla** (salt-okunur, out-of-core) açılır (MK-09).
pub const AKIS_ESIGI: u64 = 8 * 1024 * 1024; // 8 MiB

/// Editör kod fontunun boyutu (px).
const KOD_FONT: f32 = 13.0;

// ─── Metin tamponu ──────────────────────────────────────────────────────────

/// Bir belgenin metin deposu: düzenlenebilir (RAM) **veya** akış (out-of-core, salt-okunur).
pub enum MetinTampon {
    /// RAM'e sığan, düzenlenebilir metin (egui `TextEdit` ile düzenlenir).
    Duzenlenebilir {
        /// Ham metin.
        metin: String,
    },
    /// Devasa, salt-okunur dosya — bellek-eşlemeli akış görüntüleyici (RAM'e yüklenmez).
    Akis(AkisGoruntuleyici),
}

impl MetinTampon {
    /// Boş, düzenlenebilir tampon.
    pub fn bos() -> Self {
        MetinTampon::Duzenlenebilir {
            metin: String::new(),
        }
    }

    /// Verilen metinden düzenlenebilir tampon.
    pub fn metinden(metin: String) -> Self {
        MetinTampon::Duzenlenebilir { metin }
    }

    /// Bu tampon salt-okunur mu (akış)?
    pub fn salt_okunur(&self) -> bool {
        matches!(self, MetinTampon::Akis(_))
    }

    /// Satır sayısı.
    pub fn satir_sayisi(&self) -> usize {
        match self {
            MetinTampon::Duzenlenebilir { metin } => {
                if metin.is_empty() {
                    1
                } else {
                    metin.split('\n').count()
                }
            }
            MetinTampon::Akis(a) => a.satir_sayisi(),
        }
    }

    /// `i`. satırı döner (sınır dışıysa boş).
    pub fn satir(&self, i: usize) -> Cow<'_, str> {
        match self {
            MetinTampon::Duzenlenebilir { metin } => metin
                .split('\n')
                .nth(i)
                .map(Cow::Borrowed)
                .unwrap_or(Cow::Borrowed("")),
            MetinTampon::Akis(a) => Cow::Owned(a.satir(i)),
        }
    }

    /// Toplam bayt (yaklaşık bellek/teşhis).
    pub fn uzunluk_bayt(&self) -> u64 {
        match self {
            MetinTampon::Duzenlenebilir { metin } => metin.len() as u64,
            MetinTampon::Akis(a) => a.bayt(),
        }
    }
}

// ─── Out-of-core akış görüntüleyici (MK-09) ───────────────────────────────────

/// Her `KABA_ARALIK` satırda bir bayt-ofset tutulur → 1 GB için ~birkaç bin giriş (RAM-dostu).
const KABA_ARALIK: usize = 1000;

/// Devasa dosyaları **RAM'e almadan** satır satır gösteren akış görüntüleyici (MK-09).
///
/// Dosya bellek-eşlenir (mmap); açılışta tek bir akış geçişiyle **kaba** bir satır-ofset
/// indeksi kurulur (her 1000 satırda bir ofset).  Bir satır istendiğinde en yakın kaba
/// ofsetten ileri taranır → tüm dosya asla bir `String`'e kopyalanmaz.  Yalnız **görünür**
/// satırlar çizildiği için 1 GB log bile akıcı açılır.
pub struct AkisGoruntuleyici {
    mmap: Option<memmap2::Mmap>,
    kaba: Vec<usize>,
    toplam_satir: usize,
    bayt: u64,
}

impl AkisGoruntuleyici {
    /// Bir dosyayı akışla açar (bellek-eşler + kaba indeks kurar).
    pub fn ac(yol: &Path) -> Result<Self, ErrorReport> {
        let dosya = std::fs::File::open(yol).map_err(|e| dosya_hata("Dosya açılamadı", yol, e))?;
        let boyut = dosya
            .metadata()
            .map(|m| m.len())
            .map_err(|e| dosya_hata("Dosya bilgisi okunamadı", yol, e))?;

        // Boş dosya: mmap edilemez (sıfır uzunluk) → boş görüntüleyici.
        if boyut == 0 {
            return Ok(Self {
                mmap: None,
                kaba: vec![0],
                toplam_satir: 1,
                bayt: 0,
            });
        }

        // SAFETY: dosya açıkken eşlenir; salt-okunur kullanırız.  Dış değişiklik nadirdir ve
        // en kötü durumda yanlış görüntü verir (çökme değil) — kullanıcı yeniden açar.
        let mmap = unsafe { memmap2::Mmap::map(&dosya) }
            .map_err(|e| dosya_hata("Dosya belleğe eşlenemedi", yol, e))?;

        // Tek akış geçişi: satır say + kaba ofset indeksini kur (RAM'de yalnız ~boyut/1000 ofset).
        let mut kaba = vec![0usize];
        let mut satir = 0usize;
        for (i, &b) in mmap.iter().enumerate() {
            if b == b'\n' {
                satir += 1;
                if satir % KABA_ARALIK == 0 {
                    kaba.push(i + 1);
                }
            }
        }
        // Son satır newline ile bitmiyorsa o da bir satırdır.
        let toplam_satir = if mmap.last() == Some(&b'\n') {
            satir
        } else {
            satir + 1
        };

        Ok(Self {
            mmap: Some(mmap),
            kaba,
            toplam_satir: toplam_satir.max(1),
            bayt: boyut,
        })
    }

    /// Eşlenen ham baytlar (boşsa boş dilim).
    fn veri(&self) -> &[u8] {
        self.mmap.as_ref().map(|m| &m[..]).unwrap_or(&[])
    }

    /// Toplam satır sayısı.
    pub fn satir_sayisi(&self) -> usize {
        self.toplam_satir
    }

    /// Dosya boyutu (bayt).
    pub fn bayt(&self) -> u64 {
        self.bayt
    }

    /// `i`. satırı (kaba indeksten ileri tarayarak) döner — UTF-8 kayıpsız değilse `lossy`.
    pub fn satir(&self, i: usize) -> String {
        let veri = self.veri();
        if veri.is_empty() || i >= self.toplam_satir {
            return String::new();
        }
        // En yakın kaba blok.
        let blok = i / KABA_ARALIK;
        let blok = blok.min(self.kaba.len() - 1);
        let mut p = self.kaba[blok];
        let mut atlanacak = i - blok * KABA_ARALIK;
        // `atlanacak` kadar satır başı ilerle.
        while atlanacak > 0 && p < veri.len() {
            if veri[p] == b'\n' {
                atlanacak -= 1;
            }
            p += 1;
        }
        // Satır sonunu bul.
        let bas = p;
        let mut q = p;
        while q < veri.len() && veri[q] != b'\n' {
            q += 1;
        }
        String::from_utf8_lossy(&veri[bas..q]).into_owned()
    }
}

// ─── Belge ────────────────────────────────────────────────────────────────────

/// Editörde açık tek bir belge (sekme).
pub struct Belge {
    /// Dosya yolu (kaydedilmemiş yeni belgede `None`).
    pub yol: Option<PathBuf>,
    /// Sekme başlığı.
    pub baslik: String,
    /// Kod dili (vurgulama + çalıştırma için).
    pub kod_dili: KodDili,
    /// Metin deposu.
    pub tampon: MetinTampon,
    /// Kaydedilmemiş değişiklik var mı?
    pub kirli: bool,
    /// Artımlı vurgulama önbelleği.
    onbellek: VurgulamaOnbellek,
}

impl Belge {
    /// Boş, isimsiz Python belgesi (scratch).
    pub fn bos() -> Self {
        Self {
            yol: None,
            baslik: "yeni.py".into(),
            kod_dili: KodDili::Python,
            tampon: MetinTampon::bos(),
            kirli: false,
            onbellek: VurgulamaOnbellek::yeni(),
        }
    }

    /// Başlık + dil + metinle belge (test/demo).
    pub fn metinli(baslik: impl Into<String>, kod_dili: KodDili, metin: impl Into<String>) -> Self {
        Self {
            yol: None,
            baslik: baslik.into(),
            kod_dili,
            tampon: MetinTampon::metinden(metin.into()),
            kirli: false,
            onbellek: VurgulamaOnbellek::yeni(),
        }
    }

    /// Örnek Python belgesi (demo/entegrasyon).
    pub fn ornek_python() -> Self {
        let kod = "# BioCraft — örnek Python betiği\n\
import sys\n\
\n\
def selamla(ad):\n\
    \"\"\"Bir ad alır, selam döner.\"\"\"\n\
    return f\"Merhaba {ad}!\"\n\
\n\
# %% Hücre 1: temel çıktı\n\
for i in range(3):\n\
    print(selamla(f\"dünya {i}\"))\n\
\n\
# %% Hücre 2: toplam\n\
toplam = sum(range(10))\n\
print(\"toplam =\", toplam)\n";
        Self::metinli("ornek.py", KodDili::Python, kod)
    }

    /// Bir dosyayı açar; boyutu eşiği aşıyorsa **akışla** (salt-okunur), değilse düzenlenebilir.
    pub fn dosyadan(yol: impl Into<PathBuf>) -> Result<Self, ErrorReport> {
        let yol = yol.into();
        let boyut = std::fs::metadata(&yol)
            .map(|m| m.len())
            .map_err(|e| dosya_hata("Dosya bilgisi okunamadı", &yol, e))?;
        if boyut > AKIS_ESIGI {
            Self::akis_dosyadan(yol)
        } else {
            let ham = std::fs::read(&yol).map_err(|e| dosya_hata("Dosya okunamadı", &yol, e))?;
            let metin = String::from_utf8_lossy(&ham).into_owned();
            Ok(Self {
                kod_dili: KodDili::yoldan(&yol),
                baslik: dosya_adi(&yol),
                tampon: MetinTampon::metinden(metin),
                kirli: false,
                onbellek: VurgulamaOnbellek::yeni(),
                yol: Some(yol),
            })
        }
    }

    /// Bir dosyayı **her zaman akışla** (salt-okunur) açar (devasa dosya / açık seçim).
    pub fn akis_dosyadan(yol: impl Into<PathBuf>) -> Result<Self, ErrorReport> {
        let yol = yol.into();
        let goruntu = AkisGoruntuleyici::ac(&yol)?;
        Ok(Self {
            kod_dili: KodDili::yoldan(&yol),
            baslik: dosya_adi(&yol),
            tampon: MetinTampon::Akis(goruntu),
            kirli: false,
            onbellek: VurgulamaOnbellek::yeni(),
            yol: Some(yol),
        })
    }

    /// Düzenlenebilir metne erişim (akış belgesinde `None`).
    pub fn metin(&self) -> Option<&str> {
        match &self.tampon {
            MetinTampon::Duzenlenebilir { metin } => Some(metin),
            MetinTampon::Akis(_) => None,
        }
    }

    /// Kaydedildi işaretler (gerçek diske yazma Gün 23 — ruff/black kaydet kancası).
    pub fn kaydedildi_isaretle(&mut self) {
        self.kirli = false;
    }
}

// ─── Kod editörü (tuval) ───────────────────────────────────────────────────────

/// Native kod editörü: sekmeler + proje ağacı + kod alanı + çıktı paneli + çalıştırma.
pub struct KodEditoru {
    /// Açık belgeler (sekmeler).
    pub belgeler: Vec<Belge>,
    /// Etkin sekme indeksi.
    pub aktif: usize,
    /// Proje dosya ağacı.
    pub agac: ProjeAgaci,
    /// Çalıştırma durumu + çıktı.
    pub calistirma: CalistirmaDurumu,
    /// **Ortak çalışma alanı** (node ↔ kod köprüsü) — tipli paylaşılan değişkenler.
    pub calisma_alani: CalismaAlani,
    /// Etkin belgenin yerel geçmişi (kaba taneli anlık görüntüler).
    pub gecmis: YerelGecmis,
    /// Projenin izole Python ortamı (varsa) — çalıştırma + jedi bu yorumlayıcıyı kullanır.
    ortam: Option<SanalOrtam>,
    /// jedi (akıllı tamamlama) durumu — [Kur] kararı için (proje ortamına göre).
    lsp: LspDurumu,
    /// İmlecin o anki öneki için temel tamamlama önerileri.
    tamamlamalar: Vec<Tamamlama>,
    /// Bekleyen jedi (out-of-process) tamamlama isteği — asenkron, UI'yi bloklamaz.
    lsp_tutamac: Option<TamamlamaTutamac>,
    /// Sağ köprü/ortam panelini göster.
    pub kopru_paneli_acik: bool,
    /// Sistemde Python bulundu mu (çalıştır düğmesinin etkinliği için).
    python_var: bool,
    /// Etkin düzenlenebilir belgede son bilinen imleç bayt ofseti (hücre seçimi için).
    imlec_bayt: usize,
    /// Bir çalışma bitince kod→node köprüsünü (sentinel) bir kez taramak için bayrak.
    calisma_aktifti: bool,
}

impl Default for KodEditoru {
    fn default() -> Self {
        Self::yeni()
    }
}

impl KodEditoru {
    /// Boş bir Python sekmesiyle yeni editör.
    pub fn yeni() -> Self {
        let python_var = biocraft_plugin_host::python_bul().is_some();
        Self {
            belgeler: vec![Belge::bos()],
            aktif: 0,
            agac: ProjeAgaci::bos(),
            calistirma: CalistirmaDurumu::yeni(),
            calisma_alani: CalismaAlani::yeni(),
            gecmis: YerelGecmis::yeni(),
            ortam: None,
            // Henüz proje ortamı yok → sistem Python'una göre jedi durumu (genelde JediYok).
            lsp: if python_var {
                LspDurumu::JediYok
            } else {
                LspDurumu::PythonYok
            },
            tamamlamalar: Vec::new(),
            lsp_tutamac: None,
            kopru_paneli_acik: true,
            python_var,
            imlec_bayt: 0,
            calisma_aktifti: false,
        }
    }

    /// Örnek Python belgesiyle editör (demo/entegrasyon).
    pub fn ornek() -> Self {
        let mut e = Self::yeni();
        e.belgeler = vec![Belge::ornek_python()];
        e.aktif = 0;
        e
    }

    /// Etkin belge.
    pub fn aktif_belge(&self) -> Option<&Belge> {
        self.belgeler.get(self.aktif)
    }

    /// Etkin belge (değiştirilebilir).
    pub fn aktif_belge_mut(&mut self) -> Option<&mut Belge> {
        self.belgeler.get_mut(self.aktif)
    }

    /// Bir dosyayı yeni sekmede açar (zaten açıksa o sekmeye geçer).
    pub fn dosya_ac(&mut self, yol: impl Into<PathBuf>) -> Result<(), ErrorReport> {
        let yol = yol.into();
        if let Some(idx) = self
            .belgeler
            .iter()
            .position(|b| b.yol.as_deref() == Some(yol.as_path()))
        {
            self.aktif = idx;
            return Ok(());
        }
        let belge = Belge::dosyadan(yol)?;
        self.belgeler.push(belge);
        self.aktif = self.belgeler.len() - 1;
        Ok(())
    }

    /// Yeni boş sekme.
    pub fn yeni_sekme(&mut self) {
        self.belgeler.push(Belge::bos());
        self.aktif = self.belgeler.len() - 1;
    }

    /// Bir sekmeyi kapatır (son sekme kapanırsa boş bir tane bırakılır).
    pub fn sekme_kapat(&mut self, idx: usize) {
        if idx >= self.belgeler.len() {
            return;
        }
        self.belgeler.remove(idx);
        if self.belgeler.is_empty() {
            self.belgeler.push(Belge::bos());
        }
        if self.aktif >= self.belgeler.len() {
            self.aktif = self.belgeler.len() - 1;
        }
    }

    /// Her karede çağrılır: çalışan sürecin çıktısını bloklamadan toplar.
    pub fn yokla(&mut self) {
        self.calistirma.yokla();
    }

    /// Etkin belgenin tamamını **tam script** olarak ayrı süreçte çalıştırır.
    pub fn calistir_tam(&mut self) {
        if let Some(kod) = self
            .aktif_belge()
            .and_then(|b| b.metin().map(str::to_owned))
        {
            self.gecmis.anlik_al("çalıştırma", &kod);
            self.calisma_aktifti = true;
            self.calistirma.baslat(&kod, CalismaModu::TamScript);
        }
    }

    /// İmlecin bulunduğu **hücreyi** (`# %%` ile ayrılan) çalıştırır; yoksa tüm metni.
    pub fn calistir_hucre(&mut self) {
        let imlec = self.imlec_bayt;
        if let Some(tam) = self
            .aktif_belge()
            .and_then(|b| b.metin().map(str::to_owned))
        {
            self.gecmis.anlik_al("hücre", &tam);
            let hucre = hucre_bul(&tam, imlec).to_owned();
            self.calisma_aktifti = true;
            self.calistirma.baslat(&hucre, CalismaModu::Hucre);
        }
    }

    // ─── Köprü + izole ortam (Gün 23) ──────────────────────────────────────────

    /// Projenin izole Python ortamını ayarlar: çalıştırma + jedi bu yorumlayıcıyı kullanır.
    ///
    /// Ortam henüz oluşturulmamış olabilir (`var_mi()==false`); o durumda sistem Python'una
    /// düşülür ve jedi durumu yeniden hesaplanır ([Kur] yönlendirmesi için).
    pub fn proje_ortami_ayarla(&mut self, ortam: SanalOrtam) {
        let yorumlayici = ortam.var_mi().then(|| ortam.yorumlayici());
        self.lsp = lsp_durumu(yorumlayici.as_deref());
        self.calistirma.yorumlayici_ayarla(yorumlayici);
        self.ortam = Some(ortam);
    }

    /// Çalıştırma/tamamlama için kullanılacak yorumlayıcı (proje ortamı varsa onunki).
    fn aktif_yorumlayici(&self) -> Option<PathBuf> {
        match &self.ortam {
            Some(o) if o.var_mi() => Some(o.yorumlayici()),
            _ => biocraft_plugin_host::python_bul(),
        }
    }

    /// Ortak çalışma alanını ayarlar (örn. node çalıştırmasından gelen çıktılar).
    pub fn calisma_alanini_ayarla(&mut self, alan: CalismaAlani) {
        self.calisma_alani = alan;
    }

    /// **"Bu node'u kod olarak aç":** akışı köprülü Python betiğine çevirip yeni sekmede açar.
    ///
    /// Önsöz, `sonuc` verilirse node çıktılarını workspace olarak taşır (node → kod); sonsöz
    /// çalışma sonunda workspace'i geri basar (kod → node).
    pub fn node_olarak_ac(
        &mut self,
        graf: &NodeGraf,
        parametreler: &HashMap<NodeKimlik, Parametreler>,
        sonuc: Option<&crate::node::CalismaSonucu>,
    ) {
        // Köprü workspace'ini de güncelle (panelde görünür + sonraki çalıştırma okuyabilir).
        if let Some(s) = sonuc {
            self.calisma_alani = bridge::node_ciktilarini_al(graf, s);
        }
        let kod = bridge::node_olarak_kod(graf, parametreler, sonuc);
        let mut belge = Belge::metinli("akis.py", KodDili::Python, kod);
        belge.kirli = true;
        self.belgeler.push(belge);
        self.aktif = self.belgeler.len() - 1;
    }

    /// **"Bu kodu node akışına ekle":** etkin belgenin kodunu betik node'u tanımına sarar.
    ///
    /// Tam tersine-çevirme (kodu ayrıştırma) v1.x'tir; bu, kodu **olduğu gibi saran** bir node
    /// tanımı döner (girişler workspace değişkenlerinden).
    pub fn kod_dugumu_yap(&self) -> Option<KodDugumTanimi> {
        let kod = self.aktif_belge().and_then(|b| b.metin())?;
        let baslik = self
            .aktif_belge()
            .map(|b| b.baslik.clone())
            .unwrap_or_else(|| "Betik".into());
        Some(KodDugumTanimi::koddan(baslik, kod, &self.calisma_alani))
    }

    /// Çalışma bittiyse çıktıdaki **sentinel**'i tarayıp workspace'i günceller (kod → node).
    fn kopru_sonucunu_topla(&mut self) {
        if self.calisma_aktifti && !self.calistirma.calisiyor() {
            self.calisma_aktifti = false;
            let satirlar = self.calistirma.cikti.iter().map(|c| c.metin.as_str());
            if let Some(alan) = bridge::cikti_workspace_ayikla(satirlar) {
                self.calisma_alani = alan;
            }
        }
    }

    /// İmlecin o anki öneki için **temel** tamamlama önerilerini yeniler (saf-Rust, her zaman çalışır).
    fn tamamlamalari_guncelle(&mut self) {
        let Some(kod) = self.aktif_belge().and_then(|b| b.metin()) else {
            self.tamamlamalar.clear();
            return;
        };
        let (satir_metni, sutun) = imlec_satir_sutun(kod, self.imlec_bayt);
        let onek = onek_al(&satir_metni, sutun);
        if onek.len() < 2 {
            self.tamamlamalar.clear();
            return;
        }
        self.tamamlamalar = temel_tamamla(kod, &onek);
    }

    /// **jedi (out-of-process) akıllı tamamlama** başlatır — UI bloklanmaz; sonuç sonradan toplanır.
    ///
    /// Yalnız jedi hazırsa anlamlıdır; değilse sessizce yok sayılır (temel tamamlama sürer).
    pub fn jedi_oner(&mut self) {
        if !matches!(self.lsp, LspDurumu::Hazir) {
            return;
        }
        let Some(yorumlayici) = self.aktif_yorumlayici() else {
            return;
        };
        let Some(kod) = self
            .aktif_belge()
            .and_then(|b| b.metin())
            .map(str::to_owned)
        else {
            return;
        };
        let (satir, sutun) = imlec_satir_no_sutun(&kod, self.imlec_bayt);
        self.lsp_tutamac = Some(tamamla_async(
            &yorumlayici,
            TamamlamaIstegi { kod, satir, sutun },
        ));
    }

    /// Bekleyen jedi sonucunu **bloklamadan** yoklar; geldiyse öneri listesini günceller (MK-07).
    fn lsp_yokla(&mut self, ctx: &egui::Context) {
        let Some(t) = &self.lsp_tutamac else { return };
        match t.dene() {
            Some(TamamlamaYaniti::Hazir(oneriler)) => {
                if !oneriler.is_empty() {
                    self.tamamlamalar = oneriler;
                }
                self.lsp_tutamac = None;
            }
            Some(TamamlamaYaniti::JediYok) => {
                self.lsp = LspDurumu::JediYok;
                self.lsp_tutamac = None;
            }
            Some(TamamlamaYaniti::Hata(_)) => {
                // Yedek temel tamamlama zaten gösteriliyor; sessizce bırak.
                self.lsp_tutamac = None;
            }
            None => {
                // Henüz hazır değil → bir sonraki kareyi iste (donma yok).
                ctx.request_repaint();
            }
        }
    }

    /// Editörü çizer.  Açılan dosya olursa (ağaçtan tıklama) onu açar.
    pub fn ciz(&mut self, ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) {
        // Çıktıyı bloklamadan topla; çalışıyorsa sürekli yeniden çiz (akış görünür olsun).
        self.yokla();
        if self.calistirma.calisiyor() {
            ui.ctx().request_repaint();
        }
        // Çalışma bittiyse kod→node köprüsünü (sentinel) bir kez tara.
        self.kopru_sonucunu_topla();
        // İmleç önekine göre temel tamamlama önerilerini yenile.
        self.tamamlamalari_guncelle();
        // Bekleyen jedi (out-of-process) sonucunu bloklamadan yokla.
        self.lsp_yokla(ui.ctx());

        // Üst: sekme şeridi + araç çubuğu.
        egui::TopBottomPanel::top("kod_ust")
            .resizable(false)
            .show_inside(ui, |ui| {
                self.sekme_seridi(ui, dil, tok);
                ui.separator();
                self.arac_cubugu(ui, dil, tok);
            });

        // Sol: proje ağacı.
        let mut acilacak: Option<PathBuf> = None;
        egui::SidePanel::left("kod_agac")
            .resizable(true)
            .default_width(190.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    acilacak = self.agac.ciz(ui, dil, tok);
                });
            });

        // Sağ: köprü (ortak workspace) + izole ortam + tamamlama + yerel geçmiş paneli.
        if self.kopru_paneli_acik {
            egui::SidePanel::right("kod_kopru")
                .resizable(true)
                .default_width(240.0)
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        self.kopru_ortam_paneli(ui, dil, tok);
                    });
                });
        }

        // Alt: çıktı paneli.
        egui::TopBottomPanel::bottom("kod_cikti")
            .resizable(true)
            .default_height(170.0)
            .show_inside(ui, |ui| {
                cikti_paneli(ui, &self.calistirma, dil, tok);
            });

        // Merkez: kod alanı (düzenlenebilir veya akış).
        egui::CentralPanel::default().show_inside(ui, |ui| {
            kod_alani_ciz(
                ui,
                &mut self.belgeler,
                self.aktif,
                &mut self.imlec_bayt,
                dil,
                tok,
            );
        });

        if let Some(yol) = acilacak {
            // Hata olursa çıktı paneline yansıt (sessiz yutma yok).
            if let Err(r) = self.dosya_ac(yol) {
                self.calistirma
                    .olaylari_uygula([biocraft_plugin_host::exec::CalismaOlay::Hata(r)]);
            }
        }
    }

    /// Sekme şeridi (etkin sekmeyi seç, • = kaydedilmemiş, × = kapat).
    fn sekme_seridi(&mut self, ui: &mut egui::Ui, _dil: Dil, tok: &Tokenlar) {
        let mut secilecek = self.aktif;
        let mut kapanacak: Option<usize> = None;
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, b) in self.belgeler.iter().enumerate() {
                    let etkin = i == self.aktif;
                    let isaret = if b.kirli { " •" } else { "" };
                    let etiket = format!("{}{isaret}", b.baslik);
                    let renk = if etkin {
                        tok.renk.vurgu
                    } else {
                        tok.renk.metin_soluk
                    };
                    if ui
                        .selectable_label(etkin, egui::RichText::new(etiket).color(renk))
                        .clicked()
                    {
                        secilecek = i;
                    }
                    if ui
                        .add(egui::Button::new("×").small().frame(false))
                        .on_hover_text("Kapat")
                        .clicked()
                    {
                        kapanacak = Some(i);
                    }
                    ui.separator();
                }
            });
        });
        self.aktif = secilecek;
        if let Some(i) = kapanacak {
            self.sekme_kapat(i);
        }
    }

    /// Araç çubuğu (dil + Çalıştır/Hücre/Durdur + durum + Python yok uyarısı).
    fn arac_cubugu(&mut self, ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) {
        let tr = matches!(dil, Dil::Tr);
        let duzenlenebilir = self
            .aktif_belge()
            .map(|b| !b.tampon.salt_okunur())
            .unwrap_or(false);
        let python = self
            .aktif_belge()
            .map(|b| b.kod_dili == KodDili::Python)
            .unwrap_or(false);
        let calistirilabilir =
            duzenlenebilir && python && self.python_var && !self.calistirma.calisiyor();

        let mut tam = false;
        let mut hucre = false;
        let mut durdur = false;

        ui.horizontal(|ui| {
            if let Some(b) = self.aktif_belge() {
                ui.label(
                    egui::RichText::new(b.kod_dili.ad())
                        .color(tok.renk.metin_soluk)
                        .small(),
                );
                if b.tampon.salt_okunur() {
                    ui.label(
                        egui::RichText::new(if tr {
                            "● salt-okunur (akış)"
                        } else {
                            "● read-only (stream)"
                        })
                        .color(tok.renk.bilgi)
                        .small(),
                    );
                }
            }
            ui.separator();

            let calistir_btn = egui::Button::new(if tr { "▶ Çalıştır" } else { "▶ Run" });
            if ui
                .add_enabled(calistirilabilir, calistir_btn)
                .on_hover_text(if tr {
                    "Tüm betiği ayrı süreçte çalıştır"
                } else {
                    "Run whole script out-of-process"
                })
                .clicked()
            {
                tam = true;
            }
            let hucre_btn = egui::Button::new(if tr { "▶ Hücre" } else { "▶ Cell" });
            if ui
                .add_enabled(calistirilabilir, hucre_btn)
                .on_hover_text(if tr {
                    "İmleçteki # %% hücresini çalıştır"
                } else {
                    "Run the # %% cell at cursor"
                })
                .clicked()
            {
                hucre = true;
            }
            let durdur_btn = egui::Button::new(
                egui::RichText::new(if tr { "■ Durdur" } else { "■ Stop" }).color(tok.renk.hata),
            );
            if ui
                .add_enabled(self.calistirma.calisiyor(), durdur_btn)
                .clicked()
            {
                durdur = true;
            }

            ui.separator();
            ui.label(
                egui::RichText::new(self.calistirma.durum.etiket(tr))
                    .color(durum_rengi(&self.calistirma.durum, tok)),
            );

            // Python yoksa: [Kur] yönlendirmesi (tam rehber Gün 23) — TDA madde 1.
            if duzenlenebilir && python && !self.python_var {
                ui.separator();
                ui.label(
                    egui::RichText::new(if tr {
                        "⚠ Python bulunamadı — [Kur rehberi]"
                    } else {
                        "⚠ Python not found — [Setup guide]"
                    })
                    .color(tok.renk.uyari)
                    .small(),
                );
            }

            // Köprü/ortam paneli aç-kapa.
            ui.separator();
            if ui
                .selectable_label(
                    self.kopru_paneli_acik,
                    if tr { "⇄ Köprü" } else { "⇄ Bridge" },
                )
                .on_hover_text(if tr {
                    "Ortak çalışma alanı + izole ortam + tamamlama paneli"
                } else {
                    "Shared workspace + isolated env + completion panel"
                })
                .clicked()
            {
                self.kopru_paneli_acik = !self.kopru_paneli_acik;
            }
        });

        // İkinci satır: LSP durumu + debugger/AI yüzeyi (etiketli — v1.x / yapılandırılmadı).
        ui.horizontal(|ui| {
            // Temel LSP (jedi) durumu — tam zekâ v1.x.
            let (lsp_metin, lsp_renk) = match self.lsp {
                LspDurumu::Hazir => (
                    if tr { "⌨ LSP: jedi hazır (temel)" } else { "⌨ LSP: jedi ready (basic)" },
                    tok.renk.basari,
                ),
                LspDurumu::JediYok => (
                    if tr { "⌨ LSP: temel — [jedi Kur]" } else { "⌨ LSP: basic — [Install jedi]" },
                    tok.renk.uyari,
                ),
                LspDurumu::PythonYok => (
                    if tr { "⌨ LSP: Python yok" } else { "⌨ LSP: no Python" },
                    tok.renk.metin_soluk,
                ),
            };
            ui.label(egui::RichText::new(lsp_metin).color(lsp_renk).small())
                .on_hover_text(if tr {
                    "Temel tamamlama her zaman çalışır (saf-Rust). jedi: bağlam-duyarlı (ayrı süreç). Tam dil zekâsı v1.x."
                } else {
                    "Basic completion always works (pure-Rust). jedi: context-aware (separate process). Full intelligence v1.x."
                });
            ui.separator();
            // Debugger v1.x (MVP'de log-tabanlı).
            ui.label(
                egui::RichText::new(if tr { "🐞 Hata ayıklama: v1.x" } else { "🐞 Debugger: v1.x" })
                    .color(tok.renk.metin_soluk)
                    .small(),
            )
            .on_hover_text(if tr {
                "Adım adım hata ayıklama v1.x'e ertelendi; MVP'de çıktı/log tabanlı."
            } else {
                "Step debugger deferred to v1.x; MVP is log-based."
            });
            ui.separator();
            // AI yardımı yüzeyi — yapılandırılmadı (MK-48).
            ui.label(
                egui::RichText::new(if tr { "✨ AI: yapılandırılmadı" } else { "✨ AI: not configured" })
                    .color(tok.renk.metin_soluk)
                    .small(),
            )
            .on_hover_text(if tr {
                "Yapay zekâ yardımı yüzeyi hazır; gerçek AI İP-14'te bağlanır (MK-48)."
            } else {
                "AI help surface is present; real AI connects in İP-14 (MK-48)."
            });
        });

        if tam {
            self.calistir_tam();
        }
        if hucre {
            self.calistir_hucre();
        }
        if durdur {
            self.calistirma.durdur();
        }
    }

    /// Sağ panel: ortak workspace (köprü) + izole ortam + temel tamamlama + yerel geçmiş.
    fn kopru_ortam_paneli(&mut self, ui: &mut egui::Ui, dil: Dil, tok: &Tokenlar) {
        let tr = matches!(dil, Dil::Tr);

        // ── Ortak çalışma alanı (node ↔ kod köprüsü) ──
        ui.label(
            egui::RichText::new(if tr {
                "Ortak çalışma alanı"
            } else {
                "Shared workspace"
            })
            .strong()
            .color(tok.renk.metin),
        );
        ui.label(
            egui::RichText::new(if tr {
                "Node ↔ kod arasında paylaşılan tipli değişkenler"
            } else {
                "Typed variables shared between node ↔ code"
            })
            .color(tok.renk.metin_soluk)
            .small(),
        );
        if self.calisma_alani.is_empty() {
            ui.label(
                egui::RichText::new(if tr {
                    "(boş — bir akış çalıştırıp \"kod olarak aç\" deneyin)"
                } else {
                    "(empty — run a flow and \"open as code\")"
                })
                .italics()
                .color(tok.renk.metin_soluk)
                .small(),
            );
        } else {
            for (ad, deg) in self.calisma_alani.tumu() {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(ad).monospace().color(tok.renk.vurgu));
                    ui.label(
                        egui::RichText::new(format!(": {}", deg.tur_adi()))
                            .small()
                            .color(tok.renk.metin_soluk),
                    );
                });
                ui.label(
                    egui::RichText::new(format!("   {}", deg.ozet()))
                        .monospace()
                        .small()
                        .color(tok.renk.metin),
                );
            }
        }

        ui.separator();

        // ── İzole ortam (sanal ortam + paketler + sürüm kilidi) ──
        ui.label(
            egui::RichText::new(if tr {
                "İzole ortam"
            } else {
                "Isolated environment"
            })
            .strong()
            .color(tok.renk.metin),
        );
        match &self.ortam {
            None => {
                ui.label(
                    egui::RichText::new(if tr {
                        "Proje açık değil — ortam projeyle gelir."
                    } else {
                        "No project open — env comes with a project."
                    })
                    .color(tok.renk.metin_soluk)
                    .small(),
                );
            }
            Some(o) if o.var_mi() => {
                ui.label(
                    egui::RichText::new(if tr {
                        "● Sanal ortam hazır (.venv)"
                    } else {
                        "● Virtual env ready (.venv)"
                    })
                    .color(tok.renk.basari)
                    .small(),
                );
                if let Ok(kilit) = o.kilit_oku() {
                    ui.label(
                        egui::RichText::new(if tr {
                            format!("Kilitli paket: {}", kilit.len())
                        } else {
                            format!("Locked packages: {}", kilit.len())
                        })
                        .small()
                        .color(tok.renk.metin_soluk),
                    );
                }
            }
            Some(_) => {
                ui.label(
                    egui::RichText::new(if tr {
                        "○ Ortam kurulmadı — [Ortamı kur]"
                    } else {
                        "○ Env not created — [Create env]"
                    })
                    .color(tok.renk.uyari)
                    .small(),
                );
            }
        }
        // jedi [Kur] yönlendirmesi (eksik araç → [Kur] — TDA madde 1).
        if matches!(self.lsp, LspDurumu::JediYok) {
            let r = jedi_kur_rehberi();
            ui.label(
                egui::RichText::new(format!("[Kur] {}", r.nasil_cozulur))
                    .small()
                    .color(tok.renk.uyari),
            )
            .on_hover_text(r.neden);
        }

        ui.separator();

        // ── Temel tamamlama (LSP yüzeyi) ──
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(if tr {
                    "Tamamlama (temel)"
                } else {
                    "Completion (basic)"
                })
                .strong()
                .color(tok.renk.metin),
            );
            // jedi akıllı öneri (out-of-process) — yalnız hazırsa etkin.
            let jedi_hazir = matches!(self.lsp, LspDurumu::Hazir);
            if ui
                .add_enabled(
                    jedi_hazir && self.lsp_tutamac.is_none(),
                    egui::Button::new("🧠 jedi").small(),
                )
                .on_hover_text(if tr {
                    "Bağlam-duyarlı öneri (ayrı süreçte jedi)"
                } else {
                    "Context-aware suggestion (jedi, separate process)"
                })
                .clicked()
            {
                self.jedi_oner();
            }
            if self.lsp_tutamac.is_some() {
                ui.label(egui::RichText::new("…").color(tok.renk.bilgi).small());
            }
        });
        if self.tamamlamalar.is_empty() {
            ui.label(
                egui::RichText::new(if tr {
                    "Yazarken öneriler burada belirir."
                } else {
                    "Suggestions appear here as you type."
                })
                .italics()
                .color(tok.renk.metin_soluk)
                .small(),
            );
        } else {
            for t in self.tamamlamalar.iter().take(12) {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&t.etiket)
                            .monospace()
                            .color(tok.anahtar_renk(t.tur.token_anahtari())),
                    );
                    if !t.detay.is_empty() {
                        ui.label(
                            egui::RichText::new(&t.detay)
                                .small()
                                .color(tok.renk.metin_soluk),
                        );
                    }
                });
            }
        }

        ui.separator();

        // ── Yerel geçmiş (kod düzenlemelerine bağlı anlık görüntüler) ──
        ui.label(
            egui::RichText::new(if tr {
                "Yerel geçmiş"
            } else {
                "Local history"
            })
            .strong()
            .color(tok.renk.metin),
        );
        let mut geri_yukle: Option<String> = None;
        if self.gecmis.is_empty() {
            ui.label(
                egui::RichText::new(if tr {
                    "Çalıştırınca anlık görüntü alınır."
                } else {
                    "A snapshot is taken on run."
                })
                .italics()
                .color(tok.renk.metin_soluk)
                .small(),
            );
        } else {
            // En yeniden eskiye.
            for kayit in self.gecmis.kayitlar().iter().rev().take(10) {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("#{} {}", kayit.sira, kayit.etiket))
                            .small()
                            .color(tok.renk.metin_soluk),
                    );
                    if ui
                        .small_button(if tr { "Geri yükle" } else { "Restore" })
                        .clicked()
                    {
                        geri_yukle = Some(kayit.metin.clone());
                    }
                });
            }
        }
        if let Some(metin) = geri_yukle {
            // Geri yüklemeden önce mevcut hali de geçmişe al (kayıp olmasın).
            if let Some(suanki) = self
                .aktif_belge()
                .and_then(|b| b.metin())
                .map(str::to_owned)
            {
                self.gecmis.anlik_al("geri-yükleme öncesi", &suanki);
            }
            if let Some(b) = self.aktif_belge_mut() {
                if let MetinTampon::Duzenlenebilir { metin: m } = &mut b.tampon {
                    *m = metin;
                    b.kirli = true;
                }
            }
        }
    }
}

// ─── Çizim yardımcıları ────────────────────────────────────────────────────────

/// Merkez kod alanını çizer: düzenlenebilir (`TextEdit` + vurgulama) veya akış (görünür pencere).
fn kod_alani_ciz(
    ui: &mut egui::Ui,
    belgeler: &mut [Belge],
    aktif: usize,
    imlec_bayt: &mut usize,
    dil: Dil,
    tok: &Tokenlar,
) {
    let Some(belge) = belgeler.get_mut(aktif) else {
        return;
    };
    // Disjoint alan ödünçleri: tampon + önbellek + kod_dili ayrı ayrı.
    let Belge {
        tampon,
        onbellek,
        kod_dili,
        kirli,
        ..
    } = belge;
    let kod_dili = *kod_dili;

    match tampon {
        MetinTampon::Duzenlenebilir { metin } => {
            // Artımlı vurgulamayı güncelle (yalnız değişen satırlar yeniden jetonlanır).
            onbellek.guncelle(metin, kod_dili, &BasitVurgulayici);
            let onbellek_ref: &VurgulamaOnbellek = onbellek;

            let mut layouter = move |ui: &egui::Ui, text: &str, wrap: f32| -> Arc<egui::Galley> {
                let job = layout_job_kur(text, onbellek_ref, tok, wrap);
                ui.fonts(|f| f.layout_job(job))
            };

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let cikti = egui::TextEdit::multiline(metin)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter)
                        .show(ui);
                    if cikti.response.changed() {
                        *kirli = true;
                    }
                    // İmleç bayt ofsetini sakla (hücre seçimi için).
                    if let Some(aralik) = cikti.cursor_range {
                        let kar = aralik.primary.ccursor.index;
                        *imlec_bayt = bayt_ofseti(metin, kar);
                    }
                });
        }
        MetinTampon::Akis(goruntu) => {
            // Salt-okunur, yalnız görünür pencere çizilir (1 GB log için MK-09).
            let satir_yuk = ui.text_style_height(&egui::TextStyle::Monospace);
            let toplam = goruntu.satir_sayisi();
            let tr = matches!(dil, Dil::Tr);
            ui.label(
                egui::RichText::new(format!(
                    "{}  ({} {}, {:.1} MB)",
                    if tr {
                        "Akış görüntüleyici"
                    } else {
                        "Stream viewer"
                    },
                    toplam,
                    if tr { "satır" } else { "lines" },
                    goruntu.bayt() as f64 / (1024.0 * 1024.0),
                ))
                .color(tok.renk.metin_soluk)
                .small(),
            );
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show_rows(ui, satir_yuk, toplam, |ui, aralik| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for i in aralik {
                        let satir = goruntu.satir(i);
                        akis_satir_ciz(ui, i, &satir, kod_dili, tok);
                    }
                });
        }
    }
}

/// Akış görüntüleyicide tek bir satırı (numara + renkli jetonlar) çizer.
fn akis_satir_ciz(ui: &mut egui::Ui, no: usize, satir: &str, dil: KodDili, tok: &Tokenlar) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        // Satır numarası (soluk, sabit genişlik hissi).
        ui.label(
            egui::RichText::new(format!("{:>7} ", no + 1))
                .monospace()
                .color(tok.renk.metin_soluk),
        );
        // Jetonlar (her biri renkli) — boşluk dahil tüm satırı kapsar.
        let jetonlar = vurgula(satir, dil);
        if jetonlar.is_empty() {
            ui.label(egui::RichText::new(satir).monospace().color(tok.renk.metin));
        } else {
            for j in &jetonlar {
                let (bas, son) = guvenli_aralik(satir, j.baslangic, j.son);
                if bas < son {
                    let renk = tok.anahtar_renk(j.tur.token_anahtari());
                    ui.label(
                        egui::RichText::new(&satir[bas..son])
                            .monospace()
                            .color(renk),
                    );
                }
            }
        }
    });
}

/// Önbellekteki jetonlardan, `text`'i renkli kuran bir [`LayoutJob`] üretir (boşlukları doldurur).
fn layout_job_kur(
    text: &str,
    onbellek: &VurgulamaOnbellek,
    tok: &Tokenlar,
    wrap: f32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap;
    let font = FontId::monospace(KOD_FONT);
    let metin_renk = tok.renk.metin;

    for (i, satir) in text.split('\n').enumerate() {
        if i > 0 {
            job.append("\n", 0.0, bicim(&font, metin_renk));
        }
        let jetonlar = onbellek.satir_jetonlari(i);
        let mut imlec = 0usize;
        for j in jetonlar {
            let (bas, son) = guvenli_aralik(satir, j.baslangic, j.son);
            if bas > imlec {
                job.append(&satir[imlec..bas], 0.0, bicim(&font, metin_renk));
            }
            if son > bas {
                let renk = tok.anahtar_renk(j.tur.token_anahtari());
                job.append(&satir[bas..son], 0.0, bicim(&font, renk));
                imlec = son;
            }
        }
        if imlec < satir.len() {
            job.append(&satir[imlec..], 0.0, bicim(&font, metin_renk));
        }
    }
    job
}

/// Tek tip metin biçimi (font + renk).
fn bicim(font: &FontId, renk: egui::Color32) -> TextFormat {
    TextFormat {
        font_id: font.clone(),
        color: renk,
        ..Default::default()
    }
}

/// Çıktı panelini çizer (stdout/stderr renkli; durum başlığı).
fn cikti_paneli(ui: &mut egui::Ui, calistirma: &CalistirmaDurumu, dil: Dil, tok: &Tokenlar) {
    let tr = matches!(dil, Dil::Tr);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(if tr { "Çıktı" } else { "Output" })
                .color(tok.renk.metin_soluk)
                .small(),
        );
        ui.label(
            egui::RichText::new(calistirma.durum.etiket(tr))
                .color(durum_rengi(&calistirma.durum, tok)),
        );
    });
    ui.separator();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            if calistirma.cikti.is_empty() {
                ui.label(
                    egui::RichText::new(if tr {
                        "Henüz çıktı yok. ▶ ile çalıştırın."
                    } else {
                        "No output yet. Run with ▶."
                    })
                    .color(tok.renk.metin_soluk)
                    .italics(),
                );
            }
            for satir in &calistirma.cikti {
                let renk = if satir.hata_akisi {
                    tok.renk.hata
                } else {
                    tok.renk.metin
                };
                ui.label(egui::RichText::new(&satir.metin).monospace().color(renk));
            }
        });
}

/// Durum rengini token'dan seçer (MK-52).
fn durum_rengi(durum: &Calisma, tok: &Tokenlar) -> egui::Color32 {
    match durum {
        Calisma::Bosta => tok.renk.metin_soluk,
        Calisma::Calisiyor => tok.renk.bilgi,
        Calisma::Bitti(Some(0)) | Calisma::Bitti(None) => tok.renk.basari,
        Calisma::Bitti(Some(_)) => tok.renk.uyari,
        Calisma::Durduruldu => tok.renk.uyari,
        Calisma::ZamanAsimi => tok.renk.uyari,
        Calisma::Hata => tok.renk.hata,
    }
}

// ─── Saf yardımcılar (birim-testlenebilir) ──────────────────────────────────────

/// `# %%` ile ayrılan hücrelerden, `imlec` bayt ofsetini içeren hücreyi döner.
pub fn hucre_bul(metin: &str, imlec: usize) -> &str {
    let mut sinirlar = vec![0usize];
    let mut pos = 0usize;
    for satir in metin.split_inclusive('\n') {
        let t = satir.trim_start();
        if pos != 0 && (t.starts_with("# %%") || t.starts_with("#%%")) {
            sinirlar.push(pos);
        }
        pos += satir.len();
    }
    sinirlar.push(metin.len());
    let imlec = imlec.min(metin.len());
    for w in sinirlar.windows(2) {
        if imlec >= w[0] && imlec < w[1] {
            return &metin[w[0]..w[1]];
        }
    }
    metin
}

/// Bir bayt ofsetinden, o satırın metnini ve **karakter** sütununu döner (tamamlama öneki için).
fn imlec_satir_sutun(metin: &str, bayt: usize) -> (String, usize) {
    let bayt = bayt.min(metin.len());
    // Satır başını bul (son '\n' + 1).
    let satir_bas = metin[..bayt].rfind('\n').map(|i| i + 1).unwrap_or(0);
    // Satır sonunu bul.
    let satir_son = metin[satir_bas..]
        .find('\n')
        .map(|i| satir_bas + i)
        .unwrap_or(metin.len());
    let satir_metni = metin[satir_bas..satir_son].to_string();
    // İmlece kadar olan kısımdaki karakter sayısı = sütun.
    let sutun = metin[satir_bas..bayt].chars().count();
    (satir_metni, sutun)
}

/// Bir bayt ofsetinden (0-tabanlı satır numarası, karakter sütunu) döner (jedi isteği için).
fn imlec_satir_no_sutun(metin: &str, bayt: usize) -> (usize, usize) {
    let bayt = bayt.min(metin.len());
    let satir_no = metin[..bayt].bytes().filter(|&b| b == b'\n').count();
    let satir_bas = metin[..bayt].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let sutun = metin[satir_bas..bayt].chars().count();
    (satir_no, sutun)
}

/// Karakter indeksini bayt ofsetine çevirir (imleç → hücre seçimi).
fn bayt_ofseti(metin: &str, kar_indeks: usize) -> usize {
    metin
        .char_indices()
        .nth(kar_indeks)
        .map(|(b, _)| b)
        .unwrap_or(metin.len())
}

/// Bir aralığı satır uzunluğuna ve char sınırlarına göre güvenli hale getirir.
fn guvenli_aralik(satir: &str, bas: usize, son: usize) -> (usize, usize) {
    let mut bas = bas.min(satir.len());
    let mut son = son.min(satir.len());
    while bas > 0 && !satir.is_char_boundary(bas) {
        bas -= 1;
    }
    while son > 0 && son < satir.len() && !satir.is_char_boundary(son) {
        son -= 1;
    }
    (bas, son.max(bas))
}

/// Bir dosya yolunun görünen adını verir.
fn dosya_adi(yol: &Path) -> String {
    yol.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| yol.to_string_lossy().into_owned())
}

/// IO hatasını standart şemaya çevirir.
fn dosya_hata(ne: &str, yol: &Path, e: std::io::Error) -> ErrorReport {
    ErrorReport::new(
        ne,
        format!("'{}' işlenemedi", yol.display()),
        "Dosya yolunu ve okuma iznini kontrol edin",
    )
    .with_teknik_detay(e.to_string())
}
