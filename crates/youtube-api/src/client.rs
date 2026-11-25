use crate::error::YouTubeApiError;
use crate::models::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fluxtube_core::{Channel, Comment, Playlist, PlaylistItem, Video, VideoRating};
use fluxtube_network::{QuotaTracker, ResilientHttpClient};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use url::Url;

pub const DEFAULT_YOUTUBE_API_BASE: &str = "https://www.googleapis.com/youtube/v3";

/// Unified YouTube Data API v3 interface supporting live API and offline mock implementations.
#[async_trait]
pub trait YouTubeDataApi: Send + Sync {
    async fn search(&self, params: &SearchParams) -> Result<SearchResponse, YouTubeApiError>;

    async fn get_video_details(&self, video_ids: &[&str]) -> Result<Vec<Video>, YouTubeApiError>;

    async fn get_popular_videos(
        &self,
        region_code: &str,
        category_id: Option<&str>,
        page_token: Option<&str>,
        max_results: u32,
    ) -> Result<PageResult<Video>, YouTubeApiError>;

    async fn get_channel_details(&self, channel_id_or_handle: &str) -> Result<Channel, YouTubeApiError>;

    async fn get_my_channel(&self, access_token: &str) -> Result<Channel, YouTubeApiError>;

    async fn get_channel_playlists(
        &self,
        channel_id: &str,
        page_token: Option<&str>,
    ) -> Result<PageResult<Playlist>, YouTubeApiError>;

    async fn get_playlist_items(
        &self,
        playlist_id: &str,
        page_token: Option<&str>,
    ) -> Result<PageResult<PlaylistItem>, YouTubeApiError>;

    async fn get_user_playlists(
        &self,
        access_token: &str,
        page_token: Option<&str>,
    ) -> Result<PageResult<Playlist>, YouTubeApiError>;

    async fn create_playlist(
        &self,
        access_token: &str,
        title: &str,
        description: &str,
        privacy_status: &str,
    ) -> Result<Playlist, YouTubeApiError>;

    async fn add_playlist_item(
        &self,
        access_token: &str,
        playlist_id: &str,
        video_id: &str,
    ) -> Result<PlaylistItem, YouTubeApiError>;

    async fn remove_playlist_item(
        &self,
        access_token: &str,
        playlist_item_id: &str,
    ) -> Result<(), YouTubeApiError>;

    async fn get_user_subscriptions(
        &self,
        access_token: &str,
        page_token: Option<&str>,
    ) -> Result<PageResult<Channel>, YouTubeApiError>;

    async fn subscribe(
        &self,
        access_token: &str,
        channel_id: &str,
    ) -> Result<String, YouTubeApiError>;

    async fn unsubscribe(
        &self,
        access_token: &str,
        subscription_id: &str,
    ) -> Result<(), YouTubeApiError>;

    async fn get_comment_threads(
        &self,
        video_id: &str,
        order: &str,
        page_token: Option<&str>,
    ) -> Result<PageResult<Comment>, YouTubeApiError>;

    async fn post_comment(
        &self,
        access_token: &str,
        video_id: &str,
        text: &str,
    ) -> Result<Comment, YouTubeApiError>;

    async fn rate_video(
        &self,
        access_token: &str,
        video_id: &str,
        rating: VideoRating,
    ) -> Result<(), YouTubeApiError>;

    async fn get_video_rating(
        &self,
        access_token: &str,
        video_ids: &[&str],
    ) -> Result<Vec<(String, VideoRating)>, YouTubeApiError>;
}

/// Official YouTube Data API v3 client backed by `ResilientHttpClient` and `QuotaTracker`.
pub struct LiveYouTubeApiClient {
    http: ResilientHttpClient,
    quota: Arc<QuotaTracker>,
    api_key: String,
    base_url: String,
}

impl LiveYouTubeApiClient {
    pub fn new(
        http: ResilientHttpClient,
        quota: Arc<QuotaTracker>,
        api_key: impl Into<String>,
    ) -> Self {
        Self::with_base_url(http, quota, api_key, DEFAULT_YOUTUBE_API_BASE)
    }

