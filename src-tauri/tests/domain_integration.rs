//! 통합 테스트 — 도메인 컴포넌트들이 함께 동작하는지 검증.
//!
//! `src/lib.rs` 가 `domain` 모듈을 라이브러리로 노출하므로,
//! `tests/` 디렉토리의 통합 테스트에서 `rectangle_win::domain::*` 경로로
//! 각 서브모듈을 가져올 수 있다.

use rectangle_win::domain::cursor_fsm::{CursorEvent, CursorFsm, FsmContext};
use rectangle_win::domain::geometry;
use rectangle_win::domain::model::{Config, SnapTarget};
use rectangle_win::domain::presets::SnapPreset;

// statig의 상태머신 빌더 트레이트 — `.uninitialized_state_machine()` /
// `.init_with_context()` / `.handle_with_context()` / `.inner()` 를 사용하려면
// 해당 트레이트가 스코프 내에 있어야 한다. (cursor_fsm 내부 단위 테스트은
// `statig::prelude::*` 를 재노입하여 해결하지만, 통합 테스트는 라이브러리
// 경계 밖이므로 명시적으로 임포트한다.)
use statig::prelude::IntoStateMachineExt;

/// FSM 주입용 컨텍스트 생성 헬퍼.
///
/// `geometry` 함수들은 euclid 단위 타입 `Pixel` 을 요구하지만,
/// `Vector2D::new(dx, dy)` 는 문맥상 `Vector2D<f64, Pixel>` 로 추론되므로
/// 명시적 단위 마커 없이 호출 가능하다.
fn make_ctx() -> FsmContext {
    FsmContext {
        sector_count: 8,
        compute_sector: |dx, dy| geometry::compute_sector(euclid::Vector2D::new(dx, dy), 8),
        compute_distance: |dx, dy| geometry::throw_distance(euclid::Vector2D::new(dx, dy)),
    }
}

#[test]
fn config_serialize_deserialize_toml() {
    let config = Config::default();
    let toml_str = toml::to_string(&config).expect("Config 직렬화 실패");

    // Config 의 모든 최상위 섹션이 TOML 에 포함되어야 한다.
    assert!(toml_str.contains("[general]"), "general 섹션 누락");
    assert!(toml_str.contains("[snap]"), "snap 섹션 누락");
    assert!(toml_str.contains("[throw]"), "throw 섹션 누락");
    assert!(toml_str.contains("[keyboard]"), "keyboard 섹션 누락");

    // 왕복(roundtrip): 직렬화 → 역직렬화 결과가 원본과 동일해야 한다.
    let parsed: Config = toml::from_str(&toml_str).expect("Config 역직렬화 실패");
    assert_eq!(config, parsed);
}

#[test]
fn preset_targets_all_valid_ratios() {
    // 모든 프리셋의 모든 Area 타겟에 대해 비율이 [0.0, 1.0] 범위여야 한다.
    for preset in [
        SnapPreset::Minimal,
        SnapPreset::Standard,
        SnapPreset::Extended,
        SnapPreset::Full,
        SnapPreset::Portrait,
    ] {
        for target in preset.targets() {
            if let SnapTarget::Area {
                x_ratio,
                y_ratio,
                w_ratio,
                h_ratio,
                ..
            } = &target
            {
                for &r in &[*x_ratio, *y_ratio, *w_ratio, *h_ratio] {
                    assert!(
                        (0.0..=1.0).contains(&r),
                        "유효하지 않은 비율 {} in {:?}",
                        r,
                        preset
                    );
                }
            }
        }
    }
}

#[test]
fn fsm_and_geometry_integration() {
    // FSM ↔ geometry 연동:
    // 1) ModifierPressed → Armed
    // 2) MouseMoved(100, -100) → Tracking 진입, 동시에 초기 섹터 산출
    //
    // geometry.rs 의 섹터 배치(8섹터 기준): 7 = 오른쪽위 (dx>0, dy<0)
    let mut ctx = make_ctx();
    let mut fsm = CursorFsm::default()
        .uninitialized_state_machine()
        .init_with_context(&mut ctx);

    fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut ctx);

    fsm.handle_with_context(
        &CursorEvent::MouseMoved {
            delta_x: 100.0,
            delta_y: -100.0,
        },
        &mut ctx,
    );

    let sector = fsm.inner().current_sector.expect("섹터가 산출되어야 함");
    assert_eq!(sector, 7, "오른쪽위 대각선은 섹터 7");
}
