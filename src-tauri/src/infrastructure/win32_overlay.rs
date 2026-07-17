#![cfg(windows)]

//! WS_EX_LAYERED + UpdateLayeredWindow 기반 오버레이 창.
//!
//! 이전에는 `WS_EX_NOREDIRECTIONBITMAP` + DirectComposition + DXGI swap chain 을
//! 사용했으나, 해당 환경에서는 click-through(`WS_EX_TRANSPARENT` + `HTTRANSPARENT`)
//! 가 신뢰성 있게 동작하지 않아 작업표시줄 팝업/컨텍스트 메뉴/창 닫기 버튼을
//! 가로막는 문제가 있었다. 게임 오버레이에서 검증된 고전적 방식인
//! `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE` +
//! `UpdateLayeredWindow(ULW_ALPHA | AC_SRC_ALPHA)` 조합으로 전환한다.
//!
//! 렌더링 파이프라인:
//! 1. 32bpp DIB section (`CreateDIBSection`, `BI_RGB`) 을 메모리 DC 에 선택.
//! 2. `ID2D1DCRenderTarget` 를 생성하고 `BindDC` 로 해당 DC 에 바인딩.
//! 3. Direct2D 로 장면을 그린다 (BeginDraw/EndDraw). DC render target 은 GDI 와
//!    호환되므로 픽셀이 DIB 버퍼에 곧바로 기록된다.
//! 4. `UpdateLayeredWindow` 로 DIB 의 ARGB 픽셀을 layered window 에 합성.
//!
//! [`OverlayController`] trait 구현 — Rectangle Pro 스타일 snap 미리보기 사각형.
//! 색상/반경/투명도는 `OverlayConfig` 에서 읽어 반영.
//!
//! 설계 요점:
//! - 창은 항상 "현재 snap preview 사각형"만큼의 크기로 위치한다.
//!   preview 가 없으면(초기 lock-on/숨김) 1x1 로 축소한다.
//!   이렇게 하면 항상-전체화면-최상위 창이 작업표시줄 팝업/컨텍스트 메뉴/
//!   타이틀바 버튼을 가리는 문제를 막는다.
//! - show/hide: `visible=false` 시 `SW_HIDE`, `visible=true` 시 `SW_SHOWNOACTIVATE`.
//!   창이 숨겨진 동안에는 시스템 UI 를 가리지 않는다.
//! - 모든 상태 변경마다 전체 재그리기 (D2D 는 충분히 빠름).
//! - 초기화 실패 시 `resources` 가 None 이며 redraw() 는 no-op.
//!   snap 자체는 오버레이 없이도 동작 (graceful degradation).
//! - D2D 팩토리는 single-threaded 이며, 모든 호출은 입력 스레드에서 직렬로 들어온다.
//!   `Mutex<OverlayResources>` 가 접근을 직렬화한다.
//!
//! 단위 테스트는 실제 OS/GPU 상호작용이 필요하므로 작성하지 않는다
//! (기존 win32_window/win32_monitor 패턴과 동일).
//!
//! [`OverlayController`]: crate::application::ports::OverlayController

use std::sync::{Arc, Mutex};

