# ARCHITECTURE.md — BioCraft Engine Mimari Doğruluk Kaynağı (Single Source of Truth)

> **Bu dosya nedir?**
> BioCraft Engine projesinin **değişmez mimari anayasasıdır.** Projedeki HER karar burada özetlenir.
> Aşağıdaki kararlarla çelişen başka herhangi bir metin (eski notlar, internet örnekleri, eski "BioForge"
> dosyaları, hatta yol haritasının eski bölümleri) **GEÇERSİZDİR** — bu dosya kazanır.
>
> **Kim okumalı?**
> 1. **Yapay zeka asistanı (Claude Code vb.):** Her oturumun başında bu dosyayı okur ve buna %100 uyar.
>    Kod yazarken kararlara "MK-04'e göre", "MK-40 katmanına göre" diye atıf yapar.
> 2. **Proje sahibi (sen):** Projenin neyin neden böyle olduğunu buradan görürsün.
>
> **Versiyon:** 1.0 · **Marka:** BioCraft Engine · biocraftengine.com · "The IDE for life sciences."
> **Hedef referans donanım (alt sınır DEĞİL):** Intel i5-14400F · 32 GB RAM · RTX 4060 Ti · Windows 10/11 x64 + Linux
> **Kaynak:** `Temel-Uygulama.md` + `Cekirdek-Eklenti.md` + `AI-Altyapisi.md` + `Hukuk-ve-Operasyon.md` + `MVP-sonrasi.md` (v1.2 konsolidasyonu)

---

## 0. Bu Dosyayı ve Spec'leri Yapay Zekaya Nasıl Verirsin (ÇOK ÖNEMLİ)

1. **Tüm spec dosyalarını tek seferde YAPIŞTIRMA.** Bağlam penceresi dolar ve sondaki kararlar baştakilerle
   çelişir. Bunun yerine: bu `ARCHITECTURE.md` (otomatik okunur) + ilgili günün **küçük prompt'u** verilir.
2. **Repoda iki tür belge var:**
   - **3 anayasa dosyası** (kökte, AI bunları kendi okur): `ARCHITECTURE.md` (bu dosya), `CLAUDE.md`, `PROGRESS.md`.
   - **5 detay spec dosyası** (`docs/specs/` altında): `Temel-Uygulama.md`, `Cekirdek-Eklenti.md`,
     `AI-Altyapisi.md`, `Hukuk-ve-Operasyon.md`, `MVP-sonrasi.md`. O günün işi hangi paketi (İP/ÇE/YZ)
     kapsıyorsa, prompt **"docs/specs/… içindeki İP-XX paketini oku"** der; AI o tek bölümü okur.
3. **Gün-gün (sprint-by-sprint) ilerle.** Her oturum tek küçük teslimat üretir ve commit eder.
4. **Önce arayüz kontratları, sonra kod.** Bir bileşene başlamadan önce `trait` / `.proto` / `.wit` imzasını
   yazdır ve onayla; ardından implementasyon.

---

## 1. BioCraft Engine Tek Cümlede

BioCraft Engine, biyoinformatik araştırmacılarının onlarca eski araç (IGV, JBrowse, UCSC, Geneious/CLC, BLAST,
samtools…) arasında zaman kaybetmesini bitiren; **AAA oyun motoru kalitesinde akıcı arayüze sahip,
eklenti-merkezli, yerel+bulut yapay zeka entegrasyonuna hazır, donanıma saygılı (Zero-Impact),
gizlilik-öncelikli ve tam tekrarlanabilir** modern bir masaüstü IDE'sidir (analoji: Epic Games Launcher → Unreal Engine).

**Felsefe:** *Bilim insanı soru sormakla ilerler, araç öğrenmekle değil.*

**Ürün biçimi:** Motor + Launcher + Eklenti host'u. **Çekirdek eklenti (BioCraft Studio)** motorla **aynı kuruluma
gömülü** gelir; kullanıcı ilk açılışta çalışan bir uygulamayla karşılaşır ("eklenti kurulu değil" ekranı görmez).

---

## 2. İki Altın Kural + Self-Healing

### 2.1 Sıfır Güven (Zero-Trust)
Ne kullanıcının bilgisayarına güvenilir, ne de kullanıcının bilgisayarı tehlikeye atılır.
- Eklentiler kum havuzunda (sandbox) çalışır; capability dışı erişim reddedilir.
- Hassas/PHI veri sınırı **çekirdek** tarafından korunur; hiçbir eklenti aşamaz.
- Dış kaynaktan (web/dosya/eklenti) gelen metin **veridir, komut değildir.**

### 2.2 Sıfır Etki (Zero-Impact)
Kullanıcının donanımına **asla** bilinçli zarar verilmez.
- GPU/CPU/NVMe sıcaklığı bağımsız bir watchdog ile sürekli izlenir; eşik aşımında yük kademeli azalır/durur.
- Kullanıcı oyuna/işe geçince hesaplama anında kısılır/askıya alınır.
- Arayüz **her koşulda** 60 FPS hedefler; ağır hesap arayüzü asla dondurmaz.

