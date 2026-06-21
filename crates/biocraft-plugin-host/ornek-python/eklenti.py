#!/usr/bin/env python3
"""Örnek BioCraft Python eklentisi (Tier-3 — İP-07, MK-02).

Bu betik, çekirdek tarafından AYRI bir süreç olarak başlatılır.  Kontrol kanalı
satır-tabanlı JSON'dur:

    host  -> stdin :  {"fonksiyon": "<ad>"}
    host  <- stdout:  {"tamam": true, "donen": <i64>, "gunluk": [...], "hata": null}

Eklenti tek bir istek okur, işler, tek bir yanıt yazar ve çıkar.  Çekirdekle aynı
bellek alanını paylaşmaz (in-process değil); çökerse çekirdeği düşürmez.
"""
import sys
import json
import os


def calistir(fonksiyon):
    """İstenen fonksiyonu işler; (donen, gunluk) döndürür."""
    pid = os.getpid()
    if fonksiyon == "merhaba":
        return 16, ["Merhaba BioCraft (Python, pid=%d, ayri surec)" % pid]
    if fonksiyon == "topla":
        # Basit bir hesap örneği — çekirdek dışı, ayrı süreçte.
        return sum(range(1, 11)), ["1..10 toplandi (pid=%d)" % pid]
    # Bilinmeyen fonksiyon: hata olarak işaretlemek için ValueError fırlat.
    raise ValueError("bilinmeyen fonksiyon: %s" % fonksiyon)


def main():
    satir = sys.stdin.readline()
    try:
        istek = json.loads(satir)
        donen, gunluk = calistir(istek.get("fonksiyon", ""))
        yanit = {"tamam": True, "donen": donen, "gunluk": gunluk, "hata": None}
    except Exception as e:  # noqa: BLE001 — köprü her hatayı yapılandırılmış yanıta çevirir
        yanit = {"tamam": False, "donen": 0, "gunluk": [], "hata": str(e)}
    sys.stdout.write(json.dumps(yanit) + "\n")
    sys.stdout.flush()


if __name__ == "__main__":
    main()
