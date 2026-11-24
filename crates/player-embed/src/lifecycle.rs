use fluxtube_core::PerformanceProfileType;
use fluxtube_player::types::PlayerBounds;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileLifecyclePolicy {
    /// Destroy WebView on navigate away / unload to recover ~200MB RAM.
    DestroyOnNavigateAway,
    /// Suspend/pause playback, hide WebView, keep in memory for zero resume latency.
    SuspendOnNavigateAway,
    /// Keep alive and warm.
    KeepAlive,
}

impl From<PerformanceProfileType> for ProfileLifecyclePolicy {
    fn from(profile: PerformanceProfileType) -> Self {
        match profile {
            PerformanceProfileType::UltraLite => Self::DestroyOnNavigateAway,
            PerformanceProfileType::Balanced | PerformanceProfileType::Custom => {
                Self::SuspendOnNavigateAway
            }
            PerformanceProfileType::HighQuality => Self::KeepAlive,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LifecycleState {
    Uninitialized,
    Active {
        current_video_id: Option<String>,
        bounds: PlayerBounds,
        visible: bool,
    },
    Suspended {
        last_video_id: Option<String>,
        last_position: Duration,
        bounds: PlayerBounds,
    },
    Destroyed {
        last_video_id: Option<String>,
        last_position: Duration,
        bounds: PlayerBounds,
    },
}

impl LifecycleState {
    pub fn current_bounds(&self) -> PlayerBounds {
        match self {
            Self::Uninitialized => PlayerBounds::default(),
            Self::Active { bounds, .. } => *bounds,
            Self::Suspended { bounds, .. } => *bounds,
            Self::Destroyed { bounds, .. } => *bounds,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active { .. })
    }

    pub fn is_destroyed(&self) -> bool {
        matches!(self, Self::Destroyed { .. })
    }

    pub fn is_suspended(&self) -> bool {
        matches!(self, Self::Suspended { .. })
    }
}
