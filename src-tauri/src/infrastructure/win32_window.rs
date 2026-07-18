//! Win32 기반 [`WindowMover`] 구현체.
//!
//! windows-rs 0.58 바인딩을 통해 user32 API 를 호출한다.
//!
//! - `GetForegroundWindow` 로 전경창 핸들 획득
//! - `GetWindowRect` 로 창 사각형 조회
//! - `SetWindowPos` / `MoveWindow` 로 창 이동/크기조절
//! - `ShowWindow` 로 최대화/최소화/복원 실행
//!
//! [`WindowMover`]: crate::application::ports::WindowMover

#![cfg(windows)]

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    GetAncestor, GetForegroundWindow, GetWindowLongW, GetWindowThreadProcessId,
    GetWindowRect, IsIconic, IsZoomed, MoveWindow, SetWindowPos, ShowWindow,
    WindowFromPoint, GA_ROOT, GWL_STYLE, HWND_TOP, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE,
    SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    WS_SIZEBOX,
};

use crate::application::errors::{ApplicationError, AppResult};
use crate::application::ports::WindowMover;
use crate::domain::geometry::{self, MonitorBounds};
use crate::domain::model::{SnapTarget, WindowAction};

/// Win32 user32 API 위에 구현한 [`WindowMover`].
///
/// 상태을 갖지 않는 얇은 어댑터 — 모든 호출이 즉시 Win32 로 전달된다.
/// 단위 테스트는 실제 창 조작이 필요하므로 작성하지 않는다.
pub struct Win32WindowMover;

impl Win32WindowMover {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Win32WindowMover {
    fn default() -> Self {
        Self::new()
    }
}

/// 창 핸들이 우리 프로세스에 속하는지(즉, Rectangle Win 자체 창인지) 검사.
///
/// throw modifier 활성화 중 오버레이/설정창이 foreground 가 될 수 있으므로,
/// snap 대상에서 우리 앱 창을 제외하기 위해 사용한다.
fn is_own_window(hwnd: HWND) -> bool {
    // SAFETY: GetCurrentProcessId / GetWindowThreadProcessId 는 읽기 전용 조회.
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut u32));
        pid == GetCurrentProcessId()
    }
}

/// DWM 그림자를 제외한 실제 창 영역 가져오기.
///
/// `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS)` 는 DWM 이 그리는
/// 보이지 않는 그림자/테두리를 제외한 실제 창 사각형을 반환한다.
/// `GetWindowRect` 는 그림자를 포함하여 오버레이가 실제 창보다 크게 표시된다.
fn dwm_window_rect(hwnd: HWND) -> AppResult<RECT> {
    let mut rect = RECT::default();
    // SAFETY: hwnd 는 유효한 창 핸들. rect 는 로컬 스택 버퍼.
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        )
    }
    .map_err(|e| ApplicationError::WindowOperation(format!("DwmGetWindowAttribute: {e}")))?;
    Ok(rect)
}

/// FancyZones 의 `AdjustRectForSizeWindowToRect` 포팅.
///
/// zone rect(스냅 영역)를 받아, 창의 DWM 보이지 않는 테두리 보정을 적용해
/// 확장한다. `GetWindowRect`(그림자 포함)와 `DWMWA_EXTENDED_FRAME_BOUNDS`
/// (실제 보이는 프레임)의 차이를 좌우하단 마진으로 계산해 zone rect를
/// 바깥으로 밀어낸다. 이렇게 하지 않으면 보이는 창이 zone보다 작아진다.
///
/// 크기 조절 불가능한 창(WS_SIZEBOX 없음)은 원래 크기를 유지한다.
fn adjust_rect_for_border(hwnd: HWND, zone_rect: RECT) -> RECT {
    let mut new_rect = zone_rect;
    // SAFETY: hwnd는 유효한 창 핸들. 버퍼는 스택.
    unsafe {
        let mut win_rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut win_rect);

        if let Ok(frame) = dwm_window_rect(hwnd) {
            // frame - win_rect 차이가 보이지 않는 테두리 폭.
            // left는 보통 음수(그림자가 왼쪽으로 확장), right/bottom은 양수.
            let left_margin = frame.left - win_rect.left;
            let right_margin = frame.right - win_rect.right;
            let bottom_margin = frame.bottom - win_rect.bottom;
            new_rect.left = new_rect.left.saturating_sub(left_margin.max(0));
            new_rect.right = new_rect.right.saturating_sub(right_margin.max(0));
            new_rect.bottom = new_rect.bottom.saturating_sub(bottom_margin.max(0));
            // 음수 마진(그림자가 바깥으로 확장)이면 rect를 바깥으로 확장.
            if left_margin < 0 {
                new_rect.left = new_rect.left.saturating_add((-left_margin) as i32);
            }
            if right_margin < 0 {
                new_rect.right = new_rect.right.saturating_add((-right_margin) as i32);
            }
            if bottom_margin < 0 {
                new_rect.bottom = new_rect.bottom.saturating_add((-bottom_margin) as i32);
            }
        }

        // 크기 조절 불가능한 창은 크기를 그대로 유지(이동만).
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        if style & WS_SIZEBOX.0 == 0 {
            let w = win_rect.right - win_rect.left;
            let h = win_rect.bottom - win_rect.top;
            new_rect.right = new_rect.left + w;
            new_rect.bottom = new_rect.top + h;
        }
    }
    new_rect
}

