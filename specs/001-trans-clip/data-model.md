# Data Model: TransClip

**Feature Branch**: `001-trans-clip`
**Date**: 2025-12-30

## Overview

TransClip 앱의 데이터 모델은 4개의 핵심 엔티티로 구성됩니다:
1. **ClipboardItem** - 클립보드 히스토리 항목
2. **GlossaryEntry** - 사용자 정의 단어집 항목
3. **Translation** - 번역 요청/결과 (캐시 목적)
4. **UserSettings** - 앱 설정

## Entity Definitions

### 1. ClipboardItem

클립보드에 복사된 개별 항목을 나타냅니다.

```typescript
interface ClipboardItem {
  id: string;              // UUID, Primary Key
  content: string;         // 복사된 텍스트 내용 (최대 10,000자)
  contentPreview: string;  // 미리보기용 텍스트 (처음 100자)
  copiedAt: Date;          // 복사 시간
  sourceApp?: string;      // 복사된 앱 이름 (선택적)
  isPinned: boolean;       // 고정 여부 (삭제 방지)
  metadata?: {
    characterCount: number;
    wordCount: number;
  };
}
```

**Validation Rules**:
- `content`: 빈 문자열 불가, 최대 10,000자
- `copiedAt`: 현재 시간 이후 불가
- 중복 `content`는 기존 항목 업데이트 (copiedAt만 갱신)

**State Transitions**:
- Created → Active (복사 시)
- Active → Pinned (고정 시)
- Pinned → Active (고정 해제 시)
- Active → Deleted (삭제 또는 히스토리 초과 시)

---

### 2. GlossaryEntry

사용자 정의 번역 용어 사전 항목입니다.

```typescript
interface GlossaryEntry {
  id: string;              // UUID, Primary Key
  sourceText: string;      // 원어 (검색 대상)
  targetText: string;      // 번역어
  sourceLanguage: 'ko' | 'en';  // 원어 언어
  targetLanguage: 'ko' | 'en';  // 번역어 언어
  note?: string;           // 사용 메모 (선택적)
  createdAt: Date;
  updatedAt: Date;
  usageCount: number;      // 사용 횟수 (통계용)
}
```

**Validation Rules**:
- `sourceText`: 빈 문자열 불가, 최대 100자
- `targetText`: 빈 문자열 불가, 최대 200자
- `sourceLanguage` !== `targetLanguage` 필수
- (`sourceText`, `sourceLanguage`) 조합은 유니크

**Index**:
- `sourceText` + `sourceLanguage`: 번역 시 빠른 검색용

---

### 3. Translation

번역 결과를 캐싱하여 동일 텍스트 재번역 시 API 호출을 줄입니다.

```typescript
interface Translation {
  id: string;              // UUID, Primary Key
  sourceText: string;      // 원문
  translatedText: string;  // 번역문
  sourceLanguage: 'ko' | 'en';
  targetLanguage: 'ko' | 'en';
  model: string;           // 사용된 Claude 모델 (예: claude-haiku-4-5)
  createdAt: Date;
  glossaryUsed: string[];  // 적용된 단어집 ID 목록
  tokenUsage?: {
    inputTokens: number;
    outputTokens: number;
  };
}
```

**Validation Rules**:
- `sourceText`: 빈 문자열 불가
- `sourceLanguage` !== `targetLanguage` 필수
- 캐시 유효 기간: 7일 (설정 가능)

**Index**:
- `sourceText` + `sourceLanguage` + `targetLanguage`: 캐시 조회용

---

### 4. UserSettings

앱 설정을 저장합니다. API 키는 별도로 Keychain에 저장됩니다.

```typescript
interface UserSettings {
  id: 'default';           // 단일 설정, 상수 ID

  // 클립보드 설정
  maxHistoryCount: number; // 최대 히스토리 개수 (기본: 50, 범위: 10-200)

  // 번역 설정
  preferredModel: 'claude-haiku-4-5' | 'claude-sonnet-4-5' | 'claude-opus-4-5';
  autoDetectLanguage: boolean;  // 자동 언어 감지 (기본: true)

  // 단축키 설정
  doublePressInterval: number;  // Cmd+CC 감지 간격 (ms, 기본: 500)

  // 캐시 설정
  translationCacheDays: number; // 번역 캐시 유효 기간 (일, 기본: 7)

  // UI 설정
  showSourceApp: boolean;       // 소스 앱 표시 (기본: true)
  popupPosition: 'cursor' | 'center' | 'top-right';  // 팝업 위치 (기본: cursor)

  // 시스템
  launchAtLogin: boolean;       // 로그인 시 자동 실행 (기본: false)

  updatedAt: Date;
}
```

