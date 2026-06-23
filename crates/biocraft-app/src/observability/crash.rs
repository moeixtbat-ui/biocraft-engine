//! Çökme (panic) raporu — yerel dump + **opt-in** uzak rapor (İP-21, MK-57, MK-45).
//!
//! Bir `panic!` oluştuğunda:
//! 1. Önceki (varsayılan) panik kancası çağrılır → stderr'e standart çıktı korunur.
//! 2. **Anonimleştirilmiş** bir [`CokmeRaporu`] yerel diske (`<dizin>/crash/`) yazılır:
//!    correlation_id + zaman + konum + temizlenmiş mesaj.  **PHI/PII içermez** (MK-45).
//! 3. Uzak gönderim **varsayılan KAPALI**dır (MVP çevrimdışı).  `uzak_rapor_izni=true`
//!    yalnızca kullanıcı açıkça onayladıysa olur; gerçek HTTPS gönderimi insan-eli/sonraki
//!    sürüm kancasıdır (burada yalnızca niyet işaretlenir, ağ çağrısı YOK).

use std::path::PathBuf;

use biocraft_types::{pii_temizle, CorrelationId};

/// Diske yazılan anonim çökme kaydı.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CokmeRaporu {
    /// Logları bu çökmeyle eşleştiren kimlik (kullanıcı destek için bunu iletir).
    pub correlation_id: String,
    /// Çökme zamanı (UTC, RFC3339).
    pub zaman: String,
    /// Temizlenmiş panik mesajı (PII süzgecinden geçmiş).
    pub mesaj: String,
    /// Kaynak konumu (dosya:satır) — varsa.
    pub konum: Option<String>,
    /// Panikleyen thread adı.
    pub thread: String,
    /// Uygulama sürümü.
    pub surum: String,
}

impl CokmeRaporu {
    /// NDJSON/insan-okur dosya içeriği.
    fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Panik kancasını kurar.  `dizin` = uygulama veri klasörü (çökme dump'ları `dizin/crash/`).
///
/// `uzak_rapor_izni`: kullanıcı uzak (anonim) raporu açıkça onayladı mı?  Varsayılan `false`
/// (yalnız yerel).  Gerçek gönderim ileride; bugün yalnız niyet loglanır (ağ çağrısı yok).
pub fn kur_panik_kancasi(dizin: PathBuf, uzak_rapor_izni: bool) {
    let onceki = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // 1) Varsayılan davranışı koru (stderr'e yaz).
        onceki(info);

        // 2) Anonim rapor üret.
        let cid = CorrelationId::new();
        let ham_mesaj = panik_mesaji(info.payload());
        let rapor = CokmeRaporu {
            correlation_id: cid.0.to_string(),
            zaman: chrono::Utc::now().to_rfc3339(),
            mesaj: pii_temizle(&ham_mesaj), // MK-45: PHI/PII sızdırma
            konum: info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line())),
            thread: std::thread::current()
                .name()
                .unwrap_or("bilinmeyen")
                .to_string(),
            surum: env!("CARGO_PKG_VERSION").to_string(),
        };

        // 3) Yerel diske yaz (başarısızlık çökmeyi kötüleştirmemeli → sessiz).
        let crash_dizin = dizin.join("crash");
        if std::fs::create_dir_all(&crash_dizin).is_ok() {
            let yol = crash_dizin.join(format!("crash-{}.json", cid.kisa()));
            let _ = std::fs::write(&yol, rapor.to_json());
            log::error!(
                "Uygulama beklenmedik biçimde durdu (correlation_id={}). Yerel çökme raporu: {}",
                cid.kisa(),
                yol.display()
            );
        }

        // 4) Uzak rapor: yalnızca onaylıysa — MVP'de gerçek gönderim YOK (insan-eli kanca).
        if uzak_rapor_izni {
            log::info!(
                "Uzak çökme raporu izni açık; anonim rapor gönderme bağlandığında iletilecek \
                 (correlation_id={}).",
                cid.kisa()
            );
        }
    }));
}

/// Panik yükünden (`&str` veya `String`) okunur mesaj çıkarır.
/// Not: tip (`PanicHookInfo`) ADLANDIRILMAZ → MSRV 1.80 ile uyumlu (o ad 1.81'de kararlı oldu).
fn panik_mesaji(yuk: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = yuk.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = yuk.downcast_ref::<String>() {
        s.clone()
    } else {
        "bilinmeyen panik yükü".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rapor_json_alanlari_icerir() {
        let r = CokmeRaporu {
            correlation_id: "abc".into(),
            zaman: "2026-06-23T00:00:00+00:00".into(),
            mesaj: "test".into(),
            konum: Some("main.rs:1".into()),
            thread: "main".into(),
            surum: "0.1.0".into(),
        };
        let j = r.to_json();
        assert!(j.contains("correlation_id"));
        assert!(j.contains("main.rs:1"));
    }

    #[test]
    fn rapor_pii_temizler() {
        // Çökme raporundaki mesaj alanı pii_temizle'den geçirilmeli (kancanın yaptığı gibi).
        let temiz = pii_temizle(r"panik: C:\Users\Furkan\gizli.txt e-posta a@b.com");
        let r = CokmeRaporu {
            correlation_id: "x".into(),
            zaman: "z".into(),
            mesaj: temiz,
            konum: None,
            thread: "t".into(),
            surum: "0.1.0".into(),
        };
        let j = r.to_json();
        assert!(!j.contains("Furkan"));
        assert!(!j.contains("a@b.com"));
    }
}
