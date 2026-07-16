# 입력/오버레이 전면 교체 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** LL hook 기반 입력 + Tauri 웹뷰 오버레이를 RegisterHotKey + GetAsyncKeyState 폴링 + DirectComposition/Direct2D 오버레이로 전면 교체하여 PowerToys 충돌/깜빡임/포커스 문제를 근본 해결한다.

**Architecture:** 헥사고날 구조 유지 — 도메인/애플리케이션 계층(SnapService/KeyboardService/도메인)은 변경 없이 인프라 어댑터만 교체. 입력은 RegisterHotKey(키보드) + GetAsyncKeyState 폴링(throw modifier), 오버레이는 DirectComposition + DXGI + Direct2D로 GPU 직접 합성.

**Tech Stack:** Rust, windows-rs 0.58 (DirectComposition/Direct2D/D3D11/DXGI), Tauri v2, 기존 도메인 로직.

## Global Constraints

- **플랫폼:** Windows 전용 (cfg(windows)). 비Windows는 기존 TauriOverlay 유지(컴파일만).
- **windows crate:** 0.58. Features에 `Win32_Graphics_Direct2D`, `Win32_Graphics_DirectComposition`, `Win32_Graphics_Direct3D11`, `Win32_Graphics_Dxgi`, `Win32_Graphics_Dxgi_Common` 추가.
- **OverrideOs 모드:** 포기. ModifierMode enum에서 타입은 유지하되 기본값 Separate.
- **LL hook:** 완전 제거. `win32_input_hook.rs` 삭제.
- **Tauri 오버레이 창:** 제거. tauri.conf.json의 overlay 창, overlay.html, src/overlay.ts 삭제.
- **기존 순수 로직 테스트:** 88개 기존 테스트 통과 유지 필수.
- **언어:** 코드 주석/커밋 메시지 한국어 허용(기존 코드베이스 패턴 준수).

## File Structure

**삭제:**
- `src-tauri/src/infrastructure/win32_input_hook.rs` — LL hook (대체됨)
- `overlay.html` — Tauri 오버레이 HTML 진입점
- `src/overlay.ts` — Tauri 오버레이 TS 진입점

**신규 생성:**
- `src-tauri/src/infrastructure/win32_overlay.rs` — DirectComposition/Direct2D 오버레이 (OverlayController 구현)
- `src-tauri/src/infrastructure/win32_input.rs` — RegisterHotKey + 폴링 입력 리스너

**수정:**
- `src-tauri/Cargo.toml` — windows features 추가
- `src-tauri/src/infrastructure/mod.rs` — 모듈 등록 변경 (win32_input_hook 제거, win32_overlay/win32_input 추가)
- `src-tauri/src/infrastructure/win32_monitor.rs` — MonitorProvider에 동적 캐시 무효화 추가
- `src-tauri/src/presentation/state.rs` — overlay 필드 타입 변경
- `src-tauri/src/lib.rs` — emitter/오버레이 창 제어 제거, Win32InputListener 시작
- `src-tauri/tauri.conf.json` — overlay 창 제거
- `src-tauri/capabilities/default.json` — windows를 ["main"]로

---

### Task 1: 기존 LL hook + Tauri 오버레이 제거, 설정 창 기준선 확보

**Files:**
- Delete: `src-tauri/src/infrastructure/win32_input_hook.rs`
- Delete: `overlay.html`
- Delete: `src/overlay.ts`
- Modify: `src-tauri/src/infrastructure/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src-tauri/src/presentation/state.rs`

**Interfaces:**
- Consumes: 기존 TauriOverlay, OverlayController trait
- Produces: 깨끗한 기준선 (설정 창만 작동, 입력/오버레이 없음). 이후 태스크들이 이 위에 구축.

**목표:** LL hook과 Tauri 웹뷰 오버레이를 완전히 제거하고, 설정 창만 정상 작동하는 기준선을 만든다. SnapService/KeyboardService는 AppState에 그대로 유지(입력 리스너만 없는 상태).

- [ ] **Step 1: win32_input_hook.rs 삭제**

```bash
rm src-tauri/src/infrastructure/win32_input_hook.rs
```

- [ ] **Step 2: overlay.html, src/overlay.ts 삭제**

```bash
rm overlay.html src/overlay.ts
```

- [ ] **Step 3: infrastructure/mod.rs에서 win32_input_hook 제거 + win32_input/win32_overlay 추가 (빈 파일로)**

`src-tauri/src/infrastructure/mod.rs` 수정 — `#[cfg(windows)] pub mod win32_input_hook;` 줄을 제거하고, 아래 두 줄로 교체:

```rust
#[cfg(windows)]
pub mod win32_overlay;

#[cfg(windows)]
pub mod win32_input;
```

빈 파일 생성 (컴파일 통과용):

```bash
# src-tauri/src/infrastructure/win32_overlay.rs
echo '#![cfg(windows)]' > src-tauri/src/infrastructure/win32_overlay.rs
# src-tauri/src/infrastructure/win32_input.rs
echo '#![cfg(windows)]' > src-tauri/src/infrastructure/win32_input.rs
```

