use std::time::Duration;
use fluxtube_core::PlaybackState;
use crate::source::MediaSource;
use crate::types::{PlayerBounds, SubtitleTrack, ViewMode};

#[derive(Debug, Clone, PartialEq)]
pub struct PlaybackContext {
    pub state: PlaybackState,
    pub source: Option<MediaSource>,
    pub position: Duration,
    pub duration: Duration,
    pub volume: f32,
    pub muted: bool,
    pub rate: f32,
    pub view_mode: ViewMode,
    pub bounds: PlayerBounds,
    pub visible: bool,
    pub active_subtitle: Option<String>,
    pub available_subtitles: Vec<SubtitleTrack>,
}

impl Default for PlaybackContext {
    fn default() -> Self {
        Self {
            state: PlaybackState::Idle,
            source: None,
            position: Duration::ZERO,
            duration: Duration::ZERO,
            volume: 1.0,
            muted: false,
            rate: 1.0,
            view_mode: ViewMode::Normal,
            bounds: PlayerBounds::default(),
            visible: true,
            active_subtitle: None,
            available_subtitles: Vec::new(),
        }
    }
}

impl PlaybackContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_volume(&mut self, volume: f32) -> f32 {
        self.volume = volume.clamp(0.0, 1.0);
        self.volume
    }

    pub fn toggle_mute(&mut self) -> bool {
        self.muted = !self.muted;
        self.muted
    }

    pub fn set_rate(&mut self, rate: f32) -> f32 {
        self.rate = rate.clamp(0.25, 2.0);
        self.rate
    }

    pub fn seek(&mut self, position: Duration) -> Duration {
        if self.duration > Duration::ZERO {
            self.position = position.min(self.duration);
        } else {
            self.position = position;
        }
        self.position
    }

    pub fn seek_by(&mut self, offset: Duration, forward: bool) -> Duration {
        if forward {
            self.seek(self.position.saturating_add(offset))
        } else {
            self.seek(self.position.saturating_sub(offset))
        }
    }

    pub fn progress_ratio(&self) -> f32 {
        if self.duration.as_secs_f32() > 0.0 {
            (self.position.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub fn effective_volume(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.volume
        }
    }
}
