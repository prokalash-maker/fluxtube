use fluxtube_core::{PerformanceProfileType, PlaybackState};
use fluxtube_player::{
    backend::PlayerBackend,
    error::PlayerError,
    events::PlayerEvent,
    source::MediaSource,
    types::{PlayerBounds, ViewMode},
};
use fluxtube_player_embed::*;
use std::time::Duration;

#[test]
fn test_player_command_serialization() {
    let cmd = PlayerCommand::Load {
        video_id: "dQw4w9WgXcQ".to_string(),
        autoplay: true,
        start_at_secs: 15.5,
    };
    let json = serde_json::to_string(&cmd).unwrap();
    assert!(json.contains("\"action\":\"load\""));
    assert!(json.contains("\"video_id\":\"dQw4w9WgXcQ\""));
    assert!(json.contains("\"autoplay\":true"));
    assert!(json.contains("\"start_at_secs\":15.5"));

    let js_expr = cmd.to_js_expression().unwrap();
    assert!(js_expr.starts_with("window.FluxPlayer && window.FluxPlayer.dispatch("));
    assert!(js_expr.ends_with(");"));
}

#[test]
fn test_ipc_event_deserialization_ready() {
    let json = r#"{
        "type": "ready",
        "data": {
            "duration": 212.5,
            "available_rates": [0.5, 1.0, 1.5, 2.0],
            "available_qualities": ["hd1080", "hd720", "medium"]
        }
    }"#;
    let event = IpcEvent::from_json(json).unwrap();
    match event {
        IpcEvent::Ready {
            duration,
            available_rates,
            available_qualities,
        } => {
            assert_eq!(duration, 212.5);
            assert_eq!(available_rates.len(), 4);
            assert_eq!(available_qualities, vec!["hd1080", "hd720", "medium"]);
        }
        other => panic!("Unexpected event: {:?}", other),
    }
}

