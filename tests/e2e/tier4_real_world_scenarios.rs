//! Tier 4: Real-World Workload Scenarios Test Suite (Milestone E2E-M4)

use e2e_runner::prelude::*;

#[test]
fn test_tier4_harness_integration() {
    let registry = TestFixtureRegistry::new();
    assert!(registry.count() >= 40);
}

#[tokio::test]
async fn test_tier4_scenario_search_watch_bookmark_and_resume() {
    let client = MockYouTubeClient::new();
    let db = TestStorageDb::in_memory();

    // 1. User searches for Rust content
    let search = client
        .search(&SearchParams {
            query: "Rust".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    let first_id = search.items[0].id.clone();

    // 2. Fetch full video details & comment threads
    let details = client.get_video_details(&[first_id.as_str()]).await.unwrap();
    let video = &details[0];
    assert_video_valid(video);

    let comments = client.get_comment_threads(&video.id, None).await.unwrap();
    assert!(!comments.items.is_empty());

    // 3. Record watch progress and a timestamp bookmark
    db.conn
        .execute(
            "INSERT INTO local_history (account_id, video_id, title, channel_id, channel_title, last_position_seconds, duration_seconds, completed)
             VALUES (NULL, ?1, ?2, ?3, ?4, 145, ?5, 0)",
            rusqlite::params![
                video.id,
                video.title,
                video.channel_id,
                video.channel_title,
                video.duration.as_secs() as i64
            ],
        )
        .unwrap();

    db.conn
        .execute(
            "INSERT INTO video_bookmarks (video_id, position_seconds, note) VALUES (?1, 145, ?2)",
            rusqlite::params![video.id, "Great benchmark breakdown"],
        )
        .unwrap();

    // 4. Rate video and verify persisted rating
    client.rate_video(&video.id, VideoRating::Like).await.unwrap();
    assert_eq!(client.get_recorded_rating(&video.id), Some(VideoRating::Like));

    let bm_note: String = db
        .conn
        .query_row(
            "SELECT note FROM video_bookmarks WHERE video_id = ?1",
            rusqlite::params![video.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(bm_note, "Great benchmark breakdown");
}

#[tokio::test]
async fn test_tier4_scenario_channel_browse_and_local_playlist_curation() {
    let client = MockYouTubeClient::new();
    let db = TestStorageDb::in_memory();

    // 1. Open channel & derive uploads playlist
    let ch = client.get_channel_details("UCuAXFkgsw1L7xaCfnd5JJOw").await.unwrap();
    assert_channel_valid(&ch);

    let uploads = client
        .get_playlist_items(&ch.uploads_playlist_id, None)
        .await
        .unwrap();
    assert!(!uploads.items.is_empty());

    // 2. Create local playlist and add channel uploads
    db.conn
        .execute(
            "INSERT INTO local_playlists (id, title, description) VALUES ('pl_study', 'Weekend Study', 'Saved offline')",
            [],
        )
        .unwrap();

    for (idx, item) in uploads.items.iter().enumerate() {
        db.conn
            .execute(
                "INSERT INTO local_playlist_items (playlist_id, video_id, title, position) VALUES ('pl_study', ?1, ?2, ?3)",
                rusqlite::params![item.video_id, item.title, idx as i32],
            )
            .unwrap();
    }

    let saved_count: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM local_playlist_items WHERE playlist_id = 'pl_study'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(saved_count, uploads.items.len() as i64);
}
