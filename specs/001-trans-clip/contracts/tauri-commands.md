# Tauri Commands Contract: TransClip

**Feature Branch**: `001-trans-clip`
**Date**: 2025-12-30

이 문서는 Tauri의 IPC (Inter-Process Communication) 커맨드 인터페이스를 정의합니다.
프론트엔드(TypeScript)와 백엔드(Rust) 간의 계약입니다.

---

## 1. Translation Commands

### translate

선택된 텍스트를 번역합니다.

```typescript
// Request
invoke<TranslateResponse>('translate', {
  text: string;           // 번역할 텍스트 (필수, 1-10000자)
  sourceLanguage?: 'ko' | 'en' | 'auto';  // 원문 언어 (기본: auto)
  targetLanguage?: 'ko' | 'en';           // 대상 언어 (sourceLanguage 기반 자동 결정)
});

// Response
interface TranslateResponse {
  success: boolean;
  translatedText?: string;
  detectedLanguage?: 'ko' | 'en';
  fromCache: boolean;           // 캐시에서 가져왔는지
  glossaryApplied: string[];    // 적용된 단어집 ID
  tokenUsage?: {
    inputTokens: number;
    outputTokens: number;
  };
  error?: TranslateError;
}

interface TranslateError {
  code: 'EMPTY_TEXT' | 'TEXT_TOO_LONG' | 'API_ERROR' | 'NETWORK_ERROR' | 'INVALID_API_KEY';
  message: string;
}
```

---

## 2. Clipboard Commands

### get_clipboard_history

클립보드 히스토리 목록을 조회합니다.

```typescript
// Request
invoke<ClipboardHistoryResponse>('get_clipboard_history', {
  limit?: number;         // 조회 개수 (기본: 50, 최대: 200)
  offset?: number;        // 페이지네이션 오프셋 (기본: 0)
  searchQuery?: string;   // 검색어 (선택적)
});

// Response
interface ClipboardHistoryResponse {
  items: ClipboardItem[];
  total: number;
  hasMore: boolean;
}

interface ClipboardItem {
  id: string;
  content: string;
  contentPreview: string;
  copiedAt: string;       // ISO 8601 형식
  sourceApp?: string;
  isPinned: boolean;
}
```

### delete_clipboard_item

클립보드 항목을 삭제합니다.

```typescript
// Request
invoke<DeleteResponse>('delete_clipboard_item', {
  id: string;             // 삭제할 항목 ID
});

// Response
interface DeleteResponse {
  success: boolean;
  error?: {
    code: 'NOT_FOUND' | 'DELETE_FAILED';
    message: string;
  };
}
```

### toggle_pin_clipboard_item

클립보드 항목의 고정 상태를 토글합니다.

```typescript
// Request
invoke<PinResponse>('toggle_pin_clipboard_item', {
  id: string;
});

// Response
interface PinResponse {
  success: boolean;
  isPinned: boolean;      // 변경 후 상태
  error?: {
    code: 'NOT_FOUND';
    message: string;
  };
}
```

### set_clipboard

지정된 텍스트를 클립보드에 설정합니다.

```typescript
// Request
invoke<void>('set_clipboard', {
  text: string;
});

// Response: void (성공 시) 또는 에러 throw
```

### paste_text

텍스트를 현재 커서 위치에 붙여넣습니다 (Cmd+V 시뮬레이션).

```typescript
// Request
invoke<PasteResponse>('paste_text', {
  text: string;
});

// Response
interface PasteResponse {
  success: boolean;
  error?: {
    code: 'ACCESSIBILITY_DENIED' | 'PASTE_FAILED';
    message: string;
  };
}
```

---

## 3. Glossary Commands

### get_glossary_entries

단어집 항목 목록을 조회합니다.

```typescript
// Request
invoke<GlossaryListResponse>('get_glossary_entries', {
  sourceLanguage?: 'ko' | 'en';  // 필터링 (선택적)
  searchQuery?: string;          // 검색어 (선택적)
  sortBy?: 'sourceText' | 'usageCount' | 'createdAt';  // 정렬 기준
  sortOrder?: 'asc' | 'desc';
});

// Response
interface GlossaryListResponse {
  entries: GlossaryEntry[];
  total: number;
}

interface GlossaryEntry {
  id: string;
  sourceText: string;
  targetText: string;
  sourceLanguage: 'ko' | 'en';
  targetLanguage: 'ko' | 'en';
  note?: string;
  createdAt: string;
  updatedAt: string;
  usageCount: number;
}
```

### add_glossary_entry

단어집에 새 항목을 추가합니다.

```typescript
// Request
invoke<GlossaryEntry>('add_glossary_entry', {
  sourceText: string;       // 원어 (1-100자)
  targetText: string;       // 번역어 (1-200자)
  sourceLanguage: 'ko' | 'en';
  targetLanguage: 'ko' | 'en';
  note?: string;
});

// Response: GlossaryEntry (생성된 항목)

// Errors
interface GlossaryError {
  code: 'DUPLICATE_ENTRY' | 'VALIDATION_FAILED' | 'SAME_LANGUAGE';
  message: string;
}
```

### update_glossary_entry

단어집 항목을 수정합니다.

```typescript
// Request
invoke<GlossaryEntry>('update_glossary_entry', {
  id: string;
  targetText?: string;      // 번역어 수정
  note?: string;            // 메모 수정
});

// Response: GlossaryEntry (수정된 항목)
```

### delete_glossary_entry

단어집 항목을 삭제합니다.

```typescript
// Request
invoke<DeleteResponse>('delete_glossary_entry', {
  id: string;
});

// Response
interface DeleteResponse {
  success: boolean;
}
```

