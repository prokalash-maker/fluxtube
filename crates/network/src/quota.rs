use crate::error::QuotaError;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use chrono_tz::America::Los_Angeles;
use fluxtube_core::{QuotaPool, QuotaStatus};
use parking_lot::RwLock;
use std::time::Duration;

/// Thread-safe YouTube API quota manager tracking general and search pools.
pub struct QuotaTracker {
    state: RwLock<QuotaTrackerState>,
}

#[derive(Debug, Clone)]
struct QuotaTrackerState {
    general_used: u32,
    general_limit: u32,
    search_used: u32,
    search_limit: u32,
    last_reset_date: NaiveDate,
}

impl Default for QuotaTracker {
    fn default() -> Self {
        Self::new(10_000, 100)
    }
}

impl QuotaTracker {
    /// Creates a new QuotaTracker with specified daily limits.
    pub fn new(daily_general_limit: u32, daily_search_limit: u32) -> Self {
        let now = Utc::now();
        let current_date = now.with_timezone(&Los_Angeles).date_naive();
        Self {
            state: RwLock::new(QuotaTrackerState {
                general_used: 0,
                general_limit: daily_general_limit,
                search_used: 0,
                search_limit: daily_search_limit,
                last_reset_date: current_date,
            }),
        }
    }

    /// Checks if the requested units can be consumed without exceeding limits.
    pub fn can_consume(&self, cost: u32, is_search: bool) -> bool {
        let mut state = self.state.write();
        let now = Utc::now();
        Self::reset_if_new_day_internal(&mut state, now);

        if is_search {
            state.search_used + 1 <= state.search_limit
                && state.general_used + cost <= state.general_limit
        } else {
            state.general_used + cost <= state.general_limit
        }
    }

    /// Records consumption of quota units. Returns the updated QuotaStatus.
    pub fn record_consumption(
        &self,
        cost: u32,
        is_search: bool,
    ) -> Result<QuotaStatus, QuotaError> {
        let mut state = self.state.write();
        let now = Utc::now();
        Self::reset_if_new_day_internal(&mut state, now);

        if is_search {
            if state.search_used + 1 > state.search_limit {
                return Err(QuotaError::SearchQuotaExceeded {
                    used: state.search_used,
                    limit: state.search_limit,
                });
            }
            if state.general_used + cost > state.general_limit {
                return Err(QuotaError::GeneralQuotaExceeded {
                    used: state.general_used,
                    limit: state.general_limit,
                });
            }
            state.search_used += 1;
            state.general_used += cost;

            let reset_at = Self::calculate_next_pacific_midnight(now);
            Ok(QuotaStatus {
                pool: QuotaPool::Search,
                used_today: state.search_used,
                daily_limit: state.search_limit,
                remaining: state.search_limit.saturating_sub(state.search_used),
                reset_at,
            })
        } else {
            if state.general_used + cost > state.general_limit {
                return Err(QuotaError::GeneralQuotaExceeded {
                    used: state.general_used,
                    limit: state.general_limit,
                });
            }
            state.general_used += cost;

            let reset_at = Self::calculate_next_pacific_midnight(now);
            Ok(QuotaStatus {
                pool: QuotaPool::General,
                used_today: state.general_used,
                daily_limit: state.general_limit,
                remaining: state.general_limit.saturating_sub(state.general_used),
                reset_at,
            })
        }
    }

    /// Checks if a new Pacific Time day has started and resets usage counters if so.
    /// Returns true if a reset occurred.
    pub fn reset_if_new_day_pacific(&self) -> bool {
        let mut state = self.state.write();
        let now = Utc::now();
        Self::reset_if_new_day_internal(&mut state, now)
    }

    /// Checks if warning threshold percentage (0.0 to 1.0, e.g. 0.80 for 80%) has been reached.
    pub fn warning_triggered(&self, threshold_pct: f32) -> bool {
        let mut state = self.state.write();
        let now = Utc::now();
        Self::reset_if_new_day_internal(&mut state, now);

        let general_ratio = if state.general_limit == 0 {
            1.0
        } else {
            state.general_used as f32 / state.general_limit as f32
        };

        let search_ratio = if state.search_limit == 0 {
            1.0
        } else {
            state.search_used as f32 / state.search_limit as f32
        };

        general_ratio >= threshold_pct || search_ratio >= threshold_pct
    }

    /// Returns current status for specified pool.
    pub fn get_status(&self, pool: QuotaPool) -> QuotaStatus {
        let mut state = self.state.write();
        let now = Utc::now();
        Self::reset_if_new_day_internal(&mut state, now);

        let reset_at = Self::calculate_next_pacific_midnight(now);
        match pool {
            QuotaPool::General => QuotaStatus {
                pool: QuotaPool::General,
                used_today: state.general_used,
                daily_limit: state.general_limit,
                remaining: state.general_limit.saturating_sub(state.general_used),
                reset_at,
            },
            QuotaPool::Search => QuotaStatus {
                pool: QuotaPool::Search,
                used_today: state.search_used,
                daily_limit: state.search_limit,
                remaining: state.search_limit.saturating_sub(state.search_used),
                reset_at,
            },
        }
    }

