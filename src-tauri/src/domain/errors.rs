use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("snap target not found: {0}")]
    TargetNotFound(String),

    #[error("invalid sector index {index}: must be 0..{max}")]
    InvalidSector { index: u8, max: u8 },

    #[error("invalid ratio {value}: must be between 0.0 and 1.0")]
    InvalidRatio { value: f64 },

    #[error("preset not recognized: {0}")]
    UnknownPreset(String),
}

pub type DomainResult<T> = Result<T, DomainError>;
