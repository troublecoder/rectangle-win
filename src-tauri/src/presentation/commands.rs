//! Tauri IPC 명령 — 프론트엔드(Vue)에서 `invoke()` 로 호출하는 진입점들.
//!
//! 모든 명령은 `State<'_, AppState>` 에서 서비스를 가져와 도메인/애플리케이션
//! 계층에 위임한다. 에러는 직렬화 가능한 [`CommandError`] 로 변환되어
//! 프론트엔드에 `{ message, code }` 형태로 전달된다.

use tauri::State;

use crate::application::errors::ApplicationError;
use crate::application::ports::{ConfigStore, MonitorProvider};
use crate::domain::errors::DomainError;
use crate::domain::model::Config;
use crate::domain::presets::SnapPreset;
use crate::presentation::state::AppState;

/// 프론트엔드로 직렬화되는 에러 응답.
#[derive(Debug, serde::Serialize)]
pub struct CommandError {
    pub message: String,
    pub code: String,
}

impl From<ApplicationError> for CommandError {
    fn from(e: ApplicationError) -> Self {
        let code = match &e {
            ApplicationError::Domain(_) => "DOMAIN",
            ApplicationError::ConfigNotLoaded => "CONFIG_NOT_LOADED",
            ApplicationError::WindowOperation(_) => "WINDOW_OP",
            ApplicationError::OverlayOperation(_) => "OVERLAY_OP",
            ApplicationError::NoForegroundWindow => "NO_FOREGROUND",
        };
        CommandError {
            message: e.to_string(),
            code: code.to_string(),
        }
    }
}

/// 도메인 에러도 `CommandError` 로 통합 변환.
impl From<DomainError> for CommandError {
    fn from(e: DomainError) -> Self {
        ApplicationError::from(e).into()
    }
}

type CmdResult<T> = Result<T, CommandError>;

/// 현재 설정을 로드한다.
#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> CmdResult<Config> {
    Ok(state.config_store.load()?)
}

/// 설정을 저장한다. 캐시와 디스크 모두 갱신된다.
#[tauri::command]
pub fn save_config(state: State<'_, AppState>, config: Config) -> CmdResult<()> {
    state.config_store.save(&config)?;
    // LL 훅 콜백이 읽는 static config 캐시를 갱신 (디스크 I/O 를 콜백에서 피하기 위함).
    #[cfg(windows)]
    crate::infrastructure::win32_input::Win32InputListener::update_config(&config);
    Ok(())
}

/// 설정 파일의 절대 경로를 반환한다 (디버그/표시용).
#[tauri::command]
pub fn get_config_path(state: State<'_, AppState>) -> String {
    state.config_store.path().to_string_lossy().to_string()
}

/// 프리셋을 적용한다 — active_preset 및 areas 를 갱신해 저장.
#[tauri::command]
pub fn apply_preset(state: State<'_, AppState>, preset_name: String) -> CmdResult<Config> {
    let preset = SnapPreset::from_str(&preset_name)?;
    let mut config = state.config_store.load()?;
    config.snap.active_preset = preset_name;
    config.snap.areas = preset.targets();
    state.config_store.save(&config)?;
    Ok(config)
}

/// 모니터 정보 DTO (프론트엔드 직렬화용).
#[derive(Debug, serde::Serialize)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// 사용 가능한 모니터 목록을 반환한다.
#[tauri::command]
pub fn get_monitors(state: State<'_, AppState>) -> Vec<MonitorInfo> {
    state
        .monitor_provider
        .enumerate()
        .iter()
        .map(|m| MonitorInfo {
            x: m.origin.x,
            y: m.origin.y,
            width: m.width(),
            height: m.height(),
        })
        .collect()
}

/// 주어진 섹터에 매핑된 스냅 타겟을 현재 전경창에 즉시 적용한다 (프리뷰/테스트용).
#[tauri::command]
pub fn test_snap_to_sector(
    state: State<'_, AppState>,
    sector: u8,
    cursor_x: i32,
    cursor_y: i32,
) -> CmdResult<()> {
    let config = state.config_store.load()?;
    let target_id = config.throw.mapping.get(&sector).ok_or_else(|| {
        ApplicationError::Domain(DomainError::InvalidSector {
            index: sector,
            max: 8,
        })
    })?;
    let target = config
        .snap
        .areas
        .iter()
        .find(|t| t.id() == target_id.as_str())
        .ok_or_else(|| {
            ApplicationError::Domain(DomainError::TargetNotFound(target_id.clone()))
        })?;
    let window = state
        .window_mover
        .get_foreground_window()
        .ok_or(ApplicationError::NoForegroundWindow)?;
    let monitor = state.monitor_provider.monitor_at(cursor_x, cursor_y);
    state
        .window_mover
        .apply_snap_target(window, target, &monitor)?;
    Ok(())
}
