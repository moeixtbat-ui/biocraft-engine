# BioCraft Engine — Çekirdek Eklenti (BioCraft Studio) Yol Haritası

> **Belge tipi:** STATİK mühendislik kontratı. Her iş paketi (ÇE) tek başına bir kodlama aracına verilebilir.
> **Sürüm:** 1.2 (dondurulmuş taban) · **Tarih:** 2026-06
> **Kapsam:** **Çekirdek eklenti** = analiz + görüntüleme (IGV + JBrowse 2 + UCSC + Geneious/CLC seviyesi ve ötesi) **+ veritabanı erişimi (BLAST/PDB/NCBI...) birleşik.** Temel uygulamanın eklenti host'u (`Temel-Uygulama.md` → İP-07) üzerinde çalışır ve **varsayılan kurulu gelir.**
> **Önkoşul belge:** `Temel-Uygulama.md`. Bu eklenti, oradaki motor/SDK/host/render/node/bellek altyapısını kullanır.
> **1.2 değişiklikleri (karar günlüğü):** Eklenti adı **BioCraft Studio (temel kit)** olarak sabitlendi. **MVP kapsamı netleşti:** bazı paketler ilk sürümde **tam**, bazıları **temel düzey / şimdilik tam kullanılmıyor** (aşağıda "MVP Kapsam Haritası"; ertelenenler `MVP-sonrasi.md` §4'te). BGZF-farkında (sıkıştırma bloğu sınırına saygılı) okuma ve **BLAKE3 bütünlük denetimi** ÇE-01'e net eklendi. Somut edge-case eşikleri (`Temel-Uygulama.md` İP-08/İP-21 ile) referanslandı.

---

## NASIL KULLANILIR

Bu eklentinin bir paketini kodlatırken sırayla yapıştır:

1. **`Temel-Uygulama.md` → Bölüm 0 (Sabitler ve Sözleşmeler).** (Eklenti, motorun teknoloji yığını, klasör yapısı, isimlendirme ve TDA kurallarına uyar.)
2. **Bu belgedeki `Bölüm 0-CE` (Çekirdek Eklenti Sabitleri).**
3. **Kodlatmak istediğin tek bir ÇE paketi.**
4. `Temel-Uygulama.md`'deki **hazır komut şablonunu** kullan (aynısı geçerli).

> Eklenti, temel uygulamanın İP-07 host'una, İP-05 node sistemine, İP-08 bellek+donanım-koruma orkestratörüne, İP-04 render altyapısına, İP-10 gizlilik sınırına, İP-11 undo/redo ve İP-16 TDA bileşenlerine yaslanır. İlgili paket "Bağımlılıklar" satırında belirtilir.

---

## MVP KAPSAM HARİTASI (A4 kararı — ne tam, ne temel)

> Tüm paketlerin mimarisi MVP'de kurulur (gelecek için sorun çıkmaz). Ancak işlevsel derinlik şöyle ayrılır:

| Paket | MVP'de durum | Not |
| --- | --- | --- |
| ÇE-00 İskelet/SDK/Manifest | **TAM** | Temel; her şey buna bağlı |
| ÇE-01 Veri G/Ç + Format | **TAM** | BGZF-farkında + BLAKE3 + out-of-core |
| ÇE-02 Genom Tarayıcı | **TAM** | Ana görselleştirme |
| ÇE-03 Hizalama Görünümü | **TEMEL** | İleri (split-read/SV) v1.x — `MVP-sonrasi.md` §4.1 |
| ÇE-04 Varyant (VCF) | **TAM** | DuckDB + filtre |
| ÇE-05 Anotasyon/İz | **TEMEL** | İleri anotasyon türleri v1.x — `MVP-sonrasi.md` §4.2 |
| ÇE-06 Dizi/MSA | **TEMEL** | İleri klonlama/primer v1.x — `MVP-sonrasi.md` §4.3 |
| ÇE-07 3B Yapı | **TAM** | GPU/wgpu görüntüleme |
| ÇE-08 Analiz/Hesap + Köprü | **TEMEL** | Native temel hesap TAM; tam pipeline v1.x — `MVP-sonrasi.md` §4.4 |
| ÇE-09 Veritabanı Arama | **TAM** | Birleşik arama + tek-tık |
| ÇE-10 Node Entegrasyonu | **TEMEL** | Temel node seti; ileri node'lar v1.x — `MVP-sonrasi.md` §4.5 |
| ÇE-11 Dışa Aktarma/Oturum | **TAM** | Görsel/veri + temel rapor |
| ÇE-12 Perf/Erişilebilirlik/Doğruluk | **TAM** | Çapraz güvence |

> **"TEMEL" demek:** O paketin çekirdek işlevi MVP'de çalışır, ama ileri özellikleri sonraya bırakıldı. Mimari hazır olduğu için sonradan eklemek sökme gerektirmez.

---

## BÖLÜM 0-CE — ÇEKİRDEK EKLENTİ SABİTLERİ

> `Temel-Uygulama.md` Bölüm 0'daki tüm sabitler (marka, teknoloji, klasör, isimlendirme, tasarım token'ları, TDA listesi) **aynen geçerlidir.** Aşağıdakiler eklentiye özeldir.

### 0-CE.1 — Eklenti Kimliği

| Alan | Değer |
| --- | --- |
| Eklenti kimliği | `biocraft.core.studio` |
| Görünen ad | **BioCraft Studio** (temel kit — çekirdek analiz/görüntüleme/veritabanı stüdyosu) |
| Tür | Birinci-parti çekirdek eklenti; **varsayılan kurulu** |
| Çalıştığı yer | Temel uygulama İP-07 host'u (WASM + out-of-process konektörler + opsiyonel konteyner köprüsü) |
| İç yapı | Kullanıcıya **tek tutarlı eklenti**; içeride mantıksal alt-modüller |
| Sürümleme | Çekirdekten **bağımsız** sürümlenir/güncellenir, ABI (WIT) uyumu korunur |

### 0-CE.2 — İlan Edilen Capability'ler (İP-07 ile)

`fs` (proje VFS — veri okuma/yazma), `net` (veritabanı/uzak dosya — gizlilik onaylı), `gpu` (genom tuvali + 3B render), `db` (yerel SQLite/DuckDB sorgu), opsiyonel konteyner (harici bio-araçlar). Capability'ler manifest'te ilan, kurulumda kullanıcıya görünür, çalışmada denetlenir. **Hassas/PHI veri sınırı çekirdek (İP-10) tarafından korunur; eklenti bunu aşamaz.**

### 0-CE.3 — Desteklenen Formatlar (MVP hedefi)

FASTA/FASTQ · BAM/SAM/CRAM (+ .bai/.crai) · VCF/BCF (+ index) · BED · GFF/GTF · BigWig/BigBed · Wig · 2bit · PDB/mmCIF (3B) · GenBank. Hepsi **indeksli + out-of-core**; uzak URL/S3 bayt-aralığı erişimi. **Sıkıştırılmış (BGZF) dosyalar blok sınırına saygılı okunur** (ham bayt değil — bilimsel doğruluk için). Format ayrıştırma için Rust bio crate'leri (örn. `noodles` ailesi) tercih; gerekirse harici araç köprüsü.

### 0-CE.4 — Alt-Modül Haritası (Dosya Yerleşimi Kökü)

Eklenti `plugins/biocraft-core-studio/` altında; içeride alt-modüller:

