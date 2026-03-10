use thiserror::Error;

#[derive(Error, Debug)]
pub enum AttestError {
    #[error("Cryptographic operation failed: {0}")]
    Crypto(String),

    #[error("Storage error: {0}")]
    Storage(#[from] sqlx::Error),

    #[error("Cloud API error: {0}")]
    Cloud(#[from] reqwest::Error),

    #[error("Ecosystem mismatch: {0}")]
    Alignment(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}
