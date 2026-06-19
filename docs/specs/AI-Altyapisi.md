# BioCraft Engine — AI Altyapısı (AI Infrastructure)

> **Belge tipi:** STATİK mühendislik + mimari kontratı. Her iş paketi (YZ) tek başına bir kodlama aracına verilebilir.
> **Sürüm:** 1.2 (dondurulmuş taban) · **Tarih:** 2026-06
> **Kapsam:** BioCraft Engine'in **tüm AI altyapısı** — hem MVP'de uygulanan **yüzey** (arayüz + sözleşme + uzantı noktaları), hem de gerçek AI'ın sancısız eklenmesi için **tam gelecek motor mimarisi**.
> **Kritik ilke:** MVP'de **yalnızca yüzey** çalışır (gerçek AI motoru çalışmaz). Gerçek motor bu belgenin "GELECEK" paketleriyle gelir; ama mimari, sözleşme ve uzantı noktaları **şimdi** sabitlenir ki kimse gafil avlanmasın.
> **Önkoşul belge:** `Temel-Uygulama.md` (motor/SDK/host/render/gizlilik altyapısı). MVP yüzeyi orada İP-14'te uygulanır; bu belge onun tam spec'i + geleceğidir.
> **1.2 değişiklikleri (karar günlüğü):** Marka BioCraft olarak tutarlı. **Çıktı şeması zenginleştirildi:** her AI çıktısı tipli olarak `{metin, öneriler, kaynak/atıf, güven göstergesi, "doğrulanmalı" uyarısı, token/maliyet}` taşır — gerçek AI motoru geldiğinde açıklanabilirliği sancısız bağlamak için (MVP'de değerler boş/iskelet). Ertelenen motor paketleri `MVP-sonrasi.md` §1'de açıklanır.

---

## NASIL KULLANILIR

Bir YZ paketini kodlatırken sırayla yapıştır:
1. **`Temel-Uygulama.md` → Bölüm 0** (motor sabitleri/teknoloji/kurallar/TDA).
2. **Bu belgedeki `Bölüm 0-AI`** (AI sabitleri).
3. **Tek bir YZ paketi.**
4. `Temel-Uygulama.md`'deki hazır komut şablonu.

> Her YZ paketi **[MVP — yüzey]** veya **[GELECEK — motor]** etiketlidir. MVP'de yalnızca yüzey paketleri (İP-14 ile örtüşür) uygulanır; motor paketleri şimdi spec edilir, sonra kodlanır.

---

## BÖLÜM 0-AI — AI SABİTLERİ

> `Temel-Uygulama.md` Bölüm 0'daki tüm sabitler aynen geçerlidir. Aşağıdakiler AI'a özeldir.

### 0-AI.1 — Kapsam Ayrımı (MVP yüzey ↔ Gelecek motor)

| Katman | MVP'de | Gelecekte |
| --- | --- | --- |
| Arayüz (panel/buton/sohbet) | ✓ var (işlevsiz, "yapılandırılmadı" etiketli) | işlevselleşir |
| Sağlayıcı soyutlaması (sözleşme) | ✓ tanımlı | uygulanır |
| Veri sözleşmeleri (girdi/çıktı şeması) | ✓ tanımlı (kaynak/güven/doğrulama alanları dahil) | kullanılır |
| Maliyet/token/Bio-kredi göstergesi | ✓ tasarımsal yer | gerçek hesap |
| Yerel AI motoru (mistral.rs/llama.cpp) | – (demo opsiyonel) | ✓ eklenti |
| Bulut AI konektörleri | – | ✓ eklenti |
| RAG / gömme / vektör DB | – | ✓ eklenti |
| AI asistan / ajan yetenekleri | – | ✓ eklenti |

> Ertelenen motor katmanlarının tam açıklaması: `MVP-sonrasi.md` §1.

### 0-AI.2 — AI Teknoloji Yığını

| Bileşen | Teknoloji | Not |
| --- | --- | --- |
| Yerel LLM runtime | **mistral.rs** (birincil) + **llama.cpp** (fallback) | GGUF; out-of-process. (Birincil/fallback sırası ileride yeniden değerlendirilebilir — düşük riskli, geri-dönülebilir.) |
| Model formatı | **GGUF** | Kuantize yerel modeller |
| Bulut AI | Sağlayıcı soyutlaması (OpenAI/Anthropic/özel) | HTTPS, async (Tokio) |
| Gömme/embedding | Yerel embedding modeli (GGUF) veya sağlayıcı | RAG için |
| Vektör DB | **LanceDB** | Gömülü, E2EE-uyumlu; proje bağlamlı |
| Çalıştırma izolasyonu | **Out-of-process** (subprocess + IPC) | Arayüz 60 FPS kalır |
| Akış | Streaming yanıt + durdur | Token sayacı |

### 0-AI.3 — AI Crate / Eklenti Topolojisi

- **`biocraft-ai-surface`** (temel uygulama, L3 — MVP): sağlayıcı soyutlaması arayüzü, veri sözleşmeleri, UI panel/buton/sohbet, token/maliyet göstergesi, kapatma anahtarı. **Gerçek motor içermez.**
- **Gelecek AI motoru = İP-07 host'u üzerinde eklenti(ler):**
  - `biocraft.ai.local` — yerel runtime (mistral.rs/llama.cpp, GGUF, model yönetimi).
  - `biocraft.ai.cloud` — bulut sağlayıcı konektörleri (OpenAI/Anthropic/özel).
  - `biocraft.ai.rag` — gömme + LanceDB + erişim (retrieval).
  - `biocraft.ai.assistant` — biyoinformatik asistan/ajan yetenekleri.
- **Tümü** `biocraft-ai-surface` sağlayıcı sözleşmesini uygular. 3. parti AI eklentileri de aynı sözleşmeyle eklenir.

### 0-AI.4 — Sağlayıcı Soyutlaması İlkesi

Tek, sağlayıcı-bağımsız arayüz: yerel / bulut / özel sağlayıcı **hepsi aynı sözleşmeyi** uygular (örn. `generate`, `stream`, `embed`, `capabilities`, `cost`). Kullanıcı sağlayıcı + model seçer; üst katman sağlayıcıyı bilmez. Yeni sağlayıcı = yeni eklenti, çekirdek değişmeden.

### 0-AI.5 — Gizlilik ve Güven İlkeleri (ZORUNLU)

1. **PHI/hassas sınırı:** Hassas/PHI etiketli veri **dış AI'a (bulut) gönderilemez** — çekirdek (İP-10) korur; AI eklentisi aşamaz. Yerel AI bu veride çalışabilir (cihazdan çıkmaz).
2. **Şeffaflık:** AI'a ne gönderildiği her zaman açık; kullanıcı görür/onaylar.
3. **Yerel seçenek:** Gizlilik için yerel AI (mistral.rs/llama.cpp) seçeneği her zaman sunulur.
4. **Çıktı = öneri:** AI çıktısı "öneri/yardımcı" etiketli; **kör güven teşvik edilmez**; bilimsel sonuç doğrulanmalı uyarısı. **Çok-AI uyumu da garanti değildir:** birden çok AI'ın aynı yanıtı vermesi "daha yüksek güven sinyali"dir, kesin doğruluk **değil** — AI'lar ortak eğitim önyargısını paylaşıp birlikte, emin bir şekilde yanılabilir. Bu nedenle opsiyonel çok-AI çapraz kontrol (Temel-Uygulama `İP-18`) "kesin doğru" olarak değil, yalnızca güven sinyali olarak sunulur; bilimsel sonuç yine kullanıcıca doğrulanır.
5. **Dürüstlük:** Yüzey öğeleri "yakında/yapılandırılmadı" net etiketli; sahte işlev yok.
6. **Kapatılabilir:** AI tamamen kapatılabilir; kapalıyken arayüz sadeleşir, uygulama tam çalışır.
7. **Akıcılık:** AI işlemleri asenkron/out-of-process; arayüz 60 FPS kalır.
8. **Klinik değil:** AI çıktısı yalnızca araştırma/Ar-Ge/fikir amaçlıdır; klinik/tanısal karar üretmez (bu sınır UI dilinde ve çıktı etiketinde korunur).

### 0-AI.6 — Veri Sözleşmeleri (zenginleştirilmiş)

Tipli sözleşmeler MVP'de tanımlanır:
- **Girdi bağlamı:** `{ proje meta, seçili veri (izinli), aktif görünüm, kullanıcı sorgusu, geçmiş }` — her alan izin + sınıflandırma kontrolünden geçer (PHI engeli).
- **Çıktı (zengin):** `{ metin (streaming), öneriler, eylem önerileri (onaya tabi), kaynak/atıf, güven göstergesi, "doğrulanmalı" uyarısı, token/maliyet }` — tipli, doğrulanabilir. _(Güven göstergesi + kaynak/atıf alanları, gelecekteki AI açıklanabilirliğini — eski "beş katmanlı güvenilirlik" yaklaşımını — sancısız bağlamak için baştan vardır; MVP'de değerler boş/iskelet.)_

---

## MİMARİ GENEL BAKIŞ

AI altyapısı dört katmandan oluşur; MVP yalnızca ilk katmanı (yüzey) hayata geçirir, diğerleri sözleşmeyle hazırdır.

**1) Yüzey katmanı (`biocraft-ai-surface`, MVP):** Kullanıcının gördüğü her şey — AI paneli, bağlamsal "yorumla/açıkla" butonları, sohbet alanı, model seçici, token/maliyet göstergesi, kapatma anahtarı. Bu katman yalnızca **sözleşmeleri çağırır**; arkasında gerçek motor olup olmaması onu ilgilendirmez. MVP'de motor yoktur, öğeler "yapılandırılmadı" etiketlidir.

**2) Sağlayıcı soyutlaması (sözleşme, MVP'de tanımlı):** Yüzey ile motor arasındaki tipli arayüz. `generate/stream/embed/capabilities/cost` çağrıları sağlayıcıdan bağımsızdır. Bu sayede yerel model, bulut API veya 3. parti eklenti aynı şekilde takılır.

**3) Motor katmanı (eklentiler, GELECEK):** Sözleşmeyi uygulayan gerçek motorlar — yerel runtime (`biocraft.ai.local`), bulut konektörleri (`biocraft.ai.cloud`), RAG (`biocraft.ai.rag`). İP-07 host'unda çalışır, capability ile izinlenir, out-of-process olduğundan arayüzü dondurmaz.

