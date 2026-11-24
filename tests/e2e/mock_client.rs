//! Mock In-Memory YouTube Client for Fast Offline Testing.

use crate::fixtures::TestFixtureRegistry;
use crate::models::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Trait defining YouTube Data API v3 operations for FluxTube (PROJECT.md lines 145-153).
#[async_trait]
pub trait YouTubeApiClient: Send + Sync {
    async fn search(&self, params: &SearchParams) -> Result<SearchResponse, ApiError>;
    async fn get_video_details(&self, video_ids: &[&str]) -> Result<Vec<Video>, ApiError>;
    async fn get_channel_details(&self, channel_id: &str) -> Result<Channel, ApiError>;
    async fn get_playlist_items(
        &self,
        playlist_id: &str,
        page_token: Option<&str>,
    ) -> Result<PlaylistItemsResponse, ApiError>;
    async fn get_user_subscriptions(
        &self,
        page_token: Option<&str>,
    ) -> Result<SubscriptionsResponse, ApiError>;
    async fn get_comment_threads(
        &self,
        video_id: &str,
        page_token: Option<&str>,
    ) -> Result<CommentThreadsResponse, ApiError>;
    async fn rate_video(&self, video_id: &str, rating: VideoRating) -> Result<(), ApiError>;
}

/// Offline in-memory YouTube API client backed by canonical fixtures from `TestFixtureRegistry`.
#[derive(Clone)]
pub struct MockYouTubeClient {
    registry: Arc<TestFixtureRegistry>,
    quota_exceeded: Arc<RwLock<bool>>,
    offline: Arc<RwLock<bool>>,
    ratings: Arc<RwLock<HashMap<String, VideoRating>>>,
}

