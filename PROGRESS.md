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
- **Son tamamlanan gün:** Gün 8 — İP-08 Donanım Koruma (Zero-Impact'in donanım yarısı): `biocraft-mem` altında **termal eşik tablosu** (GPU/CPU/NVMe kademeli yük azaltma + kritikte acil durdurma + histerezisli otomatik devam — MK-24), **bağımsız watchdog thread'i** (ayrı `std::thread`; ana döngüye bağlı değil; duraklamaya geçişte **checkpoint** alır — veri kaybı yok), **sensör soyutlaması** (gerçek=sysinfo / sahte=betik / yok=zarif devre dışı), **disk koruması** (%10 uyarı / %2 salt-okunur + 100 MB marj + yanlış-sürücü koruması — MK-25), **auto-tuning** (donanım profili → Eco/Bio + düşük donanımda sadeleşme + 30 FPS — MK-26), **determinizm bayrağı kancası** (MK-29). Uygulamada: durum çubuğunda donanım göstergesi (CPU/GPU/RAM/sıcaklık + "soğutuluyor" rozeti), `--emulate-min` bayrağı, 'I'/'O' ısı simülasyon tuşları. (2026-06-21)
- **Sıradaki gün:** Gün 9 — İP-11 (Geri Al/Yinele — komut geçmişi / undo-redo altyapısı); `biocraft-state` (L2) tarafında komut yığını + tersine çevrilebilir işlemler
- **Derleme durumu:** ✅ `cargo build --workspace` (13 crate) + **173 test** geçiyor (types 24 + ui 36 + render 50 + **mem 63**); fmt/clippy(-D warnings)/machete/topology/**cargo-deny (exit 0)** temiz. Yeni bağımlılık: **sysinfo 0.33** (donanım izleme: CPU/RAM/sıcaklık + disk boş alan + profil; MIT, deny ok, `default-features=false` + system/disk/component). `biocraft-app` artık `biocraft-mem`'e doğrudan bağlı (L5→L2; topoloji temiz). **Donanım koruma mantığı tamamen saf/test-edilebilir** (termal tablo, disk kararı, auto-tune saf fonksiyonlar; gerçek sensör/disk/profil sysinfo arkasında, çökmesiz None ile zarif bozulma). Watchdog gerçek thread testi de geçiyor (ısınma betiği → acil durum + checkpoint). **NVML (NVIDIA GPU) gerçek entegrasyonu insan-eli/sürücü işi → MVP'de kanca** (sysinfo bileşen sıcaklığı + simülasyon override; çoğu Windows'ta termal sensör yetki/sürücü olmadan boş gelir → koruma zarifçe kapanır). **Çalışma-zamanı:** termal korumayı canlı görmek için uygulamada **'I' tuşu** GPU sıcaklığını yükseltir (watchdog kademeli kısma → soğutuluyor rozeti → acil durdurma), **'O'** kapatır; `--emulate-min` ile düşük-donanım sadeleşme uyarısı (canlı pencere görsel doğrulaması kullanıcıda). cargo-deny: zararsız "duplicate version" uyarıları (exit 0). GitHub Actions: 3 job — Gün 7 push'u sonrası izlenecek; Gün 8 push'u sonrası izlenecek
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
| 6 | 2026-06-20 | Faz 1 / İP-04 | **Render tamamlama + tasarım sistemi (MK-52, MK-04).** (1) **Token sistemi:** `assets/tokens.json` (tek renk kaynağı) → `biocraft-render/tokens.rs` saf token motoru (Renk RGBA8 + hex, Palet, Tema, **TokenDeposu**: tema değiştir O(1) <100 ms, **özel tema oluştur/JSON kaydet/geri yükle = E2**). UI tarafı `biocraft-ui/tokens.rs` ince adaptör (Renk→egui Color32, `egui_visuals()` ile TÜM egui token'dan). **Kodda sabit renk kalmadı** (named sabitler + 4 bileşendeki `Color32::WHITE` + app clear/durum renkleri token'a taşındı; grep sıfır). (2) **Tipografi:** `tipografi.rs` (Inter/JetBrains Mono/Space Grotesk rolleri + boyut + DPI ölçek); egui adaptörü font yükler, yoksa egui gömülü fontuna düşer (TDA m.1, loglu). (3) **2B plot:** `plot.rs` saf model (çizgi/scatter + veri→ekran + **culling + LOD seyreltme**) → egui `PlotWidget`; galeride coverage+varyant demosu. (4) **3B sahne:** `scene3d.rs` saf geometri/kamera (küre/silindir/top-çubuk, look-at+perspektif MVP) + **native wgpu çizici** (off-screen renk+derinlik, Lambert shader, malzeme=token) → egui'ye doku olarak; sağ panelde canlı dönen top-çubuk. (5) **LOD:** `lod.rs` görünür-alan culling + LOD kademe + seyreltme. 3 tema galeride döngüyle değişiyor. **Çalışma-zamanı:** `--cpu` WARP'ta WGSL derlendi + 3B pipeline + egui doku entegrasyonu çökmeden çalıştı (gerçek GPU görsel doğrulaması kullanıcıda) | ✅ | 108/108 (types 24 + ui 34 + render 50); fmt/clippy(-D)/machete/topology/cargo-deny temiz; serde+serde_json+bytemuck eklendi (deny ok) | Gün 7: İP-08 (Bellek Orkestratörü + Donanım Koruma) |
| 8 | 2026-06-21 | Faz 1 / İP-08 | **Donanım Koruma (Zero-Impact donanım yarısı) — MK-24/25/26/29.** `biocraft-mem`'e 6 yeni modül: (1) **thermal.rs** — saf termal eşik tablosu (GPU 70/75/80/85, CPU 75/85/95, NVMe 60/70 °C → `TermalAksiyon` TamKapasite/YukAzalt(75/50)/Duraklat/AcilDurdur), histerezisli "soğuyunca otomatik devam", `en_kotu_aksiyon` (parçalar arası en ciddi). (2) **hardware_guard.rs** — `DonanimSensoru` trait (gerçek `SistemSensoru`=sysinfo / `BetikSensor`/`SabitSensor` test / `SensorYok`), bağımsız `DonanimMuhafiz` watchdog (ayrı `std::thread`, ana döngüye bağlı değil, RAII durdur+drop), `adim_uygula` saf çekirdek: duraklamaya geçişte **checkpoint** (veri kaybı yok), **sensör yoksa zarif devre dışı + bilgi (çökme yok)**. (3) **disk_guard.rs** — %10 uyarı/%2 salt-okunur+100MB marj, `dogru_surucu_mu` yanlış-sürücü koruması (Win sürücü harfi/Unix kök), `disk_durumu_oku` sysinfo sürücü-başına. (4) **autotune.rs** — `DonanimProfili`→`DonanimSinifi`(Düşük/Orta/Yüksek)→`OtoAyar` (Eco/Bio + düşük donanımda sadeleşme+uyarı+30 FPS), `asgari_emulasyon` (--emulate-min). (5) **determinism.rs** — `DeterminizmBayragi` kancası (HizliKesif/TekrarUretilebilir + sabit tohum/sıralı indirgeme/worker kısıdı; bit-bit garanti v1.x). (6) **metrics.rs** — `TepeOrtalama` (tepe/ortalama + regresyon eşiği). App: `--emulate-min`, watchdog (sysinfo+simülasyon), durum çubuğunda donanım göstergesi + "soğutuluyor" rozeti, 'I'/'O' ısı simülasyon tuşları. **NVML gerçek entegrasyonu insan-eli/sürücü işi → kanca.** | ✅ | 173/173 (types 24 + ui 36 + render 50 + **mem 63**); fmt/clippy(-D)/machete/topology/cargo-deny(exit 0) temiz; sysinfo 0.33 eklendi (MIT, deny ok). 4 kabul senaryosu test edildi: ısınma→kademeli→acil+checkpoint, sensör-yok zarif kapanış, disk %2 salt-okunur, --emulate-min sadeleşme | Gün 9: İP-11 (Geri Al/Yinele) |
| 7 | 2026-06-21 | Faz 1 / İP-08 | **Global Bellek Orkestratörü (OOM koruması) + bütçe + out-of-core.** `biocraft-mem` (L2): (1) **orchestrator.rs** — `BellekOrkestratoru` (Arc<Mutex>, ucuz klon); RAII `Rezervasyon`/`OnbellekTutamac` (drop=otomatik iade); `rezerve_et` bütçe aşımında **panik DEĞİL** standart `ErrorReport` döner (MK-22); yer açmak için **LRU önbellek boşaltma**; `bellek_baskisi()` agresif temizleme; `durum()`+`bilesen_dokumu()` teşhis; çok-threadli güvenlik testi. (2) **budget.rs** — `dosya_butce_kontrol` (boyut × genişleme kat → tahmini RAM); sığmazsa **akış / cloud-burst (yer tutucu) / iptal** teklifi (4 TB'da bile "load all" yok — MK-09). (3) **priority.rs** — `OncelikModu` (Arayüz/Denge/Maksimum) → `hesap_plani` worker sayısı + **Zero-Impact** kullanıcı-aktif kısma kancası. (4) **akis.rs** — out-of-core: `akisla_isle`(Read) / `dosya_akisla_isle`(BufReader) / `mmap_ile_isle`(memmap2); yalnızca **bir pencere** rezerve → RAM'den büyük veri parça parça (test: 1 MB bütçe + 8 MB veri, tepe = 1 pencere). UI: **butce_dialog.rs** (mem teklifini Gün-4 stiliyle iki-dilli diyaloğa çevirir; egui L4) + galeride **canlı demo** (bellek çubuğu, +32 MB rezerve/önbellek, bellek baskısı, 4 TB bütçe diyaloğu, 8 MB akış, öncelik döngüsü). i18n'e 3 anahtar + `butce_metni`. `biocraft-ui→biocraft-mem` (L4→L2, topoloji temiz); memmap2 eklendi (deny ok). Donanım termal koruma Gün 8'e bırakıldı. | ✅ | 140/140 (types 24 + ui 36 + render 50 + mem 30); fmt/clippy(-D)/machete/topology/cargo-deny(exit 0) temiz; gallery İP-08 bölümü headless egui'de tüm tema/dilde çökmeden çizildi | Gün 8: İP-08 Donanım Koruma (termal eşik + watchdog + disk koruması + donanım göstergesi) |

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
