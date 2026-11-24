//! FluxTube E2E Test Harness & Shared Runner Library (`e2e_runner`)
//!
//! Provides the canonical mock infrastructure, offline fixtures, Wiremock loopback context,
//! in-memory YouTube API client, SQLite test factories, and custom test assertions
//! for all FluxTube E2E test suites (Tiers 1 through 4).

pub mod assertions;
pub mod factories;
pub mod fixtures;
pub mod mock_client;
pub mod mock_server;
pub mod mock_storage;
pub mod mock_vault;
pub mod models;

pub mod prelude {
    pub use crate::assertions::*;
    pub use crate::factories::*;
    pub use crate::fixtures::{fixtures_dir, load_fixture, load_fixture_json, TestFixtureRegistry};
    pub use crate::mock_client::{
        parse_iso8601_duration, MockYouTubeClient, YouTubeApiClient,
    };
    pub use crate::mock_server::MockServerContext;
    pub use crate::mock_storage::{
        apply_pragmas, create_test_db, create_test_db_on_disk, TestStorageDb,
        FLUXTUBE_SCHEMA_SQL,
    };
    pub use crate::mock_vault::MockCredentialVault;
    pub use crate::models::*;
    pub use crate::{assert_contains, assert_duration_eq};
}

// Re-export primary loader functions directly at crate root
pub use fixtures::{fixtures_dir, load_fixture, load_fixture_json, TestFixtureRegistry};
pub use mock_client::{MockYouTubeClient, YouTubeApiClient};
pub use mock_server::MockServerContext;
pub use mock_storage::{create_test_db, create_test_db_on_disk};
pub use mock_vault::MockCredentialVault;

#[cfg(test)]
mod tests {
    use super::prelude::*;

    #[test]
    fn test_e2e_runner_harness_integration() {
        // 1. Fixture Registry verification
        let registry = TestFixtureRegistry::new();
        assert!(registry.count() >= 40);

        // 2. Vault verification
        let vault = MockCredentialVault::new();
        vault.store_token("account_test", "mock_refresh_token_123").unwrap();
        assert_eq!(
            vault.get_token("account_test").unwrap(),
            Some("mock_refresh_token_123".to_string())
        );

        // 3. SQLite In-Memory DB verification
        let db = TestStorageDb::in_memory();
        assert_eq!(db.table_count(), 13);

        // 4. Factory verification
        let video = sample_video("sample123");
        assert_video_valid(&video);

        let channel = sample_channel("UCuAXFkgsw1L7xaCfnd5JJOw");
        assert_channel_valid(&channel);
    }

    #[tokio::test]
    async fn test_offline_wiremock_integration() {
        let ctx = MockServerContext::start().await;
        assert!(ctx.uri().starts_with("http://127.0.0.1:"));
        ctx.mount_search("search_videos_page1.json").await;

        let client = reqwest::Client::new();
        let res = client
            .get(format!("{}/youtube/v3/search", ctx.uri()))
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), 200);
    }

    #[tokio::test]
    async fn test_in_memory_mock_client_integration() {
        let client = MockYouTubeClient::new();
        let res = client
            .search(&SearchParams {
                query: "Rick Astley".into(),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(!res.items.is_empty());
        assert_eq!(res.items[0].id, "dQw4w9WgXcQ");
    }
}
