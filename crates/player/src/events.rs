use std::time::Duration;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use fluxtube_core::PlaybackState;
use crate::error::PlayerError;
use crate::source::MediaSource;
use crate::types::{SubtitleTrack, ViewMode};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlayerEvent {
    StateChanged(PlaybackState),
    TimeUpdated { position: Duration, duration: Duration },
    VolumeChanged { volume: f32, muted: bool },
    RateChanged(f32),
    QualityChanged(String),
    SubtitleTracksAvailable(Vec<SubtitleTrack>),
    SubtitleTrackChanged(Option<String>),
    ViewModeChanged(ViewMode),
    MediaLoaded { source: MediaSource, duration: Duration },
    MediaUnloaded,
    Error(PlayerError),
}

#[derive(Debug, Clone)]
pub struct PlayerEventBus {
    sender: broadcast::Sender<PlayerEvent>,
}

impl Default for PlayerEventBus {
    fn default() -> Self {
        Self::new(128)
    }
}

impl PlayerEventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(16));
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<PlayerEvent> {
        self.sender.subscribe()
    }

    pub fn emit(&self, event: PlayerEvent) {
        let _ = self.sender.send(event);
    }

    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}
