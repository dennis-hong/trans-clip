// ============================================
// Entity Types (from data-model.md)
// ============================================

/**
 * 클립보드에 복사된 개별 항목
 */
export interface ClipboardItem {
  id: string;
  content: string;
  contentPreview: string;
  copiedAt: string; // ISO 8601 format
  sourceApp?: string;
  isPinned: boolean;
  metadata?: {
    characterCount: number;
    wordCount: number;
  };
}

/**
 * 사용자 정의 번역 용어 사전 항목
 * 키워드와 설명을 저장하여 LLM이 번역 시 참고하도록 함
 */
export interface GlossaryEntry {
  id: string;
  keyword: string;      // 키워드 (영어든 한글이든 상관없음)
  description: string;  // 해당 키워드에 대한 설명 (번역 시 참고할 컨텍스트)
  createdAt: string;
  updatedAt: string;
  usageCount: number;
}

/**
 * 번역 결과 (캐시용)
 */
export interface Translation {
  id: string;
  sourceText: string;
  translatedText: string;
  sourceLanguage: Language;
  targetLanguage: Language;
  model: string;
  createdAt: string;
  glossaryUsed: string[];
  tokenUsage?: {
    inputTokens: number;
    outputTokens: number;
  };
}

/**
 * 사용자 설정
 */
export interface UserSettings {
  maxHistoryCount: number;
  preferredModel: ClaudeModel;
  autoDetectLanguage: boolean;
  doublePressInterval: number;
  translationCacheDays: number;
  showSourceApp: boolean;
  popupPosition: PopupPosition;
  launchAtLogin: boolean;
  pasteDelayMs: number;
}

// ============================================
// Enum Types
// ============================================

export type Language = "ko" | "en";
export type ClaudeModel = "claude-haiku-4-5-20251001" | "claude-sonnet-4-5-20250929" | "claude-opus-4-5-20251101";
export type PopupPosition = "cursor" | "center" | "top-right";

// ============================================
// Polish (Text Refinement) Types
// ============================================

/**
 * 다듬기 상황 (Context) - 글을 쓰는 목적과 대상
 */
export type PolishContext = 
  | "report-to-superior"   // 상사에게 보고
  | "team-announcement"    // 팀 공지
  | "peer-discussion"      // 동료와 논의
  | "external-formal"      // 외부 커뮤니케이션
  | "documentation";       // 문서 작성

/**
 * 다듬기 채널 (Channel) - 글을 작성하는 플랫폼/매체
 */
export type PolishChannel = 
  | "slack-message"        // 슬랙 메시지
  | "slack-thread"         // 슬랙 스레드
  | "confluence-wiki"      // 컨플루언스 위키
  | "jira-comment"         // Jira 코멘트
  | "jira-description"     // Jira 설명
  | "email"                // 이메일
  | "pr-description"       // PR 설명
  | "code-review";         // 코드 리뷰

/**
 * 다듬기 추가 옵션
 */
export type PolishOption = 
  | "shorter"              // 더 짧게
  | "longer"               // 더 자세하게
  | "bullet"               // 불릿으로 정리
  | "formal"               // 더 격식있게
  | "casual"               // 더 캐주얼하게
  | "action-clear";        // 액션 명확히

/**
 * 상황 정보
 */
export interface PolishContextInfo {
  id: PolishContext;
  name: string;
  description: string;
}

/**
 * 채널 정보
 */
export interface PolishChannelInfo {
  id: PolishChannel;
  name: string;
  description: string;
}

/**
 * 옵션 정보
 */
export interface PolishOptionInfo {
  id: PolishOption;
  name: string;
  description: string;
}

// ============================================
// API Response Types (from contracts/)
// ============================================

/**
 * 번역 응답
 */
export interface TranslateResponse {
  success: boolean;
  translatedText?: string;
  detectedLanguage?: Language;
  fromCache: boolean;
  glossaryApplied: string[];
  tokenUsage?: {
    inputTokens: number;
    outputTokens: number;
  };
  error?: TranslateError;
}

