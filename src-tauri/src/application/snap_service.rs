//! SnapService — 커서 드래그(throw) 기반 스냅 오케스트레이션.
//!
//! 도메인의 `CursorFsm`(statig 상태머신)이 검증한 전이 규칙을 그대로 모방하되,
//! statig가 생성하는 복잡한 제네릭 타입을 서비스에서 직접 들고 있지 않도록
//! 단순한 상태 enum과 `CursorFsm` 공유 저장소(struct)만 사용한다.
//!
//! 상태 전이 로직은 `domain::cursor_fsm`의 핸들러와 동일하게 동작한다:
//!   Idle   --ModifierPressed-->  Armed
//!   Armed  --MouseMoved------->  Tracking (섹터/거리 계산)
//!   Armed  --ModifierReleased->  Idle (snap 없음)
//!   Tracking --ModifierReleased{cancel:false}--> Snapping -> Idle (snap 실행)
//!   Tracking --ModifierReleased{cancel:true}-->  Idle (취소, snap 없음)

use std::sync::Arc;

use parking_lot::Mutex;

use crate::application::errors::{ApplicationError, AppResult};
use crate::application::ports::{ConfigStore, MonitorProvider, OverlayController, WindowMover};
use crate::domain::cursor_fsm::CursorFsm;
use crate::domain::geometry;

/// 서비스가 추적하는 논리 상태. `cursor_fsm::State` 와 1:1 대응.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnapState {
    Idle,
    Armed,
    Tracking,
}

/// SnapService의 가변 상태. Mutex로 보호된다.
#[derive(Debug)]
struct SnapInner {
    state: SnapState,
    /// 도메인 FSM의 공유 저장소를 데이터 홀더로 재사용.
    /// `current_sector` / `throw_distance` 만 사용한다.
    fsm: CursorFsm,
}

impl Default for SnapInner {
    fn default() -> Self {
        Self {
            state: SnapState::Idle,
            fsm: CursorFsm::default(),
        }
    }
}

/// 커서 기반 스냅 서비스.
///
/// 입력 어댑터(modifier 눌림/뗌, 마우스 이동)가 호출하는 3개의 진입점을 제공한다.
/// 내부 상태는 `parking_lot::Mutex` 로 보호되며, 오버레이 표시/스냅 실행은
/// 주입된 port trait 들을 통해 수행된다.
pub struct SnapService {
    window_mover: Arc<dyn WindowMover>,
    monitor_provider: Arc<dyn MonitorProvider>,
    overlay: Arc<dyn OverlayController>,
    config_store: Arc<dyn ConfigStore>,
    inner: Mutex<SnapInner>,
}

impl SnapService {
    pub fn new(
        window_mover: Arc<dyn WindowMover>,
        monitor_provider: Arc<dyn MonitorProvider>,
        overlay: Arc<dyn OverlayController>,
        config_store: Arc<dyn ConfigStore>,
    ) -> Self {
        Self {
            window_mover,
            monitor_provider,
            overlay,
            config_store,
            inner: Mutex::new(SnapInner::default()),
        }
    }

    /// Modifier 키 눌림 — 오버레이 표시 후 Armed 상태로 전이.
    ///
    /// Lock-on 표시: 현재 전경창의 위치를 snap_preview 사각형(RED, cursor_color)으로
    /// 표시한다. `show_reticle` 은 오버레이 창을 visible 상태로 만드는 트리거 역할만 하고
    /// 점은 그리지 않는다. `show_snap_preview` 가 active_sector=None 상태로 호출되므로
    /// draw_scene 은 cursor_color (빨강)로 렌더링한다.
    pub fn on_modifier_pressed(&self, cursor_x: i32, cursor_y: i32) -> AppResult<()> {
        let config = self.config_store.load()?;
        let sector_count = config.overlay.sector_count;

        let monitor = self.monitor_provider.monitor_at_cursor(cursor_x, cursor_y);
        let center = monitor.center();
        // show_reticle: overlay 창 visible + active_sector/snap_preview 초기화.
        // 더 이상 커서 점을 그리지 않는다 (win32_overlay draw_scene 참조).
        self.overlay
            .show_reticle(center.x, center.y, sector_count)?;

        // Lock-on: 현재 전경창의 rect 를 snap_preview 로 표시.
        // active_sector 가 None 인 상태이므로 RED(cursor_color)로 그려진다.
        if let Some(handle) = self.window_mover.get_foreground_window() {
            if let Ok(rect) = self.window_mover.get_window_rect(handle) {
                self.overlay
                    .show_snap_preview(rect.origin.x, rect.origin.y, rect.size.width, rect.size.height)?;
            }
        }

        let mut inner = self.inner.lock();
        inner.state = SnapState::Armed;
        // Armed 진입 시 섹터/거리는 초기화된 상태여야 한다.
        inner.fsm = CursorFsm::default();
        Ok(())
    }

