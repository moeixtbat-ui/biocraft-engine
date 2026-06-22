# Gün 31.2 — BÜYÜK TASARIM & DÜZEN REFORMU (UE5 + VS Code estetiği)

> Bu, sıradaki güne (İP-20/21) geçmeden yapılacak **ara gün**dür. Amaç: 30 gün boyunca işlevsel olarak kurulan her yüzeyi (launcher + ana kabuk + tüm ekranlar) **modern, tutarlı, kolay erişilebilir** bir tasarım diline taşımak — Unreal Engine 5 editörünün koyu/yoğun estetiği + Visual Studio Code'un kabuk düzeni ve okunabilirliği.
>
> **⚠️ EN ÖNEMLİ KISIT — SADECE GÖRÜNÜM:** Bu gün **hiçbir işlevsel mantık, veri akışı, durum makinesi, IPC, dosya formatı, güvenlik sınırı veya MK kararı DEĞİŞMEZ.** Yalnızca **tasarım token'ları, stiller, yerleşim (layout) ve pencere davranışı** değişir. Mevcut tüm davranışsal testler **aynen geçmeye devam etmeli.** "Refactor değil, reskin."

---

## 0. OTURUM BAŞI (Self-Grounding — kod yazmadan ÖNCE)

1. `git log --oneline -30` → son durumu gör.
2. `PROGRESS.md` oku → 30 günde ne kuruldu.
3. `ARCHITECTURE.md` **otorite** (özellikle MK-01, MK-04, MK-40, MK-52, MK-53, MK-58).
4. İlgili spec bölümlerini oku: `docs/specs/Temel-Uygulama.md` → **İP-01 (launcher), İP-03 (kabuk), İP-04 (token tema + render)**; ayrıca dokunulacak yüzeyler için İP-02/05/06/12/13/14/17/18.
5. `cargo build` → temiz mi?
6. **Token sisteminin gerçek konumunu bul:** token tema sistemi `biocraft-render` içinde (İP-04, MK-52 "renk token'dan"); kabuk `biocraft-ui/src/shell/`; launcher `biocraft-launcher` + `biocraft-app`. Önce bu modülleri oku, **mevcut token enum/struct'ı genişlet** — yenisini uydurma.

---

## 1. BUGÜNÜN HEDEFİ (özet)

Onaylanan 4 yön kararı:

| Karar | Seçim |
|---|---|
| **Tema temeli** | UE5 + VS Code koyu **karışım** (koyu öncelikli; açık tema da yeniden tokenlenir) |
| **Launcher penceresi** | **Çerçevesiz + ekran ortasında + sabit küçük boyut** (~1040×640), özel sürükleme şeridi; proje açılınca tam-ekran motora geçiş (Epic akışı) |
| **Ana düzen** | **VS Code kabuğu + UE5 stili**: Activity Bar → Side Bar → Editör/Tuval → Alt panel (Content Drawer) → Status Bar; 3B/genom tuvalinde UE5 viewport araç çubuğu |
| **Vurgu & font** | **Biyo teal/cyan** vurgu; UI = Inter/Segoe UI, kod = JetBrains Mono |

Teslim, aşağıdaki **A–G bölümleri** halinde yapılır. Büyük olduğu için **sıralı alt-commit'lere** bölünür (bkz. Bölüm G).

---

## BÖLÜM A — TASARIM DİLİ & TOKEN SİSTEMİ (temel; önce bu)

> Her renk, boşluk, yarıçap, font, gölge **token**'dan gelir (MK-52). Kodda **sabit `Color32::from_rgb(...)` / sabit piksel / sabit string YASAK.** Var olan token yapısını semantik katmanlarla genişlet.

### A.1 — Renk token'ları (koyu tema; UE5+VSCode karışımı, teal vurgu)

Aşağıdaki **semantik** token isimleri + önerilen hex değerleri (değerler ince ayar yapılabilir, **isimler kalıcı**):

**Zemin / yüzey katmanları (elevation):**
- `zemin.cukur` (viewport/tuval arka, en koyu): `#0A0B0C`
- `zemin.taban` (pencere arka): `#0E0F11`
- `yuzey.1` (panel): `#15171A`
- `yuzey.2` (yan panel / activity bar): `#1B1E22`
- `yuzey.3` (kart / girdi alanı): `#212529`
- `yuzey.4` (hover yüzeyi): `#2A2F35`
- `yuzey.secili` (seçili satır): `rgba(31,184,201,0.16)`

**Kenarlık / ayraç:**
- `kenar.ince` (ayraç): `#262A2F`
- `kenar.varsayilan`: `#34393F`
- `kenar.belirgin` (odaklı girdi): `#454C54`

**Metin:**
- `metin.birincil`: `#E6E9ED`
- `metin.ikincil`: `#A8B0B8`
- `metin.sonuk`: `#6F777E`
- `metin.devredisi`: `#4A5057`

**Vurgu (teal/cyan — marka):**
- `vurgu.taban`: `#1FB8C9`
- `vurgu.hover`: `#35CEDE`
- `vurgu.aktif`: `#129DAD`
- `vurgu.zemin` (sönük dolgu): `rgba(31,184,201,0.14)`
- `vurgu.uzeri_metin` (accent üstü yazı): `#04171A`
- `odak.halka` (focus ring): `#2FD2E3` (2px) + hafif dış parıltı

**Durum renkleri (yalnız renge bağlı kalma — ikon/etiketle de ayırt et):**
- `durum.basari`: `#3FB950`
- `durum.uyari`: `#D9A411`
- `durum.hata`: `#F85149`
- `durum.bilgi`: `#4DA3FF`

**Diğer:**
- `secim.zemin`: `rgba(31,184,201,0.22)`
- `ortu.scrim` (modal arka karartma): `rgba(0,0,0,0.55)`
- `golge.renk`: `rgba(0,0,0,0.45)`

**Node port tipi renkleri (İP-05 — token'a taşı, sabit RGB'yi kaldır):**
- `port.sayi`: `#4DA3FF` · `port.metin`: `#D98A4D` · `port.mantik`: `#3FB950` · `port.dizi`: `#B07CE8` · `port.veri`: `#1FB8C9`

