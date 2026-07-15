//! 입력 이벤트 분류/라우팅 순수 로직.
//!
//! OS 후크 계층(Task 2+3)에서 수집한 raw 입력 이벤트를 [`InputEvent`] 로
//! 정규화한 뒤 [`classify`] 에게 전달하면, 현재 활성 설정(modifier 조합,
//! [`ModifierMode`])을 기준으로 어느 서비스로 라우팅할지 [`RouteTarget`] 을
//! 돌려준다.
//!
//! 이 모듈에는 부수 효과가 전혀 없으며, Tauri/Win32 의존 없이 단위 테스트 가능하다.

use crate::domain::model::{Direction, ModifierMode};

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
    /// Win+방향키 OS snap 을 삼키고 우리 snap 으로 대체 (OverrideOs 모드).
    SwallowOsSnap(Direction),
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
/// - `throw_modifiers`: throw(커서) snap 트리거 modifier 조합
/// - `kb_modifiers`: 키보드 snap (Separate 모드) 트리거 modifier 조합
/// - `kb_mode`: 현재 키보드 snap 동작 모드
pub fn classify(
    event: &InputEvent,
    throw_modifiers: &[String],
    kb_modifiers: &[String],
    kb_mode: ModifierMode,
) -> RouteTarget {
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
        InputEvent::ArrowKey { direction, modifiers } => match kb_mode {
            ModifierMode::OverrideOs => {
                if modifiers.iter().any(|m| m == "Win") {
                    RouteTarget::SwallowOsSnap(*direction)
                } else {
                    RouteTarget::Ignore
                }
            }
            ModifierMode::Shared => {
                if modifiers_match(modifiers, throw_modifiers) {
                    RouteTarget::KeyboardService(*direction)
                } else {
                    RouteTarget::Ignore
                }
            }
            ModifierMode::Separate => {
                if modifiers_match(modifiers, kb_modifiers) {
                    RouteTarget::KeyboardService(*direction)
                } else {
                    RouteTarget::Ignore
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn throw_mods() -> Vec<String> {
        vec!["Win".into(), "Alt".into()]
    }
    fn kb_mods() -> Vec<String> {
        vec!["Ctrl".into(), "Alt".into()]
    }

    #[test]
    fn throw_modifier_routes_to_snap() {
        let event = InputEvent::ModifierPressed {
            modifiers: throw_mods(),
        };
        assert_eq!(
            classify(&event, &throw_mods(), &kb_mods(), ModifierMode::Separate),
            RouteTarget::SnapService
        );
    }

    #[test]
    fn arrow_separate_mode() {
        let event = InputEvent::ArrowKey {
            direction: Direction::Right,
            modifiers: kb_mods(),
        };
        assert_eq!(
            classify(&event, &throw_mods(), &kb_mods(), ModifierMode::Separate),
            RouteTarget::KeyboardService(Direction::Right)
        );
    }

    #[test]
    fn arrow_override_os_mode() {
        let event = InputEvent::ArrowKey {
            direction: Direction::Down,
            modifiers: vec!["Win".into()],
        };
        assert_eq!(
            classify(
                &event,
                &throw_mods(),
                &kb_mods(),
                ModifierMode::OverrideOs
            ),
            RouteTarget::SwallowOsSnap(Direction::Down)
        );
    }

    #[test]
    fn arrow_shared_mode() {
        let event = InputEvent::ArrowKey {
            direction: Direction::Left,
            modifiers: throw_mods(),
        };
        assert_eq!(
            classify(&event, &throw_mods(), &kb_mods(), ModifierMode::Shared),
            RouteTarget::KeyboardService(Direction::Left)
        );
    }

    #[test]
    fn unrelated_modifier_ignored() {
        let event = InputEvent::ModifierPressed {
            modifiers: vec!["Shift".into()],
        };
        assert_eq!(
            classify(&event, &throw_mods(), &kb_mods(), ModifierMode::Separate),
            RouteTarget::Ignore
        );
    }

    #[test]
    fn override_os_ignores_non_win_arrows() {
        let event = InputEvent::ArrowKey {
            direction: Direction::Right,
            modifiers: vec!["Ctrl".into()],
        };
        assert_eq!(
            classify(
                &event,
                &throw_mods(),
                &kb_mods(),
                ModifierMode::OverrideOs
            ),
            RouteTarget::Ignore
        );
    }

    #[test]
    fn mouse_moved_always_routes_to_snap() {
        let event = InputEvent::MouseMoved {
            delta_x: 12.0,
            delta_y: -3.5,
        };
        assert_eq!(
            classify(&event, &throw_mods(), &kb_mods(), ModifierMode::Separate),
            RouteTarget::SnapService
        );
    }

    #[test]
    fn modifier_release_with_throw_mods_routes_to_snap() {
        // throw modifier release 시 snap 서비스로 커밋/종료를 알림
        let event = InputEvent::ModifierReleased {
            modifiers: throw_mods(),
            cancel: false,
        };
        assert_eq!(
            classify(&event, &throw_mods(), &kb_mods(), ModifierMode::Separate),
            RouteTarget::SnapService
        );
    }

    #[test]
    fn modifier_release_with_other_mods_ignored() {
        let event = InputEvent::ModifierReleased {
            modifiers: vec!["Shift".into()],
            cancel: true,
        };
        assert_eq!(
            classify(&event, &throw_mods(), &kb_mods(), ModifierMode::Separate),
            RouteTarget::Ignore
        );
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
    fn arrow_separate_mode_wrong_mods_ignored() {
        // Separate 모드에서 throw_modifiers 만 눌린 방향키는 무시
        let event = InputEvent::ArrowKey {
            direction: Direction::Up,
            modifiers: throw_mods(),
        };
        assert_eq!(
            classify(&event, &throw_mods(), &kb_mods(), ModifierMode::Separate),
            RouteTarget::Ignore
        );
    }
}
