//! ÇE-11 / MK-34 — **Temel rapor** (Markdown / HTML / PDF) + **"Yöntem ve Materyaller" taslağı**.
//!
//! Seçili analiz/görünümler + kullanılan **veri kaynakları (provenance/atıf)** + **parametreler** →
//! tek belge.  Köken kaydı Gün 18 ([`crate::data_io::Provenans`]) ve atıf/lisans Gün 41
//! ([`crate::data_io::LisansAtif`]) ile aynı tiplerden gelir (tek doğruluk kaynağı); rapor bunları
//! **Yöntem ve Materyaller** taslağına dizer → kullanıcı dergi/tez metnine yapıştırır.
//!
//! **Gizlilik (Gün 18 / MK-42/43):** Rapora konan parametreler hassasiyetle etiketlenir; PHI **hiç**,
//! hassas yalnız **onayla** rapora girer ([`super::data::GizlilikSuzgeci`] ile aynı kural) → "rapora
//! hassas veri sızması" önlenir.

use biocraft_sdk::biocraft_types::Timestamp;

use crate::data_io::Provenans;
use crate::db_search::HassasiyetEtiketi;

use super::data::GizlilikSuzgeci;
use super::figure;

/// Rapor parametresi (anahtar/değer + hassasiyet etiketi).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parametre {
    /// Parametre adı.
    pub anahtar: String,
    /// Değer.
    pub deger: String,
    /// Hassasiyet (gizlilik filtresi için; varsayılan `Genel`).
    pub etiket: HassasiyetEtiketi,
}

/// Rapora gömülecek/eklenecek bir görselin referansı (görsel ayrı dosya; rapor ona atıf yapar).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GorselReferans {
    /// Dosya adı / başlık.
    pub ad: String,
    /// Açıklama (figür altyazısı).
    pub aciklama: String,
}

/// Serbest metin bölümü (analiz açıklaması/sonuç yorumu).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaporBolumu {
    /// Bölüm başlığı.
    pub baslik: String,
    /// Bölüm içeriği (düz metin / temel Markdown).
    pub icerik: String,
}

/// **Temel rapor** — başlık + özet + bölümler + görseller + parametreler + veri kaynakları (atıf).
#[derive(Debug, Clone, PartialEq)]
pub struct Rapor {
    /// Rapor başlığı.
    pub baslik: String,
    /// Kısa özet.
    pub ozet: String,
    /// Serbest metin bölümleri.
    pub bolumler: Vec<RaporBolumu>,
    /// Kullanılan veri kaynakları (provenance + lisans/atıf).
    pub kaynaklar: Vec<Provenans>,
    /// Analiz parametreleri (hassasiyet etiketli).
    pub parametreler: Vec<Parametre>,
    /// Görsel referansları (figürler).
    pub gorseller: Vec<GorselReferans>,
    /// Hassas (PHI olmayan) parametrelerin rapora girmesi onaylandı mı?
    pub onay: bool,
    /// Üretim zamanı (UTC).
    pub uretim_tarihi: Timestamp,
}

impl Rapor {
    /// Başlıkla boş rapor (şimdi=UTC; onay kapalı → yalnız genel parametreler).
    pub fn yeni(baslik: impl Into<String>) -> Self {
        Self {
            baslik: baslik.into(),
            ozet: String::new(),
            bolumler: Vec::new(),
            kaynaklar: Vec::new(),
            parametreler: Vec::new(),
            gorseller: Vec::new(),
            onay: false,
            uretim_tarihi: chrono::Utc::now(),
        }
    }

    /// Özet ayarlar (akıcı).
    pub fn with_ozet(mut self, ozet: impl Into<String>) -> Self {
        self.ozet = ozet.into();
        self
    }

    /// Hassas parametre onayını ayarlar (akıcı).
    pub fn with_onay(mut self, onay: bool) -> Self {
        self.onay = onay;
        self
    }

    /// Bölüm ekler (akıcı).
    pub fn bolum_ekle(mut self, baslik: impl Into<String>, icerik: impl Into<String>) -> Self {
        self.bolumler.push(RaporBolumu {
            baslik: baslik.into(),
            icerik: icerik.into(),
        });
        self
    }

    /// Veri kaynağı (köken/atıf) ekler (akıcı).
    pub fn kaynak_ekle(mut self, p: Provenans) -> Self {
        self.kaynaklar.push(p);
        self
    }

    /// Genel parametre ekler (akıcı).
    pub fn parametre_ekle(self, anahtar: impl Into<String>, deger: impl Into<String>) -> Self {
        self.parametre_ekle_etiketli(anahtar, deger, HassasiyetEtiketi::Genel)
    }

