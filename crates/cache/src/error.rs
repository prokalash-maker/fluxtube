use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CacheError {
    #[error("Network error fetching image from {0}: {1}")]
    Network(String, String),

    #[error("I/O error in disk cache: {0}")]
    Io(String),

    #[error("Image decode failed: {0}")]
    Decode(String),

    #[error("Image downscale failed: {0}")]
    Downscale(String),

    #[error("SingleFlight deduplication error: {0}")]
    SingleFlight(String),

    #[error("Background worker thread panic: {0}")]
    WorkerPanic(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

impl From<std::io::Error> for CacheError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}
