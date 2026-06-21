# BioCraft Engine — Fuzzing Hedefleri (İP-09)

Bu klasör, **dosya ayrıştırıcılarını** rastgele/kötü niyetli baytlara karşı sınayan
[`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) (libfuzzer) hedeflerini içerir.

> **Önemli:** Bu crate, ana workspace'in **üyesi değildir** (kök `Cargo.toml` → `exclude = ["fuzz"]`).
> Normal `cargo build/test --workspace`, clippy, machete, topoloji ve cargo-deny akışını **etkilemez**.
> Yalnızca **nightly + libfuzzer** ile derlenir/çalışır.

## Hedefler

| Hedef            | Ayrıştırıcı                                   | Neyi sınar                                  |
| ---------------- | --------------------------------------------- | ------------------------------------------- |
| `manifest_toml`  | `biocraft_data::Manifest::toml_coz`           | Proje `biocraft.toml` ayrıştırma            |
| `bcp1_zarf`      | `biocraft_data::project::integrity::zarf_coz` | BLAKE3 bütünlük zarfı çözme                 |
| `bcext_paket`    | `biocraft_plugin_host::BcextPaket::ac`        | `.bcext` eklenti paketi (zip-bomb/taşma)    |
| `sifreli_zarf`   | `SifreliVeri::duz_bayttan`                     | Şifreli veri zarfı ayrıştırma               |

**Beklenti:** Hiçbir girdi **panik/çökme** üretmemeli — sonuç ya `Ok` ya net `Err` olmalı.

## Çalıştırma

```bash
# Tek seferlik kurulum (nightly araç zinciri + cargo-fuzz):
rustup toolchain install nightly
cargo install cargo-fuzz

# Bir hedefi çalıştır (Ctrl-C ile durdur):
cargo +nightly fuzz run manifest_toml
cargo +nightly fuzz run bcext_paket

# Yalnızca derle (CI'da hızlı sağlık kontrolü; çalıştırmadan):
cargo +nightly fuzz build
```

Bir çökme bulunursa `fuzz/artifacts/<hedef>/` altına örnek girdi yazılır; bu girdi
regresyon testine (golden) eklenmelidir.

> Bağımlılık güvenliği (cargo-audit) zaten ana CI hattında her push'ta çalışır (MK-60).
