use fluxtube_player::*;
use std::time::Duration;

#[test]
fn test_media_source_inspectors() {
    let yt = MediaSource::youtube("dQw4w9WgXcQ");
    assert!(yt.is_youtube());
    assert!(!yt.is_direct_stream());
    assert!(!yt.is_local_file());
    assert_eq!(yt.identifier(), "dQw4w9WgXcQ");

    let stream = MediaSource::direct_url("https://example.com/live.m3u8");
    assert!(!stream.is_youtube());
    assert!(stream.is_direct_stream());
    assert_eq!(stream.identifier(), "https://example.com/live.m3u8");

    let local = MediaSource::local("videos/clip.mp4");
    assert!(local.is_local_file());
    assert_eq!(local.identifier(), "videos/clip.mp4");
}

#[test]
fn test_video_quality_conversion() {
    assert_eq!(VideoQuality::Auto.as_youtube_str(), "auto");
    assert_eq!(VideoQuality::HighDefinition1080p.as_youtube_str(), "hd1080");
    assert_eq!(VideoQuality::HighDefinition720p.as_youtube_str(), "hd720");
    assert_eq!(VideoQuality::Large480p.as_youtube_str(), "large");
    assert_eq!(VideoQuality::Medium360p.as_youtube_str(), "medium");
    assert_eq!(VideoQuality::Small240p.as_youtube_str(), "small");
    assert_eq!(VideoQuality::Tiny144p.as_youtube_str(), "tiny");

    assert_eq!(VideoQuality::from_youtube_str("hd1080"), VideoQuality::HighDefinition1080p);
    assert_eq!(VideoQuality::from_youtube_str("medium"), VideoQuality::Medium360p);
    assert_eq!(VideoQuality::from_youtube_str("4k"), VideoQuality::Custom("4k".to_string()));
}

#[test]
fn test_playback_capabilities_presets() {
    let embed = PlaybackCapabilities::embed_defaults();
    assert!(embed.supports_youtube_embed);
    assert!(!embed.supports_local_files);
    assert!(embed.hardware_accelerated);

    let native = PlaybackCapabilities::native_defaults();
    assert!(!native.supports_youtube_embed);
    assert!(native.supports_local_files);
    assert!(native.supports_direct_streams);
}

#[test]
fn test_playback_context_volume_and_rate_clamping() {
    let mut ctx = PlaybackContext::new();
    assert_eq!(ctx.volume, 1.0);
    assert!(!ctx.muted);

    ctx.set_volume(1.5);
    assert_eq!(ctx.volume, 1.0);

    ctx.set_volume(-0.2);
    assert_eq!(ctx.volume, 0.0);

    ctx.set_volume(0.75);
    assert_eq!(ctx.volume, 0.75);
    assert_eq!(ctx.effective_volume(), 0.75);

    ctx.toggle_mute();
    assert!(ctx.muted);
    assert_eq!(ctx.effective_volume(), 0.0);

    ctx.set_rate(5.0);
    assert_eq!(ctx.rate, 2.0);

    ctx.set_rate(0.1);
    assert_eq!(ctx.rate, 0.25);

    ctx.set_rate(1.25);
    assert_eq!(ctx.rate, 1.25);
}

#[test]
fn test_playback_context_seek_bounds() {
    let mut ctx = PlaybackContext::new();
    ctx.duration = Duration::from_secs(100);

    ctx.seek(Duration::from_secs(50));
    assert_eq!(ctx.position, Duration::from_secs(50));
    assert_eq!(ctx.progress_ratio(), 0.5);

    ctx.seek(Duration::from_secs(150));
    assert_eq!(ctx.position, Duration::from_secs(100));
    assert_eq!(ctx.progress_ratio(), 1.0);

    ctx.seek_by(Duration::from_secs(20), false);
    assert_eq!(ctx.position, Duration::from_secs(80));

    ctx.seek_by(Duration::from_secs(10), true);
    assert_eq!(ctx.position, Duration::from_secs(90));
}

#[tokio::test]
async fn test_player_event_broadcast_fanout() {
    let bus = PlayerEventBus::new(32);
    let mut rx1 = bus.subscribe();
    let mut rx2 = bus.subscribe();

    bus.emit(PlayerEvent::StateChanged(PlaybackState::Playing));
    bus.emit(PlayerEvent::VolumeChanged { volume: 0.5, muted: false });

    match rx1.recv().await.unwrap() {
        PlayerEvent::StateChanged(s) => assert_eq!(s, PlaybackState::Playing),
        other => panic!("Unexpected event: {:?}", other),
    }

    match rx2.recv().await.unwrap() {
        PlayerEvent::StateChanged(s) => assert_eq!(s, PlaybackState::Playing),
        other => panic!("Unexpected event: {:?}", other),
    }

    match rx1.recv().await.unwrap() {
        PlayerEvent::VolumeChanged { volume, muted } => {
            assert_eq!(volume, 0.5);
            assert!(!muted);
        }
        other => panic!("Unexpected event: {:?}", other),
    }
}

#[tokio::test]
async fn test_mock_player_backend_lifecycle() {
    let player = MockPlayerBackend::new();
    let mut rx = player.subscribe_events();

    assert_eq!(player.backend_id(), "mock-player");
    assert_eq!(player.context().state, PlaybackState::Idle);

    player.load(MediaSource::youtube("test_123"), true, Some(Duration::from_secs(10))).await.unwrap();

    let event1 = rx.recv().await.unwrap();
    match event1 {
        PlayerEvent::MediaLoaded { source, .. } => assert_eq!(source, MediaSource::youtube("test_123")),
        other => panic!("Unexpected event: {:?}", other),
    }

    let event2 = rx.recv().await.unwrap();
    match event2 {
        PlayerEvent::StateChanged(s) => assert_eq!(s, PlaybackState::Playing),
        other => panic!("Unexpected event: {:?}", other),
    }

    player.pause().await.unwrap();
    assert_eq!(player.context().state, PlaybackState::Paused);

    player.seek(Duration::from_secs(45)).await.unwrap();
    assert_eq!(player.context().position, Duration::from_secs(45));

    player.set_volume(0.8).await.unwrap();
    assert_eq!(player.context().volume, 0.8);

    player.set_view_mode(ViewMode::Theater).await.unwrap();
    assert_eq!(player.context().view_mode, ViewMode::Theater);

    player.update_bounds(10, 20, 800, 600).unwrap();
    assert_eq!(player.context().bounds, PlayerBounds::new(10, 20, 800, 600));

    player.unload().await.unwrap();
    assert_eq!(player.context().state, PlaybackState::Idle);
    assert_eq!(player.context().source, None);
}
