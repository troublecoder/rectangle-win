//! KeyboardService — 키보드 방향키 체인 사이클 기반 스냅 오케스트레이션.
//!
//! 도메인의 `ChainCycle` 을 사용해 같은 방향 + 같은 창 + 타임아웃 내의
//! 연속 탭을 추적하고, 체인에서 선택된 SnapTarget id 로 스냅을 실행한다.

use std::sync::Arc;

use parking_lot::Mutex;

use crate::application::errors::{ApplicationError, AppResult};
use crate::application::ports::{ConfigStore, MonitorProvider, WindowMover};
use crate::domain::keyboard_chain::ChainCycle;
use crate::domain::model::Direction;

/// 키보드 스냅 서비스.
///
/// 입력 어댑터(단축키 핸들러)가 `on_direction_key` 를 호출하면,
/// ChainCycle이 결정한 체인 인덱스의 SnapTarget id로 스냅을 실행한다.
pub struct KeyboardService {
    window_mover: Arc<dyn WindowMover>,
    monitor_provider: Arc<dyn MonitorProvider>,
    config_store: Arc<dyn ConfigStore>,
    /// ChainCycle과 마지막으로 적용된 타임아웃(ms). 설정의 타임아웃이 바뀌면
    /// 다음 호출에서 cycle을 재생성한다.
    cycle: Mutex<(u64, ChainCycle)>,
}

impl KeyboardService {
    pub fn new(
        window_mover: Arc<dyn WindowMover>,
        monitor_provider: Arc<dyn MonitorProvider>,
        config_store: Arc<dyn ConfigStore>,
    ) -> Self {
        // Config는 생성 시점에 동기적으로 읽기 어려울 수 있으므로, 기본 타임아웃으로
        // 시작하고 첫 호출에서 설정값과 비교하여 필요시 재생성한다.
        let default_timeout = crate::domain::model::KeyboardConfig::default().cycle_timeout_ms;
        Self {
            window_mover,
            monitor_provider,
            config_store,
            cycle: Mutex::new((default_timeout, ChainCycle::new(default_timeout))),
        }
    }

    /// 방향키 입력 처리. 체인 인덱스를 결정하고 대상 SnapTarget을 찾아 스냅 실행.
    /// 키보드 기능이 비활성화된 경우 아무 동작도 하지 않는다.
    /// 반환값: 실행된 SnapTarget의 id (snap을 실행한 경우).
    pub fn on_direction_key(
        &self,
        direction: Direction,
        cursor_x: i32,
        cursor_y: i32,
    ) -> AppResult<Option<String>> {
        let config = self.config_store.load()?;
        if !config.keyboard.enabled {
            return Ok(None);
        }

        // 방향에 따라 체인 선택.
        let chain = if direction.is_horizontal() {
            &config.keyboard.chains.horizontal
        } else if direction.is_vertical() {
            &config.keyboard.chains.vertical
        } else {
            // 대각선 방향은 현재 지원하지 않음 — 무시.
            return Ok(None);
        };

        if chain.is_empty() {
            return Ok(None);
        }

        let window = self
            .window_mover
            .get_foreground_window()
            .ok_or(ApplicationError::NoForegroundWindow)?;

        // 설정의 타임아웃이 바뀌었으면 cycle 재생성 (기존 진행 상태는 포기).
        let configured_timeout = config.keyboard.cycle_timeout_ms;
        let index = {
            let mut guard = self.cycle.lock();
            if guard.0 != configured_timeout {
                *guard = (configured_timeout, ChainCycle::new(configured_timeout));
            }
            guard.1.next_index(direction, window, chain)?
        };

        let target_id = &chain[index];

        // 체인의 SnapTarget id로 areas에서 실제 타겟을 찾는다.
        let target = config
            .snap
            .areas
            .iter()
            .find(|t| t.id() == target_id.as_str())
            .ok_or_else(|| {
                ApplicationError::Domain(crate::domain::errors::DomainError::TargetNotFound(
                    target_id.clone(),
                ))
            })?;

        let monitor = self.monitor_provider.monitor_at_cursor(cursor_x, cursor_y);
        self.window_mover
            .apply_snap_target(window, target, &monitor)?;

        Ok(Some(target.id().to_string()))
    }

