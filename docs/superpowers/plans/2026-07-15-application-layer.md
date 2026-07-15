# Rectangle Win — 애플리케이션 계층 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 도메인 코어 위에 application 계층(port traits + services)을 구현한다. Win32 없이 mock으로 단위 테스트 가능한 유스케이스 오케스트레이션 계층.

**Architecture:** Clean Architecture의 `application/` 계층. domain 계층에만 의존. port traits를 정의하고, 이를 사용하는 서비스 로직을 구현. 인프라(Win32) 계층은 아직 없으므로 테스트는 mock 구현체로 검증.

**Tech Stack:** Rust 2024 edition, domain 계층 (이미 구현됨), thiserror, parking_lot (동시성 — 서비스 상태 보호용).

---

## 파일 구조

```
src-tauri/src/
├── application/
│   ├── mod.rs                 # 모듈 re-export
│   ├── ports.rs               # trait: WindowMover, MonitorProvider, ConfigStore, OverlayController
│   ├── errors.rs              # ApplicationError (thiserror)
│   ├── snap_service.rs        # throw 오케스트레이션 (FSM 이벤트 → 섹터 → snap 실행)
│   ├── keyboard_service.rs    # 방향키 체인 순환 → snap 실행
│   └── mock.rs                # 테스트용 mock 구현체 (ports 구현)
```

각 파일의 책임:
- `ports.rs`: 인프라가 구현할 trait들. domain 모델만 사용. Send + Sync.
- `errors.rs`: ApplicationError — DomainError 전파 + 서비스 고유 에러.
- `snap_service.rs`: Window Throw 오케스트레이션. FSM 이벤트 수신 → 섹터 산출 → SnapTarget 조회 → WindowMover/OverlayController 호출.
- `keyboard_service.rs`: Keyboard Snap 오케스트레이션. 방향키 이벤트 → 체인 순환 → SnapTarget 조회 → WindowMover 호출.
- `mock.rs`: ports의 mock 구현체. 단위 테스트에서 사용.

---

## Task 1: 애플리케이션 에러 및 포트 traits

**Files:**
- Create: `src-tauri/src/application/mod.rs`
- Create: `src-tauri/src/application/errors.rs`
- Create: `src-tauri/src/application/ports.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: `src-tauri/src/application/errors.rs` 작성**

```rust
use thiserror::Error;

use crate::domain::errors::DomainError;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error("configuration not loaded")]
    ConfigNotLoaded,

    #[error("window operation failed: {0}")]
    WindowOperation(String),

    #[error("overlay operation failed: {0}")]
    OverlayOperation(String),

    #[error("no foreground window")]
    NoForegroundWindow,
}

pub type AppResult<T> = Result<T, ApplicationError>;
```

- [ ] **Step 2: `src-tauri/src/application/ports.rs` 작성**

```rust
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
    fn get_window_rect(&self, window_handle: u64) -> AppResult<crate::domain::geometry::MonitorBounds>;
}

/// 모니터 정보 조회
pub trait MonitorProvider: Send + Sync {
    /// 모든 모니터 목록
    fn enumerate(&self) -> Vec<MonitorBounds>;

    /// 지정된 픽셀 좌표가 속한 모니터
    fn monitor_at(&self, x: i32, y: i32) -> MonitorBounds;

    /// 커서 위치가 속한 모니터 (편의 메서드)
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
    /// reticle 표시 (파이 차트). center는 모니터 중앙 픽셀 좌표.
    fn show_reticle(&self, center_x: i32, center_y: i32, sector_count: u8) -> AppResult<()>;

    /// 커서 포인터(빨간 반투명 원) 위치 갱신
    fn update_cursor_indicator(&self, x: i32, y: i32) -> AppResult<()>;

    /// 활성 섹터 하이라이트 갱신
    fn highlight_sector(&self, sector: u8) -> AppResult<()>;

