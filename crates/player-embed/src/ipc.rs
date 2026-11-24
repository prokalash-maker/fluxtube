//! Bidirectional IPC protocol definitions for Rust <-> JavaScript communication.

use fluxtube_core::PlaybackState;
use fluxtube_player::error::PlayerError;
use serde::{Deserialize, Serialize};

/// Commands sent from Rust to JavaScript via evaluate_script.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload", rename_all = "snake_case")]
pub enum PlayerCommand {
    Load {
        video_id: String,
        autoplay: bool,
        start_at_secs: f64,
    },
    Unload,
    Play,
    Pause,
    TogglePlay,
    Seek {
        position_secs: f64,
    },
    SeekBy {
        offset_secs: f64,
        forward: bool,
    },
    SetVolume {
        volume: f32, // 0.0 ..= 1.0
    },
    SetMuted {
        muted: bool,
    },
    ToggleMute,
    SetPlaybackRate {
        rate: f32, // 0.25 ..= 2.0
    },
    SetPlaybackQuality {
        quality: String,
    },
}

impl PlayerCommand {
    /// Formats the command into an executable JavaScript expression.
    pub fn to_js_expression(&self) -> Result<String, serde_json::Error> {
        let json = serde_json::to_string(self)?;
        Ok(format!("window.FluxPlayer && window.FluxPlayer.dispatch({});", json))
    }
}

/// Events received from JavaScript via window.ipc.postMessage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum IpcEvent {
    Ready {
        duration: f64,
        available_rates: Vec<f32>,
        available_qualities: Vec<String>,
    },
    StateChanged {
        state: EmbedPlaybackState,
    },
    TimeUpdated {
        position: f64,
        duration: f64,
    },
    VolumeChanged {
        volume: f32,
        muted: bool,
    },
    RateChanged {
        rate: f32,
    },
    QualityChanged {
        quality: String,
    },
    Error {
        code: i32,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbedPlaybackState {
    Unstarted,
    Ended,
    Playing,
    Paused,
    Buffering,
    Cued,
}

impl EmbedPlaybackState {
    pub fn to_core_playback_state(&self) -> PlaybackState {
        match self {
            Self::Unstarted => PlaybackState::Idle,
            Self::Ended => PlaybackState::Ended,
            Self::Playing => PlaybackState::Playing,
            Self::Paused => PlaybackState::Paused,
            Self::Buffering | Self::Cued => PlaybackState::Buffering,
        }
    }
}

impl IpcEvent {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_player_error(&self) -> Option<PlayerError> {
        match self {
            Self::Error { code, message } => {
                Some(PlayerError::IpcError(format!("YouTube Embed error [{}]: {}", code, message)))
            }
            _ => None,
        }
    }
}
