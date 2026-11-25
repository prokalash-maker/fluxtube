use chrono::{DateTime, Utc};
use fluxtube_core::{Channel, Comment, Playlist, PlaylistItem, Thumbnail, ThumbnailQuality, Video, VideoRating};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchOrder {
    Relevance,
    Date,
    ViewCount,
    Rating,
    Title,
}

impl SearchOrder {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::Date => "date",
            Self::ViewCount => "viewCount",
            Self::Rating => "rating",
            Self::Title => "title",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoDurationFilter {
    Any,
    Short,
    Medium,
    Long,
}

impl VideoDurationFilter {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Short => "short",
            Self::Medium => "medium",
            Self::Long => "long",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchResourceType {
    Video,
    Channel,
    Playlist,
}

impl SearchResourceType {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Channel => "channel",
            Self::Playlist => "playlist",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SearchParams {
    pub query: String,
    pub page_token: Option<String>,
    pub max_results: Option<u32>,
    pub order: Option<SearchOrder>,
    pub video_duration: Option<VideoDurationFilter>,
    pub resource_type: Option<SearchResourceType>,
    pub published_after: Option<DateTime<Utc>>,
    pub published_before: Option<DateTime<Utc>>,
    pub channel_id: Option<String>,
    pub event_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub kind: String,
    pub id: String,
    pub title: String,
    pub description: String,
    pub channel_id: String,
    pub channel_title: String,
    pub thumbnail_url: String,
    pub published_at: DateTime<Utc>,
    pub live_broadcast_content: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageResult<T> {
    pub items: Vec<T>,
    pub next_page_token: Option<String>,
    pub prev_page_token: Option<String>,
    pub total_results: u64,
}

impl<T> Default for PageResult<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            next_page_token: None,
            prev_page_token: None,
            total_results: 0,
        }
    }
}

pub type SearchResponse = PageResult<SearchResultItem>;

/// Parses an ISO 8601 YouTube duration string (e.g. `PT1H2M30S`, `PT4M13S`, `P0D`) into a `Duration`.
pub fn parse_iso8601_duration(iso: &str) -> Duration {
    let s = iso.trim();
    if !s.starts_with('P') {
        return Duration::ZERO;
    }

    let mut days = 0u64;
    let mut hours = 0u64;
    let mut minutes = 0u64;
    let mut seconds = 0u64;

    let mut in_time = false;
    let mut num_buf = String::new();

    for ch in s[1..].chars() {
        if ch == 'T' {
            in_time = true;
            num_buf.clear();
            continue;
        }
        if ch.is_ascii_digit() {
            num_buf.push(ch);
            continue;
        }
        let val: u64 = num_buf.parse().unwrap_or(0);
        num_buf.clear();
        match (in_time, ch) {
            (false, 'D') => days = val,
            (true, 'H') => hours = val,
            (true, 'M') => minutes = val,
            (true, 'S') => seconds = val,
            _ => {}
        }
    }

    Duration::from_secs(days * 86400 + hours * 3600 + minutes * 60 + seconds)
}

/// Derives the standard YouTube uploads playlist ID from a channel ID (`UC...` -> `UU...`).
pub fn derive_uploads_playlist_id(channel_id: &str) -> String {
    if let Some(rest) = channel_id.strip_prefix("UC") {
        format!("UU{}", rest)
    } else {
        channel_id.to_string()
    }
}

// ============================================================================
// Raw YouTube Data API v3 Serde Models
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawApiErrorEnvelope {
    pub error: RawApiErrorBody,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawApiErrorBody {
    #[allow(dead_code)]
    pub code: Option<u16>,
    pub message: Option<String>,
    #[serde(default)]
    pub errors: Vec<RawApiErrorDetail>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawApiErrorDetail {
    pub reason: Option<String>,
    #[allow(dead_code)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawPageInfo {
    pub total_results: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawThumbnail {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawThumbnailSet {
    pub default: Option<RawThumbnail>,
    pub medium: Option<RawThumbnail>,
    pub high: Option<RawThumbnail>,
    pub standard: Option<RawThumbnail>,
    pub maxres: Option<RawThumbnail>,
}

impl RawThumbnailSet {
    pub fn into_domain(&self) -> (String, Vec<Thumbnail>) {
        let mut out = Vec::new();
        let mut push = |opt: &Option<RawThumbnail>, q: ThumbnailQuality| {
            if let Some(t) = opt {
                out.push(Thumbnail {
                    url: t.url.clone(),
                    width: t.width,
                    height: t.height,
                    quality: q,
                });
            }
        };
        push(&self.default, ThumbnailQuality::Default);
        push(&self.medium, ThumbnailQuality::Medium);
        push(&self.high, ThumbnailQuality::High);
        push(&self.standard, ThumbnailQuality::Standard);
        push(&self.maxres, ThumbnailQuality::Maxres);

        let preferred = self
            .medium
            .as_ref()
            .or(self.high.as_ref())
            .or(self.default.as_ref())
            .map(|t| t.url.clone())
            .unwrap_or_default();
        (preferred, out)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawSearchListResponse {
    #[serde(default)]
    pub items: Vec<RawSearchResult>,
    pub next_page_token: Option<String>,
    pub prev_page_token: Option<String>,
    pub page_info: Option<RawPageInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawSearchResult {
    pub id: RawResourceId,
    pub snippet: Option<RawSearchSnippet>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawResourceId {
    pub kind: Option<String>,
    pub video_id: Option<String>,
    pub channel_id: Option<String>,
    pub playlist_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawSearchSnippet {
    pub title: Option<String>,
    pub description: Option<String>,
    pub channel_id: Option<String>,
    pub channel_title: Option<String>,
    pub published_at: Option<String>,
    pub thumbnails: Option<RawThumbnailSet>,
    pub live_broadcast_content: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawVideoListResponse {
    #[serde(default)]
    pub items: Vec<RawVideoItem>,
    pub next_page_token: Option<String>,
    pub prev_page_token: Option<String>,
    pub page_info: Option<RawPageInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawVideoItem {
    pub id: String,
    pub snippet: Option<RawVideoSnippet>,
    pub content_details: Option<RawVideoContentDetails>,
    pub statistics: Option<RawVideoStatistics>,
    pub status: Option<RawVideoStatus>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawVideoSnippet {
    pub title: Option<String>,
    pub description: Option<String>,
    pub channel_id: Option<String>,
    pub channel_title: Option<String>,
    pub published_at: Option<String>,
    pub thumbnails: Option<RawThumbnailSet>,
    pub live_broadcast_content: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawVideoContentDetails {
    pub duration: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawVideoStatistics {
    pub view_count: Option<String>,
    pub like_count: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawVideoStatus {
    pub embeddable: Option<bool>,
}

impl RawVideoItem {
    pub fn into_domain(self) -> Video {
        let snippet = self.snippet.unwrap_or_default();
        let content = self.content_details.unwrap_or_default();
        let stats = self.statistics.unwrap_or_default();
        let status = self.status.unwrap_or_default();
        let (thumbnail_url, thumbnails) = snippet
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

        Video {
            id: self.id,
            title: snippet.title.unwrap_or_default(),
            description: snippet.description.unwrap_or_default(),
            channel_id: snippet.channel_id.unwrap_or_default(),
            channel_title: snippet.channel_title.unwrap_or_default(),
            thumbnail_url,
            thumbnails,
            duration: content
                .duration
                .as_deref()
                .map(parse_iso8601_duration)
                .unwrap_or(Duration::ZERO),
            view_count: stats
                .view_count
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            like_count: stats.like_count.as_deref().and_then(|s| s.parse().ok()),
            published_at,
            embeddable: status.embeddable.unwrap_or(true),
            live_broadcast_content: snippet
                .live_broadcast_content
                .filter(|v| v != "none"),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawChannelListResponse {
    #[serde(default)]
    pub items: Vec<RawChannelItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawChannelItem {
    pub id: String,
    pub snippet: Option<RawChannelSnippet>,
    pub content_details: Option<RawChannelContentDetails>,
    pub statistics: Option<RawChannelStatistics>,
    pub branding_settings: Option<RawBrandingSettings>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawChannelSnippet {
    pub title: Option<String>,
    pub description: Option<String>,
    pub custom_url: Option<String>,
    pub thumbnails: Option<RawThumbnailSet>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawChannelContentDetails {
    pub related_playlists: Option<RawRelatedPlaylists>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawRelatedPlaylists {
    pub uploads: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawChannelStatistics {
    pub subscriber_count: Option<String>,
    pub video_count: Option<String>,
    pub view_count: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawBrandingSettings {
    pub image: Option<RawBrandingImage>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawBrandingImage {
    pub banner_external_url: Option<String>,
}

impl RawChannelItem {
    pub fn into_domain(self) -> Channel {
        let snippet = self.snippet.unwrap_or_default();
        let stats = self.statistics.unwrap_or_default();
        let uploads = self
            .content_details
            .and_then(|cd| cd.related_playlists)
            .and_then(|rp| rp.uploads)
            .unwrap_or_else(|| derive_uploads_playlist_id(&self.id));
        let banner_url = self
            .branding_settings
            .and_then(|bs| bs.image)
            .and_then(|img| img.banner_external_url);
        let (avatar_url, _) = snippet
            .thumbnails
            .as_ref()
            .map(RawThumbnailSet::into_domain)
            .unwrap_or_default();

        Channel {
            id: self.id,
            title: snippet.title.unwrap_or_default(),
            description: snippet.description.unwrap_or_default(),
            custom_url: snippet.custom_url,
            avatar_url,
            banner_url,
            subscriber_count: stats
                .subscriber_count
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            video_count: stats
                .video_count
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            view_count: stats.view_count.as_deref().and_then(|s| s.parse().ok()),
            uploads_playlist_id: uploads,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawPlaylistListResponse {
    #[serde(default)]
    pub items: Vec<RawPlaylistItemEntry>,
    pub next_page_token: Option<String>,
    pub prev_page_token: Option<String>,
    pub page_info: Option<RawPageInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawPlaylistItemEntry {
    pub id: String,
    pub snippet: Option<RawPlaylistSnippet>,
    pub content_details: Option<RawPlaylistContentDetails>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawPlaylistSnippet {
    pub title: Option<String>,
    pub description: Option<String>,
    pub channel_id: Option<String>,
    pub channel_title: Option<String>,
    pub published_at: Option<String>,
    pub thumbnails: Option<RawThumbnailSet>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawPlaylistContentDetails {
    pub item_count: Option<u32>,
}

impl RawPlaylistItemEntry {
    pub fn into_domain(self) -> Playlist {
        let snippet = self.snippet.unwrap_or_default();
        let count = self.content_details.and_then(|c| c.item_count).unwrap_or(0);
        let (thumb, _) = snippet
            .thumbnails
            .as_ref()
            .map(RawThumbnailSet::into_domain)
            .unwrap_or_default();
        let created_at = snippet
            .published_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Playlist {
            id: self.id,
            title: snippet.title.unwrap_or_default(),
            description: snippet.description.unwrap_or_default(),
            item_count: count,
            thumbnail_url: if thumb.is_empty() { None } else { Some(thumb) },
            channel_id: snippet.channel_id,
            channel_title: snippet.channel_title,
            is_local: false,
            account_id: None,
            created_at,
            updated_at: created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawPlaylistItemsListResponse {
    #[serde(default)]
    pub items: Vec<RawPlaylistItemRow>,
    pub next_page_token: Option<String>,
    pub prev_page_token: Option<String>,
    pub page_info: Option<RawPageInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawPlaylistItemRow {
    pub id: String,
    pub snippet: Option<RawPlaylistItemSnippet>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawPlaylistItemSnippet {
    pub playlist_id: Option<String>,
    pub title: Option<String>,
    pub position: Option<u32>,
    pub published_at: Option<String>,
    pub thumbnails: Option<RawThumbnailSet>,
    pub resource_id: Option<RawResourceId>,
}

impl RawPlaylistItemRow {
    pub fn into_domain(self, fallback_playlist_id: &str) -> PlaylistItem {
        let snippet = self.snippet.unwrap_or_default();
        let video_id = snippet
            .resource_id
            .and_then(|r| r.video_id)
            .unwrap_or_default();
        let (thumb, _) = snippet
            .thumbnails
            .as_ref()
            .map(RawThumbnailSet::into_domain)
            .unwrap_or_default();
        let added_at = snippet
            .published_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        PlaylistItem {
            id: self.id,
            playlist_id: snippet
                .playlist_id
                .unwrap_or_else(|| fallback_playlist_id.to_string()),
            video_id,
            title: snippet.title.unwrap_or_default(),
            thumbnail_url: thumb,
            position: snippet.position.unwrap_or(0),
            added_at,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawSubscriptionsListResponse {
    #[serde(default)]
    pub items: Vec<RawSubscriptionItem>,
    pub next_page_token: Option<String>,
    pub prev_page_token: Option<String>,
    pub page_info: Option<RawPageInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawSubscriptionItem {
    pub id: String,
    pub snippet: Option<RawSubscriptionSnippet>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawSubscriptionSnippet {
    pub title: Option<String>,
    pub description: Option<String>,
    pub resource_id: Option<RawResourceId>,
    pub thumbnails: Option<RawThumbnailSet>,
}

impl RawSubscriptionItem {
    pub fn into_domain(self) -> Channel {
        let snippet = self.snippet.unwrap_or_default();
        let channel_id = snippet
            .resource_id
            .and_then(|r| r.channel_id)
            .unwrap_or(self.id);
        let (avatar_url, _) = snippet
            .thumbnails
            .as_ref()
            .map(RawThumbnailSet::into_domain)
            .unwrap_or_default();
        let uploads = derive_uploads_playlist_id(&channel_id);
        Channel {
            id: channel_id,
            title: snippet.title.unwrap_or_default(),
            description: snippet.description.unwrap_or_default(),
            custom_url: None,
            avatar_url,
            banner_url: None,
            subscriber_count: 0,
            video_count: 0,
            view_count: None,
            uploads_playlist_id: uploads,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawCommentThreadsListResponse {
    #[serde(default)]
    pub items: Vec<RawCommentThreadItem>,
    pub next_page_token: Option<String>,
    pub prev_page_token: Option<String>,
    pub page_info: Option<RawPageInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawCommentThreadItem {
    pub id: String,
    pub snippet: Option<RawCommentThreadSnippet>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawCommentThreadSnippet {
    pub video_id: Option<String>,
    pub total_reply_count: Option<u32>,
    pub top_level_comment: Option<RawCommentResource>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawCommentResource {
    pub id: String,
    pub snippet: Option<RawCommentSnippet>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawCommentSnippet {
    pub video_id: Option<String>,
    pub author_display_name: Option<String>,
    pub author_profile_image_url: Option<String>,
    pub author_channel_id: Option<RawAuthorChannelId>,
    pub text_display: Option<String>,
    pub text_original: Option<String>,
    pub like_count: Option<u32>,
    pub published_at: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct RawAuthorChannelId {
    pub value: Option<String>,
}

impl RawCommentThreadItem {
    pub fn into_domain(self, fallback_video_id: &str) -> Comment {
        let thread = self.snippet.unwrap_or_default();
        let reply_count = thread.total_reply_count.unwrap_or(0);
        let top = thread.top_level_comment;
        let top_id = top.as_ref().map(|c| c.id.clone()).unwrap_or(self.id);
        let cs = top.and_then(|c| c.snippet).unwrap_or_default();

        let published_at = cs
            .published_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        Comment {
            id: top_id,
            video_id: thread
                .video_id
                .or(cs.video_id)
                .unwrap_or_else(|| fallback_video_id.to_string()),
            author_name: cs.author_display_name.unwrap_or_default(),
            author_avatar_url: cs.author_profile_image_url.unwrap_or_default(),
            author_channel_id: cs.author_channel_id.and_then(|a| a.value),
            text: cs.text_original.or(cs.text_display).unwrap_or_default(),
            like_count: cs.like_count.unwrap_or(0),
            published_at,
            reply_count,
            is_channel_owner: false,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawVideoRatingsResponse {
    #[serde(default)]
    pub items: Vec<RawVideoRatingItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawVideoRatingItem {
    pub video_id: String,
    pub rating: String,
}

impl RawVideoRatingItem {
    pub fn into_domain(self) -> (String, VideoRating) {
        let rating = match self.rating.as_str() {
            "like" => VideoRating::Like,
            "dislike" => VideoRating::Dislike,
            _ => VideoRating::None,
        };
        (self.video_id, rating)
    }
}
