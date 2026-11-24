use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use fluxtube_core::PlaybackState;
use fluxtube_player::{
    error::PlayerError,
    events::{PlayerEvent, PlayerEventBus},
    source::MediaSource,
    state::PlaybackContext,
};

pub struct NativePlayerStub {
    context: Arc<RwLock<PlaybackContext>>,
    event_bus: PlayerEventBus,
    ticker_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    is_running: Arc<AtomicBool>,
}

impl Default for NativePlayerStub {
    fn default() -> Self {
        Self::new()
    }
}

impl NativePlayerStub {
    pub fn new() -> Self {
        Self {
            context: Arc::new(RwLock::new(PlaybackContext::new())),
            event_bus: PlayerEventBus::default(),
            ticker_handle: Arc::new(RwLock::new(None)),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_event_bus(event_bus: PlayerEventBus) -> Self {
        Self {
            context: Arc::new(RwLock::new(PlaybackContext::new())),
            event_bus,
            ticker_handle: Arc::new(RwLock::new(None)),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn context(&self) -> PlaybackContext {
        self.context.read().clone()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<PlayerEvent> {
        self.event_bus.subscribe()
    }

    pub async fn load(
        &self,
        source: MediaSource,
        autoplay: bool,
        start_at: Option<Duration>,
    ) -> Result<(), PlayerError> {
        // Stop any running ticker
        self.stop_ticker().await;

        // 1. YouTube video rejection
        if source.is_youtube() {
            return Err(PlayerError::UnsupportedSource {
                backend: "player-native".to_string(),
                reason: "YouTube video playback must be handled by player-embed".to_string(),
            });
        }

        // 2. Local file validation
        let duration = match &source {
            MediaSource::LocalFile(path) => {
                if !path.exists() {
                    return Err(PlayerError::LoadFailed(format!(
                        "Local file does not exist: {}",
                        path.display()
                    )));
                }
                // Default 120s simulated duration for local files
                Duration::from_secs(120)
            }
            MediaSource::DirectStreamUrl(_) => {
                // Default 300s simulated duration for streams
                Duration::from_secs(300)
            }
            MediaSource::YouTubeVideoId(_) => unreachable!(),
        };

        {
            let mut ctx = self.context.write();
            ctx.source = Some(source.clone());
            ctx.duration = duration;
            ctx.position = start_at.unwrap_or(Duration::ZERO).min(duration);
            ctx.state = if autoplay {
                PlaybackState::Playing
            } else {
                PlaybackState::Paused
            };
        }

        self.event_bus.emit(PlayerEvent::MediaLoaded {
            source,
            duration,
        });

        if autoplay {
            self.event_bus.emit(PlayerEvent::StateChanged(PlaybackState::Playing));
            self.start_ticker();
        } else {
            self.event_bus.emit(PlayerEvent::StateChanged(PlaybackState::Paused));
        }

        Ok(())
    }

    pub async fn unload(&self) -> Result<(), PlayerError> {
        self.stop_ticker().await;
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

    pub async fn play(&self) -> Result<(), PlayerError> {
        let (has_source, current_state) = {
            let ctx = self.context.read();
            (ctx.source.is_some(), ctx.state)
        };

        if !has_source {
            return Err(PlayerError::PlaybackFailed("No media loaded".to_string()));
        }

        if current_state == PlaybackState::Playing {
            return Ok(());
        }

        {
            let mut ctx = self.context.write();
            // If ended, restart from beginning
            if ctx.state == PlaybackState::Ended {
                ctx.position = Duration::ZERO;
            }
            ctx.state = PlaybackState::Playing;
        }

        self.event_bus.emit(PlayerEvent::StateChanged(PlaybackState::Playing));
        self.start_ticker();
        Ok(())
    }

    pub async fn pause(&self) -> Result<(), PlayerError> {
        self.stop_ticker().await;
        {
            let mut ctx = self.context.write();
            ctx.state = PlaybackState::Paused;
        }
        self.event_bus.emit(PlayerEvent::StateChanged(PlaybackState::Paused));
        Ok(())
    }

    pub async fn toggle_play(&self) -> Result<(), PlayerError> {
        let state = self.context.read().state;
        if state == PlaybackState::Playing {
            self.pause().await
        } else {
            self.play().await
        }
    }

    pub async fn seek(&self, position: Duration) -> Result<(), PlayerError> {
        let (new_pos, dur) = {
            let mut ctx = self.context.write();
            let p = ctx.seek(position);
            (p, ctx.duration)
        };
        self.event_bus.emit(PlayerEvent::TimeUpdated {
            position: new_pos,
            duration: dur,
        });
        Ok(())
    }

    pub async fn seek_by(&self, offset: Duration, forward: bool) -> Result<(), PlayerError> {
        let (new_pos, dur) = {
            let mut ctx = self.context.write();
            let p = ctx.seek_by(offset, forward);
            (p, ctx.duration)
        };
        self.event_bus.emit(PlayerEvent::TimeUpdated {
            position: new_pos,
            duration: dur,
        });
        Ok(())
    }

    pub async fn set_volume(&self, volume: f32) -> Result<(), PlayerError> {
        let (vol, muted) = {
            let mut ctx = self.context.write();
            let v = ctx.set_volume(volume);
            (v, ctx.muted)
        };
        self.event_bus.emit(PlayerEvent::VolumeChanged { volume: vol, muted });
        Ok(())
    }

    pub async fn set_muted(&self, muted: bool) -> Result<(), PlayerError> {
        let vol = {
            let mut ctx = self.context.write();
            ctx.muted = muted;
            ctx.volume
        };
        self.event_bus.emit(PlayerEvent::VolumeChanged { volume: vol, muted });
        Ok(())
    }

    pub async fn toggle_mute(&self) -> Result<(), PlayerError> {
        let (vol, muted) = {
            let mut ctx = self.context.write();
            let m = ctx.toggle_mute();
            (ctx.volume, m)
        };
        self.event_bus.emit(PlayerEvent::VolumeChanged { volume: vol, muted });
        Ok(())
    }

    pub async fn set_playback_rate(&self, rate: f32) -> Result<(), PlayerError> {
        let r = {
            let mut ctx = self.context.write();
            ctx.set_rate(rate)
        };
        self.event_bus.emit(PlayerEvent::RateChanged(r));
        Ok(())
    }

    fn start_ticker(&self) {
        self.is_running.store(true, Ordering::SeqCst);
        let context = self.context.clone();
        let bus = self.event_bus.clone();
        let is_running = self.is_running.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            while is_running.load(Ordering::SeqCst) {
                interval.tick().await;

                let (ended, pos, dur) = {
                    let mut ctx = context.write();
                    if ctx.state != PlaybackState::Playing {
                        break;
                    }
                    let delta = Duration::from_millis(100).mul_f32(ctx.rate);
                    let new_pos = (ctx.position + delta).min(ctx.duration);
                    ctx.position = new_pos;
                    let ended = ctx.duration > Duration::ZERO && ctx.position >= ctx.duration;
                    if ended {
                        ctx.state = PlaybackState::Ended;
                    }
                    (ended, ctx.position, ctx.duration)
                };

                bus.emit(PlayerEvent::TimeUpdated {
                    position: pos,
                    duration: dur,
                });

                if ended {
                    bus.emit(PlayerEvent::StateChanged(PlaybackState::Ended));
                    is_running.store(false, Ordering::SeqCst);
                    break;
                }
            }
        });

        *self.ticker_handle.write() = Some(handle);
    }

    async fn stop_ticker(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        let handle = self.ticker_handle.write().take();
        if let Some(h) = handle {
            h.abort();
            let _ = h.await;
        }
    }
}
