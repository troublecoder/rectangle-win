use crate::application::errors::AppResult;
use crate::domain::geometry::MonitorBounds;
use crate::domain::model::{Config, SnapTarget};

/// 전경창을 snap 영역으로 이동하거나 액션 실행
pub trait WindowMover: Send + Sync {
    /// 현재 전경창 핸들 반환 (u64로 표현)
    fn get_foreground_window(&self) -> Option<u64>;

    /// 전경창을 지정된 snap 타겟(영역 또는 액션)으로 이동/실행
    fn apply_snap_target(
        &self,
        window_handle: u64,
        target: &SnapTarget,
        monitor: &MonitorBounds,
    ) -> AppResult<()>;

    /// 전경창의 현재 Rect (픽셀)
    fn get_window_rect(&self, window_handle: u64) -> AppResult<MonitorBounds>;
}

/// 모니터 정보 조회
pub trait MonitorProvider: Send + Sync {
    fn enumerate(&self) -> Vec<MonitorBounds>;
    fn monitor_at(&self, x: i32, y: i32) -> MonitorBounds;
    fn monitor_at_cursor(&self, cursor_x: i32, cursor_y: i32) -> MonitorBounds {
        self.monitor_at(cursor_x, cursor_y)
    }
}

/// TOML 설정 저장소
pub trait ConfigStore: Send + Sync {
    fn load(&self) -> AppResult<Config>;
    fn save(&self, config: &Config) -> AppResult<()>;
    fn path(&self) -> &std::path::Path;
}

/// 오버레이 창 제어 (클릭스루 투명창 위 vue-konva)
pub trait OverlayController: Send + Sync {
    fn show_reticle(&self, center_x: i32, center_y: i32, sector_count: u8) -> AppResult<()>;
    fn update_cursor_indicator(&self, x: i32, y: i32) -> AppResult<()>;
    fn highlight_sector(&self, sector: u8) -> AppResult<()>;
    fn show_snap_preview(&self, x: i32, y: i32, width: i32, height: i32) -> AppResult<()>;
    fn hide(&self) -> AppResult<()>;
}
