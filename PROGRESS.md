# PROGRESS.md — BioCraft Engine İlerleme Günlüğü

> Bu dosya BioCraft Engine'in **hafızasıdır.** Yapay zeka her oturum sonunda buraya bir satır ekler.
> Yeni bir bağlam (context) açtığında, yapay zeka önce `git log` çalıştırır + bu dosyayı okur,
> böylece "şimdiye kadar ne yaptık, nerede kaldık" sorusunu kullanıcıya geçmiş yapıştırtmadan yanıtlar.
>
> **Kural (yapay zeka için):** Her commit'ten ÖNCE aşağıdaki tabloya o günün satırını ekle.

---

## Mevcut Durum (Özet)

- **Marka:** BioCraft Engine · biocraftengine.com
- **Aktif sürüm hedefi:** MVP — tam kullanılabilir ilk sürüm (motor + çekirdek eklenti BioCraft Studio + AI yüzeyi)
- **Kapsam:** Temel uygulama İP-00…İP-21 + Çekirdek eklenti ÇE-00…ÇE-12 + AI yüzey (İP-14 / YZ-00,01,06,08)
- **Son tamamlanan gün:** Gün 0 — Ortam, Hesaplar ve Boş Depo (2026-06-20)
- **Sıradaki gün:** Gün 1 — İP-00: Cargo Workspace İskeleti (biocraft-types L0)
- **Derleme durumu:** Cargo workspace henüz oluşturulmadı (Gün 1'de başlıyor)
- **Bilinen bloke eden sorunlar:** yok

---

## Faz Haritası (gün-gün yol haritasıyla uyumlu)

- **Faz 1 — Temel + Kabuk:** İP-00, İP-16, İP-04, İP-08, İP-11, İP-03, İP-07
- **Faz 2 — Launcher/Proje/Gizlilik/Güvenlik:** İP-01, İP-02, İP-10, İP-09
- **Faz 3 — Node/Kod/Ayar/Palet:** İP-05, İP-06, İP-12, İP-13
- **Faz 4 — AI Yüzey/Kanca/Onboarding/Pazar/Göç/Paketleme/QA:** İP-14, İP-15, İP-17, İP-18, İP-19, İP-20, İP-21
- **Faz 5 — Çekirdek Eklenti (BioCraft Studio):** ÇE-00, ÇE-01, ÇE-02, ÇE-04, ÇE-07, ÇE-09, ÇE-11, ÇE-12, ÇE-03, ÇE-05, ÇE-06, ÇE-08, ÇE-10

---

## Günlük İlerleme Tablosu

| Gün | Tarih | Faz/Sprint | Ne Yapıldı | Durum | Test | Sonraki |
| --- | --- | --- | --- | --- | --- | --- |
| 0 | 2026-06-20 | Pre-Sprint | Git init + GitHub remote bağlama + iskelet dosyalar (.gitignore, rust-toolchain.toml, README.md) + anayasa (ARCHITECTURE/CLAUDE/PROGRESS) + 5 spec dosyası yerleştirildi | ✅ | — | Gün 1: İP-00 Cargo Workspace |

> Durum sembolleri: ✅ Tamam · ⚠️ Yarım/TODO var · ❌ Bloke · ⏳ Henüz başlanmadı

---

## Açık TODO'lar (devreden işler)

- (Yapay zeka yarım bıraktığı işleri buraya `// TODO(MK-xx)` referansıyla yazar.)

---

## İnsan Eli Bekleyen İşler (kod dışı — `docs/specs/Hukuk-ve-Operasyon.md`'ten)

> ⚠️ Bunların hiçbiri profesyonel görüş olmadan atılmamalı (Türkiye'de bilişim/fikri mülkiyet avukatı + mali müşavir/SMMM). Sıra, mantıksal önceliğe göredir.

- [ ] **(EN KRİTİK)** "BioCraft Engine" / "BioCraft" marka + domain çakışma kontrolü — TÜRKPATENT + uluslararası (WIPO/Madrid) ön arama. "craft" yaygın olduğundan çakışma riski ciddi araştırılmalı.
- [ ] Domain (biocraftengine.com) + sosyal hesapları kilitle (squatting önlemi).
- [ ] Lisans kararını sabitle: AGPLv3 çekirdek + Apache-2.0 SDK + ticari lisans; **AGPL madde 13 (sunucu) + ayrı-süreç mimarisi + CLA/çift-lisans + MPL-2.0 değerlendirmesi** (hukukçu onayı).
- [ ] Açık/kapalı sınırını netleştir: veri-güvenliği açık; yalnızca premium + lisans/anti-tamper kapalı.
- [ ] Temel hukuki metinler: Kullanım Koşulları (ToS) + Gizlilik Politikası taslağı (hukukçu).
- [ ] İçerik sorumluluğu: haber/pazar/kullanıcı içeriği için bildir-kaldır + moderasyon + ToS maddeleri.
- [ ] KVKK hazırlığı: VERBİS gerekliliği, açık rıza metinleri, ihlal bildirim süreci; sağlık verisi işlenecekse uzman.
- [ ] Klinik/sorumluluk feragati metinleri ("araştırma/bilgilendirme amaçlı, klinik karar için değil").
- [ ] Bilimsel veri lisans uyumu: referans genom (hg38/hg19), dbSNP, ClinVar, UniProt, PDB ticari dağıtımda.
- [ ] Şirketleşme zamanlaması (MVP doğrulanınca): **Limited (LTD)** kuruluşu (mali müşavir) → büyürse A.Ş.
- [ ] Teknokent / TÜBİTAK / KOSGEB Ar-Ge teşvik değerlendirmesi.
- [ ] Kod-imzalama sertifikası (Windows imzalama — tüzel kişilik kurulunca; dağıtım için gerekli).
- [ ] Bio-kredi konumlandırma: "kripto para değil, platform-içi puan"; gerçek ödeme öncesi hukukçu.
- [ ] Çalışan/ortak/katkıcı gelirse: iş/danışmanlık sözleşmesi + IP devri + NDA + CLA.

> (Tamamlananları işaretle. Detaylı çerçeve: `docs/specs/Hukuk-ve-Operasyon.md`. Bu liste gün-gün yol haritasındaki "İnsan Eli İşleri" ile eşleşir.)

> **Not (eski projeden ayrım):** Eski "BioForge" planındaki **Estonya e-Residency / OÜ** yolu ve **IGSC screening API** bu sürümde **kullanılmıyor.** Şirket Türkiye temelli (LTD); biyogüvenlik taraması kapsam dışı.
