# Gömülü çekirdek eklenti — BioCraft Studio (MK-19)

Bu dizin, motorla **aynı pakete gömülen** çekirdek eklentinin (BioCraft Studio: analiz/görüntüleme
+ veritabanı) yerleşimini temsil eder. Paketleme betikleri (`../windows/build-msix.ps1`,
`../linux/build-appimage.sh`, Flatpak manifesti) bu dizini kurulum kökünün altına kopyalar.

## Neden burada?

- **MK-19:** Çekirdek eklenti motorla aynı kurulumda gelir → ilk açılışta kurulu; kullanıcı "eklenti
  yok" ekranıyla **karşılaşmaz**. App bunu `cekirdek_eklenti()` ile bildirir ve pazar/host'ta
  "Kurulu" işaretler.
- **Bağımsız sürüm:** `biocraft.toml`'daki `surum` motor sürümünden ayrı ilerler; ABI uyumu (MK-14)
  `[uyumluluk]` ile korunur.

## Entegrasyon (insan-eli / CI)

Gerçek çekirdek eklenti ikilisi/`.bcext`'i `Cekirdek-Eklenti.md`'deki BioCraft Studio'nun kendi
derlemesinden gelir; CI'da bu dizine kopyalanır (`giris = biocraft-studio`). Manifest + yerleşim
sözleşmesi burada sabittir; böylece paketleme betikleri ikiliyi nereye koyacağını bilir.

> Bu MVP'de gerçek çekirdek eklenti ikilisi henüz yok (ÇE-00…ÇE-12 ayrı iş paketleri); burada
> **manifest + yerleşim sözleşmesi** vardır. Auto-updater ve paketleme bu sözleşmeye bağlıdır.