- [ ] **Step 4: tauri.conf.json에서 overlay 창 제거**

`src-tauri/tauri.conf.json`의 `app.windows` 배열에서 `"label": "overlay"` 창 객체 전체 제거. main 창만 남김:

```json
"windows": [
  {
    "title": "Rectangle Win",
    "label": "main",
    "width": 900,
    "height": 640,
    "minWidth": 720,
    "minHeight": 500,
    "resizable": true,
    "visible": false
  }
],
```

- [ ] **Step 5: capabilities/default.json을 ["main"]로**

```json
"windows": ["main"],
```

- [ ] **Step 6: lib.rs에서 emitter/NOACTIVATE/InputHookController 코드 제거**

`src-tauri/src/lib.rs`의 `.setup()` 클로저에서:
- `state.overlay.set_emitter(...)` 전체 블록 제거 (lines ~31-70)
- `#[cfg(windows)] { ... NOACTIVATE ... InputHookController::start(...) }` 전체 블록 제거 (lines ~84-112)

setup 클로저는 tray 설정 + 메인 창 close 가로채기 + debug show만 남김.

- [ ] **Step 7: state.rs — overlay 필드를 Arc<dyn OverlayController>로 변경**

`src-tauri/src/presentation/state.rs`:
- `use crate::application::ports::OverlayController;` 추가 (이미 ports.rs에서 임포트 중이면 확인)
- `pub overlay: Arc<TauriOverlay>` → `pub overlay: Arc<dyn OverlayController>`
- `AppState::new()`에서 Windows일 때: 임시로 `Arc::new(TauriOverlay::new())` 유지 (Task 2에서 Win32LayeredOverlay로 교체). TauriOverlay의 set_emitter 없이는 아무 동작 안 하지만 snap은 작동.

- [ ] **Step 8: 컴파일 확인**

Run: `cd src-tauri && cargo check`
Expected: 컴파일 성공 (warning 가능, error 없음)

- [ ] **Step 9: 테스트 확인**

Run: `cd src-tauri && cargo test`
Expected: 88 passed, 0 failed (또는 기존 1개 roundtrip 실패 유지 — 별개 이슈)

- [ ] **Step 10: Commit**

```bash
git add -A && git commit -m "refactor: LL hook + Tauri 웹뷰 오버레이 제거, 기준선 확보

win32_input_hook.rs, overlay.html, src/overlay.ts 삭제.
overlay 필드를 Arc<dyn OverlayController>로 변경.
설정 창만 작동하는 기준선 — 이후 태스크에서 새 입력/오버레이 어댑터 추가."
```

---

### Task 2: Win32 Layered 오버레이 — DirectComposition/Direct2D 구현

**Files:**
- Create: `src-tauri/src/infrastructure/win32_overlay.rs` (내용 채우기)
- Modify: `src-tauri/Cargo.toml` (windows features 추가)

**Interfaces:**
- Consumes: `OverlayController` trait (from `application/ports.rs`), `AppResult` (from `application/errors.rs`)
- Produces: `Win32LayeredOverlay` struct — `OverlayController` 구현체. `pub fn new() -> Self` 생성자.

**목표:** DirectComposition + DXGI swap chain + Direct2D로 투명 클릭스루 always-on-top 오버레이 창을 만들고, OverlayController trait을 구현하여 섹터 부채꼴/중심점/snap 미리보기 사각형을 그린다.

**참고 — windows-rs 0.58 DirectComposition 파이프라인:**
1. D3D11 device 생성 (`D3D11CreateDevice`)
2. DXGI factory → swap chain 생성 (`IDXGIFactory2::CreateSwapChainForComposition`, `DXGI_ALPHA_MODE_PREMULTIPLIED`)
3. Direct2D factory + D3D11 백버퍼에서 D2D device/context/bitmap 렌더타겟
4. `DCompositionCreateDevice` → `IDCompositionDevice` → `CreateTargetForHwnd(hwnd, true)` → `IDCompositionTarget`
5. `IDCompositionDevice::CreateVisual` → `IDCompositionVisual`
6. `visual.SetContent(swap_chain)` → `target.SetRoot(visual)` → `device.Commit()`

- [ ] **Step 1: Cargo.toml에 windows features 추가**

`src-tauri/Cargo.toml`의 windows crate features에 추가:

```toml
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_System_Threading",
    "Win32_Graphics_Gdi",
    "Win32_Graphics_Direct2D",
    "Win32_Graphics_Direct2D_Common",
    "Win32_Graphics_Direct3D",
    "Win32_Graphics_Direct3D11",
    "Win32_Graphics_Dxgi",
    "Win32_Graphics_Dxgi_Common",
    "Win32_Graphics_DirectComposition",
    "Win32_Graphics_Dwm",
] }
```

- [ ] **Step 2: win32_overlay.rs 뼈대 + 창 생성 작성**

`src-tauri/src/infrastructure/win32_overlay.rs`에 작성. 핵심 구조:

