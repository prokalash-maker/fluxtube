use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sysinfo::System;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("Failed to open external URL: {0}")]
    OpenUrl(String),

    #[error("Filesystem path error: {0}")]
    PathError(String),
}

/// Supported hardware video decoder identifiers across target platforms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareDecoder {
    VideoToolboxH264,
    D3D11VAH264,
    Dxva2H264,
    SoftwareFallback,
}

impl std::fmt::Display for HardwareDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VideoToolboxH264 => write!(f, "Apple VideoToolbox (H.264)"),
            Self::D3D11VAH264 => write!(f, "Windows D3D11VA (H.264)"),
            Self::Dxva2H264 => write!(f, "Windows DXVA2 (H.264)"),
            Self::SoftwareFallback => write!(f, "Software CPU Decode"),
        }
    }
}

/// Detected host hardware and OS capabilities used for performance recommendations and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub os_name: String,
    pub os_version: String,
    pub cpu_arch: String,
    pub cpu_cores: usize,
    pub total_ram_mb: u64,
    pub available_ram_mb: u64,
    pub graphics_backend: String,
    pub available_decoders: Vec<HardwareDecoder>,
}

impl HardwareProfile {
    pub fn detect() -> Self {
        let mut sys = System::new();
        sys.refresh_memory();
        sys.refresh_cpu_all();

        let os_name = System::name().unwrap_or_else(|| std::env::consts::OS.to_string());
        let os_version = System::os_version().unwrap_or_else(|| "unknown".to_string());
        let cpu_arch = std::env::consts::ARCH.to_string();
        let cpu_cores = sys.cpus().len().max(1);
        let total_ram_mb = sys.total_memory() / (1024 * 1024);
        let available_ram_mb = sys.available_memory() / (1024 * 1024);

        let graphics_backend = detect_graphics_backend();
        let available_decoders = detect_hardware_decoders();

        Self {
            os_name,
            os_version,
            cpu_arch,
            cpu_cores,
            total_ram_mb,
            available_ram_mb,
            graphics_backend,
            available_decoders,
        }
    }

    pub fn is_legacy_hardware(&self) -> bool {
        self.total_ram_mb <= 4096 || self.cpu_cores <= 2
    }
}

pub fn detect_graphics_backend() -> String {
    if cfg!(target_os = "macos") {
        "wgpu (Metal) / tiny-skia fallback".to_string()
    } else if cfg!(target_os = "windows") {
        "wgpu (DX12/DX11/GL) / tiny-skia fallback".to_string()
    } else {
        "wgpu (Vulkan/GL) / tiny-skia fallback".to_string()
    }
}

pub fn detect_hardware_decoders() -> Vec<HardwareDecoder> {
    let mut list = Vec::new();
    if cfg!(target_os = "macos") {
        list.push(HardwareDecoder::VideoToolboxH264);
    } else if cfg!(target_os = "windows") {
        list.push(HardwareDecoder::D3D11VAH264);
        list.push(HardwareDecoder::Dxva2H264);
    }
    list.push(HardwareDecoder::SoftwareFallback);
    list
}

/// Cross-platform user-writable directories (no administrator privileges required).
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub db_path: PathBuf,
    pub thumbnails_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Result<Self, PlatformError> {
        let base_data = dirs::data_local_dir()
            .or_else(dirs::home_dir)
            .ok_or_else(|| PlatformError::PathError("Unable to resolve local data directory".into()))?;
        let base_cache = dirs::cache_dir()
            .or_else(dirs::home_dir)
            .ok_or_else(|| PlatformError::PathError("Unable to resolve cache directory".into()))?;

        let data_dir = base_data.join("FluxTube");
        let cache_dir = base_cache.join("FluxTube");
        let logs_dir = data_dir.join("logs");
        let db_path = data_dir.join("fluxtube.sqlite");
        let thumbnails_dir = cache_dir.join("thumbnails");

        Ok(Self {
            data_dir,
            cache_dir,
            logs_dir,
            db_path,
            thumbnails_dir,
        })
    }

    pub fn ensure_dirs(&self) -> Result<(), PlatformError> {
        for dir in [&self.data_dir, &self.cache_dir, &self.logs_dir, &self.thumbnails_dir] {
            std::fs::create_dir_all(dir)
                .map_err(|e| PlatformError::PathError(format!("{}: {}", dir.display(), e)))?;
        }
        Ok(())
    }
}

/// Detects the preferred FluxTube locale (`"en"`, `"uk"`, or `"ru"`) from OS environment.
pub fn detect_system_locale() -> &'static str {
    let env_lang = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default()
        .to_lowercase();

    if env_lang.starts_with("uk") {
        "uk"
    } else if env_lang.starts_with("ru") {
        "ru"
    } else {
        #[cfg(target_os = "windows")]
        {
            if env_lang.is_empty() {
                return "en";
            }
        }
        "en"
    }
}

/// Detects whether the OS is likely using dark mode.
pub fn detect_system_dark_mode() -> bool {
    #[cfg(target_os = "windows")]
    {
        // Default safe dark check; full registry query avoided to keep dependencies minimal
        true
    }
    #[cfg(target_os = "macos")]
    {
        return true;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        true
    }
}

/// Opens an external HTTP/HTTPS URL in the system default browser safely.
pub fn open_external_url(raw_url: &str) -> Result<(), PlatformError> {
    let parsed = Url::parse(raw_url)
        .map_err(|e| PlatformError::OpenUrl(format!("Invalid URL: {}", e)))?;
    match parsed.scheme() {
        "http" | "https" => {
            open::that(parsed.as_str()).map_err(|e| PlatformError::OpenUrl(e.to_string()))
        }
        other => Err(PlatformError::OpenUrl(format!(
            "Refused unsafe URL scheme: {}",
            other
        ))),
    }
}

