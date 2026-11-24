use std::sync::Arc;
use std::time::Duration;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use fluxtube_core::PlaybackState;
use crate::backend::PlayerBackend;
use crate::capabilities::PlaybackCapabilities;
use crate::error::PlayerError;
use crate::events::{PlayerEvent, PlayerEventBus};
use crate::source::MediaSource;
use crate::state::PlaybackContext;
use crate::types::ViewMode;

#[derive(Debug, Clone, PartialEq)]
pub enum MockCall {
    Load { source: MediaSource, autoplay: bool, start_at: Option<Duration> },
    Unload,
    Play,
    Pause,
    TogglePlay,
    Seek(Duration),
    SeekBy { offset: Duration, forward: bool },
    SetVolume(f32),
    SetMuted(bool),
    ToggleMute,
    SetPlaybackRate(f32),
    SetSubtitleTrack(Option<String>),
    SetViewMode(ViewMode),
    UpdateBounds { x: i32, y: i32, width: u32, height: u32 },
    SetVisible(bool),
}

#[derive(Clone)]
pub struct MockPlayerBackend {
    pub context: Arc<RwLock<PlaybackContext>>,
    pub event_bus: PlayerEventBus,
    pub call_history: Arc<RwLock<Vec<MockCall>>>,
    pub capabilities: PlaybackCapabilities,
}

impl Default for MockPlayerBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MockPlayerBackend {
    pub fn new() -> Self {
        Self {
            context: Arc::new(RwLock::new(PlaybackContext::new())),
            event_bus: PlayerEventBus::default(),
            call_history: Arc::new(RwLock::new(Vec::new())),
            capabilities: PlaybackCapabilities::embed_defaults(),
        }
    }

    pub fn calls(&self) -> Vec<MockCall> {
        self.call_history.read().clone()
    }

    pub fn clear_calls(&self) {
        self.call_history.write().clear();
    }

    pub fn context(&self) -> PlaybackContext {
        self.context.read().clone()
    }

    pub fn set_duration(&self, duration: Duration) {
        let mut ctx = self.context.write();
        ctx.duration = duration;
    }
}

