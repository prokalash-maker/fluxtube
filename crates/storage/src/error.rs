use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("SQLite database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Migration failed at version {version}: {reason}")]
    MigrationFailed { version: i64, reason: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Database lock error: {0}")]
    Lock(String),
}
