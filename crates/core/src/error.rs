use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Invalid ID format: {0}")]
    InvalidId(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Internal domain error: {0}")]
    Internal(String),
}