/// 창을 지정된 rect로 이동/크기 조절. SetWindowPos 1회 호출 — 빠르고 즉각적.
/// HWND_TOP으로 z-order 최상위에 배치 (SWP_NOZORDER 사용 안 함 — 충돌).
fn size_window_to_rect(hwnd: HWND, rect: RECT) -> AppResult<()> {
    // SAFETY: hwnd는 유효한 창 핸들.
    unsafe {
        if IsZoomed(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        SetWindowPos(
            hwnd,
            HWND_TOP,
            rect.left,
            rect.top,
            w,
            h,
            SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        )
        .map_err(|e| ApplicationError::WindowOperation(format!("SetWindowPos: {e}")))?;
    }
    Ok(())
}
///
/// `HWND` 는 windows-rs 0.58 에서 `HWND(*mut c_void)` 이며,
/// 포인터 크기는 플랫폼에 따라 64비트이므로 `u64 → usize → *mut _` 경로로 안전하게 변환한다.
fn hwnd_from_u64(handle: u64) -> HWND {
    HWND(handle as usize as *mut _)
}

impl WindowMover for Win32WindowMover {
    fn get_foreground_window(&self) -> Option<u64> {
        // SAFETY: GetForegroundWindow 는 인자 없이 단순히 현재 전경창을 반환하는 안전한 API.
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_invalid() {
            return None;
        }
        // 우리 앱 창(설정/오버레이)은 snap 대상에서 제외.
        if is_own_window(hwnd) {
            return None;
        }
        Some(hwnd.0 as usize as u64)
    }

    fn window_at_cursor(&self, x: i32, y: i32) -> Option<u64> {
        // SAFETY: WindowFromPoint는 읽기 전용 조회. POINT는 값 전달.
        // WindowFromPoint는 가장 위에 있는 창(자식 컨트롤 포함)을 반환하므로,
        // GetAncestor(GA_ROOT)로 최상위 창을 얻는다.
        let hwnd = unsafe {
            let raw = WindowFromPoint(windows::Win32::Foundation::POINT { x, y });
            if raw.is_invalid() {
                return None;
            }
            // 자식 창 → 최상위 부모 창. GetAncestor는 실패 시 invalid HWND 반환.
            let root = GetAncestor(raw, GA_ROOT);
            if root.is_invalid() { raw } else { root }
        };
        if hwnd.is_invalid() {
            return None;
        }
        // 우리 앱 창(설정/오버레이)은 snap 대상에서 제외.
        if is_own_window(hwnd) {
            return None;
        }
        Some(hwnd.0 as usize as u64)
    }

    fn apply_snap_target(
        &self,
        window_handle: u64,
        target: &SnapTarget,
        monitor: &MonitorBounds,
        margin: i32,
    ) -> AppResult<()> {
        let hwnd = hwnd_from_u64(window_handle);

        match target {
            SnapTarget::Area {
                x_ratio,
                y_ratio,
                w_ratio,
                h_ratio,
                ..
            } => {
                let rect = geometry::ratio_to_pixels(*x_ratio, *y_ratio, *w_ratio, *h_ratio, monitor);
                // snap_margin 적용 — 각 변을 안쪽으로 margin 픽셀 축소.
                let rect = geometry::apply_margin(rect, margin);
                // zone rect → Win32 RECT (가상 화면 절대 좌표).
                let zone_rect = RECT {
                    left: rect.origin.x,
                    top: rect.origin.y,
                    right: rect.origin.x + rect.size.width,
                    bottom: rect.origin.y + rect.size.height,
                };
                // FancyZones 방식: DWM 테두리 보정 후 SetWindowPlacement 로 배치.
                // 보이지 않는 DWM 테두리(우측/하단 약 7-8px)만큼 zone rect를 바깥으로
                // 확장하지 않으면, 보이는 창이 zone보다 작아진다.
                let adjusted = adjust_rect_for_border(hwnd, zone_rect);
                size_window_to_rect(hwnd, adjusted)?;
                Ok(())
            }
            SnapTarget::Action { action, .. } => apply_action(hwnd, *action, monitor),
        }
    }

    fn get_window_rect(&self, window_handle: u64) -> AppResult<MonitorBounds> {
        let hwnd = hwnd_from_u64(window_handle);
        // DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS) 로 DWM 그림자를
        // 제외한 실제 창 영역을 가져온다. GetWindowRect 는 그림자를 포함하여
        // 오버레이가 실제 창보다 크게 표시되는 문제가 있다.
        // DWM API 실패 시 GetWindowRect 로 폴백.
        let rect: RECT = dwm_window_rect(hwnd).unwrap_or_else(|_| {
            let mut r = RECT::default();
            // SAFETY: hwnd 는 유효(로 가정된) 창 핸들.
            let _ = unsafe { GetWindowRect(hwnd, &mut r) };
            r
        });
        Ok(MonitorBounds::new(
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
        ))
    }

    fn bring_to_foreground(&self, window_handle: u64) {
        let hwnd = hwnd_from_u64(window_handle);
        // z-order만 HWND_TOP으로 변경 — 크기/위치는 그대로.
        // SWP_NOSIZE | SWP_NOMOVE로 0,0,0,0 무시. SWP_NOACTIVATE로 포커스 효과 최소화.
        // SAFETY: hwnd는 유효한 창 핸들.
        unsafe {
            let _ = SetWindowPos(
                hwnd, HWND_TOP, 0, 0, 0, 0,
                SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE,
            );
        }
    }
}

/// `WindowAction` 을 해당 Win32 호출로 실행.
fn apply_action(hwnd: HWND, action: WindowAction, monitor: &MonitorBounds) -> AppResult<()> {
    match action {
        WindowAction::Maximize => show_window(hwnd, SW_MAXIMIZE),
        WindowAction::Minimize => show_window(hwnd, SW_MINIMIZE),
        WindowAction::Restore => show_window(hwnd, SW_RESTORE),
        WindowAction::Center => {
            let cur = current_window_size(hwnd)?;
            let x = monitor.origin.x + (monitor.width() - cur.0).max(0) / 2;
            let y = monitor.origin.y + (monitor.height() - cur.1).max(0) / 2;
            move_window(hwnd, x, y, cur.0, cur.1)
        }
        WindowAction::AlmostMaximize => {
            // 모니터의 90% 크기로 중앙 배치
            let w = (monitor.width() as f64 * 0.9) as i32;
            let h = (monitor.height() as f64 * 0.9) as i32;
            let x = monitor.origin.x + (monitor.width() - w) / 2;
            let y = monitor.origin.y + (monitor.height() - h) / 2;
            move_window(hwnd, x, y, w, h)
        }
        WindowAction::MaximizeHeight => {
            // 현재 x/width 는 유지, y/height 만 모니터 전체 높이로
            let cur = current_window_size(hwnd)?;
            let x = cur.2;
            move_window(hwnd, x, monitor.origin.y, cur.0, monitor.height())
        }
        // 다중 모니터 이동은 별도 태스크에서 구현 — 현재는 no-op.
        WindowAction::NextDisplay | WindowAction::PreviousDisplay => Ok(()),
    }
}

/// `ShowWindow` 래퍼. windows-rs 0.58 의 ShowWindow 는 `BOOL` 을 반환하므로
/// (Result 가 아님) 실패 의미 판단은 단순히 false 여부로만 한다.
fn show_window(hwnd: HWND, cmd: windows::Win32::UI::WindowsAndMessaging::SHOW_WINDOW_CMD) -> AppResult<()> {
    // SAFETY: hwnd 가 유효한 창 핸들이어야 함. cmd 는 상수값.
    let ok = unsafe { ShowWindow(hwnd, cmd) };
    // ShowWindow 의 반환값은 "이전에 창이 보였는가" 이지 성공/실패가 아니다.
    // 따라서 false 라도 에러로 취급하지 않는다. 여기서는 호출 자체가 성공했다고 간주.
    let _ = ok;
    Ok(())
}

/// `MoveWindow` 래퍼. `brepaint = true` 로 항상 다시 그린다.
fn move_window(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> AppResult<()> {
    // SAFETY: hwnd 가 유효한 창 핸들이어야 함.
    unsafe { MoveWindow(hwnd, x, y, w, h, true) }
        .map_err(|e| ApplicationError::WindowOperation(e.to_string()))?;
    Ok(())
}

/// 현재 창의 (width, height, x) 좌표 반환. `MaximizeHeight` 용도로 x/width 가 필요하다.
/// 반환: `(width, height, x)`
fn current_window_size(hwnd: HWND) -> AppResult<(i32, i32, i32)> {
    let mut rect = RECT::default();
    // SAFETY: hwnd 가 유효한 창 핸들이어야 함.
    unsafe { GetWindowRect(hwnd, &mut rect) }
        .map_err(|e| ApplicationError::WindowOperation(e.to_string()))?;
    Ok((rect.right - rect.left, rect.bottom - rect.top, rect.left))
}
