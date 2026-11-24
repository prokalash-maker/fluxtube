use crate::error::CacheError;
use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
pub trait ImageFetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, CacheError>;
}

pub struct ReqwestFetcher {
    client: reqwest::Client,
}

impl Default for ReqwestFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ReqwestFetcher {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl ImageFetcher for ReqwestFetcher {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, CacheError> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| CacheError::Network(url.to_string(), e.to_string()))?;

        if !resp.status().is_success() {
            return Err(CacheError::Network(
                url.to_string(),
                format!("HTTP {}", resp.status()),
            ));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| CacheError::Network(url.to_string(), e.to_string()))?;

        Ok(bytes.to_vec())
    }
}

pub struct MockImageFetcher {
    fixtures: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    call_counts: Arc<Mutex<HashMap<String, usize>>>,
}

impl Default for MockImageFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl MockImageFetcher {
    pub fn new() -> Self {
        Self {
            fixtures: Arc::new(Mutex::new(HashMap::new())),
            call_counts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn insert_fixture(&self, url: impl Into<String>, bytes: Vec<u8>) {
        self.fixtures.lock().insert(url.into(), bytes);
    }

    pub fn get_call_count(&self, url: &str) -> usize {
        self.call_counts.lock().get(url).copied().unwrap_or(0)
    }
}

#[async_trait]
impl ImageFetcher for MockImageFetcher {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, CacheError> {
        *self.call_counts.lock().entry(url.to_string()).or_insert(0) += 1;
        match self.fixtures.lock().get(url) {
            Some(bytes) => Ok(bytes.clone()),
            None => Err(CacheError::Network(
                url.to_string(),
                "Mock 404 Not Found".to_string(),
            )),
        }
    }
}
