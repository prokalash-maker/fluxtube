//! Fixture Loader & TestFixtureRegistry for FluxTube E2E Test Suite.

use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Returns the absolute path to the canonical `tests/fixtures/youtube_v3/` directory.
/// Resolves dynamically via `env!("CARGO_MANIFEST_DIR")` for cross-platform portability.
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("youtube_v3")
}

/// Resolves a fixture path, searching directly or within known subdirectories.
pub fn resolve_fixture_path(name: &str) -> PathBuf {
    let base = fixtures_dir();
    let direct = base.join(name);
    if direct.exists() {
        return direct;
    }

    // Try searching common categories if subfolder was omitted
    let categories = [
        "search",
        "videos",
        "channels",
        "playlists",
        "subscriptions",
        "comments",
        "ratings",
        "auth",
        "errors",
    ];

    for cat in &categories {
        let candidate = base.join(cat).join(name);
        if candidate.exists() {
            return candidate;
        }
    }

    // Direct path fallback for descriptive panic
    direct
}

/// Loads raw fixture contents as a String.
pub fn load_fixture(name: &str) -> String {
    let path = resolve_fixture_path(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture file at {:?}: {}", path, e))
}

/// Loads and deserializes a fixture JSON file into target serde type.
pub fn load_fixture_json<T: DeserializeOwned>(name: &str) -> T {
    let content = load_fixture(name);
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to deserialize fixture JSON '{name}': {e}"))
}

/// In-memory registry containing all canonical YouTube v3 JSON fixtures.
#[derive(Debug, Clone)]
pub struct TestFixtureRegistry {
    fixtures: HashMap<String, String>,
}

impl Default for TestFixtureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TestFixtureRegistry {
    /// Creates and loads all JSON fixtures from `tests/fixtures/youtube_v3/`.
    pub fn new() -> Self {
        let mut fixtures = HashMap::new();
        let base = fixtures_dir();

        if base.exists() {
            Self::scan_dir(&base, &base, &mut fixtures);
        }

        Self { fixtures }
    }

    fn scan_dir(dir: &Path, base: &Path, map: &mut HashMap<String, String>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::scan_dir(&path, base, map);
                } else if path.extension().is_some_and(|ext| ext == "json") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        // Key 1: Relative path with forward slashes (e.g. "search/search_videos_page1.json")
                        if let Ok(rel) = path.strip_prefix(base) {
                            let rel_key = rel.to_string_lossy().replace('\\', "/");
                            map.insert(rel_key, content.clone());
                        }
                        // Key 2: Bare filename (e.g. "search_videos_page1.json")
                        if let Some(filename) = path.file_name() {
                            let file_key = filename.to_string_lossy().to_string();
                            map.insert(file_key, content);
                        }
                    }
                }
            }
        }
    }

    /// Checks if a fixture exists by relative path or filename.
    pub fn has_fixture(&self, name: &str) -> bool {
        let clean = name.replace('\\', "/");
        self.fixtures.contains_key(&clean)
    }

    /// Retrieves raw JSON content of a fixture.
    pub fn get_raw(&self, name: &str) -> Option<&str> {
        let clean = name.replace('\\', "/");
        self.fixtures.get(&clean).map(|s| s.as_str())
    }

    /// Deserializes a cached fixture into target serde type.
    pub fn get_json<T: DeserializeOwned>(&self, name: &str) -> Result<T, serde_json::Error> {
        match self.get_raw(name) {
            Some(raw) => serde_json::from_str(raw),
            None => panic!("Fixture '{name}' not found in registry"),
        }
    }

    /// Returns all registered fixture keys.
    pub fn all_fixture_names(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.fixtures.keys().cloned().collect();
        keys.sort();
        keys
    }

    /// Returns count of registered fixtures.
    pub fn count(&self) -> usize {
        self.fixtures.len()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::models::*;

    #[test]
    fn test_fixtures_dir_resolution() {
        let dir = fixtures_dir();
        assert!(dir.exists(), "Fixtures directory does not exist: {:?}", dir);
    }

    #[test]
    fn test_all_fixtures_parseable() {
        let registry = TestFixtureRegistry::new();
        assert!(
            registry.count() >= 40,
            "Expected at least 40 fixture keys, found {}",
            registry.count()
        );

        // 1. Search category
        let _: SearchListResponse = registry.get_json("search/search_videos_page1.json").unwrap();
        let _: SearchListResponse = registry.get_json("search/search_videos_page2.json").unwrap();
        let _: SearchListResponse = registry.get_json("search/search_channels.json").unwrap();
        let _: SearchListResponse = registry.get_json("search/search_playlists.json").unwrap();
        let empty_search: SearchListResponse = registry.get_json("search/search_empty.json").unwrap();
        assert_eq!(empty_search.items.len(), 0);
        let _: SearchListResponse = registry.get_json("search/search_filtered_duration.json").unwrap();

        // 2. Videos category
        let batch: VideoListResponse = registry.get_json("videos/videos_batch_details.json").unwrap();
        assert_eq!(batch.items.len(), 5);
        let single: VideoListResponse = registry.get_json("videos/video_single_details.json").unwrap();
        assert_eq!(single.items.len(), 1);
        let edge: VideoListResponse = registry.get_json("videos/videos_edge_cases.json").unwrap();
        assert_eq!(edge.items.len(), 8);
        let popular: VideoListResponse = registry.get_json("videos/videos_most_popular.json").unwrap();
        assert_eq!(popular.items.len(), 2);
        let empty_vids: VideoListResponse = registry.get_json("videos/videos_empty.json").unwrap();
        assert_eq!(empty_vids.items.len(), 0);

        // 3. Channels category
        let ch_primary: ChannelListResponse = registry.get_json("channels/channel_primary.json").unwrap();
        assert_eq!(ch_primary.items.len(), 1);
        let ch_cyrillic: ChannelListResponse = registry.get_json("channels/channel_cyrillic.json").unwrap();
        assert_eq!(ch_cyrillic.items.len(), 1);
        let ch_hidden: ChannelListResponse = registry.get_json("channels/channel_hidden_subscribers.json").unwrap();
        assert_eq!(ch_hidden.items[0].statistics.as_ref().unwrap().hidden_subscriber_count, Some(true));

        // 4. Playlists category
        let pl_user: PlaylistListResponse = registry.get_json("playlists/playlists_user.json").unwrap();
        assert_eq!(pl_user.items.len(), 3);
        let pl_create: RawPlaylistItem = registry.get_json("playlists/playlist_create_response.json").unwrap();
        assert_eq!(pl_create.id, "PL_NEWLY_CREATED_999");
        let pl_items1: PlaylistItemListResponse = registry.get_json("playlists/playlist_items_uploads_page1.json").unwrap();
        assert_eq!(pl_items1.items.len(), 3);
        let pl_items2: PlaylistItemListResponse = registry.get_json("playlists/playlist_items_uploads_page2.json").unwrap();
        assert_eq!(pl_items2.items.len(), 1);
        let pl_empty: PlaylistItemListResponse = registry.get_json("playlists/playlist_empty.json").unwrap();
        assert_eq!(pl_empty.items.len(), 0);

        // 5. Subscriptions category
        let sub_p1: SubscriptionListResponse = registry.get_json("subscriptions/subscriptions_page1.json").unwrap();
        assert_eq!(sub_p1.items.len(), 2);
        let sub_p2: SubscriptionListResponse = registry.get_json("subscriptions/subscriptions_page2.json").unwrap();
        assert_eq!(sub_p2.items.len(), 1);
        let sub_ins: RawSubscriptionItem = registry.get_json("subscriptions/subscription_insert_response.json").unwrap();
        assert_eq!(sub_ins.id, "sUbScRiPtIoN_NEW_SUCCESS_99");
        let sub_empty: SubscriptionListResponse = registry.get_json("subscriptions/subscriptions_empty.json").unwrap();
        assert_eq!(sub_empty.items.len(), 0);

        // 6. Comments category
        let comments: CommentThreadListResponse = registry.get_json("comments/comment_threads.json").unwrap();
        assert_eq!(comments.items.len(), 2);
        let c_ins: RawCommentThreadItem = registry.get_json("comments/comment_thread_insert_response.json").unwrap();
        assert_eq!(c_ins.id, "UgzD_NEW_POSTED_THREAD_123");
        let c_empty: CommentThreadListResponse = registry.get_json("comments/comment_threads_empty.json").unwrap();
        assert_eq!(c_empty.items.len(), 0);

        // 7. Ratings category
        let r_like: VideoGetRatingResponse = registry.get_json("ratings/rating_like.json").unwrap();
        assert_eq!(r_like.items[0].rating, "like");
        let r_none: VideoGetRatingResponse = registry.get_json("ratings/rating_none.json").unwrap();
        assert_eq!(r_none.items[0].rating, "none");
        let r_dislike: VideoGetRatingResponse = registry.get_json("ratings/rating_dislike.json").unwrap();
        assert_eq!(r_dislike.items[0].rating, "dislike");

        // 8. Auth category
        let token_ex: OAuthTokenResponse = registry.get_json("auth/token_exchange_success.json").unwrap();
        assert!(token_ex.access_token.starts_with("ya29."));
        let token_ref: OAuthTokenResponse = registry.get_json("auth/token_refresh_success.json").unwrap();
        assert!(token_ref.access_token.starts_with("ya29."));
        let err_grant: OAuthErrorResponse = registry.get_json("auth/error_invalid_grant.json").unwrap();
        assert_eq!(err_grant.error, "invalid_grant");
        let err_revoked: OAuthErrorResponse = registry.get_json("auth/error_revoked_token.json").unwrap();
        assert_eq!(err_revoked.error, "invalid_grant");
        let err_unauth: OAuthErrorResponse = registry.get_json("auth/error_unauthorized.json").unwrap();
        assert_eq!(err_unauth.error, "unauthorized_client");

        // 9. Errors category
        let err_403_quota: GoogleApiErrorResponse = registry.get_json("errors/error_quota_exceeded_403.json").unwrap();
        assert_eq!(err_403_quota.error.code, 403);
        let err_403_com: GoogleApiErrorResponse = registry.get_json("errors/error_comments_disabled_403.json").unwrap();
        assert_eq!(err_403_com.error.code, 403);
        let err_401: GoogleApiErrorResponse = registry.get_json("errors/error_unauthorized_401.json").unwrap();
        assert_eq!(err_401.error.code, 401);
        let err_404: GoogleApiErrorResponse = registry.get_json("errors/error_not_found_404.json").unwrap();
        assert_eq!(err_404.error.code, 404);
        let err_500: GoogleApiErrorResponse = registry.get_json("errors/error_backend_500.json").unwrap();
        assert_eq!(err_500.error.code, 500);
        let err_503: GoogleApiErrorResponse = registry.get_json("errors/error_service_unavailable_503.json").unwrap();
        assert_eq!(err_503.error.code, 503);
    }
}