use windows::core::{Interface, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_CAP_STYLE_FLAT, D2D1_DASH_STYLE_DASH, D2D1_DEBUG_LEVEL_NONE, D2D1_FACTORY_OPTIONS,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_STROKE_STYLE_PROPERTIES1,
    D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory1, ID2D1SolidColorBrush, ID2D1StrokeStyle,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, DIB_RGB_COLORS, HBITMAP, HDC,
    HGDIOBJ, SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetSystemMetrics, RegisterClassExW, SetWindowPos, ShowWindow,
    UpdateLayeredWindow, CS_HREDRAW, CS_VREDRAW, HTTRANSPARENT, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_HIDE, SW_SHOWNOACTIVATE,
    SWP_NOACTIVATE, SWP_NOZORDER, UPDATE_LAYERED_WINDOW_FLAGS, ULW_ALPHA, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_NCHITTEST, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::application::errors::AppResult;
use crate::application::ports::{ConfigStore, OverlayController};
use crate::domain::model::OverlayConfig;

/// 오버레이 렌더링 상태 — 그릴 내용을 보관.
#[derive(Default)]
struct OverlayDrawState {
    visible: bool,
    #[allow(dead_code)]
    center: Option<(i32, i32)>,
    #[allow(dead_code)]
    sector_count: u8,
    active_sector: Option<u8>,
    snap_preview: Option<(i32, i32, i32, i32)>,
    #[allow(dead_code)]
    cursor: Option<(i32, i32)>,
}

/// WS_EX_LAYERED + UpdateLayeredWindow 기반 오버레이.
///
/// 앱 시작 시 창을 한 번 생성하고, [`OverlayController`] 메서드 호출 시마다
/// 상태를 갱신하고 D2D 로 다시 그린 뒤 `UpdateLayeredWindow` 로 합성한다.
pub struct Win32LayeredOverlay {
    state: Mutex<OverlayDrawState>,
    // GDI/D2D 리소스 — 초기화 후 크기 변경 시 DIB/HDC 만 재생성.
    // 초기화 실패 시 None (graceful degradation: snap만 작동, 오버레이 없음).
    resources: Mutex<Option<OverlayResources>>,
    // 설정 저장소 — redraw 시 OverlayConfig 색상/반경/투명도를 로드.
    config_store: Arc<dyn ConfigStore>,
}

/// 렌더링 리소스 묶음.
///
/// - `hwnd`: layered 오버레이 창.
/// - `hdc_mem`: DIB 를 선택한 메모리 DC. UpdateLayeredWindow 의 소스 DC.
/// - `hbitmap`: 32bpp DIB section (ARGB). Direct2D 가 여기에 픽셀을 기록.
/// - `dc_render_target`: DIB DC 에 바인딩된 Direct2D render target.
/// - `brush` / `dash_style`: 매 재사용.
///
/// 모든 Win32 COM 핸들(HWND)과 windows-rs 인터페이스 포인터는 기본적으로
/// `!Send`/`!Sync` 이지만, 본 오버레이는 단일 입력 스레드에서만 접근되며
/// `Mutex` 가 직렬화를 보장하므로 `Send + Sync` 를 수동으로 선언한다.
struct OverlayResources {
    hwnd: HWND,
    hdc_mem: HDC,
    hbitmap: HBITMAP,
    /// 이전 비트맵(기본 1x1 모노). DeleteObject 로 정리용 보관.
    _previous_bmp: HGDIOBJ,
    dc_render_target: ID2D1DCRenderTarget,
    #[allow(dead_code)]
    d2d_factory: ID2D1Factory1,
    brush: ID2D1SolidColorBrush,
    /// 점선(대시) 사각형용 stroke style (snap preview).
    dash_style: ID2D1StrokeStyle,
    width: i32,
    height: i32,
}

// SAFETY: OverlayResources 의 모든 핸들/인터페이스는 단일 입력 스레드에서만
// 사용되며, Mutex<OverlayResources> 가 접근을 직렬화한다. COM 객체는
// free-threaded 가 아니지만 본 오버레이는 동시 접근이 없으므로 안전하다.
unsafe impl Send for OverlayResources {}
unsafe impl Sync for OverlayResources {}

impl Win32LayeredOverlay {
    pub fn new(config_store: Arc<dyn ConfigStore>) -> Self {
        let resources = match Self::init_resources() {
            Ok(r) => {
                eprintln!("[OVERLAY] init_resources 성공 (layered 창 + D2D 준비됨)");
                Some(r)
            }
            Err(e) => {
                eprintln!("[OVERLAY] init_resources 실패: {e} — 오버레이 비활성, snap만 작동");
                None
            }
        };
        Self {
            state: Mutex::new(OverlayDrawState::default()),
            resources: Mutex::new(resources),
            config_store,
        }
    }

    fn init_resources() -> windows::core::Result<OverlayResources> {
        // 1. 가상 데스크톱 전체 크기 (초기값 — 실제 크기는 snap preview rect 로 갱신).
        let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
        let (init_w, init_h) = if width > 0 && height > 0 {
            (width, height)
        } else {
            (1, 1)
        };

        // 2. 오버레이 창 생성 (layered-transparent-topmost-noactivate).
        let hwnd = Self::create_overlay_window(x, y, init_w, init_h)?;

        // 3. Direct2D factory.
        let d2d_factory: ID2D1Factory1 = unsafe {
            D2D1CreateFactory(
                D2D1_FACTORY_TYPE_SINGLE_THREADED,
                Some(&D2D1_FACTORY_OPTIONS {
                    debugLevel: D2D1_DEBUG_LEVEL_NONE,
                }),
            )?
        };

        // 4. DC render target — GDI 호환(DIB)용. 픽셀 포맷은 B8G8R8A8 premultiplied.
        let rt_props = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };
        let dc_render_target: ID2D1DCRenderTarget =
            unsafe { d2d_factory.CreateDCRenderTarget(&rt_props)? };

        // 5. 32bpp DIB section + 메모리 DC 생성 후 DC render target 에 바인딩.
        let (hdc_mem, hbitmap, previous_bmp) = Self::create_dib(init_w, init_h)?;
        let bind_rect = RECT {
            left: 0,
            top: 0,
            right: init_w,
            bottom: init_h,
        };
        if let Err(e) = unsafe { dc_render_target.BindDC(hdc_mem, &bind_rect) } {
            let _ = unsafe { DeleteObject(hbitmap) };
            let _ = unsafe { DeleteDC(hdc_mem) };
            return Err(e);
        }

        // 6. 범용 브러시 + 점선 stroke style.
        let brush: ID2D1SolidColorBrush = unsafe {
            dc_render_target.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
                None,
            )?
        };
        let dash_style: ID2D1StrokeStyle = unsafe {
            d2d_factory
                .CreateStrokeStyle(
                    &D2D1_STROKE_STYLE_PROPERTIES1 {
                        startCap: D2D1_CAP_STYLE_FLAT,
                        endCap: D2D1_CAP_STYLE_FLAT,
                        dashCap: D2D1_CAP_STYLE_FLAT,
                        lineJoin: Default::default(),
                        miterLimit: 10.0,
                        dashStyle: D2D1_DASH_STYLE_DASH,
                        dashOffset: 0.0,
                        transformType: Default::default(),
                    },
                    None,
                )?
                .cast()?
        };

        Ok(OverlayResources {
            hwnd,
            hdc_mem,
            hbitmap,
            _previous_bmp: previous_bmp,
            dc_render_target,
            d2d_factory,
            brush,
            dash_style,
            width: init_w,
            height: init_h,
        })
    }

    /// 32bpp DIB section + 메모리 DC 생성.
    /// 반환: (메모리 DC, DIB HBITMAP, SelectObject 로 얻은 이전 객체).
    /// 호출자는 크기 변경 시 이전 DIB 를 DeleteObject 해야 한다.
    fn create_dib(
        w: i32,
        h: i32,
    ) -> windows::core::Result<(HDC, HBITMAP, HGDIOBJ)> {
        let w = w.max(1);
        let h = h.max(1);
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                // 음수 height → top-down DIB (Direct2D 픽셀 방향과 일치).
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default(); 1],
        };
        let hdc_mem: HDC = unsafe { CreateCompatibleDC(None) };
        if hdc_mem.is_invalid() {
            return Err(windows::core::Error::from_hresult(
                windows::Win32::Foundation::E_FAIL,
            ));
        }
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbitmap: HBITMAP = unsafe {
            CreateDIBSection(hdc_mem, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)?
        };
        // DIB 를 메모리 DC 에 선택. 이전 객체는 복구/정리용으로 보관.
        let previous_bmp = unsafe { SelectObject(hdc_mem, hbitmap) };
        Ok((hdc_mem, hbitmap, previous_bmp))
    }

    /// 오버레이 창 생성.
    ///
    /// `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE`
    /// + `WS_POPUP`. 초기 위치/크기는 가상 데스크톱 전체. 생성 후 숨김 상태.
    fn create_overlay_window(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> windows::core::Result<HWND> {
        let class_name = windows::core::w!("RectangleWinOverlay");

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(overlay_wndproc),
            hInstance: HINSTANCE::default(),
            lpszClassName: class_name,
            ..Default::default()
        };
        let _atom = unsafe { RegisterClassExW(&wc) };

        let ex_style = WINDOW_EX_STYLE(
            (WS_EX_LAYERED.0
                | WS_EX_TRANSPARENT.0
                | WS_EX_TOPMOST.0
                | WS_EX_NOACTIVATE.0) as u32,
        );
        let style = WINDOW_STYLE(WS_POPUP.0);

        let hwnd = unsafe {
            CreateWindowExW(
                ex_style,
                class_name,
                PCWSTR::null(),
                style,
                x,
                y,
                width,
                height,
                None,
                None,
                HINSTANCE::default(),
                None,
            )?
        };

        // 숨김 상태로 시작. throw 활성(visible=true) 시에만 SW_SHOWNOACTIVATE.
        // 항상 떠 있는 전체화면 투명 창은 입력 이벤트 타이밍에 따라 시스템 UI 를
        // 가릴 수 있으므로, 명시적으로 show/hide 로 제어한다.
        let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };

        Ok(hwnd)
    }

    /// 현재 상태로 전체 재그리기 + 창 표시 제어.
    ///
    /// 창은 전체 가상 데스크톱 크기로 고정. 절대 좌표로 그림.
    fn redraw(&self) {
        let res_guard = self.resources.lock().unwrap();
        let Some(res) = res_guard.as_ref() else {
            return;
        };
        let state = self.state.lock().unwrap();
        if !state.visible {
            let _ = unsafe { ShowWindow(res.hwnd, SW_HIDE) };
            return;
        }
        let _ = unsafe { ShowWindow(res.hwnd, SW_SHOWNOACTIVATE) };
        let overlay_cfg = self
            .config_store
            .load()
            .map(|c| c.overlay)
            .unwrap_or_default();
        if let Err(e) = Self::draw_scene(res, &state, &overlay_cfg) {
            eprintln!("[OVERLAY] draw_scene 실패: {e}");
        }
    }

    /// 오버레이 창을 지정한 사각형(x, y, w, h — 가상 화면 좌표)으로 이동/크기 변경.
    ///
    /// `SetWindowPos` 로 창 위치/크기를 갱신하고, 크기가 바뀌면 DIB section 을
    /// 재할당하고 DC render target 을 다시 바인딩한다.
    fn position_overlay(
        res: &mut OverlayResources,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> windows::core::Result<()> {
        let w = w.max(1);
        let h = h.max(1);
        unsafe {
            SetWindowPos(
                res.hwnd,
                None,
                x,
                y,
                w,
                h,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )?;
        }
        if res.width != w || res.height != h {
            Self::resize_dib(res, w, h)?;
            res.width = w;
            res.height = h;
        }
        Ok(())
    }

    /// DIB section 재할당 + DC render target 재바인딩.
    /// 크기가 자주 바뀌면 비용이 크지만, snap preview rect 변경 시에만 호출된다.
    fn resize_dib(res: &mut OverlayResources, w: i32, h: i32) -> windows::core::Result<()> {
        // 기존 DIB 해제 순서: 이전 비트맵 복구 → DeleteObject(hbitmap) → 새 DIB 선택.
        let (new_hdc, new_bmp, new_prev) = Self::create_dib(w, h)?;
        // 기존 DC 의 DIB 정리.
        // 주의: CreateCompatibleDC 가 새 DC 를 만들었다. 기존 DC 도 교체한다.
        let _ = unsafe { SelectObject(res.hdc_mem, res._previous_bmp) };
        let _ = unsafe { DeleteObject(res.hbitmap) };
        let _ = unsafe { DeleteDC(res.hdc_mem) };
        res.hdc_mem = new_hdc;
        res.hbitmap = new_bmp;
        res._previous_bmp = new_prev;
        let bind_rect = RECT {
            left: 0,
            top: 0,
            right: w,
            bottom: h,
        };
        unsafe { res.dc_render_target.BindDC(res.hdc_mem, &bind_rect)? };
        Ok(())
    }

    /// 실제 장면 그리기 — snap 미리보기 점선 사각형만 그린다.
    ///
    /// 더 이상 커서 점/원/레티클 마커를 그리지 않는다. 색상은 `active_sector` 의
    /// 유무에 따라 자동으로 전환된다 (Option 2):
    /// - `active_sector == None` (lock-on, 현재 창): `cursor_color` (RED #E53935)
    /// - `active_sector == Some(_)` (throw target): `sector_highlight_color` (BLUE #3B82F6)
    ///
    /// 호출 패턴:
    /// - lock-on: `show_snap_preview` 만 호출 (active_sector=None → RED)
    /// - throw:   `highlight_sector` → `show_snap_preview` (active_sector=Some → BLUE)
    ///
    /// **좌표계:** 오버레이 창은 항상 snap preview rect (sx, sy, sw, sh) 와 동일한
    /// 위치/크기로 맞춰진다 (`redraw` → `position_overlay`). 따라서 사각형은
    /// 창-로컬 좌표 (0, 0)~(sw, sh) 로 그린다. (sx, sy) 가 가상 화면 좌표더라도
    /// 창 자체가 (sx, sy) 에 배치되므로 로컬 원점이 된다.
    ///
    /// 그린 후 반드시 `UpdateLayeredWindow` 로 DIB 픽셀을 창에 반영해야 한다
    /// (그렇지 않으면 화면에 나타나지 않는다).
    fn draw_scene(
        res: &OverlayResources,
        state: &OverlayDrawState,
        cfg: &OverlayConfig,
    ) -> windows::core::Result<()> {
        unsafe {
            res.dc_render_target.BeginDraw();

            // 투명하게 클리어 (premultiplied alpha). Clear/Fill/DrawRectangle 은
            // ID2D1RenderTarget 메서드로 반환형이 () 이다 (HRESULT 무시).
            res.dc_render_target.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));

            // snap 미리보기 — 절대 좌표(가상 화면 좌표)로 그림.
            if cfg.snap_preview {
                if let Some((sx, sy, sw, sh)) = state.snap_preview {
                    if sw > 0 && sh > 0 {
                        let rect = D2D_RECT_F {
                            left: sx as f32,
                            top: sy as f32,
                            right: (sx + sw) as f32,
                            bottom: (sy + sh) as f32,
                        };
                        // active_sector == None → lock-on (cursor_color, RED)
                        // active_sector == Some → throw target (sector_highlight_color, BLUE)
                        let color_hex = if state.active_sector.is_some() {
                            &cfg.sector_highlight_color
                        } else {
                            &cfg.cursor_color
                        };
                        let base_color = Self::parse_hex_color(color_hex);
                        // 채우기 (알파 0.20).
                        let mut fill_color = base_color;
                        fill_color.a = 0.20;
                        res.brush.SetColor(&fill_color);
                        res.dc_render_target.FillRectangle(&rect, &res.brush);
                        // 외곽선 (알파 0.95).
                        let mut stroke_color = base_color;
                        stroke_color.a = 0.95;
                        res.brush.SetColor(&stroke_color);
                        res.dc_render_target.DrawRectangle(
                            &rect,
                            &res.brush,
                            2.0,
                            Some(&res.dash_style),
                        );
                    }
                }
            }

            res.dc_render_target.EndDraw(None, None)?;

            // UpdateLayeredWindow 로 DIB 픽셀(ARGB premultiplied)을 layered 창에 합성.
            Self::update_layered(res)?;
        }
        Ok(())
    }

    /// `UpdateLayeredWindow` 로 DIB 픽셀을 layered window 에 반영.
    /// `ULW_ALPHA` + `AC_SRC_ALPHA` 조합으로 per-pixel 알파 합성.
    fn update_layered(res: &OverlayResources) -> windows::core::Result<()> {
        let pt_pos = POINT { x: 0, y: 0 };
        let size = SIZE {
            cx: res.width,
            cy: res.height,
        };
        let pt_src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        // hdcdst = None (화면 DC 를 시스템이 사용), hdcsrc = 메모리 DC.
        // crkey = COLORREF 기본값(0) — ULW_ALPHA 모드에서는 무시됨.
        unsafe {
            UpdateLayeredWindow(
                res.hwnd,
                None,
                Some(&pt_pos),
                Some(&size),
                res.hdc_mem,
                Some(&pt_src),
                windows::Win32::Foundation::COLORREF(0),
                Some(&blend),
                UPDATE_LAYERED_WINDOW_FLAGS(ULW_ALPHA.0),
            )?;
        }
        Ok(())
    }

    /// "#RRGGBB" (또는 "RRGGBB") 헥스 색상 → D2D1_COLOR_F (알파 1.0).
    /// snap preview 알파는 고정값(fill 0.20, stroke 0.95)을 사용.
    /// 파싱 실패 시 흰색(1,1,1)으로 폴백.
    fn parse_hex_color(hex: &str) -> D2D1_COLOR_F {
        let h = hex.trim_start_matches('#');
        let parse = |bytes: &str| u8::from_str_radix(bytes, 16).map(|v| v as f32 / 255.0);
        match (h.len(), h.get(0..2), h.get(2..4), h.get(4..6)) {
            (6, Some(r), Some(g), Some(b)) => match (parse(r), parse(g), parse(b)) {
                (Ok(r), Ok(g), Ok(b)) => D2D1_COLOR_F { r, g, b, a: 1.0 },
                _ => D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
            },
            _ => D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
        }
    }
}

