use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MediaSource {
    /// Official YouTube video identifier (e.g. "dQw4w9WgXcQ")
    YouTubeVideoId(String),
    /// Direct HTTP or HTTPS stream URL (e.g. HLS or MP4)
    DirectStreamUrl(String),
    /// Local media file on filesystem
    LocalFile(PathBuf),
}

impl MediaSource {
    pub fn youtube(id: impl Into<String>) -> Self {
        Self::YouTubeVideoId(id.into())
    }

    pub fn direct_url(url: impl Into<String>) -> Self {
        Self::DirectStreamUrl(url.into())
    }

    pub fn local(path: impl Into<PathBuf>) -> Self {
        Self::LocalFile(path.into())
    }

    pub fn is_youtube(&self) -> bool {
        matches!(self, Self::YouTubeVideoId(_))
    }

    pub fn is_direct_stream(&self) -> bool {
        matches!(self, Self::DirectStreamUrl(_))
    }

    pub fn is_local_file(&self) -> bool {
        matches!(self, Self::LocalFile(_))
    }

    pub fn identifier(&self) -> String {
        match self {
            Self::YouTubeVideoId(id) => id.clone(),
            Self::DirectStreamUrl(url) => url.clone(),
            Self::LocalFile(path) => path.to_string_lossy().to_string(),
        }
    }
}
