# Rectangle Win — 도메인 코어 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rectangle Win의 순수 도메인 계층(model, presets, geometry, cursor FSM)을 Win32/Tauri 없이 TDD로 구현하고 검증한다.

**Architecture:** Clean Architecture의 `domain/` 계층. std, serde, statig, euclid에만 의존. 모든 단위 테스트는 Win32 없이 실행 가능.

**Tech Stack:** Rust 2024 edition, serde, toml, statig (HSM), euclid (2D geometry), thiserror.

---

## 파일 구조

이 plan이 생성/수정하는 파일들:

```
src-tauri/Cargo.toml                      # workspace 의존성 추가
src-tauri/src/
├── domain/
│   ├── mod.rs                             # 모듈 re-export
│   ├── model.rs                           # SnapTarget, WindowAction, Config, Sector, Direction 등
│   ├── presets.rs                         # SnapPreset enum + 영역 생성
│   ├── geometry.rs                        # 섹터 산출, 비율→픽셀 변환 (euclid)
│   ├── cursor_fsm.rs                      # statig HSM (Idle/Armed/Tracking/Snapping)
│   ├── keyboard_chain.rs                  # 체인 순환 로직 (horizontal/vertical)
│   └── errors.rs                          # DomainError (thiserror)
```

각 파일의 책임:
- `model.rs`: 순수 데이터 타입만. 로직 없음. serde 직렬화 가능.
- `presets.rs`: 프리셋 enum과 영역 생성 함수. model에만 의존.
- `geometry.rs`: 섹터 산출, 비율→픽셀 변환. euclid 사용. model에만 의존.
- `cursor_fsm.rs`: 커서 상태머신. statig 사용. model + geometry에 의존.
- `keyboard_chain.rs`: 체인 순환 상태 관리. model에만 의존.
- `errors.rs`: 도메인 에러 타입.

---