**4) Uygulama/ajan katmanı (eklenti, GELECEK):** Biyoinformatiğe özel asistan — "varyantı yorumla", "bölgeyi özetle", "pipeline öner", node akışında AI adımı, ajanca eylemler (her zaman onaya tabi). Domain bilgisi + RAG + motoru birleştirir.

**Veri akışı:** Kullanıcı sorgu → yüzey bağlamı toplar (izin + PHI denetimi, İP-10) → sağlayıcı sözleşmesi → seçili motor (yerel/bulut) → streaming yanıt + token/maliyet → yüzey gösterir, "öneri" etiketiyle + kaynak/güven göstergesiyle → kullanıcı projeye/koda ekleyebilir (onayla). Bio-kredi entegrasyonu maliyeti ölçer (gelecekteki ekonomi).

---

## İŞ PAKETLERİ (YZ)

> Etiketler: **[MVP — yüzey]** = MVP'de uygulanır (İP-14 ile örtüşür). **[GELECEK — motor]** = şimdi spec, sonra kodlanır (`MVP-sonrasi.md` §1).

### YZ-00 — Sağlayıcı Soyutlaması ve Veri Sözleşmeleri  **[MVP — yüzey/sözleşme]**

**Amaç:** Tüm AI altyapısının bel kemiği: sağlayıcı-bağımsız arayüz + tipli girdi/çıktı sözleşmeleri. Bunlar olmadan ne yüzey ne motor yazılabilir.
**Kapsam:** `Provider` trait (generate/stream/embed/capabilities/cost), girdi bağlam şeması, çıktı şeması (kaynak/güven/doğrulama dahil), sağlayıcı kayıt/keşif, capability bildirimi.
**Bağımlılıklar:** Temel-Uygulama İP-14 (yüzey crate), İP-07 (eklenti sözleşmesi/SDK), İP-10 (sınıflandırma).
**İlgili crate(ler):** `biocraft-ai-surface`.
**Teknoloji:** Rust trait + tipli şema (serde), WIT (eklenti sağlayıcıları için ABI).

