//! W3C Trace Context — dağıtık izleme bağlamı (İP-21, MK-57).
//!
//! Her **uzun iş** ve her **dış çağrı** (eklenti subprocess, IPC, ağ, AI sağlayıcı)
//! bir [`TraceContext`] taşır.  Bu bağlam, loglar ile kullanıcıya gösterilen diyalogları
//! ve birbirini tetikleyen alt-işleri tek bir **iz (trace)** altında birleştirir.
//!
//! Biçim, W3C Trace Context standardının `traceparent` başlığıyla **birebir** uyumludur:
//! `00-<trace_id 32hex>-<span_id 16hex>-<flags 2hex>`.  Böylece çıktı OpenTelemetry
//! toplayıcılarına olduğu gibi aktarılabilir (gelecekteki gerçek OTel ihracatçısı için kanca).
//!
//! Tasarım kararı (ADR): `trace_id` = [`CorrelationId`]'nin UUID baytları.  Korelasyon kimliği
//! ile iz kimliği **aynı** 128-bit değerdir → kullanıcının gördüğü kısa kimlik (`kisa()`),
//! loglardaki `trace_id`'nin önekidir.  İki ayrı kimlik üretip eşitlemek zorunda kalmayız.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::CorrelationId;

/// W3C Trace Context sürümü — bu sürümde yalnızca `00` desteklenir (standart gereği).
const SURUM: u8 = 0x00;

/// `traceparent` bayrakları: en düşük bit = "örneklendi (sampled)".
/// MVP'de tüm izler örneklenir (yerel gözlemlenebilirlik); uzak örnekleme ileride.
pub const BAYRAK_ORNEKLENDI: u8 = 0x01;

/// Tek bir izin (trace) tek bir adımını (span) tanımlayan W3C-uyumlu bağlam.
///
/// - `trace_id`: tüm iz boyunca **sabit** 128-bit kimlik (kök işle aynı).
/// - `span_id`: bu **tekil adımın** 64-bit kimliği (her alt-iş yeni `span_id` alır).
/// - `flags`: örnekleme/öncelik bayrakları (W3C `trace-flags`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceContext {
    /// 128-bit iz kimliği (W3C `trace-id`); asla tümü-sıfır olamaz.
    pub trace_id: [u8; 16],
    /// 64-bit adım kimliği (W3C `parent-id`/`span-id`); asla tümü-sıfır olamaz.
    pub span_id: [u8; 8],
    /// W3C `trace-flags` (bit alanı; bkz. [`BAYRAK_ORNEKLENDI`]).
    pub flags: u8,
}

impl TraceContext {
    /// Yeni bir **kök** iz başlatır: taze `trace_id` + taze `span_id`, örneklendi bayraklı.
    ///
    /// Bir kullanıcı eylemi / uzun iş / dış çağrı zincirinin başında çağrılır.
    pub fn kok() -> Self {
        Self {
            trace_id: *Uuid::new_v4().as_bytes(),
            span_id: yeni_span_id(),
            flags: BAYRAK_ORNEKLENDI,
        }
    }

    /// Var olan bir [`CorrelationId`]'den iz bağlamı kurar (`trace_id` = UUID baytları).
    /// Yeni bir `span_id` üretilir.  Hata raporundan ize geçmek için kullanılır.
    pub fn from_correlation(id: CorrelationId) -> Self {
        Self {
            trace_id: *id.0.as_bytes(),
            span_id: yeni_span_id(),
            flags: BAYRAK_ORNEKLENDI,
        }
    }

    /// Bu izin **alt adımını** (child span) üretir: aynı `trace_id`, yeni `span_id`.
    /// Bir uzun iş başka bir dış çağrıyı tetiklediğinde kullanılır → zincir tek iz altında kalır.
    pub fn cocuk(&self) -> Self {
        Self {
            trace_id: self.trace_id,
            span_id: yeni_span_id(),
            flags: self.flags,
        }
    }

    /// Bu bağlamın [`CorrelationId`]'si (`trace_id`'den türetilir; iz=korelasyon aynı kimliktir).
    pub fn correlation_id(&self) -> CorrelationId {
        CorrelationId(Uuid::from_bytes(self.trace_id))
    }

    /// İz örneklendi mi? (W3C `sampled` biti)
    pub fn ornekleniyor(&self) -> bool {
        self.flags & BAYRAK_ORNEKLENDI != 0
    }

    /// W3C `traceparent` başlık değerini üretir:
    /// `00-<trace_id 32hex>-<span_id 16hex>-<flags 2hex>` (hepsi küçük harf).
    pub fn traceparent(&self) -> String {
        let mut s = String::with_capacity(55);
        s.push_str(&format!("{SURUM:02x}-"));
        for b in &self.trace_id {
            s.push_str(&format!("{b:02x}"));
        }
        s.push('-');
        for b in &self.span_id {
            s.push_str(&format!("{b:02x}"));
        }
        s.push_str(&format!("-{:02x}", self.flags));
        s
    }

