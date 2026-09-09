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