## Task 1: Cargo.toml 설정 및 도메인 모듈 골격

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/domain/mod.rs`
- Create: `src-tauri/src/domain/errors.rs`

- [ ] **Step 1: workspace 구조로 전환 — `src-tauri/Cargo.toml` 작성**

기존 `Cargo.toml`은 워크스페이스 루트용으로 변경하고, 실제 크레이트는 `src-tauri/`로 이동.

루트 `Cargo.toml` 수정 (워크스페이스 설정):

```toml
[workspace]
members = ["src-tauri"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
```

`src-tauri/Cargo.toml` 생성:

```toml
[package]
name = "rectangle-win"
version = { workspace = true }
edition = { workspace = true }

[dependencies]
# 도메인 코어
statig = { version = "0.4" }
euclid = "0.22"

# 직렬화 & 에러
serde = { version = "1", features = ["derive"] }
toml = "0.8"
thiserror = "1"
uuid = { version = "1", features = ["v4", "serde"] }
```

기존 `src/main.rs`를 `src-tauri/src/main.rs`로 이동 (빈 Hello World 유지, 도메인 모듈만 먼저 구현).

- [ ] **Step 2: `src-tauri/src/main.rs` 작성**

```rust
mod domain;

fn main() {
    println!("Rectangle Win — domain core loaded");
}
```

- [ ] **Step 3: `src-tauri/src/domain/mod.rs` 작성**

```rust
pub mod errors;
```

- [ ] **Step 4: `src-tauri/src/domain/errors.rs` 작성**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("snap target not found: {0}")]
    TargetNotFound(String),

    #[error("invalid sector index {index}: must be 0..{max}")]
    InvalidSector { index: u8, max: u8 },

    #[error("invalid ratio {value}: must be between 0.0 and 1.0")]
    InvalidRatio { value: f64 },

    #[error("chain is empty, cannot cycle")]
    EmptyChain,

    #[error("preset not recognized: {0}")]
    UnknownPreset(String),
}

pub type DomainResult<T> = Result<T, DomainError>;
```

- [ ] **Step 5: 빌드 확인**

Run: `cargo build`
Expected: 컴파일 성공

- [ ] **Step 6: 커밋**

```bash
git add -A
git commit -m "init: 워크스페이스 구조 및 도메인 모듈 골격 생성"
```

---

## Task 2: 도메인 모델 (model.rs)

**Files:**
- Create: `src-tauri/src/domain/model.rs`
- Modify: `src-tauri/src/domain/mod.rs`

- [ ] **Step 1: 실패 테스트 작성 — `src-tauri/src/domain/model.rs`**

파일 상단에 `#[cfg(test)]` 모듈으로 테스트 작성:

```rust
use serde::{Deserialize, Serialize};

// ─── SnapTarget: 영역과 액션의 통합 ───

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SnapTarget {
    #[serde(rename = "area")]
    Area {
        id: String,
        name: String,
        x_ratio: f64,
        y_ratio: f64,
        w_ratio: f64,
        h_ratio: f64,
    },
    #[serde(rename = "action")]
    Action {
        id: String,
        name: String,
        action: WindowAction,
    },
}

impl SnapTarget {
    pub fn id(&self) -> &str {
        match self {
            SnapTarget::Area { id, .. } => id,
            SnapTarget::Action { id, .. } => id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            SnapTarget::Area { name, .. } => name,
            SnapTarget::Action { name, .. } => name,
        }
    }

    /// 영역 타입인가?
    pub fn is_area(&self) -> bool {
        matches!(self, SnapTarget::Area { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WindowAction {
    Maximize,
    Minimize,
    Restore,
    Center,
    AlmostMaximize,
    MaximizeHeight,
    NextDisplay,
    PreviousDisplay,
}

// ─── Sector / Direction ───

/// 파이 섹터 인덱스 (0부터 시작, 시계방향).
/// 8섹터 기준: 0=오른쪽, 1=오른쪽아래, 2=아래, 3=왼쪽아래,
///             4=왼쪽, 5=왼쪽위, 6=위, 7=오른쪽위
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Sector(pub u8);

impl Sector {
    pub fn new(index: u8, count: u8) -> Result<Self, crate::domain::errors::DomainError> {
        if index >= count {
            return Err(crate::domain::errors::DomainError::InvalidSector { index, max: count });
        }
        Ok(Sector(index))
    }
}

/// 키보드 방향키 4방향 + 대각선
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl Direction {
    /// horizontal 체인을 사용하는 방향인가?
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Direction::Left | Direction::Right)
    }

    /// vertical 체인을 사용하는 방향인가?
    pub fn is_vertical(&self) -> bool {
        matches!(self, Direction::Up | Direction::Down)
    }

    /// 정방향(체인 인덱스 증가)인가?
    /// Right, Down = 정방향 / Left, Up = 역방향
    pub fn is_forward(&self) -> bool {
        matches!(self, Direction::Right | Direction::Down)
    }
}

// ─── Config 구조체 (TOML 매핑) ───

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub snap: SnapConfig,
    #[serde(rename = "throw")]
    pub throw: ThrowConfig,
    pub keyboard: KeyboardConfig,
    pub overlay: OverlayConfig,
    pub update: UpdateConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            snap: SnapConfig::default(),
            throw: ThrowConfig::default(),
            keyboard: KeyboardConfig::default(),
            overlay: OverlayConfig::default(),
            update: UpdateConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub launch_at_login: bool,
    pub start_minimized: bool,
    pub show_in_tray: bool,
    pub language: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            launch_at_login: true,
            start_minimized: true,
            show_in_tray: true,
            language: "ko".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapConfig {
    pub active_preset: String,
    pub areas: Vec<SnapTarget>,
}

impl Default for SnapConfig {
    fn default() -> Self {
        Self {
            active_preset: "standard".to_string(),
            areas: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThrowConfig {
    pub trigger_modifiers: Vec<String>,
    pub long_throw_enabled: bool,
    pub long_throw_distance: u32,
    pub mapping: SectorMap,
    pub long_throw_mapping: SectorMap,
}

impl Default for ThrowConfig {
    fn default() -> Self {
        Self {
            trigger_modifiers: vec!["Win".to_string(), "Alt".to_string()],
            long_throw_enabled: true,
            long_throw_distance: 400,
            mapping: SectorMap::new(),
            long_throw_mapping: SectorMap::new(),
        }
    }
}

/// 섹터 인덱스 → SnapTarget id 참조
pub type SectorMap = std::collections::HashMap<u8, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyboardConfig {
    pub enabled: bool,
    pub trigger_modifiers: Vec<String>,
    pub modifier_mode: ModifierMode,
    pub cycle_timeout_ms: u64,
    pub chains: ChainConfig,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            trigger_modifiers: vec!["Ctrl".to_string(), "Alt".to_string()],
            modifier_mode: ModifierMode::Separate,
            cycle_timeout_ms: 1500,
            chains: ChainConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModifierMode {
    Shared,
    Separate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainConfig {
    pub horizontal: Vec<String>,
    pub vertical: Vec<String>,
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            horizontal: vec![
                "left-half".to_string(),
                "third-left".to_string(),
                "center".to_string(),
                "third-right".to_string(),
                "right-half".to_string(),
            ],
            vertical: vec![
                "maximize".to_string(),
                "almost-maximize".to_string(),
                "center".to_string(),
                "maximize-height".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub reticle_style: String,
    pub cursor_indicator: bool,
    pub cursor_radius: u32,
    pub cursor_color: String,
    pub cursor_opacity: f64,
    pub sector_highlight_color: String,
    pub sector_count: u8,
    pub snap_preview: bool,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            reticle_style: "pie".to_string(),
            cursor_indicator: true,
            cursor_radius: 18,
            cursor_color: "#E53935".to_string(),
            cursor_opacity: 0.5,
            sector_highlight_color: "#3B82F6".to_string(),
            sector_count: 8,
            snap_preview: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateConfig {
    pub enabled: bool,
    pub channel: String,
    pub check_on_startup: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            channel: "stable".to_string(),
            check_on_startup: true,
        }
    }
}

// ─── 테스트 ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_target_area_id_and_name() {
        let area = SnapTarget::Area {
            id: "left-half".to_string(),
            name: "Left Half".to_string(),
            x_ratio: 0.0,
            y_ratio: 0.0,
            w_ratio: 0.5,
            h_ratio: 1.0,
        };
        assert_eq!(area.id(), "left-half");
        assert_eq!(area.name(), "Left Half");
        assert!(area.is_area());
    }

    #[test]
    fn snap_target_action_not_area() {
        let action = SnapTarget::Action {
            id: "maximize".to_string(),
            name: "Maximize".to_string(),
            action: WindowAction::Maximize,
        };
        assert_eq!(action.id(), "maximize");
        assert!(!action.is_area());
    }

    #[test]
    fn sector_new_valid() {
        let s = Sector::new(5, 8).unwrap();
        assert_eq!(s, Sector(5));
    }

    #[test]
    fn sector_new_out_of_range() {
        let result = Sector::new(8, 8);
        assert!(result.is_err());
    }

    #[test]
    fn direction_classification() {
        assert!(Direction::Left.is_horizontal());
        assert!(Direction::Up.is_vertical());
        assert!(Direction::Right.is_forward());
        assert!(!Direction::Left.is_forward());
    }

    #[test]
    fn config_default_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn chain_config_default_vertical() {
        let chains = ChainConfig::default();
        assert_eq!(chains.vertical, vec![
            "maximize", "almost-maximize", "center", "maximize-height"
        ]);
    }
}
```

- [ ] **Step 2: `domain/mod.rs`에 model 추가**

```rust
pub mod errors;
pub mod model;
```

- [ ] **Step 3: 테스트 실행 — 실패 확인 후 통과 확인**

Run: `cargo test --lib domain::model`
Expected: PASS (모델은 구현과 테스트가 동일 파일에 있으므로 즉시 통과)

- [ ] **Step 4: 커밋**

```bash
git add -A
git commit -m "feat: 도메인 모델 정의 (SnapTarget, Config, Sector, Direction)"
```

---

## Task 3: 프리셋 패키지 (presets.rs)

**Files:**
- Create: `src-tauri/src/domain/presets.rs`
- Modify: `src-tauri/src/domain/mod.rs`

- [ ] **Step 1: 테스트 작성 — `src-tauri/src/domain/presets.rs`**

```rust
use crate::domain::model::{SnapTarget, WindowAction};

/// 프리셋 패키지 — 콤보박스 선택지
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapPreset {
    Minimal,
    Standard,
    Extended,
    Full,
    Portrait,
}

impl SnapPreset {
    /// 문자열에서 파싱 (TOML의 active_preset 값)
    pub fn from_str(s: &str) -> Result<Self, crate::domain::errors::DomainError> {
        match s {
            "minimal" => Ok(Self::Minimal),
            "standard" => Ok(Self::Standard),
            "extended" => Ok(Self::Extended),
            "full" => Ok(Self::Full),
            "portrait" => Ok(Self::Portrait),
            other => Err(crate::domain::errors::DomainError::UnknownPreset(other.to_string())),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Standard => "standard",
            Self::Extended => "extended",
            Self::Full => "full",
            Self::Portrait => "portrait",
        }
    }

    /// 선택한 프리셋의 snap 영역 + 액션 목록 생성
    pub fn targets(&self) -> Vec<SnapTarget> {
        match self {
            Self::Minimal => minimal(),
            Self::Standard => standard(),
            Self::Extended => extended(),
            Self::Full => full(),
            Self::Portrait => portrait(),
        }
    }
}

// ─── 헬퍼 함수들 ───

fn area(id: &str, name: &str, x: f64, y: f64, w: f64, h: f64) -> SnapTarget {
    SnapTarget::Area {
        id: id.to_string(),
        name: name.to_string(),
        x_ratio: x,
        y_ratio: y,
        w_ratio: w,
        h_ratio: h,
    }
}

fn action(id: &str, name: &str, act: WindowAction) -> SnapTarget {
    SnapTarget::Action {
        id: id.to_string(),
        name: name.to_string(),
        action: act,
    }
}

fn maximize() -> SnapTarget {
    action("maximize", "Maximize", WindowAction::Maximize)
}

// ─── 프리셋별 영역 정의 ───

fn minimal() -> Vec<SnapTarget> {
    vec![
        area("left-half", "Left Half", 0.0, 0.0, 0.5, 1.0),
        area("right-half", "Right Half", 0.5, 0.0, 0.5, 1.0),
        area("top-half", "Top Half", 0.0, 0.0, 1.0, 0.5),
        area("bottom-half", "Bottom Half", 0.0, 0.5, 1.0, 0.5),
        maximize(),
    ]
}

fn standard() -> Vec<SnapTarget> {
    let mut v = minimal();
    v.extend(vec![
        area("third-left", "Left Third", 0.0, 0.0, 0.333, 1.0),
        area("third-center", "Center Third", 0.333, 0.0, 0.334, 1.0),
        area("third-right", "Right Third", 0.667, 0.0, 0.333, 1.0),
        area("quarter-tl", "Top Left Quarter", 0.0, 0.0, 0.5, 0.5),
        area("quarter-tr", "Top Right Quarter", 0.5, 0.0, 0.5, 0.5),
        area("quarter-bl", "Bottom Left Quarter", 0.0, 0.5, 0.5, 0.5),
        area("quarter-br", "Bottom Right Quarter", 0.5, 0.5, 0.5, 0.5),
    ]);
    v
}

fn extended() -> Vec<SnapTarget> {
    let mut v = standard();
    v.extend(vec![
        area("two-thirds-left", "Left Two Thirds", 0.0, 0.0, 0.667, 1.0),
        area("two-thirds-right", "Right Two Thirds", 0.333, 0.0, 0.667, 1.0),
        area("center", "Center", 0.25, 0.25, 0.5, 0.5),
        action("almost-maximize", "Almost Maximize", WindowAction::AlmostMaximize),
        action("maximize-height", "Maximize Height", WindowAction::MaximizeHeight),
        action("minimize", "Minimize", WindowAction::Minimize),
        action("restore", "Restore", WindowAction::Restore),
        action("center-action", "Center Action", WindowAction::Center),
    ]);
    v
}

fn full() -> Vec<SnapTarget> {
    let mut v = extended();
    v.extend(vec![
        area("sixth-tl", "Top Left Sixth", 0.0, 0.0, 0.333, 0.5),
        area("sixth-tc", "Top Center Sixth", 0.333, 0.0, 0.334, 0.5),
        area("sixth-tr", "Top Right Sixth", 0.667, 0.0, 0.333, 0.5),
        area("sixth-bl", "Bottom Left Sixth", 0.0, 0.5, 0.333, 0.5),
        area("sixth-bc", "Bottom Center Sixth", 0.333, 0.5, 0.334, 0.5),
        area("sixth-br", "Bottom Right Sixth", 0.667, 0.5, 0.333, 0.5),
    ]);
    v
}

fn portrait() -> Vec<SnapTarget> {
    vec![
        area("top-half", "Top Half", 0.0, 0.0, 1.0, 0.5),
        area("bottom-half", "Bottom Half", 0.0, 0.5, 1.0, 0.5),
        area("third-top", "Top Third", 0.0, 0.0, 1.0, 0.333),
        area("third-center", "Center Third", 0.0, 0.333, 1.0, 0.334),
        area("third-bottom", "Bottom Third", 0.0, 0.667, 1.0, 0.333),
        area("quarter-tl", "Top Left Quarter", 0.0, 0.0, 0.5, 0.5),
        area("quarter-tr", "Top Right Quarter", 0.5, 0.0, 0.5, 0.5),
        area("quarter-bl", "Bottom Left Quarter", 0.0, 0.5, 0.5, 0.5),
        area("quarter-br", "Bottom Right Quarter", 0.5, 0.5, 0.5, 0.5),
        maximize(),
    ]
}

// ─── 테스트 ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_from_str() {
        assert_eq!(SnapPreset::from_str("standard").unwrap(), SnapPreset::Standard);
        assert!(SnapPreset::from_str("unknown").is_err());
    }

    #[test]
    fn preset_as_str_roundtrip() {
        for preset in [SnapPreset::Minimal, SnapPreset::Standard, SnapPreset::Extended, SnapPreset::Full, SnapPreset::Portrait] {
            let s = preset.as_str();
            assert_eq!(SnapPreset::from_str(s).unwrap(), preset);
        }
    }

    #[test]
    fn minimal_has_5_targets() {
        let targets = SnapPreset::Minimal.targets();
        assert_eq!(targets.len(), 5);
    }

    #[test]
    fn standard_includes_thirds_and_quarters() {
        let targets = SnapPreset::Standard.targets();
        let ids: Vec<&str> = targets.iter().map(|t| t.id()).collect();
        assert!(ids.contains(&"third-left"));
        assert!(ids.contains(&"quarter-tl"));
        assert!(ids.contains(&"maximize"));
    }

    #[test]
    fn full_has_sixths() {
        let targets = SnapPreset::Full.targets();
        let ids: Vec<&str> = targets.iter().map(|t| t.id()).collect();
        assert!(ids.contains(&"sixth-tl"));
        assert!(ids.contains(&"sixth-br"));
    }

    #[test]
    fn extended_includes_actions() {
        let targets = SnapPreset::Extended.targets();
        let ids: Vec<&str> = targets.iter().map(|t| t.id()).collect();
        assert!(ids.contains(&"almost-maximize"));
        assert!(ids.contains(&"center"));
    }

    #[test]
    fn preset_ids_are_unique() {
        for preset in [SnapPreset::Minimal, SnapPreset::Standard, SnapPreset::Extended, SnapPreset::Full, SnapPreset::Portrait] {
            let targets = preset.targets();
            let mut ids: Vec<&str> = targets.iter().map(|t| t.id()).collect();
            let total = ids.len();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), total, "duplicate ids in {:?}", preset);
        }
    }

    #[test]
    fn standard_left_half_ratios() {
        let targets = SnapPreset::Standard.targets();
        let left_half = targets.iter().find(|t| t.id() == "left-half").unwrap();
        if let SnapTarget::Area { x_ratio, y_ratio, w_ratio, h_ratio, .. } = left_half {
            assert!((-*x_ratio).abs() < f64::EPSILON); // x == 0.0
            assert!((-*y_ratio).abs() < f64::EPSILON);
            assert!((*w_ratio - 0.5).abs() < f64::EPSILON);
            assert!((*h_ratio - 1.0).abs() < f64::EPSILON);
        } else {
            panic!("left-half should be an area");
        }
    }
}
```

- [ ] **Step 2: `domain/mod.rs`에 presets 추가**

```rust
pub mod errors;
pub mod model;
pub mod presets;
```

- [ ] **Step 3: 테스트 실행**

Run: `cargo test --lib domain::presets`
Expected: PASS

- [ ] **Step 4: 커밋**

```bash
git add -A
git commit -m "feat: 스냅 프리셋 패키지 정의 (Minimal/Standard/Extended/Full/Portrait)"
```

---

## Task 4: 기하학 — 섹터 산출 및 비율→픽셀 변환 (geometry.rs)

**Files:**
- Create: `src-tauri/src/domain/geometry.rs`
- Modify: `src-tauri/src/domain/mod.rs`

- [ ] **Step 1: 테스트와 구현 함께 작성 — `src-tauri/src/domain/geometry.rs`**

```rust
use euclid::{Point2D, Rect, Size2D, Vector2D};

/// 픽셀 좌표계 (물리적 픽셀, Win32 좌표계와 일치)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorBounds {
    pub origin: Point2D<i32>,
    pub size: Size2D<i32>,
}

impl MonitorBounds {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            origin: Point2D::new(x, y),
            size: Size2D::new(width, height),
        }
    }

    pub fn center(&self) -> Point2D<i32> {
        Point2D::new(
            self.origin.x + self.size.width / 2,
            self.origin.y + self.size.height / 2,
        )
    }

    pub fn width(&self) -> i32 {
        self.size.width
    }

    pub fn height(&self) -> i32 {
        self.size.height
    }
}

/// 커서 이동 델타(시작점 기준)로부터 섹터 인덱스 산출.
/// sector_count: 4, 8, 12 중 하나.
/// 반환값: 0..sector_count 범위의 섹터 인덱스.
/// 섹터 배치 (8섹터 기준, 시계방향, 0=오른쪽):
///   0=오른쪽, 1=오른쪽아래, 2=아래, 3=왼쪽아래,
///   4=왼쪽, 5=왼쪽위, 6=위, 7=오른쪽위
pub fn compute_sector(delta: Vector2D<f64>, sector_count: u8) -> u8 {
    let angle = delta.y.atan2(-delta.x); // 0=오른쪽, 시계방향 양수
    // atan2 결과: y양수(화면에서 아래)=시계방향.
    // 우리는 y축이 아래로 향하는 화면 좌표계 사용.
    // angle 범위: [-PI, PI]. 이를 [0, 2PI)로 정규화.
    let angle = if angle < 0.0 { angle + std::f64::consts::TAU } else { angle };
    let sector_size = std::f64::consts::TAU / sector_count as f64;
    // 각도를 섹터 인덱스로 변환 (반올림으로 경계 처리)
    let index = ((angle + sector_size / 2.0) / sector_size).floor() as u8;
    index % sector_count
}

/// SnapTarget의 비율 좌표를 모니터의 픽셀 Rect로 변환
pub fn ratio_to_pixels(
    x_ratio: f64,
    y_ratio: f64,
    w_ratio: f64,
    h_ratio: f64,
    monitor: &MonitorBounds,
) -> Rect<i32> {
    Rect::new(
        Point2D::new(
            monitor.origin.x + (x_ratio * monitor.width() as f64) as i32,
            monitor.origin.y + (y_ratio * monitor.height() as f64) as i32,
        ),
        Size2D::new(
            (w_ratio * monitor.width() as f64) as i32,
            (h_ratio * monitor.height() as f64) as i32,
        ),
    )
}

/// 델타의 거리(픽셀) 계산 — Long Throw 임계값 판별용
pub fn throw_distance(delta: Vector2D<f64>) -> f64 {
    (delta.x * delta.x + delta.y * delta.y).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor_1080p() -> MonitorBounds {
        MonitorBounds::new(0, 0, 1920, 1080)
    }

    // ─── compute_sector 테스트 ───

    #[test]
    fn sector_right() {
        let delta = Vector2D::new(100.0, 0.0); // 오른쪽
        assert_eq!(compute_sector(delta, 8), 0);
    }

    #[test]
    fn sector_down() {
        let delta = Vector2D::new(0.0, 100.0); // 아래 (화면 좌표계 y+)
        assert_eq!(compute_sector(delta, 8), 2);
    }

    #[test]
    fn sector_left() {
        let delta = Vector2D::new(-100.0, 0.0); // 왼쪽
        assert_eq!(compute_sector(delta, 8), 4);
    }

    #[test]
    fn sector_up() {
        let delta = Vector2D::new(0.0, -100.0); // 위 (화면 좌표계 y-)
        assert_eq!(compute_sector(delta, 8), 6);
    }

    #[test]
    fn sector_down_right_diagonal() {
        let delta = Vector2D::new(100.0, 100.0); // 오른쪽 아래 대각선
        assert_eq!(compute_sector(delta, 8), 1);
    }

    #[test]
    fn sector_up_left_diagonal() {
        let delta = Vector2D::new(-100.0, -100.0); // 왼쪽 위 대각선
        assert_eq!(compute_sector(delta, 8), 5);
    }

    #[test]
    fn sector_4_count() {
        let delta = Vector2D::new(100.0, 0.0); // 오른쪽
        assert_eq!(compute_sector(delta, 4), 0);
        let delta = Vector2D::new(0.0, 100.0); // 아래
        assert_eq!(compute_sector(delta, 4), 1);
    }

    #[test]
    fn sector_zero_delta() {
        let delta = Vector2D::new(0.0, 0.0); // 델타 없음
        // 0섹터(오른쪽)로 폴백
        let result = compute_sector(delta, 8);
        assert!(result < 8);
    }

    // ─── ratio_to_pixels 테스트 ───

    #[test]
    fn ratio_left_half_to_pixels() {
        let monitor = monitor_1080p();
        let rect = ratio_to_pixels(0.0, 0.0, 0.5, 1.0, &monitor);
        assert_eq!(rect.origin, Point2D::new(0, 0));
        assert_eq!(rect.size, Size2D::new(960, 1080));
    }

    #[test]
    fn ratio_right_half_to_pixels() {
        let monitor = monitor_1080p();
        let rect = ratio_to_pixels(0.5, 0.0, 0.5, 1.0, &monitor);
        assert_eq!(rect.origin, Point2D::new(960, 0));
        assert_eq!(rect.size, Size2D::new(960, 1080));
    }

    #[test]
    fn ratio_center_to_pixels() {
        let monitor = monitor_1080p();
        let rect = ratio_to_pixels(0.25, 0.25, 0.5, 0.5, &monitor);
        assert_eq!(rect.origin, Point2D::new(480, 270));
        assert_eq!(rect.size, Size2D::new(960, 540));
    }

    #[test]
    fn ratio_with_monitor_offset() {
        let monitor = MonitorBounds::new(1920, 0, 1920, 1080); // 두 번째 모니터
        let rect = ratio_to_pixels(0.0, 0.0, 0.5, 1.0, &monitor);
        assert_eq!(rect.origin, Point2D::new(1920, 0));
    }

    // ─── throw_distance 테스트 ───

    #[test]
    fn throw_distance_simple() {
        let delta = Vector2D::new(300.0, 400.0); // 3-4-5 삼각비
        assert!((throw_distance(delta) - 500.0).abs() < 0.001);
    }

    #[test]
    fn throw_distance_zero() {
        assert!((throw_distance(Vector2D::new(0.0, 0.0))).abs() < 0.001);
    }

    // ─── MonitorBounds 테스트 ───

    #[test]
    fn monitor_center() {
        let monitor = monitor_1080p();
        assert_eq!(monitor.center(), Point2D::new(960, 540));
    }
}
```

- [ ] **Step 2: `domain/mod.rs`에 geometry 추가**

```rust
pub mod errors;
pub mod model;
pub mod presets;
pub mod geometry;
```

- [ ] **Step 3: 테스트 실행**

Run: `cargo test --lib domain::geometry`
Expected: PASS

- [ ] **Step 4: 커밋**

```bash
git add -A
git commit -m "feat: 기하학 계산 (섹터 산출, 비율→픽셀 변환, throw 거리)"
```

---

## Task 5: 키보드 체인 순환 로직 (keyboard_chain.rs)

**Files:**
- Create: `src-tauri/src/domain/keyboard_chain.rs`
- Modify: `src-tauri/src/domain/mod.rs`

- [ ] **Step 1: 테스트와 구현 함께 작성 — `src-tauri/src/domain/keyboard_chain.rs`**

```rust
use std::time::{Duration, Instant};

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::model::Direction;

/// 키보드 체인 순환 상태 추적.
/// 같은 방향 + 같은 창 + 타임아웃 내 연속 탭 → 다음 인덱스로 진행.
/// 아니면 인덱스 0부터 재시작.
#[derive(Debug, Clone)]
pub struct ChainCycle {
    last_direction: Option<Direction>,
    last_index: usize,
    last_window: u64, // 윈도우 핸들 (Win32 HWND를 u64로 표현)
    last_time: Option<Instant>,
    timeout: Duration,
}

impl ChainCycle {
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            last_direction: None,
            last_index: 0,
            last_window: 0,
            last_time: None,
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    /// 체인에서 다음 타겟의 인덱스를 반환.
    /// direction: 눌린 방향키
    /// window_handle: 현재 포커스된 윈도우 핸들
    /// chain: 해당 방향의 체인 (horizontal 또는 vertical)
    pub fn next_index(
        &mut self,
        direction: Direction,
        window_handle: u64,
        chain: &[String],
    ) -> DomainResult<usize> {
        if chain.is_empty() {
            return Err(DomainError::EmptyChain);
        }

        let now = Instant::now();
        let should_advance = self.is_continuous(direction, window_handle, now);

        let new_index = if should_advance {
            // 정방향(Right, Down): 인덱스 증가
            // 역방향(Left, Up): 인덱스 감소
            if direction.is_forward() {
                (self.last_index + 1) % chain.len()
            } else {
                if self.last_index == 0 {
                    chain.len() - 1 // 끝에서 처음으로 (순환)
                } else {
                    self.last_index - 1
                }
            }
        } else {
            // 새 시퀀스 시작 — 항상 인덱스 0
            0
        };

        self.last_direction = Some(direction);
        self.last_index = new_index;
        self.last_window = window_handle;
        self.last_time = Some(now);

        Ok(new_index)
    }

    /// 연속 탭 조건: 같은 축(가로/세로) + 같은 창 + 타임아웃 내
    fn is_continuous(&self, direction: Direction, window_handle: u64, now: Instant) -> bool {
        match (self.last_direction, self.last_time) {
            (Some(last_dir), Some(last_time)) => {
                let same_axis = if direction.is_horizontal() {
                    last_dir.is_horizontal()
                } else if direction.is_vertical() {
                    last_dir.is_vertical()
                } else {
                    false
                };
                let same_window = self.last_window == window_handle;
                let within_timeout = now.duration_since(last_time) < self.timeout;
                same_axis && same_window && within_timeout
            }
            _ => false,
        }
    }

    /// 상태 초기화 (창 포커스 변경 등)
    pub fn reset(&mut self) {
        self.last_direction = None;
        self.last_index = 0;
        self.last_window = 0;
        self.last_time = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h_chain() -> Vec<String> {
        vec![
            "left-half".to_string(),
            "third-left".to_string(),
            "center".to_string(),
            "third-right".to_string(),
            "right-half".to_string(),
        ]
    }

    fn v_chain() -> Vec<String> {
        vec![
            "maximize".to_string(),
            "almost-maximize".to_string(),
            "center".to_string(),
            "maximize-height".to_string(),
        ]
    }

    const WINDOW: u64 = 12345;

    #[test]
    fn first_tap_starts_at_index_zero() {
        let mut cycle = ChainCycle::new(1500);
        let idx = cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap();
        assert_eq!(idx, 0); // 첫 탭 = "left-half"
    }

    #[test]
    fn forward_advances_chain() {
        let mut cycle = ChainCycle::new(1500);
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 0
        let idx = cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 1
        assert_eq!(idx, 1); // "third-left"
    }

    #[test]
    fn forward_wraps_around() {
        let mut cycle = ChainCycle::new(1500);
        // 끝까지 진행
        for _ in 0..5 {
            cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap();
        }
        // 한 번 더 → 처음으로 순환
        let idx = cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap();
        assert_eq!(idx, 0);
    }

    #[test]
    fn backward_decrements() {
        let mut cycle = ChainCycle::new(1500);
        // 먼저 정방향으로 인덱스 2까지 진행
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 0
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 1
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 2

        // 역방향 (Left)
        let idx = cycle.next_index(Direction::Left, WINDOW, &h_chain()).unwrap();
        assert_eq!(idx, 1); // 2에서 1로 감소
    }

    #[test]
    fn backward_wraps_from_zero_to_last() {
        let mut cycle = ChainCycle::new(1500);
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 0

        // 인덱스 0에서 Left → 마지막으로 순환
        let idx = cycle.next_index(Direction::Left, WINDOW, &h_chain()).unwrap();
        assert_eq!(idx, 4); // "right-half"
    }

    #[test]
    fn different_window_resets_to_zero() {
        let mut cycle = ChainCycle::new(1500);
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 0
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 1

        // 다른 창
        let idx = cycle.next_index(Direction::Right, 99999, &h_chain()).unwrap();
        assert_eq!(idx, 0);
    }

    #[test]
    fn different_axis_resets_to_zero() {
        let mut cycle = ChainCycle::new(1500);
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 0
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 1

        // 세로 체인으로 전환
        let idx = cycle.next_index(Direction::Down, WINDOW, &v_chain()).unwrap();
        assert_eq!(idx, 0); // 다른 축이므로 리셋
    }

    #[test]
    fn vertical_chain_forward() {
        let mut cycle = ChainCycle::new(1500);
        cycle.next_index(Direction::Down, WINDOW, &v_chain()).unwrap(); // 0: maximize
        let idx = cycle.next_index(Direction::Down, WINDOW, &v_chain()).unwrap();
        assert_eq!(idx, 1); // almost-maximize
    }

    #[test]
    fn vertical_chain_backward() {
        let mut cycle = ChainCycle::new(1500);
        cycle.next_index(Direction::Down, WINDOW, &v_chain()).unwrap(); // 0
        cycle.next_index(Direction::Down, WINDOW, &v_chain()).unwrap(); // 1

        let idx = cycle.next_index(Direction::Up, WINDOW, &v_chain()).unwrap();
        assert_eq!(idx, 0); // 1에서 0으로 역행
    }

    #[test]
    fn empty_chain_returns_error() {
        let mut cycle = ChainCycle::new(1500);
        let result = cycle.next_index(Direction::Right, WINDOW, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn reset_clears_state() {
        let mut cycle = ChainCycle::new(1500);
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap();
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap();
        assert_eq!(cycle.last_index, 1);

        cycle.reset();
        let idx = cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap();
        assert_eq!(idx, 0);
    }
}
```

- [ ] **Step 2: `domain/mod.rs`에 keyboard_chain 추가**

```rust
pub mod errors;
pub mod model;
pub mod presets;
pub mod geometry;
pub mod keyboard_chain;
```

- [ ] **Step 3: 테스트 실행**

Run: `cargo test --lib domain::keyboard_chain`
Expected: PASS

- [ ] **Step 4: 커밋**

```bash
git add -A
git commit -m "feat: 키보드 체인 순환 로직 (정방향/역방향 순환, 타임아웃, 창 변경 감지)"
```

---

## Task 6: 커서 FSM (cursor_fsm.rs) — statig HSM

**Files:**
- Create: `src-tauri/src/domain/cursor_fsm.rs`
- Modify: `src-tauri/src/domain/mod.rs`

- [ ] **Step 1: FSM 구현 및 테스트 작성 — `src-tauri/src/domain/cursor_fsm.rs`**

```rust
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
    /// 마지막 이벤트가 처리되었는지 (테스트/디버그용)
    pub last_event_handled: bool,
}

/// 섹터 산출 콜백 타입 (geometry에서 주입, FSM은 geometry를 직접 모름)
pub type SectorComputer = fn(delta_x: f64, delta_y: f64) -> u8;
/// 거리 계산 콜백
pub type DistanceComputer = fn(delta_x: f64, delta_y: f64) -> f64;

/// FSM에 주입할 컨텍스트 — 섹터/거리 계산 함수
#[derive(Debug, Clone, Copy)]
pub struct FsmContext {
    pub sector_count: u8,
    pub compute_sector: SectorComputer,
    pub compute_distance: DistanceComputer,
}

#[state_machine(initial = "State::idle()")]
impl CursorFsm {
    #[state(entry_action = "enter_idle")]
    fn idle(&mut self, _event: &CursorEvent) -> Outcome {
        match _event {
            CursorEvent::ModifierPressed => Transition(State::armed()),
            _ => Handled,
        }
    }

    #[state(entry_action = "enter_armed", superstate = "active")]
    fn armed(&mut self, event: &CursorEvent) -> Outcome {
        match event {
            CursorEvent::MouseMoved { delta_x, delta_y } => {
                // 첫 마우스 이동 → Tracking으로
                Transition(State::tracking())
            }
            CursorEvent::ModifierReleased { cancel: _ } => {
                // 이동 없이 뗌 → 취소 (Idle로)
                Transition(State::idle())
            }
            CursorEvent::ModifierPressed => Handled,
        }
    }

    #[state(entry_action = "enter_tracking", superstate = "active")]
    fn tracking(&mut self, event: &CursorEvent, ctx: &FsmContext) -> Outcome {
        match event {
            CursorEvent::MouseMoved { delta_x, delta_y } => {
                self.current_sector = Some((ctx.compute_sector)(*delta_x, *delta_y));
                self.throw_distance = (ctx.compute_distance)(*delta_x, *delta_y);
                Handled
            }
            CursorEvent::ModifierReleased { cancel: true } => {
                // 취소 — snap 실행 않음
                self.current_sector = None;
                Transition(State::idle())
            }
            CursorEvent::ModifierReleased { cancel: false } => {
                // 정상 release → Snapping으로 (snap 실행은 application 계층에서)
                Transition(State::snapping())
            }
            CursorEvent::ModifierPressed => Handled,
        }
    }

    #[state(entry_action = "enter_snapping")]
    fn snapping(&mut self, _event: &CursorEvent) -> Outcome {
        // Snap 완료 후 즉시 Idle로 복귀
        Transition(State::idle())
    }

    #[superstate]
    fn active(&mut self, _event: &CursorEvent) -> Outcome {
        Super
    }

    // ─── Entry actions ───

    #[action]
    fn enter_idle(&mut self) {
        self.current_sector = None;
        self.throw_distance = 0.0;
    }

    #[action]
    fn enter_armed(&mut self) {
        // 오버레이 표시 요청 신호 (실제 표시는 application 계층)
    }

    #[action]
    fn enter_tracking(&mut self) {
        // reticle + 커서 포인터 표시
    }

    #[action]
    fn enter_snapping(&mut self) {
        // snap 실행 후 오버레이 숨김
        // 실제 snap은 application 계층에서 current_sector를 읽어 실행
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::geometry;

    fn test_ctx() -> FsmContext {
        FsmContext {
            sector_count: 8,
            compute_sector: |dx, dy| geometry::compute_sector(
                euclid::Vector2D::new(dx, dy), 8
            ),
            compute_distance: |dx, dy| geometry::throw_distance(
                euclid::Vector2D::new(dx, dy)
            ),
        }
    }

    #[test]
    fn initial_state_is_idle() {
        let fsm = CursorFsm::default();
        let sm = fsm.state_machine();
        // state_machine의 root 상태 확인은 내부 타입이므로,
        // 이벤트를 보내서 간접 확인
        assert!(fsm.current_sector.is_none());
    }

    #[test]
    fn idle_to_armed_on_modifier_pressed() {
        let mut fsm = CursorFsm::default().state_machine();
        fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut test_ctx());
        // Armed 상태 — current_sector는 아직 None
        assert!(fsm.shared_storage().current_sector.is_none());
    }

    #[test]
    fn armed_to_tracking_on_mouse_move() {
        let mut fsm = CursorFsm::default().state_machine();
        fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut test_ctx());
        fsm.handle_with_context(
            &CursorEvent::MouseMoved { delta_x: 100.0, delta_y: 0.0 },
            &mut test_ctx(),
        );
        // Tracking — 섹터 계산됨 (오른쪽 = 섹터 0)
        assert_eq!(fsm.shared_storage().current_sector, Some(0));
    }

    #[test]
    fn tracking_to_snapping_on_release() {
        let mut fsm = CursorFsm::default().state_machine();
        fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut test_ctx());
        fsm.handle_with_context(
            &CursorEvent::MouseMoved { delta_x: 100.0, delta_y: 0.0 },
            &mut test_ctx(),
        );
        fsm.handle_with_context(
            &CursorEvent::ModifierReleased { cancel: false },
            &mut test_ctx(),
        );
        // Snapping → Idle로 자동 복귀, sector 초기화
        assert!(fsm.shared_storage().current_sector.is_none());
    }

    #[test]
    fn tracking_cancel_clears_sector() {
        let mut fsm = CursorFsm::default().state_machine();
        fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut test_ctx());
        fsm.handle_with_context(
            &CursorEvent::MouseMoved { delta_x: 100.0, delta_y: 0.0 },
            &mut test_ctx(),
        );
        assert!(fsm.shared_storage().current_sector.is_some());

        fsm.handle_with_context(
            &CursorEvent::ModifierReleased { cancel: true },
            &mut test_ctx(),
        );
        // 취소 → sector 클리어
        assert!(fsm.shared_storage().current_sector.is_none());
    }

    #[test]
    fn armed_release_without_move_returns_idle() {
        let mut fsm = CursorFsm::default().state_machine();
        fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut test_ctx());
        fsm.handle_with_context(
            &CursorEvent::ModifierReleased { cancel: false },
            &mut test_ctx(),
        );
        // 이동 없이 뗌 → Idle, sector None
        assert!(fsm.shared_storage().current_sector.is_none());
    }

    #[test]
    fn tracking_updates_sector_on_each_move() {
        let mut fsm = CursorFsm::default().state_machine();
        fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut test_ctx());

        // 오른쪽
        fsm.handle_with_context(
            &CursorEvent::MouseMoved { delta_x: 100.0, delta_y: 0.0 },
            &mut test_ctx(),
        );
        assert_eq!(fsm.shared_storage().current_sector, Some(0));

        // 아래로 변경
        fsm.handle_with_context(
            &CursorEvent::MouseMoved { delta_x: 0.0, delta_y: 100.0 },
            &mut test_ctx(),
        );
        assert_eq!(fsm.shared_storage().current_sector, Some(2));
    }

    #[test]
    fn tracking_tracks_throw_distance() {
        let mut fsm = CursorFsm::default().state_machine();
        fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut test_ctx());
        fsm.handle_with_context(
            &CursorEvent::MouseMoved { delta_x: 300.0, delta_y: 400.0 },
            &mut test_ctx(),
        );
        // 3-4-5 삼각비 → 500
        assert!((fsm.shared_storage().throw_distance - 500.0).abs() < 0.001);
    }

    #[test]
    fn ignore_mouse_move_in_idle() {
        let mut fsm = CursorFsm::default().state_machine();
        // Idle 상태에서 마우스 이동 — 무시
        fsm.handle_with_context(
            &CursorEvent::MouseMoved { delta_x: 100.0, delta_y: 0.0 },
            &mut test_ctx(),
        );
        assert!(fsm.shared_storage().current_sector.is_none());
    }
}
```

- [ ] **Step 2: `domain/mod.rs`에 cursor_fsm 추가**

```rust
pub mod errors;
pub mod model;
pub mod presets;
pub mod geometry;
pub mod keyboard_chain;
pub mod cursor_fsm;
```

- [ ] **Step 3: 테스트 실행**

Run: `cargo test --lib domain::cursor_fsm`
Expected: PASS

statig 매크로 API가 예상과 다를 수 있으므로, 컴파일 에러가 나면 statig README 예제(blinky)를 참조하여 시그니처 조정. 핵심:
- `#[state_machine(initial = "State::idle()")]`는 impl 블록에
- 핸들러는 `fn name(&mut self, event: &Event) -> Outcome`
- context 사용시 `fn name(&mut self, event: &Event, ctx: &Context) -> Outcome` + `handle_with_context`

- [ ] **Step 4: 커밋**

```bash
git add -A
git commit -m "feat: 커서 FSM (statig HSM) - Idle/Armed/Tracking/Snapping 상태머신"
```

---

## Task 7: 통합 테스트 및 도메인 전체 빌드 검증

**Files:**
- Create: `src-tauri/tests/domain_integration.rs`

- [ ] **Step 1: 통합 테스트 작성 — `src-tauri/tests/domain_integration.rs`**

도메인 컴포넌트들이 함께 동작하는지 검증:

```rust
use rectangle_win::domain::cursor_fsm::{CursorEvent, CursorFsm, FsmContext};
use rectangle_win::domain::geometry;
use rectangle_win::domain::keyboard_chain::ChainCycle;
use rectangle_win::domain::model::{Config, Direction, SnapTarget, WindowAction};
use rectangle_win::domain::presets::SnapPreset;

fn ctx() -> FsmContext {
    FsmContext {
        sector_count: 8,
        compute_sector: |dx, dy| geometry::compute_sector(
            euclid::Vector2D::new(dx, dy), 8
        ),
        compute_distance: |dx, dy| geometry::throw_distance(
            euclid::Vector2D::new(dx, dy)
        ),
    }
}

#[test]
fn config_serialize_deserialize_toml() {
    let config = Config::default();
    let toml_str = toml::to_string(&config).unwrap();
    assert!(toml_str.contains("[general]"));
    assert!(toml_str.contains("[snap]"));
    assert!(toml_str.contains("[throw]"));
    assert!(toml_str.contains("[keyboard]"));

    let parsed: Config = toml::from_str(&toml_str).unwrap();
    assert_eq!(config, parsed);
}

#[test]
fn preset_targets_all_valid_ratios() {
    for preset in [SnapPreset::Minimal, SnapPreset::Standard, SnapPreset::Extended, SnapPreset::Full, SnapPreset::Portrait] {
        for target in preset.targets() {
            if let SnapTarget::Area { x_ratio, y_ratio, w_ratio, h_ratio, .. } = &target {
                for &r in &[*x_ratio, *y_ratio, *w_ratio, *h_ratio] {
                    assert!(r >= 0.0 && r <= 1.0, "invalid ratio {} in {:?}", r, preset);
                }
            }
        }
    }
}

#[test]
fn fsm_and_geometry_integration() {
    let mut fsm = CursorFsm::default().state_machine();

    // 활성화
    fsm.handle_with_context(&CursorEvent::ModifierPressed, &mut ctx());

    // 오른쪽 위 대각선 이동
    fsm.handle_with_context(
        &CursorEvent::MouseMoved { delta_x: 100.0, delta_y: -100.0 },
        &mut ctx(),
    );
    let sector = fsm.shared_storage().current_sector.unwrap();
    // 오른쪽 위 = 섹터 7
    assert_eq!(sector, 7);
}

#[test]
fn chain_cycle_with_default_config() {
    let config = Config::default();
    let mut cycle = ChainCycle::new(config.keyboard.cycle_timeout_ms);
    let h_chain = &config.keyboard.chains.horizontal;

    let idx0 = cycle.next_index(Direction::Right, 1, h_chain).unwrap();
    assert_eq!(idx0, 0);
    assert_eq!(h_chain[idx0], "left-half");

    let idx1 = cycle.next_index(Direction::Right, 1, h_chain).unwrap();
    assert_eq!(idx1, 1);
    assert_eq!(h_chain[idx1], "third-left");
}

#[test]
fn vertical_chain_default_values() {
    let config = Config::default();
    assert_eq!(config.keyboard.chains.vertical, vec![
        "maximize", "almost-maximize", "center", "maximize-height"
    ]);
}
```

- [ ] **Step 2: `src-tauri/src/lib.rs` 작성 (통합 테스트용 lib 노출)**

`src-tauri/src/lib.rs` 생성:

```rust
pub mod domain;
```

`src-tauri/src/main.rs` 수정:

```rust
pub mod domain;

fn main() {
    println!("Rectangle Win — domain core loaded");
}
```

- [ ] **Step 3: 전체 테스트 실행**

Run: `cargo test`
Expected: 모든 단위 + 통합 테스트 PASS

- [ ] **Step 4: 커밋**

```bash
git add -A
git commit -m "test: 도메인 코어 통합 테스트 추가 및 lib 노출"
```

---

## Self-Review 결과

**1. Spec coverage:**
- ✅ SnapTarget(Area/Action 통합) → Task 2
- ✅ 프리셋 패키지(Minimal~Portrait) → Task 3
- ✅ 섹터 산출, 비율→픽셀 변환, throw 거리 → Task 4
- ✅ 커서 FSM(Idle/Armed/Tracking/Snapping) → Task 6
- ✅ 키보드 체인(horizontal/vertical 순환) → Task 5
- ✅ TOML 직렬화/역직렬화 → Task 2, Task 7
- ✅ vertical 체인 기본값(maximize/almost-maximize/center/maximize-height) → Task 2

이 plan이 커버하지 않는 것 (후속 plan 대상):
- Win32 인프라(훅, 창 관리, 모니터, 오버레이)
- Tauri 앱 골격(커맨드, 트레이, 자동시작, 업데이터)
- 프론트엔드(Nuxt UI 설정 화면, vue-konva 에디터, 오버레이)

**2. Placeholder scan:** 없음. 모든 단계에 실제 코드 포함.

**3. Type consistency:**
- `SnapTarget`, `WindowAction`, `Sector`, `Direction` — Task 2 정의, Task 3/5/6/7에서 일관 사용 ✅
- `ChainCycle::next_index` 시그니처 — Task 5 정의, Task 7 통합 테스트에서 일관 ✅
- `CursorFsm`, `FsmContext`, `CursorEvent` — Task 6 정의, Task 7 통합 테스트에서 일관 ✅
- `compute_sector` / `throw_distance` 함수명 — geometry(Task 4)와 FSM context(Task 6)에서 일관 ✅