```
plugins/biocraft-core-studio/
├─ Cargo.toml + manifest (biocraft.core.studio, capability ilanı, ABI sürümü)
├─ src/
│  ├─ io/            # ÇE-01: format ayrıştırma, indeksleme, BGZF-farkında, BLAKE3, uzak erişim
│  ├─ browser/       # ÇE-02: genom tarayıcı tuvali
│  ├─ alignment/     # ÇE-03: hizalama (read) görünümü [TEMEL]
│  ├─ variant/       # ÇE-04: VCF görünüm + filtre
│  ├─ annotation/    # ÇE-05: anotasyon + iz yönetimi [TEMEL]
│  ├─ sequence/      # ÇE-06: dizi görüntüleme/düzenleme + MSA [TEMEL]
│  ├─ structure3d/   # ÇE-07: 3B PDB/mmCIF görüntüleyici
│  ├─ compute/       # ÇE-08: analiz/hesap + harici araç köprüsü [TEMEL]
│  ├─ database/      # ÇE-09: birleşik veritabanı arama + konektörler
│  ├─ nodes/         # ÇE-10: node kayıtları [TEMEL]
│  ├─ export/        # ÇE-11: dışa aktarma + oturum
│  └─ perf/          # ÇE-12: performans/erişilebilirlik/doğruluk yardımcıları
```

### 0-CE.5 — Rakip Paritesi İlkesi

Eklenti, **IGV + JBrowse 2 + UCSC Genome Browser + Geneious/CLC**'nin temel görselleştirme/analiz yeteneklerini karşılar; üstüne fark özellikler ekler: node tabanlı akış entegrasyonu, gerçek 60 FPS akıcılık, entegre veritabanı arama, hibrit kod+görsel, modern GPU 3B, AI-hazır arayüz. Doğruluk, bilinen araçlarla golden test ile kanıtlanır; performans IGV ile yan yana benchmark.

---

## İŞ PAKETLERİ (ÇE)

> Her ÇE paketi için: önce `Temel-Uygulama.md` Bölüm 0 + bu belgenin Bölüm 0-CE'si + tek bir ÇE paketi yapıştırılır.

### ÇE-00 — Eklenti İskeleti, SDK Entegrasyonu ve Manifest  **[MVP: TAM]**

**Amaç:** Çekirdek eklentinin İP-07 host'una kaydolan iskeleti: manifest, capability ilanı, SDK bağlantısı, alt-modül yapısı, varsayılan-kurulu paketleme kancası.
**Kapsam:** `biocraft.core.studio` manifesti, capability (`fs/net/gpu/db`) ilanı, SDK ile UI/node/ayar uzantı kaydı iskeleti, 0-CE.4 alt-modül klasörleri (boş/stub, derlenir).
**Bağımlılıklar:** Temel-Uygulama İP-07 (host/SDK), İP-00 (workspace).
**İlgili modül(ler):** `plugins/biocraft-core-studio/` (kök + manifest).
**Teknoloji:** Rust + `biocraft-sdk`, WIT (ABI), Wasmtime hedefi.

**Somut Davranış/Spec:**
- Manifest: kimlik (`biocraft.core.studio`), sürüm (SemVer), hedef ABI sürümü, capability ilanı, sağladığı UI noktaları/node/ayar bölümleri.
- Eklenti İP-07 host tarafından keşfedilir, capability + ABI doğrulanır, yüklenir. Kullanıcı capability'leri kurulumda görür.
- Alt-modüller (0-CE.4) boş iskelet olarak derlenir; her biri SDK'nın ilgili uzantı noktasına kayıt arayüzü açar.
- **Varsayılan kurulu:** Paketleme (İP-20) bu eklentiyi motorla aynı kuruluma dahil eder; ilk açılışta hazır.
- Bağımsız sürümleme: eklenti kendi sürümünü taşır; çekirdek ABI uyumu kontrol edilir.

**Dosya/Modül Yerleşimi:** `plugins/biocraft-core-studio/{Cargo.toml, manifest.*}`, `src/lib.rs` (kayıt giriş noktası) + 0-CE.4 alt-modül `mod.rs` stub'ları.

**TDA Kontrolleri:** ABI/capability uyumsuzsa net hata (1,4); eklenti yüklenemezse çekirdek ayakta (İP-07 izolasyon); capability'ler kullanıcıya şeffaf (gizlilik).

**Kabul Kriterleri:**
- [ ] Eklenti İP-07 host'unda keşfedilir, doğrulanır, yüklenir.
- [ ] Capability'ler manifest'te ilan ve kurulumda görünür.
- [ ] Tüm alt-modül iskeletleri derlenir; SDK kayıt noktaları açık.
- [ ] Paketleme kancası eklentiyi varsayılan kuruluma ekler (İP-20 ile).

**Varsayımlar:** Görünen ad "BioCraft Studio (temel kit)". Eklenti tek paket; alt-modüller iç ayrım.
**Dikkat:** ABI sürümünü baştan sabitle; çekirdek-eklenti uyumu buna bağlı.

---

### ÇE-01 — Veri G/Ç ve Format Ayrıştırıcıları  **[MVP: TAM]**

**Amaç:** Tüm biyoinformatik formatlarını indeksli, out-of-core, hata-toleranslı, BGZF-farkında, bütünlük-denetimli okuma/yazma; uzak erişim.
**Kapsam:** 0-CE.3 formatları, indeks (.bai/.crai/.tbi) kullanımı, akışlı/out-of-core okuma, BGZF blok-farkında okuma, uzak URL/S3 bayt-aralığı, BLAKE3 bütünlük denetimi, format otomatik tanıma.
**Bağımlılıklar:** ÇE-00, İP-08 (bellek/out-of-core), İP-10 (provenance/gizlilik).
**İlgili modül(ler):** `src/io/`.
**Teknoloji:** Rust bio crate'leri (örn. `noodles`), DuckDB/Arrow (büyük tablo), Tokio (uzak I/O), mmap, BLAKE3.

**Somut Davranış/Spec:**
- **Formatlar:** FASTA/FASTQ, BAM/SAM/CRAM, VCF/BCF, BED, GFF/GTF, BigWig/BigBed, Wig, 2bit, PDB/mmCIF, GenBank. Otomatik format tanıma.
- **BGZF-farkında okuma:** Sıkıştırılmış dosyalar (BAM/VCF.gz vb.) **BGZF blok sınırlarından** okunur, ham bayt parçalama YAPILMAZ (yanlış parçalama çoğu bloğu açılamaz hale getirir).
- **Devasa dosya:** İndeksli akışlı erişim; **sadece görünen/sorgulanan bölge belleğe**, tüm dosya değil. 4 TB dosyada bile "load all" yok (mmap + region). İndeks yoksa "dosya indekssiz, indeksleyeyim mi?".
- **Uzak:** HTTP(S)/S3 üzerinden indeksli uzaktan erişim; yalnızca gerekli bayt aralığı çekilir (indirmeden açma); resumable (Range header), zaman aşımı (bağlantı 10s, boşta 60s).
- **Büyük varyant:** Milyonlarca satır DuckDB/Arrow ile; predicate pushdown; sadece görünen/filtreli kısım.
- **Bütünlük (BLAKE3):** Boyut/BLAKE3 sağlama denetimi; bozuksa net hata (satır/sütun ipucu varsa) + yeniden indirme/onarma önerisi + karantina (sessiz yükleme yok).
- **Bellek:** Dosya açmadan önce İP-08 bütçe denetimi; aşımda stream/iptal diyaloğu.
- **Provenance:** Her yüklenen veri için kaynak/sürüm/tarih/BLAKE3 kaydı (İP-10); bilimsel set ise lisans/atıf alanı da (ÇE-09).
- **Bozuk UTF-8 / sayısal:** Bozuk UTF-8 → lossy decode + uyarı + orijinal bayt yedeği + encoding seçimi; NaN/sonsuz/sıfıra bölme → checked aritmetik + UI'da "—" gösterimi.

