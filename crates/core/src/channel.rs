use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub title: String,
    pub description: String,
    pub custom_url: Option<String>,
    pub avatar_url: String,
    pub banner_url: Option<String>,
    pub subscriber_count: u64,
    pub video_count: u64,
    pub view_count: Option<u64>,
    pub uploads_playlist_id: String,
}

impl Channel {
    pub fn formatted_subscriber_count(&self) -> String {
        if self.subscriber_count >= 1_000_000 {
            format!("{:.2}M", self.subscriber_count as f64 / 1_000_000.0)
        } else if self.subscriber_count >= 1_000 {
            format!("{:.1}K", self.subscriber_count as f64 / 1_000.0)
        } else {
            self.subscriber_count.to_string()
        }
    }

    pub fn channel_url(&self) -> String {
        format!("https://www.youtube.com/channel/{}", self.id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelGroup {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelGroupItem {
    pub group_id: String,
    pub channel_id: String,
    pub channel_title: String,
    pub avatar_url: Option<String>,
    pub added_at: DateTime<Utc>,
}
