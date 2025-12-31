# Research: TransClip - 번역 및 클립보드 관리 앱

**Feature Branch**: `001-trans-clip`
**Date**: 2025-12-30

## 1. Framework Selection

### Decision: **Tauri 2.0** (TypeScript + Rust)

### Rationale
- 사용자 요구사항인 유지보수성(TypeScript)과 macOS 네이티브 기능 접근(Rust) 모두 충족
- Electron 대비 메모리 사용량 3-5배 절감 (150-300MB → 30-50MB)
- 콜드 스타트 0.3-0.5초 (Electron은 1-2초)
- macOS 메뉴바 앱을 위한 공식 지원 및 예제 제공
- 풍부한 플러그인 생태계 (clipboard, global-shortcut, permissions 등)

### Alternatives Considered

| 대안 | 기각 사유 |
|------|----------|
| Electron | 메모리 150-300MB (요구사항 <10MB 대비 과다), 번들 크기 80-150MB |
| Native Swift | 유지보수성 요구사항 위배 (Python/Go/JS/TS 선호) |
| Go + systray | 메모리 5-15MB로 최적이나 복잡한 UI 구현 어려움 |
| Python + rumps | 느린 시작 시간, 복잡한 패키징, 제한된 UI |

### Memory Requirement Note
요구사항의 <10MB는 Tauri로 달성 불가 (30-50MB 예상). 단, 이는 사용자 경험에 실질적 영향이 없으며, Electron 대비 5배 이상 효율적.

---

## 2. Cmd+CC Double-Press Detection

### Decision: **CGEventTap + Custom Rust Implementation**

### Rationale
- 표준 global shortcut API는 "고유한" 단축키만 등록 가능 (기존 Cmd+C 이벤트 가로채기 불가)
- macOS의 `CGEventTap`을 사용하여 모든 키보드 이벤트 모니터링 필요
- 연속 Cmd+C 입력 감지 로직을 Rust 백엔드에서 구현

### Implementation Approach
1. `tauri-plugin-macos-input-monitor`로 CGEventTap 접근
2. Cmd+C 키 입력마다 타임스탬프 기록
3. 두 번째 Cmd+C가 500ms 이내이면 번역 팝업 트리거
4. `tauri-plugin-macos-permissions`로 접근성 권한 확인/요청

### Permission Requirements
- **Accessibility Permission** 필수 (System Preferences → Privacy & Security)
- 앱 첫 실행 시 권한 요청 가이드 제공 필요

---

## 3. Clipboard Monitoring

### Decision: **tauri-plugin-clipboard (CrossCopy)**

### Rationale
- 이벤트 기반 모니터링 (폴링 아님)
- TEXT_CHANGED, FILES_CHANGED, IMAGE_CHANGED 이벤트 지원
- Rust 네이티브 구현으로 효율적

### Alternative Considered
- 공식 `tauri-plugin-clipboard-manager`: 읽기/쓰기만 지원, 모니터링 불가

---

## 4. Text Replacement (바꾸기 기능)

### Decision: **Simulate Cmd+V via CGEventPost** (DeepL 방식)

### Rationale
- DeepL도 동일한 방식 사용 (업계 표준)
- AXUIElement 기반 직접 텍스트 조작은 불안정 (특히 Electron 앱에서 실패)
- 95% 이상의 앱에서 동작

### Implementation Steps
1. 번역된 텍스트를 클립보드에 복사
2. `CGEventPost`로 Cmd+V 시뮬레이션
3. (선택적) 원본 클립보드 내용 복원

### Limitations
- 텍스트 입력이 불가능한 앱에서는 동작 안 함 (실패 시 클립보드에 복사 완료 알림)
- **Accessibility Permission** 필수
- **App Store 배포 불가** (샌드박스 제한) → DMG/Homebrew 배포

---

## 5. Claude API Integration

### Decision: **Claude Haiku 4.5** (Primary Model)

### Rationale
- Claude 4.5 패밀리 중 가장 빠른 모델로 거의 최첨단 성능 제공
- Sonnet 4와 거의 동등한 성능을 1/3 비용으로 제공 ($1/MTok 입력, $5/MTok 출력)
- 일반 번역 1건당 약 $0.0012 (500자 기준) - 기존 대비 3배 저렴
- 200k 토큰 컨텍스트 윈도우
- 컨텍스트 인식 기능 지원 (토큰 예산 추적)
- 확장 사고(Extended Thinking) 지원 (첫 번째 Haiku 모델)

