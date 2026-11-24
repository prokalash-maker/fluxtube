pub mod config;
pub mod controller;
pub mod ipc;
pub mod lifecycle;
pub mod shim;

pub use config::EmbedPlayerConfig;
pub use controller::EmbedController;
pub use ipc::{EmbedPlaybackState, IpcEvent, PlayerCommand};
pub use lifecycle::{LifecycleState, ProfileLifecyclePolicy};
pub use shim::{get_initialization_script, H264IFY_SHIM_JS};

use async_trait::async_trait;
use fluxtube_core::PlaybackState;
use fluxtube_player::{
    backend::PlayerBackend,
    capabilities::PlaybackCapabilities,
    error::PlayerError,
    events::PlayerEvent,
    source::MediaSource,
    state::PlaybackContext,
    types::{PlayerBounds, ViewMode},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};

pub const EMBED_HTML: &str = include_str!("../assets/player.html");

/// Translates `PlayerBounds` to `wry::Rect` using logical coordinates.
pub fn bounds_to_wry_rect(bounds: PlayerBounds) -> wry::Rect {
    wry::Rect {
        position: wry::dpi::Position::Logical(wry::dpi::LogicalPosition::new(
            bounds.x as f64,
            bounds.y as f64,
        )),
        size: wry::dpi::Size::Logical(wry::dpi::LogicalSize::new(
            bounds.width as f64,
            bounds.height as f64,
        )),
    }
}

pub struct EmbedPlayerBackend {
    controller: Arc<EmbedController>,
}

impl Default for EmbedPlayerBackend {
    fn default() -> Self {
        let (backend, _) = Self::new_with_channel(EmbedPlayerConfig::default());
        backend
    }
}

impl EmbedPlayerBackend {
    pub fn new(config: EmbedPlayerConfig) -> (Self, mpsc::UnboundedReceiver<PlayerCommand>) {
        Self::new_with_channel(config)
    }

    pub fn new_with_channel(
        config: EmbedPlayerConfig,
    ) -> (Self, mpsc::UnboundedReceiver<PlayerCommand>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let controller = Arc::new(EmbedController::new(config, tx));
        (Self { controller }, rx)
    }

    pub fn controller(&self) -> &Arc<EmbedController> {
        &self.controller
    }

    pub fn context(&self) -> PlaybackContext {
        self.controller.context.read().clone()
    }

    pub fn handle_ipc_event(&self, event: IpcEvent) {
        self.controller.handle_ipc_event(event);
    }
}