    /// 체인 상태 초기화 (창 포커스 변경 등).
    pub fn reset_cycle(&self) {
        self.cycle.lock().1.reset();
    }
}

// ────────────────────────────────────────────────────────────────────
// 테스트
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::mock::{
        MockConfigStore, MockMonitorProvider, MockWindowCall, MockWindowMover,
    };
    use crate::domain::model::SnapTarget;

    const WINDOW: u64 = 12345;

    /// 테스트용 KeyboardService 구성.
    /// 기본 Config(chains: horizontal=[left-half, ...], vertical=[maximize, ...])
    /// 와 areas에 해당 id 들을 추가한다.
    fn make_service() -> (
        KeyboardService,
        Arc<MockWindowMover>,
        Arc<MockMonitorProvider>,
        Arc<MockConfigStore>,
    ) {
        let window_mover = Arc::new(MockWindowMover::new());
        window_mover.set_foreground(WINDOW);
        let monitor_provider = Arc::new(MockMonitorProvider::default());
        let config_store = Arc::new(MockConfigStore::default());

        // Config::default()는 이미 기본 체인(horizontal/vertical)을 갖는다.
        // areas만 추가.
        {
            let mut cfg = config_store.config.lock().unwrap();
            cfg.snap.areas = standard_areas();
            // keyboard.enabled = true (기본값)
        }

        let service = KeyboardService::new(
            window_mover.clone(),
            monitor_provider.clone(),
            config_store.clone(),
        );

        (service, window_mover, monitor_provider, config_store)
    }

    /// 기본 체인에서 참조하는 모든 id 를 포함하는 areas.
    /// horizontal: left-half, third-left, center, third-right, right-half
    /// vertical:   maximize, almost-maximize, center, maximize-height
    fn standard_areas() -> Vec<SnapTarget> {
        use crate::domain::model::WindowAction::*;
        vec![
            SnapTarget::Area {
                id: "left-half".to_string(),
                name: "Left Half".to_string(),
                x_ratio: 0.0,
                y_ratio: 0.0,
                w_ratio: 0.5,
                h_ratio: 1.0,
            },
            SnapTarget::Area {
                id: "right-half".to_string(),
                name: "Right Half".to_string(),
                x_ratio: 0.5,
                y_ratio: 0.0,
                w_ratio: 0.5,
                h_ratio: 1.0,
            },
            SnapTarget::Area {
                id: "third-left".to_string(),
                name: "Left Third".to_string(),
                x_ratio: 0.0,
                y_ratio: 0.0,
                w_ratio: 0.333,
                h_ratio: 1.0,
            },
            SnapTarget::Area {
                id: "third-right".to_string(),
                name: "Right Third".to_string(),
                x_ratio: 0.667,
                y_ratio: 0.0,
                w_ratio: 0.333,
                h_ratio: 1.0,
            },
            SnapTarget::Area {
                id: "center".to_string(),
                name: "Center".to_string(),
                x_ratio: 0.25,
                y_ratio: 0.25,
                w_ratio: 0.5,
                h_ratio: 0.5,
            },
            SnapTarget::Action {
                id: "maximize".to_string(),
                name: "Maximize".to_string(),
                action: Maximize,
            },
            SnapTarget::Action {
                id: "almost-maximize".to_string(),
                name: "Almost Maximize".to_string(),
                action: AlmostMaximize,
            },
            SnapTarget::Action {
                id: "maximize-height".to_string(),
                name: "Maximize Height".to_string(),
                action: MaximizeHeight,
            },
        ]
    }

    #[test]
    fn first_right_tap_snaps_left_half() {
        // Right 방향 첫 탭 -> 인덱스 0 -> horizontal[0] = "left-half"
        let (service, window_mover, _m, _c) = make_service();
        let result = service.on_direction_key(Direction::Right, 100, 100).unwrap();
        assert_eq!(result, Some("left-half".to_string()));

        let calls = window_mover.snap_calls();
        assert_eq!(calls.len(), 1);
        if let MockWindowCall::ApplySnap { target_id, .. } = &calls[0] {
            assert_eq!(target_id, "left-half");
        } else {
            panic!("expected ApplySnap");
        }
    }

    #[test]
    fn second_right_tap_advances() {
        // 두 번째 Right 탭 -> 인덱스 1 -> horizontal[1] = "third-left"
        let (service, window_mover, _m, _c) = make_service();
        service.on_direction_key(Direction::Right, 100, 100).unwrap(); // 0
        let result = service.on_direction_key(Direction::Right, 100, 100).unwrap(); // 1

        assert_eq!(result, Some("third-left".to_string()));
        assert_eq!(window_mover.snap_calls().len(), 2);
    }

    #[test]
    fn vertical_down_snaps_maximize() {
        // Down 첫 탭 -> 인덱스 0 -> vertical[0] = "maximize"
        let (service, window_mover, _m, _c) = make_service();
        let result = service.on_direction_key(Direction::Down, 100, 100).unwrap();
        assert_eq!(result, Some("maximize".to_string()));

        let calls = window_mover.snap_calls();
        assert_eq!(calls.len(), 1);
        if let MockWindowCall::ApplySnap {
            target_id, is_action, ..
        } = &calls[0]
        {
            assert_eq!(target_id, "maximize");
            assert!(*is_action); // maximize는 액션
        } else {
            panic!("expected ApplySnap");
        }
    }

    #[test]
    fn reset_cycle_starts_over() {
        // 두 번 탭 후 reset -> 다시 인덱스 0
        let (service, window_mover, _m, _c) = make_service();
        service.on_direction_key(Direction::Right, 100, 100).unwrap(); // 0
        service.on_direction_key(Direction::Right, 100, 100).unwrap(); // 1
        assert_eq!(window_mover.snap_calls().len(), 2);

        service.reset_cycle();
        let result = service.on_direction_key(Direction::Right, 100, 100).unwrap();
        assert_eq!(result, Some("left-half".to_string())); // 다시 0
    }

    #[test]
    fn disabled_keyboard_does_nothing() {
        let (service, window_mover, _m, config_store) = make_service();
        {
            let mut cfg = config_store.config.lock().unwrap();
            cfg.keyboard.enabled = false;
        }
        let result = service.on_direction_key(Direction::Right, 100, 100).unwrap();
        assert_eq!(result, None);
        assert!(window_mover.snap_calls().is_empty());
    }

    #[test]
    fn different_window_resets_chain() {
        // ChainCycle 자체 기능이지만 서비스 레벨에서도 검증.
        let (service, window_mover, _m, _c) = make_service();
        service.on_direction_key(Direction::Right, 100, 100).unwrap(); // 0
        service.on_direction_key(Direction::Right, 100, 100).unwrap(); // 1

        // 다른 창으로 전환
        window_mover.set_foreground(99999);
        let result = service.on_direction_key(Direction::Right, 100, 100).unwrap();
        assert_eq!(result, Some("left-half".to_string())); // 0으로 리셋
    }

    #[test]
    fn no_foreground_window_returns_error() {
        let (service, window_mover, _m, _c) = make_service();
        window_mover
            .foreground_window
            .lock()
            .unwrap()
            .take(); // None으로 설정

        let result = service.on_direction_key(Direction::Right, 100, 100);
        assert!(matches!(result, Err(ApplicationError::NoForegroundWindow)));
    }

    #[test]
    fn diagonal_direction_is_ignored() {
        // 대각선(UpRight 등)은 현재 지원하지 않음 -> None 반환
        let (service, window_mover, _m, _c) = make_service();
        let result = service
            .on_direction_key(Direction::UpRight, 100, 100)
            .unwrap();
        assert_eq!(result, None);
        assert!(window_mover.snap_calls().is_empty());
    }
}