**Kod söz dizimi token'ları (İP-06 — VS Code Dark+ uyumlu, teal'e harmonik):**
- `kod.anahtar`: `#569CD6` · `kod.dize`: `#CE9178` · `kod.yorum`: `#6A9955` · `kod.sayi`: `#B5CEA8` · `kod.fonksiyon`: `#DCDCAA` · `kod.tip`: `#4EC9B0` · `kod.degisken`: `#9CDCFE`

### A.2 — Açık tema (light)
Aynı **semantik** token isimleri, açık karşılıklarıyla yeniden tokenlenir (zemin `#F4F5F7`/`#FFFFFF`, metin `#1B1E22`, vurgu aynı teal ama kontrastı korunur). Dark **varsayılan/öncelikli**; light tam çalışır kalmalı (mevcut `TemaSecimi` + İP-12 dil/tema senkronu bozulmaz).

### A.3 — Tipografi
- UI fontu: **Inter** (yoksa Segoe UI/sistem fallback). Kod fontu: **JetBrains Mono** (fallback Consolas). Fontlar **`include_bytes!` ile gömülü** (ağ yok), egui `FontDefinitions`'a eklenir.
- Type scale (UI yoğun, VS Code 13px tabanı):
  - `display` 28/600 · `h1` 22/600 · `h2` 18/600 · `h3` (panel başlık) 15/600 · `govde` 13/400 · `govde_kalin` 13/600 · `kucuk` 12/400 · `etiket` 11/500 (harf aralığı +0.3, BÜYÜK harf rozetlerde) · `kod` 13/400 (satır yüksekliği 1.5)
- Satır yüksekliği UI: ~1.35; kontrol yüksekliği 26–28px; activity/araç çubuğu ikon hedefleri ≥ 28px.

### A.4 — Boşluk / yarıçap / kenarlık / gölge / hareket
- **Spacing skalası (4px tabanı):** 2, 4, 6, 8, 12, 16, 20, 24, 32, 40.
- **Yarıçap:** `r0`=0 (panel kenarları/dock), `r-sm`=3 (girdi/buton), `r-md`=6 (kart/modal), `r-pill`=9999 (rozet/switch).
- **Kenarlık:** 1px (varsayılan), 2px (odak/aktif vurgu çizgisi).
- **Gölge (elevation):** `e1`(0 1 2, %30), `e2`(0 2 6, %40), `e3`(0 8 24, %50 — modal/drawer). egui'de gerçek blur yok; düz katman + ince kenarlık + gölgeyle yaklaş.
- **Hareket:** `instant`=0, `hizli`=90ms, `taban`=150ms, `yavas`=240ms; easing ease-out. **MK-04/60FPS**: animasyon hafif; kare bütçesini bozma.

### A.5 — Yoğunluk (density) & ikonografi
- İki yoğunluk: **Kompakt** (UE5 yoğun, varsayılan) / **Rahat**. İP-12'deki `panel_yogunlugu` ayarına **görsel olarak** bağlanır (model zaten var).
- Tutarlı ikon dili (egui çizimi / unicode / gömülü SVG-path), çizgi kalınlığı 1.5px, boyutlar 14/16/18/20.

### A.6 — Sabit boyut sözlüğü (layout metrikleri)
`title_bar=32` · `activity_bar=48` (ikon 22) · `side_bar=260` (min 180, max 480) · `tab=35` · `status_bar=22` · `alt_panel_varsayilan=220` · `inspector=300`.

> **A çıktısı:** `biocraft-render` token modülü genişletilir; egui `Style`/`Visuals` bu token'lardan kurulur (tema değiştir → tek yerden). Tüm UI bileşenleri bu token'ları okur.

---

## BÖLÜM B — LAUNCHER REFORMU (çerçevesiz · ortalanmış · Epic-tarzı)

### B.1 — Pencere davranışı (winit; MK-01 — Tauri/Electron YOK)
- `with_decorations(false)` (OS başlık çubuğu yok), `with_resizable(false)`, sabit iç boyut **~1040×640**.
- **Ekran ortasında** aç: aktif monitörün çalışma alanından merkez hesapla → `set_outer_position`.
- İnce 1px `kenar.varsayilan` çerçeve + `e3` gölge; mümkünse köşe yarıçapı `r-md`.
- **Özel başlık şeridi (32px):** solda BioCraft logo + ad; sağda **küçült / kapat** (özel ikon butonları). Şeridin boş alanı **sürüklenebilir** → pointer-down'da `window.drag_window()`.

### B.2 — Launcher yerleşimi (Epic Games launcher mantığı)
- **Sol dikey nav rail (~72px, ikon+etiket):** Ana Sayfa · Kitaplık (Projeler) · Mağaza · Öğren · Ayarlar. Aktif öğe: sol kenarda 2px `vurgu.taban` çizgi + `yuzey.secili` zemin.
- **Üst hero/banner:** öne çıkan kart — son açılan proje için büyük **"Devam Et"** veya yeni kullanıcı için **"Yeni Proje"** CTA (birincil teal buton). Sağda duyuru/sürüm notu kartı (İP-18 haber akışından beslenir, davranış aynı).
- **Proje kartı ızgarası:** kapak/ikon + ad + son açılma tarihi + format sürüm rozeti; sağ-tık menü (Aç / Klasörü Göster / Kaldır). Üstte **arama + sıralama (Son/Ad) + filtre**. Mevcut `recent` listesi verisiyle beslenir.
- **Alt durum şeridi:** uygulama sürümü · güncelleme durumu · (çevrimiçi/oturum yer tutucu).
- **Boş durum:** proje yokken merkezde "İlk projeni oluştur" + **şablon kısayolları** (İP-17 onboarding şablonlarıyla uyumlu) + "🎓 Tur" ve "▶ Demo Projesi" düğmeleri Epic-tarzı yerleştirilir.

### B.3 — Geçiş
- Proje açılınca launcher penceresi kapanır/gizlenir; **motor penceresi maksimize** açılır (mevcut `MotoraGec`/launch akışı korunur — **yalnızca pencere stili + boyut/decorations davranışı** değişir, mantık değil).

