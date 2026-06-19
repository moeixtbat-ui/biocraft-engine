# BioCraft Engine — Temel Uygulama (Base App) Yol Haritası

> **Belge tipi:** STATİK mühendislik kontratı. **Yeniden beslenmeyi gerektirmez.** Her iş paketi (İP) kendi kendine yeten olacak şekilde yazıldı; tek tek, sıfır/farklı bağlamda bir kodlama aracına (örn. Claude Code) verilebilir.
> **Sürüm:** 1.2 (dondurulmuş taban) · **Tarih:** 2026-06 · **Kapsam:** Yalnızca **temel uygulama** (motor + kabuk + launcher + eklenti host'u). Çekirdek eklenti `Cekirdek-Eklenti.md`'de; AI altyapısı `AI-Altyapisi.md`'de; hukuk/operasyon `Hukuk-ve-Operasyon.md`'de.
> **Marka:** BioCraft Engine · biocraftengine.com · "The IDE for life sciences."
> **1.2 değişiklikleri (karar günlüğü):** Donanım Koruma / Zero-Impact katmanı **tam haliyle** MVP'ye alındı (İP-08). Determinizm bayrağı kanca olarak eklendi (İP-02/İP-08). Proje formatı zenginleştirildi: BLAKE3 bütünlük + ORCID + harici-veri yol ipucu + dosya-başına sınıflandırma + göç geçmişi (İP-02). Klasik menü çubuğu (İP-03), yan-yana split görünüm (İP-03), temel panel-ayırma (İP-03), Vim/Emacs tuş-profili kancası (İP-13), BGFZ-farkında okuma (çekirdek eklenti ile tutarlı), somut edge-case eşikleri (İP-08/İP-21) eklendi. Kod editörü LSP/debugger'ın v1.x'e ertelendiği netleştirildi (İP-06). Yeni TDA maddeleri eklendi (0.11). Ertelenen her özellik `MVP-sonrasi.md`'de açıklanır.

---

## NASIL KULLANILIR (yazılım bilgisi gerektirmez)

Bu belgeyi **tek seferde** bir yapay zekâya yapıştırma. Parça parça ilerle. Her parçayı kodlatma sırası:

1. **Her zaman önce `Bölüm 0`'ı (Sabitler ve Sözleşmeler) yapıştır.** Bu, tüm paketlerin ortak zeminidir (marka, teknoloji, klasör yapısı, kurallar).
2. **Sonra kodlatmak istediğin tek bir İş Paketini (örn. `İP-01 Launcher`) yapıştır.**
3. **Sonra aşağıdaki hazır komutu kullan.**

**Kodlama aracına verilecek hazır komut şablonu (kopyala-yapıştır):**

```
Aşağıda BioCraft Engine adlı bir masaüstü uygulamasının (1) ortak sabitler/sözleşmeler bloğu ve (2) tek bir iş paketi var.
Görevin: SADECE bu iş paketini, Bölüm 0'daki sabitlere ve sözleşmelere harfiyen uyarak kodlamak.
Kurallar:
- Bölüm 0'daki teknoloji yığınının, klasör yapısının ve isimlendirme kurallarının DIŞINA çıkma.
- İş paketindeki "Kabul Kriterleri"nin hepsini karşıla.
- İş paketindeki "TDA Kontrolleri"nin hepsini uygula (boş durum, hata mesajı, geri alma, durum bildirimi vb.).
- Önce dosya/modül iskeletini ve genel arayüz kontratlarını oluştur, sonra implementasyonu yaz.
- Belirsizlik olursa, İş paketindeki "Varsayımlar" bölümündeki kararları esas al; yeni varsayım gerekiyorsa açıkça belirt.
- Çıktı: tam, derlenebilir kod + kısa bir "ne yaptım / nasıl test edilir" notu.

[BURAYA BÖLÜM 0'I YAPIŞTIR]

[BURAYA TEK BİR İŞ PAKETİNİ YAPIŞTIR]
```

> **İpucu:** Paketleri sırayla (İP-00 → İP-01 → ...) ilerlet. Bir paket başka pakete bağlıysa "Bağımlılıklar" satırında yazılıdır; önce onu kodlat. Aynı paketi ikinci kez kodlatman gerekirse Bölüm 0 + o paket yine yeter; eski sohbeti açmana gerek yok.
> **Not (kontrat eki):** Bir İP, bağımlı olduğu paketin gerçek arayüz imzalarına ihtiyaç duyarsa (örn. İP-05, İP-04'ün render API'sini çağıracaksa), o bağımlı paketin **"Dosya/Modül Yerleşimi" + "Somut Davranış/Spec"** bölümünü de yanına yapıştır. Bölüm 0 ortak zemini taşır; çapraz arayüz imzaları ilgili İP'dedir.

---

## BÖLÜM 0 — SABİTLER VE SÖZLEŞMELER

> Bu bölüm her kodlama oturumunda yapıştırılır. Tüm iş paketleri buna atıf yapar.

### 0.1 — Marka ve Kimlik

| Alan | Değer |
| --- | --- |
| Ürün/motor adı | **BioCraft Engine** |
| Kısa ad | **BioCraft** |
| Launcher adı | **BioCraft** (açılış istemcisi) |
| Çekirdek eklenti adı | **BioCraft Studio** (temel kit) |
| Eklenti markası | **BioCraft Plugins** |
| Pazar markası | **BioCraft Market** |
| Domain | biocraftengine.com |
| Slogan | "The IDE for life sciences." |
| Kod ad alanı (crate ön eki) | `biocraft-` |
| Uygulama tanımlayıcısı | `com.biocraft.engine` |

> **Tarihçe notu (tek):** Motorun eski adı "BioForge" idi; domain/marka çakışması nedeniyle **BioCraft Engine** olarak değişti. Tüm kod, dosya ve kullanıcı-görünür metinlerde **yalnızca BioCraft** kullanılır.

### 0.2 — Ürün Tanımı ve Felsefe

BioCraft Engine; biyoinformatik için **motor + launcher + eklenti host'u**dur (analoji: Epic Games Launcher → Unreal Engine). Temel ilkeler:

- **Modüler çekirdek + eklenti mimarisi** (VS Code / Blender modeli). Çekirdek minimaldir; yetenekler eklentiyle büyür. En temel analiz/görüntüleme bile **çekirdek eklentiden** gelir (`Cekirdek-Eklenti.md`).
- **Temel uygulama kiti varsayılan kurulu gelir** (motor + çekirdek eklenti birlikte paketlenir). Kullanıcı ilk açılışta çalışan bir uygulamayla karşılaşır; "eklenti kurulu değil" ekranı görmez. Eklenti mimarisi içeride korunur.
- **Akıcılık birinci önceliktir:** Arayüz her koşulda 60 FPS hedefler; ağır hesap asla arayüzü dondurmaz. Hedef maksimum hızdır; çok düşük donanımda kabul edilebilir taban 30 FPS + uyarıdır.
- **Zero-Impact (donanım dostu):** Uygulama kullanıcının donanımına asla bilinçli zarar vermez. Sıcaklık/yük izlenir, kritik eşikte yük kademeli azaltılır veya durdurulur (İP-08 Donanım Koruma). Bu, akıcılıkla birlikte ikinci temel güvencedir.
- **Foolproof (3. derece) tasarım:** Her özellik eksik durum, hata toleransı, geri alma ve durum bildirimi ile gelir (bkz. 0.11).
- **Gizlilik-öncelikli:** Varsayılan hiçbir veri dışarı gitmez; her dış iletişim açık onaya tabidir.
- **Açık çekirdek + kapalı ticari katman (denetlenebilirlik = güven):** Çekirdek motor, **veri-koruma güvenliğinin tamamı** (şifreleme, anahtar yönetimi, sandbox, capability, PHI sınırı, gizlilik), dosya/proje formatları, SDK ve özelliklerin büyük çoğunluğu **açık kaynaktır.** Hassas veri emanet eden kullanıcı için güvenin temeli, güvenlik kodunun **denetlenebilir** olmasıdır (Kerckhoffs ilkesi: güvenlik gizlilikle değil, sağlam ve açık tasarımla gelir). **Kapalı olan yalnızca:** premium/kurumsal özellikler, **lisans/aktivasyon ve anti-tamper/anti-korsanlık** katmanı (şirketi korur, kullanıcı verisini değil) ve opsiyonel tescilli performans optimizasyonlarıdır. Kapalı parçalar çekirdeğe **statik bağlanmaz**; SDK/IPC sınırı üzerinden ayrı süreç/eklenti olarak çalışır (lisans/AGPL temizliği — detay `Hukuk-ve-Operasyon.md`).

### 0.3 — Hedef Donanım ve Performans Bütçesi

| Ölçüt | Hedef |
| --- | --- |
| Referans donanım (**alt sınır değil**) | Intel i5-14400F (10 çekirdek / 16 iş parçacığı), 32 GB RAM, RTX 4060 Ti (8/16 GB VRAM sınıfı) |
| Kare bütçesi | ~16 ms/kare (60 FPS); arayüz hiçbir kareyi kaçırmaz |
| Olay işleme bütçesi (kare başına) | ≤3 ms (geri-basınç: olaylar pull-based, sınırlı tampon — bkz. İP-16/0.10) |
| GPU iş parçası | ≤100 ms batch (TDR-güvenli) |
| Açılış (UI görünür) | <500 ms (aşamalı başlatma; ağır bileşenler tembel yüklenir) |
| Tema değişimi | <100 ms, flicker yok |
| Komut paleti arama | <50 ms (p99) |
| Düşük donanım | Çalışır + uyarır; ağır özellikler sadeleşir/kapanır (kimse dışlanmaz). LOD/downsampling/out-of-core/Eco devreye girer. Çok düşükte 30 FPS kabul edilebilir hedef + uyarı. |
| Donanım koruma (Zero-Impact) | Sıcaklık/yük sürekli izlenir; eşik aşımında kademeli yük azaltma/durdurma (İP-08). |

> **Not:** Referans donanım bir **performans hedefidir, minimum sistem gereksinimi değildir.** Uygulama daha zayıf donanımda da çalışır; bütün optimizasyon katmanları otomatik devreye girer. Kurulumdan önce donanım profili çıkarılır; çok düşük sistemde potansiyel sınırlamalar uyarıyla bildirilir.

### 0.4 — Tam Teknoloji Yığını Matrisi

> **🔒 Geri dönülemez seçimler:** Rust (dil), wgpu+egui (UI), Wasmtime (eklenti runtime), WIT (ABI), proje/dosya formatı. Bunlar değiştirilmez. *WASI sürüm seviyesi bu listeye dahil değildir — sürümlü ve güncellenebilirdir (aşağıya bkz.).*

| Katman | Teknoloji | Not |
| --- | --- | --- |
| Dil (çekirdek) | **Rust** | Bellek güvenliği + native performans + Cargo |
| Pencere | **winit** | Çapraz platform pencere/olay |
| GPU render | **wgpu** | WebGPU; birincil GPU yolu (2B + 3B dahil) |
| Arayüz | **egui** | Immediate-mode; hızlı yineleme |
| Ağır 3B/genom tuvali | **Bevy ECS** (opsiyonel/gelecek) | **v1'de KULLANILMAZ**; canvas wgpu/egui ile. Yoğun 3B/genom sahnesi ileride ECS gerektirirse değerlendirilir (`MVP-sonrasi.md` §5.1). |
| Asenkron | **Tokio** + **Rayon** + **Crossbeam** | async I/O + veri-paralel + mesajlaşma |
| Eklenti runtime | **Wasmtime** + WASI (component model) | Sandbox; AOT cache. **WASI sürüm seviyesi güncellenebilir** (ABI WIT ile sürümlü); eksik capability host SDK üzerinden köprülenir. |
| Python eklenti | **Süreç-dışı (subprocess) + IPC** | PyO3 in-process DEĞİL |
| R / diğer diller | Subprocess + IPC (+ opsiyonel konteyner) | |
| Konteyner | **Apptainer** (+ Docker fallback) — **opsiyonel** | Rootless, HPC standardı. En sık işlemler **native Rust** ile (konteyner gerekmez); Windows'ta WSL2/Docker yoksa yalnızca konteyner-bağımlı ağır araçlar rehberle açılır (bkz. `Cekirdek-Eklenti.md` ÇE-08). |
| Donanım izleme | **NVML** (NVIDIA) / **sysinfo** + OS sensör API'leri | GPU/CPU/NVMe sıcaklık, fan, kullanım (İP-08 Donanım Koruma) |
| Yerel DB | **SQLite** (config/meta) + **DuckDB** (analitik) + **RocksDB** (KV/cache) | Hepsi gömülü |
| Analitik | **DuckDB + Apache Arrow** | predicate pushdown, out-of-core |
| Bütünlük (checksum) | **BLAKE3** | Hızlı; veri/proje/güncelleme bütünlüğü (İP-02/İP-09/İP-11) |
| GPU compute | **wgpu** (birincil) + **cudarc** (opsiyonel, `--features cuda`) + **CPU SIMD** fallback | Tek aktif backend/workload |
| P2P (yalnız arayüz) | **Iroh** (QUIC, NAT traversal) | Gerçek kullanım dağıtık ağ eklentisinde |
| Yerel AI (yüzeysel) | **mistral.rs** (birincil) + llama.cpp fallback; **GGUF** | MVP'de sadece arayüz + demo. (Birincil/fallback sırası ileride yeniden değerlendirilebilir — düşük riskli, geri-dönülebilir; `AI-Altyapisi.md`.) |
| Vektör DB (yüzeysel) | **LanceDB** | Opsiyonel; gerçek kullanım AI motor eklentisinde |
| IPC transport | Named Pipes (Win) + UDS (Linux) + **tonic (gRPC)** | |
| Büyük veri taşıma | **Arrow Flight** + shared memory | |
| Serileştirme | Protobuf (kontrol) · Arrow (veri) · TOML (config) · JSON (dışa aktarım) | |
| Eklenti ABI | **WIT** (+ opsiyonel .proto) | SemVer'li kontrat |
| Build | **Cargo** (+ opsiyonel Nix flake); Python: pip/uv | Cargo.lock kilitli |
| CI/CD | **GitHub Actions** (ilk sprint'ten) | test/lint/golden/topology-check |
| Test | Rust test + **proptest** + **Loom** + **insta** (snapshot) + **golden test** | |
| Lint/güvenlik | clippy + rustfmt + cargo-audit + cargo-vet + cargo-machete + cargo-deny | |
| Paketleme | Win: MSIX/Squirrel · Linux: AppImage/Flatpak | imzalı binary + auto-updater (Velopack değerlendirilebilir alternatif) |
| Komut paleti | nucleo (fuzzy) | <50 ms p99 |
| Dokümantasyon | rustdoc + mdBook + bu .md dosyaları | |
| SDK dilleri | Rust + Python (önce); JS/TS (sonra) | |

### 0.5 — Crate / Modül Topolojisi ve Klasör Yapısı

**Kural:** Tek yönlü bağımlılık (DAG). `biocraft-types` hiçbir şeye bağlı değildir; üst katmanlar alta bağlanır, ASLA tersi. Döngüsel bağımlılık yasaktır (CI'da `cargo-machete` + topology-check ile denetlenir). Eklentiler birbirine doğrudan bağlanmaz, yalnızca **`biocraft-sdk`** üzerinden konuşur.

```
biocraft-engine/                 # Cargo workspace kökü
├─ Cargo.toml                    # workspace; üyeler + kilitli sürümler
├─ crates/
│  ├─ biocraft-types/            # L0: temel tipler (hiçbir şeye bağlı değil)
│  ├─ biocraft-sdk/              # L1: eklenti SDK'sı + ortak yardımcılar (types'a bağlı)
│  ├─ biocraft-ipc/              # L1: IPC/gRPC/Arrow Flight köprüleri
│  ├─ biocraft-data/             # L2: veri katmanı (SQLite/DuckDB/RocksDB, proje formatı)
│  ├─ biocraft-state/            # L2: state, otomatik kayıt, undo/redo (Command Pattern)
│  ├─ biocraft-mem/              # L2: Global Memory Orchestrator + Donanım Koruma (Zero-Impact)
│  ├─ biocraft-render/           # L3: wgpu/egui render altyapısı + tasarım token'ları
│  ├─ biocraft-plugin-host/      # L3: Wasmtime + capability + subprocess yönetimi
│  ├─ biocraft-net/              # L3: Iroh arayüzü (yalnız kanca)
│  ├─ biocraft-ai-surface/       # L3: AI yüzey/iskelet (yüzeysel — AI-Altyapisi.md)
│  ├─ biocraft-ui/               # L4: kabuk (6-bölge), menü, paneller, komut paleti, ayarlar
│  ├─ biocraft-launcher/         # L4: açılış istemcisi
│  └─ biocraft-app/              # L5: her şeyi birleştiren ana binary
├─ plugins/                      # birinci-parti eklentiler (ayrı paketlenir)
│  └─ ...                        # çekirdek eklenti burada (bkz. Cekirdek-Eklenti.md)
├─ assets/                       # tokens.json, fontlar, ikonlar, splash, demo veri, şablonlar
├─ .github/workflows/            # CI
└─ docs/                         # mdBook + .md kararlar
```

**Bounded context'ler (mantıksal sınırlar):** Görselleştirme · Analiz · Veri/IO · Eklenti · Ağ · AI. Her biri net arayüzle konuşur; sızıntı yok.

### 0.6 — İsimlendirme Kuralları

- Crate adları: `biocraft-<alan>` (kebab-case).
- Rust modül/dosya: `snake_case`; tip/trait: `PascalCase`; fonksiyon/değişken: `snake_case`; sabit: `SCREAMING_SNAKE_CASE`.
- Eklenti kimliği: `biocraft.<yayinci>.<eklenti>` (örn. `biocraft.core.studio`).
- Olaylar (event): `<alan>.<eylem>` (örn. `project.opened`, `data.loaded`).
- Kullanıcıya görünen ürün adı her zaman **"BioCraft Engine"** (kısa: "BioCraft"); kodda `biocraft`.
- Dosya formatı uzantıları: proje klasörü manifesti `biocraft.toml`; node akışı `.bcflow`; eklenti paketi `.bcext`; taşınabilir proje paketi `.bcproj` (ZIP).

### 0.7 — Kod ve Mühendislik Kuralları

- **Hata modeli:** `panic` yerine `Result` + tipli hata enum'ları. Her hata standart şemaya uyar: `{ne oldu, neden, nasıl çözülür (eylem/buton), teknik detay (katlanır), correlation_id}` (İP-16). C/C++ araç çağrıları (segfault riski) **subprocess izolasyonunda** çalışır; çöküş ana süreci etkilemez.
- **Eş zamanlılık:** Actor modeli — Tokio kanalları + typestate + cancellation token. Paylaşılan global mutable state yerine mesajlaşma. Arayüz tek thread'de 60 FPS döner; ağır iş arka plan runtime'ında. Olay akışı **pull-based + kare bütçeli** (geri-basınç: sınırlı tampon, kare başına ≤3 ms olay işle).
- **Loglama:** `tracing` + yapılandırılmış log + zorunlu `correlation_id` (W3C Trace Context). Log seviyeleri (error/warn/info hep tutulur; debug/trace bayrakla) + rotasyon. Loglar PII içermez (sanitize).
- **Konfigürasyon:** Katmanlı (varsayılan → kullanıcı → workspace → oturum), TOML, hot-reload.
- **Bellek:** Her bileşen **Global Memory Orchestrator**'dan (`biocraft-mem`) rezervasyonla bellek ister. Dosya açmadan önce bellek bütçesi kontrol edilir (bkz. İP-08).
- **Geri alma:** Düzenlenebilir her işlem Command Pattern ile geri alınabilir (bkz. İP-11).
- **Eklenti erişimi:** Eklentiler dosya sistemine doğrudan erişemez; capability-tabanlı sanal dosya sistemi (WASI VFS) + SDK çağrısı. Yetkiler manifest'te ilan edilir, kullanıcı onaylar.
- **Sürümleme:** SemVer + WIT kontratı. Kırıcı değişiklik = major sürüm.
- **Test:** Her İP kendi birim testlerini getirir; bilimsel/render çıktıları için golden test. Kritik yollar yüksek kapsam. TDA davranışları da test edilir (0.11).
- **Karar kaydı:** Mimari kararlar ADR formatında `docs/`'ta; "neden X değil Y" tablosu tutulur (örn. neden Electron değil native).

### 0.8 — Tasarım Token'ları ve Tipografi

Tüm renkler **`assets/tokens.json`**'dan gelir (kodda sabit renk yok). Tema değişimi <100 ms, flicker yok. Koyu/Açık/Yüksek-kontrast + özel tema.

| Token | Koyu | Açık |
| --- | --- | --- |
| bg.primary | #0A1628 | #FAFAFA |
| bg.secondary | #0F1E33 | #F0F2F5 |
| accent.primary | #00E5FF | #0288D1 |
| text.primary | #E6EDF3 | #1A1A1A |
| (diğerleri tokens.json'da tanımlanır) | | |

**Tipografi:** Inter (arayüz/gövde, 14px) · JetBrains Mono (kod, 13px) · Space Grotesk (başlık/display). Hepsi açık/ücretsiz lisanslı. DPI/ölçek farkındalığı baştan; 4K + çoklu monitör akıcı.

**Erişilebilirlik (zorunlu):** Klavye-tam erişim, ekran okuyucu, yüksek kontrast, renk körü dostu paletler (renge ek şekil/desen ipucu), ölçeklenebilir font.

### 0.9 — Pencere Düzeni (6-Bölge) Referansı

Ana kabuk altı bölgeden oluşur (VS Code benzeri). **Klasik menü çubuğu** Title Bar içinde/altında bulunur (İP-03).

| Bölge | Boyut | İçerik |
| --- | --- | --- |
| Title Bar (+ Menü) | 32px | Başlık, pencere kontrolleri, klasik menü (Dosya/Düzen/Görünüm/Eklenti/Yardım), komut paleti tetikleyici, hızlı eylemler |
| Activity Bar | 48px (sol) | Ana mod ikonları (proje, eklenti, arama, AI, veritabanı, ayar...) |
| Side Panel | 200–600px (yeniden boyutlanır) | Bağlama göre içerik (dosya ağacı, eklenti paneli, inspector) |
| Editor/Canvas | esnek (sekmeli + split) | Tuval / kod editörü / node editörü (sekmeler + yan-yana bölme) |
| Panel (alt) | yeniden boyutlanır | Konsol/çıktı, arka plan işleri, AI sohbet, günlük |
| Status Bar | 22px (alt) | Durum, FPS/bellek/donanım (opsiyonel), token sayacı, bağlantı durumu |

### 0.10 — Ortak Altyapı Sözleşmeleri

Bu arayüzler tüm paketlerce paylaşılır (somut imzalar İP'lerde):

- **Olay veriyolu (event bus):** Yayınla-abone ol; `<alan>.<eylem>` olayları. **Pull-based + sınırlı tampon + kare bütçeli** (eklenti binlerce olay bassa bile arayüz donmaz — geri-basınç). Eklentiler ilgili olaylara abone olur.
- **İş/ilerleme API'si:** Her uzun işlem `Job` döndürür: ilerleme (%), tahmini süre, iptal token'ı, durum (bekliyor/çalışıyor/bitti/hata). Arka plan işleri panelinde görünür.
- **Capability API'si:** `net`, `fs`, `gpu`, `ai`, `db` izinleri; kurulumda ilan, çalışmada denetim.
- **VFS:** Eklentiye yol değil, capability-kısıtlı handle verilir.
- **Provenance (köken):** Her veri için kaynak, sürüm, tarih, **BLAKE3 checksum** kaydı (`biocraft-data`). Bilimsel veri setleri için lisans/atıf alanı da tutulur (bkz. İP-10). Köken kayıtları basit bir **köken gezgini** panelinde görülebilir (İP-10).
- **Tema/i18n:** Tüm metin i18n katmanından (**EN varsayılan + TR**; mimari çok-dilli, yeni diller eklenebilir, dil ayarı sürüm alanlı); tüm renk token'dan.

### 0.11 — TDA (3. Derece Özellikler) Kontrol Listesi — ZORUNLU

> **Her iş paketi bu listeyi karşılamak zorundadır.** Bir özellik bunlar olmadan "bitmiş" sayılmaz.

1. **Eksik bağımlılık görünür:** Bir işlev eksik eklenti/araca bağlıysa, durum açıkça gösterilir ("X kurulu değil") + **[İndir/Kur]** butonu + neden. Asla sessiz başarısızlık.
2. **Geri al/ileri al:** Düzenlenebilir her işlem geri alınabilir (çok adımlı geçmiş).
3. **Durum bildirimi:** Asenkron işlemde ilerleme + tahmini süre + iptal + "ne yapılıyor" açıklaması.
4. **Anlamlı hata mesajı:** Ne oldu + neden + nasıl çözülür (eylem/buton). Asla kriptik kod.
5. **Boş durum rehberi:** Boş panel/liste "ne yapılacağını" anlatır + birincil eylem butonu.
6. **Yükleniyor durumu:** Anlamlı gösterge (iskelet/ilerleme), donuk ekran değil.
7. **Yıkıcı işlemde onay:** Geri-döndürülemez işlemde onay + mümkünse geri alma.
8. **Anlık girdi doğrulama:** Geçersiz girdi anında + açıklamayla işaretlenir.
9. **Akıllı varsayılan:** Bağlama/donanıma uygun varsayılan; ama her şey ayarlanabilir.
10. **Otomatik kaydetme + kurtarma:** İş kaybı önlenir; çökme sonrası oturum kurtarılır.
11. **Durum farkındalığı:** Çevrimdışı/çevrimiçi, kaynak/donanım yetersizliği vb. net gösterilir.
12. **İptal edilebilirlik:** Her uzun işlem temiz iptal edilebilir (yarım durum bırakmaz).
13. **Keşfedilebilirlik:** Gizli özellik yok; menü/palet/tooltip ile bulunabilir.
14. **Tutarlılık:** İkon/renk/terminoloji tutarlı; tema ve i18n'e uyar; eklentiler de uyar.
15. **Son işlem geri bildirimi:** İşlem bitince net sonuç ("Kaydedildi", "5 varyant bulundu"); sessiz tamamlama yok.
16. **Büyük işlem öncesi tahmin + onay:** Uzun sürecek işlemde "bu ~X dk sürebilir, devam?" uyarısı.
17. **Kaydedilmemiş kapatma koruması:** Kaydedilmemiş işle kapatmada uyarı + kaydet seçeneği; düzen/sekme durumu kaydedilir.
18. **Çakışma tespiti:** Aynı dosya iki yerde değişirse tespit + uyarı + sürüm seçimi; sessiz ezme yok.
19. **Taşınmış/eksik kaynak kurtarma:** Bulunamayan dosya/proje için "yeniden bağla" yolu; yarı yolda bırakmaz.
20. **Bağlamsal yardım:** Her öğede tooltip/"bu ne işe yarar" + ilgili dokümana bağlantı; biyoloji kavramları için opsiyonel ipuçları.

### 0.12 — Her İş Paketinin Şablonu

Her İP şu alanları içerir: **Amaç · Kapsam · Bağımlılıklar · İlgili crate(ler) · Teknoloji · Somut Davranış/Spec · Dosya/Modül Yerleşimi · TDA Kontrolleri · Kabul Kriterleri · Varsayımlar · Dikkat.**

---
## İŞ PAKETLERİ

> Sıra: İP-00 → İP-21. "Bağımlılıklar" satırındaki paketleri önce kodlat.

### İP-00 — Proje İskeleti, Crate Topolojisi ve CI

**Amaç:** Derlenebilir boş workspace + bağımlılık DAG'ı + CI hattı kurmak. Her şeyin üstüne ekleneceği zemin.
**Kapsam:** Cargo workspace, 0.5'teki tüm crate'lerin boş iskeletleri (derlenir), GitHub Actions, lint/topology denetimi.
**Bağımlılıklar:** Yok (ilk paket).
**İlgili crate(ler):** Hepsi (boş iskelet).
**Teknoloji:** Cargo, GitHub Actions, clippy/rustfmt/cargo-audit/cargo-vet/cargo-machete/cargo-deny.

**Somut Davranış/Spec:**
- 0.5'teki klasör yapısını birebir oluştur. Her crate `cargo build` ile derlenir (içi boş/stub olabilir).
- `biocraft-types` hiçbir workspace crate'ine bağlı değil; bağımlılık yönü yalnızca aşağıdan yukarı.
- `biocraft-app` çalıştırıldığında boş bir pencere açar (winit + wgpu + egui "merhaba" penceresi) ve <500 ms'de görünür.
- CI: her push'ta `cargo build`, `cargo test`, `clippy -D warnings`, `rustfmt --check`, `cargo-audit`, ve topology-check (döngüsel/kullanılmayan bağımlılık → hata).
- `Cargo.lock` kilitli; `cargo-deny` ile lisans politikası (açık çekirdek AGPLv3 ile **uyumsuz** bağımlılık reddedilir — detay `Hukuk-ve-Operasyon.md`).

**Dosya/Modül Yerleşimi:** Kök `Cargo.toml` (workspace), `crates/*/Cargo.toml` + `src/lib.rs` (veya `main.rs` for app), `.github/workflows/ci.yml`, `deny.toml`, `rustfmt.toml`, `clippy.toml`.

**TDA Kontrolleri:** Boş pencere bile "yükleniyor" yerine anlamlı ilk kare gösterir (madde 6). Derleme/CI hataları net raporlanır (madde 4).

**Kabul Kriterleri:**
- [ ] `cargo build --workspace` ve `cargo test --workspace` hatasız.
- [ ] `biocraft-app` boş pencereyi <500 ms'de açar.
- [ ] CI yeşil; topology-check döngüsel bağımlılıkta kırmızıya döner (test edilmiş).
- [ ] `cargo-deny` lisans politikası aktif.

**Varsayımlar:** İlk sürüm Windows + Linux hedefler (macOS sonra — `MVP-sonrasi.md` §11.1). GPU testleri için self-hosted runner sonradan eklenir.
**Dikkat:** Bağımlılık yönünü baştan doğru kur; sonradan döngü çözmek pahalıdır.

---

### İP-01 — Launcher (Açılış İstemcisi)

**Amaç:** Uygulama açıldığında ilk görünen, motoru başlatan istemci. Bilim haberleri, şirket duyuruları ve son projeleri gösterir.
**Kapsam:** Launcher penceresi, son projeler listesi (taşınmış proje kurtarma dahil), haber/duyuru akışı (asenkron), "Yeni Proje" (→ İP-02) ve "Proje Aç" eylemleri, donanım ön-kontrolü, motora geçiş.
**Bağımlılıklar:** İP-00. (Haber akışı için ağ; offline çalışmalı.)
**İlgili crate(ler):** `biocraft-launcher`, `biocraft-ui`, `biocraft-render`.
**Teknoloji:** egui/wgpu; ağ için Tokio (asenkron, donmaz).

**Somut Davranış/Spec:**
- Açılışta launcher <500 ms görünür (hedef <150 MB RAM, hafif); haber akışı arka planda yüklenir (iskelet gösterilir, gelince dolar).
- **Son Projeler:** Ad, yol, son açılma tarihi, küçük önizleme; tıkla → motorda aç. Sabitleme (pin)/kaldırma/arama. Taşınmış proje için **"bulunamadı — yeniden bağla"** (madde 19). Liste boşsa "Henüz proje yok — Yeni Proje oluştur" rehberi (boş durum).
- **Haber/Duyuru:** İlgi alanına göre (opsiyonel) bilim haberleri + şirket duyuruları (sürüm notları). Kaynaklar küratörlü; "doğrulanmış" rozeti. Tıkla → detay/dış bağlantı (onayla).
- **Eylemler:** [Yeni Proje] → İP-02 sihirbazı; [Proje Aç] → dosya seçici; [Ayarlar] → İP-12; [Yardım/Dokümanlar] + eğitim modunu tekrar başlat.
- **Donanım ön-kontrolü:** İlk açılışta sistem donanımı tespit edilir; referans altıysa uyarı + "yetenek matrisi" (ne yapılabilir/yapılamaz). Kullanıcı dışlanmaz, bilgilendirilir.
- **Changelog:** Her sürümde "Yenilikler" kartı (görmezden gelinebilir, Yardım'dan tekrar okunabilir).
- Çevrimdışıyken: "Çevrimdışı" göstergesi + son önbellek; bağlantı gelince güncellenir. Uygulama etkilenmez.
- Launcher hafiftir; ana uygulamadan ayrı, performansı etkilemez. Tema/dil ana uygulamayla ortak. Sistem tray'de kalma opsiyonel (varsayılan tray).
- **Başlatma protokolü:** Launcher motoru argümanlarla (proje yolu, sürüm, mod) başlatır; durum IPC/dosya ile paylaşılır.

**Dosya/Modül Yerleşimi:** `crates/biocraft-launcher/src/{lib.rs, news.rs, recent.rs, hardware_check.rs, view.rs, launch.rs}`.

**TDA Kontrolleri:** Boş durum rehberi (5); taşınmış proje kurtarma (19); haber yüklenirken iskelet (6); çevrimdışı durumu net (11); haber alınamazsa sessiz değil, "şu an haber yüklenemiyor" + tekrar (4); dış bağlantıya gitmeden önce kullanıcı onayı.

**Kabul Kriterleri:**
- [ ] Launcher <500 ms görünür; haber asenkron gelir, arayüz hiç donmaz.
- [ ] Son projeler doğru listelenir ve açılır; boş liste rehberi + taşınmış proje "yeniden bağla" çalışır.
- [ ] Çevrimdışı modda launcher tam çalışır (önbellek + durum göstergesi).
- [ ] Donanım ön-kontrolü + yetenek matrisi uyarısı çalışır.
- [ ] "Yeni Proje" sihirbazı ve "Proje Aç" çalışır.

**Varsayımlar:** Haber kaynağı MVP'de basit bir uzak JSON akışı (RSS/REST). Tam doğrulanmış haber ağı + Bilim Pazarı sonra olgunlaşır (İP-18, `MVP-sonrasi.md` §10.2). Yan-yana çoklu motor sürümü v1.x (`MVP-sonrasi.md` §8.3); MVP'de tek sürüm + proje sürüm kaydı.
**Dikkat:** Haber akışı asla launcher'ı bloklamamalı; ağ tamamen asenkron. Ayrı launcher (Epic-benzeri) **kalıcı bir üründür**, kaldırılmaz.

---

### İP-02 — Proje Sihirbazı ve Proje Formatı

**Amaç:** Yeni proje oluştururken veri/gizlilik/proje-bilgisi/dağıtık-ağ seçeneklerini toplamak; taşınabilir, zengin proje formatını kurmak.
**Kapsam:** Çok adımlı sihirbaz, `biocraft.toml` manifest + klasör yapısı, gizlilik profili, veri sınıflandırma, dağıtık ağ eklenti durumu gösterimi, BLAKE3 bütünlük, taşınabilir paket export.
**Bağımlılıklar:** İP-00. (Proje formatı burada tanımlanır ve `biocraft-data`'da uygulanır.)
**İlgili crate(ler):** `biocraft-ui` (sihirbaz), `biocraft-data` (proje formatı/manifest).
**Teknoloji:** egui (sihirbaz), TOML (manifest), BLAKE3 (bütünlük/provenance).

**Somut Davranış/Spec:**
- **Adımlar:** (1) Şablon/tür (Genomik, Proteomik, CRISPR/Gen Düzenleme, Boş — seçim eklenti/panel ön-kurulumunu belirler) → (2) Proje adı/konum/açıklama/kurum/etiketler/opsiyonel ORCID → (3) Veri ayarları (yerel/bağlantılı; büyük veri referansla mı gömülü mü) → (4) **Veri sınıflandırma (zorunlu: Normal / Hassas-PHI / Sentetik)** + gizlilik profili (varsayılan: tamamen yerel; "anonimleşmiş sonuçları AI havuzuna katkı: Hayır" varsayılan) + güvenlik (şifreleme açık/kapalı, varsayılan şifreli-yerel) → (5) Dağıtık ağ (eklenti kuruluysa seçenek; **kurulu değilse "Dağıtık ağ için eklenti gerekli — [İndir]" gösterilir**) → (6) Özet + Oluştur.
- Her adımda ilerleme göstergesi + Geri/İleri + anlık doğrulama; "İleri" geçersiz adımda pasif; iptal temiz (yarım kalıntı bırakmaz). Donanıma göre akıllı varsayılan (düşük RAM'de stream modu önerisi).
- **Proje formatı (zengin):** Bir klasör + `biocraft.toml` manifesti + `data/` (inputs/intermediate) + `flows/` (.bcflow) + `scripts/` + `provenance/` + `.biocraft_meta/` (format sürümü, oluşturma tarihi, uygulanan göçler, BLAKE3 bütünlük). Manifest alanları:
  - Kimlik: proje adı, açıklama, oluşturulduğu **BioCraft sürümü**, format sürümü, oluşturma/değiştirme tarihi.
  - Oluşturan: opsiyonel ORCID + ad/kurum.
  - **Veri sınıflandırma** (proje geneli) + uyumluluk etiketleri (örn. "Akademik", "GDPR-OK") + lisans.
  - **Harici büyük veri referansları:** her biri için `{mantıksal yol, gerçek yol ipucu, boyut, BLAKE3, sınıflandırma}` (50 GB BAM klasörde değil, kullanıcı diskinde — referansla tutulur).
  - Uygulanan göç (migration) geçmişi + etiketler + işbirlikçi alanı (gelecek için).
- **Bütünlük (BLAKE3):** Manifest + meta + her veri referansı için BLAKE3; açılışta doğrulanır, bozuk/eksik dosya net bildirilir (sessiz açma yok).
- **Determinizm bayrağı (kanca):** Proje ayarında "hızlı keşif / tekrarüretilebilir (bilimsel)" bayrağı yer alır; MVP'de görünür/seçilebilir, gerçek bit-bit garanti v1.x (`MVP-sonrasi.md` §9.1).
- **Taşınabilirlik:** "Proje paketi olarak dışa aktar" → açık klasör + ek olarak tek-dosya `.bcproj` (ZIP, stored): manifest + küçük veri + büyük veri referansları/checksum; tam veri opsiyonel gömülür. Hassas/şifreli ayarlar export'ta varsayılan hariç (madde 7, onaysız sızmaz).
- Her proje kendi gizlilik profilini taşır (global varsayılanı geçersiz kılabilir). Tüm proje ayarları sonradan Proje Ayarları panelinden değiştirilebilir. Proje kopyalama/klonlama (ayarlar kopyalanır, referans/checksum korunur).

**Dosya/Modül Yerleşimi:** `crates/biocraft-ui/src/wizard/{mod.rs, steps.rs}`, `crates/biocraft-data/src/project/{manifest.rs, format.rs, provenance.rs, integrity.rs, export.rs}`.

**TDA Kontrolleri:** Dağıtık ağ eklentisi yoksa görünür + [İndir] (madde 1); akıllı varsayılanlar (9); adım navigasyonu + anlık doğrulama (8); açılışta bütünlük denetimi + net hata (4); oluşturma iptal edilebilir (12); hassas ayar sızmaz (7).

**Kabul Kriterleri:**
- [ ] Sihirbaz tüm adımları gezdirir; Geri/İleri, anlık doğrulama ve özet çalışır.
- [ ] Oluşturulan proje açık klasör + geçerli `biocraft.toml` (ORCID/sınıflandırma/BLAKE3/sürüm alanları dahil) üretir.
- [ ] Gizlilik varsayılanları doğru (yerel; AI havuzu = Hayır); veri sınıflandırma zorunlu.
- [ ] Dağıtık ağ eklentisi yokken [İndir] yönlendirmesi görünür.
- [ ] Proje "paket olarak dışa aktar" (`.bcproj`, checksum'lı, hassas ayar hariç) çalışır.
- [ ] Açılışta BLAKE3 bütünlük denetimi; bozuk dosya net bildirilir.

**Varsayımlar:** MVP'de "bağlantılı veri" referans + checksum ile tutulur; tam bulut senkron sonra (`MVP-sonrasi.md` §7.1). Şablonlu başlangıç İP-17'de zenginleşir. Kullanıcı şablonu kaydetme v1.x.
**Dikkat:** Manifest formatı ileride genişleyecek; **sürüm + göç geçmişi alanlarını baştan koy** ki eski projeler göç edebilsin (İP-19 göç).

---
### İP-03 — Ana Kabuk / Pencere Düzeni (6-Bölge + Menü + Split)

**Amaç:** Motorun ana arayüz iskeleti: 0.9'daki 6-bölge düzeni, klasik menü çubuğu, sekmeli + yan-yana (split) editör/tuval alanı, temel panel yönetimi.
**Kapsam:** Title Bar + klasik menü, Activity Bar, Side Panel (yeniden boyutlanır), Editor/Canvas (sekmeli + split + temel detach), alt Panel, Status Bar; bölge boyutlarının kalıcılığı; özel düzen kaydet/yükle.
**Bağımlılıklar:** İP-00.
**İlgili crate(ler):** `biocraft-ui`, `biocraft-render`.
**Teknoloji:** egui (düzen), wgpu (render).

**Somut Davranış/Spec:**
- 0.9 tablosundaki bölgeleri ve boyutları uygula. Side Panel ve alt Panel yeniden boyutlanır (sürükle); boyutlar oturumlar arası kalıcı.
- **Klasik menü çubuğu:** Title Bar'da Dosya/Düzen/Görünüm/Eklenti/Yardım menüleri (yeni kullanıcı için) + komut paleti (güç kullanıcı için) birlikte. Her aksiyon iki yoldan da erişilebilir.
- **Sekmeli + split editör:** Tuval / kod / node editörü sekme olarak açılır; sekme sürükle-yeniden sırala, kapat, "kaydedilmemiş" işareti, sabitleme (pin). **Yan-yana bölme (split, en az yatay/dikey ikiye)** — iki veriyi (örn. iki BAM) karşılaştırma için.
- **Temel panel-ayırma (detach):** Panel temel düzeyde ayrı pencereye taşınabilir + çoklu monitör + DPI ölçekleme akıcı. (Tam serbest dock düzeni v1.x — `MVP-sonrasi.md` §8.1.)
- **Activity Bar:** Proje, Eklentiler, Arama, AI, Veritabanı, Ayar modları arası geçiş; aktif mod Side Panel içeriğini değiştirir. Eklentiler kendi ikonunu ekleyebilir.
- **Inspector:** Seçili öğenin (track/node/varyant) özelliklerini bağlamsal gösteren panel; düzenlenebilir.
- **Status Bar:** Bağlantı durumu, opsiyonel FPS/bellek/**donanım göstergesi** (CPU/GPU/RAM/sıcaklık — İP-08 ile), token sayacı (AI), aktif işlem özeti.
- **Alt Panel:** Konsol/çıktı, "Arka Plan İşleri" (her Job ilerleme/iptal ile), AI sohbet, günlük sekmeleri.
- **Özel düzen kaydet/yükle:** Kullanıcı düzeni kaydeder/isimlendirir/yükler; %100 sadakatle geri gelir. "Sade/yoğun mod" geçişi.
- Tüm renk token'dan, tüm metin i18n'den (dil hot-swap). 4K + çoklu monitör + DPI ölçekleme akıcı. Glassmorphism/efektler ayardan kapatılabilir, düşük donanımda otomatik sadeleşir (60 FPS her zaman önce).

**Dosya/Modül Yerleşimi:** `crates/biocraft-ui/src/shell/{mod.rs, menu_bar.rs, activity_bar.rs, side_panel.rs, editor_area.rs, split.rs, bottom_panel.rs, status_bar.rs, title_bar.rs, layout.rs}`.

**TDA Kontrolleri:** Boş editör alanı "başlamak için..." rehberi + eylem (5); panel boyutları/düzen kalıcı (9/17); kaydedilmemiş sekme kapanırken uyarı (7/17); arka plan işleri panelde görünür + iptal (3,12); tutarlı ikon/tema (14).

**Kabul Kriterleri:**
- [ ] 6 bölge doğru boyut/davranışta; klasik menü + komut paleti birlikte çalışır.
- [ ] Paneller yeniden boyutlanır ve kalıcı; temel detach + çoklu monitör çalışır.
- [ ] Sekmeler açılır/kapanır/sıralanır/sabitlenir; yan-yana split çalışır; kaydedilmemiş işareti çalışır.
- [ ] Status Bar ve Arka Plan İşleri paneli canlı durum (donanım göstergesi dahil) gösterir.
- [ ] Özel düzen kaydet/yükle %100 sadakatle çalışır; 60 FPS korunur; 4K/çoklu monitörde akıcı.

**Varsayımlar:** Tuval/editör içeriği boş iskelet (gerçek tuval İP-04, kod İP-06, node İP-05). Tam serbest panel düzeni + çoklu pencere/aynı proje v1.x (`MVP-sonrasi.md` §8.1).
**Dikkat:** egui immediate-mode'da kalıcı durum (sekme/boyut/düzen) ayrı state'te tutulmalı (İP-11 ile uyumlu).

---

### İP-04 — Render ve Tuval Altyapısı

**Amaç:** Yüksek performanslı çizim altyapısı: wgpu render hattı, egui entegrasyonu, 2B/3B/genom tuvali, tasarım token sistemi.
**Kapsam:** wgpu cihaz/kuyruk yönetimi, kare bütçesi, batching/culling/LOD altyapısı, GPU TDR güvenliği, tema/token render, 2B plot widget temeli, 3B sahne temeli.
**Bağımlılıklar:** İP-00. (İP-03 ile birlikte çalışır.)
**İlgili crate(ler):** `biocraft-render`.
**Teknoloji:** wgpu (2B + 3B birincil), egui, cudarc (opsiyonel), CPU SIMD fallback. *Bevy ECS v1'de kullanılmaz (opsiyonel/gelecek — `MVP-sonrasi.md` §5.1).*

**Somut Davranış/Spec:**
- **Kare bütçesi:** ~16 ms/kare; aşan çizim işi parçalanır/ertelenir; arayüz hiçbir kareyi kaçırmaz.
- **GPU TDR güvenliği:** GPU işleri ≤100 ms batch'lere bölünür; sürücü çökerse (TDR/DeviceLost) durum kaydedilir → cihaz yeniden oluşturulur → geri yüklenir (**hedef <5 sn kurtarma**); iş CPU'ya düşürülebilir. Bildirim: "GPU yeniden başlatıldı".
- **Ölçeklenebilirlik altyapısı:** Görünür-alan culling + LOD API'si (büyük genom/node tuvali için); statik ekranda FPS düşürme (Eco/güç tasarrufu).
- **GPU backend seçimi:** wgpu birincil; `--features cuda` + NVIDIA + uygun workload varsa cudarc; aksi halde wgpu/CPU. Tek aktif backend/workload; VRAM bütçe yöneticisi (wgpu ve CUDA aynı anda VRAM kullanmaz, interop CPU üzerinden).
- **Token render:** `tokens.json` yüklenir; tema değişimi <100 ms, flicker yok. DPI/ölçek farkındalığı.
- **2B çizim:** egui çizim API'si + özel plot widget temeli (coverage/plot için; ağır olanlar wgpu shader).
- **3B temeli:** wgpu ile özel shader/geometri (kürdele/top-çubuk/yüzey için temel; çekirdek eklenti ÇE-07 bunu kullanır). r128 THREE.js benzeri kütüphane YOK (native).
- Sayısal hassasiyet: görselde camera-relative + determinizm; gerektiğinde f64.

**Dosya/Modül Yerleşimi:** `crates/biocraft-render/src/{lib.rs, frame_budget.rs, gpu.rs, tdr.rs, lod.rs, tokens.rs, plot.rs, scene3d.rs, backend/{wgpu.rs, cuda.rs, cpu.rs}}`.

**TDA Kontrolleri:** GPU yoksa CPU fallback + uyarı (1/11); düşük donanımda sadeleşme + uyarı (11); render hatası net (4); performans göstergesi opsiyonel (şeffaflık).

**Kabul Kriterleri:**
- [ ] Referans donanımda 60 FPS; ağır sahnede kare bütçesi korunur.
- [ ] GPU TDR simülasyonunda uygulama çökmez, <5 sn'de kurtarır.
- [ ] GPU yokken CPU fallback ile çalışır.
- [ ] Tema değişimi <100 ms, flicker yok.

**Varsayımlar:** 3B/genom tuvali tamamen wgpu ile (Bevy ECS v1'de yok). cudarc MVP'de opsiyonel/kapalı varsayılan (`MVP-sonrasi.md` §5.2).
**Dikkat:** wgpu ve CUDA aynı anda VRAM kullanmamalı; interop CPU üzerinden.

---

### İP-05 — Node Motoru (Görsel Akış Sistemi)

**Amaç:** Node tabanlı görsel akış motoru: tuval, tipli portlar, DAG çalıştırma, sonuç önbelleği. (Node'ların çoğu eklentilerden gelir; motor temelde.)
**Kapsam:** Node tuvali (pan/zoom/minimap), tipli/renkli portlar, DAG kısıtı, paralel çalıştırma, önbellek, .bcflow kaydı, gruplama, undo/redo entegrasyonu.
**Bağımlılıklar:** İP-00, İP-04 (render), İP-11 (undo/redo), İP-08 (bellek/paralel iş).
**İlgili crate(ler):** `biocraft-ui` (node tuvali) + `biocraft-sdk` (node kayıt arayüzü).
**Teknoloji:** egui (tuval), Tokio/Rayon (paralel çalıştırma), RON/JSON (.bcflow).

**Somut Davranış/Spec:**
- **Tuval:** Pan/zoom, minimap, "tümünü sığdır"; node ekleme (sağ tık + aranabilir palet + sürükle); sticky note/etiket.
- **Portlar:** Tipli ve renkli; uyumsuz portlar bağlanamaz, uyumlu olanlar vurgulanır; gerekirse otomatik dönüştürücü node önerilir. Çıktı çok node'a (fan-out), giriş tek bağlantı.
- **DAG:** Döngü oluşturulamaz (anlık görsel uyarı + bağlantı reddi).
- **Çalıştırma:** Manuel "Çalıştır" + opsiyonel canlı mod (ağır node'larda uyarı). Bağımsız dallar paralel (Rayon/Tokio), bellek bütçesi gözetilir (İP-08). Değişmeyen node sonucu önbellekten; sadece değişen alt-graf yeniden hesaplanır.
- **Durum:** Her node durum halkası (bekliyor/çalışıyor/bitti/hata); hata o dalı durdurur, bağımsız dallar devam; kabloya tıkla → ara veri önizleme.
- **Kayıt:** `.bcflow` (RON/JSON; sürüm + node id + bağlantı + parametre); git-diff alınabilir. Görsel dışa aktarma (PNG/SVG).
- **Eklenti entegrasyonu:** Eklentiler `biocraft-sdk` üzerinden node kaydeder (giriş/çıkış portları, parametre şeması, çalıştırma fonksiyonu).
- **Node → Kod:** Temel node'lar için "eşdeğer Python/komut script'i olarak dışa aktar" (köprü; Kod → Node ters yön MVP'de yok — `MVP-sonrasi.md` §3.3).

**Dosya/Modül Yerleşimi:** `crates/biocraft-ui/src/node/{canvas.rs, port.rs, dag.rs, run.rs, cache.rs, serialize.rs}`, `crates/biocraft-sdk/src/node.rs` (kayıt arayüzü).

**TDA Kontrolleri:** Boş tuval rehberi + şablon önerisi (5, İP-17); yanlış kablo takılamaz (8/foolproof); node işlemleri geri alınabilir (2); çalışma ilerleme/iptal (3,12); geçersiz parametre anlık işaretlenir (8); büyük grafikte culling (İP-04).

**Kabul Kriterleri:**
- [ ] Node ekleme/bağlama/taşıma/silme + undo/redo çalışır.
- [ ] Tipsiz/döngüsel bağlantı engellenir; uyumlu portlar bağlanır.
- [ ] Bağımsız dallar paralel çalışır; önbellek değişmeyen node'u atlar; arayüz donmaz.
- [ ] `.bcflow` kaydet/aç doğru; PNG/SVG dışa aktarma çalışır.
- [ ] Eklenti SDK'sı ile örnek node kaydı çalışır.

**Varsayımlar:** Çift yönlü canlı senkron (node↔kod aynı anda) MVP'de yok; aktif görünüm tektir. Subgraph/gruplama temel düzeyde. 1000+ node için culling/LOD (İP-04). İleri node'lar sürümlerle artar (`MVP-sonrasi.md` §4.5).
**Dikkat:** Çalıştırma motoru bellek orkestratörünü (İP-08) kullanmalı; aksi halde paralel ağır node'lar OOM riski.

---
### İP-06 — Kod Editörü ve Hibrit Köprü

**Amaç:** Native kod editörü + node sistemiyle hibrit köprü. (Dil zekâsı/LSP eklentiyle genişler; editör çekirdeği temelde.)
**Kapsam:** Native metin editörü, Tree-sitter söz dizimi, Python öncelikli **temel** LSP (out-of-process), hücreli + tam script çalıştırma, node↔kod köprüsü, izole ortam.
**Bağımlılıklar:** İP-00, İP-03, İP-05 (node köprüsü), İP-11 (undo/redo).
**İlgili crate(ler):** `biocraft-ui` (editör) + `biocraft-plugin-host` (out-of-process çalıştırma).
**Teknoloji:** egui + ropey (metin), Tree-sitter (vurgulama), subprocess + IPC (Python/R çalıştırma), pyright/jedi (temel LSP, out-of-process).

**Somut Davranış/Spec:**
- **Editör:** Native (egui + ropey/özel render); Monaco/web YOK. Çoklu dosya/sekme, proje ağacı, dosyalar arası gezinme.
- **Diller:** Python öncelik; R, Bash, JSON/YAML/RON. Tree-sitter ile artımlı vurgulama (büyük dosyada akıcı).
- **Tamamlama (TEMEL):** Python için **temel** LSP (pyright/jedi) **out-of-process**. ⚠️ **Tam akıllı tamamlama, diğer diller ve tam dil zekâsı v1.x'e ertelendi** (`MVP-sonrasi.md` §3.1).
- **Çalıştırma:** Hem hücreli (Jupyter benzeri) hem tam script. Kod **out-of-process** çalışır (arayüz donmaz; "durdur" her an); sonsuz döngü/kötü kod zaman/bellek limiti + zorla sonlandırma ile izole.
- **Büyük dosya:** Sanal/akışlı yükleme (out-of-core); 1 GB log RAM'e alınmadan açılır.
- **Köprü:** "Bu node'u kod olarak aç" / "bu kodu node akışına ekle"; ortak çalışma alanı (workspace) değişkenleri (node çıktısı kodda, kod sonucu node'da).
- **İzole ortam:** Her proje kendi sanal ortamı/konteyneri; arayüzden paket arama/kurma, sürüm kilidi.
- **Biçimlendirme/lint:** Python için ruff/black (out-of-process), kaydet'te opsiyonel otomatik biçimlendirme.
- **AI yardımı:** Yüzeysel yer (buton/panel hazır); gerçek AI İP-14 + `AI-Altyapisi.md`.
- ⚠️ **Debugger (breakpoint/adım adım) v1.x'e ertelendi** (MVP'de log-tabanlı — `MVP-sonrasi.md` §3.2).

**Dosya/Modül Yerleşimi:** `crates/biocraft-ui/src/editor/{mod.rs, syntax.rs, run.rs, bridge.rs, tree.rs}`, `crates/biocraft-plugin-host/src/exec/{python.rs, lsp.rs}`.

**TDA Kontrolleri:** Kötü kod arayüzü kilitlemez (out-of-process izolasyon); çalıştırma ilerleme/durdur (3,12); hata anlamlı + satır (4); kod değişiklikleri undo/redo + yerel geçmiş (2,10); eksik araç/paket → [Kur] (1).

**Kabul Kriterleri:**
- [ ] Python dosyası vurgulanır, çalıştırılır (hücreli + tam), çıktı alt panelde.
- [ ] Sonsuz döngülü kod arayüzü dondurmaz; "durdur" çalışır.
- [ ] 1 GB dosya çökmeden akışlı açılır.
- [ ] Node↔kod köprüsü veri paylaşır (ortak workspace).
- [ ] İzole ortamda paket kurma + sürüm kilidi çalışır.
- [ ] Temel Python LSP (out-of-process) çalışır; tam zekâ "v1.x" diye işaretli.

**Varsayımlar:** Tam adımlı debugger (breakpoint/DAP) ve tam LSP/diğer diller v1.x. Kod→Node ters dönüşüm yok.
**Dikkat:** PyO3 in-process KULLANMA; Python her zaman ayrı süreçte. Donmama bundan gelir.

---

### İP-07 — Eklenti Host, SDK ve Capability/Sandbox

**Amaç:** Eklenti mimarisinin kalbi: güvenli yükleme, capability izinleri, sandbox izolasyon, SDK arayüzü, UI uzantı noktaları, marketplace kurulum motoru.
**Kapsam:** Manifest keşfi/doğrulama, WASM (Wasmtime) + subprocess + konteyner runtime, capability denetimi, VFS, UI uzantı kayıt (panel/sekme/menü/komut/node/ayar), imza doğrulama, kurulum/güncelleme/kaldırma, izolasyon/çökme yalıtımı, güvenli mod.
**Bağımlılıklar:** İP-00. (Tüm eklenti-bağlı paketlerin temeli.)
**İlgili crate(ler):** `biocraft-plugin-host`, `biocraft-sdk`, `biocraft-ipc`.
**Teknoloji:** Wasmtime + WASI (component model, AOT cache), subprocess + IPC (tonic/Named Pipes/UDS), Apptainer/Docker (opsiyonel), WIT (ABI), kriptografik imza.

**Somut Davranış/Spec:**
- **Keşif/yükleme:** Manifest-tabanlı (`biocraft.<yayinci>.<eklenti>`); klasör taranır, manifest okunur, capability + ABI (WIT/SemVer) doğrulanır, sonra yüklenir. Çekirdek sürüm uyumu (min/max) denetlenir; uyumsuzsa kurulum engellenir + neden.
- **Runtime katmanları:** Birincil WASM (sandbox); Python/R **out-of-process**; ağır native araçlar **opsiyonel** konteyner (en sık işlemler native — bkz. `Cekirdek-Eklenti.md` ÇE-08). Eklentiler birbirine doğrudan bağlanmaz — yalnızca `biocraft-sdk` üzerinden. **WASM 4 GB bellek duvarı:** büyük-bellek işleri WASM içinde değil, host'a delege edilir veya out-of-process çalışır (`MVP-sonrasi.md`'de değil, mimaride çözülü).
- **Capability:** `net/fs/gpu/ai/db` izinleri manifest'te ilan; kurulumda kullanıcı görür/onaylar; çalışmada denetlenir. Varsayılan en az yetki. Eklenti dosya sistemine doğrudan erişemez (WASI VFS handle).
- **UI uzantı noktaları:** Eklenti panel/sekme/menü/komut/node/ayar kaydeder; çekirdek güvenli alanlarda gösterir. İki eklenti aynı alanı isterse öncelik/sıra yönetimi + kullanıcı düzenler (sessiz bozmaz).
- **İzolasyon:** Eklenti çökerse yalıtılır, kapatılır, kullanıcı bilgilendirilir, çekirdek + diğerleri ayakta; "yeniden başlat" sunulur. Kaynak kullanımı (CPU/RAM/GPU) eklenti başına görünür.
- **İmza/güvenlik:** Kriptografik imza + bütünlük; imzasız eklenti net uyarı. "Doğrulanmış/resmi" rozeti.
- **Kurulum/güncelleme:** Mağazadan veya `.bcext` dosyadan (çevrimdışı/kurumsal). Güncelleme otomatik/manuel, changelog, geri alınabilir; aktif eklenti güncellemesi güvenli ana ertelenir. Kaldırınca "ayarları koru/sil" sorulur (varsayılan koru).
- **Olay/iş:** Eklenti olaylara abone olur (`project.opened` vb.); uzun işlemde standart ilerleme/iptal API'si (çekirdek ilerleme çubuğunda görünür).
- **Güvenli mod:** Tüm 3. parti eklentiler kapalı başlatma (teşhis).
- **WASI esnekliği:** Eklenti çalışma ortamı WASI component model üzerinedir; WASI sürüm seviyesi güncellenebilir, eksik bir capability host SDK üzerinden köprülenir (eklenti ekosistemi standart değişiminde kırılmaz).

**Dosya/Modül Yerleşimi:** `crates/biocraft-plugin-host/src/{discover.rs, manifest.rs, runtime/{wasm.rs, subprocess.rs, container.rs}, capability.rs, vfs.rs, ui_ext.rs, install.rs, signature.rs, isolate.rs, safe_mode.rs}`, `crates/biocraft-sdk/src/{lib.rs, node.rs, ui.rs, data.rs}`.

**TDA Kontrolleri:** Eksik/uyumsuz eklenti → net durum + [İndir]/[Güncelle] (1); çökme yalıtımı + kurtarma (self-healing/10); izinler kullanıcıya açık (gizlilik); kurulum/kaldırma onaylı + geri alınabilir (7); güvenli mod (teşhis); kaynak şeffaflığı (11).

**Kabul Kriterleri:**
- [ ] Örnek WASM + örnek Python eklentisi keşfedilir, doğrulanır, yüklenir.
- [ ] Capability ihlali engellenir; eklenti VFS dışına çıkamaz.
- [ ] Çöken eklenti çekirdeği düşürmez; yeniden başlatılır.
- [ ] İmzasız eklenti uyarı verir; `.bcext` çevrimdışı kurulum çalışır.
- [ ] Eklenti panel/menü/node/komut kaydı çekirdekte görünür; aynı alan çakışması yönetilir.
- [ ] Güvenli mod tüm 3. parti eklentileri kapatır.

**Varsayımlar:** Çok sürüm yan yana (A/B) MVP'de yok (tek aktif sürüm). Tam mağaza ekonomisi (ödeme/puan) İP-18 + Hukuk dosyasında olgunlaşır (`MVP-sonrasi.md` §10.1). Çekirdek eklenti bu host üzerinde çalışır ve varsayılan kurulu gelir.
**Dikkat:** Sandbox sertleştirme İP-09 ile tamamlanır. ABI'yi (WIT) baştan sürümle; eklenti ekosistemi buna bağlı. **Kapalı ticari/premium parçalar da bu SDK/IPC sınırından geçer; çekirdeğe statik bağlanmaz (AGPL temizliği — `Hukuk-ve-Operasyon.md`).**

---

### İP-08 — Bellek Orkestratörü, Performans ve Donanım Koruma (Zero-Impact)

**Amaç:** OOM'u önleyen global bellek yönetimi + performans altyapısı + **donanıma zarar vermeyen (Zero-Impact) koruma katmanı:** rezervasyon-tabanlı tahsis, dosya-öncesi bütçe kontrolü, öncelik modları, donanım auto-tuning, sıcaklık/yük izleme ve kademeli koruma.
**Kapsam:** Global Memory Orchestrator, bellek bütçe diyaloğu, işleme öncelik modları, out-of-core zorunluluğu, donanım profili/auto-tuning, **Donanım Koruma (termal izleme + eşik tablosu + kademeli yük azaltma + kritik durdurma)**, performans metrikleri, determinizm bayrağı uygulaması.
**Bağımlılıklar:** İP-00. (İP-04, İP-05, İP-06 bunu kullanır.)
**İlgili crate(ler):** `biocraft-mem`.
**Teknoloji:** Rust (rezervasyon yöneticisi), DuckDB/Arrow (out-of-core), Tokio/Rayon (paralel), NVML/sysinfo + OS sensör API'leri (donanım izleme).

**Somut Davranış/Spec:**
- **Orkestratör:** Tüm subprocess + DuckDB + UI + eklentiler belleği rezervasyonla ister; toplam bütçe aşılırsa talep reddedilir + kullanıcı bilgilendirilir (OOM çökmesi yok). Bellek baskısında (OS sinyali) agresif cache temizleme + boştaki eklenti LRU boşaltma.
- **Dosya-öncesi kontrol:** Dosya açmadan önce boyut + tahmini RAM hesaplanır; yetersizse **"stream modunda aç / cloud-burst (yer-tutucu) / iptal"** diyaloğu (bir TDA).
- **Öncelik modları:** Arayüz öncelikli / denge / maksimum hesap; kullanıcı seçer. Kullanıcı başka işe/oyuna geçince hesap kısılabilir (Zero-Impact).
- **Out-of-core:** Büyük veri için zorunlu akışlı işleme (DuckDB/Arrow, mmap); tüm dosya RAM'e alınmaz.
- **Auto-tuning:** Başlangıçta donanım profili (CPU/RAM/GPU) çıkarılır; ayarlar otomatik uyarlanır (Eco/Bio modları). Düşük donanımda sadeleşme + uyarı; çok düşükte 30 FPS hedefi.
- **Donanım Koruma (Zero-Impact — tam):** Yüksek-öncelikli ayrı izleme thread'i (watchdog) donanım sağlığını sürekli izler. İzlenenler: GPU/CPU/NVMe sıcaklık, fan/RPM (varsa), kullanım yüzdesi, disk durumu. **Termal eşik tablosu (kademeli):**

  | Bileşen | Sıcaklık | Aksiyon |
  | --- | --- | --- |
  | GPU | <70°C | Tam kapasite |
  | GPU | 70–75°C | Yük ~%75 |
  | GPU | 75–80°C | Yük ~%50 |
  | GPU | 80–85°C | Duraklat, soğumayı bekle |
  | GPU | >85°C | Acil durdur + uyarı |
  | CPU | <75°C | Tam kapasite |
  | CPU | 75–85°C | Yük ~%75 |
  | CPU | 85–95°C | Duraklat |
  | CPU | >95°C | Acil durdur |
  | NVMe | <60°C | Tam I/O |
  | NVMe | 60–70°C | I/O ~%50 |
  | NVMe | >70°C | I/O duraklat |

  Duraklamada checkpoint alınır (veri kaybı yok); soğuyunca otomatik devam. Status bar'da "soğutuluyor" rozeti. Sensör okunamayan donanımda koruma kademeli devre dışı + bilgi (çökme değil). Eşikler ayardan ince ayarlanabilir.
- **Disk/kaynak koruması (somut eşikler):** Yazma öncesi disk kontrolü; **%10 boşta uyarı, %2'de salt-okunur + 100 MB güvenlik marjı.** Yanlış sürücüye yazma koruması; sürücü-başına izleme.
- **Metrikler:** Kare süresi, bellek tepe/ortalama, CPU/GPU, I/O gecikmesi, **sıcaklık** izlenir; opsiyonel panelde/status bar'da gösterilir; CI'de regresyon benchmark.
- **Zarif bozulma:** Kapasite aşılırsa özet/örnekleme moduna geçer ("çöktü" yerine "sadeleşti" + uyarı).
- **Enerji:** Eco modu; boştayken düşük güç, statik ekranda FPS düşür.
- **Determinizm bayrağı (kanca):** Proje/iş "tekrarüretilebilir (bilimsel)" işaretliyse hesap yolu deterministik moda hazırlanır; gerçek bit-bit garanti v1.x (`MVP-sonrasi.md` §9.1).

**Dosya/Modül Yerleşimi:** `crates/biocraft-mem/src/{orchestrator.rs, budget.rs, priority.rs, autotune.rs, hardware_guard.rs, thermal.rs, disk_guard.rs, metrics.rs, determinism.rs}`.

**TDA Kontrolleri:** Dosya-öncesi bütçe diyaloğu (1/11); kaynak/donanım yetersizliği proaktif uyarı (11/16); öncelik modu kontrolü; zarif bozulma (hata toleransı); metrik şeffaflığı; düşük donanımda uyarı + sadeleşme (11); termal duraklamada checkpoint (10).

**Kabul Kriterleri:**
- [ ] Bellek aşımı çökme yerine reddetme + bildirim üretir.
- [ ] Büyük dosya açılışında bütçe diyaloğu doğru tetiklenir; out-of-core ile RAM'den büyük veri işlenir.
- [ ] Auto-tuning donanıma göre ayar yapar; düşük donanımda sadeleşir.
- [ ] **Donanım Koruma: termal eşik tablosu uygulanır (simüle sıcaklıkta kademeli yük azaltma + kritikte durdurma + checkpoint); sensör yoksa zarif devre dışı.**
- [ ] **Disk dolu: %10 uyarı, %2 salt-okunur + güvenlik marjı çalışır.**
- [ ] CI'de performans benchmark eşik aşımını yakalar.

**Varsayımlar:** Eklenti-arası shared-memory fast-path v1.x'e ertelenebilir (MVP'de basit yol). Cloud-burst seçeneği temel kanca; gerçek bulut sonra (`MVP-sonrasi.md` §7.3). Fan akustik smoothing/voltaj gibi ileri donanım koruma kısımları v1.x; MVP'de termal eşik + yük azaltma + disk koruma var.
**Dikkat:** Tüm ağır iş (node, kod, render, eklenti) bu orkestratöre + donanım korumaya uymalı; aksi halde OOM/aşırı ısınma garantisi bozulur. Donanım izleme watchdog'u ana süreç çökse bile bağımsız çalışmalı. *Tescilli performans optimizasyonları kapalı katmanda olabilir; ancak temel orkestratör + bütçe + donanım koruma mantığı açıktır.*

---
### İP-09 — Güvenlik, Şifreleme ve Sandbox Sertleştirme

**Amaç:** Çekirdek güvenlik katmanı: veri şifreleme, sandbox sertleştirme, kimlik bilgisi saklama, bütünlük, güvenli güncelleme temeli, kötü veri saldırılarına direnç.
**Kapsam:** AES-256 dinlenmede şifreleme, OS güvenli kimlik deposu, dosya doğrulama/limit, sandbox kaçış savunması, imzalı güncelleme bütünlüğü, güvenli silme, fuzzing hedefleri.
**Bağımlılıklar:** İP-00, İP-07 (sandbox), İP-08.
**İlgili crate(ler):** `biocraft-data` (şifreleme/silme), `biocraft-plugin-host` (sandbox sertleştirme), `biocraft-app` (güncelleme bütünlüğü).
**Teknoloji:** Rust kripto (AES-256-GCM), OS keychain/credential manager, BLAKE3 (bütünlük), cargo-audit/vet, fuzzing (cargo-fuzz).

> **Açık/kapalı ilkesi (önemli):** Veri-koruma güvenliğinin **tamamı açık kaynaktır ve denetlenebilir** (şifreleme, anahtar yönetimi, sandbox, capability, bütünlük, güvenli silme, PHI sınırı). Hassas veri emanet eden kullanıcı için güven, güvenlik kodunun görülebilir/denetlenebilir olmasından gelir (Kerckhoffs ilkesi). **Kapalı olan yalnızca lisans/aktivasyon ve anti-tamper/anti-korsanlık katmanıdır** — amacı ticari koruma (şirketi korumak), kullanıcı verisini korumak değildir.

**Somut Davranış/Spec:**
- **Şifreleme:** Hassas veri dinlenmede AES-256-GCM; anahtarlar OS güvenli deposunda (Keychain/Credential Manager), asla düz metin/kodda. Opsiyonel ek kullanıcı parolası (güç kullanıcı). *(Açık kaynak — denetlenebilir.)*
- **Sandbox sertleştirme:** WASM/süreç izolasyonu + en az yetki + çekirdek API sıkı doğrulama; capability dışı erişim reddedilir. Kötü dosya (zip bomb, bozuk format): boyut/kaynak limiti + güvenli ayrıştırma (Rust bellek güvenliği) + sandbox'ta açma. *(Açık kaynak.)*
- **Bütünlük:** Veri/eklenti/güncelleme için **BLAKE3 checksum** + imza; bozulma/yetkisiz değişiklik tespit edilir. Güncelleme imzalı + güvenli kanal (TLS); sahte/değiştirilmiş güncelleme reddedilir.
- **Kimlik:** API anahtarı/şifre OS güvenli deposunda şifreli; tek seferlik giriş.
- **Güvenli silme:** "Sildim" gerçekten siler; hassas veri için üzerine yazma opsiyonu.
- **Loglar:** PII içermez (sanitize); hata raporu opt-in + anonimleştirilmiş.
- **Bağımlılık güvenliği:** cargo-audit/vet CI'da; yeni açık → etkilenen sürüm uyarısı. cargo-deny lisans politikası.
- **Lisans/anti-tamper (KAPALI katman):** Premium aktivasyon, lisans denetimi ve anti-tamper/anti-korsanlık katmanı native derlenir + (gerekirse) sunucu-taraflı denetim; amacı ticari koruma. Bu katman ayrı süreç/bileşendir, açık veri-güvenlik koduyla karışmaz (detay `Hukuk-ve-Operasyon.md`).
- **Fuzzing:** Dosya ayrıştırıcılar fuzzing hedefi.
- **Yan-kanal (ileri):** Çok-kiracılı/ağ senaryoları için zamanlayıcı çözünürlüğü düşürme gibi önlemler mimaride yer alır (tam implementasyon v1.x — `MVP-sonrasi.md` §11.3; yerel-öncelikli MVP'de risk düşük).

**Dosya/Modül Yerleşimi:** `crates/biocraft-data/src/security/{crypto.rs, secure_delete.rs, integrity.rs, credentials.rs}`, `crates/biocraft-plugin-host/src/harden.rs`, `fuzz/`. *(Lisans/aktivasyon katmanı kapalı, ayrı paketlenir.)*

**TDA Kontrolleri:** Güvenlik sürtünmesiz (şifreleme otomatik/şeffaf, madde 9); imzasız/bozuk → net uyarı (1,4); izinsiz veri çıkışı yok (varsayılan); güvenlik hatası anlamlı (4).

**Kabul Kriterleri:**
- [ ] Hassas veri dinlenmede şifreli; anahtar OS deposunda.
- [ ] Capability dışı eklenti erişimi reddedilir (sandbox testi).
- [ ] Bozuk/kötü dosya çökme yerine güvenli reddedilir.
- [ ] Sahte/değiştirilmiş güncelleme (BLAKE3 + imza) reddedilir.
- [ ] Güvenli silme gerçekten siler; loglar PII içermez.
- [ ] cargo-audit/fuzzing CI'da aktif.

**Varsayımlar:** Hesap/2FA online özellikler için opsiyonel (yerel kullanım etkilenmez). Pentest yayın öncesi/periyodik (`MVP-sonrasi.md` §11.2).
**Dikkat:** Veri sınıflandırma (İP-10) güvenliğin önkoşuludur; hassas veri P2P/AI/dış API sınırını çekirdek seviyesinde korur. **Veri-güvenlik kodunu kapatma (denetlenebilirlik = güven); yalnızca lisans/anti-tamper kapalıdır.**

---

### İP-10 — Gizlilik, Veri Yönetimi ve Provenance

**Amaç:** Gizlilik-öncelikli veri yönetimi: yerel-varsayılan, veri sınıflandırma, granüler gizlilik ayarları, köken kaydı + köken gezgini, KVKK/GDPR hakları (erişim/silme/taşıma).
**Kapsam:** Veri sınıflandırma (normal/hassas/PHI), gizlilik profili (global + proje), anonimleştirme temeli, provenance + köken gezgini, veri ihracı/silme, dış iletişim onay akışı.
**Bağımlılıklar:** İP-00, İP-02 (proje gizlilik profili), İP-09 (şifreleme/silme).
**İlgili crate(ler):** `biocraft-data`.
**Teknoloji:** Rust, SQLite/RocksDB (meta/sınıflandırma), BLAKE3 (provenance).

**Somut Davranış/Spec:**
- **Yerel-varsayılan:** Varsayılan tamamen yerel; bulut/paylaşım yalnızca açık opt-in. Çevrimdışı = tam gizlilik.
- **Veri sınıflandırma:** Normal / Hassas / PHI etiketleri. PHI/hassas asla otomatik dışarı, P2P'ye veya dış AI'a gitmez (çekirdek seviyesi engel). Sınıf görünür. (Proje sihirbazında zorunlu seçilir — İP-02.)
- **Gizlilik profili:** Global varsayılan + proje bazında geçersiz kılma (İP-02); granüler (ne/kiminle/ne zaman) ama akıllı varsayılan; sade dilde, öne çıkan ayar bölümü (İP-12).
- **Dış iletişim onayı:** Her dış gönderim (telemetri/AI/paylaşım/veritabanı sorgusu) açık onay; ne gönderildiği şeffaf. Varsayılan telemetri kapalı/minimal+anonim, kapatılabilir.
- **Anonimleştirme:** AI havuzuna katkı opt-in (varsayılan Hayır); güçlü anonimleştirme + diferansiyel gizlilik temeli; geri-tanımlama testine açık.
- **Provenance + köken gezgini:** Her veri için kaynak, sürüm, tarih, BLAKE3 checksum; yerelde tutulur; paylaşılırsa onay. **Bilimsel veri setleri için lisans/atıf alanı:** referans genom, dbSNP/ClinVar gibi setlerin lisans + atıf yükümlülüğü kaydedilir (akademik kullanım + yöntem bölümü için — `Cekirdek-Eklenti.md` ÇE-09 ile tutarlı). Basit **köken gezgini paneli**: bir verinin nereden/ne zaman/hangi sürümle/hangi lisansla geldiğini gösterir.
- **Haklar:** Tam veri ihracı (taşınabilirlik), tam silme (unutulma hakkı, güvenli), erişim/görüntüleme. Veri minimizasyonu (sadece gerekli).
- **Saklama:** Yerel veri kullanıcı kontrolünde; bulut/paylaşım için açık saklama politikası.

**Dosya/Modül Yerleşimi:** `crates/biocraft-data/src/privacy/{classify.rs, profile.rs, consent.rs, anonymize.rs, provenance.rs, lineage_browser.rs, export.rs}`.

**TDA Kontrolleri:** Gizlilik ayarları erişilebilir/anlaşılır (5); dış gönderim öncesi onay + şeffaflık (gizlilik); hassas veri sınırı görünür (11); silme onayı + geri-döndürülemez uyarı (7); veri her zaman dışa aktarılabilir (kilitlenmeme); köken gezgini keşfedilebilir (13).

**Kabul Kriterleri:**
- [ ] Varsayılan hiçbir veri dışarı gitmez; her dış gönderim onay ister.
- [ ] PHI/hassas etiketli veri P2P/dış AI/dış API'ye gidemez (test edilmiş).
- [ ] Proje bazlı gizlilik profili global'i geçersiz kılar.
- [ ] Tam veri ihracı + tam güvenli silme çalışır.
- [ ] Provenance kaydı (lisans/atıf dahil) her veri için doğru tutulur; köken gezgini gösterir.

**Varsayımlar:** Tam bulut senkron + federe paylaşım sonra (`MVP-sonrasi.md` §7.1); MVP'de yerel + opt-in temel. KVKK/GDPR şirket süreçleri `Hukuk-ve-Operasyon.md`'de.
**Dikkat:** Veri sınıflandırma motoru tüm dış kanalların (P2P/AI/DB) önünde durmalı; bu, dağıtık ağ ve AI eklendiğinde de korunmalı.

---

### İP-11 — Durum, Otomatik Kayıt, Kurtarma ve Undo/Redo

**Amaç:** Self-healing durum altyapısı: kalıcı durum, periyodik otomatik kayıt, çökme kurtarma, kapsamlı geri al/ileri al (Command Pattern).
**Kapsam:** State yönetimi (sekme/boyut/düzen/görünüm/tercih), otomatik kayıt, crash recovery, undo/redo motoru, veri deposu tutarlılığı, çakışma tespiti.
**Bağımlılıklar:** İP-00. (İP-03, İP-05, İP-06, İP-12 bunu kullanır.)
**İlgili crate(ler):** `biocraft-state`.
**Teknoloji:** Rust (Command Pattern), SQLite/RocksDB (state/cache), atomik yazma, BLAKE3 (bütünlük).

**Somut Davranış/Spec:**
- **Kalıcı durum:** Açık dosyalar/sekmeler, panel boyutları, kayıtlı düzen, görünüm, tercihler kaydedilir; her açılışta geri yüklenir (oturumlar arası).
- **Otomatik kayıt:** Periyodik + değişiklikte; kaydedilmemiş iş kaybı önlenir.
- **Çökme kurtarma:** Çökme sonrası açılışta "kurtarılan oturum" sunulur (açık dosyalar + kaydedilmemiş değişiklikler). Uygulama "tam çökmez" (self-healing); eklenti/araç çökmesi yalıtılır (İP-07).
- **Undo/Redo:** Command Pattern + ters-işlem (inverse) yakalama; **düzenlenebilir her işlem** geri alınabilir (dizi/anotasyon/node/parametre/ayar/görünüm); çok adımlı geçmiş.
- **Atomiklik (gerçekçi kapsam):** Her komut **tek mantıksal depoya** dokunacak şekilde tasarlanır; o depoda yazma atomiktir ve geri alınabilir. Birden çok depoya dokunan nadir işlemler tek komutta birleştirilmez (saga/iki-aşamalı yerine basit atomik birim). Bu, üç ayrı motorda (SQLite/DuckDB/RocksDB) ortak işlem yöneticisi olmadan tutarlılığı garanti eder.
- **Çakışma:** Aynı dosya iki yerde değişirse tespit + uyarı + çözüm (hangi sürüm); sessiz ezme yok (madde 18).
- **Kapatma koruması:** Kaydedilmemiş değişiklikle kapatmada uyarı + kaydet seçeneği (madde 17).
- **Yerel geçmiş:** Zaman damgalı anlık görüntüler (temel düzey); tam git entegrasyonu sonra.

**Dosya/Modül Yerleşimi:** `crates/biocraft-state/src/{state.rs, autosave.rs, recovery.rs, undo.rs, command.rs, conflict.rs, history.rs}`.

**TDA Kontrolleri:** Otomatik kayıt + kurtarma (10); kapsamlı undo/redo (2); kapatmada uyarı (7/17); çakışma tespiti (18); durum kalıcılığı (9).

**Kabul Kriterleri:**
- [ ] Tüm UI durumu (düzen dahil) oturumlar arası kalıcı.
- [ ] Otomatik kayıt çalışır; simüle çökme sonrası oturum kurtarılır.
- [ ] Undo/redo node/kod/ayar/görünüm işlemlerini kapsar; her komut tek depoda atomik.
- [ ] Dosya çakışması tespit + çözüm sunar.
- [ ] Kaydedilmemiş kapatmada uyarı çıkar.

**Varsayımlar:** Tam sürüm kontrolü (git) entegrasyonu temel; cihazlar arası state senkronu opsiyonel (İP-12), sonra (`MVP-sonrasi.md` §7.1).
**Dikkat:** Command Pattern baştan tüm düzenleme yollarına uygulanmalı; sonradan retrofit etmek zordur. egui immediate-mode'da kalıcı state ayrı tutulur. **"Çok-depoda tek atomik işlem" vaat etme; her komutu tek depoya sınırla.**

---
### İP-12 — Ayarlar Sistemi (3. Derece)

**Amaç:** Kapsamlı, aranabilir, kategorize ayar sistemi; her şey ayarlanabilir ama akıllı varsayılan.
**Kapsam:** Görünüm/davranış/performans/donanım-koruma/gizlilik/kısayol/eklenti/AI/dil ayarları; arama, açıklama, varsayılana dön, dışa/içe aktarma, profiller, opsiyonel senkron.
**Bağımlılıklar:** İP-00, İP-11 (kalıcı durum). Diğer paketler ayar bölümlerini buraya kaydeder.
**İlgili crate(ler):** `biocraft-ui` (ayar ekranı), `biocraft-state` (kalıcılık).
**Teknoloji:** egui, TOML (katmanlı config), hot-reload.

**Somut Davranış/Spec:**
- **Kapsam:** Tema/font/panel boyutu/ikon/araç çubuğu boyutu (3. derece detaylar); davranış; performans/donanım (öncelik modu, GPU, bellek limiti, **termal eşik ince ayarı**, Eco/Bio — İP-08); gizlilik (granüler, sade — İP-10); klavye kısayolları (tam özelleştirme + **tuş seti profili kancası** — İP-13); eklenti ayarları (merkezi bölüm, her eklenti kendi ayarını kaydeder); AI (yüzeysel — İP-14); dil/i18n (**EN varsayılan + TR**, tarih/sayı formatı); bildirim türleri (ayrı ayrı kısılabilir).
- **Organizasyon:** Kategorize + aranabilir; her ayarın açıklaması/tooltip; "varsayılana dön" (ayar bazında + tümü).
- **Uygulama:** Çoğu anlık (canlı önizleme); yeniden başlatma gerekenler net işaretli.
- **Taşıma:** Ayar profili dışa/içe aktarma (yeni cihaz/ekip/yedek); profiller (örn. "sunum modu"); opsiyonel bulut senkron (gizlilik gözeterek, varsayılan kapalı — sonra).
- **Gelişmiş ayrımı:** Deneysel/gelişmiş ayarlar ayrı + uyarı (kazara bozulmaz). Ayar değişikliği geri alınabilir. "Fabrika ayarlarına dön" (onaylı).

**Dosya/Modül Yerleşimi:** `crates/biocraft-ui/src/settings/{mod.rs, search.rs, sections.rs, profiles.rs}`.

**TDA Kontrolleri:** Arama + kategori + açıklama (keşfedilebilirlik 13); sıfırlama (geri alma 2); gelişmiş ayrımı (7); anlık önizleme/yeniden başlatma şeffaflığı (11); açıklamalı her ayar (5/20).

**Kabul Kriterleri:**
- [ ] Tüm kategoriler + arama çalışır; her ayarın açıklaması var.
- [ ] Ayar değişiklikleri anlık veya net "yeniden başlat" işaretiyle uygulanır.
- [ ] Varsayılana dön (bazda + tümü) + fabrika ayarları çalışır.
- [ ] Ayar profili dışa/içe aktarma + profil geçişi çalışır.
- [ ] Eklentiler ayar bölümlerini merkezi ekrana kaydeder; bildirim türleri ayrı kısılabilir.

**Varsayımlar:** Bulut senkron opsiyonel/sonra (`MVP-sonrasi.md` §7.1); MVP'de yerel + profil dışa aktarma yeterli.
**Dikkat:** Ayarlar verimli saklanmalı (hızlı okuma/yazma); açılışı yavaşlatmamalı. Dil ayarı **sürüm alanlı** olsun (yeni diller eklendiğinde göç sorunsuz — İP-19).

---

### İP-13 — Komut Paleti ve Klavye Kısayolları

**Amaç:** Hızlı, keşfedilebilir komut erişimi + tam özelleştirilebilir kısayol sistemi.
**Kapsam:** Fuzzy komut paleti (<50 ms p99), kapsamlı kısayollar, özelleştirme, tuş seti profili kancası, çakışma uyarısı, kısayol referansı.
**Bağımlılıklar:** İP-00, İP-03, İP-12 (kısayol ayarı).
**İlgili crate(ler):** `biocraft-ui`.
**Teknoloji:** nucleo (fuzzy matcher), egui.

**Somut Davranış/Spec:**
- **Komut paleti:** Tüm komutlar aranabilir (fuzzy, <50 ms p99); son kullanılanlar öncelikli; eklenti komutları da listede.
- **Kısayollar:** Tüm sık işlemler kısayollu; her kısayol yeniden atanabilir; çakışma uyarısı; varsayılana dön; dışa aktarma. Kısayol referansı görünür (keşfedilebilir).
- **Tuş seti profili (kanca):** Kısayol sistemi "tuş seti profilleri" kavramını destekler (modern varsayılan MVP'de; Vim/Emacs emülasyonu v1.x — `MVP-sonrasi.md` §8.2).
- **İpuçları:** Bağlamda kısayol ipucu ("bunu Ctrl+S ile de yapabilirsiniz"); kademeli güç kullanıcıya dönüşüm.

**Dosya/Modül Yerleşimi:** `crates/biocraft-ui/src/command/{palette.rs, shortcuts.rs, keymap_profile.rs}`.

**TDA Kontrolleri:** Keşfedilebilirlik (gizli özellik yok, 13); kısayol özelleştirme + çakışma uyarısı (8); referans erişilebilir; bağlam ipuçları (kademeli öğrenme).

**Kabul Kriterleri:**
- [ ] Komut paleti <50 ms aranır; eklenti komutları görünür.
- [ ] Kısayollar yeniden atanır; çakışma uyarılır; referans gösterilir.
- [ ] Tuş seti profili kancası mevcut (modern varsayılan çalışır).

**Varsayımlar:** Vim/Emacs emülasyonu + kayıtlı makrolar v1.x (`MVP-sonrasi.md` §8.2, §8.4).
**Dikkat:** Eklenti komutları paletine güvenli kaydedilmeli (İP-07 uzantı noktası).

---

### İP-14 — AI Yüzey / İskelet (Yüzeysel/Tasarımsal)

**Amaç:** AI'ı sonradan sancısız eklemek için arayüz/sözleşme/uzantı noktalarını hazırlamak. **Gerçek AI motoru YOK** (tam spec: `AI-Altyapisi.md`).
**Kapsam:** AI paneli + bağlamsal AI butonları (işlevsiz iskelet), sağlayıcı soyutlaması, model seçim arayüzü, token/maliyet göstergesi, veri sözleşmeleri.
**Bağımlılıklar:** İP-00, İP-03.
**İlgili crate(ler):** `biocraft-ai-surface`.
**Teknoloji:** egui (arayüz); sözleşme tanımı (Rust trait); (gerçek motor: mistral.rs/llama.cpp/GGUF — sonra, `AI-Altyapisi.md`).

**Somut Davranış/Spec:**
- **Arayüz:** AI paneli (yan/alt) + bağlamsal butonlar ("yorumla", "açıkla" — görselleştirmede/kodda); sohbet alanı. Hepsi **"yakında/yapılandırılmadı" net etiketli** (sahte işlev izlenimi yok).
- **Sağlayıcı soyutlaması:** Sağlayıcı-bağımsız arayüz (yerel/bulut/özel hepsi aynı sözleşme); model seçici + API anahtarı alanı + parametreler (işlevsiz iskelet). 3. parti AI eklentilerine açık (gelecek).
- **Token/maliyet göstergesi:** Anlık token/maliyet göstergesi tasarımda yer alır; akışlı yanıt yeri (streaming + durdur).
- **Veri sözleşmeleri:** AI'ın alacağı bağlam (proje/seçili veri/görünüm — izinli, şeffaf) ve döneceği yanıt şeması (tipli: metin + öneri + **kaynak/atıf + güven göstergesi + "doğrulanmalı" uyarısı**). Bağlam erişimi gizlilik gözetir (PHI dış AI'a gitmez — İP-10).
- **Kapatılabilir:** AI tamamen kapatılabilir; kapalıyken arayüz sadeleşir, uygulama tam çalışır.

**Dosya/Modül Yerleşimi:** `crates/biocraft-ai-surface/src/{panel.rs, provider.rs, contract.rs, cost.rs}`.

**TDA Kontrolleri:** Çalışmayan öğeler net etiketli (dürüstlük, madde 4); AI yoksa uygulama tam çalışır (1/11); maliyet göstergesi (şeffaflık); kapatılabilir (tercih/gizlilik).

**Kabul Kriterleri:**
- [ ] AI paneli + butonlar görünür ama "yapılandırılmadı" etiketli; sahte işlev yok.
- [ ] Sağlayıcı soyutlaması + veri sözleşmeleri (kaynak/güven/doğrulama alanları dahil) tanımlı.
- [ ] Token/maliyet göstergesi yeri mevcut.
- [ ] AI kapatılınca uygulama tam çalışır.

**Varsayımlar:** Gerçek AI motoru, RAG, model çalıştırma `AI-Altyapisi.md`'de (`MVP-sonrasi.md` §1). Bio-kredi bağlanışı tasarımda yer, gerçek bağlanış sonra.
**Dikkat:** Hiçbir AI öğesi "çalışıyormuş gibi" görünmemeli; aksi halde güven kırılır. İşlemler asenkron tasarlanmalı (donmama).

---
### İP-15 — Dağıtık Ağ Kancaları (Temel Uygulama Tarafı)

**Amaç:** Dağıtık ağ eklentisi sonradan takıldığında sorunsuz çalışsın diye **yalnızca temel kancaları** sağlamak. **Dağıtık ağ varsayılan kurulu DEĞİL; tam mimari ayrı eklentide.**
**Kapsam:** Iroh arayüz kancası, veri paylaşım sözleşmesi (sınıflandırma-zorlamalı), kimlik/güven arayüzü, iş dağıtım soyutlaması, kaynak paylaşım sınırı arayüzü, eklenti-yok durumu gösterimi.
**Bağımlılıklar:** İP-00, İP-07 (eklenti host), İP-10 (veri sınıflandırma).
**İlgili crate(ler):** `biocraft-net`.
**Teknoloji:** Iroh (yalnız arayüz tanımı; gerçek kullanım eklentide).

**Somut Davranış/Spec:**
- **Kancalar (pasif):** Ağ keşif noktası, veri paylaşım sözleşmesi, kimlik/güven arayüzü, iş tanımı/sonuç toplama soyutlaması, kaynak paylaşım sınırı arayüzü. Eklenti yokken **sıfır maliyet** (pasif), hiçbir ağ etkinliği yok.
- **Veri sınırı (çekirdek zorlamalı):** Yalnızca meta veri/sonuç/eklenti P2P'ye uygun; ham/hassas veri yerelde, PHI asla. Eklenti ne yaparsa yapsın bu sınırı çekirdek (İP-10) korur.
- **Eklenti-yok durumu:** Proje sihirbazında/ilgili yerde "Dağıtık ağ için eklenti gerekli — [İndir]" net gösterilir (İP-02 ile tutarlı).
- **Hazır arayüz noktaları:** Sihirbaz seçeneği, durum göstergesi yeri — eklenti gelince doldurur (sancısız entegrasyon).
- **Kaynak/güven (arayüz):** Kaynak sınırı arayüzü (ne kadar CPU/GPU paylaşılır; varsayılan paylaşım yok), sonuç doğrulama/itibar arayüzü, kötü düğüm izolasyonu kancası, dayanıklılık öngörüsü (iş yeniden atama/kısmi sonuç) — gerçek mantık eklentide.

**Dosya/Modül Yerleşimi:** `crates/biocraft-net/src/{hooks.rs, contract.rs, identity.rs, job.rs, limits.rs}`.

**TDA Kontrolleri:** Eklenti yoksa görünür + [İndir] (1); hassas veri sınırı çekirdek seviyesinde korunur (11/gizlilik); kullanılmıyorken sıfır maliyet (performans); kaynak paylaşımı opt-in (özerklik).

**Kabul Kriterleri:**
- [ ] Kancalar pasif; eklenti yokken ağ etkinliği/maliyet yok.
- [ ] Veri sınıflandırma sınırı arayüz seviyesinde zorlanır (PHI dışarı çıkamaz).
- [ ] Eklenti-yok durumu [İndir] yönlendirmesi gösterir.
- [ ] Eklenti takıldığında arayüz noktaları doldurulabilir (kontrat testi).

**Varsayımlar:** Tüm gerçek dağıtık hesaplama/P2P/gönüllü hesaplama/ekonomi bağlanışı ayrı dağıtık ağ eklentisinde (bu turun kapsamı dışı; sadece kanca — `MVP-sonrasi.md` §2). Bio-kredi ölçüm arayüzü temel kanca.
**Dikkat:** Kancalar gerçekten pasif olmalı; yanlışlıkla ağ açan kod olmamalı. Veri sınırı asla eklentiye emanet edilmez — çekirdek korur.

---

### İP-16 — Bildirim, Hata ve Durum Arayüz Bileşenleri (TDA Altyapısı)

**Amaç:** Tüm paketlerin kullanacağı ortak TDA bileşenleri: bildirim/toast, hata diyaloğu, boş durum, yükleme iskeleti, onay diyaloğu, ilerleme/iş göstergesi, durum rozetleri. Bir kez yazılır, her yerde kullanılır.
**Kapsam:** Yeniden kullanılabilir egui bileşenleri + standart davranışlar; 0.11 TDA listesinin arayüz karşılığı.
**Bağımlılıklar:** İP-00, İP-04 (render). (İP-03+ bunları kullanır.)
**İlgili crate(ler):** `biocraft-ui` (ortak bileşenler modülü).
**Teknoloji:** egui, tasarım token'ları.

**Somut Davranış/Spec:**
- **Bildirim/toast:** Başarı/uyarı/hata/bilgi; otomatik kapanma + kalıcı seçenek; eylem butonu opsiyonel. **Tür bazında kısılabilir** (İP-12 ile). **Son işlem geri bildirimi** standart (madde 15).
- **Hata diyaloğu:** Standart şema (ne oldu + neden + nasıl çözülür + teknik detay katlanır + correlation_id); asla kriptik kod (madde 4).
- **Boş durum:** İkon + "ne yapılacağı" + birincil eylem (örn. "Yeni Proje" / "Veri yükle").
- **Yükleme iskeleti:** Donuk ekran yerine iskelet/ilerleme; uzun yüklemede iptal + bilgi.
- **Onay diyaloğu:** Yıkıcı işlemde "emin misiniz?" + (mümkünse) "geri alınabilir" notu.
- **Büyük işlem öncesi tahmin:** "Bu ~X dk sürebilir, devam?" (madde 16).
- **İş/ilerleme bileşeni:** Job ilerleme (%) + tahmini süre + iptal + durum; Arka Plan İşleri paneliyle entegre (İP-03).
- **Durum rozetleri:** Çevrimdışı/çevrimiçi, kaynak/donanım yetersizliği, "soğutuluyor" (İP-08), eklenti-yok ([İndir]), taşınmış kaynak ([yeniden bağla]) — standart görünüm.
- Tümü token/tema + i18n + erişilebilir (klavye/ekran okuyucu).

**Dosya/Modül Yerleşimi:** `crates/biocraft-ui/src/components/{toast.rs, error_dialog.rs, empty_state.rs, skeleton.rs, confirm.rs, estimate.rs, progress.rs, status_badge.rs}`.

**TDA Kontrolleri:** Bu paket TDA listesinin (0.11) arayüz altyapısıdır; 3,4,5,6,7,11,15,16,19 maddeleri burada somutlaşır.

**Kabul Kriterleri:**
- [ ] Tüm bileşenler örnek galeride çalışır; tema/i18n/erişilebilirliğe uyar.
- [ ] Hata diyaloğu "ne/neden/çözüm" şablonunu zorlar.
- [ ] İlerleme bileşeni iptal edilebilir ve Arka Plan İşleri paneliyle entegre.
- [ ] Durum rozetleri (donanım/çevrimdışı/taşınmış kaynak dahil) standart görünür.

**Varsayımlar:** Diğer paketler kendi hata/boş durumlarını bu bileşenlerle kurar (kopyala değil, yeniden kullan).
**Dikkat:** Bu paketi erken yaz; İP-03 sonrası her paket buna yaslanır. Tutarlılık (madde 14) buradan gelir.

---
### İP-17 — Onboarding, Eğitim Modu ve Şablonlar

**Amaç:** İlk deneyimi kolaylaştırmak: atlanabilir tur, hazır proje/akış şablonları, gömülü demo veri, bağlam içi öğreticiler.
**Kapsam:** Hoş geldin turu, "rolün?" seçimi, şablon galerisi, demo veri setleri, etkileşimli öğretici, bağlamsal yardım, uyarlanabilir ipuçları.
**Bağımlılıklar:** İP-00, İP-02 (proje/şablon), İP-03, İP-16.
**İlgili crate(ler):** `biocraft-ui` (onboarding/şablon), `biocraft-data` (demo veri/şablon yükleme).
**Teknoloji:** egui, gömülü demo veri.

**Somut Davranış/Spec:**
- **Hoş geldin turu:** Kısa (3-5 adım), **atlanabilir**: launcher → proje oluştur → ilk veri → ilk görselleştirme. "Rolün?" (Öğrenci/Araştırmacı/Geliştirici) → içerik/varsayılan uyarlanır. Deneyimliye dayatılmaz; ilerleme kaydedilir.
- **Şablonlar:** Hazır proje/akış (örn. "Varyant görselleştirme", "RNA-seq inceleme", "Protein 3B inceleme", "Dizi hizalama") + örnek veri; node şablonlarıyla entegre (İP-05).
- **Demo veri:** Küçük, indirme gerektirmeyen gömülü setler (kendi verisi olmadan deneme).
- **Öğretici:** Gerçek arayüzde adım adım ("şimdi bu butona tıklayın"); hata olursa nazik düzeltme + açıklama. Kavram ipuçları opsiyonel (BAM/VCF nedir).
- **Bağlamsal yardım:** Her öğede tooltip/"bu ne işe yarar" + ilgili dokümana bağlantı; entegre yardım (arama, çevrimdışı temel doküman).
- **Uyarlanabilir:** "Yeni başlayan/deneyimli" seçimi; içerik buna göre; ipuçları kapatılabilir.

**Dosya/Modül Yerleşimi:** `crates/biocraft-ui/src/onboarding/{tour.rs, role.rs, templates.rs, tutorial.rs, help.rs}`, `assets/demo/`, `assets/templates/`.

**TDA Kontrolleri:** Boş tuval/proje rehberi (5); atlanabilir/kapatılabilir (özerklik); öğreticide hata toleransı (4); bağlamsal yardım (keşfedilebilirlik 13/20).

**Kabul Kriterleri:**
- [ ] Tur çalışır ve atlanabilir; "rolün?" içeriği uyarlar; ilerleme kaydedilir.
- [ ] En az 3-4 şablon + demo veriyle çalışır (boş tuval yerine çalışan örnek).
- [ ] Bağlamsal yardım/tooltip + entegre doküman erişimi çalışır.

**Varsayımlar:** Topluluk/paylaşılan şablonlar (İP-18 ile) sonra; MVP'de resmi şablonlar. Tam etkileşimli kurs v1.x (`MVP-sonrasi.md` §10).
**Dikkat:** Eğitim hafif olmalı (akıcılığı bozmaz) + tamamen kapatılabilir. Öğretici gerçek araçta (oyuncak mod değil).

---

### İP-18 — Bilim Pazarı ve Doğrulanmış Haber Akışı

**Amaç:** Launcher'daki haber/duyuru akışı + eklenti/şablon/veri mağazası (BioCraft Market) arayüzü.
**Kapsam:** Doğrulanmış haber akışı (kişiselleştirilebilir), şirket duyuruları, eklenti mağazası (arama/kategori/filtre/puan), kurulum entegrasyonu, Bio-kredi yer tutucu, moderasyon/raporlama temeli, opsiyonel çok-AI çapraz kontrol.
**Bağımlılıklar:** İP-00, İP-01 (launcher), İP-07 (eklenti kurulum), İP-16.
**İlgili crate(ler):** `biocraft-launcher` (haber), `biocraft-ui` (mağaza), `biocraft-plugin-host` (kurulum).
**Teknoloji:** egui, Tokio (asenkron), uzak REST/JSON.

**Somut Davranış/Spec:**
- **Haber akışı:** Güvenilir bilim kaynakları (küratörlü) + şirket duyuruları (sürüm notları); "doğrulanmış" rozeti; ilgi alanına göre opsiyonel kişiselleştirme; yanlış bilgi filtresi/şeffaf kaynak. Çevrimdışı önbellek + durum.
- **Opsiyonel çok-AI çapraz kontrol:** Kullanıcı bir haberi/veriyi **isteğe bağlı olarak** birden çok AI sağlayıcısında çapraz kontrol ettirebilir; sistem uyum/uyuşmazlığı işaretler ("kaynaklar hemfikir / ayrışıyor"). **Önemli — dürüstlük:** Bu bir **doğruluk garantisi DEĞİLDİR.** AI'lar ortak eğitim önyargısı paylaşır ve birlikte emin bir şekilde yanılabilir; sonuç "daha yüksek güven sinyali" olarak sunulur, "kesin doğru" olarak değil. Bilimsel sonuç yine kullanıcı tarafından doğrulanmalıdır (`AI-Altyapisi.md` YZ-08 ile tutarlı). Zorunlu kapı değil; mevcut küratörlü + rozet akışını tamamlar. Yalnızca araştırma/Ar-Ge/fikir amaçlı; klinik karar üretmez.
- **Mağaza (BioCraft Market):** Eklentiler + iş akışı şablonları + veri setleri; kategori (analiz/görselleştirme/veritabanı/AI...), arama, sıralama (popüler/yeni/puan), filtre; "doğrulanmış/resmi" rozeti, geliştirici kimliği, indirme sayısı, son güncelleme. Tek tık kurulum (İP-07).
- **Denetim:** İnceleme + imza (İP-07) + puan/yorum + kötü içerik raporlama; spam/sahte filtreleme; itibar. *İçerik sorumluluğu (yanlış bilgi/kullanıcı içeriği/telif/takedown) hukuki çerçevesi `Hukuk-ve-Operasyon.md`'de.*
- **Bio-kredi (yer tutucu):** Ücretsiz/ücretli/açık kaynak etiketi; ücretli için Bio-kredi yer tutucu (gerçek ödeme/ekonomi `Hukuk-ve-Operasyon.md` + sonra). Atıf/lisans gösterimi.
- Haber/mağaza asenkron + hafif; ana uygulamayı yavaşlatmaz; gizlilik (okuma takibi minimal/opt-in).

**Dosya/Modül Yerleşimi:** `crates/biocraft-launcher/src/news.rs` (İP-01 ile), `crates/biocraft-ui/src/market/{mod.rs, search.rs, detail.rs, reviews.rs}`.

**TDA Kontrolleri:** Çevrimdışı durumu (11); haber/mağaza yüklenirken iskelet (6); kurulum onaylı + geri alınabilir (7, İP-07); doğrulanmış rozeti (güven); çok-AI sonucunda "garanti değil" etiketi (dürüstlük 4); dış bağlantı onayı.

**Kabul Kriterleri:**
- [ ] Haber akışı asenkron yüklenir, kişiselleştirilir, çevrimdışı önbellekle çalışır.
- [ ] Mağaza arama/kategori/filtre/sıralama + tek tık kurulum çalışır.
- [ ] Puan/yorum + raporlama temeli mevcut; doğrulanmış rozeti görünür.
- [ ] Opsiyonel çok-AI çapraz kontrol çalışır ve "doğruluk garantisi değil" etiketi gösterir.
- [ ] Ücretsiz/ücretli etiket + atıf/lisans gösterilir (Bio-kredi yer tutucu).

**Varsayımlar:** Tam doğrulanmış haber ağı + gerçek ödeme/Bio-kredi ekonomisi + 3. parti yayın akışı v1.x'te olgunlaşır (`MVP-sonrasi.md` §10.1, §10.2); MVP'de küratörlü haber + temel mağaza + kurulum + opsiyonel çok-AI çapraz kontrol.
**Dikkat:** Mağaza ekonomisi/ödeme + haber/yorum içerik sorumluluğu yasal konular içerir (`Hukuk-ve-Operasyon.md`); MVP'de gerçek para akışı yerine yer tutucu güvenli. Çok-AI çapraz kontrol asla "doğruluk garantisi" diye pazarlanmaz.

---
### İP-19 — Göç ve Sürüm Uyumu

**Amaç:** Eski proje/ayar/format dosyalarının yeni sürümde sorunsuz açılması (geriye dönük uyumluluk + göç).
**Kapsam:** Proje manifest göçü, ayar göçü, format sürüm denetimi, kırıcı değişiklik öncesi uyarı + dönüştürücü.
**Bağımlılıklar:** İP-00, İP-02 (proje formatı), İP-11 (state), İP-12 (ayar).
**İlgili crate(ler):** `biocraft-data` (proje/format göçü), `biocraft-state` (ayar göçü).
**Teknoloji:** Rust, sürümlü manifest/şema.

**Somut Davranış/Spec:**
- **Sürüm denetimi:** Her proje/ayar dosyasında sürüm alanı + uygulanan göç geçmişi (İP-02); açılışta sürüm okunur.
- **Göç:** Eski sürüm otomatik yeni şemaya göç edilir (deterministik göç fonksiyonları); kullanıcı manuel taşımaz. Kırıcı değişiklik öncesi uyarı + (mümkünse) otomatik dönüştürücü + yedek. Daha yeni sürümle yapılmış dosya → salt-okunur + "bu daha yeni BioCraft gerektiriyor" uyarısı.
- **Uyumsuzluk:** Çok eski/desteklenmeyen format net açıklanır + çözüm; sessiz bozulma yok.

**Dosya/Modül Yerleşimi:** `crates/biocraft-data/src/migrate/{mod.rs, project.rs}`, `crates/biocraft-state/src/migrate.rs`.

**TDA Kontrolleri:** Göç şeffaf + yedekli (hata toleransı 10); uyumsuzluk net + çözüm (4); kırıcı değişiklik uyarısı (7).

**Kabul Kriterleri:**
- [ ] Eski sürüm proje/ayar yeni sürümde açılır (göç testi).
- [ ] Kırıcı değişiklikte uyarı + yedek + dönüştürücü çalışır.
- [ ] Daha yeni/desteklenmeyen format net hata + çözüm sunar.

**Varsayımlar:** İlk sürümde göç altyapısı (sürüm alanı + çerçeve + göç geçmişi) kurulur; gerçek göç kuralları sürümler ilerledikçe eklenir (`MVP-sonrasi.md` §9.1 ilişkili).
**Dikkat:** Sürüm + göç geçmişi alanlarını İP-02/İP-12'de baştan koy; sonradan eklemek göçü zorlaştırır.

---

### İP-20 — Paketleme, Güncelleme ve Dağıtım

**Amaç:** Native kurulum + güvenli otomatik güncelleme + çevrimdışı/kurumsal dağıtım. **Temel uygulama kiti (motor + çekirdek eklenti) tek pakette kurulu gelir.**
**Kapsam:** Platform installer'ları, kit paketleme (motor + çekirdek eklenti birlikte), imzalı auto-updater (delta, geri alınabilir), çevrimdışı/kurumsal kurulum, downgrade.
**Bağımlılıklar:** İP-00, İP-07 (eklenti), İP-09 (imza/bütünlük), İP-19 (göç).
**İlgili crate(ler):** `biocraft-app` (updater), build/CI yapılandırması.
**Teknoloji:** Win: MSIX/Squirrel · Linux: AppImage/Flatpak; imzalı binary; delta güncelleme. (Velopack değerlendirilebilir alternatif.)

**Somut Davranış/Spec:**
- **Kit paketleme:** Motor + çekirdek eklenti (BioCraft Studio: analiz/görüntüleme + veritabanı — bkz. `Cekirdek-Eklenti.md`) **tek kurulumda birlikte** gelir; kutudan çıkar çalışır. En sık analiz işlemleri native Rust ile (konteyner gerekmez); native, küçük paket (Electron'dan çok küçük); gereksiz bağımlılık yok.
- **Installer:** Windows MSIX/Squirrel, Linux AppImage/Flatpak; tek tık, bağımlılık dahil; kod imzalama (OS güveni). *Kod imzalama sertifikası maliyeti/süreci `Hukuk-ve-Operasyon.md`'de (şirketleşme sıralamasıyla ilişkili).*
- **Auto-updater:** İmzalı (İP-09), güvenli kanal (TLS), bütünlük denetimi (BLAKE3); delta güncelleme; arka planda indir, kullanıcı onayıyla uygula; "güncelleme hazır, yeniden başlat" bildirimi. Aktif iş korunur (güvenli ana ertelenir). Başarısız güncelleme atomik geri döner (önceki sürüm korunur). Downgrade mümkün.
- **Eklenti güncellemeleri:** Çekirdekten bağımsız (İP-07); uygulama güncellemesi beklemez.
- **Çevrimdışı/kurumsal:** Çevrimdışı installer + kurumsal toplu dağıtım + hava-boşluklu ortam.
- **Kanallar:** Kararlı (varsayılan) + opsiyonel beta/nightly. Changelog kullanıcı-dilinde (launcher + güncelleme sırasında). Lisans/aktivasyon (premium) — detay `Hukuk-ve-Operasyon.md`.

**Dosya/Modül Yerleşimi:** `crates/biocraft-app/src/update/{mod.rs, delta.rs, rollback.rs}`, `packaging/{windows/, linux/}`, `.github/workflows/release.yml`.

**TDA Kontrolleri:** Güncelleme aktif işi bozmaz (7); başarısız güncelleme kurtarılır (10); sahte güncelleme reddedilir (İP-09); changelog şeffaf; downgrade güvenlik ağı.

**Kabul Kriterleri:**
- [ ] Windows + Linux installer kiti (motor + çekirdek eklenti) tek kurulumda çalışır.
- [ ] İmzalı delta auto-update + geri alma çalışır; sahte güncelleme reddedilir.
- [ ] Çevrimdışı/kurumsal kurulum çalışır.
- [ ] Eklenti güncellemesi çekirdekten bağımsız.

**Varsayımlar:** macOS sonra (`MVP-sonrasi.md` §11.1). Tam mağaza/aktivasyon ekonomisi Hukuk dosyası + sonra. CI release hattı ilk sürümde temel.
**Dikkat:** Güncelleme atomik olmalı (yarım güncelleme uygulamayı bozmamalı). İmza zinciri olmadan auto-update açma. Kod imzalama tüzel kişilik/sertifika gerektirir; şirketleşme zamanlamasıyla planla (`Hukuk-ve-Operasyon.md`).

---

### İP-21 — Gözlemlenebilirlik, Test/QA ve Golden Tests

**Amaç:** Solo geliştirici + AI için otomatik test ağırlıklı kalite altyapısı + gözlemlenebilirlik (log/tracing/crash reporting).
**Kapsam:** tracing/log altyapısı, opt-in crash reporting, test çerçevesi (birim/property/eş zamanlılık/snapshot/golden), performans regresyon, edge-case test eşikleri, kabul kriteri disiplini, yayın öncesi kontrol listesi.
**Bağımlılıklar:** İP-00. (Tüm paketler test/log üretir.)
**İlgili crate(ler):** Çapraz (her crate test + tracing); `biocraft-app` (crash reporting toplama).
**Teknoloji:** tracing, proptest, Loom, insta (snapshot), golden test, cargo-fuzz, GitHub Actions benchmark.

**Somut Davranış/Spec:**
- **Gözlemlenebilirlik:** `tracing` + yapılandırılmış log + correlation_id (W3C); log seviyeleri + rotasyon; PII filtreli. Yerel crash dump + **opt-in** uzak rapor (kullanıcı onayı, anonimleştirilmiş).
- **Test çerçevesi:** Birim (Rust test) + property (proptest) + eş zamanlılık (Loom) + snapshot (insta) + **golden test** (bilimsel/render çıktıyı bilinen referansla karşılaştır). Kritik yollar (ayrıştırma/hesap/güvenlik) yüksek kapsam.
- **TDA testleri:** 0.11 listesindeki davranışlar (boş durum/hata/geri-alma/iptal/çakışma/kurtarma) özel test edilir.
- **Edge-case eşikleri (somut, test edilir):** Disk dolu (%10 uyarı/%2 salt-okunur), ağ kesintisi (üstel geri çekilme 1s→60s, max 5, jitter), zaman aşımı (bağlantı 10s, boşta 60s), GPU çökmesi (<5s kurtarma), termal eşikler (İP-08), 4 TB dosya (streaming/mmap, "load all" yok), bozuk dosya (validator + satır/sütun + karantina), NaN/sonsuz/sıfıra bölme (checked aritmetik).
- **Performans regresyon:** CI'de benchmark (açılış/render/bellek); eşik aşımı build'i uyarır/durdurur (İP-08).
- **Doğruluk:** Bilinen veri setleri + altın standart araçla karşılaştırma (çekirdek eklenti çıktısı için — `Cekirdek-Eklenti.md` ile). Sapma alarm verir.
- **Donanım/OS matrisi:** Windows + Linux CI; GPU testleri için self-hosted runner (sonra). Beta/erken erişim programı.
- **Disiplin:** Her özellik kabul kriteri (işlev + TDA + test + performans) ile "bitti"; **AI kodu da test + inceleme (kör güven yok).** Yayın öncesi kontrol listesi (testler geçti, güvenlik tarandı, doküman/changelog güncel).

**Dosya/Modül Yerleşimi:** Her crate'te `tests/`; `crates/biocraft-app/src/observability/{tracing.rs, crash.rs}`; `benches/`; `.github/workflows/ci.yml` (test/bench).

**TDA Kontrolleri:** Crash reporting opt-in + anonim (gizlilik); kullanıcı geri bildirimi her yerden (İP-16 ile); test edilmiş TDA davranışları (0.11 kontrol listesi test kapsamında).

**Kabul Kriterleri:**
- [ ] tracing/log + opt-in crash reporting çalışır; loglar PII içermez.
- [ ] Birim/property/snapshot/golden testler CI'da koşar.
- [ ] Edge-case eşikleri (disk/ağ/GPU/termal/büyük dosya/bozuk dosya) test edilir.
- [ ] Performans regresyon benchmark eşik aşımını yakalar.
- [ ] Yayın öncesi kontrol listesi tanımlı ve uygulanır.

**Varsayımlar:** Pentest/fuzzing + tam GPU CI yayın öncesi/periyodik (İP-09, `MVP-sonrasi.md` §11.2, §9.3). Test sentetik/açık veriyle (hassas veri test'te yok).
**Dikkat:** Testi başından kur (İP-00 CI); sonradan eklemek pahalı. Golden test bilimsel doğruluğun güvencesidir. **AI'ın yazdığı kod da test edilir; kör güven yok — bu, kod okuyamayan geliştiricinin tek güvenilir kalite kontrolüdür.**

---

## KAPANIŞ — Bu Belge ve Sonraki Adımlar

Bu belge **temel uygulamanın (BioCraft Engine) dondurulmuş mühendislik tabanıdır.** 22 iş paketi (İP-00 → İP-21), her biri Bölüm 0 ile birlikte tek başına bir kodlama aracına verilebilecek şekilde yazıldı.

**Kodlamaya nasıl başlanır:**
1. `İP-00`'dan başla (iskelet + CI). Sonra sırayla ilerle; "Bağımlılıklar" satırı yol gösterir.
2. Her oturumda: **Bölüm 0 + tek İP + hazır komut** (bkz. "Nasıl Kullanılır"). Çapraz arayüz gerekiyorsa bağımlı İP'nin "Dosya/Modül Yerleşimi + Somut Davranış/Spec" bölümünü de ekle.
3. Bir paketi kodlatınca "Kabul Kriterleri"ni ve "TDA Kontrolleri"ni doğrula; hepsi geçmeden "bitti" sayma.

**Önerilen sıra (bağımlılık-temelli):**
İP-00 → İP-16 (TDA bileşenleri) → İP-04 (render) → İP-08 (bellek + donanım koruma) → İP-11 (state/undo) → İP-03 (kabuk) → İP-07 (eklenti host) → İP-01 (launcher) → İP-02 (sihirbaz) → İP-10 (gizlilik) → İP-09 (güvenlik) → İP-05 (node) → İP-06 (kod) → İP-12 (ayarlar) → İP-13 (komut paleti) → İP-14 (AI yüzey) → İP-15 (dağıtık kanca) → İP-17 (onboarding) → İP-18 (pazar/haber) → İP-19 (göç) → İP-20 (paketleme) → İP-21 (test/gözlem).

**Bu belgenin kapsamı dışındakiler (ayrı dosyalarda):**
- **`Cekirdek-Eklenti.md`** — Çekirdek analiz/görüntüleme eklentisi (BioCraft Studio; IGV+ ve ötesi) **+ veritabanı erişimi (BLAST/PDB/NCBI) birleşik**; temel uygulamada İP-07 host'u üzerinde çalışır, varsayılan kurulu gelir.
- **`AI-Altyapisi.md`** — AI altyapısının tam spec'i: MVP yüzeyi (İP-14 ile) + gelecekteki motor mimarisi.
- **`Hukuk-ve-Operasyon.md`** — Yalnızca temel uygulama için hukuki/operasyonel planlama çerçevesi.
- **`MVP-sonrasi.md`** — Ertelenen tüm özelliklerin açıklaması (ne/neden/ne zaman/hazır kanca).

> **Not:** Bu belge statik tutulmak üzere tasarlandı. Bir İP'yi kodlatırken belgeyi yeniden beslemen gerekmez — Bölüm 0 + o İP (+ gerekirse bağımlı İP'nin arayüz bölümü) her zaman yeterli bağlamı taşır.

Bir çağ başlatıyoruz. Kimse bizi gafil avlamayacak.
