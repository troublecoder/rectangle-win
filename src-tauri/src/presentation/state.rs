//! `AppState` — Tauri managed state 컨테이너.
//!
//! 도메인/애플리케이션 계층의 서비스와 인프라 어댑터를 묶어
//! `tauri::Builder::manage` 로 등록한다.
//! 각 `#[tauri::command]` 는 `State<'_, AppState>` 로 이를 참조한다.
//!
//! 플랫폼 의존성:
//! - Windows 타겟: `Win32WindowMover` / `Win32MonitorProvider` (실제 user32/gdi32)
//! - 기타 타겟: `MockWindowMover` / `MockMonitorProvider` (비-Windows 컴파일/CI용)

use std::sync::Arc;

use crate::application::keyboard_service::KeyboardService;
use crate::application::ports::{MonitorProvider, OverlayController, WindowMover};
use crate::application::snap_service::SnapService;
use crate::infrastructure::overlay_window::TauriOverlay;
use crate::infrastructure::toml_config::TomlConfigStore;

/// 애플리케이션 전역 공유 상태.
///
/// 모든 필드는 불변이며, 가변 상태는 각 서비스/어댑터 내부의 Mutex 로 보호된다.
/// `TomlConfigStore` 만 명령에서 경로 조회를 위해 구체적 타입으로 보관하고,
/// 나머지 어댑터는 trait object (`Arc<dyn ...>`) 로 보관한다.
/// `snap_service` / `keyboard_service` 는 입력 어댑터와 공유하기 위해
/// `Arc` 로 감싸 보관한다 (현재 기준선에서는 입력 리스너가 없음).
pub struct AppState {
    pub config_store: Arc<TomlConfigStore>,
    pub window_mover: Arc<dyn WindowMover>,
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

        #[cfg(windows)]
        let monitor_provider: Arc<dyn MonitorProvider> =
            Arc::new(crate::infrastructure::win32_monitor::Win32MonitorProvider::new());
        #[cfg(not(windows))]
        let monitor_provider: Arc<dyn MonitorProvider> =
            Arc::new(crate::application::mock::MockMonitorProvider::default());

        // 임시: 기존 TauriOverlay 를 trait object 로 보관.
        // set_emitter 가 없으므로 오버레이 표시는 동작하지 않지만 snap 로직은 정상 작동.
        // Task 2 에서 Win32LayeredOverlay (DirectComposition) 로 교체 예정.
        let overlay: Arc<dyn OverlayController> = Arc::new(TauriOverlay::new());

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