---

## BÖLÜM C — ANA KABUK REFORMU (VS Code kabuğu + UE5 stili)

> İP-03'teki dört bölge (Title/Activity/Side/Status) **yeniden düzenlenir ve stillenir**; mevcut sekme/split/detach/inspector davranışı korunur.

### C.1 — Üst şerit (VS Code custom title bar + entegre menü)
- 32px entegre üst şerit: **solda kompakt menü** (Dosya/Düzen/Görünüm/… — mevcut klasik menü, VS Code kompakt stiline uyarlanır), **ortada** proje/başlık, **sağda** pencere kontrolleri. Motor penceresinde de OS başlık çubuğu yerine **özel şerit** önerilir (launcher ile tutarlı); pencere yine maksimize/yeniden boyutlandırılabilir kalır.

### C.2 — Activity Bar (en sol, 48px ikon şeridi)
- Öğeler: Gezgin/Proje · Arama · Node Akışı · Kod · 3B/Sahne · Eklenti & Pazar · AI · Ayarlar (en altta). Aktif: sol kenarda 2px `vurgu.taban` çizgi + ikon `metin.birincil`, pasif `metin.sonuk`. Tooltip'ler i18n'den.

### C.3 — Side Bar (Activity seçimine göre içerik, ~260px)
- Üstte BÜYÜK-harf bölüm başlığı (`etiket` stili) + daraltılabilir bölümler; proje ağacı / arama sonuçları / vb. UE5'in temiz hizalı satır yoğunluğu.

### C.4 — Merkez (Editör / Tuval)
- **Sekmeler (35px):** VS Code stili — aktif sekme `yuzey.1` + üstte 2px accent, pasif `yuzey.2`, değişmiş dosyada ● noktası, kapat ×. Mevcut split/detach korunur, sadece stillenir.
- **3B/genom tuvali:** UE5-tarzı **viewport araç çubuğu** (üst-sağ overlay): kamera/perspektif, görüntü modu, gizmo, ızgara, snap. Arka plan `zemin.cukur` + ince ızgara.

### C.5 — Alt Panel (UE5 Content Drawer mantığı, ~220px)
- Sekmeler: Konsol/Çıktı · Sorunlar · AI · (Node log). Alttan kayarak açılır-kapanır (`yavas` animasyon); status bar'dan tek tık ile açılır. AI sekmesi mevcut İP-14 panelini gösterir (davranış aynı).

### C.6 — Status Bar (en alt, 22px)
- Sol: mod/bağlam bilgisi. Orta: ilerleme/iş durumu. Sağ: **FPS · RAM · °C · token** göstergeleri (İP-12 ayarından aç/kapa; MK-48 token "—"), satır/sütun, dil, kodlama. Tıklanabilir öğeler (drawer aç vb.).

### C.7 — Inspector / Details paneli (sağ, UE5 Details estetiği, ~300px)
- UE5 Details paneli gibi: **daraltılabilir kategori başlıkları**, hizalı **etiket | değer** satırları, sayısal alanlar (sürükle-düzenle hissi), renk alanı, her satırda **↺ sıfırla** ok ikonu, üstte arama. **İP-12 Ayarlar ekranı bu Details estetiğine hizalanır** (model/mantık değişmez).

---

## BÖLÜM D — ATOMİK BİLEŞEN KÜTÜPHANESİ (tutarlı restyle)

