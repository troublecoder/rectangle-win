//! TOML 기반 [`ConfigStore`] 구현체.
//!
//! - `dirs` 크레이트로 OS 표준 설정 디렉터리 해석
//!   (Windows: `%APPDATA%`, macOS: `~/Library/Application Support`, Linux: `~/.config`)
//! - 파일이 없으면 기본 [`Config`] 를 생성해 디스크에 기록
//! - [`Mutex`] 안에서 [`Config`] 를 캐싱하여 디스크 I/O 최소화
//!
//! [`ConfigStore`]: crate::application::ports::ConfigStore

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::application::errors::{ApplicationError, AppResult};
use crate::application::ports::ConfigStore;
use crate::domain::model::Config;

/// 디스크의 TOML 파일을 백킹 스토어로 사용하는 [`ConfigStore`] 구현체.
///
/// 생성 시점에 파일을 읽지 않고, 최초 [`load`](ConfigStore::load) 호출 시점에
/// lazy 하게 읽어 [`Mutex`] 내부 캐시에 저장한다.
pub struct TomlConfigStore {
    path: PathBuf,
    cache: Mutex<Option<Config>>,
}

impl TomlConfigStore {
    /// OS 표준 설정 디렉터리의 `rectangle-win/config.toml` 경로를 사용.
    /// 디렉터리가 없으면 생성을 시도한다 (실패해도 경로는 유지).
    pub fn default_path() -> Self {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        let dir = config_dir.join("rectangle-win");
        let _ = std::fs::create_dir_all(&dir);
        Self {
            path: dir.join("config.toml"),
            cache: Mutex::new(None),
        }
    }

    /// 테스트 또는 커스텀 위치 목적으로 임의 경로를 지정.
    pub fn at_path(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            cache: Mutex::new(None),
        }
    }

    fn read_from_disk(&self) -> AppResult<Config> {
        if !self.path.exists() {
            let default = Self::config_with_defaults();
            self.write_to_disk(&default)?;
            return Ok(default);
        }
        let contents = std::fs::read_to_string(&self.path).map_err(|e| {
            ApplicationError::WindowOperation(format!("failed to read config: {}", e))
        })?;
        let mut config: Config = toml::from_str(&contents).map_err(|e| {
            ApplicationError::WindowOperation(format!("failed to parse config: {}", e))
        })?;

        // snap 영역이 비어 있으면(이전 버전 config 등) 활성 프리셋으로 채운다.
        if config.snap.areas.is_empty() {
            config = Self::config_with_defaults();
            self.write_to_disk(&config)?;
        }
        Ok(config)
    }

    /// 기본 Config 에 standard 프리셋 + 8섹터 throw 매핑을 적용해 반환.
    fn config_with_defaults() -> Config {
        use crate::domain::model::SectorMap;
        use crate::domain::presets::SnapPreset;

        let mut config = Config::default();
        let preset = SnapPreset::from_str(&config.snap.active_preset)
            .unwrap_or(SnapPreset::Standard);
        config.snap.areas = preset.targets();

        // 8섹터 throw 매핑 — 시계방향, 0번=오른쪽(E).
        // 섹터 번호는 domain::model::Sector 주석 기준:
        //   0=오른쪽, 1=오른쪽아래, 2=아래, 3=왼쪽아래,
        //   4=왼쪽, 5=왼쪽위, 6=위, 7=오른쪽위
        let mut mapping = SectorMap::new();
        mapping.insert(0, "right-half".to_string());      // → 우 — 우측절반
        mapping.insert(1, "quarter-br".to_string());      // ↘ 우하 — 우하분기
        mapping.insert(2, "bottom-half".to_string());     // ↓ 아래 — 하단절반
        mapping.insert(3, "quarter-bl".to_string());      // ↙ 좌하 — 좌하분기
        mapping.insert(4, "left-half".to_string());       // ← 좌 — 좌측절반
        mapping.insert(5, "quarter-tl".to_string());      // ↖ 좌상 — 좌상분기
        mapping.insert(6, "maximize".to_string());        // ↑ 위 — 최대화
        mapping.insert(7, "quarter-tr".to_string());      // ↗ 우상 — 우상분기
        config.throw.mapping = mapping;

        config
    }

    fn write_to_disk(&self, config: &Config) -> AppResult<()> {
        let contents = toml::to_string_pretty(config).map_err(|e| {
            ApplicationError::WindowOperation(format!("failed to serialize config: {}", e))
        })?;
        std::fs::write(&self.path, contents).map_err(|e| {
            ApplicationError::WindowOperation(format!("failed to write config: {}", e))
        })
    }
}

impl ConfigStore for TomlConfigStore {
    fn load(&self) -> AppResult<Config> {
        let mut cache = self.cache.lock().unwrap();
        if let Some(ref cached) = *cache {
            return Ok(cached.clone());
        }
        let config = self.read_from_disk()?;
        *cache = Some(config.clone());
        Ok(config)
    }

    fn save(&self, config: &Config) -> AppResult<()> {
        self.write_to_disk(config)?;
        *self.cache.lock().unwrap() = Some(config.clone());
        Ok(())
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_creates_default_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.toml");
        let store = TomlConfigStore::at_path(&path);
        let config = store.load().unwrap();
        assert_eq!(config.general.language, "ko");
        assert!(path.exists());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.toml");
        let store = TomlConfigStore::at_path(&path);
        let mut config = Config::default();
        config.general.language = "en".to_string();
        store.save(&config).unwrap();

        // 새 store 인스턴스(캐시 없음)는 디스크에서 읽는다
        let store2 = TomlConfigStore::at_path(&path);
        let loaded = store2.load().unwrap();
        assert_eq!(loaded.general.language, "en");
    }

    #[test]
    fn path_returns_configured_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("custom.toml");
        let store = TomlConfigStore::at_path(&path);
        assert_eq!(store.path(), path.as_path());
    }

    #[test]
    fn cache_prevents_disk_reads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache_test.toml");
        let store = TomlConfigStore::at_path(&path);

        // 첫 로드가 파일을 생성하고 캐싱
        let config1 = store.load().unwrap();

        // 파일 삭제 — 두 번째 로드는 캐시를 사용해야 하며 실패하지 않는다
        std::fs::remove_file(&path).unwrap();
        let config2 = store.load().unwrap();

        assert_eq!(config1, config2);
    }

    #[test]
    fn save_updates_cache_for_same_instance() {
        // 동일 인스턴스에서 save 이후 load 시 디스크를 다시 읽지 않고도
        // 갱신된 값이 보이는지 검증 (캐시 일관성).
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache_update.toml");
        let store = TomlConfigStore::at_path(&path);

        let mut config = Config::default();
        config.general.language = "ja".to_string();
        store.save(&config).unwrap();

        // 파일을 오염시켜도 캐시가 우선되어야 한다
        std::fs::write(&path, "garbage = true").unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.general.language, "ja");
    }
}
