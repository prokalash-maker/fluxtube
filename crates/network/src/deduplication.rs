use crate::error::NetworkError;
use parking_lot::Mutex;
use std::collections::HashMap;
use tokio::sync::broadcast;

/// Cached in-memory representation of an HTTP response for deduplication.
#[derive(Clone, Debug)]
pub struct ResilientResponse {
    pub status: reqwest::StatusCode,
    pub headers: reqwest::header::HeaderMap,
    pub body: bytes::Bytes,
}

impl ResilientResponse {
    pub async fn from_reqwest(response: reqwest::Response) -> Result<Self, reqwest::Error> {
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.bytes().await?;
        Ok(Self {
            status,
            headers,
            body,
        })
    }
}

/// In-flight request deduplicator coalescing identical concurrent GET requests.
pub struct InFlightDeduplicator {
    in_flight: Mutex<HashMap<String, broadcast::Sender<Result<ResilientResponse, String>>>>,
}

impl Default for InFlightDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

impl InFlightDeduplicator {
    pub fn new() -> Self {
        Self {
            in_flight: Mutex::new(HashMap::new()),
        }
    }

    /// Executes `fetch_future` or waits for an existing in-flight request for the same `url_key`.
    pub async fn get_or_execute<F, Fut>(
        &self,
        url_key: &str,
        fetch_future: F,
    ) -> Result<ResilientResponse, NetworkError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<ResilientResponse, NetworkError>>,
    {
        let (tx, mut rx, is_leader) = {
            let mut map = self.in_flight.lock();
            if let Some(existing_tx) = map.get(url_key) {
                (existing_tx.clone(), existing_tx.subscribe(), false)
            } else {
                let (tx, rx) = broadcast::channel(1);
                map.insert(url_key.to_string(), tx.clone());
                (tx, rx, true)
            }
        };

        if is_leader {
            // Guard ensuring removal from map on completion or panic
            struct Guard<'a> {
                map: &'a Mutex<HashMap<String, broadcast::Sender<Result<ResilientResponse, String>>>>,
                key: &'a str,
            }
            impl<'a> Drop for Guard<'a> {
                fn drop(&mut self) {
                    let mut lock = self.map.lock();
                    lock.remove(self.key);
                }
            }
            let _guard = Guard {
                map: &self.in_flight,
                key: url_key,
            };

            let result = fetch_future().await;
            match &result {
                Ok(resp) => {
                    let _ = tx.send(Ok(resp.clone()));
                }
                Err(err) => {
                    let _ = tx.send(Err(err.to_string()));
                }
            }
            result
        } else {
            // Follower waits for leader broadcast
            match rx.recv().await {
                Ok(Ok(response)) => Ok(response),
                Ok(Err(err_msg)) => Err(NetworkError::MaxRetriesExceeded {
                    max_retries: 0,
                    last_error: err_msg,
                }),
                Err(_) => Err(NetworkError::DeduplicationFailed),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn test_request_deduplication() {
        let deduplicator = Arc::new(InFlightDeduplicator::new());
        let execution_counter = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..10 {
            let dedup = deduplicator.clone();
            let counter = execution_counter.clone();
            handles.push(tokio::spawn(async move {
                dedup
                    .get_or_execute("https://api.youtube.com/test", || async {
                        counter.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        Ok(ResilientResponse {
                            status: reqwest::StatusCode::OK,
                            headers: reqwest::header::HeaderMap::new(),
                            body: bytes::Bytes::from_static(b"shared response body"),
                        })
                    })
                    .await
            }));
        }

        let mut responses = Vec::new();
        for handle in handles {
            let res = handle.await.unwrap().expect("request should succeed");
            responses.push(res);
        }

        assert_eq!(responses.len(), 10);
        // Only 1 execution should have taken place across all 10 concurrent requests!
        assert_eq!(execution_counter.load(Ordering::SeqCst), 1);
        for res in responses {
            assert_eq!(res.body, bytes::Bytes::from_static(b"shared response body"));
        }
    }
}