**Dosya/Modül Yerleşimi:** `src/io/{detect.rs, fasta.rs, bam.rs, vcf.rs, bed_gff.rs, bigwig.rs, structure.rs, remote.rs, index.rs, bgzf.rs, integrity.rs}`.

**TDA Kontrolleri:** İndekssiz/bozuk dosya → net + çözüm butonu (1,4); büyük dosya bütçe diyaloğu (İP-08); yükleme ilerleme/iptal (3,12); format yanlışsa anlamlı hata (4); bozuk dosya karantina (19).

**Kabul Kriterleri:**
- [ ] 0-CE.3 formatları açılır; format otomatik tanınır.
- [ ] Büyük BAM/CRAM indeksli + BGZF-farkında akışla açılır; tüm dosya RAM'e alınmaz.
- [ ] Uzak URL'den bayt-aralığı (resumable) erişimi çalışır (indirmeden).
- [ ] İndekssiz dosya için "indeksleyeyim mi?" akışı çalışır.
- [ ] Bozuk dosya çökme yerine güvenli reddedilir + BLAKE3 ile tespit + çözüm önerir.

**Varsayımlar:** Bazı nadir formatlar/yerel BLAST DB sonra (`MVP-sonrasi.md` §4.6). Yazma (export) ÇE-11 ile koordine.
**Dikkat:** Tüm büyük okuma İP-08 orkestratörüne uymalı; aksi halde OOM. Ayrıştırıcılar fuzzing hedefi (İP-09/İP-21).

---

### ÇE-02 — Genom Tarayıcı (Genome Browser)  **[MVP: TAM]**

**Amaç:** IGV+JBrowse+UCSC seviyesinde, gerçek 60 FPS akıcılıkta çok-izli genom tarayıcı tuvali.
**Kapsam:** Çok-iz görünüm, pan/zoom/minimap, referans dizi, koordinat cetveli, bölge gezinme/arama, downsampling/LOD, çoklu örnek senkron, ölçüm araçları.
**Bağımlılıklar:** ÇE-00, ÇE-01 (veri), İP-04 (render/LOD), İP-08 (bellek), İP-16 (TDA bileşenleri).
**İlgili modül(ler):** `src/browser/`.
**Teknoloji:** wgpu (tuval çizimi), egui (kontroller). _Bevy ECS v1'de KULLANILMAZ; tuval doğrudan wgpu/egui ile (`MVP-sonrasi.md` §5.1)._

**Somut Davranış/Spec:**
- **Çok-iz:** Referans dizi izi, gen/anotasyon izleri, hizalama izi, kapsama izi, varyant izi; pan/zoom akıcı; koordinat gezinme.
- **Referans genom:** Hazır referanslar (hg38/hg19, fare vb.) tek tık; özel referans yükleme; veritabanı (ÇE-09) ile entegre indirme. İndirilen referansın lisans/atıf bilgisi provenance'a işlenir (ÇE-09/İP-10). _(Referans setler gömülü gelmez; ÇE-09'dan indirilir.)_
- **Hızlı gezinme:** Gen adı/koordinat/rsID ile atlama, motif/dizi arama, yer imi (bookmark), ileri-geri geçmiş.
- **Yüksek kapsama:** Otomatik downsampling/yoğunluk + LOD; ham↔özet geçişi; 60 FPS korunur (İP-04).
- **Çoklu örnek:** Çoklu örnek yan yana/üst üste izler; **senkron gezinme** (biri kayınca hepsi); karşılaştırma modu.
- **Ölçüm:** Cetvel/mesafe ölçme, bölge seçme/işaretleme, vurgulama, açıklama ekleme.
- **Durum:** Açık izler/bölge/ayarlar oturum/proje ile kaydedilir; açılışta geri yüklenir (İP-11).
- **Veri tipi:** DNA/RNA/protein ayırt edilir; uygun görünüm otomatik; yanlış eşleşme uyarılır.

**Dosya/Modül Yerleşimi:** `src/browser/{canvas.rs, tracks.rs, ruler.rs, navigate.rs, reference.rs, lod.rs, multisample.rs, measure.rs}`.

**TDA Kontrolleri:** Boş tarayıcı rehberi (5); büyük veride sadeleşme + akıcılık (11, İP-04); ölçüm/işaret geri alınabilir (2); gezinme geçmişi (ileri-geri); referans yoksa [İndir] (1, ÇE-09).

**Kabul Kriterleri:**
- [ ] Çok-iz görünüm referans donanımda 60 FPS pan/zoom.
- [ ] Gen/koordinat/rsID/motif arama + bookmark + geçmiş çalışır.
- [ ] Yüksek kapsamada downsampling/LOD ile akıcılık korunur.
- [ ] Çoklu örnek senkron gezinme + karşılaştırma çalışır.
- [ ] Görünüm durumu proje ile kaydedilir/geri yüklenir.

**Varsayımlar:** Bazı ileri iz tipleri/özel render v1.x. Hazır referans seti sınırlı başlar, ÇE-09 ile genişler.
**Dikkat:** Tuval çizimi İP-04 kare bütçesine + LOD'a uymalı; aksi halde büyük genomda FPS düşer.

---

### ÇE-03 — Hizalama (Alignment) Görünümü  **[MVP: TEMEL]**

> **MVP'de temel düzey:** Temel read yığını görünümü çalışır; ileri görselleştirmeler (split-read/SV detayı) v1.x (`MVP-sonrasi.md` §4.1).

**Amaç:** BAM/CRAM okuma hizalamalarının detaylı, akıcı görselleştirilmesi (IGV seviyesi temel).
**Kapsam:** Okuma yığını, eşleşmeme renklendirme, çift-uç bağlama, kalite, CIGAR, gruplama/sıralama.
**Bağımlılıklar:** ÇE-00, ÇE-01, ÇE-02 (tuval/iz), İP-04.
**İlgili modül(ler):** `src/alignment/`.
**Teknoloji:** wgpu (yoğun read render), egui (kontrol).

**Somut Davranış/Spec:**
- **Read yığını:** Okumalar yığın halinde; eşleşmemeler (mismatch) renkli; çift-uç (paired-end) bağlama gösterimi; insert size.
- **Kalite/CIGAR:** Baz kalitesi renklendirme; CIGAR (insertion/deletion/soft-clip) görsel; MAPQ.
- **Gruplama/sıralama:** Strand/örnek/etikete göre grupla; başlangıç/kalite/uzunluğa göre sırala; renk şemaları (renk körü dostu).
- **Yoğunlukta:** Çok derin bölgede downsampling + özet; akıcılık korunur.
- **Etkileşim:** Read üstüne gel → detay (pozisyon, kalite, etiketler); seçili read'i vurgula.

**Dosya/Modül Yerleşimi:** `src/alignment/{view.rs, stack.rs, color.rs, group_sort.rs}`.

