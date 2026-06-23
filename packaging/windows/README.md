# Windows paketleme (MSIX / Squirrel)

## MSIX (önerilen)

```powershell
# Test (yerel kendinden-imzalı sertifika; yalnız bu makine):
./build-msix.ps1 -Version 0.1.0 -SelfSign

# Üretim (gerçek kod-imzalama sertifikası — insan-eli):
./build-msix.ps1 -Version 0.1.0 -CertPath C:\secrets\biocraft.pfx -CertPassword (Read-Host -AsSecureString)
```

Gereksinimler: Windows 10/11 SDK (`makeappx`, `signtool`). CI'da `release.yml` bunu otomatik çağırır.

`AppxManifest.xml` içindeki `Publisher` (CN=…) sertifikanın özne adıyla **birebir** eşleşmelidir;
aksi hâlde Windows paketi reddeder.

## Squirrel / Velopack (alternatif)

MSIX dağıtım kısıtları (kurumsal/yan-yükleme) sorun olursa **Velopack** (modern Squirrel) ile
`.exe` installer + delta güncelleme üretilebilir. API sözleşmesi aynıdır; updater motorumuz
(`biocraft_data::update`) zaten delta + atomik + geri-alma sağlar, yalnız kabuk (installer/runner)
değişir. Bu MVP'de MSIX birincil; Velopack değerlendirilen alternatiftir (ARCHITECTURE MK-56).

## "Bilinmeyen yayıncı" uyarısı

İmzasız ya da yalnız kendinden-imzalı paketlerde Windows "bilinmeyen yayıncı" der. Çözüm: tüzel
kişilik adına alınmış **kod-imzalama (Authenticode) sertifikası** (insan-eli;
`docs/specs/Hukuk-ve-Operasyon.md`). Sertifika gelene dek test için `-SelfSign` kullanın; bu paket
**dağıtılmaz**.

## Çevrimdışı / kurumsal kurulum

- **Çevrimdışı:** Üretilen `.msix` tek dosyadır; internet olmadan
  `Add-AppxPackage .\BioCraftEngine-<sürüm>-x64.msix` ile kurulur. Çekirdek eklenti (MK-19) içinde
  gömülü olduğundan ilk açılışta hazırdır (ağ gerekmez).
- **Kurumsal sessiz kurulum:** `DISM /Online /Add-ProvisionedAppxPackage` ya da Intune/SCCM ile
  toplu, etkileşimsiz dağıtım. Hava-boşluklu (air-gapped) ortamda da `.msix` + çevrimdışı `.bcext`
  eklentiler elle kopyalanıp kurulur.
- **`.bcext` çevrimdışı eklenti:** Mağazaya erişmeden eklenti kurmak için `.bcext` dosyası
  uygulamadan (Eklenti → Çevrimdışı kur) açılır; imza/bütünlük host (İP-07) tarafından doğrulanır.
