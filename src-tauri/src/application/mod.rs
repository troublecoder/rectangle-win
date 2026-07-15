pub mod errors;
pub mod ports;
pub mod snap_service;
pub mod keyboard_service;

// mock 은 단위 테스트에서 사용되며, 비-Windows 타겟의 presentation/state.rs
// (실제 인프라가 없는 환경) 에서도 컴파일되어야 한다.
#[cfg(any(test, not(windows)))]
pub mod mock;
