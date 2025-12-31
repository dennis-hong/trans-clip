# Tasks: TransClip - 번역 및 클립보드 관리 앱

**Input**: Design documents from `/specs/001-trans-clip/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: 테스트는 spec.md에 명시적 요청이 없어 포함하지 않음.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

- **Frontend**: `src/` (TypeScript/React)
- **Backend**: `src-tauri/src/` (Rust)
- **Tests**: `tests/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and Tauri 2.0 structure setup

- [x] T001 Create Tauri 2.0 project with React template using `pnpm create tauri-app`
- [x] T002 [P] Configure package.json with required dependencies (@tauri-apps/api, react, zustand, typescript)
- [x] T003 [P] Configure Cargo.toml with Rust dependencies (tauri, sqlx, reqwest, keyring, uuid, chrono)
- [x] T004 [P] Setup tauri.conf.json with tray-icon feature and window configuration
- [x] T005 [P] Create src-tauri/entitlements.plist for macOS permissions
- [x] T006 [P] Create src-tauri/Info.plist with LSUIElement=true for menu bar app
- [x] T007 Define TypeScript types in src/types/index.ts (ClipboardItem, GlossaryEntry, Translation, UserSettings, API responses)
- [x] T008 Setup Vitest for frontend testing in vite.config.ts

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure required by ALL user stories

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T009 Setup SQLite database schema with migrations in src-tauri/src/database.rs
- [x] T010 [P] Create clipboard_items table with indexes per data-model.md
- [x] T011 [P] Create glossary_entries table with indexes per data-model.md
- [x] T012 [P] Create translations table (cache) with indexes per data-model.md
- [x] T013 [P] Create user_settings table with defaults per data-model.md
- [x] T014 Implement Keychain integration for API key storage in src-tauri/src/keychain.rs
- [x] T015 Implement base Tauri commands structure in src-tauri/src/commands.rs
- [x] T016 Setup Tauri event system for frontend-backend communication in src-tauri/src/lib.rs
- [x] T017 Create common UI components (Button, Modal) in src/components/common/
- [x] T018 Setup Zustand stores structure in src/store/ (clipboardStore.ts, settingsStore.ts, glossaryStore.ts)
- [x] T019 Configure Tauri capabilities in src-tauri/capabilities/default.json

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - 빠른 번역 (Priority: P1) 🎯 MVP

**Goal**: Cmd+CC 더블 프레스로 번역 팝업 표시, 원문/번역문 표시, 복사/바꾸기 기능

**Independent Test**: 텍스트 선택 후 Cmd+CC 입력 → 번역 팝업 표시 → 복사 또는 바꾸기 동작 확인

### Backend Implementation for User Story 1

- [x] T020 [US1] Implement CGEventTap-based Cmd+CC double-press detection in src-tauri/src/hotkey.rs
- [x] T021 [US1] Implement accessibility permission check/request in src-tauri/src/hotkey.rs
- [x] T022 [US1] Implement Claude API translation service in src-tauri/src/translate.rs
- [x] T023 [US1] Add language auto-detection logic in src-tauri/src/translate.rs
- [x] T024 [US1] Implement translation caching (check cache before API call) in src-tauri/src/translate.rs
- [x] T025 [US1] Implement 'translate' Tauri command per contracts in src-tauri/src/commands.rs
- [x] T026 [US1] Implement 'paste_text' command (CGEventPost Cmd+V simulation) in src-tauri/src/commands.rs
- [x] T027 [US1] Implement 'show_translation_popup' and 'hide_translation_popup' commands in src-tauri/src/commands.rs
- [x] T028 [US1] Emit 'double_copy_detected' event when Cmd+CC detected in src-tauri/src/hotkey.rs

### Frontend Implementation for User Story 1