**TDA Kontrolleri:** Derin bölgede sadeleşme + uyarı (11); read detayı keşfedilebilir (13); renk körü dostu; boş hizalama bölgesi rehberi (5).

**Kabul Kriterleri:**
- [ ] Read yığını eşleşmeme/çift-uç/kalite ile doğru render edilir.
- [ ] Gruplama/sıralama/renk şemaları çalışır.
- [ ] Derin bölgede downsampling ile akıcılık korunur.
- [ ] Read detayı etkileşimle görünür.

**Varsayımlar:** Çok ileri hizalama görselleştirmeleri (split-read/SV detay) v1.x. Varyant çağırma köprüsü ÇE-08'de.
**Dikkat:** Yoğun read render İP-04 LOD + İP-08 bellek ile koordine.

---

### ÇE-04 — Varyant (VCF) Görünümü ve Filtreleme  **[MVP: TAM]**

**Amaç:** Varyantların görselleştirilmesi + çok-örnekli genotip + güçlü filtreleme.
**Kapsam:** Varyant izi, genotip ızgarası (çok örnek), filtreleme, zigosite renklendirme, INFO/FORMAT detayı, kayıtlı filtre setleri.
**Bağımlılıklar:** ÇE-00, ÇE-01 (büyük VCF/DuckDB), ÇE-02 (iz), İP-08.
**İlgili modül(ler):** `src/variant/`.
**Teknoloji:** egui (ızgara/filtre), DuckDB/Arrow (büyük VCF sorgu), wgpu (iz).

**Somut Davranış/Spec:**
- **Varyant izi:** Genomik konumda varyantlar; tip (SNV/indel/SV) görsel ayrım.
- **Genotip ızgarası:** Çok örnekli genotip matrisi; zigosite (hom/het/ref) renklendirme; sanal liste (binlerce satır akıcı).
- **Filtreleme:** Kalite/derinlik/tip/bölge filtresi; **kayıtlı filtre setleri**; görsel + sayısal sonuç; INFO/FORMAT alan filtreleri.
- **Detay:** Varyant seç → INFO/FORMAT tam detay; rsID/anotasyon bağlantısı.
- **Büyük VCF:** İndeksli + DuckDB sorgu; sadece görünen/filtreli işlenir; arayüz donmaz.
- **Dışa aktarma:** Filtrelenen varyantlar VCF/BED/CSV (ÇE-11).

**Dosya/Modül Yerleşimi:** `src/variant/{track.rs, genotype_grid.rs, filter.rs, detail.rs, query.rs}`.

**TDA Kontrolleri:** Filtre anlık doğrulama (8); büyük sette sanal liste + akıcılık (6,11); filtre işlemleri geri alınabilir/kaydedilebilir (2); boş sonuç rehberi (5).

**Kabul Kriterleri:**
- [ ] Varyant izi + çok-örnekli genotip ızgarası doğru render.
- [ ] Kalite/derinlik/tip/bölge filtresi + kayıtlı setler çalışır.
- [ ] Milyonlarca varyantta arayüz akıcı (sanal liste + DuckDB).
- [ ] INFO/FORMAT detayı görünür; filtreli dışa aktarma çalışır.

**Varsayımlar:** Varyant anotasyon/yorumlama (gerçek) AI/araç köprüsüyle sonra; MVP'de görselleştirme + filtre + temel istatistik.
**Dikkat:** Büyük VCF kesinlikle out-of-core (İP-08); tüm dosyayı belleğe alma.

---

### ÇE-05 — Anotasyon ve İz (Track) Yönetimi  **[MVP: TEMEL]**

> **MVP'de temel düzey:** Temel anotasyon (GFF/GTF/BED + ekle/düzenle/sil) + iz yönetimi çalışır; ileri anotasyon türleri/özel iz render v1.x (`MVP-sonrasi.md` §4.2).

**Amaç:** Gen/özellik anotasyonlarının görüntülenmesi + kullanıcı anotasyonu düzenleme + kapsamlı iz yönetimi.
**Kapsam:** Anotasyon izleri, özel anotasyon ekle/düzenle, renk/etiket, GFF/GTF/BED içe-dışa, iz ekle/kaldır/sırala/grupla/yükseklik/gizle, toplu küçük dosya.
**Bağımlılıklar:** ÇE-00, ÇE-01, ÇE-02, İP-11 (undo/redo).
**İlgili modül(ler):** `src/annotation/`.
**Teknoloji:** egui, wgpu (iz render).

**Somut Davranış/Spec:**
- **Anotasyon:** Gen/özellik izleri (GFF/GTF/BED); özel anotasyon ekleme/düzenleme/silme; renk/etiket; içe-dışa aktarma. Düzenlemeler **undo/redo** (İP-11).
- **İz yönetimi:** İz ekle/kaldır/sırala (sürükle)/grupla/yükseklik ayarla/renk/gizle-göster; iz başına ayar paneli; oturumda kaydet.
- **Toplu:** 100+ küçük BED/GFF toplu yükleme + grup yönetimi + oturum dosyasıyla tek tık geri yükleme.
- **Etkileşim:** Anotasyona tıkla → detay; bölgeye git; düzenle.

**Dosya/Modül Yerleşimi:** `src/annotation/{tracks.rs, edit.rs, manage.rs, batch.rs, io.rs}`.

**TDA Kontrolleri:** Anotasyon düzenleme geri alınabilir (2); toplu yükleme ilerleme (3); iz durumu kalıcı (9, İP-11); boş iz rehberi (5).

**Kabul Kriterleri:**
- [ ] GFF/GTF/BED anotasyon izleri yüklenir; özel anotasyon eklenir/düzenlenir (undo/redo ile).
- [ ] İz ekle/kaldır/sırala/grupla/yükseklik/renk/gizle çalışır.
- [ ] 100+ küçük dosya toplu yüklenir; oturumla geri yüklenir.

**Varsayımlar:** İleri anotasyon türleri/özel iz render v1.x. Anotasyon paylaşımı (ÇE-09/proje) ile.
**Dikkat:** İz durumu İP-11 ile kaydedilmeli; aksi halde oturum geri yüklenmez.

---

### ÇE-06 — Dizi Görüntüleme, Düzenleme ve Hizalama (MSA)  **[MVP: TEMEL]**

> **MVP'de temel düzey:** Temel dizi düzenleme + çiftli/çoklu hizalama görünümü + çeviri/ORF çalışır; ileri moleküler klonlama/primer tasarımı/tam restriksiyon analizi v1.x (`MVP-sonrasi.md` §4.3).

**Amaç:** Dizi (DNA/RNA/protein) görüntüleme + düzenleme + çiftli/çoklu hizalama görünümü (Geneious/CLC seviyesi temel).
**Kapsam:** Dizi editörü, düzenleme işlemleri, çiftli/çoklu hizalama (pairwise/MSA) görünümü + temel algoritmalar/köprü, çeviri/ORF, veri tipi tanıma.
**Bağımlılıklar:** ÇE-00, ÇE-01, İP-11 (undo/redo), ÇE-08 (algoritma/köprü için).
**İlgili modül(ler):** `src/sequence/`.
**Teknoloji:** egui (editör), Rust (hizalama algoritmaları) veya harici araç köprüsü (ÇE-08).