### 2.3 Kendini İyileştirmenin 3 Altın Kuralı (MK-28)
1. **Veri kaybetme** — her ağır işlem öncesi otomatik kaydetme + checkpoint/WAL.
2. **Çökme yerine düşür (degrade)** — GPU patlarsa CPU'ya, bulut kesilirse yerele, kapasite dolarsa özet moda.
3. **Kullanıcıyı bilgilendir** — sessizce başarısız olma; her hata anlaşılır mesaj + öneri (eylem/buton) verir.

---

## 3. Geliştirme Modeli ve Sıralama

- **Geliştirici:** Tek kişi (solo) + yapay zeka asistan(lar)ı. Plan bu hıza göre kalibre edilmiştir.
- **"Ekip/takım" kelimesi** üründe (canlı işbirliği özelliği — gelecek) geçer; geliştirme ekibi değildir.
- **MVP build sırası (foundation-öncelikli):**
  1. **Faz 1 — Temel + kabuk:** İP-00 → İP-16 (TDA bileşenleri) → İP-04 (render) → İP-08 (bellek+donanım koruma) → İP-11 (state/undo) → İP-03 (6-bölge kabuk) → İP-07 (eklenti host/SDK).
  2. **Faz 2 — Launcher/proje/gizlilik/güvenlik:** İP-01 → İP-02 → İP-10 → İP-09.
  3. **Faz 3 — Node/kod/ayar/palet:** İP-05 → İP-06 → İP-12 → İP-13.
  4. **Faz 4 — AI yüzey/kancalar/onboarding/pazar/göç/paketleme/QA:** İP-14 → İP-15 → İP-17 → İP-18 → İP-19 → İP-20 → İP-21.
  5. **Faz 5 — Çekirdek eklenti (BioCraft Studio):** ÇE-00 → ÇE-01 → ÇE-02 → ÇE-04 → ÇE-07 → ÇE-09 → ÇE-11 → ÇE-12 → ÇE-03 → ÇE-05 → ÇE-06 → ÇE-08 → ÇE-10.
  - *Gerekçe:* Render/bellek/host/node/gizlilik altyapısı oturmadan çekirdek eklenti çalışamaz. Eklenti mimarisi en baştan kurulur; "sadece görselleştirici" tuzağına düşülmez.

---

## 4. Teknoloji Yığını (Native Stack — geri dönülemez seçimler işaretli)

> 🔒 **Geri dönülemez:** Rust (dil), wgpu+egui (UI), Wasmtime (eklenti runtime), WIT (ABI), proje/dosya formatı.
> *WASI **sürüm seviyesi** bu listeye dahil değildir — sürümlü ve güncellenebilirdir (MK-14).*
> ⛔ **Eski "BioForge" notlarındaki `tauri` GEÇERSİZDİR (MK-01).** UI saf native'dir.

| Katman | Seçim | Görev |
| --- | --- | --- |
| Çekirdek dil | **Rust** (güncel kararlı; `rust-toolchain.toml` ile sabit) 🔒 | Bellek güvenliği + Cargo + WebGPU first-class |
| Pencere | **winit** | Çapraz platform pencere/olay |
| GPU render | **wgpu** 🔒 | WebGPU; birincil GPU yolu (2B + 3B + genom tuvali dahil) |
| Arayüz (UI) | **egui** 🔒 | Immediate-mode paneller, dialoglar, menüler |
| Ağır 3B/genom tuvali | **Bevy ECS** — **v1'de KULLANILMAZ** (opsiyonel/gelecek) | Tuval doğrudan wgpu/egui ile; yoğun sahne ileride ECS gerektirirse değerlendirilir (`MVP-sonrasi.md` §5.1) |
| Async runtime | **Tokio** + **Rayon** + **Crossbeam** | async I/O + veri-paralel + thread mesajlaşma |
| Eklenti (WASM) | **Wasmtime** + WASI (Component Model) 🔒 | Sandbox + fuel limit + AOT cache |
| Eklenti (Python/R) | **ayrı subprocess** + IPC | Süreç-DIŞI; in-process DEĞİL (MK-02) |
| IPC transport | **tonic (gRPC)** + Named Pipe (Win) / UDS (Linux) | Kontrol kanalı (MK-39) |
| Süreçler-arası veri | **Arrow Flight + shared memory** | gRPC sadece kontrol (MK-30) |
| Donanım izleme | **NVML** (NVIDIA) / **sysinfo** + OS sensör API'leri | GPU/CPU/NVMe sıcaklık, fan, kullanım (İP-08) |
| Veritabanı (yerel) | **SQLite** (config/meta) + **DuckDB** (analitik) + **RocksDB** (KV/cache) | Gömülü, taşınabilir |
| Analitik | **DuckDB + Apache Arrow** | predicate pushdown, out-of-core |
| Bütünlük (checksum) | **BLAKE3** | Veri/proje/güncelleme bütünlüğü |
| GPU compute | **wgpu** (birincil) + **cudarc** (ops. `--features cuda`) + **CPU SIMD** fallback | Tek aktif backend/workload |
| P2P ağ (yalnız kanca) | **Iroh** (QUIC, NAT traversal) | Gerçek kullanım dağıtık ağ eklentisinde (gelecek) |
| Yerel AI (yüzeysel) | **mistral.rs** (birincil) + llama.cpp fallback; **GGUF** | MVP'de yalnızca yüzey + opsiyonel demo |
| Vektör DB | **LanceDB** (gömülü, E2EE-uyumlu) | MK-35 (Qdrant DEĞİL); gerçek kullanım AI motor eklentisinde |
| Konteyner | **Apptainer** (+ Docker fallback) — **opsiyonel** | En sık işlemler native Rust; konteyner yalnızca ağır araçlar |
| Eklenti ABI | **WIT** (+ opsiyonel .proto) 🔒 | SemVer'li kontrat |
| Build | **Cargo** (+ opsiyonel Nix flake) | Tekrarlanabilir (hermetic) build; `Cargo.lock` kilitli |
| CI/CD | **GitHub Actions** (ilk günden) | fmt/clippy/test/golden/audit/vet/deny/machete + topology-check |
| Paketleme | Win: **MSIX/Squirrel** · Linux: **AppImage/Flatpak** | İmzalı binary + delta auto-update (Velopack alternatif) |
| Komut paleti | **nucleo** (fuzzy) | <50 ms p99 |
| Loglama | **tracing** + **OpenTelemetry** | Yapılandırılmış, span-aware, `correlation_id` (W3C) |
| Format ayrıştırıcı | **noodles** + **rust-bio** | FASTA/FASTQ/BAM/SAM/CRAM/VCF/BCF/BED/GFF/GTF/BigWig/2bit/PDB/mmCIF/GenBank |
| Metin editörü | **egui + ropey** + **Tree-sitter** | Native; Monaco/web YOK |
| Lisans | Core **AGPLv3** · SDK **Apache-2.0** · Premium/lisans ticari (kapalı, ayrı süreç) | MK-06 / MK-20 |