```rust
#![cfg(windows)]

//! DirectComposition + Direct2D 기반 오버레이 창.
//!
//! WS_EX_NOREDIRECTIONBITMAP | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE
//! 창을 만들고, D3D11/DXGI/Direct2D/DirectComposition 파이프라인으로 GPU 직접 합성.
//! OverlayController trait 구현 — 섹터 부채꼴, 중심점, snap 미리보기 사각형 그리기.

use std::sync::Mutex;

use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::DirectComposition::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Gdi::{GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::application::errors::AppResult;
use crate::application::ports::OverlayController;

/// 오버레이 렌더링 상태 — 그릴 내용을 보관.
#[derive(Default)]
struct OverlayDrawState {
    visible: bool,
    center: Option<(i32, i32)>,
    sector_count: u8,
    active_sector: Option<u8>,
    snap_preview: Option<(i32, i32, i32, i32)>,
    cursor: Option<(i32, i32)>,
}

/// DirectComposition/Direct2D 기반 오버레이.
///
/// 앱 시작 시 창을 한 번 생성하고, OverlayController 메서드 호출 시마다
/// 상태를 갱신하고 D2D로 다시 그린다. show/hide는 visible 플래그로만 제어
/// (창 자체를 show/hide 반복하지 않음 → 깜빡임 없음).
pub struct Win32LayeredOverlay {
    state: Mutex<OverlayDrawState>,
    // D3D11/DXGI/D2D/DComp 리소스 — 초기화 후 불변.
    // 초기화 실패 시 None (graceful degradation: snap만 작동, 오버레이 없음).
    resources: Mutex<Option<OverlayResources>>,
}

/// GPU 렌더링 리소스 묶음.
struct OverlayResources {
    hwnd: HWND,
    _d3d_device: ID3D11Device,
    dxgi_factory: IDXGIFactory2,
    swap_chain: IDXGISwapChain1,
    d2d_factory: ID2D1Factory1,
    d2d_device: ID2D1Device,
    d2d_context: ID2D1DeviceContext,
    dcomp_device: IDCompositionDevice,
    dcomp_target: IDCompositionTarget,
    dcomp_visual: IDCompositionVisual,
    width: i32,
    height: i32,
}
```

- [ ] **Step 3: new() — 리소스 초기화 + 창 생성**

```rust
impl Win32LayeredOverlay {
    pub fn new() -> Self {
        let resources = Self::init_resources().ok();
        Self {
            state: Mutex::new(OverlayDrawState::default()),
            resources: Mutex::new(resources),
        }
    }

    fn init_resources() -> windows::core::Result<OverlayResources> {
        // 1. 가상 데스크톱 전체 크기.
        let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

        // 2. D3D11 device 생성.
        let mut d3d_device: Option<ID3D11Device> = None;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                Default::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                &[],
                D3D11_SDK_VERSION as u32,
                Some(&mut d3d_device),
                None,
                None,
            )?;
        }
        let d3d_device = d3d_device.unwrap();

        // 3. DXGI factory + swap chain (composition용, premultiplied alpha).
        let dxgi_device: IDXGIDevice = d3d_device.cast()?;
        let dxgi_factory: IDXGIFactory2 = dxgi_device.GetParent()?;

        let swap_desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: width as u32,
            Height: height as u32,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            Flags: 0,
            AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
            ..Default::default()
        };
        let swap_chain = unsafe { dxgi_factory.CreateSwapChainForComposition(&dxgi_device, &swap_desc, None)? };

        // 4. 오버레이 창 생성 (layered-transparent-topmost-noactivate).
        let hwnd = Self::create_overlay_window(x, y, width, height)?;

        // 5. Direct2D factory + device (D3D11에서).
        let mut d2d_factory: Option<ID2D1Factory1> = None;
        let d2d_options = D2D1_FACTORY_OPTIONS { debugLevel: D2D1_DEBUG_LEVEL_NONE };
        unsafe {
            D2D1CreateFactory(
                D2D1_FACTORY_TYPE_SINGLE_THREADED,
                &d2d_options,
                &mut d2d_factory,
            )?;
        }
        let d2d_factory = d2d_factory.unwrap();
        let d2d_device = unsafe { d2d_factory.CreateDevice(&dxgi_device)? };
        let d2d_context = unsafe { d2d_device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)? };

        // 6. DirectComposition device + target + visual.
        let dcomp_device: IDCompositionDevice = unsafe { DCompositionCreateDevice(&dxgi_device, &IDCompositionDevice::IID)? };
        let dcomp_target = unsafe { dcomp_device.CreateTargetForHwnd(hwnd, true)? };
        let dcomp_visual = unsafe { dcomp_device.CreateVisual()? };
        unsafe {
            dcomp_visual.SetContent(&swap_chain)?;
            dcomp_target.SetRoot(&dcomp_visual)?;
            dcomp_device.Commit()?;
        }

        Ok(OverlayResources {
            hwnd,
            _d3d_device: d3d_device,
            dxgi_factory,
            swap_chain,
            d2d_factory,
            d2d_device,
            d2d_context,
            dcomp_device,
            dcomp_target,
            dcomp_visual,
            width,
            height,
        })
    }

    fn create_overlay_window(x: i32, y: i32, width: i32, height: i32) -> windows::core::Result<HWND> {
        // ... CreateWindowExW with WS_EX_NOREDIRECTIONBITMAP | WS_EX_TRANSPARENT |
        //     WS_EX_TOPMOST | WS_EX_NOACTIMATE. WS_POPUP. 전체 가상 데스크톱 위치/크기.
        // 창 생성 후 ShowWindow(SW_SHOWNOACTIVATE)로 표시 (한 번만, 계속 떠 있음).
        // 실제 구현 — windows-rs CreateWindowExW + RegisterClassExW 패턴 (win32_window.rs 참고).
        // ... (구현 — 아래 "참고" 섹션)
        unimplemented!("Task 2 Step 3에서 구현")
    }
}
```