    /// Duration remaining until next Pacific Time midnight reset.
    pub fn time_until_reset(&self) -> Duration {
        let now = Utc::now();
        let next_reset = Self::calculate_next_pacific_midnight(now);
        next_reset
            .signed_duration_since(now)
            .to_std()
            .unwrap_or(Duration::ZERO)
    }

    /// Next Pacific Time midnight reset timestamp in UTC.
    pub fn next_reset_at(&self) -> DateTime<Utc> {
        Self::calculate_next_pacific_midnight(Utc::now())
    }

    /// Manually sets used amounts (useful for testing or state recovery).
    pub fn set_used(&self, general: u32, search: u32) {
        let mut state = self.state.write();
        state.general_used = general;
        state.search_used = search;
    }

    /// Manually sets last reset date (useful for testing date rollover).
    #[cfg(test)]
    pub fn set_last_reset_date(&self, date: NaiveDate) {
        let mut state = self.state.write();
        state.last_reset_date = date;
    }

    fn reset_if_new_day_internal(state: &mut QuotaTrackerState, now: DateTime<Utc>) -> bool {
        let pacific_date = now.with_timezone(&Los_Angeles).date_naive();
        if pacific_date > state.last_reset_date {
            state.general_used = 0;
            state.search_used = 0;
            state.last_reset_date = pacific_date;
            tracing::info!(
                pacific_date = %pacific_date,
                "YouTube API daily quota reset at Pacific Time midnight"
            );
            true
        } else {
            false
        }
    }

    /// Calculates next Pacific Time midnight from a reference UTC time.
    pub fn calculate_next_pacific_midnight(now: DateTime<Utc>) -> DateTime<Utc> {
        let pacific_now = now.with_timezone(&Los_Angeles);
        let tomorrow_pacific = pacific_now.date_naive().succ_opt().unwrap_or(pacific_now.date_naive());
        let midnight_local = tomorrow_pacific.and_hms_opt(0, 0, 0).unwrap();
        let midnight_pacific = Los_Angeles
            .from_local_datetime(&midnight_local)
            .earliest()
            .unwrap_or(pacific_now);
        midnight_pacific.with_timezone(&Utc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_tracker_consumption_and_limits() {
        let tracker = QuotaTracker::new(10_000, 100);

        assert!(tracker.can_consume(1, false));
        assert!(tracker.can_consume(100, true));

        // Consume 100 general units
        let status = tracker.record_consumption(100, false).unwrap();
        assert_eq!(status.used_today, 100);
        assert_eq!(status.remaining, 9_900);

        // Consume 1 search call (costs 100 general units as well)
        let search_status = tracker.record_consumption(100, true).unwrap();
        assert_eq!(search_status.used_today, 1);
        assert_eq!(search_status.remaining, 99);

        let general_status = tracker.get_status(QuotaPool::General);
        assert_eq!(general_status.used_today, 200);
        assert_eq!(general_status.remaining, 9_800);
    }

    #[test]
    fn test_quota_tracker_warnings() {
        let tracker = QuotaTracker::new(10_000, 100);
        tracker.set_used(7_999, 0);

        assert!(!tracker.warning_triggered(0.80));

        tracker.record_consumption(1, false).unwrap();
        assert!(tracker.warning_triggered(0.80));
    }

    #[test]
    fn test_quota_tracker_search_limit_exceeded() {
        let tracker = QuotaTracker::new(10_000, 2);

        tracker.record_consumption(100, true).unwrap();
        tracker.record_consumption(100, true).unwrap();

        assert!(!tracker.can_consume(100, true));
        let err = tracker.record_consumption(100, true).unwrap_err();
        assert_eq!(
            err,
            QuotaError::SearchQuotaExceeded { used: 2, limit: 2 }
        );
    }

    #[test]
    fn test_quota_tracker_pacific_midnight_reset() {
        let tracker = QuotaTracker::new(10_000, 100);
        tracker.set_used(5_000, 50);

        // Under current Pacific date, reset does not occur
        assert!(!tracker.reset_if_new_day_pacific());
        assert_eq!(tracker.get_status(QuotaPool::General).used_today, 5_000);

        // Simulate previous date in state
        let yesterday = Utc::now()
            .with_timezone(&Los_Angeles)
            .date_naive()
            .pred_opt()
            .unwrap();
        tracker.set_last_reset_date(yesterday);

        // Next check resets to 0
        assert!(tracker.reset_if_new_day_pacific());
        assert_eq!(tracker.get_status(QuotaPool::General).used_today, 0);
        assert_eq!(tracker.get_status(QuotaPool::Search).used_today, 0);
    }
}