- [x] T029 [US1] Create TranslationPopup component in src/components/TranslationPopup/TranslationPopup.tsx
- [x] T030 [P] [US1] Create SourceText component in src/components/TranslationPopup/SourceText.tsx
- [x] T031 [P] [US1] Create TranslatedText component in src/components/TranslationPopup/TranslatedText.tsx
- [x] T032 [US1] Implement useTranslation hook in src/hooks/useTranslation.ts
- [x] T033 [US1] Create claudeApi service wrapper in src/services/claudeApi.ts
- [x] T034 [US1] Listen for 'double_copy_detected' event and show popup in src/App.tsx
- [x] T035 [US1] Implement Copy button functionality (invoke set_clipboard) in TranslationPopup
- [x] T036 [US1] Implement Replace button functionality (invoke paste_text) in TranslationPopup
- [x] T037 [US1] Add loading state and error handling UI in TranslationPopup
- [x] T038 [US1] Implement popup close on ESC key or outside click

**Checkpoint**: User Story 1 완료 - Cmd+CC로 번역 기능 독립 테스트 가능

---

## Phase 4: User Story 4 - 편리한 GUI (Priority: P2)

**Goal**: 메뉴바 상주, 클립보드 히스토리 접근, 설정 관리 UI

**Independent Test**: 메뉴바 아이콘 클릭 → 패널 표시 → 설정에서 API 키 입력/저장 확인

**Note**: US4를 US2보다 먼저 구현하여 앱 기본 GUI 프레임워크를 완성

### Backend Implementation for User Story 4

- [x] T039 [US4] Setup system tray with menu in src-tauri/src/lib.rs
- [x] T040 [US4] Implement 'get_settings' and 'update_settings' commands in src-tauri/src/commands.rs
- [x] T041 [US4] Implement 'get_api_key', 'set_api_key', 'delete_api_key' commands in src-tauri/src/commands.rs
- [x] T042 [US4] Implement 'check_accessibility_permission' and 'request_accessibility_permission' commands in src-tauri/src/commands.rs
- [x] T043 [US4] Add API key validation on set_api_key in src-tauri/src/keychain.rs

### Frontend Implementation for User Story 4

- [x] T044 [US4] Create SettingsPanel component in src/components/Settings/SettingsPanel.tsx
- [x] T045 [P] [US4] Create ApiKeyInput component in src/components/Settings/ApiKeyInput.tsx
- [x] T046 [US4] Implement settingsStore with Zustand in src/store/settingsStore.ts
- [x] T047 [US4] Create main App layout with tray menu panel in src/App.tsx
- [ ] T048 [US4] Add settings navigation and view switching
- [ ] T049 [US4] Implement first-run onboarding flow for API key and accessibility permission

**Checkpoint**: User Story 4 완료 - 메뉴바 앱 기본 GUI 독립 테스트 가능

---

## Phase 5: User Story 2 - 클립보드 히스토리 관리 (Priority: P2)

**Goal**: Cmd+C 복사 시 히스토리 저장, 이전 항목 재사용, 삭제 기능

**Independent Test**: 여러 텍스트 복사 → 히스토리에서 이전 항목 선택 → Cmd+V로 붙여넣기 확인

### Backend Implementation for User Story 2

- [x] T050 [US2] Implement clipboard monitoring with tauri-plugin-clipboard in src-tauri/src/clipboard.rs
- [x] T051 [US2] Implement clipboard history save on TEXT_CHANGED event in src-tauri/src/clipboard.rs
- [x] T052 [US2] Implement 'get_clipboard_history' command with pagination in src-tauri/src/commands.rs
- [x] T053 [US2] Implement 'delete_clipboard_item' command in src-tauri/src/commands.rs
- [x] T054 [US2] Implement 'toggle_pin_clipboard_item' command in src-tauri/src/commands.rs
- [x] T055 [US2] Implement 'set_clipboard' command in src-tauri/src/commands.rs
- [x] T056 [US2] Emit 'clipboard_changed' event on new clipboard content in src-tauri/src/clipboard.rs
- [x] T057 [US2] Implement auto-cleanup of old items when exceeding maxHistoryCount in src-tauri/src/clipboard.rs