    pub fn with_base_url(
        http: ResilientHttpClient,
        quota: Arc<QuotaTracker>,
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            http,
            quota,
            api_key: api_key.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    pub fn quota(&self) -> &Arc<QuotaTracker> {
        &self.quota
    }

    fn build_url(&self, path: &str, query: &[(&str, String)]) -> Result<Url, YouTubeApiError> {
        let mut url = Url::parse(&format!("{}/{}", self.base_url, path.trim_start_matches('/')))
            .map_err(|e| YouTubeApiError::InvalidInput(format!("Invalid endpoint URL: {}", e)))?;

        {
            let mut pairs = url.query_pairs_mut();
            if !self.api_key.is_empty() {
                pairs.append_pair("key", &self.api_key);
            }
            for (k, v) in query {
                pairs.append_pair(k, v);
            }
        }
        Ok(url)
    }

    async fn execute_json<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(&str, String)],
        bearer_token: Option<&str>,
        json_body: Option<&serde_json::Value>,
        is_search: bool,
        quota_units: u32,
    ) -> Result<T, YouTubeApiError> {
        self.quota.record_consumption(quota_units, is_search)?;

        let url = self.build_url(path, query)?;
        let mut req_builder = self.http.client().request(method, url);

        if let Some(token) = bearer_token {
            req_builder = req_builder.bearer_auth(token);
        }
        if let Some(body) = json_body {
            req_builder = req_builder.json(body);
        }

        let request = req_builder.build()?;
        let response = self.http.execute(request).await?;
        let status = response.status();
        let bytes = response.bytes().await?;

        if !status.is_success() {
            return Err(Self::map_api_error(status, &bytes));
        }

        serde_json::from_slice(&bytes)
            .map_err(|e| YouTubeApiError::ParseError(format!("JSON decode error: {}", e)))
    }

    async fn execute_empty(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(&str, String)],
        bearer_token: Option<&str>,
        quota_units: u32,
    ) -> Result<(), YouTubeApiError> {
        self.quota.record_consumption(quota_units, false)?;
        let url = self.build_url(path, query)?;
        let mut req_builder = self.http.client().request(method, url);
        if let Some(token) = bearer_token {
            req_builder = req_builder.bearer_auth(token);
        }

        let request = req_builder.build()?;
        let response = self.http.execute(request).await?;
        let status = response.status();
        if !status.is_success() {
            let bytes = response.bytes().await?;
            return Err(Self::map_api_error(status, &bytes));
        }
        Ok(())
    }

    fn map_api_error(status: reqwest::StatusCode, body: &[u8]) -> YouTubeApiError {
        if let Ok(env) = serde_json::from_slice::<RawApiErrorEnvelope>(body) {
            let msg = env
                .error
                .message
                .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
            let reason = env
                .error
                .errors
                .first()
                .and_then(|e| e.reason.clone())
                .unwrap_or_default();

            return match status.as_u16() {
                400 => YouTubeApiError::BadRequest(msg),
                401 => YouTubeApiError::Unauthorized(msg),
                403 => match reason.as_str() {
                    "quotaExceeded" | "dailyLimitExceeded" | "rateLimitExceeded" => {
                        YouTubeApiError::QuotaExceeded(msg)
                    }
                    "commentsDisabled" => YouTubeApiError::CommentsDisabled(msg),
                    _ => YouTubeApiError::Forbidden { reason, message: msg },
                },
                404 => YouTubeApiError::NotFound(msg),
                500..=599 => YouTubeApiError::ServerError {
                    status: status.as_u16(),
                    message: msg,
                },
                _ => YouTubeApiError::BadRequest(msg),
            };
        }

        let raw_text = String::from_utf8_lossy(body).into_owned();
        match status.as_u16() {
            401 => YouTubeApiError::Unauthorized(raw_text),
            403 => YouTubeApiError::Forbidden {
                reason: "forbidden".to_string(),
                message: raw_text,
            },
            404 => YouTubeApiError::NotFound(raw_text),
            500..=599 => YouTubeApiError::ServerError {
                status: status.as_u16(),
                message: raw_text,
            },
            _ => YouTubeApiError::BadRequest(raw_text),
        }
    }
}

