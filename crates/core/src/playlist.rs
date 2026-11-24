use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub title: String,
    pub description: String,
    pub item_count: u32,
    pub thumbnail_url: Option<String>,
    pub channel_id: Option<String>,
    pub channel_title: Option<String>,
    pub is_local: bool,
    pub account_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub id: String,
    pub playlist_id: String,
    pub video_id: String,
    pub title: String,
    pub thumbnail_url: String,
    pub position: u32,
    pub added_at: DateTime<Utc>,
}