impl Default for MockYouTubeClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockYouTubeClient {
    /// Creates a new mock client backed by preloaded canonical fixtures.
    pub fn new() -> Self {
        Self {
            registry: Arc::new(TestFixtureRegistry::new()),
            quota_exceeded: Arc::new(RwLock::new(false)),
            offline: Arc::new(RwLock::new(false)),
            ratings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Toggles the 403 Quota Exceeded error state.
    pub fn set_quota_exceeded(&self, enabled: bool) {
        *self.quota_exceeded.write().unwrap() = enabled;
    }

    /// Toggles simulated offline network error state.
    pub fn set_offline(&self, enabled: bool) {
        *self.offline.write().unwrap() = enabled;
    }

    /// Returns recorded user rating for a video.
    pub fn get_recorded_rating(&self, video_id: &str) -> Option<VideoRating> {
        self.ratings.read().unwrap().get(video_id).copied()
    }

    fn check_preconditions(&self) -> Result<(), ApiError> {
        if *self.offline.read().unwrap() {
            return Err(ApiError::Network("No internet connection (offline mode)".into()));
        }
        if *self.quota_exceeded.read().unwrap() {
            return Err(ApiError::QuotaExceeded(
                "The request cannot be completed because you have exceeded your quota.".into(),
            ));
        }
        Ok(())
    }
}

/// Helper function to parse ISO 8601 duration string (e.g. "PT4M13S", "PT1H12M45S", "PT0S").
pub fn parse_iso8601_duration(s: &str) -> Duration {
    let re = regex::Regex::new(r"PT(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?").unwrap();
    if let Some(caps) = re.captures(s) {
        let hours: u64 = caps.get(1).map_or(0, |m| m.as_str().parse().unwrap_or(0));
        let mins: u64 = caps.get(2).map_or(0, |m| m.as_str().parse().unwrap_or(0));
        let secs: u64 = caps.get(3).map_or(0, |m| m.as_str().parse().unwrap_or(0));
        Duration::from_secs(hours * 3600 + mins * 60 + secs)
    } else {
        Duration::ZERO
    }
}

#[async_trait]
impl YouTubeApiClient for MockYouTubeClient {
    async fn search(&self, params: &SearchParams) -> Result<SearchResponse, ApiError> {
        self.check_preconditions()?;

        if params.query.trim().is_empty() {
            let empty: SearchListResponse = self
                .registry
                .get_json("search/search_empty.json")
                .map_err(|e| ApiError::ParseError(e.to_string()))?;
            return Ok(SearchResponse {
                items: vec![],
                next_page_token: empty.next_page_token,
                prev_page_token: empty.prev_page_token,
                total_results: 0,
            });
        }

        let fixture = if params.page_token.as_deref() == Some("CAUQAA") {
            "search/search_videos_page2.json"
        } else if params.video_duration.as_deref() == Some("short") {
            "search/search_filtered_duration.json"
        } else if params.resource_type.as_deref() == Some("channel") {
            "search/search_channels.json"
        } else if params.resource_type.as_deref() == Some("playlist") {
            "search/search_playlists.json"
        } else {
            "search/search_videos_page1.json"
        };

        let raw: SearchListResponse = self
            .registry
            .get_json(fixture)
            .map_err(|e| ApiError::ParseError(e.to_string()))?;

        let items = raw
            .items
            .into_iter()
            .map(|i| {
                let id = i
                    .id
                    .video_id
                    .or(i.id.channel_id)
                    .or(i.id.playlist_id)
                    .unwrap_or_default();
                let snippet = i.snippet.unwrap_or_default();
                let thumb = snippet
                    .thumbnails
                    .and_then(|t| t.medium.or(t.default))
                    .map(|t| t.url)
                    .unwrap_or_default();
                SearchResultItem {
                    kind: i.id.kind.unwrap_or_default(),
                    id,
                    title: snippet.title.unwrap_or_default(),
                    description: snippet.description.unwrap_or_default(),
                    channel_id: snippet.channel_id.unwrap_or_default(),
                    channel_title: snippet.channel_title.unwrap_or_default(),
                    thumbnail_url: thumb,
                    published_at: snippet.published_at.unwrap_or_default(),
                }
            })
            .collect();

        Ok(SearchResponse {
            items,
            next_page_token: raw.next_page_token,
            prev_page_token: raw.prev_page_token,
            total_results: raw.page_info.map(|p| p.total_results).unwrap_or(0),
        })
    }

    async fn get_video_details(&self, video_ids: &[&str]) -> Result<Vec<Video>, ApiError> {
        self.check_preconditions()?;

        if video_ids.is_empty() {
            return Ok(vec![]);
        }

        // Load canonical video batch fixture
        let raw: VideoListResponse = self
            .registry
            .get_json("videos/videos_batch_details.json")
            .map_err(|e| ApiError::ParseError(e.to_string()))?;

        let mut results = vec![];

        for item in raw.items {
            if video_ids.is_empty() || video_ids.contains(&item.id.as_str()) {
                let snippet = item.snippet.unwrap_or_default();
                let content = item.content_details.unwrap_or_default();
                let stats = item.statistics.unwrap_or_default();
                let status = item.status.unwrap_or_default();

                let duration = content
                    .duration
                    .as_deref()
                    .map(parse_iso8601_duration)
                    .unwrap_or(Duration::ZERO);

                let view_count = stats
                    .view_count
                    .as_deref()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);

                let published_at = snippet
                    .published_at
                    .as_deref()
                    .and_then(|p| DateTime::parse_from_rfc3339(p).ok())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

                let thumb = snippet
                    .thumbnails
                    .and_then(|t| t.medium.or(t.default))
                    .map(|t| t.url)
                    .unwrap_or_default();

                results.push(Video {
                    id: item.id,
                    title: snippet.title.unwrap_or_default(),
                    channel_id: snippet.channel_id.unwrap_or_default(),
                    channel_title: snippet.channel_title.unwrap_or_default(),
                    thumbnail_url: thumb,
                    duration,
                    view_count,
                    published_at,
                    embeddable: status.embeddable.unwrap_or(true),
                });
            }
        }

        Ok(results)
    }

    async fn get_channel_details(&self, channel_id: &str) -> Result<Channel, ApiError> {
        self.check_preconditions()?;

        let raw: ChannelListResponse = self
            .registry
            .get_json("channels/channel_primary.json")
            .map_err(|e| ApiError::ParseError(e.to_string()))?;

        if let Some(item) = raw.items.into_iter().next() {
            let snippet = item.snippet.unwrap_or_default();
            let content = item.content_details.unwrap_or_default();
            let stats = item.statistics.unwrap_or_default();
            let branding = item.branding_settings.unwrap_or_default();

            let uploads_playlist_id = content
                .related_playlists
                .and_then(|r| r.uploads)
                .unwrap_or_else(|| {
                    if let Some(rest) = channel_id.strip_prefix("UC") {
                        format!("UU{rest}")
                    } else {
                        channel_id.to_string()
                    }
                });

            let subscriber_count = stats
                .subscriber_count
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            let avatar = snippet
                .thumbnails
                .and_then(|t| t.medium.or(t.default))
                .map(|t| t.url)
                .unwrap_or_default();

            let banner = branding.image.and_then(|i| i.banner_external_url);

            Ok(Channel {
                id: item.id,
                title: snippet.title.unwrap_or_default(),
                description: snippet.description.unwrap_or_default(),
                avatar_url: avatar,
                banner_url: banner,
                subscriber_count,
                uploads_playlist_id,
            })
        } else {
            Err(ApiError::NotFound(format!("Channel '{channel_id}' not found")))
        }
    }

    async fn get_playlist_items(
        &self,
        _playlist_id: &str,
        page_token: Option<&str>,
    ) -> Result<PlaylistItemsResponse, ApiError> {
        self.check_preconditions()?;

        let fixture = if page_token.is_some() {
            "playlists/playlist_items_uploads_page2.json"
        } else {
            "playlists/playlist_items_uploads_page1.json"
        };

        let raw: PlaylistItemListResponse = self
            .registry
            .get_json(fixture)
            .map_err(|e| ApiError::ParseError(e.to_string()))?;

        let items = raw
            .items
            .into_iter()
            .map(|i| {
                let snippet = i.snippet.unwrap_or_default();
                let content = i.content_details.unwrap_or_default();
                PlaylistItem {
                    id: i.id,
                    playlist_id: snippet.playlist_id.unwrap_or_default(),
                    video_id: content.video_id.unwrap_or_default(),
                    title: snippet.title.unwrap_or_default(),
                    position: snippet.position.unwrap_or(0),
                }
            })
            .collect();

        Ok(PlaylistItemsResponse {
            items,
            next_page_token: raw.next_page_token,
            prev_page_token: raw.prev_page_token,
            total_results: raw.page_info.map(|p| p.total_results as u32).unwrap_or(0),
        })
    }

    async fn get_user_subscriptions(
        &self,
        page_token: Option<&str>,
    ) -> Result<SubscriptionsResponse, ApiError> {
        self.check_preconditions()?;

        let fixture = if page_token.is_some() {
            "subscriptions/subscriptions_page2.json"
        } else {
            "subscriptions/subscriptions_page1.json"
        };

        let raw: SubscriptionListResponse = self
            .registry
            .get_json(fixture)
            .map_err(|e| ApiError::ParseError(e.to_string()))?;

        let items = raw
            .items
            .into_iter()
            .map(|i| {
                let snippet = i.snippet.unwrap_or_default();
                let channel_id = snippet
                    .resource_id
                    .and_then(|r| r.channel_id)
                    .unwrap_or_default();
                let avatar = snippet
                    .thumbnails
                    .and_then(|t| t.medium.or(t.default))
                    .map(|t| t.url)
                    .unwrap_or_default();

                Channel {
                    id: channel_id.clone(),
                    title: snippet.title.unwrap_or_default(),
                    description: snippet.description.unwrap_or_default(),
                    avatar_url: avatar,
                    banner_url: None,
                    subscriber_count: 0,
                    uploads_playlist_id: if let Some(rest) = channel_id.strip_prefix("UC") {
                        format!("UU{rest}")
                    } else {
                        channel_id
                    },
                }
            })
            .collect();

        Ok(SubscriptionsResponse {
            items,
            next_page_token: raw.next_page_token,
            prev_page_token: raw.prev_page_token,
            total_results: raw.page_info.map(|p| p.total_results as u32).unwrap_or(0),
        })
    }

    async fn get_comment_threads(
        &self,
        _video_id: &str,
        _page_token: Option<&str>,
    ) -> Result<CommentThreadsResponse, ApiError> {
        self.check_preconditions()?;

        let raw: CommentThreadListResponse = self
            .registry
            .get_json("comments/comment_threads.json")
            .map_err(|e| ApiError::ParseError(e.to_string()))?;

        let items = raw
            .items
            .into_iter()
            .map(|t| {
                let snippet = t.snippet.unwrap_or_default();
                let top = snippet.top_level_comment.and_then(|c| c.snippet).unwrap_or_default();

                let published_at = top
                    .published_at
                    .as_deref()
                    .and_then(|p| DateTime::parse_from_rfc3339(p).ok())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

                Comment {
                    id: t.id,
                    author_name: top.author_display_name.unwrap_or_default(),
                    author_avatar_url: top.author_profile_image_url.unwrap_or_default(),
                    text: top.text_display.unwrap_or_default(),
                    like_count: top.like_count.unwrap_or(0),
                    published_at,
                    reply_count: snippet.total_reply_count.unwrap_or(0),
                }
            })
            .collect();

        Ok(CommentThreadsResponse {
            items,
            next_page_token: raw.next_page_token,
            total_results: raw.page_info.map(|p| p.total_results as u32).unwrap_or(0),
        })
    }

    async fn rate_video(&self, video_id: &str, rating: VideoRating) -> Result<(), ApiError> {
        self.check_preconditions()?;
        self.ratings
            .write()
            .unwrap()
            .insert(video_id.to_string(), rating);
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_client_search() {
        let client = MockYouTubeClient::new();
        let res = client
            .search(&SearchParams {
                query: "Rust".into(),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(!res.items.is_empty());
        assert_eq!(res.items[0].id, "dQw4w9WgXcQ");
    }

    #[tokio::test]
    async fn test_mock_client_video_details() {
        let client = MockYouTubeClient::new();
        let vids = client.get_video_details(&["dQw4w9WgXcQ"]).await.unwrap();

        assert_eq!(vids.len(), 1);
        assert_eq!(vids[0].id, "dQw4w9WgXcQ");
        assert_eq!(vids[0].duration, Duration::from_secs(213)); // PT3M33S
        assert_eq!(vids[0].view_count, 1549281920);
    }

    #[tokio::test]
    async fn test_mock_client_quota_exceeded() {
        let client = MockYouTubeClient::new();
        client.set_quota_exceeded(true);

        let err = client
            .search(&SearchParams {
                query: "Test".into(),
                ..Default::default()
            })
            .await
            .unwrap_err();

        match err {
            ApiError::QuotaExceeded(_) => {}
            other => panic!("Expected QuotaExceeded, got {other:?}"),
        }
    }
}
