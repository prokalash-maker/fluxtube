use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThumbnailQuality {
    Default,
    Medium,
    High,
    Standard,
    Maxres,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thumbnail {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub quality: ThumbnailQuality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoRating {
    Like,
    Dislike,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackState {
    Idle,
    Buffering,
    Playing,
    Paused,
    Ended,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub description: String,
    pub channel_id: String,
    pub channel_title: String,
    pub thumbnail_url: String,
    pub thumbnails: Vec<Thumbnail>,
    pub duration: Duration,
    pub view_count: u64,
    pub like_count: Option<u64>,
    pub published_at: DateTime<Utc>,
    pub embeddable: bool,
    pub live_broadcast_content: Option<String>,
}

impl Video {
    pub fn formatted_duration(&self) -> String {
        let total_secs = self.duration.as_secs();
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        let secs = total_secs % 60;
        if hours > 0 {
            format!("{}:{:02}:{:02}", hours, mins, secs)
        } else {
            format!("{}:{:02}", mins, secs)
        }
    }

    pub fn formatted_view_count(&self) -> String {
        if self.view_count >= 1_000_000_000 {
            format!("{:.1}B", self.view_count as f64 / 1_000_000_000.0)
        } else if self.view_count >= 1_000_000 {
            format!("{:.1}M", self.view_count as f64 / 1_000_000.0)
        } else if self.view_count >= 1_000 {
            format!("{:.1}K", self.view_count as f64 / 1_000.0)
        } else {
            self.view_count.to_string()
        }
    }

    pub fn is_live(&self) -> bool {
        self.live_broadcast_content.as_deref() == Some("live")
    }

    pub fn is_short(&self) -> bool {
        self.duration.as_secs() > 0 && self.duration.as_secs() <= 60
    }

    pub fn watch_url(&self) -> String {
        format!("https://www.youtube.com/watch?v={}", self.id)
    }

    pub fn embed_url(&self) -> String {
        format!(
            "https://www.youtube-nocookie.com/embed/{}?enablejsapi=1&origin=https://localhost",
            self.id
        )
    }
}