    /// 마우스 이동 — Tracking 상태에서 섹터/거리를 갱신하고 오버레이에 반영.
    /// Armed 상태에서 첫 이동 시 Tracking 으로 전이하며 초기 섹터를 계산한다.
    /// Idle 상태에서는 무시한다(FSM의 idle 핸들러와 동일).
    ///
    /// 임계값(MIN_THROW_DISTANCE) 이하의 이동에서는 throw 로 간주하지 않고
    /// lock-on(현재 창 RED 사각형) 상태를 유지한다. 임계값 이상에서만
    /// highlight_sector + show_snap_preview(TARGET 영역, BLUE) 로 전환한다.
    pub fn on_mouse_moved(
        &self,
        cursor_x: i32,
        cursor_y: i32,
        delta_x: f64,
        delta_y: f64,
    ) -> AppResult<()> {
        // throw 로 간주하기 위한 최소 이동 거리 (픽셀).
        // 이 값 미만에서는 lock-on(현재 창 RED 사각형)을 유지.
        const MIN_THROW_DISTANCE: f64 = 8.0;

        let config = self.config_store.load()?;
        let sector_count = config.overlay.sector_count;

        let compute_sector = |dx: f64, dy: f64| {
            geometry::compute_sector(euclid::Vector2D::new(dx, dy), sector_count)
        };

        let mut inner = self.inner.lock();
        match inner.state {
            SnapState::Idle => {
                // Idle에서는 이벤트 무시 (FSM과 동일)
            }
            SnapState::Armed => {
                // Armed -> Tracking 전이. 첫 이동 이벤트에서 즉시 섹터/거리 계산.
                inner.fsm.current_sector = Some(compute_sector(delta_x, delta_y));
                inner.fsm.throw_distance = geometry::throw_distance(euclid::Vector2D::new(
                    delta_x,
                    delta_y,
                ));
                inner.state = SnapState::Tracking;
            }
            SnapState::Tracking => {
                inner.fsm.current_sector = Some(compute_sector(delta_x, delta_y));
                inner.fsm.throw_distance = geometry::throw_distance(euclid::Vector2D::new(
                    delta_x,
                    delta_y,
                ));
            }
        }

        // Tracking 상태에서만 오버레이 갱신. 섹터값을 복사한 뒤 잠금을 풀고 overlay 호출.
        // 임계값 미만의 throw_distance 에서는 lock-on 상태 유지 (preview 갱신 안 함).
        let sector_to_highlight = match inner.state {
            SnapState::Tracking if inner.fsm.throw_distance >= MIN_THROW_DISTANCE => {
                inner.fsm.current_sector
            }
            _ => None,
        };
        if let Some(sector) = sector_to_highlight {
            // throw target 표시: highlight_sector 가 active_sector 를 Some 으로 만들어
            // draw_scene 이 sector_highlight_color (BLUE)로 snap_preview 를 그리도록 함.
            self.overlay.highlight_sector(sector)?;
            // snap 미리보기 — 해당 sector 에 매핑된 SnapTarget 의 픽셀 영역을 표시.
            if let Ok(preview) = self.compute_snap_preview(sector, cursor_x, cursor_y) {
                if let Some((x, y, w, h)) = preview {
                    self.overlay.show_snap_preview(x, y, w, h)?;
                }
            }
        }
        Ok(())
    }

