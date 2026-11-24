pub mod backend;
pub mod capabilities;
pub mod error;
pub mod events;
pub mod mock;
pub mod source;
pub mod state;
pub mod types;

pub use backend::PlayerBackend;
pub use capabilities::PlaybackCapabilities;
pub use error::PlayerError;
pub use events::{PlayerEvent, PlayerEventBus};
pub use mock::MockPlayerBackend;
pub use source::MediaSource;
pub use state::PlaybackContext;
pub use types::{PlayerBounds, SubtitleTrack, VideoQuality, ViewMode};

// Re-export core PlaybackState for seamless integration across crates
pub use fluxtube_core::PlaybackState;
