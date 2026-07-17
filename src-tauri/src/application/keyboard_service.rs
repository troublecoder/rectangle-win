//! KeyboardService — 키보드 방향키 기반 snap 오케스트레이션 (재설계).
//!
//! 기존 ChainCycle(같은 방향 + 타임아웃 연속 탭) 기반 동작을 폐기하고,
//! 방향키가 의미하는 "스냅 타겟 순회" 모델로 단순화한다:
//!
//! - **← / →**: throw 매핑(`throw.mapping` 섹터 → SnapTarget id)을
//!   섹터 오름차순 정렬한 뒤 순환. → 는 정방향(0→1→…→wrap), ← 는 역방향.
//! - **↑ / ↓**: 고정 액션 체인 `[maximize, restore, center, minimize]` 순환.
//!   ↓ 이 정방향(maximize → restore → center → minimize → maximize …),
//!   ↑ 이 역방향.
//!
//! 내부 상태는 두 개의 인덱스(`sector_index`, `vindex`)만 보존하며,
//! `reset_cycle()` 이 둘을 모두 0으로 되돌린다.
//!
//! 설정에서 throw 매핑이나 snap.areas 가 바뀌어도 인덱스는 클램프/래핑으로
//! 대응(범위 밖이면 0부터 재시작)한다.

use std::sync::Arc;

use parking_lot::Mutex;

use crate::application::errors::{ApplicationError, AppResult};
use crate::application::ports::{ConfigStore, MonitorProvider, WindowMover};
use crate::domain::model::Direction;

/// ↑/↓ 액션 순환 체인 (고정). snap.areas 에서 이 id 들을 SnapTarget 으로 찾는다.
/// ↓ 를 누를 때마다 한 단계씩 아래로 이동하며, 끝(minimize)에서 다시 maximize 로 랩.
const VERTICAL_ACTION_IDS: &[&str] = &["maximize", "restore", "center", "minimize"];

/// 키보드 스냅 서비스.
///
/// 입력 어댑터(단축키 핸들러)가 `on_direction_key` 를 호출하면, 현재 인덱스의
/// SnapTarget id 로 스냅을 실행하고 인덱스를 전진시킨다.
pub struct KeyboardService {
    window_mover: Arc<dyn WindowMover>,
    monitor_provider: Arc<dyn MonitorProvider>,
    config_store: Arc<dyn ConfigStore>,
    overlay: Arc<dyn crate::application::ports::OverlayController>,
    /// ←/→ throw 매핑 순회 인덱스.
    sector_index: Mutex<usize>,
    /// ↑/↓ 액션 체인 인덱스.
    vindex: Mutex<usize>,
    /// 마지막으로 snap 한 창 핸들 — minimize 후 foreground 가 없을 때 사용.
    last_window: Mutex<Option<u64>>,
}

impl KeyboardService {
    pub fn new(
        window_mover: Arc<dyn WindowMover>,
        monitor_provider: Arc<dyn MonitorProvider>,
        config_store: Arc<dyn ConfigStore>,
        overlay: Arc<dyn crate::application::ports::OverlayController>,
    ) -> Self {
        Self {
            window_mover,
            monitor_provider,
            config_store,
            overlay,
            sector_index: Mutex::new(0),
            vindex: Mutex::new(0),
            last_window: Mutex::new(None),
        }
    }

    /// 방향키 입력 처리. 현재 인덱스의 SnapTarget 을 찾아 스냅 실행 후
    /// 인덱스를 전진/후진 시킨다. 키보드 기능이 비활성화된 경우 None 반환.
    /// 반환값: 실행된 SnapTarget 의 id (snap 을 실행한 경우).
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

        // foreground 창 획득 — minimize 후 등 foreground 가 없으면 last_window 사용.
        let window = match self.window_mover.get_foreground_window() {
            Some(w) => {
                *self.last_window.lock() = Some(w);
                w
            }
            None => {
                // 최소화된 창 등 — 마지막 snap 창으로 복원 시도.
                self.last_window.lock().ok_or(ApplicationError::NoForegroundWindow)?
            }
        };