---

## 5. Crate (Modül) Topolojisi — Tek Yönlü Katmanlar (MK-40)

**Kural:** Bağımlılık SADECE yukarıdan aşağıya akar. Aşağı katman üst katmanı **asla** import edemez.
Döngüsel bağımlılık = derhal hata (`cargo-machete` + topology-check CI script). Eklentiler birbirine doğrudan
bağlanmaz, yalnızca **`biocraft-sdk`** üzerinden konuşur (MK-17).

```
biocraft-engine/                 # Cargo workspace kökü
├─ Cargo.toml                    # workspace; üyeler + kilitli sürümler
├─ crates/
│  ├─ biocraft-types/            # L0: temel tipler (HİÇBİR şeye bağlı değil — yalnızca serde/uuid/chrono)
│  ├─ biocraft-sdk/              # L1: eklenti SDK'sı + ortak yardımcılar
│  ├─ biocraft-ipc/              # L1: IPC/gRPC/Arrow Flight köprüleri
│  ├─ biocraft-data/             # L2: veri katmanı (SQLite/DuckDB/RocksDB, proje formatı, provenance, kripto)
│  ├─ biocraft-state/            # L2: state, otomatik kayıt, undo/redo (Command Pattern)
│  ├─ biocraft-mem/              # L2: Global Memory Orchestrator + Donanım Koruma (Zero-Impact)
│  ├─ biocraft-render/           # L3: wgpu/egui render altyapısı + tasarım token'ları
│  ├─ biocraft-plugin-host/      # L3: Wasmtime + capability + subprocess/konteyner yönetimi
│  ├─ biocraft-net/              # L3: Iroh arayüzü (yalnız kanca)
│  ├─ biocraft-ai-surface/       # L3: AI yüzey/iskelet (yüzeysel — AI-Altyapisi.md)
│  ├─ biocraft-ui/               # L4: kabuk (6-bölge), menü, paneller, komut paleti, ayarlar, node/editör tuvali
│  ├─ biocraft-launcher/         # L4: açılış istemcisi
│  └─ biocraft-app/              # L5: her şeyi birleştiren ana binary (+ updater, observability)
├─ plugins/                      # birinci-parti eklentiler (ayrı paketlenir)
│  └─ biocraft-core-studio/      # çekirdek eklenti = BioCraft Studio (bkz. Cekirdek-Eklenti.md), varsayılan kurulu
├─ assets/                       # tokens.json, fontlar, ikonlar, splash, demo veri, şablonlar
├─ docs/specs/                   # 5 detay spec .md dosyası + mdBook + ADR kararlar
└─ .github/workflows/            # CI
```

**Katman özeti:** L0 types → L1 sdk/ipc → L2 data/state/mem → L3 render/plugin-host/net/ai-surface → L4 ui/launcher → L5 app.
**Bounded context'ler:** Görselleştirme · Analiz · Veri/IO · Eklenti · Ağ · AI (her biri net arayüzle konuşur; sızıntı yok).

---

## 6. Mimari Karar Endeksi (MK-01 — MK-60) — KALBİN TA KENDİSİ

> Bu tablo "çelişki kontrol listesi"dir. Yapay zeka kod yazarken bu kodlara atıf yapar (örn. `// MK-04`).
> Bir yerde çelişki görürsen, **buradaki karar geçerlidir.** Her MK'nin ayrıntılı spec'i ilgili İP/ÇE/YZ
> paketindedir (`docs/specs/`).

