//! Node → Kod köprüsü — temel akışı **eşdeğer Python betiği** olarak dışa aktarır (İP-05).
//!
//! Kullanıcı görsel akışını "kod olarak" görebilir/dışa aktarabilir.  Üretilen betik akışın
//! **yapısını** (node sırası + veri akışı + parametreler) gösterir; her node türü için bir
//! **iskelet fonksiyon** üretilir (gerçek bilim işlevi eklenti/araca göre doldurulur).
//!
//! ⚠️ **Ters yön (Kod → Node) MVP'de YOKTUR** (`MVP-sonrasi.md` §3.3); bu köprü tek yönlüdür.
// MK-54: motor temelde; gerçek işlev eklentiden.  MK-02: üretilen kod ayrı süreçte çalışır (İP-06).

use std::collections::HashMap;

use biocraft_sdk::node::{ParametreDeger, Parametreler};

use super::dag::topolojik_sira;
use super::graph::{NodeGraf, NodeKimlik, PortRef};
use super::port::PortYonu;

/// Akışı eşdeğer bir Python betiğine çevirir (topolojik sıra; döngü varsa uyarı satırı).
pub fn python_disa_aktar(
    graf: &NodeGraf,
    parametreler: &HashMap<NodeKimlik, Parametreler>,
) -> String {
    let mut s = String::new();
    s.push_str("#!/usr/bin/env python3\n");
    s.push_str("# -*- coding: utf-8 -*-\n");
    s.push_str(&format!(
        "\"\"\"BioCraft Engine — '{}' akışından üretilen eşdeğer Python betiği.\n",
        graf.kimlik
    ));
    s.push_str(
        "Node → Kod köprüsü (İP-05). Akışın yapısını gösterir; gerçek işlevleri doldurun.\n",
    );
    s.push_str("Ters yön (Kod → Node) bu sürümde yoktur.\n\"\"\"\n\n");

    // Topolojik sıra — döngü varsa çalıştırma sırası tanımsız.
    let sira = match topolojik_sira(graf) {
        Some(s) => s,
        None => {
            s.push_str("# UYARI: Akışta döngü var; geçerli bir çalıştırma sırası üretilemedi.\n");
            graf.nodelar().iter().map(|n| n.kimlik).collect()
        }
    };

    // ── Her benzersiz node türü için iskelet fonksiyon ──
    s.push_str("# ── Node tür fonksiyonları (eklenti/araç gerçeğiyle doldurun) ──\n");
    let mut gorulen: Vec<String> = Vec::new();
    for n in graf.nodelar() {
        if gorulen.contains(&n.tur_kimligi) {
            continue;
        }
        gorulen.push(n.tur_kimligi.clone());
        let fn_ad = fonksiyon_adi(&n.tur_kimligi);
        let mut arglar: Vec<String> = n.girisler.iter().map(|p| kimlik_temizle(&p.ad)).collect();
        // Yinelenen/boş arg adlarını benzersizleştir.
        benzersizlestir(&mut arglar, "girdi");
        let mut imza = arglar.join(", ");
        if !imza.is_empty() {
            imza.push_str(", ");
        }
        imza.push_str("**parametreler");
        s.push_str(&format!("def {fn_ad}({imza}):\n"));
        s.push_str(&format!(
            "    \"\"\"{} — {}\"\"\"\n",
            kacir_docstring(&n.baslik),
            n.tur_kimligi
        ));
        s.push_str(&format!(
            "    raise NotImplementedError(\"{}\")\n\n",
            n.tur_kimligi
        ));
    }

    // ── Akışın kendisi (topolojik sıra) ──
    s.push_str("\n# ── Akış (topolojik sıra) ──\n");
    s.push_str("def calistir():\n");
    if sira.is_empty() {
        s.push_str("    pass  # boş akış\n");
    }
    for k in &sira {
        let Some(n) = graf.node(*k) else { continue };
        let var = degisken_adi(*k);
        let fn_ad = fonksiyon_adi(&n.tur_kimligi);
        // Girdiler: her giriş portu için bağlantının kaynağındaki node değişkeni (yoksa None).
        let mut args: Vec<String> = Vec::new();
        for gi in 0..n.girisler.len() {
            let hedef = PortRef::yeni(*k, PortYonu::Giris, gi);
            let kaynak = graf
                .baglantilar()
                .iter()
                .find(|b| b.hedef == hedef)
                .map(|b| degisken_adi(b.kaynak.node));
            args.push(kaynak.unwrap_or_else(|| "None".to_string()));
        }
        // Parametreler: kwargs.
        if let Some(p) = parametreler.get(k) {
            for (ad, deg) in p.tumu() {
                args.push(format!("{}={}", kimlik_temizle(ad), python_literal(deg)));
            }
        }
        s.push_str(&format!(
            "    {var} = {fn_ad}({})  # {}\n",
            args.join(", "),
            n.baslik
        ));
    }
    // Çıktı node'larını (çıkışı olmayan) döndür.
    let cikti_varlar: Vec<String> = sira
        .iter()
        .filter(|k| {
            graf.node(**k)
                .map(|n| n.cikislar.is_empty())
                .unwrap_or(false)
        })
        .map(|k| degisken_adi(*k))
        .collect();
    if cikti_varlar.is_empty() {
        s.push_str("    return None\n");
    } else {
        s.push_str(&format!("    return ({},)\n", cikti_varlar.join(", ")));
    }

    s.push_str("\n\nif __name__ == \"__main__\":\n    calistir()\n");
    s
}

