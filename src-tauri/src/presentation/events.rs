//! 오버레이 창(vue-konva)로 전달되는 Tauri 이벤트 페이로드.
//!
//! `OverlayController` 의 세분화된 상태 갱신을 프론트엔드 이벤트로 매핑한다.
//! `#[serde(tag = "type")]` 으로 태그된 union 직렬화를 사용해
//! 프론트엔드에서 `event.payload.type` 으로 분기할 수 있게 한다.

use serde::{Deserialize, Serialize};

/// 오버레이에 전달되는 이벤트.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OverlayEvent {
    /// 조준점 표시 — 섹터 피(pie) 렌더링 시작.
    Show {
        center_x: i32,
        center_y: i32,
        sector_count: u8,
    },
    /// 커서 위치 갱신.
    CursorUpdate { x: i32, y: i32 },
    /// 활성 섹터 하이라이트.
    SectorHighlight { sector: u8 },
    /// 스냅 미리보기 사각형 (픽셀 좌표).
    SnapPreview {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    /// 오버레이 숨김.
    Hide,
}
