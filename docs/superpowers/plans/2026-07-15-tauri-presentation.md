# Rectangle Win — Tauri 앱 골격 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development.

**Goal:** Tauri v2 앱 골격을 세팅하고, domain/application/infrastructure 계층을 Tauri command로 노출한다.

**Architecture:** Clean Architecture의 `presentation/` 계층. `#[tauri::command]`로 프론트엔드 API 정의. AppState에 서비스들을 주입(DI). 시스템 트레이 + 자동시작 + 업데이터 설정.

**Tech Stack:** Tauri v2, tauri-plugin-autostart, tauri-plugin-updater, tauri-plugin-dialog, tauri-plugin-log.

---

## 파일 구조

```
src-tauri/
├── tauri.conf.json          # Tauri v2 설정
├── capabilities/
│   └── default.json         # 권한 정의
├── icons/                   # 앱 아이콘 (임시)
├── Cargo.toml               # tauri 의존성 추가
└── src/
    ├── main.rs              # 진입점 (Tauri Builder)
    ├── lib.rs               # 기존 (domain/application/infrastructure 노출)
    └── presentation/
        ├── mod.rs
        ├── state.rs         # AppState (서비스들 보관)
        ├── commands.rs      # #[tauri::command] 프론트엔드 API
        ├── events.rs        # emit 이벤트 정의
        └── tray.rs          # 시스템 트레이 메뉴
```

---

## Task 1: Tauri v2 프로젝트 세팅

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`
- Create: `src-tauri/icons/` (임시 아이콘)
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: `Cargo.toml`에 Tauri 의존성 추가**

```toml
# 기존 의존성 아래에 추가
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-autostart = "2"
tauri-plugin-updater = "2"
tauri-plugin-dialog = "2"
tauri-plugin-log = "2"

# 로깅 (presentation 계층)
tracing = "0.1"
tracing-subscriber = "0.3"

# 비동기 런타임 (Tauri가 tokio 사용)
tokio = { version = "1", features = ["full"] }
```

- [ ] **Step 2: `tauri.conf.json` 작성**

```json
{
  "$schema": "https://raw.githubusercontent.com/nicehash/tauri/dev/crates/tauri-config-schema/schema.json",
  "productName": "Rectangle Win",
  "version": "0.1.0",
  "identifier": "com.troublecoder.rectangle-win",
  "build": {
    "frontendDist": "../src",
    "devUrl": "http://localhost:3000",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [
      {
        "title": "Rectangle Win",
        "width": 900,
        "height": 640,
        "minWidth": 720,
        "minHeight": 500,
        "resizable": true,
        "visible": false
      }
    ],
    "security": {
      "csp": null
    },
    "trayIcon": {
      "id": "main",
      "iconPath": "icons/icon.png",
      "tooltip": "Rectangle Win"
    }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis", "msi"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.ico",
      "icons/icon.png"
    ]
  },
  "plugins": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://github.com/troublecoder/rectangle-win/releases/latest/download/latest.json"
      ],
      "dialog": true,
      "pubkey": ""
    }
  }
}
```

- [ ] **Step 3: `capabilities/default.json` 작성**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "autostart:allow-enable",
    "autostart:allow-disable",
    "autostart:allow-is-enabled",
    "updater:default",
    "dialog:allow-open",
    "dialog:allow-save",
    "log:default"
  ]
}
```

- [ ] **Step 4: 임시 아이콘 생성**

Tauri는 빌드시 아이콘을 요구합니다. `tauri icon` 명령으로 생성하거나 임시 1x1 PNG를 배치.

Run: `npx @tauri-apps/cli icon --help` 로 확인, 또는 수동으로 `icons/` 폴더에 더미 파일 배치.

최소 필요 파일:
- `icons/icon.png` (512x512)
- `icons/icon.ico` (Windows)
- `icons/32x32.png`, `icons/128x128.png`, `icons/128x128@2x.png`

임시 방법: 단순한 파란색 사각형 PNG를 생성.

