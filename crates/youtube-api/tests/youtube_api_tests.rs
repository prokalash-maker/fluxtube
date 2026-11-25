use fluxtube_core::{QuotaPool, VideoRating};
use fluxtube_network::{QuotaTracker, ResilientHttpClient};
use fluxtube_youtube_api::{
    derive_uploads_playlist_id, parse_iso8601_duration, LiveYouTubeApiClient, MockYouTubeApiClient,
    SearchOrder, SearchParams, SearchResourceType, VideoDurationFilter, YouTubeApiError,
    YouTubeDataApi,
};
use std::sync::Arc;
use std::time::Duration;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_iso8601_duration_and_uploads_derivation() {
    assert_eq!(parse_iso8601_duration("PT4M13S"), Duration::from_secs(253));
    assert_eq!(
        parse_iso8601_duration("PT1H12M45S"),
        Duration::from_secs(4365)
    );
    assert_eq!(parse_iso8601_duration("PT0S"), Duration::ZERO);
    assert_eq!(parse_iso8601_duration("P1DT2H"), Duration::from_secs(93600));
    assert_eq!(parse_iso8601_duration("invalid"), Duration::ZERO);

    assert_eq!(derive_uploads_playlist_id("UCabc123"), "UUabc123");
    assert_eq!(derive_uploads_playlist_id("PLabc123"), "PLabc123");
}

#[tokio::test]
async fn test_mock_youtube_api_client_full_flow() {
    let client = MockYouTubeApiClient::new();

    // Search videos
    let search_res = client
        .search(&SearchParams {
            query: "Rust".to_string(),
            order: Some(SearchOrder::ViewCount),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(!search_res.items.is_empty());

    // Get video details
    let details = client
        .get_video_details(&["dQw4w9WgXcQ", "aqz-KE-bpKQ"])
        .await
        .unwrap();
    assert_eq!(details.len(), 2);
    assert!(details.iter().any(|v| v.is_short()));

    // Popular videos
    let popular = client
        .get_popular_videos("US", None, None, 3)
        .await
        .unwrap();
    assert_eq!(popular.items.len(), 3);
    assert!(popular.items[0].view_count >= popular.items[1].view_count);

    // Channel lookup by ID and handle
    let ch = client.get_channel_details("@rustsystemslab").await.unwrap();
    assert_eq!(ch.id, "UCrustlang00001");
    assert_eq!(ch.uploads_playlist_id, "UUrustlang00001");

    // Channel uploads playlist
    let uploads = client
        .get_playlist_items(&ch.uploads_playlist_id, None)
        .await
        .unwrap();
    assert!(!uploads.items.is_empty());

    // Comments & posting
    let cmts = client
        .get_comment_threads("dQw4w9WgXcQ", "relevance", None)
        .await
        .unwrap();
    assert_eq!(cmts.items.len(), 2);
    let posted = client
        .post_comment("token", "dQw4w9WgXcQ", "Great video!")
        .await
        .unwrap();
    assert_eq!(posted.text, "Great video!");

    // Rating
    client
        .rate_video("token", "dQw4w9WgXcQ", VideoRating::Like)
        .await
        .unwrap();
    let ratings = client
        .get_video_rating("token", &["dQw4w9WgXcQ", "missing"])
        .await
        .unwrap();
    assert_eq!(ratings[0], ("dQw4w9WgXcQ".to_string(), VideoRating::Like));
    assert_eq!(ratings[1], ("missing".to_string(), VideoRating::None));

    // Subscriptions & playlists
    let sub_id = client.subscribe("token", "UCambientlofi03").await.unwrap();
    let subs = client.get_user_subscriptions("token", None).await.unwrap();
    assert_eq!(subs.items.len(), 3);
    client.unsubscribe("token", &sub_id).await.unwrap();
    let subs_after = client.get_user_subscriptions("token", None).await.unwrap();
    assert_eq!(subs_after.items.len(), 2);

    let created_pl = client
        .create_playlist("token", "My Test List", "Desc", "private")
        .await
        .unwrap();
    let added = client
        .add_playlist_item("token", &created_pl.id, "dQw4w9WgXcQ")
        .await
        .unwrap();
    client.remove_playlist_item("token", &added.id).await.unwrap();
}

#[tokio::test]
async fn test_live_youtube_api_wiremock_search_videos_and_quota() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("q", "iced rust"))
        .and(query_param("type", "video"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "nextPageToken": "PAGE_2",
            "pageInfo": { "totalResults": 100 },
            "items": [{
                "id": { "kind": "youtube#video", "videoId": "vid_xyz" },
                "snippet": {
                    "title": "Iced GUI Tutorial",
                    "description": "Fast Rust UI",
                    "channelId": "UCtest",
                    "channelTitle": "TestChannel",
                    "publishedAt": "2025-02-01T12:00:00Z",
                    "thumbnails": { "medium": { "url": "https://img.example/1.jpg", "width": 320, "height": 180 } }
                }
            }]
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/videos"))
        .and(query_param("id", "vid_xyz"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [{
                "id": "vid_xyz",
                "snippet": {
                    "title": "Iced GUI Tutorial",
                    "description": "Fast Rust UI",
                    "channelId": "UCtest",
                    "channelTitle": "TestChannel",
                    "publishedAt": "2025-02-01T12:00:00Z",
                    "liveBroadcastContent": "none"
                },
                "contentDetails": { "duration": "PT12M34S" },
                "statistics": { "viewCount": "45000", "likeCount": "1200" },
                "status": { "embeddable": true }
            }]
        })))
        .mount(&server)
        .await;

    let http = ResilientHttpClient::builder().build().unwrap();
    let quota = Arc::new(QuotaTracker::new(10_000, 3_000));
    let api = LiveYouTubeApiClient::with_base_url(http, quota.clone(), "test-key", server.uri());

    let res = api
        .search(&SearchParams {
            query: "iced rust".to_string(),
            resource_type: Some(SearchResourceType::Video),
            video_duration: Some(VideoDurationFilter::Medium),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(res.items.len(), 1);
    assert_eq!(res.items[0].id, "vid_xyz");
    assert_eq!(res.next_page_token.as_deref(), Some("PAGE_2"));
    assert_eq!(quota.get_status(QuotaPool::Search).used_today, 1);

    let vids = api.get_video_details(&["vid_xyz"]).await.unwrap();
    assert_eq!(vids.len(), 1);
    assert_eq!(vids[0].duration, Duration::from_secs(754));
    assert_eq!(vids[0].view_count, 45_000);
    assert_eq!(quota.get_status(QuotaPool::General).used_today, 101);
}

#[tokio::test]
async fn test_live_youtube_api_error_mapping() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/commentThreads"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "error": {
                "code": 403,
                "message": "The video identified by the videoId parameter has disabled comments.",
                "errors": [{ "reason": "commentsDisabled" }]
            }
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/videos"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "error": {
                "code": 403,
                "message": "Quota exceeded",
                "errors": [{ "reason": "quotaExceeded" }]
            }
        })))
        .mount(&server)
        .await;

    let http = ResilientHttpClient::builder().build().unwrap();
    let quota = Arc::new(QuotaTracker::new(10_000, 3_000));
    let api = LiveYouTubeApiClient::with_base_url(http, quota, "test-key", server.uri());

    let err_comments = api
        .get_comment_threads("v1", "relevance", None)
        .await
        .unwrap_err();
    assert!(matches!(err_comments, YouTubeApiError::CommentsDisabled(_)));

    let err_quota = api.get_video_details(&["v1"]).await.unwrap_err();
    assert!(matches!(err_quota, YouTubeApiError::QuotaExceeded(_)));
}

