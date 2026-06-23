<#
.SYNOPSIS
  BioCraft Engine — Windows MSIX kurulum paketi üretir (İP-20, MK-56, MK-19).

.DESCRIPTION
  1. Release binary'sini (biocraft.exe) toplar.
  2. Çekirdek eklentiyi (MK-19) pakete gömer (core-plugin/).
  3. AppxManifest.xml + varlıkları yerleştirir.
  4. MSIX paketler (makeappx) ve imzalar (signtool).

  Kod-imzalama sertifikası İNSAN-ELİ iştir (Hukuk-ve-Operasyon.md). Sertifika gelene dek
  -SelfSign ile yerel test sertifikası üretilir (yalnız bu makinede güvenilir; DAĞITILMAZ).

.PARAMETER Version
  SemVer sürüm (örn. 0.1.0). Manifest'e a.b.c.0 olarak yazılır.

.PARAMETER CertPath
  PFX kod-imzalama sertifikası yolu (gerçek dağıtım). Verilmezse -SelfSign gerekir.

.PARAMETER SelfSign
  Yerel/kendinden-imzalı test sertifikası üretip imzalar (yalnız test).

.EXAMPLE
  ./build-msix.ps1 -Version 0.1.0 -SelfSign
  ./build-msix.ps1 -Version 0.1.0 -CertPath C:\secrets\biocraft.pfx -CertPassword (Read-Host -AsSecureString)
#>
param(
  [string]$Version = "0.1.0",
  [string]$CertPath = "",
  [System.Security.SecureString]$CertPassword = $null,
  [switch]$SelfSign,
  [string]$OutDir = "dist"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot          # repo/packaging
$RepoRoot = Split-Path -Parent $Root              # repo
$Stage = Join-Path $OutDir "msix-stage"
$Pkg = Join-Path $OutDir "BioCraftEngine-$Version-x64.msix"

Write-Host "==> BioCraft Engine MSIX paketleme (sürüm $Version)"

# --- 1. Release binary'sini derle/topla -------------------------------------
Write-Host "--> Release binary derleniyor (cargo build --release)..."
Push-Location $RepoRoot
cargo build --release --locked -p biocraft-app
Pop-Location
$Exe = Join-Path $RepoRoot "target\release\biocraft.exe"
if (-not (Test-Path $Exe)) { throw "biocraft.exe bulunamadı: $Exe" }

# --- 2. Staging: binary + çekirdek eklenti (MK-19) + manifest + varlıklar ----
if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item $Exe (Join-Path $Stage "biocraft.exe")

# Çekirdek eklenti gömme (MK-19): core-plugin/ kurulum köküne kopyalanır → ilk açılışta kurulu.
$CorePlugin = Join-Path $Root "core-plugin"
if (Test-Path $CorePlugin) {
  Copy-Item -Recurse $CorePlugin (Join-Path $Stage "core-plugin")
  Write-Host "--> Çekirdek eklenti gömüldü (MK-19)."
} else {
  Write-Warning "core-plugin/ yok — çekirdek eklenti dosyaları derleme entegrasyonuyla eklenecek."
}

# Manifest (sürüm enjekte edilir: a.b.c.0).
$ManifestSrc = Join-Path $PSScriptRoot "AppxManifest.xml"
$Manifest = Get-Content $ManifestSrc -Raw
$Manifest = $Manifest -replace 'Version="[0-9.]+"', "Version=`"$Version.0`""
Set-Content -Path (Join-Path $Stage "AppxManifest.xml") -Value $Manifest -Encoding utf8

# Varlıklar (logo vb.) — yoksa boş Assets (gerçek logolar tasarım işidir).
$Assets = Join-Path $Stage "Assets"
New-Item -ItemType Directory -Force -Path $Assets | Out-Null

# --- 3. MSIX paketle ---------------------------------------------------------
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
Write-Host "--> makeappx ile paketleniyor..."
& makeappx pack /d $Stage /p $Pkg /o
if ($LASTEXITCODE -ne 0) { throw "makeappx başarısız (Windows SDK kurulu mu?)" }

# --- 4. İmzala ---------------------------------------------------------------
if ($SelfSign) {
  Write-Host "--> [TEST] Kendinden-imzalı sertifika üretiliyor (yalnız bu makine; DAĞITILMAZ)..."
  $cert = New-SelfSignedCertificate -Type Custom -Subject "CN=BioCraft Engine, O=BioCraft Engine, C=TR" `
            -KeyUsage DigitalSignature -FriendlyName "BioCraft Engine Test" `
            -CertStoreLocation "Cert:\CurrentUser\My" `
            -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
  & signtool sign /fd SHA256 /a /sha1 $cert.Thumbprint $Pkg
  if ($LASTEXITCODE -ne 0) { throw "signtool (self-sign) başarısız" }
  Write-Warning "Bu paket yalnız TEST imzalıdır; üretim dağıtımı için gerçek sertifika gerekir."
}
elseif ($CertPath) {
  Write-Host "--> Sertifika ile imzalanıyor: $CertPath"
  $pwPlain = if ($CertPassword) {
    [Runtime.InteropServices.Marshal]::PtrToStringAuto(
      [Runtime.InteropServices.Marshal]::SecureStringToBSTR($CertPassword))
  } else { "" }
  & signtool sign /fd SHA256 /f $CertPath /p $pwPlain $Pkg
  if ($LASTEXITCODE -ne 0) { throw "signtool başarısız" }
}
else {
  Write-Warning "İMZASIZ paket üretildi (yalnız CI artifact denemesi). DAĞITILMAZ — -SelfSign veya -CertPath verin."
}

Write-Host "==> Tamam: $Pkg"
