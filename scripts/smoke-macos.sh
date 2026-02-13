#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "[smoke:macos] This script must run on macOS."
  exit 1
fi

echo "[smoke:macos] Running frontend checks..."
pnpm ci:check

echo "[smoke:macos] Running Rust tests and lints..."
pnpm test:rust
pnpm lint:rust

echo "[smoke:macos] Building Tauri macOS bundle..."
pnpm tauri build

shopt -s nullglob
DMG_FILES=(src-tauri/target/release/bundle/dmg/*.dmg)
APP_ARCHIVE_FILES=(src-tauri/target/release/bundle/macos/*.app.tar.gz)
APP_BUNDLE_FILES=(src-tauri/target/release/bundle/macos/*.app)
shopt -u nullglob

if [[ ! -e "${DMG_FILES[0]}" ]]; then
  echo "[smoke:macos] ERROR: DMG artifact not found"
  exit 1
fi

if [[ ${#APP_ARCHIVE_FILES[@]} -eq 0 && ${#APP_BUNDLE_FILES[@]} -eq 0 ]]; then
  echo "[smoke:macos] ERROR: macOS app artifact not found (.app or .app.tar.gz)"
  exit 1
fi

echo "[smoke:macos] Artifacts created:"
ls -1 src-tauri/target/release/bundle/dmg/*.dmg

if [[ ${#APP_ARCHIVE_FILES[@]} -gt 0 ]]; then
  ls -1 src-tauri/target/release/bundle/macos/*.app.tar.gz
fi

if [[ ${#APP_BUNDLE_FILES[@]} -gt 0 ]]; then
  ls -1d src-tauri/target/release/bundle/macos/*.app
fi
