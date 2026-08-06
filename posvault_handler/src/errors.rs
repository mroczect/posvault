use thiserror::Error;

#[derive(Error, Debug)]
pub enum PosVaultError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Sync error: {0}")]
    Sync(String),

    #[error("Journal error: {0}")]
    Journal(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("External error: {0}")]
    External(Box<dyn std::error::Error + Send + Sync>),
}

pub type Result<T> = std::result::Result<T, PosVaultError>;
