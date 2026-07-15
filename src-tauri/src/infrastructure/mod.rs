//! 인프라 계층 — 외부 시스템과의 어댑터 구현체.
//!
//! 크로스 플랫폼 순수 로직 (현재 구현됨):
//! - [`toml_config`] — `ConfigStore` 의 TOML 구현체
//! - [`overlay_window`] — `OverlayController` 의 Tauri/콜백 기반 구현체
//! - [`input_dispatcher`] — 입력 이벤트 분류/라우팅 순수 로직
//!
//! Win32 전용 모듈(`win32_window`, `win32_monitor`)은 Task 2+3에서 추가된다.

pub mod toml_config;
pub mod overlay_window;
pub mod input_dispatcher;
