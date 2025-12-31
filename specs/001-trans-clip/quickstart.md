# Quick Start Guide: TransClip Development

**Feature Branch**: `001-trans-clip`
**Date**: 2025-12-30

## Prerequisites

### System Requirements
- **macOS**: 12.0 (Monterey) 이상
- **Xcode Command Line Tools**: `xcode-select --install`

### Required Tools
```bash
# Node.js (v18+)
brew install node

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# pnpm (권장) 또는 npm
npm install -g pnpm
```

## Project Setup

### 1. Tauri 프로젝트 초기화

```bash
# 프로젝트 루트에서
pnpm create tauri-app --template react-ts

# 또는 기존 디렉토리에서
cd trans-clip
pnpm init
pnpm add -D @tauri-apps/cli@latest
pnpm tauri init
```

### 2. Dependencies 설치

#### Frontend (package.json)
```json
{
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-global-shortcut": "^2.0.0",
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "zustand": "^4.5.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "typescript": "^5.3.0",
    "vite": "^5.0.0",
    "@vitejs/plugin-react": "^4.2.0",
    "vitest": "^1.0.0"
  }
}
```

```bash
pnpm install
```

#### Backend (src-tauri/Cargo.toml)
```toml
[package]
name = "trans-clip"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-global-shortcut = "2"
tauri-plugin-sql = { version = "2", features = ["sqlite"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio"] }
keyring = "2"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }

# macOS specific
[target.'cfg(target_os = "macos")'.dependencies]
cocoa = "0.25"
objc = "0.2"
core-graphics = "0.23"
```

### 3. Tauri Configuration (src-tauri/tauri.conf.json)

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "TransClip",
  "version": "0.1.0",
  "identifier": "com.transclip.app",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist"
  },
  "app": {
    "withGlobalTauri": true,
    "trayIcon": {
      "iconPath": "icons/tray.png",
      "iconAsTemplate": true
    },
    "windows": [
      {
        "label": "main",
        "title": "TransClip",
        "width": 400,
        "height": 500,
        "visible": false,
        "skipTaskbar": true,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": true
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns"
    ],
    "macOS": {
      "entitlements": "./entitlements.plist",
      "minimumSystemVersion": "12.0",
      "signingIdentity": null,
      "providerShortName": null
    }
  },
  "plugins": {
    "global-shortcut": {}
  }
}
```

### 4. macOS Entitlements (src-tauri/entitlements.plist)

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.app-sandbox</key>
    <false/>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
    <key>com.apple.security.network.client</key>
    <true/>
</dict>
</plist>
```

### 5. Info.plist 설정 (src-tauri/Info.plist)

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
```

## Development Workflow

### Start Development Server
```bash
# 프론트엔드 + Tauri 동시 실행
pnpm tauri dev
```

### Run Tests
```bash
# Frontend tests
pnpm test

# Backend tests
cd src-tauri && cargo test
```

### Build for Production
```bash
# DMG 생성
pnpm tauri build

# 결과물 위치: src-tauri/target/release/bundle/dmg/
```

## Project Structure Overview

```
trans-clip/
├── src/                      # React 프론트엔드
│   ├── components/
│   ├── hooks/
│   ├── services/
│   ├── store/
│   ├── types/
│   ├── App.tsx
│   └── main.tsx
├── src-tauri/                # Rust 백엔드
│   ├── src/
│   │   ├── lib.rs
│   │   ├── clipboard.rs
│   │   ├── hotkey.rs
│   │   ├── translate.rs
│   │   ├── database.rs
│   │   └── keychain.rs
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── capabilities/
├── tests/
├── package.json
└── specs/001-trans-clip/     # 설계 문서
    ├── spec.md
    ├── plan.md
    ├── research.md
    ├── data-model.md
    └── contracts/
```

## First Steps

### 1. 기본 메뉴바 앱 설정
```rust
// src-tauri/src/lib.rs
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let menu = Menu::with_items(app, &[
                &MenuItem::with_id(app, "show", "Show", true, None::<&str>)?,
                &MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?,
            ])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 2. 클립보드 모니터링 시작
```rust
// src-tauri/src/clipboard.rs
// CrossCopy tauri-plugin-clipboard 사용
```

### 3. Cmd+CC 감지 구현
```rust
// src-tauri/src/hotkey.rs
// CGEventTap을 사용한 더블 프레스 감지
```

## Claude API 설정

### 1. API 키 발급
1. [Anthropic Console](https://console.anthropic.com/) 접속
2. API Keys 섹션에서 새 키 생성
3. 앱 설정에서 API 키 입력

### 2. 요금 참고
- Claude Haiku 4.5: $1/MTok (입력), $5/MTok (출력)
- 평균 번역당 약 $0.0012 (500자 기준)
- Model ID: `claude-haiku-4-5-20251001`

## Troubleshooting

### Accessibility Permission 오류
```
"앱이 컴퓨터를 제어할 수 없습니다"
```
→ System Preferences → Privacy & Security → Accessibility에서 TransClip 허용

### Keychain 접근 오류
→ 처음 실행 시 Keychain 접근 허용 팝업에서 "항상 허용" 선택

### 빌드 오류: "linker cc not found"
```bash
xcode-select --install
```

## Next Steps

1. [ ] `pnpm tauri dev`로 개발 환경 확인
2. [ ] 기본 메뉴바 앱 동작 확인
3. [ ] `/speckit.tasks`로 상세 구현 태스크 생성
