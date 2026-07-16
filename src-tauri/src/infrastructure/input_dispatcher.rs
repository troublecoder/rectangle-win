//! 입력 이벤트 분류/라우팅 순수 로직.
//!
//! OS 후크 계층(Task 2+3)에서 수집한 raw 입력 이벤트를 [`InputEvent`] 로
//! 정규화한 뒤 [`classify`] 에게 전달하면, 현재 활성 modifier 조합을 기준으로
//! 어느 서비스로 라우팅할지 [`RouteTarget`] 을 돌려준다.
//!
//! 이 모듈에는 부수 효과가 전혀 없으며, Tauri/Win32 의존 없이 단위 테스트 가능하다.

use crate::domain::model::Direction;

/// 정규화된 입력 이벤트. OS 후크 계층이 이벤트를 이 형태로 변환한다.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// throw 트리거 modifier 조합이 눌림
    ModifierPressed { modifiers: Vec<String> },
    /// modifier 가 올려짐. `cancel == true` 면 사용자가 의도적 취소로 간주.
    ModifierReleased { modifiers: Vec<String>, cancel: bool },
    /// throw 모드에서의 마우스 이동
    MouseMoved { delta_x: f64, delta_y: f64 },
    /// 방향키 입력 (modifier 와 함께)
    ArrowKey {
        direction: Direction,
        modifiers: Vec<String>,
    },
}

/// [`classify`] 결과 — 어느 애플리케이션 서비스 / OS 처리로 보낼지.
#[derive(Debug, Clone, PartialEq)]
pub enum RouteTarget {
    /// [`SnapService`] (throw 모드 커서 기반 snap).
    ///
    /// [`SnapService`]: crate::application::snap_service
    SnapService,
    /// [`KeyboardService`] 로 방향키 snap 위임.
    ///
    /// [`KeyboardService`]: crate::application::keyboard_service
    KeyboardService(Direction),
    /// 무시 (다른 핫키 / 일반 입력).
    Ignore,
}

/// 두 modifier 리스트가 (순서 무관하게) 동일한 원소 집합인지 검사.
fn modifiers_match(pressed: &[String], expected: &[String]) -> bool {
    if pressed.len() != expected.len() {
        return false;
    }
    let mut p = pressed.to_vec();
    let mut e = expected.to_vec();
    p.sort();
    e.sort();
    p == e
}

/// 정규화된 [`InputEvent`] 를 어디로 라우팅할지 결정한다.
///
/// 인자:
/// - `throw_modifiers`: throw(커서) 및 키보드 snap 이 공유하는 트리거 modifier 조합
///
/// 키보드 snap 은 항상 throw 와 동일한 modifier 조합을 사용한다 (Shared 고정).
pub fn classify(event: &InputEvent, throw_modifiers: &[String]) -> RouteTarget {
    match event {
        InputEvent::ModifierPressed { modifiers } => {
            if modifiers_match(modifiers, throw_modifiers) {
                RouteTarget::SnapService
            } else {
                RouteTarget::Ignore
            }
        }
        InputEvent::MouseMoved { .. } => RouteTarget::SnapService,
        InputEvent::ModifierReleased { modifiers, .. } => {
            if modifiers_match(modifiers, throw_modifiers) {
                RouteTarget::SnapService
            } else {
                RouteTarget::Ignore
            }
        }
        InputEvent::ArrowKey { direction, modifiers } => {
            if modifiers_match(modifiers, throw_modifiers) {
                RouteTarget::KeyboardService(*direction)
            } else {
                RouteTarget::Ignore
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn throw_mods() -> Vec<String> {
        vec!["Win".into(), "Alt".into()]
    }

    #[test]
    fn throw_modifier_routes_to_snap() {
        let event = InputEvent::ModifierPressed {
            modifiers: throw_mods(),
        };
        assert_eq!(
            classify(&event, &throw_mods()),
            RouteTarget::SnapService
        );
    }

    #[test]
    fn arrow_with_throw_mods_routes_to_keyboard() {
        // 키보드 snap 은 throw modifier 조합을 공유 (Win+Alt+방향키).
        let event = InputEvent::ArrowKey {
            direction: Direction::Right,
            modifiers: throw_mods(),
        };
        assert_eq!(
            classify(&event, &throw_mods()),
            RouteTarget::KeyboardService(Direction::Right)
        );
    }

    #[test]
    fn arrow_left_with_throw_mods_routes_to_keyboard() {
        let event = InputEvent::ArrowKey {
            direction: Direction::Left,
            modifiers: throw_mods(),
        };
        assert_eq!(
            classify(&event, &throw_mods()),
            RouteTarget::KeyboardService(Direction::Left)
        );
    }

    #[test]
    fn unrelated_modifier_ignored() {
        let event = InputEvent::ModifierPressed {
            modifiers: vec!["Shift".into()],
        };
        assert_eq!(classify(&event, &throw_mods()), RouteTarget::Ignore);
    }

    #[test]
    fn arrow_with_non_throw_mods_ignored() {
        let event = InputEvent::ArrowKey {
            direction: Direction::Right,
            modifiers: vec!["Ctrl".into()],
        };
        assert_eq!(classify(&event, &throw_mods()), RouteTarget::Ignore);
    }

    #[test]
    fn mouse_moved_always_routes_to_snap() {
        let event = InputEvent::MouseMoved {
            delta_x: 12.0,
            delta_y: -3.5,
        };
        assert_eq!(classify(&event, &throw_mods()), RouteTarget::SnapService);
    }

    #[test]
    fn modifier_release_with_throw_mods_routes_to_snap() {
        // throw modifier release 시 snap 서비스로 커밋/종료를 알림
        let event = InputEvent::ModifierReleased {
            modifiers: throw_mods(),
            cancel: false,
        };
        assert_eq!(
            classify(&event, &throw_mods()),
            RouteTarget::SnapService
        );
    }

    #[test]
    fn modifier_release_with_other_mods_ignored() {
        let event = InputEvent::ModifierReleased {
            modifiers: vec!["Shift".into()],
            cancel: true,
        };
        assert_eq!(classify(&event, &throw_mods()), RouteTarget::Ignore);
    }

    #[test]
    fn modifiers_match_is_order_independent() {
        // 내부 helper 검증 — 정렬 후 비교이므로 순서가 달라도 일치해야 한다.
        assert!(modifiers_match(
            &["Alt".to_string(), "Win".to_string()],
            &["Win".to_string(), "Alt".to_string()],
        ));
        assert!(!modifiers_match(
            &["Alt".to_string()],
            &["Alt".to_string(), "Win".to_string()],
        ));
    }

    #[test]
    fn arrow_partial_mods_ignored() {
        // modifier 중 일부만 눌린 방향키는 무시
        let event = InputEvent::ArrowKey {
            direction: Direction::Up,
            modifiers: vec!["Win".into()],
        };
        assert_eq!(classify(&event, &throw_mods()), RouteTarget::Ignore);
    }
}
