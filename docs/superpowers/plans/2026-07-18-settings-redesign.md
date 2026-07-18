# Settings 화면 재설계 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 기존 손수 만든 설정 UI를 Nuxt UI dashboard 컴포넌트 기반으로 전면 재작성하고, 백엔드 스키마를 정리하며, Catppuccin 커스텀 테마를 제거한다.

**Architecture:** 프론트엔드는 `UDashboardGroup`/`Sidebar`/`Panel`/`Navbar` + flat 섹션(소제목 + USeparator + UFormField). SnapEditor는 `UTable` expandable rows. 백엔드는 `OverlayConfig`에 `preview_colors` 객체 추가, 미사용 필드 제거. Vue+Vite 환경에서 `@nuxt/ui/vue-plugin` 사용.

**Tech Stack:** Vue 3.5, Vite 6, Nuxt UI 4, Tailwind 4, TanStack Table (Nuxt UI 내장), vue-i18n 9, Pinia 2, Zod 3, Rust + windows-rs (Tauri 2).

**Spec:** `docs/superpowers/specs/2026-07-18-settings-redesign-design.md`

## Global Constraints

- **하위 호환성 없음**: 기존 설정 파일의 구 필드(`sector_highlight_color`, `sector_count`, `cycle_timeout_ms`)는 마이그레이션 없이 제거. 기존 사용자 설정 파일이 serde 오류를 일으키면 파일 삭제 후 재생성으로 대응 (개발 단계 수용).
- **Nuxt UI 컴포넌트 우선**: custom div 조립 대신 Nuxt UI 컴포넌트 사용. `UPageCard`(랜딩용) 금지, `UCard`는 독립 데이터 영역(테이블/매핑/앱 정보)에만 제한적 사용.
- **Semantic 색상만**: `text-default`, `bg-elevated`, `border-muted`, `color="primary"` 등. raw Tailwind palette(`text-gray-500`) 금지.
- **기본 테마**: Catppuccin 커스텀 제거. Nuxt UI 기본(green/blue/slate). Pretendard 폰트만 유지.
- **modifier 토큰 포맷**: `"Win"`, `"Alt"`, `"Ctrl"`, `"Shift"` (대소문자 구분 — `win32_input.rs:459` 호환).
- **i18n 기본 locale**: `'ko'` (`src/i18n/index.ts` + `default-config.ts` 일치).
- **commit 메시지**: 한글, `feat:`/`refactor:`/`chore:`/`fix:` prefix.

---

## File Structure

### 신규 파일

| 파일 | 책임 |
|---|---|
| `src/layouts/SettingsLayout.vue` | UDashboardGroup + Sidebar + RouterView. footer에 활성화 토글 + 종료. |
| `src/components/SaveActions.vue` | UDashboardNavbar `#right`용 저장/초기화 버튼 묶음. |
| `src/components/UHotkeyInput.vue` | 키보드 이벤트 캡처 → modifier string[] emit. |
| `src/components/ColorLockField.vue` | throw/long_throw 색상 + lock 토글 (양방향 동기화). |
| `src/components/SectorMappingTable.vue` | 8섹터 × (throw/long_throw) 매핑 편집 표. Throw 페이지용. |

### 재작성

| 파일 | 변경 |
|---|---|
| `src/components/SnapCanvas.vue` | 축소형 (확장 영역 미니 캔버스). |
| `src/components/SnapProperties.vue` | UTable #expanded 슬롯용 폼. |

### 수정

`src/App.vue`, `src/main.ts`(라우트), `src/entities/{config,default-config}.ts`, `src/features/config-store.ts`, `src/i18n/index.ts`, `src/i18n/locales/{en,ko}.ts`, `src/pages/{General,Throw,SnapEditor,Display,About}.vue`, `src/assets/css/main.css`, `vite.config.ts`, `package.json`.

### 백엔드 수정

`src-tauri/src/domain/model.rs`, `src-tauri/src/domain/cursor_fsm.rs`, `src-tauri/src/application/{ports,snap_service,keyboard_service}.rs`, `src-tauri/src/infrastructure/{win32_overlay,toml_config,win32_input}.rs`, `src-tauri/tests/*`.

### 삭제

`src/components/{SettingsLayout,PageHeader,SaveBar,USection,SectorMapping}.vue`, `src/pages/Keyboard.vue`, `src/assets/css/_catppuccin.css`, `scripts/gen-catppuccin-theme.js`.

---

## Task 1: Catppuccin 테마 제거 + 기본 테마 전환

**Files:**
- Delete: `src/assets/css/_catppuccin.css`, `scripts/gen-catppuccin-theme.js`
- Modify: `src/assets/css/main.css`, `vite.config.ts`, `package.json`

**Interfaces:**
- Produces: 기본 Nuxt UI 테마 활성 상태. 이후 모든 태스크는 semantic 색상(`text-default`, `color="primary"`)만 사용.

- [ ] **Step 1: `_catppuccin.css` 삭제**

```bash
rm src/assets/css/_catppuccin.css
rm scripts/gen-catppuccin-theme.js
```

- [ ] **Step 2: `main.css`에서 catppuccin import 제거**

`src/assets/css/main.css` 수정 — 첫 두 줄 `@import` 아래의 `@import "./_catppuccin.css";` 줄을 제거. Pretendard `@font-face`와 `@theme static { --font-sans }`는 유지. 결과 파일 전체:

```css
@import "tailwindcss";
@import "@nuxt/ui";

/* ── Pretendard 폰트 (로컬 에셋, 오프라인 대응) ── */
@font-face {
  font-family: "Pretendard";
  font-style: normal;
  font-weight: 400;
  font-display: swap;
  src: url("../fonts/Pretendard-Regular.subset.woff2") format("woff2");
}
@font-face {
  font-family: "Pretendard";
  font-style: normal;
  font-weight: 500;
  font-display: swap;
  src: url("../fonts/Pretendard-Medium.subset.woff2") format("woff2");
}
@font-face {
  font-family: "Pretendard";
  font-style: normal;
  font-weight: 600;
  font-display: swap;
  src: url("../fonts/Pretendard-SemiBold.subset.woff2") format("woff2");
}
@font-face {
  font-family: "Pretendard";
  font-style: normal;
  font-weight: 700;
  font-display: swap;
  src: url("../fonts/Pretendard-Bold.subset.woff2") format("woff2");
}

@theme static {
  --font-sans: "Pretendard", ui-sans-serif, system-ui, sans-serif;
}
```

- [ ] **Step 3: `vite.config.ts`에서 `ui.colors` 설정 제거**

`ui({ ui: { colors: { ... } } })` 블록을 인자 없는 `ui()`로 교체:

```ts
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import ui from '@nuxt/ui/vite'
import { resolve } from 'path'

export default defineConfig({
  plugins: [
    vue(),
    ui(),
  ],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  clearScreen: false,
  server: {
    port: 3000,
    strictPort: true,
    watch: {
      ignored: ['**/target/**', '**/src-tauri/target/**', '**/dist/**'],
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: 'es2021',
    outDir: 'dist',
  },
})
```

- [ ] **Step 4: `package.json`에서 catppuccuin 의존성 제거**

`dependencies`에서 `"@catppuccin/palette": "^1.8.0",` 줄 제거. `scripts`에 catppuccin 테마 생성 스크립트가 있으면 제거 (현재는 없음).

```bash
pnpm remove @catppuccin/palette
```

- [ ] **Step 5: 빌드 검증**

```bash
pnpm build
```

Expected: `vue-tsc --noEmit` 통과, `vite build` 성공. 에러가 없어야 함.

- [ ] **Step 6: 커밋**

```bash
git add -A
git commit -m "chore: Catppuccin 커스텀 테마 제거 — Nuxt UI 기본 테마 전환

- _catppuccin.css 및 gen-catppuccin-theme.js 스크립트 삭제
- vite.config.ts ui() colors 매핑 제거 (완전 순정)
- @catppuccin/palette 의존성 제거
- Pretendard 폰트 설정만 유지"
```

---

## Task 2: 백엔드 dead code 제거

**Files:**
- Modify: `src-tauri/src/infrastructure/win32_overlay.rs` (`OverlayDrawState`, `OverlayResources`, `OverlayController` impl)
- Modify: `src-tauri/src/infrastructure/win32_input.rs` (`Win32InputListener::stop`)
- Modify: `src-tauri/src/application/ports.rs` (`OverlayController::update_cursor_indicator`)
- Modify: `src-tauri/src/application/mock.rs` (`update_cursor_indicator` mock)

**Interfaces:**
- Produces: `OverlayController` 트레이트에서 `update_cursor_indicator` 제거.
  `OverlayDrawState`에서 `cursor`/`sector_count` 필드 제거. `OverlayResources`에서
  `d2d_factory`/`_previous_bmp` 필드 제거. `Win32InputListener::stop` 제거.
  이후 모든 코드는 실사용 필드만 참조.

> **원칙**: 호환성 무시. `#[allow(dead_code)]` 디렉티브 자체를 제거하고
> 해당 필드/함수를 삭제. 단, `OverlayDrawState.center`는 `show_reticle`이
> 설정하고 draw_scene이 읽으므로 **실사용** — `#[allow]`만 잘못 붙은 것이므로
> 디렉티브만 제거하고 필드는 유지.

- [ ] **Step 1: `win32_input.rs` — `stop()` 메서드 제거**

`src-tauri/src/infrastructure/win32_input.rs` line 254-264의 `stop` 메서드 전체 제거 (호출처 0건). 메서드 위 주석 line 254-257도 함께 제거.

제거 대상:
```rust
    /// 입력 리스너 정지 — 스레드에 WM_QUIT 게시.
    ///
    /// 메시지 루프가 WM_QUIT 을 받으면 훅 해제 후 종료된다.
    /// 스레드 자체의 join 은 수행하지 않는다 (best-effort).
    #[allow(dead_code)]
    pub fn stop(&self) {
        // SAFETY: PostThreadMessageW 는 thread id 가 유효하면 안전.
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
```

제거 후 `PostThreadMessageW` import가 미사용이면 `use` 문에서도 제거 (`cargo build` 경고로 확인).

- [ ] **Step 2: `win32_overlay.rs` — `OverlayDrawState.cursor` 필드 제거**

`OverlayDrawState` (line 71-82)에서:
- `cursor: Option<(i32, i32)>` 필드와 그 위 `#[allow(dead_code)]` 제거 (line 80-81).
- `update_cursor_indicator` impl (line 541-547)에서 `state.cursor = Some((x, y));` 제거.

