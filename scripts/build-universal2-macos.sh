#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ "${OSTYPE:-}" != darwin* ]]; then
  echo "This script is macOS-only."
  exit 1
fi

BIN_NAME="restgap"
ARM_TARGET="aarch64-apple-darwin"
X64_TARGET="x86_64-apple-darwin"

OUT_DIR="target/universal2-apple-darwin/release"
ARM_BIN="target/${ARM_TARGET}/release/${BIN_NAME}"
X64_BIN="target/${X64_TARGET}/release/${BIN_NAME}"
OUT_BIN="${OUT_DIR}/${BIN_NAME}"

if ! command -v rustup >/dev/null 2>&1; then
  echo "❌ rustup 未安装：无法自动添加 target。请先安装 Rust（含 rustup）。"
  exit 1
fi

if ! rustup run stable rustc -V >/dev/null 2>&1; then
  echo "❌ 未检测到 rustup 的 stable 工具链。请先执行：rustup toolchain install stable"
  exit 1
fi

if ! command -v lipo >/dev/null 2>&1; then
  echo "❌ 未找到 lipo。请先安装/启用 Xcode Command Line Tools：xcode-select --install"
  exit 1
fi

RUSTUP_CARGO="$(rustup which --toolchain stable cargo)"
RUSTUP_RUSTC="$(rustup which --toolchain stable rustc)"

echo "🔧 准备构建 universal2（${ARM_TARGET} + ${X64_TARGET}）..."
rustup target add --toolchain stable "${ARM_TARGET}" "${X64_TARGET}"

echo "🏗️  构建 ${ARM_TARGET}..."
RUSTC="${RUSTUP_RUSTC}" "${RUSTUP_CARGO}" build --release --target "${ARM_TARGET}"

echo "🏗️  构建 ${X64_TARGET}..."
RUSTC="${RUSTUP_RUSTC}" "${RUSTUP_CARGO}" build --release --target "${X64_TARGET}"

mkdir -p "${OUT_DIR}"
echo "🧬 合并为 universal2：${OUT_BIN}"
lipo -create -output "${OUT_BIN}" "${ARM_BIN}" "${X64_BIN}"
chmod +x "${OUT_BIN}"

echo "✅ 完成：$(lipo -info "${OUT_BIN}")"