    /// Modifier 키 뗌 — 오버레이 숨김, cancel=false 이고 섹터가 있으면 snap 실행.
    pub fn on_modifier_released(
        &self,
        cancel: bool,
        cursor_x: i32,
        cursor_y: i32,
    ) -> AppResult<()> {
        // 상태와 섹터를 임계구역 안에서 읽어온 뒤 즉시 Idle로 전이.
        let (prev_state, sector, throw_distance) = {
            let mut inner = self.inner.lock();
            let prev = inner.state;
            let sec = if cancel { None } else { inner.fsm.current_sector };
            let dist = inner.fsm.throw_distance;
            // Snapping -> Idle 전이와 동일. enter_idle 정리.
            inner.state = SnapState::Idle;
            inner.fsm = CursorFsm::default();
            (prev, sec, dist)
        };

        // 오버레이는 항상 숨김.
        self.overlay.hide()?;

        // Armed 상태에서의 release(이동 없음) 또는 Idle/취소인 경우 snap 없음.
        if cancel || prev_state != SnapState::Tracking {
            return Ok(());
        }

        let sector = match sector {
            Some(s) => s,
            None => return Ok(()),
        };

        let config = self.config_store.load()?;
        let monitor = self.monitor_provider.monitor_at_cursor(cursor_x, cursor_y);

        // Long Throw 임계값 판별 — 거리가 임계값 이상이면 long_throw_mapping 사용.
        let long_throw_enabled = config.throw.long_throw_enabled;
        let long_throw_distance = config.throw.long_throw_distance as f64;
        let mapping = if long_throw_enabled && throw_distance >= long_throw_distance {
            &config.throw.long_throw_mapping
        } else {
            &config.throw.mapping
        };

        let target_id = match mapping.get(&sector) {
            Some(id) => id.clone(),
            None => return Ok(()), // 매핑 없음 — snap 없이 종료
        };

        let target = config
            .snap
            .areas
            .iter()
            .find(|t| t.id() == target_id)
            .ok_or_else(|| ApplicationError::Domain(
                crate::domain::errors::DomainError::TargetNotFound(target_id.clone()),
            ))?;

        let window = self
            .window_mover
            .get_foreground_window()
            .ok_or(ApplicationError::NoForegroundWindow)?;

        self.window_mover.apply_snap_target(window, target, &monitor)?;
        Ok(())
    }

    /// 강제로 Idle로 리셋 (포커스 변경 등 외부 트리거용).
    pub fn reset(&self) {
        let mut inner = self.inner.lock();
        inner.state = SnapState::Idle;
        inner.fsm = CursorFsm::default();
    }

    /// 현재 섹터(테스트/디버그용).

    /// 주어진 sector 에 매핑된 SnapTarget 의 픽셀 영역을 계산 (미리보기용).
    /// Area 타입만 미리보기 가능 — Action 타입은 None 반환.
    fn compute_snap_preview(&self, sector: u8, cursor_x: i32, cursor_y: i32) -> AppResult<Option<(i32, i32, i32, i32)>> {
        let config = self.config_store.load()?;
        let mapping = &config.throw.mapping;
        let target_id = match mapping.get(&sector) {
            Some(id) => id,
            None => return Ok(None),
        };
        let target = config
            .snap
            .areas
            .iter()
            .find(|t| t.id() == target_id.as_str());
        let target = match target {
            Some(t) => t,
            None => return Ok(None),
        };
        // Area 타입만 좌표 계산 가능.
        if let crate::domain::model::SnapTarget::Area { x_ratio, y_ratio, w_ratio, h_ratio, .. } =
            target
        {
            let monitor = self.monitor_provider.monitor_at_cursor(cursor_x, cursor_y);
            let rect = geometry::ratio_to_pixels(*x_ratio, *y_ratio, *w_ratio, *h_ratio, &monitor);
            Ok(Some((rect.origin.x, rect.origin.y, rect.size.width, rect.size.height)))
        } else {
            Ok(None)
        }
    }