    /// snap 미리보기 영역 표시
    fn show_snap_preview(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> AppResult<()>;

    /// 오버레이 전체 숨김
    fn hide(&self) -> AppResult<()>;
}
```

- [ ] **Step 3: `src-tauri/src/application/mod.rs` 작성**

```rust
pub mod errors;
pub mod ports;
```

- [ ] **Step 4: `src-tauri/src/lib.rs` 업데이트**

```rust
pub mod application;
pub mod domain;
```

- [ ] **Step 5: 빌드 확인**

Run: `cargo build`
Expected: 컴파일 성공 (geometry 타입 참조가 정상)

- [ ] **Step 6: 커밋**

```bash
git add -A
git commit -m "feat: 애플리케이션 계층 포트 traits 및 에러 정의"
```

---

## Task 2: Mock 구현체 (테스트용)

**Files:**
- Create: `src-tauri/src/application/mock.rs`
- Modify: `src-tauri/src/application/mod.rs`

- [ ] **Step 1: `src-tauri/src/application/mock.rs` 작성**

ports의 모든 trait을 구현하는 mock. 단위 테스트에서 서비스 로직 검증용.

```rust
use std::sync::Mutex;

use crate::application::errors::AppResult;
use crate::application::ports::{ConfigStore, MonitorProvider, OverlayController, WindowMover};
use crate::domain::geometry::MonitorBounds;
use crate::domain::model::{Config, SnapTarget, WindowAction};

/// WindowMover mock — 호출 기록을 저장하여 검증
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

/// MonitorProvider mock
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

/// ConfigStore mock
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

/// OverlayController mock
#[derive(Debug, Default)]
pub struct MockOverlayController {
    pub visible: Mutex<bool>,
    pub last_cursor: Mutex<Option<(i32, i32)>>,
    pub last_sector: Mutex<Option<u8>>,
}

impl OverlayController for MockOverlayController {
    fn show_reticle(&self, _cx: i32, _cy: i32, _count: u8) -> AppResult<()> {
        *self.visible.lock().unwrap() = true;
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

    fn show_snap_preview(&self, _x: i32, _y: i32, _w: i32, _h: i32) -> AppResult<()> {
        Ok(())
    }

    fn hide(&self) -> AppResult<()> {
        *self.visible.lock().unwrap() = false;
        Ok(())
    }
}
```

- [ ] **Step 2: `src-tauri/src/application/mod.rs` 업데이트**

```rust
pub mod errors;
pub mod ports;

#[cfg(test)]
pub mod mock;
```

- [ ] **Step 3: 빌드 확인**

Run: `cargo build`
Expected: 컴파일 성공

- [ ] **Step 4: 커밋**

```bash
git add -A
git commit -m "test: 애플리케이션 포트 mock 구현체 추가"
```

---

## Task 3: SnapService (Window Throw 오케스트레이션)

**Files:**
- Create: `src-tauri/src/application/snap_service.rs`
- Modify: `src-tauri/src/application/mod.rs`

- [ ] **Step 1: `src-tauri/src/application/snap_service.rs` 작성**

Window Throw의 전체 흐름을 오케스트레이션:
1. modifier down → 오버레이 표시
2. mouse move → 섹터 산출, 오버레이 갱신
3. modifier up → snap 실행, 오버레이 숨김

```rust
use std::sync::Arc;

use parking_lot::Mutex;

use crate::application::errors::{AppResult, ApplicationError};
use crate::application::ports::{
    ConfigStore, MonitorProvider, OverlayController, WindowMover,
};
use crate::domain::cursor_fsm::{CursorEvent, CursorFsm, FsmContext};
use crate::domain::geometry::{self, MonitorBounds};
use crate::domain::model::SnapTarget;

/// Window Throw 서비스 — FSM과 ports를 조율
pub struct SnapService {
    window_mover: Arc<dyn WindowMover>,
    monitor_provider: Arc<dyn MonitorProvider>,
    overlay: Arc<dyn OverlayController>,
    config_store: Arc<dyn ConfigStore>,
    /// FSM 상태 (shared storage + state machine)
    fsm: Mutex<CursorFsm>,
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
            fsm: Mutex::new(CursorFsm::default()),
        }
    }

    fn make_context(&self, sector_count: u8) -> FsmContext {
        FsmContext {
            sector_count,
            compute_sector: |dx, dy| {
                geometry::compute_sector(euclid::Vector2D::new(dx, dy), 8)
            },
            compute_distance: |dx, dy| {
                geometry::throw_distance(euclid::Vector2D::new(dx, dy))
            },
        }
    }

    /// modifier 키 눌림 처리
    pub fn on_modifier_pressed(
        &self,
        cursor_x: i32,
        cursor_y: i32,
    ) -> AppResult<()> {
        let config = self.config_store.load()?;
        let monitor = self.monitor_provider.monitor_at(cursor_x, cursor_y);
        let center = monitor.center();

        let ctx = self.make_context(config.overlay.sector_count);
        let mut guard = self.fsm.lock();
        // statig: init_with_context로 state machine 생성 후 handle
        let mut sm = guard.clone().uninitialized_state_machine().init_with_context(&mut ctx.clone());
        sm.handle_with_context(&CursorEvent::ModifierPressed, &mut ctx.clone());
        *guard = sm.inner().clone();
        drop(guard);

        self.overlay.show_reticle(center.x, center.y, config.overlay.sector_count)?;
        Ok(())
    }