**참고 — CreateWindowExW 패턴:** `win32_window.rs`의 HWND 변환 패턴(`hwnd.0 as usize as u64`)을 따르되, 오버레이 창은 message-only가 아님 (DirectComposition target이므로). `WS_EX_NOREDIRECTIONBITMAP`은 DWM이 리다이렉션 비트맵을 할당하지 않게 함. 창 클래스 등록 + CreateWindowExW + ShowWindow(SW_SHOWNOACTIVATE).

- [ ] **Step 4: redraw() — D2D 백버퍼 획득 + 그리기**

```rust
impl Win32LayeredOverlay {
    /// 현재 상태로 전체 재그리기.
    fn redraw(&self) {
        let mut res_guard = self.resources.lock().unwrap();
        let Some(res) = res_guard.as_ref() else { return }; // 초기화 실패 — no-op
        let state = self.state.lock().unwrap();
        if !state.visible {
            // 숨김 — 투명하게 클리어.
            let _ = Self::clear_buffer(res);
            return;
        }
        let _ = Self::draw_scene(res, &state);
    }

    fn clear_buffer(res: &OverlayResources) -> windows::core::Result<()> {
        // swap chain 백버퍼 → ID2D1Bitmap1 렌더타겟 바인딩 → BeginDraw → Clear(투명) → EndDraw.
        unsafe {
            let back_buffer: IDXGISurface = res.swap_chain.GetBuffer(0)?;
            let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
                dpiX: 96.0, dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                colorContext: None,
            };
            let bitmap: ID2D1Bitmap1 = res.d2d_context.CreateBitmapFromDxgiSurface(&back_buffer, Some(&bitmap_props))?;
            res.d2d_context.SetTarget(&bitmap);
            res.d2d_context.BeginDraw();
            res.d2d_context.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));
            res.d2d_context.EndDraw(None, None)?;
            res.swap_chain.Present(1, DXGI_PRESENT_DO_NOT_SEQUENCE.0 as u32).ok();
        }
        Ok(())
    }

    fn draw_scene(res: &OverlayResources, state: &OverlayDrawState) -> windows::core::Result<()> {
        // 백버퍼 바인딩 (clear_buffer와 동일) 후 BeginDraw.
        // Clear(투명) → 섹터 부채꼴(Pie) → 중심점 원 → snap 미리보기 점선 사각형.
        // 부채꼴: D2D path geometry + arc segment. 활성 섹터는 채우기 색 다르게.
        // EndDraw → Present.
        // 실제 D2D 그리기 코드 — geometry 생성, brush 생성(단색 CreateSolidColorBrush),
        // FillGeometry/DrawGeometry. 안티앨리어싱 기본 적용.
        unimplemented!("Task 2 Step 4에서 구현 — D2D 그리기 상세")
    }
}
```

- [ ] **Step 5: OverlayController trait 구현**

```rust
impl OverlayController for Win32LayeredOverlay {
    fn show_reticle(&self, center_x: i32, center_y: i32, sector_count: u8) -> AppResult<()> {
        let mut state = self.state.lock().unwrap();
        state.visible = true;
        state.center = Some((center_x, center_y));
        state.sector_count = sector_count;
        state.active_sector = None;
        state.snap_preview = None;
        drop(state);
        self.redraw();
        Ok(())
    }

    fn update_cursor_indicator(&self, x: i32, y: i32) -> AppResult<()> {
        let mut state = self.state.lock().unwrap();
        state.cursor = Some((x, y));
        drop(state);
        self.redraw();
        Ok(())
    }

    fn highlight_sector(&self, sector: u8) -> AppResult<()> {
        let mut state = self.state.lock().unwrap();
        state.active_sector = Some(sector);
        drop(state);
        self.redraw();
        Ok(())
    }

    fn show_snap_preview(&self, x: i32, y: i32, width: i32, height: i32) -> AppResult<()> {
        let mut state = self.state.lock().unwrap();
        state.snap_preview = Some((x, y, width, height));
        drop(state);
        self.redraw();
        Ok(())
    }

    fn hide(&self) -> AppResult<()> {
        let mut state = self.state.lock().unwrap();
        state.visible = false;
        drop(state);
        self.redraw();
        Ok(())
    }
}

impl Default for Win32LayeredOverlay {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 6: 컴파일 확인**

Run: `cd src-tauri && cargo check`
Expected: 컴파일 성공 (D2D/DComp API 모두 windows-rs에서 제공). warning 가능.

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat: DirectComposition/Direct2D 오버레이 어댑터

WS_EX_NOREDIRECTIONBITMAP 투명 클릭스루 창 + D3D11/DXGI/Direct2D/DirectComposition
GPU 합성 파이프라인. OverlayController 구현 (섹터/중심점/snap 미리보기).
깜빡임 없음 — 창은 한 번 생성 후 visible 플래그로만 제어."
```

