//! Win32 기반 [`MonitorProvider`] 구현체.
//!
//! windows-rs 0.58 바인딩을 통해 user32/gdi32 API 를 호출한다.
//!
//! - `EnumDisplayMonitors` + `MONITORENUMPROC` 콜백으로 모니터 열거
//! - `GetMonitorInfoW` 로 모니터 사각형(`rcMonitor`) 조회
//! - `MonitorFromPoint` 로 좌표 기반 모니터 탐색
//!
//! [`MonitorProvider`]: crate::application::ports::MonitorProvider

#![cfg(windows)]

use std::sync::Mutex;

use windows::Win32::Foundation::{BOOL, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromPoint, HMONITOR, MONITOR_DEFAULTTONEAREST,
    MONITORINFO,
};

use crate::application::ports::MonitorProvider;
use crate::domain::geometry::MonitorBounds;

/// Win32 API 위에 구현한 [`MonitorProvider`].
///
/// 상태를 갖지 않는 얇은 어댑터.
/// 단위 테스트는 실제 모니터 열거가 필요하므로 작성하지 않는다.
pub struct Win32MonitorProvider;

impl Win32MonitorProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Win32MonitorProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MonitorProvider for Win32MonitorProvider {
    fn enumerate(&self) -> Vec<MonitorBounds> {
        // 콜백으로 모니터를 수집할 공유 벡터. EnumDisplayMonitors 의 dwdata 로
        // 이 박스의 raw 포인터를 전달한다.
        let collected: Box<Mutex<Vec<MonitorBounds>>> = Box::new(Mutex::new(Vec::new()));
        let ptr = Box::into_raw(collected);

        // SAFETY:
        // - hdc=None 전체 가상 화면 열거, lprcclip=None 제한 없음
        // - dwdata 로 Box 의 raw 포인터를 전달 — 동기 호출이므로 수명은 안전
        // - ptr 은 아래 동일 함수에서 reclaim 되므로 leak 되지 않음
        let result = unsafe {
            EnumDisplayMonitors(
                None,
                None,
                Some(enum_proc),
                LPARAM(ptr as isize),
            )
        };

        // 콜백 실행이 끝났으므로 Box 를 되찾아 소유권을 복구한다.
        // SAFETY: ptr 은 위에서 Box::into_raw 로 만든 것이며, 단일 스레드 동기 호출이므로
        // 여기서 정확히 한 번 해제된다.
        let collected = unsafe { Box::from_raw(ptr) };
        let monitors = collected.lock().unwrap().clone();

        if result.as_bool() {
            monitors
        } else {
            // 열거 실패 — 빈 벡터 반환. 호출자가 적절히 폴백하도록 둔다.
            Vec::new()
        }
    }

    fn monitor_at(&self, x: i32, y: i32) -> MonitorBounds {
        let pt = POINT { x, y };
        // SAFETY: 인자가 단순 값/상수. 반환된 HMONITOR 로 GetMonitorInfoW 호출.
        let hmon = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST) };
        monitor_bounds_from_hmonitor(hmon).unwrap_or_else(|| {
            // GetMonitorInfoW 가 실패하는 극단적 케이스 — 주 디스플레이 1920x1080 폴백.
            // 더 정확한 폴백은 GetSystemMetrics(SM_CXSCREEN) 등이 가능하지만,
            // MonitorFromPoint + MONITOR_DEFAULTTONEAREST 가 항상 유효한 HMONITOR 를
            // 반환하므로 이 폴백은 사실상 도달 불가능하다.
            MonitorBounds::new(0, 0, 1920, 1080)
        })
    }
}

/// `EnumDisplayMonitors` 콜백. 각 모니터마다 호출되어 bounds 를 수집 벡터에 push.
///
/// 반환값이 `true` 여야 열거가 계속된다.
unsafe extern "system" fn enum_proc(
    hmon: HMONITOR,
    _hdc: windows::Win32::Graphics::Gdi::HDC,
    _lprc: *mut RECT,
    dwdata: LPARAM,
) -> BOOL {
    // dwdata 는 enumerate() 에서 전달한 Box<Mutex<Vec<MonitorBounds>>> 의 raw 포인터.
    let ptr = dwdata.0 as *mut Mutex<Vec<MonitorBounds>>;
    if ptr.is_null() {
        return BOOL(1);
    }
    // monitor_bounds_from_hmonitor 자체는 안전 함수 (내부에 unsafe 블록 포함).
    if let Some(b) = monitor_bounds_from_hmonitor(hmon) {
        // SAFETY: 호출자(enumerate)가 살아있는 Box 의 포인터를 보증. 동기 호출이므로 수명 안전.
        // Rust 2024 부터 unsafe fn 내부도 명시적 unsafe 블록이 필요하다.
        unsafe { (*ptr).lock().unwrap().push(b) };
    }
    // 열거 계속 = TRUE
    BOOL(1)
}

/// HMONITOR → `MonitorBounds` 변환. 실패 시 None.
fn monitor_bounds_from_hmonitor(hmon: HMONITOR) -> Option<MonitorBounds> {
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: hmon 이 유효한 모니터 핸들이어야 함. info 버퍼는 충분히 큼.
    let ok = unsafe { GetMonitorInfoW(hmon, &mut info) };
    if !ok.as_bool() {
        return None;
    }
    let rc = info.rcMonitor;
    Some(MonitorBounds::new(
        rc.left,
        rc.top,
        rc.right - rc.left,
        rc.bottom - rc.top,
    ))
}