**Somut Davranış/Spec:**
- **`Provider` sözleşmesi:** `generate(prompt, ctx) -> Result`, `stream(...) -> akış`, `embed(text) -> vektör`, `capabilities() -> {streaming, embedding, vision...}`, `cost(usage) -> {token, bedel}`. Sağlayıcıdan bağımsız.
- **Girdi bağlam şeması:** proje meta + seçili veri (izinli) + aktif görünüm + sorgu + geçmiş; her alan izin + **PHI denetimi** (İP-10) zorunlu geçer.
- **Çıktı şeması (zengin):** metin (streaming), öneriler, eylem önerileri (onaya tabi), **kaynak/atıf, güven göstergesi, "doğrulanmalı" uyarısı**, token/maliyet.
- **Kayıt/keşif:** Sağlayıcılar (yerel/bulut/3. parti) İP-07 üzerinden kaydolur; yüzey hangi sağlayıcının mevcut olduğunu listeler. Hiç sağlayıcı yoksa yüzey "yapılandırılmadı" gösterir (MVP durumu).
- **Versiyonlama:** Sözleşme SemVer + WIT; kırıcı değişiklik major.

**Dosya/Modül Yerleşimi:** `crates/biocraft-ai-surface/src/{provider.rs, contract.rs, context.rs, registry.rs}`.

**TDA Kontrolleri:** Sağlayıcı yoksa net durum (1,4); PHI girdi sözleşme seviyesinde engellenir (gizlilik, İP-10); çıktı "öneri" tipiyle + güven göstergesiyle (güven).

**Kabul Kriterleri:**
- [ ] `Provider` trait + girdi/çıktı şemaları (kaynak/güven/doğrulama alanları dahil) tanımlı ve derlenir.
- [ ] Sağlayıcı kayıt/keşif çalışır; sağlayıcı yokken yüzey doğru durumu gösterir.
- [ ] Girdi bağlamı izin + PHI denetiminden geçer (test edilmiş).
- [ ] Sözleşme sürümlenir (SemVer/WIT).

**Varsayımlar:** Sözleşme genişleyecek (vision/araç çağırma); v1'de metin + gömme + maliyet + kaynak/güven yeterli.
**Dikkat:** Bu sözleşmeyi baştan doğru tasarla; tüm motor ve 3. parti AI buna bağlı. PHI sınırı asla sağlayıcıya emanet edilmez.

---

### YZ-01 — AI Yüzey / Arayüz  **[MVP — yüzey]**