**Somut Davranış/Spec:**
- **Düzenleme:** Dizi görüntüleme + düzenleme — kes/yapıştır/sil/ekle, **ters-tümleyen (reverse-complement)**, çevir (translate); anotasyon ekleme; tümü **undo/redo** (İP-11).
- **Hizalama:** Çiftli (pairwise) ve çoklu (MSA) hizalama görünümü; temel hizalama algoritmaları (Rust) veya harici araç köprüsü (MAFFT/MUSCLE via ÇE-08); hizalama renklendirme/konsensüs.
- **Veri tipi:** DNA/RNA/protein otomatik tanıma; uygun araçlar (örn. çeviri sadece nükleotid); yanlış işlem uyarısı.
- **Temel moleküler biyoloji araçları:** Çeviri çerçeveleri/ORF bulma, temel restriksiyon bölgesi/primer gösterimi (Geneious benzeri, MVP düzeyi).
- **Görsel:** Renkli baz/amino asit; konum cetveli; arama (motif/alt-dizi).

**Dosya/Modül Yerleşimi:** `src/sequence/{editor.rs, edit_ops.rs, align.rs, translate.rs, tools.rs}`.

**TDA Kontrolleri:** Tüm düzenleme geri alınabilir (2); yanlış işlem (örn. proteini çevir) uyarılır (8); hizalama ilerleme/iptal (3,12); boş editör rehberi (5).

**Kabul Kriterleri:**
- [ ] Dizi düzenleme (kes/yapıştır/ters-tümleyen/çevir) + undo/redo çalışır.
- [ ] Çiftli + çoklu hizalama görünümü (algoritma veya köprü) çalışır.
- [ ] DNA/RNA/protein tanınır; uygun araçlar sunulur.
- [ ] Çeviri/ORF + temel restriksiyon/primer gösterimi çalışır.

**Varsayımlar:** İleri moleküler klonlama/primer tasarımı/tam restriksiyon analizi v1.x; MVP'de temel set. Ağır MSA harici araç köprüsüyle (ÇE-08).
**Dikkat:** Düzenleme işlemleri Command Pattern (İP-11) ile; aksi halde geri alma çalışmaz.

---

### ÇE-07 — 3B Yapı Görüntüleyici (PDB/mmCIF)  **[MVP: TAM]**

**Amaç:** Protein/molekül 3B yapılarının GPU-hızlandırmalı görselleştirilmesi; düşük donanımda ölçeklenir.
**Kapsam:** PDB/mmCIF 3B görüntüleme, döndür/yakınlaştır, gösterim modları, ölçeklenir performans, CPU fallback, veritabanından tek-tık.
**Bağımlılıklar:** ÇE-00, ÇE-01 (PDB/mmCIF), İP-04 (wgpu), İP-08.
**İlgili modül(ler):** `src/structure3d/`.
**Teknoloji:** wgpu + özel shader/geometri. _Bevy ECS v1'de KULLANILMAZ; tüm 3B doğrudan wgpu ile. r128 THREE.js benzeri kütüphane YOK (native) (`MVP-sonrasi.md` §5.1)._

**Somut Davranış/Spec:**
- **Görüntüleme:** PDB/mmCIF 3B yapı; döndür/yakınlaştır/kaydır; gösterim modları (kurdele/cartoon, top-çubuk/ball-stick, yüzey/surface); zincir/kalıntı renklendirme.
- **Etkileşim:** Kalıntı/atom seç → detay; mesafe ölçme; zincir gizle/göster; arka plan/aydınlatma.
- **Ölçeklenir:** Büyük yapı/düşük donanımda basitleştirilmiş gösterim + uyarı; **GPU yoksa CPU fallback** (yavaş ama çalışır).
- **Entegrasyon:** Veritabanından (ÇE-09) seçilen PDB yapısı **tek tıkla** burada açılır.
- **Görsel dışa aktarma:** Yüksek çözünürlüklü 3B anlık görüntü (PNG, ÇE-11).

**Dosya/Modül Yerleşimi:** `src/structure3d/{viewer.rs, render.rs, modes.rs, interact.rs, fallback.rs}`.

**TDA Kontrolleri:** GPU yoksa CPU fallback + uyarı (1,11); büyük yapıda sadeleşme + uyarı (11); seçim/ölçüm geri alınabilir (2); boş görüntüleyici rehberi (5).

**Kabul Kriterleri:**
- [ ] PDB/mmCIF yapı 3B render; döndür/yakınlaştır akıcı.
- [ ] Cartoon/ball-stick/surface modları + renklendirme çalışır.
- [ ] GPU yokken CPU fallback ile çalışır; büyük yapıda sadeleşir.
- [ ] Veritabanından (ÇE-09) tek-tık 3B açma çalışır.

**Varsayımlar:** İleri yapısal analiz (hizalama/dokuma/elektron yoğunluğu) v1.x; MVP'de görüntüleme + temel etkileşim. Molekül dinamiği yok (`MVP-sonrasi.md` §4.6).
**Dikkat:** wgpu ve CUDA aynı anda VRAM kullanmamalı (İP-04); büyük yapı bellek bütçesine uymalı (İP-08).

---

### ÇE-08 — Analiz/Hesaplama ve Harici Araç Köprüsü  **[MVP: TEMEL]**

> **MVP'de temel düzey:** Native temel hesap (kapsama/istatistik/bölge karşılaştırma — Windows dahil konteynersiz) **TAM** çalışır; opsiyonel harici araç köprüsü çalışır. Tam uçtan uca pipeline v1.x (`MVP-sonrasi.md` §4.4).

**Amaç:** "Gerçek veri üretebilme" — görselleştirmenin ötesinde hesap + yaygın CLI araçlarına güvenli köprü.
**Kapsam:** Kapsama/istatistik/bölge karşılaştırma/hizalama hesabı, varyant çağırma köprüsü, harici CLI (samtools/bcftools/BWA...) köprüsü, eksik araç/runtime kurulumu, uzun iş yönetimi, veri dışa aktarma.
**Bağımlılıklar:** ÇE-00, ÇE-01, İP-07 (subprocess/konteyner), İP-08 (bellek/paralel + donanım koruma), İP-15 (bulut/dağıtık kanca), İP-16.
**İlgili modül(ler):** `src/compute/`.
**Teknoloji:** Rust (yerel hesap — **birincil**), Apptainer/Docker + out-of-process (**opsiyonel** harici araç), Arrow Flight (veri taşıma), Tokio/Rayon.

**Somut Davranış/Spec:**
- **Üç katmanlı çalıştırma (müsaitlik sırasıyla denenir, kullanıcı asla takılmaz):**
  1. **Native Rust hesap (birincil — bağımlılıksız):** En sık işlemler (kapsama, temel istatistik, indeksleme, bölge karşılaştırma, BAM/CRAM/VCF işleme) native Rust (`noodles` vb.) ile, **konteyner gerektirmeden, her platformda — Windows dahil — kutudan çıkar çalışır.**
  2. **Opsiyonel konteyner:** Yalnızca bundle'lanması zor ağır araçlar (MAFFT/MUSCLE, GATK, karmaşık pipeline) için Apptainer/Docker. Windows'ta WSL2/Docker varsa kullanılır; yoksa **"Bu araç için WSL2/Docker gerekli — [Rehber/Kur]"** gösterilir. **Uygulamayı bloklamaz; yalnızca o konteyner-bağımlı araç açılır**, native yolla yapılabilen her şey çalışmaya devam eder.
  3. **Bulut/dağıtık (sonra):** İkisi de yoksa ileride ağ üzerinde çalıştırma (kanca İP-15; gerçek mantık dağıtık ağ eklentisinde — `MVP-sonrasi.md` §2).
