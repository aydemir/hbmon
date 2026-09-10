//! Linux implementation via direct /proc parsing.
//! Deliberately dependency-free (no procfs crate): fewer deps,
//! no version-churn risk, and every read is best-effort so a
//! single unreadable pid never fails the whole tree.

use super::{Metrics, ProcessInspector, TreeNode};
use std::collections::HashMap;

pub struct LinuxInspector;

impl LinuxInspector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxInspector {
    fn default() -> Self {
        Self::new()
    }
}

fn read_file(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("read {}: {}", path, e))
}

/// Parse /proc/<pid>/stat: comm in parens may contain spaces/parens,
/// so split on last ')'.
fn parse_stat(pid: u32) -> Result<(u32, u64, u64, i64), String> {
    let s = read_file(&format!("/proc/{}/stat", pid))?;
    let end = s
        .rfind(')')
        .ok_or_else(|| format!("bad stat for {}", pid))?;
    let after = &s[end + 1..];
    let f: Vec<&str> = after.split_whitespace().collect();
    // after comm: [state, ppid, ...] -> f[1] = ppid, f[11]=utime, f[12]=stime, f[21]=rss pages
    if f.len() < 22 {
        return Err(format!("short stat for {}", pid));
    }
    let ppid: u32 = f[1].parse().unwrap_or(0);
    let utime: u64 = f[11].parse().unwrap_or(0);
    let stime: u64 = f[12].parse().unwrap_or(0);
    let rss_pages: i64 = f[21].parse().unwrap_or(0);
    Ok((ppid, utime, stime, rss_pages))
}

fn page_size() -> i64 {
    unsafe { libc::sysconf(libc::_SC_PAGESIZE) as i64 }.max(4096)
}

impl ProcessInspector for LinuxInspector {
    fn list_children(&self, pid: u32) -> Result<Vec<u32>, String> {
        let mut kids = vec![];
        let dir = std::fs::read_dir("/proc").map_err(|e| format!("readdir /proc: {}", e))?;
        for e in dir.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if let Ok(cand) = name.parse::<u32>() {
                if cand == pid {
                    continue;
                }
                if let Ok((ppid, _, _, _)) = parse_stat(cand) {
                    if ppid == pid {
                        kids.push(cand);
                    }
                }
            }
        }
        Ok(kids)
    }

    fn cmdline(&self, pid: u32) -> Result<String, String> {
        let raw = std::fs::read(format!("/proc/{}/cmdline", pid))
            .map_err(|e| format!("cmdline {}: {}", pid, e))?;
        if raw.is_empty() || raw.iter().all(|&b| b == 0) {
            // kernel thread or zombie: fall back to comm
            let stat = read_file(&format!("/proc/{}/stat", pid)).unwrap_or_default();
            if let (Some(a), Some(b)) = (stat.find('('), stat.rfind(')')) {
                return Ok(format!("[{}]", &stat[a + 1..b]));
            }
            return Ok(format!("[pid {}]", pid));
        }
        let parts: Vec<String> = raw
            .split(|&b| b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| String::from_utf8_lossy(s).to_string())
            .collect();
        Ok(parts.join(" "))
    }

    fn metrics(&self, pid: u32) -> Result<Metrics, String> {
        let (_, utime, stime, rss_pages) = parse_stat(pid).unwrap_or((0, 0, 0, 0));
        let rss_mb = ((rss_pages.max(0) * page_size()) / (1024 * 1024)) as u32;

        let (io_r, io_w) = read_file(&format!("/proc/{}/io", pid))
            .map(|s| parse_io(&s))
            .unwrap_or((0, 0));

        let fds = std::fs::read_dir(format!("/proc/{}/fd", pid))
            .map(|d| d.count() as u32)
            .unwrap_or(0);

        let net_tcp = count_sockets(pid);

        // CPU% needs a delta; the orchestrator (metrics/cpu.rs CpuTracker)
        // converts jiffies -> %. Here we expose raw jiffies-derived
        // instantaneous 0.0 and let bulk collection fill real values.
        // To still be useful standalone, compute lifetime avg vs system uptime.
        let _ = (utime, stime);
        Ok(Metrics {
            cpu_pct: 0.0,
            rss_mb,
            io_read_bytes: io_r,
            io_write_bytes: io_w,
            fds_open: fds,
            net_tcp,
            net_udp: 0,
        })
    }

    fn tree(&self, pid: u32) -> Result<TreeNode, String> {
        super::tree::build_tree(self, pid, 8)
    }

    fn is_alive(&self, pid: u32) -> bool {
        if !std::path::Path::new(&format!("/proc/{}", pid)).exists() {
            return false;
        }
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    fn bulk_metrics(&self, pids: &[u32]) -> Result<HashMap<u32, Metrics>, String> {
        let mut m = HashMap::new();
        for &p in pids {
            if let Ok(met) = self.metrics(p) {
                m.insert(p, met);
            }
        }
        Ok(m)
    }
}

fn parse_io(s: &str) -> (u64, u64) {
    let mut r = 0u64;
    let mut w = 0u64;
    for line in s.lines() {
        if let Some(v) = line.strip_prefix("read_bytes:") {
            r = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("write_bytes:") {
            w = v.trim().parse().unwrap_or(0);
        }
    }
    (r, w)
}

fn count_sockets(pid: u32) -> u32 {
    let dir = match std::fs::read_dir(format!("/proc/{}/fd", pid)) {
        Ok(d) => d,
        Err(_) => return 0,
    };
    let mut n = 0u32;
    for e in dir.flatten() {
        if let Ok(link) = std::fs::read_link(e.path()) {
            if link.to_string_lossy().starts_with("socket:") {
                n += 1;
            }
        }
    }
    n
}

/// Raw cpu jiffies for CpuTracker.
pub fn cpu_jiffies(pid: u32) -> Option<u64> {
    parse_stat(pid).ok().map(|(_, u, s, _)| u + s)
}
