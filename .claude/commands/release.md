# TransClip 릴리즈 스킬

이 스킬은 **GitHub Actions 기반 자동 릴리즈**로 TransClip 새 버전을 배포합니다.

## 입력

- `$ARGUMENTS`: 새 버전 번호 (예: `0.1.15`) 또는 `patch` / `minor` / `major`

## 사전 준비 (최초 1회 또는 키 교체 시)

### 서명 키 생성

```bash
# 비밀번호를 반드시 명시적으로 지정 (빈 문자열 -p "" 사용 금지!)
# CI 환경변수가 설정되어 있으면 간섭하므로 CI= 로 해제
PASS="$(openssl rand -base64 24 | tr -d '\n')"
mkdir -p ~/.tauri
printf "%s" "$PASS" > ~/.tauri/transclip.key.pass
chmod 600 ~/.tauri/transclip.key.pass
CI= pnpm tauri signer generate -w ~/.tauri/transclip.key --ci -p "$PASS"
```

### 로컬 서명 검증

키 생성 후 반드시 로컬에서 서명이 동작하는지 확인합니다.

```bash
printf "test" > /tmp/sign-test.txt
PASS="$(<~/.tauri/transclip.key.pass)"
pnpm tauri signer sign -f ~/.tauri/transclip.key -p "$PASS" /tmp/sign-test.txt
rm /tmp/sign-test.txt /tmp/sign-test.txt.sig
```

실패하면 키를 재생성합니다 (`--force` 추가).

### 공개키 설정

```bash
cat ~/.tauri/transclip.key.pub
```

출력된 값을 `src-tauri/tauri.conf.json`의 `plugins.updater.pubkey`에 설정합니다.
`REPLACE_WITH_TAURI_UPDATER_PUBLIC_KEY` 상태면 워크플로가 실패합니다.

### GitHub Secrets 등록

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/transclip.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD < ~/.tauri/transclip.key.pass
```

> **주의**: `echo` 나 `--body ""`로 시크릿을 설정하면 줄바꿈이 포함되어 비밀번호 불일치가 발생합니다. 반드시 `< 파일` 또는 `printf` 파이프를 사용하세요.

등록 확인:

```bash
gh secret list
```

`TAURI_SIGNING_PRIVATE_KEY`와 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 두 개가 보여야 합니다.

### 워크플로 파일 확인

`.github/workflows/release.yml`이 기본 브랜치에 반영되어 있어야 합니다.

## 실행 단계

### 1) 현재 상태 확인

```bash
git fetch --tags
git tag --sort=-v:refname | head -5      # 마지막 태그 확인
git log <last-tag>..HEAD --oneline       # 변경사항 확인
```

`src-tauri/tauri.conf.json`에서 현재 버전도 확인합니다.

변경사항이 없으면 "릴리즈할 변경 사항이 없습니다"라고 알리고 중단합니다.

### 2) 사전 준비 상태 점검

태그 푸시 전에 반드시 아래를 확인합니다.

```bash
# 1. 서명 키 존재 확인
ls ~/.tauri/transclip.key ~/.tauri/transclip.key.pub ~/.tauri/transclip.key.pass

# 2. GitHub Secrets 설정 확인
gh secret list  # TAURI_SIGNING_PRIVATE_KEY, TAURI_SIGNING_PRIVATE_KEY_PASSWORD 존재 필수

# 3. 공개키가 플레이스홀더가 아닌지 확인
grep '"pubkey"' src-tauri/tauri.conf.json  # REPLACE_WITH... 이면 안 됨

# 4. 로컬 서명 테스트
printf "test" > /tmp/sign-test.txt
PASS="$(<~/.tauri/transclip.key.pass)"
pnpm tauri signer sign -f ~/.tauri/transclip.key -p "$PASS" /tmp/sign-test.txt
rm /tmp/sign-test.txt /tmp/sign-test.txt.sig
```

하나라도 실패하면 "사전 준비" 섹션을 따라 설정 후 재시도합니다.

### 3) 버전 결정

`$ARGUMENTS` 규칙:
- `patch`: 패치 버전 증가 (`0.1.12 -> 0.1.13`)
- `minor`: 마이너 버전 증가 (`0.1.12 -> 0.2.0`)
- `major`: 메이저 버전 증가 (`0.1.12 -> 1.0.0`)
- 숫자 형식 (`0.1.13`): 해당 버전 사용

입력이 없으면 `patch`를 사용합니다.

### 4) 버전 파일 업데이트

다음 파일 버전을 동일하게 맞춥니다.
- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `README.md` (배지 등 버전 표기가 있는 경우)

### 5) 커밋/푸시

```bash
git add -A
git commit -m "chore: bump version to X.X.X"
git push origin main
```

### 6) 릴리즈 태그 생성/푸시

```bash
git tag "vX.X.X"
git push origin "vX.X.X"
```

태그 푸시 시 GitHub Actions `release` 워크플로가 자동 실행됩니다.

### 7) 워크플로 검증

빌드는 약 **8분** 소요됩니다. 아래 순서로 폴링합니다.

```bash
# 실행 ID 확인
gh run list --workflow release --limit 1

