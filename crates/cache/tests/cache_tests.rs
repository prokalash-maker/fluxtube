use fluxtube_cache::*;
use std::io::Cursor;
use std::sync::Arc;
use tempfile::tempdir;

fn make_test_png(w: u32, h: u32) -> Vec<u8> {
    let mut img = image::RgbaImage::new(w, h);
    for pixel in img.pixels_mut() {
        *pixel = image::Rgba([200, 100, 50, 255]);
    }
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

#[tokio::test]
async fn test_cache_miss_fetch_and_l1_l2_population() {
    let dir = tempdir().unwrap();
    let config = CacheConfig::new(1024 * 1024, 10 * 1024 * 1024, dir.path().to_path_buf());
    let fetcher = Arc::new(MockImageFetcher::new());
    fetcher.insert_fixture("http://example.com/img1.png", make_test_png(100, 100));

    let cache = ThumbnailCache::with_fetcher(config, fetcher.clone()).unwrap();
    let img = cache.get_or_fetch("http://example.com/img1.png", None).await.unwrap();

    assert_eq!(img.width, 100);
    assert_eq!(img.height, 100);
    assert_eq!(img.rgba.len(), 100 * 100 * 4);
    assert_eq!(fetcher.get_call_count("http://example.com/img1.png"), 1);

    // Second call must hit L1 without calling fetcher
    let img2 = cache.get_or_fetch("http://example.com/img1.png", None).await.unwrap();
    assert_eq!(fetcher.get_call_count("http://example.com/img1.png"), 1);
    assert!(Arc::ptr_eq(&img, &img2));
}

#[tokio::test]
async fn test_cache_downscaling_to_widget_dims() {
    let dir = tempdir().unwrap();
    let config = CacheConfig::new(1024 * 1024, 10 * 1024 * 1024, dir.path().to_path_buf());
    let fetcher = Arc::new(MockImageFetcher::new());
    fetcher.insert_fixture("http://example.com/big.png", make_test_png(400, 200));

    let cache = ThumbnailCache::with_fetcher(config, fetcher).unwrap();
    let target = Some(ImageDimensions { width: 200, height: 100 });
    let img = cache.get_or_fetch("http://example.com/big.png", target).await.unwrap();

    assert_eq!(img.width, 200);
    assert_eq!(img.height, 100);
    assert_eq!(img.rgba.len(), 200 * 100 * 4);
}

#[tokio::test]
async fn test_l1_byte_capacity_eviction() {
    let dir = tempdir().unwrap();
    // Capacity 30 KB
    let config = CacheConfig::new(30 * 1024, 10 * 1024 * 1024, dir.path().to_path_buf());
    let fetcher = Arc::new(MockImageFetcher::new());
    // 50x50 RGBA = 10 KB per image
    fetcher.insert_fixture("http://example.com/1.png", make_test_png(50, 50));
    fetcher.insert_fixture("http://example.com/2.png", make_test_png(50, 50));
    fetcher.insert_fixture("http://example.com/3.png", make_test_png(50, 50));
    fetcher.insert_fixture("http://example.com/4.png", make_test_png(50, 50));

    let cache = ThumbnailCache::with_fetcher(config, fetcher).unwrap();
    cache.get_or_fetch("http://example.com/1.png", None).await.unwrap();
    cache.get_or_fetch("http://example.com/2.png", None).await.unwrap();
    cache.get_or_fetch("http://example.com/3.png", None).await.unwrap();
    cache.get_or_fetch("http://example.com/4.png", None).await.unwrap();

    let stats = cache.stats();
    assert!(stats.l1_bytes <= 30 * 1024);
    assert!(stats.l1_entries <= 3);
}

#[tokio::test]
async fn test_singleflight_concurrent_deduplication() {
    let dir = tempdir().unwrap();
    let config = CacheConfig::new(1024 * 1024, 10 * 1024 * 1024, dir.path().to_path_buf());
    let fetcher = Arc::new(MockImageFetcher::new());
    fetcher.insert_fixture("http://example.com/shared.png", make_test_png(60, 60));

    let cache = Arc::new(ThumbnailCache::with_fetcher(config, fetcher.clone()).unwrap());

    // Spawn 10 concurrent requests for the exact same URL
    let mut handles = Vec::new();
    for _ in 0..10 {
        let c = cache.clone();
        handles.push(tokio::spawn(async move {
            c.get_or_fetch("http://example.com/shared.png", None).await
        }));
    }

    for h in handles {
        let res = h.await.unwrap().unwrap();
        assert_eq!(res.width, 60);
    }

    // Network fetch must have executed exactly once
    assert_eq!(fetcher.get_call_count("http://example.com/shared.png"), 1);
}

#[tokio::test]
async fn test_l2_disk_persistence_and_lru_eviction() {
    let dir = tempdir().unwrap();
    // Capacity 3 KB
    let config = CacheConfig::new(1024 * 1024, 3000, dir.path().to_path_buf());
    let fetcher = Arc::new(MockImageFetcher::new());
    fetcher.insert_fixture("http://example.com/a.png", make_test_png(30, 30));
    fetcher.insert_fixture("http://example.com/b.png", make_test_png(30, 30));
    fetcher.insert_fixture("http://example.com/c.png", make_test_png(30, 30));

    let cache = ThumbnailCache::with_fetcher(config, fetcher).unwrap();
    cache.get_or_fetch("http://example.com/a.png", None).await.unwrap();
    cache.get_or_fetch("http://example.com/b.png", None).await.unwrap();
    cache.get_or_fetch("http://example.com/c.png", None).await.unwrap();

    let stats = cache.stats();
    assert!(stats.l2_bytes <= 3000);
}

#[tokio::test]
async fn test_clear_l1_and_l2() {
    let dir = tempdir().unwrap();
    let config = CacheConfig::new(1024 * 1024, 10 * 1024 * 1024, dir.path().to_path_buf());
    let fetcher = Arc::new(MockImageFetcher::new());
    fetcher.insert_fixture("http://example.com/clear.png", make_test_png(40, 40));

    let cache = ThumbnailCache::with_fetcher(config, fetcher).unwrap();
    cache.get_or_fetch("http://example.com/clear.png", None).await.unwrap();

    assert!(cache.stats().l1_entries > 0);
    assert!(cache.stats().l2_entries > 0);

    cache.clear_l1();
    assert_eq!(cache.stats().l1_entries, 0);
    assert_eq!(cache.stats().l1_bytes, 0);

    cache.clear_l2().unwrap();
    assert_eq!(cache.stats().l2_entries, 0);
    assert_eq!(cache.stats().l2_bytes, 0);
}

#[tokio::test]
async fn test_invalid_image_bytes_decode_error() {
    let dir = tempdir().unwrap();
    let config = CacheConfig::new(1024 * 1024, 10 * 1024 * 1024, dir.path().to_path_buf());
    let fetcher = Arc::new(MockImageFetcher::new());
    fetcher.insert_fixture("http://example.com/corrupt.png", vec![1, 2, 3, 4, 5]);

    let cache = ThumbnailCache::with_fetcher(config, fetcher).unwrap();
    let err = cache.get_or_fetch("http://example.com/corrupt.png", None).await.unwrap_err();
    match err {
        CacheError::Decode(_) => {}
        other => panic!("Expected Decode error, got: {:?}", other),
    }
}
