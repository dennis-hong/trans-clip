# TransClip 🌐

클립보드의 텍스트를 Claude AI를 사용해 자동으로 번역해주는 macOS 앱입니다.

![TransClip Screenshot](https://img.shields.io/badge/Platform-macOS-blue) ![Version](https://img.shields.io/badge/Version-0.1.17-green) ![License](https://img.shields.io/badge/License-MIT-yellow)

## ✨ 주요 기능

- 🔄 **즉시 번역** - 텍스트 선택 후 `Cmd+C` 두 번으로 바로 번역
- ✨ **윤문(Polish)** - 텍스트 선택 후 `Cmd+D` 두 번으로 문법/어조/명확성 개선
- 📋 **클립보드 히스토리** - `Cmd+Shift+V`로 앱 창 표시 및 복사 기록 관리
- 📝 **Post-It 메모** - 히스토리 패널에서 빠른 메모 작성
- 📚 **용어집 관리** - 자주 사용하는 용어 등록으로 일관된 번역
- 🌐 **다국어 지원** - 한국어, 영어, 일본어, 중국어 등 자동 감지

## 📥 다운로드 및 설치

### 1. 다운로드

[**📦 최신 버전 다운로드**](https://github.com/dennis-hong/trans-clip/releases/latest)

위 링크에서 `TransClip_<version>_aarch64.app.zip` 파일을 다운로드하세요.

### 2. 설치 방법

1. 다운로드한 `TransClip_<version>_aarch64.app.zip` 파일의 압축을 해제합니다
2. `TransClip.app`을 **Applications** 폴더로 드래그하여 이동합니다
3. 실행 시 macOS가 앱을 차단하면 터미널에서 다음 명령어를 실행합니다 (quarantine 속성 제거):

```bash
xattr -cr /Applications/TransClip.app
```

4. 이제 앱을 더블클릭하여 실행할 수 있습니다
5. 설정 화면의 **업데이트 확인** 버튼으로 최신 버전을 확인할 수 있습니다

> ⚠️ **"수정 또는 손상됨" 오류가 발생하는 경우**: 위의 `xattr -cr` 명령어를 반드시 실행해주세요. 이 앱은 개인 개발자가 만든 앱으로 Apple 공증을 받지 않았기 때문에 macOS가 차단합니다.

### 2-1. 자동 업데이트

- 최초 설치 후에는 앱 내부에서 새 버전을 확인하고 설치할 수 있습니다
- 업데이트가 있으면 설정 화면 하단에 `새 버전` 배지가 표시되고 `업데이트` 버튼이 활성화됩니다
- 업데이트 설치가 완료되면 앱이 재시작되며 최신 버전이 적용됩니다
- 인앱 업데이트 경로로 설치된 경우에는 보통 `xattr -cr`를 매 업데이트마다 다시 실행할 필요가 없습니다

### 3. 권한 설정

앱이 정상적으로 동작하려면 다음 권한이 필요합니다:

- **접근성 권한** (단축키 감지 및 클립보드 모니터링용):
  - 시스템 설정 > 개인 정보 보호 및 보안 > 접근성
  - TransClip 앱을 허용 목록에 추가
  - **⚠️ 권한 설정 후 앱을 반드시 재시작해주세요!** (완전히 종료 후 다시 실행)
  - 재시작하지 않으면 `Cmd+C` 두 번, `Cmd+D` 두 번 등의 단축키가 동작하지 않습니다

## 🔑 Claude API 키 설정

TransClip은 Claude AI를 사용하여 번역합니다. API 키가 필요합니다:

1. [Anthropic Console](https://console.anthropic.com/)에서 API 키 발급
2. TransClip 앱 실행 후 **설정** 탭에서 API 키 입력
3. API 키는 macOS 키체인에 안전하게 저장됩니다

## 🎮 사용 방법

### 번역하기

1. 웹페이지나 문서에서 **번역할 텍스트를 선택**
2. **`Cmd+C`를 빠르게 두 번** 누르기 (500ms 이내)
3. 번역 팝업이 자동으로 나타나고 번역이 시작됩니다
4. 결과 활용:
   - `Cmd+Enter` 또는 **Replace**: 원문을 번역문으로 교체하고 자동 붙여넣기
   - **Copy**: 번역문을 클립보드에 복사
   - `Escape`: 팝업 닫기

### 윤문(Polish)하기

1. **다듬을 텍스트를 선택**
2. **`Cmd+D`를 빠르게 두 번** 누르기 (500ms 이내)
3. 윤문 팝업이 나타나고 텍스트가 개선됩니다
4. 옵션 선택 가능:
   - **Context**: Technical, Academic, Business, Creative, Casual
   - **Channel**: Email, Blog, Social
   - **Options**: Fix Grammar, Improve Clarity, Improve Tone

### 클립보드 히스토리

1. **`Cmd+Shift+V`** 또는 메뉴바 아이콘 클릭으로 앱 창 표시
2. 최근 복사한 항목들이 표시됩니다
3. `Escape`로 창을 숨기고, 다시 `Cmd+Shift+V`로 불러올 수 있습니다
4. 숫자 키로 빠르게 선택:
   - `1-9`: 해당 항목을 클립보드에 복사
   - `Shift+숫자`: 해당 항목 번역
   - `Ctrl+숫자`: 해당 항목 윤문
5. `Cmd+N`: 새 Post-It 메모 작성

### 단축키 요약

#### 글로벌 단축키 (어디서든 사용 가능)

| 기능 | 단축키 |
|------|--------|
| 번역 팝업 | `Cmd+C` 두 번 (500ms 이내) |
| 윤문 팝업 | `Cmd+D` 두 번 (500ms 이내) |
| 앱 창 표시 / 클립보드 히스토리 | `Cmd+Shift+V` |

#### 팝업 내 단축키

| 기능 | 단축키 |
|------|--------|
| 결과로 교체 (자동 붙여넣기) | `Cmd+Enter` |
| 팝업 닫기 | `Escape` |

#### 히스토리 패널 단축키

| 기능 | 단축키 |
|------|--------|
| 항목 복사 | `1-9` |
| 항목 번역 | `Shift+1-9` |
| 항목 윤문 | `Ctrl+1-9` |
| 모니터 이동 (다중 모니터) | `Alt/Option+1-5` |
| 새 Post-It 메모 | `Cmd+N` |
| 패널 닫기 | `Escape` |

## 💻 시스템 요구사항

- **macOS**: 12.0 (Monterey) 이상
- **칩셋**: Apple Silicon (M1/M2/M3/M4)
- **인터넷 연결**: Claude API 호출에 필요

> ⚠️ **참고**: 현재 Apple Silicon 버전만 제공됩니다. Intel Mac 지원이 필요하시면 Issue를 남겨주세요.

## 🛠️ 개발자용 빌드

로컬에서 직접 빌드하려면:

```bash
# 저장소 클론
git clone https://github.com/dennis-hong/trans-clip.git
cd trans-clip

# 의존성 설치
pnpm install

# 개발 모드 실행
pnpm tauri dev

# 프로덕션 빌드
pnpm tauri build
```

### 필수 도구

- [Node.js](https://nodejs.org/) 18+
- [pnpm](https://pnpm.io/)
- [Rust](https://www.rust-lang.org/)
- [Tauri CLI](https://tauri.app/)

## 🧪 테스트 & CI

로컬 기본 검증:

```bash
pnpm ci:check
pnpm test:rust
pnpm lint:rust
```

자세한 macOS 전용 테스트 전략(Unit → Integration → 수동 스모크)은 `docs/testing-strategy.md`를 참고하세요.

## 📄 라이선스

MIT License

## 🤝 기여하기

이슈와 PR은 언제나 환영합니다!

1. 이 저장소를 Fork
2. 기능 브랜치 생성 (`git checkout -b feature/AmazingFeature`)
3. 변경사항 커밋 (`git commit -m 'Add some AmazingFeature'`)
4. 브랜치에 Push (`git push origin feature/AmazingFeature`)
5. Pull Request 생성

---

Made with ❤️ using [Tauri](https://tauri.app/) + [React](https://reactjs.org/) + [Claude AI](https://www.anthropic.com/)
