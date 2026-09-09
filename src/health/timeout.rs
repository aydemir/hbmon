use std::time::{Duration, Instant};

pub struct TimeoutWatchdog {
    deadline: Option<Instant>,
    limit: u64,
}

impl TimeoutWatchdog {
    pub fn new(limit_sec: Option<u64>) -> Self {
        Self {
            deadline: limit_sec.map(|s| Instant::now() + Duration::from_secs(s)),
            limit: limit_sec.unwrap_or(0),
        }
    }

    pub fn expired(&self) -> bool {
        self.deadline.map(|d| Instant::now() >= d).unwrap_or(false)
    }

    pub fn limit(&self) -> u64 {
        self.limit
    }
}