export interface TranslateError {
  code: "EMPTY_TEXT" | "TEXT_TOO_LONG" | "API_ERROR" | "NETWORK_ERROR" | "INVALID_API_KEY";
  message: string;
}

/**
 * 다듬기 응답
 */
export interface PolishResponse {
  success: boolean;
  polishedText?: string;
  detectedLanguage?: Language;
  tokenUsage?: {
    inputTokens: number;
    outputTokens: number;
  };
  error?: TranslateError;
}

/**
 * 클립보드 히스토리 응답
 */
export interface ClipboardHistoryResponse {
  items: ClipboardItem[];
  total: number;
  hasMore: boolean;
}

/**
 * 단어집 목록 응답
 */
export interface GlossaryListResponse {
  entries: GlossaryEntry[];
  total: number;
}

/**
 * API 키 상태
 */
export interface ApiKeyStatus {
  exists: boolean;
  isValid?: boolean;
  lastValidated?: string;
}

/**
 * API 키 설정 응답
 */
export interface SetApiKeyResponse {
  success: boolean;
  isValid: boolean;
  error?: {
    code: "INVALID_KEY" | "KEYCHAIN_ERROR";
    message: string;
  };
}

/**
 * 권한 상태
 */
export interface PermissionStatus {
  granted: boolean;
}

/**
 * 삭제 응답
 */
export interface DeleteResponse {
  success: boolean;
  error?: {
    code: string;
    message: string;
  };
}

/**
 * Response from clear_clipboard_history command
 */
export interface ClearHistoryResponse {
  success: boolean;
  deletedCount: number;
  error?: {
    code: string;
    message: string;
  };
}

/**
 * 붙여넣기 응답
 */
export interface PasteResponse {
  success: boolean;
  error?: {
    code: "ACCESSIBILITY_DENIED" | "PASTE_FAILED";
    message: string;
  };
}

/**
 * 고정 토글 응답
 */
export interface PinResponse {
  success: boolean;
  isPinned: boolean;
  error?: {
    code: "NOT_FOUND";
    message: string;
  };
}

/**
 * 단어집 가져오기 응답
 */
export interface ImportGlossaryResponse {
  imported: number;
  skipped: number;
  errors: Array<{
    line: number;
    message: string;
  }>;
}

/**
 * 단어집 내보내기 응답
 */
export interface ExportGlossaryResponse {
  success: boolean;
  exportedCount: number;
}

// ============================================
// Event Payload Types
// ============================================

/**
 * 클립보드 변경 이벤트 페이로드
 */
export interface ClipboardChangedPayload {
  item: ClipboardItem;
}

/**
 * 더블 복사 감지 이벤트 페이로드 (번역용)
 */
export interface DoubleCopyPayload {
  text: string;
  timestamp: string;
}

/**
 * 다듬기 감지 이벤트 페이로드
 */
export interface PolishPayload {
  text: string;
  timestamp: string;
}

/**
 * 히스토리 표시 이벤트 페이로드 (Cmd+Shift+V 또는 트레이 클릭)
 */
export interface ShowHistoryPayload {
  timestamp?: string;
}

/**
 * 번역 시작 이벤트 페이로드
 */
export interface TranslationStartedPayload {
  requestId: string;
}

/**
 * 번역 완료 이벤트 페이로드
 */
export interface TranslationCompletedPayload {
  requestId: string;
  result: TranslateResponse;
}

// ============================================
// Common Error Codes
// ============================================

export type ErrorCode =
  | "NETWORK_ERROR"
  | "API_ERROR"
  | "INVALID_API_KEY"
  | "ACCESSIBILITY_DENIED"
  | "NOT_FOUND"
  | "VALIDATION_FAILED"
  | "DATABASE_ERROR"
  | "KEYCHAIN_ERROR";

export interface AppError {
  code: ErrorCode;
  message: string;
}
