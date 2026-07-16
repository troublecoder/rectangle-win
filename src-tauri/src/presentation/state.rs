//! `AppState` — Tauri managed state 컨테이너.
//!
//! 도메인/애플리케이션 계층의 서비스와 인프라 어댑터를 묶어
//! `tauri::Builder::manage` 로 등록한다.
//! 각 `#[tauri::command]` 는 `State<'_, AppState>` 로 이를 참조한다.
//!
//! 플랫폼 의존성:
//! - Windows 타겟: `Win32WindowMover` / `Win32MonitorProvider` (실제 user32/gdi32)
//!   + `Win32LayeredOverlay` (DirectComposition) + `Win32InputListener` (RegisterHotKey)
//! - 기타 타겟: `MockWindowMover` / `MockMonitorProvider` (비-Windows 컴파일/CI용)

use std::sync::Arc;

use crate::application::keyboard_service::KeyboardService;
use crate::application::ports::{OverlayController, WindowMover};
// `MonitorProvider` trait은 비Windows cfg(not(windows)) 분기에서만 사용 —
// Windows 에서는 monitor_provider 필드가 구체 Win32MonitorProvider 타입이므로
// 이 위치에서 trait 을 직접 참조하지 않는다.
#[cfg(not(windows))]
use crate::application::ports::MonitorProvider;
use crate::application::snap_service::SnapService;
use crate::infrastructure::toml_config::TomlConfigStore;

/// 애플리케이션 전역 공유 상태.
///
/// 모든 필드는 불변이며, 가변 상태는 각 서비스/어댑터 내부의 Mutex 로 보호된다.
/// `TomlConfigStore` 만 명령에서 경로 조회를 위해 구체적 타입으로 보관한다.
///
/// `monitor_provider` 는 플랫폼에 따라 타입이 다르다:
/// - Windows: 구체 타입 `Arc<Win32MonitorProvider>` — 동일 인스턴스를
///   `Win32InputListener::start` 에 전달하고 `WM_DISPLAYCHANGE` 의
///   `invalidate()` 가 snap 경로에 닿도록 한다.
/// - 비Windows: trait object `Arc<dyn MonitorProvider>` (Mock).
///
/// `snap_service` / `keyboard_service` 는 입력 어댑터(`Win32InputListener`)와
/// 공유하기 위해 `Arc` 로 감싸 보관한다.
pub struct AppState {
    pub config_store: Arc<TomlConfigStore>,
    pub window_mover: Arc<dyn WindowMover>,
    #[cfg(windows)]
    pub monitor_provider: Arc<crate::infrastructure::win32_monitor::Win32MonitorProvider>,
    #[cfg(not(windows))]
    pub monitor_provider: Arc<dyn MonitorProvider>,
    pub overlay: Arc<dyn OverlayController>,
    pub snap_service: Arc<SnapService>,
    pub keyboard_service: Arc<KeyboardService>,
}

impl AppState {
    /// 모든 의존성을 조립해 새 상태를 생성한다.
    pub fn new() -> Self {
        let config_store = Arc::new(TomlConfigStore::default_path());

        #[cfg(windows)]
        let window_mover: Arc<dyn WindowMover> =
            Arc::new(crate::infrastructure::win32_window::Win32WindowMover::new());
        #[cfg(not(windows))]
        let window_mover: Arc<dyn WindowMover> =
            Arc::new(crate::application::mock::MockWindowMover::new());

        // 모니터 프로바이더 — Windows 에서는 구체 타입 인스턴스를 한 번만 생성하고
        // clone 하여 SnapService / KeyboardService / Win32InputListener 에 모두 전달.
        // 동일 인스턴스여야 invalidate() 가 snap 경로의 캐시에 영향을 준다.
        #[cfg(windows)]
        let monitor_provider: Arc<crate::infrastructure::win32_monitor::Win32MonitorProvider> =
            Arc::new(crate::infrastructure::win32_monitor::Win32MonitorProvider::new());
        #[cfg(not(windows))]
        let monitor_provider: Arc<dyn MonitorProvider> =
            Arc::new(crate::application::mock::MockMonitorProvider::default());

        // 오버레이 컨트롤러 — Windows 는 DirectComposition 기반 Win32LayeredOverlay,
        // 비Windows 는 TauriOverlay (emit 없이 상태만 보관, CI/컴파일용).
        #[cfg(windows)]
        let overlay: Arc<dyn OverlayController> = Arc::new(
            crate::infrastructure::win32_overlay::Win32LayeredOverlay::new(config_store.clone()),
        );
        #[cfg(not(windows))]
        let overlay: Arc<dyn OverlayController> =
            Arc::new(crate::infrastructure::overlay_window::TauriOverlay::new());

        let snap_service = Arc::new(SnapService::new(
            window_mover.clone(),
            monitor_provider.clone(),
            overlay.clone(),
            config_store.clone(),
        ));
        let keyboard_service = Arc::new(KeyboardService::new(
            window_mover.clone(),
            monitor_provider.clone(),
            config_store.clone(),
            overlay.clone(),
        ));

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

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
