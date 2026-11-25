use chrono::Utc;
use fluxtube_auth::{AuthSession, OAuthManager};
use fluxtube_cache::{CacheConfig, ThumbnailCache};
use fluxtube_core::{
    Bookmark, Channel, Comment, HistoryEntry, PerformanceProfileType, PerformanceSettings,
    Playlist, QuotaPool, Video, VideoRating,
};
use fluxtube_diagnostics::DiagnosticsCollector;
use fluxtube_localization::LocalizationManager;
use fluxtube_network::{QuotaTracker, ResilientHttpClient};
use fluxtube_platform::{
    detect_system_dark_mode, detect_system_locale, format_timestamp_share_url, parse_youtube_url,
    AppPaths, HardwareProfile, YouTubeLink,
};
use fluxtube_player::{MediaSource, PlayerBackend};
use fluxtube_player_embed::{EmbedPlayerBackend, EmbedPlayerConfig, PlayerCommand};
use fluxtube_player_native::NativePlayerBackend;
use fluxtube_security::CredentialVault;
use fluxtube_settings::{recommend_profile, SettingsManager};
use fluxtube_storage::StorageManager;
use fluxtube_ui::{extract_chapters, render_app, Route, UiMessage, UiState};
use fluxtube_updater::{UpdateChannel, UpdateCheckOutcome, UpdaterClient};
use fluxtube_youtube_api::{
    LiveYouTubeApiClient, MockYouTubeApiClient, SearchParams, YouTubeDataApi,
};
use iced::keyboard::{self, Key};
use iced::{window, Element, Subscription, Task};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum AppMessage {
    Ui(UiMessage),
    InitialFeedLoaded(Result<InitialFeedPayload, String>),
    SearchCompleted(Result<Vec<fluxtube_youtube_api::SearchResultItem>, String>),
    VideoOpened(Result<VideoOpenPayload, String>),
    ChannelLoaded(Result<ChannelPayload, String>),
    CommentPosted(Result<Comment, String>),
    ThumbnailLoaded(String, Result<iced::widget::image::Handle, String>),
    AuthCompleted(Result<AuthSession, String>),
    UpdateChecked(Result<UpdateCheckOutcome, String>),
    Tick,
    KeyPressed(Key, keyboard::Modifiers),
    WindowResized(f32, f32),
}

#[derive(Debug, Clone)]
pub struct InitialFeedPayload {
    pub popular: Vec<Video>,
    pub subscriptions: Vec<Channel>,
    pub playlists: Vec<Playlist>,
}

#[derive(Debug, Clone)]
pub struct VideoOpenPayload {
    pub video: Video,
    pub comments: Vec<Comment>,
    pub resume_secs: u64,
}

#[derive(Debug, Clone)]
pub struct ChannelPayload {
    pub channel: Channel,
    pub videos: Vec<Video>,
    pub playlists: Vec<Playlist>,
}

pub struct FluxTubeApp {
    pub ui: UiState,
    pub loc: LocalizationManager,
    pub storage: StorageManager,
    pub vault: CredentialVault,
    pub settings_mgr: SettingsManager,
    pub quota: Arc<QuotaTracker>,
    pub http: ResilientHttpClient,
    pub api: Arc<dyn YouTubeDataApi>,
    pub auth: Arc<OAuthManager>,
    pub cache: Arc<ThumbnailCache>,
    pub embed_player: Arc<EmbedPlayerBackend>,
    pub _embed_cmd_rx: Arc<std::sync::Mutex<mpsc::UnboundedReceiver<PlayerCommand>>>,
    pub _native_player: Arc<NativePlayerBackend>,
    pub diagnostics: DiagnosticsCollector,
    pub paths: AppPaths,
    pub search_generation: u64,
}

impl FluxTubeApp {
    pub fn bootstrap(in_memory: bool) -> Result<(Self, Task<AppMessage>), String> {
        let paths = AppPaths::resolve().map_err(|e| e.to_string())?;
        if !in_memory {
            paths.ensure_dirs().map_err(|e| e.to_string())?;
        }

        let storage = if in_memory {
            StorageManager::open_in_memory().map_err(|e| e.to_string())?
        } else {
            StorageManager::open(&paths.db_path).map_err(|e| e.to_string())?
        };

        let vault = if in_memory {
            CredentialVault::in_memory()
        } else {
            CredentialVault::auto("fluxtube")
        };

        let hardware = HardwareProfile::detect();
        let mut settings_mgr =
            SettingsManager::load_from_storage(&storage).map_err(|e| e.to_string())?;

        // Apply automatic hardware recommendation on very first launch if no settings row existed
        if storage.get_setting("app_settings").ok().flatten().is_none() {
            let rec = recommend_profile(hardware.cpu_cores, hardware.total_ram_mb);
            settings_mgr.current_mut().performance = match rec {
                PerformanceProfileType::UltraLite => PerformanceSettings::ultra_lite(),
                PerformanceProfileType::Balanced => PerformanceSettings::balanced(),
                PerformanceProfileType::HighQuality => PerformanceSettings::high_quality(),
                PerformanceProfileType::Custom => PerformanceSettings::balanced(),
            };
        }

        let effective_locale = if settings_mgr.current().language == "auto" {
            detect_system_locale()
        } else {
            settings_mgr.current().language.as_str()
        };
        let loc = LocalizationManager::new(effective_locale);

        let quota = Arc::new(QuotaTracker::new(10_000, 100));
        let http = ResilientHttpClient::builder()
            .quota_tracker(quota.clone())
            .build()
            .map_err(|e| e.to_string())?;

        let api_key = std::env::var("YOUTUBE_API_KEY").unwrap_or_default();
        let force_mock = std::env::var("FLUXTUBE_OFFLINE_MOCK")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        let is_mock_mode = force_mock || api_key.trim().is_empty();

        let api: Arc<dyn YouTubeDataApi> = if is_mock_mode {
            Arc::new(MockYouTubeApiClient::new())
        } else {
            Arc::new(LiveYouTubeApiClient::new(
                http.clone(),
                quota.clone(),
                api_key,
            ))
        };

        let client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default();
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").ok().filter(|s| !s.is_empty());
        let auth = Arc::new(OAuthManager::new(
            vault.clone(),
            storage.clone(),
            http.client().clone(),
            client_id,
            client_secret,
            vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
                "https://www.googleapis.com/auth/youtube.readonly".to_string(),
            ],
        ));

