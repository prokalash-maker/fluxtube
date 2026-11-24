//! Domain Models & YouTube Data API v3 Response Schemas for FluxTube E2E Test Harness.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================================================
// 1. Core Domain Models (matching PROJECT.md §Interface Contracts)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub channel_id: String,
    pub channel_title: String,
    pub thumbnail_url: String,
    pub duration: Duration,
    pub view_count: u64,
    pub published_at: DateTime<Utc>,
    pub embeddable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub title: String,
    pub description: String,
    pub avatar_url: String,
    pub banner_url: Option<String>,
    pub subscriber_count: u64,
    pub uploads_playlist_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub title: String,
    pub description: String,
    pub item_count: u32,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub id: String,
    pub playlist_id: String,
    pub video_id: String,
    pub title: String,
    pub position: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub author_name: String,
    pub author_avatar_url: String,
    pub text: String,
    pub like_count: u32,
    pub published_at: DateTime<Utc>,
    pub reply_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoRating {
    Like,
    Dislike,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PerformanceProfile {
    UltraLite,
    Balanced,
    HighQuality,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SearchParams {
    pub query: String,
    pub page_token: Option<String>,
    pub max_results: Option<u32>,
    pub order: Option<String>,
    pub video_duration: Option<String>,
    pub resource_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SearchResponse {
    pub items: Vec<SearchResultItem>,
    pub next_page_token: Option<String>,
    pub prev_page_token: Option<String>,
    pub total_results: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub kind: String,
    pub id: String,
    pub title: String,
    pub description: String,
    pub channel_id: String,
    pub channel_title: String,
    pub thumbnail_url: String,
    pub published_at: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PlaylistItemsResponse {
    pub items: Vec<PlaylistItem>,
    pub next_page_token: Option<String>,
    pub prev_page_token: Option<String>,
    pub total_results: u32,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SubscriptionsResponse {
    pub items: Vec<Channel>,
    pub next_page_token: Option<String>,
    pub prev_page_token: Option<String>,
    pub total_results: u32,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommentThreadsResponse {
    pub items: Vec<Comment>,
    pub next_page_token: Option<String>,
    pub total_results: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    Network(String),
    NotFound(String),
    Unauthorized(String),
    QuotaExceeded(String),
    CommentsDisabled(String),
    BadRequest(String),
    ServerError(u16, String),
    ParseError(String),
    Other(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(m) => write!(f, "Network error: {m}"),
            Self::NotFound(m) => write!(f, "Not found: {m}"),
            Self::Unauthorized(m) => write!(f, "Unauthorized: {m}"),
            Self::QuotaExceeded(m) => write!(f, "Quota exceeded: {m}"),
            Self::CommentsDisabled(m) => write!(f, "Comments disabled: {m}"),
            Self::BadRequest(m) => write!(f, "Bad request: {m}"),
            Self::ServerError(code, m) => write!(f, "Server error {code}: {m}"),
            Self::ParseError(m) => write!(f, "Parse error: {m}"),
            Self::Other(m) => write!(f, "API error: {m}"),
        }
    }
}

impl std::error::Error for ApiError {}

// ============================================================================
// 2. YouTube Data API v3 Canonical Serde Schemas (for fixture parsing)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawPageInfo {
    #[serde(rename = "totalResults", default)]
    pub total_results: u64,
    #[serde(rename = "resultsPerPage", default)]
    pub results_per_page: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawThumbnail {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawThumbnails {
    pub default: Option<RawThumbnail>,
    pub medium: Option<RawThumbnail>,
    pub high: Option<RawThumbnail>,
    pub standard: Option<RawThumbnail>,
    pub maxres: Option<RawThumbnail>,
}

// --- Search ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchListResponse {
    pub kind: Option<String>,
    pub etag: Option<String>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    #[serde(rename = "prevPageToken")]
    pub prev_page_token: Option<String>,
    #[serde(rename = "regionCode")]
    pub region_code: Option<String>,
    #[serde(rename = "pageInfo")]
    pub page_info: Option<RawPageInfo>,
    #[serde(default)]
    pub items: Vec<RawSearchResultItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawSearchResultItem {
    pub kind: Option<String>,
    pub etag: Option<String>,
    pub id: RawResourceId,
    pub snippet: Option<RawSearchSnippet>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawResourceId {
    pub kind: Option<String>,
    #[serde(rename = "videoId")]
    pub video_id: Option<String>,
    #[serde(rename = "channelId")]
    pub channel_id: Option<String>,
    #[serde(rename = "playlistId")]
    pub playlist_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawSearchSnippet {
    #[serde(rename = "publishedAt")]
    pub published_at: Option<String>,
    #[serde(rename = "channelId")]
    pub channel_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnails: Option<RawThumbnails>,
    #[serde(rename = "channelTitle")]
    pub channel_title: Option<String>,
    #[serde(rename = "liveBroadcastContent")]
    pub live_broadcast_content: Option<String>,
    #[serde(rename = "publishTime")]
    pub publish_time: Option<String>,
}

// --- Videos ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoListResponse {
    pub kind: Option<String>,
    pub etag: Option<String>,
    #[serde(rename = "pageInfo")]
    pub page_info: Option<RawPageInfo>,
    #[serde(default)]
    pub items: Vec<RawVideoItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawVideoItem {
    pub kind: Option<String>,
    pub etag: Option<String>,
    pub id: String,
    pub snippet: Option<RawVideoSnippet>,
    #[serde(rename = "contentDetails")]
    pub content_details: Option<RawContentDetails>,
    pub status: Option<RawVideoStatus>,
    pub statistics: Option<RawVideoStatistics>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawVideoSnippet {
    #[serde(rename = "publishedAt")]
    pub published_at: Option<String>,
    #[serde(rename = "channelId")]
    pub channel_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnails: Option<RawThumbnails>,
    #[serde(rename = "channelTitle")]
    pub channel_title: Option<String>,
    pub tags: Option<Vec<String>>,
    #[serde(rename = "categoryId")]
    pub category_id: Option<String>,
    #[serde(rename = "liveBroadcastContent")]
    pub live_broadcast_content: Option<String>,
    #[serde(rename = "defaultLanguage")]
    pub default_language: Option<String>,
    #[serde(rename = "defaultAudioLanguage")]
    pub default_audio_language: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawContentDetails {
    pub duration: Option<String>,
    pub dimension: Option<String>,
    pub definition: Option<String>,
    pub caption: Option<String>,
    #[serde(rename = "licensedContent")]
    pub licensed_content: Option<bool>,
    pub projection: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawVideoStatus {
    #[serde(rename = "uploadStatus")]
    pub upload_status: Option<String>,
    #[serde(rename = "privacyStatus")]
    pub privacy_status: Option<String>,
    pub license: Option<String>,
    pub embeddable: Option<bool>,
    #[serde(rename = "publicStatsViewable")]
    pub public_stats_viewable: Option<bool>,
    #[serde(rename = "madeForKids")]
    pub made_for_kids: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawVideoStatistics {
    #[serde(rename = "viewCount")]
    pub view_count: Option<String>,
    #[serde(rename = "likeCount")]
    pub like_count: Option<String>,
    #[serde(rename = "favoriteCount")]
    pub favorite_count: Option<String>,
    #[serde(rename = "commentCount")]
    pub comment_count: Option<String>,
}

// --- Channels ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelListResponse {
    pub kind: Option<String>,
    pub etag: Option<String>,
    #[serde(rename = "pageInfo")]
    pub page_info: Option<RawPageInfo>,
    #[serde(default)]
    pub items: Vec<RawChannelItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawChannelItem {
    pub kind: Option<String>,
    pub etag: Option<String>,
    pub id: String,
    pub snippet: Option<RawChannelSnippet>,
    #[serde(rename = "contentDetails")]
    pub content_details: Option<RawChannelContentDetails>,
    pub statistics: Option<RawChannelStatistics>,
    #[serde(rename = "brandingSettings")]
    pub branding_settings: Option<RawBrandingSettings>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawChannelSnippet {
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "customUrl")]
    pub custom_url: Option<String>,
    #[serde(rename = "publishedAt")]
    pub published_at: Option<String>,
    pub thumbnails: Option<RawThumbnails>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawChannelContentDetails {
    #[serde(rename = "relatedPlaylists")]
    pub related_playlists: Option<RawRelatedPlaylists>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawRelatedPlaylists {
    pub likes: Option<String>,
    pub uploads: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawChannelStatistics {
    #[serde(rename = "viewCount")]
    pub view_count: Option<String>,
    #[serde(rename = "subscriberCount")]
    pub subscriber_count: Option<String>,
    #[serde(rename = "hiddenSubscriberCount")]
    pub hidden_subscriber_count: Option<bool>,
    #[serde(rename = "videoCount")]
    pub video_count: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawBrandingSettings {
    pub channel: Option<RawBrandingChannel>,
    pub image: Option<RawBrandingImage>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawBrandingChannel {
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawBrandingImage {
    #[serde(rename = "bannerExternalUrl")]
    pub banner_external_url: Option<String>,
}

// --- Playlists ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaylistListResponse {
    pub kind: Option<String>,
    pub etag: Option<String>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    #[serde(rename = "pageInfo")]
    pub page_info: Option<RawPageInfo>,
    #[serde(default)]
    pub items: Vec<RawPlaylistItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawPlaylistItem {
    pub kind: Option<String>,
    pub etag: Option<String>,
    pub id: String,
    pub snippet: Option<RawPlaylistSnippet>,
    pub status: Option<RawPlaylistStatus>,
    #[serde(rename = "contentDetails")]
    pub content_details: Option<RawPlaylistContentDetails>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawPlaylistSnippet {
    #[serde(rename = "publishedAt")]
    pub published_at: Option<String>,
    #[serde(rename = "channelId")]
    pub channel_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnails: Option<RawThumbnails>,
    #[serde(rename = "channelTitle")]
    pub channel_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawPlaylistStatus {
    #[serde(rename = "privacyStatus")]
    pub privacy_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawPlaylistContentDetails {
    #[serde(rename = "itemCount")]
    pub item_count: Option<u32>,
}

// --- Playlist Items ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaylistItemListResponse {
    pub kind: Option<String>,
    pub etag: Option<String>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    #[serde(rename = "prevPageToken")]
    pub prev_page_token: Option<String>,
    #[serde(rename = "pageInfo")]
    pub page_info: Option<RawPageInfo>,
    #[serde(default)]
    pub items: Vec<RawPlaylistItemResource>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawPlaylistItemResource {
    pub kind: Option<String>,
    pub etag: Option<String>,
    pub id: String,
    pub snippet: Option<RawPlaylistItemSnippet>,
    #[serde(rename = "contentDetails")]
    pub content_details: Option<RawPlaylistItemContentDetails>,
    pub status: Option<RawPlaylistStatus>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawPlaylistItemSnippet {
    #[serde(rename = "publishedAt")]
    pub published_at: Option<String>,
    #[serde(rename = "channelId")]
    pub channel_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnails: Option<RawThumbnails>,
    #[serde(rename = "channelTitle")]
    pub channel_title: Option<String>,
    #[serde(rename = "playlistId")]
    pub playlist_id: Option<String>,
    pub position: Option<u32>,
    #[serde(rename = "resourceId")]
    pub resource_id: Option<RawResourceId>,
    #[serde(rename = "videoOwnerChannelTitle")]
    pub video_owner_channel_title: Option<String>,
    #[serde(rename = "videoOwnerChannelId")]
    pub video_owner_channel_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawPlaylistItemContentDetails {
    #[serde(rename = "videoId")]
    pub video_id: Option<String>,
    #[serde(rename = "videoPublishedAt")]
    pub video_published_at: Option<String>,
}

// --- Subscriptions ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscriptionListResponse {
    pub kind: Option<String>,
    pub etag: Option<String>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    #[serde(rename = "prevPageToken")]
    pub prev_page_token: Option<String>,
    #[serde(rename = "pageInfo")]
    pub page_info: Option<RawPageInfo>,
    #[serde(default)]
    pub items: Vec<RawSubscriptionItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawSubscriptionItem {
    pub kind: Option<String>,
    pub etag: Option<String>,
    pub id: String,
    pub snippet: Option<RawSubscriptionSnippet>,
    #[serde(rename = "contentDetails")]
    pub content_details: Option<RawSubscriptionContentDetails>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawSubscriptionSnippet {
    #[serde(rename = "publishedAt")]
    pub published_at: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "resourceId")]
    pub resource_id: Option<RawResourceId>,
    #[serde(rename = "channelId")]
    pub channel_id: Option<String>,
    pub thumbnails: Option<RawThumbnails>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawSubscriptionContentDetails {
    #[serde(rename = "totalItemCount")]
    pub total_item_count: Option<u32>,
    #[serde(rename = "newItemCount")]
    pub new_item_count: Option<u32>,
    #[serde(rename = "activityType")]
    pub activity_type: Option<String>,
}

// --- Comments ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommentThreadListResponse {
    pub kind: Option<String>,
    pub etag: Option<String>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    #[serde(rename = "pageInfo")]
    pub page_info: Option<RawPageInfo>,
    #[serde(default)]
    pub items: Vec<RawCommentThreadItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawCommentThreadItem {
    pub kind: Option<String>,
    pub etag: Option<String>,
    pub id: String,
    pub snippet: Option<RawCommentThreadSnippet>,
    pub replies: Option<RawCommentReplies>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawCommentThreadSnippet {
    #[serde(rename = "channelId")]
    pub channel_id: Option<String>,
    #[serde(rename = "videoId")]
    pub video_id: Option<String>,
    #[serde(rename = "topLevelComment")]
    pub top_level_comment: Option<RawCommentItem>,
    #[serde(rename = "canReply")]
    pub can_reply: Option<bool>,
    #[serde(rename = "totalReplyCount")]
    pub total_reply_count: Option<u32>,
    #[serde(rename = "isPublic")]
    pub is_public: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawCommentReplies {
    #[serde(default)]
    pub comments: Vec<RawCommentItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawCommentItem {
    pub kind: Option<String>,
    pub etag: Option<String>,
    pub id: String,
    pub snippet: Option<RawCommentSnippet>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawCommentSnippet {
    #[serde(rename = "channelId")]
    pub channel_id: Option<String>,
    #[serde(rename = "videoId")]
    pub video_id: Option<String>,
    #[serde(rename = "textDisplay")]
    pub text_display: Option<String>,
    #[serde(rename = "textOriginal")]
    pub text_original: Option<String>,
    #[serde(rename = "authorDisplayName")]
    pub author_display_name: Option<String>,
    #[serde(rename = "authorProfileImageUrl")]
    pub author_profile_image_url: Option<String>,
    #[serde(rename = "authorChannelUrl")]
    pub author_channel_url: Option<String>,
    #[serde(rename = "canRate")]
    pub can_rate: Option<bool>,
    #[serde(rename = "viewerRating")]
    pub viewer_rating: Option<String>,
    #[serde(rename = "likeCount")]
    pub like_count: Option<u32>,
    #[serde(rename = "publishedAt")]
    pub published_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
}

// --- Ratings ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoGetRatingResponse {
    pub kind: Option<String>,
    pub etag: Option<String>,
    #[serde(default)]
    pub items: Vec<RawRatingItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawRatingItem {
    #[serde(rename = "videoId")]
    pub video_id: String,
    pub rating: String,
}

// --- Auth & Errors ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub token_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OAuthErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoogleApiErrorResponse {
    pub error: RawGoogleErrorDetails,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawGoogleErrorDetails {
    pub code: u16,
    pub message: String,
    pub status: Option<String>,
    pub errors: Option<Vec<RawGoogleErrorItem>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawGoogleErrorItem {
    pub message: Option<String>,
    pub domain: Option<String>,
    pub reason: Option<String>,
    pub location: Option<String>,
    #[serde(rename = "locationType")]
    pub location_type: Option<String>,
}
