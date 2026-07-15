//! [`OverlayController`] 의 Tauri/콜백 기반 구현체.
//!
//! 실제 Tauri `AppHandle` 의존 없이, 외부에서 주입한 emit 콜백을 통해
//! 프론트엔드(vue-konva 오버레이)로 상태 변경을 전달한다.
//! 이렇게 분리하면 인프라 계층을 Tauri 런타임 없이 단위 테스트 할 수 있다.
//!
//! [`OverlayController`]: crate::application::ports::OverlayController

use serde::Serialize;
use std::sync::Mutex;

use crate::application::errors::AppResult;
use crate::application::ports::OverlayController;

/// 오버레이 창에 전달되는 직렬화 가능한 뷰 모델.
///
/// `update_and_emit` 호출 시마다 emit 콜백으로 복제되어 전달된다.
#[derive(Debug, Clone, Serialize)]
pub struct OverlayState {
    pub visible: bool,
    pub center: Option<(i32, i32)>,
    pub sector_count: u8,
    pub cursor_pos: Option<(i32, i32)>,
    pub active_sector: Option<u8>,
    /// `(x, y, width, height)` 픽셀 단위.
    pub snap_preview: Option<(i32, i32, i32, i32)>,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            visible: false,
            center: None,
            sector_count: 8,
            cursor_pos: None,
            active_sector: None,
            snap_preview: None,
        }
    }
}

/// 오버레이 상태를 Tauri 프론트엔드로 전파하는 emit 콜백 타입.
type EmitFn = Box<dyn Fn(&OverlayState) + Send + Sync>;

/// Tauri 이벤트 emit 을 추상화한 콜백 기반 오버레이 컨트롤러.
///
/// `set_emitter` 로 주입된 클로저는 상태가 변경될 때마다 호출된다.
/// emitter 가 설정되지 않은 상태에서도 모든 동작은 정상 동작한다 (emit만 생략).
pub struct TauriOverlay {
    state: Mutex<OverlayState>,
    emit_fn: Mutex<Option<EmitFn>>,
}

impl TauriOverlay {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(OverlayState::default()),
            emit_fn: Mutex::new(None),
        }
    }

    /// 상태 변경 시 호출될 emit 콜백을 주입한다.
    /// Tauri `AppHandle` 이 available 한 시점에 주입하면 된다.
    pub fn set_emitter(&self, f: impl Fn(&OverlayState) + Send + Sync + 'static) {
        *self.emit_fn.lock().unwrap() = Some(Box::new(f));
    }

    /// 상태를 갱신하고 (변이 후) 현재 상태를 emitter 로 전달한다.
    /// emitter 잠금과 state 잠금을 중첩하지 않도록 두 단계로 분리했다.
    fn update_and_emit(&self, f: impl FnOnce(&mut OverlayState)) -> AppResult<()> {
        {
            let mut state = self.state.lock().unwrap();
            f(&mut state);
        }
        let state = self.state.lock().unwrap();
        if let Some(emit) = self.emit_fn.lock().unwrap().as_ref() {
            emit(&state);
        }
        Ok(())
    }
}

impl Default for TauriOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlayController for TauriOverlay {
    fn show_reticle(&self, center_x: i32, center_y: i32, sector_count: u8) -> AppResult<()> {
        self.update_and_emit(|s| {
            s.visible = true;
            s.center = Some((center_x, center_y));
            s.sector_count = sector_count;
            s.active_sector = None;
            s.snap_preview = None;
        })
    }

    fn update_cursor_indicator(&self, x: i32, y: i32) -> AppResult<()> {
        self.update_and_emit(|s| {
            s.cursor_pos = Some((x, y));
        })
    }

    fn highlight_sector(&self, sector: u8) -> AppResult<()> {
        self.update_and_emit(|s| {
            s.active_sector = Some(sector);
        })
    }

    fn show_snap_preview(&self, x: i32, y: i32, width: i32, height: i32) -> AppResult<()> {
        self.update_and_emit(|s| {
            s.snap_preview = Some((x, y, width, height));
        })
    }

    fn hide(&self) -> AppResult<()> {
        self.update_and_emit(|s| {
            s.visible = false;
            s.center = None;
            s.cursor_pos = None;
            s.active_sector = None;
            s.snap_preview = None;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    #[test]
    fn show_sets_visible_and_center() {
        let overlay = TauriOverlay::new();
        overlay.show_reticle(960, 540, 8).unwrap();
        let state = overlay.state.lock().unwrap();
        assert!(state.visible);
        assert_eq!(state.center, Some((960, 540)));
    }

    #[test]
    fn hide_clears_state() {
        let overlay = TauriOverlay::new();
        overlay.show_reticle(960, 540, 8).unwrap();
        overlay.update_cursor_indicator(100, 100).unwrap();
        overlay.hide().unwrap();
        let state = overlay.state.lock().unwrap();
        assert!(!state.visible);
        assert!(state.center.is_none());
        assert!(state.cursor_pos.is_none());
    }

    #[test]
    fn emit_called_on_update() {
        let overlay = TauriOverlay::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();
        overlay.set_emitter(move |_state| {
            called_clone.store(true, Ordering::SeqCst);
        });
        overlay.show_reticle(0, 0, 8).unwrap();
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn highlight_updates_sector() {
        let overlay = TauriOverlay::new();
        overlay.highlight_sector(3).unwrap();
        let state = overlay.state.lock().unwrap();
        assert_eq!(state.active_sector, Some(3));
    }

    #[test]
    fn snap_preview_state_round_trips() {
        let overlay = TauriOverlay::new();
        overlay.show_snap_preview(10, 20, 300, 400).unwrap();
        let state = overlay.state.lock().unwrap();
        assert_eq!(state.snap_preview, Some((10, 20, 300, 400)));
    }

    #[test]
    fn emit_fires_each_update() {
        let overlay = TauriOverlay::new();
        let count = Arc::new(AtomicU32::new(0));
        let count_clone = count.clone();
        overlay.set_emitter(move |_state| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });
        overlay.show_reticle(0, 0, 8).unwrap();
        overlay.highlight_sector(1).unwrap();
        overlay.update_cursor_indicator(2, 3).unwrap();
        overlay.hide().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 4);
    }
}