---

### Task 3: Win32 입력 리스너 — RegisterHotKey + GetAsyncKeyState 폴링

**Files:**
- Create: `src-tauri/src/infrastructure/win32_input.rs` (내용 채우기)

**Interfaces:**
- Consumes: `Arc<SnapService>`, `Arc<KeyboardService>`, `Arc<dyn ConfigStore>`, `Arc<dyn MonitorProvider>` (from state.rs)
- Consumes: `Direction` (from domain/model.rs), config 구조체 (`KeyboardConfig.trigger_modifiers`, `ThrowConfig.trigger_modifiers`)
- Produces: `Win32InputListener` struct, `pub fn start(snap_service, keyboard_service, config_store) -> Self` — 전용 스레드 시작.

**목표:** message-only 창에서 RegisterHotKey(키보드 snap) + GetAsyncKeyState 폴링(마우스 throw)으로 SnapService/KeyboardService를 구동.

- [ ] **Step 1: Win32InputListener 뼈대**

`src-tauri/src/infrastructure/win32_input.rs`:

```rust
#![cfg(windows)]

//! RegisterHotKey + GetAsyncKeyState 폴링 기반 입력 리스너.
//!
//! LL hook 없이 전역 핫키(키보드 snap)와 modifier 조합 감지(마우스 throw) 처리.
//! message-only 창에서 GetMessage 루프 + MsgWaitForMultipleObjects 타임아웃 폴링.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::application::keyboard_service::KeyboardService;
use crate::application::ports::ConfigStore;
use crate::application::snap_service::SnapService;
use crate::domain::model::Direction;

/// 핫키 ID (방향키 매핑).
const HOTKEY_LEFT: i32 = 1;
const HOTKEY_RIGHT: i32 = 2;
const HOTKEY_UP: i32 = 3;
const HOTKEY_DOWN: i32 = 4;

/// 폴링 주기 (마우스 throw modifier 감지).
const POLL_INTERVAL: Duration = Duration::from_millis(16);

/// 입력 리스너. start()로 전용 스레드를 시작한다.
pub struct Win32InputListener;

/// 스레드 간 공유 상태.
struct InputState {
    /// throw 활성 시 origin 커서 좌표.
    origin: Option<(i32, i32)>,
    /// throw modifier 조합이 현재 눌려 있는지.
    throw_active: bool,
}

impl Win32InputListener {
    /// 입력 리스너 스레드 시작.
    pub fn start(
        snap_service: Arc<SnapService>,
        keyboard_service: Arc<KeyboardService>,
        config_store: Arc<dyn ConfigStore>,
    ) {
        thread::Builder::new()
            .name("win32-input".into())
            .spawn(move || {
                let state = Arc::new(Mutex::new(InputState {
                    origin: None,
                    throw_active: false,
                }));
                if let Err(e) = run_message_loop(snap_service, keyboard_service, config_store, state) {
                    eprintln!("입력 리스너 오류: {e}");
                }
            })
            .expect("입력 리스너 스레드 시작 실패");
    }
}
```

- [ ] **Step 2: message-only 창 생성 + RegisterHotKey**

```rust
/// message-only 창에서 GetMessage 루프 + 폴링.
fn run_message_loop(
    snap_service: Arc<SnapService>,
    keyboard_service: Arc<KeyboardService>,
    config_store: Arc<dyn ConfigStore>,
    state: Arc<Mutex<InputState>>,
) -> windows::core::Result<()> {
    // message-only 창 생성.
    let hwnd = create_message_window()?;

    // RegisterHotKey — config에서 keyboard trigger_modifiers + 방향키.
    register_hotkeys(&hwnd, &config_store);

    // 메시지 + 폴링 루프.
    let mut msg = MSG::default();
    loop {
        // MsgWaitForMultipleObjects로 타임아웃 대기 — 메시지 오면 즉시, 아니면 폴링 주기.
        let result = unsafe {
            MsgWaitForMultipleObjects(
                None,
                false,
                POLL_INTERVAL,
                QS_ALLINPUT,
            )
        };
        // 메시지 처리.
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.into_bool() {
            if msg.message == WM_QUIT {
                unregister_hotkeys(&hwnd);
                return Ok(());
            }
            // WM_HOTKEY 처리 — 방향키 snap.
            if msg.message == WM_HOTKEY {
                handle_hotkey(msg.wParam.0 as i32, &keyboard_service, &config_store);
            }
            // WM_DISPLAYCHANGE — 멀티 모니터 갱신 (Task 4에서 처리).
            unsafe { TranslateMessage(&msg) };
            unsafe { DispatchMessageW(&msg) };
        }
        // 폴링 — throw modifier 감지.
        poll_throw(&snap_service, &config_store, &state);
    }
}

fn create_message_window() -> windows::core::Result<HWND> {
    // RegisterClassExW + CreateWindowExW(WS_EX_MESSAGE or HWND_MESSAGE parent).
    // WndProc — DefWindowProcW (message-only, 직접 처리는 GetMessage 루프에서).
    // ... 구현
    unimplemented!("Task 3 Step 2")
}

fn register_hotkeys(hwnd: &HWND, config_store: &Arc<dyn ConfigStore>) {
    // config.keyboard.trigger_modifiers → MOD_* 플래그 조합.
    // MOD_NOREPEAT | modifier flags. VK_LEFT/RIGHT/UP/DOWN으로 4개 등록.
    // RegisterHotKey 실패(점유됨) 시 로깅 + 해당 핫키 스킵.
    // ... 구현
}

fn unregister_hotkeys(hwnd: &HWND) {
    // UnregisterHotKey for all 4 IDs.
}
```

