//! 프레젠테이션 계층 — Tauri 명령, 상태, 이벤트, 시스템 트레이.
//!
//! - [`state`] — `AppState` (서비스 의존성 컨테이너)
//! - [`commands`] — 프론트엔드에 노출되는 `#[tauri::command]` 진입점
//! - [`tray`] — 시스템 트레이 아이콘 및 컨텍스트 메뉴

pub mod commands;
pub mod state;
pub mod tray;
