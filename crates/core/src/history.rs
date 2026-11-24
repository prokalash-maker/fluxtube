use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: Option<i64>,
    pub video_id: String,
    pub title: String,
    pub channel_title: String,
    pub thumbnail_url: String,
    pub duration_seconds: u64,
    pub watched_at: DateTime<Utc>,
    pub playback_position_seconds: u64,
    pub completed: bool,
    pub account_id: Option<String>,
}

impl HistoryEntry {
    pub fn progress_ratio(&self) -> f32 {
        if self.duration_seconds == 0 {
            0.0
        } else {
            (self.playback_position_seconds as f32 / self.duration_seconds as f32).clamp(0.0, 1.0)
        }
    }

    pub fn is_resumable(&self) -> bool {
        !self.completed && self.playback_position_seconds > 5 && self.progress_ratio() < 0.95
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: i64,
    pub video_id: String,
    pub title: String,
    pub timestamp_seconds: u64,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Bookmark {
    pub fn formatted_timestamp(&self) -> String {
        let hours = self.timestamp_seconds / 3600;
        let mins = (self.timestamp_seconds % 3600) / 60;
        let secs = self.timestamp_seconds % 60;
        if hours > 0 {
            format!("{}:{:02}:{:02}", hours, mins, secs)
        } else {
            format!("{}:{:02}", mins, secs)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoPosition {
    pub video_id: String,
    pub position_seconds: u64,
    pub duration_seconds: u64,
    pub updated_at: DateTime<Utc>,
}

impl VideoPosition {
    pub fn progress_percent(&self) -> f32 {
        if self.duration_seconds == 0 {
            0.0
        } else {
            ((self.position_seconds as f32 / self.duration_seconds as f32) * 100.0).clamp(0.0, 100.0)
        }
    }

    pub fn is_finished(&self) -> bool {
        self.progress_percent() >= 90.0
            || (self.duration_seconds > 0
                && self.duration_seconds.saturating_sub(self.position_seconds) <= 5)
    }
}
