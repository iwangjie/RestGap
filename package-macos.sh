#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

CARGO_CMD=(cargo)

./scripts/release-preflight.sh

if ! command -v cargo-packager >/dev/null 2>&1; then
  "${CARGO_CMD[@]}" install cargo-packager --locked
fi

./scripts/build-universal2-macos.sh

"${CARGO_CMD[@]}" packager --release --formats app --binaries-dir target/universal2-apple-darwin/release

PRODUCT_NAME="RestGap"
APP_PATH="dist/${PRODUCT_NAME}.app"

FAT_BIN="target/universal2-apple-darwin/release/restgap"
APP_BIN="${APP_PATH}/Contents/MacOS/restgap"

if [[ ! -f "${FAT_BIN}" ]]; then
  echo "❌ 未找到 universal2 二进制：${FAT_BIN}"
  exit 1
fi

if [[ ! -f "${APP_BIN}" ]]; then
  echo "❌ 未找到 app 内可执行文件：${APP_BIN}"
  exit 1
fi

LIPO_INFO="$(lipo -info "${FAT_BIN}")"
if command -v rg >/dev/null 2>&1; then
  if ! echo "${LIPO_INFO}" | rg -q 'x86_64.*arm64|arm64.*x86_64'; then
    echo "❌ universal2 二进制看起来不是 fat file：${LIPO_INFO}"
    exit 1
  fi
else
  if ! echo "${LIPO_INFO}" | grep -Eq 'x86_64.*arm64|arm64.*x86_64'; then
    echo "❌ universal2 二进制看起来不是 fat file：${LIPO_INFO}"
    exit 1
  fi
fi

echo "🧩 写入 universal2 可执行文件到 .app..."
cp "${FAT_BIN}" "${APP_BIN}"
chmod +x "${APP_BIN}"

INFO_PLIST="${APP_PATH}/Contents/Info.plist"
PKG_INFO="${APP_PATH}/Contents/PkgInfo"

if [[ ! -f "${INFO_PLIST}" ]]; then
  echo "❌ 未找到 app Info.plist：${INFO_PLIST}"
  exit 1
fi

# cargo-packager 0.11.x currently emits LSRequiresCarbon for .app bundles.
# Modern 64-bit AppKit apps must not advertise Carbon; LaunchServices may reject
# the bundle as invalid before the menu bar app can start.
/usr/libexec/PlistBuddy -c "Delete :LSRequiresCarbon" "${INFO_PLIST}" >/dev/null 2>&1 || true
printf 'APPL????' > "${PKG_INFO}"

echo "🔏 ad-hoc 签名 .app..."
codesign --force --deep --sign - "${APP_PATH}" >/dev/null

if command -v jq >/dev/null 2>&1; then
  VERSION=$("${CARGO_CMD[@]}" metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name=="restgap") | .version' | head -n 1)
else
  VERSION="$(awk -F '\"' '/^version =/ {print $2; exit}' Cargo.toml)"
fi

DMG_PATH="dist/${PRODUCT_NAME}_${VERSION}_universal2.dmg"
./scripts/build-dmg-macos.sh "${APP_PATH}" "${PRODUCT_NAME}" "${DMG_PATH}"