    /// Hassasiyet etiketli parametre ekler (akıcı).
    pub fn parametre_ekle_etiketli(
        mut self,
        anahtar: impl Into<String>,
        deger: impl Into<String>,
        etiket: HassasiyetEtiketi,
    ) -> Self {
        self.parametreler.push(Parametre {
            anahtar: anahtar.into(),
            deger: deger.into(),
            etiket,
        });
        self
    }

    /// Görsel referansı ekler (akıcı).
    pub fn gorsel_ekle(mut self, ad: impl Into<String>, aciklama: impl Into<String>) -> Self {
        self.gorseller.push(GorselReferans {
            ad: ad.into(),
            aciklama: aciklama.into(),
        });
        self
    }

    /// Gizlilik filtresinden **geçen** (rapora konulabilir) parametreler — PHI hiç, hassas yalnız onayla.
    pub fn gorunur_parametreler(&self) -> Vec<&Parametre> {
        let suzgec = GizlilikSuzgeci { onay: self.onay };
        self.parametreler
            .iter()
            .filter(|p| suzgec.izinli(p.etiket))
            .collect()
    }

    /// **Yöntem ve Materyaller** taslağı (düz metin paragraf(lar)ı) — kaynaklar (atıf) + parametreler.
    /// Kullanıcı bunu dergi/tez "Methods" bölümüne uyarlar.
    pub fn yontem_taslagi(&self) -> String {
        let mut s = String::new();
        if self.kaynaklar.is_empty() {
            s.push_str("Bu analizde dış veri kaynağı kaydı bulunmamaktadır. ");
        } else {
            let adlar: Vec<String> = self
                .kaynaklar
                .iter()
                .map(|k| {
                    if k.kaynak.is_empty() {
                        k.veri_kimligi.clone()
                    } else {
                        format!("{} ({})", k.veri_kimligi, k.kaynak)
                    }
                })
                .collect();
            s.push_str(&format!(
                "Analizde şu veri kaynakları kullanılmıştır: {}. ",
                adlar.join("; ")
            ));
        }

        let p = self.gorunur_parametreler();
        if !p.is_empty() {
            let cogu: Vec<String> = p
                .iter()
                .map(|x| format!("{}={}", x.anahtar, x.deger))
                .collect();
            s.push_str(&format!("Kullanılan parametreler: {}. ", cogu.join(", ")));
        }

        s.push_str(
            "Tüm görseller ve tablolar BioCraft Studio ile üretilmiş olup her veri öğesinin \
kökeni (provenance) ve lisans/atıf yükümlülüğü korunmuştur.",
        );

        // Atıf yükümlülüğü olan kaynaklar için kaynakça satırları.
        let atifli: Vec<&Provenans> = self
            .kaynaklar
            .iter()
            .filter(|k| k.lisans_atif.is_some())
            .collect();
        if !atifli.is_empty() {
            s.push_str("\n\nAtıflar:\n");
            for k in atifli {
                if let Some(la) = &k.lisans_atif {
                    s.push_str(&format!("- {} (Lisans: {})\n", la.atif, la.lisans));
                }
            }
        }
        s
    }

    /// Raporu **Markdown** olarak üretir.
    pub fn markdown(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("# {}\n\n", self.baslik));
        s.push_str(&format!(
            "_Üretim tarihi: {}_\n\n",
            self.uretim_tarihi.format("%Y-%m-%d %H:%M UTC")
        ));

        if !self.ozet.is_empty() {
            s.push_str("## Özet\n\n");
            s.push_str(&self.ozet);
            s.push_str("\n\n");
        }

        for b in &self.bolumler {
            s.push_str(&format!("## {}\n\n{}\n\n", b.baslik, b.icerik));
        }

        if !self.gorseller.is_empty() {
            s.push_str("## Görseller\n\n");
            for g in &self.gorseller {
                s.push_str(&format!("- **{}** — {}\n", g.ad, g.aciklama));
            }
            s.push('\n');
        }

        let p = self.gorunur_parametreler();
        if !p.is_empty() {
            s.push_str("## Parametreler\n\n| Parametre | Değer |\n| --- | --- |\n");
            for x in &p {
                s.push_str(&format!("| {} | {} |\n", x.anahtar, x.deger));
            }
            s.push('\n');
        }

        s.push_str("## Yöntem ve Materyaller\n\n");
        s.push_str(&self.yontem_taslagi());
        s.push_str("\n\n");