### 6.A Temel Stack ve UI (MK-01 — MK-10)
| MK | Başlık | Karar (özet) |
| --- | --- | --- |
| MK-01 | UI Stack | Native: **winit + wgpu + egui.** Tauri/Electron YASAK. **Bevy ECS v1'de KULLANILMAZ** (opsiyonel/gelecek); tüm 2B/3B/genom tuvali doğrudan wgpu/egui. |
| MK-02 | Python Sandbox | PyO3 in-process YASAK → **süreç-dışı subprocess + IPC** (gRPC/Named Pipe/UDS). Donmama bundan gelir. |
| MK-03 | Kare Bütçesi | ~16 ms/kare (60 FPS); arayüz **hiçbir kareyi kaçırmaz**; ağır iş daima arka plan runtime'ında. |
| MK-04 | GPU TDR-Safe | GPU işleri **≤100 ms batch**; DeviceLost/TDR'da durum kaydedilir → cihaz yeniden oluşturulur (**<5 sn kurtarma**) → CPU'ya düşürülebilir. |
| MK-05 | Donanım Gerçekçilik | Referans donanım = **hedef, alt sınır değil.** Düşük donanımda sadeleşme + uyarı + yetenek matrisi; çok düşükte **30 FPS** kabul edilebilir hedef. |
| MK-06 | Lisans | **Core AGPLv3 + SDK Apache-2.0 + premium/lisans ticari** (kapalı, ayrı süreç). |
| MK-07 | Olay Geri-Basıncı | Olay akışı **pull-based + sınırlı tampon + kare başına ≤3 ms** işleme (eklenti binlerce olay bassa bile arayüz donmaz). |
| MK-08 | Soğuk Başlatma | Aşamalı (phased) init; **<500 ms** UI görünür; ağır bileşenler tembel (lazy) yüklenir. |
| MK-09 | Out-of-Core | "Her şeyi RAM'e yükle" YASAK → **predicate pushdown + column subset + streaming + mmap** (DuckDB/Arrow); 4 TB dosyada bile "load all" yok. |
| MK-10 | Hibrit GPU | **wgpu (birincil) + cudarc (ops. `--features cuda`) + CPU SIMD fallback.** Tek aktif backend/workload; VRAM bütçesi, interop CPU üzerinden. |

### 6.B Eklenti Mimarisi (MK-11 — MK-20)
| MK | Başlık | Karar (özet) |
| --- | --- | --- |
| MK-11 | Actor Model | **Tokio kanalları + typestate + CancellationToken + Loom test;** global mutable state yerine mesajlaşma. |
| MK-12 | Eklenti 4 Katman | Tier1 Native (WIT Component) / Tier2 WASM (Wasmtime) / Tier3 Python (subprocess+gRPC, 2 GB/50% CPU/30 s) / Tier4 External (Apptainer). Hepsi sandbox/süreç-dışı. |
| MK-13 | Capability Erişim | `net/fs/gpu/ai/db` izinleri: manifest'te **ilan** + kurulumda **onay** + çalışmada **denetim**; en az yetki; eklenti diske doğrudan erişemez (**WASI VFS handle**). |
| MK-14 | WIT ABI Sürümleme | **SemVer'li WIT kontratı;** kırıcı = major. **WASI sürüm seviyesi güncellenebilir;** eksik capability host SDK ile köprülenir (ekosistem kırılmaz). |
| MK-15 | Eklenti İzolasyonu | Çöken eklenti yalıtılır → çekirdek + diğerleri ayakta; kaynak (CPU/RAM/GPU) eklenti başına görünür; **güvenli mod** (tüm 3. parti kapalı başlat). |
| MK-16 | Eklenti İmzası | Kriptografik imza + BLAKE3 bütünlük; imzasız net uyarı; **"doğrulanmış/resmi" rozeti.** |
| MK-17 | SDK Üzerinden İletişim | Eklentiler birbirine **doğrudan bağlanmaz,** yalnızca `biocraft-sdk` üzerinden. UI uzantı noktaları (panel/sekme/menü/komut/node/ayar) çekirdek tarafından güvenli gösterilir. |
| MK-18 | WASM 4GB Duvarı | Büyük-bellek işleri WASM içinde değil → **host'a delege / out-of-process / streaming + resource handle.** |
| MK-19 | Çekirdek Eklenti | **BioCraft Studio motorla aynı pakette varsayılan kurulu** (İP-20); ilk açılışta hazır; **bağımsız sürümlenir** (ABI uyumu korunur). |
| MK-20 | Açık/Kapalı Sınır | **Veri-koruma güvenliğinin TAMAMI açık** (Kerckhoffs); yalnızca **premium + lisans/anti-tamper kapalı.** Kapalı parça çekirdeğe **statik bağlanmaz** (AGPL temizliği — SDK/IPC sınırı). |

