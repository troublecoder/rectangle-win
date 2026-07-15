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