/// `girdi.dizi_oku` → `girdi_dizi_oku` (geçerli Python fonksiyon adı).
fn fonksiyon_adi(tur: &str) -> String {
    let temiz = kimlik_temizle(tur);
    if temiz.is_empty() {
        "node".to_string()
    } else {
        temiz
    }
}

/// Node kimliğinden kararlı değişken adı.
fn degisken_adi(k: NodeKimlik) -> String {
    format!("n{}", k.0)
}

/// Bir dizgeyi geçerli (ASCII) Python tanımlayıcısına çevirir; Türkçe harfler çevrilir.
fn kimlik_temizle(s: &str) -> String {
    let cevir = |c: char| -> char {
        match c {
            'ç' => 'c',
            'ş' => 's',
            'ğ' => 'g',
            'ı' => 'i',
            'ö' => 'o',
            'ü' => 'u',
            'İ' => 'i',
            'Ç' => 'C',
            'Ş' => 'S',
            'Ğ' => 'G',
            'Ö' => 'O',
            'Ü' => 'U',
            d => d,
        }
    };
    let mut out = String::new();
    for c in s.chars() {
        let c = cevir(c);
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    // Rakamla başlamasın.
    if out
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        out.insert(0, '_');
    }
    out
}

/// Boş/yinelenen arg adlarını benzersizleştirir (Python "duplicate argument" hatasını önler).
fn benzersizlestir(adlar: &mut [String], taban: &str) {
    let mut sayac: HashMap<String, usize> = HashMap::new();
    for ad in adlar.iter_mut() {
        if ad.is_empty() {
            *ad = taban.to_string();
        }
        let n = sayac.entry(ad.clone()).or_insert(0);
        if *n > 0 {
            let yeni = format!("{ad}_{n}");
            *n += 1;
            *ad = yeni;
        } else {
            *n += 1;
        }
    }
}

/// Bir parametre değerini Python literali olarak yazar.
fn python_literal(d: &ParametreDeger) -> String {
    match d {
        ParametreDeger::Metin(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        ParametreDeger::TamSayi(n) => n.to_string(),
        ParametreDeger::OndalikSayi(f) => {
            // Python float literali (sonsuz/NaN'ı güvenli yaz).
            if f.is_finite() {
                format!("{f}")
            } else {
                format!("float('{f}')")
            }
        }
        ParametreDeger::Mantik(b) => if *b { "True" } else { "False" }.to_string(),
    }
}

/// Docstring içindeki üçlü-tırnak kaçışı.
fn kacir_docstring(s: &str) -> String {
    s.replace('"', "'").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::graph::{Baglanti, NodeGraf};
    use crate::node::katalog::NodeKatalogu;

    fn akis() -> NodeGraf {
        let katalog = NodeKatalogu::ornek();
        let mut g = NodeGraf::yeni("ana");
        let ekle = |g: &mut NodeGraf, tur: &str| -> NodeKimlik {
            let k = g.yeni_node_kimlik();
            g.node_ekle_ham(katalog.bul(tur).unwrap().ornekle(k, (0.0, 0.0)));
            k
        };
        let oku = ekle(&mut g, "girdi.dizi_oku");
        let hiz = ekle(&mut g, "isle.hizala");
        let bk = g.yeni_baglanti_kimlik();
        g.baglanti_ekle_ham(Baglanti {
            kimlik: bk,
            kaynak: PortRef::yeni(oku, PortYonu::Cikis, 0),
            hedef: PortRef::yeni(hiz, PortYonu::Giris, 0),
        });
        g
    }

    #[test]
    fn python_betigi_yapi_uretir() {
        let g = akis();
        let py = python_disa_aktar(&g, &HashMap::new());
        // İskelet fonksiyonlar + çağrı sırası.
        assert!(py.contains("def girdi_dizi_oku("));
        assert!(py.contains("def isle_hizala("));
        assert!(py.contains("def calistir():"));
        // Bağlantı: hizala, oku'nun değişkenini almalı.
        assert!(py.contains("n1 = girdi_dizi_oku()"));
        assert!(py.contains("n2 = isle_hizala(n1)"));
        assert!(py.contains("__main__"));
    }

    #[test]
    fn parametreler_kwargs_olur() {
        let g = akis();
        let mut pars: HashMap<NodeKimlik, Parametreler> = HashMap::new();
        let mut p = Parametreler::yeni();
        p.ayarla("esik", ParametreDeger::TamSayi(30));
        p.ayarla("ad", ParametreDeger::Metin("test".into()));
        pars.insert(g.nodelar()[0].kimlik, p);
        let py = python_disa_aktar(&g, &pars);
        assert!(py.contains("esik=30"));
        assert!(py.contains("ad=\"test\""));
    }

    #[test]
    fn turkce_karakter_temizlenir() {
        assert_eq!(kimlik_temizle("Çıktı Özet"), "cikti_ozet");
        assert_eq!(fonksiyon_adi("girdi.dizi_oku"), "girdi_dizi_oku");
        // Rakamla başlama düzeltilir.
        assert_eq!(kimlik_temizle("3boyut"), "_3boyut");
    }
}
