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
if [[ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
  pnpm tauri build
else
  echo "[smoke:macos] TAURI_SIGNING_PRIVATE_KEY not set; disabling updater artifacts for local smoke run."
  TMP_TAURI_CONFIG="$(mktemp "${TMPDIR:-/tmp}/tauri-smoke-config-XXXXXX.json")"
  trap 'rm -f "${TMP_TAURI_CONFIG}"' EXIT

  node -e '
    const fs = require("fs");
    const out = process.argv[1];
    const conf = JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8"));
    conf.bundle = conf.bundle || {};
    conf.bundle.createUpdaterArtifacts = false;
    fs.writeFileSync(out, JSON.stringify(conf, null, 2));
  ' "${TMP_TAURI_CONFIG}"

  pnpm tauri build --config "${TMP_TAURI_CONFIG}"
fi

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