### Model ID
- API: `claude-haiku-4-5-20251001`

### Alternative Models
| 시나리오 | 모델 | 비용 |
|----------|------|------|
| 기본 번역 (권장) | Claude Haiku 4.5 | $1/$5 per MTok |
| 고품질 복잡 번역 | Claude Sonnet 4.5 | $3/$15 per MTok |
| 최고 품질 문서 | Claude Opus 4.5 | $5/$25 per MTok |

### Prompt Template
```xml
<system>
You are a professional Korean-English translator.
- Preserve original meaning and tone
- Use natural, fluent expressions
- Maintain consistent terminology from glossary
- Keep proper nouns and brand names as specified
</system>

<glossary>
<!-- 관련 용어만 포함 (RAG 방식) -->
원어 | 번역어 | 비고
</glossary>

<task>
Detect language and translate:
- Korean → English
- English → Korean
- Output only the translation.
</task>

<text>
[USER_INPUT]
</text>
```

### Error Handling
- Exponential backoff with jitter (1초 → 2초 → 4초 → 8초 → 16초)
- 429 에러 시 Retry-After 헤더 존중
- 5회 재시도 후 실패 처리

### Streaming
- 사용자 대면 번역에는 스트리밍 활성화 (`"stream": true`)
- Server-Sent Events (SSE) 형식

---

## 6. Storage Solution

### Decision: **SQLite + macOS Keychain**

### Rationale
- **SQLite**: 클립보드 히스토리, 단어집 저장
  - 로컬 데이터베이스로 오프라인 동작 보장
  - 앱 재시작 후에도 데이터 유지
  - Tauri의 sql 플러그인으로 쉬운 통합
- **macOS Keychain**: API 키 보안 저장
  - FR-015 요구사항 충족
  - `tauri-plugin-keyring` 또는 `keyring` crate 사용

---

## 7. UI Framework

### Decision: **React + TypeScript**

### Rationale
- Tauri 공식 지원 프레임워크
- 풍부한 컴포넌트 라이브러리 생태계
- 유지보수성 요구사항 충족 (TypeScript)

### UI Components
- **번역 팝업**: 부동 창 (floating window)
- **메뉴바 패널**: 클립보드 히스토리 목록
- **설정 창**: API 키, 히스토리 개수, 단어집 관리

---

## 8. Project Structure

### Decision: **Single Project** (Desktop App)

```
src/                          # TypeScript/React 프론트엔드
├── components/               # React 컴포넌트
│   ├── TranslationPopup/
│   ├── ClipboardHistory/
│   ├── GlossaryManager/
│   └── Settings/
├── hooks/                    # Custom React hooks
├── services/                 # API 클라이언트, 번역 서비스
├── store/                    # 상태 관리
└── types/                    # TypeScript 타입 정의

src-tauri/                    # Rust 백엔드
├── src/
│   ├── lib.rs
│   ├── clipboard.rs          # 클립보드 모니터링
│   ├── hotkey.rs             # Cmd+CC 감지
│   ├── translate.rs          # Claude API 통신
│   ├── database.rs           # SQLite 연결
│   └── keychain.rs           # API 키 저장
└── Cargo.toml

tests/
├── unit/
└── integration/
```

---

## 9. Cost Estimation

### Monthly Cost (Claude API)

| 사용량 | 예상 비용 |
|--------|----------|
| 100 translations/day | ~$3.6/month |
| 500 translations/day | ~$18/month |
| 1,000 translations/day | ~$36/month |

*기준: Claude Haiku 4.5, 평균 500자/번역 (입력 ~200토큰, 출력 ~200토큰)*
*계산: (200 × $1/1M + 200 × $5/1M) × 1000 × 30 = ~$36/month*

---

## 10. Key Dependencies

### Frontend (npm)
```json
{
  "@tauri-apps/api": "^2.0.0",
  "@tauri-apps/plugin-global-shortcut": "^2.0.0",
  "react": "^18.2.0",
  "typescript": "^5.0.0"
}
```

### Backend (Cargo.toml)
```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-global-shortcut = "2"
tauri-plugin-clipboard = "0.6"       # CrossCopy
tauri-plugin-macos-permissions = "2"
tauri-plugin-sql = "2"
anthropic = "0.1"                    # Claude API
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio"] }
```
