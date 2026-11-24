use std::time::Duration;
use fluxtube_player::{error::PlayerError, source::MediaSource};

#[async_trait::async_trait]
pub trait NativeEngine: Send + Sync {
    async fn load(&self, source: &MediaSource, autoplay: bool, start_at: Option<Duration>) -> Result<Duration, PlayerError>;
    async fn play(&self) -> Result<(), PlayerError>;
    async fn pause(&self) -> Result<(), PlayerError>;
    async fn seek(&self, position: Duration) -> Result<Duration, PlayerError>;
    async fn set_volume(&self, volume: f32) -> Result<(), PlayerError>;
    async fn set_rate(&self, rate: f32) -> Result<(), PlayerError>;
    async fn stop(&self) -> Result<(), PlayerError>;
}
