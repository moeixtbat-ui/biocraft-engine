# BioCraft Engine — MVP Sonrası Yol Haritası (Ertelenen Özellikler)

> **Bu dosya ne?** Temel sürümü (MVP) küçük ve bitirilebilir tutmak için **şimdilik ertelediğimiz** her özelliği burada topladım. Sen yazılım/biyoloji bilmiyorsun, bu yüzden her maddeyi sade dille açıkladım.
>
> **Önemli:** "Ertelendi" demek "unutuldu" demek **değildir.** MVP'de her ertelenen özellik için **bir kanca (hazırlık)** bırakıyoruz; böylece o özellik sonradan **sökülüp yeniden yazmadan, sancısız** eklenebilir. Her maddede "MVP'de şimdiden hazır olan kanca" bölümü bunu anlatır.
>
> **Bu dosya kod üretmek için değildir** — yön haritasıdır. MVP biten sürümün ardından bu listeden ilerleriz.

---

## NASIL OKUNUR

Her ertelenen özellik şu 4 satırla anlatılır:

- **Ne:** Özellik nedir (sade dille).
- **Neden MVP'de yok:** Niye şimdi yapmıyoruz.
- **Ne zaman / nasıl gelecek:** Hangi sürümde veya hangi eklentiyle.
- **MVP'de hazır olan kanca:** Sonradan sancısız eklemek için şimdiden ne koyuyoruz.

---

## ⚙️ MVP'YE TAŞINAN (artık ertelenmiyor)

Senin kararın gereği, daha önce "kısmi/sonra" diye işaretlediğim **iki şey artık tam haliyle MVP'ye alındı:**

1. **Donanım Koruma / Zero-Impact katmanı (tam):** Sıcaklık izleme (GPU/CPU/NVMe), eşik tablosu, kademeli yük azaltma, kritik sıcaklıkta durdurma, statik ekranda yük düşürme — hepsi MVP'de. ("Ekliyorsak hepsini tek seferde" kararın.)
2. **Bilimsel determinizm — bayrak (hook):** Proje ayarında "hızlı keşif / tekrarüretilebilir" seçeneği MVP'de görünür ve çalışır. Sadece **gerçek bit-bit aynı sonuç garantisi** ertelendi (aşağıda Bölüm 9'da açıklandı).

---

# 1. AI Motoru (En Büyük Ertelenen Blok)

> MVP'de AI'ın yalnızca **yüzeyi** (arayüzü) var; gerçek motor yok. Tüm "düşünen" kısım buraya ertelendi. Bunların tamamı ileride **ayrı eklentiler** olarak gelir ve hepsi MVP'de tanımlanan ortak "sağlayıcı sözleşmesini" kullanır.

### 1.1 — Yerel AI Motoru (cihazda çalışan yapay zeka)
- **Ne:** Bilgisayarında, internete çıkmadan çalışan bir yapay zeka modeli (örn. metni özetleme, kod açıklama). Gizlilik için en güvenli seçenek; veri cihazdan çıkmaz.
- **Neden MVP'de yok:** Model çalıştırma (mistral.rs/llama.cpp + GGUF model yönetimi) başlı başına büyük bir iş; MVP'yi geciktirir.
- **Ne zaman / nasıl gelecek:** `biocraft.ai.local` eklentisiyle. Hassas/PHI veride bile çalışabilir (veri cihazdan çıkmadığı için).
- **MVP'de hazır olan kanca:** AI paneli, "yorumla/açıkla" butonları, model seçici, token/maliyet göstergesi — hepsi MVP'de var ama "yapılandırılmadı" etiketli. Sağlayıcı sözleşmesi (motorun bağlanacağı arayüz) MVP'de tanımlı.