수정 후 `OverlayDrawState`:
```rust
#[derive(Default)]
struct OverlayDrawState {
    visible: bool,
    #[allow(dead_code)]
    center: Option<(i32, i32)>,
    #[allow(dead_code)]
    sector_count: u8,
    active_sector: Option<u8>,
    snap_preview: Option<(i32, i32, i32, i32)>,
}
```

> `sector_count`는 Task 3(스키마 정리)에서 완전 제거. 여기서는 `cursor`만.
> `center`는 실사용이므로 `#[allow(dead_code)]`를 제거하면 컴파일 경고 발생 가능
> — 실사용이므로 디렉티브를 제거해도 안전. 단, draw_scene line 404에서만 읽히고
> `show_reticle`이 쓰므로 디렉티브 제거 후 경고 없는지 `cargo build`로 확인.

실제로 `center`를 읽는 곳이 draw_scene line 404 (`if let Some((cx, cy)) = state.center`) 하나뿐이므로 디렉티브 제거 안전.

수정 후 (center 디렉티브 제거):
```rust
#[derive(Default)]
struct OverlayDrawState {
    visible: bool,
    center: Option<(i32, i32)>,
    #[allow(dead_code)]
    sector_count: u8,
    active_sector: Option<u8>,
    snap_preview: Option<(i32, i32, i32, i32)>,
}
```

- [ ] **Step 3: `ports.rs` + `mock.rs` — `update_cursor_indicator` 트레이트 메서드 제거**

`src-tauri/src/application/ports.rs`의 `OverlayController` (line 39-45)에서 `update_cursor_indicator` 라인 제거:

```rust
pub trait OverlayController: Send + Sync {
    fn show_reticle(&self, center_x: i32, center_y: i32, sector_count: u8) -> AppResult<()>;
    fn highlight_sector(&self, sector: u8) -> AppResult<()>;
    fn show_snap_preview(&self, x: i32, y: i32, width: i32, height: i32) -> AppResult<()>;
    fn hide(&self) -> AppResult<()>;
}
```

`src-tauri/src/application/mock.rs` line 147의 mock `update_cursor_indicator` 구현도 제거.

- [ ] **Step 4: `win32_overlay.rs` — `update_cursor_indicator` impl 제거**

`impl OverlayController for Win32LayeredOverlay`의 `update_cursor_indicator` (line 541-547) 전체 제거:

```rust
    fn update_cursor_indicator(&self, x: i32, y: i32) -> AppResult<()> {
        let mut state = self.state.lock().unwrap();
        state.cursor = Some((x, y));
        drop(state);
        self.redraw();
        Ok(())
    }
```

- [ ] **Step 5: `win32_overlay.rs` — `OverlayResources.d2d_factory` 필드 제거**

`OverlayResources` (line 107-120)에서 `d2d_factory: ID2D1Factory1` 필드와 `#[allow(dead_code)]` (line 113-114) 제거.

`init_resources` (line 147-)에서 `d2d_factory`를 로컬 변수로 유지 (생성 후 `CreateDCRenderTarget`에 사용 line 185), 구조체 필드에서는 제거. line 217 (`d2d_factory`를 구조체 리터럴에 넣는 부분)과 line 239도 제거.

수정 후 `OverlayResources`:
```rust
struct OverlayResources {
    hwnd: HWND,
    hdc_mem: HDC,
    /// 이전 비트맵(기본 1x1 모노). DeleteObject 로 정리용 보관.
    _previous_bmp: HGDIOBJ,
    dc_render_target: ID2D1DCRenderTarget,
    brush: ID2D1SolidColorBrush,
    /// 점선(대시) 사각형용 stroke style (snap preview).
    dash_style: ID2D1StrokeStyle,
    width: i32,
    height: i32,
}
```

(`_previous_bmp`는 Step 6에서 처리, 여기서는 `d2d_factory`만)

`init_resources`에서 `let d2d_factory: ID2D1Factory1 = ...` 로컬은 유지하고 구조체 생성 시 넘기지 않음.

- [ ] **Step 6: `win32_overlay.rs` — `_previous_bmp` 필드 제거**

`OverlayResources._previous_bmp` (line 110-111)와 주석 제거. `init_resources`의 `create_dib` 반환값에서 `previous_bmp`를 받되 보관하지 않음 (로컬 `_` 로 무시 또는 반환 튜플에서 제거).

`create_dib` (line 248-285) 시그니처 변경 검토 — `previous_bmp`를 반환할 필요 없으면 `SelectObject` 결과를 로컬 `_` 로 처리:

```rust
// create_dib 내 (line 284):
let _previous_bmp = unsafe { SelectObject(hdc_mem, hbitmap) };
Ok((hdc_mem, hbitmap))
```

반환 튜플을 `(hdc_mem, hbitmap)`으로 축소. `init_resources` 호출부도 업데이트.

> 주의: `_previous_bmp`를 삭제하면 크기 변경 시 원본 비트맵 복원이 불가능해지지만,
> 현재 코드는 크기 변경 시 `hbitmap`만 `DeleteObject`하고 `previous_bmp`는
> 정리에 쓰이지 않으므로 제거해도 안전 (실사용 0건).

- [ ] **Step 7: cargo build로 dead_code 경고 확인**

```bash
cd src-tauri && cargo build
```

Expected: `#[allow(dead_code)]` 제거 후 새 dead_code 경고 없음. `center` 필드 관련 경고도 없어야 함 (실사용 중). 남은 `sector_count`는 Task 3에서 제거하므로 여기서는 허용.

- [ ] **Step 8: cargo test**

```bash
cd src-tauri && cargo test
```

Expected: 테스트 통과 (mock이 `update_cursor_indicator` 제거에 맞춰 업데이트됨).

- [ ] **Step 9: 커밋**

```bash
git add -A
git commit -m "refactor: 백엔드 dead code 제거 — 호환성 무시 정리

- Win32InputListener::stop 제거 (호출처 0건)
- OverlayDrawState.cursor 필드 + update_cursor_indicator 트레이트/impl/mock 제거
- OverlayResources.d2d_factory 필드 제거 (로컬 변수로 강등)
- OverlayResources._previous_bmp 필드 제거 (실사용 0건)
- OverlayDrawState.center의 잘못된 #[allow(dead_code)] 제거 (실사용 중)"
```

---

## Task 3: 백엔드 스키마 정리 — 미사용 필드 제거 + preview_colors 추가

**Files:**
- Modify: `src-tauri/src/domain/model.rs:243-283` (KeyboardConfig, OverlayConfig)
- Modify: `src-tauri/src/domain/cursor_fsm.rs:38,132` (sector_count)
- Modify: `src-tauri/src/application/{ports,snap_service,keyboard_service}.rs`
- Modify: `src-tauri/src/infrastructure/{win32_overlay,toml_config}.rs`
- Modify: `src-tauri/tests/*` (영향받는 테스트)

**Interfaces:**
- Produces: `OverlayConfig` 재구성 — `cursor: CursorConfig`, `snap_preview: SnapPreviewConfig { enabled, colors: PreviewColors }`. `ThrowConfig` — `long_throw: LongThrowConfig { enabled, distance, mapping }`. `KeyboardConfig`는 `enabled: bool`만. `OverlayController::show_reticle` 시그니처에서 `sector_count` 파라미터 제거. `OverlayController::show_snap_preview`에 `is_long_throw: bool` 파라미터 추가.

> **계층화 원칙**: 강결합 필드(항상 함께 의미를 갖는 3개 이상)는 객체로.
> 단일/독립 필드는 평면 유지. `GeneralConfig`는 평면 유지(독립적 필드들).

- [ ] **Step 1: `model.rs` — KeyboardConfig에서 cycle_timeout_ms 제거**

`src-tauri/src/domain/model.rs`의 `KeyboardConfig` (line 243-256) 교체:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyboardConfig {
    pub enabled: bool,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
        }
    }
}
```

- [ ] **Step 2: `model.rs` — ThrowConfig 계층화 (long_throw 객체화)**

같은 파일의 `ThrowConfig` 영역. 기존 `long_throw_enabled`, `long_throw_distance`, `long_throw_mapping` 평면 필드를 객체로 묶기:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LongThrowConfig {
    pub enabled: bool,
    pub distance: u32,
    pub mapping: SectorMap,
}

impl Default for LongThrowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            distance: 400,
            mapping: SectorMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThrowConfig {
    pub trigger_modifiers: Vec<String>,
    pub mapping: SectorMap,
    pub long_throw: LongThrowConfig,
}
```

`Config::default()`의 throw 부분도 새 구조로 업데이트. `long_throw_mapping` 기본값을 `long_throw.mapping`으로 이동.

- [ ] **Step 3: `model.rs` — OverlayConfig 계층화 + reticle_style 제거**

`OverlayConfig` (line 258-283) 교체. `reticle_style`은 데드 필드(그리는 코드 없음)이므로 제거. 보조 구조체들 먼저 추가:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CursorConfig {
    pub indicator: bool,
    pub radius: u32,
    pub color: String,
    pub opacity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewColors {
    pub throw_color: String,
    pub long_throw_color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapPreviewConfig {
    pub enabled: bool,
    pub colors: PreviewColors,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub cursor: CursorConfig,
    pub snap_preview: SnapPreviewConfig,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            cursor: CursorConfig {
                indicator: true,
                radius: 18,
                color: "#E53935".to_string(),
                opacity: 0.5,
            },
            snap_preview: SnapPreviewConfig {
                enabled: true,
                colors: PreviewColors {
                    throw_color: "#3B82F6".to_string(),
                    long_throw_color: "#3B82F6".to_string(),
                },
            },
        }
    }
}
```

- [ ] **Step 4: `cursor_fsm.rs` — sector_count 제거**

`src-tauri/src/domain/cursor_fsm.rs` line 38의 `pub sector_count: u8,` 제거. line 132의 `sector_count: 8,` 제거.

`CursorFsmConfig` (또는 해당 구조체)에서 `sector_count` 필드와 Default 값 제거.

- [ ] **Step 5: `ports.rs` — OverlayController 트레이트 시그니처 변경**

`src-tauri/src/application/ports.rs`의 `OverlayController` (line 39-45) 교체:

```rust
pub trait OverlayController: Send + Sync {
    fn show_reticle(&self, center_x: i32, center_y: i32) -> AppResult<()>;
    fn update_cursor_indicator(&self, x: i32, y: i32) -> AppResult<()>;
    fn highlight_sector(&self, sector: u8) -> AppResult<()>;
    fn show_snap_preview(&self, x: i32, y: i32, width: i32, height: i32, is_long_throw: bool) -> AppResult<()>;
    fn hide(&self) -> AppResult<()>;
}
```

- [ ] **Step 6: `win32_overlay.rs` — OverlayDrawState + draw_scene 교체**

`src-tauri/src/infrastructure/win32_overlay.rs`:
- `OverlayDrawState`에서 `sector_count` 필드 제거, `is_long_throw: bool` 추가.
- `draw_scene`의 cursor 참조 경로 업데이트: `cfg.cursor_color` → `cfg.cursor.color`, `cfg.cursor_radius` → `cfg.cursor.radius`, `cfg.cursor_opacity` → `cfg.cursor.opacity`, `cfg.cursor_indicator` → `cfg.cursor.indicator`.
- snap_preview 색상 선택 (line 429-435) 교체 — `state.is_long_throw` 분기 + `cfg.snap_preview.colors.*` 경로.

```rust
struct OverlayDrawState {
    visible: bool,
    #[allow(dead_code)]
    center: Option<(i32, i32)>,
    active_sector: Option<u8>,
    snap_preview: Option<(i32, i32, i32, i32)>,
    is_long_throw: bool,
    #[allow(dead_code)]
    cursor: Option<(i32, i32)>,
}
```

```rust
// snap_preview 색상 (기존 active_sector 분기 → is_long_throw 분기)
let color_hex = if state.is_long_throw {
    &cfg.snap_preview.colors.long_throw_color
} else {
    &cfg.snap_preview.colors.throw_color
};