#[async_trait]
impl YouTubeDataApi for LiveYouTubeApiClient {
    async fn search(&self, params: &SearchParams) -> Result<SearchResponse, YouTubeApiError> {
        let mut query = vec![
            ("part", "snippet".to_string()),
            ("q", params.query.clone()),
            ("maxResults", params.max_results.unwrap_or(20).clamp(1, 50).to_string()),
        ];

        if let Some(ref pt) = params.page_token {
            query.push(("pageToken", pt.clone()));
        }
        if let Some(order) = params.order {
            query.push(("order", order.as_api_str().to_string()));
        }
        if let Some(rt) = params.resource_type {
            query.push(("type", rt.as_api_str().to_string()));
        } else {
            query.push(("type", "video".to_string()));
        }
        if let Some(dur) = params.video_duration {
            if dur != VideoDurationFilter::Any {
                query.push(("videoDuration", dur.as_api_str().to_string()));
            }
        }
        if let Some(after) = params.published_after {
            query.push(("publishedAfter", after.to_rfc3339()));
        }
        if let Some(before) = params.published_before {
            query.push(("publishedBefore", before.to_rfc3339()));
        }
        if let Some(ref cid) = params.channel_id {
            query.push(("channelId", cid.clone()));
        }
        if let Some(ref evt) = params.event_type {
            query.push(("eventType", evt.clone()));
        }

        let raw: RawSearchListResponse = self
            .execute_json(reqwest::Method::GET, "search", &query, None, None, true, 100)
            .await?;

        let items = raw
            .items
            .into_iter()
            .map(|r| {
                let kind = r.id.kind.unwrap_or_else(|| "youtube#video".to_string());
                let id = r
                    .id
                    .video_id
                    .or(r.id.channel_id)
                    .or(r.id.playlist_id)
                    .unwrap_or_default();
                let snippet = r.snippet.unwrap_or_default();
                let (thumbnail_url, _) = snippet
                    .thumbnails
                    .as_ref()
                    .map(RawThumbnailSet::into_domain)
                    .unwrap_or_default();
                let published_at = snippet
                    .published_at
                    .as_deref()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

                SearchResultItem {
                    kind,
                    id,
                    title: snippet.title.unwrap_or_default(),
                    description: snippet.description.unwrap_or_default(),
                    channel_id: snippet.channel_id.unwrap_or_default(),
                    channel_title: snippet.channel_title.unwrap_or_default(),
                    thumbnail_url,
                    published_at,
                    live_broadcast_content: snippet
                        .live_broadcast_content
                        .filter(|v| v != "none"),
                }
            })
            .collect();

        Ok(PageResult {
            items,
            next_page_token: raw.next_page_token,
            prev_page_token: raw.prev_page_token,
            total_results: raw.page_info.and_then(|p| p.total_results).unwrap_or(0),
        })
    }

    async fn get_video_details(&self, video_ids: &[&str]) -> Result<Vec<Video>, YouTubeApiError> {
        if video_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        for chunk in video_ids.chunks(50) {
            let query = vec![
                ("part", "snippet,contentDetails,statistics,status".to_string()),
                ("id", chunk.join(",")),
            ];
            let raw: RawVideoListResponse = self
                .execute_json(reqwest::Method::GET, "videos", &query, None, None, false, 1)
                .await?;
            results.extend(raw.items.into_iter().map(RawVideoItem::into_domain));
        }

        Ok(results)
    }