### 1.2 — Bulut AI Konektörleri (OpenAI, Anthropic vb.)
- **Ne:** İnternet üzerinden büyük yapay zeka sağlayıcılarına bağlanma.
- **Neden MVP'de yok:** API entegrasyonu + güvenli anahtar saklama + maliyet hesabı motorla birlikte gelir.
- **Ne zaman / nasıl gelecek:** `biocraft.ai.cloud` eklentisiyle.
- **MVP'de hazır olan kanca:** API anahtarı alanı (işlevsiz iskelet), sağlayıcı seçici, **hassas/PHI verinin dış AI'a gidememe sınırı** (bu güvenlik kuralı MVP'de çekirdekte hazır).

### 1.3 — RAG (Proje Verisinde Anlamsal Arama)
- **Ne:** Yapay zekanın senin proje verilerini/notlarını "anlayarak" daha isabetli yanıt vermesi (proje içeriğine dayalı cevap).
- **Neden MVP'de yok:** Gömme (embedding) üretimi + vektör veritabanı (LanceDB) ileri bir altyapı.
- **Ne zaman / nasıl gelecek:** `biocraft.ai.rag` eklentisiyle.
- **MVP'de hazır olan kanca:** Çıktı şeması "kaynak/atıf" alanı içerir (RAG'in döndüreceği kaynaklar için yer hazır).

### 1.4 — AI Node Entegrasyonu (görsel akışta AI adımı)
- **Ne:** Node tabanlı görsel akışına "AI sorgu/analiz" kutusu ekleyebilme.
- **Neden MVP'de yok:** Gerçek çalışma motora bağlı.
- **Ne zaman / nasıl gelecek:** AI motoru eklentileri geldiğinde node'lar kaydolur.
- **MVP'de hazır olan kanca:** Node sisteminin kayıt arayüzü (SDK) MVP'de hazır; AI node yeri tasarımda var.

### 1.5 — AI Asistan / Ajan (yorumlama, pipeline önerme, onaylı eylemler)
- **Ne:** Biyoinformatiğe özel akıllı asistan — "varyantı yorumla", "bölgeyi özetle", "bana bir analiz akışı öner". Her eylem **senin onayına tabi** (otomatik yıkıcı işlem yok).
- **Neden MVP'de yok:** Domain bilgisi + RAG + motor birleşimi gerektirir; en üst katman.
- **Ne zaman / nasıl gelecek:** `biocraft.ai.assistant` eklentisiyle.
- **MVP'de hazır olan kanca:** Bağlamsal butonların yeri + "çıktı = öneri, doğrulanmalı" etiketleme şeması MVP'de hazır.

### 1.6 — AI İleri Yetenekleri
- **Ne:** Görüntü anlama (vision/multimodal), modelleri kendi verinle eğitme (fine-tuning), tam otonom ajan.
- **Neden MVP'de yok:** Hepsi temel motorun çok ötesinde.
- **Ne zaman / nasıl gelecek:** Sağlayıcı sözleşmesi genişledikçe kademeli. (Tam otonom ajan **planlanmıyor** — insan onayı her zaman zorunlu kalacak.)
- **MVP'de hazır olan kanca:** Sağlayıcı sözleşmesi sürümlenebilir (SemVer); yeni yetenekler kırmadan eklenir.

---

# 2. Dağıtık Ağ ve Bio-Kredi Ekonomisi

> Bu, senin talebin gereği **ayrı bir eklenti** olacak ve şimdilik onun için dosya oluşturmuyorum. Burada sadece ileride ne olacağını özetliyorum.

### 2.1 — Dağıtık İşlem Ağı (P2P süperbilgisayar)
- **Ne:** Kullanıcıların boştaki işlem güçlerini (CPU/GPU) bir ağa bağlayıp birlikte ağır hesap yaptığı sistem; karşılığında "Bio-kredi" kazanma.
- **Neden MVP'de yok:** Devasa ve bağımsız bir sistem (P2P ağ, güven/itibar, iş dağıtımı, kötü düğüm izolasyonu); MVP'nin kapsamı dışı.
- **Ne zaman / nasıl gelecek:** Ayrı "dağıtık ağ eklentisi" olarak (kendi dosyası ileride yazılır).
- **MVP'de hazır olan kanca:** Temel uygulama tarafında **pasif kancalar** var (İP-15): ağ keşif noktası, veri paylaşım sözleşmesi, kaynak sınırı arayüzü. Eklenti yokken **sıfır maliyet** (hiç ağ etkinliği yok). En önemlisi: **hassas/PHI veri sınırı çekirdekte korunuyor** — gelecekteki ağ eklentisi bu sınırı asla aşamaz.

