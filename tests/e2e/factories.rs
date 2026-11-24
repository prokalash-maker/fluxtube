//! Domain Synthesizers and Builders for FluxTube Testing.

use crate::models::*;
use chrono::Utc;
use std::time::Duration;

/// Synthesizes a canonical sample `Video` model.
pub fn sample_video(id: &str) -> Video {
    Video {
        id: id.to_string(),
        title: format!("Test Video: {id}"),
        channel_id: "UCuAXFkgsw1L7xaCfnd5JJOw".to_string(),
        channel_title: "FluxTube Creator".to_string(),
        thumbnail_url: format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg"),
        duration: Duration::from_secs(213),
        view_count: 1_000_000,
        published_at: Utc::now(),
        embeddable: true,
    }
}

/// Synthesizes a canonical sample `Channel` model.
pub fn sample_channel(id: &str) -> Channel {
    let uploads_id = if let Some(rest) = id.strip_prefix("UC") {
        format!("UU{rest}")
    } else {
        id.to_string()
    };

    Channel {
        id: id.to_string(),
        title: format!("Channel {id}"),
        description: "Official channel for testing.".to_string(),
        avatar_url: format!("https://yt3.ggpht.com/{id}_avatar.jpg"),
        banner_url: Some(format!("https://yt3.ggpht.com/{id}_banner.jpg")),
        subscriber_count: 500_000,
        uploads_playlist_id: uploads_id,
    }
}

/// Synthesizes a canonical sample `Playlist` model.
pub fn sample_playlist(id: &str) -> Playlist {
    Playlist {
        id: id.to_string(),
        title: format!("Playlist {id}"),
        description: "Test playlist collection.".to_string(),
        item_count: 10,
        thumbnail_url: Some("https://i.ytimg.com/vi/sample/hqdefault.jpg".to_string()),
    }
}

/// Synthesizes a canonical sample `Comment` model.
pub fn sample_comment(id: &str) -> Comment {
    Comment {
        id: id.to_string(),
        author_name: "RustFan".to_string(),
        author_avatar_url: "https://yt3.ggpht.com/avatar.jpg".to_string(),
        text: "Great video!".to_string(),
        like_count: 42,
        published_at: Utc::now(),
        reply_count: 0,
    }
}

/// Synthesizes default `SearchParams`.
pub fn sample_search_params(query: &str) -> SearchParams {
    SearchParams {
        query: query.to_string(),
        page_token: None,
        max_results: Some(25),
        order: Some("relevance".to_string()),
        video_duration: None,
        resource_type: None,
    }
}

/// Fluent builder for constructing customized `Video` instances.
#[derive(Debug, Clone)]
pub struct VideoBuilder {
    video: Video,
}

impl Default for VideoBuilder {
    fn default() -> Self {
        Self::new("sample_video_id")
    }
}

impl VideoBuilder {
    pub fn new(id: &str) -> Self {
        Self {
            video: sample_video(id),
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.video.title = title.into();
        self
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.video.duration = duration;
        self
    }

    pub fn view_count(mut self, count: u64) -> Self {
        self.video.view_count = count;
        self
    }

    pub fn embeddable(mut self, embeddable: bool) -> Self {
        self.video.embeddable = embeddable;
        self
    }

    pub fn build(self) -> Video {
        self.video
    }
}