// cursor indicator (경로만 변경)
if cfg.cursor.indicator {
    // ...
    let origin_color = Self::parse_hex_color(&cfg.cursor.color);
    let r = cfg.cursor.radius as f32;
    let c.a = cfg.cursor.opacity as f32;
    // ...
}
```

주석 line 374, 430도 업데이트 — "throw target (colors.throw_color)" / "long throw target (colors.long_throw_color)".

- [ ] **Step 7: `win32_overlay.rs` — OverlayController impl 시그니처 업데이트**

`impl OverlayController for Win32LayeredOverlay` (line 523-571)에서:
- `show_reticle`에서 `sector_count` 파라미터 제거 (line 528, 532)
- `show_snap_preview`에 `is_long_throw: bool` 추가, `state.is_long_throw = is_long_throw` 설정

```rust
fn show_reticle(&self, center_x: i32, center_y: i32) -> AppResult<()> {
    let mut state = self.state.lock().unwrap();
    state.visible = true;
    state.center = Some((center_x, center_y));
    state.active_sector = None;
    state.snap_preview = None;
    state.is_long_throw = false;
    drop(state);
    self.redraw();
    Ok(())
}

fn show_snap_preview(&self, x: i32, y: i32, width: i32, height: i32, is_long_throw: bool) -> AppResult<()> {
    let mut state = self.state.lock().unwrap();
    state.snap_preview = Some((x, y, width, height));
    state.is_long_throw = is_long_throw;
    drop(state);
    self.redraw();
    Ok(())
}
```

- [ ] **Step 8: `snap_service.rs` — 호출부 + long_throw 경로 업데이트**

`src-tauri/src/application/snap_service.rs`:
- line 87-94: `sector_count` 변수 제거, `show_reticle(cursor_x, cursor_y)` 호출로 교체
- line 122-125: `geometry::compute_sector` 호출에서 `sector_count` 대신 상수 `8` 사용
- line 174 주석 업데이트
- long throw 판별 (line 222-228) 경로 변경: `config.throw.long_throw_enabled` → `config.throw.long_throw.enabled`, `config.throw.long_throw_distance` → `config.throw.long_throw.distance`, `config.throw.long_throw_mapping` → `config.throw.long_throw.mapping`
- `let is_long_throw = config.throw.long_throw.enabled && throw_distance >= config.throw.long_throw.distance as f64;` 추출 후 `show_snap_preview`에 전달

```rust
let is_long_throw = config.throw.long_throw.enabled
    && throw_distance >= config.throw.long_throw.distance as f64;
let mapping = if is_long_throw {
    &config.throw.long_throw.mapping
} else {
    &config.throw.mapping
};
```

- [ ] **Step 9: `keyboard_service.rs` — show_reticle 호출 업데이트**

`src-tauri/src/application/keyboard_service.rs` line 176:

```rust
let _ = self.overlay.show_reticle(center.x, center.y);
```

- [ ] **Step 10: `toml_config.rs` + 테스트 수정**

`src-tauri/src/infrastructure/toml_config.rs` 테스트와 `src-tauri/src/application/snap_service.rs` 내 테스트(line 373-376 등)에서 구 필드/구 경로 참조 제거:
- `KeyboardConfig { enabled, cycle_timeout_ms }` → `KeyboardConfig { enabled }`
- `OverlayConfig`의 평면 `cursor_*`/`sector_*`/`snap_preview` → 새 객체 구조 (`cursor: CursorConfig::default()`, `snap_preview: SnapPreviewConfig::default()`)
- `ThrowConfig`의 `long_throw_enabled`/`long_throw_distance`/`long_throw_mapping` → `long_throw: LongThrowConfig { ... }`
- snap_service 테스트의 long_throw 매핑 검증도 새 경로 사용

- [ ] **Step 11: cargo test 실행**

```bash
cd src-tauri && cargo test
```

Expected: 전체 테스트 통과. 컴파일 에러 없음.

- [ ] **Step 12: 커밋**

```bash
git add -A
git commit -m "refactor: 백엔드 스키마 계층화 — cursor/snap_preview/long_throw 객체화

- OverlayConfig: cursor_*/snap_preview/preview_colors → cursor/snap_preview 객체
- ThrowConfig: long_throw_* 3개 → long_throw 객체
- KeyboardConfig: cycle_timeout_ms 제거
- OverlayController: show_reticle에서 sector_count 제거, show_snap_preview에 is_long_throw 추가
- win32_overlay draw_scene: throw 거리 기반 색상 분기 + cursor 경로 업데이트
- 하위 호환성 없음 (마이그레이션 생략)"
```

---

## Task 4: 프론트엔드 스키마 동기화 (Zod + default-config)

**Files:**
- Modify: `src/entities/config.ts`, `src/entities/default-config.ts`

**Interfaces:**
- Produces: TS 타입이 백엔드와 1:1. 이후 모든 페이지/컴포넌트는 새 스키마 사용.

- [ ] **Step 1: `config.ts` — overlay/throw/keyboard 스키마 계층화**

`src/entities/config.ts` 재정의. 기존 평면 필드를 객체로 재구성:

```ts
export const keyboardConfigSchema = z.object({
  enabled: z.boolean(),
})
export type KeyboardConfig = z.infer<typeof keyboardConfigSchema>

export const cursorConfigSchema = z.object({
  indicator: z.boolean(),
  radius: z.number().int(),
  color: z.string(),
  opacity: z.number(),
})
export type CursorConfig = z.infer<typeof cursorConfigSchema>

export const previewColorsSchema = z.object({
  throw_color: z.string(),
  long_throw_color: z.string(),
})
export type PreviewColors = z.infer<typeof previewColorsSchema>

export const snapPreviewConfigSchema = z.object({
  enabled: z.boolean(),
  colors: previewColorsSchema,
})
export type SnapPreviewConfig = z.infer<typeof snapPreviewConfigSchema>

export const overlayConfigSchema = z.object({
  reticle_style: z.string(),
  cursor: cursorConfigSchema,
  snap_preview: snapPreviewConfigSchema,
})
export type OverlayConfig = z.infer<typeof overlayConfigSchema>

export const longThrowConfigSchema = z.object({
  enabled: z.boolean(),
  distance: z.number().int(),
  mapping: sectorMapSchema,
})
export type LongThrowConfig = z.infer<typeof longThrowConfigSchema>

export const throwConfigSchema = z.object({
  trigger_modifiers: z.array(z.string()),
  mapping: sectorMapSchema,
  long_throw: longThrowConfigSchema,
})
export type ThrowConfig = z.infer<typeof throwConfigSchema>
```

`sector_highlight_color`, `sector_count`, `cycle_timeout_ms`, 평면 `cursor_*`/`long_throw_*` 제거 확인.

- [ ] **Step 2: `default-config.ts` — 기본값 계층화**

`src/entities/default-config.ts`의 `keyboard`, `throw`, `overlay` 교체:

```ts
  keyboard: {
    enabled: true,
  },
  throw: {
    trigger_modifiers: ['Win', 'Alt'],
    mapping: { '0': 'right-half', '2': 'bottom-half', '4': 'left-half', '6': 'top-half' },
    long_throw: {
      enabled: true,
      distance: 400,
      mapping: { '0': 'third-right', '2': 'maximize', '4': 'third-left', '6': 'maximize-height' },
    },
  },
  overlay: {
    reticle_style: 'pie',
    cursor: {
      indicator: true,
      radius: 18,
      color: '#E53935',
      opacity: 0.5,
    },
    snap_preview: {
      enabled: true,
      colors: {
        throw_color: '#3B82F6',
        long_throw_color: '#3B82F6',
      },
    },
  },