        if !self.kaynaklar.is_empty() {
            s.push_str("## Veri Kaynakları ve Köken (Provenance)\n\n");
            for (i, k) in self.kaynaklar.iter().enumerate() {
                s.push_str(&format!(
                    "{}. **{}** — Kaynak: {}; Format: {}; Erişim: {}; BLAKE3: `{}`\n",
                    i + 1,
                    k.veri_kimligi,
                    if k.kaynak.is_empty() {
                        "—"
                    } else {
                        &k.kaynak
                    },
                    if k.format.is_empty() {
                        "—"
                    } else {
                        &k.format
                    },
                    k.tarih.format("%Y-%m-%d"),
                    blake3_kisa(&k.blake3),
                ));
                if let Some(la) = &k.lisans_atif {
                    s.push_str(&format!("   - Lisans: {}; Atıf: {}", la.lisans, la.atif));
                    if let Some(u) = &la.url {
                        s.push_str(&format!(" <{u}>"));
                    }
                    s.push('\n');
                }
            }
            s.push('\n');
        }

        s
    }

    /// Raporu **HTML** olarak üretir (tek dosya; basit gömülü stil).
    pub fn html(&self) -> String {
        let mut s = String::new();
        s.push_str("<!DOCTYPE html>\n<html lang=\"tr\">\n<head>\n<meta charset=\"utf-8\">\n");
        s.push_str(&format!("<title>{}</title>\n", html_kacis(&self.baslik)));
        s.push_str(
            "<style>body{font-family:sans-serif;max-width:800px;margin:2rem auto;line-height:1.5}\
table{border-collapse:collapse}td,th{border:1px solid #ccc;padding:4px 8px}\
code{background:#f4f4f4;padding:1px 4px}</style>\n</head>\n<body>\n",
        );
        s.push_str(&format!("<h1>{}</h1>\n", html_kacis(&self.baslik)));
        s.push_str(&format!(
            "<p><em>Üretim tarihi: {}</em></p>\n",
            self.uretim_tarihi.format("%Y-%m-%d %H:%M UTC")
        ));

        if !self.ozet.is_empty() {
            s.push_str(&format!(
                "<h2>Özet</h2>\n<p>{}</p>\n",
                html_kacis(&self.ozet)
            ));
        }

        for b in &self.bolumler {
            s.push_str(&format!(
                "<h2>{}</h2>\n<p>{}</p>\n",
                html_kacis(&b.baslik),
                html_kacis(&b.icerik)
            ));
        }

        if !self.gorseller.is_empty() {
            s.push_str("<h2>Görseller</h2>\n<ul>\n");
            for g in &self.gorseller {
                s.push_str(&format!(
                    "<li><strong>{}</strong> — {}</li>\n",
                    html_kacis(&g.ad),
                    html_kacis(&g.aciklama)
                ));
            }
            s.push_str("</ul>\n");
        }

        let p = self.gorunur_parametreler();
        if !p.is_empty() {
            s.push_str(
                "<h2>Parametreler</h2>\n<table>\n<tr><th>Parametre</th><th>Değer</th></tr>\n",
            );
            for x in &p {
                s.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td></tr>\n",
                    html_kacis(&x.anahtar),
                    html_kacis(&x.deger)
                ));
            }
            s.push_str("</table>\n");
        }

        s.push_str("<h2>Yöntem ve Materyaller</h2>\n");
        for paragraf in self.yontem_taslagi().split("\n\n") {
            s.push_str(&format!("<p>{}</p>\n", html_kacis(paragraf)));
        }

        if !self.kaynaklar.is_empty() {
            s.push_str("<h2>Veri Kaynakları ve Köken (Provenance)</h2>\n<ol>\n");
            for k in &self.kaynaklar {
                s.push_str(&format!(
                    "<li><strong>{}</strong> — Kaynak: {}; Format: {}; Erişim: {}; BLAKE3: <code>{}</code>",
                    html_kacis(&k.veri_kimligi),
                    html_kacis(if k.kaynak.is_empty() { "—" } else { &k.kaynak }),
                    html_kacis(if k.format.is_empty() { "—" } else { &k.format }),
                    k.tarih.format("%Y-%m-%d"),
                    html_kacis(&blake3_kisa(&k.blake3)),
                ));
                if let Some(la) = &k.lisans_atif {
                    s.push_str(&format!(
                        "<br>Lisans: {}; Atıf: {}",
                        html_kacis(&la.lisans),
                        html_kacis(&la.atif)
                    ));
                }
                s.push_str("</li>\n");
            }
            s.push_str("</ol>\n");
        }

        s.push_str("</body>\n</html>\n");
        s
    }

    /// Raporu **temel PDF** olarak üretir (Markdown'ın düz-metin satırlarını Helvetica ile dizer).
    /// Tam şablonlu/sayfalı yayın PDF'i v1.x; bu MVP "temel" çıktısıdır.
    pub fn pdf(&self) -> Vec<u8> {
        let satirlar: Vec<String> = self.markdown().lines().map(|l| l.to_string()).collect();
        figure::metin_pdf(&satirlar, 10.0)
    }
}

/// BLAKE3 özetini kısaltır (rapor okunabilirliği; ilk 12 hex).
fn blake3_kisa(h: &str) -> String {
    if h.len() <= 12 {
        h.to_string()
    } else {
        format!("{}…", &h[..12])
    }
}