    /// Bir `traceparent` dizgesini ayrıştırır.  Geçersizse (yanlış sürüm/uzunluk veya
    /// tümü-sıfır kimlik) `None` döner → bozuk başlık sessizce yeni kök ize düşülerek ele alınır.
    pub fn parse_traceparent(s: &str) -> Option<Self> {
        let parcalar: Vec<&str> = s.trim().split('-').collect();
        if parcalar.len() != 4 {
            return None;
        }
        // Sürüm: yalnızca "00".
        if hex_bayt(parcalar[0])? != [SURUM][..] {
            return None;
        }
        let trace_vec = hex_bayt(parcalar[1])?;
        let span_vec = hex_bayt(parcalar[2])?;
        let flag_vec = hex_bayt(parcalar[3])?;
        if trace_vec.len() != 16 || span_vec.len() != 8 || flag_vec.len() != 1 {
            return None;
        }
        let mut trace_id = [0u8; 16];
        let mut span_id = [0u8; 8];
        trace_id.copy_from_slice(&trace_vec);
        span_id.copy_from_slice(&span_vec);
        // W3C: tümü-sıfır trace_id/span_id geçersizdir.
        if trace_id == [0u8; 16] || span_id == [0u8; 8] {
            return None;
        }
        Some(Self {
            trace_id,
            span_id,
            flags: flag_vec[0],
        })
    }

    /// `trace_id`'nin onaltılı (hex) tam biçimi — 32 karakter, OTel log alanı.
    pub fn trace_id_hex(&self) -> String {
        self.trace_id.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// `span_id`'nin onaltılı (hex) tam biçimi — 16 karakter, OTel log alanı.
    pub fn span_id_hex(&self) -> String {
        self.span_id.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl Default for TraceContext {
    /// Varsayılan = yeni kök iz (her uzun işin bir bağlamı olmalı; "boş" bağlam yoktur).
    fn default() -> Self {
        Self::kok()
    }
}

/// Taze 64-bit `span_id` üretir.  Harici `rand` bağımlılığı eklemeden, zaten ağaçta olan
/// `uuid` v4 (OS CSPRNG) baytlarının ilk 8'ini kullanır.
fn yeni_span_id() -> [u8; 8] {
    let mut id = [0u8; 8];
    id.copy_from_slice(&Uuid::new_v4().as_bytes()[..8]);
    id
}

/// Onaltılı dizgeyi bayt vektörüne çevirir; geçersiz karakter/tek uzunluk → `None`.
fn hex_bayt(s: &str) -> Option<Vec<u8>> {
    if s.is_empty() || s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kok_iz_sifir_olmayan_kimlikler_uretir() {
        let c = TraceContext::kok();
        assert_ne!(c.trace_id, [0u8; 16]);
        assert_ne!(c.span_id, [0u8; 8]);
        assert!(c.ornekleniyor());
    }

    #[test]
    fn traceparent_bicimi_55_karakter_w3c() {
        let c = TraceContext::kok();
        let tp = c.traceparent();
        // 2 + 1 + 32 + 1 + 16 + 1 + 2 = 55
        assert_eq!(tp.len(), 55);
        assert!(tp.starts_with("00-"));
        assert_eq!(tp.matches('-').count(), 3);
    }

    #[test]
    fn traceparent_gidip_gelme_ayni_baglam() {
        let c = TraceContext::kok();
        let geri = TraceContext::parse_traceparent(&c.traceparent()).unwrap();
        assert_eq!(c, geri);
    }

    #[test]
    fn cocuk_ayni_trace_farkli_span() {
        let kok = TraceContext::kok();
        let cocuk = kok.cocuk();
        assert_eq!(kok.trace_id, cocuk.trace_id);
        assert_ne!(kok.span_id, cocuk.span_id);
    }

    #[test]
    fn correlation_id_trace_id_ile_ayni_kimlik() {
        let c = TraceContext::kok();
        let cid = c.correlation_id();
        assert_eq!(*cid.0.as_bytes(), c.trace_id);
        // Kullanıcının gördüğü kısa kimlik, trace_id hex önekidir.
        assert!(c.trace_id_hex().starts_with(&cid.kisa()));
    }

    #[test]
    fn from_correlation_trace_id_korur() {
        let cid = CorrelationId::new();
        let c = TraceContext::from_correlation(cid);
        assert_eq!(c.correlation_id(), cid);
    }

    #[test]
    fn bozuk_traceparent_none_doner() {
        assert!(TraceContext::parse_traceparent("").is_none());
        assert!(TraceContext::parse_traceparent("00-yanlis").is_none());
        // Yanlış sürüm.
        assert!(TraceContext::parse_traceparent(
            "ff-00000000000000000000000000000001-0000000000000001-01"
        )
        .is_none());
        // Tümü-sıfır trace_id geçersiz.
        assert!(TraceContext::parse_traceparent(
            "00-00000000000000000000000000000000-0000000000000001-01"
        )
        .is_none());
        // Tümü-sıfır span_id geçersiz.
        assert!(TraceContext::parse_traceparent(
            "00-00000000000000000000000000000001-0000000000000000-01"
        )
        .is_none());
        // Hex olmayan karakter.
        assert!(TraceContext::parse_traceparent(
            "00-zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz-0000000000000001-01"
        )
        .is_none());
    }

    #[test]
    fn hex_uzunluklari_dogru() {
        let c = TraceContext::kok();
        assert_eq!(c.trace_id_hex().len(), 32);
        assert_eq!(c.span_id_hex().len(), 16);
    }
}
