pub fn parse_io_content(s: &str) -> (u64, u64) {
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
