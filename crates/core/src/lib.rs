pub mod channel;
pub mod comment;
pub mod error;
pub mod history;
pub mod playlist;
pub mod quota;
pub mod settings;
pub mod user;
pub mod video;

pub use channel::{Channel, ChannelGroup, ChannelGroupItem};
pub use comment::Comment;
pub use error::CoreError;
pub use history::{Bookmark, HistoryEntry, VideoPosition};
pub use playlist::{Playlist, PlaylistItem};
pub use quota::{QuotaPool, QuotaStatus};
pub use settings::{PerformanceProfileType, PerformanceSettings};
pub use user::UserProfile;
pub use video::{PlaybackState, Thumbnail, ThumbnailQuality, Video, VideoRating};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::time::Duration;

    #[test]
    fn test_video_duration_formatting() {
        let v1 = Video {
            id: "v1".to_string(),
            title: "Short Video".to_string(),
            description: "".to_string(),
            channel_id: "c1".to_string(),
            channel_title: "Channel".to_string(),
            thumbnail_url: "".to_string(),
            thumbnails: vec![],
            duration: Duration::from_secs(65),
            view_count: 500,
            like_count: Some(50),
            published_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            embeddable: true,
            live_broadcast_content: None,
        };
        assert_eq!(v1.formatted_duration(), "1:05");
        assert!(!v1.is_short()); // 65 > 60

        let v2 = Video {
            duration: Duration::from_secs(3665),
            ..v1.clone()
        };
        assert_eq!(v2.formatted_duration(), "1:01:05");
        assert_eq!(v2.watch_url(), "https://www.youtube.com/watch?v=v1");
        assert_eq!(
            v2.embed_url(),
            "https://www.youtube-nocookie.com/embed/v1?enablejsapi=1&origin=https://localhost"
        );
    }

    #[test]
    fn test_video_view_count_formatting() {
        let make_video = |views: u64| Video {
            id: "id".to_string(),
            title: "t".to_string(),
            description: "".to_string(),
            channel_id: "c".to_string(),
            channel_title: "ct".to_string(),
            thumbnail_url: "".to_string(),
            thumbnails: vec![],
            duration: Duration::from_secs(10),
            view_count: views,
            like_count: None,
            published_at: Utc::now(),
            embeddable: true,
            live_broadcast_content: None,
        };

        assert_eq!(make_video(500).formatted_view_count(), "500");
        assert_eq!(make_video(1500).formatted_view_count(), "1.5K");
        assert_eq!(make_video(2_400_000).formatted_view_count(), "2.4M");
        assert_eq!(make_video(1_800_000_000).formatted_view_count(), "1.8B");
    }

    #[test]
    fn test_channel_formatting() {
        let ch = Channel {
            id: "UC123".to_string(),
            title: "Rustaceans".to_string(),
            description: "Rust videos".to_string(),
            custom_url: Some("@rustaceans".to_string()),
            avatar_url: "avatar.jpg".to_string(),
            banner_url: None,
            subscriber_count: 1_250_000,
            video_count: 140,
            view_count: Some(50_000_000),
            uploads_playlist_id: "UU123".to_string(),
        };
        assert_eq!(ch.formatted_subscriber_count(), "1.25M");
        assert_eq!(ch.channel_url(), "https://www.youtube.com/channel/UC123");
    }

    #[test]
    fn test_history_and_bookmark() {
        let history = HistoryEntry {
            id: Some(1),
            video_id: "vid_1".to_string(),
            title: "Intro".to_string(),
            channel_title: "Chan".to_string(),
            thumbnail_url: "thumb.jpg".to_string(),
            duration_seconds: 100,
            watched_at: Utc::now(),
            playback_position_seconds: 50,
            completed: false,
            account_id: None,
        };
        assert_eq!(history.progress_ratio(), 0.5);
        assert!(history.is_resumable());

        let finished = HistoryEntry {
            playback_position_seconds: 96,
            completed: false,
            ..history.clone()
        };
        assert!(!finished.is_resumable()); // ratio 0.96 >= 0.95

        let bookmark = Bookmark {
            id: 1,
            video_id: "vid_1".to_string(),
            title: "Awesome moment".to_string(),
            timestamp_seconds: 3725,
            note: Some("Check this code".to_string()),
            created_at: Utc::now(),
        };
        assert_eq!(bookmark.formatted_timestamp(), "1:02:05");
    }

    #[test]
    fn test_video_position_progress() {
        let pos = VideoPosition {
            video_id: "v".to_string(),
            position_seconds: 45,
            duration_seconds: 50,
            updated_at: Utc::now(),
        };
        assert_eq!(pos.progress_percent(), 90.0);
        assert!(pos.is_finished());

        let pos2 = VideoPosition {
            video_id: "v2".to_string(),
            position_seconds: 10,
            duration_seconds: 100,
            updated_at: Utc::now(),
        };
        assert_eq!(pos2.progress_percent(), 10.0);
        assert!(!pos2.is_finished());
    }

    #[test]
    fn test_quota_status() {
        let status = QuotaStatus {
            pool: QuotaPool::General,
            used_today: 8000,
            daily_limit: 10000,
            remaining: 2000,
            reset_at: Utc::now(),
        };
        assert_eq!(status.usage_ratio(), 0.8);
        assert!(!status.is_exhausted());
        assert!(status.can_afford(1500));
        assert!(!status.can_afford(2500));
    }

    #[test]
    fn test_performance_settings() {
        let ultra = PerformanceSettings::ultra_lite();
        assert_eq!(ultra.profile, PerformanceProfileType::UltraLite);
        assert_eq!(ultra.max_resolution, "480p");
        assert!(!ultra.enable_animations);
        assert_eq!(ultra.max_parallel_requests, 2);

        let bal = PerformanceSettings::balanced();
        assert_eq!(bal.profile, PerformanceProfileType::Balanced);
        assert_eq!(bal.max_resolution, "720p");
        assert!(bal.enable_animations);
        assert_eq!(bal.max_parallel_requests, 4);

        let hq = PerformanceSettings::high_quality();
        assert_eq!(hq.profile, PerformanceProfileType::HighQuality);
        assert_eq!(hq.max_resolution, "1080p");
        assert_eq!(hq.max_parallel_requests, 8);
    }

    #[test]
    fn test_serde_roundtrip() {
        let video = Video {
            id: "vid1".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            channel_id: "cid".to_string(),
            channel_title: "Chan".to_string(),
            thumbnail_url: "url".to_string(),
            thumbnails: vec![Thumbnail {
                url: "t_url".to_string(),
                width: Some(320),
                height: Some(180),
                quality: ThumbnailQuality::Medium,
            }],
            duration: Duration::from_secs(120),
            view_count: 999,
            like_count: Some(10),
            published_at: Utc::now(),
            embeddable: true,
            live_broadcast_content: None,
        };
        let json = serde_json::to_string(&video).expect("serialize video");
        let decoded: Video = serde_json::from_str(&json).expect("deserialize video");
        assert_eq!(video, decoded);
    }
}