- [ ] **Step 5: `main.rs` 임시 Tauri 진입점**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    rectangle_win::run();
}
```

- [ ] **Step 6: `lib.rs`에 `run()` 함수 추가**

기존 `lib.rs` 아래에 `run()` 함수를 추가하되, 이 태스크에서는 빈 Builder만:

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some("com.troublecoder.rectangle-win"),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.show().unwrap();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 7: 빌드 확인**

Run: `cargo build`
Expected: Tauri 관련 의존성 다운로드 후 컴파일 성공 (frontend가 없으므로 실행은 불가, 컴파일만 확인)

- [ ] **Step 8: 커밋**

```bash
git add -A
git commit -m "feat: Tauri v2 프로젝트 세팅 (tauri.conf.json, capabilities, 플러그인)"
```

---

## Task 2: AppState (의존성 주입)

**Files:**
- Create: `src-tauri/src/presentation/mod.rs`
- Create: `src-tauri/src/presentation/state.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: `src-tauri/src/presentation/state.rs`**

모든 서비스와 인프라 구현체를 보관하는 AppState.

```rust
use std::sync::Arc;

use crate::application::ports::{ConfigStore, MonitorProvider, OverlayController, WindowMover};
use crate::application::{keyboard_service::KeyboardService, snap_service::SnapService};
use crate::infrastructure::overlay_window::TauriOverlay;
use crate::infrastructure::toml_config::TomlConfigStore;

/// 앱 전역 상태 — Tauri의 managed state로 등록.
/// 모든 서비스와 인프라 구현체를 보관.
pub struct AppState {
    pub config_store: Arc<TomlConfigStore>,
    pub window_mover: Arc<dyn WindowMover>,
    pub monitor_provider: Arc<dyn MonitorProvider>,
    pub overlay: Arc<TauriOverlay>,
    pub snap_service: SnapService,
    pub keyboard_service: KeyboardService,
}

impl AppState {
    pub fn new() -> Self {
        let config_store = Arc::new(TomlConfigStore::default_path());
        let window_mover: Arc<dyn WindowMover> = {
            #[cfg(windows)]
            {
                Arc::new(crate::infrastructure::win32_window::Win32WindowMover::new())
            }
            #[cfg(not(windows))]
            {
                Arc::new(crate::application::mock::MockWindowMover::new())
            }
        };
        let monitor_provider: Arc<dyn MonitorProvider> = {
            #[cfg(windows)]
            {
                Arc::new(crate::infrastructure::win32_monitor::Win32MonitorProvider::new())
            }
            #[cfg(not(windows))]
            {
                Arc::new(crate::application::mock::MockMonitorProvider::default())
            }
        };
        let overlay = Arc::new(TauriOverlay::new());

        let snap_service = SnapService::new(
            window_mover.clone(),
            monitor_provider.clone(),
            overlay.clone(),
            config_store.clone(),
        );
        let keyboard_service = KeyboardService::new(
            window_mover.clone(),
            monitor_provider.clone(),
            config_store.clone(),
        );

        Self {
            config_store,
            window_mover,
            monitor_provider,
            overlay,
            snap_service,
            keyboard_service,
        }
    }
}
```

- [ ] **Step 2: `presentation/mod.rs`**

```rust
pub mod state;
```

- [ ] **Step 3: `lib.rs`에 presentation 모듈 추가 + run()에 state 등록**

```rust
pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;
```

`run()` 함수에 `.manage(AppState::new())` 추가.

- [ ] **Step 4: 빌드 확인**

Run: `cargo build`

- [ ] **Step 5: 커밋**

```bash
git add -A
git commit -m "feat: AppState 의존성 주입 (서비스 + 인프라 통합)"
```

---

## Task 3: Tauri Commands (프론트엔드 API)

**Files:**
- Create: `src-tauri/src/presentation/commands.rs`
- Create: `src-tauri/src/presentation/events.rs`
- Modify: `src-tauri/src/presentation/mod.rs`
- Modify: `src-tauri/src/lib.rs` (run에 command 등록)

- [ ] **Step 1: `src-tauri/src/presentation/commands.rs`**

프론트엔드에서 호출하는 Tauri command들:

