use std::time::Duration;
use tempfile::NamedTempFile;
use fluxtube_core::PlaybackState;
use fluxtube_player::{
    backend::PlayerBackend,
    error::PlayerError,
    events::PlayerEvent,
    source::MediaSource,
};
use fluxtube_player_native::NativePlayerBackend;

#[tokio::test]
async fn test_native_player_rejects_youtube_video_id() {
    let player = NativePlayerBackend::new();
    let res = player.load(MediaSource::youtube("dQw4w9WgXcQ"), true, None).await;

    match res {
        Err(PlayerError::UnsupportedSource { backend, reason }) => {
            assert_eq!(backend, "player-native");
            assert!(reason.contains("YouTube"));
        }
        other => panic!("Expected UnsupportedSource error, got {:?}", other),
    }
}

#[tokio::test]
async fn test_native_player_local_file_not_found() {
    let player = NativePlayerBackend::new();
    let missing_path = std::path::PathBuf::from("nonexistent_video_file_12345.mp4");
    let res = player.load(MediaSource::local(missing_path), true, None).await;

    match res {
        Err(PlayerError::LoadFailed(msg)) => {
            assert!(msg.contains("does not exist"));
        }
        other => panic!("Expected LoadFailed error, got {:?}", other),
    }
}

#[tokio::test]
async fn test_native_player_local_file_ticker_lifecycle() {
    let temp_file = NamedTempFile::new().unwrap();
    let player = NativePlayerBackend::new();
    let mut rx = player.subscribe_events();

    player
        .load(MediaSource::local(temp_file.path()), true, None)
        .await
        .unwrap();

    let mut saw_loaded = false;
    let mut saw_playing = false;

    for _ in 0..5 {
        if let Ok(event) = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
            match event.unwrap() {
                PlayerEvent::MediaLoaded { .. } => saw_loaded = true,
                PlayerEvent::StateChanged(PlaybackState::Playing) => saw_playing = true,
                PlayerEvent::TimeUpdated { position, .. } => {
                    if position > Duration::ZERO {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    assert!(saw_loaded, "Should emit MediaLoaded");
    assert!(saw_playing, "Should emit StateChanged(Playing)");

    // Pause player
    player.pause().await.unwrap();
    assert_eq!(player.context().state, PlaybackState::Paused);

    // Seek player
    player.seek(Duration::from_secs(30)).await.unwrap();
    assert_eq!(player.context().position, Duration::from_secs(30));

    // Volume adjustment
    player.set_volume(0.6).await.unwrap();
    assert_eq!(player.context().volume, 0.6);

    // Mute toggle
    player.toggle_mute().await.unwrap();
    assert!(player.context().muted);

    // Unload player
    player.unload().await.unwrap();
    assert_eq!(player.context().state, PlaybackState::Idle);
    assert_eq!(player.context().source, None);
}

#[tokio::test]
async fn test_native_player_direct_stream_support() {
    let player = NativePlayerBackend::new();
    assert!(player.capabilities().supports_direct_streams);
    assert!(player.capabilities().supports_local_files);
    assert!(!player.capabilities().supports_youtube_embed);

    let res = player.load(MediaSource::direct_url("http://example.com/test.mp4"), false, None).await;
    assert!(res.is_ok());
    assert_eq!(player.context().state, PlaybackState::Paused);
    assert_eq!(player.context().duration, Duration::from_secs(300));
}