- [ ] **Step 3: WM_HOTKEY 핸들러 (키보드 snap)**

```rust
fn handle_hotkey(
    hotkey_id: i32,
    keyboard_service: &Arc<KeyboardService>,
    _config_store: &Arc<dyn ConfigStore>,
) {
    let direction = match hotkey_id {
        HOTKEY_LEFT => Direction::Left,
        HOTKEY_RIGHT => Direction::Right,
        HOTKEY_UP => Direction::Up,
        HOTKEY_DOWN => Direction::Down,
        _ => return,
    };
    // 커서 위치 획득.
    let (cx, cy) = current_cursor();
    let _ = keyboard_service.on_direction_key(direction, cx, cy);
}

fn current_cursor() -> (i32, i32) {
    let mut pt = POINT::default();
    unsafe { let _ = GetCursorPos(&mut pt) };
    (pt.x, pt.y)
}
```

- [ ] **Step 4: 폴링 — throw modifier 감지 + SnapService 구동**

```rust
fn poll_throw(
    snap_service: &Arc<SnapService>,
    config_store: &Arc<dyn ConfigStore>,
    state: &Arc<Mutex<InputState>>,
) {
    let config = match config_store.load() {
        Ok(c) => c,
        Err(_) => return,
    };
    // throw modifier 조합이 모두 눌렸는지.
    let modifiers_held = check_modifiers(&config.throw.trigger_modifiers);

    let mut st = state.lock().unwrap();
    if modifiers_held && !st.throw_active {
        // Idle → Held.
        st.throw_active = true;
        let (cx, cy) = current_cursor();
        st.origin = Some((cx, cy));
        drop(st);
        let _ = snap_service.on_modifier_pressed(cx, cy);
    } else if modifiers_held && st.throw_active {
        // Held 유지 — delta 계산.
        if let Some((ox, oy)) = st.origin {
            let (cx, cy) = current_cursor();
            let dx = (cx - ox) as f64;
            let dy = (cy - oy) as f64;
            drop(st);
            let _ = snap_service.on_mouse_moved(cx, cy, dx, dy);
        }
    } else if !modifiers_held && st.throw_active {
        // Held → Idle.
        st.throw_active = false;
        let (cx, cy) = current_cursor();
        st.origin = None;
        drop(st);
        let _ = snap_service.on_modifier_released(false, cx, cy);
    }
}

/// modifier 문자열 목록이 모두 눌려 있는지 GetAsyncKeyState로 확인.
fn check_modifiers(mods: &[String]) -> bool {
    for m in mods {
        let held = match m.as_str() {
            "Win" => unsafe { GetAsyncKeyState(VK_LWIN.0 as i32) < 0 || GetAsyncKeyState(VK_RWIN.0 as i32) < 0 },
            "Alt" => unsafe { GetAsyncKeyState(VK_MENU.0 as i32) < 0 },
            "Ctrl" => unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) < 0 },
            "Shift" => unsafe { GetAsyncKeyState(VK_SHIFT.0 as i32) < 0 },
            _ => false,
        };
        if !held {
            return false;
        }
    }
    true
}
```

- [ ] **Step 5: 컴파일 확인**

Run: `cd src-tauri && cargo check`
Expected: 컴파일 성공.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: RegisterHotKey + GetAsyncKeyState 폴링 입력 리스너

