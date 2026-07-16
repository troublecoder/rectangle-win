pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

use tauri::{Manager, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(presentation::state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            presentation::commands::get_config,
            presentation::commands::save_config,
            presentation::commands::get_config_path,
            presentation::commands::apply_preset,
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

            // Win32 입력/오버레이 어댑터는 이후 태스크(Task 2/3+)에서
            // RegisterHotKey + DirectComposition 기반으로 다시 연결된다.
            // 현재 기준선에서는 SnapService / KeyboardService 가 AppState 에
            // 보관되지만 입력 리스너가 없으므로 snap 액션은 트리거되지 않는다.

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