        // 방향에 따라 순회 대상 결정.
        //
        // 순환 모델 (스펙 표기 "Right: 0→1→2→…→7→0", "Left: 7→6→…→0→7" 의 직역):
        // - 정방향(Right/Down): 현재 인덱스를 적용한 뒤 전진.
        //   즉 첫 Right = 인덱스 0, 첫 Down = maximize.
        // - 역방향(Left/Up): 인덱스를 먼저 후진(랩)시킨 뒤 적용.
        //   즉 첫 Left = 마지막 섹터, 첫 Up = minimize.
        // 이렇게 하면 →/← 가 각각 "0번부터" / "끝번부터" 시작하며 스펙 표기와 일치한다.
        let target_id = if direction.is_horizontal() {
            // throw 매핑 → 정렬된 (sector, id) 목록.
            let mut entries: Vec<(u8, &String)> =
                config.throw.mapping.iter().map(|(s, id)| (*s, id)).collect();
            entries.sort_by_key(|(s, _)| *s);

            if entries.is_empty() {
                return Ok(None);
            }

            let len = entries.len();
            let mut idx_guard = self.sector_index.lock();
            if *idx_guard >= len {
                *idx_guard = 0;
            }
            let idx = *idx_guard;

            let (apply_idx, next_idx) = match direction {
                // Right: 현재 인덱스 적용 후 전진.
                Direction::Right => (idx, (idx + 1) % len),
                // Left: 후진(랩) 후 적용.
                Direction::Left => {
                    let prev = if idx == 0 { len - 1 } else { idx - 1 };
                    (prev, prev)
                }
                _ => (idx, idx), // 도달 불가 (is_horizontal)
            };
            *idx_guard = next_idx;
            drop(idx_guard);

            entries[apply_idx].1.clone()
        } else if direction.is_vertical() {
            let len = VERTICAL_ACTION_IDS.len();
            let mut idx_guard = self.vindex.lock();
            if *idx_guard >= len {
                *idx_guard = 0;
            }
            let idx = *idx_guard;

            let (apply_idx, next_idx) = match direction {
                // ↓ 정방향: 현재 인덱스 적용 후 전진.
                Direction::Down => (idx, (idx + 1) % len),
                // ↑ 역방향: 후진(랩) 후 적용.
                Direction::Up => {
                    let prev = if idx == 0 { len - 1 } else { idx - 1 };
                    (prev, prev)
                }
                _ => (idx, idx),
            };
            *idx_guard = next_idx;
            drop(idx_guard);

            VERTICAL_ACTION_IDS[apply_idx].to_string()
        } else {
            // 대각선은 미지원.
            return Ok(None);
        };

        // snap.areas 에서 실제 SnapTarget 조회.
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

        // snap 후 새 창 위치를 overlay 로 표시 — 사용자가 어디에 락온되어 있는지 확인.
        // show_reticle 으로 overlay 창을 visible + active_sector=None (RED lock-on 색상).
        let center = monitor.center();
        let _ = self.overlay.show_reticle(center.x, center.y, config.overlay.sector_count);
        // snap 된 창의 새 rect 를 overlay 에 표시.
        if let Ok(new_rect) = self.window_mover.get_window_rect(window) {
            let _ = self.overlay.show_snap_preview(
                new_rect.origin.x,
                new_rect.origin.y,
                new_rect.size.width,
                new_rect.size.height,
            );
        }

        Ok(Some(target.id().to_string()))
    }

    /// 순회 인덱스 초기화 (창 포커스 변경 등).
    pub fn reset_cycle(&self) {
        *self.sector_index.lock() = 0;
        *self.vindex.lock() = 0;
    }
}

