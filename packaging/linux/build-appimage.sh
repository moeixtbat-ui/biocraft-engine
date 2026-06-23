#!/usr/bin/env bash
# BioCraft Engine — Linux AppImage kurulum paketi üretir (İP-20, MK-56, MK-19).
#
# 1. Release binary'sini derler.
# 2. AppDir iskeletini kurar (binary + çekirdek eklenti + .desktop + ikon + AppRun).
# 3. appimagetool ile tek dosya .AppImage üretir.
#
# Kod-imzalama: AppImage'ler dağıtım için GPG ile imzalanabilir (--sign). İmza anahtarı insan-eli
# (Hukuk-ve-Operasyon.md); gelene dek imzasız test paketi üretilir (DAĞITILMAZ).
#
# Kullanım: ./build-appimage.sh <sürüm> [--sign]
set -euo pipefail

VERSION="${1:-0.1.0}"
SIGN="${2:-}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
OUT_DIR="${REPO_ROOT}/dist"
APPDIR="${OUT_DIR}/BioCraftEngine.AppDir"

echo "==> BioCraft Engine AppImage paketleme (sürüm ${VERSION})"

# --- 1. Release binary ------------------------------------------------------
echo "--> Release binary derleniyor..."
( cd "${REPO_ROOT}" && cargo build --release --locked -p biocraft-app )
BIN="${REPO_ROOT}/target/release/biocraft"
[ -x "${BIN}" ] || { echo "HATA: biocraft bulunamadı: ${BIN}" >&2; exit 1; }

# --- 2. AppDir iskeleti -----------------------------------------------------
rm -rf "${APPDIR}"
mkdir -p "${APPDIR}/usr/bin" "${APPDIR}/usr/lib/biocraft" \
         "${APPDIR}/usr/share/applications" \
         "${APPDIR}/usr/share/icons/hicolor/256x256/apps"

cp "${BIN}" "${APPDIR}/usr/bin/biocraft"
cp "${SCRIPT_DIR}/AppRun" "${APPDIR}/AppRun"
chmod +x "${APPDIR}/AppRun"
cp "${SCRIPT_DIR}/biocraft.desktop" "${APPDIR}/biocraft.desktop"
cp "${SCRIPT_DIR}/biocraft.desktop" "${APPDIR}/usr/share/applications/biocraft.desktop"

# İkon (yoksa placeholder; gerçek ikon tasarım işidir).
ICON_SRC="${REPO_ROOT}/assets/icons/biocraft-256.png"
if [ -f "${ICON_SRC}" ]; then
  cp "${ICON_SRC}" "${APPDIR}/biocraft.png"
  cp "${ICON_SRC}" "${APPDIR}/usr/share/icons/hicolor/256x256/apps/biocraft.png"
else
  echo "--> UYARI: ikon yok (assets/icons/biocraft-256.png); AppImage ikonsuz üretilecek."
  : > "${APPDIR}/biocraft.png"
fi

# Çekirdek eklenti gömme (MK-19): ilk açılışta kurulu → "eklenti yok" ekranı görünmez.
CORE_PLUGIN="${SCRIPT_DIR}/../core-plugin"
if [ -d "${CORE_PLUGIN}" ]; then
  cp -r "${CORE_PLUGIN}" "${APPDIR}/usr/lib/biocraft/core-plugin"
  echo "--> Çekirdek eklenti gömüldü (MK-19)."
else
  echo "--> UYARI: core-plugin/ yok — çekirdek eklenti dosyaları derleme entegrasyonuyla eklenecek."
fi

# --- 3. appimagetool ---------------------------------------------------------
mkdir -p "${OUT_DIR}"
OUTPUT="${OUT_DIR}/BioCraftEngine-${VERSION}-x86_64.AppImage"
SIGN_ARGS=""
if [ "${SIGN}" = "--sign" ]; then
  SIGN_ARGS="--sign"
  echo "--> GPG imzalama etkin."
else
  echo "--> UYARI: imzasız AppImage (yalnız test/CI artifact). DAĞITILMAZ."
fi

if command -v appimagetool >/dev/null 2>&1; then
  ARCH=x86_64 appimagetool ${SIGN_ARGS} "${APPDIR}" "${OUTPUT}"
  echo "==> Tamam: ${OUTPUT}"
else
  echo "--> appimagetool bulunamadı; AppDir hazır: ${APPDIR}"
  echo "    (CI bu betiği appimagetool kurulu ortamda çalıştırır.)"
fi