LL hook 없이 message-only 창에서 키보드 snap(RegisterHotKey)과
마우스 throw(GetAsyncKeyState 폴링) 처리. PowerToys 충돌 없음."
```

---

### Task 4: 멀티 모니터 동적 감지 (WM_DISPLAYCHANGE)

**Files:**
- Modify: `src-tauri/src/infrastructure/win32_monitor.rs`
- Modify: `src-tauri/src/infrastructure/win32_input.rs`

**Interfaces:**
- Consumes: 기존 `Win32MonitorProvider`
- Produces: `Win32MonitorProvider`에 `invalidate_cache()` 또는 동적 캐시. 입력 리스너에서 WM_DISPLAYCHANGE 수신 시 호출.

**목표:** 모니터 연결/해제 시 MonitorProvider가 최신 모니터 정보를 반환하도록 캐시 무효화.

- [ ] **Step 1: Win32MonitorProvider에 캐시 추가**

`src-tauri/src/infrastructure/win32_monitor.rs`:
- `Win32MonitorProvider`에 `cached_monitors: Mutex<Option<Vec<MonitorBounds>>>` 필드 추가
- `enumerate()`에서 캐시가 있으면 반환, 없으면 EnumDisplayMonitors 호출 후 캐싱
- `pub fn invalidate(&self)` 메서드 추가 — 캐시를 None으로

```rust
pub struct Win32MonitorProvider {
    cached: Mutex<Option<Vec<MonitorBounds>>>,
}

impl Win32MonitorProvider {
    pub fn new() -> Self {
        Self { cached: Mutex::new(None) }
    }

    /// 캐시 무효화 — WM_DISPLAYCHANGE 수신 시 호출.
    pub fn invalidate(&self) {
        *self.cached.lock().unwrap() = None;
    }
}

impl MonitorProvider for Win32MonitorProvider {
    fn enumerate(&self) -> Vec<MonitorBounds> {
        let mut cache = self.cached.lock().unwrap();
        if let Some(ref monitors) = *cache {
            return monitors.clone();
        }
        // 기존 EnumDisplayMonitors 로직으로 수집.
        let monitors = /* ... 기존 코드 ... */;
        *cache = Some(monitors.clone());
        monitors
    }
    // monitor_at 은 enumerate 캐시 사용 또는 직접 MonitorFromPoint (변경 없음 가능).
}
```

- [ ] **Step 2: 입력 리스너에서 WM_DISPLAYCHANGE 처리**

`src-tauri/src/infrastructure/win32_input.rs`의 메시지 루프에서:

```rust
if msg.message == WM_DISPLAYCHANGE {
    // MonitorProvider 캐시 무효화.
    // Win32InputListener가 Arc<Win32MonitorProvider>를 보유하도록 start() 시그니처 변경.
    if let Some(mp) = monitor_provider.as_win32() {
        mp.invalidate();
    }
}
```

`Win32InputListener::start()`에 `monitor_provider: Arc<dyn MonitorProvider>` 매개변수 추가. Windows 전용 downcast는 어려우므로, `Arc<Win32MonitorProvider>`를 별도로 받거나 `Arc<dyn MonitorProvider>` + `invalidate`를 trait에 추가하는 방식 선택. 간단하게 — `Win32InputListener::start()`에 `Arc<Win32MonitorProvider>`를 구체 타입으로 전달.

- [ ] **Step 3: 컴파일 + 테스트**

Run: `cd src-tauri && cargo check && cargo test`
Expected: 컴파일 성공, 기존 테스트 통과.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: 멀티 모니터 동적 감지 (WM_DISPLAYCHANGE)

MonitorProvider 캐시 무효화. 모니터 연결/해제 시 실시간 갱신."
```

---

### Task 5: wiring — state.rs/lib.rs 연결 + 설정 기본값 정리

**Files:**
- Modify: `src-tauri/src/presentation/state.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/domain/model.rs` (ModifierMode 기본값 — 이미 Separate, 확인만)

**Interfaces:**
- Consumes: `Win32LayeredOverlay` (Task 2), `Win32InputListener` (Task 3), `Win32MonitorProvider` (Task 4)
- Produces: 완전한 wiring — 앱 시작 시 오버레이 + 입력 리스너 + 모니터 감지 작동.

- [ ] **Step 1: state.rs — overlay를 Win32LayeredOverlay로 (Windows)**

`src-tauri/src/presentation/state.rs`:
- Windows: `let overlay: Arc<dyn OverlayController> = Arc::new(crate::infrastructure::win32_overlay::Win32LayeredOverlay::new());`
- 비Windows: 기존 `TauriOverlay::new()` 유지.
- `snap_service`에 `overlay.clone()` 전달 (기존과 동일).
- `Win32MonitorProvider`를 구체 타입으로 보관 (입력 리스너에 전달용).

```rust
pub struct AppState {
    pub config_store: Arc<TomlConfigStore>,
    pub window_mover: Arc<dyn WindowMover>,
    pub monitor_provider: Arc<Win32MonitorProvider>,  // Windows 구체 타입 (cfg(windows))
    pub overlay: Arc<dyn OverlayController>,
    pub snap_service: Arc<SnapService>,
    pub keyboard_service: Arc<KeyboardService>,
}
```

비Windows용으로는 trait object `Arc<dyn MonitorProvider>` 유지 + cfg 분기.

- [ ] **Step 2: lib.rs — Win32InputListener 시작**

`src-tauri/src/lib.rs`의 setup 클로저에서:

