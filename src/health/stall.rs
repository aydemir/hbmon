//! Adaptive stall detection (RFC 7.2):
//! warmup 60s -> rolling p95 of silent gaps ->
//! stall if current_idle > 3*p95 AND idle > 30s.
//! Short builds (<10s) never stall.

use std::collections::VecDeque;
use std::time::Instant;

/// Sessiz aralık örneklerinin kayan penceresi (en fazla 120).
const WINDOW: usize = 120;
/// Örnek yokken tarafsız önsel (soğuk başlangıçta eşik 30s olur).
const NEUTRAL_P95: f32 = 5.0;

pub struct StallDetector {
    start: Instant,
    silent_samples: VecDeque<f32>,
    p95_cache: f32,
    current_idle: f32,
    in_stall: bool,
}

impl StallDetector {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            silent_samples: VecDeque::new(),
            p95_cache: NEUTRAL_P95,
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
                // after that keep rolling window of last WINDOW samples)
                if self.elapsed() <= 60.0 || self.silent_samples.len() < WINDOW {
                    self.silent_samples.push_back(self.current_idle);
                } else {
                    self.silent_samples.pop_front();
                    self.silent_samples.push_back(self.current_idle);
                }
                self.p95_cache = Self::p95_of(&self.silent_samples);
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

    fn p95_of(samples: &VecDeque<f32>) -> f32 {
        if samples.is_empty() {
            return NEUTRAL_P95; // neutral prior
        }
        let mut v: Vec<f32> = samples.iter().copied().collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[((v.len() as f32 * 0.95) as usize).min(v.len() - 1)]
    }

    fn p95(&self) -> f32 {
        self.p95_cache
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
            silent_samples: VecDeque::new(),
            p95_cache: NEUTRAL_P95,
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

    #[test]
    fn rolling_window_caps_length_and_tracks_p95() {
        let mut s = aged(61);
        for _ in 0..130 {
            s.tick(false, 2.0);
            s.tick(true, 0.5);
        }
        assert_eq!(s.silent_samples.len(), 120);
        // all samples are 2.0 → p95 tracks the pushed value, threshold stays 30s floor
        assert_eq!(s.p95_idle(), 2.0);
        assert_eq!(s.threshold(), 30.0);
    }
}
