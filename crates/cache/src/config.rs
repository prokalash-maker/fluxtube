use fluxtube_core::PerformanceProfileType;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl DecodedImage {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self { width, height, rgba }
    }

    pub fn byte_size(&self) -> usize {
        std::mem::size_of::<Self>() + self.rgba.len()
    }
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub l1_capacity_bytes: usize,
    pub l2_capacity_bytes: u64,
    pub l2_dir: PathBuf,
    pub worker_threads: usize,
}

impl CacheConfig {
    pub fn new(l1_capacity_bytes: usize, l2_capacity_bytes: u64, l2_dir: PathBuf) -> Self {
        Self {
            l1_capacity_bytes,
            l2_capacity_bytes,
            l2_dir,
            worker_threads: 2,
        }
    }

    pub fn from_profile(profile: PerformanceProfileType, l2_dir: PathBuf) -> Self {
        match profile {
            PerformanceProfileType::UltraLite => Self {
                l1_capacity_bytes: 32 * 1024 * 1024,      // 32 MB L1
                l2_capacity_bytes: 128 * 1024 * 1024,     // 128 MB L2
                l2_dir,
                worker_threads: 1,
            },
            PerformanceProfileType::Balanced => Self {
                l1_capacity_bytes: 64 * 1024 * 1024,      // 64 MB L1
                l2_capacity_bytes: 512 * 1024 * 1024,     // 512 MB L2
                l2_dir,
                worker_threads: 2,
            },
            PerformanceProfileType::HighQuality => Self {
                l1_capacity_bytes: 128 * 1024 * 1024,     // 128 MB L1
                l2_capacity_bytes: 2 * 1024 * 1024 * 1024, // 2 GB L2
                l2_dir,
                worker_threads: 4,
            },
            PerformanceProfileType::Custom => Self::from_profile(PerformanceProfileType::Balanced, l2_dir),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub l1_entries: usize,
    pub l1_bytes: usize,
    pub l1_capacity_bytes: usize,
    pub l2_entries: usize,
    pub l2_bytes: u64,
    pub l2_capacity_bytes: u64,
}