Her yeniden-kullanılabilir bileşen **token'lardan** tek tip stillenir (tüm ekranlar aynı dili konuşur):
butonlar (**birincil** teal / **ikincil** yüzey / **hayalet** / **tehlike** kırmızı) · giriş alanı · ComboBox · onay kutusu / radio / **switch** (pill) · slider (UE5 sürükle-değer) · sekme · ağaç öğesi · liste/kart · **rozet/chip** (etiket stili) · toast/banner bildirimi · modal/diyalog (scrim + `e3`) · tooltip · menü · **scrollbar** (ince, hover'da belirir) · ilerleme çubuğu/spinner · ayraç · boş-durum görseli · kısayol kartı · arama kutusu. **İP-13 komut paleti** VS Code Command Palette'e birebir uyarlanır (ortada üstte, sönük zemin, klavye ipuçları).

---

## BÖLÜM E — HER İP YÜZEYİNİN YENİDEN STİLLENMESİ (eşleme; davranış sabit)

| Yüzey | Hedef estetik | Dokunulan yer | Not |
|---|---|---|---|
| **İP-01 Launcher** | Bölüm B | `biocraft-launcher`, `biocraft-app` pencere kurulumu | Pencere davranışı + stil; recent verisi aynı |
| **İP-03 Kabuk** | Bölüm C | `biocraft-ui/src/shell/` | Bölge davranışı korunur |
| **İP-02 Proje Sihirbazı** | Modern wizard: sol adım listesi + sağ içerik + alt İleri/Geri; zorunlu sınıflandırma adımı vurgulu | `biocraft-ui/src/wizard/` | Adımlar/doğrulama aynı |
| **İP-05 Node editörü** | UE5 Blueprint/Control Rig: node kartları, tipli port renkleri (A.1 token), ızgara arka plan, eğri bağlantılar, minimap | node modülleri | DAG/çalıştırma aynı |
| **İP-06 Kod editörü** | VS Code: gutter + satır no + minimap + breadcrumb + sekme; söz dizimi A.1 kod token'ları | `biocraft-ui/src/editor/` | Vurgulayıcı/çalıştırma aynı |
| **İP-04 2B/3B + tema** | Viewport toolbar + gizmo stili; plot temaları token'a | `biocraft-render` | Render çıktısı aynı (golden güncellenir) |
| **İP-12 Ayarlar** | UE5 Details estetiği (C.7) | `biocraft-ui/src/settings/` | Katman/arama/profil aynı |
| **İP-13 Komut paleti** | VS Code Command Palette | `biocraft-ui/src/command/` | Bulanık arama aynı |
| **İP-14 AI paneli** | Alt panel sekmesi; sohbet/güven/uyarı stilleri | `biocraft-ui/src/ai/` | "Yapılandırılmadı"/PHI kapısı aynı |
| **İP-17 Onboarding** | Rol/tur/şablon galerisi modern kartlar; ipuçları toast stili | `biocraft-ui/src/onboarding/` | Akış/i18n aynı |
| **İP-18 Pazar/Mağaza** | Epic-store kart ızgarası + rozet | `biocraft-ui/src/market/` | İçerik/güvenlik aynı |
| **İP-16 Hata diyalogları** | Şema (ne/neden/çözüm/teknik-katlanır) görsel modal | hata gösterimi | Şema/correlation_id aynı |

---

## BÖLÜM F — KISITLAR & İLKELER (ihlal = geri al)

- **MK-01:** winit + wgpu + egui. Tauri/Electron/web view YOK. Çerçevesiz pencere winit ile.
- **MK-52 / MK-53:** Renk/boyut/metin **TOKEN + i18n tek kaynak**. Sabit `Color32::from_rgb`, sabit piksel literali, sabit kullanıcı-metni **yasak** → token/i18n'e taşı.
- **MK-40:** Katman yönü korunur (token/stil `biocraft-render`'da; UI `biocraft-ui` L4). Döngü yok.
- **MK-04:** 60 FPS + kare bütçesi korunur; animasyon/gölge maliyeti hafif.
- **MK-58:** Golden **render** testleri token değişimiyle yenilenebilir (beklenen); **davranışsal** testler değişmemeli.
- **İşlev sabit:** Hiçbir mantık/veri/IPC/format/güvenlik sınırı değişmez. PHI/güvenlik kodu (MK-20/42/43) kapatılmaz.
- **Erişilebilirlik:** Metin/zemin kontrastı **WCAG AA ≥ 4.5:1**; her etkileşimli öğede **odak halkası**; klavye gezinmesi korunur; durum yalnız renge bağlı değil (ikon+etiket).
- **Tutarlılık:** Tek token kaynağı, tek bileşen seti; her ekran aynı dili konuşur.

---

## BÖLÜM G — ADIM ADIM TESLİM (sıralı alt-commit'ler)

Büyük olduğu için tek günde **sıralı alt-adımlar** halinde, her biri kendi commit'iyle (CLAUDE.md "küçük teslimat"):

1. **31.2a — Token temeli:** `biocraft-render` token sistemi (renk/tipografi/spacing/radius/gölge/motion + font gömme). egui `Visuals` token'dan kurulur. Commit: `Gün: 31.2a`.
2. **31.2b — Atomik bileşenler (Bölüm D):** ortak widget stilleri. Commit: `Gün: 31.2b`.
3. **31.2c — Launcher (Bölüm B):** çerçevesiz/ortalanmış pencere + Epic yerleşim. Commit: `Gün: 31.2c`.
4. **31.2d — Ana kabuk (Bölüm C):** Activity/Side/Editör/Drawer/Status + Inspector. Commit: `Gün: 31.2d`.
5. **31.2e — Yüzey eşlemesi (Bölüm E):** wizard/node/kod/ayarlar/palet/AI/onboarding/pazar/hata. Commit: `Gün: 31.2e`.

> Tercihen her alt-adım ayrı oturum/commit; istenirse hepsi tek gün içinde art arda. Her commit sonrası fmt/clippy/test temiz olmalı.

---

## BÖLÜM H — ULTRA-DETAY EKİ (her durum, her anatomi, her mikro-etkileşim)

> Bölüm D atomik bileşenleri **isimlendirir**; bu bölüm onların **her durumunu ve anatomisini** açar. Aşağıdaki her madde, ilgili alt-commit'e (G) beslenir. Hiçbiri davranış değiştirmez — yalnız görünüm/yerleşim.

### H.1 — Menü sistemi (menü çubuğu + açılır menü + bağlam menüsü)
- **Menü çubuğu (üst şeritte, C.1):** üst-seviye başlıklar (Dosya/Düzen/Görünüm/Çalıştır/Eklenti/Yardım) `govde` 13/500, yatay padding 8px, hover'da `yuzey.4` zemin, açıkken `yuzey.3` + alt 2px accent yok (sadece zemin). Tıkla-aç ve hover-geçiş (bir menü açıkken diğerine hover ile geçer).
- **Açılır menü (dropdown) anatomisi:** `yuzey.3` zemin + 1px `kenar.varsayilan` + `e2` gölge + `r-sm` yarıçap; min genişlik 200px; iç padding 4px (öğe arası 1px).
- **Menü öğesi (satır):** sol **ikon alanı (16px)** | **etiket** | sağda **kısayol ipucu** (`metin.sonuk`, monospace değil, "Ctrl+S"); satır yüksekliği 26px; hover'da `yuzey.4` + `metin.birincil`.
- **Öğe varyantları:** `checkable` (sol ikon alanında ✓), `radio` (•), **alt-menü** (sağda ▶ + hover'da yan açılır), **devre-dışı** (`metin.devredisi`, hover yok — ama menüde GÖRÜNÜR = keşfedilebilirlik), **tehlike** (Sil vb. → `durum.hata` metin).
- **Ayraç (separator):** 1px `kenar.ince`, dikey 4px boşlukla gruplar arası.
- **Son dosyalar alt-menüsü:** Dosya → "Son Açılanlar" ▶ (recent listesinden, davranış aynı).
- **Bağlam menüsü (sağ-tık):** aynı dropdown anatomisi; imleç konumunda açılır, ekran kenarına taşarsa ters yöne hizalanır.
- **Tam menü yapısı + kısayollar:** mevcut `KabukAksiyon::tumu()` ve `kisayol()` (İP-13) **tek kaynaktan** beslenir — yeni öğe ekleme, sadece var olanı stille.

### H.2 — Side Bar iç anatomisi (ağaç + bölümler)
- **Bölüm başlığı:** BÜYÜK-HARF `etiket` 11/500 (`metin.sonuk`), sağda **bölüm aksiyon ikonları** (örn. yeni dosya/yenile/daralt-hepsini) yalnız hover'da belirir; tıkla → bölümü daralt/genişlet (chevron sol).
- **Ağaç öğesi (tree item) anatomisi:** **girinti kılavuz çizgileri** (her seviye 12px, ince `kenar.ince` dikey çizgi) | **chevron** (▸/▾, yalnız klasörde, 12px) | **tip ikonu** (klasör/dosya/.bcflow/.py/veri — token renkli) | **etiket** (`govde` 13) | sağda opsiyonel **rozet/sayaç** (chip).
- **Öğe durumları:** hover `yuzey.4`; **seçili** `yuzey.secili` + sol 2px accent çizgi; **odak** (klavye) odak halkası; **inline yeniden adlandırma** (F2 → satır içi metin alanı); **sürükle** (drag ghost yarı saydam + drop hedefi `vurgu.zemin` highlight).
- **Boş durum:** "Henüz dosya yok" + küçük CTA; **overflow:** ince scrollbar (H.4); **resize handle:** Side Bar sağ kenarında 4px sürükleme bölgesi (hover'da `vurgu.taban`), min 180 / max 480px.

### H.3 — Dock & pencere yönetimi (taşıma / ayırma görsel geri bildirimi) — KRİTİK
> İP-03'teki sekme/split/detach **mantığı aynen korunur**; bu bölüm yalnız **sürükleme sırasındaki görsel geri bildirimi** ekler.
- **Sekme sürükleme:** sürüklenen sekmenin **ghost'u** (yarı saydam kopya imlece yapışık); kalan sekmeler kayar (`hizli` animasyon).
- **Drop-zone overlay (5 yön):** sürükleme bir bölge üstündeyken yarı saydam `vurgu.zemin` highlight + **5 hedef** gösterilir — **sol / sağ / üst / alt** (o kenara split) ve **merkez** (sekme olarak ekle). Aktif hedef daha koyu accent + ince accent kenarlık.
- **Split önizleme:** hangi yöne bölüneceğini gösteren yarı saydam dikdörtgen önizleme (bırakmadan önce).
- **Panel ayırma (detach → ayrı pencere):** sekme pencere dışına sürüklenince "ayrı pencere olarak aç" ipucu; ayrılan pencere de yeni tasarım dilini taşır.
- **Yeniden birleştirme (re-dock):** ayrı pencere ana pencereye sürüklenince drop-zone'lar belirir.
- **Panel daralt/genişlet:** her panelin başlığında daraltma oku; daraltılınca yalnız ikon şeridi kalır.
- **Bölücü (splitter):** paneller arası 4px sürükleme çubuğu, hover'da `vurgu.taban`, çift-tık → varsayılan orana sıfırla; min boyut sınırları korunur.

### H.4 — Input / kontrol durum matrisi (her durum)
- **Metin alanı:** durumlar — varsayılan (`yuzey.3`+`kenar.varsayilan`) / **hover** (`kenar.belirgin`) / **odak** (accent kenarlık + odak halkası) / **hata** (`durum.hata` kenarlık + altta hata metni) / **devre-dışı** (`metin.devredisi`, soluk) / **salt-okunur** (zemin biraz daha koyu, imleç yok). **Placeholder** `metin.sonuk`. **Etiket** üstte (`kucuk`) veya solda (Details'te solda). Opsiyonel: **prefix/suffix ikon**, **temizle ×** (içerik varken), **karakter sayacı** (azami uzunlukta).
- **Sayısal alan (UE5 sürükle-değer):** alan üstünde yatay sürükle → değer değişir (imleç ↔); sağda küçük ▲▼ ok düğmeleri; min/max sıkıştırma görseli; ondalık hassasiyet; birim eki (px/°C vb.). İP-12'deki Slider/TamSayi/Ondalik buna uyar.
- **Şifre/hassas alan:** göster/gizle (göz ikonu); İP-12 "hassas" bayraklı alanlar (API anahtarı) maskeli.
- **ComboBox:** kapalı (seçili değer + ▾) / açık (liste `e2` gölge, seçenek hover `yuzey.4`, seçili ✓+accent) / **aranabilir** (üstte filtre kutusu) / **çok-seçim** (seçilenler chip olarak).
- **Checkbox / radio / switch:** her biri için boş/işaretli/karışık(checkbox)/hover/odak/devre-dışı; switch `r-pill` + accent dolu, geçiş animasyonu `hizli`.
- **Slider:** track (`yuzey.4`) + dolu kısım (`vurgu.taban`) + thumb (daire, hover'da büyür) + sürüklerken **değer baloncuğu** + opsiyonel işaret (tick).
- **Doğrulama görünümü:** geçersiz girişte kenarlık `durum.hata` + altta `kucuk` hata metni (i18n) + ikon; İP-16 hata şemasıyla tutarlı.

### H.5 — Şablon galerisi & onboarding (İP-17) kart anatomisi
- **Şablon kartı:** üstte **önizleme** (thumbnail/ikon, `r-md`, 16:9) | **ad** (`h3`) | **açıklama** (`kucuk`, 2 satır kırpma) | **kategori rozeti** (chip) | "**Hangi panelleri kurar**" mini etiketleri | hover'da `e2` yükselme + accent kenarlık | seçili'de accent çerçeve | altta **"Bu şablonu kullan"** birincil buton.
- **Galeri:** üstte kategori filtre sekmeleri + arama; responsive grid (kart min 240px); boş arama sonucu durumu.
- **Rol seçim (K1) kartları:** Öğrenci/Araştırmacı/Geliştirici büyük ikon kartları; seçili'de accent; "Atla" ikincil.
- **Tur baloncuğu:** hedef öğeyi vurgulayan **spotlight** (etrafı `ortu.scrim`) + ok + balon (adım x/7 göstergesi noktalı) + İleri/Geri/Atla; Esc=atla.
- **İpucu (tooltip-tip) toast:** kapatılabilir (×) + "bir daha gösterme"; `yuzey.3`+`e2`.
- **Kavram mikro-öğretici popover:** "FASTA nedir?" gibi — küçük popover, başlık+kısa metin+opsiyonel "Daha fazla".

### H.6 — Proje ayarları & sihirbaz (İP-02/İP-12) ultra-detay
- **Sihirbaz (İP-02):** sol **dikey adım göstergesi** (numaralı; tamamlanan ✓, aktif accent, gelecek sönük) + sağ içerik + alt **Geri / İleri / İptal** (İleri birincil); her adım form düzeni (etiket solda/üstte tutarlı); **zorunlu alan** işareti (*); canlı doğrulama; **veri sınıflandırma adımı** PHI seçilince belirgin **uyarı şeridi** (`durum.uyari` zemin); son **özet/onay** adımı kart listesi.
- **Proje ayarları (İP-12 + manifest):** UE5 Details estetiği (C.7); **katman göstergesi** her satırda küçük rozet (Proje / Kullanıcı / Varsayılan) + nereden geldiği; **↺ varsayılana dön** (yalnız değişmişse aktif); **"yeniden başlat gerekir"** rozeti (turuncu); **hassas** alan maskesi + uyarı; üstte **arama** (anında filtre) + sol **kategori** listesi (ikonlu); kategori boş sonuçta yönlendirme; **fabrika sıfırla** (onay diyaloğu).

### H.7 — Diğer mikro-etkileşimler & durumlar (kataloğu)
- **Boş durumlar kataloğu:** her liste/panel için (proje yok / dosya yok / sonuç yok / eklenti yok / AI yapılandırılmadı) — tutarlı ikon + başlık + açıklama + opsiyonel CTA.
- **Yükleniyor / iskelet (skeleton):** ağ/asenkron yüklemede (İP-18 pazar, haber) gri parıltılı iskelet kartlar; spinner yalnız kısa işlerde.
- **Toast/bildirim yığını:** sağ-altta istiflenir, otomatik kapanır (süre token), tür ikonu (başarı/uyarı/hata/bilgi), kapat ×, "geri al" eylemi (varsa).
- **Breadcrumb:** kod editörü/derin gezinmede yol göstergesi (› ayraçlı, tıklanır).
- **Tooltip:** 500ms gecikme, imleç yakınında, ekran kenarına taşmaz, `kucuk` metin.
- **Scrollbar:** ince (8px), normalde sönük/şeffaf, hover/scroll'da `kenar.belirgin` belirir; köşe yok.
- **Sürüm/güncelleme rozetleri:** "Güncelleme var" (accent nokta), format sürüm rozeti (proje kartında).
- **Tema geçiş:** dark↔light anında (animasyonsuz veya `hizli`), tüm token tek karede yeniden uygulanır.
- **Klavye odak sırası:** mantıklı tab order; her etkileşimli öğe odak halkalı; modal açıkken odak hapsi (focus trap) + Esc kapatır.
- **Sürükle-bırak (genel):** dosya/öğe sürüklemede ghost + geçerli/geçersiz hedef imleci.

---

## BÖLÜM I — NODE EDİTÖRÜ (İP-05) DERİN TASARIM
- **Node kartı:** başlık barı **kategori-renk kodlu** (token) + ikon + ad + collapse oku + bağlam (×); gövdede port satırları; `r-md` köşe + `e2` gölge; **seçili**'de accent çerçeve; çalışırken üstte ince ilerleme şeridi; **hata**'da `durum.hata` kenarlık.
- **Port:** sol=giriş / sağ=çıkış; tip renkleri A.1 (sayi/metin/mantik/dizi/veri) + **şekil** (daire=tekil, kare=dizi); hover'da büyür + etiket tooltip; bağlı/boş durumu; bağlanmamış girişte **node-içi inline değer alanı**.
- **Bağlantı (kablo):** Bezier eğri, tip renginde; veri akarken **hafif akış animasyonu** (MK-04 ucuz); seçili'de kalın+parlak; geçersiz hedefe sürüklerken kırmızı; sürükleme ucunda "yeni node ekle" ipucu.
- **Tuval:** sonsuz ızgara (zoom'a göre yoğunluk), pan (orta-tık/boşluk), zoom (Ctrl+tekerlek), kutu-seçim, hizalama **snap + kılavuz çizgileri**, "tümünü çerçevele".
- **Minimap** (sağ-alt) · **node arama paleti** (boşlukta çift-tık → kategorize liste, İP-13 fuzzy) · **reroute** düğümü · **grup/yorum kutusu** (renkli arka + başlık).

## BÖLÜM J — 3B / SAHNE VIEWPORT (İP-04) DERİN TASARIM
- **Viewport toolbar** (üst overlay, yarı saydam `yuzey.2`): kamera (perspektif/ortho) · görüntü modu (Lit/Unlit/Wireframe) · gizmo modu (taşı/döndür/ölçek) · snap aç-kapa+değer · grid · gizmo uzayı (Dünya/Yerel) · kamera hızı.
- **Gizmo:** eksen renkleri X=`#F85149` / Y=`#3FB950` / Z=`#4DA3FF`; hover'da parlama; aktif eksen vurgulu; döndürmede halkalar, ölçekte küpler.
- **Eksen göstergesi** (sağ-üst mini triad/ViewCube) · zemin ızgarası (uzakta fade) · **seçim outline** (accent) · kamera HUD (FPS/koordinat opsiyonel) · arka plan `zemin.cukur` + opsiyonel gradient.

## BÖLÜM K — GRAFİK / PLOT TEMA (İP-04 2B) DERİN
- **Eksen:** `kenar.varsayilan` çizgi, `metin.ikincil` etiket, grid `kenar.ince`; başlık `h3`; legend kartı (`yuzey.3`+`e1`).
- **Veri serisi:** kategorik **8-renk paleti** (token; teal öncelikli, çakışmasız) + sürekli için **colormap** (viridis-benzeri, renk-körü güvenli).
- **Etkileşim:** hover tooltip (değer) · crosshair · zoom dikdörtgeni (`secim.zemin`) · pan · reset; koyu+açık tema plot karşılığı; PNG/SVG dışa aktarımda tema korunur (davranış aynı, renkler token'dan).

## BÖLÜM L — GENOM TARAYICI & ÇEKİRDEK STUDIO (ÇE) DERİN
- **Track düzeni:** yatay şeritler (üstte koordinat cetveli; dizi/varyant/hizalama/anotasyon track'leri sıralı); track başlığı sol panelde (ad + göster/gizle + yükseklik).
- **Baz renkleri token** (renk-körü için harf de görünür): örn. A=`#3FB950` · T=`#F85149` · G=`#D9A411` · C=`#4DA3FF`.
- **Varyant işaretleri** (SNP/indel renk+şekil) · okuma yığını (hizalama) · zoom/pan · bölge seçimi · sağ-tık (detay/dışa aktar). Hepsi token'dan; davranış aynı.

## BÖLÜM M — VERİ TABLOSU / GRID GÖRÜNÜMÜ
- **Başlık:** ad + sıralama oku (▲▼) + filtre ikonu + sürükle-yeniden-boyutlandır; **dondurulmuş** başlık satırı.
- **Satır:** zebra (`yuzey.1`/`yuzey.2`) / hover `yuzey.4` / seçili `yuzey.secili`; hücre hizası tipe göre (sayı sağa, metin sola); dondurulmuş ilk sütun.
- **Büyük veri:** **sanal kaydırma** (virtualization — MK-09 out-of-core ile uyumlu, yalnız görünür satır çizilir); sayfalama opsiyonel; boş durum + yükleme iskeleti.

## BÖLÜM N — KOMUT PALETİ & KISAYOL YAKALAMA (İP-13) DERİN
- **Palet:** ekran üst-ortasında, scrim arka; arama kutusu (otomatik odak); sonuç satırı: ikon + ad + kategori (sağda sönük) + kısayol ipucu; eşleşen karakterler accent vurgulu; **son/sık kullanılan üstte**; ↑↓/Enter/Esc; "şunu mu demek istediniz".
- **Kısayol ayar ekranı:** komut listesi (kategorize) + atanmış tuş rozeti + "yeniden ata" (**tuşa-bas yakalama modu**, canlı gösterim) + **çakışma uyarısı** (⚠ + çakışan komut) + profil seçici (Modern; Vim/Emacs gelecekte) + tek/tüm "varsayılana dön".

## BÖLÜM O — BİLDİRİM / HATA / DİYALOG SİSTEMİ (İP-16) DERİN
- **Hata modalı:** scrim + `e3`; başlık + tür ikonu (renk-kodlu); **ne oldu / neden / nasıl çözülür** üç blok; **"teknik detay"** katlanır bölüm; **correlation_id kopyala**; eylem butonları (çözüm + kapat).
- **Onay diyaloğu:** yıkıcı eylemde birincil buton `durum.hata` (kırmızı) + "Vazgeç".
- **Toast** (sağ-alt yığın): başarı/uyarı/hata/bilgi ikon+renk; otomatik kapanır; "geri al"; kapat ×.
- **Banner** (panel üstü kalıcı uyarı, örn. salt-okunur proje) · **progress diyaloğu** (yüzde + iptal).

## BÖLÜM P — DURUM ÇUBUĞU & GÖSTERGELER (İP-08/12) DERİN
- **Sol:** aktif mod / proje adı. **Orta:** arka plan iş ilerlemesi (mini bar + metin). **Sağ:** FPS (yeşil/sarı/kırmızı eşik) · RAM (mini bar) · °C · GPU · token ("—" MK-48) · satır:sütun · dil · kodlama. Her segment **tıklanabilir** (ilgili paneli/ayarı açar); İP-12'den aç-kapa; performans modu rozeti (Eco/Bio/Max).

## BÖLÜM Q — AI PANELİ (İP-14) DERİN
- **Sohbet:** kullanıcı balonu (sağ, `yuzey.3`) / asistan (sol, `yuzey.2`); akan "yazıyor…" + durdur (■).
- **Çıktı bloğu:** **güven çubuğu** (seviye renk) + **"doğrulanmalı" uyarı şeridi** (her zaman) + **"klinik değil" şeridi** (MK-49) + **kaynaklar** + **önerilen eylemler** (her biri onay butonlu + geri-alınabilir rozeti) + **token/maliyet rozeti**.
- **Durumlar:** AI kapalı / **yapılandırılmadı** (sahte işlev YOK, "sağlayıcı ekle" CTA) / hazır. Sağlayıcı seçici (yerel/bulut; dış=PHI uyarısı).

## BÖLÜM R — PAZAR / MAĞAZA (İP-18) DERİN
- **Öğe kartı:** kapak/ikon + ad + yayıncı (+doğrulama rozeti) + kategori chip + fiyat (Ücretsiz/Açık Kaynak/Ücretli) + yıldız puanı + indirme sayısı + **kur/güncelle/kaldır** (duruma göre).
- **Detay:** ekran görüntüsü galerisi (etiketli) + açıklama + sürüm + **izin/capability listesi** + yorumlar (salt-okur, rapor) + çok-AI uyum göstergesi ("garanti değil").
- **Liste:** sol kategori + sağ kart ızgarası; üstte arama/sıralama/filtre; haber sekmesi (kaynak+tarih+rozet, düz metin). Doğrulama rozetleri (Resmi/Doğrulanmış/Küratörlü/Beklemede) **dürüst**.

## BÖLÜM S — ONBOARDING & YARDIM (İP-17) DERİN (H.5 genişletmesi)
- **Hoş geldin ekranı** (logo + kısa tanıtım + Başla/Atla) · **rol kartları** (K1) · **7-adım tur** (spotlight) · **şablon galerisi** · **"Demo Projeyi Aç"** kartı · **ipucu balonları** · **kavram popover** · **yardım penceresi** (arama + kavramlar + kısayol kartı + çevrimdışı doküman + onaylı dış bağlantı). Tümü TR/EN (MK-53).

## BÖLÜM T — ERİŞİLEBİLİRLİK & RESPONSIVE & ÇOKLU-MONİTÖR
- **Kontrast** AA (≥4.5:1 metin, ≥3:1 UI); her etkileşimli öğede **odak halkası**; tam klavye gezinme + tab order + **focus trap** (modal) + Esc; durum yalnız renge bağlı değil (ikon/etiket); opsiyonel **yüksek kontrast** token seti.
- **Responsive:** küçük pencerede Side Bar/Inspector daraltılır/gizlenir (ikon şeridine düşer); panel min boyutları.
- **DPI ölçekleme** (winit scale factor → token piksel değerleri ölçeklenir) · **çoklu-monitör** (launcher aktif monitöre ortalanır; pencere taşınınca DPI'a uyum).

## BÖLÜM U — HAREKET / ANİMASYON KATALOĞU
- Süreler (A.4 token): hover `hizli` 90ms · sekme/panel `taban` 150ms · drawer/modal `yavas` 240ms · toast 150ms; easing ease-out.
- **"Azaltılmış hareket"** erişilebilirlik tercihi (İP-12 ayarı): açıkken tüm animasyon `instant`. **MK-04:** hiçbir animasyon kare bütçesini bozmaz; ağır blur/gölge yok.

## BÖLÜM V — İKONOGRAFİ, MARKA & İLLÜSTRASYON
- Tek **ikon seti** (çizgi 1.5px; 14/16/18/20) · durum ikonları (başarı/uyarı/hata/bilgi) · **boş-durum illüstrasyonları** (sade tek-renk + accent) · **BioCraft logosu** (launcher başlığı/splash/pencere ikonu). İkonlar token renk alır (sabit renk yok).

## BÖLÜM W — MİKRO-KOPYA (UX YAZIMI) İLKELERİ
- Butonlar **fiil-odaklı** ("Projeyi Aç", "Kaydet") · hata mesajı **İP-16 şeması** · tooltip kısa · boş-durum cesaretlendirici + CTA · tarih/sayı **yerelleştirme** (TR/EN, MK-53) · tutarlı **terminoloji sözlüğü** (tek terim=tek karşılık) · tüm kullanıcı metni **i18n tek kaynaktan** (sabit string yok, MK-53).

> **Not:** Bölüm H–W tümü, Bölüm G alt-commit'lerine beslenir (token → bileşen → launcher → kabuk → yüzeyler). Hepsi **yalnız görünüm**; hiçbir davranış değişmez.

---

## TEST & KABUL KRİTERLERİ

- [ ] **Mevcut tüm davranışsal testler geçer** (davranış değişmedi); test sayısı düşmez.
- [ ] `cargo fmt --check` · `cargo clippy --workspace --all-targets -D warnings` · `cargo machete` · `python scripts/check-topology.py` · `cargo deny check` → hepsi temiz (exit 0).
- [ ] **Launcher:** çerçevesiz, ekran ortasında, sabit boyut açılır; başlık şeridi sürüklenebilir; küçült/kapat çalışır; proje açınca motor maksimize açılır.
- [ ] **Ana kabuk:** Activity Bar + Side Bar + sekmeli editör + alt drawer + status bar yeni düzende; inspector UE5 Details estetiğinde.
- [ ] **Token tek kaynak:** kod tabanında sabit renk/metin literali kalmadı — doğrula: `grep -rn "Color32::from_rgb\|from_rgba_unmultiplied" crates/*/src` yalnız token tanım dosyasında çıkmalı.
- [ ] **Erişilebilirlik:** kontrast AA; odak halkası; klavye gezinmesi; dark+light ikisi de tutarlı.
- [ ] **Görsel doğrulama:** ilgili `cargo run -p ... --example ...` demoları + canlı `cargo run -p biocraft-app` yeni tasarımı gösterir (ekran görüntüsü al).
- [ ] **60 FPS** korunur (kare bütçesi düşmez).

---

## MUHTEMEL HATALAR VE ÇÖZÜMLERİ

| Olası Hata / Belirti | Çözüm |
|---|---|
| Çerçevesiz pencere sürüklenemiyor | Özel başlık şeridinin pointer-down olayında winit `window.drag_window()` çağır; butonların üstünde tetikleme. |
| Pencere ekran ortasında açılmıyor | Aktif monitörün **work area**'sından merkez hesapla; `set_outer_position` ile yerleştir (taskbar'ı hesaba kat). |
| Kod tabanında sabit renk literali kaldı | `grep` ile tara (`Color32::from_rgb`), her birini token enum'una taşı (MK-52). |
| Golden **render** testleri kırıldı | Beklenen — token değişti; golden referansları **bilinçli** yenile. Davranışsal testler kırılırsa **DUR**: mantığa dokunmuşsundur, geri al. |
| Font gömülemedi / ağ denemesi | Fontu `include_bytes!` ile göm, egui `FontDefinitions`'a ekle; **ağ/indirme yok** (çevrimdışı ilke). |
| FPS düştü / takılma | Gölge/animasyon/blur maliyetini azalt; hover geçişlerini ucuzlat; kare bütçesini koru (MK-04). |
| egui'de gerçek blur/glassmorphism yok | Düz katman + ince kenarlık + `e2/e3` gölge ile yaklaş; cam efekti taklidi için yarı saydam `yuzey` dolgu kullan. |
| Dark güzel, light bozuk | Light tema **ayrı** token değerleriyle; ikisini de gözden geçir; kontrastı her ikisinde doğrula. |
| Tema değişince bazı yerler eski renkte | O bileşen token okumuyor demektir; doğrudan `Visuals`/sabit yerine token'a bağla. |

---

## OTURUM SONU (zorunlu)

1. **Sade özet** (Ne yaptık / Ne işe yarar / Sırada ne var) — proje sahibi yazılım bilmiyor (CLAUDE.md §0).
2. `PROGRESS.md` tablosuna **Gün 31.2** satır(lar)ı + üst "Mevcut Durum" özeti güncellenir.
3. **Commit:** `feat(ui): Gün 31.2 — tasarım & düzen reformu (UE5+VSCode estetiği)` formatında, footer: `Gün: 31.2 | MK: MK-01, MK-04, MK-52, MK-53, MK-58`.
4. Push için **kullanıcıya sor** (onaysız push yok).

> Bu gün bittiğinde uygulama **işlevsel olarak aynı** ama **görsel olarak UE5 + VS Code seviyesinde modern, tutarlı ve kolay erişilebilir** olmalı. Sıradaki gün: **İP-20 (Paketleme/Güncelleme)** veya **İP-21 (Test/Gözlemlenebilirlik)**.
