use statig::prelude::*;

/// 커서 이벤트 — 입력 어댑터가 발생
#[derive(Debug, Clone, PartialEq)]
pub enum CursorEvent {
    /// modifier 키 눌림
    ModifierPressed,
    /// modifier 키 뗌. cancel=true면 snap 실행 않고 취소.
    ModifierReleased { cancel: bool },
    /// 마우스 이동. delta는 활성화 시점부터의 누적 이동량.
    MouseMoved { delta_x: f64, delta_y: f64 },
}

/// FSM 공유 저장소 — 현재 상태에서 계산된 값들을 보관
#[derive(Debug, Clone, Default)]
pub struct CursorFsm {
    /// Tracking 상태에서 현재 섹터 (snap 실행용)
    pub current_sector: Option<u8>,
    /// 현재 throw 거리 (Long Throw 판별용)
    pub throw_distance: f64,
}

/// 섹터 산출 콜백 타입 (geometry에서 주입, FSM은 geometry를 직접 모름)
pub type SectorComputer = fn(delta_x: f64, delta_y: f64) -> u8;
/// 거리 계산 콜백
pub type DistanceComputer = fn(delta_x: f64, delta_y: f64) -> f64;

/// FSM에 주입할 컨텍스트 — 섹터/거리 계산 함수
///
/// statig의 context는 항상 `&mut`로 전달되므로, 불변 함수 포인터 필드만
/// 갖더라도 `&mut FsmContext` 형태로 주입된다. 필드 자체는 `Copy` 가능한
/// 함수 포인터이므로 `Clone + Copy`를 유지한다.
#[derive(Debug, Clone, Copy)]
pub struct FsmContext {
    /// 설정된 섹터 수 — 클로저가 캡처하지만 FSM 본체는 직접 사용하지 않음.
    /// 향후 디버그/로그 용도로 노출.
    #[allow(dead_code)]
    pub sector_count: u8,
    pub compute_sector: SectorComputer,
    pub compute_distance: DistanceComputer,
}

/// statig Outcome 타입 별칭 — 모든 상태 핸들러의 반환형.
/// `#[state_machine]` 매크로는 반환형을 자동으로 치환하지 않으므로
/// 명시적으로 `Outcome<State>`를 기입해야 한다.
pub type Outcome = statig::Outcome<State>;

#[state_machine(initial = "State::idle()")]
impl CursorFsm {
    #[state(entry_action = "enter_idle")]
    fn idle(&mut self, event: &CursorEvent) -> Outcome {
        match event {
            CursorEvent::ModifierPressed => Transition(State::armed()),
            _ => Handled,
        }
    }

    #[state(entry_action = "enter_armed", superstate = "active")]
    fn armed(&mut self, context: &mut FsmContext, event: &CursorEvent) -> Outcome {
        match event {
            // 첫 이동 이벤트에서 즉시 섹터/거리를 계산한 뒤 Tracking으로 전이.
            // statig의 전이는 해당 이벤트를 소비하므로, Tracking 핸들러가
            // 같은 이벤트를 다시 받지 않는다. 따라서 전이를 발생시킨 시점에
            // 초기 섹터를 저장해야 Tracking 진입 직후부터 값이 유효하다.
            CursorEvent::MouseMoved { delta_x, delta_y } => {
                self.current_sector = Some((context.compute_sector)(*delta_x, *delta_y));
                self.throw_distance = (context.compute_distance)(*delta_x, *delta_y);
                Transition(State::tracking())
            }
            CursorEvent::ModifierReleased { .. } => Transition(State::idle()),
            CursorEvent::ModifierPressed => Handled,
        }
    }

    #[state(entry_action = "enter_tracking", superstate = "active")]
    fn tracking(&mut self, context: &mut FsmContext, event: &CursorEvent) -> Outcome {
        match event {
            CursorEvent::MouseMoved { delta_x, delta_y } => {
                self.current_sector = Some((context.compute_sector)(*delta_x, *delta_y));
                self.throw_distance = (context.compute_distance)(*delta_x, *delta_y);
                Handled
            }
            CursorEvent::ModifierReleased { cancel: true } => {
                self.current_sector = None;
                Transition(State::idle())
            }
            CursorEvent::ModifierReleased { cancel: false } => Transition(State::snapping()),
            CursorEvent::ModifierPressed => Handled,
        }
    }

    #[state(entry_action = "enter_snapping")]
    fn snapping(&mut self, event: &CursorEvent) -> Outcome {
        let _ = event;
        Transition(State::idle())
    }

    #[superstate]
    fn active(&mut self, event: &CursorEvent) -> Outcome {
        let _ = event;
        Super
    }

    #[action]
    fn enter_idle(&mut self) {
        self.current_sector = None;
        self.throw_distance = 0.0;
    }

    #[action]
    fn enter_armed(&mut self) {}

    #[action]
    fn enter_tracking(&mut self) {}