    /// 마우스 이동 처리
    pub fn on_mouse_moved(
        &self,
        cursor_x: i32,
        cursor_y: i32,
        delta_x: f64,
        delta_y: f64,
    ) -> AppResult<()> {
        let config = self.config_store.load()?;
        let ctx = self.make_context(config.overlay.sector_count);

        let mut guard = self.fsm.lock();
        let mut sm = guard.clone().uninitialized_state_machine().init_with_context(&mut ctx.clone());
        sm.handle_with_context(
            &CursorEvent::MouseMoved { delta_x, delta_y },
            &mut ctx.clone(),
        );
        let new_state = sm.inner().clone();
        *guard = new_state;
        drop(guard);

        // 오버레이 갱신
        if config.overlay.cursor_indicator {
            self.overlay.update_cursor_indicator(cursor_x, cursor_y)?;
        }
        if let Some(sector) = new_state.current_sector {
            self.overlay.highlight_sector(sector)?;
        }

        Ok(())
    }

    /// modifier 키 뗌 → snap 실행
    pub fn on_modifier_released(
        &self,
        cancel: bool,
        cursor_x: i32,
        cursor_y: i32,
    ) -> AppResult<()> {
        let config = self.config_store.load()?;
        let ctx = self.make_context(config.overlay.sector_count);

        // snap 전 섹터 캡처 (release 후 FSM이 클리어하므로)
        let sector_before = self.fsm.lock().current_sector;

        let mut guard = self.fsm.lock();
        let mut sm = guard.clone().uninitialized_state_machine().init_with_context(&mut ctx.clone());
        sm.handle_with_context(
            &CursorEvent::ModifierReleased { cancel },
            &mut ctx.clone(),
        );
        *guard = sm.inner().clone();
        drop(guard);

        self.overlay.hide()?;

        if cancel || sector_before.is_none() {
            return Ok(());
        }

        // snap 실행
        let sector = sector_before.unwrap();
        let is_long_throw = config.throw.long_throw_enabled
            && self.fsm.lock().throw_distance > config.throw.long_throw_distance as f64;

        let mapping = if is_long_throw {
            &config.throw.long_throw_mapping
        } else {
            &config.throw.mapping
        };

        let target_id = mapping.get(&sector).ok_or_else(|| {
            ApplicationError::Domain(crate::domain::errors::DomainError::TargetNotFound(
                format!("sector {}", sector),
            ))
        })?;

        // snap pool에서 타겟 찾기
        let target = config
            .snap
            .areas
            .iter()
            .find(|t| t.id() == target_id)
            .ok_or_else(|| {
                ApplicationError::Domain(crate::domain::errors::DomainError::TargetNotFound(
                    target_id.clone(),
                ))
            })?;

        let window = self
            .window_mover
            .get_foreground_window()
            .ok_or(ApplicationError::NoForegroundWindow)?;
        let monitor = self.monitor_provider.monitor_at(cursor_x, cursor_y);

        self.window_mover
            .apply_snap_target(window, target, &monitor)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // 테스트는 Task 5에서 작성 (mock 구현체 사용)
}
```

**NOTE**: 이 구현은 FSM을 매 이벤트마다 clone→init→handle→clone back 하는 패턴을 사용합니다. 이는 statig state machine이 shared storage를 소유하기 때문입니다. 더 효율적인 방법은 state machine을 Mutex 안에 직접 보관하는 것이지만, statig의 `InitializedStateMachine`이 `CursorFsm`과 별개 타입이므로 이 방식이 더 단순합니다 (KISS).

- [ ] **Step 2: `src-tauri/src/application/mod.rs` 업데이트**

```rust
pub mod errors;
pub mod ports;
pub mod snap_service;

#[cfg(test)]
pub mod mock;
```

- [ ] **Step 3: `src-tauri/Cargo.toml`에 parking_lot 추가**

```toml
parking_lot = "0.12"
```

- [ ] **Step 4: 빌드 확인**

Run: `cargo build`
Expected: 컴파일 성공

- [ ] **Step 5: 커밋**

```bash
git add -A
git commit -m "feat: SnapService - Window Throw 오케스트레이션 (FSM + ports)"
```

---

## Task 4: KeyboardService (키보드 스냅 오케스트레이션)

**Files:**
- Create: `src-tauri/src/application/keyboard_service.rs`
- Modify: `src-tauri/src/application/mod.rs`

- [ ] **Step 1: `src-tauri/src/application/keyboard_service.rs` 작성**

```rust
use std::sync::Arc;

use parking_lot::Mutex;

use crate::application::errors::{AppResult, ApplicationError};
use crate::application::ports::{ConfigStore, MonitorProvider, WindowMover};
use crate::domain::geometry::MonitorBounds;
use crate::domain::keyboard_chain::ChainCycle;
use crate::domain::model::Direction;

/// Keyboard Snap 서비스 — 방향키 체인 순환 + snap 실행
pub struct KeyboardService {
    window_mover: Arc<dyn WindowMover>,
    monitor_provider: Arc<dyn MonitorProvider>,
    config_store: Arc<dyn ConfigStore>,
    cycle: Mutex<ChainCycle>,
}

impl KeyboardService {
    pub fn new(
        window_mover: Arc<dyn WindowMover>,
        monitor_provider: Arc<dyn MonitorProvider>,
        config_store: Arc<dyn ConfigStore>,
    ) -> Self {
        let config = config_store.load().unwrap_or_default();
        let timeout = config.keyboard.cycle_timeout_ms;
        Self {
            window_mover,
            monitor_provider,
            config_store,
            cycle: Mutex::new(ChainCycle::new(timeout)),
        }
    }