```rust
use tauri::State;

use crate::application::errors::ApplicationError;
use crate::domain::model::Config;
use crate::domain::presets::SnapPreset;
use crate::presentation::state::AppState;

/// Command 에러를 프론트엔드로 직렬화
#[derive(Debug, serde::Serialize)]
pub struct CommandError {
    pub message: String,
    pub code: String,
}

impl From<ApplicationError> for CommandError {
    fn from(e: ApplicationError) -> Self {
        let code = match &e {
            ApplicationError::Domain(_) => "DOMAIN",
            ApplicationError::ConfigNotLoaded => "CONFIG_NOT_LOADED",
            ApplicationError::WindowOperation(_) => "WINDOW_OP",
            ApplicationError::OverlayOperation(_) => "OVERLAY_OP",
            ApplicationError::NoForegroundWindow => "NO_FOREGROUND",
        };
        CommandError {
            message: e.to_string(),
            code: code.to_string(),
        }
    }
}

type CmdResult<T> = Result<T, CommandError>;

// ─── 설정 CRUD ───

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> CmdResult<Config> {
    Ok(state.config_store.load()?)
}

#[tauri::command]
pub fn save_config(state: State<'_, AppState>, config: Config) -> CmdResult<()> {
    state.config_store.save(&config)?;
    Ok(())
}

#[tauri::command]
pub fn get_config_path(state: State<'_, AppState>) -> String {
    state.config_store.path().to_string_lossy().to_string()
}

// ─── 프리셋 ───

#[tauri::command]
pub fn apply_preset(preset_name: String, state: State<'_, AppState>) -> CmdResult<Config> {
    let preset = SnapPreset::from_str(&preset_name)?;
    let mut config = state.config_store.load()?;
    config.snap.active_preset = preset_name;
    config.snap.areas = preset.targets();
    state.config_store.save(&config)?;
    Ok(config)
}

// ─── 테스트용 ───

#[tauri::command]
pub fn get_monitors(state: State<'_, AppState>) -> Vec<MonitorInfo> {
    state.monitor_provider.enumerate().iter().map(|m| MonitorInfo {
        x: m.origin.x,
        y: m.origin.y,
        width: m.width(),
        height: m.height(),
    }).collect()
}

#[derive(serde::Serialize)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[tauri::command]
pub fn test_snap_to_sector(sector: u8, cursor_x: i32, cursor_y: i32, state: State<'_, AppState>) -> CmdResult<()> {
    // 테스트용: 특정 섹터로 강제 snap 실행 (설정 화면 미리보기용)
    // SnapService의 내부 로직을 직접 호출하는 대신,
    // 실제 throw 시뮬레이션 없이 sector → target 매핑 → snap
    use crate::domain::errors::DomainError;
    let config = state.config_store.load()?;
    let target_id = config.throw.mapping.get(&sector)
        .ok_or_else(|| ApplicationError::Domain(DomainError::InvalidSector { index: sector, max: 8 }))?;
    let target = config.snap.areas.iter().find(|t| t.id() == target_id)
        .ok_or_else(|| ApplicationError::Domain(DomainError::TargetNotFound(target_id.clone())))?;
    let window = state.window_mover.get_foreground_window()
        .ok_or(ApplicationError::NoForegroundWindow)?;
    let monitor = state.monitor_provider.monitor_at(cursor_x, cursor_y);
    state.window_mover.apply_snap_target(window, target, &monitor)?;
    Ok(())
}
```

- [ ] **Step 2: `src-tauri/src/presentation/events.rs`**

```rust
use serde::{Deserialize, Serialize};

/// 프론트엔드로 emit하는 오버레이 이벤트
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OverlayEvent {
    Show {
        center_x: i32,
        center_y: i32,
        sector_count: u8,
    },
    CursorUpdate {
        x: i32,
        y: i32,
    },
    SectorHighlight {
        sector: u8,
    },
    SnapPreview {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    Hide,
}

/// 설정 변경 이벤트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangedEvent {
    pub config: crate::domain::model::Config,
}
```

- [ ] **Step 3: `presentation/mod.rs` 업데이트**

```rust
pub mod commands;
pub mod events;
pub mod state;
```

