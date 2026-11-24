use crate::config::{DecodedImage, ImageDimensions};
use crate::error::CacheError;
use image::imageops::FilterType;
use image::GenericImageView;
use std::sync::Arc;

pub struct ThumbnailPipeline {
    pool: Arc<rayon::ThreadPool>,
}

impl ThumbnailPipeline {
    pub fn new(threads: usize) -> Result<Self, CacheError> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads.max(1))
            .thread_name(|i| format!("fluxtube-cache-{}", i))
            .build()
            .map_err(|e| CacheError::WorkerPanic(e.to_string()))?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    pub async fn decode_and_resize(
        &self,
        raw_bytes: Vec<u8>,
        target_dims: Option<ImageDimensions>,
    ) -> Result<Arc<DecodedImage>, CacheError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pool.spawn(move || {
            let res = decode_and_resize_blocking(&raw_bytes, target_dims);
            let _ = tx.send(res);
        });

        rx.await
            .map_err(|_| CacheError::WorkerPanic("Worker thread dropped channel".to_string()))?
    }
}

fn decode_and_resize_blocking(
    raw_bytes: &[u8],
    target_dims: Option<ImageDimensions>,
) -> Result<Arc<DecodedImage>, CacheError> {
    let img = image::load_from_memory(raw_bytes)
        .map_err(|e| CacheError::Decode(e.to_string()))?;

    let final_img = match target_dims {
        Some(dims) if dims.width > 0 && dims.height > 0 => {
            // Triangle filter provides optimal bilinear SIMD performance
            img.resize(dims.width, dims.height, FilterType::Triangle)
        }
        _ => img,
    };

    let (width, height) = final_img.dimensions();
    let rgba = final_img.to_rgba8().into_raw();

    Ok(Arc::new(DecodedImage::new(width, height, rgba)))
}
