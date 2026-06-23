# BioCraft Engine — Paketleme & Dağıtım (İP-20, MK-56, MK-19)

> Bu klasör, BioCraft Engine'i **kurulabilir/güncellenebilir** hâle getiren platform paketleme
> betikleri + manifestleridir. Tek komutla (CI'da otomatik) Windows ve Linux kurulum paketleri
> üretir. Mimari kararlar için → `ARCHITECTURE.md` (MK-56 paketleme/güncelleme, MK-19 gömülü
> çekirdek eklenti). Spec → `docs/specs/Temel-Uygulama.md` → İP-20.

## Ne üretilir?

| Platform | Biçim | Betik | Not |
|----------|-------|-------|-----|
| Windows  | **MSIX** (+ Squirrel/Velopack alternatifi) | `windows/build-msix.ps1` | Kod-imzalama gerekli (insan-eli) |
| Linux    | **AppImage** | `linux/build-appimage.sh` | Tek dosya, bağımlılık dahil |
| Linux    | **Flatpak**  | `linux/flatpak/com.biocraftengine.BioCraftEngine.yml` | Sandbox'lı mağaza dağıtımı |

CI (`.github/workflows/release.yml`) bir sürüm etiketi (`v*`) push'unda her iki platformu derler,
paketleri **artifact** olarak yükler ve imzalı güncelleme bildirimini (`bildirim.json` + `imza.hex`)
üretir.

## Temel ilkeler (kabul kriterleri)

1. **Kit tek pakette (MK-19).** Motor + **çekirdek eklenti BioCraft Studio** *aynı* kuruluma gömülür
   (`core-plugin/`). İlk açılışta kurulu gelir → **"eklenti yok" ekranı asla görünmez**. Çekirdek
   eklenti **bağımsız sürümlenir** (motor sürümünden ayrı). App tarafı bunu
   `crates/biocraft-app/src/update/mod.rs::cekirdek_eklenti()` ile bildirir.
2. **İmzalı güncelleme (MK-56).** İkili kod-imzalanır (OS güveni); güncelleme bildirimi Ed25519 ile
   imzalanır + BLAKE3 bütünlük. İmza zinciri olmadan auto-update **kapalıdır**.
3. **Delta + atomik + geri alma.** Yalnız değişen parça indirilir; güncelleme atomik uygulanır
   (yarıda kalırsa eski sürüm çalışır) + geri alınabilir (downgrade güvenlik ağı). Motor:
   `biocraft_data::update` (saf, test-edilebilir L2 çekirdeği).
4. **Çevrimdışı/kurumsal.** İnternetsiz kurulum + `.bcext` çevrimdışı eklenti + (kurumsal) sessiz
   kurulum. Bkz. `windows/README.md` ve `linux/README.md`.

## İNSAN-ELİ İŞLER (kod tarafı hazır, bunlar dışarıdan gelir)

- **Kod-imzalama sertifikası** (Windows Authenticode / tüzel kişilik). Maliyet/süreç →
  `docs/specs/Hukuk-ve-Operasyon.md`. Gelene dek test için **yerel/kendinden-imzalı** sertifika
  kullanılır (aşağıdaki betikler destekler); imzasız sürüm **dağıtılmaz**.
- **Resmi yayın anahtarı (Ed25519).** Güncelleme bildirimini imzalayan **özel** anahtar yalnız CI
  gizlisi (secret) olarak yaşar; **açık** anahtar çekirdeğe gömülür
  (`crates/biocraft-app/src/update/mod.rs::RESMI_YAYIN_ANAHTARI` — şu an yer tutucu).
- **Dağıtım altyapısı** (TLS sunucu / CDN / changelog yayını). MVP'de updater **kaynağı
  yapılandırılmamıştır**; gerçek HTTPS indirme sağlayıcı bağlanınca aktifleşir.

## Çekirdek eklenti gömme (MK-19)

`core-plugin/` dizini çekirdek eklentinin (BioCraft Studio) paket içine kopyalanan dosyalarını
temsil eder. Paketleme betikleri bu dizini kurulum kökünün altına koyar; uygulama ilk açılışta
gömülü eklentiyi **kurulu** olarak işaretler (kimlik `biocraft.studio.core`). Gerçek eklenti
ikilisi/`.bcext`'i derleme zamanında buraya yerleştirmek (çekirdek eklenti kendi deposu/CI'sından)
bir entegrasyon adımıdır; manifest + yerleşim sözleşmesi burada sabittir.