    /// 현재 섹터(테스트/디버그용).
    #[cfg(test)]
    pub(crate) fn current_sector(&self) -> Option<u8> {
        self.inner.lock().fsm.current_sector
    }

    /// 현재 논리 상태(테스트용).
    #[cfg(test)]
    pub(crate) fn state(&self) -> &'static str {
        match self.inner.lock().state {
            SnapState::Idle => "idle",
            SnapState::Armed => "armed",
            SnapState::Tracking => "tracking",
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// 테스트
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::mock::{
        MockConfigStore, MockMonitorProvider, MockOverlayController, MockWindowMover,
    };
    use crate::domain::model::{SnapTarget, ThrowConfig};
    use std::collections::HashMap;

    /// 테스트용 SnapService 구성.
    /// 섹터 0(오른쪽) -> "right-half", 섹터 4(왼쪽) -> "left-half" 매핑.
    fn make_service() -> (
        SnapService,
        Arc<MockWindowMover>,
        Arc<MockMonitorProvider>,
        Arc<MockOverlayController>,
        Arc<MockConfigStore>,
    ) {
        let window_mover = Arc::new(MockWindowMover::new());
        window_mover.set_foreground(1001);
        let monitor_provider = Arc::new(MockMonitorProvider::default());
        let overlay = Arc::new(MockOverlayController::default());
        let config_store = Arc::new(MockConfigStore::default());

        // 기본 매핑: 0 -> right-half, 4 -> left-half
        let mut mapping = crate::domain::model::SectorMap::new();
        mapping.insert(0u8, "right-half".to_string());
        mapping.insert(4u8, "left-half".to_string());
        let mut long_mapping = crate::domain::model::SectorMap::new();
        long_mapping.insert(0u8, "maximize".to_string());

        {
            let mut cfg = config_store.config.lock().unwrap();
            cfg.throw = ThrowConfig {
                trigger_modifiers: vec!["Win".to_string()],
                long_throw_enabled: true,
                long_throw_distance: 400,
                mapping,
                long_throw_mapping: long_mapping,
            };
            // areas에 right-half, left-half, maximize 추가
            cfg.snap.areas = vec![
                SnapTarget::Area {
                    id: "right-half".to_string(),
                    name: "Right Half".to_string(),
                    x_ratio: 0.5,
                    y_ratio: 0.0,
                    w_ratio: 0.5,
                    h_ratio: 1.0,
                },
                SnapTarget::Area {
                    id: "left-half".to_string(),
                    name: "Left Half".to_string(),
                    x_ratio: 0.0,
                    y_ratio: 0.0,
                    w_ratio: 0.5,
                    h_ratio: 1.0,
                },
                SnapTarget::Action {
                    id: "maximize".to_string(),
                    name: "Maximize".to_string(),
                    action: crate::domain::model::WindowAction::Maximize,
                },
            ];
        }

        let service = SnapService::new(
            window_mover.clone(),
            monitor_provider.clone(),
            overlay.clone(),
            config_store.clone(),
        );

        (service, window_mover, monitor_provider, overlay, config_store)
    }

    #[test]
    fn modifier_press_shows_overlay() {
        let (service, _w, _m, overlay, _c) = make_service();
        // 기본 visible=false 확인
        assert!(!*overlay.visible.lock().unwrap());

        service.on_modifier_pressed(100, 100).unwrap();
        assert!(*overlay.visible.lock().unwrap());
        assert_eq!(service.state(), "armed");
    }

    #[test]
    fn modifier_release_hides_overlay() {
        let (service, _w, _m, overlay, _c) = make_service();
        service.on_modifier_pressed(100, 100).unwrap();
        assert!(*overlay.visible.lock().unwrap());

        service.on_modifier_released(false, 100, 100).unwrap();
        assert!(!*overlay.visible.lock().unwrap());
    }

    #[test]
    fn throw_right_snaps_to_right_half() {
        // 섹터 0 = 오른쪽 -> 매핑 "right-half"
        let (service, window_mover, _m, _o, _c) = make_service();
        service.on_modifier_pressed(960, 540).unwrap();
        // 오른쪽으로 이동 (delta_x > 0, delta_y = 0) -> 섹터 0
        service.on_mouse_moved(1060, 540, 100.0, 0.0).unwrap();
        assert_eq!(service.current_sector(), Some(0));
        assert_eq!(service.state(), "tracking");

        service.on_modifier_released(false, 1060, 540).unwrap();

        // on_modifier_pressed 가 lock-on 표시를 위해 GetRect 를 호출하므로
        // ApplySnap 호출만 필터링하여 검증.
        let calls = window_mover.apply_snap_calls();
        assert_eq!(calls.len(), 1);
        match &calls[0] {
            crate::application::mock::MockWindowCall::ApplySnap {
                window,
                target_id,
                is_action,
            } => {
                assert_eq!(*window, 1001);
                assert_eq!(target_id, "right-half");
                assert!(!*is_action);
            }
            other => panic!("expected ApplySnap, got {:?}", other),
        }
        assert_eq!(service.state(), "idle");
    }

    #[test]
    fn cancel_does_not_snap() {
        let (service, window_mover, _m, _o, _c) = make_service();
        service.on_modifier_pressed(960, 540).unwrap();
        service.on_mouse_moved(1060, 540, 100.0, 0.0).unwrap();
        assert_eq!(service.current_sector(), Some(0));

        // cancel=true -> snap 실행 안 함
        service.on_modifier_released(true, 1060, 540).unwrap();

        assert!(window_mover.apply_snap_calls().is_empty());
        assert_eq!(service.state(), "idle");
        assert!(service.current_sector().is_none());
    }

    #[test]
    fn release_without_move_does_not_snap() {
        // Armed 상태에서 이동 없이 release -> snap 없음
        let (service, window_mover, _m, _o, _c) = make_service();
        service.on_modifier_pressed(960, 540).unwrap();
        assert_eq!(service.state(), "armed");

        service.on_modifier_released(false, 960, 540).unwrap();

        assert!(window_mover.apply_snap_calls().is_empty());
        assert_eq!(service.state(), "idle");
    }

    #[test]
    fn long_throw_uses_long_throw_mapping() {
        // 거리 500 >= 400 임계값 -> long_throw_mapping[0] = "maximize"
        let (service, window_mover, _m, _o, _c) = make_service();
        service.on_modifier_pressed(960, 540).unwrap();
        // (300, 400) 이동 -> 거리 500, 섹터 1(오른쪽아래)... 하지만 long_mapping은 섹터 0만.
        // 섹터 0이 되도록 순수 오른쪽 이동: (500, 0) -> 거리 500, 섹터 0
        service.on_mouse_moved(1460, 540, 500.0, 0.0).unwrap();
        assert_eq!(service.current_sector(), Some(0));

        service.on_modifier_released(false, 1460, 540).unwrap();

        let calls = window_mover.apply_snap_calls();
        assert_eq!(calls.len(), 1);
        if let crate::application::mock::MockWindowCall::ApplySnap { target_id, .. } = &calls[0] {
            assert_eq!(target_id, "maximize");
        } else {
            panic!("expected ApplySnap");
        }
    }

    #[test]
    fn mouse_move_in_idle_is_ignored() {
        let (service, _w, _m, _o, _c) = make_service();
        // Idle 상태에서 이동 -> 상태 변화 없음
        service.on_mouse_moved(100, 100, 100.0, 0.0).unwrap();
        assert_eq!(service.state(), "idle");
        assert!(service.current_sector().is_none());
    }

    #[test]
    fn unmapped_sector_does_not_snap() {
        // 섹터 2(아래)는 매핑에 없음 -> snap 없이 종료
        let (service, window_mover, _m, _o, _c) = make_service();
        service.on_modifier_pressed(960, 540).unwrap();
        service.on_mouse_moved(960, 640, 0.0, 100.0).unwrap();
        assert_eq!(service.current_sector(), Some(2));

        service.on_modifier_released(false, 960, 640).unwrap();

        assert!(window_mover.apply_snap_calls().is_empty());
        assert_eq!(service.state(), "idle");
    }

    #[test]
    fn reset_clears_state() {
        let (service, _w, _m, _o, _c) = make_service();
        service.on_modifier_pressed(960, 540).unwrap();
        service.on_mouse_moved(1060, 540, 100.0, 0.0).unwrap();
        assert_eq!(service.state(), "tracking");

        service.reset();
        assert_eq!(service.state(), "idle");
        assert!(service.current_sector().is_none());
    }

    #[test]
    fn modifier_press_shows_lockon_current_window_rect() {
        // Armed 진입 시 현재 전경창(1001)의 rect 가 snap_preview 로 표시되어야 한다.
        // MockWindowMover::get_window_rect 는 (0,0,1920,1080) 을 반환.
        let (service, _w, _m, overlay, _c) = make_service();
        service.on_modifier_pressed(960, 540).unwrap();

        // active_sector 는 None (lock-on 상태) — RED 색상 신호.
        assert!(overlay.last_sector.lock().unwrap().is_none());
        // snap_preview 가 현재 창 rect 로 설정되어 있어야 한다.
        let preview = *overlay.last_snap_preview.lock().unwrap();
        assert_eq!(preview, Some((0, 0, 1920, 1080)));
    }

    #[test]
    fn mouse_move_below_threshold_keeps_lockon() {
        // 임계값(8px) 미만 이동 시 snap_preview 가 갱신되지 않음 (lock-on 유지).
        let (service, _w, _m, overlay, _c) = make_service();
        service.on_modifier_pressed(960, 540).unwrap();
        // lock-on rect 설정됨
        assert_eq!(
            *overlay.last_snap_preview.lock().unwrap(),
            Some((0, 0, 1920, 1080))
        );

        // 5px 이동 (< 8.0 임계값) — preview 갱신 안 됨, sector 도 None 유지
        service.on_mouse_moved(965, 540, 5.0, 0.0).unwrap();
        assert_eq!(service.state(), "tracking");
        assert!(overlay.last_sector.lock().unwrap().is_none());
        // lock-on rect 가 그대로 유지됨
        assert_eq!(
            *overlay.last_snap_preview.lock().unwrap(),
            Some((0, 0, 1920, 1080))
        );
    }

    #[test]
    fn mouse_move_above_threshold_switches_to_throw_target() {
        // 임계값 이상 이동 시 highlight_sector + throw target snap_preview 로 전환.
        let (service, _w, _m, overlay, _c) = make_service();
        service.on_modifier_pressed(960, 540).unwrap();

        // 100px 오른쪽 이동 (>= 8.0 임계값) — 섹터 0, throw target 표시
        service.on_mouse_moved(1060, 540, 100.0, 0.0).unwrap();
        assert_eq!(service.current_sector(), Some(0));
        // active_sector 가 Some 으로 전환 — BLUE 색상 신호.
        assert_eq!(*overlay.last_sector.lock().unwrap(), Some(0));
        // snap_preview 가 throw target(right-half: x=960,y=0,w=960,h=1080) 으로 갱신.
        let preview = *overlay.last_snap_preview.lock().unwrap();
        assert!(preview.is_some());
        let (x, y, w, h) = preview.unwrap();
        // right-half 매핑: monitor 1920x1080 기준 x_ratio=0.5 -> x=960, w=960
        assert_eq!(x, 960);
        assert_eq!(y, 0);
        assert_eq!(w, 960);
        assert_eq!(h, 1080);
    }

    #[test]
    fn lockon_skips_preview_when_no_foreground_window() {
        // 전경창이 없으면 lock-on preview 없이 visible 만 true.
        let (service, window_mover, _m, overlay, _c) = make_service();
        *window_mover.foreground_window.lock().unwrap() = None;

        service.on_modifier_pressed(960, 540).unwrap();
        assert!(*overlay.visible.lock().unwrap());
        assert!(overlay.last_snap_preview.lock().unwrap().is_none());
    }
}
