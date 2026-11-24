use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ViewMode {
    #[default]
    Normal,
    Theater,
    Fullscreen,
    PictureInPicture,
    MiniPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum VideoQuality {
    #[default]
    Auto,
    HighDefinition1080p,
    HighDefinition720p,
    Large480p,
    Medium360p,
    Small240p,
    Tiny144p,
    Custom(String),
}

impl VideoQuality {
    pub fn as_youtube_str(&self) -> &str {
        match self {
            Self::Auto => "auto",
            Self::HighDefinition1080p => "hd1080",
            Self::HighDefinition720p => "hd720",
            Self::Large480p => "large",
            Self::Medium360p => "medium",
            Self::Small240p => "small",
            Self::Tiny144p => "tiny",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn from_youtube_str(s: &str) -> Self {
        match s {
            "auto" => Self::Auto,
            "hd1080" => Self::HighDefinition1080p,
            "hd720" => Self::HighDefinition720p,
            "large" => Self::Large480p,
            "medium" => Self::Medium360p,
            "small" => Self::Small240p,
            "tiny" => Self::Tiny144p,
            other => Self::Custom(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitleTrack {
    pub id: String,
    pub language_code: String,
    pub label: String,
    pub is_auto_generated: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PlayerBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl PlayerBounds {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}
