---
name: trans-clip-project
description: Development workflow and guardrails for the trans-clip Tauri app (React + TypeScript + Rust). Use when implementing or debugging features in this repository, including clipboard/translation flow, Tauri commands, Zustand state, and desktop integration under src-tauri.
---

# Trans Clip Project Skill

Follow this workflow for changes in this repository.

## Workflow

1. Read relevant files first with `rg` and focused `sed` ranges.
2. Prefer minimal diffs that preserve existing architecture.
3. For frontend changes, keep strict TypeScript and existing formatting (2 spaces, double quotes, semicolons).
4. For Tauri backend changes, keep Rust modules idiomatic and register commands clearly.
5. Validate with the project command set before finishing.

## Project map

- Frontend app: `src/`
- UI components: `src/components/`
- Shared logic: `src/hooks/`, `src/store/`, `src/services/`, `src/utils/`
- Tauri backend: `src-tauri/src/`
- Specs and planning: `specs/`

## Validation commands

Run what matches the scope of the change:

- `pnpm lint`
- `pnpm test`
- `pnpm build`
- `pnpm tauri dev` for integration checks when needed

## Guardrails

- Avoid introducing `any` unless justified.
- Avoid unrelated refactors in feature/fix tasks.
- Never commit secrets or API keys.
- Surface macOS permission implications for Accessibility or clipboard behavior.