    /// 방향키 눌림 처리 → 체인 순환 → snap 실행
    pub fn on_direction_key(
        &self,
        direction: Direction,
        cursor_x: i32,
        cursor_y: i32,
    ) -> AppResult<()> {
        let config = self.config_store.load()?;
        if !config.keyboard.enabled {
            return Ok(());
        }

        let window = self
            .window_mover
            .get_foreground_window()
            .ok_or(ApplicationError::NoForegroundWindow)?;

        // 방향에 맞는 체인 선택
        let chain = if direction.is_horizontal() {
            &config.keyboard.chains.horizontal
        } else if direction.is_vertical() {
            &config.keyboard.chains.vertical
        } else {
            // 대각선은 현재 미지원 (필요시 별도 체인 추가)
            return Ok(());
        };

        let mut cycle = self.cycle.lock();
        let idx = cycle.next_index(direction, window, chain)?;
        drop(cycle);

        let target_id = &chain[idx];

        // snap pool에서 타겟 찾기
        let target = config
            .snap
            .areas
            .iter()
            .find(|t| t.id() == target_id)
            .ok_or_else(|| {
                ApplicationError::Domain(crate::domain::errors::DomainError::TargetNotFound(
                    target_id.clone(),
                ))
            })?;

        let monitor = self.monitor_provider.monitor_at(cursor_x, cursor_y);
        self.window_mover
            .apply_snap_target(window, target, &monitor)?;

        Ok(())
    }

