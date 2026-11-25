use crate::messages::{ChannelTab, Route, SettingsTab};
use fluxtube_core::{
    Bookmark, Channel, Comment, HistoryEntry, Playlist, QuotaStatus, UserProfile, Video,
    VideoRating,
};
use fluxtube_design_system::{DensityMode, LayoutMetrics, ThemeKind, ThemePalette};
use fluxtube_diagnostics::DiagnosticReport;
use fluxtube_platform::HardwareProfile;
use fluxtube_settings::AppSettings;
use fluxtube_youtube_api::{SearchOrder, SearchResourceType, SearchResultItem, VideoDurationFilter};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarItemConfig {
    pub route: Route,
    pub pinned: bool,
    pub visible: bool,
}

pub struct UiState {
    pub route: Route,
    pub window_width: f32,
    pub window_height: f32,
    pub sidebar_collapsed: bool,
    pub sidebar_items: Vec<SidebarItemConfig>,
    pub compact_subscriptions: bool,
    pub system_dark: bool,

    pub settings: AppSettings,
    pub settings_tab: SettingsTab,
    pub private_mode: bool,
    pub hide_shorts: bool,
    pub shorts_autoplay: bool,

    pub is_mock_mode: bool,
    pub active_account: Option<UserProfile>,
    pub accounts: Vec<UserProfile>,
    pub show_account_modal: bool,

    pub home_category: String,
    pub home_videos: Vec<Video>,
    pub trending_videos: Vec<Video>,
    pub continue_watching: Vec<HistoryEntry>,
    pub subscriptions: Vec<Channel>,
    pub subscribed_ids: HashSet<String>,

    pub search_query: String,
    pub search_history: Vec<String>,
    pub search_results: Vec<SearchResultItem>,
    pub search_type: SearchResourceType,
    pub search_duration: VideoDurationFilter,
    pub search_order: SearchOrder,
    pub search_loading: bool,

    pub current_video: Option<Video>,
    pub current_video_rating: VideoRating,
    pub chapters: Vec<(u64, String)>,
    pub comments: Vec<Comment>,
    pub comments_visible: bool,
    pub comment_draft: String,
    pub note_draft: String,
    pub video_notes: HashMap<String, String>,
    pub queue: Vec<Video>,
    pub bookmarks: Vec<Bookmark>,

    pub player_playing: bool,
    pub player_position_secs: u64,
    pub player_duration_secs: u64,
    pub player_volume: f32,
    pub player_muted: bool,
    pub player_rate: f32,
    pub theater_mode: bool,
    pub fullscreen_mode: bool,
    pub mini_player_active: bool,

    pub active_channel: Option<Channel>,
    pub channel_tab: ChannelTab,
    pub channel_videos: Vec<Video>,
    pub channel_playlists: Vec<Playlist>,
    pub channel_filter: String,
    pub favorite_channels: HashSet<String>,

    pub history: Vec<HistoryEntry>,
    pub watch_later: Vec<Video>,
    pub liked_videos: Vec<Video>,
    pub playlists: Vec<Playlist>,
    pub new_playlist_draft: String,

    pub shorts_videos: Vec<Video>,
    pub shorts_index: usize,
    pub live_videos: Vec<Video>,
    pub live_chat_enabled: bool,

    pub thumbnails: HashMap<String, iced::widget::image::Handle>,

    pub hardware: HardwareProfile,
    pub diagnostics: Option<DiagnosticReport>,
    pub quota_general: Option<QuotaStatus>,
    pub quota_search: Option<QuotaStatus>,
    pub cache_size_mb: u64,
    pub update_status_text: Option<String>,

    pub error_banner: Option<String>,
    pub toast: Option<(String, bool)>,
}