```

- [ ] **Step 3: 타입 체크**

```bash
pnpm build
```

Expected: vue-tsc 통과. (페이지 코드는 아직 구 필드 참조하지만, 다음 태스크에서 수정하므로 페이지 에러는 무시. entities 자체는 통과해야 함 — 먼저 페이지들을 주석처리하거나 임시로 빈 객체 반환. **간단화**: 이 태스크에서는 entities만 검증하고, 페이지는 Task 4-9에서 순차 수정. 빌드가 entities만 검증할 방법이 없으므로, `pnpm build` 대신 `pnpm vue-tsc --noEmit src/entities/*.ts` 형태는 불가 → Task 4에서 첫 페이지 교체 후 함께 빌드.)

실제 검증: `cd src && npx tsc --noEmit entities/config.ts entities/default-config.ts` (vue-tsc 없이 타입만). 또는 생략하고 Task 9 끝에서 전체 빌드.

- [ ] **Step 4: 커밋**

```bash
git add src/entities/
git commit -m "refactor: 프론트엔드 Zod 스키마 동기화 — preview_colors 추가

- overlayConfigSchema: sector_* 제거, preview_colors 추가
- keyboardConfigSchema: cycle_timeout_ms 제거
- default-config.ts 동기화"
```

---

## Task 5: SettingsLayout + App.vue 재작성 (UDashboard 기반)

**Files:**
- Create: `src/layouts/SettingsLayout.vue`, `src/components/SaveActions.vue`
- Modify: `src/App.vue`, `src/main.ts` (라우트 — keyboard 제거)
- Delete: `src/components/SettingsLayout.vue`, `src/components/PageHeader.vue`, `src/components/SaveBar.vue`, `src/components/USection.vue`

**Interfaces:**
- Produces: `SettingsLayout` (layout), `SaveActions` 컴포넌트. 모든 페이지는 이 레이아웃 안에서 `UDashboardPanel` 사용.

- [ ] **Step 1: 기존 컴포넌트 삭제**

```bash
rm src/components/SettingsLayout.vue
rm src/components/PageHeader.vue
rm src/components/SaveBar.vue
rm src/components/USection.vue
rm src/pages/Keyboard.vue
```

- [ ] **Step 2: `SaveActions.vue` 생성**

`src/components/SaveActions.vue`:

```vue
<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{
  dirty: boolean
  saving?: boolean
}>()
const emit = defineEmits<{ save: []; reset: [] }>()
const { t } = useI18n()
</script>

<template>
  <div class="flex items-center gap-2">
    <UButton
      :label="t('common.reset')"
      icon="i-lucide-rotate-ccw"
      color="neutral"
      variant="ghost"
      :disabled="!dirty || saving"
      @click="emit('reset')"
    />
    <UButton
      :label="t('common.save')"
      icon="i-lucide-save"
      color="primary"
      variant="solid"
      :loading="saving"
      :disabled="!dirty"
      @click="emit('save')"
    />
  </div>
</template>
```

- [ ] **Step 3: `SettingsLayout.vue` (layout) 생성**

`src/layouts/SettingsLayout.vue`:

```vue
<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { NavigationMenuItem } from '@nuxt/ui'
import { useConfigStore } from '@/features/config-store'

const { t } = useI18n()
const store = useConfigStore()

const navItems = computed<NavigationMenuItem[][]>(() => [
  [
    { label: t('nav.general'), icon: 'i-lucide-settings', to: '/general' },
    { label: t('nav.throw'), icon: 'i-lucide-mouse-pointer-click', to: '/throw' },
    { label: t('nav.snapEditor'), icon: 'i-lucide-layout-grid', to: '/snap-editor' },
    { label: t('nav.display'), icon: 'i-lucide-monitor', to: '/display' },
    { label: t('nav.about'), icon: 'i-lucide-info', to: '/about' },
  ],
])

async function toggleEnabled(value: boolean) {
  if (!store.draft) return
  store.draft.keyboard.enabled = value
  await store.save()
}
</script>

<template>
  <UDashboardGroup>
    <UDashboardSidebar>
      <template #header>
        <div class="flex w-full items-center justify-between">
          <div class="flex items-center gap-2">
            <UIcon name="i-lucide-square" class="size-5 text-primary" />
            <span class="text-sm font-semibold">{{ t('app.name') }}</span>
          </div>
          <UColorModeButton />
        </div>
      </template>

      <template #default>
        <UNavigationMenu :items="navItems" orientation="vertical" class="w-full" />
      </template>

      <template #footer>
        <USeparator class="mb-3" />
        <div class="space-y-3">
          <div class="flex items-center justify-between gap-2">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-zap" class="size-4 text-muted" />
              <span class="text-sm">{{ t('nav.enableSnap') }}</span>
            </div>
            <USwitch
              :model-value="store.draft?.keyboard.enabled ?? false"
              @update:model-value="toggleEnabled($event as boolean)"
            />
          </div>
          <UButton
            :label="t('nav.quit')"
            icon="i-lucide-power"
            color="error"
            variant="ghost"
            block
          />
        </div>
      </template>
    </UDashboardSidebar>

    <RouterView />
  </UDashboardGroup>
</template>
```

- [ ] **Step 4: `App.vue` 수정**

`src/App.vue`:

```vue
<script setup lang="ts">
import SettingsLayout from '@/layouts/SettingsLayout.vue'
</script>

<template>
  <UApp>
    <SettingsLayout />
  </UApp>
</template>
```

- [ ] **Step 5: `main.ts` 라우트 — keyboard 제거**

`src/main.ts`의 routes 배열에서 keyboard 라우트 제거:

```ts
const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/general' },
    { path: '/general', name: 'general', component: () => import('./pages/General.vue') },
    { path: '/throw', name: 'throw', component: () => import('./pages/Throw.vue') },
    { path: '/snap-editor', name: 'snap-editor', component: () => import('./pages/SnapEditor.vue') },
    { path: '/display', name: 'display', component: () => import('./pages/Display.vue') },
    { path: '/about', name: 'about', component: () => import('./pages/About.vue') },
  ],
})
```

- [ ] **Step 6: i18n 키 임시 추가 (레이아웃 동작용)**

`src/i18n/locales/ko.ts`와 `en.ts`의 `nav` 섹션에 `enableSnap` 추가, `pause` 제거, `keyboard` 제거:

ko.ts nav 부분:
```ts
  nav: {
    settings: '설정',
    general: '일반',
    throw: '스로우',
    snapEditor: '스냅 에디터',
    display: '디스플레이',
    about: '정보',
    enableSnap: '스냅 활성화',
    quit: '종료',
  },
```

en.ts nav 부분:
```ts
  nav: {
    settings: 'Settings',
    general: 'General',
    throw: 'Throw',
    snapEditor: 'Snap Editor',
    display: 'Display',
    about: 'About',
    enableSnap: 'Enable snap',
    quit: 'Quit',
  },
```

- [ ] **Step 7: 커밋 (페이지들이 깨지지만 순차 수정 예정)**

이 시점에서 페이지들은 구 USection/PageHeader/SaveBar를 참조하므로 빌드가 깨짐. 다음 태스크들에서 페이지별로 수정. 먼저 커밋:

```bash
git add -A
git commit -m "refactor: SettingsLayout을 UDashboard 기반으로 재작성

- UDashboardGroup/Sidebar/Panel 구조 도입
- SaveActions 컴포넌트 (UDashboardNavbar #right용)
- 사이드바 footer: 스냅 활성화 토글 + 종료
- Keyboard 페이지/라우트 제거
- 구 SettingsLayout/PageHeader/SaveBar/USection 제거
- 페이지들은 후속 태스크에서 순차 마이그레이션"
```

---

## Task 6: General 페이지 재작성

**Files:**
- Modify: `src/pages/General.vue`
- Modify: `src/i18n/index.ts`, `src/i18n/locales/{en,ko}.ts`

**Interfaces:**
- Consumes: `SaveActions` (Task 4), `useConfigStore`
- Produces: 언어 단일 진실 공급원 (`config.general.language` ↔ `locale` 동기화).

- [ ] **Step 1: `i18n/index.ts` — 기본 locale 'ko' + config 연동 준비**

`src/i18n/index.ts` 수정 — 기본 locale을 'ko'로:

```ts
export const i18n = createI18n({
  legacy: false,
  locale: 'ko',
  fallbackLocale: 'en',
  messages: {
    en: { ...en, ...enMessages },
    ko: { ...ko, ...koMessages },
  },
})
```

- [ ] **Step 2: `config-store.ts` — load 후 locale 동기화**

`src/features/config-store.ts`의 `load()` 함수에서 draft 설정 후 i18n locale 적용. 파일 상단에 `import { i18n } from '@/i18n'` 추가 후:

```ts
  async function load() {
    loading.value = true
    error.value = null
    try {
      const config = await api.getConfig()
      saved.value = config
      draft.value = JSON.parse(JSON.stringify(config))
      // 언어 단일 진실: config.general.language → i18n locale 동기화
      // (legacy: false 이므로 i18n.global.locale은 ref, .value로 접근)
      i18n.global.locale.value = config.general.language
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }
```

- [ ] **Step 3: `General.vue` 재작성**

`src/pages/General.vue` 전체 교체:

```vue
<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/features/config-store'
import { i18n } from '@/i18n'
import SaveActions from '@/components/SaveActions.vue'

const { t } = useI18n()
const store = useConfigStore()

onMounted(() => store.load())

function changeLanguage(lang: string) {
  i18n.global.locale.value = lang
  if (store.draft) {
    store.draft.general.language = lang
  }
}

const languageItems = [
  { label: 'English', value: 'en' },
  { label: '한국어', value: 'ko' },
]
</script>

<template>
  <UDashboardPanel>
    <template #header>
      <UDashboardNavbar :title="t('general.title')">
        <template #right>
          <SaveActions
            v-if="store.draft"
            :dirty="store.isDirty"
            :saving="store.saving"
            @save="store.save()"
            @reset="store.reset()"
          />
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <UContainer class="max-w-3xl space-y-10 py-8">
        <div v-if="store.loading" class="py-8 text-center text-muted">
          <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin" />
        </div>
        <UAlert
          v-else-if="store.error"
          color="error"
          variant="soft"
          icon="i-lucide-alert-circle"
          :title="store.error"
        />

        <template v-else-if="store.draft">
          <!-- 시작 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-power" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('general.startup') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('general.launchAtLogin')" :description="t('general.launchAtLoginDesc')">
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted" />
                <USwitch v-model="store.draft.general.launch_at_login" />
              </div>
            </UFormField>
            <UFormField :label="t('general.startMinimized')" :description="t('general.startMinimizedDesc')">
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted" />
                <USwitch v-model="store.draft.general.start_minimized" />
              </div>
            </UFormField>
          </section>

          <!-- 시스템 트레이 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-tray" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('general.tray') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('general.showInTray')" :description="t('general.showInTrayDesc')">
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted" />
                <USwitch v-model="store.draft.general.show_in_tray" />
              </div>
            </UFormField>
          </section>

          <!-- 언어 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-languages" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('general.language') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('general.language')" :description="t('general.languageDesc')">
              <USelect
                :model-value="store.draft.general.language"
                :items="languageItems"
                value-key="value"
                class="w-full"
                @update:model-value="changeLanguage($event as string)"
              />
            </UFormField>
          </section>
        </template>
      </UContainer>
    </template>
  </UDashboardPanel>
</template>
```

- [ ] **Step 4: 수동 검증**

```bash
pnpm tauri:dev
```

브라우저에서 `/general` 진입. 언어 셀렉트 변경 시 UI 즉시 전환, 사이드바에 언어 셀렉트 없음 확인. 저장/초기화 버튼이 navbar 우측에 표시되는지.

- [ ] **Step 5: 커밋**

```bash
git add -A
git commit -m "feat: General 페이지 재작성 — flat 섹션 + 언어 단일 진실

- UDashboardPanel/Navbar + UContainer flat 섹션 (소제목 + USeparator)
- SaveActions를 navbar #right로 이동
- 언어: config.general.language 단일 진실, i18n locale 동기화
- i18n 기본 locale 'ko'로 변경"
```

---

## Task 7: UHotkeyInput 컴포넌트 + Throw 페이지 재작성

**Files:**
- Create: `src/components/UHotkeyInput.vue`, `src/components/SectorMappingTable.vue`
- Modify: `src/pages/Throw.vue`, `src/i18n/locales/{en,ko}.ts`

**Interfaces:**
- Produces: `UHotkeyInput` (`v-model: string[]` modifier 배열), `SectorMappingTable` (`mapping`/`longThrowMapping` props + update 이벤트).

- [ ] **Step 1: `UHotkeyInput.vue` 생성**

`src/components/UHotkeyInput.vue`:

```vue
<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  modelValue: string[]
}>()
const emit = defineEmits<{ 'update:modelValue': [value: string[]] }>()
const { t } = useI18n()

const capturing = ref(false)

// modifier 키 매핑 (e.code → 백엔드 토큰)
const CODE_TO_TOKEN: Record<string, string> = {
  ControlLeft: 'Ctrl',
  ControlRight: 'Ctrl',
  ShiftLeft: 'Shift',
  ShiftRight: 'Shift',
  AltLeft: 'Alt',
  AltRight: 'Alt',
  MetaLeft: 'Win',
  MetaRight: 'Win',
}

const displayBadges = computed(() =>
  props.modelValue.length > 0 ? props.modelValue : [t('throw.noHotkey')],
)

function toggleCapture() {
  capturing.value = !capturing.value
}

function onKeydown(e: KeyboardEvent) {
  if (!capturing.value) return
  e.preventDefault()
  e.stopPropagation()

  if (e.key === 'Escape') {
    capturing.value = false
    return
  }

  // 눌린 modifier들을 수집
  const mods = new Set<string>()
  // e.code 기반으로 모든 눌린 modifier 감지는 불가 (단일 이벤트만).
  // 따라서 현재 이벤트의 modifier 플래그 사용.
  if (e.ctrlKey) mods.add('Ctrl')
  if (e.shiftKey) mods.add('Shift')
  if (e.altKey) mods.add('Alt')
  if (e.metaKey) mods.add('Win')

  // modifier 키 자체를 누른 경우에도 해당 토큰 추가
  const token = CODE_TO_TOKEN[e.code]
  if (token) mods.add(token)

  // 빈 조합은 거부 (백엔드 check_throw_modifiers가 빈 조합 거부)
  if (mods.size === 0) return

  // 정해진 순서로 정렬 (Win, Ctrl, Alt, Shift)
  const order = ['Win', 'Ctrl', 'Alt', 'Shift']
  const sorted = order.filter((m) => mods.has(m))

  emit('update:modelValue', sorted)
  capturing.value = false
}

function onBlur() {
  capturing.value = false
}

function clearHotkey() {
  emit('update:modelValue', [])
}
</script>

<template>
  <div class="space-y-2">
    <div class="flex items-center gap-2">
      <button
        type="button"
        class="flex flex-1 items-center gap-2 rounded-md border border-default px-3 py-2 text-left text-sm transition-colors"
        :class="capturing ? 'border-primary bg-primary/5' : 'hover:bg-elevated/50'"
        @click="toggleCapture"
        @keydown="onKeydown"
        @blur="onBlur"
      >
        <UIcon
          v-if="capturing"
          name="i-lucide-keyboard"
          class="size-4 animate-pulse text-primary"
        />
        <UBadge
          v-for="mod in displayBadges"
          :key="mod"
          :label="mod"
          color="neutral"
          variant="subtle"
          size="sm"
        />
        <span v-if="capturing" class="ml-auto text-xs text-muted">
          {{ t('throw.capturing') }}
        </span>
      </button>
      <UButton
        v-if="modelValue.length > 0"
        icon="i-lucide-x"
        color="neutral"
        variant="ghost"
        size="sm"
        @click="clearHotkey"
      />
    </div>
    <p class="text-xs text-muted">
      {{ capturing ? t('throw.captureHint') : t('throw.captureHotkeyDesc') }}
    </p>
  </div>
</template>
```

- [ ] **Step 2: `SectorMappingTable.vue` 생성 (SectorMapping 대체)**

`src/components/SectorMappingTable.vue`:

```vue
<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SnapTarget, SectorMap } from '@/entities/config'

const props = defineProps<{
  targets: SnapTarget[]
  mapping: SectorMap
  longThrowMapping: SectorMap
}>()

const emit = defineEmits<{
  'update:mapping': [value: SectorMap]
  'update:longThrowMapping': [value: SectorMap]
}>()

const { t } = useI18n()

// 8 섹터 고정 (백엔드 기본값)
const SECTOR_COUNT = 8
const sectors = computed(() => Array.from({ length: SECTOR_COUNT }, (_, i) => i))

const targetOptions = computed(() =>
  props.targets.map((tgt) => ({ label: tgt.name, value: tgt.id })),
)

const sectorLabels: Record<number, string> = {
  0: '→', 1: '↘', 2: '↓', 3: '↙',
  4: '←', 5: '↖', 6: '↑', 7: '↗',
}

function getTarget(map: SectorMap, sector: number): string {
  return map[String(sector)] ?? ''
}

function setTarget(map: SectorMap, sector: number, targetId: string): SectorMap {
  const next = { ...map }
  if (targetId) next[String(sector)] = targetId
  else delete next[String(sector)]
  return next
}
</script>

<template>
  <UCard variant="subtle">
    <div class="grid grid-cols-2 gap-6">
      <!-- 기본 throw 매핑 -->
      <div>
        <h4 class="mb-3 text-sm font-medium text-muted">{{ t('throw.title') }}</h4>
        <div class="space-y-2">
          <div v-for="sector in sectors" :key="sector" class="flex items-center gap-2">
            <span class="w-8 text-center text-lg">{{ sectorLabels[sector] ?? sector }}</span>
            <USelect
              :model-value="getTarget(mapping, sector)"
              :items="targetOptions"
              value-key="value"
              size="sm"
              class="flex-1"
              @update:model-value="emit('update:mapping', setTarget(mapping, sector, $event as string))"
            />
          </div>
        </div>
      </div>
      <!-- Long throw 매핑 -->
      <div>
        <h4 class="mb-3 text-sm font-medium text-muted">{{ t('throw.longThrow') }}</h4>
        <div class="space-y-2">
          <div v-for="sector in sectors" :key="sector" class="flex items-center gap-2">
            <span class="w-8 text-center text-lg">{{ sectorLabels[sector] ?? sector }}</span>
            <USelect
              :model-value="getTarget(longThrowMapping, sector)"
              :items="targetOptions"
              value-key="value"
              size="sm"
              class="flex-1"
              @update:model-value="emit('update:longThrowMapping', setTarget(longThrowMapping, sector, $event as string))"
            />
          </div>
        </div>
      </div>
    </div>
  </UCard>
</template>
```

- [ ] **Step 3: `Throw.vue` 재작성**

`src/pages/Throw.vue` 전체 교체:

```vue
<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/features/config-store'
import SaveActions from '@/components/SaveActions.vue'
import UHotkeyInput from '@/components/UHotkeyInput.vue'
import SectorMappingTable from '@/components/SectorMappingTable.vue'

const { t } = useI18n()
const store = useConfigStore()

onMounted(() => store.load())
</script>

<template>
  <UDashboardPanel>
    <template #header>
      <UDashboardNavbar :title="t('throw.title')">
        <template #right>
          <SaveActions
            v-if="store.draft"
            :dirty="store.isDirty"
            :saving="store.saving"
            @save="store.save()"
            @reset="store.reset()"
          />
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <UContainer class="max-w-3xl space-y-10 py-8">
        <div v-if="store.loading" class="py-8 text-center text-muted">
          <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin" />
        </div>

        <template v-else-if="store.draft">
          <!-- 핫키 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-keyboard" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('throw.triggerModifiers') }}</h3>
            </div>
            <USeparator />
            <UFormField :description="t('throw.triggerModifiersDesc')">
              <UHotkeyInput v-model="store.draft.throw.trigger_modifiers" />
            </UFormField>
          </section>

          <!-- Long Throw -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-arrow-right" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('throw.longThrow') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('throw.longThrowEnabled')" :description="t('throw.longThrowEnabledDesc')">
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted" />
                <USwitch v-model="store.draft.throw.long_throw_enabled" />
              </div>
            </UFormField>
            <UFormField
              v-if="store.draft.throw.long_throw_enabled"
              :label="t('throw.longThrowDistance')"
              :description="t('throw.longThrowDistanceDesc')"
            >
              <USlider
                v-model="store.draft.throw.long_throw_distance"
                :min="100"
                :max="1000"
                :step="50"
                class="w-full"
              />
              <template #hint>{{ store.draft.throw.long_throw_distance }}px</template>
            </UFormField>
          </section>

          <!-- Sector 매핑 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-pie-chart" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('throw.sectorMapping') }}</h3>
            </div>
            <USeparator />
            <SectorMappingTable
              :targets="store.draft.snap.areas"
              :mapping="store.draft.throw.mapping"
              :long-throw-mapping="store.draft.throw.long_throw_mapping"
              @update:mapping="store.draft!.throw.mapping = $event"
              @update:long-throw-mapping="store.draft!.throw.long_throw_mapping = $event"
            />
          </section>
        </template>
      </UContainer>
    </template>
  </UDashboardPanel>
</template>
```

- [ ] **Step 4: i18n 키 추가 (throw 섹션)**

ko.ts throw 부분에 추가:
```ts
  throw: {
    title: '윈도우 스로우',
    description: '커서 이동 스냅 설정',
    triggerModifiers: '트리거 수정자',
    triggerModifiersDesc: '이 키들을 누르고 있으면 스로우 활성화',
    captureHotkeyDesc: '클릭하여 단축키를 입력하세요 (modifier 키 조합)',
    capturing: '입력 대기 중...',
    captureHint: 'ESC로 취소. modifier 키(Win/Ctrl/Alt/Shift)를 누르세요',
    noHotkey: '설정 안 됨',
    sectorMapping: '섹터 매핑',
    longThrow: '롱 스로우',
    longThrowEnabled: '롱 스로우 활성화',
    longThrowEnabledDesc: '커서를 멀리 이동할 때 별도 매핑 사용',
    longThrowDistance: '롱 스로우 거리',
    longThrowDistanceDesc: '픽셀 단위 거리 임계값',
  },
```

en.ts throw 부분에 추가:
```ts
  throw: {
    title: 'Window Throw',
    description: 'Cursor movement snap settings',
    triggerModifiers: 'Trigger Modifiers',
    triggerModifiersDesc: 'Hold these keys to activate throw',
    captureHotkeyDesc: 'Click to input a shortcut (modifier key combination)',
    capturing: 'Waiting for input...',
    captureHint: 'ESC to cancel. Press modifier keys (Win/Ctrl/Alt/Shift)',
    noHotkey: 'Not set',
    sectorMapping: 'Sector Mapping',
    longThrow: 'Long Throw',
    longThrowEnabled: 'Enable long throw',
    longThrowEnabledDesc: 'Use separate mapping when cursor moves far',
    longThrowDistance: 'Long throw distance',
    longThrowDistanceDesc: 'Distance threshold in pixels',
  },
```

- [ ] **Step 5: 수동 검증 + 커밋**

```bash
pnpm tauri:dev
```

핫키 캡처 박스 클릭 → Win+Alt 입력 → 배지 표시 → 저장. 매핑 표에서 sector별 영역 선택.

```bash
git add -A
git commit -m "feat: Throw 페이지 재작성 — 핫키 캡처 + 매핑 표

- UHotkeyInput 컴포넌트 (키보드 이벤트 캡처 → modifier string[])
- SectorMappingTable 컴포넌트 (8섹터 throw/long_throw 매핑)
- Throw 페이지 flat 섹션 구조
- SectorMapping.vue 제거 (SectorMappingTable로 대체)"
```

> 참고: `src/components/SectorMapping.vue`가 아직 남아있다면 이 태스크에서 삭제. Task 4에서 이미 삭제 지시했으나 누락 시 여기서 `rm src/components/SectorMapping.vue`.

---

## Task 8: ColorLockField 컴포넌트 + Display 페이지 재작성

**Files:**
- Create: `src/components/ColorLockField.vue`
- Modify: `src/pages/Display.vue`, `src/i18n/locales/{en,ko}.ts`

**Interfaces:**
- Consumes: `PreviewColors` 타입 (Task 3)
- Produces: `ColorLockField` (`v-model: PreviewColors`).

- [ ] **Step 1: `ColorLockField.vue` 생성**

`src/components/ColorLockField.vue`:

```vue
<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { PreviewColors } from '@/entities/config'

const props = defineProps<{
  modelValue: PreviewColors
}>()
const emit = defineEmits<{ 'update:modelValue': [value: PreviewColors] }>()
const { t } = useI18n()

// 잠금 상태: 초기 두 색상이 같으면 잠금, 다르면 열림
const locked = ref(props.modelValue.throw_color === props.modelValue.long_throw_color)

function patch(patch: Partial<PreviewColors>) {
  emit('update:modelValue', { ...props.modelValue, ...patch })
}

// throw 변경 — 잠금 시 long_throw 동기화
function updateThrow(value: string) {
  if (locked.value) {
    patch({ throw_color: value, long_throw_color: value })
  } else {
    patch({ throw_color: value })
  }
}

// long_throw 변경 — 잠금 시 throw 동기화
function updateLongThrow(value: string) {
  if (locked.value) {
    patch({ throw_color: value, long_throw_color: value })
  } else {
    patch({ long_throw_color: value })
  }
}

function toggleLock() {
  locked.value = !locked.value
  if (locked.value) {
    // 잠금 시 두 색상을 throw 기준으로 동기화
    patch({ long_throw_color: props.modelValue.throw_color })
  }
}

// 외부 modelValue 변경 시(초기 로드 등) 잠금 상태 재판별
watch(
  () => props.modelValue,
  (val) => {
    locked.value = val.throw_color === val.long_throw_color
  },
)
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center justify-end">
      <UButton
        :icon="locked ? 'i-lucide-lock' : 'i-lucide-lock-open'"
        :label="locked ? t('display.colorsLocked') : t('display.colorsUnlocked')"
        color="neutral"
        variant="ghost"
        size="xs"
        @click="toggleLock"
      />
    </div>
    <UFormField :label="t('display.throwColor')">
      <UColorPicker
        :model-value="modelValue.throw_color"
        @update:model-value="updateThrow($event as string)"
      />
    </UFormField>
    <UFormField :label="t('display.longThrowColor')">
      <UColorPicker
        :model-value="modelValue.long_throw_color"
        @update:model-value="updateLongThrow($event as string)"
      />
    </UFormField>
  </div>
</template>
```

- [ ] **Step 2: `Display.vue` 재작성**

`src/pages/Display.vue` 전체 교체:

```vue
<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/features/config-store'
import SaveActions from '@/components/SaveActions.vue'
import ColorLockField from '@/components/ColorLockField.vue'

const { t } = useI18n()
const store = useConfigStore()

onMounted(() => store.load())

const reticleStyles = [
  { label: 'Pie', value: 'pie' },
  { label: 'Crosshair', value: 'crosshair' },
  { label: 'Minimal', value: 'minimal' },
]
</script>

<template>
  <UDashboardPanel>
    <template #header>
      <UDashboardNavbar :title="t('display.title')">
        <template #right>
          <SaveActions
            v-if="store.draft"
            :dirty="store.isDirty"
            :saving="store.saving"
            @save="store.save()"
            @reset="store.reset()"
          />
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <UContainer class="max-w-3xl space-y-10 py-8">
        <div v-if="store.loading" class="py-8 text-center text-muted">
          <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin" />
        </div>

        <template v-else-if="store.draft">
          <!-- 레티클 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-target" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('display.reticle') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('display.reticleStyle')">
              <USelect
                v-model="store.draft.overlay.reticle_style"
                :items="reticleStyles"
                value-key="value"
                class="w-full"
              />
            </UFormField>
          </section>

          <!-- 커서 표시기 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-mouse-pointer" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('display.cursorIndicator') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('display.cursorIndicator')" :description="t('display.cursorIndicatorDesc')">
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted" />
                <USwitch v-model="store.draft.overlay.cursor_indicator" />
              </div>
            </UFormField>
            <template v-if="store.draft.overlay.cursor_indicator">
              <UFormField :label="t('display.cursorColor')">
                <UColorPicker v-model="store.draft.overlay.cursor_color" />
              </UFormField>
              <UFormField :label="t('display.cursorRadius')">
                <USlider
                  v-model="store.draft.overlay.cursor_radius"
                  :min="5"
                  :max="50"
                  :step="1"
                  class="w-full"
                />
                <template #hint>{{ store.draft.overlay.cursor_radius }}px</template>
              </UFormField>
              <UFormField :label="t('display.cursorOpacity')">
                <USlider
                  v-model="store.draft.overlay.cursor_opacity"
                  :min="0.1"
                  :max="1"
                  :step="0.05"
                  class="w-full"
                />
                <template #hint>{{ Math.round(store.draft.overlay.cursor_opacity * 100) }}%</template>
              </UFormField>
            </template>
          </section>

          <!-- Snap Preview -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-square-dashed" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('display.snapPreview') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('display.snapPreview')" :description="t('display.snapPreviewDesc')">
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted" />
                <USwitch v-model="store.draft.overlay.snap_preview" />
              </div>
            </UFormField>
            <template v-if="store.draft.overlay.snap_preview">
              <ColorLockField v-model="store.draft.overlay.preview_colors" />
            </template>
          </section>
        </template>
      </UContainer>
    </template>
  </UDashboardPanel>
</template>
```

- [ ] **Step 3: i18n 키 업데이트 (display 섹션)**

ko.ts display 부분 (sector 관련 키 제거, 색상 키 추가):
```ts
  display: {
    title: '디스플레이',
    description: '레티클 및 오버레이 외관',
    reticle: '레티클',
    reticleStyle: '레티클 스타일',
    cursorIndicator: '커서 표시기',
    cursorIndicatorDesc: '커서 위치에 빨간색 반투명 원 표시',
    cursorRadius: '커서 반지름',
    cursorColor: '커서 색상',
    cursorOpacity: '커서 불투명도',
    snapPreview: '스냅 미리보기',
    snapPreviewDesc: '대상 영역 사각형 미리보기 표시',
    throwColor: 'Throw 색상',
    longThrowColor: 'Long Throw 색상',
    colorsLocked: '색상 통일 (잠금)',
    colorsUnlocked: '색상 분리 (열림)',
  },
```

en.ts display 부분:
```ts
  display: {
    title: 'Display',
    description: 'Reticle and overlay appearance',
    reticle: 'Reticle',
    reticleStyle: 'Reticle style',
    cursorIndicator: 'Cursor indicator',
    cursorIndicatorDesc: 'Show red translucent circle at cursor',
    cursorRadius: 'Cursor radius',
    cursorColor: 'Cursor color',
    cursorOpacity: 'Cursor opacity',
    snapPreview: 'Snap preview',
    snapPreviewDesc: 'Show target area rectangle preview',
    throwColor: 'Throw color',
    longThrowColor: 'Long throw color',
    colorsLocked: 'Colors unified (locked)',
    colorsUnlocked: 'Colors separate (unlocked)',
  },
```

`sectorCount`, `sectorCountDesc`, `sectorHighlightColor`, `sectors` 키 제거.

- [ ] **Step 4: 수동 검증 + 커밋**

```bash
pnpm tauri:dev
```

잠금 상태에서 throw 색상 변경 → long_throw 자동 동기화. 열기 → 두 색상 개별 편집. 다시 잠금 → throw 기준으로 동기화.

```bash
git add -A
git commit -m "feat: Display 페이지 재작성 — Snap Preview 색상 lock 토글

- ColorLockField 컴포넌트 (throw/long_throw 색상 양방향 동기화)
- 잠금 상태: 한쪽 변경 시 반대쪽 즉시 동기화
- 열림 상태: 두 색상 개별 편집
- 초기 진입: 두 값이 같으면 잠금, 다르면 열림 자동 판별
- sector 섹션 제거 (8 고정)"
```

---

## Task 9: SnapEditor 재작성 (UTable expandable rows)

**Files:**
- Modify: `src/pages/SnapEditor.vue`, `src/components/SnapCanvas.vue`, `src/components/SnapProperties.vue`
- Modify: `src/i18n/locales/{en,ko}.ts`
- Modify: `src/features/api.ts` (applyPreset 제거 검토 — 스펙에서 active_preset 유지하되 UI 숨김이므로 api는 유지)

**Interfaces:**
- Consumes: `SnapTarget`, `SnapTargetArea` (Task 3), `SaveActions`
- Produces: SnapEditor 페이지. SnapAreas만 편집, 매핑/프리셋/Import-Export 제거.

- [ ] **Step 1: `SnapCanvas.vue` 축소형으로 재작성**

`src/components/SnapCanvas.vue` 전체 교체 — 더 작은 고정 폭:

```vue
<script setup lang="ts">
import { computed } from 'vue'
import type { SnapTargetArea } from '@/entities/config'

const props = defineProps<{
  area: SnapTargetArea | null
}>()

const emit = defineEmits<{
  update: [id: string, patch: Partial<SnapTargetArea>]
}>()

const CANVAS_W = 320
const CANVAS_H = Math.round(CANVAS_W * 9 / 16)

const rectConfig = computed(() => {
  if (!props.area) return null
  return {
    x: props.area.x_ratio * CANVAS_W,
    y: props.area.y_ratio * CANVAS_H,
    width: props.area.w_ratio * CANVAS_W,
    height: props.area.h_ratio * CANVAS_H,
    fill: 'rgb(var(--ui-color-primary-500) / 0.3)',
    stroke: 'rgb(var(--ui-color-primary-500))',
    strokeWidth: 2,
    cornerRadius: 2,
    draggable: true,
  }
})

function onDragEnd(e: any) {
  if (!props.area) return
  const x = e.target.x() / CANVAS_W
  const y = e.target.y() / CANVAS_H
  emit('update', props.area.id, {
    x_ratio: Math.max(0, Math.min(1 - props.area.w_ratio, x)),
    y_ratio: Math.max(0, Math.min(1 - props.area.h_ratio, y)),
  })
}

function onTransformEnd(e: any) {
  if (!props.area) return
  const t = e.target
  const scaleX = t.scaleX()
  const scaleY = t.scaleY()
  t.scaleX(1)
  t.scaleY(1)
  emit('update', props.area.id, {
    x_ratio: Math.max(0, t.x() / CANVAS_W),
    y_ratio: Math.max(0, t.y() / CANVAS_H),
    w_ratio: Math.max(0.05, Math.min(1, (t.width() * scaleX) / CANVAS_W)),
    h_ratio: Math.max(0.05, Math.min(1, (t.height() * scaleY) / CANVAS_H)),
  })
}
</script>

<template>
  <div class="flex items-center justify-center rounded-md border border-default bg-elevated/30 p-3">
    <v-stage :config="{ width: CANVAS_W, height: CANVAS_H }" class="rounded">
      <v-layer>
        <v-rect
          :config="{
            x: 0, y: 0,
            width: CANVAS_W, height: CANVAS_H,
            stroke: 'rgb(var(--ui-color-neutral-500))',
            strokeWidth: 1,
            cornerRadius: 4,
          }"
        />
        <v-rect
          v-if="rectConfig"
          :config="rectConfig"
          @dragend="onDragEnd"
          @transformend="onTransformEnd"
        />
      </v-layer>
    </v-stage>
  </div>
</template>
```

- [ ] **Step 2: `SnapProperties.vue` 재작성 (확장용 폼)**

`src/components/SnapProperties.vue` 전체 교체 — 헤더/불필요한 카드 제거:

```vue
<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { SnapTarget, WindowAction } from '@/entities/config'

const props = defineProps<{
  target: SnapTarget | null
}>()

const emit = defineEmits<{
  update: [patch: Partial<SnapTarget>]
}>()

const { t } = useI18n()

const actionOptions: { label: string; value: WindowAction }[] = [
  { label: 'Maximize', value: 'Maximize' },
  { label: 'Minimize', value: 'Minimize' },
  { label: 'Restore', value: 'Restore' },
  { label: 'Center', value: 'Center' },
  { label: 'Almost Maximize', value: 'AlmostMaximize' },
  { label: 'Maximize Height', value: 'MaximizeHeight' },
  { label: 'Next Display', value: 'NextDisplay' },
  { label: 'Previous Display', value: 'PreviousDisplay' },
]

function updateArea(field: 'x_ratio' | 'y_ratio' | 'w_ratio' | 'h_ratio', value: number) {
  if (props.target?.kind === 'area') {
    emit('update', { [field]: value } as Partial<SnapTarget>)
  }
}
</script>

<template>
  <div v-if="target" class="grid gap-4 sm:grid-cols-2">
    <UFormField :label="t('snapEditor.name')" class="sm:col-span-2">
      <UInput
        :model-value="target.name"
        class="w-full"
        @update:model-value="emit('update', { name: $event as string } as Partial<SnapTarget>)"
      />
    </UFormField>

    <template v-if="target.kind === 'area'">
      <UFormField label="X">
        <USlider :model-value="target.x_ratio" :min="0" :max="1" :step="0.01" class="w-full"
          @update:model-value="updateArea('x_ratio', $event)" />
        <template #hint>{{ (target.x_ratio * 100).toFixed(0) }}%</template>
      </UFormField>
      <UFormField label="Y">
        <USlider :model-value="target.y_ratio" :min="0" :max="1" :step="0.01" class="w-full"
          @update:model-value="updateArea('y_ratio', $event)" />
        <template #hint>{{ (target.y_ratio * 100).toFixed(0) }}%</template>
      </UFormField>
      <UFormField :label="t('snapEditor.width')">
        <USlider :model-value="target.w_ratio" :min="0.05" :max="1" :step="0.01" class="w-full"
          @update:model-value="updateArea('w_ratio', $event)" />
        <template #hint>{{ (target.w_ratio * 100).toFixed(0) }}%</template>
      </UFormField>
      <UFormField :label="t('snapEditor.height')">
        <USlider :model-value="target.h_ratio" :min="0.05" :max="1" :step="0.01" class="w-full"
          @update:model-value="updateArea('h_ratio', $event)" />
        <template #hint>{{ (target.h_ratio * 100).toFixed(0) }}%</template>
      </UFormField>
    </template>

    <UFormField v-else :label="t('snapEditor.action')" class="sm:col-span-2">
      <USelect
        :model-value="target.action"
        :items="actionOptions"
        value-key="value"
        class="w-full"
        @update:model-value="emit('update', { action: $event as WindowAction } as Partial<SnapTarget>)"
      />
    </UFormField>
  </div>
</template>
```

- [ ] **Step 3: `SnapEditor.vue` 재작성 (UTable expandable)**

`src/pages/SnapEditor.vue` 전체 교체:

```vue
<script setup lang="ts">
import { h, onMounted, ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ColumnDef } from '@tanstack/vue-table'
import { useConfigStore } from '@/features/config-store'
import SaveActions from '@/components/SaveActions.vue'
import SnapCanvas from '@/components/SnapCanvas.vue'
import SnapProperties from '@/components/SnapProperties.vue'
import type { SnapTarget, SnapTargetArea } from '@/entities/config'

const { t } = useI18n()
const store = useConfigStore()

onMounted(() => store.load())

const expanded = ref<string[]>([])

function selectTarget(id: string): string {
  return id
}

function updateTarget(id: string, patch: Partial<SnapTarget>) {
  if (!store.draft) return
  const idx = store.draft.snap.areas.findIndex((a) => a.id === id)
  if (idx >= 0) {
    store.draft.snap.areas[idx] = { ...store.draft.snap.areas[idx], ...patch } as SnapTarget
  }
}

function deleteTarget(id: string) {
  if (!store.draft) return
  store.draft.snap.areas = store.draft.snap.areas.filter((a) => a.id !== id)
  expanded.value = expanded.value.filter((e) => e !== id)
}

function addTarget(kind: 'area' | 'action') {
  if (!store.draft) return
  const id = kind === 'area' ? `area-${Date.now()}` : `action-${Date.now()}`
  const name = kind === 'area' ? t('snapEditor.newArea') : t('snapEditor.newAction')
  const target: SnapTarget = kind === 'area'
    ? { kind: 'area', id, name, x_ratio: 0.1, y_ratio: 0.1, w_ratio: 0.3, h_ratio: 0.3 }
    : { kind: 'action', id, name, action: 'Maximize' }
  store.draft.snap.areas.push(target)
  expanded.value = [...expanded.value, id]
}

const columns = computed<ColumnDef<SnapTarget>[]>(() => [
  {
    id: 'expand',
    header: '',
    cell: ({ row }) =>
      h(
        UButton,
        {
          icon: row.getIsExpanded() ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right',
          color: 'neutral',
          variant: 'ghost',
          size: 'xs',
          onClick: () => row.toggleExpanded(),
        },
      ),
  },
  {
    accessorKey: 'name',
    header: () => t('snapEditor.name'),
  },
  {
    id: 'kind',
    header: () => t('snapEditor.type'),
    cell: ({ row }) =>
      h(UBadge, {
        label: row.original.kind === 'area' ? t('snapEditor.area') : t('snapEditor.action'),
        color: row.original.kind === 'area' ? 'primary' : 'info',
        variant: 'soft',
        size: 'sm',
      }),
  },
  {
    id: 'actions',
    header: '',
    cell: ({ row }) =>
      h(UDropdownMenu, {
        items: [
          {
            label: t('common.delete'),
            icon: 'i-lucide-trash-2',
            onSelect: () => deleteTarget(row.original.id),
          },
        ],
      }, () => h(UButton, { icon: 'i-lucide-more-horizontal', color: 'neutral', variant: 'ghost', size: 'xs' })),
  },
])

function expandedContent(target: SnapTarget) {
  const areaTarget = target.kind === 'area' ? (target as SnapTargetArea) : null
  return h('div', { class: 'grid gap-6 px-4 py-4 lg:grid-cols-[1fr_320px]' }, [
    h(
      'div',
      { class: 'space-y-4' },
      h(SnapProperties, {
        target,
        onUpdate: (patch: Partial<SnapTarget>) => updateTarget(target.id, patch),
      }),
    ),
    areaTarget
      ? h(SnapCanvas, {
          area: areaTarget,
          onUpdate: (id: string, patch: Partial<SnapTargetArea>) => updateTarget(id, patch),
        })
      : null,
  ])
}
</script>

<template>
  <UDashboardPanel>
    <template #header>
      <UDashboardNavbar :title="t('snapEditor.title')">
        <template #right>
          <div class="flex items-center gap-2">
            <UDropdownMenu
              :items="[
                { label: t('snapEditor.area'), icon: 'i-lucide-square', onSelect: () => addTarget('area') },
                { label: t('snapEditor.action'), icon: 'i-lucide-zap', onSelect: () => addTarget('action') },
              ]"
            >
              <UButton icon="i-lucide-plus" color="primary" variant="soft" size="sm" :label="t('snapEditor.addTarget')" />
            </UDropdownMenu>
            <SaveActions
              v-if="store.draft"
              :dirty="store.isDirty"
              :saving="store.saving"
              @save="store.save()"
              @reset="store.reset()"
            />
          </div>
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <UContainer class="py-8">
        <div v-if="store.loading" class="py-8 text-center text-muted">
          <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin" />
        </div>

        <template v-else-if="store.draft">
          <UCard variant="subtle">
            <UTable
              :data="store.draft.snap.areas"
              :columns="columns"
              v-model:expanded="expanded"
              :row-key="(row) => row.id"
              class="w-full"
            >
              <template #expanded="{ row }">
                {{ expandedContent(row.original) }}
              </template>
            </UTable>
          </UCard>
        </template>
      </UContainer>
    </template>
  </UDashboardPanel>
</template>
```

> 참고: `#expanded` 슬롯에서 렌더 함수(`h`)를 반환하는 패턴은 Nuxt UI Table의 expandable 예시와 일치. 슬롯에서 컴포넌트를 직접 템플릿으로 쓰기 어려워 `expandedContent` 헬퍼 사용. 만약 `#expanded`가 vnode를 요구하지 않고 템플릿을 허용하면 그쪽이 더 깔끔 — 구현 시 동작 확인 후 템플릿 방식으로 단순화.

- [ ] **Step 4: i18n 키 정리 (snapEditor 섹션)**

ko.ts snapEditor 부분 (preset/import/export/tabs 제거, 새 키 추가):
```ts
  snapEditor: {
    title: '스냅 에디터',
    description: '스냅 영역 편집',
    targets: '대상',
    addTarget: '대상 추가',
    properties: '속성',
    type: '유형',
    area: '영역',
    action: '동작',
    name: '이름',
    width: '너비',
    height: '높이',
    newArea: '새 영역',
    newAction: '새 동작',
  },
```

en.ts snapEditor 부분:
```ts
  snapEditor: {
    title: 'Snap Editor',
    description: 'Edit snap areas',
    targets: 'Targets',
    addTarget: 'Add target',
    properties: 'Properties',
    type: 'Type',
    area: 'Area',
    action: 'Action',
    name: 'Name',
    width: 'Width',
    height: 'Height',
    newArea: 'New Area',
    newAction: 'New Action',
  },
```

`preset*`, `import`, `export`, `tabs.*` 키 제거.

- [ ] **Step 5: 수동 검증 + 커밋**

```bash
pnpm tauri:dev
```

테이블 행의 chevron 클릭 → 확장 → 속성 폼 + 축소 캔버스 표시. 드래그로 영역 편집. 추가/삭제 동작. 프리셋/Import/Export/Mapping 탭 없는지 확인.

```bash
git add -A
git commit -m "feat: SnapEditor 재작성 — UTable expandable rows

- UTable + expandable rows로 단순화 (3패널 폐기)
- 확장 시 SnapProperties 폼 + 축소 SnapCanvas
- 프리셋 셀렉트/Import/Export/Sector Mapping 탭 제거
- active_preset은 'full' 고정 (UI 숨김)
- SnapCanvas 축소형 (320px), SnapProperties 폼 단순화"
```

---

## Task 10: About 페이지 마이그레이션 + 전체 빌드/검증

**Files:**
- Modify: `src/pages/About.vue`

**Interfaces:**
- Consumes: `SaveActions`, `UDashboardPanel` 패턴.

- [ ] **Step 1: `About.vue` 마이그레이션**

`src/pages/About.vue` 전체 교체:

```vue
<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/features/config-store'
import SaveActions from '@/components/SaveActions.vue'

const { t } = useI18n()
const store = useConfigStore()

const appVersion = ref('0.1.0')
const checking = ref(false)
const updateStatus = ref<'idle' | 'available' | 'up-to-date'>('idle')

onMounted(() => store.load())

const channelItems = [
  { label: t('about.channelStable'), value: 'stable' },
  { label: t('about.channelBeta'), value: 'beta' },
]

async function checkForUpdates() {
  checking.value = true
  updateStatus.value = 'idle'
  try {
    await new Promise((resolve) => setTimeout(resolve, 1500))
    updateStatus.value = 'up-to-date'
  } finally {
    checking.value = false
  }
}
</script>

<template>
  <UDashboardPanel>
    <template #header>
      <UDashboardNavbar :title="t('about.title')">
        <template #right>
          <SaveActions
            v-if="store.draft"
            :dirty="store.isDirty"
            :saving="store.saving"
            @save="store.save()"
            @reset="store.reset()"
          />
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <UContainer class="max-w-3xl space-y-10 py-8">
        <div v-if="store.loading" class="py-8 text-center text-muted">
          <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin" />
        </div>

        <template v-else-if="store.draft">
          <!-- 앱 정보 -->
          <section class="space-y-4">
            <UCard variant="subtle">
              <div class="flex items-center gap-4">
                <div class="flex size-12 items-center justify-center rounded-lg bg-primary/10">
                  <UIcon name="i-lucide-square" class="size-6 text-primary" />
                </div>
                <div>
                  <p class="font-semibold">{{ t('app.name') }}</p>
                  <p class="text-sm text-muted">{{ t('about.version') }} {{ appVersion }}</p>
                </div>
              </div>
              <UButton
                :label="t('about.github')"
                icon="i-lucide-github"
                color="neutral"
                variant="outline"
                to="https://github.com/troublecoder/rectangle-win"
                target="_blank"
                class="mt-4"
              />
            </UCard>
          </section>

          <!-- 업데이트 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-refresh-cw" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('about.update') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('about.autoUpdate')" :description="t('about.autoUpdateDesc')">
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted" />
                <USwitch v-model="store.draft.update.enabled" />
              </div>
            </UFormField>
            <template v-if="store.draft.update.enabled">
              <UFormField :label="t('about.updateChannel')">
                <USelect
                  v-model="store.draft.update.channel"
                  :items="channelItems"
                  value-key="value"
                  class="w-full"
                />
              </UFormField>
              <div class="flex items-center gap-3">
                <UButton
                  :label="checking ? t('about.checking') : t('about.checkForUpdates')"
                  icon="i-lucide-refresh-cw"
                  :loading="checking"
                  color="primary"
                  variant="soft"
                  @click="checkForUpdates"
                />
                <UBadge
                  v-if="updateStatus === 'up-to-date'"
                  color="success"
                  variant="soft"
                  :label="t('about.upToDate')"
                  icon="i-lucide-check"
                />
                <UBadge
                  v-else-if="updateStatus === 'available'"
                  color="warning"
                  variant="soft"
                  :label="t('about.updateAvailable')"
                  icon="i-lucide-arrow-up-circle"
                />
              </div>
            </template>
          </section>
        </template>
      </UContainer>
    </template>
  </UDashboardPanel>
</template>
```

- [ ] **Step 2: 전체 빌드 (타입 체크)**

```bash
pnpm build
```

Expected: `vue-tsc --noEmit` 통과, `vite build` 성공. 모든 구 필드 참조 에러 해결되어야 함.

- [ ] **Step 3: cargo test 재확인**

```bash
cd src-tauri && cargo test
```

Expected: 전체 통과.

- [ ] **Step 4: 수동 전체 검증**

```bash
pnpm tauri:dev
```

체크리스트:
- 사이드바 5개 항목 (General/Throw/SnapEditor/Display/About), Keyboard 없음
- 사이드바 footer: 스냅 활성화 토글 + 종료
- 각 페이지 헤더 타이틀 단일 (UDashboardNavbar), 내부 중복 없음
- 언어 전환 즉시 반영, 사이드바에 언어 셀렉트 없음
- Throw 핫키 캡처 동작
- SnapEditor expandable rows 동작, 드래그 편집
- Display 색상 lock/unlock 동작
- 다크/라이트 전환 정상 (기본 테마)

- [ ] **Step 5: 최종 커밋**

```bash
git add -A
git commit -m "feat: About 페이지 UDashboardPanel 마이그레이션 — 재설계 완료

- About 페이지 UDashboardPanel + flat 섹션 구조
- 전체 설정 화면 재설계 완료
- 빌드/테스트 통과"
```

---

## Self-Review 결과

**1. Spec coverage:**
- ✅ 전체 구조 (UDashboardGroup/Sidebar/Panel) — Task 4
- ✅ 페이지 6→5 축소 (Keyboard 제거) — Task 4
- ✅ 언어 단일 진실 — Task 5
- ✅ 핫키 캡처 — Task 6
- ✅ SnapEditor expandable rows — Task 8
- ✅ Snap Preview 색상 객체 + lock — Task 7
- ✅ 백엔드 preview_colors + 미사용 필드 제거 — Task 2
- ✅ Catppuccin 제거 — Task 1
- ✅ 사이드바 footer 활성화 토글 — Task 4
- ✅ 매핑 Throw 이동 — Task 6
- ✅ flat 섹션 (USection/UPageCard 제거) — Task 4+

**2. Placeholder scan:** TODO/TBD 없음. 모든 단계에 실제 코드 포함.

**3. Type consistency:**
- `PreviewColors` 타입: `config.ts`(Zod) ↔ `default-config.ts` ↔ Rust `PreviewColors` 일치.
- `OverlayController` 시그니처: `show_reticle(x, y)` / `show_snap_preview(x, y, w, h, is_long_throw)` — ports.rs, win32_overlay.rs, snap_service.rs, keyboard_service.rs 모두 Task 2에서 일괄 업데이트.
- `UHotkeyInput` emit: `string[]` — `trigger_modifiers`와 호환 (백엔드 토큰 `"Win"`/`"Alt"` 등 대소문자 일치).
- `ColorLockField` v-model: `PreviewColors` 객체.

**잠재 이슈 (구현 시 주의):**
1. `config-store.ts`에서 `i18n.global.locale.value` 접근 — vue-i18n composition API의 `locale` ref를 global에서 접근하는 패턴. `i18n.global.locale`이 ref인지 string인지 vue-i18n 버전에 따라 다름 (`legacy: false`이므로 ref). 구현 시 확인.
2. UTable `#expanded` 슬롯이 vnode vs 템플릿 — Nuxt UI 버전에 따라 다를 수 있음. Task 8 Step 3에서 구현 시 테스트 후 단순화.
3. vue-konva 컬러에 CSS variable(`rgb(var(--ui-color-primary-500))`) 사용 — konva가 이를 파싱 못할 수 있음. hex 문자열로 폴백 필요 시 Task 8에서 조정.

이슈는 구현 단계에서 발견되면 그 태스크 내에서 해결.
