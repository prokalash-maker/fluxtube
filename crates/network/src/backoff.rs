use rand::Rng;
use std::time::Duration;

/// Configuration for exponential backoff retry strategy with jitter.
#[derive(Debug, Clone)]
pub struct BackoffConfig {
    pub max_retries: usize,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
    pub jitter_factor: f64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
            jitter_factor: 0.25,
        }
    }
}

impl BackoffConfig {
    pub fn new(
        max_retries: usize,
        initial_delay: Duration,
        max_delay: Duration,
        multiplier: f64,
        jitter_factor: f64,
    ) -> Self {
        Self {
            max_retries,
            initial_delay,
            max_delay,
            multiplier,
            jitter_factor,
        }
    }

    /// Calculates delay for a given attempt index (0-based) with full jitter.
    pub fn calculate_delay(&self, attempt: usize) -> Duration {
        let factor = self.multiplier.powi(attempt as i32);
        let base_millis = (self.initial_delay.as_secs_f64() * factor * 1000.0)
            .min(self.max_delay.as_secs_f64() * 1000.0);

        // Apply symmetric jitter around base_millis: [base * (1 - jitter), base * (1 + jitter)]
        let mut rng = rand::thread_rng();
        let jitter_range = base_millis * self.jitter_factor;
        let min_val = (base_millis - jitter_range).max(1.0);
        let max_val = (base_millis + jitter_range).min(self.max_delay.as_secs_f64() * 1000.0);

        let randomized = rng.gen_range(min_val..=max_val);
        Duration::from_millis(randomized.round() as u64)
    }

    /// Checks if an HTTP status code is transient/retriable.
    pub fn is_retriable_status(status: reqwest::StatusCode) -> bool {
        matches!(
            status,
            reqwest::StatusCode::TOO_MANY_REQUESTS // 429
                | reqwest::StatusCode::INTERNAL_SERVER_ERROR // 500
                | reqwest::StatusCode::BAD_GATEWAY // 502
                | reqwest::StatusCode::SERVICE_UNAVAILABLE // 503
                | reqwest::StatusCode::GATEWAY_TIMEOUT // 504
        )
    }

    /// Checks if a reqwest error is transient/retriable.
    pub fn is_retriable_error(err: &reqwest::Error) -> bool {
        err.is_timeout() || err.is_connect() || err.is_request()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_delays_and_jitter() {
        let config = BackoffConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
            jitter_factor: 0.25,
        };

        // Attempt 0: ~500ms ± 25% (375ms .. 625ms)
        for _ in 0..10 {
            let delay0 = config.calculate_delay(0);
            assert!(
                delay0 >= Duration::from_millis(370) && delay0 <= Duration::from_millis(630),
                "delay0 out of range: {:?}",
                delay0
            );
        }

        // Attempt 1: ~1000ms ± 25% (750ms .. 1250ms)
        for _ in 0..10 {
            let delay1 = config.calculate_delay(1);
            assert!(
                delay1 >= Duration::from_millis(740) && delay1 <= Duration::from_millis(1260),
                "delay1 out of range: {:?}",
                delay1
            );
        }

        // High attempt capped by max_delay = 10s
        for _ in 0..10 {
            let delay_high = config.calculate_delay(10);
            assert!(
                delay_high <= Duration::from_secs(10),
                "delay_high should not exceed max_delay: {:?}",
                delay_high
            );
        }
    }

    #[test]
    fn test_retriable_status_codes() {
        assert!(BackoffConfig::is_retriable_status(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(BackoffConfig::is_retriable_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR));
        assert!(BackoffConfig::is_retriable_status(reqwest::StatusCode::BAD_GATEWAY));
        assert!(BackoffConfig::is_retriable_status(reqwest::StatusCode::SERVICE_UNAVAILABLE));
        assert!(BackoffConfig::is_retriable_status(reqwest::StatusCode::GATEWAY_TIMEOUT));

        assert!(!BackoffConfig::is_retriable_status(reqwest::StatusCode::OK));
        assert!(!BackoffConfig::is_retriable_status(reqwest::StatusCode::BAD_REQUEST));
        assert!(!BackoffConfig::is_retriable_status(reqwest::StatusCode::UNAUTHORIZED));
        assert!(!BackoffConfig::is_retriable_status(reqwest::StatusCode::FORBIDDEN));
        assert!(!BackoffConfig::is_retriable_status(reqwest::StatusCode::NOT_FOUND));
    }
}