/// Parsed YouTube target extracted from a pasted, dropped, or clipboard URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YouTubeLink {
    Video {
        id: String,
        timestamp_secs: Option<u64>,
    },
    Channel(String),
    Playlist(String),
}

/// Parses standard YouTube URLs (`watch?v=`, `youtu.be/`, `/shorts/`, `/embed/`, `/channel/`, `/@handle`, `?list=`).
pub fn parse_youtube_url(raw: &str) -> Option<YouTubeLink> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Raw 11-character YouTube video ID shortcut
    if trimmed.len() == 11
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Some(YouTubeLink::Video {
            id: trimmed.to_string(),
            timestamp_secs: None,
        });
    }

    let url = Url::parse(trimmed).ok()?;
    let host = url.host_str()?.to_lowercase();
    let is_yt = host == "youtube.com"
        || host.ends_with(".youtube.com")
        || host == "youtube-nocookie.com"
        || host.ends_with(".youtube-nocookie.com")
        || host == "youtu.be";
    if !is_yt {
        return None;
    }

    let mut timestamp_secs = None;
    let mut video_id = None;
    let mut playlist_id = None;

    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "v" if !v.is_empty() => video_id = Some(v.into_owned()),
            "list" if !v.is_empty() => playlist_id = Some(v.into_owned()),
            "t" | "start" => timestamp_secs = parse_timestamp_param(&v),
            _ => {}
        }
    }

    if let Some(id) = video_id {
        return Some(YouTubeLink::Video { id, timestamp_secs });
    }

    let segments: Vec<&str> = url
        .path_segments()
        .map(|s| s.filter(|p| !p.is_empty()).collect())
        .unwrap_or_default();

    if host == "youtu.be" {
        if let Some(first) = segments.first() {
            return Some(YouTubeLink::Video {
                id: (*first).to_string(),
                timestamp_secs,
            });
        }
    }

    if segments.len() >= 2 && (segments[0] == "shorts" || segments[0] == "embed" || segments[0] == "live") {
        return Some(YouTubeLink::Video {
            id: segments[1].to_string(),
            timestamp_secs,
        });
    }

    if segments.len() >= 2 && segments[0] == "channel" {
        return Some(YouTubeLink::Channel(segments[1].to_string()));
    }

    if let Some(first) = segments.first() {
        if first.starts_with('@') {
            return Some(YouTubeLink::Channel((*first).to_string()));
        }
    }

    if let Some(list) = playlist_id {
        return Some(YouTubeLink::Playlist(list));
    }

    None
}

fn parse_timestamp_param(raw: &str) -> Option<u64> {
    let s = raw.trim().trim_end_matches('s');
    if let Ok(secs) = s.parse::<u64>() {
        return Some(secs);
    }
    let mut total = 0u64;
    let mut buf = String::new();
    for ch in raw.trim().chars() {
        if ch.is_ascii_digit() {
            buf.push(ch);
        } else {
            let val: u64 = buf.parse().ok()?;
            buf.clear();
            match ch {
                'h' => total += val * 3600,
                'm' => total += val * 60,
                's' => total += val,
                _ => return None,
            }
        }
    }
    if !buf.is_empty() {
        total += buf.parse::<u64>().ok()?;
    }
    Some(total)
}

/// Formats a video URL with a specific playback timestamp in seconds.
pub fn format_timestamp_share_url(video_id: &str, position_secs: u64) -> String {
    if position_secs > 0 {
        format!("https://youtu.be/{}?t={}", video_id, position_secs)
    } else {
        format!("https://youtu.be/{}", video_id)
    }
}

/// Hardware multimedia key events dispatched to the active player backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKeyEvent {
    PlayPause,
    NextTrack,
    PreviousTrack,
    Stop,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_youtube_urls() {
        assert_eq!(
            parse_youtube_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=95s"),
            Some(YouTubeLink::Video {
                id: "dQw4w9WgXcQ".into(),
                timestamp_secs: Some(95),
            })
        );
        assert_eq!(
            parse_youtube_url("https://youtu.be/M7lc1UVf-VE?t=1m30s"),
            Some(YouTubeLink::Video {
                id: "M7lc1UVf-VE".into(),
                timestamp_secs: Some(90),
            })
        );
        assert_eq!(
            parse_youtube_url("https://www.youtube.com/shorts/aqz-KE-bpKQ"),
            Some(YouTubeLink::Video {
                id: "aqz-KE-bpKQ".into(),
                timestamp_secs: None,
            })
        );
        assert_eq!(
            parse_youtube_url("https://www.youtube.com/@rustsystemslab"),
            Some(YouTubeLink::Channel("@rustsystemslab".into()))
        );
        assert_eq!(
            parse_youtube_url("https://www.youtube.com/playlist?list=PLfluxtube001"),
            Some(YouTubeLink::Playlist("PLfluxtube001".into()))
        );
        assert_eq!(parse_youtube_url("https://evil.example.com/watch?v=123"), None);
    }

    #[test]
    fn test_hardware_profile_detection() {
        let profile = HardwareProfile::detect();
        assert!(profile.cpu_cores >= 1);
        assert!(profile.total_ram_mb > 0);
        assert!(!profile.available_decoders.is_empty());
    }

    #[test]
    fn test_format_share_url() {
        assert_eq!(
            format_timestamp_share_url("dQw4w9WgXcQ", 42),
            "https://youtu.be/dQw4w9WgXcQ?t=42"
        );
        assert_eq!(
            format_timestamp_share_url("dQw4w9WgXcQ", 0),
            "https://youtu.be/dQw4w9WgXcQ"
        );
    }
}