### 2.2 — Bio-Kredi Gerçek Ekonomisi (ödeme/transfer)
- **Ne:** Bio-kredi ile gerçek ödeme, premium eklenti satın alma, ağ gücü kiralama.
- **Neden MVP'de yok:** Gerçek para girişi/çıkışı ciddi finansal mevzuat doğurur (hukukçu görüşü şart).
- **Ne zaman / nasıl gelecek:** Hukuki çerçeve netleşince; "kripto para" değil, platform-içi puan olarak.
- **MVP'de hazır olan kanca:** Bio-kredi yalnızca **yer-tutucu/gösterge** olarak var (mağazada "ücretsiz/ücretli" etiketi, AI maliyet göstergesi); gerçek para akışı yok.

---

# 3. Kod Editörü Derinliği

### 3.1 — Akıllı Kod Tamamlama (LSP) — tam sürüm
- **Ne:** Kod yazarken otomatik tamamlama, hata altını çizme, fonksiyon bilgisi (modern editörlerdeki zekâ).
- **Neden MVP'de yok:** Tam dil zekâsı (pyright/jedi tam entegrasyonu, diğer diller) büyük iş.
- **Ne zaman / nasıl gelecek:** v1.x. MVP'de Python için **temel** tamamlama olacak; diğer diller ve tam zekâ sonra.
- **MVP'de hazır olan kanca:** LSP ayrı süreçte (out-of-process) çalışacak şekilde tasarlandı; genişletmek kolay.

### 3.2 — Debugger (adım adım hata ayıklama / breakpoint)
- **Ne:** Kodu satır satır durdurup değişkenleri inceleyerek hata bulma.
- **Neden MVP'de yok:** Tam debugger (breakpoint/DAP protokolü) ayrı bir büyük iş.
- **Ne zaman / nasıl gelecek:** v1.x. MVP'de log-tabanlı (çıktıya yazdırma) ile idare edilir.
- **MVP'de hazır olan kanca:** Kod out-of-process çalıştığı için debugger eklemek mimariye uygun.

### 3.3 — Node ↔ Kod Çift Yönlü Canlı Senkron
- **Ne:** Görsel akış (node) ile kodun **aynı anda** birbirini canlı güncellemesi; ayrıca koddan node'a ters dönüşüm.
- **Neden MVP'de yok:** Çift yönlü canlı senkron karmaşık ve hata riskli.
- **Ne zaman / nasıl gelecek:** Sonra. MVP'de tek yön var: "bu node'u kod olarak aç" + ortak çalışma alanı değişkenleri. Kod→Node ters dönüşüm MVP'de yok.
- **MVP'de hazır olan kanca:** Node ve kod ortak bir çalışma alanı (workspace) paylaşıyor; çift yön buna eklenebilir.

---

# 4. Çekirdek Eklenti (BioCraft Studio) — Temel Düzeyde Kalan / Ertelenen Kısımlar

> Senin A4 kararın gereği: aşağıdaki paketler MVP'de **ya temel düzeyde** ya da **şimdilik tam kullanılmıyor.** Yeniden yazdığım dosyada bunları açıkça işaretleyeceğim.

### 4.1 — Hizalama Görünümü ileri özellikleri (ÇE-03)
- **Ne:** Okuma hizalamalarının çok ileri görselleştirmeleri (split-read, yapısal varyant detayı).
- **Neden MVP'de yok:** Temel hizalama görünümü MVP'de var; ileri detaylar sonra.
- **Ne zaman / nasıl gelecek:** v1.x.
- **MVP'de hazır olan kanca:** Temel read yığını + eşleşmeme + kalite render MVP'de çalışır; üstüne eklenir.

### 4.2 — Anotasyon düzenleme ileri türleri (ÇE-05)
- **Ne:** İleri anotasyon türleri ve özel iz render.
- **Neden MVP'de yok:** Temel anotasyon (GFF/GTF/BED + ekle/düzenle/sil) MVP'de yeterli.
- **Ne zaman / nasıl gelecek:** v1.x.
- **MVP'de hazır olan kanca:** İz yönetimi (ekle/kaldır/sırala/grupla) MVP'de hazır.

