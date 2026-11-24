use crate::config::DecodedImage;
use crate::error::CacheError;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::oneshot;

type Waiter = oneshot::Sender<Result<Arc<DecodedImage>, CacheError>>;

#[derive(Default)]
pub struct SingleFlight {
    in_flight: Mutex<HashMap<String, Vec<Waiter>>>,
}

impl SingleFlight {
    pub fn new() -> Self {
        Self {
            in_flight: Mutex::new(HashMap::new()),
        }
    }

    pub async fn execute<F, Fut>(&self, key: &str, op: F) -> Result<Arc<DecodedImage>, CacheError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Arc<DecodedImage>, CacheError>>,
    {
        // 1. Check if another request is in-flight for key
        let rx = {
            let mut map = self.in_flight.lock();
            if let Some(waiters) = map.get_mut(key) {
                let (tx, rx) = oneshot::channel();
                waiters.push(tx);
                Some(rx)
            } else {
                map.insert(key.to_string(), Vec::new());
                None
            }
        };

        // 2. If already in flight, await the existing request's outcome
        if let Some(rx) = rx {
            return match rx.await {
                Ok(res) => res,
                Err(_) => Err(CacheError::SingleFlight("Request was cancelled".to_string())),
            };
        }

        // 3. First caller executes op
        let result = op().await;

        // 4. Notify all registered waiters and remove from map
        let waiters = {
            let mut map = self.in_flight.lock();
            map.remove(key).unwrap_or_default()
        };

        for waiter in waiters {
            let _ = waiter.send(result.clone());
        }

        result
    }
}
