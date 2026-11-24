use std::time::Duration;
use tokio::sync::broadcast;
use crate::capabilities::PlaybackCapabilities;
use crate::error::PlayerError;
use crate::events::PlayerEvent;
use crate::source::MediaSource;
use crate::types::ViewMode;

#[async_trait::async_trait]
pub trait PlayerBackend: Send + Sync {
    fn backend_id(&self) -> &'static str;
    fn capabilities(&self) -> PlaybackCapabilities;
    fn subscribe_events(&self) -> broadcast::Receiver<PlayerEvent>;
    async fn load(&self, source: MediaSource, autoplay: bool, start_at: Option<Duration>) -> Result<(), PlayerError>;
    async fn unload(&self) -> Result<(), PlayerError>;
    async fn play(&self) -> Result<(), PlayerError>;
    async fn pause(&self) -> Result<(), PlayerError>;
    async fn toggle_play(&self) -> Result<(), PlayerError>;
    async fn seek(&self, position: Duration) -> Result<(), PlayerError>;
    async fn seek_by(&self, offset: Duration, forward: bool) -> Result<(), PlayerError>;
    async fn set_volume(&self, volume: f32) -> Result<(), PlayerError>;
    async fn set_muted(&self, muted: bool) -> Result<(), PlayerError>;
    async fn toggle_mute(&self) -> Result<(), PlayerError>;
    async fn set_playback_rate(&self, rate: f32) -> Result<(), PlayerError>;
    async fn set_subtitle_track(&self, track_id: Option<&str>) -> Result<(), PlayerError>;
    async fn set_view_mode(&self, mode: ViewMode) -> Result<(), PlayerError>;
    fn update_bounds(&self, x: i32, y: i32, width: u32, height: u32) -> Result<(), PlayerError>;
    fn set_visible(&self, visible: bool) -> Result<(), PlayerError>;
}
