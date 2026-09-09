//! Adaptive stall detection (RFC 7.2):
//! warmup 60s -> rolling p95 of silent gaps ->
//! stall if current_idle > 3*p95 AND idle > 30s.
//! Short builds (<10s) never stall.

use std::time::Instant;

pub struct StallDetector {
    start: Instant,
    silent_samples: Vec<f32>,
    current_idle: f32,
    in_stall: bool,
}

impl StallDetector {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            silent_samples: vec![],
            current_idle: 0.0,
            in_stall: false,
        }
    }

    /// Call each tick (1Hz). `active` = cpu>0 || io_delta>0 || new_child.
    /// Returns (entered_stall, resolved_stall).
    pub fn tick(&mut self, active: bool, dt: f32) -> (bool, bool) {
        if active {
            if self.elapsed() >= 1.0 && self.current_idle >= 1.0 {
                // record silent gap (warmup only, first 60s builds the baseline;
                // after that keep rolling window of last 120 samples)
                if self.elapsed() <= 60.0 || self.silent_samples.len() < 120 {
                    self.silent_samples.push(self.current_idle);
                } else {
                    self.silent_samples.remove(0);
                    self.silent_samples.push(self.current_idle);
                }
            }
            let was = self.in_stall;
            self.current_idle = 0.0;
            self.in_stall = false;
            (false, was)
        } else {
            self.current_idle += dt;
            if self.should_stall() && !self.in_stall {
                self.in_stall = true;
                (true, false)
            } else {
                (false, false)
            }
        }
    }

    fn elapsed(&self) -> f32 {
        self.start.elapsed().as_secs_f32()
    }

    fn p95(&self) -> f32 {
        if self.silent_samples.is_empty() {
            return 5.0; // neutral prior
        }
        let mut v = self.silent_samples.clone();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[((v.len() as f32 * 0.95) as usize).min(v.len() - 1)]
    }

    pub fn threshold(&self) -> f32 {
        (3.0 * self.p95()).max(30.0)
    }

    fn should_stall(&self) -> bool {
        if self.elapsed() < 10.0 {
            return false; // short-build guard
        }
        self.current_idle > self.threshold()
    }

    pub fn score(&self) -> f32 {
        if self.in_stall {
            1.0
        } else {
            (self.current_idle / self.threshold()).clamp(0.0, 1.0)
        }
    }

    pub fn idle(&self) -> f32 {
        self.current_idle
    }
    pub fn p95_idle(&self) -> f32 {
        self.p95()
    }
    pub fn in_stall(&self) -> bool {
        self.in_stall
    }
}

impl Default for StallDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn aged(secs: u64) -> StallDetector {
        StallDetector {
            start: Instant::now() - Duration::from_secs(secs),
            silent_samples: vec![],
            current_idle: 0.0,
            in_stall: false,
        }
    }

    #[test]
    fn fresh_detector_never_stalls_short_build() {
        let mut s = StallDetector::new();
        let (entered, _) = s.tick(false, 100.0);
        assert!(!entered);
        assert!(!s.in_stall());
    }

    #[test]
    fn prolonged_silence_enters_stall() {
        let mut s = aged(61);
        let (entered, resolved) = s.tick(false, 31.0);
        assert!(entered);
        assert!(!resolved);
        assert!(s.in_stall());
        assert_eq!(s.score(), 1.0);
    }

    #[test]
    fn activity_resolves_stall() {
        let mut s = aged(61);
        s.tick(false, 31.0);
        let (entered, resolved) = s.tick(true, 0.5);
        assert!(!entered);
        assert!(resolved);
        assert!(!s.in_stall());
    }

    #[test]
    fn default_threshold_is_30s() {
        let s = aged(61);
        assert_eq!(s.threshold(), 30.0);
        assert_eq!(s.p95_idle(), 5.0);
    }

    #[test]
    fn idle_accumulates_and_resets() {
        let mut s = aged(61);
        s.tick(false, 5.0);
        assert_eq!(s.idle(), 5.0);
        s.tick(true, 0.5);
        assert_eq!(s.idle(), 0.0);
    }
}
