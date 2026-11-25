//! Tier 3: Pairwise Combinatorial Test Suite (Milestone E2E-M4)

use e2e_runner::prelude::*;

#[test]
fn test_tier3_harness_integration() {
    let vault = MockCredentialVault::new();
    let db = TestStorageDb::in_memory();
    assert!(vault.is_empty());
    assert_eq!(db.table_count(), 13);
}

#[tokio::test]
async fn test_tier3_search_type_and_order_combinations() {
    let client = MockYouTubeClient::new();

    for resource_type in [None, Some("channel"), Some("playlist")] {
        for order in [None, Some("date"), Some("viewCount")] {
            let res = client
                .search(&SearchParams {
                    query: "Rust".into(),
                    resource_type: resource_type.map(str::to_string),
                    order: order.map(str::to_string),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert!(!res.items.is_empty());
        }
    }
}

#[tokio::test]
async fn test_tier3_account_isolation_vault_plus_sqlite() {
    let vault = MockCredentialVault::new();
    let db = TestStorageDb::in_memory();

    for (acc, tok) in [("acc_a", "rt_alpha"), ("acc_b", "rt_beta")] {
        vault.store_token(acc, tok).unwrap();
        db.conn
            .execute(
                "INSERT INTO accounts (id, email, display_name, is_active) VALUES (?1, ?2, ?3, 0)",
                rusqlite::params![acc, format!("{acc}@example.com"), acc],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO local_history (account_id, video_id, title, channel_id, channel_title, last_position_seconds, duration_seconds, completed)
                 VALUES (?1, ?2, ?3, 'c1', 'Chan', 15, 120, 0)",
                rusqlite::params![acc, format!("vid_{acc}"), format!("Video {acc}")],
            )
            .unwrap();
    }

    // Delete account A -> cascades history for A while leaving B intact
    vault.delete_token("acc_a").unwrap();
    db.conn
        .execute("DELETE FROM accounts WHERE id = 'acc_a'", [])
        .unwrap();

    assert_eq!(vault.get_token("acc_a").unwrap(), None);
    assert_eq!(vault.get_token("acc_b").unwrap(), Some("rt_beta".to_string()));

    let remaining_b: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM local_history WHERE account_id = 'acc_b'", [], |r| r.get(0))
        .unwrap();
    let remaining_a: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM local_history WHERE account_id = 'acc_a'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(remaining_a, 0);
    assert_eq!(remaining_b, 1);
}

#[tokio::test]
async fn test_tier3_wiremock_and_mock_client_error_recovery() {
    let ctx = MockServerContext::start().await;
    ctx.mount_search("search_videos_page1.json").await;

    let http = reqwest::Client::new();
    let resp = http
        .get(format!("{}/youtube/v3/search", ctx.uri()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let client = MockYouTubeClient::new();
    client.set_offline(true);
    assert!(client.get_user_subscriptions(None).await.is_err());
    client.set_offline(false);
    assert!(client.get_user_subscriptions(None).await.is_ok());
}
