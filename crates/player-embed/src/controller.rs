use crate::config::EmbedPlayerConfig;
use crate::ipc::{IpcEvent, PlayerCommand};
use crate::lifecycle::{LifecycleState, ProfileLifecyclePolicy};
use fluxtube_player::{
    error::PlayerError,
    events::{PlayerEvent, PlayerEventBus},
    state::PlaybackContext,
};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

pub struct EmbedController {
    pub config: EmbedPlayerConfig,
    pub context: Arc<RwLock<PlaybackContext>>,
    pub event_bus: PlayerEventBus,
    pub lifecycle: Arc<RwLock<LifecycleState>>,
    pub command_tx: mpsc::UnboundedSender<PlayerCommand>,
}

impl EmbedController {
    pub fn new(
        config: EmbedPlayerConfig,
        command_tx: mpsc::UnboundedSender<PlayerCommand>,
    ) -> Self {
        let event_bus = PlayerEventBus::default();
        let context = Arc::new(RwLock::new(PlaybackContext::new()));
        let lifecycle = Arc::new(RwLock::new(LifecycleState::Uninitialized));

        Self {
            config,
            context,
            event_bus,
            lifecycle,
            command_tx,
        }
    }

    pub fn handle_ipc_event(&self, event: IpcEvent) {
        match event {
            IpcEvent::Ready { duration, .. } => {
                let mut ctx = self.context.write();
                ctx.duration = Duration::from_secs_f64(duration.max(0.0));
            }
            IpcEvent::StateChanged { state } => {
                let core_state = state.to_core_playback_state();
                {
                    let mut ctx = self.context.write();
                    ctx.state = core_state;
                }
                self.event_bus.emit(PlayerEvent::StateChanged(core_state));
            }
            IpcEvent::TimeUpdated { position, duration } => {
                let pos = Duration::from_secs_f64(position.max(0.0));
                let dur = Duration::from_secs_f64(duration.max(0.0));
                {
                    let mut ctx = self.context.write();
                    ctx.position = pos;
                    if dur > Duration::ZERO {
                        ctx.duration = dur;
                    }
                }
                self.event_bus.emit(PlayerEvent::TimeUpdated {
                    position: pos,
                    duration: dur,
                });
            }
            IpcEvent::VolumeChanged { volume, muted } => {
                {
                    let mut ctx = self.context.write();
                    ctx.volume = volume;
                    ctx.muted = muted;
                }
                self.event_bus.emit(PlayerEvent::VolumeChanged { volume, muted });
            }
            IpcEvent::RateChanged { rate } => {
                {
                    let mut ctx = self.context.write();
                    ctx.rate = rate;
                }
                self.event_bus.emit(PlayerEvent::RateChanged(rate));
            }
            IpcEvent::QualityChanged { quality } => {
                self.event_bus.emit(PlayerEvent::QualityChanged(quality));
            }
            IpcEvent::Error { code, message } => {
                self.event_bus.emit(PlayerEvent::Error(PlayerError::IpcError(format!(
                    "Embed error [{}]: {}",
                    code, message
                ))));
            }
        }
    }

    pub fn on_visibility_change(&self, visible: bool) {
        let policy = ProfileLifecyclePolicy::from(self.config.profile);
        let mut lc = self.lifecycle.write();
        let bounds = self.context.read().bounds;

        if !visible {
            match policy {
                ProfileLifecyclePolicy::DestroyOnNavigateAway => {
                    let last_vid = self.context.read().source.as_ref().map(|s| s.identifier());
                    let last_pos = self.context.read().position;
                    *lc = LifecycleState::Destroyed {
                        last_video_id: last_vid,
                        last_position: last_pos,
                        bounds,
                    };
                }
                ProfileLifecyclePolicy::SuspendOnNavigateAway => {
                    let last_vid = self.context.read().source.as_ref().map(|s| s.identifier());
                    let last_pos = self.context.read().position;
                    *lc = LifecycleState::Suspended {
                        last_video_id: last_vid,
                        last_position: last_pos,
                        bounds,
                    };
                }
                ProfileLifecyclePolicy::KeepAlive => {}
            }
        } else {
            let cur_vid = self.context.read().source.as_ref().map(|s| s.identifier());
            *lc = LifecycleState::Active {
                current_video_id: cur_vid,
                bounds,
                visible: true,
            };
        }
    }
}