impl UiState {
    pub fn new(settings: AppSettings, hardware: HardwareProfile, system_dark: bool, is_mock_mode: bool) -> Self {
        let sidebar_items = vec![
            SidebarItemConfig { route: Route::Home, pinned: true, visible: true },
            SidebarItemConfig { route: Route::Trending, pinned: true, visible: true },
            SidebarItemConfig { route: Route::Subscriptions, pinned: true, visible: true },
            SidebarItemConfig { route: Route::Shorts, pinned: false, visible: true },
            SidebarItemConfig { route: Route::Live, pinned: false, visible: true },
            SidebarItemConfig { route: Route::History, pinned: false, visible: true },
            SidebarItemConfig { route: Route::WatchLater, pinned: false, visible: true },
            SidebarItemConfig { route: Route::Liked, pinned: false, visible: true },
            SidebarItemConfig { route: Route::Playlists, pinned: false, visible: true },
            SidebarItemConfig { route: Route::Favorites, pinned: false, visible: true },
            SidebarItemConfig { route: Route::Diagnostics, pinned: false, visible: true },
            SidebarItemConfig { route: Route::Settings, pinned: true, visible: true },
        ];

        let comments_visible = settings.performance.enable_comments;
        let default_vol = settings.default_volume;
        let start_route = match settings.start_page.as_str() {
            "trending" => Route::Trending,
            "subscriptions" => Route::Subscriptions,
            "library" => Route::Playlists,
            _ => Route::Home,
        };

        Self {
            route: start_route,
            window_width: 1280.0,
            window_height: 760.0,
            sidebar_collapsed: false,
            sidebar_items,
            compact_subscriptions: false,
            system_dark,
            settings,
            settings_tab: SettingsTab::Performance,
            private_mode: false,
            hide_shorts: false,
            shorts_autoplay: false,
            is_mock_mode,
            active_account: None,
            accounts: Vec::new(),
            show_account_modal: false,
            home_category: "All".to_string(),
            home_videos: Vec::new(),
            trending_videos: Vec::new(),
            continue_watching: Vec::new(),
            subscriptions: Vec::new(),
            subscribed_ids: HashSet::new(),
            search_query: String::new(),
            search_history: Vec::new(),
            search_results: Vec::new(),
            search_type: SearchResourceType::Video,
            search_duration: VideoDurationFilter::Any,
            search_order: SearchOrder::Relevance,
            search_loading: false,
            current_video: None,
            current_video_rating: VideoRating::None,
            chapters: Vec::new(),
            comments: Vec::new(),
            comments_visible,
            comment_draft: String::new(),
            note_draft: String::new(),
            video_notes: HashMap::new(),
            queue: Vec::new(),
            bookmarks: Vec::new(),
            player_playing: false,
            player_position_secs: 0,
            player_duration_secs: 0,
            player_volume: default_vol,
            player_muted: false,
            player_rate: 1.0,
            theater_mode: false,
            fullscreen_mode: false,
            mini_player_active: false,
            active_channel: None,
            channel_tab: ChannelTab::Videos,
            channel_videos: Vec::new(),
            channel_playlists: Vec::new(),
            channel_filter: String::new(),
            favorite_channels: HashSet::new(),
            history: Vec::new(),
            watch_later: Vec::new(),
            liked_videos: Vec::new(),
            playlists: Vec::new(),
            new_playlist_draft: String::new(),
            shorts_videos: Vec::new(),
            shorts_index: 0,
            live_videos: Vec::new(),
            live_chat_enabled: false,
            thumbnails: HashMap::new(),
            hardware,
            diagnostics: None,
            quota_general: None,
            quota_search: None,
            cache_size_mb: 0,
            update_status_text: None,
            error_banner: None,
            toast: None,
        }
    }

    pub fn palette(&self) -> ThemePalette {
        ThemePalette::resolve(
            ThemeKind::from_key(&self.settings.theme),
            &self.settings.accent_color,
            self.system_dark,
        )
    }

    pub fn metrics(&self) -> LayoutMetrics {
        LayoutMetrics::new(
            DensityMode::from_key(&self.settings.density),
            self.settings.ui_scale,
        )
    }

    pub fn content_width(&self) -> f32 {
        let m = self.metrics();
        let sb = if self.fullscreen_mode {
            0.0
        } else if self.sidebar_collapsed {
            m.sidebar_collapsed_width
        } else {
            m.sidebar_expanded_width
        };
        (self.window_width - sb - m.space_lg * 2.0).max(320.0)
    }
}

/// Extracts timestamp chapters (`0:00 Intro`, `1:12:04 Topic`) from a video description.
pub fn extract_chapters(description: &str) -> Vec<(u64, String)> {
    let mut chapters = Vec::new();
    for line in description.lines() {
        let trimmed = line.trim();
        if let Some((first, rest)) = trimmed.split_once(char::is_whitespace) {
            if let Some(secs) = parse_hms_timestamp(first.trim_matches(|c| c == '[' || c == ']')) {
                let title = rest.trim().trim_start_matches('-').trim();
                if !title.is_empty() {
                    chapters.push((secs, title.to_string()));
                }
            }
        }
    }
    chapters
}

fn parse_hms_timestamp(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        2 => {
            let m: u64 = parts[0].parse().ok()?;
            let sec: u64 = parts[1].parse().ok()?;
            if sec < 60 { Some(m * 60 + sec) } else { None }
        }
        3 => {
            let h: u64 = parts[0].parse().ok()?;
            let m: u64 = parts[1].parse().ok()?;
            let sec: u64 = parts[2].parse().ok()?;
            if m < 60 && sec < 60 {
                Some(h * 3600 + m * 60 + sec)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn format_seconds_hms(total_secs: u64) -> String {
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}