        let cache_dir = if in_memory {
            std::env::temp_dir().join("fluxtube_smoke_cache")
        } else {
            paths.thumbnails_dir.clone()
        };
        let l2_bytes = (settings_mgr.current().performance.cache_size_mb as u64) * 1024 * 1024;
        let cache = Arc::new(
            ThumbnailCache::new(CacheConfig::new(16 * 1024 * 1024, l2_bytes.max(1024 * 1024), cache_dir))
                .map_err(|e| e.to_string())?,
        );

        let (embed_player, embed_rx) = EmbedPlayerBackend::new(EmbedPlayerConfig::new(
            settings_mgr.current().performance.profile,
        ));
        let embed_player = Arc::new(embed_player);
        let native_player = Arc::new(NativePlayerBackend::new());

        let mut ui = UiState::new(
            settings_mgr.current().clone(),
            hardware,
            detect_system_dark_mode(),
            is_mock_mode,
        );

        ui.accounts = storage.get_all_accounts().unwrap_or_default();
        ui.active_account = storage.get_active_account().unwrap_or(None);
        let acc_id = ui.active_account.as_ref().map(|a| a.id.clone());
        ui.history = storage.get_history(acc_id.as_deref(), 50).unwrap_or_default();
        ui.continue_watching = ui
            .history
            .iter()
            .filter(|h| h.is_resumable())
            .cloned()
            .collect();
        ui.search_history = storage.get_search_history(15).unwrap_or_default();
        ui.bookmarks = storage.get_all_bookmarks().unwrap_or_default();
        ui.playlists = storage.get_playlists(acc_id.as_deref()).unwrap_or_default();

        let mut diagnostics = DiagnosticsCollector::new();
        ui.diagnostics = Some(diagnostics.generate_report(
            env!("CARGO_PKG_VERSION"),
            &format!("{:?}", ui.settings.performance.profile),
            &ui.settings.theme,
            "Isolated WebView (wry + h264ify)",
            &ui.hardware.graphics_backend,
            ui.settings.h264ify_enabled,
            ui.active_account.is_some(),
        ));
        ui.quota_general = Some(quota.get_status(QuotaPool::General));
        ui.quota_search = Some(quota.get_status(QuotaPool::Search));

        let app = Self {
            ui,
            loc,
            storage,
            vault,
            settings_mgr,
            quota,
            http,
            api,
            auth,
            cache,
            embed_player,
            _embed_cmd_rx: Arc::new(std::sync::Mutex::new(embed_rx)),
            _native_player: native_player,
            diagnostics,
            paths,
            search_generation: 0,
        };

