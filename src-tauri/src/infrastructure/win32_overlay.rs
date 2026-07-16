#![cfg(windows)]

//! DirectComposition + Direct2D 기반 오버레이 창.
//!
//! `WS_EX_NOREDIRECTIONBITMAP | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE`
//! 창을 만들고, D3D11/DXGI/Direct2D/DirectComposition 파이프라인으로 GPU 직접 합성.
//! [`OverlayController`] trait 구현 — Rectangle Pro 스타일의 커서 점 마커와
//! snap 미리보기 사각형을 그린다. 색상/반경/투명도는 `OverlayConfig` 에서 읽어 반영.
//!
//! 설계 요점:
//! - 창은 시작 시 한 번 생성되어 계속 떠 있다. `visible` 플래그로만 내용 노출을 제어
//!   (창 자체를 show/hide 반복하지 않음 → 깜빡임 없음).
//! - 모든 상태 변경마다 전체 재그리기 (D2D 는 충분히 빠름).
//! - D3D11/DComp 초기화 실패 시 `resources` 가 None 이며 redraw() 는 no-op.
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
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1,
    D2D1_CAP_STYLE_FLAT, D2D1_DASH_STYLE_DASH, D2D1_DEBUG_LEVEL_NONE,
    D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_FACTORY_OPTIONS,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_STROKE_STYLE_PROPERTIES1, D2D1CreateFactory,
    ID2D1Bitmap1, ID2D1Device, ID2D1DeviceContext, ID2D1Factory1, ID2D1SolidColorBrush,
    ID2D1StrokeStyle,
};

use crate::application::errors::AppResult;
use crate::application::ports::{ConfigStore, OverlayController};
use crate::domain::model::OverlayConfig;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, ID3D11Device,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, DXGI_PRESENT, DXGI_SWAP_CHAIN_DESC1,
    DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIDevice, IDXGIFactory2,
    IDXGISurface, IDXGISwapChain1,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetSystemMetrics, RegisterClassExW, ShowWindow,
    SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, CS_HREDRAW,
    CS_VREDRAW, SW_HIDE, SW_SHOWNOACTIVATE, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSEXW,
    WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
    HTTRANSPARENT, WM_NCHITTEST,
};

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
/// 앱 시작 시 창을 한 번 생성하고, [`OverlayController`] 메서드 호출 시마다
/// 상태를 갱신하고 D2D 로 다시 그린다. show/hide는 `visible` 플래그로만 제어
/// (창 자체를 show/hide 반복하지 않음 → 깜빡임 없음).
pub struct Win32LayeredOverlay {
    state: Mutex<OverlayDrawState>,
    // D3D11/DXGI/D2D/DComp 리소스 — 초기화 후 불변.
    // 초기화 실패 시 None (graceful degradation: snap만 작동, 오버레이 없음).
    resources: Mutex<Option<OverlayResources>>,
    // 설정 저장소 — redraw 시 OverlayConfig 색상/반경/투명도를 로드.
    config_store: Arc<dyn ConfigStore>,
}

/// GPU 렌더링 리소스 묶음.
///
/// 모든 Win32 COM 핸들(HWND)과 windows-rs 인터페이스 포인터는 기본적으로
/// `!Send`/`!Sync` 이지만, 본 오버레이는 단일 입력 스레드에서만 접근되며
/// `Mutex` 가 직렬화를 보장하므로 `Send + Sync` 를 수동으로 선언한다.
///
/// 일부 핸들(hwnd/factory 등)은 향후 태스크(리사이즈, 모니터 변경 대응)에서
/// 사용될 수 있어 보관용으로 유지한다.
#[allow(dead_code)]
struct OverlayResources {
    hwnd: HWND,
    _d3d_device: ID3D11Device,
    _dxgi_device: IDXGIDevice,
    dxgi_factory: IDXGIFactory2,
    swap_chain: IDXGISwapChain1,
    d2d_factory: ID2D1Factory1,
    _d2d_device: ID2D1Device,
    d2d_context: ID2D1DeviceContext,
    _dcomp_device: IDCompositionDevice,
    _dcomp_target: IDCompositionTarget,
    _dcomp_visual: IDCompositionVisual,
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
                eprintln!("[OVERLAY] init_resources 성공 (창+DComp 준비됨)");
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
        // 1. 가상 데스크톱 전체 크기.
        let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
        if width <= 0 || height <= 0 {
            return Err(windows::core::Error::from_hresult(
                windows::Win32::Foundation::E_FAIL,
            ));
        }

