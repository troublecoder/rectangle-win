# Rectangle Win — 설계 문서

> **날짜**: 2026-07-15
> **상태**: 설계 완료, 구현 대기
> **대상 플랫폼**: Windows 10/11

## 1. 개요

### 1.1 목표

[Rectangle Pro](https://rectangleapp.com/pro/)의 **Cursor Movement** 기능을 Windows로 가져오는 데스크톱 앱.

핵심 기능:
1. **Window Throw** — modifier 키 홀드 + 마우스 이동 → 파이(pie) 오버레이로 방향 지정 → snap 실행
2. **Long Throw** — 마우스를 더 멀리 이동하면 별도 매핑 실행
3. **Keyboard Snap** — modifier + 방향키로 snap 영역/액션 순환
4. **Snap Areas / Actions** — 비율 기반 사각 영역과 단일 액션(최대화/최소화/중앙정렬 등)을 통합 관리
5. **시스템 트레이 상주** + 로그인 시 시작 + 최소화 시작
6. **Tauri 자동 업데이트** (GitHub Releases)

### 1.2 기술 스택

| 계층 | 기술 |
|---|---|
| 백엔드 | Rust + Tauri v2 |
| Win32 바인딩 | [windows-rs](https://github.com/microsoft/windows-rs) (Microsoft 공식) |
| FSM | [statig](https://github.com/mdeloof/statig) (hierarchical state machine) |
| 프론트엔드 | Vue 3 + [Nuxt UI](https://ui.nuxt.com) |
| 캔버스 | [vue-konva](https://konvajs.org/docs/vue/index.html) (에디터 + 오버레이) |
| 설정 | TOML (`%APPDATA%\rectangle-win\config.toml`) |
| 업데이트 | Tauri Updater + GitHub Releases |

### 1.3 설계 원칙

Clean Architecture + Port/Adapter, Rust idiom, **KISS**, **OCP**, **DRY**.

---

## 2. 아키텍처

### 2.1 백엔드 구조 (Clean Architecture + Port/Adapter)

```
src-tauri/src/
├── main.rs                     # 진입점
├── lib.rs                      # Tauri Builder, 의존성 조립 (DI)
│
├── domain/                     # 🟦 핵심 도메인 (외부 의존 0)
│   ├── mod.rs
│   ├── model.rs                # SnapTarget, WindowAction, Direction, Sector, Config
│   ├── presets.rs              # SnapPreset enum (Minimal/Standard/Extended/Full/Portrait)
│   ├── geometry.rs             # 섹터 산출, 비율→픽셀 변환 (euclid)
│   └── cursor_fsm.rs           # statig HSM (Idle/Armed/Tracking/Snapping)
│
├── application/                # 🟩 유스케이스 (trait만 알고 구현체 모름)
│   ├── mod.rs
│   ├── ports.rs                # trait: WindowMover, InputSource, ConfigStore, OverlayController
│   ├── snap_service.rs         # throw 오케스트레이션 (FSM 이벤트 → 섹터 → snap)
│   └── keyboard_snap_service.rs # 방향키 체인 순환 → snap
│
├── infrastructure/             # 🟨 Win32 어댑터 (ports 구현체)
│   ├── mod.rs
│   ├── win32_input.rs          # SetWindowsHookExW (WH_KEYBOARD_LL + WH_MOUSE_LL)
│   ├── win32_window.rs         # SetWindowPos/MoveWindow → WindowMover
│   ├── win32_monitor.rs        # EnumDisplayMonitors → MonitorProvider
│   ├── overlay_window.rs       # 클릭스루 투명창 → OverlayController
│   └── toml_config.rs          # serde + toml → ConfigStore
│
└── presentation/               # 🟥 Tauri 커맨드 (프론트엔드 API 경계)
    ├── mod.rs
    ├── commands.rs             # #[tauri::command] 프론트엔드 API
    ├── events.rs               # 프론트엔드로 emit 하는 이벤트 정의
    └── tray.rs                 # 시스템 트레이 메뉴
```

### 2.2 프론트엔드 구조 (FSD 경량화 + 백엔드 대칭)

```
src/
├── app/                        # 🟥 presentation ↔ 백엔드 presentation
│   ├── app.vue                 #   Nuxt UI DashboardLayout 루트
│   ├── app.config.ts           #   Nuxt UI 설정
│   └── router.ts
│
├── features/                   # 🟩 application ↔ 백엔드 application
│   ├── settings/
│   │   ├── composables/        #   useConfig, useTriggerKeys, useStartup
│   │   └── api/commands.ts     #   Tauri invoke 래퍼 (port 역할)
│   ├── cursor/                 #   throw 설정
│   ├── editor/                 #   snap 영역 에디터 로직 (useCanvasEditor)
│   ├── keyboard-snap/          #   키보드 체인 편집
│   └── overlay/                #   reticle 오버레이 로직 (useReticle)
│
├── entities/                   # 🟦 domain ↔ 백엔드 domain
│   ├── snap-target.ts          #   SnapTarget, WindowAction 타입 (TS 미러)
│   ├── config.ts               #   AppConfig 타입
│   └── monitor.ts              #   MonitorBounds, Rect 타입
│
├── widgets/                    # 복합 UI 블록
│   ├── settings-layout/        #   사이드바 + 콘텐츠 레이아웃
│   ├── snap-editor/            #   vue-konva 에디터 (3패널)
│   │   ├── TargetList.vue      #   좌측: 영역/액션 목록
│   │   ├── MonitorCanvas.vue   #   중앙: v-stage 캔버스
│   │   ├── PropertyPanel.vue   #   우측: 속성 폼 (양방향 동기화)
│   │   ├── SectorMapper.vue    #   throw 섹터 매핑 UI
│   │   └── ChainEditor.vue     #   horizontal/vertical 체인 순서 편집
│   └── reticle-overlay/        #   오버레이 위젯
│       ├── ReticleOverlay.vue  #   파이 차트 + 커서 포인터 원
│       └── PieSector.vue
│
├── pages/                      # 라우트 페이지 = 사이드바 카테고리 (1:1 매칭)
│   ├── general.vue             #   시작옵션, 트레이, 언어
│   ├── throw.vue               #   Window Throw + Long Throw 설정
│   ├── snap-editor.vue         #   snap 영역 + 섹터 매핑 + 체인 (통합)
│   ├── keyboard.vue            #   키보드 스냅 + modifier 모드 + 체인
│   ├── display.vue             #   reticle/오버레이 색상·크기
│   └── about.vue               #   버전, GitHub 링크 + 자동 업데이트 통합
│
└── shared/
    ├── ui/                     #   Nuxt UI 기반 공통 컴포넌트
    ├── lib/                    #   유틸 (geometry 계산 등)
    └── config/                 #   상수, 테마
```

**의존 규칙** (단방향, FSD 원칙):
```
pages → widgets → features → entities → shared
```
- Vue 컴포넌트(widgets)는 절대 직접 Tauri command 호출 금지 — 반드시 `features/*/composables` 통해
- `entities`는 순수 TS 타입만 (도메인 로직은 백엔드에 있으므로 얇음)

### 2.3 백엔드-프론트엔드 대칭 매핑

| 백엔드 (Rust) | 프론트엔드 (Vue) | 역할 |
|---|---|---|
| `domain/` | `entities/` | 순수 타입/모델 |
| `application/ports.rs` | `features/*/api/commands.ts` | Tauri command = port |
| `application/*_service.rs` | `features/*/composables/` | 유스케이스 |
| `presentation/` | `app/` + `pages/` | 사용자 인터페이스 |
| `infrastructure/` | (Tauri가 처리) | 백엔드 담당 |

---

## 3. 도메인 모델

### 3.1 SnapTarget — 영역과 액션의 통합

snap 영역(비율 기반 사각)과 단일 액션(최대화/최소화 등)을 **동등한 매핑 대상**으로 통합. throw 섹터와 키보드 체인 모두 SnapTarget을 참조.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SnapTarget {
    #[serde(rename = "area")]
    Area {
        id: String,
        name: String,
        x_ratio: f64,
        y_ratio: f64,
        w_ratio: f64,
        h_ratio: f64,
    },
    #[serde(rename = "action")]
    Action {
        id: String,
        name: String,
        action: WindowAction,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowAction {
    Maximize,
    Minimize,
    Restore,
    Center,
    AlmostMaximize,
    MaximizeHeight,
    NextDisplay,
    PreviousDisplay,
}
```

### 3.2 프리셋 패키지

사용자가 snap 영역을 일일이 정의하지 않도록 콤보박스로 프리셋 선택. 선택시 해당 프리셋의 영역이 자동 생성되고 커스텀 영역은 보존.

| 프리셋 | 포함 |
|---|---|
| Minimal | 반(4) + 최대화 |
| Standard (기본) | 반(4) + 1/3(3) + 코너(4) + 최대화 |
| Extended | Standard + 2/3(2) + 중앙 + 거의 최대화 |
| Full | Extended + 1/6(6) |
| Portrait | 세로 모니터용 (3분할이 상하 방향) |

**id 명명 규칙** (프리셋이 생성하는 영역 id — chain/mapping 참조용):
- Halves: `left-half`, `right-half`, `top-half`, `bottom-half`
- Thirds: `third-left`, `third-center`, `third-right`
- Two-thirds: `two-thirds-left`, `two-thirds-right`
- Quarters: `quarter-tl`, `quarter-tr`, `quarter-bl`, `quarter-br`
- Sixths: `sixth-tl`, `sixth-tc`, `sixth-tr`, `sixth-bl`, `sixth-bc`, `sixth-br`
- 기타: `maximize`, `almost-maximize`, `center`, `maximize-height`

### 3.3 커서 FSM (statig HSM)

```
Idle ──modifier down──► Armed ──mouse move──► Tracking ──modifier up──► Snapping ──► Idle
```

- `Tracking` 상태는 현재 섹터와 throw 거리를 추적
- Long Throw 임계값 초과시 long_throw 매핑 사용
- 성장 포인트: `Tracking` 안에 `Normal`/`LongThrow` 하위 상태를 중첩 가능 (statig HSM 구조)

---

## 4. 기능 상세

### 4.1 Window Throw (Cursor Movement)

modifier 홀드 + 마우스 이동으로 동작:

1. modifier down → `Armed`, 오버레이 표시 (reticle 중앙 + 빨간 반투명 원 포인터)
2. mouse move → `Tracking`, 커서 델타로 파이 섹터 산출 (4/8/12 섹터), 활성 섹터 하이라이트
3. throw 거리가 `long_throw_distance` 초과시 long_throw 매핑 사용
4. modifier up → `Snapping`, 활성 섹터의 SnapTarget 실행 → 오버레이 숨김 → `Idle`

**오버레이**: 투명 클릭스루 Tauri 창(`WS_EX_LAYERED | WS_EX_TRANSPARENT`) 위에 vue-konva로 렌더링. 에디터와 오버레이를 하나의 캔버스 기술로 통일 (DRY).

### 4.2 Keyboard Snap

modifier + 방향키로 snap 영역/액션 순환. **포커스된 윈도우 기준**.

두 개의 체인 (snap pool의 id 배열):
- **horizontal**: 좌=역방향, 우=정방향
- **vertical**: 위=역방향, 아래=정방향

기본 vertical 체인: `[maximize, almost-maximize, center, maximize-height]`
- Down 탭: maximize → almost-maximize → center → maximize-height → maximize (순환)
- Up 탭: maximize → maximize-height → center → almost-maximize (역순환)

`cycle_timeout_ms` 내 연속 탭만 순환, 초과시 첫 항부터 재시작.

### 4.3 Modifier 모드

| 모드 | 동작 |
|---|---|
| `separate` (기본) | throw=`[Win, Alt]`, keyboard=`[Ctrl, Alt]` 별개 |
| `shared` | 같은 modifier 조합. 마우스 이동=throw, 방향키=keyboard snap |
| `override_os` | Windows 기본 Win+방향키 Snap을 가로채서 우리 snap으로 대체. WH_KEYBOARD_LL에서 Win+방향키만 swallow(이벤트 차단), 나머지 Win 조합은 정상 통과. FancyZones의 "Override Windows Snap"과 동일 메커니즘 |

Win32 입력 디스패처가 modifier 조합 + 이벤트 타입(키 vs 마우스)으로 라우팅. `override_os` 모드에서는 Win+방향키 감지시 `CallNextHookEx` 호출 없이 non-zero 반환하여 OS 기본 Snap 동작을 차단함.

### 4.4 멀티 모니터

모니터 독립 처리. 커서가 있는 모니터의 bounds를 기준으로:
- reticle center = 해당 모니터 중앙
- snap 영역 = 모니터 로컬 비율 좌표계 (0.0~1.0)
- DPI: per-monitor DPI awareness 설정

---

## 5. 설정 스키마 (TOML)

경로: `%APPDATA%\rectangle-win\config.toml` (`dirs::config_dir()`)

```toml
[general]
launch_at_login     = true
start_minimized     = true
show_in_tray        = true
language            = "ko"

# ─── 공통 snap pool (throw + keyboard가 공유) ───
[snap]
active_preset = "standard"

[[snap.areas]]
id = "left-half"
name = "Left Half"
x_ratio = 0.0
y_ratio = 0.0
w_ratio = 0.5
h_ratio = 1.0

[[snap.areas]]
id = "center"
name = "Center"
x_ratio = 0.25
y_ratio = 0.25
w_ratio = 0.5
h_ratio = 0.5

[[snap.actions]]
id = "maximize"
name = "Maximize"
action = "Maximize"

[[snap.actions]]
id = "minimize"
name = "Minimize"
action = "Minimize"

# ... 기타 프리셋 영역/액션

# ─── Window Throw (cursor movement) ───
[throw]
trigger_modifiers = ["Win", "Alt"]
long_throw_enabled = true
long_throw_distance = 400

[throw.mapping]
# 섹터 인덱스(0~7) → SnapTarget id
0 = "right-half"
1 = "quarter-br"
2 = "minimize"
3 = "quarter-bl"
4 = "left-half"
5 = "quarter-tl"
6 = "maximize"
7 = "quarter-tr"

[throw.long_throw_mapping]
0 = "maximize"
# ...

# ─── Keyboard Snap ───
[keyboard]
enabled = true
trigger_modifiers = ["Ctrl", "Alt"]
modifier_mode = "separate"
cycle_timeout_ms = 1500

[keyboard.chains]
horizontal = ["left-half", "third-left", "center", "third-right", "right-half"]
vertical   = ["maximize", "almost-maximize", "center", "maximize-height"]

# ─── Overlay ───
[overlay]
reticle_style           = "pie"
cursor_indicator        = true
cursor_radius           = 18
cursor_color            = "#E53935"
cursor_opacity          = 0.5
sector_highlight_color  = "#3B82F6"
sector_count            = 8
snap_preview            = true

# ─── Update ───
[update]
enabled          = true
channel          = "stable"
check_on_startup = true
```

---

## 6. Snap Editor UI

### 6.1 전체 앱 레이아웃 — 사이드바 내비게이션

앱은 설정 기능만 제공하므로, **모든 설정 카테고리를 좌측 사이드바에 배치**하고 클릭시 우측 콘텐츠 영역이 전환됩니다. Nuxt UI의 `UDashboardLayout` + `UVerticalNavigation` 또는 `UNavigationMenu` 기반.

```
┌──────────────────────────────────────────────────────────────┐
│  ● Rectangle Win                                    _ □ ×     │
├────────────┬─────────────────────────────────────────────────┤
│            │                                                 │
│  GENERAL   │   ◄── 선택된 카테고리의 콘텐츠                   │
│  THROW     │       (General / Throw / Snap Editor /          │
│ ▸SNAP EDIT │        Keyboard / Display / About)              │
│  KEYBOARD  │                                                 │
│  DISPLAY   │                                                 │
│  ABOUT     │                                                 │
│            │                                                 │
│            │                                                 │
│ ─────────  │                                                 │
│ ⏸ Pause    │                                                 │
│ Quit       │                                                 │
└────────────┴─────────────────────────────────────────────────┘
```

사이드바 항목:
- **General** — 시작 옵션(로그인 시 시작, 최소화 시작), 트레이, 언어
- **Throw** — Window Throw 활성화, trigger modifier, Long Throw 거리/매핑
- **Snap Editor** — snap 영역/액션 관리 + 섹터 매핑 + 체인 편집 (통합 페이지)
- **Keyboard** — 키보드 스냅 활성화, trigger modifier, modifier 모드(shared/separate), 체인
- **Display** — reticle 스타일, 커서 포인터 색상/크기, 섹터 수, 미리보기
- **About** — 버전, GitHub 링크, **자동 업데이트**(활성화, 채널 stable/beta, 수동 확인) 통합

하단: 앱 일시정지(Pause) / 종료(Quit) 버튼.

### 6.2 Snap Editor 페이지 — 3패널 (양방향 동기화)

Snap Editor 카테고리 선택시 우측에 표시되는 통합 에디터. 영역 관리 + 섹터 매핑 + 체인 편집이 하단 탭으로 전환.

```
┌──────────────────────────────────────────────────────────────┐
│  Snap Editor               Preset: [Standard ▼]  [Import][Export]
├────────────┬──────────────────────────────────────┬──────────┤
│  영역 목록  │      모니터 도화감 (vue-konva)        │  속성 패널│
│            │                                      │          │
│ ┌────────┐ │   ┌──────────────────────────────┐  │ Type:    │
│ │▸Left Hf│ │   │                              │  │ ● Area   │
│ │ Right  │ │   │   ┌─────────┐                │  │ ○ Action │
│ │ Center │ │   │   │ 드래그    │                │  │          │
│ │ Maximize│ │   │   │ 박스      │                │  │ Name:    │
│ │ Minimize│ │   │   └─────────┘                │  │ [____]   │
│ │[+Add]  │ │   │                              │  │ X: [0.25]│
│ └────────┘ │   └──────────────────────────────┘  │ Y: [0.25]│
│            │                                      │ W: [0.50]│
│            │                                      │ H: [0.50]│
│            │                                      │ ── 또 ── │
│            │                                      │ Action:  │
│            │                                      │ [Maximize▼│
├────────────┴──────────────────────────────────────┴──────────┤
│  [ Snap Areas ]  [ Sector Mapping ]  [ Keyboard Chains ]    │
└──────────────────────────────────────────────────────────────┘
```

하단 탭 전환:
- **Snap Areas**: 3패널 영역 에디터 (좌측 목록 / 중앙 캔버스 / 우측 속성)
- **Sector Mapping**: 8 파이 섹터 시각화 + 각 섹터별 SnapTarget 선택기
- **Keyboard Chains**: horizontal/vertical 체인 드래그 순서 편집

**양방향 동기화 원칙** (단일 진실 소스):
- 폼 입력 → store 갱신 → 캔버스 자동 리렌더
- 캔버스 드래그/resize → 비율 변환 → store 갱신 → 폼 자동 갱신
- store 변경시 debounce(300ms) → Tauri command → TOML write

### 6.3 Sector Mapper (Sector Mapping 탭)

8개 파이 섹터를 원형으로 시각적으로 표시. 각 섹터 클릭 → SnapTarget 선택기 (영역/액션 콤보박스 모두 선택 가능).

```
         ┌───────────┐
         │ ▲ Maximize │  (섹터 6 = 위)
         └──┬─────┬──┘
  ┌────────┐ │     │ ┌────────┐
  │◤ TL Qt │ │  •  │ │TR Qt ◥ │  (섹터 5/7 = 대각선)
  └────────┘ │     │ └────────┘
  ┌────────┐ │     │ ┌────────┐
  │◀ L Half│ │     │ │R Half ▶│  (섹터 4/0 = 좌우)
  └────────┘ │     │ └────────┘
  ┌────────┐ │     │ ┌────────┐
  │◣ BL Qt │ │     │ │BR Qt ◢ │  (섹터 3/1 = 대각선)
  └────────┘ │     │ └────────┘
         ┌──┴─────┴──┐
         │▼ Minimize  │  (섹터 2 = 아래)
         └───────────┘
```

### 6.4 Chain Editor (Keyboard Chains 탭)

- **horizontal 체인**: 드래그로 순서 편집 (좌=역방향, 우=정방향 순회용)
- **vertical 체인**: 드래그로 순서 편집 (위=역방향, 아래=정방향 순회용)
- 각 체인은 snap pool의 id를 참조하는 단순 배열. 항목 추가/제거 버튼.
- snap pool의 영역/액션이 드롭다운으로 표시되어 체인에 추가 가능.

---

## 7. 데이터 흐름

### 7.1 Throw 실행 시퀀스

```
Win32 훅 (win32_input.rs)
    │
    ├─ modifier down ──► SnapService ──► FSM: Idle→Armed
    │                                   └─► OverlayController.show_reticle()
    │
    ├─ mouse move ────► SnapService ──► FSM: Armed→Tracking
    │                                   ├─► geometry::compute_sector(delta)
    │                                   ├─► OverlayController.update_cursor_indicator()
    │                                   └─► OverlayController.highlight_sector()
    │
    └─ modifier up ───► SnapService ──► FSM: Tracking→Snapping
                                        ├─► sector → SnapTarget 매핑 조회
                                        ├─► WindowMover.move_foreground_window()
                                        ├─► OverlayController.hide()
                                        └─► FSM: Snapping→Idle
```

### 7.2 FE ↔ BE 통신

| 방향 | 방식 | 용도 |
|---|---|---|
| FE → BE | `#[tauri::command]` | 설정 CRUD, 에디터 조작 |
| BE → FE | `emit` 이벤트 | 실시간 오버레이 갱신 (sector-changed, overlay-show/hide) |

---

## 8. 서드파티 크레이트

```toml
[dependencies]
# === Tauri ===
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-autostart = "2"
tauri-plugin-updater    = "2"
tauri-plugin-dialog     = "2"
tauri-plugin-log        = "2"

# === Win32 ===
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_HiDpi",
    "Win32_Graphics_Gdi",
    "Win32_Devices_Display",
]}

# === 도메인 코어 ===
statig = { version = "0.4", features = ["async"] }
euclid = "0.22"

# === 에러 처리 ===
thiserror = "1"
anyhow    = "1"

# === 동시성 ===
parking_lot = "0.12"
dashmap     = "6"

# === 로깅 ===
tracing            = "0.1"
tracing-subscriber = "0.3"

# === 직렬화 & 데이터 ===
serde     = { version = "1", features = ["derive"] }
toml      = "0.8"
uuid      = { version = "1", features = ["v4", "serde"] }

# === 유틸리티 ===
dirs        = "5"
itertools   = "0.13"
time        = { version = "0.3", features = ["serde", "formatting"] }

# === 비동기 런타임 ===
tokio = { version = "1", features = ["full"] }
```

---

## 9. 오류 처리

### 9.1 계층별 에러 타입

| 계층 | 크레이트 | 에러 타입 |
|---|---|---|
| domain | `thiserror` | `DomainError` (TargetNotFound, InvalidSector, InvalidRatio) |
| application | `thiserror` | `ApplicationError` (Domain 전파 + ConfigNotLoaded, WindowOperation) |
| infrastructure | `anyhow` | Win32 호출 실패 + `.context()` 맥락 |
| presentation | `thiserror` + `serde` | `CommandError` (프론트엔드 전달용 직렬화 가능) |

### 9.2 에러 흐름

```
Win32 호출 실패 (windows::core::Error)
    │ anyhow::Result + .context()
    ▼
infrastructure 어댑터
    │ ApplicationError::WindowOperation
    ▼
application 서비스
    │
    ▼
presentation command → CommandError (serde) → 프론트엔드 toast/알림
```

---

## 10. 테스팅 전략

Clean Architecture의 이점: **Win32 없이 도메인/유스케이스 단위 테스트 가능**.

| 계층 | 도구 | Win32 필요 | 비율 |
|---|---|---|---|
| 도메인 단위 (geometry, FSM, presets) | `#[test]` | ❌ | 60% |
| 애플리케이션 단위 (mock traits) | `#[test]` | ❌ | 25% |
| 인프라 통합 (Win32 호출) | `#[test]` + `#[cfg(windows)]` | ✅ | 10% |
| 프론트엔드 | Vitest + Vue Test Utils | ❌ | 5% |

핵심 테스트 대상:
- 섹터 산출 (8/4/12 섹터, 대각선 경계)
- 비율→픽셀 변환 (멀티 모니터, DPI)
- FSM 전이 (Idle→Armed→Tracking→Snapping, 취소)
- 키보드 체인 순환 (정방향/역방향, 타임아웃 리셋)
- 프리셋 영역 생성

---

## 11. 위험 완화

| 위험 | 대응 |
|---|---|
| 전역 훅이 다른 앱 간섭 | modifier 불일치시 즉시 `CallNextHookEx` 전달 (차단 안 함) |
| 오버레이 창이 입력 가로챔 | `WS_EX_TRANSPARENT \| WS_EX_LAYERED` 클릭스루 보장 |
| TOML 손상시 앱 시작 실패 | 손상시 백업 생성 + 기본 설정 폴백 + 트레이 알림 |
| 멀티 모니터 DPI 불일치 | `SetProcessDpiAwarenessContext` per-monitor DPI 인식 |
| 훅 스레드 지연 → Windows가 훅 제거 | 타임아웃 300ms 초과시 즉시 처리 반환 |
| modifier 충돌 (다른 앱과) | TOML에서 자유 변경 + 기본값은 충돌 적은 Win+Alt |

---

## 12. 시스템 트레이 & 자동 시작

- **트레이 아이이콘**: tauri `tray-icon` feature + `tauri-plugin-autostart`
- **트레이 메뉴**: Settings / Pause / Quit
- **로그인 시 시작**: `tauri-plugin-autostart` (Windows 레지스트리 `Run` 키)
- **최소화로 시작**: `start_minimized` 플래그 → 창 숨기고 트레이로

---

## 13. 자동 업데이트

- Tauri Updater (`tauri-plugin-updater`)
- 배포: GitHub Releases (바이너리 + 서명 + JSON manifest)
- 채널: stable / beta
- 시작시 자동 확인 (`check_on_startup`) + 수동 확인 (About 페이지 내 업데이트 섹션)
