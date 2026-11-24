use crate::backoff::BackoffConfig;
use crate::circuit::{CircuitBreaker, CircuitBreakerConfig};
use crate::deduplication::{InFlightDeduplicator, ResilientResponse};
use crate::error::NetworkError;
use crate::quota::QuotaTracker;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::time::Duration;

/// Resilient HTTP Client providing connection pooling, automatic retries with jitter,
/// circuit breaker pattern, and GET request deduplication.
#[derive(Clone)]
pub struct ResilientHttpClient {
    client: reqwest::Client,
    backoff: BackoffConfig,
    circuit_breaker: Arc<CircuitBreaker>,
    deduplicator: Arc<InFlightDeduplicator>,
    quota_tracker: Option<Arc<QuotaTracker>>,
}

impl ResilientHttpClient {
    pub fn new(
        client: reqwest::Client,
        backoff: BackoffConfig,
        circuit_config: CircuitBreakerConfig,
        quota_tracker: Option<Arc<QuotaTracker>>,
    ) -> Self {
        Self {
            client,
            backoff,
            circuit_breaker: Arc::new(CircuitBreaker::new(circuit_config)),
            deduplicator: Arc::new(InFlightDeduplicator::new()),
            quota_tracker,
        }
    }

    pub fn builder() -> ResilientHttpClientBuilder {
        ResilientHttpClientBuilder::default()
    }

    /// Access inner reqwest client.
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Access Circuit Breaker.
    pub fn circuit_breaker(&self) -> &Arc<CircuitBreaker> {
        &self.circuit_breaker
    }

    /// Access Quota Tracker if configured.
    pub fn quota_tracker(&self) -> Option<&Arc<QuotaTracker>> {
        self.quota_tracker.as_ref()
    }

    /// Access In-Flight Deduplicator.
    pub fn deduplicator(&self) -> &Arc<InFlightDeduplicator> {
        &self.deduplicator
    }

    /// Executes an HTTP request with circuit breaker check and exponential backoff retry.
    pub async fn execute(&self, request: reqwest::Request) -> Result<reqwest::Response, NetworkError> {
        self.circuit_breaker.can_execute()?;

        let mut last_error_message = String::new();
        let max_retries = self.backoff.max_retries;

        for attempt in 0..=max_retries {
            let req_clone = match request.try_clone() {
                Some(r) => r,
                None => {
                    // Cannot clone (e.g. streaming body), execute once
                    let resp = self.client.execute(request).await?;
                    if resp.status().is_success() {
                        self.circuit_breaker.record_success();
                    } else if BackoffConfig::is_retriable_status(resp.status()) {
                        self.circuit_breaker.record_failure();
                    }
                    return Ok(resp);
                }
            };

            match self.client.execute(req_clone).await {
                Ok(response) => {
                    let status = response.status();
                    if BackoffConfig::is_retriable_status(status) && attempt < max_retries {
                        last_error_message = format!("Server returned retriable status {}", status);
                        let delay = self.backoff.calculate_delay(attempt);
                        tracing::warn!(
                            attempt,
                            status = %status,
                            delay_ms = delay.as_millis(),
                            "Retriable HTTP status; backing off"
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    if status.is_server_error() {
                        self.circuit_breaker.record_failure();
                    } else {
                        self.circuit_breaker.record_success();
                    }

                    return Ok(response);
                }
                Err(err) => {
                    let is_retriable = BackoffConfig::is_retriable_error(&err);
                    last_error_message = err.to_string();

                    if is_retriable && attempt < max_retries {
                        let delay = self.backoff.calculate_delay(attempt);
                        tracing::warn!(
                            attempt,
                            error = %err,
                            delay_ms = delay.as_millis(),
                            "Retriable network error; backing off"
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    self.circuit_breaker.record_failure();
                    return Err(NetworkError::Http(err));
                }
            }
        }

        self.circuit_breaker.record_failure();
        Err(NetworkError::MaxRetriesExceeded {
            max_retries,
            last_error: last_error_message,
        })
    }

    /// Fetches bytes for a URL with GET request deduplication.
    pub async fn get_bytes(&self, url: &str) -> Result<bytes::Bytes, NetworkError> {
        let resp = self.get_resilient_response(url).await?;
        if !resp.status.is_success() {
            let body_str = String::from_utf8_lossy(&resp.body).into_owned();
            return Err(NetworkError::Status {
                status: resp.status,
                body: body_str,
            });
        }
        Ok(resp.body)
    }

    /// Fetches and deserializes JSON from URL with deduplication.
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T, NetworkError> {
        let bytes = self.get_bytes(url).await?;
        serde_json::from_slice(&bytes).map_err(|e| NetworkError::Status {
            status: reqwest::StatusCode::OK,
            body: format!("JSON deserialization failed: {}", e),
        })
    }

    /// Internal helper executing GET request with deduplication.
    pub async fn get_resilient_response(&self, url: &str) -> Result<ResilientResponse, NetworkError> {
        let url_owned = url.to_string();
        let client = self.clone();

        self.deduplicator
            .get_or_execute(url, || async move {
                let req = client
                    .client
                    .get(&url_owned)
                    .build()
                    .map_err(NetworkError::Http)?;
                let resp = client.execute(req).await?;
                ResilientResponse::from_reqwest(resp)
                    .await
                    .map_err(NetworkError::Http)
            })
            .await
    }
}

pub struct ResilientHttpClientBuilder {
    timeout: Duration,
    backoff: BackoffConfig,
    circuit: CircuitBreakerConfig,
    quota_tracker: Option<Arc<QuotaTracker>>,
    user_agent: Option<String>,
}

impl Default for ResilientHttpClientBuilder {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            backoff: BackoffConfig::default(),
            circuit: CircuitBreakerConfig::default(),
            quota_tracker: None,
            user_agent: Some("FluxTube/0.1.0 (Desktop; Rust)".to_string()),
        }
    }
}

impl ResilientHttpClientBuilder {
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn backoff(mut self, backoff: BackoffConfig) -> Self {
        self.backoff = backoff;
        self
    }

    pub fn circuit_breaker(mut self, circuit: CircuitBreakerConfig) -> Self {
        self.circuit = circuit;
        self
    }

    pub fn quota_tracker(mut self, tracker: Arc<QuotaTracker>) -> Self {
        self.quota_tracker = Some(tracker);
        self
    }

    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    pub fn build(self) -> Result<ResilientHttpClient, NetworkError> {
        let mut builder = reqwest::Client::builder().timeout(self.timeout);
        if let Some(ua) = self.user_agent {
            builder = builder.user_agent(ua);
        }

        let client = builder.build()?;
        Ok(ResilientHttpClient::new(
            client,
            self.backoff,
            self.circuit,
            self.quota_tracker,
        ))
    }
}
