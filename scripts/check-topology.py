#!/usr/bin/env python3
"""
check-topology.py — MK-40: Crate katman topolojisini doğrula.

Kural: Bağımlılık YALNIZCA aşağıdan yukarıya akar (L0 → L5).
Hiçbir crate kendisinden YÜKSEK katmandaki bir workspace crate'ine bağımlı olamaz.
Döngüsel bağımlılık da yasaktır (cargo bunu zaten reddeder; bu script katman yönünü denetler).

Kullanım:
    python3 scripts/check-topology.py
    # veya CI'da:
    python3 scripts/check-topology.py && echo "Topoloji temiz"

Çıkış kodu:
    0 — hiç ihlal yok
    1 — en az bir katman-yönü ihlali bulundu
"""

import json
import subprocess
import sys

# Windows'ta varsayılan encoding cp1252 olur; Türkçe/Unicode karakterler için UTF-8 zorla
sys.stdout.reconfigure(encoding="utf-8")
sys.stderr.reconfigure(encoding="utf-8")

# MK-40: Her crate'in katmanı (L0 en düşük, L5 en yüksek)
LAYERS: dict[str, int] = {
    # L0 — temel tipler
    "biocraft-types": 0,
    # L1 — SDK + IPC
    "biocraft-sdk": 1,
    "biocraft-ipc": 1,
    # L2 — veri / state / bellek
    "biocraft-data": 2,
    "biocraft-state": 2,
    "biocraft-mem": 2,
    # L3 — render / eklenti host / ağ / AI yüzey
    "biocraft-render": 3,
    "biocraft-plugin-host": 3,
    "biocraft-net": 3,
    "biocraft-ai-surface": 3,
    # L4 — UI kabuk + launcher
    "biocraft-ui": 4,
    "biocraft-launcher": 4,
    # L5 — ana binary (çekirdek eklenti de L5 sayılır)
    "biocraft-app": 5,
    "biocraft-core-studio": 5,
}


def get_metadata() -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"[HATA] 'cargo metadata' başarısız:\n{result.stderr}", file=sys.stderr)
        sys.exit(1)
    return json.loads(result.stdout)


def check(metadata: dict) -> list[str]:
    """İhlalleri döndürür; boş liste = temiz."""
    violations: list[str] = []
    workspace_members = set(metadata.get("workspace_members", []))

    for pkg in metadata["packages"]:
        pkg_id = pkg["id"]
        if pkg_id not in workspace_members:
            continue  # dış bağımlılıkları atla

        pkg_name = pkg["name"]
        pkg_layer = LAYERS.get(pkg_name)
        if pkg_layer is None:
            print(
                f"[UYARI] '{pkg_name}' LAYERS haritasında yok — "
                "scripts/check-topology.py içine ekle!",
                file=sys.stderr,
            )
            continue

        for dep in pkg["dependencies"]:
            dep_name = dep["name"]
            dep_layer = LAYERS.get(dep_name)
            if dep_layer is None:
                continue  # harici crate — katman haritasında değil, atla

            if dep_layer > pkg_layer:
                violations.append(
                    f"[İHLAL] {pkg_name} (L{pkg_layer}) → {dep_name} (L{dep_layer}) "
                    f"— üst katmana bağımlılık YASAK (MK-40)!"
                )

    return violations


def main() -> None:
    print("BioCraft Engine — Crate Topoloji Kontrolü (MK-40)")
    print("=" * 55)

    metadata = get_metadata()
    violations = check(metadata)

    # Mevcut katman haritasını göster
    print("\nKatman Haritası:")
    for name, layer in sorted(LAYERS.items(), key=lambda x: (x[1], x[0])):
        print(f"  L{layer}  {name}")

    print()
    if violations:
        print(f"[BAŞARISIZ] {len(violations)} ihlal bulundu:\n")
        for v in violations:
            print(f"  {v}")
        sys.exit(1)
    else:
        print("[BAŞARILI] Topoloji temiz — hiç katman-yönü ihlali yok.")
        sys.exit(0)


if __name__ == "__main__":
    main()