### import_glossary

CSV/JSON 파일에서 단어집을 가져옵니다.

```typescript
// Request
invoke<ImportResponse>('import_glossary', {
  filePath: string;
  format: 'csv' | 'json';
  overwrite: boolean;       // 기존 항목 덮어쓰기
});

// Response
interface ImportResponse {
  imported: number;
  skipped: number;
  errors: Array<{
    line: number;
    message: string;
  }>;
}
```

### export_glossary

단어집을 파일로 내보냅니다.

```typescript
// Request
invoke<ExportResponse>('export_glossary', {
  filePath: string;
  format: 'csv' | 'json';
});

// Response
interface ExportResponse {
  success: boolean;
  exportedCount: number;
}
```

---

## 4. Settings Commands

### get_settings

현재 설정을 조회합니다.

```typescript
// Request
invoke<UserSettings>('get_settings');

// Response
interface UserSettings {
  maxHistoryCount: number;
  preferredModel: string;
  autoDetectLanguage: boolean;
  doublePressInterval: number;
  translationCacheDays: number;
  showSourceApp: boolean;
  popupPosition: 'cursor' | 'center' | 'top-right';
  launchAtLogin: boolean;
}
```

### update_settings

설정을 변경합니다.

```typescript
// Request
invoke<UserSettings>('update_settings', {
  settings: Partial<UserSettings>;
});

// Response: UserSettings (변경된 전체 설정)
```

### get_api_key

API 키 존재 여부를 확인합니다 (키 값은 반환하지 않음).

```typescript
// Request
invoke<ApiKeyStatus>('get_api_key');

// Response
interface ApiKeyStatus {
  exists: boolean;
  isValid?: boolean;        // 마지막 검증 결과 (선택적)
  lastValidated?: string;   // 마지막 검증 시간
}
```

### set_api_key

Claude API 키를 저장합니다 (Keychain에 저장).

```typescript
// Request
invoke<SetApiKeyResponse>('set_api_key', {
  apiKey: string;
});

// Response
interface SetApiKeyResponse {
  success: boolean;
  isValid: boolean;         // API 키 유효성 검증 결과
  error?: {
    code: 'INVALID_KEY' | 'KEYCHAIN_ERROR';
    message: string;
  };
}
```

### delete_api_key

저장된 API 키를 삭제합니다.

```typescript
// Request
invoke<DeleteResponse>('delete_api_key');

// Response
interface DeleteResponse {
  success: boolean;
}
```

---

## 5. System Commands

### check_accessibility_permission

접근성 권한 상태를 확인합니다.

```typescript
// Request
invoke<PermissionStatus>('check_accessibility_permission');

// Response
interface PermissionStatus {
  granted: boolean;
}
```

### request_accessibility_permission

접근성 권한을 요청합니다 (시스템 설정 창 열기).

```typescript
// Request
invoke<void>('request_accessibility_permission');

// Response: void (시스템 설정 창이 열림)
```

### show_translation_popup

번역 팝업 창을 표시합니다.

```typescript
// Request
invoke<void>('show_translation_popup', {
  text: string;             // 번역할 텍스트
  position?: {              // 팝업 위치 (선택적, 설정에 따라)
    x: number;
    y: number;
  };
});
```

### hide_translation_popup

번역 팝업 창을 숨깁니다.

```typescript
// Request
invoke<void>('hide_translation_popup');
```

---

## 6. Events (Backend → Frontend)

Tauri 이벤트 시스템을 통해 백엔드에서 프론트엔드로 전송됩니다.

### clipboard_changed

클립보드 내용이 변경되었을 때 발생합니다.

```typescript
listen<ClipboardChangedPayload>('clipboard_changed', (event) => {
  // Handle event
});

interface ClipboardChangedPayload {
  item: ClipboardItem;
}
```

### double_copy_detected

Cmd+CC (더블 프레스)가 감지되었을 때 발생합니다.

```typescript
listen<DoubleCopyPayload>('double_copy_detected', (event) => {
  // Show translation popup
});

interface DoubleCopyPayload {
  text: string;             // 클립보드에 복사된 텍스트
  timestamp: string;
}
```

### translation_started / translation_completed

번역 진행 상태를 알립니다.

```typescript
listen<TranslationProgressPayload>('translation_started', (event) => {
  // Show loading indicator
});

listen<TranslationCompletePayload>('translation_completed', (event) => {
  // Update UI with result
});

interface TranslationProgressPayload {
  requestId: string;
}

interface TranslationCompletePayload {
  requestId: string;
  result: TranslateResponse;
}
```

---

## Error Handling

모든 커맨드는 에러 발생 시 Tauri의 에러 시스템을 통해 전파됩니다.

```typescript
try {
  const result = await invoke('some_command', { ... });
} catch (error) {
  // error는 { code: string, message: string } 형태
  if (error.code === 'NETWORK_ERROR') {
    // Handle network error
  }
}
```

### Common Error Codes

| Code | Description |
|------|-------------|
| `NETWORK_ERROR` | 네트워크 연결 실패 |
| `API_ERROR` | Claude API 오류 |
| `INVALID_API_KEY` | API 키가 유효하지 않음 |
| `ACCESSIBILITY_DENIED` | 접근성 권한 없음 |
| `NOT_FOUND` | 요청한 리소스를 찾을 수 없음 |
| `VALIDATION_FAILED` | 입력값 검증 실패 |
| `DATABASE_ERROR` | SQLite 작업 실패 |
| `KEYCHAIN_ERROR` | Keychain 접근 실패 |
