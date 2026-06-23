#!/usr/bin/env python3
"""İP-21 (MK-58) — Performans regresyon denetçisi.

Saf-Rust benchmark harness'ının (crates/biocraft-types/benches/cekirdek_bench.rs) ürettiği
sonuçları `benchmarks/baseline.json` ile karşılaştırır.  Her ölçüm bir KALİBRASYON işine
ORANlandığı için karşılaştırma makineden ~bağımsızdır (mutlak ns değil).

Kullanım:
    python scripts/check-bench.py [current.json] [baseline.json]

Çıkış kodu:
    0 → tüm ölçümler tolerans içinde (uyarılar build'i durdurmaz)
    1 → en az bir ölçüm 'hata_kat' eşiğini aştı (CİDDİ regresyon) → build durur

Bağımlılık YOK (yalnız Python 3 stdlib) — CI'da topoloji scripti gibi çalışır.
"""
import json
import sys
from pathlib import Path


def yukle(yol: Path):
    if not yol.exists() or yol.stat().st_size == 0:
        return None
    try:
        with yol.open(encoding="utf-8") as f:
            return json.load(f)
    except (json.JSONDecodeError, UnicodeDecodeError) as e:
        print(f"HATA: {yol} okunamadı/geçersiz JSON: {e}")
        return None


def main() -> int:
    kok = Path(__file__).resolve().parent.parent
    current_yol = Path(sys.argv[1]) if len(sys.argv) > 1 else kok / "benchmarks" / "current.json"
    baseline_yol = Path(sys.argv[2]) if len(sys.argv) > 2 else kok / "benchmarks" / "baseline.json"

    current = yukle(current_yol)
    baseline = yukle(baseline_yol)

    if current is None:
        print(f"HATA: benchmark sonucu yok: {current_yol}")
        print("  (Önce: cargo bench -p biocraft-types --bench cekirdek_bench 2>/dev/null > "
              f"{current_yol})")
        return 1
    if baseline is None:
        print(f"UYARI: temel (baseline) yok: {baseline_yol} — ilk çalıştırma sayılır, atlanıyor.")
        return 0

    tol = baseline.get("tolerans", {})
    uyari_kat = float(tol.get("uyari_kat", 2.0))
    hata_kat = float(tol.get("hata_kat", 4.0))

    temel_olc = baseline.get("olcumler", {})
    simdi_olc = current.get("olcumler", {})

    print(f"Performans regresyon denetimi (uyarı x{uyari_kat}, hata x{hata_kat})")
    print(f"  kalibrasyon: {current.get('kalibrasyon_ns', '?')} ns/iter")
    print(f"  {'ölçüm':<20}{'temel':>10}{'şimdi':>10}{'kat':>8}  durum")
    print("  " + "-" * 56)

    hata = False
    uyari = False
    for ad, temel_oran in sorted(temel_olc.items()):
        if ad not in simdi_olc:
            print(f"  {ad:<20}{temel_oran:>10.2f}{'YOK':>10}{'-':>8}  UYARI (ölçülmedi)")
            uyari = True
            continue
        simdi_oran = float(simdi_olc[ad]["oran"])
        kat = simdi_oran / temel_oran if temel_oran > 0 else float("inf")
        if kat > hata_kat:
            durum = "HATA (regresyon)"
            hata = True
        elif kat > uyari_kat:
            durum = "uyarı"
            uyari = True
        else:
            durum = "ok"
        print(f"  {ad:<20}{temel_oran:>10.2f}{simdi_oran:>10.2f}{kat:>7.2f}x  {durum}")

    # Temelde olmayan yeni ölçümler — bilgi.
    yeni = [a for a in simdi_olc if a not in temel_olc]
    if yeni:
        print(f"  (yeni ölçümler, temele eklenebilir: {', '.join(sorted(yeni))})")

    print("  " + "-" * 56)
    if hata:
        print("SONUÇ: CİDDİ performans regresyonu saptandı → build durduruldu.")
        print("  Gerçek bir iyileştirme/donanım değişikliğiyse benchmarks/baseline.json güncelleyin.")
        return 1
    if uyari:
        print("SONUÇ: uyarı(lar) var ama eşik aşılmadı → build devam ediyor.")
    else:
        print("SONUÇ: tüm ölçümler tolerans içinde.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