#[tokio::test]
async fn test_live_youtube_api_my_channel_and_401() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/channels"))
        .and(query_param("mine", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [{
                "id": "UC_my_channel_001",
                "snippet": {
                    "title": "My Personal Channel",
                    "description": "Tech & Rust",
                    "customUrl": "@mypersonal"
                },
                "statistics": {
                    "subscriberCount": "1200",
                    "videoCount": "18"
                },
                "contentDetails": {
                    "relatedPlaylists": {
                        "uploads": "UU_my_channel_001"
                    }
                }
            }]
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/channels"))
        .and(query_param("mine", "true"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": {
                "code": 401,
                "message": "Invalid Credentials"
            }
        })))
        .mount(&server)
        .await;

    let http = ResilientHttpClient::builder().build().unwrap();
    let quota = Arc::new(QuotaTracker::new(10_000, 3_000));
    let api = LiveYouTubeApiClient::with_base_url(http, quota, "", server.uri());

    let ch = api.get_my_channel("ya29.valid_tok").await.unwrap();
    assert_eq!(ch.id, "UC_my_channel_001");
    assert_eq!(ch.title, "My Personal Channel");
    assert_eq!(ch.custom_url.as_deref(), Some("@mypersonal"));

    let err_401 = api.get_my_channel("ya29.expired_tok").await.unwrap_err();
    assert!(matches!(err_401, YouTubeApiError::Unauthorized(_)));
}