- **Yerel hesap:** Kapsama (coverage), temel istatistik, bölge karşılaştırma, temel dizi hizalama; native Rust; sonuç görselleştirmeye + dışa aktarmaya gider. Bu yol harici bağımlılık istemez.
- **Harici araç köprüsü:** samtools, bcftools, BWA, minimap2, MAFFT vb. **opsiyonel konteyner/out-of-process** ile; arayüzden parametre; sonuç arayüze döner. Varyant çağırma harici araç köprüsüyle. Native Rust alternatifi olmayan araçlar için konteyner devreye girer.
- **Eksik araç / eksik runtime:** Araç yoksa "kur" butonu + otomatik konteyner/kurulum. Windows'ta konteyner runtime (WSL2/Docker) yoksa net rehber + [Kur]; kullanıcı yarı yolda kalmaz; native yolla yapılabilen işlemler etkilenmez (TDA).
- **Uzun iş:** Arka planda; ilerleme/iptal; bitince bildirim; gerekirse duraklat/sürdür; arayüz akıcı kalır (İP-08, İP-16). Ağır iş donanım korumaya uyar (İP-08; aşırı ısınmada kademeli yavaşlar).
- **Dışa aktarma:** İşlenen/filtrelenen veri VCF/BED/FASTA/CSV (ÇE-11).
- **Doğruluk:** Sonuçlar bilinen araçlarla golden test (İP-21).

**Dosya/Modül Yerleşimi:** `src/compute/{coverage.rs, stats.rs, compare.rs, bridge.rs, tools.rs, jobs.rs}`.

**TDA Kontrolleri:** Eksik araç/runtime → [Kur]/[Rehber] (1); native yol konteynersiz çalışır (Windows); uzun iş ilerleme/iptal/duraklat (3,12); hata anlamlı + çözüm (4); araç çıktısı doğrulanır.

**Kabul Kriterleri:**
- [ ] Kapsama/istatistik/bölge karşılaştırma **native** hesabı konteyner olmadan (Windows dahil) çalışır; sonuç görselleştirilir.
- [ ] En az samtools/bcftools köprüsü opsiyonel konteyner/out-of-process ile çalışır.
- [ ] Konteyner runtime yokken uygulama kilitlenmez; native işlemler etkilenmez; eksik runtime için rehber + [Kur] gösterilir.
- [ ] Uzun iş arka planda ilerleme/iptal ile; arayüz donmaz.
- [ ] Golden test ile sonuç doğruluğu (bilinen araçla karşılaştırma).

**Varsayımlar:** Tam pipeline (örn. uçtan uca varyant çağırma) iş akışı şablonlarıyla (İP-17) + node ile (ÇE-10) sonra; MVP'de temel hesap (native) + opsiyonel araç köprüsü. Ağır analiz hep out-of-process. **Varsayılan kurulum native yolla Windows'ta çalışır; konteyner yalnızca konteyner-bağımlı araçlar için gerekir.**
**Dikkat:** Tüm harici araç İP-07 sandbox/konteyner içinde (güvenlik); ağır iş İP-08 bütçesine + donanım korumaya uymalı. Konteyner **opsiyoneldir** — uygulamayı konteyner yokluğunda kilitleme; native yol her zaman çalışmalı.

---

### ÇE-09 — Veritabanı Erişimi: Birleşik Arama ve Tek-Tık Yükleme  **[MVP: TAM]**

