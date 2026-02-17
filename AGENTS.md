# Repository Guidelines

## Project Structure & Module Organization
- `src/`: React + TypeScript frontend (Vite).
- `src/components/`: UI features (`TranslationPopup`, `DrawerPanel`, `Settings`, etc.).
- `src/hooks/`, `src/store/`, `src/services/`, `src/utils/`: shared app logic, Zustand state, API integration, utilities.
- `src-tauri/`: Rust/Tauri backend (commands, hotkey handling, database, keychain, OS integration).
- `specs/`: product/design specs and planning docs, not runtime code.
- Entry points: `src/main.tsx` (web UI) and `src-tauri/src/main.rs` + `src-tauri/src/lib.rs` (desktop shell).

## Build, Test, and Development Commands
- `pnpm install`: install JS dependencies.
- `pnpm dev`: run Vite frontend dev server.
- `pnpm tauri dev`: run full desktop app in development (frontend + Rust backend).
- `pnpm build`: type-check and build frontend assets.
- `pnpm tauri build`: create production desktop build.
- `pnpm test`: run Vitest tests.
- `pnpm lint`: run ESLint on `src/**/*.{ts,tsx}`.

## Coding Style & Naming Conventions
- TypeScript uses strict mode (`tsconfig.json`); avoid `any` unless justified.
- Frontend formatting: 2-space indentation, double quotes, semicolons (match existing files like `src/App.tsx`).
- React components: PascalCase file and export names (`HistoryPanel.tsx`).
- Hooks: `useXxx` naming (`useTranslationStream.ts`).
- Stores/services/utils: camelCase file names (`clipboardStore.ts`, `claudeApi.ts`).
- Rust modules: snake_case file names and idiomatic `rustfmt` style.

## Testing Guidelines
- Primary framework: Vitest (`pnpm test`, `pnpm test:ui`).
- Place frontend tests near source as `*.test.ts` / `*.test.tsx` (for example `src/hooks/usePolish.test.ts`).
- Add or update tests for changed behavior in hooks, stores, and command-invocation logic.
- Before PRs, run at least `pnpm lint`, `pnpm test`, and `pnpm build`.

## Commit & Pull Request Guidelines
- Follow Conventional Commit style seen in history: `feat:`, `fix:`, `chore:`, `docs:`.
- Keep commits focused and descriptive (example: `fix: reorder hide/paste sequence for reliability`).
- PRs should include:
  - clear summary of user-visible/backend impact,
  - linked issue(s) when applicable,
  - screenshots or short recordings for UI changes,
  - local verification steps and command results.

## Security & Configuration Tips
- Never commit API keys or secrets; API keys are stored via macOS keychain integration.
- Desktop behavior depends on macOS Accessibility permissions; document permission-related changes in PR notes.