#[async_trait::async_trait]
impl PlayerBackend for MockPlayerBackend {
    fn backend_id(&self) -> &'static str {
        "mock-player"
    }

    fn capabilities(&self) -> PlaybackCapabilities {
        self.capabilities
    }

    fn subscribe_events(&self) -> broadcast::Receiver<PlayerEvent> {
        self.event_bus.subscribe()
    }

    async fn load(&self, source: MediaSource, autoplay: bool, start_at: Option<Duration>) -> Result<(), PlayerError> {
        self.call_history.write().push(MockCall::Load {
            source: source.clone(),
            autoplay,
            start_at,
        });

        {
            let mut ctx = self.context.write();
            ctx.source = Some(source.clone());
            ctx.position = start_at.unwrap_or(Duration::ZERO);
            ctx.duration = Duration::from_secs(180); // mock default 3m duration
            ctx.state = if autoplay {
                PlaybackState::Playing
            } else {
                PlaybackState::Paused
            };
        }

        self.event_bus.emit(PlayerEvent::MediaLoaded {
            source,
            duration: Duration::from_secs(180),
        });
        let state = self.context.read().state;
        self.event_bus.emit(PlayerEvent::StateChanged(state));
        Ok(())
    }

    async fn unload(&self) -> Result<(), PlayerError> {
        self.call_history.write().push(MockCall::Unload);
        {
            let mut ctx = self.context.write();
            ctx.source = None;
            ctx.state = PlaybackState::Idle;
            ctx.position = Duration::ZERO;
            ctx.duration = Duration::ZERO;
        }
        self.event_bus.emit(PlayerEvent::MediaUnloaded);
        self.event_bus.emit(PlayerEvent::StateChanged(PlaybackState::Idle));
        Ok(())
    }

    async fn play(&self) -> Result<(), PlayerError> {
        self.call_history.write().push(MockCall::Play);
        {
            let mut ctx = self.context.write();
            ctx.state = PlaybackState::Playing;
        }
        self.event_bus.emit(PlayerEvent::StateChanged(PlaybackState::Playing));
        Ok(())
    }

    async fn pause(&self) -> Result<(), PlayerError> {
        self.call_history.write().push(MockCall::Pause);
        {
            let mut ctx = self.context.write();
            ctx.state = PlaybackState::Paused;
        }
        self.event_bus.emit(PlayerEvent::StateChanged(PlaybackState::Paused));
        Ok(())
    }

    async fn toggle_play(&self) -> Result<(), PlayerError> {
        self.call_history.write().push(MockCall::TogglePlay);
        let next_state = {
            let mut ctx = self.context.write();
            if ctx.state == PlaybackState::Playing {
                ctx.state = PlaybackState::Paused;
            } else {
                ctx.state = PlaybackState::Playing;
            }
            ctx.state
        };
        self.event_bus.emit(PlayerEvent::StateChanged(next_state));
        Ok(())
    }

    async fn seek(&self, position: Duration) -> Result<(), PlayerError> {
        self.call_history.write().push(MockCall::Seek(position));
        let (pos, dur) = {
            let mut ctx = self.context.write();
            let p = ctx.seek(position);
            (p, ctx.duration)
        };
        self.event_bus.emit(PlayerEvent::TimeUpdated {
            position: pos,
            duration: dur,
        });
        Ok(())
    }

    async fn seek_by(&self, offset: Duration, forward: bool) -> Result<(), PlayerError> {
        self.call_history.write().push(MockCall::SeekBy { offset, forward });
        let (pos, dur) = {
            let mut ctx = self.context.write();
            let p = ctx.seek_by(offset, forward);
            (p, ctx.duration)
        };
        self.event_bus.emit(PlayerEvent::TimeUpdated {
            position: pos,
            duration: dur,
        });
        Ok(())
    }

    async fn set_volume(&self, volume: f32) -> Result<(), PlayerError> {
        self.call_history.write().push(MockCall::SetVolume(volume));
        let (vol, muted) = {
            let mut ctx = self.context.write();
            let v = ctx.set_volume(volume);
            (v, ctx.muted)
        };
        self.event_bus.emit(PlayerEvent::VolumeChanged { volume: vol, muted });
        Ok(())
    }

    async fn set_muted(&self, muted: bool) -> Result<(), PlayerError> {
        self.call_history.write().push(MockCall::SetMuted(muted));
        let vol = {
            let mut ctx = self.context.write();
            ctx.muted = muted;
            ctx.volume
        };
        self.event_bus.emit(PlayerEvent::VolumeChanged { volume: vol, muted });
        Ok(())
    }

    async fn toggle_mute(&self) -> Result<(), PlayerError> {
        self.call_history.write().push(MockCall::ToggleMute);
        let (vol, muted) = {
            let mut ctx = self.context.write();
            let m = ctx.toggle_mute();
            (ctx.volume, m)
        };
        self.event_bus.emit(PlayerEvent::VolumeChanged { volume: vol, muted });
        Ok(())
    }

    async fn set_playback_rate(&self, rate: f32) -> Result<(), PlayerError> {
        self.call_history.write().push(MockCall::SetPlaybackRate(rate));
        let r = {
            let mut ctx = self.context.write();
            ctx.set_rate(rate)
        };
        self.event_bus.emit(PlayerEvent::RateChanged(r));
        Ok(())
    }

    async fn set_subtitle_track(&self, track_id: Option<&str>) -> Result<(), PlayerError> {
        self.call_history.write().push(MockCall::SetSubtitleTrack(track_id.map(str::to_string)));
        {
            let mut ctx = self.context.write();
            ctx.active_subtitle = track_id.map(str::to_string);
        }
        self.event_bus.emit(PlayerEvent::SubtitleTrackChanged(track_id.map(str::to_string)));
        Ok(())
    }

    async fn set_view_mode(&self, mode: ViewMode) -> Result<(), PlayerError> {
        self.call_history.write().push(MockCall::SetViewMode(mode));
        {
            let mut ctx = self.context.write();
            ctx.view_mode = mode;
        }
        self.event_bus.emit(PlayerEvent::ViewModeChanged(mode));
        Ok(())
    }

    fn update_bounds(&self, x: i32, y: i32, width: u32, height: u32) -> Result<(), PlayerError> {
        self.call_history.write().push(MockCall::UpdateBounds { x, y, width, height });
        let mut ctx = self.context.write();
        ctx.bounds.x = x;
        ctx.bounds.y = y;
        ctx.bounds.width = width;
        ctx.bounds.height = height;
        Ok(())
    }

    fn set_visible(&self, visible: bool) -> Result<(), PlayerError> {
        self.call_history.write().push(MockCall::SetVisible(visible));
        let mut ctx = self.context.write();
        ctx.visible = visible;
        Ok(())
    }
}
