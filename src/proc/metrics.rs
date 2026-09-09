use super::{Metrics, ProcessInspector};
use std::collections::HashMap;

pub fn aggregate(insp: &dyn ProcessInspector, root: u32) -> (Metrics, Vec<u32>) {
    let pids = super::collect_descendants(insp, root, 1000);
    let bulk = insp.bulk_metrics(&pids).unwrap_or_default();
    let mut tot = Metrics::default();
    for m in bulk.values() {
        tot.cpu_pct += m.cpu_pct;
        tot.rss_mb = tot.rss_mb.saturating_add(m.rss_mb);
        tot.io_read_bytes = tot.io_read_bytes.saturating_add(m.io_read_bytes);
        tot.io_write_bytes = tot.io_write_bytes.saturating_add(m.io_write_bytes);
        tot.fds_open = tot.fds_open.saturating_add(m.fds_open);
        tot.net_tcp = tot.net_tcp.saturating_add(m.net_tcp);
        tot.net_udp = tot.net_udp.saturating_add(m.net_udp);
    }
    (tot, pids)
}

#[allow(dead_code)]
pub fn empty_totals(_m: &HashMap<u32, Metrics>) -> Metrics {
    Metrics::default()
}
