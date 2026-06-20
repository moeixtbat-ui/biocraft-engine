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
- **Son tamamlanan gün:** Gün 5 — İP-04: winit + wgpu + egui render host (`biocraft-render` motoru + `biocraft-app` pencere host'u); kare bütçesi (MK-03), GPU TDR kurtarma (MK-04), CPU fallback + örnek TDA galerisi canlı pencerede (2026-06-20)
- **Sıradaki gün:** Gün 6 — İP-08: Bellek Orkestratörü + Performans + Donanım Koruma (Zero-Impact); render/iş katmanları bellek bütçesinden rezervasyon ister (MK-21/MK-22)
- **Derleme durumu:** ✅ `cargo build --workspace` (13 crate) + **70 test** geçiyor (types 24 + ui 27 + render 19); fmt/clippy(-D warnings)/machete/topology/**cargo-deny (exit 0)** temiz. Render/UI yığını workspace'te sabitlendi: wgpu 22 + winit 0.30 + egui 0.29 (+ egui-wgpu/egui-winit). Çalışma-zamanı doğrulandı: gerçek GPU (RTX 4060 Ti, `Bgra8Unorm`) **ve** `--cpu` yazılım fallback (WARP) çökmeden başlatıldı. cargo-deny: 14 "duplicate version" uyarısı (wgpu/winit ekosistemi; zararsız, exit 0). GitHub Actions: 3 job (ubuntu+windows+deny) — **Gün 5 push'u (d196c25) CI'da 3 job da yeşil doğrulandı** (deny 40s + ubuntu 1m50s + windows 4m40s; wgpu/winit ilk kez CI'da sorunsuz derlendi, 2026-06-20)
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
| 1 | 2026-06-20 | Faz 1 / İP-00 | Cargo workspace (resolver=2) + biocraft-types L0 crate: ProjectId, PluginId, Version(SemVer), DataClassification(MK-42), Capability(MK-13), JobStatus, Blake3Hash, Timestamp — Türkçe döküman yorumları + 18 birim testi | ✅ | 18/18 geçti | Gün 2: İP-16 (TDA hata şeması) veya diğer İP-00 parçası (iskelet crate'ler + CI) |
| 2 | 2026-06-20 | Faz 1 / İP-00 | 12 stub crate iskelet (L1–L5): biocraft-sdk/ipc/data/state/mem/render/plugin-host/net/ai-surface/ui/launcher/app — kök Cargo.toml 13 üye, hepsini derliyor; cargo-machete (0 kullanılmayan dep); MK-40 topoloji kontrol scripti (scripts/check-topology.py, Python+cargo metadata); .github/workflows/ci.yml (build/test/fmt/clippy/audit/machete/topology); fmt+clippy temiz | ✅ | 18/18 geçti (stubs sıfır test, types geçiyor) | Gün 3: CI hattı genişletme |
| 3 | 2026-06-20 | Faz 1 / İP-00 | CI hattı genişletme (MK-58/MK-60): Windows+Linux matrix; cargo-deny + deny.toml (lisans politikası + advisory — Hukuk-ve-Operasyon §1); rustfmt.toml + clippy.toml; ayrı `deny` job'u. **Canlı CI'da 3 sorun çıktı→çözüldü:** (1) Windows'ta topoloji scripti UnicodeEncodeError → `sys.stdout.reconfigure(utf-8)`; (2) deny.toml deprecated anahtarlar (unlicensed/allow-osi-fsf-free) silindi; (3) 13 crate'e `license.workspace=true`+`publish=false`, deny.toml `allow-wildcard-paths=true`. **Doğrulama testleri:** Actions'ta workflow çalıştı✅, tüm adımlar yeşil✅, bilerek format hatası→CI kırmızı (fmt adımı fail) doğrulandı✅ (feature dalında, sonra silindi) | ✅ | 18/18; fmt/clippy/topology/cargo-deny temiz; 3 job yeşil (canlı) | Gün 4: İP-16 (TDA hata şeması — `biocraft-types` içine `BioCraftError`, standart hata yapısı, `correlation_id`) |
| 4 | 2026-06-20 | Faz 1 / İP-16 | Ortak TDA (3. derece) arayüz bileşenleri (MK-53), `biocraft-ui` altında egui ile, "bir kez yaz her yerde kullan": **8 bileşen** (toast/bildirim, hata diyaloğu, boş durum, yükleme iskeleti, onay diyaloğu, büyük işlem tahmini, ilerleme/iş, durum rozetleri) + **örnek galeri ekranı** (tema açık/koyu + dil TR/EN değiştirici). Ek modüller: `tokens` (tema-duyarlı tasarım token'ları) + `i18n` (EN/TR, anahtar enum'u → eksik çeviri derlenmez). `biocraft-types`'a **standart hata şeması**: `ErrorReport` (ne/neden/çözüm tip düzeyinde zorunlu) + `CorrelationId`. egui 0.29 bağımlılığı; egui'nin gömülü fontları için deny.toml'a OFL-1.1 + UFL-1.0 muafiyeti. Zaman/iptal/throttle mantığı egui'siz birim-testlenebilir tutuldu; her diyalog headless egui karesiyle test edildi | ✅ | 51/51 (types 24 + ui 27); fmt/clippy(-D warnings)/machete/topology/cargo-deny **temiz** (yerelde) + CI'da 3 job yeşil (canlı) | Gün 5: İP-04 (winit+wgpu+egui pencere; örnek galeriyi canlı göster) |
| 5 | 2026-06-20 | Faz 1 / İP-04 | **Render ve tuval altyapısı temeli.** `biocraft-render` (L3) saf/test-edilebilir motor: `frame_budget` (~16 ms kare bütçesi, FPS, Eco mod, GPU ≤100 ms batch — MK-03/MK-04), `tdr` (DeviceLost kurtarma durum makinesi, <5 sn hedef, tekrarlı çökmede CPU'ya düş — MK-04), `backend` (tek aktif backend seçimi: wgpu/CUDA-iskelet/CPU), `gpu` (wgpu cihaz/kuyruk/yüzey, winit penceresi, CPU fallback = WARP fallback adapter). `biocraft-app` (L5) pencere host'u: winit 0.30 olay döngüsü + egui↔wgpu köprüsü → **örnek TDA galerisi canlı pencerede**; alt durum çubuğunda FPS + backend + CPU uyarısı; **T tuşu = TDR simülasyonu** ("GPU yeniden başlatıldı" bildirimi). MK-40: render egui'ye bağlı değil (köprü host'ta). Sürümler workspace'te sabit (wgpu 22/winit 0.30/egui 0.29). **Çalışma-zamanı doğrulandı:** gerçek GPU (RTX 4060 Ti) + `--cpu` WARP fallback çökmeden başladı; egui için doğrusal `Bgra8Unorm` format (renk doğruluğu) | ✅ | 70/70 (types 24 + ui 27 + render 19); fmt/clippy(-D warnings)/machete/topology temiz; cargo-deny exit 0 (14 zararsız duplicate uyarısı) | Gün 6: İP-08 (Bellek Orkestratörü + Donanım Koruma) |

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
