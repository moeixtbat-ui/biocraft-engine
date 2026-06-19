# BioCraft Engine

> **"The IDE for life sciences."**

BioCraft Engine, biyoinformatik araştırmacılarının onlarca eski araç (IGV, JBrowse, UCSC, Geneious/CLC, BLAST, samtools…) arasında zaman kaybetmesini bitiren; AAA oyun motoru kalitesinde akıcı arayüze sahip, eklenti-merkezli, yerel+bulut yapay zeka entegrasyonuna hazır, donanıma saygılı (Zero-Impact), gizlilik-öncelikli ve tam tekrarlanabilir modern bir masaüstü IDE'sidir.

**Felsefe:** *Bilim insanı soru sormakla ilerler, araç öğrenmekle değil.*

---

## Teknoloji Yığını

| Katman | Seçim |
|--------|-------|
| Dil | Rust (kararlı) |
| Pencere | winit |
| GPU Render | wgpu |
| Arayüz | egui |
| Async | Tokio + Rayon |
| Eklenti (WASM) | Wasmtime + WASI Component Model |

## Proje Yapısı

```
biocraft-engine/
├── ARCHITECTURE.md      # Mimari anayasa (otorite)
├── CLAUDE.md            # Yapay zeka kılavuzu
├── PROGRESS.md          # Oturum-oturum ilerleme günlüğü
├── rust-toolchain.toml  # Rust sürümü sabit
├── docs/specs/          # Detay paket spec'leri
│   ├── Temel-Uygulama.md
│   ├── Cekirdek-Eklenti.md
│   ├── AI-Altyapisi.md
│   ├── Hukuk-ve-Operasyon.md
│   └── MVP-sonrasi.md
└── (Cargo workspace — ilerleyen günlerde)
```

## Lisans

Core: **AGPLv3** · SDK: **Apache-2.0** · Premium/Ticari: ayrı sözleşme

---

*Detaylar için: `ARCHITECTURE.md` (mimari kararlar) ve `docs/specs/` (paket spec'leri).*
