//! 시스템 트레이 아이콘과 컨텍스트 메뉴 설정.
//!
//! 메뉴 항목:
//! - 설정 (메인 창 표시)
//! - 일시정지 (예약: 입력 리스너 토글)
//! - 종료 (프로세스 종료)
//!
//! 트레이 아이콘 더블클릭 시에도 메인 창을 표시한다.

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    App, Manager,
};

/// 앱에 시스템 트레이를 설정한다. `.setup` 콜백에서 호출한다.
pub fn setup_tray(app: &App) -> tauri::Result<()> {
    let show_i = MenuItem::with_id(app, "show", "설정", true, None::<&str>)?;
    let pause_i = MenuItem::with_id(app, "pause", "일시정지", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_i, &pause_i, &sep, &quit_i])?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Rectangle Win")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