impl OverlayController for Win32LayeredOverlay {
    /// Lock-on 진입 트리거 — 오버레이 창을 visible 상태로 전환한다.
    /// 더 이상 커서 점/레티클을 그리지 않는다. snap_preview 별도 표시 필요.
    /// active_sector=None, snap_preview=None 으로 클리어하여 다음 show_snap_preview 가
    /// RED(lock-on) 로 그려지도록 한다.
    fn show_reticle(&self, center_x: i32, center_y: i32, sector_count: u8) -> AppResult<()> {
        // 상태만 갱신 — redraw는 호출하지 않음.
        // show_snap_preview가 이후에 호출되어 한 번에 그리도록 함.
        // (show_reticle → show_snap_preview 순서로 호출되므로, redraw를 여기서
        // 호출하면 이전 프리뷰가 잠깐 보였다가 새 락온으로 바뀌는 깜빡임 발생)
        let mut state = self.state.lock().unwrap();
        state.visible = true;
        state.center = Some((center_x, center_y));
        state.sector_count = sector_count;
        state.active_sector = None;
        state.snap_preview = None;
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

/// 오버레이 창의 window proc.
///
/// WS_EX_TRANSPARENT 만으로도 마우스 입력은 통과하지만, 일부 환경에서는
/// 신뢰성을 보장하기 위해 WM_NCHITTEST 에 HTTRANSPARENT 를 반환한다
/// (belt and suspenders).
unsafe extern "system" fn overlay_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCHITTEST {
        return LRESULT(HTTRANSPARENT as isize);
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