        let init_task = app.load_initial_feed_task();
        Ok((app, init_task))
    }

    fn load_initial_feed_task(&self) -> Task<AppMessage> {
        let api = self.api.clone();
        Task::perform(
            async move {
                let popular = api
                    .get_popular_videos("US", None, None, 20)
                    .await
                    .map(|p| p.items)
                    .map_err(|e| e.to_string())?;
                let subscriptions = api
                    .get_user_subscriptions("", None)
                    .await
                    .map(|p| p.items)
                    .unwrap_or_default();
                let playlists = api
                    .get_user_playlists("", None)
                    .await
                    .map(|p| p.items)
                    .unwrap_or_default();
                Ok(InitialFeedPayload {
                    popular,
                    subscriptions,
                    playlists,
                })
            },
            AppMessage::InitialFeedLoaded,
        )
    }

    fn prefetch_thumbnails(&self, videos: &[Video]) -> Task<AppMessage> {
        let limit = if self.ui.settings.performance.profile == PerformanceProfileType::UltraLite {
            6
        } else {
            16
        };
        let mut tasks = Vec::new();
        for v in videos.iter().take(limit) {
            if v.thumbnail_url.is_empty() || self.ui.thumbnails.contains_key(&v.thumbnail_url) {
                continue;
            }
            let url = v.thumbnail_url.clone();
            let cache = self.cache.clone();
            tasks.push(Task::perform(
                async move {
                    let res = cache
                        .get_or_fetch(&url, None)
                        .await
                        .map(|img| {
                            iced::widget::image::Handle::from_rgba(
                                img.width,
                                img.height,
                                img.rgba.clone(),
                            )
                        })
                        .map_err(|e| e.to_string());
                    (url, res)
                },
                |(u, r)| AppMessage::ThumbnailLoaded(u, r),
            ));
        }
        Task::batch(tasks)
    }

    pub fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match message {
            AppMessage::InitialFeedLoaded(Ok(payload)) => {
                self.ui.home_videos = payload.popular.clone();
                self.ui.trending_videos = payload.popular.clone();
                self.ui.shorts_videos = payload.popular.iter().filter(|v| v.is_short()).cloned().collect();
                self.ui.live_videos = payload.popular.iter().filter(|v| v.is_live()).cloned().collect();
                self.ui.subscriptions = payload.subscriptions;
                self.ui.subscribed_ids = self.ui.subscriptions.iter().map(|c| c.id.clone()).collect();

                for pl in payload.playlists {
                    if !self.ui.playlists.iter().any(|existing| existing.id == pl.id) {
                        self.ui.playlists.push(pl);
                    }
                }
                self.ui.quota_general = Some(self.quota.get_status(QuotaPool::General));
                self.ui.quota_search = Some(self.quota.get_status(QuotaPool::Search));
                self.prefetch_thumbnails(&self.ui.home_videos)
            }
            AppMessage::InitialFeedLoaded(Err(e)) => {
                self.ui.error_banner = Some(e);
                Task::none()
            }
            AppMessage::SearchCompleted(Ok(items)) => {
                self.ui.search_loading = false;
                self.ui.search_results = items;
                self.ui.quota_search = Some(self.quota.get_status(QuotaPool::Search));
                Task::none()
            }
            AppMessage::SearchCompleted(Err(e)) => {
                self.ui.search_loading = false;
                self.ui.error_banner = Some(e);
                Task::none()
            }
            AppMessage::VideoOpened(Ok(payload)) => {
                let vid = payload.video;
                self.ui.chapters = extract_chapters(&vid.description);
                self.ui.comments = payload.comments;
                self.ui.player_duration_secs = vid.duration.as_secs();
                self.ui.player_position_secs = payload.resume_secs;
                self.ui.player_playing = true;
                self.ui.route = Route::Watch;
                self.ui.mini_player_active = false;
                self.ui.note_draft = self
                    .ui
                    .video_notes
                    .get(&vid.id)
                    .cloned()
                    .unwrap_or_default();

                if !self.ui.private_mode {
                    let entry = HistoryEntry {
                        id: None,
                        video_id: vid.id.clone(),
                        title: vid.title.clone(),
                        channel_title: vid.channel_title.clone(),
                        thumbnail_url: vid.thumbnail_url.clone(),
                        duration_seconds: vid.duration.as_secs(),
                        watched_at: Utc::now(),
                        playback_position_seconds: payload.resume_secs,
                        completed: false,
                        account_id: self.ui.active_account.as_ref().map(|a| a.id.clone()),
                    };
                    let _ = self.storage.add_history_entry(&entry);
                    self.ui.history.insert(0, entry);
                }

                let embed = self.embed_player.clone();
                let source = MediaSource::youtube(&vid.id);
                let start = if payload.resume_secs > 0 {
                    Some(Duration::from_secs(payload.resume_secs))
                } else {
                    None
                };
                self.ui.current_video = Some(vid);

                Task::perform(
                    async move {
                        let _ = embed.load(source, true, start).await;
                    },
                    |_| AppMessage::Ui(UiMessage::RefreshDiagnostics),
                )
            }
            AppMessage::VideoOpened(Err(e)) => {
                self.ui.error_banner = Some(e);
                Task::none()
            }
            AppMessage::ChannelLoaded(Ok(payload)) => {
                self.ui.active_channel = Some(payload.channel);
                self.ui.channel_videos = payload.videos;
                self.ui.channel_playlists = payload.playlists;
                self.ui.route = Route::Channel;
                self.prefetch_thumbnails(&self.ui.channel_videos)
            }
            AppMessage::ChannelLoaded(Err(e)) => {
                self.ui.error_banner = Some(e);
                Task::none()
            }
            AppMessage::CommentPosted(Ok(c)) => {
                self.ui.comments.insert(0, c);
                self.ui.comment_draft.clear();
                self.ui.toast = Some(("Comment posted".to_string(), false));
                Task::none()
            }
            AppMessage::CommentPosted(Err(e)) => {
                self.ui.toast = Some((e, true));
                Task::none()
            }
            AppMessage::ThumbnailLoaded(url, Ok(handle)) => {
                self.ui.thumbnails.insert(url, handle);
                self.ui.cache_size_mb = self.cache.stats().l2_bytes / (1024 * 1024);
                Task::none()
            }
            AppMessage::ThumbnailLoaded(_, Err(_)) => Task::none(),
            AppMessage::AuthCompleted(Ok(session)) => {
                self.ui.active_account = Some(session.profile);
                self.ui.accounts = self.storage.get_all_accounts().unwrap_or_default();
                self.ui.show_account_modal = false;
                self.ui.toast = Some(("Signed in with Google OAuth".to_string(), false));
                self.load_initial_feed_task()
            }
            AppMessage::AuthCompleted(Err(e)) => {
                self.ui.toast = Some((format!("Auth failed: {}", e), true));
                Task::none()
            }
            AppMessage::UpdateChecked(Ok(outcome)) => {
                self.ui.update_status_text = Some(match outcome {
                    UpdateCheckOutcome::UpToDate { current_version } => {
                        format!("Up to date (v{})", current_version)
                    }
                    UpdateCheckOutcome::UpdateAvailable(m) => {
                        format!("Update available: v{} ({:?})", m.version, m.channel)
                    }
                });
                Task::none()
            }
            AppMessage::UpdateChecked(Err(e)) => {
                self.ui.update_status_text = Some(format!("Update check: {}", e));
                Task::none()
            }
            AppMessage::Tick => {
                if self.ui.player_playing {
                    if let Some(ref vid) = self.ui.current_video {
                        let dur = self.ui.player_duration_secs.max(vid.duration.as_secs());
                        if dur == 0 || self.ui.player_position_secs < dur {
                            self.ui.player_position_secs += 1;
                            if !self.ui.private_mode && self.ui.player_position_secs % 5 == 0 {
                                let _ = self.storage.save_position(
                                    &vid.id,
                                    self.ui.player_position_secs,
                                    dur,
                                );
                            }
                        } else if self.ui.settings.autoplay_next && !self.ui.queue.is_empty() {
                            return self.update(AppMessage::Ui(UiMessage::PlayNextInQueue));
                        } else {
                            self.ui.player_playing = false;
                        }
                    }
                }
                Task::none()
            }
            AppMessage::WindowResized(w, h) => {
                self.ui.window_width = w;
                self.ui.window_height = h;
                Task::none()
            }
            AppMessage::KeyPressed(key, mods) => self.handle_shortcut(key, mods),
            AppMessage::Ui(msg) => self.handle_ui_message(msg),
        }
    }

    fn handle_shortcut(&mut self, key: Key, mods: keyboard::Modifiers) -> Task<AppMessage> {
        let ctrl_or_cmd = mods.command() || mods.control();
        match key {
            Key::Named(keyboard::key::Named::Space) if self.ui.route == Route::Watch => {
                self.handle_ui_message(UiMessage::PlayerTogglePlay)
            }
            Key::Named(keyboard::key::Named::Escape) if self.ui.fullscreen_mode => {
                self.ui.fullscreen_mode = false;
                Task::none()
            }
            Key::Named(keyboard::key::Named::ArrowLeft) if self.ui.route == Route::Watch => {
                self.handle_ui_message(UiMessage::PlayerSeekRelative(-5))
            }
            Key::Named(keyboard::key::Named::ArrowRight) if self.ui.route == Route::Watch => {
                self.handle_ui_message(UiMessage::PlayerSeekRelative(5))
            }
            Key::Named(keyboard::key::Named::ArrowUp) if self.ui.route == Route::Watch => {
                self.handle_ui_message(UiMessage::PlayerVolumeChanged((self.ui.player_volume + 0.05).min(1.0)))
            }
            Key::Named(keyboard::key::Named::ArrowDown) if self.ui.route == Route::Watch => {
                self.handle_ui_message(UiMessage::PlayerVolumeChanged((self.ui.player_volume - 0.05).max(0.0)))
            }
            Key::Character(ref c) if ctrl_or_cmd && c.eq_ignore_ascii_case("r") => {
                self.handle_ui_message(UiMessage::RefreshFeed)
            }
            Key::Character(ref c) if ctrl_or_cmd && c.eq_ignore_ascii_case("f") => {
                self.handle_ui_message(UiMessage::Navigate(Route::Search))
            }
            Key::Character(ref c) if ctrl_or_cmd && c.eq_ignore_ascii_case("l") => {
                self.handle_ui_message(UiMessage::OpenUrlOrClipboard)
            }
            Key::Character(ref c) if !ctrl_or_cmd && self.ui.route == Route::Watch => {
                match c.to_lowercase().as_str() {
                    "k" => self.handle_ui_message(UiMessage::PlayerTogglePlay),
                    "j" => self.handle_ui_message(UiMessage::PlayerSeekRelative(-10)),
                    "l" => self.handle_ui_message(UiMessage::PlayerSeekRelative(10)),
                    "m" => self.handle_ui_message(UiMessage::PlayerToggleMute),
                    "f" => self.handle_ui_message(UiMessage::PlayerToggleFullscreen),
                    "t" => self.handle_ui_message(UiMessage::PlayerToggleTheater),
                    _ => Task::none(),
                }
            }
            _ => Task::none(),
        }
    }

    fn handle_ui_message(&mut self, msg: UiMessage) -> Task<AppMessage> {
        match msg {
            UiMessage::Navigate(route) => {
                self.ui.route = route;
                self.ui.error_banner = None;
                if route != Route::Watch && self.ui.current_video.is_some() {
                    let keep = self.ui.settings.performance.profile != PerformanceProfileType::UltraLite;
                    self.ui.mini_player_active = keep;
                }
                Task::none()
            }
            UiMessage::ToggleSidebar => {
                self.ui.sidebar_collapsed = !self.ui.sidebar_collapsed;
                Task::none()
            }
            UiMessage::PinSection(route) => {
                if let Some(item) = self.ui.sidebar_items.iter_mut().find(|i| i.route == route) {
                    item.pinned = !item.pinned;
                }
                Task::none()
            }
            UiMessage::HideSection(route) => {
                if let Some(item) = self.ui.sidebar_items.iter_mut().find(|i| i.route == route) {
                    item.visible = !item.visible;
                }
                Task::none()
            }
            UiMessage::MoveSectionUp(route) => {
                if let Some(idx) = self.ui.sidebar_items.iter().position(|i| i.route == route) {
                    if idx > 0 {
                        self.ui.sidebar_items.swap(idx, idx - 1);
                    }
                }
                Task::none()
            }
            UiMessage::MoveSectionDown(route) => {
                if let Some(idx) = self.ui.sidebar_items.iter().position(|i| i.route == route) {
                    if idx + 1 < self.ui.sidebar_items.len() {
                        self.ui.sidebar_items.swap(idx, idx + 1);
                    }
                }
                Task::none()
            }
            UiMessage::ToggleCompactSubscriptions => {
                self.ui.compact_subscriptions = !self.ui.compact_subscriptions;
                Task::none()
            }
            UiMessage::SearchInputChanged(val) => {
                self.ui.search_query = val;
                Task::none()
            }
            UiMessage::SelectSearchSuggestion(q) => {
                self.ui.search_query = q;
                self.handle_ui_message(UiMessage::SubmitSearch)
            }
            UiMessage::SubmitSearch => {
                let q = self.ui.search_query.trim().to_string();
                if let Some(link) = parse_youtube_url(&q) {
                    return self.open_parsed_link(link);
                }
                self.ui.route = Route::Search;
                if q.is_empty() {
                    return Task::none();
                }
                if !self.ui.private_mode {
                    let _ = self.storage.add_search_query(&q);
                    self.ui.search_history = self.storage.get_search_history(15).unwrap_or_default();
                }
                self.search_generation += 1;
                self.ui.search_loading = true;
                let api = self.api.clone();
                let params = SearchParams {
                    query: q,
                    order: Some(self.ui.search_order),
                    video_duration: Some(self.ui.search_duration),
                    resource_type: Some(self.ui.search_type),
                    ..Default::default()
                };
                Task::perform(
                    async move { api.search(&params).await.map(|r| r.items).map_err(|e| e.to_string()) },
                    AppMessage::SearchCompleted,
                )
            }
            UiMessage::ClearSearchHistory => {
                let _ = self.storage.clear_search_history();
                self.ui.search_history.clear();
                Task::none()
            }
            UiMessage::SetSearchType(t) => {
                self.ui.search_type = t;
                self.handle_ui_message(UiMessage::SubmitSearch)
            }
            UiMessage::SetSearchDuration(d) => {
                self.ui.search_duration = d;
                self.handle_ui_message(UiMessage::SubmitSearch)
            }
            UiMessage::SetSearchOrder(o) => {
                self.ui.search_order = o;
                self.handle_ui_message(UiMessage::SubmitSearch)
            }
            UiMessage::OpenUrlOrClipboard => {
                if let Some(link) = parse_youtube_url(&self.ui.search_query) {
                    return self.open_parsed_link(link);
                }
                self.ui.toast = Some(("Paste a YouTube video/channel/playlist URL into the search box".to_string(), false));
                Task::none()
            }
            UiMessage::RefreshFeed => self.load_initial_feed_task(),
            UiMessage::SelectHomeCategory(cat) => {
                self.ui.home_category = cat;
                Task::none()
            }
            UiMessage::OpenVideo(id) => self.open_video_task(id, None),
            UiMessage::OpenVideoAt(id, secs) => self.open_video_task(id, Some(secs)),
            UiMessage::PlayerTogglePlay => {
                self.ui.player_playing = !self.ui.player_playing;
                let embed = self.embed_player.clone();
                let playing = self.ui.player_playing;
                Task::perform(
                    async move {
                        let _ = if playing { embed.play().await } else { embed.pause().await };
                    },
                    |_| AppMessage::Ui(UiMessage::RefreshDiagnostics),
                )
            }
            UiMessage::PlayerSeekRelative(delta) => {
                let dur = self.ui.player_duration_secs;
                let next = if delta < 0 {
                    self.ui.player_position_secs.saturating_sub(delta.unsigned_abs())
                } else {
                    (self.ui.player_position_secs + delta as u64).min(dur.max(self.ui.player_position_secs + delta as u64))
                };
                self.ui.player_position_secs = next;
                let embed = self.embed_player.clone();
                Task::perform(
                    async move {
                        let _ = embed.seek(Duration::from_secs(next)).await;
                    },
                    |_| AppMessage::Ui(UiMessage::RefreshDiagnostics),
                )
            }
            UiMessage::PlayerVolumeChanged(vol) => {
                self.ui.player_volume = vol.clamp(0.0, 1.0);
                let embed = self.embed_player.clone();
                Task::perform(
                    async move {
                        let _ = embed.set_volume(vol).await;
                    },
                    |_| AppMessage::Ui(UiMessage::RefreshDiagnostics),
                )
            }
            UiMessage::PlayerToggleMute => {
                self.ui.player_muted = !self.ui.player_muted;
                let muted = self.ui.player_muted;
                let embed = self.embed_player.clone();
                Task::perform(
                    async move {
                        let _ = embed.set_muted(muted).await;
                    },
                    |_| AppMessage::Ui(UiMessage::RefreshDiagnostics),
                )
            }
            UiMessage::PlayerRateChanged(rate) => {
                self.ui.player_rate = rate;
                let embed = self.embed_player.clone();
                Task::perform(
                    async move {
                        let _ = embed.set_playback_rate(rate).await;
                    },
                    |_| AppMessage::Ui(UiMessage::RefreshDiagnostics),
                )
            }
            UiMessage::PlayerToggleTheater => {
                self.ui.theater_mode = !self.ui.theater_mode;
                Task::none()
            }
            UiMessage::PlayerToggleFullscreen => {
                self.ui.fullscreen_mode = !self.ui.fullscreen_mode;
                Task::none()
            }
            UiMessage::PlayerToggleMini => {
                self.ui.mini_player_active = !self.ui.mini_player_active;
                Task::none()
            }
            UiMessage::ToggleCommentsPanel => {
                self.ui.comments_visible = !self.ui.comments_visible;
                Task::none()
            }
            UiMessage::CommentDraftChanged(txt) => {
                self.ui.comment_draft = txt;
                Task::none()
            }
            UiMessage::SubmitComment => {
                let text = self.ui.comment_draft.trim().to_string();
                let Some(ref vid) = self.ui.current_video else {
                    return Task::none();
                };
                if text.is_empty() {
                    return Task::none();
                }
                let video_id = vid.id.clone();
                let api = self.api.clone();
                let is_mock = self.ui.is_mock_mode;
                let auth = self.auth.clone();
                Task::perform(
                    async move {
                        let token = if is_mock {
                            String::new()
                        } else {
                            auth.get_active_access_token()
                                .await
                                .map_err(|e| e.to_string())?
                                .ok_or_else(|| "Google sign-in required to post comments".to_string())?
                        };
                        api.post_comment(&token, &video_id, &text)
                            .await
                            .map_err(|e| e.to_string())
                    },
                    AppMessage::CommentPosted,
                )
            }
            UiMessage::RateCurrentVideo(rating) => {
                let next = if self.ui.current_video_rating == rating {
                    VideoRating::None
                } else {
                    rating
                };
                self.ui.current_video_rating = next;
                if let Some(ref v) = self.ui.current_video {
                    if next == VideoRating::Like && !self.ui.liked_videos.iter().any(|x| x.id == v.id) {
                        self.ui.liked_videos.insert(0, v.clone());
                    }
                    let vid_id = v.id.clone();
                    let api = self.api.clone();
                    return Task::perform(
                        async move {
                            let _ = api.rate_video("", &vid_id, next).await;
                        },
                        |_| AppMessage::Ui(UiMessage::RefreshDiagnostics),
                    );
                }
                Task::none()
            }
            UiMessage::ToggleWatchLater(video_id) => {
                if let Some(pos) = self.ui.watch_later.iter().position(|v| v.id == video_id) {
                    self.ui.watch_later.remove(pos);
                    self.ui.toast = Some(("Removed from Watch Later".to_string(), false));
                } else if let Some(v) = self
                    .ui
                    .home_videos
                    .iter()
                    .chain(self.ui.current_video.iter())
                    .find(|v| v.id == video_id)
                    .cloned()
                {
                    self.ui.watch_later.push(v);
                    self.ui.toast = Some(("Saved to local Watch Later".to_string(), false));
                }
                Task::none()
            }
            UiMessage::AddToQueue(video_id) => {
                if let Some(v) = self
                    .ui
                    .home_videos
                    .iter()
                    .chain(self.ui.trending_videos.iter())
                    .find(|v| v.id == video_id)
                    .cloned()
                {
                    self.ui.queue.push(v);
                    self.ui.toast = Some(("Added to playback queue".to_string(), false));
                }
                Task::none()
            }
            UiMessage::PlayNextInQueue => {
                if !self.ui.queue.is_empty() {
                    let next = self.ui.queue.remove(0);
                    return self.open_video_task(next.id, None);
                }
                Task::none()
            }
            UiMessage::AddTimestampBookmark => {
                if let Some(ref v) = self.ui.current_video {
                    let bm = Bookmark {
                        id: 0,
                        video_id: v.id.clone(),
                        title: v.title.clone(),
                        timestamp_seconds: self.ui.player_position_secs,
                        note: None,
                        created_at: Utc::now(),
                    };
                    if let Ok(id) = self.storage.add_bookmark(&bm) {
                        let mut saved = bm;
                        saved.id = id;
                        self.ui.bookmarks.insert(0, saved);
                        self.ui.toast = Some(("Timestamp bookmark saved".to_string(), false));
                    }
                }
                Task::none()
            }
            UiMessage::CopyTimestampUrl => {
                if let Some(ref v) = self.ui.current_video {
                    let link = format_timestamp_share_url(&v.id, self.ui.player_position_secs);
                    self.ui.toast = Some((format!("Share link: {}", link), false));
                }
                Task::none()
            }
            UiMessage::NoteDraftChanged(txt) => {
                self.ui.note_draft = txt;
                Task::none()
            }
            UiMessage::SaveVideoNote => {
                if let Some(ref v) = self.ui.current_video {
                    self.ui.video_notes.insert(v.id.clone(), self.ui.note_draft.clone());
                    let _ = self.storage.set_setting(&format!("note:{}", v.id), &self.ui.note_draft);
                    self.ui.toast = Some(("Local note saved".to_string(), false));
                }
                Task::none()
            }
            UiMessage::OpenInExternalBrowser(url) => {
                let _ = fluxtube_platform::open_external_url(&url);
                Task::none()
            }
            UiMessage::OpenChannel(channel_id) => {
                let api = self.api.clone();
                Task::perform(
                    async move {
                        let channel = api
                            .get_channel_details(&channel_id)
                            .await
                            .map_err(|e| e.to_string())?;
                        let uploads = api
                            .get_playlist_items(&channel.uploads_playlist_id, None)
                            .await
                            .map_err(|e| e.to_string())?;
                        let ids: Vec<String> = uploads.items.iter().map(|i| i.video_id.clone()).collect();
                        let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
                        let videos = api.get_video_details(&id_refs).await.unwrap_or_default();
                        let playlists = api
                            .get_channel_playlists(&channel.id, None)
                            .await
                            .map(|p| p.items)
                            .unwrap_or_default();
                        Ok(ChannelPayload {
                            channel,
                            videos,
                            playlists,
                        })
                    },
                    AppMessage::ChannelLoaded,
                )
            }
            UiMessage::SwitchChannelTab(tab) => {
                self.ui.channel_tab = tab;
                Task::none()
            }
            UiMessage::ToggleChannelSubscribe(cid) => {
                if self.ui.subscribed_ids.contains(&cid) {
                    self.ui.subscribed_ids.remove(&cid);
                    self.ui.subscriptions.retain(|c| c.id != cid);
                } else {
                    self.ui.subscribed_ids.insert(cid.clone());
                    if let Some(ref ch) = self.ui.active_channel {
                        if ch.id == cid {
                            self.ui.subscriptions.push(ch.clone());
                        }
                    }
                }
                Task::none()
            }
            UiMessage::ToggleChannelFavorite(cid) => {
                if self.ui.favorite_channels.contains(&cid) {
                    self.ui.favorite_channels.remove(&cid);
                } else {
                    self.ui.favorite_channels.insert(cid);
                }
                Task::none()
            }
            UiMessage::ChannelFilterChanged(f) => {
                self.ui.channel_filter = f;
                Task::none()
            }
            UiMessage::ShortsPrev => {
                if self.ui.shorts_index > 0 {
                    self.ui.shorts_index -= 1;
                }
                Task::none()
            }
            UiMessage::ShortsNext => {
                if self.ui.shorts_index + 1 < self.ui.shorts_videos.len() {
                    self.ui.shorts_index += 1;
                }
                Task::none()
            }
            UiMessage::ToggleShortsAutoplay(v) => {
                self.ui.shorts_autoplay = v;
                Task::none()
            }
            UiMessage::ToggleHideShorts(v) => {
                self.ui.hide_shorts = v;
                Task::none()
            }
            UiMessage::ToggleLiveChat => {
                self.ui.live_chat_enabled = !self.ui.live_chat_enabled;
                Task::none()
            }
            UiMessage::CreateLocalPlaylistDraftChanged(s) => {
                self.ui.new_playlist_draft = s;
                Task::none()
            }
            UiMessage::CreateLocalPlaylist => {
                let title = self.ui.new_playlist_draft.trim().to_string();
                if title.is_empty() {
                    return Task::none();
                }
                let pl = Playlist {
                    id: format!("local_pl_{}", Utc::now().timestamp_millis()),
                    title,
                    description: "Device-local playlist".to_string(),
                    item_count: 0,
                    thumbnail_url: None,
                    channel_id: None,
                    channel_title: None,
                    is_local: true,
                    account_id: self.ui.active_account.as_ref().map(|a| a.id.clone()),
                    created_at: Some(Utc::now()),
                    updated_at: Some(Utc::now()),
                };
                if self.storage.create_playlist(&pl).is_ok() {
                    self.ui.playlists.insert(0, pl);
                    self.ui.new_playlist_draft.clear();
                }
                Task::none()
            }
            UiMessage::ExportLocalPlaylists => {
                if let Ok(json) = serde_json::to_string_pretty(&self.ui.playlists) {
                    let out = self.paths.data_dir.join("playlists_export.json");
                    let _ = std::fs::write(&out, json);
                    self.ui.toast = Some((format!("Exported to {}", out.display()), false));
                }
                Task::none()
            }
            UiMessage::ClearWatchHistory => {
                let acc = self.ui.active_account.as_ref().map(|a| a.id.clone());
                let _ = self.storage.clear_history(acc.as_deref());
                let _ = self.storage.clear_all_positions();
                self.ui.history.clear();
                self.ui.continue_watching.clear();
                self.ui.toast = Some(("Watch history cleared".to_string(), false));
                Task::none()
            }
            UiMessage::SwitchSettingsTab(tab) => {
                self.ui.settings_tab = tab;
                Task::none()
            }
            UiMessage::SelectTheme(kind) => {
                self.ui.settings.theme = kind.as_key().to_string();
                Task::none()
            }
            UiMessage::AccentColorChanged(hex) => {
                self.ui.settings.accent_color = hex;
                Task::none()
            }
            UiMessage::SelectDensity(d) => {
                self.ui.settings.density = d.as_key().to_string();
                Task::none()
            }
            UiMessage::UiScaleChanged(s) => {
                self.ui.settings.ui_scale = s;
                Task::none()
            }
            UiMessage::SelectLocale(locale) => {
                if self.loc.set_locale(&locale).is_ok() {
                    self.ui.settings.language = locale;
                }
                Task::none()
            }
            UiMessage::SelectPerformanceProfile(profile) => {
                self.ui.settings.performance = match profile {
                    PerformanceProfileType::UltraLite => PerformanceSettings::ultra_lite(),
                    PerformanceProfileType::Balanced => PerformanceSettings::balanced(),
                    PerformanceProfileType::HighQuality => PerformanceSettings::high_quality(),
                    PerformanceProfileType::Custom => PerformanceSettings {
                        profile: PerformanceProfileType::Custom,
                        ..self.ui.settings.performance.clone()
                    },
                };
                self.ui.comments_visible = self.ui.settings.performance.enable_comments;
                Task::none()
            }
            UiMessage::RecommendProfileForHardware => {
                let rec = recommend_profile(self.ui.hardware.cpu_cores, self.ui.hardware.total_ram_mb);
                self.handle_ui_message(UiMessage::SelectPerformanceProfile(rec))
            }
            UiMessage::ToggleH264ify(v) => {
                self.ui.settings.h264ify_enabled = v;
                Task::none()
            }
            UiMessage::ToggleAnimations(v) => {
                self.ui.settings.performance.enable_animations = v;
                self.ui.settings.performance.profile = PerformanceProfileType::Custom;
                Task::none()
            }
            UiMessage::ToggleAutoComments(v) => {
                self.ui.settings.performance.enable_comments = v;
                self.ui.comments_visible = v;
                self.ui.settings.performance.profile = PerformanceProfileType::Custom;
                Task::none()
            }
            UiMessage::TogglePrivateMode(v) => {
                self.ui.private_mode = v;
                self.ui.settings.save_history = !v;
                Task::none()
            }
            UiMessage::ClearThumbnailCache => {
                self.cache.clear_l1();
                let _ = self.cache.clear_l2();
                self.ui.thumbnails.clear();
                self.ui.cache_size_mb = 0;
                self.ui.toast = Some(("Thumbnail cache cleared".to_string(), false));
                Task::none()
            }
            UiMessage::SaveSettings => {
                self.settings_mgr.set(self.ui.settings.clone());
                match self.settings_mgr.sync_to_storage(&self.storage) {
                    Ok(_) => self.ui.toast = Some(("Settings saved".to_string(), false)),
                    Err(e) => self.ui.toast = Some((e.to_string(), true)),
                }
                Task::none()
            }
            UiMessage::RefreshDiagnostics => {
                self.ui.diagnostics = Some(self.diagnostics.generate_report(
                    env!("CARGO_PKG_VERSION"),
                    &format!("{:?}", self.ui.settings.performance.profile),
                    &self.ui.settings.theme,
                    "Isolated WebView (wry + h264ify)",
                    &self.ui.hardware.graphics_backend,
                    self.ui.settings.h264ify_enabled,
                    self.ui.active_account.is_some(),
                ));
                self.ui.quota_general = Some(self.quota.get_status(QuotaPool::General));
                self.ui.quota_search = Some(self.quota.get_status(QuotaPool::Search));
                Task::none()
            }
            UiMessage::ExportDiagnosticReport => {
                if let Some(ref rep) = self.ui.diagnostics {
                    let path = self.paths.data_dir.join("fluxtube_diagnostics.md");
                    let _ = std::fs::write(&path, rep.to_markdown());
                    self.ui.toast = Some((format!("Report exported to {}", path.display()), false));
                }
                Task::none()
            }
            UiMessage::CheckForUpdates => {
                let http = self.http.clone();
                let url = "https://api.github.com/repos/prokalash-maker/fluxtube/releases".to_string();
                let client = UpdaterClient::new(http, url, [0u8; 32]);
                Task::perform(
                    async move {
                        client
                            .check_for_updates(env!("CARGO_PKG_VERSION"), UpdateChannel::Stable)
                            .await
                            .map_err(|e| e.to_string())
                    },
                    AppMessage::UpdateChecked,
                )
            }
            UiMessage::ToggleAccountModal(open) => {
                self.ui.show_account_modal = open;
                Task::none()
            }
            UiMessage::StartGoogleSignIn => {
                if std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default().is_empty() {
                    self.ui.toast = Some((
                        "Configure GOOGLE_CLIENT_ID in .env (see GOOGLE_OAUTH_SETUP.md)".to_string(),
                        true,
                    ));
                    return Task::none();
                }
                let auth = self.auth.clone();
                Task::perform(
                    async move { auth.start_pkce_login().await.map_err(|e| e.to_string()) },
                    AppMessage::AuthCompleted,
                )
            }
            UiMessage::SwitchToAccount(acc_opt) => {
                let _ = self.auth.switch_account(acc_opt.as_deref());
                self.ui.active_account = self.storage.get_active_account().unwrap_or(None);
                self.ui.show_account_modal = false;
                Task::none()
            }
            UiMessage::SignOutAccount(acc_id) => {
                let _ = self.vault.delete_token(&acc_id);
                let _ = self.storage.delete_account(&acc_id);
                self.ui.accounts = self.storage.get_all_accounts().unwrap_or_default();
                self.ui.active_account = self.storage.get_active_account().unwrap_or(None);
                Task::none()
            }
            UiMessage::DismissToast => {
                self.ui.toast = None;
                Task::none()
            }
            UiMessage::WindowResized(w, h) => {
                self.ui.window_width = w;
                self.ui.window_height = h;
                Task::none()
            }
        }
    }

    fn open_parsed_link(&mut self, link: YouTubeLink) -> Task<AppMessage> {
        match link {
            YouTubeLink::Video { id, timestamp_secs } => self.open_video_task(id, timestamp_secs),
            YouTubeLink::Channel(cid) => self.handle_ui_message(UiMessage::OpenChannel(cid)),
            YouTubeLink::Playlist(pid) => {
                self.ui.search_query = pid;
                self.handle_ui_message(UiMessage::SubmitSearch)
            }
        }
    }

    fn open_video_task(&self, video_id: String, override_secs: Option<u64>) -> Task<AppMessage> {
        let api = self.api.clone();
        let saved_pos = self
            .storage
            .get_position(&video_id)
            .ok()
            .flatten()
            .filter(|p| !p.is_finished())
            .map(|p| p.position_seconds)
            .unwrap_or(0);
        let resume_secs = override_secs.unwrap_or(saved_pos);
        let fallback_video = self
            .ui
            .home_videos
            .iter()
            .chain(self.ui.trending_videos.iter())
            .find(|v| v.id == video_id)
            .cloned();

        Task::perform(
            async move {
                let mut details = api
                    .get_video_details(&[video_id.as_str()])
                    .await
                    .unwrap_or_default();
                let video = details
                    .pop()
                    .or(fallback_video)
                    .ok_or_else(|| format!("Video '{}' not found", video_id))?;
                let comments = api
                    .get_comment_threads(&video.id, "relevance", None)
                    .await
                    .map(|p| p.items)
                    .unwrap_or_default();
                Ok(VideoOpenPayload {
                    video,
                    comments,
                    resume_secs,
                })
            },
            AppMessage::VideoOpened,
        )
    }

    pub fn view(&self) -> Element<'_, AppMessage> {
        render_app(&self.ui, &self.loc).map(AppMessage::Ui)
    }

    pub fn subscription(&self) -> Subscription<AppMessage> {
        let tick = if self.ui.player_playing {
            iced::time::every(Duration::from_secs(1)).map(|_| AppMessage::Tick)
        } else {
            Subscription::none()
        };

        let keys = keyboard::on_key_press(|key, mods| Some(AppMessage::KeyPressed(key, mods)));
        let resizes = window::resize_events().map(|(_id, size)| AppMessage::WindowResized(size.width, size.height));

        Subscription::batch([tick, keys, resizes])
    }
}
