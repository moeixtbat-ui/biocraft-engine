# Linux paketleme (AppImage / Flatpak)

## AppImage (taşınabilir, tek dosya)

```bash
# Test (imzasız):
./build-appimage.sh 0.1.0

# Üretim (GPG imzalı):
./build-appimage.sh 0.1.0 --sign
```

Gereksinimler: `appimagetool` (CI'da kurulur). Çıktı: `dist/BioCraftEngine-<sürüm>-x86_64.AppImage`.
Tek dosyadır, çoğu modern dağıtımda bağımlılıksız çalışır; çekirdek eklenti (MK-19) içinde gömülü
olduğundan **ilk açılışta hazırdır** (ağ gerekmez).

## Flatpak (sandbox'lı mağaza dağıtımı)

```bash
flatpak-builder --force-clean --install --user build-dir \
  packaging/linux/flatpak/com.biocraftengine.BioCraftEngine.yml
```

İzinler bilinçli **dar** tutulur (ağ yalnız imzalı güncelleme + onaylı dış kaynak; PHI sınırı
çekirdekte). Flathub yayını insan-eli adımdır.

## Çevrimdışı / kurumsal kurulum

- **Çevrimdışı:** `.AppImage` tek dosyadır — `chmod +x` + çift tık ile internet olmadan çalışır.
  Flatpak için `flatpak build-bundle` ile tek `.flatpak` dosyası üretilip elden kurulabilir
  (hava-boşluklu ortam dahil).
- **Kurumsal:** AppImage'i ortak ağ paylaşımına/`/opt`'a kopyalayıp `.desktop` ile dağıtın; ya da
  `flatpak install --system` ile sessiz toplu kurulum.
- **`.bcext` çevrimdışı eklenti:** Mağazaya erişmeden eklenti kurmak için uygulamadan
  (Eklenti → Çevrimdışı kur) `.bcext` açılır; imza/bütünlük host (İP-07) tarafından doğrulanır.

## İmzalama

AppImage dağıtım için GPG ile imzalanmalıdır (`--sign`); anahtar insan-eli iştir
(`docs/specs/Hukuk-ve-Operasyon.md`). İmzalı güncelleme bildirimi (`bildirim.json` + `imza.hex`)
ayrıca Ed25519 ile imzalanır ve uygulama içi auto-updater (`biocraft_data::update`) tarafından
doğrulanır — paket imzasından bağımsız ikinci bir güvence (MK-56).
