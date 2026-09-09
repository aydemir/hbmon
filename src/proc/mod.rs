pub mod linux;
pub mod macos;
pub mod metrics;
pub mod tree;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metrics {
    pub cpu_pct: f32,
    pub rss_mb: u32,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    pub fds_open: u32,
    pub net_tcp: u32,
    pub net_udp: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    pub pid: u32,
    pub cmd: String,
    pub cpu: f32,
    pub rss_mb: u32,
    pub state: String,
    #[serde(default)]
    pub children: Vec<TreeNode>,
}

/// OS-independent contract (RFC Section 10).
pub trait ProcessInspector: Send + Sync {
    fn list_children(&self, pid: u32) -> Result<Vec<u32>, String>;
    fn metrics(&self, pid: u32) -> Result<Metrics, String>;
    fn cmdline(&self, pid: u32) -> Result<String, String>;
    fn tree(&self, pid: u32) -> Result<TreeNode, String>;
    fn is_alive(&self, pid: u32) -> bool;
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

/// Collect full descendant set (BFS) with cycle guard.
pub fn collect_descendants(
    insp: &dyn ProcessInspector,
    root: u32,
    max: usize,
) -> Vec<u32> {
    let mut out = vec![root];
    let mut queue = vec![root];
    let mut seen = std::collections::HashSet::new();
    seen.insert(root);
    while let Some(pid) = queue.pop() {
        if out.len() >= max {
            break;
        }
        if let Ok(kids) = insp.list_children(pid) {
            for k in kids {
                if seen.insert(k) {
                    out.push(k);
                    queue.push(k);
                    if out.len() >= max {
                        break;
                    }
                }
            }
        }
    }
    out
}
