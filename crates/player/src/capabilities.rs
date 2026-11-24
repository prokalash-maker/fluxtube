use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaybackCapabilities {
    pub can_change_rate: bool,
    pub can_change_volume: bool,
    pub can_seek: bool,
    pub can_select_quality: bool,
    pub can_select_subtitles: bool,
    pub supports_pip: bool,
    pub supports_fullscreen: bool,
    pub supports_view_modes: bool,
    pub supports_local_files: bool,
    pub supports_direct_streams: bool,
    pub supports_youtube_embed: bool,
    pub hardware_accelerated: bool,
}

impl PlaybackCapabilities {
    pub fn embed_defaults() -> Self {
        Self {
            can_change_rate: true,
            can_change_volume: true,
            can_seek: true,
            can_select_quality: true,
            can_select_subtitles: true,
            supports_pip: true,
            supports_fullscreen: true,
            supports_view_modes: true,
            supports_local_files: false,
            supports_direct_streams: false,
            supports_youtube_embed: true,
            hardware_accelerated: true,
        }
    }

    pub fn native_defaults() -> Self {
        Self {
            can_change_rate: true,
            can_change_volume: true,
            can_seek: true,
            can_select_quality: false,
            can_select_subtitles: false,
            supports_pip: false,
            supports_fullscreen: true,
            supports_view_modes: true,
            supports_local_files: true,
            supports_direct_streams: true,
            supports_youtube_embed: false,
            hardware_accelerated: false,
        }
    }
}