#[test]
fn test_ipc_event_deserialization_state_changed() {
    let states = [
        ("playing", EmbedPlaybackState::Playing, PlaybackState::Playing),
        ("paused", EmbedPlaybackState::Paused, PlaybackState::Paused),
        ("buffering", EmbedPlaybackState::Buffering, PlaybackState::Buffering),
        ("ended", EmbedPlaybackState::Ended, PlaybackState::Ended),
        ("unstarted", EmbedPlaybackState::Unstarted, PlaybackState::Idle),
    ];

    for (str_val, embed_st, core_st) in states {
        let json = format!(r#"{{"type":"state_changed","data":{{"state":"{}"}}}}"#, str_val);
        let event = IpcEvent::from_json(&json).unwrap();
        match event {
            IpcEvent::StateChanged { state } => {
                assert_eq!(state, embed_st);
                assert_eq!(state.to_core_playback_state(), core_st);
            }
            other => panic!("Unexpected event: {:?}", other),
        }
    }
}

#[test]
fn test_ipc_event_deserialization_time_and_volume() {
    let time_json = r#"{"type":"time_updated","data":{"position":42.5,"duration":180.0}}"#;
    let event = IpcEvent::from_json(time_json).unwrap();
    match event {
        IpcEvent::TimeUpdated { position, duration } => {
            assert_eq!(position, 42.5);
            assert_eq!(duration, 180.0);
        }
        other => panic!("Unexpected event: {:?}", other),
    }

    let vol_json = r#"{"type":"volume_changed","data":{"volume":0.85,"muted":false}}"#;
    let event = IpcEvent::from_json(vol_json).unwrap();
    match event {
        IpcEvent::VolumeChanged { volume, muted } => {
            assert_eq!(volume, 0.85);
            assert!(!muted);
        }
        other => panic!("Unexpected event: {:?}", other),
    }
}

#[test]
fn test_ipc_event_deserialization_error_mapping() {
    let err_json = r#"{"type":"error","data":{"code":150,"message":"Embedding restricted"}}"#;
    let event = IpcEvent::from_json(err_json).unwrap();
    let player_err = event.to_player_error().unwrap();
    match player_err {
        PlayerError::IpcError(msg) => {
            assert!(msg.contains("150"));
            assert!(msg.contains("Embedding restricted"));
        }
        other => panic!("Unexpected error: {:?}", other),
    }
}

#[test]
fn test_h264ify_shim_content() {
    let script = get_initialization_script(true).unwrap();
    assert!(script.contains("BLOCKED_CODECS"));
    assert!(script.contains("vp8|vp9|vp09|av01|av1"));
    assert!(script.contains("video/webm"));
    assert!(script.contains("avc1"));

    assert!(get_initialization_script(false).is_none());
}

#[test]
fn test_profile_lifecycle_policy_transitions() {
    let ultra_policy = ProfileLifecyclePolicy::from(PerformanceProfileType::UltraLite);
    assert_eq!(ultra_policy, ProfileLifecyclePolicy::DestroyOnNavigateAway);

    let bal_policy = ProfileLifecyclePolicy::from(PerformanceProfileType::Balanced);
    assert_eq!(bal_policy, ProfileLifecyclePolicy::SuspendOnNavigateAway);

    let hq_policy = ProfileLifecyclePolicy::from(PerformanceProfileType::HighQuality);
    assert_eq!(hq_policy, ProfileLifecyclePolicy::KeepAlive);

    // Test controller visibility transition
    let (player, _rx) = EmbedPlayerBackend::new(EmbedPlayerConfig::new(PerformanceProfileType::UltraLite));
    player.update_bounds(0, 0, 1280, 720).unwrap();
    player.set_visible(false).unwrap();

    let lc = player.controller().lifecycle.read().clone();
    assert!(lc.is_destroyed());
    assert_eq!(lc.current_bounds(), PlayerBounds::new(0, 0, 1280, 720));
}

#[tokio::test]
async fn test_embed_player_backend_source_validation() {
    let (player, mut rx) = EmbedPlayerBackend::new(EmbedPlayerConfig::default());

    // Valid YouTube source
    let res = player
        .load(MediaSource::youtube("dQw4w9WgXcQ"), true, Some(Duration::from_secs(10)))
        .await;
    assert!(res.is_ok());

    let cmd = rx.recv().await.unwrap();
    assert_eq!(
        cmd,
        PlayerCommand::Load {
            video_id: "dQw4w9WgXcQ".to_string(),
            autoplay: true,
            start_at_secs: 10.0,
        }
    );

    // Invalid local file source
    let local_res = player.load(MediaSource::local("video.mp4"), true, None).await;
    match local_res {
        Err(PlayerError::UnsupportedSource { backend, reason }) => {
            assert_eq!(backend, "player-embed");
            assert!(reason.contains("YouTubeVideoId"));
        }
        other => panic!("Expected UnsupportedSource error, got {:?}", other),
    }
}

#[tokio::test]
async fn test_embed_player_backend_commands_and_events() {
    let (player, mut rx) = EmbedPlayerBackend::new(EmbedPlayerConfig::default());
    let mut event_rx = player.subscribe_events();

    player.play().await.unwrap();
    assert_eq!(rx.recv().await.unwrap(), PlayerCommand::Play);

    player.pause().await.unwrap();
    assert_eq!(rx.recv().await.unwrap(), PlayerCommand::Pause);

    player.seek(Duration::from_secs(120)).await.unwrap();
    assert_eq!(
        rx.recv().await.unwrap(),
        PlayerCommand::Seek {
            position_secs: 120.0
        }
    );

    player.set_volume(0.75).await.unwrap();
    assert_eq!(
        rx.recv().await.unwrap(),
        PlayerCommand::SetVolume { volume: 0.75 }
    );

    player.set_view_mode(ViewMode::Fullscreen).await.unwrap();
    let ev = event_rx.recv().await.unwrap();
    assert_eq!(ev, PlayerEvent::ViewModeChanged(ViewMode::Fullscreen));

    // Simulate IPC event from JS
    player.handle_ipc_event(IpcEvent::TimeUpdated {
        position: 45.0,
        duration: 300.0,
    });

    let ev = event_rx.recv().await.unwrap();
    assert_eq!(
        ev,
        PlayerEvent::TimeUpdated {
            position: Duration::from_secs(45),
            duration: Duration::from_secs(300),
        }
    );
    assert_eq!(player.context().position, Duration::from_secs(45));
}

#[test]
fn test_bounds_logical_to_rect_conversion() {
    let bounds = PlayerBounds::new(100, 200, 800, 450);
    let rect = bounds_to_wry_rect(bounds);
    match rect.position {
        wry::dpi::Position::Logical(pos) => {
            assert_eq!(pos.x, 100.0);
            assert_eq!(pos.y, 200.0);
        }
        _ => panic!("Expected logical position"),
    }
    match rect.size {
        wry::dpi::Size::Logical(sz) => {
            assert_eq!(sz.width, 800.0);
            assert_eq!(sz.height, 450.0);
        }
        _ => panic!("Expected logical size"),
    }
}
