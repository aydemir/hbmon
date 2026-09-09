use std::collections::HashMap;
use std::time::Instant;

/// Delta-based CPU% tracker (RFC 10.3). jiffies come from
/// /proc/<pid>/stat (utime+stime); Hz assumed 100.
pub struct CpuTracker {
    prev: HashMap<u32, (Instant, u64)>,
}

impl CpuTracker {
    pub fn new() -> Self {
        Self {
            prev: HashMap::new(),
        }
    }

    pub fn update(&mut self, pid: u32, jiffies: u64) -> f32 {
        let now = Instant::now();
        let pct = match self.prev.get(&pid) {
            Some((t_prev, j_prev)) => {
                let dt = now.duration_since(*t_prev).as_secs_f32();
                if dt <= 0.0 {
                    0.0
                } else {
                    let dj = jiffies.saturating_sub(*j_prev) as f32;
                    (dj / dt / 100.0 * 100.0).clamp(0.0, 3200.0)
                }
            }
            None => 0.0,
        };
        self.prev.insert(pid, (now, jiffies));
        pct
    }

    pub fn evict_gone(&mut self, alive: &[u32]) {
        let keep: std::collections::HashSet<u32> = alive.iter().copied().collect();
        self.prev.retain(|k, _| keep.contains(k));
    }
}

impl Default for CpuTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_sample_is_zero() {
        let mut t = CpuTracker::new();
        assert_eq!(t.update(100, 5000), 0.0);
    }

    #[test]
    fn no_jiffy_delta_is_zero() {
        let mut t = CpuTracker::new();
        t.update(100, 5000);
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert_eq!(t.update(100, 5000), 0.0);
    }

    #[test]
    fn jiffy_delta_is_positive_and_bounded() {
        let mut t = CpuTracker::new();
        t.update(100, 1000);
        std::thread::sleep(std::time::Duration::from_millis(5));
        let pct = t.update(100, 1100);
        assert!(pct > 0.0 && pct <= 3200.0);
    }

    #[test]
    fn evict_gone_drops_dead_pids() {
        let mut t = CpuTracker::new();
        t.update(1, 10);
        t.update(2, 20);
        t.evict_gone(&[2]);
        assert_eq!(t.update(1, 999), 0.0);
    }
}
