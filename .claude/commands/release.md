# TransClip 릴리즈 스킬

이 스킬은 **GitHub Actions 기반 자동 릴리즈**로 TransClip 새 버전을 배포합니다.

## 입력

- `$ARGUMENTS`: 새 버전 번호 (예: `0.1.13`) 또는 `patch` / `minor` / `major`

## 사전 준비 (최초 1회 또는 키 교체 시)

1. 업데이터 서명 키를 준비합니다.
   - 필요 시 생성: `pnpm tauri signer generate -w ~/.tauri/transclip.key`
2. `src-tauri/tauri.conf.json`의 `plugins.updater.pubkey`를 실제 공개키로 설정합니다.
   - `REPLACE_WITH_TAURI_UPDATER_PUBLIC_KEY` 상태면 워크플로가 실패합니다.
3. GitHub Repository Secrets에 아래 값을 등록합니다.
   - `TAURI_SIGNING_PRIVATE_KEY`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (비밀번호를 사용한 경우)
4. `.github/workflows/release.yml`이 기본 브랜치에 반영되어 있어야 합니다.

## 실행 단계

### 1) 현재 상태 확인

- `git fetch --tags`
- `gh release list` 또는 `git tag --sort=-v:refname | head`로 마지막 태그 확인
- `git log <last-tag>..HEAD --oneline`으로 릴리즈 이후 변경사항 확인
- `src-tauri/tauri.conf.json` 현재 버전 확인

변경사항이 없으면 "릴리즈할 변경 사항이 없습니다"라고 알리고 중단합니다.

### 2) 버전 결정

`$ARGUMENTS` 규칙:
- `patch`: 패치 버전 증가 (`0.1.12 -> 0.1.13`)
- `minor`: 마이너 버전 증가 (`0.1.12 -> 0.2.0`)
- `major`: 메이저 버전 증가 (`0.1.12 -> 1.0.0`)
- 숫자 형식 (`0.1.13`): 해당 버전 사용

입력이 없으면 `patch`를 사용합니다.

### 3) 버전 파일 업데이트

다음 파일 버전을 동일하게 맞춥니다.
- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `README.md` (배지 등 버전 표기가 있는 경우)

### 4) 커밋/푸시

```bash
git add -A
git commit -m "chore: bump version to X.X.X"
git push origin main
```

### 5) 릴리즈 태그 생성/푸시

```bash
git tag "vX.X.X"
git push origin "vX.X.X"
```

태그 푸시 시 GitHub Actions `release` 워크플로가 자동 실행됩니다.

### 6) 워크플로 검증

아래 순서로 배포 결과를 확인합니다.

```bash
gh run list --workflow release --limit 1
gh run watch <run-id>
gh release view "vX.X.X" --json url,assets
```

아래 아티팩트가 릴리즈에 업로드되어야 합니다.
- `TransClip_X.X.X_aarch64.app.zip` (최초 설치용)
- `TransClip.app.tar.gz` (업데이트용)
- `TransClip.app.tar.gz.sig` (서명)
- `latest.json` (업데이터 메타데이터)

추가 검증:
- `https://github.com/dennis-hong/trans-clip/releases/latest/download/latest.json` 접근 확인

### 7) 완료 보고

- 릴리즈 URL 전달
- 자동업데이트 동작 체크 결과 전달
  - 앱 실행
  - 설정 > 업데이트 확인
  - 새 버전 감지/설치/재시작 성공 여부