**Validation Rules**:
- `maxHistoryCount`: 10 ≤ value ≤ 200
- `doublePressInterval`: 200 ≤ value ≤ 1000
- `translationCacheDays`: 1 ≤ value ≤ 30

---

## Database Schema (SQLite)

```sql
-- 클립보드 히스토리
CREATE TABLE clipboard_items (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    content_preview TEXT NOT NULL,
    copied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    source_app TEXT,
    is_pinned INTEGER NOT NULL DEFAULT 0,
    character_count INTEGER,
    word_count INTEGER
);

CREATE INDEX idx_clipboard_copied_at ON clipboard_items(copied_at DESC);
CREATE INDEX idx_clipboard_content ON clipboard_items(content);

-- 단어집
CREATE TABLE glossary_entries (
    id TEXT PRIMARY KEY,
    source_text TEXT NOT NULL,
    target_text TEXT NOT NULL,
    source_language TEXT NOT NULL CHECK(source_language IN ('ko', 'en')),
    target_language TEXT NOT NULL CHECK(target_language IN ('ko', 'en')),
    note TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    usage_count INTEGER NOT NULL DEFAULT 0,
    UNIQUE(source_text, source_language)
);

CREATE INDEX idx_glossary_source ON glossary_entries(source_text, source_language);

-- 번역 캐시
CREATE TABLE translations (
    id TEXT PRIMARY KEY,
    source_text TEXT NOT NULL,
    translated_text TEXT NOT NULL,
    source_language TEXT NOT NULL CHECK(source_language IN ('ko', 'en')),
    target_language TEXT NOT NULL CHECK(target_language IN ('ko', 'en')),
    model TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    glossary_used TEXT,  -- JSON array of glossary IDs
    input_tokens INTEGER,
    output_tokens INTEGER
);

CREATE INDEX idx_translation_lookup ON translations(source_text, source_language, target_language);

-- 사용자 설정
CREATE TABLE user_settings (
    id TEXT PRIMARY KEY DEFAULT 'default',
    max_history_count INTEGER NOT NULL DEFAULT 50,
    preferred_model TEXT NOT NULL DEFAULT 'claude-haiku-4-5',
    auto_detect_language INTEGER NOT NULL DEFAULT 1,
    double_press_interval INTEGER NOT NULL DEFAULT 500,
    translation_cache_days INTEGER NOT NULL DEFAULT 7,
    show_source_app INTEGER NOT NULL DEFAULT 1,
    popup_position TEXT NOT NULL DEFAULT 'cursor',
    launch_at_login INTEGER NOT NULL DEFAULT 0,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

---

## Relationships

```
┌─────────────────┐         ┌─────────────────┐
│ ClipboardItem   │         │ GlossaryEntry   │
│                 │         │                 │
│ - id (PK)       │         │ - id (PK)       │
│ - content       │         │ - sourceText    │
│ - copiedAt      │         │ - targetText    │
│ - ...           │         │ - ...           │
└─────────────────┘         └────────┬────────┘
                                     │
                                     │ uses (0..N)
                                     ▼
┌─────────────────┐         ┌─────────────────┐
│ UserSettings    │         │ Translation     │
│                 │         │                 │
│ - id (PK)       │────────▶│ - id (PK)       │
│ - preferredModel│ applies │ - sourceText    │
│ - ...           │         │ - glossaryUsed  │
└─────────────────┘         │ - ...           │
                            └─────────────────┘
```

- **Translation ↔ GlossaryEntry**: N:N 관계. Translation.glossaryUsed에 사용된 GlossaryEntry ID 저장
- **UserSettings → Translation**: UserSettings.preferredModel이 Translation.model 결정

---

## Data Lifecycle

### ClipboardItem
1. **생성**: Cmd+C 감지 시 자동 생성
2. **갱신**: 동일 내용 재복사 시 `copiedAt` 업데이트
3. **삭제**:
   - 수동 삭제 (사용자 액션)
   - 자동 삭제 (`maxHistoryCount` 초과 시 가장 오래된 항목부터)
   - 고정된 항목(`isPinned=true`)은 자동 삭제 제외

### GlossaryEntry
1. **생성**: 사용자가 단어집에 추가
2. **갱신**: 번역어 수정, 사용 시 `usageCount` 증가
3. **삭제**: 사용자 수동 삭제만 가능

### Translation (Cache)
1. **생성**: 번역 API 호출 완료 시
2. **조회**: 동일 텍스트 번역 요청 시 캐시 우선 사용
3. **만료**: `translationCacheDays` 이후 자동 삭제 (백그라운드 정리)
