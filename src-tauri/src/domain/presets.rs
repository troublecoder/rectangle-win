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