        // 2. D3D11 device 생성 — 명시적 feature level 배열 (11_1 → 10_1).
        // 빈 배열(Some(&[]))은 일부 환경에서 DXGI_ERROR_UNSUPPORTED(0x887A0004)로 실패.
        let feature_levels = [
            windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_1,
            windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0,
            windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_10_1,
            windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_10_0,
        ];
        let mut d3d_device: Option<ID3D11Device> = None;
        let d3d_result = unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                windows::Win32::Foundation::HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&feature_levels),
                D3D11_SDK_VERSION,
                Some(&mut d3d_device),
                None,
                None,
            )
        };
        let d3d_device = match d3d_result {
            Ok(()) => d3d_device.ok_or_else(|| {
                windows::core::Error::from_hresult(windows::Win32::Foundation::E_FAIL)
            })?,
            Err(hardware_err) => {
                // 하드웨어 device 실패 시 WARP(소프트웨어 래스터라이저)로 폴백.
                eprintln!("[OVERLAY] D3D11 하드웨어 실패({hardware_err}), WARP 폴백 시도");
                let mut warp_device: Option<ID3D11Device> = None;
                unsafe {
                    D3D11CreateDevice(
                        None,
                        windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_WARP,
                        windows::Win32::Foundation::HMODULE::default(),
                        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                        Some(&feature_levels),
                        D3D11_SDK_VERSION,
                        Some(&mut warp_device),
                        None,
                        None,
                    )?;
                }
                warp_device.ok_or_else(|| {
                    windows::core::Error::from_hresult(windows::Win32::Foundation::E_FAIL)
                })?
            }
        };

        // 3. DXGI factory + swap chain (composition용, premultiplied alpha).
        // IDXGIDevice.GetParent::<IDXGIFactory2> 는 직접 부모(IDXGIAdapter)만
        // 반환하므로 실패한다. 대신 CreateDXGIFactory2 로 factory 를 직접 생성.
        let dxgi_device: IDXGIDevice = d3d_device.cast().map_err(|e| {
            eprintln!("[OVERLAY] d3d_device.cast::<IDXGIDevice> 실패: {e}");
            e
        })?;
        let dxgi_factory: IDXGIFactory2 = unsafe { CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)) }
            .map_err(|e| {
                eprintln!("[OVERLAY] CreateDXGIFactory2 실패: {e}");
                e
            })?;

        let swap_desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: width as u32,
            Height: height as u32,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
            ..Default::default()
        };
        let swap_chain = unsafe { dxgi_factory.CreateSwapChainForComposition(&dxgi_device, &swap_desc, None) }
            .map_err(|e| {
                eprintln!("[OVERLAY] CreateSwapChainForComposition 실패: {e}");
                e
            })?;

        // 4. 오버레이 창 생성 (layered-transparent-topmost-noactivate).
        let hwnd = Self::create_overlay_window(x, y, width, height)?;

        // 5. Direct2D factory + device (D3D11에서).
        let d2d_factory: ID2D1Factory1 = unsafe {
            D2D1CreateFactory(
                D2D1_FACTORY_TYPE_SINGLE_THREADED,
                Some(&D2D1_FACTORY_OPTIONS {
                    debugLevel: D2D1_DEBUG_LEVEL_NONE,
                }),
            )?
        };
        let d2d_device = unsafe { d2d_factory.CreateDevice(&dxgi_device)? };
        let d2d_context =
            unsafe { d2d_device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)? };

        // 6. DirectComposition device + target + visual.
        let dcomp_device: IDCompositionDevice =
            unsafe { DCompositionCreateDevice(&dxgi_device)? };
        let dcomp_target = unsafe { dcomp_device.CreateTargetForHwnd(hwnd, true)? };
        let dcomp_visual = unsafe { dcomp_device.CreateVisual()? };
        unsafe {
            dcomp_visual.SetContent(&swap_chain)?;
            dcomp_target.SetRoot(&dcomp_visual)?;
            dcomp_device.Commit()?;
        }

        // 7. snap preview 용 대시 stroke style.
        //    ID2D1Factory1::CreateStrokeStyle 는 _PROPERTIES1 + ID2D1StrokeStyle1 을
        //    반환하므로, base ID2D1StrokeStyle 로 캐스트하여 보관/사용한다.
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
            _d3d_device: d3d_device,
            _dxgi_device: dxgi_device,
            dxgi_factory,
            swap_chain,
            d2d_factory,
            _d2d_device: d2d_device,
            d2d_context,
            _dcomp_device: dcomp_device,
            _dcomp_target: dcomp_target,
            _dcomp_visual: dcomp_visual,
            dash_style,
            width,
            height,
        })
    }

    /// 오버레이 창 생성.
    ///
    /// `WS_EX_NOREDIRECTIONBITMAP | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE`
    /// + `WS_POPUP`. 가상 데스크톱 전체 위치/크기. 생성 후 `SW_SHOWNOACTIVATE` 로
    /// 한 번만 표시 (이후 계속 떠 있음 — visible 플래그로만 내용 노출 제어).
    fn create_overlay_window(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> windows::core::Result<HWND> {
        // SAFETY: HSTRING → PCWSTR 변환은 NUL 종료를 보장. 클래스 이름은 앱 고유.
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
        // 0 이면 이미 등록되었거나 실패 — CreateWindowExW 가 클래스를 찾지 못하면
        // 에러를 반환하므로 여기서는 별도 처리하지 않는다.

        let ex_style = WINDOW_EX_STYLE(
            (WS_EX_NOREDIRECTIONBITMAP.0
                | WS_EX_TRANSPARENT.0
                | WS_EX_TOPMOST.0
                | WS_EX_NOACTIVATE.0) as u32,
        );
        let style = WINDOW_STYLE(WS_POPUP.0);

        // SAFETY: 클래스는 위에서 등록했음. HWND/HMENU 는 0(없음), HINSTANCE 도 0.
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

        // 창은 생성 후 숨김 상태로 시작. throw 활성(visible=true) 시에만 표시.
        // 항상 떠 있는 전체화면 투명 창은 WS_EX_TRANSPARENT/HTTRANSPARENT 와 무관하게
        // 일부 환경에서 입력을 삼키는 문제가 있으므로, 숨김/표시로 제어한다.
        // (DirectComposition + NOREDIRECTIONBITMAP 은 창을 숨겼다 보여도 리소스 유지)
        // ShowWindow 호출하지 않음 — 숨김 상태로 둠.

        Ok(hwnd)
    }

    /// 현재 상태로 전체 재그리기 + 창 show/hide 제어.
    fn redraw(&self) {
        let res_guard = self.resources.lock().unwrap();
        let Some(res) = res_guard.as_ref() else {
            return; // 초기화 실패 — no-op
        };
        let state = self.state.lock().unwrap();
        if !state.visible {
            // 숨김 — 창 자체를 hide. 투명 클리어 불필요 (창이 안 보임).
            unsafe { ShowWindow(res.hwnd, SW_HIDE) };
            return;
        }
        // 표시 — 창을 보이게 하고(포커스 없이) 그리기.
        unsafe { ShowWindow(res.hwnd, SW_SHOWNOACTIVATE) };
        // OverlayConfig 로드 실패 시 기본값으로 폴백 (오버레이는 계속 동작).
        let overlay_cfg = self
            .config_store
            .load()
            .map(|c| c.overlay)
            .unwrap_or_default();
        let _ = Self::draw_scene(res, &state, &overlay_cfg);
    }

    /// 백버퍼를 투명하게 클리어하고 Present.
    fn clear_buffer(res: &OverlayResources) -> windows::core::Result<()> {
        let bitmap = Self::bind_back_buffer(res)?;
        unsafe {
            res.d2d_context.SetTarget(&bitmap);
            res.d2d_context.BeginDraw();
            res.d2d_context.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));
            res.d2d_context.EndDraw(None, None)?;
            let _ = res.swap_chain.Present(1, DXGI_PRESENT(0));
        }
        Ok(())
    }

    /// swap chain 백버퍼(0) → ID2D1Bitmap1 (렌더타겟) 바인딩.
    fn bind_back_buffer(res: &OverlayResources) -> windows::core::Result<ID2D1Bitmap1> {
        unsafe {
            let back_buffer: IDXGISurface = res.swap_chain.GetBuffer(0)?;
            let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                colorContext: std::mem::ManuallyDrop::new(None),
            };
            res.d2d_context
                .CreateBitmapFromDxgiSurface(&back_buffer, Some(&bitmap_props))
        }
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
    /// ID2D1RenderTarget 계열 도형 메서드(Fill*/Draw*)는 HRESULT 를 반환하지 않고
    /// 내부적으로 무시하므로 `?` 를 쓰지 않는다. EndDraw/Present 만 결과를 검사한다.
    fn draw_scene(
        res: &OverlayResources,
        state: &OverlayDrawState,
        cfg: &OverlayConfig,
    ) -> windows::core::Result<()> {
        let bitmap = Self::bind_back_buffer(res)?;
        unsafe {
            res.d2d_context.SetTarget(&bitmap);
            res.d2d_context.BeginDraw();

            // 투명하게 클리어.
            res.d2d_context.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));

            // 범용 단색 브러시 — 색은 SetColor 로 매번 변경.
            let brush: ID2D1SolidColorBrush = res.d2d_context.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
                None,
            )?;

            // snap 미리보기 — 점선 사각형 외곽 + 반투명 채우기.
            // config.snap_preview 가 true 일 때만 그린다.
            // 색상 전환: active_sector 유무로 lock-on(RED) vs throw-target(BLUE) 구분.
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
                        brush.SetColor(&fill_color);
                        res.d2d_context.FillRectangle(&rect, &brush);
                        // 외곽선 (알파 0.95).
                        let mut stroke_color = base_color;
                        stroke_color.a = 0.95;
                        brush.SetColor(&stroke_color);
                        res.d2d_context
                            .DrawRectangle(&rect, &brush, 2.0, Some(&res.dash_style));
                    }
                }
            }

            res.d2d_context.EndDraw(None, None)?;
            let _ = res.swap_chain.Present(1, DXGI_PRESENT(0));
        }
        Ok(())
    }

    /// "#RRGGBB" (또는 "RRGGBB") 헥스 색상 → D2D1_COLOR_F (알파 1.0).
    /// 커서 알파는 별도(cursor_opacity)로 적용하고, snap preview 알파는 고정값을 사용.
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

/// 오버레이 창의 window proc.
///
/// WS_EX_NOREDIRECTIONBITMAP 창에서 WS_EX_TRANSPARENT 만으로는 hit-testing 이
/// 통과하지 않을 수 있다. WM_NCHITTEST 에 HTTRANSPARENT 를 반환하여 모든 마우스
/// 입력이 아래 창으로 통과하도록 보장한다.
unsafe extern "system" fn overlay_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // WM_NCHITTEST — 투명 처리. 마우스 입력이 아래 창으로 전달된다.
    if msg == WM_NCHITTEST {
        return LRESULT(HTTRANSPARENT as isize);
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