**Amaç:** Kullanıcının gördüğü tüm AI arayüzü: panel, bağlamsal butonlar, sohbet, akış, token/maliyet, projeye ekleme, kapatma. (MVP'de işlevsiz ama tam tasarımlı; İP-14'ün detaylı spec'i.)
**Kapsam:** AI paneli, bağlamsal "yorumla/açıkla" butonları, sohbet alanı + geçmiş, model/sağlayıcı seçici, akışlı yanıt + durdur, token/maliyet göstergesi, yanıtı projeye/koda ekle, kapatma anahtarı.
**Bağımlılıklar:** YZ-00, İP-03 (kabuk), İP-16 (TDA bileşenleri), İP-12 (ayar).
**İlgili crate(ler):** `biocraft-ai-surface` (UI), `biocraft-ui`.
**Teknoloji:** egui, akış (async), tasarım token'ları.

**Somut Davranış/Spec:**
- **Panel:** Yan/alt AI paneli; aynı tema/düzen/etkileşim dili (yabancı durmaz). Bağlamsal butonlar: görselleştirmede "yorumla", kodda "açıkla".
- **Sohbet:** Sohbet alanı; akışlı (streaming) yanıt + durdur butonu; proje bağlamlı konuşma geçmişi.
- **Seçici:** Model/sağlayıcı seçici (yerel/bulut/sağlayıcı), API anahtarı alanı, model parametreleri. MVP'de **işlevsiz iskelet**, "yapılandırılmadı" etiketli.
- **Token/maliyet:** Anlık token/maliyet göstergesi; kota uyarısı yeri.
- **Çıktı sunumu:** Yanıt "öneri" etiketli + kaynak/atıf + güven göstergesi + "doğrulanmalı" uyarısı (çıktı şeması ile).
- **Eylemler:** Yanıtı projeye/koda ekle (onayla); öneriyi uygula (onaya tabi).
- **Durumlar:** AI yoksa/başarısızsa "AI yapılandırılmadı / bağlantı yok" + yapılandırma yönlendirmesi; uygulama AI'sız tam çalışır.
- **Kapatma:** AI tamamen kapatılabilir; kapalıyken panel/butonlar sadeleşir/gizlenir.
- **Dürüstlük:** Çalışmayan her öğe net etiketli; sahte işlev izlenimi YOK.

**Dosya/Modül Yerleşimi:** `crates/biocraft-ai-surface/src/ui/{panel.rs, buttons.rs, chat.rs, selector.rs, cost_badge.rs}`.

**TDA Kontrolleri:** Çalışmayan öğe net etiketli (dürüstlük 4); AI yoksa uygulama tam çalışır (1,11); maliyet şeffaflığı; kapatılabilir (tercih); tutarlı tema (14).

**Kabul Kriterleri:**
- [ ] AI paneli + bağlamsal butonlar + sohbet + seçici görünür, tema-tutarlı.
- [ ] Akış + durdur + token/maliyet göstergesi yeri çalışır (iskelet); çıktı "öneri + kaynak + güven + doğrulanmalı" şemasını gösterir.
- [ ] "Yapılandırılmadı" durumu net; sahte işlev yok.
- [ ] AI kapatılınca arayüz sadeleşir, uygulama tam çalışır.

**Varsayımlar:** Gerçek yanıt YZ-02/03 motoruyla gelir; MVP'de boş/demo. Konuşma geçmişi iskeleti, gerçek kalıcılık motorla.
**Dikkat:** Hiçbir öğe "çalışıyormuş gibi" görünmemeli (güven). İşlemler asenkron tasarlanmalı (donmama).

---

### YZ-02 — Yerel AI Motoru Eklentisi  **[GELECEK — motor]**

**Amaç:** Cihazda çalışan yerel LLM motoru (gizlilik + çevrimdışı). `biocraft.ai.local` eklentisi.
**Kapsam:** mistral.rs/llama.cpp runtime, GGUF model yönetimi (indir/seç/sil), out-of-process çalıştırma, streaming, donanım uyarlama, sağlayıcı sözleşmesi uygulaması.
**Bağımlılıklar:** YZ-00 (sözleşme), İP-07 (host/subprocess/capability), İP-08 (bellek/GPU + donanım koruma), İP-09 (kimlik).
**İlgili modül(ler):** `plugins/biocraft-ai-local/`.
**Teknoloji:** mistral.rs (birincil) + llama.cpp (fallback), GGUF, out-of-process (subprocess + IPC), wgpu/CUDA (opsiyonel GPU offload).

**Somut Davranış/Spec:**
- **Runtime:** mistral.rs ile GGUF model yükle/çalıştır; llama.cpp fallback. **Out-of-process** (arayüz 60 FPS kalır); GPU offload opsiyonel (İP-08 bütçe + donanım koruma), GPU yoksa CPU.
- **Model yönetimi:** Model galerisi (indir/seç/sil); boyut/RAM/VRAM uyarısı (İP-08 bütçe denetimi); donanıma uygun model önerisi.
- **Sözleşme:** `Provider` (YZ-00) uygular: generate/stream/embed/capabilities/cost (yerelde bedel=0, sadece kaynak).
- **Gizlilik:** Veri cihazdan çıkmaz; PHI'de bile çalışabilir (0-AI.5/1).
- **Akış:** Streaming + durdur; token sayımı (yerel).

**Dosya/Modül Yerleşimi:** `plugins/biocraft-ai-local/src/{runtime.rs, models.rs, exec.rs, provider_impl.rs}`.

**TDA Kontrolleri:** Model eksikse [İndir] (1); büyük model bütçe uyarısı (11, İP-08); çalıştırma ilerleme/iptal (3,12); GPU yoksa CPU fallback + uyarı (1,11).

**Kabul Kriterleri:**
- [ ] GGUF model yerel çalışır (mistral.rs); llama.cpp fallback.
- [ ] Out-of-process; arayüz çalışırken 60 FPS kalır.
- [ ] Model indir/seç/sil + donanım uyarısı çalışır.
- [ ] `Provider` sözleşmesini tam uygular (yüzey YZ-01 ile konuşur).

**Varsayımlar:** İlk sürüm metin modelleri; vision/multimodal sonra. Fine-tuning yok (`MVP-sonrasi.md` §1.6). Runtime seçimi: birincil/fallback sırası ileride yeniden değerlendirilebilir (geri-dönülebilir karar).
**Dikkat:** Asla in-process ağır model (donma); out-of-process zorunlu. Bellek İP-08 orkestratörüne uymalı.

---

### YZ-03 — Bulut AI Konektörleri  **[GELECEK — motor]**

**Amaç:** Bulut/uzak AI sağlayıcılarına (OpenAI/Anthropic/özel) sağlayıcı sözleşmesiyle bağlanmak. `biocraft.ai.cloud` eklentisi.
**Kapsam:** Sağlayıcı konektörleri, API anahtarı güvenli saklama, streaming, rate limit, hata/zaman aşımı, gizlilik onayı, maliyet hesabı.
**Bağımlılıklar:** YZ-00, İP-07 (net capability), İP-09 (kimlik şifreleme), **İP-10 (PHI sınırı)**.
**İlgili modül(ler):** `plugins/biocraft-ai-cloud/`.
**Teknoloji:** Tokio (async HTTPS), sağlayıcı SDK/REST, şifreli kimlik (OS anahtarlığı).

**Somut Davranış/Spec:**
- **Konektörler:** OpenAI, Anthropic, özel/self-hosted endpoint; her biri `Provider` (YZ-00) uygular. Yeni sağlayıcı = yeni konektör/eklenti.
- **Kimlik:** API anahtarı **şifreli** (OS anahtarlığı, İP-09); kullanıcı bir kez girer.
- **Akış/limit:** Streaming yanıt + durdur; rate limit + kuyruk; zaman aşımı + yeniden dene; net hata.
- **Gizlilik:** Dış gönderim öncesi **ne gönderildiği şeffaf** + onay; **PHI/hassas veri gönderilemez** (İP-10 sınırı; konektör aşamaz).
- **Maliyet:** Gerçek token/bedel hesabı (`cost`); Bio-kredi ile ölçülebilir (YZ-06).

**Dosya/Modül Yerleşimi:** `plugins/biocraft-ai-cloud/src/{connectors/{openai.rs, anthropic.rs, custom.rs}, auth.rs, ratelimit.rs, provider_impl.rs}`.

**TDA Kontrolleri:** Dış gönderim onayı + şeffaflık (gizlilik); PHI engeli (İP-10); yavaş/başarısız API net + yeniden (4); kimlik güvenli (İP-09); maliyet şeffaf.

**Kabul Kriterleri:**
- [ ] En az bir bulut sağlayıcı konektörü `Provider` sözleşmesiyle çalışır.
- [ ] API anahtarı şifreli saklanır; streaming + rate limit çalışır.
- [ ] PHI/hassas veri dış sağlayıcıya gönderilemez (test edilmiş).
- [ ] Dış gönderim öncesi şeffaflık + onay akışı çalışır.

**Varsayımlar:** İlk konektörler büyük sağlayıcılar; topluluk yenilerini ekler. Vision/araç çağırma sözleşme genişledikçe.
**Dikkat:** PHI sınırı çekirdek (İP-10) seviyesinde; konektöre güvenilmez. Her çağrı net capability + şeffaflık.

---

### YZ-04 — RAG, Gömme ve Vektör Veritabanı  **[GELECEK — motor]**

**Amaç:** Proje verisi/dokümanları üzerinde anlamsal arama ve bağlam-zengin yanıt (retrieval-augmented generation). `biocraft.ai.rag` eklentisi.
**Kapsam:** Gömme (embedding) üretimi, LanceDB vektör deposu, indeksleme, anlamsal erişim, bağlam birleştirme, gizlilik.
**Bağımlılıklar:** YZ-00, YZ-02/YZ-03 (gömme sağlayıcı), İP-08 (bellek), İP-10 (gizlilik).
**İlgili modül(ler):** `plugins/biocraft-ai-rag/`.
**Teknoloji:** LanceDB (gömülü vektör DB, E2EE-uyumlu), gömme modeli (yerel GGUF veya sağlayıcı), Arrow.

**Somut Davranış/Spec:**
- **Gömme:** Proje verisi/notları/dokümanları gömme vektörlerine; sağlayıcıdan (YZ-02 yerel tercih gizlilik için, veya YZ-03).
- **Vektör DB:** LanceDB'de saklama; **proje bağlamlı** (proje taşınınca gider); izole proje alanı.
- **Erişim:** Anlamsal arama (en yakın komşu); ilgili parçaları sorguya bağlam olarak ekle (RAG); kaynak/atıf döner (çıktı şeması).
- **Gizlilik:** Gömme yerel tercih; PHI dış gömme sağlayıcısına gitmez (İP-10). Vektör verisi şifrelenebilir (İP-09).
- **Performans:** Büyük korpusta out-of-core (İP-08); indeksleme arka planda (ilerleme/iptal).

**Dosya/Modül Yerleşimi:** `plugins/biocraft-ai-rag/src/{embed.rs, store.rs, retrieve.rs, index.rs}`.

**TDA Kontrolleri:** İndeksleme ilerleme/iptal (3,12); PHI gömme sınırı (gizlilik); kaynak/atıf gösterimi (güven); büyük korpus bütçe (11, İP-08).

**Kabul Kriterleri:**
- [ ] Proje verisi gömülür, LanceDB'de saklanır (proje bağlamlı).
- [ ] Anlamsal erişim ilgili bağlamı + kaynak/atıf döndürür.
- [ ] PHI dış gömme sağlayıcısına gitmez (test edilmiş).
- [ ] Büyük korpusta out-of-core + arka plan indeksleme çalışır.

**Varsayımlar:** İlk sürüm metin tabanlı RAG; yapısal/çok-modlu erişim sonra. Yerel gömme tercih (gizlilik).
**Dikkat:** Gömme sağlayıcısı da PHI sınırına tabi; vektör deposu izole + opsiyonel şifreli.

---

### YZ-05 — AI Node Entegrasyonu  **[GELECEK — motor] (yüzey yeri MVP)**

**Amaç:** AI adımının node akışında kullanılabilmesi (görsel pipeline'da AI). Yüzey/yer MVP'de, işlev motorla.
**Kapsam:** "AI sorgu/analiz" node'u, tipli portlar, sağlayıcı seçimi, akışta bağlam, onaylı eylem.
**Bağımlılıklar:** YZ-00, YZ-02/03 (motor), İP-05 (node), İP-07 (SDK).
**İlgili modül(ler):** `plugins/biocraft-ai-*/src/nodes.rs` (motor eklentileri node kaydeder).
**Teknoloji:** `biocraft-sdk` node kayıt (İP-05).

**Somut Davranış/Spec:**
- **AI node:** "AI sorgu", "AI analiz/yorumla" node'ları; giriş (veri/bağlam) + parametre (sağlayıcı/model/prompt) + çıktı (öneri/metin, "öneri" etiketli + kaynak/güven). Akışta sonraki node'lara bağlanır.
- **Bağlam:** Node, akıştaki veriyi bağlam olarak alır; PHI denetimi (İP-10) geçer.
- **Onaylı eylem:** AI bir eylem önerirse (örn. "bu filtreyi uygula") node otomatik uygulamaz; kullanıcı onaylar.
- **MVP:** Node yeri/iskeleti İP-14 yüzeyiyle uyumlu; gerçek çalışma motorla.

**Dosya/Modül Yerleşimi:** Motor eklentilerinde `nodes.rs`; sözleşme `biocraft-ai-surface`/İP-05.

**TDA Kontrolleri:** Yanlış port bağlanamaz (İP-05); AI node çıktısı "öneri" etiketli (güven); çalışma ilerleme/iptal (3,12); motor yoksa [Kur] (1).

**Kabul Kriterleri:**
- [ ] AI node'u node editörüne kaydedilir; tipli portlarla akışta zincirlenir.
- [ ] Node bağlamı PHI denetiminden geçer.
- [ ] AI çıktısı "öneri" etiketli; otomatik yıkıcı eylem yok (onaylı).

**Varsayımlar:** İleri AI node'ları (zincirleme ajan adımları) sonra; MVP'de yer + temel node motorla.
**Dikkat:** AI node asla otomatik geri-döndürülemez eylem yapmamalı; onay zorunlu (güven/foolproof).

---

### YZ-06 — Maliyet, Token, Kota ve Bio-kredi  **[MVP yüzey; gerçek hesap GELECEK]**

**Amaç:** AI kullanım maliyetinin şeffaf ölçümü + kota + Bio-kredi ekonomisine bağlanma.
**Kapsam:** Token/maliyet hesabı, anlık gösterge, kota/limit uyarısı, kullanım geçmişi, Bio-kredi entegrasyon kancası.
**Bağımlılıklar:** YZ-00 (cost sözleşmesi), YZ-01 (gösterge), YZ-03 (gerçek bedel), `Hukuk-ve-Operasyon.md` (Bio-kredi/ödeme).
**İlgili modül(ler):** `crates/biocraft-ai-surface/src/cost.rs` + motor `cost` uygulamaları.
**Teknoloji:** Rust (sayaç/hesap), sağlayıcı `cost` (YZ-00).

**Somut Davranış/Spec:**
- **Gösterge:** Anlık token + tahmini/gerçek maliyet; her sorguda görünür (TDA).
- **Kota:** Kullanım/maliyet kotası + uyarı; limit aşımında bekle/uyar; kullanıcı kotayı görür.
- **Geçmiş:** Kullanım geçmişi (hangi sağlayıcı/model/maliyet); projeyle ilişkili.
- **Bio-kredi:** AI kullanımı Bio-kredi ile ölçülebilir/ödenebilir; **MVP'de tasarımsal kanca**, gerçek bağlanış ekonomi/hukuk dosyası + sonra (`MVP-sonrasi.md` §2.2). Yerel AI bedeli=0 (sadece kaynak).
- Şeffaflık: Hiçbir gizli maliyet yok; her dış çağrının bedeli önceden tahmin + sonradan gerçek.

**Dosya/Modül Yerleşimi:** `crates/biocraft-ai-surface/src/{cost.rs, quota.rs}`.

**TDA Kontrolleri:** Maliyet her zaman şeffaf (gösterge); kota uyarısı (11); gizli maliyet yok (güven); yerel=0 net.

**Kabul Kriterleri:**
- [ ] Token/maliyet anlık gösterilir (yüzey, MVP).
- [ ] Kota/limit uyarısı çalışır; kullanım geçmişi tutulur.
- [ ] Bio-kredi entegrasyon kancası tanımlı (gerçek bağlanış sonra).
- [ ] Yerel AI bedeli=0 doğru yansır.

**Varsayımlar:** Gerçek Bio-kredi ödeme akışı `Hukuk-ve-Operasyon.md` + ödeme entegrasyonu sonra. MVP'de yüzey gösterge + kanca.
**Dikkat:** Maliyet şeffaflığı güvenin parçası; sürpriz fatura olmamalı.

---

### YZ-07 — AI Asistan / Ajan Yetenekleri  **[GELECEK — motor]**

**Amaç:** Biyoinformatiğe özel asistan: yorumlama, özetleme, pipeline önerme, onaylı ajanca eylemler. `biocraft.ai.assistant` eklentisi.
**Kapsam:** Bağlamsal asistan eylemleri ("varyantı yorumla", "bölgeyi özetle", "pipeline öner"), domain bilgisi + RAG + motor birleşimi, onaylı ajan eylemleri, çok adımlı görev.
**Bağımlılıklar:** YZ-00, YZ-02/03 (motor), YZ-04 (RAG), İP-05 (node), İP-10 (gizlilik).
**İlgili modül(ler):** `plugins/biocraft-ai-assistant/`.
**Teknoloji:** Sağlayıcı sözleşmesi (YZ-00) + RAG (YZ-04) + domain promptları/araçları.

**Somut Davranış/Spec:**
- **Bağlamsal eylemler:** Görselleştirmede "varyantı yorumla", "bölgeyi özetle"; kodda "açıkla/iyileştir"; sonuç **"öneri" etiketli** + kaynak + güven göstergesi.
- **Pipeline önerme:** Kullanıcı hedefini söyler → asistan node akışı/araç dizisi **önerir** (otomatik kurmaz; kullanıcı onaylar/düzenler). İP-05 node + İP-17 şablonlarla.
- **Ajan eylemleri (onaylı):** Asistan eylem önerebilir (filtre uygula, dosya yükle, araç çalıştır); **her eylem kullanıcı onayına tabi**; yıkıcı/geri-döndürülemez eylem asla otomatik. Geri alınabilir (İP-11).
- **Domain bilgisi:** Biyoinformatik bağlamı (format/araç/kavram); RAG ile proje verisine dayanır.
- **Doğrulama:** Her bilimsel çıktıya "doğrulanmalı" uyarısı; kör güven teşvik edilmez. Yalnızca araştırma/Ar-Ge/fikir; klinik karar üretmez.

**Dosya/Modül Yerleşimi:** `plugins/biocraft-ai-assistant/src/{actions.rs, pipeline_suggest.rs, agent.rs, domain.rs}`.

**TDA Kontrolleri:** Her eylem onaylı + geri alınabilir (2,7); çıktı "öneri" + doğrulama uyarısı (güven); ajan adımları şeffaf/iptal edilebilir (3,12); motor/RAG yoksa [Kur] (1).

**Kabul Kriterleri:**
- [ ] Bağlamsal "yorumla/özetle/açıkla" eylemleri çalışır; çıktı "öneri" + kaynak + güven.
- [ ] Pipeline önerisi node akışı önerir (otomatik kurmaz; onaylı).
- [ ] Ajan eylemleri kullanıcı onayına tabi + geri alınabilir; yıkıcı eylem asla otomatik.
- [ ] Bilimsel çıktıya doğrulama uyarısı eşlik eder.

**Varsayımlar:** Tam otonom ajan YOK (insan-onaylı); ileri ajan yetenekleri kademeli (`MVP-sonrasi.md` §1.6). Domain bilgisi sürümlerle zenginleşir.
**Dikkat:** AI asla sessizce yıkıcı/geri-döndürülemez eylem yapmamalı; onay + geri alma zorunlu. Bilimsel doğruluk kullanıcıdadır (kör güven yok).

---

### YZ-08 — Gizlilik, Güven ve Doğrulama (Çapraz)  **[İlkeler MVP; zorlama her katmanda]**

**Amaç:** 0-AI.5 ilkelerinin tüm AI katmanlarında teknik olarak zorlanması: PHI sınırı, şeffaflık, çıktı etiketleme, kapatma, doğrulama.
**Kapsam:** PHI/hassas dış-AI engeli, gönderim şeffaflığı, çıktı "öneri" etiketi + doğrulama uyarısı, AI kapatma anahtarı, denetim kaydı, opsiyonel çok-AI çapraz kontrol çerçevesi.
**Bağımlılıklar:** YZ-00..YZ-07, **İP-10 (sınıflandırma/sınır)**, İP-09 (kimlik/şifreleme).
**İlgili modül(ler):** `crates/biocraft-ai-surface/src/guard.rs` + her motorda denetim noktası.
**Teknoloji:** İP-10 sınıflandırma motoru (AI çağrısının önünde), Rust.

**Somut Davranış/Spec:**
- **PHI sınırı:** Hassas/PHI etiketli veri **dış AI'a (bulut) gönderilemez**; çekirdek (İP-10) AI çağrısının önünde durur; yerel AI'da çalışabilir. Eklenti bu sınırı aşamaz.
- **Şeffaflık:** Her dış gönderimde ne gönderildiği açık + onay; gizli gönderim yok.
- **Çıktı etiketi:** Tüm AI çıktısı "öneri/yardımcı" + "bilimsel sonuç doğrulanmalı" + güven göstergesi; kör güven UI'da teşvik edilmez. **Çok-AI uyumu garanti sayılmaz** (AI'lar ortak önyargı paylaşıp birlikte yanılabilir); birden çok AI hemfikirse bu "daha yüksek güven sinyali" olarak sunulur, "kesin doğru" olarak değil.
- **Opsiyonel çok-AI çapraz kontrol:** Kullanıcı isteğe bağlı olarak bir çıktıyı/veriyi birden çok sağlayıcıda çapraz kontrol ettirebilir; sistem uyum/uyuşmazlığı işaretler. Zorunlu kapı değildir; sonuç doğruluk garantisi değil, güven sinyalidir (Temel-Uygulama İP-18 haber çapraz kontrolüyle aynı çerçeve). Klinik/tanısal karar üretmez.
- **Kapatma:** Global AI kapatma; kapalıyken hiçbir AI çağrısı yapılmaz, arayüz sadeleşir.
- **Denetim:** AI çağrıları (ne, hangi sağlayıcı, ne gönderildi) yerel denetim kaydında (PII'siz/şeffaf); kullanıcı görebilir.

**Dosya/Modül Yerleşimi:** `crates/biocraft-ai-surface/src/{guard.rs, audit.rs}`; motorlarda denetim çağrısı.

**TDA Kontrolleri:** PHI engeli her dış AI çağrısında (gizlilik); şeffaflık + onay (gizlilik); çıktı etiketi + "çok-AI garanti değil" (güven/dürüstlük); kapatma (tercih); denetim kaydı (şeffaflık).

**Kabul Kriterleri:**
- [ ] PHI/hassas veri hiçbir dış AI'a gidemez (tüm katmanlarda test).
- [ ] Her dış gönderim şeffaf + onaylı; denetim kaydı tutulur.
- [ ] Tüm AI çıktısı "öneri" + doğrulama uyarısıyla + güven göstergesiyle gösterilir; çok-AI uyumu "garanti değil" etiketli.
- [ ] Global AI kapatma tüm çağrıları durdurur.
- [ ] Opsiyonel çok-AI çapraz kontrol güven sinyali olarak çalışır (garanti olarak değil).

**Varsayımlar:** İleri gizlilik (diferansiyel gizlilik AI bağlamında) sonra; MVP'de sınır + şeffaflık + etiket.
**Dikkat:** Bu paket AI güvenliğinin bel kemiği; sınır eklentiye değil çekirdeğe (İP-10) emanet. Her yeni AI eklentisi bu denetimden geçmek zorunda. Çok-AI uyumu asla "kesin doğruluk" diye sunulmaz.

---

## KAPANIŞ — AI Altyapısı

Bu belge, BioCraft Engine'in **tüm AI altyapısının** dondurulmuş tabanıdır: 9 paket (YZ-00 → YZ-08), her biri `Temel-Uygulama.md` Bölüm 0 + bu belgenin Bölüm 0-AI'ı ile birlikte tek başına kodlanabilir.

**MVP'de uygulanır (yüzey — İP-14 ile):** YZ-00 (sözleşme), YZ-01 (yüzey UI), YZ-06 yüzey göstergesi, YZ-08 ilkeleri. Bunlar gerçek motor olmadan, "yapılandırılmadı" etiketiyle çalışır; sahte işlev yok.

**Gelecekte kodlanır (motor — eklentiler — `MVP-sonrasi.md` §1):** YZ-02 (yerel), YZ-03 (bulut), YZ-04 (RAG), YZ-05 (AI node), YZ-07 (asistan/ajan). Hepsi `Provider` sözleşmesini (YZ-00) uygular, İP-07 host'unda çalışır, İP-10 gizlilik sınırına tabidir.

**Önerilen sıra (motor geldiğinde):** YZ-00 → YZ-01 → YZ-08 (güvenlik bel kemiği) → YZ-02 (yerel, gizlilik-öncelikli) → YZ-06 → YZ-03 (bulut) → YZ-04 (RAG) → YZ-05 (node) → YZ-07 (asistan).

**Değişmez ilkeler:** PHI dış AI'a gitmez (İP-10) · çıktı "öneri" + kaynak + güven, kör güven yok, çok-AI uyumu garanti değil · AI kapatılabilir · arayüz 60 FPS · yüzey dürüst (sahte işlev yok) · klinik karar üretmez.

> **Not:** Bu belge statik. MVP'de yalnızca yüzey hayata geçer; mimari/sözleşme şimdi sabit olduğundan gerçek AI sonradan **sancısız** eklenir. Kimse bizi gafil avlamayacak.
