pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

use tauri::{Emitter, Manager, WindowEvent};

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

            // 오버레이 emit 콜백 주입 — OverlayController 호출을 프론트엔드 이벤트로 변환.
            let state = app.state::<presentation::state::AppState>();
            let app_handle = app.handle().clone();
            state.overlay.set_emitter(move |overlay_state| {
                use presentation::events::OverlayEvent;
                let event = if !overlay_state.visible {
                    OverlayEvent::Hide
                } else if let Some((cx, cy)) = overlay_state.center {
                    OverlayEvent::Show {
                        center_x: cx,
                        center_y: cy,
                        sector_count: overlay_state.sector_count,
                    }
                } else {
                    OverlayEvent::Hide
                };
                let _ = app_handle.emit("overlay", event);
            });

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