/// HTML metin kaçışı (`&`, `<`, `>`, `"`).
fn html_kacis(s: &str) -> String {
    let mut c = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => c.push_str("&amp;"),
            '<' => c.push_str("&lt;"),
            '>' => c.push_str("&gt;"),
            '"' => c.push_str("&quot;"),
            _ => c.push(ch),
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_io::{LisansAtif, Provenans};

    fn ncbi_kaynak() -> Provenans {
        Provenans {
            veri_kimligi: "NM_007294.fasta".into(),
            kaynak: "NCBI nucleotide (efetch)".into(),
            format: "FASTA".into(),
            surum: String::new(),
            tarih: chrono::Utc::now(),
            blake3: "a".repeat(64),
            boyut_bayt: 1234,
            lisans_atif: Some(LisansAtif {
                lisans: "Public Domain".into(),
                atif: "NCBI, NLM, E-utilities".into(),
                url: Some("https://ncbi.nlm.nih.gov".into()),
            }),
        }
    }

    fn ornek_rapor() -> Rapor {
        Rapor::yeni("BRCA1 Varyant Analizi")
            .with_ozet("chr17 üzerindeki BRCA1 bölgesinde varyantlar incelendi.")
            .bolum_ekle("Bulgular", "3 patojenik varyant saptandı.")
            .kaynak_ekle(ncbi_kaynak())
            .parametre_ekle("QUAL eşiği", "30")
            .parametre_ekle("Filtre", "PASS")
            .gorsel_ekle("sekil1.svg", "BRCA1 bölgesi genom tarayıcı görünümü")
    }

    #[test]
    fn markdown_baslik_kaynak_atif_icerir() {
        let md = ornek_rapor().markdown();
        assert!(md.starts_with("# BRCA1 Varyant Analizi"));
        assert!(md.contains("## Özet"));
        assert!(md.contains("## Yöntem ve Materyaller"));
        assert!(md.contains("NM_007294.fasta"));
        assert!(md.contains("Public Domain"));
        assert!(md.contains("NCBI, NLM, E-utilities"));
        assert!(md.contains("| QUAL eşiği | 30 |"));
    }

    #[test]
    fn yontem_taslagi_kaynak_ve_parametre_birlestirir() {
        let taslak = ornek_rapor().yontem_taslagi();
        assert!(taslak.contains("NM_007294.fasta"));
        assert!(taslak.contains("QUAL eşiği=30"));
        assert!(taslak.contains("provenance"));
        assert!(taslak.contains("Atıflar:"));
    }

    #[test]
    fn html_gecerli_ve_kacisli() {
        let r = Rapor::yeni("a<b>&\"c").bolum_ekle("x", "1 < 2 & 3");
        let html = r.html();
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("a&lt;b&gt;&amp;&quot;c"));
        assert!(html.contains("1 &lt; 2 &amp; 3"));
        assert!(html.trim_end().ends_with("</html>"));
    }

    #[test]
    fn pdf_gecerli_iskelet() {
        let pdf = ornek_rapor().pdf();
        let metin = String::from_utf8_lossy(&pdf);
        assert!(metin.starts_with("%PDF-1.4"));
        assert!(metin.contains("/BaseFont /Helvetica"));
        assert!(metin.trim_end().ends_with("%%EOF"));
    }

    #[test]
    fn gizlilik_phi_parametre_rapora_girmez() {
        let r = Rapor::yeni("Gizli")
            .parametre_ekle("genel", "1")
            .parametre_ekle_etiketli("hasta_id", "12345", HassasiyetEtiketi::Phi)
            .parametre_ekle_etiketli("not", "gizli", HassasiyetEtiketi::Hassas);

        // Onaysız: yalnız genel.
        let gorunur = r.gorunur_parametreler();
        assert_eq!(gorunur.len(), 1);
        assert_eq!(gorunur[0].anahtar, "genel");

        let md = r.markdown();
        assert!(!md.contains("hasta_id"));
        assert!(!md.contains("12345"));
        assert!(!md.contains("gizli"));

        // Onaylı: hassas girer, PHI yine giremez.
        let onayli = r.with_onay(true);
        let g2 = onayli.gorunur_parametreler();
        assert_eq!(g2.len(), 2); // genel + hassas
        let md2 = onayli.markdown();
        assert!(md2.contains("not"));
        assert!(!md2.contains("hasta_id")); // PHI asla
        assert!(!md2.contains("12345"));
    }

    #[test]
    fn blake3_kisaltma() {
        assert_eq!(blake3_kisa(&"f".repeat(64)), format!("{}…", "f".repeat(12)));
        assert_eq!(blake3_kisa("abc"), "abc");
    }
}