    async fn get_popular_videos(
        &self,
        region_code: &str,
        category_id: Option<&str>,
        page_token: Option<&str>,
        max_results: u32,
    ) -> Result<PageResult<Video>, YouTubeApiError> {
        let mut query = vec![
            ("part", "snippet,contentDetails,statistics,status".to_string()),
            ("chart", "mostPopular".to_string()),
            ("regionCode", region_code.to_string()),
            ("maxResults", max_results.clamp(1, 50).to_string()),
        ];
        if let Some(cat) = category_id {
            query.push(("videoCategoryId", cat.to_string()));
        }
        if let Some(pt) = page_token {
            query.push(("pageToken", pt.to_string()));
        }

        let raw: RawVideoListResponse = self
            .execute_json(reqwest::Method::GET, "videos", &query, None, None, false, 1)
            .await?;

        let items = raw.items.into_iter().map(RawVideoItem::into_domain).collect();
        Ok(PageResult {
            items,
            next_page_token: raw.next_page_token,
            prev_page_token: raw.prev_page_token,
            total_results: raw.page_info.and_then(|p| p.total_results).unwrap_or(0),
        })
    }

    async fn get_channel_details(&self, channel_id_or_handle: &str) -> Result<Channel, YouTubeApiError> {
        let mut query = vec![(
            "part",
            "snippet,contentDetails,statistics,brandingSettings".to_string(),
        )];
        if channel_id_or_handle.starts_with('@') {
            query.push(("forHandle", channel_id_or_handle.to_string()));
        } else {
            query.push(("id", channel_id_or_handle.to_string()));
        }

        let raw: RawChannelListResponse = self
            .execute_json(reqwest::Method::GET, "channels", &query, None, None, false, 1)
            .await?;

        raw.items
            .into_iter()
            .next()
            .map(RawChannelItem::into_domain)
            .ok_or_else(|| {
                YouTubeApiError::NotFound(format!("Channel '{}' not found", channel_id_or_handle))
            })
    }

    async fn get_my_channel(&self, access_token: &str) -> Result<Channel, YouTubeApiError> {
        let query = vec![
            (
                "part",
                "snippet,contentDetails,statistics,brandingSettings".to_string(),
            ),
            ("mine", "true".to_string()),
        ];

        let raw: RawChannelListResponse = self
            .execute_json(
                reqwest::Method::GET,
                "channels",
                &query,
                Some(access_token),
                None,
                false,
                1,
            )
            .await?;

        raw.items
            .into_iter()
            .next()
            .map(RawChannelItem::into_domain)
            .ok_or_else(|| {
                YouTubeApiError::NotFound("No YouTube channel found for signed-in Google account".to_string())
            })
    }

    async fn get_channel_playlists(
        &self,
        channel_id: &str,
        page_token: Option<&str>,
    ) -> Result<PageResult<Playlist>, YouTubeApiError> {
        let mut query = vec![
            ("part", "snippet,contentDetails".to_string()),
            ("channelId", channel_id.to_string()),
            ("maxResults", "25".to_string()),
        ];
        if let Some(pt) = page_token {
            query.push(("pageToken", pt.to_string()));
        }

        let raw: RawPlaylistListResponse = self
            .execute_json(reqwest::Method::GET, "playlists", &query, None, None, false, 1)
            .await?;

        let items = raw
            .items
            .into_iter()
            .map(RawPlaylistItemEntry::into_domain)
            .collect();
        Ok(PageResult {
            items,
            next_page_token: raw.next_page_token,
            prev_page_token: raw.prev_page_token,
            total_results: raw.page_info.and_then(|p| p.total_results).unwrap_or(0),
        })
    }

    async fn get_playlist_items(
        &self,
        playlist_id: &str,
        page_token: Option<&str>,
    ) -> Result<PageResult<PlaylistItem>, YouTubeApiError> {
        let mut query = vec![
            ("part", "snippet,contentDetails".to_string()),
            ("playlistId", playlist_id.to_string()),
            ("maxResults", "25".to_string()),
        ];
        if let Some(pt) = page_token {
            query.push(("pageToken", pt.to_string()));
        }

        let raw: RawPlaylistItemsListResponse = self
            .execute_json(
                reqwest::Method::GET,
                "playlistItems",
                &query,
                None,
                None,
                false,
                1,
            )
            .await?;

        let items = raw
            .items
            .into_iter()
            .map(|r| r.into_domain(playlist_id))
            .collect();
        Ok(PageResult {
            items,
            next_page_token: raw.next_page_token,
            prev_page_token: raw.prev_page_token,
            total_results: raw.page_info.and_then(|p| p.total_results).unwrap_or(0),
        })
    }

