# TransClip Testing Strategy (macOS-only)

이 프로젝트는 macOS에서만 서비스되므로, 테스트는 아래 3계층으로 운영합니다.

## 1) Unit Tests

목표: 순수 로직의 빠른 회귀 검증

- TypeScript: `src/utils`, `src/hooks`, `src/store`
- Rust: 프롬프트/언어 감지/모니터 계산/DB 정리 로직

실행:

```bash
pnpm test:run
pnpm test:rust
```

## 2) Integration Tests

목표: 화면 상태 전이 + 명령 경계(Tauri invoke/event) 검증

- `src/App.test.tsx`: 이벤트(`double_copy_detected`, `show_history`) 기준 팝업/히스토리 흐름 검증
- Rust 통합 단위: `src-tauri` 내부 command/helper 상호작용 검증

이 계층이 macOS에서 자동화 가능한 "사실상 상위 E2E 대체" 역할을 합니다.

## 3) E2E (Desktop Native)

Tauri v2 공식 문서 기준으로 **Desktop WebDriver는 macOS 미지원**입니다.

- 지원: Windows, Linux
- 미지원: macOS (WKWebView용 WebDriver 클라이언트 부재)

참고 문서(확인일: 2026-02-13):

- https://v2.tauri.app/develop/tests/
- https://v2.tauri.app/develop/tests/webdriver/

따라서 macOS-only 프로젝트에서는 아래 방식으로 운영합니다.

- 자동화: Unit + Integration을 CI에서 강제
- 수동: 릴리즈 전 macOS 수동 스모크 테스트 실행

## macOS 수동 스모크 테스트

개발 빌드:

```bash
pnpm tauri dev
```

릴리즈 빌드:

```bash
pnpm tauri build
```

체크 항목:

1. 앱 시작 후 접근성 권한 상태 표시/재시작 동작
2. `Cmd+C` 두 번으로 번역 팝업 표시
3. `Cmd+D` 두 번으로 윤문 팝업 표시
4. `Cmd+Shift+V`로 히스토리 패널 열기/닫기
5. 번역/윤문 결과 `Copy`, `Replace` 동작
6. Post-It 생성/수정 후 히스토리 반영

## CI/CD 운영 규칙

CI(`.github/workflows/ci.yml`):

```bash
pnpm ci:check
pnpm test:rust
pnpm lint:rust
```

Release(`.github/workflows/release.yml`):

- 태그(`v*`) 푸시 시 macOS 번들 생성
- DMG/APP 서명 아티팩트 업로드

## 향후 확장 (선택)

진짜 자동 Desktop E2E가 꼭 필요하면:

1. Linux 보조 러너를 추가해 `tauri-driver` E2E를 수행
2. macOS는 현재 전략(Unit+Integration+수동 스모크) 유지
