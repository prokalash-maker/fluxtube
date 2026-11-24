use crate::error::NetworkError;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use parking_lot::RwLock;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: usize,
    pub cool_off_duration: Duration,
    pub half_open_success_threshold: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            cool_off_duration: Duration::from_secs(30),
            half_open_success_threshold: 2,
        }
    }
}

#[derive(Debug)]
struct CircuitBreakerState {
    state: CircuitState,
    consecutive_failures: usize,
    half_open_successes: usize,
    cool_off_until: Option<DateTime<Utc>>,
}

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    inner: RwLock<CircuitBreakerState>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            inner: RwLock::new(CircuitBreakerState {
                state: CircuitState::Closed,
                consecutive_failures: 0,
                half_open_successes: 0,
                cool_off_until: None,
            }),
        }
    }

    /// Gets current circuit state, automatically evaluating cool-off expiry.
    pub fn state(&self) -> CircuitState {
        let mut inner = self.inner.write();
        self.evaluate_state(&mut inner);
        inner.state
    }

    /// Checks if a request is permitted to execute through the circuit.
    pub fn can_execute(&self) -> Result<(), NetworkError> {
        let mut inner = self.inner.write();
        self.evaluate_state(&mut inner);

        match inner.state {
            CircuitState::Closed | CircuitState::HalfOpen => Ok(()),
            CircuitState::Open => {
                let cool_off_until = inner.cool_off_until.unwrap_or_else(Utc::now);
                Err(NetworkError::CircuitBreakerOpen { cool_off_until })
            }
        }
    }

    /// Records a successful request execution.
    pub fn record_success(&self) {
        let mut inner = self.inner.write();
        self.evaluate_state(&mut inner);

        match inner.state {
            CircuitState::Closed => {
                inner.consecutive_failures = 0;
            }
            CircuitState::HalfOpen => {
                inner.half_open_successes += 1;
                if inner.half_open_successes >= self.config.half_open_success_threshold {
                    inner.state = CircuitState::Closed;
                    inner.consecutive_failures = 0;
                    inner.half_open_successes = 0;
                    inner.cool_off_until = None;
                    tracing::info!("CircuitBreaker transitioned from HalfOpen to Closed");
                }
            }
            CircuitState::Open => {
                // Should not happen, but if success occurs, reset
                inner.state = CircuitState::Closed;
                inner.consecutive_failures = 0;
                inner.half_open_successes = 0;
                inner.cool_off_until = None;
            }
        }
    }

    /// Records a failed request execution.
    pub fn record_failure(&self) {
        let mut inner = self.inner.write();
        let now = Utc::now();

        match inner.state {
            CircuitState::Closed => {
                inner.consecutive_failures += 1;
                if inner.consecutive_failures >= self.config.failure_threshold {
                    inner.state = CircuitState::Open;
                    let cool_off = ChronoDuration::from_std(self.config.cool_off_duration)
                        .unwrap_or(ChronoDuration::seconds(30));
                    inner.cool_off_until = Some(now + cool_off);
                    tracing::warn!(
                        consecutive_failures = inner.consecutive_failures,
                        "CircuitBreaker tripped to Open"
                    );
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in HalfOpen trips directly back to Open
                inner.state = CircuitState::Open;
                let cool_off = ChronoDuration::from_std(self.config.cool_off_duration)
                    .unwrap_or(ChronoDuration::seconds(30));
                inner.cool_off_until = Some(now + cool_off);
                inner.half_open_successes = 0;
                tracing::warn!("CircuitBreaker trial failed; tripped back to Open");
            }
            CircuitState::Open => {
                // Prolong cool-off
                let cool_off = ChronoDuration::from_std(self.config.cool_off_duration)
                    .unwrap_or(ChronoDuration::seconds(30));
                inner.cool_off_until = Some(now + cool_off);
            }
        }
    }

    /// Manually resets circuit breaker to Closed.
    pub fn reset(&self) {
        let mut inner = self.inner.write();
        inner.state = CircuitState::Closed;
        inner.consecutive_failures = 0;
        inner.half_open_successes = 0;
        inner.cool_off_until = None;
    }

    fn evaluate_state(&self, inner: &mut CircuitBreakerState) {
        if inner.state == CircuitState::Open {
            if let Some(until) = inner.cool_off_until {
                if Utc::now() >= until {
                    inner.state = CircuitState::HalfOpen;
                    inner.half_open_successes = 0;
                    tracing::info!("CircuitBreaker cool-off elapsed; transitioned to HalfOpen");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_transitions() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            cool_off_duration: Duration::from_millis(100),
            half_open_success_threshold: 2,
        };
        let cb = CircuitBreaker::new(config);

        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.can_execute().is_ok());

        // 1st failure
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.can_execute().is_ok());

        // 2nd failure
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        // 3rd failure -> trips to Open
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(cb.can_execute().is_err());

        // Wait for cool off
        std::thread::sleep(Duration::from_millis(120));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(cb.can_execute().is_ok());

        // 1st success in HalfOpen
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // 2nd success in HalfOpen -> returns to Closed
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.can_execute().is_ok());
    }

    #[test]
    fn test_circuit_breaker_half_open_failure_reopens() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            cool_off_duration: Duration::from_millis(50),
            half_open_success_threshold: 2,
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Failure in HalfOpen immediately trips back to Open
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(cb.can_execute().is_err());
    }
}