    /// 창 포커스 변경시 체인 상태 리셋
    pub fn reset_cycle(&self) {
        self.cycle.lock().reset();
    }
}

#[cfg(test)]
mod tests {
    // 테스트는 Task 5에서 작성
}
```

- [ ] **Step 2: `src-tauri/src/application/mod.rs` 업데이트**

```rust
pub mod errors;
pub mod ports;
pub mod snap_service;
pub mod keyboard_service;

#[cfg(test)]
pub mod mock;
```

- [ ] **Step 3: 빌드 확인**

Run: `cargo build`
Expected: 컴파일 성공

- [ ] **Step 4: 커밋**

```bash
git add -A
git commit -m "feat: KeyboardService - 방향키 체인 순환 및 snap 실행"
```

---

## Task 5: 서비스 단위 테스트 (mock 사용)

**Files:**
- Modify: `src-tauri/src/application/snap_service.rs` (tests 모듈 채우기)
- Modify: `src-tauri/src/application/keyboard_service.rs` (tests 모듈 채우기)

- [ ] **Step 1: `snap_service.rs` tests 모듈 작성**

`snap_service.rs`의 `#[cfg(test)] mod tests`를 다음으로 교체:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::mock::{
        MockConfigStore, MockMonitorProvider, MockOverlayController, MockWindowMover,
    };
    use crate::domain::model::*;

    fn make_service() -> (
        SnapService,
        Arc<MockWindowMover>,
        Arc<MockOverlayController>,
    ) {
        let window_mover = Arc::new(MockWindowMover::new());
        let monitor = Arc::new(MockMonitorProvider::default());
        let overlay = Arc::new(MockOverlayController::default());
        let config_store = Arc::new(MockConfigStore::default());

        // 기본 설정에 snap 영역과 매핑 추가
        {
            let mut cfg = config_store.config.lock().unwrap();
            cfg.snap.areas = vec![
                SnapTarget::Area {
                    id: "right-half".into(),
                    name: "Right Half".into(),
                    x_ratio: 0.5,
                    y_ratio: 0.0,
                    w_ratio: 0.5,
                    h_ratio: 1.0,
                },
                SnapTarget::Action {
                    id: "maximize".into(),
                    name: "Maximize".into(),
                    action: WindowAction::Maximize,
                },
            ];
            cfg.throw.mapping.insert(0, "right-half".into()); // sector 0 = right
            cfg.throw.mapping.insert(6, "maximize".into()); // sector 6 = up
        }

        window_mover.set_foreground(42);

        let service = SnapService::new(
            window_mover.clone(),
            monitor,
            overlay.clone(),
            config_store,
        );
        (service, window_mover, overlay)
    }

    #[test]
    fn modifier_press_shows_overlay() {
        let (service, _wm, overlay) = make_service();
        service.on_modifier_pressed(960, 540).unwrap();
        assert!(*overlay.visible.lock().unwrap());
    }

    #[test]
    fn modifier_release_hides_overlay() {
        let (service, _wm, overlay) = make_service();
        service.on_modifier_pressed(960, 540).unwrap();
        service.on_modifier_released(true, 960, 540).unwrap();
        assert!(!*overlay.visible.lock().unwrap());
    }

    #[test]
    fn throw_right_snaps_to_right_half() {
        let (service, wm, _overlay) = make_service();
        service.on_modifier_pressed(960, 540).unwrap();
        service.on_mouse_moved(1060, 540, 100.0, 0.0).unwrap();
        service.on_modifier_released(false, 1060, 540).unwrap();

        let calls = wm.snap_calls();
        assert!(!calls.is_empty());
        // 첫 번째(이자 유일한) snap 호출이 right-half여야 함
        if let crate::application::mock::MockWindowCall::ApplySnap { target_id, .. } = &calls[0] {
            assert_eq!(target_id, "right-half");
        } else {
            panic!("expected ApplySnap call");
        }
    }

    #[test]
    fn cancel_does_not_snap() {
        let (service, wm, _overlay) = make_service();
        service.on_modifier_pressed(960, 540).unwrap();
        service.on_mouse_moved(1060, 540, 100.0, 0.0).unwrap();
        service.on_modifier_released(true, 1060, 540).unwrap();

        let calls = wm.snap_calls();
        assert!(calls.is_empty(), "cancel should not produce snap calls");
    }

    #[test]
    fn release_without_move_does_not_snap() {
        let (service, wm, _overlay) = make_service();
        service.on_modifier_pressed(960, 540).unwrap();
        service.on_modifier_released(false, 960, 540).unwrap();

        let calls = wm.snap_calls();
        assert!(calls.is_empty());
    }
}
```

- [ ] **Step 2: `keyboard_service.rs` tests 모듈 작성**

`keyboard_service.rs`의 `#[cfg(test)] mod tests`를 다음으로 교체:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::mock::{
        MockConfigStore, MockMonitorProvider, MockWindowMover,
    };
    use crate::domain::model::*;
    use std::sync::Arc;

    fn make_service() -> (KeyboardService, Arc<MockWindowMover>) {
        let window_mover = Arc::new(MockWindowMover::new());
        let monitor = Arc::new(MockMonitorProvider::default());
        let config_store = Arc::new(MockConfigStore::default());

        {
            let mut cfg = config_store.config.lock().unwrap();
            cfg.snap.areas = vec![
                SnapTarget::Area {
                    id: "left-half".into(),
                    name: "Left Half".into(),
                    x_ratio: 0.0,
                    y_ratio: 0.0,
                    w_ratio: 0.5,
                    h_ratio: 1.0,
                },
                SnapTarget::Area {
                    id: "third-left".into(),
                    name: "Left Third".into(),
                    x_ratio: 0.0,
                    y_ratio: 0.0,
                    w_ratio: 0.333,
                    h_ratio: 1.0,
                },
                SnapTarget::Action {
                    id: "maximize".into(),
                    name: "Maximize".into(),
                    action: WindowAction::Maximize,
                },
                SnapTarget::Action {
                    id: "almost-maximize".into(),
                    name: "Almost Maximize".into(),
                    action: WindowAction::AlmostMaximize,
                },
            ];
            cfg.keyboard.chains.horizontal = vec![
                "left-half".into(),
                "third-left".into(),
            ];
            cfg.keyboard.chains.vertical = vec![
                "maximize".into(),
                "almost-maximize".into(),
            ];
        }

        window_mover.set_foreground(42);

        let service = KeyboardService::new(
            window_mover.clone(),
            monitor,
            config_store,
        );
        (service, window_mover)
    }

    #[test]
    fn first_right_tap_snaps_left_half() {
        let (service, wm) = make_service();
        // Right = forward, 첫 탭 = index 0 = "left-half"
        service.on_direction_key(Direction::Right, 960, 540).unwrap();

        let calls = wm.snap_calls();
        assert_eq!(calls.len(), 1);
        if let crate::application::mock::MockWindowCall::ApplySnap { target_id, .. } = &calls[0] {
            assert_eq!(target_id, "left-half");
        }
    }

    #[test]
    fn second_right_tap_advances_to_third_left() {
        let (service, wm) = make_service();
        service.on_direction_key(Direction::Right, 960, 540).unwrap();
        service.on_direction_key(Direction::Right, 960, 540).unwrap();

        let calls = wm.snap_calls();
        assert_eq!(calls.len(), 2);
        if let crate::application::mock::MockWindowCall::ApplySnap { target_id, .. } = &calls[1] {
            assert_eq!(target_id, "third-left");
        }
    }

    #[test]
    fn vertical_down_snaps_maximize() {
        let (service, wm) = make_service();
        service.on_direction_key(Direction::Down, 960, 540).unwrap();

        let calls = wm.snap_calls();
        assert_eq!(calls.len(), 1);
        if let crate::application::mock::MockWindowCall::ApplySnap { target_id, .. } = &calls[0] {
            assert_eq!(target_id, "maximize");
        }
    }

    #[test]
    fn reset_cycle_starts_over() {
        let (service, wm) = make_service();
        service.on_direction_key(Direction::Right, 960, 540).unwrap();
        service.on_direction_key(Direction::Right, 960, 540).unwrap();
        service.reset_cycle();
        service.on_direction_key(Direction::Right, 960, 540).unwrap();

        let calls = wm.snap_calls();
        // reset 후 첫 탭 = index 0 = "left-half"
        if let crate::application::mock::MockWindowCall::ApplySnap { target_id, .. } = &calls[2] {
            assert_eq!(target_id, "left-half");
        }
    }
}
```

- [ ] **Step 3: 전체 테스트 실행**

Run: `cargo test`
Expected: 모든 테스트 PASS (도메인 50 + 통합 5 + 애플리케이션 신규)

- [ ] **Step 4: 커밋**

```bash
git add -A
git commit -m "test: SnapService 및 KeyboardService 단위 테스트 (mock 사용)"
```

---

## Self-Review 결과

**1. Spec coverage:**
- ✅ WindowMover port (apply_snap_target, get_foreground_window) → Task 1
- ✅ MonitorProvider port (enumerate, monitor_at) → Task 1
- ✅ ConfigStore port (load, save, path) → Task 1
- ✅ OverlayController port (show_reticle, update_cursor, highlight, hide) → Task 1
- ✅ SnapService (FSM → sector → snap) → Task 3
- ✅ KeyboardService (chain cycle → snap) → Task 4
- ✅ Mock implementations → Task 2
- ✅ 단위 테스트 → Task 5

**2. Placeholder scan:** 없음. 모든 단계에 실제 코드 포함.

**3. Type consistency:**
- `SnapTarget`, `WindowAction` — domain에서 정의, application에서 일관 사용 ✅
- `ChainCycle::next_index` — domain에서 정의, KeyboardService에서 동일 시그니처 사용 ✅
- `CursorFsm`, `FsmContext`, `CursorEvent` — domain에서 정의, SnapService에서 일관 사용 ✅
- `MonitorBounds`, `compute_sector`, `throw_distance` — geometry에서 정의, application에서 참조 ✅
- port traits의 메서드명이 mock과 service에서 일관됨 ✅

**주의사항**: SnapService의 FSM clone→init→handle 패턴은 statig의 타입 제약 때문입니다. 실제 사용시 매 이벤트마다 FSM을 재생성하므로 상태가 올바르게 복원되는지 Task 5 테스트에서 검증합니다. 만약 테스트가 실패하면 FSM을 Mutex<InitializedStateMachine<...>>로 보관하는 방식으로 리팩토링 필요.
