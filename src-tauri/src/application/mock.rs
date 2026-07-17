use std::sync::Mutex;

use crate::application::errors::AppResult;
use crate::application::ports::{ConfigStore, MonitorProvider, OverlayController, WindowMover};
use crate::domain::geometry::MonitorBounds;
use crate::domain::model::{Config, SnapTarget};

#[derive(Debug, Default)]
pub struct MockWindowMover {
    pub calls: Mutex<Vec<MockWindowCall>>,
    pub foreground_window: Mutex<Option<u64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MockWindowCall {
    ApplySnap {
        window: u64,
        target_id: String,
        is_action: bool,
    },
    GetRect { window: u64 },
}

impl MockWindowMover {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_foreground(&self, handle: u64) {
        *self.foreground_window.lock().unwrap() = Some(handle);
    }

    pub fn snap_calls(&self) -> Vec<MockWindowCall> {
        self.calls.lock().unwrap().clone()
    }

    /// ApplySnap 호출만 필터링 (GetRect 등 다른 호출 제외).
    /// on_modifier_pressed 가 lock-on 표시를 위해 get_window_rect 를 호출하므로,
    /// snap 실행 여부를 검증할 때는 이 메서드를 사용해야 한다.
    pub fn apply_snap_calls(&self) -> Vec<MockWindowCall> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| matches!(c, MockWindowCall::ApplySnap { .. }))
            .cloned()
            .collect()
    }
}

impl WindowMover for MockWindowMover {
    fn get_foreground_window(&self) -> Option<u64> {
        *self.foreground_window.lock().unwrap()
    }

    fn apply_snap_target(
        &self,
        window_handle: u64,
        target: &SnapTarget,
        _monitor: &MonitorBounds,
    ) -> AppResult<()> {
        let call = MockWindowCall::ApplySnap {
            window: window_handle,
            target_id: target.id().to_string(),
            is_action: !target.is_area(),
        };
        self.calls.lock().unwrap().push(call);
        Ok(())
    }

    fn get_window_rect(&self, window_handle: u64) -> AppResult<MonitorBounds> {
        self.calls.lock().unwrap().push(MockWindowCall::GetRect { window: window_handle });
        Ok(MonitorBounds::new(0, 0, 1920, 1080))
    }
}

#[derive(Debug)]
pub struct MockMonitorProvider {
    pub monitors: Vec<MonitorBounds>,
}

impl Default for MockMonitorProvider {
    fn default() -> Self {
        Self {
            monitors: vec![MonitorBounds::new(0, 0, 1920, 1080)],
        }
    }
}

impl MonitorProvider for MockMonitorProvider {
    fn enumerate(&self) -> Vec<MonitorBounds> {
        self.monitors.clone()
    }

    fn monitor_at(&self, x: i32, y: i32) -> MonitorBounds {
        for m in &self.monitors {
            let in_x = x >= m.origin.x && x < m.origin.x + m.width();
            let in_y = y >= m.origin.y && y < m.origin.y + m.height();
            if in_x && in_y {
                return *m;
            }
        }
        self.monitors[0]
    }
}

#[derive(Debug, Default)]
pub struct MockConfigStore {
    pub config: Mutex<Config>,
}

impl ConfigStore for MockConfigStore {
    fn load(&self) -> AppResult<Config> {
        Ok(self.config.lock().unwrap().clone())
    }

    fn save(&self, config: &Config) -> AppResult<()> {
        *self.config.lock().unwrap() = config.clone();
        Ok(())
    }

    fn path(&self) -> &std::path::Path {
        std::path::Path::new("mock_config.toml")
    }
}

#[derive(Debug, Default)]
pub struct MockOverlayController {
    pub visible: Mutex<bool>,
    pub last_cursor: Mutex<Option<(i32, i32)>>,
    pub last_sector: Mutex<Option<u8>>,
    /// 마지막으로 show_snap_preview 에 전달된 사각형.
    pub last_snap_preview: Mutex<Option<(i32, i32, i32, i32)>>,
}

impl OverlayController for MockOverlayController {
    fn show_reticle(&self, _cx: i32, _cy: i32, _count: u8) -> AppResult<()> {
        let mut s = self.visible.lock().unwrap();
        *s = true;
        drop(s);
        // Win32LayeredOverlay 와 동일: show_reticle 은 active_sector/snap_preview 클리어.
        *self.last_sector.lock().unwrap() = None;
        *self.last_snap_preview.lock().unwrap() = None;
        Ok(())
    }

    fn update_cursor_indicator(&self, x: i32, y: i32) -> AppResult<()> {
        *self.last_cursor.lock().unwrap() = Some((x, y));
        Ok(())
    }

    fn highlight_sector(&self, sector: u8) -> AppResult<()> {
        *self.last_sector.lock().unwrap() = Some(sector);
        Ok(())
    }

    fn show_snap_preview(&self, x: i32, y: i32, w: i32, h: i32) -> AppResult<()> {
        *self.last_snap_preview.lock().unwrap() = Some((x, y, w, h));
        Ok(())
    }

    fn hide(&self) -> AppResult<()> {
        *self.visible.lock().unwrap() = false;
        Ok(())
    }
}
