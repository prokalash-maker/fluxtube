use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuotaPool {
    General,
    Search,
}

impl std::fmt::Display for QuotaPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuotaPool::General => write!(f, "general"),
            QuotaPool::Search => write!(f, "search"),
        }
    }
}

impl std::str::FromStr for QuotaPool {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "general" => Ok(QuotaPool::General),
            "search" => Ok(QuotaPool::Search),
            other => Err(format!("Unknown quota pool: {}", other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaStatus {
    pub pool: QuotaPool,
    pub used_today: u32,
    pub daily_limit: u32,
    pub remaining: u32,
    pub reset_at: DateTime<Utc>,
}

impl QuotaStatus {
    pub fn usage_ratio(&self) -> f32 {
        if self.daily_limit == 0 {
            1.0
        } else {
            (self.used_today as f32 / self.daily_limit as f32).clamp(0.0, 1.0)
        }
    }

    pub fn is_exhausted(&self) -> bool {
        self.remaining == 0
    }

    pub fn can_afford(&self, units: u32) -> bool {
        self.remaining >= units
    }
}
