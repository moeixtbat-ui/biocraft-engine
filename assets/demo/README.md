# BioCraft Engine — Gömülü Demo Veri Setleri (İP-17)

Bu klasördeki dosyalar, ilk kullanıcı deneyimi (onboarding) için **kutudan çıkan**
küçük örnek verilerdir. "Demo Projeyi Aç" özelliği bu dosyaları kullanır, böylece
kullanıcı kendi verisi olmadan da motoru **boş ekranla karşılaşmadan** deneyebilir.

## Köken / Provenance (ÖNEMLİ)

- **Kaynak:** Tamamı **BioCraft tarafından üretilmiş sentetik** içeriktir.
- **Lisans:** CC0-1.0 (kamu malı eşdeğeri) — sentetik veri; telif/atıf yükümlülüğü yoktur.
- **Gerçek hasta/birey verisi DEĞİLDİR (MK-42).** Hiçbiri PHI/hassas değildir; hepsi
  `Sentetik` sınıfında ele alınır. Gerçek hasta verisi **asla** depoya girmez (CLAUDE.md §7).
- Dosyalar derleme zamanında ikiliye **gömülür** (`include_str!`); çalışmak için indirme/ağ gerekmez.

Bu köken bilgisi kod tarafında her demo veriye iliştirilir (`onboarding::templates::DemoVeri`)
ve gerçek bir proje oluşturulduğunda `koken.jsonl`'e yazılabilir (İP-10 provenance ile tutarlı).

## Dosyalar

| Dosya | Biçim | İçerik (sentetik) |
|-------|-------|-------------------|
| `mini.fasta` | FASTA | 3 kısa örnek dizi |
| `mini.vcf`   | VCF v4.2 | 5 örnek varyant (2 kromozom) |
| `mini.sam`   | SAM (BAM'in metin biçimi) | 3 örnek hizalanmış okuma |
| `mini.pdb`   | PDB | Sentetik 3-kalıntılı mini peptit (gerçek yapı değil) |

> Not: BAM ikili bir biçimdir; demo amacıyla okunabilir/denetlenebilir **SAM** metin biçimi
> tercih edilmiştir (aynı bilgi, telif/ikili-dosya riski yok).
