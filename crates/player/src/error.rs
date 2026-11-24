use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum PlayerError {
    #[error("Player backend not initialized")]
    NotInitialized,

    #[error("Media source unsupported by backend '{backend}': {reason}")]
    UnsupportedSource { backend: String, reason: String },

    #[error("Feature '{feature}' not supported by backend '{backend}'")]
    UnsupportedFeature { backend: String, feature: String },

    #[error("Media load failed: {0}")]
    LoadFailed(String),

    #[error("Playback operation failed: {0}")]
    PlaybackFailed(String),

    #[error("Seek operation failed: {0}")]
    SeekFailed(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("IPC communication error: {0}")]
    IpcError(String),

    #[error("Native backend failure: {0}")]
    NativeBackendError(String),

    #[error("Window or view bounds error: {0}")]
    BoundsError(String),

    #[error("Internal player error: {0}")]
    Internal(String),
}
