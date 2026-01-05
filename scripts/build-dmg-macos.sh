#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ "${OSTYPE:-}" != darwin* ]]; then
  echo "This script is macOS-only."
  exit 1
fi

APP_PATH="${1:-}"
VOLNAME="${2:-}"
OUT_DMG="${3:-}"

if [[ -z "${APP_PATH}" || -z "${VOLNAME}" || -z "${OUT_DMG}" ]]; then
  echo "Usage: scripts/build-dmg-macos.sh <AppPath> <VolumeName> <OutDmgPath>"
  exit 2
fi

if [[ ! -d "${APP_PATH}" ]]; then
  echo "❌ 未找到 .app：${APP_PATH}"
  exit 1
fi

if ! command -v hdiutil >/dev/null 2>&1; then
  echo "❌ 未找到 hdiutil（macOS 系统工具）。"
  exit 1
fi

STAGING_DIR="$(mktemp -d)"
cleanup() { rm -rf "${STAGING_DIR}"; }
trap cleanup EXIT

APP_NAME="$(basename "${APP_PATH}")"
cp -R "${APP_PATH}" "${STAGING_DIR}/${APP_NAME}"
ln -s /Applications "${STAGING_DIR}/Applications"

mkdir -p "$(dirname "${OUT_DMG}")"
rm -f "${OUT_DMG}"

echo "📦 生成 DMG：${OUT_DMG}"
hdiutil create \
  -volname "${VOLNAME}" \
  -fs HFS+ \
  -srcfolder "${STAGING_DIR}" \
  -ov \
  -format UDZO \
  "${OUT_DMG}" >/dev/null

echo "✅ DMG 完成"
