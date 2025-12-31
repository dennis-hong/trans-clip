# TransClip 🌐

클립보드의 텍스트를 Claude AI를 사용해 자동으로 번역해주는 macOS 앱입니다.

![TransClip Screenshot](https://img.shields.io/badge/Platform-macOS-blue) ![Version](https://img.shields.io/badge/Version-0.1.0-green) ![License](https://img.shields.io/badge/License-MIT-yellow)

## ✨ 주요 기능

- 📋 **클립보드 모니터링** - 복사한 텍스트를 자동으로 감지
- 🔄 **자동 번역** - Claude AI를 활용한 고품질 번역
- 🌐 **다국어 지원** - 한국어, 영어, 일본어, 중국어 등 지원
- ⌨️ **글로벌 단축키** - 빠른 번역을 위한 단축키 설정
- 📚 **용어집 관리** - 자주 사용하는 용어 등록 및 일관된 번역
- 📜 **번역 히스토리** - 이전 번역 내용 확인 및 재사용

## 📥 다운로드 및 설치

### 1. 다운로드

[**📦 최신 버전 다운로드 (v0.1.0)**](https://github.com/dennis-hong/trans-clip/releases/latest)

위 링크에서 `TransClip_0.1.0_aarch64.app.zip` 파일을 다운로드하세요.

### 2. 설치 방법

1. 다운로드한 `TransClip_0.1.0_aarch64.app.zip` 파일의 압축을 해제합니다
2. `TransClip.app`을 **Applications** 폴더로 드래그하여 이동합니다
3. **처음 실행 시** 다음 보안 설정이 필요합니다:
   - `TransClip.app`을 더블클릭
   - "확인되지 않은 개발자" 경고가 나타나면:
     - **시스템 설정** > **개인 정보 보호 및 보안** 으로 이동
     - 하단의 "TransClip.app을 열도록 허용" 옆의 **확인 없이 열기** 클릭
   - 또는 `TransClip.app`을 우클릭 > **열기** 선택

### 3. 권한 설정

앱이 정상적으로 동작하려면 다음 권한이 필요합니다:

- **접근성 권한** (클립보드 모니터링용):
  - 시스템 설정 > 개인 정보 보호 및 보안 > 접근성
  - TransClip 앱을 허용 목록에 추가

## 🔑 Claude API 키 설정

TransClip은 Claude AI를 사용하여 번역합니다. API 키가 필요합니다:

1. [Anthropic Console](https://console.anthropic.com/)에서 API 키 발급
2. TransClip 앱 실행 후 **설정** 탭에서 API 키 입력
3. API 키는 macOS 키체인에 안전하게 저장됩니다

## 🎮 사용 방법

1. **번역할 텍스트 복사** - 웹페이지, 문서 등에서 텍스트를 복사 (Cmd+C)
2. **자동 번역** - 클립보드의 텍스트가 자동으로 감지되어 번역됩니다
3. **결과 확인** - 번역 결과가 팝업으로 표시됩니다
4. **복사하여 사용** - 번역된 텍스트를 클릭하여 복사

### 단축키

| 기능 | 단축키 |
|------|--------|
| 번역 팝업 표시/숨기기 | `Cmd + Shift + T` |

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
