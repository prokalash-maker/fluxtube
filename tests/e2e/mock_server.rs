//! Wiremock Mock Server Context for FluxTube Offline Testing.

use crate::fixtures::load_fixture;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Thread-safe Wiremock Mock Server context bound to `127.0.0.1:0` (ephemeral port).
pub struct MockServerContext {
    pub server: MockServer,
}

impl MockServerContext {
    /// Starts a new isolated Wiremock instance on an ephemeral local loopback port.
    pub async fn start() -> Self {
        let server = MockServer::start().await;
        Self { server }
    }

    /// Returns the base URI string (e.g. `http://127.0.0.1:54321`).
    pub fn uri(&self) -> String {
        self.server.uri()
    }

    /// Mounts `/youtube/v3/search` with the given fixture.
    pub async fn mount_search(&self, fixture_name: &str) {
        let body = load_fixture(fixture_name);
        Mock::given(method("GET"))
            .and(path("/youtube/v3/search"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
    }

    /// Mounts `/youtube/v3/videos` with the given fixture.
    pub async fn mount_videos(&self, fixture_name: &str) {
        let body = load_fixture(fixture_name);
        Mock::given(method("GET"))
            .and(path("/youtube/v3/videos"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
    }

    /// Mounts `/youtube/v3/channels` with the given fixture.
    pub async fn mount_channels(&self, fixture_name: &str) {
        let body = load_fixture(fixture_name);
        Mock::given(method("GET"))
            .and(path("/youtube/v3/channels"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
    }

    /// Mounts `/youtube/v3/playlists` with the given fixture.
    pub async fn mount_playlists(&self, fixture_name: &str) {
        let body = load_fixture(fixture_name);
        Mock::given(method("GET"))
            .and(path("/youtube/v3/playlists"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
    }

    /// Mounts `/youtube/v3/playlistItems` with the given fixture.
    pub async fn mount_playlist_items(&self, fixture_name: &str) {
        let body = load_fixture(fixture_name);
        Mock::given(method("GET"))
            .and(path("/youtube/v3/playlistItems"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
    }

    /// Mounts `/youtube/v3/subscriptions` with the given fixture.
    pub async fn mount_subscriptions(&self, fixture_name: &str) {
        let body = load_fixture(fixture_name);
        Mock::given(method("GET"))
            .and(path("/youtube/v3/subscriptions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
    }

    /// Mounts `/youtube/v3/commentThreads` with the given fixture.
    pub async fn mount_comments(&self, fixture_name: &str) {
        let body = load_fixture(fixture_name);
        Mock::given(method("GET"))
            .and(path("/youtube/v3/commentThreads"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
    }

    /// Mounts `/youtube/v3/videos/getRating` with the given fixture.
    pub async fn mount_ratings(&self, fixture_name: &str) {
        let body = load_fixture(fixture_name);
        Mock::given(method("GET"))
            .and(path("/youtube/v3/videos/getRating"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
    }

    /// Mounts 403 Quota Exceeded error.
    pub async fn mount_quota_exceeded(&self) {
        let body = load_fixture("errors/error_quota_exceeded_403.json");
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(403)
                    .insert_header("content-type", "application/json")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
    }

    /// Mounts Google OAuth token endpoint (`/token`).
    pub async fn mount_oauth_token(&self, fixture_name: &str) {
        let body = load_fixture(fixture_name);
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
    }

    /// Mounts a custom endpoint and HTTP error response.
    pub async fn mount_error(&self, http_method: &str, endpoint_path: &str, status: u16, fixture_name: &str) {
        let body = load_fixture(fixture_name);
        Mock::given(method(http_method))
            .and(path(endpoint_path))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("content-type", "application/json")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wiremock_offline_loopback() {
        let ctx = MockServerContext::start().await;
        ctx.mount_search("search/search_videos_page1.json").await;

        let client = reqwest::Client::new();
        let url = format!("{}/youtube/v3/search", ctx.uri());
        let res = client.get(&url).send().await.expect("Failed to send request");

        assert_eq!(res.status(), 200);
        let text = res.text().await.expect("Failed to get response text");
        assert!(text.contains("youtube#searchListResponse"));
        assert!(text.contains("dQw4w9WgXcQ"));
    }

    #[tokio::test]
    async fn test_wiremock_quota_error() {
        let ctx = MockServerContext::start().await;
        ctx.mount_quota_exceeded().await;

        let client = reqwest::Client::new();
        let url = format!("{}/youtube/v3/videos", ctx.uri());
        let res = client.get(&url).send().await.expect("Failed to send request");

        assert_eq!(res.status(), 403);
        let text = res.text().await.expect("Failed to get response text");
        assert!(text.contains("quotaExceeded"));
    }
}