### 4.3 — Dizi/MSA ileri araçları (ÇE-06)
- **Ne:** İleri moleküler klonlama, primer tasarımı, tam restriksiyon analizi.
- **Neden MVP'de yok:** Temel dizi düzenleme + çiftli/çoklu hizalama görünümü MVP'de yeterli.
- **Ne zaman / nasıl gelecek:** v1.x. Ağır hizalama (MAFFT/MUSCLE) harici araç köprüsüyle (ÇE-08).
- **MVP'de hazır olan kanca:** Temel düzenleme (kes/yapıştır/ters-tümleyen/çeviri) + temel hizalama görünümü MVP'de.

### 4.4 — Analiz/Hesap motoru ileri kısımları (ÇE-08)
- **Ne:** Uçtan uca tam pipeline'lar (örn. ham veriden varyant çağırmaya kadar tüm zincir).
- **Neden MVP'de yok:** MVP'de **temel native hesap** (kapsama, istatistik, bölge karşılaştırma — Windows dahil konteynersiz çalışır) + **opsiyonel** harici araç köprüsü (samtools/bcftools) var. Tam pipeline sonra.
- **Ne zaman / nasıl gelecek:** İş akışı şablonları (İP-17) + node entegrasyonu (ÇE-10) ile birlikte.
- **MVP'de hazır olan kanca:** Üç katmanlı çalıştırma (native → opsiyonel konteyner → ileride bulut) MVP'de tasarlanmış; native yol her zaman çalışır.

### 4.5 — Node entegrasyonu ileri node'lar (ÇE-10)
- **Ne:** Özel pipeline adımları için ileri node'lar.
- **Neden MVP'de yok:** MVP'de temel node seti (dosya yükle, bölge seç, filtrele, görselleştir, dışa aktar, veritabanı ara) yeterli.
- **Ne zaman / nasıl gelecek:** Sürümlerle artar.
- **MVP'de hazır olan kanca:** Node kayıt mimarisi MVP'de hazır; yeni node eklemek kolay.

### 4.6 — Çekirdek eklenti diğer ertelenenler
- **Ne:** Nadir formatlar/yerel BLAST DB (ÇE-01); ileri 3B yapısal analiz — hizalama/elektron yoğunluğu, molekül dinamiği (ÇE-07); tam otomatik yayın raporu — şablonlu PDF (ÇE-11).
- **Neden MVP'de yok:** Hepsi temel görüntüleme/analiz ötesi.
- **Ne zaman / nasıl gelecek:** v1.x.
- **MVP'de hazır olan kanca:** Temel görüntüleme/dışa aktarma + temel rapor (köken + lisans/atıf kaydı) MVP'de var.

---

# 5. Görselleştirme İleri Altyapısı

### 5.1 — Bevy ECS (yoğun 3B/genom sahnesi motoru)
- **Ne:** Çok sayıda nesne (binlerce atom, devasa genom) yöneten gelişmiş bir oyun-motoru tekniği.
- **Neden MVP'de yok:** MVP'de tüm 2B/3B/genom tuvali doğrudan wgpu ile çiziliyor; bu yeterli. Bevy ECS ekstra karmaşıklık.
- **Ne zaman / nasıl gelecek:** Yoğun bir sahne ileride ECS gerektirirse opsiyonel/gelecek olarak değerlendirilir.
- **MVP'de hazır olan kanca:** Render mimarisi (wgpu) katmanlı; gerekirse ECS ayrı değerlendirilir, çekirdek değişmeden.

