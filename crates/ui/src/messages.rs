use fluxtube_core::{PerformanceProfileType, VideoRating};
use fluxtube_design_system::{DensityMode, ThemeKind};
use fluxtube_youtube_api::{SearchOrder, SearchResourceType, VideoDurationFilter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Route {
    Home,
    Trending,
    Subscriptions,
    Shorts,
    Live,
    History,
    WatchLater,
    Liked,
    Playlists,
    Favorites,
    Search,
    Watch,
    Channel,
    Settings,
    Diagnostics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelTab {
    Videos,
    Playlists,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Appearance,
    Performance,
    Privacy,
    Updates,
}

#[derive(Debug, Clone)]
pub enum UiMessage {
    // Navigation & Sidebar
    Navigate(Route),
    ToggleSidebar,
    PinSection(Route),
    HideSection(Route),
    MoveSectionUp(Route),
    MoveSectionDown(Route),
    ToggleCompactSubscriptions,

    // Search & URL input
    SearchInputChanged(String),
    SubmitSearch,
    SelectSearchSuggestion(String),
    ClearSearchHistory,
    SetSearchType(SearchResourceType),
    SetSearchDuration(VideoDurationFilter),
    SetSearchOrder(SearchOrder),
    OpenUrlOrClipboard,

    // Home & Feeds
    RefreshFeed,
    SelectHomeCategory(String),

    // Video playback & Watch page
    OpenVideo(String),
    OpenVideoAt(String, u64),
    PlayerTogglePlay,
    PlayerSeekRelative(i64),
    PlayerVolumeChanged(f32),
    PlayerToggleMute,
    PlayerRateChanged(f32),
    PlayerToggleTheater,
    PlayerToggleFullscreen,
    PlayerToggleMini,
    ToggleCommentsPanel,
    CommentDraftChanged(String),
    SubmitComment,
    RateCurrentVideo(VideoRating),
    ToggleWatchLater(String),
    AddToQueue(String),
    PlayNextInQueue,
    AddTimestampBookmark,
    CopyTimestampUrl,
    NoteDraftChanged(String),
    SaveVideoNote,
    OpenInExternalBrowser(String),

    // Channel view
    OpenChannel(String),
    SwitchChannelTab(ChannelTab),
    ToggleChannelSubscribe(String),
    ToggleChannelFavorite(String),
    ChannelFilterChanged(String),

    // Shorts & Live
    ShortsPrev,
    ShortsNext,
    ToggleShortsAutoplay(bool),
    ToggleHideShorts(bool),
    ToggleLiveChat,

    // Library & History
    CreateLocalPlaylistDraftChanged(String),
    CreateLocalPlaylist,
    ExportLocalPlaylists,
    ClearWatchHistory,

    // Settings & Performance Center
    SwitchSettingsTab(SettingsTab),
    SelectTheme(ThemeKind),
    AccentColorChanged(String),
    SelectDensity(DensityMode),
    UiScaleChanged(f32),
    SelectLocale(String),
    SelectPerformanceProfile(PerformanceProfileType),
    RecommendProfileForHardware,
    ToggleH264ify(bool),
    ToggleAnimations(bool),
    ToggleAutoComments(bool),
    TogglePrivateMode(bool),
    ClearThumbnailCache,
    SaveSettings,

    // Diagnostics & Updates
    RefreshDiagnostics,
    ExportDiagnosticReport,
    CheckForUpdates,

    // Auth & Accounts
    ToggleAccountModal(bool),
    StartGoogleSignIn,
    SwitchToAccount(Option<String>),
    SignOutAccount(String),

    // System / Window
    DismissToast,
    WindowResized(f32, f32),
}