#[async_trait]
impl PlayerBackend for EmbedPlayerBackend {
    fn backend_id(&self) -> &'static str {
        "player-embed"
    }

    fn capabilities(&self) -> PlaybackCapabilities {
        PlaybackCapabilities::embed_defaults()
    }

    fn subscribe_events(&self) -> broadcast::Receiver<PlayerEvent> {
        self.controller.event_bus.subscribe()
    }

    async fn load(
        &self,
        source: MediaSource,
        autoplay: bool,
        start_at: Option<Duration>,
    ) -> Result<(), PlayerError> {
        let video_id = match &source {
            MediaSource::YouTubeVideoId(id) => id.clone(),
            _ => {
                return Err(PlayerError::UnsupportedSource {
                    backend: "player-embed".to_string(),
                    reason: "Only YouTubeVideoId is supported by player-embed".to_string(),
                });
            }
        };

        let start_at_secs = start_at.map(|d| d.as_secs_f64()).unwrap_or(0.0);
        let cmd = PlayerCommand::Load {
            video_id,
            autoplay,
            start_at_secs,
        };

        {
            let mut ctx = self.controller.context.write();
            ctx.source = Some(source.clone());
            ctx.state = PlaybackState::Buffering;
        }

        self.controller
            .command_tx
            .send(cmd)
            .map_err(|e| PlayerError::IpcError(e.to_string()))?;

        self.controller
            .event_bus
            .emit(PlayerEvent::StateChanged(PlaybackState::Buffering));
        self.controller.event_bus.emit(PlayerEvent::MediaLoaded {
            source,
            duration: Duration::ZERO,
        });

        Ok(())
    }

    async fn unload(&self) -> Result<(), PlayerError> {
        {
            let mut ctx = self.controller.context.write();
            ctx.source = None;
            ctx.state = PlaybackState::Idle;
            ctx.position = Duration::ZERO;
            ctx.duration = Duration::ZERO;
        }

        self.controller
            .command_tx
            .send(PlayerCommand::Unload)
            .map_err(|e| PlayerError::IpcError(e.to_string()))?;

        self.controller.event_bus.emit(PlayerEvent::MediaUnloaded);
        self.controller
            .event_bus
            .emit(PlayerEvent::StateChanged(PlaybackState::Idle));
        Ok(())
    }

    async fn play(&self) -> Result<(), PlayerError> {
        self.controller
            .command_tx
            .send(PlayerCommand::Play)
            .map_err(|e| PlayerError::IpcError(e.to_string()))
    }

    async fn pause(&self) -> Result<(), PlayerError> {
        self.controller
            .command_tx
            .send(PlayerCommand::Pause)
            .map_err(|e| PlayerError::IpcError(e.to_string()))
    }

    async fn toggle_play(&self) -> Result<(), PlayerError> {
        self.controller
            .command_tx
            .send(PlayerCommand::TogglePlay)
            .map_err(|e| PlayerError::IpcError(e.to_string()))
    }

    async fn seek(&self, position: Duration) -> Result<(), PlayerError> {
        let pos_secs = position.as_secs_f64();
        {
            let mut ctx = self.controller.context.write();
            ctx.seek(position);
        }
        self.controller
            .command_tx
            .send(PlayerCommand::Seek {
                position_secs: pos_secs,
            })
            .map_err(|e| PlayerError::IpcError(e.to_string()))
    }

    async fn seek_by(&self, offset: Duration, forward: bool) -> Result<(), PlayerError> {
        let offset_secs = offset.as_secs_f64();
        {
            let mut ctx = self.controller.context.write();
            ctx.seek_by(offset, forward);
        }
        self.controller
            .command_tx
            .send(PlayerCommand::SeekBy {
                offset_secs,
                forward,
            })
            .map_err(|e| PlayerError::IpcError(e.to_string()))
    }

    async fn set_volume(&self, volume: f32) -> Result<(), PlayerError> {
        let clamped = volume.clamp(0.0, 1.0);
        {
            let mut ctx = self.controller.context.write();
            ctx.set_volume(clamped);
        }
        self.controller
            .command_tx
            .send(PlayerCommand::SetVolume { volume: clamped })
            .map_err(|e| PlayerError::IpcError(e.to_string()))
    }

    async fn set_muted(&self, muted: bool) -> Result<(), PlayerError> {
        {
            let mut ctx = self.controller.context.write();
            ctx.muted = muted;
        }
        self.controller
            .command_tx
            .send(PlayerCommand::SetMuted { muted })
            .map_err(|e| PlayerError::IpcError(e.to_string()))
    }

    async fn toggle_mute(&self) -> Result<(), PlayerError> {
        {
            let mut ctx = self.controller.context.write();
            ctx.toggle_mute();
        }
        self.controller
            .command_tx
            .send(PlayerCommand::ToggleMute)
            .map_err(|e| PlayerError::IpcError(e.to_string()))
    }

    async fn set_playback_rate(&self, rate: f32) -> Result<(), PlayerError> {
        let clamped = rate.clamp(0.25, 2.0);
        {
            let mut ctx = self.controller.context.write();
            ctx.set_rate(clamped);
        }
        self.controller
            .command_tx
            .send(PlayerCommand::SetPlaybackRate { rate: clamped })
            .map_err(|e| PlayerError::IpcError(e.to_string()))
    }

    async fn set_subtitle_track(&self, _track_id: Option<&str>) -> Result<(), PlayerError> {
        Ok(())
    }

    async fn set_view_mode(&self, mode: ViewMode) -> Result<(), PlayerError> {
        {
            let mut ctx = self.controller.context.write();
            ctx.view_mode = mode;
        }
        self.controller
            .event_bus
            .emit(PlayerEvent::ViewModeChanged(mode));
        Ok(())
    }

    fn update_bounds(&self, x: i32, y: i32, width: u32, height: u32) -> Result<(), PlayerError> {
        let bounds = PlayerBounds {
            x,
            y,
            width,
            height,
        };
        {
            let mut ctx = self.controller.context.write();
            ctx.bounds = bounds;
        }
        Ok(())
    }

    fn set_visible(&self, visible: bool) -> Result<(), PlayerError> {
        {
            let mut ctx = self.controller.context.write();
            ctx.visible = visible;
        }
        self.controller.on_visibility_change(visible);
        Ok(())
    }
}