### 5.2 — CUDA hızlandırma (opsiyonel GPU compute)
- **Ne:** NVIDIA kartlarda ekstra hız için doğrudan CUDA kullanımı.
- **Neden MVP'de yok:** MVP'de wgpu (her GPU'da çalışan) birincil; CUDA opsiyonel ve varsayılan kapalı.
- **Ne zaman / nasıl gelecek:** `--features cuda` ile isteyene açık; tam optimizasyon sonra.
- **MVP'de hazır olan kanca:** GPU backend seçimi (wgpu/CUDA/CPU) mimaride var; CUDA opsiyonel bayrak.

---

# 6. Veritabanı İleri Özellikleri

### 6.1 — Federe Çoklu Arama (birden çok kaynakta aynı anda)
- **Ne:** Tek aramayla NCBI + UniProt + Ensembl gibi birçok kaynağı birlikte sorgulama.
- **Neden MVP'de yok:** Farklı API'lerin uyumu karmaşık; MVP'de tek kaynak/arama yeterli.
- **Ne zaman / nasıl gelecek:** v1.x.
- **MVP'de hazır olan kanca:** Çerçeve + kaynak-başına konektör mimarisi MVP'de; federe arama bunun üstüne gelir.

### 6.2 — Çevrimdışı Veritabanı Paketleri (yerel kopya)
- **Ne:** İnternet olmadan kullanmak için veritabanlarının yerel kopyalarını indirme.
- **Neden MVP'de yok:** Büyük boyut + senkron yönetimi.
- **Ne zaman / nasıl gelecek:** Sonra.
- **MVP'de hazır olan kanca:** Önbellek altyapısı MVP'de var; yerel paketler buna eklenir.

---

# 7. Bulut, Senkronizasyon ve İşbirliği

### 7.1 — Bulut Senkronizasyonu (cihazlar arası)
- **Ne:** Projelerin/ayarların buluta yedeklenip başka cihazda devam etmesi.
- **Neden MVP'de yok:** Bulut altyapısı + güvenlik + senkron çakışma çözümü ileri iş.
- **Ne zaman / nasıl gelecek:** Sonra (gizlilik gözeterek, varsayılan kapalı).
- **MVP'de hazır olan kanca:** Ayar profili dışa/içe aktarma MVP'de var (yeni cihaza manuel taşıma mümkün); bulut senkron opsiyonel olarak eklenir.

### 7.2 — Canlı İşbirliği (aynı projede birlikte çalışma)
- **Ne:** Birden çok kişinin aynı projeyi eş zamanlı düzenlemesi (Google Docs gibi).
- **Neden MVP'de yok:** Eş zamanlı senkron çok karmaşık (split-brain/çakışma çözümü).
- **Ne zaman / nasıl gelecek:** Sonra.
- **MVP'de hazır olan kanca:** Proje "paket olarak dışa aktar" ile paylaşım MVP'de var; canlı işbirliği ayrı eklenir.

### 7.3 — Cloud-Burst (ağır işi buluta gönderme)
- **Ne:** Yerel donanım yetmediğinde hesabı geçici olarak buluta taşıma.
- **Neden MVP'de yok:** Gerçek bulut entegrasyonu sonra.
- **Ne zaman / nasıl gelecek:** Bulut/dağıtık altyapı geldiğinde.
- **MVP'de hazır olan kanca:** Bellek yetersizliğinde "stream / cloud-burst / iptal" diyaloğunda **cloud-burst seçeneği yer-tutucu** olarak var.

---

# 8. Arayüz İleri Özellikleri

### 8.1 — Tam Serbest Panel Düzeni (panel ayırıp ikinci ekrana taşıma)
- **Ne:** Panelleri tamamen serbestçe sürükleyip ayrı pencerelere/ekranlara koyma.
- **Neden MVP'de yok:** Tam serbest dock sistemi karmaşık ve hata kaynağı.
- **Ne zaman / nasıl gelecek:** v1.x. MVP'de **temel** çoklu-monitör + panel boyutlandırma + sekme taşıma var.
- **MVP'de hazır olan kanca:** Panel/sekme durumu kalıcı; serbest düzen üstüne eklenir.

### 8.2 — Vim/Emacs Klavye Emülasyonu
- **Ne:** Kod editöründe Vim/Emacs tuş düzenleri.
- **Neden MVP'de yok:** Az kullanıcı ister; MVP'de modern varsayılan + özelleştirilebilir kısayollar yeterli.
- **Ne zaman / nasıl gelecek:** Sonra.
- **MVP'de hazır olan kanca:** Kısayol sistemi "tuş seti profilleri" kavramını destekleyecek şekilde tasarlandı.

### 8.3 — Yan Yana Çoklu Motor Sürümü (Unreal mantığı)
- **Ne:** Farklı BioCraft sürümlerinin aynı anda kurulu olması; proje belirli sürüme kilitlenmesi.
- **Neden MVP'de yok:** Çoklu-sürüm yönetimi MVP için fazla.
- **Ne zaman / nasıl gelecek:** v1.x. MVP'de proje "hangi sürümle yapıldı" kaydedilir + farklı sürümde göç uyarısı verilir.
- **MVP'de hazır olan kanca:** Proje manifestinde sürüm alanı + göç çerçevesi MVP'de var.

### 8.4 — Kayıtlı Makrolar / Sık İş Akışları
- **Ne:** Tekrarlanan işlemleri makro olarak kaydedip tek tıkla çalıştırma.
- **Neden MVP'de yok:** MVP'de temel düzey yeterli.
- **Ne zaman / nasıl gelecek:** Şablonlarla (İP-17) ilişkili olarak sonra.
- **MVP'de hazır olan kanca:** Komut paleti + şablon altyapısı MVP'de var.

---

# 9. Bilimsel Determinizm ve Doğrulama

### 9.1 — Gerçek Bit-Bit Determinizm (tekrarüretilebilirlik)
- **Ne:** Aynı veriyle aynı analizin **her seferinde tıpatıp aynı sonucu** vermesi (yayın düzeyi için kritik). Farklı bilgisayarlarda bile bit-bit aynı.
- **Neden MVP'de yok:** Bunu garanti etmek (kayan nokta determinizmi, sıralı işlem) ileri bir mühendislik konusu.
- **Ne zaman / nasıl gelecek:** v1.x. MVP'de **bayrak görünür** ("hızlı keşif / tekrarüretilebilir") ama gerçek garanti sonra.
- **MVP'de hazır olan kanca:** Proje/iş ayarında determinizm bayrağı + format MVP'de var; motor olgunlaşınca "strict" mod açılır.

### 9.2 — Validator Ağı (N-of-M sonuç doğrulama)
- **Ne:** Kritik bir sonucu birden çok bağımsız hesaplamayla çapraz doğrulama (RAM bit hatalarına karşı).
- **Neden MVP'de yok:** MVP için aşırı; basit checksum yeterli.
- **Ne zaman / nasıl gelecek:** Yayın düzeyi iş akışları olgunlaşınca.
- **MVP'de hazır olan kanca:** Kritik okuma/yazmada BLAKE3 bütünlük denetimi MVP'de var (sessiz bozulmaya karşı asgari koruma).

### 9.3 — Golden Test'in Tam Kapsamı + GPU CI
- **Ne:** Tüm bilimsel çıktıların bilinen araçlarla (IGV/samtools) otomatik karşılaştırılması ve GPU'lu otomatik test makinesi.
- **Neden MVP'de yok:** GPU CI için özel donanımlı sunucu (self-hosted runner) gerekir.
- **Ne zaman / nasıl gelecek:** Sonra. MVP'de temel golden test + CPU CI var.
- **MVP'de hazır olan kanca:** Golden test çerçevesi + benchmark MVP'de ilk paketten kurulu.

---

# 10. Pazar Ekonomisi ve Yayıncılık

### 10.1 — Gerçek Mağaza Ekonomisi (ödeme + komisyon)
- **Ne:** BioCraft Market'te ücretli eklenti/şablon satışı, komisyon, gerçek ödeme.
- **Neden MVP'de yok:** Ödeme/komisyon yasal ve finansal altyapı gerektirir.
- **Ne zaman / nasıl gelecek:** Hukuki çerçeve netleşince.
- **MVP'de hazır olan kanca:** Mağaza arayüzü (arama/kurulum/puan/rozet) + "ücretsiz/ücretli" etiketi MVP'de; gerçek para akışı yer-tutucu.

### 10.2 — Tam Doğrulanmış Haber Ağı + 3. Parti Yayın Akışı
- **Ne:** Çok kaynaklı, doğrulanmış bilimsel haber ağı ve dış yayıncıların akış eklemesi.
- **Neden MVP'de yok:** İçerik doğrulama altyapısı + moderasyon olgunlaşmalı.
- **Ne zaman / nasıl gelecek:** v1.x. MVP'de küratörlü haber + opsiyonel çok-AI çapraz kontrol var.
- **MVP'de hazır olan kanca:** Haber akışı + "doğrulanmış" rozeti + opsiyonel çok-AI çapraz kontrol MVP'de.

### 10.3 — Tam Otomatik Yayın Raporu (şablonlu PDF)
- **Ne:** Analizden otomatik, yayın kalitesinde PDF rapor üretme.
- **Neden MVP'de yok:** Tam otomatik raporlama ileri iş.
- **Ne zaman / nasıl gelecek:** v1.x. MVP'de temel rapor (görsel + özet + köken + lisans/atıf) var.
- **MVP'de hazır olan kanca:** Dışa aktarma + köken kaydı altyapısı MVP'de.

### 10.4 — Hesap, Doğrulama ve İtibar Sistemi
- **Ne:** Kullanıcı hesapları, doğrulanmış akademik kimlik, katkı/itibar puanı.
- **Neden MVP'de yok:** MVP yerel-öncelikli; zorunlu hesap benimsenmeyi düşürür.
- **Ne zaman / nasıl gelecek:** Online özellikler (bulut/pazar) olgunlaşınca; hesap her zaman opsiyonel kalır.
- **MVP'de hazır olan kanca:** Yerel kullanım hesapsız tam çalışır; opsiyonel ORCID alanı projede var.

---

# 11. Platform ve Kalite Genişlemesi

### 11.1 — macOS Desteği
- **Ne:** Uygulamanın Mac'lerde çalışması.
- **Neden MVP'de yok:** Apple imzalama/notarization ek yük; akademik kitle ağırlıkla Windows/Linux.
- **Ne zaman / nasıl gelecek:** Sonra.
- **MVP'de hazır olan kanca:** Mimari (wgpu/Metal) macOS'u engellemiyor; kapı açık.

### 11.2 — Güvenlik Denetimleri (pentest, ileri fuzzing)
- **Ne:** Profesyonel sızma testi ve kapsamlı güvenlik denetimi.
- **Neden MVP'de yok:** Yayın öncesi/periyodik yapılır.
- **Ne zaman / nasıl gelecek:** Yayın öncesi ve düzenli aralıklarla.
- **MVP'de hazır olan kanca:** Dosya ayrıştırıcılar fuzzing hedefi + cargo-audit CI'da MVP'den itibaren.

### 11.3 — Yan-Kanal Saldırı Koruması (ileri)
- **Ne:** Çok-kiracılı/ağ senaryolarında zamanlama tabanlı saldırılara karşı koruma.
- **Neden MVP'de yok:** Yerel-öncelikli MVP'de risk düşük.
- **Ne zaman / nasıl gelecek:** Ağ/çok-kiracılı senaryolar geldiğinde.
- **MVP'de hazır olan kanca:** Mimaride yer bırakıldı (not düzeyinde).

---

# 12. Şirketleşme ve Hukuki Olgunlaşma

> Bunlar teknik değil **operasyonel** ertelenenler. Hukuk dosyasında detaylı; burada özet.

- **Şirketleşme (LTD → A.Ş.):** MVP doğrulanıp ticari gelir/yatırım/istihdam gündeme gelince. (MVP'de gerek yok.)
- **Teknokent/TÜBİTAK/KOSGEB teşvikleri:** Şirketleşmeyle birlikte değerlendirilir.
- **Uluslararası genişleme uyumu:** Hedef pazar (AB/ABD) veri ve içerik yasaları, kademeli.
- **Kurumsal/bulut sürümler:** Ticari olgunlaşmayla.
- **Kod imzalama sertifikası:** Tüzel kişilik (şirket) kurulunca alınır (dağıtım için gerekli).

> **Not:** Hukuk dosyasının da defalarca vurguladığı gibi, bu adımların hiçbiri profesyonel görüş olmadan atılmamalı (avukat + mali müşavir).

---

# SON — Bu Dosya ve MVP İlişkisi

Bu dosya, **MVP'yi küçük ve bitirilebilir tutmak** için neyi ertelediğimizi ve neyi şimdiden hazırladığımızı gösterir. Üç önemli güvence:

1. **Hiçbir ertelenen özellik için sökme/yeniden yazma gerekmeyecek** — her birinin MVP'de bir kancası var.
2. **En kritik kurallar (hassas/PHI veri sınırı, açık güvenlik, ABI sözleşmesi) MVP'de baştan doğru kuruluyor** — gelecekteki hiçbir eklenti bunları kıramaz.
3. **Senin kararınla Donanım Koruma (Zero-Impact) ve determinizm bayrağı MVP'ye taşındı** — yani "ne kadar az ertelersek o kadar iyi" ilken uygulandı.

MVP bittikten sonra bu listeden, en çok değer katacak özellikten başlayarak ilerleriz.
