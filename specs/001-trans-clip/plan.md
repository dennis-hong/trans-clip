# Implementation Plan: TransClip - 번역 및 클립보드 관리 앱

**Branch**: `001-trans-clip` | **Date**: 2025-12-30 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-trans-clip/spec.md`

## Summary

macOS 메뉴바에서 동작하는 한영/영한 번역 및 클립보드 히스토리 관리 앱. Cmd+CC (더블 프레스)로 번역 팝업을 호출하고, Claude API를 사용하여 번역 수행. 사용자 정의 단어집으로 번역 정확도 향상. Tauri 2.0 (TypeScript + Rust)으로 구현하여 유지보수성과 네이티브 성능을 동시에 확보.

## Technical Context

**Language/Version**: TypeScript 5.x (Frontend), Rust (Backend via Tauri 2.0)
**Primary Dependencies**: Tauri 2.0, React 18, tauri-plugin-clipboard, tauri-plugin-global-shortcut, tauri-plugin-sql
**Storage**: SQLite (클립보드 히스토리, 단어집), macOS Keychain (API 키)
**Testing**: Vitest (Frontend), cargo test (Backend)
**Target Platform**: macOS 12+ (Monterey 이상)
**Project Type**: Single (Desktop Application)
**Performance Goals**: 번역 팝업 1초 이내 표시, 번역 결과 3초 이내 (200자 기준)
**Constraints**: 유휴 시 30-50MB 메모리, Accessibility Permission 필수, App Store 배포 불가 (DMG/Homebrew)
**Scale/Scope**: 개인/소규모 팀 사용자, 클립보드 히스토리 최대 100개

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Constitution이 아직 프로젝트에 맞게 커스터마이징되지 않아 기본 템플릿 상태입니다.
이 프로젝트에서는 다음 원칙을 적용합니다:

1. **유지보수성 우선**: TypeScript + Rust로 유지보수 용이한 코드 작성
2. **모듈식 설계**: 기능별 독립적 모듈로 구성
3. **보안 우선**: API 키는 반드시 Keychain에 저장
4. **테스트 필수**: 핵심 비즈니스 로직은 테스트 커버리지 확보

**Gate Status**: ✅ PASS

## Project Structure

### Documentation (this feature)

```text
specs/001-trans-clip/
├── plan.md              # This file
├── research.md          # Phase 0 output (완료)
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
src/                              # TypeScript/React 프론트엔드
├── components/
│   ├── TranslationPopup/         # 번역 팝업 UI
│   │   ├── TranslationPopup.tsx
│   │   ├── SourceText.tsx
│   │   └── TranslatedText.tsx
│   ├── ClipboardHistory/         # 클립보드 히스토리 UI
│   │   ├── HistoryPanel.tsx
│   │   └── HistoryItem.tsx
│   ├── GlossaryManager/          # 단어집 관리 UI
│   │   ├── GlossaryList.tsx
│   │   └── GlossaryEditor.tsx
│   ├── Settings/                 # 설정 UI
│   │   ├── SettingsPanel.tsx
│   │   └── ApiKeyInput.tsx
│   └── common/                   # 공통 컴포넌트
│       ├── Button.tsx
│       └── Modal.tsx
├── hooks/
│   ├── useTranslation.ts
│   ├── useClipboard.ts
│   └── useGlossary.ts
├── services/
│   ├── claudeApi.ts              # Claude API 통신
│   ├── languageDetector.ts       # 언어 감지
│   └── storage.ts                # SQLite 래퍼
├── store/
│   ├── clipboardStore.ts
│   ├── glossaryStore.ts
│   └── settingsStore.ts
├── types/
│   └── index.ts                  # TypeScript 타입 정의
├── App.tsx
└── main.tsx

src-tauri/                        # Rust 백엔드
├── src/
│   ├── lib.rs                    # Tauri 엔트리포인트
│   ├── clipboard.rs              # 클립보드 모니터링
│   ├── hotkey.rs                 # Cmd+CC 감지 (CGEventTap)
│   ├── translate.rs              # Claude API 호출
│   ├── database.rs               # SQLite 연결
│   ├── keychain.rs               # macOS Keychain 연동
│   └── commands.rs               # Tauri 커맨드 정의
├── Cargo.toml
├── tauri.conf.json
└── capabilities/
    └── default.json              # 권한 설정

tests/
├── unit/                         # 유닛 테스트
│   ├── services/
│   └── components/
└── integration/                  # 통합 테스트
    └── translation.test.ts
```

**Structure Decision**: Desktop 전용 단일 프로젝트. Tauri의 표준 구조를 따르며, 프론트엔드(src/)와 백엔드(src-tauri/)로 분리.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Rust 백엔드 | CGEventTap, Keychain 등 macOS 네이티브 API 접근 필요 | Node.js 네이티브 모듈은 유지보수 어려움 |
| SQLite | 클립보드 히스토리 영속성, 앱 재시작 후 데이터 유지 필요 | 메모리 저장소는 재시작 시 데이터 손실 |
