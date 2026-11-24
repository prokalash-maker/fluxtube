use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use fluxtube_player::{
    backend::PlayerBackend,
    capabilities::PlaybackCapabilities,
    error::PlayerError,
    events::PlayerEvent,
    source::MediaSource,
    state::PlaybackContext,
    types::ViewMode,
};
use crate::stub::NativePlayerStub;

pub struct NativePlayerBackend {
    stub: Arc<NativePlayerStub>,
}

impl Default for NativePlayerBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl NativePlayerBackend {
    pub fn new() -> Self {
        Self {
            stub: Arc::new(NativePlayerStub::new()),
        }
    }

    pub fn context(&self) -> PlaybackContext {
        self.stub.context()
    }
}

#[async_trait::async_trait]
impl PlayerBackend for NativePlayerBackend {
    fn backend_id(&self) -> &'static str {
        "player-native"
    }

    fn capabilities(&self) -> PlaybackCapabilities {
        PlaybackCapabilities::native_defaults()
    }

    fn subscribe_events(&self) -> broadcast::Receiver<PlayerEvent> {
        self.stub.subscribe_events()
    }

    async fn load(&self, source: MediaSource, autoplay: bool, start_at: Option<Duration>) -> Result<(), PlayerError> {
        self.stub.load(source, autoplay, start_at).await
    }

    async fn unload(&self) -> Result<(), PlayerError> {
        self.stub.unload().await
    }

    async fn play(&self) -> Result<(), PlayerError> {
        self.stub.play().await
    }

    async fn pause(&self) -> Result<(), PlayerError> {
        self.stub.pause().await
    }

    async fn toggle_play(&self) -> Result<(), PlayerError> {
        self.stub.toggle_play().await
    }

    async fn seek(&self, position: Duration) -> Result<(), PlayerError> {
        self.stub.seek(position).await
    }

    async fn seek_by(&self, offset: Duration, forward: bool) -> Result<(), PlayerError> {
        self.stub.seek_by(offset, forward).await
    }

    async fn set_volume(&self, volume: f32) -> Result<(), PlayerError> {
        self.stub.set_volume(volume).await
    }

    async fn set_muted(&self, muted: bool) -> Result<(), PlayerError> {
        self.stub.set_muted(muted).await
    }

    async fn toggle_mute(&self) -> Result<(), PlayerError> {
        self.stub.toggle_mute().await
    }

    async fn set_playback_rate(&self, rate: f32) -> Result<(), PlayerError> {
        self.stub.set_playback_rate(rate).await
    }

    async fn set_subtitle_track(&self, _track_id: Option<&str>) -> Result<(), PlayerError> {
        // Native stub does not provide subtitle tracks
        Ok(())
    }

    async fn set_view_mode(&self, _mode: ViewMode) -> Result<(), PlayerError> {
        let bus = self.stub.subscribe_events();
        drop(bus);
        Ok(())
    }

    fn update_bounds(&self, _x: i32, _y: i32, _width: u32, _height: u32) -> Result<(), PlayerError> {
        Ok(())
    }

    fn set_visible(&self, _visible: bool) -> Result<(), PlayerError> {
        Ok(())
    }
}
