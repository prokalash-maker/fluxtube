//! Tier 2: Boundary Value & Corner Cases Test Suite (Milestone E2E-M3)

use e2e_runner::prelude::*;
use std::time::Duration;

#[test]
fn test_tier2_harness_integration() {
    let registry = TestFixtureRegistry::new();
    assert!(registry.has_fixture("videos_edge_cases.json"));
    assert!(registry.has_fixture("search_empty.json"));
}

#[tokio::test]
async fn test_tier2_empty_and_whitespace_search_queries() {
    let client = MockYouTubeClient::new();
    for q in ["", "   ", "\t\n"] {
        let search = client
            .search(&SearchParams {
                query: q.into(),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(search.items.len(), 0);
    }
}

#[tokio::test]
async fn test_tier2_empty_video_ids_batch() {
    let client = MockYouTubeClient::new();
    let res = client.get_video_details(&[]).await.unwrap();
    assert!(res.is_empty());
}

#[tokio::test]
async fn test_tier2_offline_and_quota_exceeded_boundaries() {
    let client = MockYouTubeClient::new();
    client.set_offline(true);
    let err = client
        .search(&SearchParams {
            query: "rust".into(),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApiError::Network(_)));

    client.set_offline(false);
    client.set_quota_exceeded(true);
    let err2 = client.get_video_details(&["dQw4w9WgXcQ"]).await.unwrap_err();
    assert!(matches!(err2, ApiError::QuotaExceeded(_)));
}

#[test]
fn test_tier2_iso8601_duration_edge_cases() {
    assert_eq!(parse_iso8601_duration("PT0S"), Duration::ZERO);
    assert_eq!(parse_iso8601_duration("PT59S"), Duration::from_secs(59));
    assert_eq!(parse_iso8601_duration("PT10H0M1S"), Duration::from_secs(36001));
    assert_eq!(parse_iso8601_duration(""), Duration::ZERO);
}

#[test]
fn test_tier2_storage_unicode_and_boundary_positions() {
    let db = TestStorageDb::in_memory();
    db.conn
        .execute(
            "INSERT INTO accounts (id, email, display_name, is_active) VALUES (?1, ?2, ?3, 1)",
            rusqlite::params!["acc_uni", "тест@example.com", "Користувач 🚀"],
        )
        .unwrap();

    db.conn
        .execute(
            "INSERT INTO local_history (account_id, video_id, title, channel_id, channel_title, last_position_seconds, duration_seconds, completed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params!["acc_uni", "vid_0", "Відео без тривалості", "ch1", "Канал", 0, 0, 0],
        )
        .unwrap();

    let count: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM local_history WHERE account_id = 'acc_uni'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_tier2_vault_overwrite_and_missing_key_boundary() {
    let vault = MockCredentialVault::new();
    assert_eq!(vault.get_token("missing").unwrap(), None);
    vault.delete_token("missing").unwrap();
    vault.store_token("u1", "tok_v1").unwrap();
    vault.store_token("u1", "tok_v2").unwrap();
    assert_eq!(vault.get_token("u1").unwrap(), Some("tok_v2".to_string()));
}
