use crate::client::YouTubeDataApi;
use crate::error::YouTubeApiError;
use crate::models::*;
use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use fluxtube_core::{Channel, Comment, Playlist, PlaylistItem, Video, VideoRating};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// Built-in offline/demo YouTube Data API v3 implementation used when no API key is provided
/// or during offline development and UI verification.
pub struct MockYouTubeApiClient {
    videos: RwLock<Vec<Video>>,
    channels: RwLock<Vec<Channel>>,
    playlists: RwLock<Vec<Playlist>>,
    playlist_items: RwLock<HashMap<String, Vec<PlaylistItem>>>,
    comments: RwLock<HashMap<String, Vec<Comment>>>,
    subscriptions: RwLock<HashSet<String>>,
    ratings: RwLock<HashMap<String, VideoRating>>,
}

impl Default for MockYouTubeApiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockYouTubeApiClient {
    pub fn new() -> Self {
        let now = Utc::now();
        let channels = vec![
            Channel {
                id: "UCrustlang00001".to_string(),
                title: "Rust Systems Lab".to_string(),
                description: "Low-overhead systems programming, Iced UI, and native desktop engineering.".to_string(),
                custom_url: Some("@rustsystemslab".to_string()),
                avatar_url: String::new(),
                banner_url: None,
                subscriber_count: 412_000,
                video_count: 148,
                view_count: Some(18_400_000),
                uploads_playlist_id: "UUrustlang00001".to_string(),
            },
            Channel {
                id: "UCvintagehardware02".to_string(),
                title: "Legacy Mac & ThinkPad Revival".to_string(),
                description: "Benchmarking lightweight native apps on macOS 11 Big Sur and 4GB RAM laptops.".to_string(),
                custom_url: Some("@legacymacrevival".to_string()),
                avatar_url: String::new(),
                banner_url: None,
                subscriber_count: 195_000,
                video_count: 92,
                view_count: Some(7_200_000),
                uploads_playlist_id: "UUvintagehardware02".to_string(),
            },
            Channel {
                id: "UCambientlofi03".to_string(),
                title: "Deep Focus Radio".to_string(),
                description: "24/7 ambient programming streams and acoustic sessions.".to_string(),
                custom_url: Some("@deepfocusradio".to_string()),
                avatar_url: String::new(),
                banner_url: None,
                subscriber_count: 1_240_000,
                video_count: 320,
                view_count: Some(95_000_000),
                uploads_playlist_id: "UUambientlofi03".to_string(),
            },
        ];

        let videos = vec![
            Video {
                id: "dQw4w9WgXcQ".to_string(),
                title: "Building a 90 MB RAM Native Desktop Client in Rust & Iced".to_string(),
                description: "Full walkthrough of replacing Electron overhead with native wgpu/tiny-skia rendering.\n\n0:00 Introduction\n2:15 Memory Profiling\n8:40 Hardware H.264 Acceleration\n14:20 macOS 11 Big Sur Compatibility".to_string(),
                channel_id: "UCrustlang00001".to_string(),
                channel_title: "Rust Systems Lab".to_string(),
                thumbnail_url: String::new(),
                thumbnails: vec![],
                duration: Duration::from_secs(1140),
                view_count: 284_500,
                like_count: Some(18_200),
                published_at: now - ChronoDuration::hours(6),
                embeddable: true,
                live_broadcast_content: None,
            },
            Video {
                id: "M7lc1UVf-VE".to_string(),
                title: "Why Intel HD 4000 Struggles with VP9 & AV1 (And How H.264 Fixes It)".to_string(),
                description: "Comparing CPU usage and battery temperature on 2013 MacBook Air running Big Sur.\n\n0:00 Test Setup\n3:10 VP9 Software Decode\n6:45 H.264 Hardware VPU".to_string(),
                channel_id: "UCvintagehardware02".to_string(),
                channel_title: "Legacy Mac & ThinkPad Revival".to_string(),
                thumbnail_url: String::new(),
                thumbnails: vec![],
                duration: Duration::from_secs(785),
                view_count: 142_300,
                like_count: Some(9_400),
                published_at: now - ChronoDuration::days(1),
                embeddable: true,
                live_broadcast_content: None,
            },
            Video {
                id: "jfKfPfyJRdk".to_string(),
                title: "Deep Focus Synth & Coding Stream [LIVE 24/7]".to_string(),
                description: "Low-CPU background audio stream for long coding sessions.".to_string(),
                channel_id: "UCambientlofi03".to_string(),
                channel_title: "Deep Focus Radio".to_string(),
                thumbnail_url: String::new(),
                thumbnails: vec![],
                duration: Duration::ZERO,
                view_count: 18_920,
                like_count: Some(45_000),
                published_at: now - ChronoDuration::days(14),
                embeddable: true,
                live_broadcast_content: Some("live".to_string()),
            },
            Video {
                id: "aqz-KE-bpKQ".to_string(),
                title: "SQLite WAL Mode & Off-Thread Image Decoding in 45 Seconds".to_string(),
                description: "Quick tip: keep your UI thread completely lock-free with Rayon and WAL.".to_string(),
                channel_id: "UCrustlang00001".to_string(),
                channel_title: "Rust Systems Lab".to_string(),
                thumbnail_url: String::new(),
                thumbnails: vec![],
                duration: Duration::from_secs(45),
                view_count: 89_100,
                like_count: Some(5_100),
                published_at: now - ChronoDuration::days(2),
                embeddable: true,
                live_broadcast_content: None,
            },
            Video {
                id: "ScMzIvxBSi4".to_string(),
                title: "4GB RAM Windows 10 Laptop Challenge: Native Apps vs Browsers".to_string(),
                description: "Real-world memory and thermal test on an old dual-core Celeron laptop with an HDD.".to_string(),
                channel_id: "UCvintagehardware02".to_string(),
                channel_title: "Legacy Mac & ThinkPad Revival".to_string(),
                thumbnail_url: String::new(),
                thumbnails: vec![],
                duration: Duration::from_secs(1520),
                view_count: 512_000,
                like_count: Some(31_400),
                published_at: now - ChronoDuration::days(4),
                embeddable: true,
                live_broadcast_content: None,
            },
            Video {
                id: "ysz5S6PUM-U".to_string(),
                title: "Google OAuth 2.0 PKCE S256 for Desktop Apps Explained".to_string(),
                description: "Securing desktop authentication using OS Keychain, DPAPI, and loopback redirects.".to_string(),
                channel_id: "UCrustlang00001".to_string(),
                channel_title: "Rust Systems Lab".to_string(),
                thumbnail_url: String::new(),
                thumbnails: vec![],
                duration: Duration::from_secs(930),
                view_count: 67_800,
                like_count: Some(4_210),
                published_at: now - ChronoDuration::days(6),
                embeddable: true,
                live_broadcast_content: None,
            },
        ];

        let playlists = vec![Playlist {
            id: "PLfluxtube001".to_string(),
            title: "Rust Desktop Architecture".to_string(),
            description: "Curated videos on building fast native apps.".to_string(),
            item_count: 2,
            thumbnail_url: None,
            channel_id: Some("UCrustlang00001".to_string()),
            channel_title: Some("Rust Systems Lab".to_string()),
            is_local: false,
            account_id: None,
            created_at: Some(now - ChronoDuration::days(10)),
            updated_at: Some(now - ChronoDuration::days(1)),
        }];

        let mut playlist_items = HashMap::new();
        playlist_items.insert(
            "PLfluxtube001".to_string(),
            vec![
                PlaylistItem {
                    id: "PLI_1".to_string(),
                    playlist_id: "PLfluxtube001".to_string(),
                    video_id: "dQw4w9WgXcQ".to_string(),
                    title: videos[0].title.clone(),
                    thumbnail_url: String::new(),
                    position: 0,
                    added_at: now - ChronoDuration::days(5),
                },
                PlaylistItem {
                    id: "PLI_2".to_string(),
                    playlist_id: "PLfluxtube001".to_string(),
                    video_id: "ysz5S6PUM-U".to_string(),
                    title: videos[5].title.clone(),
                    thumbnail_url: String::new(),
                    position: 1,
                    added_at: now - ChronoDuration::days(3),
                },
            ],
        );

        let mut comments = HashMap::new();
        comments.insert(
            "dQw4w9WgXcQ".to_string(),
            vec![
                Comment {
                    id: "cmt_1".to_string(),
                    video_id: "dQw4w9WgXcQ".to_string(),
                    author_name: "@dev_macos".to_string(),
                    author_avatar_url: String::new(),
                    author_channel_id: None,
                    text: "Runs smoothly on my 2014 MacBook Air on Big Sur without spinning up the fans!".to_string(),
                    like_count: 142,
                    published_at: now - ChronoDuration::hours(3),
                    reply_count: 4,
                    is_channel_owner: false,
                },
                Comment {
                    id: "cmt_2".to_string(),
                    video_id: "dQw4w9WgXcQ".to_string(),
                    author_name: "Rust Systems Lab".to_string(),
                    author_avatar_url: String::new(),
                    author_channel_id: Some("UCrustlang00001".to_string()),
                    text: "Thanks! Keeping memory below 120 MB in Ultra Lite mode was the top priority.".to_string(),
                    like_count: 58,
                    published_at: now - ChronoDuration::hours(2),
                    reply_count: 0,
                    is_channel_owner: true,
                },
            ],
        );

        let mut subscriptions = HashSet::new();
        subscriptions.insert("UCrustlang00001".to_string());
        subscriptions.insert("UCvintagehardware02".to_string());

        Self {
            videos: RwLock::new(videos),
            channels: RwLock::new(channels),
            playlists: RwLock::new(playlists),
            playlist_items: RwLock::new(playlist_items),
            comments: RwLock::new(comments),
            subscriptions: RwLock::new(subscriptions),
            ratings: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl YouTubeDataApi for MockYouTubeApiClient {
    async fn search(&self, params: &SearchParams) -> Result<SearchResponse, YouTubeApiError> {
        let q = params.query.trim().to_lowercase();
        let videos = self.videos.read();
        let channels = self.channels.read();
        let playlists = self.playlists.read();

        let items: Vec<SearchResultItem> = match params.resource_type {
            Some(SearchResourceType::Channel) => channels
                .iter()
                .filter(|c| q.is_empty() || c.title.to_lowercase().contains(&q) || c.description.to_lowercase().contains(&q))
                .map(|c| SearchResultItem {
                    kind: "youtube#channel".to_string(),
                    id: c.id.clone(),
                    title: c.title.clone(),
                    description: c.description.clone(),
                    channel_id: c.id.clone(),
                    channel_title: c.title.clone(),
                    thumbnail_url: c.avatar_url.clone(),
                    published_at: Utc::now(),
                    live_broadcast_content: None,
                })
                .collect(),
            Some(SearchResourceType::Playlist) => playlists
                .iter()
                .filter(|p| q.is_empty() || p.title.to_lowercase().contains(&q))
                .map(|p| SearchResultItem {
                    kind: "youtube#playlist".to_string(),
                    id: p.id.clone(),
                    title: p.title.clone(),
                    description: p.description.clone(),
                    channel_id: p.channel_id.clone().unwrap_or_default(),
                    channel_title: p.channel_title.clone().unwrap_or_default(),
                    thumbnail_url: p.thumbnail_url.clone().unwrap_or_default(),
                    published_at: p.created_at.unwrap_or_else(Utc::now),
                    live_broadcast_content: None,
                })
                .collect(),
            _ => {
                let mut matched: Vec<&Video> = videos
                    .iter()
                    .filter(|v| {
                        let matches_query = q.is_empty()
                            || v.title.to_lowercase().contains(&q)
                            || v.channel_title.to_lowercase().contains(&q)
                            || v.description.to_lowercase().contains(&q);
                        let matches_channel = params
                            .channel_id
                            .as_ref()
                            .map(|cid| &v.channel_id == cid)
                            .unwrap_or(true);
                        let matches_dur = match params.video_duration {
                            Some(VideoDurationFilter::Short) => v.duration.as_secs() > 0 && v.duration.as_secs() < 240,
                            Some(VideoDurationFilter::Medium) => v.duration.as_secs() >= 240 && v.duration.as_secs() <= 1200,
                            Some(VideoDurationFilter::Long) => v.duration.as_secs() > 1200,
                            _ => true,
                        };
                        let matches_live = match params.event_type.as_deref() {
                            Some("live") => v.is_live(),
                            _ => true,
                        };
                        matches_query && matches_channel && matches_dur && matches_live
                    })
                    .collect();

                match params.order {
                    Some(SearchOrder::Date) => matched.sort_by_key(|v| std::cmp::Reverse(v.published_at)),
                    Some(SearchOrder::ViewCount) => matched.sort_by_key(|v| std::cmp::Reverse(v.view_count)),
                    Some(SearchOrder::Rating) => matched.sort_by_key(|v| std::cmp::Reverse(v.like_count.unwrap_or(0))),
                    Some(SearchOrder::Title) => matched.sort_by(|a, b| a.title.cmp(&b.title)),
                    _ => {}
                }

                matched
                    .into_iter()
                    .map(|v| SearchResultItem {
                        kind: "youtube#video".to_string(),
                        id: v.id.clone(),
                        title: v.title.clone(),
                        description: v.description.clone(),
                        channel_id: v.channel_id.clone(),
                        channel_title: v.channel_title.clone(),
                        thumbnail_url: v.thumbnail_url.clone(),
                        published_at: v.published_at,
                        live_broadcast_content: v.live_broadcast_content.clone(),
                    })
                    .collect()
            }
        };

        let total = items.len() as u64;
        Ok(PageResult {
            items,
            next_page_token: None,
            prev_page_token: None,
            total_results: total,
        })
    }

    async fn get_video_details(&self, video_ids: &[&str]) -> Result<Vec<Video>, YouTubeApiError> {
        let videos = self.videos.read();
        Ok(videos
            .iter()
            .filter(|v| video_ids.contains(&v.id.as_str()))
            .cloned()
            .collect())
    }

    async fn get_popular_videos(
        &self,
        _region_code: &str,
        _category_id: Option<&str>,
        _page_token: Option<&str>,
        max_results: u32,
    ) -> Result<PageResult<Video>, YouTubeApiError> {
        let mut list = self.videos.read().clone();
        list.sort_by_key(|v| std::cmp::Reverse(v.view_count));
        list.truncate(max_results as usize);
        let total = list.len() as u64;
        Ok(PageResult {
            items: list,
            next_page_token: None,
            prev_page_token: None,
            total_results: total,
        })
    }

    async fn get_channel_details(&self, channel_id_or_handle: &str) -> Result<Channel, YouTubeApiError> {
        let channels = self.channels.read();
        channels
            .iter()
            .find(|c| {
                c.id == channel_id_or_handle
                    || c.custom_url.as_deref() == Some(channel_id_or_handle)
            })
            .cloned()
            .ok_or_else(|| {
                YouTubeApiError::NotFound(format!("Channel '{}' not found", channel_id_or_handle))
            })
    }

    async fn get_my_channel(&self, _access_token: &str) -> Result<Channel, YouTubeApiError> {
        self.channels
            .read()
            .first()
            .cloned()
            .ok_or_else(|| YouTubeApiError::NotFound("No mock channel configured".to_string()))
    }

    async fn get_channel_playlists(
        &self,
        channel_id: &str,
        _page_token: Option<&str>,
    ) -> Result<PageResult<Playlist>, YouTubeApiError> {
        let list: Vec<Playlist> = self
            .playlists
            .read()
            .iter()
            .filter(|p| p.channel_id.as_deref() == Some(channel_id))
            .cloned()
            .collect();
        let total = list.len() as u64;
        Ok(PageResult {
            items: list,
            next_page_token: None,
            prev_page_token: None,
            total_results: total,
        })
    }

    async fn get_playlist_items(
        &self,
        playlist_id: &str,
        _page_token: Option<&str>,
    ) -> Result<PageResult<PlaylistItem>, YouTubeApiError> {
        if let Some(items) = self.playlist_items.read().get(playlist_id) {
            return Ok(PageResult {
                total_results: items.len() as u64,
                items: items.clone(),
                next_page_token: None,
                prev_page_token: None,
            });
        }
        // Derive uploads playlist (UU...)
        if let Some(rest) = playlist_id.strip_prefix("UU") {
            let cid = format!("UC{}", rest);
            let items: Vec<PlaylistItem> = self
                .videos
                .read()
                .iter()
                .filter(|v| v.channel_id == cid)
                .enumerate()
                .map(|(idx, v)| PlaylistItem {
                    id: format!("UPLI_{}_{}", cid, idx),
                    playlist_id: playlist_id.to_string(),
                    video_id: v.id.clone(),
                    title: v.title.clone(),
                    thumbnail_url: v.thumbnail_url.clone(),
                    position: idx as u32,
                    added_at: v.published_at,
                })
                .collect();
            return Ok(PageResult {
                total_results: items.len() as u64,
                items,
                next_page_token: None,
                prev_page_token: None,
            });
        }

        Ok(PageResult::default())
    }

    async fn get_user_playlists(
        &self,
        _access_token: &str,
        _page_token: Option<&str>,
    ) -> Result<PageResult<Playlist>, YouTubeApiError> {
        let items = self.playlists.read().clone();
        let total = items.len() as u64;
        Ok(PageResult {
            items,
            next_page_token: None,
            prev_page_token: None,
            total_results: total,
        })
    }

    async fn create_playlist(
        &self,
        _access_token: &str,
        title: &str,
        description: &str,
        _privacy_status: &str,
    ) -> Result<Playlist, YouTubeApiError> {
        let mut pls = self.playlists.write();
        let pl = Playlist {
            id: format!("PLmock_{}", pls.len() + 1),
            title: title.to_string(),
            description: description.to_string(),
            item_count: 0,
            thumbnail_url: None,
            channel_id: None,
            channel_title: Some("Current User".to_string()),
            is_local: false,
            account_id: None,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        };
        pls.push(pl.clone());
        Ok(pl)
    }

    async fn add_playlist_item(
        &self,
        _access_token: &str,
        playlist_id: &str,
        video_id: &str,
    ) -> Result<PlaylistItem, YouTubeApiError> {
        let title = self
            .videos
            .read()
            .iter()
            .find(|v| v.id == video_id)
            .map(|v| v.title.clone())
            .unwrap_or_else(|| format!("Video {}", video_id));
        let mut map = self.playlist_items.write();
        let list = map.entry(playlist_id.to_string()).or_default();
        let item = PlaylistItem {
            id: format!("PLI_{}_{}", playlist_id, list.len() + 1),
            playlist_id: playlist_id.to_string(),
            video_id: video_id.to_string(),
            title,
            thumbnail_url: String::new(),
            position: list.len() as u32,
            added_at: Utc::now(),
        };
        list.push(item.clone());
        Ok(item)
    }

    async fn remove_playlist_item(
        &self,
        _access_token: &str,
        playlist_item_id: &str,
    ) -> Result<(), YouTubeApiError> {
        let mut map = self.playlist_items.write();
        for list in map.values_mut() {
            list.retain(|i| i.id != playlist_item_id);
        }
        Ok(())
    }

    async fn get_user_subscriptions(
        &self,
        _access_token: &str,
        _page_token: Option<&str>,
    ) -> Result<PageResult<Channel>, YouTubeApiError> {
        let subs = self.subscriptions.read();
        let items: Vec<Channel> = self
            .channels
            .read()
            .iter()
            .filter(|c| subs.contains(&c.id))
            .cloned()
            .collect();
        let total = items.len() as u64;
        Ok(PageResult {
            items,
            next_page_token: None,
            prev_page_token: None,
            total_results: total,
        })
    }

    async fn subscribe(
        &self,
        _access_token: &str,
        channel_id: &str,
    ) -> Result<String, YouTubeApiError> {
        self.subscriptions.write().insert(channel_id.to_string());
        Ok(format!("sub_{}", channel_id))
    }

    async fn unsubscribe(
        &self,
        _access_token: &str,
        subscription_id: &str,
    ) -> Result<(), YouTubeApiError> {
        let cid = subscription_id.strip_prefix("sub_").unwrap_or(subscription_id);
        self.subscriptions.write().remove(cid);
        Ok(())
    }

    async fn get_comment_threads(
        &self,
        video_id: &str,
        _order: &str,
        _page_token: Option<&str>,
    ) -> Result<PageResult<Comment>, YouTubeApiError> {
        let items = self
            .comments
            .read()
            .get(video_id)
            .cloned()
            .unwrap_or_default();
        let total = items.len() as u64;
        Ok(PageResult {
            items,
            next_page_token: None,
            prev_page_token: None,
            total_results: total,
        })
    }

    async fn post_comment(
        &self,
        _access_token: &str,
        video_id: &str,
        text: &str,
    ) -> Result<Comment, YouTubeApiError> {
        let mut map = self.comments.write();
        let list = map.entry(video_id.to_string()).or_default();
        let c = Comment {
            id: format!("cmt_local_{}", list.len() + 1),
            video_id: video_id.to_string(),
            author_name: "You".to_string(),
            author_avatar_url: String::new(),
            author_channel_id: None,
            text: text.to_string(),
            like_count: 0,
            published_at: Utc::now(),
            reply_count: 0,
            is_channel_owner: false,
        };
        list.insert(0, c.clone());
        Ok(c)
    }

    async fn rate_video(
        &self,
        _access_token: &str,
        video_id: &str,
        rating: VideoRating,
    ) -> Result<(), YouTubeApiError> {
        self.ratings.write().insert(video_id.to_string(), rating);
        Ok(())
    }

    async fn get_video_rating(
        &self,
        _access_token: &str,
        video_ids: &[&str],
    ) -> Result<Vec<(String, VideoRating)>, YouTubeApiError> {
        let ratings = self.ratings.read();
        Ok(video_ids
            .iter()
            .map(|id| {
                let r = ratings.get(*id).copied().unwrap_or(VideoRating::None);
                ((*id).to_string(), r)
            })
            .collect())
    }
}
