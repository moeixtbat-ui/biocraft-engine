# BioCraft Engine — Hukuk ve Operasyon (Temel Uygulama Kapsamı)

> ## ⚠️ ÖNEMLİ UYARI — BU BELGE BAĞLAYICI HUKUKİ/MALİ TAVSİYE DEĞİLDİR
> Bu belge yalnızca **genel bilgilendirme ve planlama çerçevesidir.** Hukuki veya mali tavsiye niteliği taşımaz, bir avukatın veya mali müşavirin görüşünün yerine geçmez. Yasalar değişir, durumlar kişiye/şirkete özeldir.
> **Herhangi bir adım atmadan önce Türkiye'de yetkili bir avukat (özellikle bilişim/fikri mülkiyet hukuku) ve bir mali müşavir (SMMM/YMM) ile çalışman zorunludur.** Aşağıdaki maddeler, o görüşmelere hazırlıklı gitmen için bir kontrol listesi ve düşünme çerçevesidir.

> **Belge tipi:** Operasyonel/hukuki planlama çerçevesi (statik). **Kapsam:** Yalnızca **temel uygulama** (BioCraft Engine motoru + çekirdek eklenti dağıtımı). İleri ekonomi/pazar detayları gelecekte ayrıca ele alınır (`MVP-sonrasi.md` §10, §12).
> **Sürüm:** 1.2 · **Tarih:** 2026-06 · **Marka:** BioCraft Engine · biocraftengine.com
> **1.2 değişiklikleri:** Marka BioCraft olarak tutarlı (eski "BioForge" tarihçe notu Temel-Uygulama'da). Ertelenen operasyonel adımlar `MVP-sonrasi.md` §12'ye referanslandı. İçerik aynı dikkatli çerçevede korundu.

---

## BÖLÜM 0-H — KAPSAM VE TEMEL KARARLAR

| Alan | Karar (genel çerçeve — profesyonel onayı gerekir) |
| --- | --- |
| Marka | **BioCraft Engine** (eski "BioForge" düştü) — tescil/çakışma kontrolü **öncelikli** (Bölüm 2) |
| Lisans modeli | **Açık çekirdek + ticari katman** — **veri-güvenliği dahil çekirdek açık**; yalnızca premium + lisans/anti-tamper kapalı (Bölüm 1) |
| Şirket türü | Başlangıç **Limited (LTD)** → büyürse **A.Ş.** (Bölüm 5) |
| Şirketleşme zamanı | MVP doğrulandıktan, ticari gelir/yatırım/istihdam gündeme gelince (Bölüm 5) |
| Ticari model | Freemium + abonelik + pazar komisyonu + Bio-kredi (Bölüm 6) |
| İçerik sorumluluğu | Haber/pazar/kullanıcı içeriği için bildir-kaldır + moderasyon (Bölüm 4-A) |
| Veri sahipliği | **Kullanıcının** (veri egemenliği); şirket yalnızca izinli işler (Bölüm 3) |
| Veri konumu | Varsayılan tamamen yerel; bulut/paylaşım opt-in (Bölüm 3) |

> **En kritik ilk adım:** "BioCraft" adının marka/domain müsaitliğini doğrulamak (Bölüm 2). Marka çakışması lansman sonrası isim değişimine zorlarsa felakettir.

---

## BÖLÜM 1 — LİSANS STRATEJİSİ (Açık Çekirdek + Ticari Katman)

**Genel çerçeve:**
- **Çekirdek motor: AGPLv3.** Güçlü copyleft; kodu alıp kapalı ticari ürün yapan, kendi kaynağını da açmak zorunda kalır (ticari koruma + topluluk dostu). Açık çekirdek topluluğu projeyi sürdürülebilir kılar.
- **SDK / eklenti arayüzü: izin verici lisans (örn. Apache-2.0).** Üçüncü partilerin (ticari dahil) eklenti geliştirmesini kolaylaştırır; ekosistem büyür. _AGPLv3 ile birlikte kullanımın uyumu hukukçuyla netleştirilmeli._
- **Premium/ticari katman: ayrı ticari lisans.** Kapalı ticari özellikler ve kurumsal sürümler ayrı, ticari lisansla (açık kaynak yükümlülüğü dışında).
- **Bağımlılık lisansları:** `cargo-deny` ile politika (Temel-Uygulama İP-00/İP-09); AGPL ile uyumsuz/çakışan bağımlılıklar CI'da reddedilir.

**Açık/Kapalı sınırı (KRİTİK İLKE — denetlenebilirlik = güven):**
- **Açık kaynak (çekirdek):** Motor + **veri-koruma güvenliğinin tamamı** (şifreleme, anahtar yönetimi, sandbox, capability, PHI sınırı, gizlilik), dosya/proje formatları, SDK ve özelliklerin büyük çoğunluğu. Hassas (genomik/sağlık) veri emanet eden kullanıcı için güven, güvenlik kodunun **denetlenebilir** olmasından gelir (Kerckhoffs ilkesi: güvenlik gizlilikle değil, açık + sağlam tasarımla sağlanır). Güvenlik/şifreleme kodunu kapatmak güveni **azaltır**; bu nedenle veri-güvenlik **açıktır.**
- **Kapalı kaynak (ticari koruma):** Yalnızca premium/kurumsal özellikler + **lisans/aktivasyon + anti-tamper/anti-korsanlık** katmanı + (opsiyonel) tescilli performans optimizasyonları. Bunların amacı **şirketi** korumaktır, kullanıcı verisini değil. Anti-tamper katmanının kapalı olması meşrudur (amacı zaten tersine mühendisliğe direnmektir).

**AGPL mimari temizliği (avukatla netleştir):**
- Kapalı parçalar AGPLv3 çekirdeğe **statik bağlanmaz**; SDK/IPC sınırı üzerinden **ayrı süreç/eklenti** olarak çalışır (host eklentileri zaten out-of-process/WASM çalıştırır — Temel-Uygulama İP-07). Bu, kapalı katmanın türev eser sayılma riskini azaltan standart yaklaşımdır — **yine de hukuken tartışmalıdır; avukat onayı şart.**
- **Telif sahipliği sende kalmalı (CLA):** Katkıda bulunanlardan **Katkı Lisans Sözleşmesi (CLA)** alınırsa, ticari katmanı **çift-lisanslayabilirsin** (AGPL + ticari). Telif birçok kişiye dağılırsa bu imkân kaybolur.
- **AGPL madde 13 (ağ kullanımı) — ÖNEMLİ:** AGPLv3, yazılımla **ağ üzerinden** etkileşen kullanıcılara kaynak kodu sunmayı gerektirir. Herhangi bir sunucu/bulut bileşeni (BioCraft Market backend'i, haber sunucusu, AI bulut proxy'si, dağıtık ağ koordinatörü) AGPL çekirdek kodunu çalıştırıyorsa, o sunucunun kaynağını sunma yükümlülüğü doğabilir. Sunucu tarafını **ayrı / kendi telifinle** tasarla; avukatla netleştir.
- **Hafif alternatif — MPL-2.0:** AGPLv3, "kapalı eklentili açık-çekirdek" modeli için fazla agresif kalabilir. **MPL-2.0** (dosya bazlı copyleft) kapalı kodla birleşmeye daha müsaittir ve madde 13 ağ yükünü taşımaz; ancak rakibin tüm projeyi kapatmasına karşı AGPL kadar güçlü koruma sağlamaz. Bu denge (güçlü copyleft koruması ↔ kapalı-eklenti esnekliği) avukatla konuşulmalı. _(Şimdilik plan: AGPLv3 çekirdek.)_

**Topluluk ↔ ticari denge:** Şeffaf yönetişim; çekirdek açık ve topluluk-dostu kalır, ticari katman net biçimde ayrılır. Açık çekirdek, projenin uzun-vade sürekliliğini (proje ölmez) garanti eder; ticari taraf büyütülebilir/devredilebilir.

**Lisans ihlali:** Lisans şartları net yazılır; AGPLv3 ihlali (kaynak açmama) tespit edilip orantılı şekilde (uyarı → hukuki yol) takip edilir.

> **Avukata sor:** AGPLv3 + Apache-2.0 SDK + ticari katman kombinasyonunun uyumu; **AGPL madde 13'ün sunucu bileşenlerine etkisi**; kapalı parçaların ayrı-süreç mimarisinin türev-eser açısından yeterliliği; **MPL-2.0'ın alternatif olarak uygunluğu**; ticari lisans metni; CLA gerekliliği ve çift-lisanslama.

---

## BÖLÜM 2 — FİKRİ MÜLKİYET (IP)

**Marka — ÖNCELİKLİ:**
- "BioCraft Engine" / "BioCraft" adı ve logosu için **tescil/kullanım kontrolü**: Türkiye (TÜRKPATENT) + uluslararası (WIPO/Madrid, hedef pazarlar). "craft" isimlendirmesi yazılım ve biyoteknolojide yaygın olduğundan **çakışma riski ciddi araştırılmalı.**
- Domain (biocraftengine.com) + sosyal hesaplar lansmandan önce kilitlenmeli (squatting önlemi).
- Çakışma çıkarsa: tescilden ve markaya yatırımdan **önce** alternatif değerlendirilmeli (sonradan isim değişimi maliyetlidir).

**Diğer IP:**
- **Telif (kod):** Üretilen kod telifle korunur; açık kaynak lisansı bunu düzenler (Bölüm 1).
- **Ticari sır:** Yalnızca **kapalı premium katman + lisans/anti-tamper** kodu ticari sır + native derleme ile korunur (Temel-Uygulama İP-09). **Güvenlik/şifreleme kodu ticari sır DEĞİLDİR — açıktır ve denetlenebilir** (güven için; Bölüm 1 açık/kapalı sınırı).
- **Üçüncü parti veri lisansları:** Referans genom (hg38/hg19), dbSNP, ClinVar, UniProt, PDB gibi bilimsel veri setlerinin **kendi lisans/atıf koşulları** vardır. Ürün bunları provenance'ta kaydeder ve kullanıcıya gösterir (Temel-Uygulama İP-10, Çekirdek ÇE-09). Ticari dağıtım/paketlemede bu veri lisanslarının uyumu kontrol edilmeli (bazıları ticari kullanımı veya yeniden dağıtımı kısıtlayabilir).
- **Patent:** Gerçekten yeni/patentlenebilir bir yöntem varsa değerlendirilir; çoğu yazılım için telif + marka yeterli olabilir — hukukçu yönlendirsin.

> **Avukata sor:** Marka araması + başvuru stratejisi (önce hangi sınıflar/ülkeler); logonun telif/tasarım tescili; ticari sır koruma sözleşmeleri; **bilimsel veri setlerinin lisans uyumu** (ticari dağıtımda).
> **İstersen ben şimdi web'de "BioCraft" için ön bir marka/domain çakışma taraması yapabilirim** (profesyonel hukuki aramanın yerine geçmez, sadece erken sinyal).

---

## BÖLÜM 3 — KVKK / GDPR UYUMU VE VERİ YÖNETİMİ

> Teknik uygulama Temel-Uygulama İP-10 (Gizlilik) + İP-09 (Güvenlik)'te. Bu bölüm hukuki/operasyonel tarafı çerçeveler.

**Temel ilkeler (privacy by design):**
- **Veri minimizasyonu:** Yalnızca işlev için gerekli veri.
- **Açık rıza:** Her dış işlem (telemetri/AI/paylaşım) açık onayla; varsayılan yerel/kapalı.
- **Haklar:** Erişim, dışa aktarma (taşınabilirlik), silme (unutulma hakkı — güvenli, geri-döndürülemez), düzeltme.
- **Veri egemenliği:** Kullanıcı verisi **kullanıcınındır**; şirket yalnızca izinli işler; ToS'ta açık.
- **Çevrimdışı = tam gizlilik:** Çevrimdışı hiçbir veri dışarı gitmez.
- **Asla paylaşılmaz:** PHI/klinik kimlik, ham hassas veri, kimlik bilgileri — veri sınıflandırmasıyla kesin engel (İP-10).

**Şirket yükümlülükleri:**
- **VERBİS** (Veri Sorumluları Sicili) kaydı (gerekiyorsa).
- Veri işleme envanteri; açık rıza metinleri; **ihlal bildirim süreci** (yasal süre içinde).
- Gerekirse **Veri İşleme Sözleşmesi (DPA)** (üçüncü parti/bulut sağlayıcılarla).
- Üçüncü parti eklenti veri toplama beyanı + kullanıcı onayı.

**Özel nitelikli veri (sağlık/klinik):**
- Sağlık verisi KVKK madde 6 kapsamında **özel nitelikli**; ek koruma/açık rıza/onay gerekir.
- **Klinik kullanım** ek düzenleyici onaylar (tıbbi cihaz/yazılım mevzuatı) gerektirebilir — bu ciddi bir alandır, **mutlaka uzman hukukçu.** (Ürün konumlandırması: "araştırma/bilgilendirme amaçlı, klinik karar için değil" — Bölüm 4/10.)
- **Çocuk/öğrenci verisi:** Eğitim bağlamında ek koruma; minimizasyon; gerekirse ebeveyn/kurum onayı.

**Uluslararası veri:** AB (GDPR), ABD vb. hedef pazarların veri yasaları; sınır-ötesi veri aktarımı kuralları (`MVP-sonrasi.md` §11).

> **Avukata sor:** VERBİS gerekliliği; rıza metinleri; DPA şablonları; sağlık verisi işlenecekse tüm ek yükümlülükler ve klinik kullanım mevzuatı.

---

## BÖLÜM 4 — KULLANIM KOŞULLARI (ToS) VE GİZLİLİK SÖZLEŞMESİ

**Gerekli belgeler — hukukçu hazırlamalı:**
- **Kullanım Koşulları (ToS):** Kullanım hakları/sınırları, sorumluluk sınırlama (Bölüm 10), veri sahipliği (kullanıcının), lisans atıfları, içerik sorumluluğu (Bölüm 4-A).
- **Gizlilik Politikası:** Açık, sade dilde; hangi veri nasıl işlenir, kullanıcı hakları, iletişim. İlk kurulumda özet + detaya erişim; değişiklikte bildirim.
- **Veri İşleme Sözleşmesi (DPA):** Kurumsal/üçüncü parti senaryolarında gerekebilir.

**Kritik maddeler:**
- **Sorumluluk feragati:** "Araç bilgilendirme/araştırma amaçlıdır, klinik/teşhis kararı için değildir" — net feragat. Yanlış analiz sonucu sorumluluğu sınırlama.
- **Veri sahipliği:** Kullanıcı verisi kullanıcınındır; şirket yalnızca izinli işler.
- **Lisans bağlantısı:** Açık kaynak (AGPLv3) + premium ticari lisans atıfları (Bölüm 1).

**Gizlilik ihlali mekanizması:** Şeffaf rapor kanalı + veri erişim/silme + ihlal bildirimi + hızlı müdahale.

> **Avukata sor:** ToS + Gizlilik Politikası + (gerekirse) DPA metinleri; sorumluluk sınırlama maddelerinin Türk hukukunda geçerliliği; feragatin kapsamı.

---

## BÖLÜM 4-A — İÇERİK SORUMLULUĞU (Haber Akışı, Pazar, Kullanıcı İçeriği)

> Temel-Uygulama İP-18 (haber akışı + BioCraft Market) ve İP-01 (launcher haberleri) içerik **barındırır/dağıtır.** Bu, aracı (host) hukuki sorumluluğu doğurabilir.

**Haber/yorum içeriği (yanlış bilgi riski):**
- Küratörlü haber akışı + **opsiyonel çok-AI çapraz kontrol** bir **doğruluk garantisi vermez** (İP-18; AI YZ-08). AI'lar ortak önyargı paylaşıp birlikte yanılabilir; özellik "öneri / daha yüksek güven sinyali, kesin doğruluk değil" olarak sunulur. ToS'ta: gösterilen bilgilerin doğruluğu garanti edilmez, bilimsel/klinik karar için değildir.
- Üçüncü parti kaynaklara/dış bağlantılara yönlendirmede sorumluluk sınırlaması + kaynak şeffaflığı.

**Kullanıcı tarafından üretilen içerik (eklenti/şablon/yorum — BioCraft Market):**
- Kullanıcıların yüklediği eklenti/şablon/yorum için **bildir-kaldır (notice-and-takedown)** prosedürü, moderasyon, kötü/yasadışı içerik raporlama (İP-18 ile teknik altyapı mevcut).
- Eklenti imzası + "doğrulanmış/resmi" rozeti + geliştirici kimliği → sorumluluk izlenebilirliği. İmzasız/3. parti içerik net uyarıyla işaretlenir.
- Zararlı/kötü amaçlı eklenti riski: capability/sandbox sınırı (İP-07/İP-09) + raporlama + yaptırım.

**Telif/fikri mülkiyet (üçüncü parti):**
- Pazarda paylaşılan içerik üçüncü parti telifini ihlal ederse **takedown** süreci; tekrar eden ihlalde hesap yaptırımı.
- Kullanıcı, yüklediği içeriğin haklarına sahip olduğunu/dağıtım iznine sahip olduğunu **beyan eder** (ToS).

**Sorumluluk sınırı:** Platform, kullanıcı içeriğinin doğruluğundan/yasallığından birincil sorumlu olmamakla birlikte, **etkin bir bildir-kaldır + moderasyon** işletmek zorundadır (aracı sorumluluğu rejimine göre). Hedef pazara göre farklı rejimler geçerlidir (örn. AB Dijital Hizmetler Yasası — DSA, ABD ilgili düzenlemeleri, Türk mevzuatı).

> **Avukata sor:** Aracı (host) sorumluluk rejimi (TR + hedef pazarlar); bildir-kaldır prosedürünün hukuki gereklilikleri; kullanıcı içeriği + telif ToS maddeleri; **AI-üretilen/AI-desteklenen içerik** için sorumluluk; haber akışı yanlış bilgi sorumluluğu.

---

## BÖLÜM 5 — ŞİRKETLEŞME, VERGİ VE TEŞVİKLER

**Ne zaman:** MVP'yi açık kaynak/kişisel başlatıp **doğrula**; ticari gelir, yatırım veya istihdam gündeme gelince şirketleş (genellikle ilk gelir/yatırım öncesi). Önce ürün/traction (bootstrapping), acele yatırım değil. (`MVP-sonrasi.md` §12.)

**Şirket türü — mali müşavirle netleştir:**
- Başlangıç: **Limited (LTD)** — esnek, düşük maliyet, hızlı kuruluş.
- Büyüme/yatırım/ortaklık: **Anonim (A.Ş.)**'ye geçiş (hisse devri/yatırım için uygun).

**Vergi/muhasebe:** Mali müşavirle **kuruluştan itibaren** düzenli muhasebe. Yazılım/Ar-Ge teşviklerini değerlendir.

**Devlet desteği/teşvik:**
- **Teknokent (Teknopark):** Yazılım Ar-Ge için ciddi vergi avantajları (gelir/kurumlar vergisi istisnası, vb.).
- **TÜBİTAK / KOSGEB:** Yazılım Ar-Ge hibe/destek programları.
- Başvurular için danışmanlık almak verimli olur.

**Çalışan/ortak:** İş/danışmanlık sözleşmesi + **IP devir maddesi** (üretilen kod şirkete ait) + gizlilik (NDA). Solo başlasan bile ileride katkıda bulunan/ortak gelirse bu baştan netleşmeli. _(Açık kaynak katkıları için CLA — Bölüm 1.)_

> **Mali müşavire sor:** LTD kuruluş süreci/maliyeti; Teknokent/Ar-Ge teşvik uygunluğu ve başvurusu; muhasebe düzeni; vergi yükümlülükleri.
> **Avukata sor:** Kuruluş sözleşmesi; ortak/çalışan sözleşmeleri + IP devri + NDA + CLA.

---

## BÖLÜM 6 — TİCARİ MODEL

**Çok kanallı gelir:**
- **Ücretsiz çekirdek (freemium):** Temel uygulama + çekirdek eklenti ücretsiz (açık kaynak); benimseme için.
- **Premium özellik / abonelik:** Gelişmiş özellikler, kurumsal yetenekler, öncelikli destek (kapalı katman — Bölüm 1).
- **Pazar komisyonu:** BioCraft Market'te ücretli eklenti/şablon satışından komisyon.
- **Bio-kredi:** Platform-içi kullanım birimi (örn. AI/bulut hesaplama).
- **Gelecekte:** Kurumsal/bulut sürümler (`MVP-sonrasi.md` §10.1).

**Bio-kredi yasal konumlandırma — DİKKAT:**
- Bio-kredi'yi **"kripto para" DEĞİL**, platform-içi kredi/puan olarak tasarla. Para/menkul kıymet çağrışımından kaçın (yasal karmaşıklık azalır).
- Gerçek para girişi/çıkışı, geri ödeme, transfer edilebilirlik gibi özellikler **ciddi finansal mevzuat** doğurabilir — bu özellikler eklenmeden önce **mutlaka hukukçu.**
- MVP'de Bio-kredi yalnızca **yer tutucu/kanca** (Temel-Uygulama İP-18, AI YZ-06); gerçek ödeme akışı yok (`MVP-sonrasi.md` §2.2).

> **Avukata sor:** Bio-kredi'nin yasal niteliği (ödeme aracı/puan); ödeme entegrasyonu yapılırsa mevzuat (BDDK/ödeme hizmetleri); pazar komisyonu sözleşmeleri.
> **Mali müşavire sor:** Gelir kanallarının vergilendirilmesi; abonelik/komisyon muhasebesi.

---

## BÖLÜM 7 — DAĞITIM, AKTİVASYON VE LİSANS ZORLAMA

> Teknik taraf Temel-Uygulama İP-20 (Paketleme/Güncelleme) + İP-09 (İmza/Güvenlik)'te.

- **Dağıtım kanalları:** Kendi sitesi (biocraftengine.com) + uygulama mağazaları (Microsoft Store vb.); imzalı binary (kod imzalama sertifikası gerekir — maliyet/süreç; tüzel kişilik zamanlamasıyla planla, Bölüm 5).
- **Premium aktivasyon:** Ticari/premium katman için lisans aktivasyonu/anahtarı (Temel-Uygulama İP-20; kapalı katman); açık kaynak çekirdek serbest.
- **AGPLv3 zorlama:** Açık kaynak ihlali (kaynak açmama) tespit + uyarı + orantılı hukuki yol. Madde 13 (ağ) yükümlülüğü için Bölüm 1.
- **Mağaza politikaları:** Her mağazanın kendi kuralları/komisyonu/veri politikası; uyum gerekir.
- **İhracat kontrolü (kriptografi):** Şifreleme içeren yazılımın ihracatı bazı ülkelerde kontrole tabi; hedef pazarlara göre değerlendir. _(Not: açık kaynak kriptografi bazı yargı bölgelerinde daha hafif muameleye tabi olabilir, ancak yine de kontrol edilmelidir.)_

> **Avukata sor:** Kod imzalama/dağıtım sözleşmeleri; mağaza şartlarının uyumu; ihracat kontrolü (şifreleme) yükümlülükleri.

---

## BÖLÜM 8 — ULUSLARARASI GENİŞLEME

**Çerçeve:** Kademeli uluslararasılaşma; her hedef pazarın kendi kuralları. _(Ürün global/EN-öncelikli; şirket/hukuk TR temelli — Temel-Uygulama Bölüm 0 i18n.)_ Detaylı erteleme: `MVP-sonrasi.md` §11.
- **Veri yasaları:** AB → GDPR (KVKK'ya benzer ama ek yükümlülükler), ABD → eyalet/sektör bazlı; sınır-ötesi veri aktarımı kuralları.
- **İçerik/aracı sorumluluğu:** AB → DSA, hedef pazarlara göre değişen rejimler (Bölüm 4-A).
- **İhracat kontrolü:** Şifreleme içeren yazılım bazı ülkelerde ihracat kontrolüne tabi (Bölüm 7).
- **Vergi:** Uluslararası satışta KDV/dijital hizmet vergisi vb.; mali müşavirle.
- **Marka:** Hedef pazarlarda marka tescili (Bölüm 2).

> **Avukat/mali müşavire sor:** Hangi pazara açılırken hangi uyum; uluslararası vergi; veri aktarım mekanizmaları; aracı sorumluluğu rejimleri.

---

## BÖLÜM 9 — SÜRDÜRÜLEBİLİRLİK, YÖNETİŞİM VE ÇIKIŞ

**Yönetişim:** Şeffaf yönetişim; çekirdek açık ve topluluk-dostu, ticari katman net ayrı. Katkıda bulunan sözleşmesi (CLA) ve karar süreçleri baştan tanımlı olursa topluluk güveni korunur (CLA aynı zamanda çift-lisanslamayı mümkün kılar — Bölüm 1).

**Sürdürülebilirlik/çıkış:** Uzun vade düşün. **Açık çekirdek**, projenin sürekliliğini sağlar (şirket bir şekilde sonlansa bile topluluk projeyi sürdürebilir — proje ölmez). Ticari taraf büyütülebilir veya devredilebilir; esnek strateji. Bu hem kullanıcı güveni hem yatırımcı çekiciliği açısından değerli.

---

## BÖLÜM 10 — SORUMLULUK, RİSK VE SİGORTA

**Çerçeve — hukukçu yazmalı:**
- **Sorumluluk sınırlama:** ToS'ta net sorumluluk sınırlama maddeleri (Bölüm 4).
- **Klinik/teşhis feragati:** "Araç bilgilendirme/araştırma amaçlıdır; klinik, teşhis veya tedavi kararı için kullanılamaz." Bu, sağlık/biyoinformatik alanında **kritik**. AI çıktısı da yalnızca araştırma/Ar-Ge/fikir amaçlıdır (AI YZ-08).
- **Sigorta:** Mesleki sorumluluk/ürün sorumluluğu sigortası değerlendirilebilir (özellikle kurumsal/sağlık kullanımı büyürse).
- **Yanlış sonuç riski:** Yazılımın ürettiği analiz hatalı olabilir; doğrulama kullanıcı sorumluluğundadır (ürün içi "öneri/doğrulanmalı" uyarılarıyla tutarlı — AI YZ-08, Çekirdek ÇE-12 doğruluk testleri). Çok-AI uyumu bile doğruluk garantisi değildir (Bölüm 4-A).

> **Avukata sor:** Sorumluluk sınırlama maddelerinin geçerliliği; feragat kapsamı; sigorta ihtiyacı.

---

## BÖLÜM 11 — YAYIN SONRASI HUKUKİ BAKIM

**Çerçeve:**
- **Periyodik gözden geçirme:** ToS / Gizlilik Politikası / lisans metinleri / içerik politikalarını güncel tut; yasa değişiminde uyumu yenile.
- **Sürekli ilişki:** Avukat ve mali müşavirle sürekli/düzenli ilişki (tek seferlik değil).
- **Değişiklik bildirimi:** Sözleşme/politika değişiminde kullanıcıya bildirim (Bölüm 4).

---

## BÖLÜM 12 — ÖNCELİKLİ AKSİYON LİSTESİ (SIRALI)

> Hepsi profesyonel destekle yapılır. Sıra, mantıksal önceliğe göredir.

1. **Marka/domain çakışma kontrolü ("BioCraft")** — TÜRKPATENT + uluslararası ön arama; domain + sosyal hesap kilitleme. _(Bölüm 2 — en kritik, lansmandan önce.)_
2. **Lisans kararını sabitle** — AGPLv3 çekirdek + Apache-2.0 SDK + ticari lisans; **madde 13 (sunucu) + ayrı-süreç mimarisi + CLA/çift-lisans + MPL-2.0 değerlendirmesi**; bağımlılık politikası (cargo-deny). _(Bölüm 1 — hukukçu onayı.)_
3. **Açık/kapalı sınırını netleştir** — veri-güvenliği açık; yalnızca premium + lisans/anti-tamper kapalı; mimari ayrım. _(Bölüm 1.)_
4. **Temel hukuki metinler** — ToS + Gizlilik Politikası taslağı (hukukçu). _(Bölüm 4.)_
5. **İçerik sorumluluğu** — haber/pazar/kullanıcı içeriği için bildir-kaldır + moderasyon + ToS maddeleri. _(Bölüm 4-A.)_
6. **KVKK hazırlığı** — VERBİS gerekliliği, rıza metinleri, ihlal süreci; sağlık verisi işlenecekse uzman. _(Bölüm 3.)_
7. **Şirketleşme zamanlaması** — MVP doğrulanınca LTD kuruluşu (mali müşavir); Teknokent/Ar-Ge teşvik değerlendirmesi. _(Bölüm 5.)_
8. **Ticari model + Bio-kredi konumlandırma** — gerçek ödeme öncesi hukuki görüş. _(Bölüm 6.)_
9. **Çalışan/ortak/katkıcı gelirse** — sözleşme + IP devri + NDA + CLA. _(Bölüm 5.)_
10. **Sorumluluk/feragat metinleri** — klinik feragati dahil. _(Bölüm 10.)_
11. **Bilimsel veri lisans uyumu** — referans genom/dbSNP/ClinVar vb. ticari dağıtımda. _(Bölüm 2.)_
12. **Uluslararası açılımda** — hedef pazar uyumu (veri + içerik + ihracat). _(Bölüm 8.)_
13. **Sürekli hukuki/mali bakım** — periyodik gözden geçirme. _(Bölüm 11.)_

---

## KAPANIŞ

Bu belge, BioCraft Engine **temel uygulamasının** hukuki/operasyonel **planlama çerçevesidir** — bir kontrol listesi ve düşünme zeminidir.

> ## ⚠️ TEKRAR — BAĞLAYICI DEĞİLDİR
> Buradaki hiçbir madde hukuki veya mali tavsiye değildir ve profesyonel görüşün yerine geçmez. **Herhangi bir adımı atmadan önce Türkiye'de yetkili bir avukat (bilişim/fikri mülkiyet/sağlık hukuku) ve bir mali müşavir ile çalış.** Özellikle: marka tescili, lisans metinleri (AGPL madde 13 + açık/kapalı sınırı + MPL alternatifi), KVKK/sağlık verisi, içerik/aracı sorumluluğu, Bio-kredi/ödeme, sorumluluk feragati ve şirketleşme — bunların her biri uzman gerektirir.

İstersen ilk ve en kritik adım olarak **"BioCraft" marka/domain çakışma ön taramasını** web'de şimdi yapabilirim (yalnızca erken sinyal; profesyonel hukuki aramanın yerine geçmez).
