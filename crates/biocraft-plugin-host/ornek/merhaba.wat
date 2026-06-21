;; Örnek "merhaba" WASM eklentisi (İP-07 — Gün 13).
;; Çekirdek modül (Component Model değil) — elle yazılmış WAT, derleme adımı GEREKTİRMEZ.
;; Host bunu Wasmtime ile çalışma-zamanında derler (wat özelliği) → CI'da tekrar üretilebilir.
;;
;; İki host fonksiyonunu (import) "biocraft" ad alanından kullanır:
;;   gunluk_yaz(ptr,len)        — yetki GEREKTİRMEZ (zararsız log).
;;   dosya_oku(ptr,len)->i32    — fs YETKİSİ gerektirir; yoksa host reddeder (trap).
(module
  (import "biocraft" "gunluk_yaz" (func $gunluk_yaz (param i32 i32)))
  (import "biocraft" "dosya_oku"  (func $dosya_oku  (param i32 i32) (result i32)))

  ;; Doğrusal bellek: 1 sayfa (64 KiB). Host bellek limiti + fuel uygular (MK-18).
  (memory (export "memory") 1)

  ;; Selamlama metni 0. ofsete (16 bayt: "Merhaba BioCraft").
  (data (i32.const 0) "Merhaba BioCraft")
  ;; Okunacak dosya yolu 64. ofsete (14 bayt: "veri/ornek.txt").
  (data (i32.const 64) "veri/ornek.txt")

  ;; merhaba(): selamı host günlüğüne yazar, metnin uzunluğunu (16) döndürür.
  (func (export "merhaba") (result i32)
    (call $gunluk_yaz (i32.const 0) (i32.const 16))
    (i32.const 16))

  ;; dosya_dene(): VFS üzerinden "veri/ornek.txt" okumayı dener.
  ;; fs yetkisi yoksa host bu çağrıyı reddeder (capability denetimi).
  (func (export "dosya_dene") (result i32)
    (call $dosya_oku (i32.const 64) (i32.const 14)))
)