    async fn get_user_playlists(
        &self,
        access_token: &str,
        page_token: Option<&str>,
    ) -> Result<PageResult<Playlist>, YouTubeApiError> {
        let mut query = vec![
            ("part", "snippet,contentDetails".to_string()),
            ("mine", "true".to_string()),
            ("maxResults", "25".to_string()),
        ];
        if let Some(pt) = page_token {
            query.push(("pageToken", pt.to_string()));
        }

        let raw: RawPlaylistListResponse = self
            .execute_json(
                reqwest::Method::GET,
                "playlists",
                &query,
                Some(access_token),
                None,
                false,
                1,
            )
            .await?;

        let items = raw
            .items
            .into_iter()
            .map(RawPlaylistItemEntry::into_domain)
            .collect();
        Ok(PageResult {
            items,
            next_page_token: raw.next_page_token,
            prev_page_token: raw.prev_page_token,
            total_results: raw.page_info.and_then(|p| p.total_results).unwrap_or(0),
        })
    }

    async fn create_playlist(
        &self,
        access_token: &str,
        title: &str,
        description: &str,
        privacy_status: &str,
    ) -> Result<Playlist, YouTubeApiError> {
        let query = vec![("part", "snippet,status".to_string())];
        let body = serde_json::json!({
            "snippet": {
                "title": title,
                "description": description
            },
            "status": {
                "privacyStatus": privacy_status
            }
        });

        let raw: RawPlaylistItemEntry = self
            .execute_json(
                reqwest::Method::POST,
                "playlists",
                &query,
                Some(access_token),
                Some(&body),
                false,
                50,
            )
            .await?;
        Ok(raw.into_domain())
    }

    async fn add_playlist_item(
        &self,
        access_token: &str,
        playlist_id: &str,
        video_id: &str,
    ) -> Result<PlaylistItem, YouTubeApiError> {
        let query = vec![("part", "snippet".to_string())];
        let body = serde_json::json!({
            "snippet": {
                "playlistId": playlist_id,
                "resourceId": {
                    "kind": "youtube#video",
                    "videoId": video_id
                }
            }
        });

        let raw: RawPlaylistItemRow = self
            .execute_json(
                reqwest::Method::POST,
                "playlistItems",
                &query,
                Some(access_token),
                Some(&body),
                false,
                50,
            )
            .await?;
        Ok(raw.into_domain(playlist_id))
    }

    async fn remove_playlist_item(
        &self,
        access_token: &str,
        playlist_item_id: &str,
    ) -> Result<(), YouTubeApiError> {
        let query = vec![("id", playlist_item_id.to_string())];
        self.execute_empty(
            reqwest::Method::DELETE,
            "playlistItems",
            &query,
            Some(access_token),
            50,
        )
        .await
    }

    async fn get_user_subscriptions(
        &self,
        access_token: &str,
        page_token: Option<&str>,
    ) -> Result<PageResult<Channel>, YouTubeApiError> {
        let mut query = vec![
            ("part", "snippet".to_string()),
            ("mine", "true".to_string()),
            ("maxResults", "50".to_string()),
        ];
        if let Some(pt) = page_token {
            query.push(("pageToken", pt.to_string()));
        }

        let raw: RawSubscriptionsListResponse = self
            .execute_json(
                reqwest::Method::GET,
                "subscriptions",
                &query,
                Some(access_token),
                None,
                false,
                1,
            )
            .await?;

        let items = raw
            .items
            .into_iter()
            .map(RawSubscriptionItem::into_domain)
            .collect();
        Ok(PageResult {
            items,
            next_page_token: raw.next_page_token,
            prev_page_token: raw.prev_page_token,
            total_results: raw.page_info.and_then(|p| p.total_results).unwrap_or(0),
        })
    }