```rust
#[cfg(windows)]
{
    let state = app.state::<presentation::state::AppState>();
    crate::infrastructure::win32_input::Win32InputListener::start(
        state.snap_service.clone(),
        state.keyboard_service.clone(),
        state.config_store.clone(),
        state.monitor_provider.clone(),
    );
}
```

- [ ] **Step 3: model.rs — ModifierMode 기본값 확인**

`src-tauri/src/domain/model.rs`의 `KeyboardConfig::default()`가 `modifier_mode: ModifierMode::Separate`인지 확인 (이미 그래야 함). OverrideOs는 enum에 남기되 기본값에서 사용 안 함.

- [ ] **Step 4: 컴파일 확인**

Run: `cd src-tauri && cargo check`
Expected: 컴파일 성공.

- [ ] **Step 5: 테스트 확인**

Run: `cd src-tauri && cargo test`
Expected: 기존 테스트 통과 (88 passed).

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: Win32InputListener + Win32LayeredOverlay wiring

앱 시작 시 DirectComposition 오버레이 + RegisterHotKey/폴링 입력 리스너 시작.
MonitorProvider 구체 타입 보관 (WM_DISPLAYCHANGE 무효화용)."
```

---

### Task 6: 통합 검증 — 빌드 + 수동 테스트

**Files:** 없음 (검증만)

- [ ] **Step 1: 전체 빌드**

```bash
cd "C:/Users/troub/Projects/rectangle-win"
npm run tauri dev
```

Expected: 에러 없이 빌드 + 앱 실행. 설정 창 정상.

- [ ] **Step 2: 설정 UI 정상 확인**

- 트레이 아이콘 → 설정 클릭 → 설정 창 표시
- General/About/Keyboard 탭 전환 정상
- `[object Object]` 등 깨짐 없음

- [ ] **Step 3: 키보드 snap 테스트 (Ctrl+Alt+방향키)**

- 다른 앱 창(메모장) 포커스
- Ctrl+Alt+→ → 창이 오른쪽 절반으로 snap
- Ctrl+Alt+→ 반복 → 체인 사이클(left-half → third-left → center → ...)
- Ctrl+Alt+← → 역방향

- [ ] **Step 4: 마우스 throw 테스트 (Win+Alt + 드래그)**

- 다른 앱 창 포커스
- Win+Alt 누르고 마우스 이동 → DirectComposition 오버레이에 조준경(섹터 부채꼴) + snap 미리보기(점선 사각형) 표시
- 깜빡임 없음 (Layered 창 show/hide 반복 X)
- 떼면 해당 영역으로 snap

- [ ] **Step 5: maximize 후 snap 테스트**

- 창을 maximize (Win+위 또는 버튼)
- Win+Alt + 다른 방향 → SW_RESTORE로 복원 후 snap (작동해야 함)

- [ ] **Step 6: PowerToys 충돌 테스트**

- PowerToys KeyboardManager 활성화 상태에서
- Ctrl+Alt+방향키, Win+Alt 드래그 모두 작동
- PowerToys 리매핑과 충돌 없음

- [ ] **Step 7: 모니터 변경 테스트**

- (모니터 2대 이상 환경) 모니터 연결/해제
- snap이 올바른 모니터 기준으로 작동 (WM_DISPLAYCHANGE 감지)

- [ ] **Step 8: 최종 Commit**

```bash
git add -A && git commit -m "test: 통합 검증 완료 — LL hook 없는 입력/오버레이 전면 교체

DirectComposition 오버레이 + RegisterHotKey/폴링 입력.
PowerToys 충돌 없음, 깜빡임 없음, 멀티 모니터 동적 감지."
```

---

## Self-Review 체크

**1. Spec coverage:**
- LL hook 제거 → Task 1 ✓
- DirectComposition/Direct2D 오버레이 → Task 2 ✓
- RegisterHotKey + 폴링 → Task 3 ✓
- 멀티 모니터 동적 감지 → Task 4 ✓
- wiring → Task 5 ✓
- 검증 → Task 6 ✓
- SW_RESTORE (maximize 후 snap) → Task 1에서 이미 기존 코드에 있음 (확인 필요 — 없으면 win32_window.rs에 추가)

**2. Placeholder scan:** Task 2/3에 `unimplemented!()` 마커가 있으나, 이는 "구현 상세를 이 태스크에서 채운다"는 의미이지 빈 단계가 아님. 실제 D2D 그리기/창 생성 코드는 태스크 실행 시 작성.

**3. Type consistency:**
- `Win32LayeredOverlay::new() -> Self` — Task 2 정의, Task 5 사용 ✓
- `Win32InputListener::start(snap, keyboard, config, monitor_provider)` — Task 3 정의, Task 5 사용. Task 4에서 monitor_provider 매개변수 추가 ✓
- `OverlayController` trait — 기존 ports.rs, 변경 없음 ✓
- `Direction` — 기존 domain/model.rs ✓

**누락 보충:** win32_window.rs의 SW_RESTORE이 기존 코드에 이미 추가되어 있는지 확인 필요. 이전 작업에서 추가했으므로 존재해야 함. Task 1 Step 6 이후 확인.
