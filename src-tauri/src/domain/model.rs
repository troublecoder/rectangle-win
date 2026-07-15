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