    #[action]
    fn enter_snapping(&mut self) {
        // Snap 실행 상태 진입 — 섹터 정보는 소비됨.
        // 다음 idle 진입 시 enter_idle 이 다시 한번 정리하지만,
        // Snapping 상태 도중에는 current_sector가 유효하지 않아야 한다.
        self.current_sector = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::geometry;

    fn test_ctx() -> FsmContext {
        FsmContext {
            sector_count: 8,
            compute_sector: |dx, dy| {
                geometry::compute_sector(euclid::Vector2D::new(dx, dy), 8)
            },
            compute_distance: |dx, dy| geometry::throw_distance(euclid::Vector2D::new(dx, dy)),
        }
    }

    #[test]
    fn initial_state_is_idle() {
        // `state_machine()` 은 lazy-init StateMachine을 반환.
        // 초기 상태(Idle)의 entry action 이 아직 실행되지 않았더라도
        // CursorFsm::default() 의 필드 기본값이 곧 Idle 상태의 저장소 값이다.
        let sm = CursorFsm::default().state_machine();
        assert!(sm.inner().current_sector.is_none());
        assert!((sm.inner().throw_distance - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn idle_to_armed_on_modifier_pressed() {
        let mut ctx = test_ctx();
        let mut fsm = CursorFsm::default()
            .uninitialized_state_machine()
            .init_with_context(&mut ctx);
        fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut ctx);
        assert!(fsm.inner().current_sector.is_none());
    }

    #[test]
    fn armed_to_tracking_on_mouse_move() {
        let mut ctx = test_ctx();
        let mut fsm = CursorFsm::default()
            .uninitialized_state_machine()
            .init_with_context(&mut ctx);
        fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut ctx);
        fsm.handle_with_context(
            &CursorEvent::MouseMoved {
                delta_x: 100.0,
                delta_y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(fsm.inner().current_sector, Some(0));
    }

    #[test]
    fn tracking_to_snapping_on_release() {
        let mut ctx = test_ctx();
        let mut fsm = CursorFsm::default()
            .uninitialized_state_machine()
            .init_with_context(&mut ctx);
        fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut ctx);
        fsm.handle_with_context(
            &CursorEvent::MouseMoved {
                delta_x: 100.0,
                delta_y: 0.0,
            },
            &mut ctx,
        );
        fsm.handle_with_context(
            &CursorEvent::ModifierReleased { cancel: false },
            &mut ctx,
        );
        assert!(fsm.inner().current_sector.is_none());
    }

    #[test]
    fn tracking_cancel_clears_sector() {
        let mut ctx = test_ctx();
        let mut fsm = CursorFsm::default()
            .uninitialized_state_machine()
            .init_with_context(&mut ctx);
        fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut ctx);
        fsm.handle_with_context(
            &CursorEvent::MouseMoved {
                delta_x: 100.0,
                delta_y: 0.0,
            },
            &mut ctx,
        );
        assert!(fsm.inner().current_sector.is_some());

        fsm.handle_with_context(
            &CursorEvent::ModifierReleased { cancel: true },
            &mut ctx,
        );
        assert!(fsm.inner().current_sector.is_none());
    }

    #[test]
    fn armed_release_without_move_returns_idle() {
        let mut ctx = test_ctx();
        let mut fsm = CursorFsm::default()
            .uninitialized_state_machine()
            .init_with_context(&mut ctx);
        fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut ctx);
        fsm.handle_with_context(
            &CursorEvent::ModifierReleased { cancel: false },
            &mut ctx,
        );
        assert!(fsm.inner().current_sector.is_none());
    }

    #[test]
    fn tracking_updates_sector_on_each_move() {
        let mut ctx = test_ctx();
        let mut fsm = CursorFsm::default()
            .uninitialized_state_machine()
            .init_with_context(&mut ctx);
        fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut ctx);

        fsm.handle_with_context(
            &CursorEvent::MouseMoved {
                delta_x: 100.0,
                delta_y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(fsm.inner().current_sector, Some(0));

        fsm.handle_with_context(
            &CursorEvent::MouseMoved {
                delta_x: 0.0,
                delta_y: 100.0,
            },
            &mut ctx,
        );
        assert_eq!(fsm.inner().current_sector, Some(2));
    }

    #[test]
    fn tracking_tracks_throw_distance() {
        let mut ctx = test_ctx();
        let mut fsm = CursorFsm::default()
            .uninitialized_state_machine()
            .init_with_context(&mut ctx);
        fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut ctx);
        fsm.handle_with_context(
            &CursorEvent::MouseMoved {
                delta_x: 300.0,
                delta_y: 400.0,
            },
            &mut ctx,
        );
        assert!((fsm.inner().throw_distance - 500.0).abs() < 0.001);
    }

    #[test]
    fn ignore_mouse_move_in_idle() {
        let mut ctx = test_ctx();
        let mut fsm = CursorFsm::default()
            .uninitialized_state_machine()
            .init_with_context(&mut ctx);
        fsm.handle_with_context(
            &CursorEvent::MouseMoved {
                delta_x: 100.0,
                delta_y: 0.0,
            },
            &mut ctx,
        );
        assert!(fsm.inner().current_sector.is_none());
    }
}