// ────────────────────────────────────────────────────────────────────
// 테스트
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::mock::{
        MockConfigStore, MockMonitorProvider, MockOverlayController, MockWindowCall,
        MockWindowMover,
    };
    use crate::domain::model::{SectorMap, SnapTarget};

    const WINDOW: u64 = 12345;

    /// 테스트용 KeyboardService + throw 매핑/areas 구성.
    /// throw.mapping: 0→left-half, 2→center, 4→right-half
    /// areas: left-half, right-half, center, maximize, restore, center(action id? x), minimize
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

        {
            let mut cfg = config_store.config.lock().unwrap();
            cfg.snap.areas = areas();
            cfg.throw.mapping = throw_mapping();
            // keyboard.enabled = true (기본값)
        }

        let overlay: Arc<dyn crate::application::ports::OverlayController> =
            Arc::new(MockOverlayController::default());
        let service = KeyboardService::new(
            window_mover.clone(),
            monitor_provider.clone(),
            config_store.clone(),
            overlay,
        );

        (service, window_mover, monitor_provider, config_store)
    }

    /// throw 매핑: 섹터 0→left-half, 2→center, 4→right-half.
    /// 정렬 시 [0,2,4] 순서.
    fn throw_mapping() -> SectorMap {
        let mut m = SectorMap::new();
        m.insert(0, "left-half".to_string());
        m.insert(2, "center".to_string());
        m.insert(4, "right-half".to_string());
        m
    }

    fn areas() -> Vec<SnapTarget> {
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
                id: "restore".to_string(),
                name: "Restore".to_string(),
                action: Restore,
            },
            SnapTarget::Action {
                id: "minimize".to_string(),
                name: "Minimize".to_string(),
                action: Minimize,
            },
        ]
    }

    #[test]
    fn first_right_snaps_first_sector() {
        // Right 첫 탭 → 인덱스 0 → 정렬된 매핑[0] = (0, "left-half")
        let (service, window_mover, _m, _c) = make_service();
        let result = service.on_direction_key(Direction::Right, 100, 100).unwrap();
        assert_eq!(result, Some("left-half".to_string()));

        let calls = window_mover.apply_snap_calls();
        assert_eq!(calls.len(), 1);
        if let MockWindowCall::ApplySnap { target_id, .. } = &calls[0] {
            assert_eq!(target_id, "left-half");
        } else {
            panic!("expected ApplySnap");
        }
    }

    #[test]
    fn second_right_advances_to_next_sector() {
        // Right 두 번째 탭 → 인덱스 1 → 매핑[1] = (2, "center")
        let (service, window_mover, _m, _c) = make_service();
        service.on_direction_key(Direction::Right, 100, 100).unwrap(); // 0 → left-half
        let result = service.on_direction_key(Direction::Right, 100, 100).unwrap(); // 1 → center

        assert_eq!(result, Some("center".to_string()));
        assert_eq!(window_mover.apply_snap_calls().len(), 2);
    }

    #[test]
    fn right_wraps_around() {
        // 3개 매핑. Right * 3 → 인덱스 0,1,2 (right-half). 한 번 더 → 0 (left-half).
        let (service, window_mover, _m, _c) = make_service();
        let r1 = service.on_direction_key(Direction::Right, 100, 100).unwrap();
        let r2 = service.on_direction_key(Direction::Right, 100, 100).unwrap();
        let r3 = service.on_direction_key(Direction::Right, 100, 100).unwrap();
        let r4 = service.on_direction_key(Direction::Right, 100, 100).unwrap();
        assert_eq!(r1, Some("left-half".to_string()));
        assert_eq!(r2, Some("center".to_string()));
        assert_eq!(r3, Some("right-half".to_string()));
        assert_eq!(r4, Some("left-half".to_string())); // wrap
        assert_eq!(window_mover.apply_snap_calls().len(), 4);
    }

    #[test]
    fn left_from_start_wraps_to_last_sector() {
        // 인덱스 0에서 Left → 마지막 인덱스(2) → right-half
        let (service, _window_mover, _m, _c) = make_service();
        let result = service.on_direction_key(Direction::Left, 100, 100).unwrap();
        assert_eq!(result, Some("right-half".to_string()));
    }

    #[test]
    fn left_then_right_returns_to_start() {
        // Left(idx0→prev2, right-half) → Right(idx2→apply2, right-half)
        // 역방향 후 정방향은 같은 항목에서 만난다(인덱스가 2에서 만남).
        let (service, _window_mover, _m, _c) = make_service();
        let r1 = service.on_direction_key(Direction::Left, 100, 100).unwrap();
        let r2 = service.on_direction_key(Direction::Right, 100, 100).unwrap();
        assert_eq!(r1, Some("right-half".to_string()));
        assert_eq!(r2, Some("right-half".to_string()));
    }

    #[test]
    fn down_snaps_maximize_then_restore() {
        // ↓ 첫 탭 → maximize, ↓ 또 → restore
        let (service, window_mover, _m, _c) = make_service();
        let r1 = service.on_direction_key(Direction::Down, 100, 100).unwrap();
        let r2 = service.on_direction_key(Direction::Down, 100, 100).unwrap();
        assert_eq!(r1, Some("maximize".to_string()));
        assert_eq!(r2, Some("restore".to_string()));

        let calls = window_mover.apply_snap_calls();
        assert_eq!(calls.len(), 2);
        if let MockWindowCall::ApplySnap {
            target_id, is_action, ..
        } = &calls[0]
        {
            assert_eq!(target_id, "maximize");
            assert!(*is_action);
        } else {
            panic!("expected ApplySnap");
        }
    }

    #[test]
    fn down_cycles_through_all_actions_and_wraps() {
        // ↓ * 4 → maximize, restore, center, minimize. 한 번 더 → maximize (wrap).
        let (service, _window_mover, _m, _c) = make_service();
        let r1 = service.on_direction_key(Direction::Down, 100, 100).unwrap();
        let r2 = service.on_direction_key(Direction::Down, 100, 100).unwrap();
        let r3 = service.on_direction_key(Direction::Down, 100, 100).unwrap();
        let r4 = service.on_direction_key(Direction::Down, 100, 100).unwrap();
        let r5 = service.on_direction_key(Direction::Down, 100, 100).unwrap();
        assert_eq!(r1, Some("maximize".to_string()));
        assert_eq!(r2, Some("restore".to_string()));
        assert_eq!(r3, Some("center".to_string()));
        assert_eq!(r4, Some("minimize".to_string()));
        assert_eq!(r5, Some("maximize".to_string())); // wrap
    }

    #[test]
    fn up_goes_backward() {
        // 처음 ↑ = 역방향 시작 = 체인의 마지막(minimize). 그 다음 ↑ = center.
        let (service, _window_mover, _m, _c) = make_service();
        let r1 = service.on_direction_key(Direction::Up, 100, 100).unwrap();
        let r2 = service.on_direction_key(Direction::Up, 100, 100).unwrap();
        assert_eq!(r1, Some("minimize".to_string()));
        assert_eq!(r2, Some("center".to_string()));
    }

    #[test]
    fn reset_cycle_resets_both_indices() {
        let (service, _window_mover, _m, _c) = make_service();
        // 두 축 모두 전진.
        let _ = service.on_direction_key(Direction::Right, 100, 100).unwrap(); // sector 0
        let _ = service.on_direction_key(Direction::Right, 100, 100).unwrap(); // sector 1
        let _ = service.on_direction_key(Direction::Down, 100, 100).unwrap(); // vindex 0

        service.reset_cycle();
        // 다시 첫 항목으로.
        let r_h = service.on_direction_key(Direction::Right, 100, 100).unwrap();
        let r_v = service.on_direction_key(Direction::Down, 100, 100).unwrap();
        assert_eq!(r_h, Some("left-half".to_string()));
        assert_eq!(r_v, Some("maximize".to_string()));
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
        assert!(window_mover.apply_snap_calls().is_empty());
    }

    #[test]
    fn no_foreground_window_returns_error() {
        let (service, window_mover, _m, _c) = make_service();
        window_mover.foreground_window.lock().unwrap().take();

        let result = service.on_direction_key(Direction::Right, 100, 100);
        assert!(matches!(result, Err(ApplicationError::NoForegroundWindow)));
    }

    #[test]
    fn diagonal_direction_is_ignored() {
        let (service, window_mover, _m, _c) = make_service();
        let result = service
            .on_direction_key(Direction::UpRight, 100, 100)
            .unwrap();
        assert_eq!(result, None);
        assert!(window_mover.apply_snap_calls().is_empty());
    }

    #[test]
    fn empty_throw_mapping_right_returns_none() {
        // throw 매핑이 비면 ←/→ 는 no-op.
        let (service, window_mover, _m, config_store) = make_service();
        {
            let mut cfg = config_store.config.lock().unwrap();
            cfg.throw.mapping = SectorMap::new();
        }
        let result = service.on_direction_key(Direction::Right, 100, 100).unwrap();
        assert_eq!(result, None);
        assert!(window_mover.apply_snap_calls().is_empty());
    }
}