# 초기 상태 확인 (검증 단계 통과 여부)
sleep 30 && gh run view <run-id> --json jobs --jq '.jobs[0].steps[] | .name + " → " + .status + " " + .conclusion'

# 빌드 중 주기적 확인 (2~3분 간격)
gh run view --job=<job-id>

# 완료 또는 실패 확인
gh run view <run-id> --json status,conclusion
```

> **주의**: `gh run watch`는 출력이 과도하고 장시간 대기합니다. 위 폴링 방식을 권장합니다.

### 8) 릴리즈 자산 확인

```bash
gh release view "vX.X.X" --json url,assets
```

아래 아티팩트가 릴리즈에 업로드되어야 합니다.
- `TransClip_X.X.X_aarch64.app.zip` (최초 설치용)
- `TransClip.app.tar.gz` (업데이트용)
- `TransClip.app.tar.gz.sig` (서명)
- `latest.json` (업데이터 메타데이터)

추가 검증:

```bash
curl -sSfL "https://github.com/dennis-hong/trans-clip/releases/latest/download/latest.json"
```

### 9) 릴리즈 노트 작성

자동 생성된 릴리즈 노트를 아래 템플릿으로 교체합니다.

```bash
gh release edit "vX.X.X" --notes "$(cat <<'NOTES'
## TransClip vX.X.X 릴리즈

### 변경 사항

#### 새로운 기능
* **기능 이름** — 설명

#### 버그 수정
* **수정 내용** — 설명

---

## 설치 방법

### 1. 자동 업데이트
이미 TransClip을 사용 중이라면 **설정 > 업데이트 확인**을 통해 자동으로 업데이트할 수 있습니다.

### 2. 수동 다운로드
아래 `TransClip_X.X.X_aarch64.app.zip` 파일을 다운로드합니다.

### 3. 설치
1. 다운로드한 zip 파일의 압축을 해제합니다
2. `TransClip.app`을 **Applications** 폴더로 이동합니다
3. **처음 실행 전** 터미널에서 다음 명령어를 실행합니다:
```
xattr -cr /Applications/TransClip.app
```
4. 앱을 실행합니다

> ⚠️ **"손상됨" 오류 발생 시**: 위의 `xattr -cr` 명령어를 반드시 실행해주세요.

### 4. 권한 설정 (중요!)
* **접근성 권한**: 시스템 설정 > 개인 정보 보호 및 보안 > 접근성 > TransClip 허용
* **⚠️ 권한 설정 후 앱을 반드시 재시작해주세요!** (완전히 종료 후 다시 실행)
* 재시작하지 않으면 `Cmd+C` 두 번, `Cmd+D` 두 번 등의 단축키가 동작하지 않습니다

### 5. API 키 설정
1. [Anthropic Console](https://console.anthropic.com/)에서 Claude API 키 발급
2. 앱 설정에서 API 키 입력

---

## 시스템 요구사항
* macOS 12.0 (Monterey) 이상
* Apple Silicon (M1/M2/M3/M4)
* 인터넷 연결 필요
NOTES
)"
```

`git log <prev-tag>..vX.X.X --oneline --no-merges`를 참고해 새로운 기능과 버그 수정을 정리합니다. `chore:` 커밋(버전 범프 등)은 릴리즈 노트에서 제외합니다.

### 10) 완료 보고

- 릴리즈 URL 전달
- 업로드된 아티팩트 목록 전달
- `latest.json` 엔드포인트 접근 확인 결과 전달

## 실패 복구

워크플로가 실패한 경우 아래 절차를 따릅니다.

### 원인 확인

```bash
gh run view --log-failed --job=<job-id> 2>&1 | tail -30
```

### 일반적인 실패 원인과 해결

| 에러 메시지 | 원인 | 해결 |
|---|---|---|
| `TAURI_SIGNING_PRIVATE_KEY secret is not set` | GitHub Secret 미등록 | "사전 준비 > GitHub Secrets 등록" 참조 |
| `Updater public key is not configured` | pubkey 플레이스홀더 상태 | `tauri.conf.json` pubkey 업데이트 |
| `incorrect updater private key password` | 비밀번호 불일치 | 키 재생성 후 Secret/pubkey 재동기화 |
| `Wrong password for that key` | 빈 비밀번호로 키 생성 후 빈 문자열이 아닌 값이 Secret에 등록됨 | 키 재생성 (명시적 비밀번호 사용) |

### 실패 후 재릴리즈

1. 실패한 태그/릴리즈를 정리합니다:

```bash
gh release delete vX.X.X --yes 2>/dev/null  # 릴리즈가 생성된 경우
git push origin :refs/tags/vX.X.X            # 원격 태그 삭제
git tag -d vX.X.X                            # 로컬 태그 삭제
```

2. 문제를 해결합니다 (키 재생성, Secret 재등록 등).
3. 설정 변경이 있으면 버전을 하나 더 올립니다 (예: `0.1.13` 실패 → `0.1.14`로 릴리즈).
4. "실행 단계"를 처음부터 다시 수행합니다.

> **주의**: 같은 태그를 삭제 후 재생성하면 GitHub 캐시 문제가 발생할 수 있으므로, 설정 변경이 포함된 경우 새 버전 번호 사용을 권장합니다.
