use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub video_id: String,
    pub author_name: String,
    pub author_avatar_url: String,
    pub author_channel_id: Option<String>,
    pub text: String,
    pub like_count: u32,
    pub published_at: DateTime<Utc>,
    pub reply_count: u32,
    pub is_channel_owner: bool,
}
