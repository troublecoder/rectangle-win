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
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, MoveWindow, SetWindowPos, ShowWindow, HWND_TOP,
    SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SWP_NOZORDER, SWP_SHOWWINDOW,
};

use crate::application::errors::{ApplicationError, AppResult};
use crate::application::ports::WindowMover;
use crate::domain::geometry::{self, MonitorBounds};
use crate::domain::model::{SnapTarget, WindowAction};

/// Win32 user32 API 위에 구현한 [`WindowMover`].
///
/// 상태를 갖지 않는 얇은 어댑터 — 모든 호출이 즉시 Win32 로 전달된다.
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

/// `u64` 창 핸들 → Win32 `HWND` 로 변환.
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
            None
        } else {
            Some(hwnd.0 as usize as u64)
        }
    }

    fn apply_snap_target(
        &self,
        window_handle: u64,
        target: &SnapTarget,
        monitor: &MonitorBounds,
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
                let x = rect.origin.x;
                let y = rect.origin.y;
                let w = rect.size.width;
                let h = rect.size.height;
                // SAFETY: hwnd 는 호출자가 전달한 유효(로 가정된) 창 핸들.
                // SWP_NOZORDER 로 z-order 보존, SWP_SHOWWINDOW 로 창이 숨겨져 있으면 표시.
                unsafe {
                    SetWindowPos(
                        hwnd,
                        HWND_TOP,
                        x,
                        y,
                        w,
                        h,
                        SWP_NOZORDER | SWP_SHOWWINDOW,
                    )
                }
                .map_err(|e| ApplicationError::WindowOperation(e.to_string()))?;
                Ok(())
            }
            SnapTarget::Action { action, .. } => apply_action(hwnd, *action, monitor),
        }
    }

    fn get_window_rect(&self, window_handle: u64) -> AppResult<MonitorBounds> {
        let hwnd = hwnd_from_u64(window_handle);
        let mut rect = RECT::default();
        // SAFETY: hwnd 는 호출자가 전달한 유효(로 가정된) 창 핸들.
        // 출력 버퍼 rect 는 로컬 스택 변수로 충분히 초기화되어 있다.
        unsafe { GetWindowRect(hwnd, &mut rect) }
            .map_err(|e| ApplicationError::WindowOperation(e.to_string()))?;
        Ok(MonitorBounds::new(
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
        ))
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
