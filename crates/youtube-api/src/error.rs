use fluxtube_network::{NetworkError, QuotaError};
use thiserror::Error;

/// Errors returned by the YouTube Data API v3 client.
#[derive(Debug, Error)]
pub enum YouTubeApiError {
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("YouTube API quota exceeded: {0}")]
    QuotaExceeded(String),

    #[error("Quota tracker error: {0}")]
    QuotaTracker(#[from] QuotaError),

    #[error("Unauthorized or expired credentials: {0}")]
    Unauthorized(String),

    #[error("Forbidden ({reason}): {message}")]
    Forbidden { reason: String, message: String },

    #[error("Comments are disabled for this video: {0}")]
    CommentsDisabled(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("YouTube server error ({status}): {message}")]
    ServerError { status: u16, message: String },

    #[error("Response parse error: {0}")]
    ParseError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}
