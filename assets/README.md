# assets/ — BioCraft Engine varlıkları

Bu klasör motorun **kod-dışı** varlıklarını tutar (MK-52 token sistemi + tipografi).

## tokens.json — Tasarım Token'ları (renkler)

Tüm renkler buradan gelir; **kodda sabit renk yoktur** (MK-52, İP-04 / Bölüm 0.8).
`biocraft-render` bu dosyayı derleme zamanında gömer (`include_str!`) → uygulama dosya olmadan da
çalışır; kullanıcı kendi **özel temasını** (E2, Gün 24) çalışma zamanında bunun üstüne ekleyip
kaydedebilir.

- `anahtarlar`: her temanın doldurmak **zorunda** olduğu anlamsal renk anahtarları (eksikse yükleme hata verir).
- `temalar`: Koyu · Açık · Yüksek-kontrast (her biri tüm anahtarları içerir).
- 4 çapa değer (`bg.primary`, `bg.secondary`, `accent.primary`, `text.primary`) Spec 0.8 tablosuyla birebirdir.

## fonts/ — Tipografi (İP-04 / Bölüm 0.8)

Açık/ücretsiz lisanslı üç aile kullanılır:

| Rol | Aile | Boyut | Lisans | Beklenen dosya |
| --- | --- | --- | --- | --- |
| Gövde / arayüz | **Inter** | 14 px | OFL-1.1 | `fonts/Inter-Regular.ttf` |
| Kod | **JetBrains Mono** | 13 px | OFL-1.1 | `fonts/JetBrainsMono-Regular.ttf` |
| Başlık / display | **Space Grotesk** | 20 px | OFL-1.1 | `fonts/SpaceGrotesk-Medium.ttf` |

> **İnsan eli işi (opsiyonel):** Bu `.ttf` dosyaları depoya konmaz (boyut + lisans dağıtım
> kararı hukukçu onayına bağlı — bkz. `PROGRESS.md` İnsan Eli İşler). Dosyalar yoksa motor
> egui'nin gömülü açık-lisanslı fontlarına **sessizce değil, bilgilendirerek** düşer (TDA madde 1);
> boyut/ölçek (DPI farkındalığı) yine token tipografi sisteminden uygulanır. `.ttf`'ler bu klasöre
> konduğunda otomatik yüklenir.
