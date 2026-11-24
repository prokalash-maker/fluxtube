use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PerformanceProfileType {
    UltraLite,
    Balanced,
    HighQuality,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerformanceSettings {
    pub profile: PerformanceProfileType,
    pub max_resolution: String, // "360p", "480p", "720p", "1080p", "auto"
    pub enable_animations: bool,
    pub enable_thumbnails_preload: bool,
    pub enable_comments: bool,
    pub cache_size_mb: u32,
    pub max_parallel_requests: usize,
    pub hardware_acceleration: bool,
}

impl PerformanceSettings {
    pub fn ultra_lite() -> Self {
        Self {
            profile: PerformanceProfileType::UltraLite,
            max_resolution: "480p".to_string(),
            enable_animations: false,
            enable_thumbnails_preload: false,
            enable_comments: false,
            cache_size_mb: 50,
            max_parallel_requests: 2,
            hardware_acceleration: false,
        }
    }

    pub fn balanced() -> Self {
        Self {
            profile: PerformanceProfileType::Balanced,
            max_resolution: "720p".to_string(),
            enable_animations: true,
            enable_thumbnails_preload: true,
            enable_comments: true,
            cache_size_mb: 200,
            max_parallel_requests: 4,
            hardware_acceleration: true,
        }
    }

    pub fn high_quality() -> Self {
        Self {
            profile: PerformanceProfileType::HighQuality,
            max_resolution: "1080p".to_string(),
            enable_animations: true,
            enable_thumbnails_preload: true,
            enable_comments: true,
            cache_size_mb: 500,
            max_parallel_requests: 8,
            hardware_acceleration: true,
        }
    }

    pub fn for_profile(profile: PerformanceProfileType) -> Self {
        match profile {
            PerformanceProfileType::UltraLite => Self::ultra_lite(),
            PerformanceProfileType::Balanced => Self::balanced(),
            PerformanceProfileType::HighQuality => Self::high_quality(),
            PerformanceProfileType::Custom => Self::balanced(),
        }
    }
}
