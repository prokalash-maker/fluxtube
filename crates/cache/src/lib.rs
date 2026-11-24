pub mod config;
pub mod error;
pub mod fetcher;
pub mod l1;
pub mod l2;
pub mod pipeline;
pub mod singleflight;

pub use config::{CacheConfig, CacheStats, DecodedImage, ImageDimensions};
pub use error::CacheError;
pub use fetcher::{ImageFetcher, MockImageFetcher, ReqwestFetcher};

use l1::{L1Key, MemoryLruCache};
use l2::DiskL2Cache;
use pipeline::ThumbnailPipeline;
use singleflight::SingleFlight;

use parking_lot::Mutex;
use std::sync::Arc;

pub struct ThumbnailCache {
    config: CacheConfig,
    l1: Mutex<MemoryLruCache>,
    l2: Mutex<DiskL2Cache>,
    pipeline: ThumbnailPipeline,
    single_flight: SingleFlight,
    fetcher: Arc<dyn ImageFetcher>,
}

impl ThumbnailCache {
    pub fn new(config: CacheConfig) -> Result<Self, CacheError> {
        let fetcher = Arc::new(ReqwestFetcher::new());
        Self::with_fetcher(config, fetcher)
    }

    pub fn with_fetcher(
        config: CacheConfig,
        fetcher: Arc<dyn ImageFetcher>,
    ) -> Result<Self, CacheError> {
        let l1 = Mutex::new(MemoryLruCache::new(config.l1_capacity_bytes));
        let l2 = Mutex::new(DiskL2Cache::open(
            config.l2_dir.clone(),
            config.l2_capacity_bytes,
        )?);
        let pipeline = ThumbnailPipeline::new(config.worker_threads)?;
        let single_flight = SingleFlight::new();

        Ok(Self {
            config,
            l1,
            l2,
            pipeline,
            single_flight,
            fetcher,
        })
    }

    pub async fn get_or_fetch(
        &self,
        url: &str,
        target_dims: Option<ImageDimensions>,
    ) -> Result<Arc<DecodedImage>, CacheError> {
        let l1_key = L1Key {
            url: url.to_string(),
            dims: target_dims,
        };

        // 1. Fast Path: Check L1 In-Memory LRU Cache
        if let Some(cached) = self.l1.lock().get(&l1_key) {
            return Ok(cached);
        }

        // Deduplication key combines URL and dimensions to deduplicate identical tasks
        let flight_key = format!(
            "{}@{:?}",
            url,
            target_dims.map(|d| (d.width, d.height))
        );

        // 2. Slow Path with SingleFlight deduplication
        let result = self
            .single_flight
            .execute(&flight_key, || async {
                // Re-check L1 in case a concurrent task just finished it
                if let Some(cached) = self.l1.lock().get(&l1_key) {
                    return Ok(cached);
                }

                // Check L2 On-Disk Cache
                let raw_bytes = match self.l2.lock().get(url)? {
                    Some(bytes) => bytes,
                    None => {
                        // Cache Miss: Fetch over network via ImageFetcher
                        let downloaded = self.fetcher.fetch(url).await?;
                        // Save pristine raw bytes to L2 disk cache
                        let _ = self.l2.lock().put(url, &downloaded)?;
                        downloaded
                    }
                };

                // Off-thread Rayon SIMD decode & downscale
                let decoded = self.pipeline.decode_and_resize(raw_bytes, target_dims).await?;

                // Store decoded image in L1 memory cache
                self.l1.lock().put(l1_key.clone(), decoded.clone());

                Ok(decoded)
            })
            .await?;

        Ok(result)
    }

    pub fn clear_l1(&self) {
        self.l1.lock().clear();
    }

    pub fn clear_l2(&self) -> Result<(), CacheError> {
        self.l2.lock().clear()
    }

    pub fn stats(&self) -> CacheStats {
        let (l1_entries, l1_bytes, l1_cap) = self.l1.lock().stats();
        let (l2_entries, l2_bytes, l2_cap) = self.l2.lock().stats();
        CacheStats {
            l1_entries,
            l1_bytes,
            l1_capacity_bytes: l1_cap,
            l2_entries,
            l2_bytes,
            l2_capacity_bytes: l2_cap,
        }
    }

    pub fn config(&self) -> &CacheConfig {
        &self.config
    }
}