### Frontend Implementation for User Story 2

- [x] T058 [US2] Create HistoryPanel component in src/components/ClipboardHistory/HistoryPanel.tsx
- [x] T059 [P] [US2] Create HistoryItem component in src/components/ClipboardHistory/HistoryItem.tsx
- [x] T060 [US2] Implement clipboardStore with Zustand in src/store/clipboardStore.ts
- [x] T061 [US2] Implement useClipboard hook in src/hooks/useClipboard.ts
- [x] T062 [US2] Listen for 'clipboard_changed' event and update store in src/App.tsx
- [x] T063 [US2] Add click-to-copy functionality on history item selection
- [x] T064 [US2] Add delete and pin/unpin UI actions on history items
- [x] T065 [US2] Add search/filter functionality to clipboard history

**Checkpoint**: User Story 2 완료 - 클립보드 히스토리 기능 독립 테스트 가능

---

## Phase 6: User Story 3 - 사용자 정의 단어집 (Priority: P3)

**Goal**: 단어집에 용어 등록, 번역 시 단어집 용어 우선 적용

**Independent Test**: 단어집에 "회사명=CompanyName" 등록 → "회사명을 입력하세요" 번역 → 단어집 반영 확인

### Backend Implementation for User Story 3

- [x] T066 [US3] Implement 'get_glossary_entries' command with search/sort in src-tauri/src/commands.rs
- [x] T067 [US3] Implement 'add_glossary_entry' command with validation in src-tauri/src/commands.rs
- [x] T068 [US3] Implement 'update_glossary_entry' command in src-tauri/src/commands.rs
- [x] T069 [US3] Implement 'delete_glossary_entry' command in src-tauri/src/commands.rs
- [x] T070 [US3] Implement 'import_glossary' command (CSV/JSON) in src-tauri/src/commands.rs
- [x] T071 [US3] Implement 'export_glossary' command (CSV/JSON) in src-tauri/src/commands.rs
- [x] T072 [US3] Integrate glossary lookup into translation service in src-tauri/src/translate.rs
- [x] T073 [US3] Include glossary terms in Claude API prompt per research.md template in src-tauri/src/translate.rs

### Frontend Implementation for User Story 3

- [x] T074 [US3] Create GlossaryList component in src/components/GlossaryManager/GlossaryList.tsx
- [x] T075 [P] [US3] Create GlossaryEditor component (add/edit form) in src/components/GlossaryManager/GlossaryEditor.tsx
- [x] T076 [US3] Implement glossaryStore with Zustand in src/store/glossaryStore.ts
- [x] T077 [US3] Implement useGlossary hook in src/hooks/useGlossary.ts
- [ ] T078 [US3] Add glossary management view to Settings panel
- [ ] T079 [US3] Add import/export UI buttons in GlossaryList

**Checkpoint**: User Story 3 완료 - 단어집 기능 독립 테스트 가능

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T080 [P] Add error boundary and global error handling in src/App.tsx
- [ ] T081 [P] Implement app auto-launch at login setting in src-tauri/src/lib.rs
- [ ] T082 Add keyboard shortcuts for common actions (settings, history panel)
- [ ] T083 Implement translation cache cleanup (expired entries) background task in src-tauri/src/translate.rs
- [ ] T084 Add memory usage optimization (lazy loading, virtualized list for history)
- [ ] T085 Implement streaming translation response UI for better UX
- [ ] T086 Add app icon and tray icon assets in src-tauri/icons/
- [ ] T087 Create DMG installer configuration for distribution
- [ ] T088 Run quickstart.md validation and verify all acceptance scenarios

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies - start immediately
- **Phase 2 (Foundational)**: Depends on Phase 1 - **BLOCKS all user stories**
- **Phase 3 (US1 번역)**: Depends on Phase 2 - **MVP, complete first**
- **Phase 4 (US4 GUI)**: Depends on Phase 2 - Can run in parallel with US1 if staffed
- **Phase 5 (US2 클립보드)**: Depends on Phase 2, integrates with US4 GUI
- **Phase 6 (US3 단어집)**: Depends on Phase 2, integrates with US1 translation
- **Phase 7 (Polish)**: Depends on all user stories being complete