### 6.C Bellek, Donanım Koruma, Performans (MK-21 — MK-30)
| MK | Başlık | Karar (özet) |
| --- | --- | --- |
| MK-21 | Bellek Orkestratörü | Tüm subprocess + DuckDB + UI + eklenti belleği **rezervasyonla** ister; bütçe aşılırsa talep **reddedilir** (OOM çökmesi YOK); baskıda LRU boşaltma. |
| MK-22 | Dosya-Öncesi Bütçe | Dosya açmadan önce boyut + tahmini RAM; yetersizse **"stream / cloud-burst (yer-tutucu) / iptal"** diyaloğu. |
| MK-23 | İşleme Öncelik Modları | Arayüz öncelikli / denge / maksimum hesap; kullanıcı başka işe/oyuna geçince hesap **kısılır** (Zero-Impact). |
| MK-24 | Donanım Koruma (Zero-Impact) | Bağımsız **watchdog thread;** GPU/CPU/NVMe sıcaklık + fan + kullanım izleme; **kademeli termal eşik tablosu** (≥85°C GPU acil durdur); kritikte checkpoint; sensör yoksa zarif devre dışı. |
| MK-25 | Disk Koruması | **%10 boşta uyarı, %2 salt-okunur + 100 MB marj;** yanlış sürücüye yazma koruması; sürücü-başına izleme. |
| MK-26 | Donanım Auto-Tuning | Başlangıçta donanım profili; **Eco/Bio modları;** otomatik ayar; düşük donanımda sadeleşme + uyarı. |
| MK-27 | Zarif Bozulma | Kapasite aşımında **özet/örnekleme moduna** geç ("çöktü" yerine "sadeleşti" + uyarı). |
| MK-28 | Self-Healing 3 Kural | (1) Veri kaybetme (otomatik kayıt + checkpoint/WAL), (2) Çökme yerine düşür (degrade chain), (3) Kullanıcıyı bilgilendir (sessiz başarısızlık YOK). |
| MK-29 | Determinizm Bayrağı | Proje/iş **"hızlı keşif / tekrarüretilebilir (bilimsel)"** bayrağı (MVP'de görünür/seçilebilir **kanca**); gerçek bit-bit garanti (Scientific_Strict) v1.x (`MVP-sonrasi.md` §9.1). |
| MK-30 | Süreç-Arası Veri | **Arrow Flight + shared memory;** gRPC sadece kontrol; IPC overhead ~1-3 ms (`process_batch([...])` ile amortize). |

### 6.D Veri, Format, State (MK-31 — MK-40)
| MK | Başlık | Karar (özet) |
| --- | --- | --- |
| MK-31 | Proje Formatı | Klasör + **`biocraft.toml`** manifest + `data/` `flows/(.bcflow)` `scripts/` `provenance/` `.biocraft_meta/`; taşınabilir **`.bcproj`** (ZIP stored); büyük dosyalar **referansla** (`large_data_refs`). Eklenti paketi **`.bcext`**. |
| MK-32 | BGZF-Farkında Okuma | Sıkıştırılmış dosyalar (BAM/VCF.gz) **BGZF blok sınırından** okunur; ham bayt parçalama **YASAK** (bilimsel doğruluk). |
| MK-33 | BLAKE3 Bütünlük | Veri/proje/güncelleme için **BLAKE3** + (gerekli yerde) imza; bozulma tespit + **karantina** + yeniden indir; sessiz açma YOK. |
| MK-34 | Provenance | Her veri için kaynak/sürüm/tarih/BLAKE3 + bilimsel set **lisans/atıf**; **köken gezgini paneli** (yöntem/teşekkür bölümü için). |
| MK-35 | E2EE Vektör DB | **LanceDB** (gömülü, E2EE-uyumlu); **Qdrant DEĞİL.** Gerçek kullanım AI/RAG motorunda (gelecek). |
| MK-36 | Undo/Redo | **Command Pattern + inverse capture;** düzenlenebilir **her** işlem geri alınabilir (dizi/anotasyon/node/parametre/ayar/görünüm); çok adımlı geçmiş. |
| MK-37 | Tek-Depo Atomiklik | Her komut **tek mantıksal depoya** dokunur (SQLite/DuckDB/RocksDB ayrı); **"çok-depoda tek atomik işlem" VAAT EDİLMEZ** (saga/2PC yok). |
| MK-38 | Otomatik Kayıt + Kurtarma | Periyodik + değişiklikte; çökme sonrası **"kurtarılan oturum";** kalıcı UI durumu (sekme/boyut/düzen/görünüm). |
| MK-39 | IPC Transport | **Windows Named Pipes + Linux UDS + tonic;** tek port multiplex; plugin diske direkt erişemez. |
| MK-40 | Crate Topolojisi | **`biocraft-types` foundation (L0);** L0→L5 **tek yön;** döngü YASAK (`cargo-machete` + topology-check); eklenti iletişimi SDK üzerinden. |

### 6.E Gizlilik, Güvenlik, AI (MK-41 — MK-50)
| MK | Başlık | Karar (özet) |
| --- | --- | --- |
| MK-41 | Yerel-Varsayılan | Varsayılan **hiçbir veri dışarı gitmez;** her dış iletişim açık onay; çevrimdışı = tam gizlilik; telemetri varsayılan kapalı/minimal+anonim. |
| MK-42 | Veri Sınıflandırma | **Normal / Hassas-PHI / Sentetik** (proje sihirbazında **zorunlu**); PHI/hassas **asla** P2P/dış-AI/dış-API'ye gitmez. |
| MK-43 | PHI Sınırı Çekirdekte | Sınıflandırma motoru **tüm dış kanalların (P2P/AI/DB) önünde** durur; **eklentiye emanet edilmez.** |
| MK-44 | Şifreleme | Hassas veri dinlenmede **AES-256-GCM;** anahtar OS güvenli deposunda (Keychain/Credential Manager); asla düz metin/kodda (`zeroize`). |
| MK-45 | Güvenli Silme + Log | "Sildim" gerçekten siler (üzerine yazma opsiyonu); loglar **PII içermez;** crash raporu **opt-in + anonim.** |
| MK-46 | AI Sağlayıcı Soyutlaması | Tek sözleşme (`generate/stream/embed/capabilities/cost`); yerel/bulut/özel **aynı arayüz;** yeni sağlayıcı = yeni eklenti, çekirdek değişmeden. |
| MK-47 | AI Çıktı Şeması | `{metin, öneriler, eylem önerileri (onaylı), kaynak/atıf, güven göstergesi, "doğrulanmalı" uyarısı, token/maliyet}`; **kör güven yok; çok-AI uyumu GARANTİ DEĞİL.** |
| MK-48 | AI Dürüstlük + Kapatılabilir | Yüzey öğeleri **"yapılandırılmadı" etiketli** (sahte işlev yok); AI **tamamen kapatılabilir** (arayüz tam çalışır); asenkron/out-of-process (60 FPS). |
| MK-49 | Klinik Değil | AI ve tüm analiz çıktısı **yalnızca araştırma/Ar-Ge/fikir** amaçlı; **klinik/tanısal karar üretmez** (UI dili + çıktı etiketi). |
| MK-50 | Dağıtık Ağ Kancaları | İP-15 yalnızca **pasif kanca;** eklenti yokken **sıfır maliyet;** P2P sadece **metadata/sonuç/eklenti;** ham/PHI **asla;** varsayılan **KAPALI.** |

### 6.F Düzen, Dağıtım, Kalite (MK-51 — MK-60)
| MK | Başlık | Karar (özet) |
| --- | --- | --- |
| MK-51 | 6-Bölge Kabuk | Title+Menü / Activity / Side / Editor (sekme+split) / Panel / Status; **klasik menü + komut paleti birlikte;** özel düzen kaydet/yükle; çoklu monitör + DPI; temel detach. |
| MK-52 | Tasarım Token + i18n | Tüm renk **`tokens.json`'dan** (kodda sabit renk yok); tema <100 ms; tüm metin **i18n (EN varsayılan + TR,** sürüm alanlı); erişilebilirlik (klavye/ekran okuyucu/renk körü dostu). |
| MK-53 | TDA (3. Derece) Zorunlu | Her İP **20 maddelik TDA** kontrol listesini karşılar (boş durum/hata/geri-alma/iptal/çakışma/kurtarma/onay…); ortak bileşenler **İP-16'da** (bir kez yaz, her yerde kullan). |
| MK-54 | Node Motoru | **Tipli/renkli portlar + DAG (döngü yok) + paralel (Rayon/Tokio) + sonuç önbelleği + `.bcflow`;** node'lar SDK ile kaydedilir; çalışma İP-08 bütçesine uyar. |
| MK-55 | Kod Editörü | Native (egui+ropey), Tree-sitter; **Python out-of-process** çalıştırma; **temel** LSP (tam zekâ/debugger v1.x); **node↔kod köprüsü** (tek yön); izole ortam. |
| MK-56 | Paketleme/Güncelleme | Win MSIX/Squirrel + Linux AppImage/Flatpak; **imzalı** (kod-signing sertifikası tüzel kişilikle); **delta auto-update + atomik geri alma;** çekirdek eklenti aynı pakette; offline/kurumsal kurulum. |
| MK-57 | Gözlemlenebilirlik | **tracing** + yapılandırılmış log + zorunlu **`correlation_id`** (W3C Trace Context); OpenTelemetry uyumlu; PII'siz. |
| MK-58 | Test/Golden | Birim + proptest + Loom + insta(snapshot) + **golden test** (IGV/samtools/bcftools ile doğruluk); edge-case eşikleri; CI'de benchmark regresyon; **AI kodu da test edilir** (kör güven yok). |
| MK-59 | Göç/Sürüm Uyumu | **Sürümlü manifest + göç geçmişi + deterministik göç fonksiyonları;** kırıcı değişiklik öncesi uyarı + yedek; daha yeni format → salt-okunur uyarı. |
| MK-60 | Hermetic Build + Bağımlılık | `Cargo.lock` kilitli (+ ops. Nix); **cargo-audit/vet/deny/machete** CI'da; **AGPL ile uyumsuz bağımlılık reddedilir;** gizli anahtar koda gömülmez. |

### 6.G Operasyonel Prensipler (0.E.1 — 0.E.5)
| Kod | Başlık | Karar |
| --- | --- | --- |
| 0.E.1 | Hardware Emulation Mode | Geliştirmede `--emulate-min` ile minimum 32 GB / düşük-donanım simülasyonu (düşük sistemde davranışı test et). |
| 0.E.2 | Shared Runtime Layer | Ortak kütüphane katmanı; binary şişmesi (her eklenti kendi runtime'ını taşımaz) engellenir. |
| 0.E.3 | Observability Contract | Zorunlu `correlation_id` + W3C Trace Context; her uzun iş `Job` (ilerleme/iptal/durum) döndürür. |
| 0.E.4 | Side-Channel Notu | Çok-kiracılı/ağ senaryoları için coarse-grained timer + jitter **mimaride yer tutar;** tam implementasyon v1.x (yerel-öncelikli MVP'de risk düşük — `MVP-sonrasi.md` §11.3). |
| 0.E.5 | AI Kodu da Test Edilir | Yapay zekanın yazdığı kod kör güvenle kabul edilmez; test + golden + kabul kriteri zorunlu (kod okuyamayan geliştiricinin tek güvenilir kalite kontrolü). |

---

## 7. "Opsiyonel" Bileşenlerin KESİN Aktivasyon Koşulları

> Yapay zekanın "her şeyi ekle" veya "hiçbirini ekleme" belirsizliğini gidermek için:

| Bileşen | NE ZAMAN aktif olur |
| --- | --- |
| **Bevy ECS canvas** | **v1'de KULLANILMAZ.** Tüm 2B/3B/genom tuvali wgpu/egui ile. İleride yoğun sahne gerektirirse opsiyonel değerlendirilir (`MVP-sonrasi.md` §5.1). |
| **cudarc (CUDA)** | SADECE: build `--features cuda` **VE** NVIDIA GPU+driver var **VE** workload bio-CUDA-optimize. Aksi halde wgpu/CPU fallback. Varsayılan: **KAPALI.** |
| **Apptainer/Docker konteyner** | SADECE: native Rust yolu olmayan ağır araç (MAFFT/MUSCLE/GATK). Windows'ta WSL2/Docker yoksa rehber + [Kur]; **uygulamayı bloklamaz** (native yol çalışmaya devam eder). |
| **Dağıtık hesaplama ağı** | SADECE kullanıcı opt-in etti **VE** veri sınıfı public/synthetic. PHI **asla.** Varsayılan: **KAPALI** (eklenti — gelecek). |
| **Cloud burst** | SADECE veri lokal RAM'i aşıyor (MK-22 tetikler) **VE** kullanıcı cloud stratejisi seçti. MVP'de **yer-tutucu.** |
| **Gerçek AI motoru** | MVP'de **YOK** (yalnızca yüzey). Yerel/bulut/RAG/asistan motorları **eklenti** olarak gelecek (`AI-Altyapisi.md` GELECEK paketleri). |
| **wasm64** | v1.0'da **KAPALI** (wasm32 only). |

---

## 8. Eklenti Sistemi — 4 Katman (Tümü Sandbox/Süreç-Dışı)

| Tier | Mekanizma | Güvenlik | Kullanım |
| --- | --- | --- | --- |
| **Tier 1: Native Rust** | WIT Component Model (Wasmtime Component) | Capability-based, ABI sabit | En hızlı; Rust crate |
| **Tier 2: WASM** | Wasmtime sandbox (memory limit + CPU fuel) | Tam sandbox; capability explicit | Herhangi bir dil → WASM |
| **Tier 3: Python/R** | Ayrı subprocess + gRPC + OS sandbox | Süreç-dışı; 2 GB hard limit; 50% CPU; 30 s timeout | numpy, scanpy, biopython… |
| **Tier 4: External Binary** | Apptainer/Docker container (opsiyonel) | Tam izolasyon; mount points | BWA, samtools, GATK, MAFFT… |

**En sık işlemler native Rust** (konteyner gerekmez; Windows dahil kutudan çıkar çalışır). IPC overhead ~1-3 ms;
hot loop'larda `process_batch([...])` ile 10K item tek çağrıda.

---

## 9. Veri ve Format Kararları (özet)

- **Proje:** klasör + `biocraft.toml` + alt klasörler + `.biocraft_meta` (format sürümü, göç geçmişi, BLAKE3). Taşınabilir `.bcproj` (ZIP stored).
- **Büyük dosyalar (BAM 50GB+) ZIP içinde DEĞİL** — `large_data_refs` ile dış referans (hash + yol).
- **Out-of-Core zorunlu (MK-09):** 100-500 GB NGS verisi 32 GB RAM'e sığmaz. Parquet column subset, predicate pushdown, streaming.
- **BGZF-farkında (MK-32) + BLAKE3 bütünlük (MK-33).**
- **Determinizm (MK-29):** MVP'de bayrak (kanca); bit-exact garanti v1.x.
- **Provenance (MK-34):** Her analizin izi + lisans/atıf; köken gezgini.
- **Dosya uzantıları:** proje manifesti `biocraft.toml` · node akışı `.bcflow` · eklenti paketi `.bcext` · taşınabilir proje `.bcproj`.

---

## 10. Sürüm Takvimi (MVP odaklı)

| Sürüm | Hedef | Anahtar Çıktı |
| --- | --- | --- |
| **v0.1 Pre-Alpha** | Faz 1 sonu | Cargo workspace (L0-L5) + TDA bileşenleri + render + bellek/donanım koruma + state/undo + 6-bölge kabuk + eklenti host/SDK |
| **v0.2 Alpha** | Faz 2-3 sonu | Launcher + proje sihirbazı/format + gizlilik/güvenlik + node + kod editörü + ayarlar + komut paleti |
| **v0.3 Alpha** | Faz 4 sonu | AI yüzey + dağıtık kanca + onboarding/şablon + pazar/haber + göç + paketleme + test/QA |
| **v0.5 Beta (MVP)** | Faz 5 sonu | **Çekirdek eklenti (BioCraft Studio):** genom tarayıcı + varyant + 3B + veritabanı arama + temel hizalama/anotasyon/dizi/hesap/node → **tam kullanılabilir ilk sürüm** |
| **v1.x+** | MVP sonrası | `MVP-sonrasi.md`'deki ertelenenler (gerçek AI motoru, dağıtık ağ, debugger/tam LSP, ileri görselleştirme, bulut, vb.) |

> **Gerçekçilik notu:** Solo + AI hızıyla MVP ~45-60 gün-oturum (her oturum 1-3 takvim günü olabilir). Gecikme normaldir; her fazda buffer bırak.

---

## 11. ÇELİŞKİ ÇÖZME KURALI (Otorite Sırası)

Bir karar çatışması olduğunda öncelik sırası (yukarıdaki kazanır):
1. **Bu ARCHITECTURE.md** (Bölüm 0 / MK / 0.E kararları).
2. `docs/specs/` detay spec dosyaları — **Somut Davranış/Spec + Kabul Kriterleri** bölümleri (`.proto`/`.wit`/CREATE TABLE/şema).
3. `docs/specs/` paketlerinin **diğer** bölümleri (Amaç/Kapsam/Varsayımlar).
4. `MVP-sonrasi.md` (neyin **ertelendiğini** ve kancasını söyler — MVP'ye dahil değil).
5. Diğer her şey (eski "BioForge" notları, internet örnekleri) — **yalnızca yukarıdakilerle çelişmediği sürece.**

---

## 12. Yapay Zeka Asistanına Kalıcı Talimatlar

1. Her oturum başında: `git log --oneline -30` çalıştır + `PROGRESS.md` oku → mevcut durumu anla.
2. Bu dosyadaki MK kararlarına %100 uy; kod yorumlarında ilgili MK'ye atıf yap (`// MK-13: capability denetimi`).
3. O günün işi hangi paketi kapsıyorsa **`docs/specs/…` içindeki ilgili İP/ÇE/YZ bölümünü oku** (tüm dosyayı değil).
4. Belirsizlik varsa **VARSAYMA, SOR.** Yanlış varsayım maliyetlidir.
5. Küçük, derlenen, test edilen adımlar at. Her oturum sonunda commit + `PROGRESS.md` güncelle (push için kullanıcıya sor).
6. Önce arayüz kontratı (trait/.proto/.wit), sonra implementasyon.
7. Kullanıcı **yazılım bilmiyor** — her oturum sonunda sade Türkçe özet ver: "Ne yaptık · Ne işe yarar / yapılmazsa ne olur · Sırada ne var."

---

## 13. Güvenlik ve Uyumluluk Kırmızı Çizgileri (ASLA İHLAL ETME)

- **PHI/hassas veri ASLA P2P/dış-AI/dış-API'ye girmez** (MK-42/MK-43). Veri sınıflandırma zorunlu (İP-02/İP-10).
- **Veri-koruma güvenlik kodu AÇIKTIR** (MK-20, Kerckhoffs); kapatma = güveni azaltır. Yalnızca lisans/anti-tamper kapalı.
- **AGPL temizliği:** kapalı parça çekirdeğe statik bağlanmaz; ayrı süreç/eklenti (SDK/IPC sınırı). *Avukat onayı — `Hukuk-ve-Operasyon.md`.*
- **Klinik değil:** ürün ve AI çıktısı yalnızca araştırma/bilgilendirme amaçlı (MK-49); klinik/teşhis için kullanılamaz feragati.
- **BioCraft Credits bir kripto-para DEĞİLDİR** — yalnızca platform-içi hizmet birimi (regülasyon riskinden kaçınmak için; gerçek ödeme öncesi hukukçu).
- **Telifli/lisanssız kod/içerik üretme** — yalnızca açık lisanslı bağımlılıklar (cargo-deny).
- **Gizli anahtar/parola/token kod içine gömülmez** — `zeroize` + OS secret manager.
- **Çok-AI uyumu "kesin doğruluk" diye sunulmaz** (MK-47); yalnızca güven sinyali; bilimsel sonuç kullanıcıca doğrulanır.

> **Not (eski mimariden ayrım):** Eski "BioForge" mimarisindeki IGSC patojen taraması ve çift-kullanım kapısı **bu sürümün kapsamında değildir** (yerel-öncelikli araştırma aracı). Veri sınırı PHI sınıflandırmasıyla korunur; biyogüvenlik taraması ileride dağıtık ağ/yayın senaryolarıyla yeniden değerlendirilebilir.

---

*Bu dosyanın sonu. `CLAUDE.md` ile birlikte okunur (ikisi BioCraft Engine'in "anayasası + iç tüzüğü"dür). Güncellemeler: her büyük mimari değişiklik bu dosyaya geri beslenir (yaşayan belge).*
