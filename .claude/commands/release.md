# TransClip 릴리즈 스킬

이 스킬은 TransClip 앱의 새 버전을 릴리즈합니다.

## 입력

- `$ARGUMENTS`: 새 버전 번호 (예: 0.1.7) 또는 "patch", "minor", "major"

## 실행 단계

### 1. 현재 상태 확인

먼저 다음을 확인하세요:
- `git log` 또는 `gh release list`로 마지막 릴리즈 태그 확인
- `git log <last-tag>..HEAD --oneline`으로 릴리즈 이후 변경 사항 확인
- `src-tauri/tauri.conf.json`에서 현재 버전 확인

변경 사항이 없으면 "릴리즈할 변경 사항이 없습니다"라고 알리고 중단하세요.

### 2. 버전 결정

`$ARGUMENTS`가 제공된 경우:
- "patch": 현재 버전의 패치 버전 증가 (0.1.5 → 0.1.6)
- "minor": 현재 버전의 마이너 버전 증가 (0.1.5 → 0.2.0)
- "major": 현재 버전의 메이저 버전 증가 (0.1.5 → 1.0.0)
- 숫자 형식 (예: 0.1.7): 해당 버전 사용

`$ARGUMENTS`가 없으면 patch 버전을 증가시키세요.

### 3. 버전 파일 업데이트

다음 파일들의 버전을 새 버전으로 업데이트:
- `package.json`: `"version": "X.X.X"`
- `src-tauri/Cargo.toml`: `version = "X.X.X"`
- `src-tauri/tauri.conf.json`: `"version": "X.X.X"`
- `README.md`: 버전 배지와 다운로드 링크의 버전 번호들

### 4. 커밋 및 푸시

```bash
git add -A
git commit -m "chore: Bump version to X.X.X

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
git push origin main
```

### 5. 앱 빌드

```bash
pnpm tauri build
```

DMG 생성 실패는 무시하세요 (create-dmg 미설치). `.app` 파일만 있으면 됩니다.

### 6. ZIP 압축

```bash
cd src-tauri/target/release/bundle/macos
zip -r TransClip_X.X.X_aarch64.app.zip TransClip.app
```

### 7. GitHub 릴리즈 생성

`gh release create` 명령으로 릴리즈를 생성하세요.

릴리즈 노트 템플릿:

```markdown
## TransClip vX.X.X 릴리즈

### 변경 사항

[마지막 릴리즈 이후 커밋들을 분석하여 작성]
- feat: 커밋은 "새로운 기능" 섹션에
- fix: 커밋은 "버그 수정" 섹션에
- docs: 커밋은 "문서" 섹션에
- 기타는 적절한 섹션에

---

## 설치 방법

### 1. 다운로드

아래 `TransClip_X.X.X_aarch64.app.zip` 파일을 다운로드합니다.

### 2. 설치

1. 다운로드한 zip 파일의 압축을 해제합니다
2. `TransClip.app`을 **Applications** 폴더로 이동합니다
3. **처음 실행 전** 터미널에서 다음 명령어를 실행합니다:

\`\`\`bash
xattr -cr /Applications/TransClip.app
\`\`\`

4. 앱을 실행합니다

> ⚠️ **"손상됨" 오류 발생 시**: 위의 `xattr -cr` 명령어를 반드시 실행해주세요.

### 3. 권한 설정 (중요!)

- **접근성 권한**: 시스템 설정 > 개인 정보 보호 및 보안 > 접근성 > TransClip 허용
- **⚠️ 권한 설정 후 앱을 반드시 재시작해주세요!** (완전히 종료 후 다시 실행)
- 재시작하지 않으면 `Cmd+C` 두 번, `Cmd+D` 두 번 등의 단축키가 동작하지 않습니다

### 4. API 키 설정

1. [Anthropic Console](https://console.anthropic.com/)에서 Claude API 키 발급
2. 앱 설정에서 API 키 입력

---

## 시스템 요구사항

- macOS 12.0 (Monterey) 이상
- Apple Silicon (M1/M2/M3/M4)
- 인터넷 연결 필요
```

### 8. 완료 보고

릴리즈 URL을 사용자에게 알려주세요.
