//! cgroup-aware resource accounting (RFC v1.5, minimal wiring).
//!
//! - v2 unified hierarchy only; v1 / Android-hybrid / macOS → `None`.
//! - Usability probe: `cpu.stat` must exist, else the mount is a
//!   restricted stub (e.g. Android's cgroup2fs without controllers).
//! - `/proc` accounting stays primary; cgroup adds exact CPU time,
//!   memory peak, and a trustworthy OOM signal (`memory.events`).

use std::path::PathBuf;

pub struct Cgroup {
    pub dir: PathBuf,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub cpu_usec: u64,
    pub mem_bytes: u64,
    pub mem_peak: u64,
    pub pids: u32,
    pub oom_kills: u64,
    pub io_rbytes: u64,
    pub io_wbytes: u64,
}

/// Locate pid's v2 cgroup dir. `None` = unavailable (not an error).
pub fn locate(pid: u32) -> Option<Cgroup> {
    let content = std::fs::read_to_string(format!("/proc/{}/cgroup", pid)).ok()?;
    for line in content.lines() {
        // v2 unified line looks like "0::/user.slice/...".
        if let Some(rel) = line.strip_prefix("0::") {
            let dir = PathBuf::from("/sys/fs/cgroup").join(rel.trim_start_matches('/'));
            if dir.join("cpu.stat").exists() {
                return Some(Cgroup { dir });
            }
            return None;
        }
    }
    None
}

pub fn read_stats(cg: &Cgroup) -> Option<Stats> {
    let cpu = std::fs::read_to_string(cg.dir.join("cpu.stat")).ok()?;
    let mem: u64 = read_trim_u64(&cg.dir.join("memory.current"));
    let peak: u64 = read_trim_u64(&cg.dir.join("memory.peak"));
    let pids: u32 = read_trim_u64(&cg.dir.join("pids.current")) as u32;
    let events = std::fs::read_to_string(cg.dir.join("memory.events")).unwrap_or_default();
    let io = std::fs::read_to_string(cg.dir.join("io.stat")).unwrap_or_default();
    let (io_r, io_w) = parse_io_stat(&io);
    Some(Stats {
        cpu_usec: parse_cpu_stat(&cpu),
        mem_bytes: mem,
        mem_peak: peak,
        pids,
        oom_kills: parse_kv_u64(&events, "oom_kill"),
        io_rbytes: io_r,
        io_wbytes: io_w,
    })
}

fn read_trim_u64(path: &std::path::Path) -> u64 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

/// `cpu.stat` → `usage_usec` value (microseconds of CPU consumed).
pub fn parse_cpu_stat(s: &str) -> u64 {
    parse_kv_u64(s, "usage_usec")
}

/// First `"<key> <int>"` line wins; missing key → 0.
pub fn parse_kv_u64(s: &str, key: &str) -> u64 {
    for line in s.lines() {
        let mut it = line.split_whitespace();
        if it.next() == Some(key) {
            if let Some(v) = it.next().and_then(|x| x.parse().ok()) {
                return v;
            }
        }
    }
    0
}

/// `io.stat` lines look like `8:0 rbytes=123 wbytes=456 ...`; sums all.
pub fn parse_io_stat(s: &str) -> (u64, u64) {
    let mut r = 0u64;
    let mut w = 0u64;
    for line in s.lines() {
        for tok in line.split_whitespace().skip(1) {
            if let Some(v) = tok.strip_prefix("rbytes=") {
                r = r.saturating_add(v.parse().unwrap_or(0));
            } else if let Some(v) = tok.strip_prefix("wbytes=") {
                w = w.saturating_add(v.parse().unwrap_or(0));
            }
        }
    }
    (r, w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_stat_usage() {
        let s = "usage_usec 123456\nuser_usec 100000\nsystem_usec 23456\n";
        assert_eq!(parse_cpu_stat(s), 123456);
    }

    #[test]
    fn memory_events_oom() {
        let s = "low 0\nhigh 5\nmax 1\noom 0\noom_kill 3\n";
        assert_eq!(parse_kv_u64(s, "oom_kill"), 3);
        assert_eq!(parse_kv_u64(s, "nope"), 0);
    }

    #[test]
    fn io_stat_sums_devices() {
        let s = "8:0 rbytes=100 wbytes=200 rios=1 wios=2\n8:16 rbytes=50 wbytes=25 rios=0 wios=0\n";
        assert_eq!(parse_io_stat(s), (150, 225));
    }

    #[test]
    fn locate_never_panics() {
        // Returns Some on cgroup-v2 hosts, None elsewhere (e.g. Android).
        let _ = locate(std::process::id());
    }

    #[test]
    fn missing_dir_is_none() {
        let cg = Cgroup {
            dir: PathBuf::from("/nonexistent-hbmon-cgroup-xyz"),
        };
        assert!(read_stats(&cg).is_none());
    }
}
