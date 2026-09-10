use super::{ProcessInspector, TreeNode};

/// Recursive tree builder with depth cap + cycle guard (RFC 13.3: >1000 procs).
pub fn build_tree(
    insp: &dyn ProcessInspector,
    pid: u32,
    max_depth: usize,
) -> Result<TreeNode, String> {
    let mut seen = std::collections::HashSet::new();
    build_into(insp, pid, max_depth, &mut seen)
}

fn build_into(
    insp: &dyn ProcessInspector,
    pid: u32,
    depth: usize,
    seen: &mut std::collections::HashSet<u32>,
) -> Result<TreeNode, String> {
    if !seen.insert(pid) {
        return Err("cycle".to_string());
    }
    let cmd = insp
        .cmdline(pid)
        .unwrap_or_else(|_| format!("[pid {}]", pid));
    let (cpu, rss) = insp
        .metrics(pid)
        .map(|m| (m.cpu_pct, m.rss_mb))
        .unwrap_or((0.0, 0));
    let alive = insp.is_alive(pid);
    let mut children = vec![];
    if depth > 0 && alive {
        if let Ok(kids) = insp.list_children(pid) {
            for k in kids.into_iter().take(64) {
                if let Ok(n) = build_into(insp, k, depth - 1, seen) {
                    children.push(n);
                }
                if children.len() >= 64 {
                    break;
                }
            }
        }
    }
    Ok(TreeNode {
        pid,
        cmd,
        cpu,
        rss_mb: rss,
        state: if alive {
            "running".to_string()
        } else {
            "exited".to_string()
        },
        children,
    })
}