**Amaç:** Kullanıcının yerel indirme yerine, arayüzden bilimsel veritabanlarını (BLAST/PDB/NCBI...) arayıp **tek tıkla** veriyi görselleştirmeye/projeye yükleyebilmesi.
**Kapsam:** Ortak veritabanı çerçevesi + kaynak konektörleri, birleşik arama paneli, sonuç listesi/önizleme, tek-tık yükleme, BLAST (uzak+yerel), önbellek, geçmiş, kimlik, gizlilik sınırı, rate limit, toplu indirme, provenance/atıf/lisans.
**Bağımlılıklar:** ÇE-00, ÇE-01 (yükleme/format), ÇE-02/ÇE-07 (görselleştirmeye/3B'ye gönderme), İP-07 (net capability/konektör), İP-09 (kimlik şifreleme), **İP-10 (PHI sınırı + provenance lisans alanı)**, İP-16.
**İlgili modül(ler):** `src/database/`.
**Teknoloji:** Rust/WASM çerçeve; ağ async (Tokio); ağır işleme out-of-process Python konektör; şifreli kimlik (OS anahtarlığı, İP-09).

**Somut Davranış/Spec:**
- **Kaynaklar:** Öncelik NCBI (nucleotide/protein/gene), BLAST, UniProt, PDB (3B), Ensembl, UCSC. **Çerçeve + kaynak başına konektör modülü**; yeni konektör güncellemeyle eklenir, çekirdek değişmeden.
- **Birleşik arama:** Tek panel — kaynak seç, sorgu (gen adı/dizi/accession/anahtar kelime), sonuç listesi, önizle, yükle. Tek kaynak/arama MVP'de yeterli; federe çoklu arama v1.x (`MVP-sonrasi.md` §6.1).
- **BLAST:** Uzak (NCBI BLAST API) varsayılan; opsiyonel **yerel BLAST+** (konteyner) hız/gizlilik için.
- **Sonuçlar:** Tablo + detay önizleme; sıralama/filtreleme (skor/e-değer/organizma); **sanal/sayfalı liste** (binlerce sonuç akıcı); arka plan yükleme.
- **Önizleme:** Yüklemeden önce özet (uzunluk/organizma/açıklama) — doğru kaydı seçmek için.
- **Tek-tık yükleme:** Seçilen kaydı indir + uygun formata çevir + **doğrudan** görselleştirme izine/3B görünümüne yükle (ara adım yok). 3B yapı → ÇE-07.
- **Çevrimdışı:** Durum net; önbellekteki veri sunulur; bağlantı gelince devam; yarı yolda bırakmaz.
- **Önbellek:** Yerel önbellek (boyut limiti + temizleme); aynı sorgu hızlı döner; önbellek izole proje alanında.
- **Geçmiş:** Son arama/yükleme kaydedilir, tekrar çalıştırılır, projeyle ilişkilendirilir.
- **Büyük indirme:** İlerleme/iptal/arka plan; çok büyükse uyarı+onay; akışlı/parçalı (resumable); accession listesiyle toplu indirme + aşırı yük koruması.
- **Kimlik:** API anahtarı gereken kaynaklar için **şifreli saklama** (OS anahtarlığı, İP-09); kullanıcı bir kez girer; NCBI API key opsiyonel (hız).
- **Rate limit:** Otomatik hız sınırlama + kuyruk; aşımda bekle/uyar; kullanıcı limiti görür.
- **Gizlilik:** Sorgu kullanıcı verisi içeriyorsa **uyarı** (örn. "dizi BLAST'a gönderiliyor"); ne paylaşıldığı şeffaf. **Hassas/PHI etiketli veri dış sorguya gönderilemez** (İP-10 çekirdek sınırı; eklenti aşamaz).
- **Provenance/atıf/lisans:** Her veri için kaynak/accession/sürüm/tarih; **atıf + lisans yükümlülüğü** gösterimi (akademik). Referans genom (hg38/hg19), dbSNP, ClinVar gibi setlerin lisans/kullanım koşulu ve atıf metni kaydedilir; proje köken kaydına işlenir (yöntem/teşekkür bölümü için). İP-10 provenance lisans alanıyla tutarlı.
- **Kaydetme:** Sonuç proje veri klasörüne + meta; proje taşınınca veri de gider.
- **API kırılması:** Konektör sürümlenir; API değişimi tespit + bildirim + konektör güncelleme; bozuk indirmede BLAKE3 bütünlük denetimi + yeniden indir.

**Dosya/Modül Yerleşimi:** `src/database/{framework.rs, search.rs, results.rs, load.rs, cache.rs, history.rs, credentials.rs, ratelimit.rs, privacy.rs, connectors/{ncbi.rs, blast.rs, uniprot.rs, pdb.rs, ensembl.rs, ucsc.rs}}`.

**TDA Kontrolleri:** Çevrimdışı durumu (11); yavaş API'de async + ilerleme/iptal/zaman aşımı (3,12,13); dış gönderim uyarısı + PHI engeli (gizlilik, İP-10); bozuk indirme net + yeniden (4); kimlik güvenli (İP-09); sonuçta atıf/lisans (güven).

**Kabul Kriterleri:**
- [ ] Birleşik panelden NCBI/PDB/UniProt vb. arama + sonuç listesi + önizleme çalışır.
- [ ] Tek-tık yükleme veriyi doğrudan görselleştirmeye/3B'ye getirir (ara adım yok).
- [ ] BLAST uzak (NCBI API) çalışır; yerel BLAST+ opsiyonu mevcut.
- [ ] PHI/hassas veri dış sorguya gönderilemez (test edilmiş, İP-10).
- [ ] Önbellek + geçmiş + rate limit + güvenli kimlik çalışır.
- [ ] Provenance + atıf/lisans her veri için kaydedilir (referans genom/dbSNP/ClinVar dahil).

**Varsayımlar:** Çevrimdışı veritabanı paketleri (yerel kopya) ve federe çoklu arama sonra (`MVP-sonrasi.md` §6.1, §6.2). Bio-kredi/ücretli kaynak bağlanışı `Hukuk-ve-Operasyon.md` + sonra. Konektör seti öncelik kaynaklarıyla başlar, toplulukla genişler.
**Dikkat:** PHI sınırı asla eklentiye emanet edilmez — çekirdek (İP-10) korur. Her dış çağrı net capability + kullanıcı şeffaflığı gerektirir.

---

### ÇE-10 — Node Entegrasyonu  **[MVP: TEMEL]**

> **MVP'de temel düzey:** Temel node seti (yükle/bölge seç/filtrele/görselleştir/dışa aktar/veritabanı ara) çalışır; ileri node'lar v1.x (`MVP-sonrasi.md` §4.5).

**Amaç:** Çekirdek eklentinin yetenekleri node editöründe de kullanılabilsin (görsel akış).
**Kapsam:** Veri/bölge/filtre/görselleştirme/dışa aktarma node'ları + veritabanı node'ları; İP-05 node sistemine SDK ile kayıt.
**Bağımlılıklar:** ÇE-00..ÇE-09, İP-05 (node motoru), İP-07 (SDK kayıt).
**İlgili modül(ler):** `src/nodes/`.
**Teknoloji:** `biocraft-sdk` node kayıt API'si (İP-05).

**Somut Davranış/Spec:**
- **Görselleştirme/analiz node'ları:** "dosya yükle", "bölge seç", "filtrele", "görselleştir", "dışa aktar", "kapsama hesapla", "hizala" node'ları; tipli portlar; akışta zincirlenir.
- **Veritabanı node'ları:** "NCBI ara", "BLAST çalıştır", "PDB getir", "veritabanı sorgusu" node'ları; sonuç sonraki node'lara veri sağlar.
- **Tutarlılık:** Node davranışı eklentinin GUI davranışıyla aynı sonucu üretir; parametre şeması açık.
- **Köprü:** Node sonucu görselleştirmeye, görselleştirme seçimi node'a (İP-05/İP-06 köprüsüyle).

**Dosya/Modül Yerleşimi:** `src/nodes/{viz_nodes.rs, analysis_nodes.rs, database_nodes.rs, register.rs}`.

**TDA Kontrolleri:** Yanlış port bağlanamaz (İP-05); node çalışması ilerleme/iptal (3,12); eksik araç/eklenti node'da da [İndir]/[Kur] (1); boş akış rehberi (5, İP-17).

**Kabul Kriterleri:**
- [ ] Görselleştirme/analiz/veritabanı node'ları node editörüne kaydedilir.
- [ ] Node'lar tipli portlarla akışta zincirlenir; sonuç doğru.
- [ ] Veritabanı sorgusu node'u akışa veri sağlar.
- [ ] Node sonucu = GUI sonucu (tutarlılık testi).

**Varsayımlar:** İleri node'lar (özel pipeline adımları) sürümlerle artar; MVP'de temel set. Şablon akışlar İP-17'de.
**Dikkat:** Node kaydı İP-05 SDK kontratına uymalı; node çalışması İP-08 bellek bütçesine.

---

### ÇE-11 — Dışa Aktarma ve Oturum  **[MVP: TAM]**

**Amaç:** Yayın kalitesi görsel + veri dışa aktarma + oturum/geçmiş kalıcılığı.
**Kapsam:** Yüksek çözünürlüklü görsel (PNG/SVG/PDF), veri dışa aktarma (VCF/BED/FASTA/CSV), görünüm/oturum durumu, eklenti içi geçmiş, temel rapor.
**Bağımlılıklar:** ÇE-02..ÇE-09, İP-11 (durum/proje).
**İlgili modül(ler):** `src/export/`.
**Teknoloji:** wgpu (yüksek çözünürlük render), SVG/PDF üretimi, Rust.

**Somut Davranış/Spec:**
- **Görsel:** Mevcut görünümü (genom tarayıcı/3B/plot) **yüksek çözünürlüklü PNG/SVG/PDF** olarak dışa aktar (yayın/sunum).
- **Veri:** İşlenen/filtrelenen veri VCF/BED/FASTA/CSV.
- **Oturum/durum:** Açık dosyalar/izler/bölge/ayarlar oturum/proje ile kaydedilir; eklenti içi geçmiş (son açılan dosya/bölge/işlem) → hızlı tekrar erişim. Açılışta tam geri yüklenir (İP-11).
- **Rapor:** MVP'de temel (görsel + özet + hangi sorgu/sonuç/tarih köken kaydı + kullanılan veri lisans/atıfı, yöntem bölümü için); tam otomatik PDF rapor v1.x (`MVP-sonrasi.md` §10.3).

**Dosya/Modül Yerleşimi:** `src/export/{figure.rs, data.rs, session.rs, history.rs, report.rs}`.

**TDA Kontrolleri:** Dışa aktarma ilerleme (3); oturum kalıcı + kurtarma (10, İP-11); boş geçmiş rehberi (5); dışa aktarma hatası net (4).

**Kabul Kriterleri:**
- [ ] Görünüm yüksek çözünürlüklü PNG/SVG/PDF dışa aktarılır.
- [ ] Filtreli veri VCF/BED/FASTA/CSV dışa aktarılır.
- [ ] Oturum/görünüm durumu proje ile kaydedilir/geri yüklenir.
- [ ] Eklenti içi geçmiş + temel rapor (köken + lisans/atıf kaydı) çalışır.

**Varsayımlar:** Tam otomatik yayın raporu (şablonlu PDF) v1.x. Paylaşılabilir oturum/proje (İP-02 export) ile.
**Dikkat:** Yüksek çözünürlük render bellek bütçesine uymalı (İP-08).

---

### ÇE-12 — Performans, Erişilebilirlik ve Doğruluk  **[MVP: TAM]**

**Amaç:** Eklentinin "IGV'den iyi/hızlı" iddiasını ölçülebilir kılmak + erişilebilirlik + bilimsel doğruluk güvencesi.
**Kapsam:** Out-of-core/LOD disiplini, bellek orkestratörü + donanım koruma uyumu, renk körü/erişilebilirlik, IGV benchmark, golden test (doğruluk), 3B ölçekleme.
**Bağımlılıklar:** ÇE-01..ÇE-09, İP-04 (LOD), İP-08 (bellek + donanım koruma), İP-21 (test/benchmark).
**İlgili modül(ler):** `src/perf/` + çapraz test.
**Teknoloji:** wgpu LOD, DuckDB/Arrow out-of-core, CPU SIMD fallback, benchmark/golden test (İP-21).

**Somut Davranış/Spec:**
- **Performans:** Tüm büyük veri out-of-core; genom tuvali/3B/yoğun iz GPU (wgpu) + CPU SIMD fallback; dosya açmadan İP-08 bütçe denetimi; 60 FPS hedefi (referans donanımda; düşük donanımda sadeleşme + uyarı). Ağır işler donanım korumaya uyar (İP-08).
- **Erişilebilirlik:** Renk körü dostu paletler; özelleştirilebilir renkler; renge ek görsel ipucu (şekil/desen); klavye/ekran okuyucu (Bölüm 0).
- **IGV benchmark:** Açılış süresi, pan/zoom FPS, bellek, büyük dosya işleme — IGV ile **yan yana** ölçüm; CI'de regresyon.
- **Doğruluk:** Bilinen veri setleriyle **golden test** (IGV/samtools/bcftools sonuçlarıyla karşılaştırma); sapma alarm verir.
- **3B ölçekleme:** Donanıma göre basitleştirme + uyarı; GPU yoksa CPU.

**Dosya/Modül Yerleşimi:** `src/perf/{lod.rs, accessibility.rs}`, `benches/` (IGV karşılaştırma), `tests/golden/` (doğruluk).

**TDA Kontrolleri:** Düşük donanımda sadeleşme + uyarı (11); erişilebilir renk/şekil; performans şeffaflığı (opsiyonel gösterge).

**Kabul Kriterleri:**
- [ ] Büyük genom/BAM/VCF out-of-core; referans donanımda 60 FPS.
- [ ] Renk körü dostu paletler + erişilebilirlik çalışır.
- [ ] IGV ile yan yana benchmark; CI regresyon eşiği yakalar.
- [ ] Golden test bilinen araç sonuçlarıyla doğruluğu doğrular.

**Varsayımlar:** GPU CI self-hosted runner sonra (İP-21, `MVP-sonrasi.md` §9.3). Tam erişilebilirlik denetimi yayın öncesi.
**Dikkat:** Doğruluk eklentinin güven temeli; golden test olmadan "gerçek veri" iddiası riskli.

---

## RAKİP PARİTESİ MATRİSİ

| Yetenek | IGV | JBrowse 2 | UCSC | Geneious/CLC | BioCraft Studio |
| --- | --- | --- | --- | --- | --- |
| Genom tarayıcı (çok-iz, pan/zoom) | ✓ | ✓ | ✓ | ✓ | ✓ (ÇE-02, gerçek 60 FPS) |
| BAM/CRAM hizalama görünümü | ✓ | ✓ | kısmi | ✓ | ✓ (ÇE-03, temel) |
| VCF varyant + genotip | ✓ | ✓ | ✓ | ✓ | ✓ (ÇE-04, DuckDB) |
| Anotasyon + iz yönetimi | ✓ | ✓ | ✓ | ✓ | ✓ (ÇE-05, temel) |
| Dizi düzenleme + MSA | kısmi | – | – | ✓ | ✓ (ÇE-06, temel) |
| 3B yapı (PDB) | – | – | – | ✓ | ✓ (ÇE-07, GPU/wgpu) |
| Harici araç köprüsü (samtools...) | kısmi | kısmi | – | ✓ | ✓ (ÇE-08, native + opsiyonel konteyner) |
| Entegre veritabanı arama + tek-tık | kısmi | – | ✓ (kendi) | kısmi | ✓ (ÇE-09, birleşik) |
| **Node tabanlı görsel akış** | – | – | – | kısmi | ✓ (ÇE-10) — **fark** |
| **Hibrit kod + görsel** | – | – | – | kısmi | ✓ (İP-06) — **fark** |
| **Yayın kalitesi dışa aktarma** | kısmi | kısmi | kısmi | ✓ | ✓ (ÇE-11) |
| **AI-hazır arayüz** | – | – | – | – | ✓ (İP-14) — **fark** |
| **Gerçek 60 FPS + modern GPU** | – | – | – | kısmi | ✓ (İP-04) — **fark** |
| **Donanıma zarar vermeyen (Zero-Impact)** | – | – | – | – | ✓ (İP-08) — **fark** |

> Fark özellikler: node akışı, gerçek 60 FPS, entegre veritabanı, hibrit kod+görsel, modern GPU 3B, AI-hazır, Zero-Impact donanım koruma. Pariteler golden test (doğruluk) + IGV benchmark (hız) ile kanıtlanır.

---

## KAPANIŞ — Çekirdek Eklenti

Bu belge, **BioCraft Studio** (temel kit — çekirdek analiz/görüntüleme/veritabanı eklentisi) için dondurulmuş tabandır. 13 paket (ÇE-00 → ÇE-12), her biri `Temel-Uygulama.md` Bölüm 0 + bu belgenin Bölüm 0-CE'si ile birlikte tek başına kodlanabilir.

**Önerilen sıra:** ÇE-00 → ÇE-01 → ÇE-02 → ÇE-04 → ÇE-07 → ÇE-09 → ÇE-11 → ÇE-12 (MVP'de TAM olanlar önce) → sonra ÇE-03 → ÇE-05 → ÇE-06 → ÇE-08 → ÇE-10 (TEMEL olanlar). (ÇE-09 veritabanı, ÇE-01 ve ÇE-02/ÇE-07'ye yaslanır; ondan önce onları kodlat.)

**Önkoşul:** Temel uygulamada en az İP-07 (host/SDK), İP-04 (render), İP-08 (bellek + donanım koruma), İP-05 (node), İP-10 (gizlilik), İP-11 (undo), İP-16 (TDA bileşenleri) hazır olmalı.

**MVP kapsam hatırlatması:** ÇE-03, ÇE-05, ÇE-06, ÇE-08, ÇE-10 **temel düzeydedir**; ileri özellikleri `MVP-sonrasi.md` §4'te açıklanır. Bu paketlerin mimarisi MVP'de kurulur, böylece ileride sökme gerekmez.

> **Not:** Bu eklenti motorla birlikte **varsayılan kurulu** gelir (İP-20). Kullanıcı ilk açılışta çalışan analiz/görüntüleme + veritabanı yeteneğiyle karşılaşır. En sık analiz işlemleri native Rust ile, konteyner olmadan (Windows dahil) çalışır.

Sıradaki dosya: **`AI-Altyapisi.md`** (MVP yüzeyi + gelecekteki motor mimarisi).
