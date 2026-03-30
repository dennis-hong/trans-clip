# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TransClip is a macOS menu bar app for instant AI-powered translation (Korean ↔ English) and clipboard management. Built with Tauri 2.0 (Rust backend + React/TypeScript frontend).

**Core interactions**: Double-press Cmd+C → translate, double-press Cmd+E → polish/refine text, Cmd+Option+V → clipboard history.

## Commands

```bash
# Development
pnpm install              # Install frontend dependencies
pnpm tauri dev            # Run full app (Vite dev server + Tauri)
pnpm dev                  # Frontend only (Vite on port 1420)

# Build
pnpm tauri build          # Production build → DMG in src-tauri/target/release/bundle/dmg/

# Test
pnpm test                 # Vitest (frontend)
pnpm test:ui              # Vitest interactive UI
cd src-tauri && cargo test # Rust tests

# Lint
pnpm lint                 # ESLint for src/
cd src-tauri && cargo clippy  # Rust lints
```

## Architecture

### Frontend → Backend Communication

All IPC goes through Tauri's `invoke` API with typed responses. Streaming uses Tauri `Channel` for real-time deltas (translation/polish results streamed token-by-token).

```
Frontend (React)  ──invoke()──►  src-tauri/src/commands/*.rs
                  ◄──Channel──   (streaming events: started → delta → completed)
```

### Key Directories

- `src/` — React frontend
  - `store/` — Zustand stores (clipboardStore, settingsStore, glossaryStore, polishStore)
  - `hooks/` — Custom hooks wrapping Tauri invoke calls (useTranslationStream, usePolishStream, useClipboard, useGlossary)
  - `components/` — UI (TranslationPopup, PolishPopup, DrawerPanel, ClipboardHistory, GlossaryManager, Settings)
  - `types/index.ts` — All TypeScript type definitions (440+ lines, single source of truth)
- `src-tauri/src/` — Rust backend
  - `lib.rs` — App initialization, hotkey setup, clipboard monitor start
  - `commands/` — IPC handlers: translate.rs, polish.rs, clipboard.rs, glossary.rs, settings.rs, window.rs, system.rs
  - `database.rs` — SQLite operations via sqlx (async)
  - `hotkey.rs` — macOS CGEventTap FFI for global hotkey detection (double-press logic)
  - `clipboard.rs` — Polling-based clipboard monitor (500ms interval)
  - `keychain.rs` — macOS Keychain FFI for secure API key storage
  - `prompts/` — Claude API prompt templates
  - `utils/streaming.rs` — SSE stream parsing for Claude API
- `specs/001-trans-clip/` — Feature specs, data model, IPC contracts

### State Management

- **Frontend**: Zustand stores for reactive state. Each store wraps Tauri invoke calls.
- **Backend**: `AppState` with `Arc<Mutex<Database>>` for thread-safe SQLite access. Database at `~/Library/Application Support/com.transclip.app/transclip.db`.

### Popup Modes

App.tsx manages a `popupMode` state (`"none" | "translate" | "polish" | "history"`) that determines which UI is shown. Backend events (`double_copy_detected`, `polish_detected`, `toggle_drawer`) trigger mode switches via Tauri event listeners.

### macOS-Specific

- Hotkeys use raw `CGEventTapCreate` FFI (not Tauri's global-shortcut plugin)
- Clipboard monitoring uses NSPasteboard polling
- API keys stored in macOS Keychain via Security framework FFI
- Window: transparent, no decorations, always-on-top
- Requires Accessibility permission for hotkey interception

## Code Conventions

- TypeScript path alias: `@/*` → `src/*`
- All Tauri commands return `Result<T, String>` on the Rust side
- Streaming events follow the pattern: `{ event: "started" | "delta" | "completed" | "error", data: {...} }`
- SQLite tables: `clipboard_items`, `translations` (cache), `glossary_entries`, `settings`
- Translation caching: cached results bypass Claude API calls entirely