- [ ] **Step 4: `lib.rs` run()에 command 등록**

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some("com.troublecoder.rectangle-win"),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(presentation::state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            presentation::commands::get_config,
            presentation::commands::save_config,
            presentation::commands::get_config_path,
            presentation::commands::apply_preset,
            presentation::commands::get_monitors,
            presentation::commands::test_snap_to_sector,
        ])
        .setup(|app| {
            // AppState의 overlay emitter를 Tauri AppHandle로 설정
            let state = app.state::<presentation::state::AppState>();
            let app_handle = app.handle().clone();
            state.overlay.set_emitter(move |overlay_state| {
                use presentation::events::OverlayEvent;
                let event = if overlay_state.visible {
                    OverlayEvent::Show {
                        center_x: overlay_state.center.unwrap_or((0, 0)).0,
                        center_y: overlay_state.center.unwrap_or((0, 0)).1,
                        sector_count: overlay_state.sector_count,
                    }
                } else {
                    OverlayEvent::Hide
                };
                let _ = app_handle.emit("overlay", event);
            });

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.show().unwrap();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: 빌드 확인**

Run: `cargo build`
Expected: 컴파일 성공

- [ ] **Step 6: 커밋**

```bash
git add -A
git commit -m "feat: Tauri command API (설정 CRUD, 프리셋, 모니터, 테스트 snap)"
```

---

## Task 4: 시스템 트레이

**Files:**
- Create: `src-tauri/src/presentation/tray.rs`
- Modify: `src-tauri/src/presentation/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: `src-tauri/src/presentation/tray.rs`**

```rust
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, MenuEvent},
    tray::TrayIconBuilder,
    App, Manager, WindowEvent,
};

pub fn setup_tray(app: &App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "설정", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "일시정지", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show, &pause, &separator, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Rectangle Win")
        .menu(&menu)
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "pause" => {
                    // TODO: SnapService/KeyboardService 비활성화
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
```

- [ ] **Step 2: `presentation/mod.rs` 업데이트**

```rust
pub mod commands;
pub mod events;
pub mod state;
pub mod tray;
```

- [ ] **Step 3: `lib.rs` setup에 트레이 호출 + 창 닫기 가로채기**

```rust
.setup(|app| {
    // ... 기존 설정 ...
    
    // 시스템 트레이 설정
    presentation::tray::setup_tray(app)?;

    // 창 닫기 → 트레이로 최소화 (종료 아님)
    let window = app.get_webview_window("main").unwrap();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            // 숨기기만 (트레이에 남음)
            // window는 move로 가져왔으므로 여기서 접근 불가 —
            // 실제 구현에서는 app_handle로 처리
        }
    });

    Ok(())
})
```

**참고**: 창 닫기 가로채기는 `on_window_event`에서 `api.prevent_close()` 호출 후 `window.hide()`. 하지만 closure 캡처 문제가 있으므로 실제 구현에서는 `app_handle`을 clone해서 사용.

- [ ] **Step 4: 빌드 확인**

Run: `cargo build`

- [ ] **Step 5: 커밋**

```bash
git add -A
git commit -m "feat: 시스템 트레이 (설정/일시정지/종료 메뉴, 트레이로 최소화)"
```

---

## Self-Review

**1. Spec coverage:**
- ✅ Tauri v2 프로젝트 세팅 → Task 1
- ✅ AppState 의존성 주입 → Task 2
- ✅ Tauri commands (get_config, save_config, apply_preset, get_monitors, test_snap) → Task 3
- ✅ 시스템 트레이 → Task 4
- ✅ 자동시작 플러그인 → Task 1 (플러그인 등록, 커맨드는 프론트엔드에서 호출)
- ✅ 업데이터 플러그인 → Task 1 (플러그인 등록, pubkey는 배포시 설정)

**2. 주의사항:**
- 아이콘 파일이 없으면 빌드 실패. 임시 아이콘 생성 필수.
- `frontendDist`가 아직 없으므로 실제 실행은 불가. 컴파일만 확인.
- tauri.conf.json의 `$schema` 경로는 Tauri 버전에 따라 다를 수 있음.
- updater `pubkey`는 빈 문자열 — 실제 배포시 키 쌍 생성 필요.
- 창 닫기 가로채기 closure 캡처 문제는 실제 구현에서 해결 (app_handle clone).