### User Story Dependencies

```
Phase 2 (Foundational)
        │
        ├──────────────────────────────────────┐
        │                                      │
        ▼                                      ▼
Phase 3 (US1 번역) ◄────────────────► Phase 4 (US4 GUI)
   P1 MVP                               P2
        │                                      │
        │                                      │
        └──────────┬───────────────────────────┘
                   │
        ┌──────────┴──────────┐
        │                     │
        ▼                     ▼
Phase 5 (US2 클립보드)    Phase 6 (US3 단어집)
   P2                        P3
        │                     │
        └──────────┬──────────┘
                   │
                   ▼
          Phase 7 (Polish)
```

### Within Each User Story

- Backend tasks before dependent frontend tasks
- Core functionality before integration
- Error handling after happy path

### Parallel Opportunities

**Phase 1 (Setup):**
- T002, T003, T004, T005, T006 can run in parallel

**Phase 2 (Foundational):**
- T010, T011, T012, T013 can run in parallel (different tables)

**User Stories:**
- US1 and US4 can be developed in parallel after Foundational
- US2 and US3 can be developed in parallel after US1 and US4

**Within User Story 1:**
- T030, T031 can run in parallel (different components)

---

## Parallel Example: Phase 2 Foundational

```bash
# Launch all table creation tasks in parallel:
Task: "Create clipboard_items table with indexes in database.rs"
Task: "Create glossary_entries table with indexes in database.rs"
Task: "Create translations table with indexes in database.rs"
Task: "Create user_settings table with defaults in database.rs"
```

## Parallel Example: User Story 1

```bash
# Launch TranslationPopup child components in parallel:
Task: "Create SourceText component in src/components/TranslationPopup/SourceText.tsx"
Task: "Create TranslatedText component in src/components/TranslationPopup/TranslatedText.tsx"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (**CRITICAL**)
3. Complete Phase 3: User Story 1 (번역)
4. **STOP and VALIDATE**: Cmd+CC → 팝업 → 번역 → 복사/바꾸기 동작 확인
5. Deploy/demo if ready

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add User Story 1 (번역) → MVP 배포 가능!
3. Add User Story 4 (GUI) → 완전한 앱 구조
4. Add User Story 2 (클립보드) → 생산성 기능 추가
5. Add User Story 3 (단어집) → 번역 정확도 향상
6. Polish → 최종 배포 준비

### Parallel Team Strategy

With 2+ developers:

1. Team completes Setup + Foundational together
2. After Foundational:
   - Developer A: User Story 1 (번역 - MVP)
   - Developer B: User Story 4 (GUI)
3. After US1 + US4:
   - Developer A: User Story 2 (클립보드)
   - Developer B: User Story 3 (단어집)
4. Both complete Polish together

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story
- Each user story is independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- MVP scope: Phase 1 + Phase 2 + Phase 3 (User Story 1)

---

## Summary

| Phase | Tasks | Description |
|-------|-------|-------------|
| Phase 1 | T001-T008 (8) | Setup |
| Phase 2 | T009-T019 (11) | Foundational |
| Phase 3 | T020-T038 (19) | US1 번역 (MVP) |
| Phase 4 | T039-T049 (11) | US4 GUI |
| Phase 5 | T050-T065 (16) | US2 클립보드 |
| Phase 6 | T066-T079 (14) | US3 단어집 |
| Phase 7 | T080-T088 (9) | Polish |
| **Total** | **88 tasks** | |
