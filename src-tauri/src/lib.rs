pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

use tauri::{Manager, WindowEvent};

pub fn run() {
    // Per-Monitor DPI-Aware V2 설정 — GetSystemMetrics/모니터 좌표가 물리 픽셀 기준으로
    // 일관되게 동작. 125%/150% 스케일 환경에서 오버레이 창 크기와 snap 좌표가 맞지 않는
    // 문제(DPI 스케일링 불일치)를 해결한다.
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::HiDpi::{
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
        };
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(presentation::state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            presentation::commands::get_config,
            presentation::commands::save_config,
            presentation::commands::get_config_path,
            presentation::commands::get_builtin_targets,
            presentation::commands::get_monitors,
            presentation::commands::test_snap_to_sector,
        ])
        .setup(|app| {
            // 시스템 트레이 설정 (메뉴 + 아이콘).
            presentation::tray::setup_tray(app)?;

            // 메인 창 닫기 요청을 가로채어 트레이로 숨긴다 (종료 아님).
            if let Some(main_window) = app.get_webview_window("main") {
                let win_clone = main_window.clone();
                main_window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });
            }

            // Win32 입력 리스너 시작 — RegisterHotKey(키보드 snap) +
            // GetAsyncKeyState 폴링(마우스 throw) + WM_DISPLAYCHANGE 모니터 감지.
            // AppState 의 구체 Win32MonitorProvider 인스턴스를 전달하여
            // 디스플레이 변경 시 invalidate() 가 snap 경로 캐시에 닿도록 한다.
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

            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.show().unwrap();
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
