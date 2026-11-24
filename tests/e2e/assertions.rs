//! Custom Assertion Helpers and Macros for FluxTube E2E Testing.

use crate::mock_client::parse_iso8601_duration;
use crate::models::{Channel, Video};

/// Asserts that a Video model has non-empty identifiers, titles, and valid duration.
pub fn assert_video_valid(video: &Video) {
    assert!(!video.id.is_empty(), "Video id must not be empty");
    assert!(!video.title.is_empty(), "Video title must not be empty");
    assert!(!video.channel_id.is_empty(), "Video channel_id must not be empty");
    assert!(!video.channel_title.is_empty(), "Video channel_title must not be empty");
}

/// Asserts that a Channel model has valid uploads playlist ID (starting with "UU" for "UC" channels).
pub fn assert_channel_valid(channel: &Channel) {
    assert!(!channel.id.is_empty(), "Channel id must not be empty");
    assert!(!channel.title.is_empty(), "Channel title must not be empty");
    if channel.id.starts_with("UC") {
        assert!(
            channel.uploads_playlist_id.starts_with("UU"),
            "Channel with UC id must derive uploads playlist with UU prefix, got: {}",
            channel.uploads_playlist_id
        );
    }
}

/// Asserts that an ISO 8601 duration string parses to the expected number of seconds.
pub fn assert_iso8601_duration(raw: &str, expected_secs: u64) {
    let parsed = parse_iso8601_duration(raw);
    assert_eq!(
        parsed.as_secs(),
        expected_secs,
        "Failed parsing duration '{raw}': expected {expected_secs}s, got {}s",
        parsed.as_secs()
    );
}

/// Asserts that a floating point value is within tolerance epsilon of expected.
pub fn assert_within_tolerance(actual: f64, expected: f64, epsilon: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= epsilon,
        "Value {actual} not within tolerance {epsilon} of expected {expected} (diff: {diff})"
    );
}

/// Helper macro asserting that a string contains a substring.
#[macro_export]
macro_rules! assert_contains {
    ($haystack:expr, $needle:expr) => {
        assert!(
            $haystack.contains($needle),
            "Expected {:?} to contain {:?}",
            $haystack,
            $needle
        );
    };
}

/// Helper macro asserting duration equals expected seconds.
#[macro_export]
macro_rules! assert_duration_eq {
    ($actual:expr, $expected_secs:expr) => {
        assert_eq!(
            $actual.as_secs(),
            $expected_secs,
            "Duration mismatch: expected {}s, got {}s",
            $expected_secs,
            $actual.as_secs()
        );
    };
}