    async fn subscribe(
        &self,
        access_token: &str,
        channel_id: &str,
    ) -> Result<String, YouTubeApiError> {
        let query = vec![("part", "snippet".to_string())];
        let body = serde_json::json!({
            "snippet": {
                "resourceId": {
                    "kind": "youtube#channel",
                    "channelId": channel_id
                }
            }
        });
        let raw: RawSubscriptionItem = self
            .execute_json(
                reqwest::Method::POST,
                "subscriptions",
                &query,
                Some(access_token),
                Some(&body),
                false,
                50,
            )
            .await?;
        Ok(raw.id)
    }

    async fn unsubscribe(
        &self,
        access_token: &str,
        subscription_id: &str,
    ) -> Result<(), YouTubeApiError> {
        let query = vec![("id", subscription_id.to_string())];
        self.execute_empty(
            reqwest::Method::DELETE,
            "subscriptions",
            &query,
            Some(access_token),
            50,
        )
        .await
    }

    async fn get_comment_threads(
        &self,
        video_id: &str,
        order: &str,
        page_token: Option<&str>,
    ) -> Result<PageResult<Comment>, YouTubeApiError> {
        let mut query = vec![
            ("part", "snippet".to_string()),
            ("videoId", video_id.to_string()),
            ("order", order.to_string()),
            ("maxResults", "20".to_string()),
            ("textFormat", "plainText".to_string()),
        ];
        if let Some(pt) = page_token {
            query.push(("pageToken", pt.to_string()));
        }

        let raw: RawCommentThreadsListResponse = self
            .execute_json(
                reqwest::Method::GET,
                "commentThreads",
                &query,
                None,
                None,
                false,
                1,
            )
            .await?;

        let items = raw
            .items
            .into_iter()
            .map(|r| r.into_domain(video_id))
            .collect();
        Ok(PageResult {
            items,
            next_page_token: raw.next_page_token,
            prev_page_token: raw.prev_page_token,
            total_results: raw.page_info.and_then(|p| p.total_results).unwrap_or(0),
        })
    }

    async fn post_comment(
        &self,
        access_token: &str,
        video_id: &str,
        text: &str,
    ) -> Result<Comment, YouTubeApiError> {
        let query = vec![("part", "snippet".to_string())];
        let body = serde_json::json!({
            "snippet": {
                "videoId": video_id,
                "topLevelComment": {
                    "snippet": {
                        "textOriginal": text
                    }
                }
            }
        });

        let raw: RawCommentThreadItem = self
            .execute_json(
                reqwest::Method::POST,
                "commentThreads",
                &query,
                Some(access_token),
                Some(&body),
                false,
                50,
            )
            .await?;
        Ok(raw.into_domain(video_id))
    }

    async fn rate_video(
        &self,
        access_token: &str,
        video_id: &str,
        rating: VideoRating,
    ) -> Result<(), YouTubeApiError> {
        let rating_str = match rating {
            VideoRating::Like => "like",
            VideoRating::Dislike => "dislike",
            VideoRating::None => "none",
        };
        let query = vec![
            ("id", video_id.to_string()),
            ("rating", rating_str.to_string()),
        ];
        self.execute_empty(
            reqwest::Method::POST,
            "videos/rate",
            &query,
            Some(access_token),
            50,
        )
        .await
    }

    async fn get_video_rating(
        &self,
        access_token: &str,
        video_ids: &[&str],
    ) -> Result<Vec<(String, VideoRating)>, YouTubeApiError> {
        if video_ids.is_empty() {
            return Ok(Vec::new());
        }
        let query = vec![("id", video_ids.join(","))];
        let raw: RawVideoRatingsResponse = self
            .execute_json(
                reqwest::Method::GET,
                "videos/getRating",
                &query,
                Some(access_token),
                None,
                false,
                1,
            )
            .await?;
        Ok(raw
            .items
            .into_iter()
            .map(RawVideoRatingItem::into_domain)
            .collect())
    }
}
