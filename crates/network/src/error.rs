use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Circuit breaker is open until {cool_off_until}")]
    CircuitBreakerOpen {
        cool_off_until: chrono::DateTime<chrono::Utc>,
    },

    #[error("Request timed out after {0:?}")]
    Timeout(Duration),

    #[error("Max retries ({max_retries}) exceeded: {last_error}")]
    MaxRetriesExceeded {
        max_retries: usize,
        last_error: String,
    },

    #[error("Quota exceeded: {0}")]
    QuotaExceeded(#[from] QuotaError),

    #[error("Deduplication broadcast dropped")]
    DeduplicationFailed,

    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Status error ({status}): {body}")]
    Status {
        status: reqwest::StatusCode,
        body: String,
    },
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum QuotaError {
    #[error("General quota exhausted ({used}/{limit} units)")]
    GeneralQuotaExceeded { used: u32, limit: u32 },

    #[error("Search quota exhausted ({used}/{limit} calls)")]
    SearchQuotaExceeded { used: u32, limit: u32 },
}
