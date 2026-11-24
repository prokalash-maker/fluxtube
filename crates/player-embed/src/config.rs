use fluxtube_core::PerformanceProfileType;

#[derive(Debug, Clone)]
pub struct EmbedPlayerConfig {
    pub h264ify_enabled: bool,
    pub profile: PerformanceProfileType,
    pub default_volume: f32,
    pub autoplay: bool,
}

impl Default for EmbedPlayerConfig {
    fn default() -> Self {
        Self {
            h264ify_enabled: true,
            profile: PerformanceProfileType::Balanced,
            default_volume: 0.8,
            autoplay: true,
        }
    }
}

impl EmbedPlayerConfig {
    pub fn new(profile: PerformanceProfileType) -> Self {
        Self {
            h264ify_enabled: true,
            profile,
            default_volume: 0.8,
            autoplay: true,
        }
    }
}
